//! The sharp-bilinear upscaler: the world texture enlarged onto the
//! viewport, nearest inside each texel and a blend about one target pixel
//! wide on the seams between texels.
//!
//! The maths is `sharp-bilinear-simple`'s (in `sharp_bilinear.wgsl`,
//! whose header names its origin), applied on each axis with that axis's
//! own scale, so a widened view's world is enlarged as evenly as a classic
//! one's.

use bytemuck::Zeroable;
use wgpu::util::DeviceExt;

use super::frame_commands::WgpuCommands;
use super::renderer::{SCENE_FORMAT, pipeline, pipeline_layout};
use super::upscale::Upscale;
use crate::upscale::{Upscaler, WorldResolution, resolution_of};
use crate::viewport::PixelRect;

/// The sharp-bilinear upscaler: the world is drawn at the logical
/// resolution, then enlarged onto the viewport in one pass over the scene
/// with a linear, clamp-to-edge sampler.
pub struct SharpBilinear {
    queue: wgpu::Queue,
    device: wgpu::Device,
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    params: wgpu::Buffer,
    /// The world texture the bind group reads, and the group: remade when
    /// the renderer remakes the world texture.
    bound: Option<(wgpu::TextureView, wgpu::BindGroup)>,
}

impl SharpBilinear {
    /// The upscaler on `device`, its parameters written through `queue`.
    #[must_use]
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        let shader = device.create_shader_module(wgpu::include_wgsl!("sharp_bilinear.wgsl"));
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("nova sharp-bilinear"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let pipeline_layout = pipeline_layout(device, "nova sharp-bilinear", &[&layout]);
        let pipeline = pipeline(
            device,
            &pipeline_layout,
            &shader,
            ("sharp_vs", "sharp_fs"),
            &[],
            (SCENE_FORMAT, None),
        );
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("nova sharp-bilinear"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let params = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("nova sharp-bilinear"),
            contents: bytemuck::bytes_of(&Params::zeroed()),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        Self {
            queue: queue.clone(),
            device: device.clone(),
            pipeline,
            layout,
            sampler,
            params,
            bound: None,
        }
    }

    /// The bind group reading `world`, made when `world` is not the
    /// texture it read last.
    fn bind(&mut self, world: &wgpu::TextureView) -> &wgpu::BindGroup {
        if self.bound.as_ref().is_none_or(|(view, _)| view != world) {
            let group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("nova sharp-bilinear"),
                layout: &self.layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: self.params.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(world),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                ],
            });
            self.bound = Some((world.clone(), group));
        }
        &self.bound.as_ref().expect("bound above").1
    }
}

impl Upscale for SharpBilinear {
    fn resolution(&self) -> WorldResolution {
        resolution_of(Upscaler::SharpBilinear)
    }

    /// Draws into the frame's current encoder ([`Self::enlarge_into`]).
    fn enlarge(
        &mut self,
        commands: &mut WgpuCommands<'_>,
        world: &wgpu::TextureView,
        target: &wgpu::TextureView,
        onto: PixelRect,
        _warnings: &mut Vec<String>,
    ) {
        self.enlarge_into(commands.encoder(), world, target, onto);
    }
}

impl SharpBilinear {
    /// Writes the parameters for `world`'s size and `onto`, then draws
    /// one pass over `target` into `encoder`, loading what is there, its
    /// viewport `onto`.
    pub(super) fn enlarge_into(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        world: &wgpu::TextureView,
        target: &wgpu::TextureView,
        onto: PixelRect,
    ) {
        let size = (world.texture().width(), world.texture().height());
        self.queue
            .write_buffer(&self.params, 0, bytemuck::bytes_of(&params(size, onto)));
        let pipeline = self.pipeline.clone();
        let group = self.bind(world);
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("nova sharp-bilinear"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_viewport(
            onto.x as f32,
            onto.y as f32,
            onto.w as f32,
            onto.h as f32,
            0.0,
            1.0,
        );
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, group, &[]);
        pass.draw(0..3, 0..1);
    }
}

/// What the shader reads: the world texture's size in texels, the
/// rectangle of the target it fills (its top-left corner and size, in
/// target pixels), and each axis's scale, target pixels a texel.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, bytemuck::Pod, Zeroable)]
struct Params {
    source: [f32; 2],
    origin: [f32; 2],
    size: [f32; 2],
    scale: [f32; 2],
}

/// The parameters for enlarging a `world`-sized texture onto `onto`. A
/// scale below 1 (a window smaller than the logical space) is taken as 1:
/// sharp-bilinear is plain bilinear there, which the maths gives at 1
/// anyway, and below 1 its blend region would be negative.
fn params(world: (u32, u32), onto: PixelRect) -> Params {
    let source = [world.0 as f32, world.1 as f32];
    let size = [onto.w as f32, onto.h as f32];
    Params {
        source,
        origin: [onto.x as f32, onto.y as f32],
        size,
        scale: [
            (size[0] / source[0]).max(1.0),
            (size[1] / source[1]).max(1.0),
        ],
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    fn rect(x: u32, y: u32, w: u32, h: u32) -> PixelRect {
        PixelRect { x, y, w, h }
    }

    #[test]
    fn a_classic_world_at_1_875_is_enlarged_evenly_both_ways() {
        assert_eq!(
            params((1024, 768), rect(0, 0, 1920, 1440)),
            Params {
                source: [1024.0, 768.0],
                origin: [0.0, 0.0],
                size: [1920.0, 1440.0],
                scale: [1.875, 1.875],
            }
        );
    }

    #[test]
    fn a_widened_world_is_enlarged_by_each_axis_scale() {
        let widened = params((1365, 768), rect(0, 0, 1920, 1080));
        assert_eq!(widened.source, [1365.0, 768.0]);
        assert_eq!(widened.size, [1920.0, 1080.0]);
        assert_eq!(widened.scale, [1920.0 / 1365.0, 1.40625]);
    }

    #[test]
    fn a_letterboxed_world_starts_at_the_viewports_corner() {
        let boxed = params((1024, 768), rect(0, 60, 1920, 1440));
        assert_eq!(boxed.origin, [0.0, 60.0]);
        assert_eq!(boxed.size, [1920.0, 1440.0]);
        assert_eq!(boxed.scale, [1.875, 1.875]);
        let pillared = params((1024, 768), rect(80, 0, 2048, 1536));
        assert_eq!(pillared.origin, [80.0, 0.0]);
        assert_eq!(pillared.scale, [2.0, 2.0]);
    }

    #[test]
    fn a_shrink_is_taken_at_a_scale_of_one() {
        let shrunk = params((1024, 768), rect(0, 0, 512, 384));
        assert_eq!(shrunk.scale, [1.0, 1.0]);
        assert_eq!(shrunk.size, [512.0, 384.0]);
        let one_axis = params((1365, 768), rect(0, 0, 1000, 1536));
        assert_eq!(one_axis.scale, [1.0, 2.0]);
        assert_eq!(params((64, 48), rect(0, 0, 64, 48)).scale, [1.0, 1.0]);
    }
}
