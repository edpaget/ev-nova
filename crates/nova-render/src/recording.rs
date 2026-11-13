//! Test support: a [`Gpu`] that records what it is asked to do, and
//! [`NoImages`], an [`ImageSource`] with nothing in it.
//!
//! Compiled for this crate's tests and, through the `recording` feature,
//! for other crates' tests.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use nova_data::graphics::Image;
use nova_view::ImageKind;

use crate::gpu::{Frame, Gpu, PageId, PlanetId, PlanetImage, PlanetTextures};
use crate::images::{ImageError, ImageSource};
use crate::model::{Mesh, MeshId};
use crate::model_texture::{ModelTexture, ModelTextureId};
use crate::sky::{SkyImage, SkyImageId};
use crate::upscale::{Upscaler, UpscalerProbe, WorldResolution, resolution_of};
use crate::viewport::PixelRect;

/// An [`ImageSource`] with no images at all: every
/// [`ImageSource::frames`] is [`ImageError::Missing`], and
/// [`ImageSource::deep_frames`] keeps its default, so it is missing too.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoImages;

impl ImageSource for NoImages {
    fn frames(&self, _kind: ImageKind, _id: i16) -> Result<Vec<Image>, ImageError> {
        Err(ImageError::Missing)
    }
}

/// One call a [`RecordingGpu`] received.
#[derive(Clone, Debug, PartialEq)]
pub enum GpuCall {
    /// [`Gpu::create_page`].
    CreatePage {
        /// The page.
        page: PageId,
        /// Its side length.
        size: u32,
    },
    /// [`Gpu::upload`], with the image summarised.
    Upload {
        /// The page.
        page: PageId,
        /// Where on the page.
        rect: PixelRect,
        /// The image's width.
        width: u32,
        /// The image's height.
        height: u32,
        /// A hash of the image's pixels.
        pixels_hash: u64,
    },
    /// [`Gpu::clear_pages`].
    ClearPages,
    /// [`Gpu::upload_model_texture`].
    UploadModelTexture {
        /// The texture's id.
        texture: ModelTextureId,
        /// The texture.
        data: ModelTexture,
    },
    /// [`Gpu::upload_mesh`].
    UploadMesh {
        /// The mesh's id.
        mesh: MeshId,
        /// The mesh.
        data: Mesh,
        /// The ids its textures were uploaded under, in the mesh's order.
        textures: Vec<ModelTextureId>,
    },
    /// [`Gpu::upload_planet`].
    UploadPlanet {
        /// The planet's id.
        planet: PlanetId,
        /// Its textures.
        textures: PlanetTextures,
    },
    /// [`Gpu::release_planet`].
    ReleasePlanet {
        /// The planet's id.
        planet: PlanetId,
    },
    /// [`Gpu::upload_sky_image`].
    UploadSkyImage {
        /// The image's id.
        image: SkyImageId,
        /// The image.
        data: SkyImage,
    },
    /// [`Gpu::submit`].
    Submit(Frame),
}

impl GpuCall {
    /// The call that uploads `image` to `page` at `rect`.
    #[must_use]
    pub fn upload(page: PageId, rect: PixelRect, image: &Image) -> Self {
        let mut hasher = DefaultHasher::new();
        image.pixels().hash(&mut hasher);
        Self::Upload {
            page,
            rect,
            width: image.width(),
            height: image.height(),
            pixels_hash: hasher.finish(),
        }
    }
}

/// Records every call, in order. Asked, it draws the world at
/// [`WorldResolution::Native`] unless made [`answering`](Self::answering)
/// otherwise, or until an upscaler is set ([`Gpu::set_upscaler`]): then at
/// that upscaler's resolution. The question is not a call, and nor is
/// setting the upscaler, which is kept ([`upscaler`](Self::upscaler)). As
/// an [`UpscalerProbe`] it answers native alone unless made
/// [`probing`](Self::probing) a list.
#[derive(Clone, Debug)]
pub struct RecordingGpu {
    /// The calls received, oldest first.
    pub calls: Vec<GpuCall>,
    world_resolution: WorldResolution,
    upscaler: Upscaler,
    probed: Vec<Upscaler>,
    warnings: Vec<String>,
}

