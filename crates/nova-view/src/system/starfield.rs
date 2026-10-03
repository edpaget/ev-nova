//! The parallax starfield behind a system: three layers of dots, each
//! moving a fraction of the camera's motion, so the far stars drift slowly
//! and the near ones faster. A stellar moves with the camera at 1.
//!
//! The stars are procedural, deterministic and stateless. Each layer is
//! tiled into [`CELL_SIZE`]-unit cells, and each star is a `SplitMix64` hash
//! of (layer, cell x, cell y, star index) mapped to a point in its cell.
//! Only the cells overlapping the screen are generated (at most 5 x 4 per
//! layer), so the cost is bounded, moving away and back draws the same
//! stars, and nothing needs a random number generator.
//!
//! A star at layer point `q` is drawn at `VIEW_CENTER + q - offset`, where
//! the layer's offset is the camera's centre times the layer's factor.

use std::ops::RangeInclusive;

use super::camera::{Camera, VIEW_CENTER, VIEW_SIZE};
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

/// The most columns and rows of cells that can overlap the screen, which
/// is 4 x 3 cells: one more each way when it straddles cell edges.
const MAX_COLUMNS: usize = 5;
const MAX_ROWS: usize = 4;

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

/// The columns and rows of the cells overlapping the screen, for a layer
/// offset by `offset`: at most [`MAX_COLUMNS`] x [`MAX_ROWS`].
fn visible_cells(offset: Point) -> (RangeInclusive<i32>, RangeInclusive<i32>) {
    let (half_width, half_height) = (VIEW_SIZE.0 / 2.0, VIEW_SIZE.1 / 2.0);
    // The cells from the one holding `low` to the one holding `high`.
    let cells =
        |low: f32, high: f32| (low / CELL_SIZE).floor() as i32..=(high / CELL_SIZE).floor() as i32;
    (
        cells(offset.x - half_width, offset.x + half_width),
        cells(offset.y - half_height, offset.y + half_height),
    )
}

/// Every star on screen, layer by layer, far first.
fn stars(camera: &Camera) -> Vec<Star> {
    let mut stars = Vec::new();
    for (index, layer) in LAYERS.iter().enumerate() {
        let offset = layer_offset(camera, layer);
        let (columns, rows) = visible_cells(offset);
        // Never more cells than can overlap the screen, so the work per
        // frame is bounded whatever the camera does.
        for y in rows.take(MAX_ROWS) {
            for x in columns.clone().take(MAX_COLUMNS) {
                for point in cell_stars(index, x, y) {
                    let screen = Point::new(
                        VIEW_CENTER.x + (point.x - offset.x),
                        VIEW_CENTER.y + (point.y - offset.y),
                    );
                    let on_screen = (0.0..=VIEW_SIZE.0).contains(&screen.x)
                        && (0.0..=VIEW_SIZE.1).contains(&screen.y);
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

    #[test]
    fn at_most_5_by_4_cells_can_overlap_the_screen() {
        assert_eq!((MAX_COLUMNS, MAX_ROWS), (5, 4));
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
            let (columns, rows) = visible_cells(offset);
            assert!(
                columns.clone().count() <= MAX_COLUMNS,
                "{offset:?}: {columns:?}"
            );
            assert!(rows.clone().count() <= MAX_ROWS, "{offset:?}: {rows:?}");
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
            for (layer, &n) in per_layer.iter().enumerate() {
                assert!(n > 0, "layer {layer} at ({x}, {y})");
                assert!(
                    n <= (MAX_COLUMNS * MAX_ROWS) as u32 * LAYERS[layer].stars_per_cell,
                    "{n} on layer {layer}"
                );
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
