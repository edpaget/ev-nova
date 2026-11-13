//! The [`Renderer`](crate::Renderer)'s sky images: which of them the GPU
//! holds, and the [`Batch::SkyImages`] a sky's decorations become.
//!
//! A sky ([`nova_view::DrawCommand::Sky`]) draws the decorations its
//! seed's list holds ([`SkyLooks::decorations`]), placed by the core
//! ([`sky::placed`]) for the view and the texels of the canvas it draws
//! into, as one batch after the sky's own. A sky whose decorations place
//! none (an empty list, or every one off the view) is no batch.
//!
//! # Which images the GPU holds
//!
//! An image is numbered ([`SkyImageId`]) and uploaded
//! ([`Gpu::upload_sky_image`]) the first frame that draws a decoration
//! holding its pointer, before that frame is submitted, and every later
//! decoration holding the same pointer names the same number: systems
//! that list one asset share one upload, as meshes share a model texture
//! (`crate::model_texture`). Nothing is hashed or compared. An image never
//! drawn is never uploaded. Sky images are not atlas pages, so a new
//! enlarger leaves them where they are, and a new set of looks
//! ([`Renderer::set_sky_looks`](crate::Renderer::set_sky_looks)) holding
//! the same pointers uploads nothing again. An image no look holds any
//! more stays on the GPU: the port has no call that drops one.

use std::collections::HashMap;
use std::sync::{Arc, Weak};

use nova_view::SkyPose;

use crate::gpu::{Batch, Gpu, SkyImageInstance};
use crate::sky::{self, SkyImage, SkyImageId, SkyLooks};
use crate::viewport::LogicalSize;

/// The sky images the GPU holds, by pointer (see the [module docs](self)).
#[derive(Debug, Default)]
pub(crate) struct SkyImages {
    /// Each image uploaded, by its pointer's address, with the number it
    /// took. The weak pointer keeps the address its own: an address is
    /// not handed to another image while any pointer to its first
    /// holder, weak or strong, is alive.
    held: HashMap<usize, (Weak<SkyImage>, SkyImageId)>,
    /// The number the next new image takes.
    next: u32,
}

impl SkyImages {
    /// Draws the decorations of sky `pose` onto `batches`, as `looks`
    /// lists them for its seed, in a view of `logical` size drawn into a
    /// canvas of `texels` texels a logical unit: one
    /// [`Batch::SkyImages`], each image uploaded through `gpu` first if
    /// its pointer hasn't been. Nothing when none is placed.
    pub(crate) fn draw(
        &mut self,
        pose: &SkyPose,
        looks: &SkyLooks,
        logical: LogicalSize,
        texels: f32,
        batches: &mut Vec<Batch>,
        gpu: &mut impl Gpu,
    ) {
        let Some(list) = looks.decorations(pose.seed) else {
            return;
        };
        let images: Vec<SkyImageInstance> = sky::placed(pose, list, logical, texels)
            .into_iter()
            .map(|(index, placement)| SkyImageInstance {
                image: self.id(&list[index].image, gpu),
                placement,
            })
            .collect();
        if !images.is_empty() {
            batches.push(Batch::SkyImages(images));
        }
    }

