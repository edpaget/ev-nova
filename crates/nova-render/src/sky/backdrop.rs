//! The sky's distant nebula backdrop ([`backdrop`]): large, faint,
//! clustered puffs on one far layer behind every star, drawn first, as
//! [`StarInstance`]s of no trail through the sky's own pipeline.
//!
//! # Its look
//!
//! A system's backdrop has a colour and an amount ([`Backdrop`]). Each
//! comes from the system's data when it sets one ([`BackdropLook`]),
//! and otherwise from the sky's seed ([`seeded`]): the colour one of
//! [`BACKDROP_HUES`] by weight, the amount `0.25 + 0.45·u²` for a draw
//! `u`, so most systems have a little and some a lot ([`resolved`]). The
//! colour is taken at full brightness ([`normalised`]): the clouds' own
//! light sets how bright they are, and a black colour draws none. An
//! amount of 0 is no backdrop.
//!
//! # Its form
//!
//! - **Depth.** One layer at [`BACKDROP_DEPTH`], parallax 0.025, half the
//!   far star layer's: flying 10,000 world units moves it 250 logical
//!   units.
//! - **Cells.** The layer is a grid of square cells [`BACKDROP_CELL`]
//!   logical units on a side. Each holds one cloud with the chance of the
//!   amount, its first draw, so a larger amount keeps every cloud a
//!   smaller one shows.
//! - **Clouds.** A cloud ([`Cloud`]) is [`PUFFS`] puffs round a centre,
//!   its extent `E` drawn from [`CLOUD_EXTENT`]. Each puff is centred
//!   within [`PUFF_SPREAD`]` · E` of the cloud's centre on each axis, its
//!   radius [`PUFF_RADIUS`]` · E` times `√(3 / n)` for a cloud of `n`
//!   puffs, so more puffs are smaller and every cloud covers about the
//!   same. Overlapping puffs of different sizes make a lumpy cloud rather
//!   than one round blob.
//! - **In its cell.** A puff reaches at most `(PUFF_SPREAD +
//!   PUFF_RADIUS.1) · E` = 0.95 E ≤ 247 units from its cloud's centre,
//!   under half the 512-unit cell, and the centre is drawn at least that
//!   far inside its cell, so every puff's square lies within its own
//!   cell: no two clouds overlap.
//! - **Light.** A cloud's light `L` is drawn from [`CLOUD_LIGHT`], and
//!   each of its `n` puffs adds the colour × `L / n` × the level / 32 at
//!   its centre, falling off by the stars' own
//!   [`coverage`](super::coverage). Where its puffs overlap a cloud
//!   reaches about two thirds of `L`, 0.15 to 0.2 of full.
//! - **The warp.** The backdrop doesn't streak: it is drawn with no trail
//!   whatever the rush, though it moves with the eye by its parallax. Its
//!   streak would be 2.5% of the near stars', a smear of a few units on
//!   puffs tens of units across, and it holds still while the stars
//!   streak, which reads as further away.
//! - **Footprint.** No puff is grown to a texel: the least radius,
//!   `0.4 · 180 · √(3/5)` ≈ 55.8 units, is far above one texel at the
//!   lowest render scale (4 units at 0.25 texels a unit), so [`backdrop`]
//!   takes no texels.
//!
//! # Its bounds, by construction
//!
//! - **Light.** Clouds don't overlap, so a point takes light from one
//!   cloud's puffs at most: `n` puffs of `L / n` each,
//!   [`coverage`](super::coverage) at most 1, the colour's brightest
//!   channel 1 and the level at most 32 give at most `L` ≤
//!   `CLOUD_LIGHT.1` = 0.3 of full in any channel
//!   ([`BACKDROP_LIGHT_BOUND`]).
//! - **Instances.** Only the cells the view can overlap are generated:
//!   `(⌈w / 512⌉ + 1) · (⌈h / 512⌉ + 1)` of them, as a cloud never
//!   reaches past its cell, with at most `PUFFS.1` = 5 puffs each
//!   ([`backdrop_instance_bound`]): 45 at 1024×768.
//! - **Shaded area.** The adapter shades each puff's square, `2r` on a
//!   side. A cloud's squares sum to at most `n · (2 · 0.6 · E ·
//!   √(3/n))²` = `4.32 · E²`, at most `4.32 · 260²` = 292,032 square
//!   units whatever `n` is. The 9 cells a 1024×768 view can overlap give
//!   at most 2,628,288, 3.342 of its 786,432; the 24 of a 2560×1080 view
//!   give at most 7,008,768, 2.535 of its 2,764,800. A point is also
//!   under at most 5 squares (one cloud's), so no view shades more than 5
//!   view areas ([`backdrop_area_bound`]). That worst case needs every
//!   cell's cloud at its largest and wholly in view; a sweep of seeds
//!   and eyes at amount 1 shades at most about a quarter of it, under
//!   0.9 view areas. At 1024×768 and the
//!   lowest render scale, 25% of a 1x backing, the worst case is about
//!   164,000 fragments of a 256×192 world texture, from a shader whose
//!   work per fragment is a star's.

use nova_view::{Point, SkyPose};

use super::look::{BackdropLook, SkyLook, normalised};
use super::{Draws, LAYERS, StarInstance, parallax, stars_of};
use crate::viewport::LogicalSize;

