//! The wgpu side of [`Batch::SkyImages`](crate::Batch::SkyImages): each
//! sky image's texture ([`crate::Gpu::upload_sky_image`]), the sky-image
//! pipeline and its draws. It decides nothing: each quad's centre, edges,
//! colour, cover and level of detail come from the core
//! ([`crate::sky::placed`]), and `sky_images.wgsl` samples the image at
//! that level and returns the light and the cover
//! [`crate::sky::lit`] defines.
//!
//! The pipeline draws one quad per decoration into [`SCENE_FORMAT`] with
//! no depth attachment. Its blend is `beneath · (1 − cover) + light`
//! ([`crate::sky::over`]): one times the source plus one less the
//! source's alpha times the destination, the destination's alpha left as
//! it is. A glow's cover is 0, so the one blend draws both kinds.
//!
//! An image's texture (its whole mip chain) and its bind group are made
//! when it is uploaded, and the one sampler ([`SKY_SAMPLER`], through
//! [`descriptor`]) with the pipeline: a frame makes none of them.

use std::collections::HashMap;
use std::ops::Range;

use bytemuck::{Pod, Zeroable};

use super::model_samplers::descriptor;
use super::planets::texture;
use super::renderer::{SCENE_FORMAT, pipeline_layout};
use crate::gpu::SkyImageInstance;
use crate::sky::{SKY_SAMPLER, SkyImage, SkyImageId};

/// One sky image as the sky-image shader reads it, per instance.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub(super) struct SkyImageData {
    /// Its centre in logical units, then the vector from it to the
    /// middle of its right edge.
    pub place: [f32; 4],
    /// The vector to the middle of its bottom edge, its cover, then its
    /// level of detail.
    pub shape: [f32; 4],
    /// Its colour, then 0.
    pub color: [f32; 4],
}

impl From<&SkyImageInstance> for SkyImageData {
    fn from(instance: &SkyImageInstance) -> Self {
        let placement = instance.placement;
        let [r, g, b] = placement.color;
        Self {
            place: [
                placement.centre.x,
                placement.centre.y,
                placement.right.x,
                placement.right.y,
            ],
            shape: [
                placement.down.x,
                placement.down.y,
                placement.cover,
                placement.lod,
            ],
            color: [r, g, b, 0.0],
        }
    }
}

/// The sky-image pipeline and the images' textures.
pub(super) struct SkyImages {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    groups: HashMap<SkyImageId, wgpu::BindGroup>,
}

impl SkyImages {
    /// The pipeline and the one sampler, built on `device` with the
    /// projection's `globals` layout as group 0, and no images.
    pub(super) fn new(device: &wgpu::Device, globals: &wgpu::BindGroupLayout) -> Self {
        let layout = image_layout(device);
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("nova sky image"),
            ..descriptor(SKY_SAMPLER)
        });
        Self {
            pipeline: pipeline(device, globals, &layout),
            layout,
            sampler,
            groups: HashMap::new(),
        }
    }

    /// Holds `data` as sky image `image`, replacing any of that id: its
    /// texture and the bind group its draws bind.
    pub(super) fn upload(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        image: SkyImageId,
        data: &SkyImage,
    ) {
        let view = texture(device, queue, "nova sky image", data.texture().levels())
            .create_view(&wgpu::TextureViewDescriptor::default());
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("nova sky image"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });
        self.groups.insert(image, group);
    }

    /// Encodes the images `instances` of `buffer`, image `images[i]` for
    /// the i-th, into `pass`, the projection's `globals` bound, one quad
    /// each. An image the GPU doesn't hold is skipped.
    pub(super) fn encode(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        globals: &wgpu::BindGroup,
        buffer: &wgpu::Buffer,
        instances: Range<u32>,
        images: &[SkyImageId],
    ) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, globals, &[]);
        pass.set_vertex_buffer(0, buffer.slice(..));
        for (instance, image) in instances.zip(images) {
            let Some(group) = self.groups.get(image) else {
                continue;
            };
            pass.set_bind_group(1, group, &[]);
            pass.draw(0..6, instance..instance + 1);
        }
    }
}

/// How a sky image combines with what is beneath it: its light added to
/// what its cover leaves, the destination's alpha kept.
const OVER: wgpu::BlendState = wgpu::BlendState {
    color: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::One,
        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
        operation: wgpu::BlendOperation::Add,
    },
    alpha: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::Zero,
        dst_factor: wgpu::BlendFactor::One,
        operation: wgpu::BlendOperation::Add,
    },
};

/// A sky image's layout: its texture and the sampler.
fn image_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("nova sky image"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    })
}

