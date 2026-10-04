//! Three-slice geometry: a rectangle cut into two fixed-length caps and a
//! stretched middle, as Nova draws its buttons and dialog frames.

use crate::geometry::{Bounds, Point};

/// The direction the slices run along.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis {
    /// Left cap, middle, right cap.
    Horizontal,
    /// Top cap, middle, bottom cap.
    Vertical,
}

/// One slice of a three-slice rectangle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Piece {
    /// The left or top cap.
    Start,
    /// The stretched middle.
    Middle,
    /// The right or bottom cap.
    End,
}

/// `rect` cut along `axis` into a `start`-long cap, a middle and an
/// `end`-long cap, in that order. Each piece spans the whole rectangle
/// across the axis.
///
/// The caps keep their length and the middle stretches to fill the rest;
/// with no length left, there is no middle. When the rectangle is shorter
/// than the two caps together, each cap gets half of it. A rectangle whose
/// width or height is not a positive, finite number has no pieces.
#[must_use]
pub fn three_slice(rect: Bounds, axis: Axis, start: f32, end: f32) -> Vec<(Piece, Bounds)> {
    let drawable = |side: f32| side.is_finite() && side > 0.0;
    if !(drawable(rect.width()) && drawable(rect.height())) {
        return Vec::new();
    }
    let length = match axis {
        Axis::Horizontal => rect.width(),
        Axis::Vertical => rect.height(),
    };
    let (start, end) = if start + end > length {
        (length / 2.0, length / 2.0)
    } else {
        (start, end)
    };
    // A span [from, to] along the axis, across the whole rectangle.
    let span = |from: f32, to: f32| match axis {
        Axis::Horizontal => Bounds {
            min: Point::new(rect.min.x + from, rect.min.y),
            max: Point::new(rect.min.x + to, rect.max.y),
        },
        Axis::Vertical => Bounds {
            min: Point::new(rect.min.x, rect.min.y + from),
            max: Point::new(rect.max.x, rect.min.y + to),
        },
    };
    let mut pieces = vec![(Piece::Start, span(0.0, start))];
    if length - start - end > 0.0 {
        pieces.push((Piece::Middle, span(start, length - end)));
    }
    pieces.push((Piece::End, span(length - end, length)));
    pieces
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    fn rect(x: f32, y: f32, w: f32, h: f32) -> Bounds {
        Bounds::at(Point::new(x, y), w, h)
    }

    #[test]
    fn a_wide_button_has_both_caps_and_a_stretched_middle() {
        assert_eq!(
            three_slice(rect(10.0, 20.0, 99.0, 25.0), Axis::Horizontal, 13.0, 13.0),
            [
                (Piece::Start, rect(10.0, 20.0, 13.0, 25.0)),
                (Piece::Middle, rect(23.0, 20.0, 73.0, 25.0)),
                (Piece::End, rect(96.0, 20.0, 13.0, 25.0)),
            ]
        );
        assert_eq!(
            three_slice(rect(0.0, 0.0, 200.0, 25.0), Axis::Horizontal, 13.0, 13.0),
            [
                (Piece::Start, rect(0.0, 0.0, 13.0, 25.0)),
                (Piece::Middle, rect(13.0, 0.0, 174.0, 25.0)),
                (Piece::End, rect(187.0, 0.0, 13.0, 25.0)),
            ]
        );
    }

    #[test]
    fn one_unit_past_the_caps_is_a_one_unit_middle() {
        let pieces = three_slice(rect(0.0, 0.0, 27.0, 25.0), Axis::Horizontal, 13.0, 13.0);
        assert_eq!(pieces[1], (Piece::Middle, rect(13.0, 0.0, 1.0, 25.0)));
        assert_eq!(pieces.len(), 3);
    }

    #[test]
    fn exactly_the_caps_has_no_middle() {
        assert_eq!(
            three_slice(rect(0.0, 0.0, 26.0, 25.0), Axis::Horizontal, 13.0, 13.0),
            [
                (Piece::Start, rect(0.0, 0.0, 13.0, 25.0)),
                (Piece::End, rect(13.0, 0.0, 13.0, 25.0)),
            ]
        );
    }

    #[test]
    fn exactly_unequal_caps_keep_their_lengths() {
        assert_eq!(
            three_slice(rect(0.0, 0.0, 30.0, 49.0), Axis::Vertical, 9.0, 40.0),
            [
                (Piece::Start, rect(0.0, 0.0, 30.0, 9.0)),
                (Piece::End, rect(0.0, 9.0, 30.0, 40.0)),
            ]
        );
    }

    #[test]
    fn shorter_than_the_caps_halves_them() {
        assert_eq!(
            three_slice(rect(5.0, 0.0, 20.0, 25.0), Axis::Horizontal, 13.0, 13.0),
            [
                (Piece::Start, rect(5.0, 0.0, 10.0, 25.0)),
                (Piece::End, rect(15.0, 0.0, 10.0, 25.0)),
            ]
        );
        assert_eq!(
            three_slice(rect(0.0, 0.0, 30.0, 10.0), Axis::Vertical, 9.0, 40.0),
            [
                (Piece::Start, rect(0.0, 0.0, 30.0, 5.0)),
                (Piece::End, rect(0.0, 5.0, 30.0, 5.0)),
            ]
        );
    }

    #[test]
    fn a_tall_frame_slices_vertically_with_unequal_caps() {
        assert_eq!(
            three_slice(rect(291.0, 227.0, 441.0, 313.0), Axis::Vertical, 9.0, 40.0),
            [
                (Piece::Start, rect(291.0, 227.0, 441.0, 9.0)),
                (Piece::Middle, rect(291.0, 236.0, 441.0, 264.0)),
                (Piece::End, rect(291.0, 500.0, 441.0, 40.0)),
            ]
        );
    }

    #[test]
    fn the_caps_stretch_across_the_axis() {
        for height in [20.0, 25.0, 40.0] {
            let pieces = three_slice(rect(0.0, 0.0, 99.0, height), Axis::Horizontal, 13.0, 13.0);
            assert!(pieces.iter().all(|(_, piece)| piece.height() == height));
        }
    }

    #[test]
    fn a_rectangle_with_no_area_has_no_pieces() {
        for (w, h) in [
            (0.0, 25.0),
            (-5.0, 25.0),
            (99.0, 0.0),
            (99.0, -1.0),
            (f32::NAN, 25.0),
            (99.0, f32::INFINITY),
        ] {
            for axis in [Axis::Horizontal, Axis::Vertical] {
                assert!(
                    three_slice(rect(0.0, 0.0, w, h), axis, 13.0, 13.0).is_empty(),
                    "{w} x {h} {axis:?}"
                );
            }
        }
    }
}
