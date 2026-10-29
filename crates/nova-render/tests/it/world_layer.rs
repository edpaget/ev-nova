//! With the native upscaler, a draw list whose world is marked
//! ([`DrawList::world`]) draws exactly the pixels the same list unmarked
//! draws, which is how every list was drawn before there were layers: at
//! 1x, 2x and a fractional scale, whether the world comes first or
//! between two overlays. The marked world is clipped
//! ([`DrawList::world_in`]) short of an overlay column, as flight's is
//! short of the status bar, and a clip no world command crosses changes no
//! pixel. A world that crosses the clip is cut at its edge, in either
//! world space. Needs a GPU adapter; skips, passing, without one.

use nova_render::fonts::FontFaces;
use nova_render::recording::RecordingGpu;
use nova_render::{LogicalSize, Renderer, Upscaler, Viewport};
use nova_view::draw::lights_tint;
use nova_view::geometry::Bounds;
use nova_view::{Color, DrawCommand, DrawLayer, DrawList, ImageKey, Point};

use super::flight_frame;
use super::wgpu_offscreen::{Images, gpu_with};

/// Logical 32x24, as the other offscreen tests draw.
const LOGICAL: LogicalSize = LogicalSize { w: 32, h: 24 };

/// The target sizes and display scales: 1x, 2x letterboxed, and 1.875x.
const SCALES: [((u32, u32), f64); 3] = [((32, 24), 1.0), ((64, 64), 2.0), ((60, 45), 1.875)];

fn at(x: f32, y: f32) -> Point {
    Point::new(x, y)
}

/// Where the marked world is clipped to: all of the logical space but a
/// column from x = 26, right of every world command and under the
/// overlay's patterned sprite, as flight's play area stops at the status
/// bar.
fn short_of_the_column() -> Bounds {
    Bounds {
        min: at(0.0, 0.0),
        max: at(26.0, 24.0),
    }
}

/// The world: a patterned picture, a normal and an additive sprite,
/// lights drawn with the OR blend over the normal sprite, a sprite for the
/// overlay's OR sprite to read, a line and a dot, a text run and a clipped
/// picture.
fn world(list: &mut DrawList) {
    list.picture(ImageKey::picture(1), at(2.0, 2.0))
        .sprite(ImageKey::sprite(2, 0), at(10.0, 6.0), Color::WHITE)
        .additive_sprite(ImageKey::sprite(5, 0), at(16.0, 6.0), Color::WHITE)
        .or_sprite(ImageKey::sprite(7, 0), at(10.0, 6.0), lights_tint(20))
        .sprite(ImageKey::sprite(8, 0), at(22.0, 6.0), Color::WHITE)
        .line(
            at(2.0, 14.0),
            at(14.0, 14.0),
            2.0,
            Color::rgba(0, 0, 255, 255),
        )
        .dot(at(20.0, 14.0), 2.0, Color::rgba(255, 255, 0, 255))
        .text("A", at(2.0, 16.0), 6.0, None, Color::WHITE)
        .clipped(
            Bounds {
                min: at(24.0, 16.0),
                max: at(26.0, 18.0),
            },
            DrawCommand::Picture {
                image: ImageKey::picture(1),
                top_left: at(24.0, 16.0),
            },
        );
}

/// The overlay: a line meeting the world's, a sprite on the world's last
/// sprite's page in the column the world is clipped short of
/// ([`short_of_the_column`]), an OR sprite over a world sprite, a picture
/// over the world's, and text.
fn overlay(list: &mut DrawList) {
    list.line(
        at(14.0, 14.0),
        at(20.0, 14.0),
        2.0,
        Color::rgba(255, 0, 0, 255),
    )
    .sprite(ImageKey::sprite(3, 0), at(28.0, 6.0), Color::WHITE)
    .or_sprite(ImageKey::sprite(7, 0), at(22.0, 6.0), Color::WHITE)
    .picture(ImageKey::picture(1), at(12.0, 5.0))
    .text("B", at(16.0, 18.0), 6.0, None, Color::WHITE);
}

/// The world then the overlay, the world marked and clipped if `marked`.
fn world_first(marked: bool) -> DrawList {
    let mut list = DrawList::new();
    if marked {
        list.world_in(short_of_the_column(), world);
    } else {
        world(&mut list);
    }
    overlay(&mut list);
    list
}

/// An overlay, the world, then the overlay again, as a dialog's map
/// over flight draws, the world marked and clipped if `marked`.
fn between_overlays(marked: bool) -> DrawList {
    let mut list = DrawList::new();
    overlay(&mut list);
    if marked {
        list.world_in(short_of_the_column(), world);
    } else {
        world(&mut list);
    }
    overlay(&mut list);
    list
}

/// `list` drawn by a fresh renderer into a fresh `size` target at
/// `scale`, read back; `None` with no adapter.
fn drawn(list: &DrawList, (size, scale): ((u32, u32), f64)) -> Option<Vec<u8>> {
    let mut gpu = gpu_with(size.0, size.1, &FontFaces::bundled())?;
    let report = Renderer::new(Images).render(list, &Viewport::new(LOGICAL, size, scale), &mut gpu);
    assert_eq!(report.new_failures, vec![]);
    Some(gpu.read_pixels().expect("read back"))
}

