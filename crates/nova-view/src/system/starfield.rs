//! The parallax starfield behind a system: three layers of dots, each
//! moving a fraction of the camera's motion, so the far stars drift slowly
//! and the near ones faster. A stellar moves with the camera at 1.
//!
//! The stars are procedural, deterministic and stateless. Each layer is
//! tiled into [`CELL_SIZE`]-unit cells, and each star is a `SplitMix64` hash
//! of (layer, cell x, cell y, star index) mapped to a point in its cell.
//! Only the cells overlapping the camera's view are generated, at most
//! ceil(view / [`CELL_SIZE`]) + 1 on each axis (5 x 4 per layer in Nova's
//! own 1024 x 768), so the cost is bounded by the view, moving away and
//! back draws the same stars, and nothing needs a random number generator.
//! There is no cap on the view's width: a widened flight view is covered
//! edge to edge.
//!
//! A star at layer point `q` is drawn at `anchor + q - offset`, where the
//! anchor is the camera's ([`Camera::anchor`]) and the layer's offset is
//! the camera's centre times the layer's factor.
//!
//! This stands in for the original's sprite starfield (`_RandomizeStars`
//! @0x0b1b5, `_HandleStars` @0x2f886 in `EV Nova.app`), filed as
//! rdm:phase/display-modes-fidelity/phase-2-original-starfield.

use std::ops::RangeInclusive;

use super::camera::Camera;
use crate::{Color, DrawList, Point};

/// One layer of stars.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Layer {
    /// How far the layer moves for each unit the camera moves.
    pub factor: f32,
    /// Its stars' colour.
    pub color: Color,
    /// Its stars' size.
    pub dot_size: f32,
    /// How many stars each cell holds.
    pub stars_per_cell: u32,
}

/// The layers, far to near: fainter, smaller and denser at the back.
pub const LAYERS: [Layer; 3] = [
    Layer {
        factor: 0.25,
        color: Color::rgba(96, 96, 96, 255),
        dot_size: 1.0,
        stars_per_cell: 4,
    },
    Layer {
        factor: 0.5,
        color: Color::rgba(160, 160, 160, 255),
        dot_size: 1.0,
        stars_per_cell: 3,
    },
    Layer {
        factor: 0.75,
        color: Color::rgba(224, 224, 224, 255),
        dot_size: 2.0,
        stars_per_cell: 2,
    },
];

/// The side of a layer's square cells, in layer units.
pub const CELL_SIZE: f32 = 256.0;

/// The most columns and rows of cells that can overlap a view `view`
/// logical units in size: as many as fit across it, rounded up, and one
/// more each way when it straddles cell edges.
fn cell_counts(view: (f32, f32)) -> (usize, usize) {
    let count = |length: f32| (length / CELL_SIZE).ceil() as usize + 1;
    (count(view.0), count(view.1))
}

/// `SplitMix64`'s increment, the golden gamma.
const GAMMA: u64 = 0x9E37_79B9_7F4A_7C15;

/// How far `layer` has moved: the camera's centre times its factor.
#[must_use]
pub fn layer_offset(camera: &Camera, layer: &Layer) -> Point {
    let center = camera.center();
    Point::new(center.x * layer.factor, center.y * layer.factor)
}

/// Draws every layer's stars that are on screen, the far layer first.
pub fn draw(list: &mut DrawList, camera: &Camera) {
    for star in stars(camera) {
        let layer = &LAYERS[star.layer];
        list.dot(star.screen, layer.dot_size, layer.color);
    }
}

/// One star on screen.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Star {
    /// The index of its layer in [`LAYERS`].
    layer: usize,
    /// Where it is in its layer.
    point: Point,
    /// Where it is drawn.
    screen: Point,
}

/// The columns and rows of the cells overlapping a view `view` logical
/// units in size, for a layer offset by `offset` and drawn about the
/// screen point `anchor`: the layer points from `offset − anchor` to
/// `offset + (view − anchor)` on each axis, at most [`cell_counts`].
fn visible_cells(
    offset: Point,
    anchor: Point,
    view: (f32, f32),
) -> (RangeInclusive<i32>, RangeInclusive<i32>) {
    // The cells from the one holding `low` to the one holding `high`.
    let cells =
        |low: f32, high: f32| (low / CELL_SIZE).floor() as i32..=(high / CELL_SIZE).floor() as i32;
    (
        cells(offset.x - anchor.x, offset.x + (view.0 - anchor.x)),
        cells(offset.y - anchor.y, offset.y + (view.1 - anchor.y)),
    )
}

