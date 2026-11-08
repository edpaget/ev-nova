//! The FSR 1 upscaler: AMD's `FidelityFX` Super Resolution 1, the world
//! texture enlarged onto the viewport by EASU (edge-adaptive spatial
//! upsampling) into a texture the viewport's size, then sharpened onto
//! the viewport by RCAS (robust contrast-adaptive sharpening).
//!
//! The shader (`fsr1.wgsl`) is a WGSL port of AMD's `ffx_fsr1.h`,
//! `FidelityFX` FSR 1.0, which is under the MIT licence:
//!
//! ```text
//! Copyright (c) 2021 Advanced Micro Devices, Inc. All rights reserved.
//! Permission is hereby granted, free of charge, to any person obtaining a copy
//! of this software and associated documentation files(the "Software"), to deal
//! in the Software without restriction, including without limitation the rights
//! to use, copy, modify, merge, publish, distribute, sublicense, and / or sell
//! copies of the Software, and to permit persons to whom the Software is
//! furnished to do so, subject to the following conditions :
//! The above copyright notice and this permission notice shall be included in
//! all copies or substantial portions of the Software.
//! THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
//! IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
//! FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.IN NO EVENT SHALL THE
//! AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
//! LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
//! OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
//! THE SOFTWARE.
//! ```
//!
//! The shader's header carries the same notice, so it ships in the binary
//! with the shader's text, and says what was changed from AMD's source.
//!
//! Both passes go in the frame's own encoder, as sharp-bilinear's does.
//! EASU scales each axis by its own ratio, so a widened or letterboxed
//! world goes straight to its content rectangle. RCAS's sharpness is
//! AMD's default, 0.2 stops below the most, and is not a setting.

use bytemuck::Zeroable;
use wgpu::util::DeviceExt;

use super::frame_commands::WgpuCommands;
use super::renderer::{SCENE_FORMAT, pipeline, pipeline_layout};
use super::upscale::Upscale;
use crate::upscale::{Upscaler, WorldResolution, resolution_of};
use crate::viewport::PixelRect;

/// RCAS's sharpness, in stops below the most: AMD's default.
const SHARPNESS_STOPS: f32 = 0.2;

/// The FSR 1 upscaler: the world is drawn at the logical resolution, then
/// enlarged by EASU into a texture the viewport's size and sharpened by
/// RCAS onto the viewport, two fragment passes reading texels with
/// `textureLoad`, clamped at the edges.
pub struct Fsr1 {
    queue: wgpu::Queue,
    device: wgpu::Device,
    easu: wgpu::RenderPipeline,
    rcas: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    params: wgpu::Buffer,
    /// The world texture EASU's bind group reads, and the group: remade
    /// when the renderer remakes the world texture.
    world: Option<(wgpu::TextureView, wgpu::BindGroup)>,
    /// EASU's output, which RCAS reads: remade when the viewport's size
    /// changes.
    upsampled: Option<Upsampled>,
}

/// EASU's output texture, its size and the RCAS bind group reading it.
struct Upsampled {
    size: (u32, u32),
    view: wgpu::TextureView,
    group: wgpu::BindGroup,
}

