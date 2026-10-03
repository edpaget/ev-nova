//! The developer tools' egui layer paints over a frame on a real GPU: an
//! offscreen target, read back pixel by pixel. Needs a GPU adapter; skips,
//! passing, without one.

#![cfg(feature = "dev-tools")]

use egui::{Color32, ColorImage, Context, Pos2, RawInput, Rect, TextureOptions, pos2, vec2};
use nova::devtools::EguiLayer;
use nova_render::wgpu::{InitError, OffscreenGpu, WithOverlay};
use nova_render::{Frame, Gpu, LogicalSize, PixelRect};
use nova_view::Color;

const SIZE: u32 = 64;

/// An offscreen GPU, or `None` (after a skip message) with no adapter.
fn gpu() -> Option<OffscreenGpu> {
    match OffscreenGpu::new(SIZE, SIZE) {
        Ok(gpu) => Some(gpu),
        Err(error @ (InitError::NoAdapter(_) | InitError::NoDevice(_))) => {
            eprintln!("skipping: no GPU adapter ({error})");
            None
        }
        Err(error) => panic!("offscreen GPU: {error}"),
    }
}

/// An empty frame cleared opaque black.
fn black_frame() -> Frame {
    Frame {
        target: (SIZE, SIZE),
        viewport: PixelRect {
            x: 0,
            y: 0,
            w: SIZE,
            h: SIZE,
        },
        logical: LogicalSize { w: 32, h: 32 },
        clear: Color::rgba(0, 0, 0, 255),
        batches: Vec::new(),
    }
}

/// The target at one pixel per point.
fn input() -> RawInput {
    RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(64.0, 64.0))),
        max_texture_side: Some(8192),
        ..RawInput::default()
    }
}

fn pixel(pixels: &[u8], x: u32, y: u32) -> [u8; 4] {
    let at = ((y * SIZE + x) * 4) as usize;
    pixels[at..at + 4].try_into().expect("four bytes")
}

/// The square (8, 8)–(24, 24).
fn square() -> Rect {
    Rect::from_min_max(pos2(8.0, 8.0), pos2(24.0, 24.0))
}

/// Runs one egui frame drawing with `draw` and prepares `layer` with it.
fn prepare(ctx: &Context, layer: &mut EguiLayer, draw: impl FnMut(&mut egui::Ui)) {
    let output = ctx.run_ui(input(), draw);
    layer.prepare(
        ctx,
        output.shapes,
        output.textures_delta,
        output.pixels_per_point,
    );
}

fn submit(gpu: &mut OffscreenGpu, layer: &mut EguiLayer) -> Vec<u8> {
    WithOverlay {
        gpu: &mut *gpu,
        painter: layer,
    }
    .submit(&black_frame());
    gpu.read_pixels().expect("read back")
}

#[test]
fn egui_shapes_are_painted_over_the_frame() {
    let Some(mut gpu) = gpu() else {
        return;
    };
    let ctx = Context::default();
    let mut layer = EguiLayer::default();
    prepare(&ctx, &mut layer, |ui| {
        ui.painter().rect_filled(square(), 0.0, Color32::WHITE);
    });
    let pixels = submit(&mut gpu, &mut layer);
    assert_eq!(pixel(&pixels, 16, 16), [255, 255, 255, 255]);
    assert_eq!(pixel(&pixels, 48, 48), [0, 0, 0, 255]);
}

#[test]
fn a_texture_from_a_dropped_frame_still_reaches_the_gpu() {
    let Some(mut gpu) = gpu() else {
        return;
    };
    let ctx = Context::default();
    let mut layer = EguiLayer::default();
    let red = ColorImage::from_rgba_unmultiplied([2, 2], &[255, 0, 0, 255].repeat(4));
    let mut texture = None;
    // This frame loads the texture, but the window drops it: never painted.
    prepare(&ctx, &mut layer, |ui| {
        texture = Some(
            ui.ctx()
                .load_texture("red", red.clone(), TextureOptions::NEAREST),
        );
    });
    let texture = texture.expect("loaded");
    let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
    prepare(&ctx, &mut layer, |ui| {
        ui.painter()
            .image(texture.id(), square(), uv, Color32::WHITE);
    });
    let pixels = submit(&mut gpu, &mut layer);
    assert_eq!(pixel(&pixels, 16, 16), [255, 0, 0, 255]);
    assert_eq!(pixel(&pixels, 48, 48), [0, 0, 0, 255]);
}

#[test]
fn without_the_layer_the_frame_stays_black() {
    let Some(mut gpu) = gpu() else {
        return;
    };
    gpu.submit(&black_frame());
    let pixels = gpu.read_pixels().expect("read back");
    assert!(
        pixels
            .as_chunks::<4>()
            .0
            .iter()
            .all(|p| *p == [0, 0, 0, 255]),
        "every pixel black"
    );
}
