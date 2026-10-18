//! The sharp-bilinear upscaler through the offscreen adapter: a flight
//! frame's world drawn at the logical size and enlarged onto the viewport,
//! with the overlay (the HUD) drawn at the target's resolution over it.
//!
//! The reference is built from parts the native renderer draws, so it
//! needs nothing but the maths: the world alone at the logical size (W),
//! W enlarged on the CPU with the same sharp-bilinear maths (E), and the
//! overlay alone drawn natively over black (B0) and over white (B1).
//! Normal blending is linear in what is beneath, so the overlay over E is
//! `B0 + (B1 - B0) x E / 255` in every channel, text antialiasing and
//! sprite edges included. The overlay uses normal blending alone for that
//! reason; the other blends over a texture world are covered by the
//! renderer's layout tests.
//!
//! Needs a GPU adapter; skips, passing, without one.

use nova_render::fonts::FontFaces;
use nova_render::{
    Gpu, LogicalSize, PixelRect, Renderer, Upscaler, UpscalerProbe, Viewport, WorldResolution,
    resolution_of,
};
use nova_view::{Color, DrawList, ImageKey, ViewShape};

use super::flight_frame::{
    LOGICAL, SOFT, Sprites, TOLERANCE, alone, as_world, at, drawn, fitted, flight, hud,
    hud_over_white, largest_difference, over, world,
};
use super::wgpu_offscreen::gpu_with;

/// The world alone drawn natively at the logical size, one pixel a unit:
/// as the world texture holds it.
fn world_at_logical_size(draw: fn(&mut DrawList), logical: LogicalSize) -> Option<Vec<u8>> {
    let viewport = fitted(logical, (logical.w, logical.h));
    drawn(&alone(draw), &viewport, Upscaler::Native)
}

/// How a CPU enlargement samples the world.
#[derive(Clone, Copy)]
enum Sampling {
    Nearest,
    SharpBilinear,
}

/// `world` (`size` texels, RGBA8) enlarged onto `onto` of a `target`-sized
/// frame, its bars opaque black, sampled as `sampling` says: the
/// shader's maths in `f32`, its bilinear filter weighing the texels about
/// the sample point by their centres, clamped at the edges.
fn enlarged(
    world: &[u8],
    size: (u32, u32),
    target: (u32, u32),
    onto: PixelRect,
    sampling: Sampling,
) -> Vec<u8> {
    let (sw, sh) = (size.0 as f32, size.1 as f32);
    let scale = ((onto.w as f32 / sw).max(1.0), (onto.h as f32 / sh).max(1.0));
    let texel = |x: i64, y: i64, c: usize| -> f32 {
        let x = x.clamp(0, i64::from(size.0) - 1) as usize;
        let y = y.clamp(0, i64::from(size.1) - 1) as usize;
        f32::from(world[(y * size.0 as usize + x) * 4 + c])
    };
    let mut out = Vec::with_capacity((target.0 * target.1 * 4) as usize);
    for py in 0..target.1 {
        for px in 0..target.0 {
            let inside =
                (onto.x..onto.x + onto.w).contains(&px) && (onto.y..onto.y + onto.h).contains(&py);
            if !inside {
                out.extend([0, 0, 0, 255]);
                continue;
            }
            let u = (px as f32 + 0.5 - onto.x as f32) / onto.w as f32 * sw;
            let v = (py as f32 + 0.5 - onto.y as f32) / onto.h as f32 * sh;
            match sampling {
                Sampling::Nearest => {
                    for c in 0..3 {
                        out.push(texel(u.floor() as i64, v.floor() as i64, c) as u8);
                    }
                }
                Sampling::SharpBilinear => {
                    let sharp = |t: f32, scale: f32| {
                        let region = 0.5 - 0.5 / scale;
                        let d = t.fract() - 0.5;
                        let f = (d - d.clamp(-region, region)) * scale + 0.5;
                        t.floor() + f
                    };
                    // The texel centres either side of the sample point.
                    let (sx, sy) = (sharp(u, scale.0) - 0.5, sharp(v, scale.1) - 0.5);
                    let (x0, y0) = (sx.floor(), sy.floor());
                    let (tx, ty) = (sx - x0, sy - y0);
                    let (x0, y0) = (x0 as i64, y0 as i64);
                    for c in 0..3 {
                        let top = texel(x0, y0, c) * (1.0 - tx) + texel(x0 + 1, y0, c) * tx;
                        let bottom =
                            texel(x0, y0 + 1, c) * (1.0 - tx) + texel(x0 + 1, y0 + 1, c) * tx;
                        out.push((top * (1.0 - ty) + bottom * ty).round() as u8);
                    }
                }
            }
            out.push(255);
        }
    }
    out
}