impl Fsr1 {
    /// The upscaler on `device`, its parameters written through `queue`.
    #[must_use]
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        let shader = device.create_shader_module(wgpu::include_wgsl!("fsr1.wgsl"));
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("nova fsr1"),
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
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        });
        let pipeline_layout = pipeline_layout(device, "nova fsr1", &[&layout]);
        let make = |fragment| {
            pipeline(
                device,
                &pipeline_layout,
                &shader,
                ("fsr_vs", fragment),
                &[],
                (SCENE_FORMAT, None),
            )
        };
        let (easu, rcas) = (make("easu_fs"), make("rcas_fs"));
        let params = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("nova fsr1"),
            contents: bytemuck::bytes_of(&Params::zeroed()),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        Self {
            queue: queue.clone(),
            device: device.clone(),
            easu,
            rcas,
            layout,
            params,
            world: None,
            upsampled: None,
        }
    }

    /// A bind group reading `texture` with the parameters.
    fn group(&self, texture: &wgpu::TextureView) -> wgpu::BindGroup {
        self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("nova fsr1"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.params.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(texture),
                },
            ],
        })
    }

    /// EASU's bind group reading `world`, made when `world` is not the
    /// texture it read last.
    fn bind_world(&mut self, world: &wgpu::TextureView) -> wgpu::BindGroup {
        if self.world.as_ref().is_none_or(|(view, _)| view != world) {
            let group = self.group(world);
            self.world = Some((world.clone(), group));
        }
        self.world.as_ref().expect("bound above").1.clone()
    }

    /// EASU's output for a viewport of `size`, made when `size` is not the
    /// size it was made at last.
    fn upsampled(&mut self, size: (u32, u32)) -> (wgpu::TextureView, wgpu::BindGroup) {
        if self.upsampled.as_ref().is_none_or(|up| up.size != size) {
            let texture = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("nova fsr1 easu"),
                size: wgpu::Extent3d {
                    width: size.0,
                    height: size.1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: SCENE_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            let group = self.group(&view);
            self.upsampled = Some(Upsampled { size, view, group });
        }
        let up = self.upsampled.as_ref().expect("made above");
        (up.view.clone(), up.group.clone())
    }

    /// Writes the parameters for `world`'s size and `onto`, then draws
    /// EASU from `world` into a texture `onto`'s size and RCAS from that
    /// onto `onto` of `target`, loading what is there, into `encoder`. An
    /// `onto` with a zero side draws nothing.
    fn enlarge_into(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        world: &wgpu::TextureView,
        target: &wgpu::TextureView,
        onto: PixelRect,
    ) {
        let Some(size) = upsampled_size(onto) else {
            return;
        };
        let input = (world.texture().width(), world.texture().height());
        self.queue
            .write_buffer(&self.params, 0, bytemuck::bytes_of(&params(input, onto)));
        let world_group = self.bind_world(world);
        let (upsampled, upsampled_group) = self.upsampled(size);
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("nova fsr1 easu"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &upsampled,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.easu);
            pass.set_bind_group(0, &world_group, &[]);
            pass.draw(0..3, 0..1);
        }
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("nova fsr1 rcas"),
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
        pass.set_pipeline(&self.rcas);
        pass.set_bind_group(0, &upsampled_group, &[]);
        pass.draw(0..3, 0..1);
    }
}

impl Upscale for Fsr1 {
    fn resolution(&self) -> WorldResolution {
        resolution_of(Upscaler::Fsr1)
    }

