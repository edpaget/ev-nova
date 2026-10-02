//! Points and rectangles.
//!
//! [`Rect`] uses the classic Mac OS QuickDraw order (top, left, bottom,
//! right), as the ResForge templates read the Bible's "rectangular bounds"
//! fields; the stock `ïntf` rectangles only make sense in that order.
//! [`Point`] reads x then y, the order in which the Bible names coordinate
//! pairs (e.g. `Button1x & y`).

use binrw::BinRead;
use serde::{Deserialize, Serialize};

/// A point, x then y.
#[derive(BinRead, Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Point {
    /// Horizontal coordinate.
    pub x: i16,
    /// Vertical coordinate.
    pub y: i16,
}

/// A rectangle in QuickDraw order.
#[derive(BinRead, Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rect {
    /// Top edge.
    pub top: i16,
    /// Left edge.
    pub left: i16,
    /// Bottom edge.
    pub bottom: i16,
    /// Right edge.
    pub right: i16,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn point_reads_x_then_y() {
        let point = Point::read_be(&mut Cursor::new([0x01, 0x5D, 0xFF, 0x9C])).expect("decodes");
        assert_eq!(point, Point { x: 349, y: -100 });
    }

    #[test]
    fn rect_reads_quickdraw_order() {
        let bytes = [0, 199, 0, 35, 0, 206, 0, 184];
        let rect = Rect::read_be(&mut Cursor::new(bytes)).expect("decodes");
        assert_eq!(
            rect,
            Rect {
                top: 199,
                left: 35,
                bottom: 206,
                right: 184,
            }
        );
    }

    #[test]
    fn json_names_each_coordinate() {
        let json = serde_json::to_value(Rect {
            top: 1,
            left: 2,
            bottom: 3,
            right: 4,
        })
        .expect("serializes");
        assert_eq!(
            json,
            serde_json::json!({"top": 1, "left": 2, "bottom": 3, "right": 4})
        );
        let json = serde_json::to_value(Point { x: 5, y: 6 }).expect("serializes");
        assert_eq!(json, serde_json::json!({"x": 5, "y": 6}));
    }
}