/// How far behind the play plane the backdrop is, in world units: twice
/// the far star layer's, so it moves 0.025 of the eye's move.
pub const BACKDROP_DEPTH: f32 = 39000.0;

/// The side of the backdrop's cells, in logical units.
pub const BACKDROP_CELL: f32 = 512.0;

/// The least and most extent of a cloud, in logical units.
pub const CLOUD_EXTENT: (f32, f32) = (180.0, 260.0);

/// The fewest and most puffs a cloud has.
pub const PUFFS: (u32, u32) = (3, 5);

/// How far from its cloud's centre a puff's centre may be on each axis,
/// as a share of the cloud's extent.
pub const PUFF_SPREAD: f32 = 0.35;

/// The least and most radius of a puff in a cloud of the fewest puffs, as
/// shares of the cloud's extent; a cloud of `n` puffs scales them by
/// `√(3 / n)`.
pub const PUFF_RADIUS: (f32, f32) = (0.4, 0.6);

/// The least and most light a cloud gives, shared among its puffs: its
/// most at any point, 0 to 1 of full.
pub const CLOUD_LIGHT: (f32, f32) = (0.24, 0.3);

/// The most light the backdrop adds at any point in any channel: one
/// cloud's most, as no two clouds overlap.
pub const BACKDROP_LIGHT_BOUND: f32 = CLOUD_LIGHT.1;

/// The least and most amount a seed gives a backdrop.
pub const SEEDED_AMOUNT: (f32, f32) = (0.25, 0.7);

/// The nebula hues a seed picks a backdrop's colour from, RGB 0 to 1 at
/// full brightness, each with its weight: mostly a deep reflection blue,
/// then teal, violet and rose, and a little dusty amber; no greens.
pub const BACKDROP_HUES: [([f32; 3], u32); 5] = [
    ([0.35, 0.5, 1.0], 5),
    ([0.3, 0.85, 1.0], 2),
    ([0.6, 0.35, 1.0], 2),
    ([1.0, 0.4, 0.6], 2),
    ([1.0, 0.68, 0.4], 1),
];

/// A backdrop's look once the data and the seed are settled.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Backdrop {
    /// The clouds' colour, RGB 0 to 1, its brightest channel 1.
    pub color: [f32; 3],
    /// The chance that each cell holds a cloud, 0 to 1.
    pub amount: f32,
}

/// The backdrop sky `seed` has with no data: one of [`BACKDROP_HUES`] by
/// weight, and an amount of `0.25 + 0.45·u²`.
#[must_use]
pub fn seeded(seed: u64) -> Backdrop {
    // Salted past the star layers, so no star changes.
    let mut draws = Draws::new(seed, LAYERS.len(), 0, 0);
    let color = hue(draws.next());
    let u = draws.unit();
    let (lo, hi) = SEEDED_AMOUNT;
    Backdrop {
        color,
        amount: lo + (hi - lo) * u * u,
    }
}

/// The hue `draw` picks from [`BACKDROP_HUES`] by their weights.
fn hue(draw: u64) -> [f32; 3] {
    let total: u32 = BACKDROP_HUES.iter().map(|&(_, weight)| weight).sum();
    let mut pick = (draw % u64::from(total)) as u32;
    for &(hue, weight) in &BACKDROP_HUES {
        if pick < weight {
            return hue;
        }
        pick -= weight;
    }
    BACKDROP_HUES[0].0
}

/// The backdrop sky `seed` draws in a system of backdrop look `look`:
/// each field the data sets, taken at full brightness or into 0 to 1,
/// and the seed's for the rest. `None` for a colour with no channel
/// above 0, which adds no light. An amount that is not finite is the
/// seed's.
#[must_use]
pub fn resolved(look: &BackdropLook, seed: u64) -> Option<Backdrop> {
    let seeded = seeded(seed);
    let color = match look.color {
        Some(color) => normalised(color)?,
        None => seeded.color,
    };
    let amount = look
        .amount
        .filter(|amount| amount.is_finite())
        .map_or(seeded.amount, |amount| amount.clamp(0.0, 1.0));
    Some(Backdrop { color, amount })
}

/// One puff of a cloud, placed from the cloud's centre.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Puff {
    /// Its centre, from the cloud's, in logical units.
    pub offset: Point,
    /// Its radius, in logical units.
    pub radius: f32,
}

/// One cloud of the backdrop, placed in its cell.
#[derive(Clone, Debug, PartialEq)]
pub struct Cloud {
    /// Its centre, from its cell's top-left corner, in logical units.
    pub centre: Point,
    /// Its extent, in logical units.
    pub extent: f32,
    /// Its light, shared among its puffs.
    pub light: f32,
    /// Its puffs.
    pub puffs: Vec<Puff>,
}