/// Every star on screen, layer by layer, far first.
fn stars(camera: &Camera) -> Vec<Star> {
    let mut stars = Vec::new();
    let view = camera.view_size();
    let anchor = camera.anchor();
    let (max_columns, max_rows) = cell_counts(view);
    for (index, layer) in LAYERS.iter().enumerate() {
        let offset = layer_offset(camera, layer);
        let (columns, rows) = visible_cells(offset, anchor, view);
        // Never more cells than can overlap the view, so the work per
        // frame is bounded whatever the camera does.
        for y in rows.take(max_rows) {
            for x in columns.clone().take(max_columns) {
                for point in cell_stars(index, x, y) {
                    let screen = Point::new(
                        anchor.x + (point.x - offset.x),
                        anchor.y + (point.y - offset.y),
                    );
                    let on_screen =
                        (0.0..=view.0).contains(&screen.x) && (0.0..=view.1).contains(&screen.y);
                    if on_screen {
                        stars.push(Star {
                            layer: index,
                            point,
                            screen,
                        });
                    }
                }
            }
        }
    }
    stars
}

/// The stars of cell (`x`, `y`) of layer `layer`, as layer points.
fn cell_stars(layer: usize, x: i32, y: i32) -> Vec<Point> {
    // The cell's seed, from its layer and coordinates (as their bits).
    let seed = mix(GAMMA.wrapping_mul(layer as u64 + 1));
    let seed = mix(seed ^ u64::from(x as u32));
    let seed = mix(seed ^ u64::from(y as u32));
    let (left, top) = (x as f32 * CELL_SIZE, y as f32 * CELL_SIZE);
    (0..LAYERS[layer].stars_per_cell)
        .map(|index| {
            let hash = mix(seed ^ GAMMA.wrapping_mul(u64::from(index) + 1));
            // Two 16-bit fractions of the cell, in steps of 1/256 unit:
            // exact in an f32 added to any cell edge within 2^16 units.
            let fraction = |bits: u64| (bits & 0xFFFF) as f32 / 65_536.0 * CELL_SIZE;
            Point::new(left + fraction(hash >> 48), top + fraction(hash >> 16))
        })
        .collect()
}

