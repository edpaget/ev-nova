//! The layered sky ([`nova_view::DrawCommand::Sky`]): stars generated in
//! layers at depth behind the play plane, worked out here so the adapter
//! only places a quad per star.
//!
//! # The projection
//!
//! A pinhole eye [`EYE_HEIGHT`] world units above the play plane, over
//! the point the flight camera centres on ([`SkyPose::eye`]), sees a
//! point at world `(x, y, d)`, `d` behind the plane, at logical
//! `anchor + (xy − eye) · parallax(d)` ([`project`]), with
//! `parallax(d) = EYE_HEIGHT / (EYE_HEIGHT + d)` ([`parallax`]). At
//! `d = 0` that is the world camera itself (1 logical unit = 1 world
//! unit, the eye drawn at the anchor), so the play plane keeps its
//! orthographic camera and only the sky gets the perspective term. A
//! layer at depth `d` moves by `parallax(d)` of the ship's move, so far
//! layers drift slower, and a star's drawn size is its world size times
//! the same factor.
//!
//! # The layers
//!
//! Five, far to near ([`LAYERS`]):
//!
//! | depth | parallax | stars per 1024×768 | drawn radius | brightness |
//! |---|---|---|---|---|
//! | 19000 | 0.05 | 160 | 1.0–1.5 | 0.45–0.7 |
//! | 9000 | 0.1 | 110 | 1.1–1.8 | 0.5–0.75 |
//! | 4555.6 | 0.18 | 70 | 1.3–2.2 | 0.55–0.85 |
//! | 2333.3 | 0.3 | 40 | 1.5–2.6 | 0.6–0.95 |
//! | 1000 | 0.5 | 20 | 1.8–3.2 | 0.65–1.0 |
//!
//! About 400 stars in a 1024×768 view, against the original's 20. A
//! layer's radius range is the radius its stars are drawn at, in logical
//! units; a star's size in the world is that over its layer's parallax.
//! The falloff ([`coverage`]) is steep, so even the far layer's least
//! star is a unit in radius, enough to light the pixel it sits on at the
//! native scale.
//! Each star's tint is drawn from a weighted palette of star temperatures
//! ([`TINTS`]): mostly white, then blue-white and pale yellow, a few
//! orange and red.
//!
//! # The stars
//!
//! Each layer is a grid of square cells [`CELL`] logical units on a side,
//! fixed to the layer, so its stars move with it. Each cell's stars are an
//! integer hash (`SplitMix64`'s mix) of the seed, the layer, the cell and the
//! star's index in it, mapped to its place, radius, brightness and tint by
//! IEEE `+ − × ÷` alone: the same pose gives the same stars on every
//! platform. Only the cells the view (grown by [`MARGIN`]) overlaps are
//! generated, at most [`cell_bound`] per layer, so the work is bounded by
//! the logical size.
//!
//! A star's colour is its tint × its brightness × the pose's level / 32,
//! so a murky system's sky dims as its 2D stars do. Drawn into a world
//! texture whose texels are larger than a star (the lowest render scale),
//! a star is drawn at least one texel in radius, its brightness lowered so
//! it gives the same light ([`stars`]). The shader adds `colour ×
//! coverage(distance / radius)` ([`coverage`]) to what is beneath.
//!
//! # The rush
//!
//! Through a jump's warp the pose carries the rush ([`SkyPose::rush`]),
//! the eye's move over the last step. Each star is then drawn as the
//! streak it swept on screen over that step: from where it is now back
//! to where it was, the rush times its layer's parallax
//! ([`StarInstance::trail`]). Near layers streak long, far ones barely.
//! The streak keeps the star's radius and colour along its length: the
//! shader adds `colour × coverage(distance to the segment / radius)`
//! ([`streak_coverage`]), which is the round star's light at no trail.
//!
//! A trail is cut down along its heading to reach at most across the
//! view each way, and a rush that is not finite draws round stars. Stars
//! are generated and kept by where their streak reaches, not their
//! centre, so a streak whose head is off the view but whose tail crosses
//! into it still shows; the extra cells keep the work within
//! [`rushing_cell_bound`]. With no rush every star is round and exactly
//! as it is without one.
//!
//! # The look
//!
//! A system's data can set its own sky look ([`SkyLook`], from its
//! `sky` block): its star density, 0 to 2 times the layers', and a tint
//! each star leans [`TINT_SHARE`] of the way toward ([`look`]). The
//! renderer holds the looks in a table keyed by the sky's seed
//! ([`SkyLooks`]); a seed with none draws the default, the engine's stars
//! and a seeded backdrop. [`stars_of`] is [`stars`] in a look.
//!
//! # The backdrop
//!
//! Behind every star layer is a distant nebula backdrop
//! ([`backdrop`](mod@backdrop)):
//! one far layer at [`BACKDROP_DEPTH`], parallax 0.025, of large, faint
//! puffs clustered into clouds, one cloud at most in each
//! [`BACKDROP_CELL`]-unit cell, with dark sky between them. Its colour
//! and amount come from the look, or from the seed ([`seeded`]). It is
//! drawn first, as [`StarInstance`]s of no trail through the same
//! pipeline ([`instances`]), doesn't streak through the warp, and keeps
//! its light, instance count and shaded area within bounds it holds by
//! construction ([`backdrop_instance_bound`], [`backdrop_area_bound`],
//! [`BACKDROP_LIGHT_BOUND`]).

use nova_view::{Point, SkyPose};

use crate::viewport::LogicalSize;

mod backdrop;
mod look;

pub use backdrop::{
    BACKDROP_CELL, BACKDROP_DEPTH, BACKDROP_HUES, BACKDROP_LIGHT_BOUND, Backdrop, CLOUD_EXTENT,
    CLOUD_LIGHT, Cloud, PUFF_RADIUS, PUFF_SPREAD, PUFFS, Puff, SEEDED_AMOUNT, backdrop,
    backdrop_area_bound, backdrop_instance_bound, cloud, instances, resolved, seeded,
};
pub use look::{BackdropLook, SkyLook, SkyLooks, TINT_SHARE, normalised};

/// How high above the play plane the eye is, in world units.
pub const EYE_HEIGHT: f32 = 1000.0;

/// The side of a layer's cells, in logical units.
pub const CELL: f32 = 128.0;

/// How far past the view's edges stars are generated, in logical units:
/// more than any star's radius, so a star just off the edge still shows
/// its part inside.
pub const MARGIN: f32 = 4.0;

/// The view a layer's star density is given for: 1024×768.
const DENSITY_AREA: f32 = 1024.0 * 768.0;

/// One layer of the sky.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SkyLayer {
    /// How far behind the play plane it is, in world units.
    pub depth: f32,
    /// How many stars it shows in a 1024×768 view, on average.
    pub density: f32,
    /// The least and most radius its stars are drawn at, in logical units.
    pub radius: (f32, f32),
    /// The least and most brightness of its stars, 0 to 1.
    pub brightness: (f32, f32),
}

