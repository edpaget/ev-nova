//! The layered sky ([`DrawCommand::Sky`]) drawn offscreen through the
//! draw list, in the world layer. Every pixel checked is predicted from
//! the core's own stars (`nova_render::sky`), placed by its projection
//! and lit by its falloff, never retyped: the stars draw where the core
//! says, each layer moves by its depth's parallax as the eye moves, the
//! murk level dims them, what is drawn after the sky covers it, every
//! layer still shows at the lowest render scale, and through a jump's
//! rush each star is drawn as its streak by depth, near ones longer, the
//! same sky at rest round.
//!
//! Logical 512 × 512 in a 512 × 512 target: a logical unit is a pixel.
//! Needs a GPU adapter; skips, passing, without one. Native only, but for
//! the render scale's world texture.

use nova_render::fonts::FontFaces;
use nova_render::sky::{LAYERS, StarInstance, layer_stars, parallax, stars, streak_coverage};
use nova_render::{LogicalSize, RenderReport, RenderScale, Renderer, Viewport};
use nova_view::geometry::Bounds;
use nova_view::{Color, DrawCommand, DrawList, Point, SkyPose};

use super::common::fill;
use super::wgpu_offscreen::{Images, gpu_with};

const SIDE: u32 = 512;
const LOGICAL: LogicalSize = LogicalSize { w: SIDE, h: SIDE };

/// The eye's move between two frames.
const DELTA: (f32, f32) = (120.0, -80.0);

/// The sky seen from `eye` at `level`.
fn pose(eye: (f32, f32), level: u8) -> SkyPose {
    SkyPose::new(
        77,
        Point::new(eye.0, eye.1),
        Point::new(256.0, 256.0),
        level,
    )
}

const EYE: (f32, f32) = (1000.0, -500.0);

fn moved() -> (f32, f32) {
    (EYE.0 + DELTA.0, EYE.1 + DELTA.1)
}

/// `list` drawn at `scale`, read back; `None` with no adapter.
fn drawn(list: &DrawList, scale: RenderScale) -> Option<Vec<u8>> {
    let mut gpu = gpu_with(SIDE, SIDE, &FontFaces::bundled())?;
    let mut renderer = Renderer::new(Images);
    renderer.set_render_scale(scale);
    let viewport = Viewport::new(LOGICAL, (SIDE, SIDE), 1.0);
    let report = renderer.render(list, &viewport, &mut gpu);
    assert_eq!(report, RenderReport::default());
    Some(gpu.read_pixels().expect("read back"))
}

/// The sky alone in the world layer, seen as `sky`, read back.
fn sky_drawn(sky: SkyPose) -> Option<Vec<u8>> {
    let mut list = DrawList::new();
    list.world(|list| {
        list.sky(sky);
    });
    drawn(&list, RenderScale::Auto)
}

fn rgb(pixels: &[u8], (x, y): (u32, u32)) -> [u8; 3] {
    let at = ((y * SIDE + x) * 4) as usize;
    [pixels[at], pixels[at + 1], pixels[at + 2]]
}

/// The pixel a logical point is in.
fn pixel_of(p: Point) -> (u32, u32) {
    (p.x.floor() as u32, p.y.floor() as u32)
}

/// The colour the core predicts at pixel `at`'s centre from `stars`, in
/// 8 bits: the sum of each star's colour × its streak's falloff there
/// (a round star's at no trail).
fn predicted(stars: &[StarInstance], (x, y): (u32, u32)) -> [f32; 3] {
    let centre = (x as f32 + 0.5, y as f32 + 0.5);
    let mut sum = [0.0; 3];
    for star in stars {
        let offset = Point::new(centre.0 - star.centre.x, centre.1 - star.centre.y);
        let lit = streak_coverage(offset, star.trail, star.radius);
        for (channel, c) in sum.iter_mut().zip(star.color) {
            *channel += c * lit;
        }
    }
    sum.map(|c| (c * 255.0).min(255.0))
}

