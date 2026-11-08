//! The FSR 1 upscaler through the offscreen adapter: a flight frame's
//! world drawn at the logical size, upsampled by EASU and sharpened by
//! RCAS onto the viewport, with the overlay (the HUD and its text) drawn
//! at the target's resolution over it.
//!
//! The reference is built as `sharp_bilinear`'s is: the world alone at
//! the logical size (W), W run through FSR 1 on the CPU
//! ([`fsr1_reference`](super::fsr1_reference)) with EASU's output stored
//! as 8 bits before RCAS, as the shader's intermediate texture stores it
//! (E), and the overlay drawn natively over black (B0) and white (B1);
//! the frame is `B0 + (B1 - B0) x E / 255`. The frame matches it within
//! [`TOLERANCE`]: the two passes' 8-bit stores and the overlay's recovery
//! formula each round by up to a level, and the GPU may fuse a multiply
//! and an add the CPU rounds twice.
//!
//! Needs a GPU adapter; skips, passing, without one.

use nova_render::fonts::FontFaces;
use nova_render::{
    Gpu, LogicalSize, PixelRect, Renderer, Upscaler, UpscalerProbe, Viewport, WorldResolution,
};
use nova_view::geometry::Bounds;
use nova_view::{Color, DrawList, ViewShape};

use super::common::fill;
use super::flight_frame::{
    LOGICAL, Sprites, TOLERANCE, alone, as_world, assert_matches, at, drawn, fitted, flight, hud,
    hud_over_white, largest_difference, over, sparse_world, world,
};
use super::fsr1_reference::{Rgba, easu, easu_pixel, nearest_four, placed, rcas};
use super::wgpu_offscreen::gpu_with;

/// The world alone drawn natively at the logical size, one pixel a unit:
/// as the world texture holds it.
fn world_at_logical_size(draw: fn(&mut DrawList), logical: LogicalSize) -> Option<Vec<u8>> {
    let viewport = fitted(logical, (logical.w, logical.h));
    drawn(&alone(draw), &viewport, Upscaler::Native)
}

/// EASU's output for `w` (`size` texels) onto `onto`, before RCAS.
fn upsampled(w: &[u8], size: (u32, u32), onto: PixelRect) -> Vec<u8> {
    let world = Rgba { pixels: w, size };
    easu(&world, (onto.w, onto.h))
}

/// `w` run through FSR 1 on the CPU onto `onto` of a `target`-sized
/// frame, its bars opaque black.
fn fsr1(w: &[u8], size: (u32, u32), target: (u32, u32), onto: PixelRect) -> Vec<u8> {
    let up = upsampled(w, size, onto);
    let sharpened = rcas(&Rgba {
        pixels: &up,
        size: (onto.w, onto.h),
    });
    placed(&sharpened, target, onto)
}

