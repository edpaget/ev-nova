//! The wgpu side of [`Batch::Sky`](crate::Batch::Sky): the sky pipeline
//! and its draws. It decides nothing: each star's centre, radius and
//! colour come from the core ([`crate::sky`]), and `sky.wgsl` mirrors the
//! core's [`coverage`](crate::sky::coverage) per fragment.
//!
//! The pipeline draws one square per star into [`SCENE_FORMAT`] with no
//! depth attachment, adding its colour to what is beneath (one times the
//! source plus one times the destination) and leaving the destination's
//! alpha as it is.

use std::ops::Range;

use bytemuck::{Pod, Zeroable};

use super::renderer::{SCENE_FORMAT, pipeline_layout};
use crate::sky::StarInstance;

/// One star as the sky shader reads it, per instance.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub(super) struct StarData {
    /// Its centre in logical units, its radius, then 0.
    pub place: [f32; 4],
    /// Its colour, then 0.
    pub color: [f32; 4],
}

impl From<&StarInstance> for StarData {
    fn from(star: &StarInstance) -> Self {
        let [r, g, b] = star.color;
        Self {
            place: [star.centre.x, star.centre.y, star.radius, 0.0],
            color: [r, g, b, 0.0],
        }
    }
}

/// The sky pipeline.
pub(super) struct Sky {
    pipeline: wgpu::RenderPipeline,
}

impl Sky {
    /// The pipeline, built on `device` with the projection's `globals`
    /// layout as group 0.
    pub(super) fn new(device: &wgpu::Device, globals: &wgpu::BindGroupLayout) -> Self {
        Self {
            pipeline: pipeline(device, globals),
        }
    }

    /// Encodes the stars `instances` of `buffer` into `pass`, the
    /// projection's `globals` bound, one square each.
    pub(super) fn encode(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        globals: &wgpu::BindGroup,
        buffer: &wgpu::Buffer,
        instances: Range<u32>,
    ) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, globals, &[]);
        pass.set_vertex_buffer(0, buffer.slice(..));
        pass.draw(0..6, instances);
    }
}

/// How a star combines with what is beneath it: its colour added, the
/// destination's alpha kept.
const ADD: wgpu::BlendState = wgpu::BlendState {
    color: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::One,
        dst_factor: wgpu::BlendFactor::One,
        operation: wgpu::BlendOperation::Add,
    },
    alpha: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::Zero,
        dst_factor: wgpu::BlendFactor::One,
        operation: wgpu::BlendOperation::Add,
    },
};

/// The sky pipeline: [`StarData`] per instance in slot 0.
fn pipeline(device: &wgpu::Device, globals: &wgpu::BindGroupLayout) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::include_wgsl!("sky.wgsl"));
    let layout = pipeline_layout(device, "nova sky", &[globals]);
    let instance = wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32x4];
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("nova sky"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("star_vs"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[Some(wgpu::VertexBufferLayout {
                array_stride: size_of::<StarData>() as u64,
                step_mode: wgpu::VertexStepMode::Instance,
                attributes: &instance,
            })],
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("star_fs"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: SCENE_FORMAT,
                blend: Some(ADD),
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

    #[test]
    fn a_star_becomes_its_centre_radius_and_colour_in_order() {
        let star = StarInstance {
            centre: Point::new(1.0, 2.0),
            radius: 3.0,
            color: [0.25, 0.5, 0.75],
            trail: Point::new(0.0, 0.0),
        };
        let data = StarData::from(&star);
        let floats: &[f32] = bytemuck::cast_slice(std::slice::from_ref(&data));
        assert_eq!(floats, [1.0, 2.0, 3.0, 0.0, 0.25, 0.5, 0.75, 0.0]);
    }

    #[test]
    fn a_star_drawn_lights_its_centre_by_its_colour_and_nothing_far_from_it() {
        use crate::fonts::FontFaces;
        use crate::gpu::{Batch, Frame, Gpu, WorldSpace};
        use crate::viewport::{LogicalSize, PixelRect};
        // Logical 32 x 32 in the 32 x 32 target: a unit a pixel.
        let Ok(mut gpu) = crate::wgpu::OffscreenGpu::new(32, 32, &FontFaces::bundled()) else {
            return;
        };
        let star = StarInstance {
            centre: Point::new(16.5, 16.5),
            radius: 4.0,
            color: [0.8, 0.4, 0.2],
            trail: Point::new(0.0, 0.0),
        };
        gpu.submit(&Frame {
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
            batches: vec![Batch::Sky(vec![star])],
            world: vec![],
            world_clip: None,
            world_tail: None,
        });
        let pixels = gpu.read_pixels().expect("read back");
        let at = |x: usize, y: usize| &pixels[(y * 32 + x) * 4..(y * 32 + x) * 4 + 3];
        // Pixel (16, 16)'s centre is the star's: whole coverage.
        let centre: Vec<i32> = at(16, 16).iter().map(|&c| i32::from(c)).collect();
        for (seen, want) in centre.iter().zip([204, 102, 51]) {
            assert!((seen - want).abs() <= 2, "{centre:?}");
        }
        assert_eq!(at(2, 2), [0, 0, 0]);
    }

    #[test]
    fn a_star_adds_its_colour_and_keeps_the_alpha_beneath() {
        assert_eq!(ADD.color.src_factor, wgpu::BlendFactor::One);
        assert_eq!(ADD.color.dst_factor, wgpu::BlendFactor::One);
        assert_eq!(ADD.color.operation, wgpu::BlendOperation::Add);
        assert_eq!(ADD.alpha.src_factor, wgpu::BlendFactor::Zero);
        assert_eq!(ADD.alpha.dst_factor, wgpu::BlendFactor::One);
    }
}
