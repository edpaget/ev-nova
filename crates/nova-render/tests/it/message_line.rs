//! Flight's message line under a frame upscaler, drawn through the wgpu
//! adapter: its box goes into the world texture under the sprites and its
//! text draws over them, as `_DisplayComm` (0x90201) erases the box into
//! the background frame and `_FloaterDrawCallback` (@0x38b52-0x38b90)
//! blits that rect over the sprites. The scene is built by hand from
//! `DrawList` primitives in flight's order; that flight draws them so is
//! nova-view's, pinned there. Needs a GPU adapter; skips, passing, without
//! one.

use nova_data::fonts::fixture::block_font;
use nova_render::fonts::FontFaces;
use nova_render::{Gpu, Renderer, Upscaler, Viewport};
use nova_view::geometry::Bounds;
use nova_view::{Color, DrawList, Font, Point};

use super::common::fill;
use super::flight_frame::{LOGICAL, fitted};
use super::wgpu_offscreen::{Images, gpu_with};

/// The window: twice the logical 32x24, so the upscaled world fills it
/// exactly.
const WINDOW: (u32, u32) = (64, 48);

/// The 5-bit colour `rgb5` widened to 8 bits, as the original's 16-bit
/// screen shows it.
fn w555(rgb5: [u8; 3]) -> [u8; 4] {
    let widen = |c: u8| (c << 3) | (c >> 2);
    [widen(rgb5[0]), widen(rgb5[1]), widen(rgb5[2]), 255]
}

fn rgba([r, g, b, a]: [u8; 4]) -> Color {
    Color::rgba(r, g, b, a)
}

fn area(left: f32, top: f32, right: f32, bottom: f32) -> Bounds {
    Bounds {
        min: Point::new(left, top),
        max: Point::new(right, bottom),
    }
}

const BLACK: [u8; 4] = [0, 0, 0, 255];
const WHITE: [u8; 4] = [255, 255, 255, 255];

/// The starfield's colour: a dark blue.
const STARS: [u8; 3] = [0, 0, 8];
/// The message box's colour, the system's background: a dark grey.
const BOX: [u8; 3] = [4, 4, 4];
/// A ship's colour: red.
const SHIP: [u8; 3] = [31, 0, 0];

/// The message box, in logical units.
fn message_box() -> Bounds {
    area(2.0, 14.0, 30.0, 22.0)
}

/// A ship over the box's left part, in logical units.
fn ship() -> Bounds {
    area(4.0, 10.0, 14.0, 20.0)
}

/// The message's text, in the block font, over the box and the ship.
fn text(list: &mut DrawList) {
    list.text_in(
        Font::Charcoal,
        "MMMMMM",
        Point::new(3.0, 13.0),
        8.0,
        None,
        Color::WHITE,
    );
}

/// Flight's message line among its world, in flight's order: the stars,
/// the box over them, a ship over the box, the box's colour OR-ed over the
/// ship (the floaters' blit), then the text, which stays in the overlay.
fn message_scene() -> DrawList {
    let mut list = DrawList::new();
    list.world_in(area(0.0, 0.0, 32.0, 24.0), |list| {
        fill(list, area(0.0, 0.0, 32.0, 24.0), rgba(w555(STARS)));
        fill(list, message_box(), rgba(w555(BOX)));
        fill(list, ship(), rgba(w555(SHIP)));
        list.or_fill(message_box(), rgba(w555(BOX)));
        text(list);
    });
    list
}

/// The text alone, unmarked, over the black clear: where its glyphs cover.
fn text_alone() -> DrawList {
    let mut list = DrawList::new();
    text(&mut list);
    list
}

/// The faces with the block font as Charcoal, so a glyph's inside is solid.
fn faces() -> FontFaces {
    FontFaces::bundled().with_charcoal(block_font("Charcoal"))
}

/// `list` drawn into `viewport` with `upscaler`, read back; `None` with no
/// adapter.
fn drawn(list: &DrawList, viewport: &Viewport, upscaler: Upscaler) -> Option<Vec<u8>> {
    let (w, h) = viewport.window_px();
    let mut gpu = gpu_with(w, h, &faces())?;
    gpu.set_upscaler(upscaler);
    let report = Renderer::new(Images).render(list, viewport, &mut gpu);
    assert_eq!(report.new_failures, vec![]);
    Some(gpu.read_pixels().expect("read back"))
}

fn pixel(pixels: &[u8], (x, y): (u32, u32)) -> [u8; 4] {
    let at = ((y * WINDOW.0 + x) * 4) as usize;
    pixels[at..at + 4].try_into().expect("four bytes")
}

/// The window pixels inside `area`, at two pixels a logical unit.
fn pixels_in(area: Bounds) -> impl Iterator<Item = (u32, u32)> {
    let (left, right) = (area.min.x as u32 * 2, area.max.x as u32 * 2);
    let (top, bottom) = (area.min.y as u32 * 2, area.max.y as u32 * 2);
    (top..bottom).flat_map(move |y| (left..right).map(move |x| (x, y)))
}

fn inside(area: Bounds, (x, y): (u32, u32)) -> bool {
    let (x, y) = (x as f32 / 2.0, y as f32 / 2.0);
    (area.min.x..area.max.x).contains(&x) && (area.min.y..area.max.y).contains(&y)
}

#[test]
fn under_sharp_bilinear_the_message_box_is_under_the_ship_and_its_text_over_it() {
    let viewport = fitted(LOGICAL, WINDOW);
    let Some(frame) = drawn(&message_scene(), &viewport, Upscaler::SharpBilinear) else {
        return;
    };
    let Some(glyphs) = drawn(&text_alone(), &viewport, Upscaler::Native) else {
        return;
    };
    let (mut glyph_over_ship, mut box_over_ship, mut box_alone) = (0, 0, 0);
    for at in pixels_in(message_box()) {
        let got = pixel(&frame, at);
        match (pixel(&glyphs, at), inside(ship(), at)) {
            // A glyph over the ship: the text over the sprite.
            (WHITE, true) => {
                assert_eq!(got, WHITE, "a glyph over the ship at {at:?}");
                glyph_over_ship += 1;
            }
            // The ship over the box, the box's colour OR-ed over it: the
            // box under the sprite, its blit over it.
            (BLACK, true) => {
                let ored = w555([SHIP[0] | BOX[0], SHIP[1] | BOX[1], SHIP[2] | BOX[2]]);
                assert_eq!(got, ored, "the ship over the box at {at:?}");
                box_over_ship += 1;
            }
            // The box alone, over the stars.
            (BLACK, false) => {
                assert_eq!(got, w555(BOX), "the box at {at:?}");
                box_alone += 1;
            }
            // A glyph's edge or a glyph over the box alone.
            _ => {}
        }
    }
    assert!(glyph_over_ship > 0, "no glyph lies over the ship");
    assert!(box_over_ship > 0, "the ship covers no bare box");
    assert!(box_alone > 0, "no bare box beside the ship");
}