    /// Draws into the frame's current encoder (`enlarge_into`).
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

/// What both passes read. EASU reads `con0`, `FsrEasuCon`'s first
/// constant: the scale from an output pixel to the input, each axis, and
/// the offset that puts an output pixel's centre on the input's texel
/// centres. `FsrEasuCon`'s other three constants place `textureGather`
/// taps, which this port reads with `textureLoad` at whole-texel offsets
/// instead, so they have no use here. RCAS reads `origin`, `onto`'s
/// top-left corner in target pixels, which it subtracts from a fragment's
/// position to find its pixel in EASU's output, and `sharpness`,
/// `FsrRcasCon`'s constant.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, bytemuck::Pod, Zeroable)]
struct Params {
    con0: [f32; 4],
    origin: [f32; 2],
    sharpness: f32,
    padding: f32,
}

/// The parameters for enlarging an `input`-sized world texture onto
/// `onto`, as `FsrEasuCon` gives them for a viewport and resource both
/// `input` and an output of `onto`'s size, and `FsrRcasCon` for
/// [`SHARPNESS_STOPS`].
fn params(input: (u32, u32), onto: PixelRect) -> Params {
    let scale = [
        input.0 as f32 / onto.w as f32,
        input.1 as f32 / onto.h as f32,
    ];
    Params {
        con0: [
            scale[0],
            scale[1],
            0.5 * scale[0] - 0.5,
            0.5 * scale[1] - 0.5,
        ],
        origin: [onto.x as f32, onto.y as f32],
        sharpness: rcas_sharpness(SHARPNESS_STOPS),
        padding: 0.0,
    }
}

/// `FsrRcasCon`'s sharpness for `stops` below the most: `2^-stops`.
fn rcas_sharpness(stops: f32) -> f32 {
    (-stops).exp2()
}

/// The size of EASU's output for `onto`: `onto`'s own, or `None` when it
/// has a zero side and there is nothing to draw.
fn upsampled_size(onto: PixelRect) -> Option<(u32, u32)> {
    (onto.w > 0 && onto.h > 0).then_some((onto.w, onto.h))
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    fn rect(x: u32, y: u32, w: u32, h: u32) -> PixelRect {
        PixelRect { x, y, w, h }
    }

    #[test]
    fn a_classic_world_at_1_875_scales_back_by_its_inverse_both_ways() {
        let classic = params((1024, 768), rect(0, 0, 1920, 1440));
        // FsrEasuCon: in / out, then 0.5 x in / out - 0.5.
        let scale = 1024.0_f32 / 1920.0;
        assert_eq!(
            classic.con0,
            [scale, scale, 0.5 * scale - 0.5, 0.5 * scale - 0.5]
        );
        assert_eq!(768.0_f32 / 1440.0, scale);
        assert_eq!(classic.origin, [0.0, 0.0]);
    }

    #[test]
    fn a_widened_world_scales_by_each_axis_own_ratio() {
        let widened = params((1365, 768), rect(0, 0, 2560, 1440));
        let (sx, sy) = (1365.0_f32 / 2560.0, 768.0_f32 / 1440.0);
        assert_eq!(widened.con0, [sx, sy, 0.5 * sx - 0.5, 0.5 * sy - 0.5]);
        assert_ne!(sx, sy);
    }

    #[test]
    fn at_1x_each_pixel_is_its_own_texel() {
        assert_eq!(
            params((64, 48), rect(0, 0, 64, 48)).con0,
            [1.0, 1.0, 0.0, 0.0]
        );
    }

    #[test]
    fn a_shrink_steps_more_than_a_texel_a_pixel() {
        assert_eq!(
            params((1024, 768), rect(0, 0, 512, 384)).con0,
            [2.0, 2.0, 0.5, 0.5]
        );
    }

    #[test]
    fn rcas_starts_at_the_viewports_corner_with_amds_default_sharpness() {
        let boxed = params((1024, 768), rect(80, 60, 1920, 1440));
        assert_eq!(boxed.origin, [80.0, 60.0]);
        assert_eq!(boxed.sharpness, (-0.2_f32).exp2());
        assert_eq!(boxed.padding, 0.0);
    }

    #[test]
    fn rcas_sharpness_halves_with_each_stop() {
        assert_eq!(rcas_sharpness(0.0), 1.0);
        assert_eq!(rcas_sharpness(1.0), 0.5);
        assert_eq!(rcas_sharpness(2.0), 0.25);
        assert!((rcas_sharpness(0.2) - 0.870_550_6).abs() < 1e-6);
    }

    #[test]
    fn easu_draws_into_a_texture_the_viewports_size() {
        assert_eq!(upsampled_size(rect(80, 60, 1920, 1440)), Some((1920, 1440)));
        assert_eq!(upsampled_size(rect(0, 0, 1, 1)), Some((1, 1)));
    }

    #[test]
    fn a_viewport_with_a_zero_side_draws_nothing() {
        assert_eq!(upsampled_size(rect(0, 0, 0, 1440)), None);
        assert_eq!(upsampled_size(rect(0, 0, 1920, 0)), None);
    }
}