/// The sky's layers, far to near.
pub const LAYERS: [SkyLayer; 5] = [
    SkyLayer {
        depth: 19000.0,
        density: 160.0,
        radius: (1.0, 1.5),
        brightness: (0.45, 0.7),
    },
    SkyLayer {
        depth: 9000.0,
        density: 110.0,
        radius: (1.1, 1.8),
        brightness: (0.5, 0.75),
    },
    SkyLayer {
        depth: 4555.6,
        density: 70.0,
        radius: (1.3, 2.2),
        brightness: (0.55, 0.85),
    },
    SkyLayer {
        depth: 2333.3,
        density: 40.0,
        radius: (1.5, 2.6),
        brightness: (0.6, 0.95),
    },
    SkyLayer {
        depth: 1000.0,
        density: 20.0,
        radius: (1.8, 3.2),
        brightness: (0.65, 1.0),
    },
];

/// The star tints, RGB 0 to 1, each with its weight: blue-white, white,
/// pale yellow, orange and red.
pub const TINTS: [([f32; 3], u32); 5] = [
    ([0.75, 0.85, 1.0], 4),
    ([1.0, 1.0, 1.0], 10),
    ([1.0, 0.95, 0.8], 3),
    ([1.0, 0.8, 0.6], 2),
    ([1.0, 0.65, 0.55], 1),
];

/// One star as the sky shader draws it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StarInstance {
    /// Its centre, in logical units.
    pub centre: Point,
    /// Its drawn radius, in logical units: the square it covers is twice
    /// that on a side.
    pub radius: f32,
    /// The light it adds at its centre, RGB 0 to 1.
    pub color: [f32; 3],
    /// Where it was drawn a step ago, from its centre, in logical units:
    /// it is drawn as the streak from its centre to there
    /// ([`streak_coverage`]). Zero for a round star.
    pub trail: Point,
}

/// How much of a layer at `depth` behind the play plane moves with the
/// eye, and how much its stars are scaled: 1 on the plane, falling with
/// depth.
#[must_use]
pub fn parallax(depth: f32) -> f32 {
    EYE_HEIGHT / (EYE_HEIGHT + depth)
}

/// Where the world point `point` (x, y, and its depth behind the play
/// plane) is drawn, in logical units, seen as `pose` sees it.
#[must_use]
pub fn project(point: [f32; 3], pose: &SkyPose) -> Point {
    let [x, y, depth] = point;
    let scale = parallax(depth);
    Point::new(
        pose.anchor.x + (x - pose.eye.x) * scale,
        pose.anchor.y + (y - pose.eye.y) * scale,
    )
}

/// How much of a star's light lands at `t` of its radius from its centre:
/// 1 at the centre, falling smoothly to 0 at its edge and beyond.
#[must_use]
pub fn coverage(t: f32) -> f32 {
    if t >= 1.0 {
        return 0.0;
    }
    let inside = 1.0 - t * t;
    inside * inside
}