/// The sky-image pipeline: [`SkyImageData`] per instance in slot 0.
fn pipeline(
    device: &wgpu::Device,
    globals: &wgpu::BindGroupLayout,
    image: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::include_wgsl!("sky_images.wgsl"));
    let layout = pipeline_layout(device, "nova sky images", &[globals, image]);
    let instance = wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32x4, 2 => Float32x4];
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("nova sky images"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("image_vs"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[Some(wgpu::VertexBufferLayout {
                array_stride: size_of::<SkyImageData>() as u64,
                step_mode: wgpu::VertexStepMode::Instance,
                attributes: &instance,
            })],
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("image_fs"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: SCENE_FORMAT,
                blend: Some(OVER),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}

#[cfg(test)]
mod tests {
    use nova_view::Point;

    use super::*;
    use crate::fonts::FontFaces;
    use crate::gpu::{Batch, Frame, Gpu, WorldSpace};
    use crate::sky::{Placement, StarInstance, lit, over};
    use crate::viewport::{LogicalSize, PixelRect};
    use crate::wgpu::OffscreenGpu;

    /// A placement unlike itself in every lane.
    fn placement() -> Placement {
        Placement {
            centre: Point::new(1.0, 2.0),
            right: Point::new(3.0, 4.0),
            down: Point::new(5.0, 6.0),
            color: [0.25, 0.5, 0.75],
            cover: 0.5,
            lod: 1.5,
        }
    }

    #[test]
    fn an_instance_becomes_its_centre_edges_cover_detail_and_colour_in_order() {
        let instance = SkyImageInstance {
            image: SkyImageId(9),
            placement: placement(),
        };
        let data = SkyImageData::from(&instance);
        let floats: &[f32] = bytemuck::cast_slice(std::slice::from_ref(&data));
        assert_eq!(
            floats,
            [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 0.5, 1.5, 0.25, 0.5, 0.75, 0.0]
        );
    }

    #[test]
    fn an_image_leaves_what_its_cover_spares_plus_its_light_and_the_alpha_beneath() {
        assert_eq!(OVER.color.src_factor, wgpu::BlendFactor::One);
        assert_eq!(OVER.color.dst_factor, wgpu::BlendFactor::OneMinusSrcAlpha);
        assert_eq!(OVER.color.operation, wgpu::BlendOperation::Add);
        assert_eq!(OVER.alpha.src_factor, wgpu::BlendFactor::Zero);
        assert_eq!(OVER.alpha.dst_factor, wgpu::BlendFactor::One);
    }

    /// A 2 by 2 image: red, green, blue, and half-clear white.
    fn four() -> SkyImage {
        let rgba = [
            [255, 0, 0, 255],
            [0, 255, 0, 255],
            [0, 0, 255, 255],
            [255, 255, 255, 128],
        ]
        .concat();
        SkyImage::new(2, 2, rgba).expect("2 by 2")
    }

    /// A 32 by 32 frame of logical 32 by 32, a unit a pixel, of `batches`.
    fn frame(batches: Vec<Batch>) -> Frame {
        Frame {
            target: (32, 32),
            viewport: PixelRect {
                x: 0,
                y: 0,
                w: 32,
                h: 32,
            },
            logical: LogicalSize { w: 32, h: 32 },
            clear: nova_view::Color::BLACK,
            world_space: WorldSpace::Target,
            batches,
            world: vec![],
            world_clip: None,
            world_tail: None,
        }
    }

    /// Pixel (`x`, `y`) of a 32-wide read-back.
    fn at(pixels: &[u8], x: usize, y: usize) -> [i32; 3] {
        let i = (y * 32 + x) * 4;
        [0, 1, 2].map(|c| i32::from(pixels[i + c]))
    }

    /// The image [`four`] 16 units across, centred in the view, its four
    /// texels eight pixels each.
    fn centred(cover: f32) -> SkyImageInstance {
        SkyImageInstance {
            image: SkyImageId(3),
            placement: Placement {
                centre: Point::new(16.0, 16.0),
                right: Point::new(8.0, 0.0),
                down: Point::new(0.0, 8.0),
                color: [1.0, 0.5, 0.25],
                cover,
                lod: -3.0,
            },
        }
    }

    #[track_caller]
    fn assert_near(seen: [i32; 3], want: [f32; 3], what: &str) {
        for (seen_c, want_c) in seen.iter().zip(want) {
            let want_c = (want_c.min(1.0) * 255.0).round() as i32;
            assert!(
                (seen_c - want_c).abs() <= 2,
                "{what}: {seen:?} for {want:?}"
            );
        }
    }

    #[test]
    fn an_image_uploaded_and_drawn_shows_the_cores_pixels_over_what_is_beneath() {
        let Ok(mut gpu) = OffscreenGpu::new(32, 32, &FontFaces::bundled()) else {
            return;
        };
        let image = four();
        gpu.upload_sky_image(SkyImageId(3), &image);
        // A faint puff over the whole target, then the image over it.
        let puff = StarInstance {
            centre: Point::new(16.0, 16.0),
            radius: 64.0,
            color: [0.2, 0.4, 0.6],
            trail: Point::new(0.0, 0.0),
        };
        for cover in [1.0, 0.0] {
            let instance = centred(cover);
            gpu.submit(&frame(vec![
                Batch::Sky(vec![puff]),
                Batch::SkyImages(vec![instance]),
            ]));
            let pixels = gpu.read_pixels().expect("read back");
            // A pixel well inside each texel, and one outside the quad.
            for (x, y) in [(11, 11), (20, 11), (11, 20), (20, 20), (3, 3), (28, 16)] {
                let point = Point::new(x as f32 + 0.5, y as f32 + 0.5);
                let offset = Point::new(point.x - 16.0, point.y - 16.0);
                let beneath = crate::sky::streak_coverage(offset, puff.trail, puff.radius);
                let beneath = puff.color.map(|c| c * beneath);
                let want = match lit(&image, &instance.placement, point) {
                    Some(light) => over(beneath, light),
                    None => beneath,
                };
                assert_near(
                    at(&pixels, x, y),
                    want,
                    &format!("cover {cover} at ({x}, {y})"),
                );
            }
            // The red texel is red, at the instance's colour.
            let red = at(&pixels, 11, 11);
            assert!(red[0] > 200 && red[2] < 160, "{red:?}");
        }
    }

    #[test]
    fn an_image_not_uploaded_draws_nothing_and_another_frame_makes_no_sampler_or_group() {
        let Ok(mut gpu) = OffscreenGpu::new(32, 32, &FontFaces::bundled()) else {
            return;
        };
        let shown = frame(vec![Batch::SkyImages(vec![centred(1.0)])]);
        // Not uploaded: skipped.
        gpu.submit(&shown);
        let pixels = gpu.read_pixels().expect("read back");
        assert_eq!(at(&pixels, 11, 11), [0, 0, 0]);
        gpu.upload_sky_image(SkyImageId(3), &four());
        gpu.submit(&shown);
        let pixels = gpu.read_pixels().expect("read back");
        assert!(at(&pixels, 11, 11)[0] > 200);
        // Everything an image is drawn with was made at its upload or
        // before: a later frame makes no sampler, texture or pipeline, and
        // no bind group but the frame's own.
        let before = gpu.resources_held();
        gpu.submit(&shown);
        gpu.submit(&shown);
        assert_eq!(gpu.resources_held(), before);
    }

    #[test]
    fn a_smaller_image_reads_its_mip_at_the_cores_level() {
        let Ok(mut gpu) = OffscreenGpu::new(32, 32, &FontFaces::bundled()) else {
            return;
        };
        // 8 by 8, each 4 by 4 block one shade: its level 2 holds the four
        // shades, one texel each.
        let shades = [40_u8, 120, 200, 80];
        let mut rgba = Vec::new();
        for row in 0..8 {
            for column in 0..8 {
                let shade = shades[(row / 4) * 2 + column / 4];
                rgba.extend([shade, shade, shade, 255]);
            }
        }
        let image = SkyImage::new(8, 8, rgba).expect("8 by 8");
        gpu.upload_sky_image(SkyImageId(0), &image);
        // Drawn 2 pixels across at level 2: each pixel is one texel of it.
        let instance = SkyImageInstance {
            image: SkyImageId(0),
            placement: Placement {
                centre: Point::new(16.0, 16.0),
                right: Point::new(1.0, 0.0),
                down: Point::new(0.0, 1.0),
                color: [1.0; 3],
                cover: 1.0,
                lod: 2.0,
            },
        };
        gpu.submit(&frame(vec![Batch::SkyImages(vec![instance])]));
        let pixels = gpu.read_pixels().expect("read back");
        for ((x, y), shade) in [(15, 15), (16, 15), (15, 16), (16, 16)]
            .into_iter()
            .zip(shades)
        {
            let point = Point::new(x as f32 + 0.5, y as f32 + 0.5);
            let want = lit(&image, &instance.placement, point).expect("inside");
            assert!((want[0] * 255.0 - f32::from(shade)).abs() < 0.5);
            assert_near(at(&pixels, x, y), [want[0], want[1], want[2]], "level 2");
        }
    }
}
