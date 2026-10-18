//! A small flight frame (a world and a HUD over it) and what the
//! upscaler tests (`sharp_bilinear`, `metal_fx`) draw and compare it with.
//!
//! Needs a GPU adapter; each drawing helper answers `None` without one.

use nova_data::graphics::Image;
use nova_render::fonts::FontFaces;
use nova_render::{Gpu, ImageError, ImageSource, LogicalSize, Renderer, Upscaler, Viewport};
use nova_view::draw::{fill_rect, lights_tint};
use nova_view::geometry::Bounds;
use nova_view::{Color, DrawList, ImageKey, ImageKind, Point};

use super::wgpu_offscreen::{Images, gpu_with};

/// Logical 32x24, as the other offscreen tests draw.
pub(crate) const LOGICAL: LogicalSize = LogicalSize { w: 32, h: 24 };

/// How far a channel of the frame may be from the reference. The GPU's
/// bilinear filter weighs texels in fixed point (at least 8 bits), so a
/// seam pixel can be a level off the float reference; rounding the
/// enlargement's and the overlay blend's 8-bit outputs (and the
/// reference's recovery formula) adds about a level each. Pixels inside a
/// texel sample its centre and match exactly.
pub(crate) const TOLERANCE: u8 = 3;

/// `rlëD` 20: a 4x4 sprite whose middle 2x2 is opaque light blue and
/// whose edge is the same at half alpha, so its edges blend.
pub(crate) const SOFT: i16 = 20;

/// [`Images`], with the soft-edged sprite.
pub(crate) struct Sprites;

impl ImageSource for Sprites {
    fn frames(&self, kind: ImageKind, id: i16) -> Result<Vec<Image>, ImageError> {
        if (kind, id) == (ImageKind::Rled, SOFT) {
            let pixels = (0..4)
                .flat_map(|y| (0..4).map(move |x| (x, y)))
                .flat_map(|(x, y)| {
                    let inside = (1..3).contains(&x) && (1..3).contains(&y);
                    [90, 160, 230, if inside { 255 } else { 128 }]
                })
                .collect();
            return Ok(vec![Image::from_rgba(4, 4, pixels).expect("4x4")]);
        }
        Images.frames(kind, id)
    }
}

pub(crate) fn at(x: f32, y: f32) -> Point {
    Point::new(x, y)
}

/// The world: a patterned picture, a soft-edged sprite, an additive
/// sprite, lights drawn with the OR blend over the soft sprite, a line
/// and a dot.
pub(crate) fn world(list: &mut DrawList) {
    list.picture(ImageKey::picture(1), at(2.0, 2.0))
        .sprite(ImageKey::sprite(SOFT, 0), at(10.0, 6.0), Color::WHITE)
        .additive_sprite(ImageKey::sprite(5, 0), at(16.0, 6.0), Color::WHITE)
        .or_sprite(ImageKey::sprite(7, 0), at(10.0, 6.0), lights_tint(20))
        .line(
            at(2.0, 14.0),
            at(14.0, 14.0),
            2.0,
            Color::rgba(0, 0, 255, 255),
        )
        .dot(at(20.0, 14.0), 2.0, Color::rgba(255, 255, 0, 255));
}

/// The HUD, normal blending alone: an opaque status-bar picture, a soft
/// sprite over the world's picture, a translucent line over the world's
/// line, and text.
pub(crate) fn hud(list: &mut DrawList) {
    list.picture(ImageKey::picture(10), at(26.0, 18.0))
        .sprite(ImageKey::sprite(SOFT, 0), at(4.0, 3.0), Color::WHITE)
        .line(
            at(6.0, 15.0),
            at(18.0, 15.0),
            1.0,
            Color::rgba(255, 0, 0, 128),
        )
        .text("B", at(16.0, 16.0), 6.0, None, Color::WHITE);
}

/// The world, marked, then the HUD.
pub(crate) fn flight() -> DrawList {
    let mut list = DrawList::new();
    list.world(world);
    hud(&mut list);
    list
}

/// `draw`'s commands alone, unmarked.
pub(crate) fn alone(draw: fn(&mut DrawList)) -> DrawList {
    let mut list = DrawList::new();
    draw(&mut list);
    list
}

/// `draw`'s commands alone, marked as the world.
pub(crate) fn as_world(draw: fn(&mut DrawList)) -> DrawList {
    let mut list = DrawList::new();
    list.world(draw);
    list
}

/// The HUD over a white fill of the whole logical space.
pub(crate) fn hud_over_white(logical: LogicalSize) -> DrawList {
    let mut list = DrawList::new();
    let all = Bounds {
        min: at(0.0, 0.0),
        max: at(logical.w as f32, logical.h as f32),
    };
    fill_rect(&mut list, all, Color::WHITE);
    hud(&mut list);
    list
}

/// `list` drawn into `viewport` by a fresh offscreen GPU drawing the world
/// with `upscaler`, read back; `None` with no adapter.
pub(crate) fn drawn(list: &DrawList, viewport: &Viewport, upscaler: Upscaler) -> Option<Vec<u8>> {
    let (w, h) = viewport.window_px();
    let mut gpu = gpu_with(w, h, &FontFaces::bundled())?;
    gpu.set_upscaler(upscaler);
    let report = Renderer::new(Sprites).render(list, viewport, &mut gpu);
    assert_eq!(report.new_failures, vec![]);
    Some(gpu.read_pixels().expect("read back"))
}

/// `logical` fitted into a `window` window at a display scale of 1.
pub(crate) fn fitted(logical: LogicalSize, window: (u32, u32)) -> Viewport {
    Viewport::new(logical, window, 1.0)
}

/// The HUD drawn over `under`: `B0 + (B1 - B0) x under / 255` in each
/// colour channel, from the HUD over black (`b0`) and over white (`b1`).
pub(crate) fn over(b0: &[u8], b1: &[u8], under: &[u8]) -> Vec<u8> {
    b0.chunks(4)
        .zip(b1.chunks(4))
        .zip(under.chunks(4))
        .flat_map(|((b0, b1), under)| {
            let mut pixel = [0, 0, 0, 255];
            for c in 0..3 {
                let (lo, hi) = (f32::from(b0[c]), f32::from(b1[c]));
                let mixed = lo + (hi - lo) * f32::from(under[c]) / 255.0;
                pixel[c] = mixed.round().clamp(0.0, 255.0) as u8;
            }
            pixel
        })
        .collect()
}

/// The largest difference of any colour channel between `a` and `b`, and
/// the pixel it is at.
pub(crate) fn largest_difference(a: &[u8], b: &[u8], width: u32) -> (u8, (u32, u32)) {
    a.chunks(4)
        .zip(b.chunks(4))
        .enumerate()
        .map(|(at, (a, b))| {
            let most = (0..3).map(|c| a[c].abs_diff(b[c])).max().unwrap_or(0);
            (most, (at as u32 % width, at as u32 / width))
        })
        .max_by_key(|&(most, _)| most)
        .unwrap_or((0, (0, 0)))
}
