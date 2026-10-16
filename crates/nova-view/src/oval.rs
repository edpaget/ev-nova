//! QuickDraw's ovals in whole pixels: the pixels `PaintOval` fills
//! ([`oval_fill_pixels`]) and `FrameOval` frames at a pen width
//! ([`oval_frame_pixels`]) in a rect, and the runs a row of them makes
//! ([`fill_pixels`]). The radar frames its stellars' ovals with them (see
//! the flight's `hud`), and the galaxy map its systems'.
//!
//! QuickDraw's own oval rasters are in the Toolbox, not in the original's
//! binary, so these are our approximation of them: a pixel is inside the
//! oval inscribed in a rect when its centre lies inside the ellipse (on it
//! included); a pen n wide frames each inside pixel within n pixels, left,
//! right, up or down, of one outside. A 2×2 oval is all 4 pixels, and a
//! 4×4 one the 8 that are not its corners.

use crate::Color;
use crate::draw::{DrawList, fill_rect};
use crate::flight::radar::IRect;
use crate::geometry::{Bounds, Point};

/// Whether the pixel at (`x`, `y`) is inside the oval inscribed in
/// `rect`: its centre in the ellipse, on it included. Only the pixels of a
/// rect, and their neighbours, are asked about, so an empty rect, which
/// has none, has none inside.
fn inside(rect: IRect, x: i32, y: i32) -> bool {
    let (width, height) = (
        i64::from(rect.right) - i64::from(rect.left),
        i64::from(rect.bottom) - i64::from(rect.top),
    );
    // Twice the pixel centre's offset from the oval's centre, scaled so
    // the test stays in whole numbers.
    let dx = 2 * (i64::from(x) - i64::from(rect.left)) + 1 - width;
    let dy = 2 * (i64::from(y) - i64::from(rect.top)) + 1 - height;
    dx * dx * height * height + dy * dy * width * width <= width * width * height * height
}

/// The pixels `PaintOval` fills in `rect`, row by row.
#[must_use]
pub fn oval_fill_pixels(rect: IRect) -> Vec<(i32, i32)> {
    let mut pixels = Vec::new();
    for y in rect.top..rect.bottom {
        for x in rect.left..rect.right {
            if inside(rect, x, y) {
                pixels.push((x, y));
            }
        }
    }
    pixels
}

/// The pixels framing the oval inscribed in `rect` at a pen `pen` wide,
/// row by row: those inside it with a pixel within `pen` of it, left,
/// right, up or down, that is not.
#[must_use]
pub fn oval_frame_pixels(rect: IRect, pen: i32) -> Vec<(i32, i32)> {
    oval_fill_pixels(rect)
        .into_iter()
        .filter(|&(x, y)| {
            (1..=pen).any(|step| {
                [(x - step, y), (x + step, y), (x, y - step), (x, y + step)]
                    .into_iter()
                    .any(|(x, y)| !inside(rect, x, y))
            })
        })
        .collect()
}

/// The runs `pixels`, given row by row and left to right, make: each
/// stretch of neighbouring pixels in a row, as its row, its first column
/// and the column just past its last.
#[must_use]
pub fn runs(pixels: &[(i32, i32)]) -> Vec<(i32, i32, i32)> {
    let mut runs: Vec<(i32, i32, i32)> = Vec::new();
    for &(x, y) in pixels {
        match runs.last_mut() {
            Some((row, _, end)) if *row == y && *end == x => *end = x + 1,
            _ => runs.push((y, x, x + 1)),
        }
    }
    runs
}

