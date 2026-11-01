//! The pen glyphs `_DrawButton` (@0x25ea8) draws in place of a label.
//!
//! A label's first character picks one: `^` (@0x26239-0x262f4) and `&`
//! (@0x262f9-0x263bc), the scroll arrows, and `+` (@0x263e1-0x264a3) and
//! `-` (@0x264a8-0x26525), the galaxy map's zoom. Each is `MoveTo` and
//! `Line` under `PenSize(2, 2)`, in integers relative to the button's
//! rectangle, with `cx = width / 2` and `cy = height / 2`.
//!
//! A diagonal `Line` under a 2 x 2 pen paints a 2 x 2 square at each step,
//! so a chevron is one square for each. A horizontal or vertical `Line` of
//! length n paints n + 2 by 2.

use crate::geometry::{Bounds, Point};

/// `PenSize`'s width and height.
const PEN: i32 = 2;

/// What `label` draws instead of its text on a button `width` by `height`,
/// as areas to fill relative to the button's corner, or `None` for a label
/// that is text.
pub(super) fn strokes(label: &str, width: f32, height: f32) -> Option<Vec<Bounds>> {
    let (width, height) = (width as i32, height as i32);
    let (cx, cy) = (width / 2, height / 2);
    match label.chars().next()? {
        '^' => {
            let arm = height / 10;
            Some(chevron(cx, cy - arm, arm, 1))
        }
        '&' => {
            let arm = height / 10;
            Some(chevron(cx, cy + arm, arm, -1))
        }
        '+' => {
            let arm = height / 8;
            Some(vec![
                area(cx - arm, cy, 2 * arm + PEN, PEN),
                area(cx, cy - arm, PEN, 2 * arm + PEN),
            ])
        }
        '-' => {
            let arm = height / 9;
            Some(vec![area(cx - arm, cy, 2 * arm + PEN, PEN)])
        }
        _ => None,
    }
}

/// Two strokes `2 * arm` long from the point at (`cx`, `tip_y`), going out
/// to both sides and on down the screen for a `step_y` of 1, or up it for
/// −1. The strokes share the point's square.
fn chevron(cx: i32, tip_y: i32, arm: i32, step_y: i32) -> Vec<Bounds> {
    let mut squares = vec![area(cx, tip_y, PEN, PEN)];
    for out in 1..=2 * arm {
        let y = tip_y + step_y * out;
        squares.push(area(cx - out, y, PEN, PEN));
        squares.push(area(cx + out, y, PEN, PEN));
    }
    squares
}

fn area(x: i32, y: i32, width: i32, height: i32) -> Bounds {
    Bounds::at(Point::new(x as f32, y as f32), width as f32, height as f32)
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    fn squares(points: &[(i32, i32)]) -> Vec<Bounds> {
        points.iter().map(|&(x, y)| area(x, y, 2, 2)).collect()
    }

    #[test]
    fn up_on_a_stock_arrow_is_the_squares_down_from_its_point() {
        // 23 x 23: arm 2, centre (11, 11).
        assert_eq!(
            strokes("^", 23.0, 23.0),
            Some(squares(&[
                (11, 9),
                (10, 10),
                (12, 10),
                (9, 11),
                (13, 11),
                (8, 12),
                (14, 12),
                (7, 13),
                (15, 13),
            ]))
        );
    }

    #[test]
    fn down_on_a_stock_arrow_is_the_squares_up_from_its_point() {
        assert_eq!(
            strokes("&", 23.0, 23.0),
            Some(squares(&[
                (11, 13),
                (10, 12),
                (12, 12),
                (9, 11),
                (13, 11),
                (8, 10),
                (14, 10),
                (7, 9),
                (15, 9),
            ]))
        );
    }

    #[test]
    fn a_taller_arrow_has_longer_arms_and_a_wider_one_its_own_centre() {
        // 41 x 30: arm 3, centre (20, 15).
        let up = strokes("^", 41.0, 30.0).expect("a glyph");
        assert_eq!(up.len(), 13);
        assert_eq!(up[0], area(20, 12, 2, 2));
        assert_eq!(up[11], area(14, 18, 2, 2));
        assert_eq!(up[12], area(26, 18, 2, 2));
        let down = strokes("&", 41.0, 30.0).expect("a glyph");
        assert_eq!(down.len(), 13);
        assert_eq!(down[0], area(20, 18, 2, 2));
        assert_eq!(down[11], area(14, 12, 2, 2));
        assert_eq!(down[12], area(26, 12, 2, 2));
    }

    #[test]
    fn plus_on_the_galaxy_maps_zoom_is_a_bar_across_and_a_bar_down() {
        // 25 x 25: arm 3, centre (12, 12).
        assert_eq!(
            strokes("+", 25.0, 25.0),
            Some(vec![area(9, 12, 8, 2), area(12, 9, 2, 8)])
        );
    }

    #[test]
    fn minus_on_the_galaxy_maps_zoom_is_a_bar_across() {
        // 25 x 25: arm 2, centre (12, 12).
        assert_eq!(strokes("-", 25.0, 25.0), Some(vec![area(10, 12, 6, 2)]));
    }

    #[test]
    fn the_arms_divide_the_height_alone_rounding_down() {
        // A wide rectangle changes the centre, not the arm.
        assert_eq!(
            strokes("+", 100.0, 39.0),
            Some(vec![area(46, 19, 10, 2), area(50, 15, 2, 10)])
        );
        assert_eq!(strokes("-", 100.0, 39.0), Some(vec![area(46, 19, 10, 2)]));
        assert_eq!(strokes("^", 100.0, 39.0).expect("a glyph").len(), 13);
    }

    #[test]
    fn a_label_that_starts_with_anything_else_is_text() {
        for label in ["", "Okay", "a^", " +"] {
            assert_eq!(strokes(label, 23.0, 23.0), None, "{label:?}");
        }
    }
}