/// The cloud cell (`cx`, `cy`) of sky `seed`'s backdrop holds at amount
/// `amount`, if any.
#[must_use]
pub fn cloud(seed: u64, amount: f32, cx: i64, cy: i64) -> Option<Cloud> {
    // Salted past the star layers and the seed's own draws ([`seeded`]).
    let mut draws = Draws::new(seed, LAYERS.len() + 1, cx, cy);
    // The first draw, so a larger amount keeps every cloud.
    if draws.unit() >= amount {
        return None;
    }
    let extent = between(CLOUD_EXTENT, draws.unit());
    // Far enough inside the cell that no puff reaches out of it.
    let reach = (PUFF_SPREAD + PUFF_RADIUS.1) * extent;
    let mut inside = || reach + (BACKDROP_CELL - 2.0 * reach) * draws.unit();
    let centre = Point::new(inside(), inside());
    let count = PUFFS.0 + (draws.next() % u64::from(PUFFS.1 - PUFFS.0 + 1)) as u32;
    let light = between(CLOUD_LIGHT, draws.unit());
    let shrink = (PUFFS.0 as f32 / count as f32).sqrt();
    let spread = PUFF_SPREAD * extent;
    let puffs = (0..count)
        .map(|_| {
            let x = spread * (2.0 * draws.unit() - 1.0);
            let y = spread * (2.0 * draws.unit() - 1.0);
            let radius = between(PUFF_RADIUS, draws.unit()) * extent * shrink;
            Puff {
                offset: Point::new(x, y),
                radius,
            }
        })
        .collect();
    Some(Cloud {
        centre,
        extent,
        light,
        puffs,
    })
}

/// `range`'s value at `t` of the way from its least to its most.
fn between((lo, hi): (f32, f32), t: f32) -> f32 {
    lo + (hi - lo) * t
}

/// The backdrop's puffs `pose` sees in a view of `logical` size in a
/// system of look `look`, each a [`StarInstance`] of no trail: none at
/// level 0, with an eye or anchor that is not finite, or with no
/// backdrop. The rush leaves it as it is.
#[must_use]
pub fn backdrop(pose: &SkyPose, look: &SkyLook, logical: LogicalSize) -> Vec<StarInstance> {
    // An eye or anchor that is not finite needs no check of its own: it
    // puts every cell's corner at a NaN, so no puff passes the view test
    // below.
    if pose.level == 0 {
        return Vec::new();
    }
    let Some(Backdrop { color, amount }) = resolved(&look.backdrop, pose.seed) else {
        return Vec::new();
    };
    let level = f32::from(pose.level.min(32)) / 32.0;
    let scale = parallax(BACKDROP_DEPTH);
    // The layer's own coordinates, in logical units: the view's top-left
    // corner on it, the first cell it overlaps, and where that cell's
    // corner is drawn.
    let left = f64::from(pose.eye.x) * f64::from(scale) - f64::from(pose.anchor.x);
    let top = f64::from(pose.eye.y) * f64::from(scale) - f64::from(pose.anchor.y);
    let cell = f64::from(BACKDROP_CELL);
    let (cx0, cy0) = ((left / cell).floor(), (top / cell).floor());
    let corner = Point::new((cx0 * cell - left) as f32, (cy0 * cell - top) as f32);
    let (width, height) = (logical.w as f32, logical.h as f32);
    let (nx, ny) = cell_span(logical);
    let mut out = Vec::new();
    for j in 0..ny as i64 {
        for i in 0..nx as i64 {
            let cx = (cx0 as i64).wrapping_add(i);
            let cy = (cy0 as i64).wrapping_add(j);
            let Some(cloud) = cloud(pose.seed, amount, cx, cy) else {
                continue;
            };
            let light = cloud.light / cloud.puffs.len() as f32 * level;
            let x0 = corner.x + i as f32 * BACKDROP_CELL + cloud.centre.x;
            let y0 = corner.y + j as f32 * BACKDROP_CELL + cloud.centre.y;
            for puff in &cloud.puffs {
                let (x, y, r) = (x0 + puff.offset.x, y0 + puff.offset.y, puff.radius);
                // Kept when its square reaches into the view.
                if x + r > 0.0 && x - r < width && y + r > 0.0 && y - r < height {
                    out.push(StarInstance {
                        centre: Point::new(x, y),
                        radius: r,
                        color: color.map(|c| c * light),
                        trail: Point::new(0.0, 0.0),
                    });
                }
            }
        }
    }
    out
}

/// The cells across and down a view of `logical` size can overlap.
fn cell_span(logical: LogicalSize) -> (usize, usize) {
    let span = |extent: u32| (extent as f32 / BACKDROP_CELL).ceil() as usize + 1;
    (span(logical.w), span(logical.h))
}

/// The most puffs [`backdrop`] gives for a view of `logical` size: the
/// cells the view can overlap, [`PUFFS`]`.1` each.
#[must_use]
pub fn backdrop_instance_bound(logical: LogicalSize) -> usize {
    let (nx, ny) = cell_span(logical);
    nx * ny * PUFFS.1 as usize
}

/// The most area, in square logical units, that [`backdrop`]'s puffs'
/// squares shade inside a view of `logical` size, overlaps counted each
/// time: the least of the cells the view can overlap times a cloud's
/// most (`PUFFS.0 · (2 · PUFF_RADIUS.1 · CLOUD_EXTENT.1)²`, whatever its
/// count, as its radii shrink by `√(PUFFS.0 / n)`), and the view's area
/// times the most squares over one point ([`PUFFS`]`.1`).
#[must_use]
pub fn backdrop_area_bound(logical: LogicalSize) -> f32 {
    let (nx, ny) = cell_span(logical);
    let side = 2.0 * PUFF_RADIUS.1 * CLOUD_EXTENT.1;
    let cloud = PUFFS.0 as f32 * side * side;
    let deepest = PUFFS.1 as f32 * logical.w as f32 * logical.h as f32;
    ((nx * ny) as f32 * cloud).min(deepest)
}

