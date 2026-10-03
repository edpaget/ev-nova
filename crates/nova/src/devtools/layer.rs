//! Painting egui's output over a frame with egui-wgpu.

use egui::Context;
use egui::epaint::{ClippedPrimitive, ClippedShape, textures::TexturesDelta};
use egui_wgpu::{Renderer, RendererOptions, ScreenDescriptor};
use nova_render::wgpu::{OverlayPainter, PaintTarget};

/// egui's output for the next frame, painted over it with egui-wgpu.
///
/// Texture changes from every prepared frame are kept until a frame is
/// painted, so a frame the window drops loses no texture upload.
pub struct EguiLayer {
    /// The renderer, created for the first target's format.
    renderer: Option<(wgpu::TextureFormat, Renderer)>,
    primitives: Vec<ClippedPrimitive>,
    pixels_per_point: f32,
    pending: TexturesDelta,
}

impl Default for EguiLayer {
    fn default() -> Self {
        Self {
            renderer: None,
            primitives: Vec::new(),
            pixels_per_point: 1.0,
            pending: TexturesDelta::default(),
        }
    }
}

impl EguiLayer {
    /// Takes one egui frame's output: tessellates its `shapes` (replacing
    /// the last frame's) and adds its texture changes to those pending.
    pub fn prepare(
        &mut self,
        ctx: &Context,
        shapes: Vec<ClippedShape>,
        textures_delta: TexturesDelta,
        pixels_per_point: f32,
    ) {
        self.primitives = ctx.tessellate(shapes, pixels_per_point);
        self.pixels_per_point = pixels_per_point;
        self.pending.append(textures_delta);
    }
}

impl OverlayPainter for EguiLayer {
    fn paint(&mut self, target: &PaintTarget<'_>) {
        if self
            .renderer
            .as_ref()
            .is_none_or(|(format, _)| *format != target.format)
        {
            let options = RendererOptions {
                dithering: false,
                ..RendererOptions::default()
            };
            let renderer = Renderer::new(target.device, target.format, options);
            self.renderer = Some((target.format, renderer));
        }
        let (_, renderer) = self.renderer.as_mut().expect("just created");
        for (id, deltas) in &self.pending.set {
            for delta in deltas {
                renderer.update_texture(target.device, target.queue, *id, delta);
            }
        }
        let screen = ScreenDescriptor {
            size_in_pixels: [target.size_px.0, target.size_px.1],
            pixels_per_point: self.pixels_per_point,
        };
        let mut encoder = target
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("nova developer tools"),
            });
        let mut buffers = renderer.update_buffers(
            target.device,
            target.queue,
            &mut encoder,
            &self.primitives,
            &screen,
        );
        {
            let mut pass = encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("nova developer tools"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: target.view,
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
                })
                .forget_lifetime();
            renderer.render(&mut pass, &self.primitives, &screen);
        }
        buffers.push(encoder.finish());
        target.queue.submit(buffers);
        for id in &self.pending.free {
            renderer.free_texture(id);
        }
        self.pending.clear();
    }
}

impl Drop for EguiLayer {
    /// Unapplied texture changes would otherwise panic as they drop.
    fn drop(&mut self) {
        self.pending.clear();
    }
}

#[cfg(test)]
mod tests {
    use egui::{ColorImage, Context, RawInput, TextureOptions};

    use super::*;

    fn input() -> RawInput {
        RawInput {
            max_texture_side: Some(8192),
            ..RawInput::default()
        }
    }

    fn red() -> ColorImage {
        ColorImage::from_rgba_unmultiplied([2, 2], &[255, 0, 0, 255].repeat(4))
    }

    #[test]
    fn deltas_of_frames_never_painted_stay_pending() {
        let ctx = Context::default();
        let mut layer = EguiLayer::default();
        let mut first = None;
        let output = ctx.run_ui(input(), |ui| {
            first = Some(
                ui.ctx()
                    .load_texture("first", red(), TextureOptions::NEAREST),
            );
        });
        let first = first.expect("loaded");
        assert!(output.textures_delta.set.contains_key(&first.id()));
        layer.prepare(&ctx, output.shapes, output.textures_delta, 1.0);

        let mut second = None;
        let output = ctx.run_ui(input(), |ui| {
            second = Some(
                ui.ctx()
                    .load_texture("second", red(), TextureOptions::NEAREST),
            );
        });
        let second = second.expect("loaded");
        layer.prepare(&ctx, output.shapes, output.textures_delta, 2.0);

        assert!(layer.pending.set.contains_key(&first.id()), "kept");
        assert!(layer.pending.set.contains_key(&second.id()), "added");
        assert!((layer.pixels_per_point - 2.0).abs() < f32::EPSILON);
        drop(layer);
    }

    #[test]
    fn a_frames_shapes_replace_the_last() {
        let ctx = Context::default();
        let mut layer = EguiLayer::default();
        let output = ctx.run_ui(input(), |ui| {
            ui.label("one");
            ui.label("two");
        });
        layer.prepare(&ctx, output.shapes, output.textures_delta, 1.0);
        assert!(!layer.primitives.is_empty());
        let output = ctx.run_ui(input(), |_| {});
        layer.prepare(&ctx, output.shapes, output.textures_delta, 1.0);
        assert!(layer.primitives.is_empty(), "nothing drawn this frame");
    }
}