/// Asserts `frame` is opaque, `reference` within [`TOLERANCE`] inside
/// `onto`, and exactly it in the bars.
#[track_caller]
fn assert_matches(frame: &[u8], reference: &[u8], target: (u32, u32), onto: PixelRect) {
    assert_eq!(frame.len(), reference.len());
    for (at, (got, want)) in frame.chunks(4).zip(reference.chunks(4)).enumerate() {
        let (x, y) = (at as u32 % target.0, at as u32 / target.0);
        assert_eq!(got[3], 255, "alpha at ({x}, {y})");
        let inside =
            (onto.x..onto.x + onto.w).contains(&x) && (onto.y..onto.y + onto.h).contains(&y);
        let tolerance = if inside { TOLERANCE } else { 0 };
        assert!(
            (0..3).all(|c| got[c].abs_diff(want[c]) <= tolerance),
            "({x}, {y}) is {got:?}, want {want:?} ±{tolerance}"
        );
    }
}

/// Draws [`flight`] into `viewport` with sharp-bilinear and checks it
/// against the reference, and that neither the native frame nor a nearest
/// enlargement passes for it.
fn assert_flight_matches_its_reference(viewport: &Viewport) {
    let logical = viewport.logical();
    let size = (logical.w, logical.h);
    let target = viewport.window_px();
    let (onto, _) = viewport.content().expect("an area");
    let (Some(frame), Some(w), Some(b0), Some(b1), Some(native)) = (
        drawn(&flight(), viewport, Upscaler::SharpBilinear),
        world_at_logical_size(world, logical),
        drawn(&alone(hud), viewport, Upscaler::Native),
        drawn(&hud_over_white(logical), viewport, Upscaler::Native),
        drawn(&flight(), viewport, Upscaler::Native),
    ) else {
        return;
    };
    let e = enlarged(&w, size, target, onto, Sampling::SharpBilinear);
    let reference = over(&b0, &b1, &e);
    assert_matches(&frame, &reference, target, onto);

    let (most, at) = largest_difference(&native, &reference, target.0);
    assert!(
        most > 8 * TOLERANCE,
        "the native frame is the reference within {most} (at {at:?}): the world was not enlarged"
    );
    let nearest = over(
        &b0,
        &b1,
        &enlarged(&w, size, target, onto, Sampling::Nearest),
    );
    let (most, at) = largest_difference(&nearest, &frame, target.0);
    assert!(
        most > TOLERANCE,
        "a nearest enlargement is the frame within {most} (at {at:?}): nothing blended"
    );
}

#[test]
fn a_flight_frame_at_1_875_is_its_world_enlarged_with_the_hud_native_over_it() {
    assert_flight_matches_its_reference(&fitted(LOGICAL, (60, 45)));
}

#[test]
fn a_letterboxed_flight_frame_at_1_875_is_enlarged_onto_its_viewport() {
    let viewport = fitted(LOGICAL, (60, 51));
    let (content, scale) = viewport.content().expect("an area");
    assert_eq!(
        (content, scale),
        (
            PixelRect {
                x: 0,
                y: 3,
                w: 60,
                h: 45
            },
            1.875
        )
    );
    assert_flight_matches_its_reference(&viewport);
}

#[test]
fn at_2x_the_world_is_its_nearest_enlargement_exactly() {
    let viewport = fitted(LOGICAL, (64, 48));
    let (Some(frame), Some(w)) = (
        drawn(&as_world(world), &viewport, Upscaler::SharpBilinear),
        world_at_logical_size(world, LOGICAL),
    ) else {
        return;
    };
    let onto = PixelRect {
        x: 0,
        y: 0,
        w: 64,
        h: 48,
    };
    let nearest = enlarged(&w, (32, 24), (64, 48), onto, Sampling::Nearest);
    let (most, at) = largest_difference(&frame, &nearest, 64);
    assert_eq!(most, 0, "differs at {at:?}");
    assert!(frame.chunks(4).all(|pixel| pixel[3] == 255));
}