    /// The number `image` is uploaded under, uploading it through `gpu`
    /// first if this pointer hasn't been.
    fn id(&mut self, image: &Arc<SkyImage>, gpu: &mut impl Gpu) -> SkyImageId {
        let address = Arc::as_ptr(image) as usize;
        if let Some((_, id)) = self.held.get(&address) {
            return *id;
        }
        let id = SkyImageId(self.next);
        self.next += 1;
        gpu.upload_sky_image(id, image);
        self.held.insert(address, (Arc::downgrade(image), id));
        id
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use nova_view::{DrawList, Point};

    use super::*;
    use crate::Renderer;
    use crate::enlarge::NativeDetail;
    use crate::gpu::Frame;
    use crate::recording::{GpuCall, NoImages, RecordingGpu};
    use crate::sky::{BackdropLook, SkyBlend, SkyDecoration, SkyLook, instances};
    use crate::viewport::Viewport;

    const SMALL: LogicalSize = LogicalSize { w: 256, h: 192 };
    const SEED: u64 = 21;

    /// Half a pixel a unit, so the target draws at 0.5 texels a unit.
    fn viewport() -> Viewport {
        Viewport::new(SMALL, (128, 96), 0.5)
    }

    fn sky(seed: u64) -> SkyPose {
        SkyPose::new(seed, Point::new(40.0, -70.0), Point::new(128.0, 96.0), 32)
    }

    /// An opaque 4 by 2 image in `shade`.
    fn image(shade: u8) -> Arc<SkyImage> {
        let rgba = [shade, shade / 2, 255 - shade, 255].repeat(8);
        Arc::new(SkyImage::new(4, 2, rgba).expect("4 by 2"))
    }

    /// A near, turned, tinted hulk painted over.
    fn hulk(image: &Arc<SkyImage>) -> SkyDecoration {
        SkyDecoration {
            parallax: 0.3,
            size: 16.0,
            turn: 30.0,
            tint: Some([1.0, 0.5, 0.25]),
            brightness: 0.5,
            ..SkyDecoration::new(Arc::clone(image), Point::new(20.0, -10.0))
        }
    }

    /// A far glow with every default but its size and blend.
    fn galaxy(image: &Arc<SkyImage>) -> SkyDecoration {
        SkyDecoration {
            size: 32.0,
            blend: SkyBlend::Glow,
            ..SkyDecoration::new(Arc::clone(image), Point::new(-30.0, 25.0))
        }
    }

    /// A look that draws no star and no backdrop.
    fn empty_sky() -> SkyLook {
        SkyLook {
            stars: 0.0,
            tint: None,
            backdrop: BackdropLook {
                color: None,
                amount: Some(0.0),
            },
        }
    }

    /// A renderer whose sky `SEED` lists `list`, over the default look.
    fn renderer(list: Vec<SkyDecoration>) -> Renderer<NoImages> {
        let mut renderer = Renderer::new(NoImages);
        let mut looks = SkyLooks::new();
        looks.set_decorations(SEED, list);
        renderer.set_sky_looks(looks);
        renderer
    }

    fn frame_of(renderer: &mut Renderer<NoImages>, seed: u64, gpu: &mut RecordingGpu) -> Frame {
        let mut list = DrawList::new();
        list.sky(sky(seed));
        renderer.render(&list, &viewport(), gpu);
        gpu.submits()
            .last()
            .map(|frame| (*frame).clone())
            .expect("a frame")
    }

    /// The sky-image uploads `gpu` received, in order.
    fn uploads(gpu: &RecordingGpu) -> Vec<(SkyImageId, SkyImage)> {
        gpu.calls
            .iter()
            .filter_map(|call| match call {
                GpuCall::UploadSkyImage { image, data } => Some((*image, data.clone())),
                _ => None,
            })
            .collect()
    }

    /// What the core places for `list` in these tests' view, each given
    /// the image number `ids` has for its index in the list.
    fn expected(seed: u64, list: &[SkyDecoration], ids: &[u32]) -> Vec<SkyImageInstance> {
        sky::placed(&sky(seed), list, SMALL, 0.5)
            .into_iter()
            .map(|(index, placement)| SkyImageInstance {
                image: SkyImageId(ids[index]),
                placement,
            })
            .collect()
    }

    #[test]
    fn a_skys_decorations_are_a_batch_of_the_cores_placements_after_its_stars() {
        let (grey, blue) = (image(90), image(200));
        let list = vec![hulk(&grey), galaxy(&blue)];
        let mut renderer = renderer(list.clone());
        let mut gpu = RecordingGpu::new();
        let frame = frame_of(&mut renderer, SEED, &mut gpu);
        let stars = instances(&sky(SEED), &SkyLook::default(), SMALL, 0.5);
        assert!(!stars.is_empty());
        // Far to near: the galaxy (the list's second) is drawn, and so
        // uploaded, first.
        let images = expected(SEED, &list, &[1, 0]);
        assert_eq!(images.len(), 2);
        assert_eq!(
            frame.batches,
            [Batch::Sky(stars), Batch::SkyImages(images.clone())]
        );
        assert_eq!(
            uploads(&gpu),
            [
                (SkyImageId(0), (*blue).clone()),
                (SkyImageId(1), (*grey).clone())
            ]
        );
        // Each at its place, size, turn and colour, at the target's half
        // texel a unit: the galaxy 32 units long at parallax 0.05, the
        // hulk 16 long, turned and tinted.
        let far = images[0].placement;
        assert_eq!(
            far.centre,
            Point::new(128.0 - 30.0 - 2.0, 96.0 + 25.0 + 3.5)
        );
        assert_eq!(
            (far.right, far.down),
            (Point::new(16.0, 0.0), Point::new(0.0, 8.0))
        );
        assert_eq!((far.color, far.cover), ([1.0; 3], 0.0));
        // Four texels across 32 units of half a texel each: magnified
        // four times.
        assert!((far.lod + 2.0).abs() < 1e-5, "{}", far.lod);
        let near = images[1].placement;
        assert_eq!(
            near.centre,
            Point::new(128.0 + 20.0 - 12.0, 96.0 - 10.0 + 21.0)
        );
        assert!((near.right.x - 8.0 * 0.866_025_4).abs() < 1e-4, "{near:?}");
        assert!((near.right.y - 4.0).abs() < 1e-4, "{near:?}");
        assert_eq!((near.color, near.cover), ([0.5, 0.25, 0.125], 1.0));
    }

    #[test]
    fn each_image_is_uploaded_once_before_the_first_frame_that_draws_it() {
        let (grey, blue) = (image(90), image(200));
        let mut renderer = renderer(vec![hulk(&grey), galaxy(&blue)]);
        let mut gpu = RecordingGpu::new();
        frame_of(&mut renderer, SEED, &mut gpu);
        let kinds: Vec<&str> = (gpu.calls.iter())
            .map(|call| match call {
                GpuCall::UploadSkyImage { .. } => "image",
                GpuCall::Submit(_) => "submit",
                _ => "other",
            })
            .collect();
        assert_eq!(kinds, ["image", "image", "submit"]);
        // A second frame draws them again and uploads nothing.
        let first = gpu.take();
        let again = frame_of(&mut renderer, SEED, &mut gpu);
        assert_eq!(uploads(&gpu), []);
        assert_eq!(gpu.calls.len(), 1, "the submit alone");
        let Some(GpuCall::Submit(before)) = first.last() else {
            panic!("a submit");
        };
        assert_eq!(again.batches, before.batches);
    }

    #[test]
    fn skies_and_decorations_that_hold_one_image_share_its_one_upload() {
        let shared = image(90);
        let mut renderer = Renderer::new(NoImages);
        let mut looks = SkyLooks::new();
        // One sky shows it twice, another once.
        let twice = vec![hulk(&shared), galaxy(&shared)];
        looks.set_decorations(SEED, twice.clone());
        looks.set_decorations(SEED + 1, vec![galaxy(&shared)]);
        renderer.set_sky_looks(looks);
        let mut gpu = RecordingGpu::new();
        let frame = frame_of(&mut renderer, SEED, &mut gpu);
        // Two instances of the one number.
        assert_eq!(
            frame.batches.last(),
            Some(&Batch::SkyImages(expected(SEED, &twice, &[0, 0])))
        );
        let other = frame_of(&mut renderer, SEED + 1, &mut gpu);
        assert_eq!(
            other.batches.last(),
            Some(&Batch::SkyImages(expected(
                SEED + 1,
                &[galaxy(&shared)],
                &[0]
            )))
        );
        assert_eq!(uploads(&gpu), [(SkyImageId(0), (*shared).clone())]);
    }

    #[test]
    fn two_pointers_to_the_same_pixels_are_two_images() {
        let (one, other) = (image(7), image(7));
        assert_eq!(*one, *other);
        let mut renderer = renderer(vec![galaxy(&one), galaxy(&other)]);
        let mut gpu = RecordingGpu::new();
        frame_of(&mut renderer, SEED, &mut gpu);
        assert_eq!(uploads(&gpu).len(), 2);
    }

    #[test]
    fn a_sky_of_decorations_alone_is_their_batch_alone() {
        let grey = image(90);
        let list = vec![galaxy(&grey)];
        let mut renderer = Renderer::new(NoImages);
        let mut looks = SkyLooks::new();
        looks.insert(SEED, empty_sky());
        looks.set_decorations(SEED, list.clone());
        renderer.set_sky_looks(looks);
        let mut gpu = RecordingGpu::new();
        let frame = frame_of(&mut renderer, SEED, &mut gpu);
        assert_eq!(
            frame.batches,
            [Batch::SkyImages(expected(SEED, &list, &[0]))]
        );
    }

    #[test]
    fn decorations_that_place_none_are_no_batch_and_no_upload() {
        let grey = image(90);
        // An empty list.
        let mut renderer = renderer(Vec::new());
        let mut gpu = RecordingGpu::new();
        let frame = frame_of(&mut renderer, SEED, &mut gpu);
        assert_eq!(frame.batches.len(), 1);
        assert_eq!(frame.batches[0].kind(), "sky");
        // Every decoration off this small view, though a 1024 by 768 one
        // would show it.
        let off = SkyDecoration {
            size: 16.0,
            ..SkyDecoration::new(Arc::clone(&grey), Point::new(400.0, 0.0))
        };
        let list = vec![off];
        assert_eq!(
            sky::placed(&sky(SEED), &list, crate::viewport::LOGICAL, 0.5).len(),
            1
        );
        let mut renderer = renderer_of(list);
        let frame = frame_of(&mut renderer, SEED, &mut gpu);
        assert_eq!(frame.batches.len(), 1);
        assert_eq!(frame.batches[0].kind(), "sky");
        assert_eq!(uploads(&gpu), []);
        // Level 0 draws no sky at all, decorations included.
        let mut renderer = renderer_of(vec![galaxy(&grey)]);
        let mut list = DrawList::new();
        list.sky(SkyPose::new(
            SEED,
            Point::new(0.0, 0.0),
            Point::new(128.0, 96.0),
            0,
        ));
        renderer.render(&list, &viewport(), &mut gpu);
        assert_eq!(gpu.submits().last().expect("a frame").batches, []);
        assert_eq!(uploads(&gpu), []);
    }

    /// [`renderer`], named apart so a test can make a second.
    fn renderer_of(list: Vec<SkyDecoration>) -> Renderer<NoImages> {
        renderer(list)
    }

    #[test]
    fn a_seed_with_no_list_draws_no_images() {
        let grey = image(90);
        // Another seed's list is not this one's.
        let mut renderer = Renderer::new(NoImages);
        let mut looks = SkyLooks::new();
        looks.set_decorations(SEED + 1, vec![galaxy(&grey)]);
        renderer.set_sky_looks(looks);
        let mut gpu = RecordingGpu::new();
        let frame = frame_of(&mut renderer, SEED, &mut gpu);
        let kinds: Vec<&str> = frame.batches.iter().map(Batch::kind).collect();
        assert_eq!(kinds, ["sky"]);
        assert_eq!(uploads(&gpu), []);
        // Nor does a renderer with no looks at all.
        let mut plain = Renderer::new(NoImages);
        let frame = frame_of(&mut plain, SEED, &mut gpu);
        let kinds: Vec<&str> = frame.batches.iter().map(Batch::kind).collect();
        assert_eq!(kinds, ["sky"]);
    }

    #[test]
    fn a_new_enlarger_and_new_looks_holding_the_same_image_upload_nothing_again() {
        let grey = image(90);
        let list = vec![galaxy(&grey)];
        let mut renderer = renderer(list.clone());
        let mut gpu = RecordingGpu::new();
        frame_of(&mut renderer, SEED, &mut gpu);
        assert_eq!(uploads(&gpu).len(), 1);
        gpu.take();
        // A new enlarger starts the atlas afresh and keeps the images.
        renderer.set_enlarger(Box::new(NativeDetail));
        let frame = frame_of(&mut renderer, SEED, &mut gpu);
        assert_eq!(uploads(&gpu), []);
        assert_eq!(
            frame.batches.last(),
            Some(&Batch::SkyImages(expected(SEED, &list, &[0])))
        );
        // New looks holding the same pointer, for another seed too.
        let mut looks = SkyLooks::new();
        looks.set_decorations(SEED, list.clone());
        looks.set_decorations(SEED + 5, list.clone());
        renderer.set_sky_looks(looks);
        frame_of(&mut renderer, SEED, &mut gpu);
        let frame = frame_of(&mut renderer, SEED + 5, &mut gpu);
        assert_eq!(uploads(&gpu), []);
        assert_eq!(
            frame.batches.last(),
            Some(&Batch::SkyImages(expected(SEED + 5, &list, &[0])))
        );
    }

    #[test]
    fn a_dropped_image_s_number_is_never_another_image_s() {
        let mut images = SkyImages::default();
        let mut gpu = RecordingGpu::new();
        // Each image is dropped before the next is made, so without the
        // table keeping its address, the next could be given the same one.
        for expected in 0..8 {
            let image = image(expected as u8);
            assert_eq!(images.id(&image, &mut gpu), SkyImageId(expected));
            assert_eq!(images.id(&image, &mut gpu), SkyImageId(expected));
        }
        let numbers: Vec<u32> = uploads(&gpu).iter().map(|(id, _)| id.0).collect();
        assert_eq!(numbers, [0, 1, 2, 3, 4, 5, 6, 7]);
    }

    #[test]
    fn a_sky_drawn_into_a_world_texture_takes_its_texels_for_its_images() {
        use crate::upscale::WorldResolution;
        let grey = image(90);
        let list = vec![galaxy(&grey)];
        let mut renderer = renderer(list.clone());
        // A world sky at the logical resolution: one texel a unit,
        // whatever the window's scale.
        let mut gpu = RecordingGpu::answering(WorldResolution::Logical);
        let mut draws = DrawList::new();
        draws.world(|list| {
            list.sky(sky(SEED));
        });
        renderer.render(&draws, &Viewport::new(SMALL, (512, 384), 2.0), &mut gpu);
        let frame = gpu.submits().last().map(|f| (*f).clone()).expect("a frame");
        let placed = sky::placed(&sky(SEED), &list, SMALL, 1.0);
        let images = placed
            .into_iter()
            .map(|(_, placement)| SkyImageInstance {
                image: SkyImageId(0),
                placement,
            })
            .collect();
        assert_eq!(frame.batches.last(), Some(&Batch::SkyImages(images)));
        // Both the sky's batches are the world's.
        assert_eq!(frame.world, [0, 1]);
    }
}