/// `SplitMix64`'s finaliser: a bijective mix of `z`'s bits.
fn mix(z: u64) -> u64 {
    let z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    let z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use super::*;
    use crate::DrawCommand;
    use crate::geometry::Bounds;
    use crate::system::camera::{Anchor, VIEW_CENTER, VIEW_SIZE};

    fn at(x: f32, y: f32) -> Point {
        Point::new(x, y)
    }

    /// A camera free to go anywhere near the system, moved to (`x`, `y`).
    fn camera_at(x: f32, y: f32) -> Camera {
        let limits = Bounds {
            min: at(-20_000.0, -20_000.0),
            max: at(20_000.0, 20_000.0),
        };
        let mut camera = Camera::new(limits);
        camera.move_by(x, y);
        camera
    }

    fn drawn(camera: &Camera) -> DrawList {
        let mut list = DrawList::new();
        draw(&mut list, camera);
        list
    }

    fn key(p: Point) -> (u32, u32) {
        (p.x.to_bits(), p.y.to_bits())
    }

    #[test]
    fn mix_matches_splitmix64s_reference_outputs_for_seed_0() {
        // SplitMix64 seeded with 0 adds the gamma before each mix.
        assert_eq!(mix(GAMMA), 0xE220_A839_7B1D_CDAF);
        assert_eq!(mix(GAMMA.wrapping_mul(2)), 0x6E78_9E6A_A1B9_65F4);
        assert_eq!(mix(GAMMA.wrapping_mul(3)), 0x06C4_5D18_8009_454F);
        assert_eq!(mix(0), 0);
    }

    #[test]
    fn the_layers_go_from_far_and_faint_to_near_and_bright() {
        let factors: Vec<f32> = LAYERS.iter().map(|l| l.factor).collect();
        assert_eq!(factors, [0.25, 0.5, 0.75]);
        let greys: Vec<u8> = LAYERS.iter().map(|l| l.color.r).collect();
        assert_eq!(greys, [96, 160, 224]);
        let sizes: Vec<f32> = LAYERS.iter().map(|l| l.dot_size).collect();
        assert_eq!(sizes, [1.0, 1.0, 2.0]);
        let counts: Vec<u32> = LAYERS.iter().map(|l| l.stars_per_cell).collect();
        assert_eq!(counts, [4, 3, 2]);
    }

    #[test]
    fn each_layer_is_offset_by_its_factor_of_the_camera() {
        let camera = camera_at(1000.0, -400.0);
        let offsets: Vec<Point> = LAYERS.iter().map(|l| layer_offset(&camera, l)).collect();
        assert_eq!(
            offsets,
            [at(250.0, -100.0), at(500.0, -200.0), at(750.0, -300.0)]
        );
    }

    #[test]
    fn each_star_is_drawn_where_its_layer_offset_puts_it() {
        let camera = camera_at(1000.0, -400.0);
        let stars = stars(&camera);
        assert!(!stars.is_empty());
        for star in stars {
            let offset = layer_offset(&camera, &LAYERS[star.layer]);
            let expected = at(
                VIEW_CENTER.x + (star.point.x - offset.x),
                VIEW_CENTER.y + (star.point.y - offset.y),
            );
            assert_eq!(star.screen, expected, "{star:?}");
        }
    }

    /// Every star drawn, as (layer, layer point) to screen point.
    fn by_layer_point(camera: &Camera) -> BTreeMap<(usize, (u32, u32)), Point> {
        stars(camera)
            .into_iter()
            .map(|star| ((star.layer, key(star.point)), star.screen))
            .collect()
    }

    #[test]
    fn a_camera_move_moves_each_layers_stars_by_its_factor() {
        let before = by_layer_point(&camera_at(300.0, 200.0));
        let after = by_layer_point(&camera_at(400.0, 200.0));
        let mut matched = [0; 3];
        for (star, moved) in &after {
            let Some(was) = before.get(star) else {
                continue;
            };
            let factor = LAYERS[star.0].factor;
            assert!(
                (moved.x - (was.x - 100.0 * factor)).abs() < 1e-3,
                "layer {}: {was:?} to {moved:?}",
                star.0
            );
            assert_eq!(moved.y, was.y);
            matched[star.0] += 1;
        }
        assert!(matched.iter().all(|&n| n > 10), "{matched:?}");
    }

    #[test]
    fn the_same_camera_draws_the_same_stars_even_after_moving_away_and_back() {
        let camera = camera_at(-1234.0, 567.0);
        assert_eq!(drawn(&camera), drawn(&camera));
        let mut moved = camera;
        moved.move_by(5000.0, -3000.0);
        assert_ne!(drawn(&moved), drawn(&camera));
        moved.move_by(-5000.0, 3000.0);
        assert_eq!(drawn(&moved), drawn(&camera));
    }

    #[test]
    fn a_cells_stars_do_not_depend_on_the_camera() {
        for (x, y) in [(0.0, 0.0), (100.0, 50.0), (-300.0, 260.0)] {
            let camera = camera_at(x, y);
            let mut cells: BTreeMap<(usize, i32, i32), BTreeSet<(u32, u32)>> = BTreeMap::new();
            for star in stars(&camera) {
                let cell = (
                    star.layer,
                    (star.point.x / CELL_SIZE).floor() as i32,
                    (star.point.y / CELL_SIZE).floor() as i32,
                );
                cells.entry(cell).or_default().insert(key(star.point));
            }
            for ((layer, cx, cy), drawn) in cells {
                let all: BTreeSet<(u32, u32)> =
                    cell_stars(layer, cx, cy).into_iter().map(key).collect();
                assert!(drawn.is_subset(&all), "cell {cx}, {cy} of layer {layer}");
            }
        }
    }

    #[test]
    fn a_cells_stars_lie_in_it() {
        for (layer, spec) in LAYERS.iter().enumerate() {
            for (cx, cy) in [(0, 0), (-1, -1), (3, -7), (-40, 22)] {
                let points = cell_stars(layer, cx, cy);
                assert_eq!(points.len(), spec.stars_per_cell as usize);
                let (left, top) = (cx as f32 * CELL_SIZE, cy as f32 * CELL_SIZE);
                for p in points {
                    assert!(p.x >= left && p.x < left + CELL_SIZE, "{p:?} in {cx}");
                    assert!(p.y >= top && p.y < top + CELL_SIZE, "{p:?} in {cy}");
                }
            }
        }
    }

    #[test]
    fn neighbouring_cells_layers_and_indices_give_distinct_stars() {
        let mut seen = BTreeSet::new();
        let mut count = 0;
        for layer in 0..LAYERS.len() {
            for cx in -2..=2 {
                for cy in -2..=2 {
                    for p in cell_stars(layer, cx, cy) {
                        // In-cell offsets, so a hash that ignored the cell
                        // would repeat them.
                        let inside = at(p.x - cx as f32 * CELL_SIZE, p.y - cy as f32 * CELL_SIZE);
                        seen.insert(key(inside));
                        count += 1;
                    }
                }
            }
        }
        assert_eq!(count, 25 * (4 + 3 + 2));
        assert_eq!(seen.len(), count, "every star in its cell is distinct");
    }

    #[test]
    fn the_starfield_is_pinned() {
        // Two cells' stars, in 1/256 units (every coordinate is a whole
        // number of them), so a change to the hash is deliberate.
        let pinned = |layer, cx, cy| {
            cell_stars(layer, cx, cy)
                .into_iter()
                .map(|p| ((p.x * 256.0) as i32, (p.y * 256.0) as i32))
                .collect::<Vec<_>>()
        };
        assert_eq!(
            pinned(0, 0, 0),
            [(32424, 4018), (12425, 26369), (9798, 42167), (25277, 30448)]
        );
        assert_eq!(pinned(2, -1, 1), [(-33040, 115_633), (-25814, 117_018)]);
    }

    /// The stars drawn by a classic camera at `(x, y)`, folded into a
    /// count and a hash of every star's layer and screen point's bits.
    fn digest(x: f32, y: f32) -> (usize, u64) {
        let stars = stars(&camera_at(x, y));
        let hash = stars.iter().fold(0u64, |hash, star| {
            let (sx, sy) = key(star.screen);
            mix(hash ^ ((star.layer as u64) << 62) ^ (u64::from(sx) << 32) ^ u64::from(sy))
        });
        (stars.len(), hash)
    }

    #[test]
    fn a_classic_camera_draws_the_stars_it_drew_before_flight_widened() {
        // Taken from the starfield before the view size was an input.
        assert_eq!(digest(0.0, 0.0), (106, 987_672_271_802_097_832));
        assert_eq!(digest(128.0, 64.0), (107, 14_446_472_362_746_336_908));
        assert_eq!(digest(-5000.0, 7777.0), (103, 12_860_078_773_641_346_053));
        assert_eq!(digest(8250.5, -7124.25), (108, 7_707_136_679_628_693_101));
    }

    #[test]
    fn at_most_5_by_4_cells_can_overlap_novas_screen() {
        assert_eq!(cell_counts(VIEW_SIZE), (5, 4));
        assert_eq!(cell_counts((1365.0, 768.0)), (7, 4));
        assert_eq!(cell_counts((3000.0, 768.0)), (13, 4));
    }

    #[test]
    fn a_widened_camera_draws_stars_across_its_whole_width_with_no_cap() {
        for width in [1365.0, 3000.0] {
            let camera = Camera::centred_on_in(at(128.0, 64.0), (width, 768.0));
            let stars = stars(&camera);
            for star in &stars {
                let s = star.screen;
                assert!((0.0..=width).contains(&s.x), "{star:?} in {width}");
                assert!((0.0..=768.0).contains(&s.y), "{star:?}");
                let offset = layer_offset(&camera, &LAYERS[star.layer]);
                let expected = at(
                    camera.anchor().x + (star.point.x - offset.x),
                    camera.anchor().y + (star.point.y - offset.y),
                );
                assert_eq!(s, expected, "{star:?}");
            }
            let edge = width - 200.0;
            assert!(
                stars.iter().any(|star| star.screen.x > edge),
                "stars past {edge} in {width}"
            );
        }
        let wide = Camera::centred_on_in(at(128.0, 64.0), (1365.0, 768.0));
        assert!(stars(&wide).iter().any(|star| star.screen.x > 1024.0));
    }

    #[test]
    fn the_cells_generated_are_exactly_those_overlapping_the_screen() {
        let offsets = [
            at(0.0, 0.0),
            at(128.0, 64.0),
            at(768.0, -512.0),
            at(-3750.0, 5832.75),
            at(-5000.0, -4000.0),
            at(8250.5, -7124.25),
        ];
        for offset in offsets {
            let (columns, rows) = visible_cells(offset, VIEW_CENTER, VIEW_SIZE);
            let (max_columns, max_rows) = cell_counts(VIEW_SIZE);
            assert!(
                columns.clone().count() <= max_columns,
                "{offset:?}: {columns:?}"
            );
            assert!(rows.clone().count() <= max_rows, "{offset:?}: {rows:?}");
            // A cell overlaps the window when it starts at or before the
            // window's far edge and ends after its near edge.
            let overlaps = |cell: i32, centre: f32, half: f32| {
                let start = cell as f32 * CELL_SIZE;
                start <= centre + half && start + CELL_SIZE > centre - half
            };
            let near =
                |range: &std::ops::RangeInclusive<i32>| (range.start() - 3)..=(range.end() + 3);
            for x in near(&columns) {
                assert_eq!(
                    columns.contains(&x),
                    overlaps(x, offset.x, VIEW_SIZE.0 / 2.0),
                    "column {x} at {offset:?}"
                );
            }
            for y in near(&rows) {
                assert_eq!(
                    rows.contains(&y),
                    overlaps(y, offset.y, VIEW_SIZE.1 / 2.0),
                    "row {y} at {offset:?}"
                );
            }
        }
    }

    #[test]
    fn only_stars_on_screen_are_drawn_and_at_most_the_cells_on_screen_of_them() {
        for (x, y) in [(0.0, 0.0), (128.0, 64.0), (-5000.0, 7777.0)] {
            let camera = camera_at(x, y);
            let stars = stars(&camera);
            let mut per_layer = [0u32; 3];
            for star in &stars {
                let s = star.screen;
                assert!(
                    (0.0..=VIEW_SIZE.0).contains(&s.x) && (0.0..=VIEW_SIZE.1).contains(&s.y),
                    "{star:?}"
                );
                per_layer[star.layer] += 1;
            }
            let (max_columns, max_rows) = cell_counts(VIEW_SIZE);
            for (layer, &n) in per_layer.iter().enumerate() {
                assert!(n > 0, "layer {layer} at ({x}, {y})");
                assert!(
                    n <= (max_columns * max_rows) as u32 * LAYERS[layer].stars_per_cell,
                    "{n} on layer {layer}"
                );
            }
        }
    }

    /// Every star of every cell within a wide margin of `camera`'s centre
    /// that lands on screen, as (layer, layer point) to screen point.
    fn every_star_on_screen(camera: &Camera) -> BTreeMap<(usize, (u32, u32)), Point> {
        let (view, anchor) = (camera.view_size(), camera.anchor());
        let mut every = BTreeMap::new();
        for (index, layer) in LAYERS.iter().enumerate() {
            let offset = layer_offset(camera, layer);
            let (cx, cy) = (
                (offset.x / CELL_SIZE).floor() as i32,
                (offset.y / CELL_SIZE).floor() as i32,
            );
            for y in cy - 8..=cy + 8 {
                for x in cx - 8..=cx + 8 {
                    for point in cell_stars(index, x, y) {
                        let screen = at(
                            anchor.x + (point.x - offset.x),
                            anchor.y + (point.y - offset.y),
                        );
                        if (0.0..=view.0).contains(&screen.x) && (0.0..=view.1).contains(&screen.y)
                        {
                            every.insert((index, key(point)), screen);
                        }
                    }
                }
            }
        }
        every
    }

    #[test]
    fn a_play_area_anchored_camera_draws_every_star_on_screen_and_no_more() {
        for view in [VIEW_SIZE, (1365.0, 768.0), (1367.0, 769.0)] {
            for (x, y) in [
                (0.0, 0.0),
                (128.0, 64.0),
                (-5000.0, 7777.0),
                (300.0, -130.0),
            ] {
                for anchor in [Anchor::PlayAreaCentre, Anchor::ViewCentre] {
                    let camera = Camera::centred_on_in(at(x, y), view).with_anchor(anchor);
                    assert_eq!(
                        by_layer_point(&camera),
                        every_star_on_screen(&camera),
                        "{anchor:?} {view:?} at ({x}, {y})"
                    );
                }
            }
        }
    }

    #[test]
    fn the_far_layer_is_drawn_first_as_dots_in_each_layers_style() {
        let camera = camera_at(640.0, -320.0);
        let list = drawn(&camera);
        let stars = stars(&camera);
        assert_eq!(list.len(), stars.len());
        let layers: Vec<usize> = stars.iter().map(|s| s.layer).collect();
        let mut sorted = layers.clone();
        sorted.sort_unstable();
        assert_eq!(layers, sorted, "far first");
        for (command, star) in list.iter().zip(&stars) {
            let layer = &LAYERS[star.layer];
            assert_eq!(
                *command,
                DrawCommand::Dot {
                    center: star.screen,
                    size: layer.dot_size,
                    color: layer.color,
                }
            );
        }
    }
}