/// Draws [`flight`] into `viewport` with FSR 1 and checks it against the
/// reference; that neither the native frame, sharp-bilinear's nor EASU's
/// output without RCAS passes for it; and that wherever the HUD is opaque
/// the frame is the HUD drawn natively, exactly.
fn assert_flight_matches_its_reference(viewport: &Viewport) {
    let logical = viewport.logical();
    let size = (logical.w, logical.h);
    let target = viewport.window_px();
    let (onto, _) = viewport.content().expect("an area");
    let (Some(frame), Some(w), Some(b0), Some(b1), Some(native), Some(sharp)) = (
        drawn(&flight(), viewport, Upscaler::Fsr1),
        world_at_logical_size(world, logical),
        drawn(&alone(hud), viewport, Upscaler::Native),
        drawn(&hud_over_white(logical), viewport, Upscaler::Native),
        drawn(&flight(), viewport, Upscaler::Native),
        drawn(&flight(), viewport, Upscaler::SharpBilinear),
    ) else {
        return;
    };
    let reference = over(&b0, &b1, &fsr1(&w, size, target, onto));
    assert_matches(&frame, &reference, target, onto);

    let (most, at) = largest_difference(&native, &frame, target.0);
    assert!(
        most > 8 * TOLERANCE,
        "the native frame is the frame within {most} (at {at:?}): the world was not enlarged"
    );
    let (most, at) = largest_difference(&sharp, &frame, target.0);
    assert!(
        most > TOLERANCE,
        "sharp-bilinear's frame is the frame within {most} (at {at:?})"
    );
    let easu_only = over(&b0, &b1, &placed(&upsampled(&w, size, onto), target, onto));
    let (most, at) = largest_difference(&easu_only, &frame, target.0);
    assert!(
        most > TOLERANCE,
        "EASU alone is the frame within {most} (at {at:?}): RCAS did not run"
    );

    // Where the HUD is opaque (the same over black and over white), the
    // frame is the HUD as drawn natively: never run through EASU or RCAS.
    let mut opaque = 0;
    for ((got, b0), b1) in frame.chunks(4).zip(b0.chunks(4)).zip(b1.chunks(4)) {
        if b0 == b1 {
            assert_eq!(got, b0);
            opaque += 1;
        }
    }
    assert!(opaque > 0, "some of the HUD is opaque");
}

#[test]
fn the_probe_offers_fsr1_on_every_platform_after_sharp_bilinear() {
    let Some(mut gpu) = gpu_with(8, 8, &FontFaces::bundled()) else {
        return;
    };
    // MetalFX may follow, on a device that has it (`metal_fx.rs`).
    let offered = UpscalerProbe::upscalers(&gpu);
    assert_eq!(
        offered[..3],
        [Upscaler::Native, Upscaler::SharpBilinear, Upscaler::Fsr1]
    );
    gpu.set_upscaler(Upscaler::Fsr1);
    assert_eq!(gpu.world_resolution(), WorldResolution::Logical);
}

#[test]
fn a_flight_frame_at_1_875_is_its_world_through_easu_and_rcas_with_the_hud_native_over_it() {
    assert_flight_matches_its_reference(&fitted(LOGICAL, (60, 45)));
}

#[test]
fn a_letterboxed_flight_frame_at_1_875_is_enlarged_onto_its_content_rectangle() {
    let viewport = fitted(LOGICAL, (60, 51));
    let (content, _) = viewport.content().expect("an area");
    assert_eq!(
        content,
        PixelRect {
            x: 0,
            y: 3,
            w: 60,
            h: 45
        }
    );
    assert_flight_matches_its_reference(&viewport);
}

#[test]
fn a_flight_frame_at_2x_is_its_world_through_easu_and_rcas() {
    assert_flight_matches_its_reference(&fitted(LOGICAL, (64, 48)));
}

#[test]
fn a_widened_world_is_enlarged_by_each_axis_own_scale() {
    let viewport = Viewport::for_shape(ViewShape::Widened, (1920, 1080), 1.0);
    let logical = viewport.logical();
    assert_eq!((logical.w, logical.h), (1365, 768));
    let (Some(frame), Some(w)) = (
        drawn(&as_world(sparse_world), &viewport, Upscaler::Fsr1),
        world_at_logical_size(sparse_world, logical),
    ) else {
        return;
    };
    let (onto, _) = viewport.content().expect("an area");
    let reference = fsr1(&w, (1365, 768), (1920, 1080), onto);
    assert_matches(&frame, &reference, (1920, 1080), onto);
}

/// The world: one flat colour over the whole logical space.
fn flat(list: &mut DrawList) {
    let all = Bounds {
        min: at(0.0, 0.0),
        max: at(LOGICAL.w as f32, LOGICAL.h as f32),
    };
    fill(list, all, Color::rgba(90, 160, 230, 255));
}