/// How much of a streak's light lands at `offset` from its centre, the
/// streak running from the centre along `trail`, `radius` wide each side:
/// [`coverage`] of the distance from the segment, in radii. With no trail
/// it is a round star's.
#[must_use]
pub fn streak_coverage(offset: Point, trail: Point, radius: f32) -> f32 {
    let length = trail.x * trail.x + trail.y * trail.y;
    let along = if length > 0.0 {
        ((offset.x * trail.x + offset.y * trail.y) / length).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let (dx, dy) = (offset.x - trail.x * along, offset.y - trail.y * along);
    coverage(dx.hypot(dy) / radius)
}

/// The most cells a layer generates for a view of `logical` size: the
/// cells the view, grown by [`MARGIN`] each way, can overlap.
#[must_use]
pub fn cell_bound(logical: LogicalSize) -> usize {
    let (nx, ny) = cell_span(logical, Point::new(0.0, 0.0));
    nx * ny
}

/// The most cells a layer generates for a view of `logical` size with
/// any rush: the cells the view, grown by [`MARGIN`] each way and by the
/// longest trail ([`trail_of`]), the view's own size, can overlap.
#[must_use]
pub fn rushing_cell_bound(logical: LogicalSize) -> usize {
    let longest = Point::new(logical.w as f32, logical.h as f32);
    let (nx, ny) = cell_span(logical, longest);
    nx * ny
}

/// The cells across and down a view of `logical` size overlaps, grown
/// by [`MARGIN`] each way and by `trail` on its side.
fn cell_span(logical: LogicalSize, trail: Point) -> (usize, usize) {
    let span = |extent: u32, trail: f32| {
        ((extent as f32 + 2.0 * MARGIN + trail.abs()) / CELL).ceil() as usize + 1
    };
    (span(logical.w, trail.x), span(logical.h, trail.y))
}

/// The trail a layer of parallax `scale` streaks by in a view of
/// `logical` size, rushing by `rush`: the rush times the parallax, cut
/// down along its heading to reach at most across the view each way,
/// and none for a rush that is not finite.
fn trail_of(rush: Point, scale: f32, logical: LogicalSize) -> Point {
    if !(rush.x.is_finite() && rush.y.is_finite()) {
        return Point::new(0.0, 0.0);
    }
    let (x, y) = (rush.x * scale, rush.y * scale);
    // At no trail the share is infinite, which the cap takes to 1.
    let fit = |reach: f32, along: f32| (reach / along.abs()).min(1.0);
    let cut = fit(logical.w as f32, x).min(fit(logical.h as f32, y));
    Point::new(x * cut, y * cut)
}

/// Whether the segment from `from` along `trail` reaches into the box
/// from `lo` to `hi`: with no trail, whether `from` is in it, its far
/// edges left out.
fn reaches(from: Point, trail: Point, lo: Point, hi: Point) -> bool {
    if trail == Point::new(0.0, 0.0) {
        return (lo.x..hi.x).contains(&from.x) && (lo.y..hi.y).contains(&from.y);
    }
    // Liang–Barsky: narrow the segment's span [enter, leave] by each slab.
    let (mut enter, mut leave) = (0.0_f32, 1.0_f32);
    for (start, step, lo, hi) in [(from.x, trail.x, lo.x, hi.x), (from.y, trail.y, lo.y, hi.y)] {
        if step == 0.0 {
            if !(lo..hi).contains(&start) {
                return false;
            }
            continue;
        }
        let (a, b) = ((lo - start) / step, (hi - start) / step);
        enter = enter.max(a.min(b));
        leave = leave.min(a.max(b));
    }
    enter <= leave
}

/// The sky `pose` sees in a view of `logical` size, drawn into a target
/// of `texels_per_unit` texels per logical unit: every layer's stars, far
/// layer first. None at level 0, or with an eye or anchor that is not
/// finite.
///
/// A star smaller than one texel in radius is drawn one texel in radius,
/// its brightness lowered by the square of the ratio, so it gives the same
/// light; a `texels_per_unit` that is not a positive, finite number
/// leaves every star as it is.
#[must_use]
pub fn stars(pose: &SkyPose, logical: LogicalSize, texels_per_unit: f32) -> Vec<StarInstance> {
    stars_of(pose, &SkyLook::default(), logical, texels_per_unit)
}

/// The stars of [`stars`], in a system of look `look`: its star density
/// and tint ([`look`]).
#[must_use]
pub fn stars_of(
    pose: &SkyPose,
    look: &SkyLook,
    logical: LogicalSize,
    texels_per_unit: f32,
) -> Vec<StarInstance> {
    (0..LAYERS.len())
        .flat_map(|layer| layer_stars_of(pose, layer, look, logical, texels_per_unit))
        .collect()
}

/// Layer `layer`'s stars ([`LAYERS`]), as [`stars`] gives them.
#[must_use]
pub fn layer_stars(
    pose: &SkyPose,
    layer: usize,
    logical: LogicalSize,
    texels_per_unit: f32,
) -> Vec<StarInstance> {
    layer_stars_of(pose, layer, &SkyLook::default(), logical, texels_per_unit)
}

/// Layer `layer`'s stars, as [`stars_of`] gives them in a system of look
/// `look`.
#[must_use]
pub fn layer_stars_of(
    pose: &SkyPose,
    layer: usize,
    look: &SkyLook,
    logical: LogicalSize,
    texels_per_unit: f32,
) -> Vec<StarInstance> {
    let Some(&sky_layer) = LAYERS.get(layer) else {
        return Vec::new();
    };
    // An eye or anchor that is not finite places every star at a point
    // that is not finite. It is turned away here: a trail's bounds test
    // ([`reaches`]) cannot be trusted to leave such a star out, as a NaN
    // slab leaves the segment's span unnarrowed.
    let finite = |point: Point| point.x.is_finite() && point.y.is_finite();
    if pose.level == 0 || !(finite(pose.eye) && finite(pose.anchor)) {
        return Vec::new();
    }
    let level = f32::from(pose.level.min(32)) / 32.0;
    let scale = parallax(sky_layer.depth);
    let trail = trail_of(pose.rush, scale, logical);
    let (nx, ny) = cell_span(logical, trail);
    // The layer's own coordinates, in logical units: the eye's point on
    // it, then the view's top-left corner (grown by the margin).
    let left = f64::from(pose.eye.x) * f64::from(scale) - f64::from(pose.anchor.x);
    let top = f64::from(pose.eye.y) * f64::from(scale) - f64::from(pose.anchor.y);
    let (left, top) = (left - f64::from(MARGIN), top - f64::from(MARGIN));
    let cell = f64::from(CELL);
    let first = |edge: f64| (edge / cell).floor();
    let (cx0, cy0) = (first(left), first(top));
    // Where cell (cx0, cy0)'s corner is drawn, in logical units. Every
    // star is placed from it, so a rush moves no star.
    let corner_x = (cx0 * cell - left) as f32 - MARGIN;
    let corner_y = (cy0 * cell - top) as f32 - MARGIN;
    // A star whose trail reaches back into the view is kept, so the
    // cells start further out on the side the trail reaches from: this
    // many cells before (cx0, cy0), none without a rush.
    let back = |edge: f64, grow: f32| (first(edge - f64::from(grow.max(0.0))) - first(edge)) as i64;
    let (back_x, back_y) = (back(left, trail.x), back(top, trail.y));
    let per_cell = sky_layer.density * CELL * CELL / DENSITY_AREA * look.star_density();
    let lo = Point::new(-MARGIN, -MARGIN);
    let hi = Point::new(logical.w as f32 + MARGIN, logical.h as f32 + MARGIN);
    let min_radius = if texels_per_unit.is_finite() && texels_per_unit > 0.0 {
        1.0 / texels_per_unit
    } else {
        0.0
    };
    let mut out = Vec::new();
    for j in (0..ny as i64).map(|j| j + back_y) {
        for i in (0..nx as i64).map(|i| i + back_x) {
            let cx = (cx0 as i64).wrapping_add(i);
            let cy = (cy0 as i64).wrapping_add(j);
            let mut draws = Draws::new(pose.seed, layer, cx, cy);
            let whole = per_cell.floor();
            let count = whole as u32 + u32::from(draws.unit() < per_cell - whole);
            for _ in 0..count {
                let x = corner_x + (i as f32 + draws.unit()) * CELL;
                let y = corner_y + (j as f32 + draws.unit()) * CELL;
                let radius = between(sky_layer.radius, draws.unit());
                let brightness = between(sky_layer.brightness, draws.unit());
                let tint = look.tinted(tint(draws.next()));
                if !reaches(Point::new(x, y), trail, lo, hi) {
                    continue;
                }
                let (radius, brightness) = if radius < min_radius {
                    let kept = radius / min_radius;
                    (min_radius, brightness * kept * kept)
                } else {
                    (radius, brightness)
                };
                let light = brightness * level;
                out.push(StarInstance {
                    centre: Point::new(x, y),
                    radius,
                    color: tint.map(|c| c * light),
                    trail,
                });
            }
        }
    }
    out
}

/// `range`'s value at `t` of the way from its least to its most.
fn between((lo, hi): (f32, f32), t: f32) -> f32 {
    lo + (hi - lo) * t
}

/// The tint `draw` picks from [`TINTS`] by their weights.
fn tint(draw: u64) -> [f32; 3] {
    let total: u32 = TINTS.iter().map(|&(_, weight)| weight).sum();
    let mut pick = (draw % u64::from(total)) as u32;
    for &(tint, weight) in &TINTS {
        if pick < weight {
            return tint;
        }
        pick -= weight;
    }
    TINTS[0].0
}

/// `SplitMix64`'s step: the hashes one cell's stars are drawn from.
struct Draws(u64);

impl Draws {
    /// The draws for cell (`cx`, `cy`) of layer `layer` of sky `seed`.
    fn new(seed: u64, layer: usize, cx: i64, cy: i64) -> Self {
        let mut state = mix(seed);
        for part in [layer as u64, cx as u64, cy as u64] {
            state = mix(state ^ part);
        }
        Self(state)
    }

    /// The next hash.
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        mix(self.0)
    }

    /// The next hash as a number in [0, 1), exactly representable.
    fn unit(&mut self) -> f32 {
        (self.next() >> 40) as f32 / (1u64 << 24) as f32
    }
}

