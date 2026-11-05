//! Stand-ins for a ship or stellar with no sprite: an [`Outline`] (a model
//! seen from above, or a ship's size box) turned and placed as a sprite
//! would be, and the [`disc`] and [`ring`] a stellar with no model draws.
//!
//! The new game has no sprites, so its data gives each ship type and
//! stellar a stand-in ([`StellarStandIn`]; a ship's is an outline), which
//! the view draws where it would draw the crossed box.
//! Everything is drawn as [`DrawCommand::Triangle`]s, in the list's layer.
//!
//! An outline's corners are in its own space: x to starboard, y to the
//! nose, as the data draws a ship nose up. [`draw`] turns it by a heading
//! as a sprite's frame is picked (degrees clockwise, 0 facing up the
//! screen, as `nova_sim_base::flight::facing` reads one) and multiplies
//! it by a scale: 1 for a ship, whose outline is in pixels already, and a
//! stellar's reach over the outline's for a stellar.
//!
//! [`DrawCommand::Triangle`]: crate::draw::DrawCommand::Triangle

use crate::color::Color;
use crate::draw::DrawList;
use crate::geometry::Point;

/// A shape seen from above, as triangles drawn in order, each in one
/// opaque colour. Its corners are `(x, y)` with x to starboard and y to
/// the nose.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Outline {
    /// The triangles, the first drawn first.
    pub triangles: Vec<OutlineTriangle>,
}

/// One filled triangle of an [`Outline`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OutlineTriangle {
    /// Its corners, `(x, y)` with x to starboard and y to the nose.
    pub corners: [Point; 3],
    /// Its colour.
    pub color: Color,
}

impl Outline {
    /// How far its farthest corner is from its centre (0 with no
    /// triangles): the radius it sweeps as it turns.
    #[must_use]
    pub fn reach(&self) -> f32 {
        self.triangles
            .iter()
            .flat_map(|t| t.corners)
            .fold(0.0, |reach, p| reach.max(p.x.hypot(p.y)))
    }
}

/// What a stellar with no sprite draws instead.
#[derive(Clone, Debug, PartialEq)]
pub enum StellarStandIn {
    /// Its model seen from above, fitted to the stellar's reach.
    Outline(Outline),
    /// A filled disc in this colour.
    Disc(Color),
    /// A ring in this colour, its centre empty: a hypergate.
    Ring(Color),
}

/// How many triangles go round a disc or a ring.
const SEGMENTS: u16 = 32;

/// A ring's inner radius, as a share of its outer.
const RING_INNER: f32 = 0.75;

/// Draws `outline` centred on `centre`, turned to `heading` (degrees
/// clockwise, 0 facing up the screen) and multiplied by `scale`: its
/// triangles in order, as [`DrawCommand::Triangle`]s.
///
/// [`DrawCommand::Triangle`]: crate::draw::DrawCommand::Triangle
pub fn draw(list: &mut DrawList, outline: &Outline, centre: Point, heading: f32, scale: f32) {
    let (sin, cos) = heading.to_radians().sin_cos();
    // The nose (+y) faces (sin, -cos) on a screen whose y is down, and
    // starboard (+x) a quarter turn clockwise of it, (cos, sin).
    let place = |p: Point| {
        Point::new(
            centre.x + scale * (p.x * cos + p.y * sin),
            centre.y + scale * (p.x * sin - p.y * cos),
        )
    };
    for triangle in &outline.triangles {
        list.triangle(triangle.corners.map(place), triangle.color);
    }
}

/// Draws a filled disc of `radius` round `centre` in `color`: a fan of
/// triangles whose outer corners lie on the circle.
pub fn disc(list: &mut DrawList, centre: Point, radius: f32, color: Color) {
    for (from, to) in round(centre, radius) {
        list.triangle([centre, from, to], color);
    }
}

/// Draws a ring of outer `radius` round `centre` in `color`, its inner
/// radius [`RING_INNER`] of it: the band between the two circles, as two
/// triangles a segment, leaving the centre empty.
pub fn ring(list: &mut DrawList, centre: Point, radius: f32, color: Color) {
    let outer = round(centre, radius);
    let inner = round(centre, radius * RING_INNER);
    for ((out_from, out_to), (in_from, in_to)) in outer.zip(inner) {
        list.triangle([out_from, out_to, in_to], color)
            .triangle([out_from, in_to, in_from], color);
    }
}

/// The frame size of a sprite holding `outline` at `scale` as it turns:
/// a square `ceil(2 × reach × scale)` on a side.
#[must_use]
pub fn extent(outline: &Outline, scale: f32) -> (u32, u32) {
    // A ship's reach in pixels: well within a `u32`.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let side = (2.0 * outline.reach() * scale).ceil() as u32;
    (side, side)
}