fn assert_near(pixels: &[u8], stars: &[StarInstance], at: (u32, u32), what: &str) {
    let seen = rgb(pixels, at);
    let expected = predicted(stars, at);
    for (s, e) in seen.iter().zip(expected) {
        assert!(
            (f32::from(*s) - e).abs() <= 3.0,
            "{what} at {at:?}: {seen:?}, predicted {expected:?}"
        );
    }
}

/// Whether `star` is well inside the view, with no other of `all` within
/// a few pixels of its edge.
fn isolated(star: &StarInstance, all: &[StarInstance]) -> bool {
    let inside = |c: f32| (40.0..SIDE as f32 - 40.0).contains(&c);
    inside(star.centre.x)
        && inside(star.centre.y)
        && all.iter().all(|other| {
            other == star
                || (other.centre.x - star.centre.x).hypot(other.centre.y - star.centre.y)
                    > star.radius + other.radius + 6.0
        })
}

/// A star of layer `layer` isolated at [`EYE`] whose moved place is
/// isolated after the eye moves by [`DELTA`]: the star at `EYE` and the
/// same star after the move.
fn picked(layer: usize) -> (StarInstance, StarInstance) {
    let before_all = stars(&pose(EYE, 32), LOGICAL, 1.0);
    let after_all = stars(&pose(moved(), 32), LOGICAL, 1.0);
    let scale = parallax(LAYERS[layer].depth);
    layer_stars(&pose(EYE, 32), layer, LOGICAL, 1.0)
        .into_iter()
        .filter(|star| isolated(star, &before_all))
        .find_map(|star| {
            let to = Point::new(
                star.centre.x - DELTA.0 * scale,
                star.centre.y - DELTA.1 * scale,
            );
            let after = after_all.iter().find(|other| {
                (other.centre.x - to.x).abs() < 1e-2 && (other.centre.y - to.y).abs() < 1e-2
            })?;
            isolated(after, &after_all).then_some((star, *after))
        })
        .unwrap_or_else(|| panic!("an isolated star in layer {layer}"))
}

/// The nearest layer and the farthest.
const NEAR: usize = LAYERS.len() - 1;
const FAR: usize = 0;

#[test]
fn the_sky_draws_its_stars_where_the_core_puts_them() {
    let sky = pose(EYE, 32);
    let Some(pixels) = sky_drawn(sky) else {
        return;
    };
    let all = stars(&sky, LOGICAL, 1.0);
    for layer in [NEAR, FAR] {
        let (star, _) = picked(layer);
        let at = pixel_of(star.centre);
        assert_near(&pixels, &all, at, &format!("layer {layer}'s star"));
        assert!(rgb(&pixels, at).iter().any(|&c| c > 0), "{star:?} lit");
    }
    // A pixel far from every star is the clear colour.
    let dark = (0..SIDE)
        .flat_map(|y| (0..SIDE).map(move |x| (x, y)))
        .find(|&(x, y)| {
            all.iter().all(|star| {
                (x as f32 + 0.5 - star.centre.x).hypot(y as f32 + 0.5 - star.centre.y)
                    > star.radius + 4.0
            })
        })
        .expect("some dark pixel");
    assert_eq!(rgb(&pixels, dark), [0, 0, 0]);
    let lit = all
        .iter()
        .filter(|star| {
            let at = pixel_of(star.centre);
            at.0 < SIDE && at.1 < SIDE && rgb(&pixels, at).iter().any(|&c| c > 0)
        })
        .count();
    assert!(lit > 100, "{lit} lit stars");
}

/// The brightness-weighted centre of the pixels within a few of `star`.
fn centroid(pixels: &[u8], star: &StarInstance) -> (f32, f32) {
    let reach = star.radius + 2.0;
    let (mut x, mut y, mut weight) = (0.0, 0.0, 0.0);
    let lo = |c: f32| (c - reach).floor().max(0.0) as u32;
    let hi = |c: f32| ((c + reach).ceil() as u32).min(SIDE - 1);
    for py in lo(star.centre.y)..=hi(star.centre.y) {
        for px in lo(star.centre.x)..=hi(star.centre.x) {
            let w: f32 = rgb(pixels, (px, py)).iter().map(|&c| f32::from(c)).sum();
            x += (px as f32 + 0.5) * w;
            y += (py as f32 + 0.5) * w;
            weight += w;
        }
    }
    assert!(weight > 0.0, "{star:?} lit");
    (x / weight, y / weight)
}