/// `SplitMix64`'s finaliser.
fn mix(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::viewport::LOGICAL;

    fn pose(seed: u64, eye: (f32, f32), level: u8) -> SkyPose {
        SkyPose::new(
            seed,
            Point::new(eye.0, eye.1),
            Point::new(512.0, 384.0),
            level,
        )
    }

    /// Plenty of texels per unit: no star is grown.
    const FINE: f32 = 8.0;

    #[test]
    fn the_same_pose_gives_the_same_stars_and_another_seed_others() {
        let one = pose(1, (300.0, -200.0), 32);
        assert_eq!(stars(&one, LOGICAL, FINE), stars(&one, LOGICAL, FINE));
        assert_ne!(
            stars(&one, LOGICAL, FINE),
            stars(&pose(2, (300.0, -200.0), 32), LOGICAL, FINE)
        );
    }

    #[test]
    fn moving_the_eye_away_and_back_gives_the_same_stars() {
        let here = stars(&pose(7, (10.0, 20.0), 32), LOGICAL, FINE);
        let _away = stars(&pose(7, (5000.0, -900.0), 32), LOGICAL, FINE);
        assert_eq!(stars(&pose(7, (10.0, 20.0), 32), LOGICAL, FINE), here);
    }

    #[test]
    fn parallax_is_one_on_the_plane_and_falls_with_depth() {
        assert_eq!(parallax(0.0), 1.0);
        let mut last = parallax(0.0);
        for depth in [1.0, 10.0, 1000.0, 5000.0, 20000.0] {
            assert!(parallax(depth) < last, "{depth}");
            last = parallax(depth);
        }
        let factors: Vec<f32> = LAYERS.iter().map(|layer| parallax(layer.depth)).collect();
        assert!(
            factors.windows(2).all(|pair| pair[0] < pair[1]),
            "{factors:?}"
        );
    }

    #[test]
    fn on_the_plane_a_point_lands_where_the_world_camera_puts_it() {
        let seen = pose(1, (300.0, -200.0), 32);
        assert_eq!(project([300.0, -200.0, 0.0], &seen), seen.anchor);
        assert_eq!(
            project([310.0, -200.0, 0.0], &seen),
            Point::new(seen.anchor.x + 10.0, seen.anchor.y)
        );
    }

    #[test]
    fn behind_the_plane_a_point_moves_by_its_parallax_of_the_eyes_move() {
        let depth = 2000.0;
        let from = project([50.0, 60.0, depth], &pose(1, (0.0, 0.0), 32));
        let to = project([50.0, 60.0, depth], &pose(1, (120.0, -80.0), 32));
        let scale = parallax(depth);
        assert!((to.x - (from.x - 120.0 * scale)).abs() < 1e-3);
        assert!((to.y - (from.y + 80.0 * scale)).abs() < 1e-3);
    }

    /// Layer `layer`'s stars at `eye`, then at `eye + delta`: every star
    /// well inside the first view is in the second, moved by
    /// −delta × the layer's parallax.
    fn moves_by_its_parallax(layer: usize, eye: (f32, f32), delta: (f32, f32)) -> f32 {
        let before = layer_stars(&pose(3, eye, 32), layer, LOGICAL, FINE);
        let after = layer_stars(
            &pose(3, (eye.0 + delta.0, eye.1 + delta.1), 32),
            layer,
            LOGICAL,
            FINE,
        );
        let scale = parallax(LAYERS[layer].depth);
        let (dx, dy) = (-delta.0 * scale, -delta.1 * scale);
        let inside = |p: Point| {
            p.x > 100.0
                && p.x < LOGICAL.w as f32 - 100.0
                && p.y > 100.0
                && p.y < LOGICAL.h as f32 - 100.0
        };
        let mut checked = 0;
        for star in before.iter().filter(|star| inside(star.centre)) {
            let moved = Point::new(star.centre.x + dx, star.centre.y + dy);
            assert!(
                after.iter().any(|other| {
                    (other.centre.x - moved.x).abs() < 1e-2
                        && (other.centre.y - moved.y).abs() < 1e-2
                        && other.radius == star.radius
                        && other.color == star.color
                }),
                "layer {layer}: {star:?} not moved to {moved:?}"
            );
            checked += 1;
        }
        assert!(checked > 0, "layer {layer} checked no star");
        dx.hypot(dy)
    }

    #[test]
    fn moving_the_eye_moves_each_layer_by_its_parallax_near_ones_further() {
        let moved: Vec<f32> = (0..LAYERS.len())
            .map(|layer| moves_by_its_parallax(layer, (-37.0, 905.0), (60.0, -40.0)))
            .collect();
        assert!(moved.windows(2).all(|pair| pair[0] < pair[1]), "{moved:?}");
    }

    #[test]
    fn the_sky_is_busy_in_every_layer_and_overall() {
        let seen = pose(11, (0.0, 0.0), 32);
        let mut total = 0;
        for (layer, sky_layer) in LAYERS.iter().enumerate() {
            let count = layer_stars(&seen, layer, LOGICAL, FINE).len();
            let target = sky_layer.density as usize;
            assert!(
                count * 2 > target && count < target * 2,
                "layer {layer}: {count} stars for {target}"
            );
            total += count;
        }
        assert!(total > 300 && total < 800, "{total}");
        assert_eq!(stars(&seen, LOGICAL, FINE).len(), total);
    }

    #[test]
    fn the_far_layer_is_drawn_first() {
        let seen = pose(11, (0.0, 0.0), 32);
        let far = layer_stars(&seen, 0, LOGICAL, FINE);
        assert_eq!(stars(&seen, LOGICAL, FINE)[..far.len()], far[..]);
    }

    #[test]
    fn each_layers_stars_vary_in_size_brightness_and_tint_from_the_palette() {
        let seen = pose(5, (40.0, 90.0), 32);
        for (layer, sky_layer) in LAYERS.iter().enumerate() {
            let stars = layer_stars(&seen, layer, LOGICAL, FINE);
            let mut radii: Vec<f32> = stars.iter().map(|star| star.radius).collect();
            radii.dedup();
            assert!(radii.len() > 1, "layer {layer}");
            let mut tints = Vec::new();
            let mut brightnesses = Vec::new();
            for star in &stars {
                let (lo, hi) = sky_layer.radius;
                assert!((lo..=hi).contains(&star.radius), "{star:?}");
                // The palette's tints all have a channel at 1, so the
                // brightest channel is the brightness.
                let brightness = star.color.iter().copied().fold(0.0, f32::max);
                let (lo, hi) = sky_layer.brightness;
                assert!((lo..=hi).contains(&brightness), "{star:?}");
                let tint = star.color.map(|c| c / brightness);
                let from = TINTS.iter().position(|&(palette, _)| {
                    palette.iter().zip(tint).all(|(a, b)| (a - b).abs() < 1e-4)
                });
                assert!(from.is_some(), "{star:?}");
                tints.push(from);
                brightnesses.push(brightness);
            }
            tints.sort_unstable();
            tints.dedup();
            brightnesses.dedup();
            assert!(tints.len() > 1, "layer {layer}");
            assert!(brightnesses.len() > 1, "layer {layer}");
        }
    }

    #[test]
    fn every_tint_draw_is_one_of_the_palettes_by_weight() {
        let total: u32 = TINTS.iter().map(|&(_, weight)| weight).sum();
        let mut seen = [0; TINTS.len()];
        for draw in 0..u64::from(total) {
            let tint = tint(draw);
            let at = TINTS.iter().position(|&(palette, _)| palette == tint);
            seen[at.expect("a palette tint")] += 1;
        }
        assert_eq!(seen, TINTS.map(|(_, weight)| weight));
    }

    #[test]
    fn a_wide_view_generates_no_more_cells_than_the_bound() {
        let wide = LogicalSize { w: 4096, h: 768 };
        let (nx, ny) = cell_span(wide, STILL);
        assert_eq!(cell_bound(wide), nx * ny);
        // 4104 / 128 → 33, plus one; 776 / 128 → 7, plus one.
        assert_eq!((nx, ny), (34, 8));
        for (layer, sky_layer) in LAYERS.iter().enumerate() {
            let per_cell = (sky_layer.density * CELL * CELL / DENSITY_AREA).ceil() as usize;
            for eye in [(0.0, 0.0), (63.9, 64.1), (-1e6, 3e5)] {
                let count = layer_stars(&pose(2, eye, 32), layer, wide, FINE).len();
                assert!(count <= cell_bound(wide) * per_cell, "{layer} {eye:?}");
                assert!(count > 0, "{layer} {eye:?}");
            }
        }
    }

    #[test]
    fn stars_fill_the_whole_view_and_its_margin_only() {
        let wide = LogicalSize { w: 2048, h: 1024 };
        let stars = stars(&pose(4, (-3000.0, 77.0), 32), wide, FINE);
        let (mut min_x, mut max_x) = (f32::MAX, f32::MIN);
        let (mut min_y, mut max_y) = (f32::MAX, f32::MIN);
        for star in &stars {
            min_x = min_x.min(star.centre.x);
            max_x = max_x.max(star.centre.x);
            min_y = min_y.min(star.centre.y);
            max_y = max_y.max(star.centre.y);
        }
        assert!((-MARGIN..40.0).contains(&min_x), "{min_x}");
        assert!(max_x < 2048.0 + MARGIN && max_x > 2008.0, "{max_x}");
        assert!((-MARGIN..40.0).contains(&min_y), "{min_y}");
        assert!(max_y < 1024.0 + MARGIN && max_y > 984.0, "{max_y}");
    }

    #[test]
    fn murk_dims_every_star_and_level_zero_draws_none() {
        let clear = stars(&pose(9, (1.0, 2.0), 32), LOGICAL, FINE);
        let half = stars(&pose(9, (1.0, 2.0), 16), LOGICAL, FINE);
        assert_eq!(clear.len(), half.len());
        for (clear, half) in clear.iter().zip(&half) {
            assert_eq!(clear.centre, half.centre);
            assert_eq!(clear.radius, half.radius);
            assert_eq!(clear.color.map(|c| c / 2.0), half.color);
        }
        assert!(stars(&pose(9, (1.0, 2.0), 0), LOGICAL, FINE).is_empty());
        assert_eq!(stars(&pose(9, (1.0, 2.0), 40), LOGICAL, FINE), clear);
    }

    #[test]
    fn a_sky_seen_from_or_drawn_at_no_finite_point_draws_nothing() {
        for rush in [(0.0, 0.0), (30.0, 18.0), (-30.0, -18.0)] {
            for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
                let mut seen = pose(1, (bad, 0.0), 32).with_rush(Point::new(rush.0, rush.1));
                assert!(stars(&seen, LOGICAL, FINE).is_empty(), "{rush:?} {bad}");
                seen.eye = Point::new(0.0, bad);
                assert!(stars(&seen, LOGICAL, FINE).is_empty(), "{rush:?} {bad}");
                seen.eye = Point::new(0.0, 0.0);
                seen.anchor = Point::new(bad, 0.0);
                assert!(stars(&seen, LOGICAL, FINE).is_empty(), "{rush:?} {bad}");
                seen.anchor = Point::new(0.0, bad);
                assert!(stars(&seen, LOGICAL, FINE).is_empty(), "{rush:?} {bad}");
            }
        }
    }

    #[test]
    fn a_star_smaller_than_a_texel_is_grown_to_one_and_keeps_its_light() {
        let seen = pose(6, (12.0, 34.0), 32);
        let fine = stars(&seen, LOGICAL, FINE);
        let coarse = stars(&seen, LOGICAL, 0.25);
        assert_eq!(fine.len(), coarse.len());
        let light = |star: &StarInstance| star.radius * star.radius * star.color[0];
        for (fine, coarse) in fine.iter().zip(&coarse) {
            assert_eq!(fine.centre, coarse.centre);
            assert!(coarse.radius * 0.25 >= 1.0 - 1e-6, "{coarse:?}");
            assert!(
                (light(fine) - light(coarse)).abs() <= light(fine) * 1e-4,
                "{fine:?} {coarse:?}"
            );
        }
        // A star already a texel wide is left as it is.
        let unit = stars(&seen, LOGICAL, 1.0);
        for (fine, unit) in fine.iter().zip(&unit) {
            if fine.radius >= 1.0 {
                assert_eq!(fine, unit);
            } else {
                assert_eq!(unit.radius, 1.0);
            }
        }
        for odd in [0.0, -1.0, f32::NAN, f32::INFINITY] {
            assert_eq!(stars(&seen, LOGICAL, odd), fine, "{odd}");
        }
    }

    #[test]
    fn coverage_is_whole_at_the_centre_and_none_from_the_edge() {
        assert_eq!(coverage(0.0), 1.0);
        assert!(coverage(0.5) > 0.0 && coverage(0.5) < 1.0);
        assert!(coverage(0.25) > coverage(0.5));
        assert_eq!(coverage(1.0), 0.0);
        assert_eq!(coverage(2.0), 0.0);
    }

    #[test]
    fn the_hash_is_splitmix64s() {
        // SplitMix64's first two outputs from a state of 0.
        assert_eq!(mix(0), 0xE220_A839_7B1D_CDAF);
        assert_eq!(mix(0x9E37_79B9_7F4A_7C15), 0x6E78_9E6A_A1B9_65F4);
    }

    #[test]
    fn a_cells_stars_stand_where_its_draws_put_them_on_the_layer() {
        // Eye and anchor at the origin: the layer's point u is drawn at u.
        let seen = SkyPose::new(3, Point::new(0.0, 0.0), Point::new(0.0, 0.0), 32);
        // The far layer: a few stars a cell.
        let layer = 0;
        let shown = layer_stars(&seen, layer, LOGICAL, FINE);
        let per_cell = LAYERS[layer].density * CELL * CELL / DENSITY_AREA;
        let mut found = 0;
        for (cx, cy) in [(1, 1), (2, 1), (3, 2)] {
            let mut hashes = Draws::new(3, layer, cx, cy);
            let whole = per_cell.floor();
            let count = whole as u32 + u32::from(hashes.unit() < per_cell - whole);
            for _ in 0..count {
                let x = (cx as f32 + hashes.unit()) * CELL;
                let y = (cy as f32 + hashes.unit()) * CELL;
                let _ = (hashes.unit(), hashes.unit(), hashes.next());
                assert!(
                    shown
                        .iter()
                        .any(|star| (star.centre.x - x).abs() < 1e-3
                            && (star.centre.y - y).abs() < 1e-3),
                    "cell ({cx}, {cy}): no star at ({x}, {y})"
                );
                found += 1;
            }
        }
        assert!(found > 0);
        // Moving the anchor moves every star with it.
        let moved = layer_stars(
            &SkyPose {
                anchor: Point::new(30.0, -20.0),
                ..seen
            },
            layer,
            LOGICAL,
            FINE,
        );
        let inside = |p: Point| (100.0..900.0).contains(&p.x) && (100.0..600.0).contains(&p.y);
        for star in shown.iter().filter(|star| inside(star.centre)) {
            assert!(
                moved
                    .iter()
                    .any(|other| (other.centre.x - star.centre.x - 30.0).abs() < 1e-3
                        && (other.centre.y - star.centre.y + 20.0).abs() < 1e-3),
                "{star:?}"
            );
        }
    }

    #[test]
    fn stars_reach_into_the_margin_on_every_side_and_no_further() {
        let big = LogicalSize { w: 4096, h: 4096 };
        let all = stars(&pose(8, (123.0, -456.0), 32), big, FINE);
        let (w, h) = (big.w as f32, big.h as f32);
        assert!(
            all.iter()
                .any(|star| (-MARGIN..0.0).contains(&star.centre.x))
        );
        assert!(
            all.iter()
                .any(|star| (w..w + MARGIN).contains(&star.centre.x))
        );
        assert!(
            all.iter()
                .any(|star| (-MARGIN..0.0).contains(&star.centre.y))
        );
        assert!(
            all.iter()
                .any(|star| (h..h + MARGIN).contains(&star.centre.y))
        );
        assert!(
            all.iter()
                .all(|star| (-MARGIN..w + MARGIN).contains(&star.centre.x)
                    && (-MARGIN..h + MARGIN).contains(&star.centre.y))
        );
    }

    #[test]
    fn a_layer_shows_its_density_on_average() {
        let big = LogicalSize { w: 4096, h: 4096 };
        let area = 4096.0 * 4096.0;
        for (layer, sky_layer) in LAYERS.iter().enumerate() {
            let count = layer_stars(&pose(12, (0.0, 0.0), 32), layer, big, FINE).len() as f32;
            let expected = sky_layer.density * area / DENSITY_AREA;
            assert!(
                (count - expected).abs() < expected * 0.08,
                "layer {layer}: {count} for {expected}"
            );
        }
    }

    #[test]
    fn the_cells_a_view_spans_count_its_margin_both_ways() {
        // 121 + 8 is just over a cell: two, plus one.
        assert_eq!(cell_span(LogicalSize { w: 121, h: 120 }, STILL), (3, 2));
        // A trail grows the span by its length on its own axis.
        assert_eq!(
            cell_span(LogicalSize { w: 121, h: 120 }, Point::new(-128.0, 9.0)),
            (4, 3)
        );
    }

    #[test]
    fn no_layer_past_the_last_has_stars() {
        assert!(layer_stars(&pose(1, (0.0, 0.0), 32), LAYERS.len(), LOGICAL, FINE).is_empty());
    }

    const STILL: Point = Point::new(0.0, 0.0);

    /// A rush along a jump heading, down and to the right.
    const RUSH: Point = Point::new(30.0, 18.0);

    #[test]
    fn a_sky_that_does_not_rush_has_round_stars() {
        let seen = pose(4, (250.0, -90.0), 32);
        assert_eq!(seen.rush, STILL);
        let all = stars(&seen, LOGICAL, FINE);
        assert!(!all.is_empty());
        assert!(all.iter().all(|star| star.trail == STILL));
        assert_eq!(stars(&seen.with_rush(STILL), LOGICAL, FINE), all);
    }

    #[test]
    fn each_stars_trail_is_the_rush_by_its_layers_parallax_near_ones_longer() {
        let seen = pose(4, (250.0, -90.0), 32).with_rush(RUSH);
        let mut lengths = Vec::new();
        for (layer, sky_layer) in LAYERS.iter().enumerate() {
            let scale = parallax(sky_layer.depth);
            let shown = layer_stars(&seen, layer, LOGICAL, FINE);
            assert!(!shown.is_empty(), "layer {layer}");
            for star in &shown {
                assert_eq!(
                    star.trail,
                    Point::new(RUSH.x * scale, RUSH.y * scale),
                    "layer {layer}"
                );
            }
            lengths.push(shown[0].trail.x.hypot(shown[0].trail.y));
        }
        assert!(
            lengths.windows(2).all(|pair| pair[0] < pair[1]),
            "{lengths:?}"
        );
    }

    #[test]
    fn a_rushing_sky_keeps_every_round_skys_star_and_streaks_it() {
        let still = pose(4, (250.0, -90.0), 32);
        let rushing = stars(&still.with_rush(RUSH), LOGICAL, FINE);
        for star in stars(&still, LOGICAL, FINE) {
            assert!(
                rushing.iter().any(|other| other.centre == star.centre
                    && other.radius == star.radius
                    && other.color == star.color
                    && other.trail != STILL),
                "{star:?}"
            );
        }
    }

    /// Whether the segment from `star`'s centre along its trail reaches
    /// into the view of `logical` size grown by the margin.
    fn reaches_in(star: &StarInstance, logical: LogicalSize) -> bool {
        let (right, bottom) = (logical.w as f32 + MARGIN, logical.h as f32 + MARGIN);
        (0..=1000).any(|step| {
            let along = step as f32 / 1000.0;
            let at = Point::new(
                star.centre.x + star.trail.x * along,
                star.centre.y + star.trail.y * along,
            );
            (-MARGIN..=right).contains(&at.x) && (-MARGIN..=bottom).contains(&at.y)
        })
    }

    #[test]
    fn a_streak_entering_from_off_the_view_is_kept_and_none_lies_wholly_outside() {
        for rush in [
            Point::new(120.0, 72.0),
            Point::new(-400.0, 90.0),
            Point::new(0.0, -700.0),
        ] {
            let all = stars(&pose(4, (250.0, -90.0), 32).with_rush(rush), LOGICAL, FINE);
            let outside = |star: &&StarInstance| {
                !(-MARGIN..LOGICAL.w as f32 + MARGIN).contains(&star.centre.x)
                    || !(-MARGIN..LOGICAL.h as f32 + MARGIN).contains(&star.centre.y)
            };
            assert!(all.iter().filter(outside).count() > 0, "{rush:?}");
            for star in &all {
                assert!(reaches_in(star, LOGICAL), "{rush:?}: {star:?}");
            }
        }
    }

    #[test]
    fn a_rushing_sky_keeps_every_star_whose_streak_reaches_into_the_view() {
        // Eyes that put the view's edges at many places in their cells, so
        // some trails reach back over a cell's edge and some don't.
        for step in 0..16 {
            let along = step as f32 * 37.0;
            keeps_every_reaching_star((250.0 + along, -90.0 - along * 1.3));
        }
    }

    /// Seen from `eye`, a rushing sky keeps every star whose streak reaches
    /// into the view, each layer.
    fn keeps_every_reaching_star(eye: (f32, f32)) {
        // The same sky seen over a view grown far past any trail, its
        // stars taken back to this view's coordinates.
        const GROWN: f32 = 512.0;
        let big = LogicalSize {
            w: LOGICAL.w + 2 * GROWN as u32,
            h: LOGICAL.h + 2 * GROWN as u32,
        };
        let wide = SkyPose::new(
            4,
            Point::new(eye.0, eye.1),
            Point::new(512.0 + GROWN, 384.0 + GROWN),
            32,
        );
        for rush in [
            Point::new(120.0, 72.0),
            Point::new(-400.0, 90.0),
            Point::new(60.0, -300.0),
        ] {
            let seen = pose(4, eye, 32).with_rush(rush);
            for (layer, sky_layer) in LAYERS.iter().enumerate() {
                let scale = parallax(sky_layer.depth);
                let trail = Point::new(rush.x * scale, rush.y * scale);
                let kept = layer_stars(&seen, layer, LOGICAL, FINE);
                let expected: Vec<StarInstance> = layer_stars(&wide, layer, big, FINE)
                    .into_iter()
                    .map(|star| StarInstance {
                        centre: Point::new(star.centre.x - GROWN, star.centre.y - GROWN),
                        trail,
                        ..star
                    })
                    .filter(|star| reaches_in(star, LOGICAL))
                    .collect();
                assert!(!expected.is_empty(), "{rush:?} layer {layer}");
                for star in &expected {
                    assert!(
                        kept.iter()
                            .any(|other| (other.centre.x - star.centre.x).abs() < 1e-2
                                && (other.centre.y - star.centre.y).abs() < 1e-2),
                        "{eye:?} {rush:?} layer {layer}: {star:?} lost"
                    );
                }
            }
        }
    }

    #[test]
    fn a_rushing_sky_generates_no_more_cells_than_the_rushing_bound() {
        let wide = LogicalSize { w: 4096, h: 768 };
        assert!(rushing_cell_bound(wide) > cell_bound(wide));
        for (layer, sky_layer) in LAYERS.iter().enumerate() {
            let per_cell = (sky_layer.density * CELL * CELL / DENSITY_AREA).ceil() as usize;
            for rush in [RUSH, Point::new(-1e6, 3e5), Point::new(f32::MAX, -f32::MAX)] {
                let seen = pose(2, (63.9, 64.1), 32).with_rush(rush);
                let count = layer_stars(&seen, layer, wide, FINE).len();
                assert!(
                    count <= rushing_cell_bound(wide) * per_cell,
                    "{layer} {rush:?}"
                );
                assert!(count > 0, "{layer} {rush:?}");
            }
        }
    }

    #[test]
    fn a_trail_reaches_at_most_across_the_view_along_the_rush() {
        let seen = pose(2, (0.0, 0.0), 32).with_rush(Point::new(-1e9, 5e8));
        for star in stars(&seen, LOGICAL, FINE) {
            assert!(star.trail.x.abs() <= LOGICAL.w as f32 + 1e-3, "{star:?}");
            assert!(star.trail.y.abs() <= LOGICAL.h as f32 + 1e-3, "{star:?}");
            assert!(star.trail.x < 0.0 && star.trail.y > 0.0, "{star:?}");
            assert!((star.trail.x / star.trail.y + 2.0).abs() < 1e-3, "{star:?}");
        }
    }

    #[test]
    fn a_rush_that_is_not_finite_gives_round_stars() {
        let seen = pose(4, (250.0, -90.0), 32);
        let round = stars(&seen, LOGICAL, FINE);
        for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            for rush in [Point::new(bad, 1.0), Point::new(1.0, bad)] {
                assert_eq!(
                    stars(&seen.with_rush(rush), LOGICAL, FINE),
                    round,
                    "{rush:?}"
                );
            }
        }
    }

    #[test]
    fn a_rushing_stars_radius_still_grows_to_a_texel() {
        let seen = pose(6, (12.0, 34.0), 32).with_rush(RUSH);
        let coarse = stars(&seen, LOGICAL, 0.25);
        assert!(!coarse.is_empty());
        assert!(coarse.iter().all(|star| star.radius * 0.25 >= 1.0 - 1e-6));
    }

    #[test]
    fn a_streak_with_no_trail_is_lit_as_a_round_star() {
        for (x, y) in [(0.0, 0.0), (1.0, 0.5), (-2.0, 1.5), (3.0, 3.0)] {
            let offset = Point::new(x, y);
            assert_eq!(
                streak_coverage(offset, STILL, 2.5),
                coverage(x.hypot(y) / 2.5),
                "{offset:?}"
            );
        }
    }

    #[test]
    fn a_streak_is_lit_along_its_trail_and_not_past_its_ends() {
        let trail = Point::new(20.0, 0.0);
        let radius = 2.0;
        // Wholly lit along the segment, its ends included.
        for x in [0.0, 5.0, 10.0, 20.0] {
            assert_eq!(
                streak_coverage(Point::new(x, 0.0), trail, radius),
                1.0,
                "{x}"
            );
        }
        // Off the segment by its distance, beside it or past an end.
        assert_eq!(
            streak_coverage(Point::new(10.0, 1.0), trail, radius),
            coverage(0.5)
        );
        assert_eq!(
            streak_coverage(Point::new(-1.0, 0.0), trail, radius),
            coverage(0.5)
        );
        assert_eq!(
            streak_coverage(Point::new(21.0, 0.0), trail, radius),
            coverage(0.5)
        );
        for off in [
            Point::new(-2.0, 0.0),
            Point::new(22.5, 0.0),
            Point::new(10.0, 2.0),
            Point::new(10.0, -3.0),
        ] {
            assert_eq!(streak_coverage(off, trail, radius), 0.0, "{off:?}");
        }
    }
}