/// The circle of `radius` round `centre` as [`SEGMENTS`] chords, each
/// from one point on it to the next, going round once.
fn round(centre: Point, radius: f32) -> impl Iterator<Item = (Point, Point)> {
    let at = move |step: u16| {
        let (sin, cos) = (f32::from(step) * std::f32::consts::TAU / f32::from(SEGMENTS)).sin_cos();
        Point::new(centre.x + radius * cos, centre.y + radius * sin)
    };
    (0..SEGMENTS).map(move |step| (at(step), at(step + 1)))
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::draw::{DrawCommand, DrawLayer};

    const RED: Color = Color::rgba(255, 0, 0, 255);
    const BLUE: Color = Color::rgba(0, 0, 255, 255);

    fn at(x: f32, y: f32) -> Point {
        Point::new(x, y)
    }

    /// A dart: its nose 4 ahead, its tail corners 1 to port and starboard
    /// and 2 astern, in red; and a blue triangle at its tail.
    fn dart() -> Outline {
        Outline {
            triangles: vec![
                OutlineTriangle {
                    corners: [at(0.0, 4.0), at(1.0, -2.0), at(-1.0, -2.0)],
                    color: RED,
                },
                OutlineTriangle {
                    corners: [at(0.0, -2.0), at(1.0, -3.0), at(-1.0, -3.0)],
                    color: BLUE,
                },
            ],
        }
    }

    /// Each triangle drawn: its corners, rounded to thousandths, and its
    /// colour.
    fn triangles(list: &DrawList) -> Vec<([(f32, f32); 3], Color)> {
        let near = |v: f32| (v * 1000.0).round() / 1000.0;
        list.iter()
            .map(|command| match command {
                DrawCommand::Triangle { corners, color } => {
                    (corners.map(|p| (near(p.x), near(p.y))), *color)
                }
                other => panic!("a triangle: {other:?}"),
            })
            .collect()
    }

    #[test]
    fn at_heading_0_the_nose_points_up_the_screen_and_starboard_right() {
        let mut list = DrawList::new();
        draw(&mut list, &dart(), at(0.0, 0.0), 0.0, 1.0);
        assert_eq!(
            triangles(&list),
            [
                ([(0.0, -4.0), (1.0, 2.0), (-1.0, 2.0)], RED),
                ([(0.0, 2.0), (1.0, 3.0), (-1.0, 3.0)], BLUE),
            ]
        );
    }

    #[test]
    fn at_heading_90_the_nose_points_right() {
        let mut list = DrawList::new();
        draw(&mut list, &dart(), at(0.0, 0.0), 90.0, 1.0);
        assert_eq!(
            triangles(&list)[0],
            ([(4.0, 0.0), (-2.0, 1.0), (-2.0, -1.0)], RED)
        );
    }

    #[test]
    fn at_heading_180_the_nose_points_down() {
        let mut list = DrawList::new();
        draw(&mut list, &dart(), at(0.0, 0.0), 180.0, 1.0);
        assert_eq!(
            triangles(&list)[0],
            ([(0.0, 4.0), (-1.0, -2.0), (1.0, -2.0)], RED)
        );
    }

    #[test]
    fn the_corners_are_multiplied_by_the_scale_and_offset_by_the_centre() {
        let mut list = DrawList::new();
        draw(&mut list, &dart(), at(100.0, 50.0), 0.0, 2.5);
        assert_eq!(
            triangles(&list)[0],
            ([(100.0, 40.0), (102.5, 55.0), (97.5, 55.0)], RED)
        );
    }

    #[test]
    fn the_triangles_are_drawn_in_the_list_s_layer() {
        let mut list = DrawList::new();
        list.world(|list| draw(list, &dart(), at(0.0, 0.0), 0.0, 1.0));
        draw(&mut list, &dart(), at(0.0, 0.0), 0.0, 1.0);
        let layers: Vec<_> = list.layered().map(|(layer, _)| layer).collect();
        assert_eq!(
            layers,
            [
                DrawLayer::World,
                DrawLayer::World,
                DrawLayer::Overlay,
                DrawLayer::Overlay
            ]
        );
    }

    /// Each corner drawn, with its distance from `centre`.
    fn distances(list: &DrawList, centre: Point) -> Vec<f32> {
        list.iter()
            .flat_map(|command| match command {
                DrawCommand::Triangle { corners, .. } => corners.to_vec(),
                other => panic!("a triangle: {other:?}"),
            })
            .map(|p| (p.x - centre.x).hypot(p.y - centre.y))
            .collect()
    }

    #[test]
    fn a_disc_is_a_fan_reaching_its_radius_in_its_colour() {
        let centre = at(50.0, 60.0);
        let mut list = DrawList::new();
        disc(&mut list, centre, 32.0, RED);
        assert_eq!(list.len(), usize::from(SEGMENTS));
        let distances = distances(&list, centre);
        assert!(distances.iter().all(|&d| d <= 32.001), "{distances:?}");
        assert!(distances.iter().any(|&d| d > 31.999));
        assert!(
            list.iter().all(|command| matches!(
                command,
                DrawCommand::Triangle { corners, color } if corners[0] == centre && *color == RED
            )),
            "each from the centre"
        );
    }

    #[test]
    fn a_disc_goes_all_the_way_round() {
        let centre = at(0.0, 0.0);
        let mut list = DrawList::new();
        disc(&mut list, centre, 10.0, RED);
        let corners: Vec<Point> = list
            .iter()
            .flat_map(|command| match command {
                DrawCommand::Triangle { corners, .. } => vec![corners[1], corners[2]],
                _ => Vec::new(),
            })
            .collect();
        for (x, y) in [(10.0, 0.0), (0.0, 10.0), (-10.0, 0.0), (0.0, -10.0)] {
            assert!(
                corners
                    .iter()
                    .any(|p| (p.x - x).abs() < 1e-4 && (p.y - y).abs() < 1e-4),
                "reaches ({x}, {y})"
            );
        }
        // Each chord ends where the next starts.
        let chords: Vec<(Point, Point)> = list
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Triangle { corners, .. } => Some((corners[1], corners[2])),
                _ => None,
            })
            .collect();
        for pair in chords.windows(2) {
            assert_eq!(pair[0].1, pair[1].0);
        }
    }

    #[test]
    fn a_ring_is_a_band_that_leaves_its_centre_empty() {
        let centre = at(50.0, 60.0);
        let mut list = DrawList::new();
        ring(&mut list, centre, 32.0, BLUE);
        assert_eq!(list.len(), 2 * usize::from(SEGMENTS));
        let distances = distances(&list, centre);
        let inner = 32.0 * RING_INNER;
        assert!(
            distances.iter().all(|&d| d >= inner - 0.001 && d <= 32.001),
            "{distances:?}"
        );
        assert!(distances.iter().any(|&d| d < inner + 0.001));
        assert!(distances.iter().any(|&d| d > 31.999));
        assert!(list.iter().all(
            |command| matches!(command, DrawCommand::Triangle { color, .. } if *color == BLUE)
        ));
    }

    #[test]
    fn a_ring_s_band_covers_each_segment_once() {
        // The two triangles of each segment share a diagonal and cover
        // the quad between the circles: their areas add up to it.
        let mut list = DrawList::new();
        ring(&mut list, at(0.0, 0.0), 10.0, BLUE);
        let area = |[a, b, c]: [Point; 3]| {
            ((b.x - a.x) * (c.y - a.y) - (c.x - a.x) * (b.y - a.y)).abs() / 2.0
        };
        let total: f32 = list
            .iter()
            .map(|command| match command {
                DrawCommand::Triangle { corners, .. } => area(*corners),
                _ => 0.0,
            })
            .sum();
        // A 32-gon's annulus: n/2 × sin(2π/n) × (R² − r²).
        let n = f32::from(SEGMENTS);
        let expected = n / 2.0 * (std::f32::consts::TAU / n).sin() * (100.0 - 56.25);
        assert!(
            (total - expected).abs() < 0.01,
            "{total} against {expected}"
        );
    }

    #[test]
    fn reach_is_the_farthest_corner_from_the_centre() {
        assert_eq!(dart().reach(), 4.0);
        assert_eq!(Outline::default().reach(), 0.0);
        let wide = Outline {
            triangles: vec![OutlineTriangle {
                corners: [at(3.0, -4.0), at(0.0, 1.0), at(-1.0, 0.0)],
                color: RED,
            }],
        };
        assert_eq!(wide.reach(), 5.0);
    }

    #[test]
    fn the_extent_holds_the_outline_at_every_heading() {
        // Reach 4: 8 across, 20 at 2.5, and 3.1 × 2 rounds up to 7.
        assert_eq!(extent(&dart(), 1.0), (8, 8));
        assert_eq!(extent(&dart(), 2.5), (20, 20));
        let small = Outline {
            triangles: vec![OutlineTriangle {
                corners: [at(0.0, 3.1), at(0.0, 0.0), at(1.0, 0.0)],
                color: RED,
            }],
        };
        assert_eq!(extent(&small, 1.0), (7, 7));
    }
}