#[test]
fn each_layer_moves_by_its_depths_parallax_as_the_eye_moves() {
    let (Some(before), Some(after)) = (sky_drawn(pose(EYE, 32)), sky_drawn(pose(moved(), 32)))
    else {
        return;
    };
    let mut moves = Vec::new();
    for layer in [NEAR, FAR] {
        let (from, to) = picked(layer);
        let (x0, y0) = centroid(&before, &from);
        let (x1, y1) = centroid(&after, &to);
        let scale = parallax(LAYERS[layer].depth);
        let expected = (-DELTA.0 * scale, -DELTA.1 * scale);
        assert!(
            (x1 - x0 - expected.0).abs() <= 1.0 && (y1 - y0 - expected.1).abs() <= 1.0,
            "layer {layer}: moved ({}, {}), expected {expected:?}",
            x1 - x0,
            y1 - y0
        );
        moves.push((x1 - x0).hypot(y1 - y0));
    }
    assert!(
        moves[0] > moves[1] + 20.0,
        "the near layer further: {moves:?}"
    );
}

#[test]
fn the_murk_level_dims_the_sky_and_level_zero_draws_none() {
    let (Some(clear), Some(half), Some(none)) = (
        sky_drawn(pose(EYE, 32)),
        sky_drawn(pose(EYE, 16)),
        sky_drawn(pose(EYE, 0)),
    ) else {
        return;
    };
    for layer in [NEAR, FAR] {
        let (star, _) = picked(layer);
        let at = pixel_of(star.centre);
        let (full, dim) = (rgb(&clear, at), rgb(&half, at));
        for (f, d) in full.iter().zip(dim) {
            assert!(
                (f32::from(*f) / 2.0 - f32::from(d)).abs() <= 1.5,
                "layer {layer} at {at:?}: {full:?} then {dim:?}"
            );
        }
        assert!(full.iter().any(|&c| c > 4), "{full:?}");
    }
    assert!(none.chunks(4).all(|px| px[..3] == [0, 0, 0]));
}

#[test]
fn what_is_drawn_after_the_sky_covers_it() {
    let sky = pose(EYE, 32);
    let (star, _) = picked(NEAR);
    let over = Bounds {
        min: Point::new(star.centre.x - 6.0, star.centre.y - 6.0),
        max: Point::new(star.centre.x + 6.0, star.centre.y + 6.0),
    };
    let mut list = DrawList::new();
    list.world(|list| {
        list.push(DrawCommand::Sky { sky });
        fill(list, over, Color::rgba(30, 90, 160, 255));
    });
    let Some(pixels) = drawn(&list, RenderScale::Auto) else {
        return;
    };
    assert_eq!(rgb(&pixels, pixel_of(star.centre)), [30, 90, 160]);
}

/// The rush along a jump heading, down and to the right.
const RUSH: Point = Point::new(80.0, 40.0);

/// The rush's heading, a unit long.
fn heading() -> Point {
    let length = RUSH.x.hypot(RUSH.y);
    Point::new(RUSH.x / length, RUSH.y / length)
}

/// The pixel `k` units along the heading from `star`'s centre.
fn along(star: &StarInstance, k: u32) -> (u32, u32) {
    let k = k as f32;
    pixel_of(Point::new(
        star.centre.x + heading().x * k,
        star.centre.y + heading().y * k,
    ))
}

/// A star of layer `layer` of the rushing sky `all` whose line along the
/// heading, from its centre `reach` units on, is well inside the view
/// and lit by no other star.
fn clear_streak(layer: usize, sky: &SkyPose, all: &[StarInstance], reach: u32) -> StarInstance {
    let inside = |(x, y): (u32, u32)| {
        let inner = 40..SIDE - 40;
        inner.contains(&x) && inner.contains(&y)
    };
    layer_stars(sky, layer, LOGICAL, 1.0)
        .into_iter()
        .find(|star| {
            let others: Vec<StarInstance> =
                all.iter().filter(|other| *other != star).copied().collect();
            (0..=reach).all(|k| {
                let at = along(star, k);
                inside(at) && predicted(&others, at).iter().all(|&c| c <= 0.0)
            })
        })
        .unwrap_or_else(|| panic!("a clear streak in layer {layer}"))
}

