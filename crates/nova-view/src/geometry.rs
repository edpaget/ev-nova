//! Points and rectangles in logical units.

use std::cmp::Ordering;

/// A point in logical units: x grows right, y grows down.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point {
    /// Horizontal position.
    pub x: f32,
    /// Vertical position.
    pub y: f32,
}

impl Point {
    /// The point (`x`, `y`).
    #[must_use]
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

/// An axis-aligned rectangle of points, edges included.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bounds {
    /// The top-left corner: the smallest x and y.
    pub min: Point,
    /// The bottom-right corner: the largest x and y.
    pub max: Point,
}

impl Bounds {
    /// The `width` x `height` rectangle whose top-left corner is
    /// `top_left`.
    #[must_use]
    pub fn at(top_left: Point, width: f32, height: f32) -> Self {
        Self {
            min: top_left,
            max: Point::new(top_left.x + width, top_left.y + height),
        }
    }

    /// The rectangle moved right by `by.x` and down by `by.y`.
    #[must_use]
    pub fn offset(&self, by: Point) -> Self {
        Self {
            min: Point::new(self.min.x + by.x, self.min.y + by.y),
            max: Point::new(self.max.x + by.x, self.max.y + by.y),
        }
    }

    /// The smallest rectangle holding every point, or `None` for none.
    pub fn around(points: impl IntoIterator<Item = Point>) -> Option<Self> {
        points.into_iter().fold(None, |bounds, p| {
            Some(match bounds {
                None => Self { min: p, max: p },
                Some(Self { min, max }) => Self {
                    min: Point::new(min.x.min(p.x), min.y.min(p.y)),
                    max: Point::new(max.x.max(p.x), max.y.max(p.y)),
                },
            })
        })
    }

    /// Its width.
    #[must_use]
    pub fn width(&self) -> f32 {
        self.max.x - self.min.x
    }

    /// Its height.
    #[must_use]
    pub fn height(&self) -> f32 {
        self.max.y - self.min.y
    }

    /// Its centre.
    #[must_use]
    pub fn center(&self) -> Point {
        Point::new(
            f32::midpoint(self.min.x, self.max.x),
            f32::midpoint(self.min.y, self.max.y),
        )
    }

    /// The point in the rectangle nearest `p`.
    #[must_use]
    pub fn clamp(&self, p: Point) -> Point {
        Point::new(
            p.x.clamp(self.min.x, self.max.x),
            p.y.clamp(self.min.y, self.max.y),
        )
    }

    /// Whether `p` is inside, edges included.
    #[must_use]
    pub fn contains(&self, p: Point) -> bool {
        (self.min.x..=self.max.x).contains(&p.x) && (self.min.y..=self.max.y).contains(&p.y)
    }

    /// The rectangle grown by `margin` on every side.
    #[must_use]
    pub fn grown(&self, margin: f32) -> Self {
        Self {
            min: Point::new(self.min.x - margin, self.min.y - margin),
            max: Point::new(self.max.x + margin, self.max.y + margin),
        }
    }
}

/// The point `t` of the way from `from` to `to`: exactly `from` at 0 and
/// `to` at 1.
pub(crate) fn lerp(from: Point, to: Point, t: f32) -> Point {
    let mix = |a: f32, b: f32| a * (1.0 - t) + b * t;
    Point::new(mix(from.x, to.x), mix(from.y, to.y))
}

/// Where `a` and `b` overlap, if they share any area.
pub(crate) fn intersection(a: Bounds, b: Bounds) -> Option<Bounds> {
    let min = Point::new(a.min.x.max(b.min.x), a.min.y.max(b.min.y));
    let max = Point::new(a.max.x.min(b.max.x), a.max.y.min(b.max.y));
    (min.x < max.x && min.y < max.y).then_some(Bounds { min, max })
}