/// How the [`Renderer`](crate::Renderer) batches a sky.
#[cfg(test)]
mod batch_tests {
    use nova_data::graphics::Image;
    use nova_galaxy_media::planet::{Image as Layer, Surface};
    use nova_view::geometry::Bounds;
    use nova_view::{
        Color, DrawCommand, DrawList, ImageKey, ImageKind, ModelKey, ModelPose, Orientation,
        PlanetKey, PlanetPose,
    };

    use super::*;
    use crate::Renderer;
    use crate::gpu::{Batch, Frame};
    use crate::images::{ImageError, ImageSource};
    use crate::model::Mesh;
    use crate::planet_source::LoadedPlanet;
    use crate::recording::{NoImages, RecordingGpu};
    use crate::render_scale::RenderScale;
    use crate::upscale::WorldResolution;
    use crate::viewport::Viewport;

    const SMALL: LogicalSize = LogicalSize { w: 256, h: 192 };

    /// The logical space at half a pixel a unit: few enough that the
    /// target's texels grow the stars, and a logical-sized world's don't.
    fn viewport() -> Viewport {
        Viewport::new(SMALL, (128, 96), 0.5)
    }

    #[test]
    fn the_texels_these_tests_draw_at_give_different_stars() {
        let [half, whole, eighth] = [0.5, 1.0, 0.125].map(|t| stars(&sky(), SMALL, t));
        assert_ne!(half, whole);
        assert_ne!(half, eighth);
        assert_ne!(whole, eighth);
    }

