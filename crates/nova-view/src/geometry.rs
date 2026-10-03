//! Points and rectangles in logical units.

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
    fn growing_moves_every_edge_out_by_the_margin() {
        let box_ = bounds((-2.0, -1.0), (3.0, 5.0));
        assert_eq!(box_.grown(10.0), bounds((-12.0, -11.0), (13.0, 15.0)));
        assert_eq!(box_.grown(0.0), box_);
        let point = bounds((4.0, 4.0), (4.0, 4.0));
        assert_eq!(point.grown(1.5), bounds((2.5, 2.5), (5.5, 5.5)));
    }
}