/// The stretch of the segment from `from` to `to` inside `area`, edges
/// included, as fractions of the way along it (Liang-Barsky); `None` when
/// it misses or only touches it at a point. A segment of no length inside
/// is all of it.
pub(crate) fn span_in(from: Point, to: Point, area: Bounds) -> Option<(f32, f32)> {
    let (dx, dy) = (to.x - from.x, to.y - from.y);
    let mut span = (0.0_f32, 1.0_f32);
    for (p, q) in [
        (-dx, from.x - area.min.x),
        (dx, area.max.x - from.x),
        (-dy, from.y - area.min.y),
        (dy, area.max.y - from.y),
    ] {
        match p.partial_cmp(&0.0) {
            Some(Ordering::Less) => span.0 = span.0.max(q / p),
            Some(Ordering::Greater) => span.1 = span.1.min(q / p),
            // Parallel to this edge: in or out along its whole length.
            _ => {
                if q < 0.0 {
                    return None;
                }
            }
        }
    }
    (span.0 < span.1).then_some(span)
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    fn at(x: f32, y: f32) -> Point {
        Point::new(x, y)
    }

    fn bounds(min: (f32, f32), max: (f32, f32)) -> Bounds {
        Bounds {
            min: at(min.0, min.1),
            max: at(max.0, max.1),
        }
    }

    #[test]
    fn bounds_hold_every_point() {
        let points = [at(3.0, -1.0), at(-2.0, 5.0), at(1.0, 0.0)];
        let around = Bounds::around(points).expect("points");
        assert_eq!(around, bounds((-2.0, -1.0), (3.0, 5.0)));
        assert_eq!((around.width(), around.height()), (5.0, 6.0));
        assert_eq!(around.center(), at(0.5, 2.0));
        assert_eq!(
            Bounds::around([at(7.0, 8.0)]),
            Some(bounds((7.0, 8.0), (7.0, 8.0)))
        );
        assert_eq!(Bounds::around([]), None);
    }

    #[test]
    fn clamping_moves_a_point_to_the_nearest_edge_on_each_axis() {
        let box_ = bounds((-2.0, -1.0), (3.0, 5.0));
        assert_eq!(box_.clamp(at(0.0, 0.0)), at(0.0, 0.0));
        assert_eq!(box_.clamp(at(-9.0, 2.0)), at(-2.0, 2.0));
        assert_eq!(box_.clamp(at(9.0, -9.0)), at(3.0, -1.0));
        assert_eq!(box_.clamp(at(1.0, 9.0)), at(1.0, 5.0));
    }

    #[test]
    fn contains_includes_the_edges() {
        let box_ = bounds((-2.0, -1.0), (3.0, 5.0));
        for inside in [
            at(0.0, 0.0),
            at(-2.0, -1.0),
            at(3.0, 5.0),
            at(-2.0, 5.0),
            at(3.0, 2.0),
        ] {
            assert!(box_.contains(inside), "{inside:?}");
        }
        for outside in [
            at(-2.001, 0.0),
            at(3.001, 0.0),
            at(0.0, -1.001),
            at(0.0, 5.001),
        ] {
            assert!(!box_.contains(outside), "{outside:?}");
        }
    }

    #[test]
    fn a_rectangle_at_a_corner_has_its_size() {
        let box_ = Bounds::at(at(3.0, -4.0), 10.0, 20.0);
        assert_eq!(box_, bounds((3.0, -4.0), (13.0, 16.0)));
        assert_eq!((box_.width(), box_.height()), (10.0, 20.0));
    }

    #[test]
    fn offsetting_moves_both_corners() {
        let box_ = bounds((-2.0, -1.0), (3.0, 5.0));
        assert_eq!(
            box_.offset(at(10.0, 100.0)),
            bounds((8.0, 99.0), (13.0, 105.0))
        );
        assert_eq!(box_.offset(at(0.0, 0.0)), box_);
    }

    #[test]
    fn growing_moves_every_edge_out_by_the_margin() {
        let box_ = bounds((-2.0, -1.0), (3.0, 5.0));
        assert_eq!(box_.grown(10.0), bounds((-12.0, -11.0), (13.0, 15.0)));
        assert_eq!(box_.grown(0.0), box_);
        let point = bounds((4.0, 4.0), (4.0, 4.0));
        assert_eq!(point.grown(1.5), bounds((2.5, 2.5), (5.5, 5.5)));
    }
}