/// Fills `pixels` (row by row, left to right) in `color`: one 1-pixel-high
/// rectangle for each of their [`runs`].
pub fn fill_pixels(list: &mut DrawList, pixels: &[(i32, i32)], color: Color) {
    for (y, from, to) in runs(pixels) {
        #[allow(clippy::cast_precision_loss)] // Pixels on a screen.
        let (y, from, to) = (y as f32, from as f32, to as f32);
        fill_rect(
            list,
            Bounds {
                min: Point::new(from, y),
                max: Point::new(to, y + 1.0),
            },
            color,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DrawCommand;

    #[test]
    fn small_ovals_frame_every_pixel_and_a_four_pixel_one_leaves_its_corners() {
        assert_eq!(
            oval_frame_pixels(IRect::new(10, 20, 12, 22), 1),
            [(10, 20), (11, 20), (10, 21), (11, 21)]
        );
        assert_eq!(
            oval_frame_pixels(IRect::new(0, 0, 4, 4), 1),
            [
                (1, 0),
                (2, 0),
                (0, 1),
                (3, 1),
                (0, 2),
                (3, 2),
                (1, 3),
                (2, 3)
            ]
        );
    }

    #[test]
    fn a_six_pixel_oval_is_rounded() {
        // Rows 0 and 5 run from 1 to 4; rows 1 to 4 are their ends alone.
        let mut expected = vec![(1, 0), (2, 0), (3, 0), (4, 0)];
        for y in 1..5 {
            expected.extend([(0, y), (5, y)]);
        }
        expected.extend([(1, 5), (2, 5), (3, 5), (4, 5)]);
        assert_eq!(oval_frame_pixels(IRect::new(0, 0, 6, 6), 1), expected);
    }

    #[test]
    fn a_clipped_oval_is_squashed_into_its_rect() {
        // 3 across and 6 down: a point at each end, the sides between.
        let mut expected = vec![(101, 0)];
        for y in 1..5 {
            expected.extend([(100, y), (102, y)]);
        }
        expected.push((101, 5));
        assert_eq!(oval_frame_pixels(IRect::new(100, 0, 103, 6), 1), expected);
        assert_eq!(oval_frame_pixels(IRect::new(0, 0, 0, 6), 1), []);
        assert_eq!(oval_fill_pixels(IRect::new(0, 0, 6, 0)), []);
    }

    /// The rows of `pixels` drawn over a `size` square, `#` set.
    fn picture(pixels: &[(i32, i32)], size: i32) -> Vec<String> {
        (0..size)
            .map(|y| {
                (0..size)
                    .map(|x| if pixels.contains(&(x, y)) { '#' } else { '.' })
                    .collect()
            })
            .collect()
    }

    #[test]
    fn a_painted_oval_fills_every_pixel_inside() {
        assert_eq!(
            picture(&oval_fill_pixels(IRect::new(0, 0, 8, 8)), 8),
            [
                "..####..", ".######.", "########", "########", "########", "########", ".######.",
                "..####..",
            ]
        );
    }

    #[test]
    fn a_wider_pen_frames_more_of_the_inside() {
        assert_eq!(
            picture(&oval_frame_pixels(IRect::new(0, 0, 12, 12), 2), 12),
            [
                "....####....",
                "..########..",
                ".###....###.",
                ".##......##.",
                "##........##",
                "##........##",
                "##........##",
                "##........##",
                ".##......##.",
                ".###....###.",
                "..########..",
                "....####....",
            ]
        );
        let fill = oval_fill_pixels(IRect::new(0, 0, 12, 12));
        assert_eq!(oval_frame_pixels(IRect::new(0, 0, 12, 12), 12), fill);
    }

    #[test]
    fn neighbouring_pixels_in_a_row_make_one_run() {
        let pixels = [(1, 0), (2, 0), (0, 1), (3, 1), (4, 1), (2, 2)];
        assert_eq!(runs(&pixels), [(0, 1, 3), (1, 0, 1), (1, 3, 5), (2, 2, 3)]);
        assert_eq!(runs(&[]), []);
    }

    #[test]
    fn filled_pixels_are_one_rectangle_a_run() {
        let mut list = DrawList::new();
        fill_pixels(&mut list, &[(1, 0), (2, 0), (5, 3)], Color::WHITE);
        let lines: Vec<DrawCommand> = list.iter().cloned().collect();
        assert_eq!(
            lines,
            [
                DrawCommand::Line {
                    from: Point::new(1.0, 0.5),
                    to: Point::new(3.0, 0.5),
                    width: 1.0,
                    color: Color::WHITE,
                },
                DrawCommand::Line {
                    from: Point::new(5.0, 3.5),
                    to: Point::new(6.0, 3.5),
                    width: 1.0,
                    color: Color::WHITE,
                },
            ]
        );
    }
}