/// Asserts the marked and unmarked lists `scene` makes are drawn alike,
/// byte for byte, at every scale.
#[track_caller]
fn assert_drawn_alike(scene: fn(bool) -> DrawList) {
    for scale in SCALES {
        let (Some(marked), Some(unmarked)) =
            (drawn(&scene(true), scale), drawn(&scene(false), scale))
        else {
            return;
        };
        assert_eq!(marked.len(), unmarked.len());
        let differ = marked
            .chunks(4)
            .zip(unmarked.chunks(4))
            .filter(|(a, b)| a != b)
            .count();
        assert_eq!(differ, 0, "{differ} pixels differ at {scale:?}");
        assert!(
            marked.chunks(4).any(|p| p != [0, 0, 0, 255]),
            "something drawn at {scale:?}"
        );
    }
}

#[test]
fn a_native_world_drawn_first_draws_the_unlayered_frames_pixels() {
    assert_drawn_alike(world_first);
}

#[test]
fn a_native_world_between_overlays_draws_the_unlayered_frames_pixels() {
    assert_drawn_alike(between_overlays);
}

#[test]
fn the_marked_scenes_frames_have_both_layers_and_the_unmarked_only_overlay() {
    for scene in [world_first, between_overlays] {
        for (marked, has_world) in [(true, true), (false, false)] {
            let mut gpu = RecordingGpu::new();
            let viewport = Viewport::new(LOGICAL, (64, 64), 2.0);
            Renderer::new(Images).render(&scene(marked), &viewport, &mut gpu);
            let frame = gpu.submits()[0].clone();
            assert_eq!(frame.in_layer(DrawLayer::World).next().is_some(), has_world);
            assert!(frame.in_layer(DrawLayer::Overlay).next().is_some());
        }
    }
}

/// Where [`crossing`]'s world is clipped: x = 24 of the 32 x 24 logical
/// space.
const EDGE: f32 = 24.0;

/// A world that crosses x = [`EDGE`], clipped there, with no overlay: a
/// line (a beam) at y = 4, a dot (a particle) about y = 12 and a white
/// sprite about y = 20, each from 2 left of the edge to 2 right of it or
/// further.
fn crossing() -> DrawList {
    let mut list = DrawList::new();
    let clip = Bounds {
        min: at(0.0, 0.0),
        max: at(EDGE, 24.0),
    };
    list.world_in(clip, |list| {
        list.line(at(16.0, 4.0), at(30.0, 4.0), 2.0, Color::WHITE)
            .dot(at(EDGE, 12.0), 4.0, Color::WHITE)
            .sprite(ImageKey::sprite(2, 0), at(EDGE, 20.0), Color::WHITE);
    });
    list
}

/// The rows of [`crossing`]'s line, dot and sprite, in logical units.
const CROSSING_ROWS: [f32; 3] = [4.0, 12.0, 20.0];

/// The pixel at (`x`, `y`) of a `width`-wide target's `pixels`.
fn pixel_at(pixels: &[u8], width: u32, (x, y): (u32, u32)) -> [u8; 4] {
    let at = ((y * width + x) * 4) as usize;
    pixels[at..at + 4].try_into().expect("four bytes")
}

/// Asserts that `pixels`, a `size` target whose content starts at
/// `origin` and is `scale` pixels a unit, is clear from column `cut` on,
/// and drawn at each of [`crossing`]'s rows in the column just left of
/// the edge.
#[track_caller]
fn assert_cut(pixels: &[u8], size: (u32, u32), (origin, scale): ((u32, u32), u32), cut: u32) {
    const CLEAR: [u8; 4] = [0, 0, 0, 255];
    let edge = origin.0 + EDGE as u32 * scale;
    for y in 0..size.1 {
        for x in cut..size.0 {
            assert_eq!(pixel_at(pixels, size.0, (x, y)), CLEAR, "({x}, {y})");
        }
    }
    for row in CROSSING_ROWS {
        let y = origin.1 + row as u32 * scale;
        let left = pixel_at(pixels, size.0, (edge - 1, y));
        assert_ne!(
            left,
            CLEAR,
            "drawn just left of the edge at ({}, {y})",
            edge - 1
        );
    }
}

#[test]
fn a_clipped_world_is_cut_at_the_clips_edge() {
    // Drawn into the target, at 1x and at 2x letterboxed (its content
    // (0, 8)-(64, 56)): nothing from the edge's column on.
    for ((size, scale), origin) in [(SCALES[0], (0, 0)), (SCALES[1], (0, 8))] {
        let Some(pixels) = drawn(&crossing(), (size, scale)) else {
            return;
        };
        let edge = EDGE as u32 * scale as u32;
        assert_cut(&pixels, size, (origin, scale as u32), edge);
    }
    // Drawn into a texture and enlarged one-to-one: the filter may blend
    // the edge's column, but nothing right of it.
    let viewport = flight_frame::fitted(LOGICAL, (32, 24));
    let Some(pixels) = flight_frame::drawn(&crossing(), &viewport, Upscaler::SharpBilinear) else {
        return;
    };
    assert_cut(&pixels, (32, 24), ((0, 0), 1), EDGE as u32 + 1);
}