impl Default for RecordingGpu {
    fn default() -> Self {
        Self {
            calls: Vec::new(),
            world_resolution: WorldResolution::Native,
            upscaler: Upscaler::Native,
            probed: vec![Upscaler::Native],
            warnings: Vec::new(),
        }
    }
}

impl RecordingGpu {
    /// A recorder with no calls.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// A recorder with no calls that draws the world at `resolution`.
    #[must_use]
    pub fn answering(resolution: WorldResolution) -> Self {
        Self {
            world_resolution: resolution,
            ..Self::default()
        }
    }

    /// A recorder with no calls whose probe answers `upscalers`.
    #[must_use]
    pub fn probing(upscalers: &[Upscaler]) -> Self {
        Self {
            probed: upscalers.to_vec(),
            ..Self::default()
        }
    }

    /// The upscaler last set ([`Gpu::set_upscaler`]), native before any.
    #[must_use]
    pub fn upscaler(&self) -> Upscaler {
        self.upscaler
    }

    /// The frames submitted, oldest first.
    #[must_use]
    pub fn submits(&self) -> Vec<&Frame> {
        self.calls
            .iter()
            .filter_map(|call| match call {
                GpuCall::Submit(frame) => Some(frame),
                _ => None,
            })
            .collect()
    }

    /// Raises `warning`, for [`Gpu::take_warnings`] to hand out. Raising
    /// one is not a call.
    pub fn warn(&mut self, warning: &str) {
        self.warnings.push(warning.to_owned());
    }

    /// Takes the calls received so far, leaving none.
    pub fn take(&mut self) -> Vec<GpuCall> {
        std::mem::take(&mut self.calls)
    }

    /// The latest textures uploaded under each planet id, of the planets
    /// not released since.
    fn planets(&self) -> std::collections::BTreeMap<PlanetId, &PlanetTextures> {
        let mut held = std::collections::BTreeMap::new();
        for call in &self.calls {
            match call {
                GpuCall::UploadPlanet { planet, textures } => {
                    held.insert(*planet, textures);
                }
                GpuCall::ReleasePlanet { planet } => {
                    held.remove(planet);
                }
                _ => {}
            }
        }
        held
    }

    /// Each planet a submitted frame draws (an instance of a
    /// [`Batch::Planets`](crate::gpu::Batch::Planets)) that it did not hold
    /// when the frame was submitted, with the frame's place among the
    /// submits ([`submits`](Self::submits)): one never uploaded, or
    /// released since. An adapter draws such a planet as nothing.
    #[must_use]
    pub fn unheld_planets_drawn(&self) -> Vec<(usize, PlanetId)> {
        let mut held = std::collections::BTreeSet::new();
        let mut unheld = Vec::new();
        let mut submits = 0;
        for call in &self.calls {
            match call {
                GpuCall::UploadPlanet { planet, .. } => {
                    held.insert(*planet);
                }
                GpuCall::ReleasePlanet { planet } => {
                    held.remove(planet);
                }
                GpuCall::Submit(frame) => {
                    for batch in &frame.batches {
                        if let crate::gpu::Batch::Planets { planets, .. } = batch {
                            unheld.extend(
                                (planets.iter())
                                    .filter(|instance| !held.contains(&instance.planet))
                                    .map(|instance| (submits, instance.planet)),
                            );
                        }
                    }
                    submits += 1;
                }
                _ => {}
            }
        }
        unheld
    }

    /// The planets it holds textures for: uploaded, and not released
    /// since.
    #[must_use]
    pub fn planets_held(&self) -> std::collections::BTreeSet<PlanetId> {
        self.planets().into_keys().collect()
    }

    /// The bytes of planet textures it holds: every level of the latest
    /// textures uploaded under each planet id not released since, surface,
    /// clouds and rings.
    #[must_use]
    pub fn planet_bytes_held(&self) -> usize {
        self.planets()
            .values()
            .map(|textures| {
                let levels =
                    |chain: &[PlanetImage]| chain.iter().map(|i| i.rgba.len()).sum::<usize>();
                levels(&textures.surface) + levels(&textures.clouds) + textures.rings.rgba.len()
            })
            .sum::<usize>()
    }
}