/// A sparse world for a widened view: the patterned picture, the soft
/// sprite and the additive one, far apart.
fn sparse_world(list: &mut DrawList) {
    list.picture(ImageKey::picture(1), at(100.0, 100.0))
        .sprite(ImageKey::sprite(SOFT, 0), at(700.0, 400.0), Color::WHITE)
        .additive_sprite(ImageKey::sprite(5, 0), at(1300.0, 700.0), Color::WHITE);
}

#[test]
fn a_widened_world_is_enlarged_by_each_axis_own_scale() {
    let viewport = Viewport::for_shape(ViewShape::Widened, (1920, 1080), 1.0);
    let logical = viewport.logical();
    assert_eq!((logical.w, logical.h), (1365, 768));
    let (Some(frame), Some(w)) = (
        drawn(&as_world(sparse_world), &viewport, Upscaler::SharpBilinear),
        world_at_logical_size(sparse_world, logical),
    ) else {
        return;
    };
    let (onto, _) = viewport.content().expect("an area");
    let reference = enlarged(&w, (1365, 768), (1920, 1080), onto, Sampling::SharpBilinear);
    assert_matches(&frame, &reference, (1920, 1080), onto);
    let nearest = enlarged(&w, (1365, 768), (1920, 1080), onto, Sampling::Nearest);
    let (most, _) = largest_difference(&nearest, &frame, 1920);
    assert!(most > TOLERANCE, "blended on the seams");
}

#[test]
fn setting_the_upscaler_switches_how_the_world_is_drawn() {
    let Some(mut gpu) = gpu_with(8, 8, &FontFaces::bundled()) else {
        return;
    };
    assert_eq!(gpu.world_resolution(), WorldResolution::Native);
    gpu.set_upscaler(Upscaler::SharpBilinear);
    assert_eq!(gpu.world_resolution(), WorldResolution::Logical);
    gpu.set_upscaler(Upscaler::Native);
    assert_eq!(gpu.world_resolution(), WorldResolution::Native);
    // As the core's rule, which the recording mock follows, says.
    for upscaler in [Upscaler::SharpBilinear, Upscaler::Native] {
        gpu.set_upscaler(upscaler);
        assert_eq!(
            gpu.world_resolution(),
            resolution_of(upscaler),
            "{upscaler:?}"
        );
    }
}

#[test]
fn the_probe_offers_native_and_sharp_bilinear() {
    let Some(gpu) = gpu_with(8, 8, &FontFaces::bundled()) else {
        return;
    };
    // MetalFX may follow, on a device that has it (`metal_fx.rs`).
    let offered = UpscalerProbe::upscalers(&gpu);
    assert_eq!(offered[..2], [Upscaler::Native, Upscaler::SharpBilinear]);
}

#[test]
fn one_gpu_draws_a_texture_world_then_another_logical_size_then_natively() {
    // The world canvas is remade for a new logical size under the same
    // upscaler, which reads the new canvas, and a native frame after a
    // texture one draws as a fresh GPU draws it.
    let Some(mut gpu) = gpu_with(64, 48, &FontFaces::bundled()) else {
        return;
    };
    let mut marked = DrawList::new();
    marked.world(world);
    hud(&mut marked);
    let small = LogicalSize { w: 16, h: 12 };
    let frames = [
        (fitted(LOGICAL, (64, 48)), Upscaler::SharpBilinear),
        (fitted(small, (64, 48)), Upscaler::SharpBilinear),
        (fitted(LOGICAL, (64, 48)), Upscaler::Native),
    ];
    let mut renderer = Renderer::new(Sprites);
    let mut set = None;
    for (viewport, upscaler) in &frames {
        // Set only on a change, so both texture frames share one upscaler.
        if set != Some(*upscaler) {
            gpu.set_upscaler(*upscaler);
            set = Some(*upscaler);
        }
        renderer.render(&marked, viewport, &mut gpu);
        let reused = gpu.read_pixels().expect("read back");
        let fresh = drawn(&marked, viewport, *upscaler).expect("an adapter");
        let (most, at) = largest_difference(&reused, &fresh, 64);
        assert_eq!(most, 0, "{upscaler:?} at {:?}: {at:?}", viewport.logical());
    }
}