#[test]
fn a_flat_world_comes_out_its_own_colour_across_the_viewport() {
    let viewport = fitted(LOGICAL, (60, 51));
    let Some(frame) = drawn(&as_world(flat), &viewport, Upscaler::Fsr1) else {
        return;
    };
    let (onto, _) = viewport.content().expect("an area");
    for (at, pixel) in frame.chunks(4).enumerate() {
        let (x, y) = (at as u32 % 60, at as u32 / 60);
        let inside =
            (onto.x..onto.x + onto.w).contains(&x) && (onto.y..onto.y + onto.h).contains(&y);
        let want = if inside {
            [90, 160, 230, 255]
        } else {
            [0, 0, 0, 255]
        };
        assert_eq!(pixel, want, "({x}, {y})");
    }
}

#[test]
fn every_easu_pixel_lies_within_the_four_texels_nearest_its_sample_point() {
    // EASU's deringing clamp, checked on the CPU port the reference uses.
    let Some(w) = world_at_logical_size(world, LOGICAL) else {
        return;
    };
    let world = Rgba {
        pixels: &w,
        size: (LOGICAL.w, LOGICAL.h),
    };
    let output = (60, 45);
    let mut strict = 0;
    for y in 0..output.1 {
        for x in 0..output.0 {
            let pixel = easu_pixel(&world, output, (x, y));
            let (four, _) = nearest_four(&world, output, (x, y));
            for c in 0..3 {
                let lo = four.iter().map(|t| t[c]).fold(f32::INFINITY, f32::min);
                let hi = four.iter().map(|t| t[c]).fold(f32::NEG_INFINITY, f32::max);
                assert!(
                    (lo..=hi).contains(&pixel[c]),
                    "({x}, {y}) channel {c}: {} outside {lo}..={hi}",
                    pixel[c]
                );
                if lo < pixel[c] && pixel[c] < hi {
                    strict += 1;
                }
            }
        }
    }
    assert!(strict > 0, "some pixel blends its four");
}

/// `list` drawn into `viewport` by a fresh `target`-sized offscreen GPU
/// drawing the world with `upscaler`, read back.
fn drawn_into(
    target: (u32, u32),
    list: &DrawList,
    viewport: &Viewport,
    upscaler: Upscaler,
) -> Vec<u8> {
    let mut gpu = gpu_with(target.0, target.1, &FontFaces::bundled()).expect("an adapter");
    gpu.set_upscaler(upscaler);
    Renderer::new(Sprites).render(list, viewport, &mut gpu);
    gpu.read_pixels().expect("read back")
}

#[test]
fn one_gpu_draws_fsr1_at_another_viewport_size_then_logical_size_then_natively() {
    // The intermediate texture is remade for a new viewport size under the
    // same logical size, the world's bind group for the new world canvas
    // of another logical size, and a native frame after an FSR 1 one
    // draws as a fresh GPU draws it.
    let Some(mut gpu) = gpu_with(64, 48, &FontFaces::bundled()) else {
        return;
    };
    let marked = flight();
    let small = LogicalSize { w: 16, h: 12 };
    let frames = [
        (fitted(LOGICAL, (64, 48)), Upscaler::Fsr1),
        (fitted(LOGICAL, (60, 45)), Upscaler::Fsr1),
        (fitted(small, (64, 48)), Upscaler::Fsr1),
        (fitted(LOGICAL, (64, 48)), Upscaler::Native),
    ];
    let mut renderer = Renderer::new(Sprites);
    let mut set = None;
    for (viewport, upscaler) in &frames {
        // Set only on a change, so the FSR 1 frames share one upscaler.
        if set != Some(*upscaler) {
            gpu.set_upscaler(*upscaler);
            set = Some(*upscaler);
        }
        renderer.render(&marked, viewport, &mut gpu);
        let reused = gpu.read_pixels().expect("read back");
        let fresh = drawn_into((64, 48), &marked, viewport, *upscaler);
        let (most, at) = largest_difference(&reused, &fresh, 64);
        assert_eq!(
            most,
            0,
            "{upscaler:?} at {:?} in {:?}: {at:?}",
            viewport.logical(),
            viewport.window_px()
        );
    }
}