impl Gpu for RecordingGpu {
    fn create_page(&mut self, page: PageId, size: u32) {
        self.calls.push(GpuCall::CreatePage { page, size });
    }

    fn upload(&mut self, page: PageId, at: PixelRect, image: &Image) {
        self.calls.push(GpuCall::upload(page, at, image));
    }

    fn clear_pages(&mut self) {
        self.calls.push(GpuCall::ClearPages);
    }

    fn upload_model_texture(&mut self, texture: ModelTextureId, data: &ModelTexture) {
        self.calls.push(GpuCall::UploadModelTexture {
            texture,
            data: data.clone(),
        });
    }

    fn upload_mesh(&mut self, mesh: MeshId, data: &Mesh, textures: &[ModelTextureId]) {
        self.calls.push(GpuCall::UploadMesh {
            mesh,
            data: data.clone(),
            textures: textures.to_vec(),
        });
    }

    fn upload_planet(&mut self, planet: PlanetId, textures: &PlanetTextures) {
        self.calls.push(GpuCall::UploadPlanet {
            planet,
            textures: textures.clone(),
        });
    }

    fn release_planet(&mut self, planet: PlanetId) {
        self.calls.push(GpuCall::ReleasePlanet { planet });
    }

    fn upload_sky_image(&mut self, image: SkyImageId, data: &SkyImage) {
        self.calls.push(GpuCall::UploadSkyImage {
            image,
            data: data.clone(),
        });
    }

    fn submit(&mut self, frame: &Frame) {
        self.calls.push(GpuCall::Submit(frame.clone()));
    }

    fn world_resolution(&self) -> WorldResolution {
        self.world_resolution
    }

    fn set_upscaler(&mut self, upscaler: Upscaler) {
        self.upscaler = upscaler;
        self.world_resolution = resolution_of(upscaler);
    }

    fn take_warnings(&mut self) -> Vec<String> {
        std::mem::take(&mut self.warnings)
    }
}