    fn sky() -> SkyPose {
        SkyPose::new(21, Point::new(40.0, -70.0), Point::new(128.0, 96.0), 32)
    }

    fn frame_of(
        renderer: &mut Renderer<NoImages>,
        list: &DrawList,
        gpu: &mut RecordingGpu,
    ) -> Frame {
        renderer.render(list, &viewport(), gpu);
        gpu.submits()
            .last()
            .map(|frame| (*frame).clone())
            .expect("a frame")
    }

    fn kinds(frame: &Frame) -> Vec<&'static str> {
        frame.batches.iter().map(Batch::kind).collect()
    }

    #[test]
    fn a_sky_is_one_batch_of_the_cores_stars_at_the_targets_texels() {
        let mut list = DrawList::new();
        list.sky(sky());
        let frame = frame_of(
            &mut Renderer::new(NoImages),
            &list,
            &mut RecordingGpu::new(),
        );
        let expected = stars(&sky(), SMALL, 0.5);
        assert!(!expected.is_empty());
        assert_eq!(frame.batches, [Batch::Sky(expected.clone())]);
        // An overlay sky keeps the target's texels whatever the world's.
        let mut gpu = RecordingGpu::answering(WorldResolution::Logical);
        let frame = frame_of(&mut Renderer::new(NoImages), &list, &mut gpu);
        assert_eq!(frame.batches, [Batch::Sky(expected)]);
    }

    #[test]
    fn a_world_sky_takes_the_world_textures_texels() {
        let mut list = DrawList::new();
        list.world(|list| {
            list.sky(sky());
        });
        let mut gpu = RecordingGpu::answering(WorldResolution::Logical);
        let frame = frame_of(&mut Renderer::new(NoImages), &list, &mut gpu);
        assert_eq!(frame.world, [0]);
        assert_eq!(frame.batches, [Batch::Sky(stars(&sky(), SMALL, 1.0))]);
        // At a quarter of the 128-wide backing, a 32-texel world: an
        // eighth of a texel a unit.
        let mut renderer = Renderer::new(NoImages);
        renderer.set_render_scale(RenderScale::Percent(25));
        let frame = frame_of(&mut renderer, &list, &mut RecordingGpu::new());
        assert_eq!(frame.batches, [Batch::Sky(stars(&sky(), SMALL, 0.125))]);
        // Drawn at the target's resolution, a world sky has its texels.
        let frame = frame_of(
            &mut Renderer::new(NoImages),
            &list,
            &mut RecordingGpu::new(),
        );
        assert_eq!(frame.batches, [Batch::Sky(stars(&sky(), SMALL, 0.5))]);
    }

    #[test]
    fn a_sky_in_the_worlds_tail_takes_the_targets_texels() {
        let mut list = DrawList::new();
        list.world(|list| {
            list.sky(sky());
            list.start_world_tail();
            list.sky(sky());
        });
        let mut gpu = RecordingGpu::answering(WorldResolution::Logical);
        let frame = frame_of(&mut Renderer::new(NoImages), &list, &mut gpu);
        assert_eq!(
            frame.batches,
            [
                Batch::Sky(stars(&sky(), SMALL, 1.0)),
                Batch::Sky(stars(&sky(), SMALL, 0.5))
            ]
        );
    }

    #[test]
    fn a_sky_on_a_widened_target_takes_the_smaller_axiss_texels() {
        // A widened view of a 305x200 window is 1171x768 logical: 305/1171
        // across, more than the 200/768 down. Of a 301x200 window it is
        // 1156x768: 301/1156 across, less than 200/768 down.
        for (window, across_is_larger) in [((305, 200), true), ((301, 200), false)] {
            let viewport = Viewport::for_shape(nova_view::ViewShape::Widened, window, 1.0);
            let logical = viewport.logical();
            let (across, down) = viewport.axis_scales().expect("a content area");
            assert_eq!(
                across > down,
                across_is_larger,
                "{across} across, {down} down"
            );
            let (smaller, larger) = if across_is_larger {
                (down, across)
            } else {
                (across, down)
            };
            let expected = stars(&sky(), logical, smaller);
            assert!(!expected.is_empty());
            assert_ne!(expected, stars(&sky(), logical, larger));
            let mut list = DrawList::new();
            list.sky(sky());
            list.world(|list| {
                list.start_world_tail();
                list.sky(sky());
            });
            let mut gpu = RecordingGpu::answering(WorldResolution::Logical);
            Renderer::new(NoImages).render(&list, &viewport, &mut gpu);
            let frame = gpu.submits().last().map(|frame| (*frame).clone());
            assert_eq!(
                frame.expect("a frame").batches,
                [Batch::Sky(expected.clone()), Batch::Sky(expected)],
                "in a {window:?} window"
            );
        }
    }

    #[test]
    fn a_sky_splits_from_its_neighbours_in_list_order() {
        let mut renderer = Renderer::new(NoImages);
        renderer.add_model(ModelKey(1), Mesh::cube([Color::WHITE; 6]));
        let mut list = DrawList::new();
        list.dot(Point::new(4.0, 4.0), 2.0, Color::WHITE)
            .sky(sky())
            .sky(sky())
            .dot(Point::new(4.0, 4.0), 2.0, Color::WHITE)
            .model(
                ModelKey(1),
                ModelPose {
                    center: Point::new(30.0, 30.0),
                    orientation: Orientation::IDENTITY,
                    scale: 4.0,
                },
            )
            .sky(sky());
        let frame = frame_of(&mut renderer, &list, &mut RecordingGpu::new());
        assert_eq!(
            kinds(&frame),
            ["solid", "sky", "sky", "solid", "models", "sky"]
        );
    }

    /// One white 4x4 sprite, `rlëD` 1.
    struct OneSprite;

    impl ImageSource for OneSprite {
        fn frames(&self, kind: ImageKind, id: i16) -> Result<Vec<Image>, ImageError> {
            match (kind, id) {
                (ImageKind::Rled, 1) => Ok(vec![
                    Image::from_rgba(4, 4, [255; 64].to_vec()).expect("4x4"),
                ]),
                _ => Err(ImageError::Missing),
            }
        }
    }

    /// A bare 4 by 2 grey planet.
    fn planet() -> LoadedPlanet {
        LoadedPlanet {
            surface: Surface {
                surface: Layer {
                    width: 4,
                    height: 2,
                    rgba: vec![9; 32],
                },
                clouds: None,
                rings: None,
                atmosphere: None,
                day: 100.0,
                tilt: 0.0,
            },
        }
    }

    #[test]
    fn a_planet_before_a_sky_and_a_sprite_after_it_keep_their_places() {
        let mut renderer = Renderer::new(OneSprite);
        renderer.add_planet(PlanetKey(1), planet());
        let mut list = DrawList::new();
        list.planet(
            PlanetKey(1),
            PlanetPose {
                center: Point::new(20.0, 20.0),
                radius: 5.0,
                time: 0.0,
            },
        )
        .sky(sky())
        .sprite(ImageKey::sprite(1, 0), Point::new(4.0, 4.0), Color::WHITE);
        let mut gpu = RecordingGpu::new();
        renderer.render(&list, &viewport(), &mut gpu);
        let frame = gpu
            .submits()
            .last()
            .map(|frame| (*frame).clone())
            .expect("a frame");
        assert_eq!(kinds(&frame), ["planets", "sky", "sprites"]);
        assert_eq!(frame.batches[1], Batch::Sky(stars(&sky(), SMALL, 0.5)));
    }

    #[test]
    fn a_sky_that_draws_nothing_is_no_batch_and_a_clip_leaves_it_uncut() {
        let mut list = DrawList::new();
        list.sky(SkyPose { level: 0, ..sky() }).sky(SkyPose {
            eye: Point::new(f32::NAN, 0.0),
            ..sky()
        });
        let frame = frame_of(
            &mut Renderer::new(NoImages),
            &list,
            &mut RecordingGpu::new(),
        );
        assert_eq!(frame.batches, []);
        let mut clipped = DrawList::new();
        clipped.clipped(
            Bounds {
                min: Point::new(0.0, 0.0),
                max: Point::new(10.0, 10.0),
            },
            DrawCommand::Sky { sky: sky() },
        );
        let frame = frame_of(
            &mut Renderer::new(NoImages),
            &clipped,
            &mut RecordingGpu::new(),
        );
        assert_eq!(frame.batches, [Batch::Sky(stars(&sky(), SMALL, 0.5))]);
    }
}