/// The whole sky `pose` sees in a view of `logical` size in a system of
/// look `look`, drawn into a target of `texels_per_unit` texels per
/// logical unit: the backdrop ([`backdrop`]), then the stars
/// ([`stars_of`]).
#[must_use]
pub fn instances(
    pose: &SkyPose,
    look: &SkyLook,
    logical: LogicalSize,
    texels_per_unit: f32,
) -> Vec<StarInstance> {
    let mut all = backdrop(pose, look, logical);
    all.extend(stars_of(pose, look, logical, texels_per_unit));
    all
}

/// The most stars the layers give at twice the stars, in `cells` cells
/// each: the star bound the sky's bound tests share.
#[cfg(test)]
pub(super) fn star_bound(cells: usize) -> usize {
    use super::{CELL, DENSITY_AREA};
    LAYERS
        .iter()
        .map(|layer| {
            cells * ((2.0 * layer.density * CELL * CELL / DENSITY_AREA).floor() as usize + 1)
        })
        .sum()
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::super::{cell_bound, coverage, rushing_cell_bound};
    use super::*;
    use crate::viewport::LOGICAL;

    const WIDE: LogicalSize = LogicalSize { w: 2560, h: 1080 };

    fn pose(seed: u64, eye: (f32, f32)) -> SkyPose {
        SkyPose::new(seed, Point::new(eye.0, eye.1), Point::new(512.0, 384.0), 32)
    }

    fn with_backdrop(color: Option<[f32; 3]>, amount: Option<f32>) -> SkyLook {
        SkyLook {
            backdrop: BackdropLook { color, amount },
            ..SkyLook::default()
        }
    }

    /// A look whose every cell holds a cloud.
    fn full() -> SkyLook {
        with_backdrop(None, Some(1.0))
    }

    /// Eyes far apart, near the origin and far from it.
    const EYES: [(f32, f32); 6] = [
        (0.0, 0.0),
        (300.0, -200.0),
        (-7777.0, 1234.5),
        (1e5, -3e4),
        (-2.5e6, 9e5),
        (511.9, 512.1),
    ];

    #[test]
    fn the_same_pose_gives_the_same_puffs_and_another_seed_others() {
        let one = pose(1, (300.0, -200.0));
        let puffs = backdrop(&one, &full(), LOGICAL);
        assert!(!puffs.is_empty());
        assert_eq!(backdrop(&one, &full(), LOGICAL), puffs);
        assert_ne!(backdrop(&pose(2, (300.0, -200.0)), &full(), LOGICAL), puffs);
    }

    #[test]
    fn a_seeds_backdrop_is_one_of_the_hues_and_amounts_vary_across_seeds() {
        let (mut low, mut high) = (false, false);
        let mut hues = Vec::new();
        for seed in 0..400 {
            let seeded = seeded(seed);
            let hue = BACKDROP_HUES
                .iter()
                .position(|&(hue, _)| hue == seeded.color);
            hues.push(hue.unwrap_or_else(|| panic!("{seed}: {seeded:?}")));
            assert!(
                (SEEDED_AMOUNT.0..=SEEDED_AMOUNT.1).contains(&seeded.amount),
                "{seed}: {seeded:?}"
            );
            low |= seeded.amount < 0.35;
            high |= seeded.amount > 0.55;
            assert_eq!(seeded, super::seeded(seed), "the same every time");
        }
        assert!(low && high);
        hues.sort_unstable();
        hues.dedup();
        assert_eq!(hues.len(), BACKDROP_HUES.len(), "every hue picked");
        assert_eq!(SEEDED_AMOUNT, (0.25, 0.7));
    }

    #[test]
    fn a_seeds_amount_is_a_quarter_plus_the_square_of_its_draw() {
        // The least is 0.25 and the most below 0.7, the median well below
        // the middle, so most systems have a little.
        let mut amounts: Vec<f32> = (0..2000).map(|seed| seeded(seed).amount).collect();
        amounts.sort_by(f32::total_cmp);
        let median = amounts[amounts.len() / 2];
        assert!((0.33..0.38).contains(&median), "{median}");
        assert!(amounts[0] < 0.2501 && amounts[amounts.len() - 1] > 0.69);
    }

    #[test]
    fn every_hue_has_full_brightness_and_none_is_green() {
        for (hue, weight) in BACKDROP_HUES {
            assert_eq!(hue.iter().copied().fold(0.0, f32::max), 1.0, "{hue:?}");
            assert!(hue[1] < hue[0].max(hue[2]), "{hue:?} is green");
            assert!(weight > 0);
        }
        let total: u32 = BACKDROP_HUES.iter().map(|&(_, weight)| weight).sum();
        assert!(BACKDROP_HUES[0].1 * 3 > total, "mostly blue");
    }

    #[test]
    fn datas_colour_and_amount_each_override_the_seeds_alone() {
        let seed = 41;
        let seeded = seeded(seed);
        let backdrop = |color, amount| resolved(&BackdropLook { color, amount }, seed);
        assert_eq!(backdrop(None, None), Some(seeded));
        assert_eq!(
            backdrop(Some([0.0, 0.0, 0.5]), None),
            Some(Backdrop {
                color: [0.0, 0.0, 1.0],
                amount: seeded.amount
            })
        );
        assert_eq!(
            backdrop(None, Some(0.9)),
            Some(Backdrop {
                color: seeded.color,
                amount: 0.9
            })
        );
        assert_eq!(
            backdrop(Some([1.0, 0.5, 0.0]), Some(0.1)),
            Some(Backdrop {
                color: [1.0, 0.5, 0.0],
                amount: 0.1
            })
        );
        assert_eq!(backdrop(Some([0.0; 3]), Some(1.0)), None, "black");
        assert_eq!(backdrop(None, Some(3.0)).map(|b| b.amount), Some(1.0));
        assert_eq!(backdrop(None, Some(-1.0)).map(|b| b.amount), Some(0.0));
        assert_eq!(
            backdrop(None, Some(f32::NAN)).map(|b| b.amount),
            Some(seeded.amount)
        );
    }

    #[test]
    fn a_black_backdrop_or_none_of_it_draws_no_puff() {
        for eye in EYES {
            let seen = pose(5, eye);
            assert!(backdrop(&seen, &with_backdrop(None, Some(0.0)), LOGICAL).is_empty());
            assert!(backdrop(&seen, &with_backdrop(Some([0.0; 3]), Some(1.0)), LOGICAL).is_empty());
        }
    }

    #[test]
    fn at_amount_one_every_cell_holds_a_cloud_and_at_zero_none() {
        for (cx, cy) in [(0, 0), (-3, 9), (1 << 40, -(1 << 40)), (i64::MAX, i64::MIN)] {
            for seed in [0, 7, u64::MAX] {
                assert!(cloud(seed, 1.0, cx, cy).is_some(), "{seed} ({cx}, {cy})");
                assert!(cloud(seed, 0.0, cx, cy).is_none(), "{seed} ({cx}, {cy})");
            }
        }
    }

    #[test]
    fn a_cell_holds_a_cloud_the_amount_of_the_time_and_a_larger_amount_keeps_it() {
        for amount in [0.25, 0.4, 0.7] {
            let mut clouds = 0;
            for cy in -50..50 {
                for cx in -50..50 {
                    if let Some(cloud) = super::cloud(9, amount, cx, cy) {
                        clouds += 1;
                        assert_eq!(super::cloud(9, amount + 0.1, cx, cy), Some(cloud));
                    }
                }
            }
            let share = clouds as f32 / 10_000.0;
            assert!((share - amount).abs() < 0.02, "{amount}: {share}");
        }
    }

    #[test]
    fn each_cloud_has_its_puffs_round_its_centre_and_lies_in_its_cell() {
        let mut counts = Vec::new();
        let mut closest = f32::MAX;
        for cy in -20..20 {
            for cx in -20..20 {
                let cloud = cloud(3, 1.0, cx, cy).expect("a cloud");
                let n = cloud.puffs.len() as u32;
                assert!((PUFFS.0..=PUFFS.1).contains(&n), "{cloud:?}");
                counts.push(n);
                let e = cloud.extent;
                assert!((CLOUD_EXTENT.0..=CLOUD_EXTENT.1).contains(&e), "{cloud:?}");
                assert!(
                    (CLOUD_LIGHT.0..=CLOUD_LIGHT.1).contains(&cloud.light),
                    "{cloud:?}"
                );
                // The centre lies a cloud's farthest reach inside its cell
                // or more, and the clouds use all the room that leaves.
                let reach = (PUFF_SPREAD + PUFF_RADIUS.1) * e;
                for at in [cloud.centre.x, cloud.centre.y] {
                    let (near, far) = (at - reach, BACKDROP_CELL - reach - at);
                    assert!(near >= -1e-3 && far >= -1e-3, "{cloud:?}");
                    closest = closest.min(near.min(far));
                }
                let shrink = (PUFFS.0 as f32 / n as f32).sqrt();
                for puff in &cloud.puffs {
                    assert!(puff.offset.x.abs() <= PUFF_SPREAD * e, "{puff:?}");
                    assert!(puff.offset.y.abs() <= PUFF_SPREAD * e, "{puff:?}");
                    let (lo, hi) = (PUFF_RADIUS.0 * e * shrink, PUFF_RADIUS.1 * e * shrink);
                    assert!((lo..=hi).contains(&puff.radius), "{puff:?}");
                    // Its square lies in the cell.
                    for (at, offset) in [
                        (cloud.centre.x, puff.offset.x),
                        (cloud.centre.y, puff.offset.y),
                    ] {
                        assert!(at + offset - puff.radius >= 0.0, "{cloud:?}");
                        assert!(at + offset + puff.radius <= BACKDROP_CELL, "{cloud:?}");
                    }
                }
            }
        }
        assert!(closest < 2.0, "{closest}");
        counts.sort_unstable();
        counts.dedup();
        assert_eq!(counts, [3, 4, 5]);
    }

    #[test]
    fn a_cloud_reaches_no_further_than_half_a_cell_from_its_centre() {
        // What keeps clouds from overlapping, and the light bound with it.
        assert!((PUFF_SPREAD + PUFF_RADIUS.1) * CLOUD_EXTENT.1 < BACKDROP_CELL / 2.0);
    }

    #[test]
    fn moving_the_eye_moves_every_puff_by_the_backdrops_parallax() {
        let scale = parallax(BACKDROP_DEPTH);
        assert!((scale - 0.025).abs() < 1e-6, "{scale}");
        assert!(BACKDROP_DEPTH > LAYERS[0].depth);
        assert!(scale < parallax(LAYERS[0].depth));
        let before = backdrop(&pose(3, (-37.0, 905.0)), &full(), LOGICAL);
        let after = backdrop(&pose(3, (-37.0 + 2000.0, 905.0 - 1200.0)), &full(), LOGICAL);
        let (dx, dy) = (-2000.0 * scale, 1200.0 * scale);
        let inside = |p: Point| (100.0..900.0).contains(&p.x) && (100.0..650.0).contains(&p.y);
        let mut checked = 0;
        for puff in before.iter().filter(|puff| inside(puff.centre)) {
            assert!(
                after.iter().any(|other| {
                    (other.centre.x - puff.centre.x - dx).abs() < 1e-2
                        && (other.centre.y - puff.centre.y - dy).abs() < 1e-2
                        && other.radius == puff.radius
                        && other.color == puff.color
                }),
                "{puff:?} not moved"
            );
            checked += 1;
        }
        assert!(checked > 0);
    }

    #[test]
    fn a_cells_puffs_stand_where_its_cloud_puts_them() {
        // Eye and anchor at the origin: the layer's point u is drawn at u.
        let seen = SkyPose::new(3, Point::new(0.0, 0.0), Point::new(0.0, 0.0), 32);
        let shown = backdrop(&seen, &full(), LOGICAL);
        let hue = seeded(3).color;
        let mut found = 0;
        for (cx, cy) in [(0, 0), (1, 0), (1, 1)] {
            let cloud = cloud(3, 1.0, cx, cy).expect("a cloud");
            let n = cloud.puffs.len() as f32;
            for puff in &cloud.puffs {
                let x = cx as f32 * BACKDROP_CELL + cloud.centre.x + puff.offset.x;
                let y = cy as f32 * BACKDROP_CELL + cloud.centre.y + puff.offset.y;
                if y - puff.radius >= LOGICAL.h as f32 {
                    continue;
                }
                let expected = StarInstance {
                    centre: Point::new(x, y),
                    radius: puff.radius,
                    color: hue.map(|c| c * cloud.light / n),
                    trail: Point::new(0.0, 0.0),
                };
                assert!(
                    shown.iter().any(|other| {
                        (other.centre.x - x).abs() < 1e-3
                            && (other.centre.y - y).abs() < 1e-3
                            && other.radius == expected.radius
                            && (other.color.iter().zip(&expected.color))
                                .all(|(a, b)| (a - b).abs() < 1e-7)
                            && other.trail == expected.trail
                    }),
                    "({cx}, {cy}): no {expected:?}"
                );
                found += 1;
            }
        }
        assert!(found > 0);
    }

    #[test]
    fn murk_dims_the_puffs_and_level_zero_or_no_finite_point_draws_none() {
        let clear = backdrop(&pose(9, (1.0, 2.0)), &full(), LOGICAL);
        let half = backdrop(
            &SkyPose {
                level: 16,
                ..pose(9, (1.0, 2.0))
            },
            &full(),
            LOGICAL,
        );
        assert!(!clear.is_empty());
        assert_eq!(clear.len(), half.len());
        for (clear, half) in clear.iter().zip(&half) {
            assert_eq!((clear.centre, clear.radius), (half.centre, half.radius));
            assert_eq!(clear.color.map(|c| c / 2.0), half.color);
        }
        let at = |level| SkyPose {
            level,
            ..pose(9, (1.0, 2.0))
        };
        assert!(backdrop(&at(0), &full(), LOGICAL).is_empty());
        assert_eq!(backdrop(&at(40), &full(), LOGICAL), clear);
        for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            for (eye, anchor) in [
                ((bad, 0.0), (0.0, 0.0)),
                ((0.0, bad), (0.0, 0.0)),
                ((0.0, 0.0), (bad, 0.0)),
                ((0.0, 0.0), (0.0, bad)),
            ] {
                let seen = SkyPose::new(
                    9,
                    Point::new(eye.0, eye.1),
                    Point::new(anchor.0, anchor.1),
                    32,
                );
                assert!(backdrop(&seen, &full(), LOGICAL).is_empty(), "{seen:?}");
            }
        }
    }

    #[test]
    fn a_rushing_backdrop_is_the_still_ones() {
        for eye in EYES {
            let still = pose(4, eye);
            let puffs = backdrop(&still, &full(), LOGICAL);
            for rush in [(30.0, 18.0), (-1e6, 3e5), (f32::NAN, 1.0)] {
                let rushing = still.with_rush(Point::new(rush.0, rush.1));
                assert_eq!(backdrop(&rushing, &full(), LOGICAL), puffs, "{rush:?}");
            }
        }
    }

    #[test]
    fn no_puff_is_smaller_than_a_texel_at_the_lowest_render_scale() {
        let least = PUFF_RADIUS.0 * CLOUD_EXTENT.0 * (PUFFS.0 as f32 / PUFFS.1 as f32).sqrt();
        assert!(least > 1.0 / 0.25, "{least}");
        for (seed, eye) in (0..20).zip(EYES.iter().cycle()) {
            for puff in backdrop(&pose(seed, *eye), &full(), LOGICAL) {
                assert!(puff.radius >= least - 1e-3, "{puff:?}");
            }
        }
    }

    /// The light the backdrop `puffs` add at `at`, the brightest channel.
    fn light_at(puffs: &[StarInstance], at: Point) -> f32 {
        let mut sum = [0.0_f32; 3];
        for puff in puffs {
            let t = (at.x - puff.centre.x).hypot(at.y - puff.centre.y) / puff.radius;
            for (sum, c) in sum.iter_mut().zip(puff.color) {
                *sum += c * coverage(t);
            }
        }
        sum.into_iter().fold(0.0, f32::max)
    }

    #[test]
    fn the_backdrop_is_faint_at_every_point() {
        const { assert!(BACKDROP_LIGHT_BOUND <= 0.3) };
        let mut brightest = 0.0_f32;
        for (seed, eye) in (0..12).zip(EYES.iter().cycle()) {
            let puffs = backdrop(&pose(seed, *eye), &full(), LOGICAL);
            // Every puff gives at most its cloud's light over its fewest
            // puffs.
            for puff in &puffs {
                for c in puff.color {
                    assert!(c <= CLOUD_LIGHT.1 / PUFFS.0 as f32 + 1e-6, "{puff:?}");
                }
            }
            for y in (0..LOGICAL.h).step_by(8) {
                for x in (0..LOGICAL.w).step_by(8) {
                    let light = light_at(&puffs, Point::new(x as f32, y as f32));
                    assert!(light <= BACKDROP_LIGHT_BOUND + 1e-5, "({x}, {y}): {light}");
                    brightest = brightest.max(light);
                }
            }
        }
        // Faint, but there.
        assert!(brightest > 0.08, "{brightest}");
    }

    #[test]
    fn every_puff_reaching_the_view_is_kept_and_none_lies_wholly_outside() {
        const GROWN: f32 = 1024.0;
        let big = LogicalSize {
            w: LOGICAL.w + 2 * GROWN as u32,
            h: LOGICAL.h + 2 * GROWN as u32,
        };
        let (w, h) = (LOGICAL.w as f32, LOGICAL.h as f32);
        let reaches = |puff: &StarInstance| {
            puff.centre.x + puff.radius > 0.0
                && puff.centre.x - puff.radius < w
                && puff.centre.y + puff.radius > 0.0
                && puff.centre.y - puff.radius < h
        };
        let mut outside_edges = 0;
        for eye in EYES {
            let kept = backdrop(&pose(6, eye), &full(), LOGICAL);
            assert!(kept.iter().all(reaches), "{eye:?}");
            let wide = SkyPose {
                anchor: Point::new(512.0 + GROWN, 384.0 + GROWN),
                ..pose(6, eye)
            };
            let expected: Vec<StarInstance> = backdrop(&wide, &full(), big)
                .into_iter()
                .map(|puff| StarInstance {
                    centre: Point::new(puff.centre.x - GROWN, puff.centre.y - GROWN),
                    ..puff
                })
                .filter(reaches)
                .collect();
            assert!(!expected.is_empty());
            for puff in &expected {
                assert!(
                    kept.iter().any(|other| {
                        (other.centre.x - puff.centre.x).abs() < 1e-2
                            && (other.centre.y - puff.centre.y).abs() < 1e-2
                            && other.radius == puff.radius
                    }),
                    "{eye:?}: {puff:?} lost"
                );
            }
            assert_eq!(kept.len(), expected.len(), "{eye:?}");
            outside_edges += kept
                .iter()
                .filter(|puff| {
                    !(0.0..w).contains(&puff.centre.x) || !(0.0..h).contains(&puff.centre.y)
                })
                .count();
        }
        assert!(outside_edges > 0, "a puff centred off the view still shows");
    }

    #[test]
    fn the_sky_is_its_backdrop_then_its_stars() {
        let seen = pose(8, (40.0, 90.0)).with_rush(Point::new(30.0, 18.0));
        let look = SkyLook {
            stars: 1.5,
            tint: Some([1.0, 0.5, 0.0]),
            backdrop: BackdropLook {
                color: Some([0.2, 0.4, 1.0]),
                amount: Some(0.8),
            },
        };
        let backdrop = backdrop(&seen, &look, LOGICAL);
        let stars = stars_of(&seen, &look, LOGICAL, 0.5);
        assert!(!backdrop.is_empty() && !stars.is_empty());
        let all = instances(&seen, &look, LOGICAL, 0.5);
        assert_eq!(all[..backdrop.len()], backdrop[..]);
        assert_eq!(all[backdrop.len()..], stars[..]);
        // The default look draws the seeded backdrop under the engine's stars.
        let seen = pose(8, (40.0, 90.0));
        let plain = instances(&seen, &SkyLook::default(), LOGICAL, 0.5);
        let seeded = super::backdrop(&seen, &SkyLook::default(), LOGICAL);
        assert_eq!(
            plain[seeded.len()..],
            super::super::stars(&seen, LOGICAL, 0.5)[..]
        );
    }

    #[test]
    fn the_backdrops_instance_bound_is_its_cells_puffs() {
        // ⌈1024 / 512⌉ + 1 across, ⌈768 / 512⌉ + 1 down.
        assert_eq!(cell_span(LOGICAL), (3, 3));
        assert_eq!(backdrop_instance_bound(LOGICAL), 45);
        // ⌈2560 / 512⌉ + 1, ⌈1080 / 512⌉ + 1.
        assert_eq!(cell_span(WIDE), (6, 4));
        assert_eq!(backdrop_instance_bound(WIDE), 120);
        assert_eq!(cell_span(LogicalSize { w: 513, h: 1 }), (3, 2));
        for logical in [LOGICAL, WIDE] {
            for (seed, eye) in (0..30).zip(EYES.iter().cycle()) {
                let count = backdrop(&pose(seed, *eye), &full(), logical).len();
                assert!(
                    count <= backdrop_instance_bound(logical),
                    "{eye:?}: {count}"
                );
            }
        }
    }

    #[test]
    fn the_backdrops_area_bound_is_its_cells_clouds_or_its_deepest_overlap() {
        // Three puffs of 0.6 · 260 = 156 at most, or more puffs as much
        // smaller: 3 · 312².
        let cloud = 292_032.0;
        assert_eq!(backdrop_area_bound(LOGICAL), 9.0 * cloud);
        assert_eq!(backdrop_area_bound(WIDE), 24.0 * cloud);
        // A small view: five squares deep over all of it.
        let small = LogicalSize { w: 100, h: 50 };
        assert_eq!(backdrop_area_bound(small), 5.0 * 100.0 * 50.0);
        // Whatever its count, a cloud's squares sum to no more.
        for cy in -15..15 {
            for cx in -15..15 {
                let cloud_cells = super::cloud(2, 1.0, cx, cy).expect("a cloud");
                let squares: f32 = (cloud_cells.puffs.iter())
                    .map(|puff| 4.0 * puff.radius * puff.radius)
                    .sum();
                assert!(squares <= cloud + 1.0, "{cloud_cells:?}");
            }
        }
    }

    /// The area `instance`'s rectangle (its streak grown by its radius)
    /// covers inside a view of `logical` size.
    fn clipped_area(instance: &StarInstance, logical: LogicalSize) -> f32 {
        let span = |at: f32, trail: f32, extent: u32| {
            let lo = (at.min(at + trail) - instance.radius).max(0.0);
            let hi = (at.max(at + trail) + instance.radius).min(extent as f32);
            (hi - lo).max(0.0)
        };
        span(instance.centre.x, instance.trail.x, logical.w)
            * span(instance.centre.y, instance.trail.y, logical.h)
    }

    /// The most share of a 1024×768 or a 2560×1080 view the whole sky
    /// shades at the lowest render scale, with no rush: the backdrop's
    /// worst case, 3.342 view areas at 1024×768 (see the module docs),
    /// and the stars' at twice the density, each a square 8 units on a
    /// side, 0.118.
    const SKY_AREA_BOUND: f32 = 3.5;

    #[test]
    fn the_whole_sky_stays_within_its_bound_at_the_lowest_render_scale() {
        const LOWEST: f32 = 0.25;
        let look = SkyLook {
            stars: 2.0,
            ..full()
        };
        // Every star is grown to a texel, 4 units, at the lowest scale:
        // the still stars' squares are at most 8 units on a side.
        let star_square = (2.0 / LOWEST) * (2.0 / LOWEST);
        for logical in [LOGICAL, WIDE] {
            let view = logical.w as f32 * logical.h as f32;
            // By construction.
            let backdrop_most = backdrop_area_bound(logical);
            let stars_most = star_bound(cell_bound(logical)) as f32 * star_square;
            assert!(
                backdrop_most / view < 3.35,
                "{logical:?}: {}",
                backdrop_most / view
            );
            assert!(
                stars_most / view < 0.12,
                "{logical:?}: {}",
                stars_most / view
            );
            assert!(
                (backdrop_most + stars_most) / view <= SKY_AREA_BOUND,
                "{logical:?}"
            );
            for (seed, eye) in (0..24).zip(EYES.iter().cycle()) {
                for rush in [(0.0, 0.0), (30.0, 18.0), (-400.0, 90.0), (-1e6, 3e5)] {
                    let rush = Point::new(rush.0, rush.1);
                    let seen = pose(seed, *eye).with_rush(rush);
                    let all = instances(&seen, &look, logical, LOWEST);
                    let puffs = backdrop(&seen, &look, logical);
                    let star_cells = if rush == Point::new(0.0, 0.0) {
                        cell_bound(logical)
                    } else {
                        rushing_cell_bound(logical)
                    };
                    assert!(
                        all.len() <= star_bound(star_cells) + backdrop_instance_bound(logical),
                        "{logical:?} {eye:?} {rush:?}: {}",
                        all.len()
                    );
                    let shaded: f32 = puffs.iter().map(|puff| clipped_area(puff, logical)).sum();
                    assert!(shaded <= backdrop_most, "{logical:?} {eye:?}: {shaded}");
                    if rush == Point::new(0.0, 0.0) {
                        let whole: f32 = all.iter().map(|one| clipped_area(one, logical)).sum();
                        assert!(
                            whole <= SKY_AREA_BOUND * view,
                            "{logical:?} {eye:?}: {}",
                            whole / view
                        );
                    }
                }
            }
        }
    }
}