impl UpscalerProbe for RecordingGpu {
    fn upscalers(&self) -> Vec<Upscaler> {
        self.probed.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_draws_the_world_natively_unless_told_and_asking_is_no_call() {
        let gpu = RecordingGpu::new();
        assert_eq!(gpu.world_resolution(), WorldResolution::Native);
        let logical = RecordingGpu::answering(WorldResolution::Logical);
        assert_eq!(logical.world_resolution(), WorldResolution::Logical);
        assert_eq!(logical.calls, []);
    }

    #[test]
    fn its_upscaler_is_kept_not_recorded_and_decides_the_resolution() {
        let mut gpu = RecordingGpu::new();
        assert_eq!(gpu.upscaler(), Upscaler::Native);
        gpu.set_upscaler(Upscaler::SharpBilinear);
        assert_eq!(gpu.upscaler(), Upscaler::SharpBilinear);
        assert_eq!(gpu.world_resolution(), WorldResolution::Logical);
        gpu.set_upscaler(Upscaler::Native);
        assert_eq!(gpu.upscaler(), Upscaler::Native);
        assert_eq!(gpu.world_resolution(), WorldResolution::Native);
        assert_eq!(gpu.calls, [], "not a call");
    }

    #[test]
    fn answering_still_decides_the_resolution_until_an_upscaler_is_set() {
        let mut gpu = RecordingGpu::answering(WorldResolution::Logical);
        assert_eq!(gpu.world_resolution(), WorldResolution::Logical);
        gpu.set_upscaler(Upscaler::Native);
        assert_eq!(gpu.world_resolution(), WorldResolution::Native);
    }

    #[test]
    fn clearing_the_pages_is_a_call_recorded_in_order() {
        let mut gpu = RecordingGpu::new();
        gpu.create_page(PageId(0), 64);
        gpu.clear_pages();
        assert_eq!(
            gpu.calls,
            [
                GpuCall::CreatePage {
                    page: PageId(0),
                    size: 64
                },
                GpuCall::ClearPages
            ]
        );
    }

    #[test]
    fn recording_gpu_records_a_mesh_upload_and_its_textures_ids() {
        let mut gpu = RecordingGpu::new();
        let cube = Mesh::cube([nova_view::Color::WHITE; 6]);
        gpu.upload_mesh(MeshId(3), &cube, &[ModelTextureId(4), ModelTextureId(1)]);
        assert_eq!(
            gpu.calls,
            [GpuCall::UploadMesh {
                mesh: MeshId(3),
                data: cube,
                textures: vec![ModelTextureId(4), ModelTextureId(1)],
            }]
        );
    }

    #[test]
    fn recording_gpu_records_a_model_texture_upload() {
        let mut gpu = RecordingGpu::new();
        let texture = ModelTexture::new(2, 1, vec![1, 2, 3, 4, 5, 6, 7, 8]).expect("two texels");
        gpu.upload_model_texture(ModelTextureId(6), &texture);
        assert_eq!(
            gpu.calls,
            [GpuCall::UploadModelTexture {
                texture: ModelTextureId(6),
                data: texture,
            }]
        );
    }

    #[test]
    fn recording_gpu_records_a_sky_image_upload() {
        let mut gpu = RecordingGpu::new();
        let image = SkyImage::new(2, 1, vec![1, 2, 3, 255, 5, 6, 7, 255]).expect("two texels");
        gpu.upload_sky_image(SkyImageId(4), &image);
        assert_eq!(
            gpu.calls,
            [GpuCall::UploadSkyImage {
                image: SkyImageId(4),
                data: image,
            }]
        );
    }

    /// Planet textures whose surface is `width` by `width / 2`, with no
    /// clouds (a 2 by 1 clear layer) and a ring profile of `ring` texels.
    fn planet_textures(width: u32, ring: u32) -> PlanetTextures {
        let image = |width: u32, height: u32| crate::gpu::PlanetImage {
            width,
            height,
            rgba: vec![0; (width * height * 4) as usize],
        };
        PlanetTextures {
            surface: vec![image(width, width / 2), image(width / 2, width / 4)],
            clouds: vec![image(2, 1), image(1, 1)],
            rings: image(ring, 1),
        }
    }

    #[test]
    fn recording_gpu_counts_the_planet_bytes_the_gpu_holds() {
        let mut gpu = RecordingGpu::new();
        assert_eq!(gpu.planet_bytes_held(), 0);
        // 8x4 + 4x2 surface, 2x1 + 1x1 clouds, 3x1 rings: 4 bytes a texel.
        gpu.upload_planet(PlanetId(0), &planet_textures(8, 3));
        let first = (32 + 8 + 2 + 1 + 3) * 4;
        assert_eq!(gpu.planet_bytes_held(), first);
        // 16x8 + 8x4 surface, the same clouds, 2x1 rings.
        gpu.upload_planet(PlanetId(1), &planet_textures(16, 2));
        let second = (128 + 32 + 2 + 1 + 2) * 4;
        assert_eq!(
            gpu.planet_bytes_held(),
            first + second,
            "every level of both"
        );
        // A second upload under one id replaces the first.
        gpu.upload_planet(PlanetId(0), &planet_textures(16, 2));
        assert_eq!(gpu.planet_bytes_held(), 2 * second);
    }

    #[test]
    fn recording_gpu_records_a_planet_release_and_holds_it_no_longer() {
        let mut gpu = RecordingGpu::new();
        gpu.upload_planet(PlanetId(0), &planet_textures(8, 3));
        gpu.upload_planet(PlanetId(1), &planet_textures(16, 2));
        assert_eq!(
            gpu.planets_held(),
            [PlanetId(0), PlanetId(1)].into_iter().collect()
        );
        gpu.release_planet(PlanetId(0));
        assert_eq!(
            gpu.calls[2],
            GpuCall::ReleasePlanet {
                planet: PlanetId(0)
            }
        );
        assert_eq!(gpu.planets_held(), [PlanetId(1)].into_iter().collect());
        assert_eq!(gpu.planet_bytes_held(), (128 + 32 + 2 + 1 + 2) * 4);
        // Uploaded again, it is held again: the latest upload counts.
        gpu.upload_planet(PlanetId(0), &planet_textures(8, 3));
        assert_eq!(gpu.planets_held().len(), 2);
        assert_eq!(
            gpu.planet_bytes_held(),
            (128 + 32 + 2 + 1 + 2) * 4 + (32 + 8 + 2 + 1 + 3) * 4
        );
        gpu.release_planet(PlanetId(1));
        gpu.release_planet(PlanetId(0));
        assert!(gpu.planets_held().is_empty());
        assert_eq!(gpu.planet_bytes_held(), 0);
    }

    /// A frame drawing planets `ids`, each in a batch of its own.
    fn drawing(ids: &[u32]) -> Frame {
        let figure = crate::planet::Figure {
            rings: None,
            atmosphere: None,
            day: 10.0,
            tilt: 0.0,
        };
        let pose = nova_view::PlanetPose {
            center: nova_view::Point::new(10.0, 10.0),
            radius: 4.0,
            time: 0.0,
        };
        let batches = ids
            .iter()
            .map(|&id| crate::gpu::Batch::Planets {
                light: crate::planet::PLANET_LIGHT,
                planets: vec![
                    crate::planet::figure_instance(PlanetId(id), &figure, &pose).expect("drawable"),
                ],
                clip: None,
            })
            .collect();
        Frame {
            target: (64, 48),
            viewport: PixelRect {
                x: 0,
                y: 0,
                w: 64,
                h: 48,
            },
            logical: crate::viewport::LogicalSize { w: 32, h: 24 },
            clear: nova_view::Color::BLACK,
            world_space: crate::gpu::WorldSpace::Target,
            batches,
            world: Vec::new(),
            world_clip: None,
            world_tail: None,
        }
    }

    #[test]
    fn recording_gpu_finds_each_planet_a_frame_draws_that_it_did_not_hold() {
        let mut gpu = RecordingGpu::new();
        gpu.submit(&drawing(&[]));
        gpu.upload_planet(PlanetId(0), &planet_textures(8, 3));
        gpu.submit(&drawing(&[0]));
        assert_eq!(gpu.unheld_planets_drawn(), []);
        gpu.submit(&drawing(&[0, 1]));
        gpu.release_planet(PlanetId(0));
        gpu.submit(&drawing(&[0]));
        gpu.upload_planet(PlanetId(0), &planet_textures(8, 3));
        gpu.upload_planet(PlanetId(1), &planet_textures(8, 3));
        gpu.submit(&drawing(&[1, 0]));
        assert_eq!(
            gpu.unheld_planets_drawn(),
            [(2, PlanetId(1)), (3, PlanetId(0))],
            "by the frame's place among the submits"
        );
    }

    #[test]
    fn its_warnings_are_handed_out_once_and_are_no_call() {
        let mut gpu = RecordingGpu::new();
        assert_eq!(gpu.take_warnings(), Vec::<String>::new());
        gpu.warn("first");
        gpu.warn("second");
        assert_eq!(gpu.take_warnings(), ["first", "second"]);
        assert_eq!(gpu.take_warnings(), Vec::<String>::new(), "once");
        assert_eq!(gpu.calls, []);
    }

    #[test]
    fn its_probe_answers_native_alone_unless_given_a_list() {
        assert_eq!(RecordingGpu::new().upscalers(), [Upscaler::Native]);
        let both = [Upscaler::SharpBilinear, Upscaler::Native];
        let gpu = RecordingGpu::probing(&both);
        assert_eq!(gpu.upscalers(), both);
        assert_eq!(gpu.calls, []);
        assert_eq!(gpu.world_resolution(), WorldResolution::Native);
        assert_eq!(RecordingGpu::probing(&[]).upscalers(), []);
    }

    #[test]
    fn no_images_has_every_image_missing() {
        assert_eq!(
            NoImages.frames(ImageKind::Pict, 128),
            Err(ImageError::Missing)
        );
        assert_eq!(
            NoImages.deep_frames(ImageKind::Pict, 128),
            Err(ImageError::Missing)
        );
    }
}