/// How many units along the heading from `star`'s centre are lit before
/// the first dark pixel, up to `reach`.
fn lit_run(pixels: &[u8], star: &StarInstance, reach: u32) -> u32 {
    (0..=reach)
        .find(|&k| rgb(pixels, along(star, k)) == [0, 0, 0])
        .unwrap_or(reach + 1)
}

#[test]
fn through_the_rush_each_star_is_drawn_as_its_streak_near_ones_longer() {
    let sky = pose(EYE, 32).with_rush(RUSH);
    let Some(pixels) = sky_drawn(sky) else {
        return;
    };
    let all = stars(&sky, LOGICAL, 1.0);
    let near_trail = RUSH.x.hypot(RUSH.y) * parallax(LAYERS[NEAR].depth);
    let reach = near_trail.ceil() as u32 + 8;
    let near = clear_streak(NEAR, &sky, &all, reach);
    let far = clear_streak(FAR, &sky, &all, reach);
    // Along the near streak, its centre, middle and tail are lit as the
    // core predicts.
    let length = near.trail.x.hypot(near.trail.y);
    for share in [0.0, 0.5, 1.0] {
        let at = along(&near, (length * share) as u32);
        assert_near(&pixels, &all, at, &format!("the near streak at {share}"));
        assert!(rgb(&pixels, at).iter().any(|&c| c > 0), "lit at {share}");
    }
    // The near streak runs its trail's length, the far one barely past its
    // own.
    let (near_run, far_run) = (
        lit_run(&pixels, &near, reach),
        lit_run(&pixels, &far, reach),
    );
    let far_length = far.trail.x.hypot(far.trail.y);
    assert!(near_run as f32 >= length - 1.0, "{near_run} for {length}");
    assert!(
        far_run as f32 <= far_length + far.radius + 2.0,
        "{far_run} for {far_length}"
    );
    assert!(near_run > far_run + 20, "{near_run} then {far_run}");
}

#[test]
fn the_same_sky_not_rushing_draws_round_stars() {
    let rushing = pose(EYE, 32).with_rush(RUSH);
    let (Some(streaked), Some(round)) = (sky_drawn(rushing), sky_drawn(pose(EYE, 32))) else {
        return;
    };
    let all = stars(&rushing, LOGICAL, 1.0);
    let near_trail = RUSH.x.hypot(RUSH.y) * parallax(LAYERS[NEAR].depth);
    let reach = near_trail.ceil() as u32 + 8;
    let near = clear_streak(NEAR, &rushing, &all, reach);
    let middle = along(&near, (near_trail / 2.0) as u32);
    assert!(rgb(&streaked, middle).iter().any(|&c| c > 0));
    assert_eq!(rgb(&round, middle), [0, 0, 0], "no streak at rest");
    assert!(rgb(&round, pixel_of(near.centre)).iter().any(|&c| c > 0));
    assert_ne!(streaked, round);
}

#[test]
fn at_the_lowest_render_scale_every_layer_still_lights_pixels() {
    let sky = pose(EYE, 32);
    let mut list = DrawList::new();
    list.world(|list| {
        list.sky(sky);
    });
    let Some(pixels) = drawn(&list, RenderScale::Percent(RenderScale::MIN)) else {
        return;
    };
    // A quarter of the 512-wide backing: a texel is four units.
    let texels = f32::from(RenderScale::MIN) / 100.0;
    for layer in 0..LAYERS.len() {
        let layer_sky = layer_stars(&sky, layer, LOGICAL, texels);
        assert!(!layer_sky.is_empty());
        let shown = layer_sky
            .iter()
            .filter(|star| {
                let (x, y) = pixel_of(star.centre);
                x < SIDE && y < SIDE && rgb(&pixels, (x, y)).iter().any(|&c| c > 0)
            })
            .count();
        assert!(
            shown * 2 > layer_sky.len(),
            "layer {layer}: {shown} of {} lit",
            layer_sky.len()
        );
    }
}
