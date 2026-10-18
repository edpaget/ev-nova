//! Where the main screen's picture goes, as `_SetupInterfaceBackground`
//! (@0x1631d) places `PICT` 8000.
//!
//! It centres the picture in the port (`_CenterPict` @0x8652, halving
//! with a truncation toward zero), moves it up 60 when the port is at
//! most 767 high (@0x16377-0x1639b) and left 60 when it is at most 640
//! wide (@0x163a0-0x163c4), and takes `mainMenuCenter` as the picture's
//! centre and `menuTopCorner` as that centre less (512, 384)
//! (@0x163c9-0x1640e). Every main-screen position (the buttons, the logo,
//! the rollover) is an offset from `menuTopCorner`.
//!
//! The engine's main menu is always drawn in the classic 1024 x 768
//! frame, so the two shifts never apply there; the function is ported
//! whole all the same.

use crate::geometry::Point;

/// How far the picture moves up or left in a small port.
const SHIFT: i32 = 60;
/// The tallest port the picture moves up in.
const SHORT_PORT: i32 = 767;
/// The widest port the picture moves left in.
const NARROW_PORT: i32 = 640;
/// What `menuTopCorner` is less than the picture's centre.
const HALF_FRAME: (i32, i32) = (512, 384);

/// Where the main screen's picture and positions go in a port.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Background {
    /// The picture's top-left corner.
    pub picture_top_left: Point,
    /// The picture's centre: `mainMenuCenter`.
    pub center: Point,
    /// What every main-screen position is offset from: `menuTopCorner`.
    pub top_corner: Point,
}

/// The background for a picture `picture` (width, height) in a port
/// `port` (width, height) whose top-left is (0, 0).
#[must_use]
pub fn interface_background(port: (u32, u32), picture: (u32, u32)) -> Background {
    let wide = |n: u32| i32::try_from(n).unwrap_or(i32::MAX);
    let (port_w, port_h) = (wide(port.0), wide(port.1));
    let (pic_w, pic_h) = (wide(picture.0), wide(picture.1));
    let mut left = (port_w - pic_w) / 2;
    let mut top = (port_h - pic_h) / 2;
    if port_h <= SHORT_PORT {
        top -= SHIFT;
    }
    if port_w <= NARROW_PORT {
        left -= SHIFT;
    }
    let center = (left.midpoint(left + pic_w), top.midpoint(top + pic_h));
    let point = |(x, y): (i32, i32)| Point::new(x as f32, y as f32);
    Background {
        picture_top_left: point((left, top)),
        center: point(center),
        top_corner: point((center.0 - HALF_FRAME.0, center.1 - HALF_FRAME.1)),
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    fn background(port: (u32, u32), picture: (u32, u32)) -> (Point, Point, Point) {
        let found = interface_background(port, picture);
        (found.picture_top_left, found.center, found.top_corner)
    }

    #[test]
    fn the_stock_picture_fills_the_classic_port_from_its_corner() {
        assert_eq!(
            background((1024, 768), (1024, 768)),
            (
                Point::new(0.0, 0.0),
                Point::new(512.0, 384.0),
                Point::new(0.0, 0.0)
            )
        );
    }

    #[test]
    fn a_smaller_picture_is_centred_and_the_corner_stays_put() {
        assert_eq!(
            background((1024, 768), (100, 60)),
            (
                Point::new(462.0, 354.0),
                Point::new(512.0, 384.0),
                Point::new(0.0, 0.0)
            )
        );
    }

    #[test]
    fn halving_truncates_toward_zero() {
        // An odd leftover of 3 halves to 1, and of -3 to -1, as
        // `_CenterPict`'s `(n + sign) >> 1` does.
        let odd = interface_background((1027, 771), (1024, 768));
        assert_eq!(odd.picture_top_left, Point::new(1.0, 1.0));
        let bigger = interface_background((1021, 1765), (1024, 1768));
        assert_eq!(bigger.picture_top_left, Point::new(-1.0, -1.0));
        assert_eq!(bigger.center, Point::new(511.0, 883.0));
    }

    #[test]
    fn a_port_at_most_767_high_moves_the_picture_up_60() {
        assert_eq!(
            background((1024, 767), (1024, 768)),
            (
                Point::new(0.0, -60.0),
                Point::new(512.0, 324.0),
                Point::new(0.0, -60.0)
            )
        );
        let tall = interface_background((1024, 768), (1024, 768));
        assert_eq!(tall.picture_top_left.y, 0.0, "768 high is not short");
    }

    #[test]
    fn a_port_at_most_640_wide_moves_the_picture_left_60() {
        assert_eq!(
            background((640, 768), (1024, 768)),
            (
                Point::new(-252.0, 0.0),
                Point::new(260.0, 384.0),
                Point::new(-252.0, 0.0)
            )
        );
        let wide = interface_background((641, 768), (1024, 768));
        assert_eq!(wide.picture_top_left.x, -191.0, "641 wide is not narrow");
    }

    #[test]
    fn a_small_port_moves_it_both_ways() {
        assert_eq!(
            background((640, 480), (1024, 768)),
            (
                Point::new(-252.0, -204.0),
                Point::new(260.0, 180.0),
                Point::new(-252.0, -204.0)
            )
        );
    }
}
