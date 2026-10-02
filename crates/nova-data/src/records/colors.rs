//! `cölr`: game-wide interface colours, fonts and menu positions.
//!
//! Layout: field names and meanings from the EV Nova Bible ("The cölr
//! resource"); byte order and widths from the ResForge Nova template
//! (`TMPB` 502), which sums to the 244 bytes of the stock record. Colours are
//! `00RRGGBB`. The Bible lists `GridDim` before `GridBright`; the bytes store
//! the bright colour first (the stock record has red there, the selection
//! square, and dark grey second).

use binrw::BinRead;
use nova_rsrc::ResType;
use serde::{Deserialize, Serialize};

use crate::decode::Record;
use crate::wire::geometry::{Point, Rect};
use crate::wire::string::{MacString, fixed_c_string};

/// Interface colours and positions. Only the first `cölr` is used.
#[derive(BinRead, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[br(big)]
pub struct Colors {
    /// Bible `ButtonUp` (offset 0x00, u32).
    pub button_up: u32,
    /// Bible `ButtonDown` (offset 0x04, u32).
    pub button_down: u32,
    /// Bible `ButtonGrey` (offset 0x08, u32).
    pub button_grey: u32,
    /// Bible `MenuFont` (offset 0x0C, 64-byte C string).
    #[br(parse_with = fixed_c_string::<_, 64>)]
    pub menu_font: MacString,
    /// Bible `MenuFontSize` (offset 0x4C, i16).
    pub menu_font_size: i16,
    /// Bible `MenuColor1` (offset 0x4E, u32): bright.
    pub menu_color1: u32,
    /// Bible `MenuColor2` (offset 0x52, u32): dim.
    pub menu_color2: u32,
    /// Bible `GridBright` (offset 0x56, u32): selection square.
    pub grid_bright: u32,
    /// Bible `GridDim` (offset 0x5A, u32): shipyard/outfit grid.
    pub grid_dim: u32,
    /// Bible `ProgressBar` (offset 0x5E, rect): relative to the window
    /// centre.
    pub progress_bar: Rect,
    /// Bible `ProgBright` (offset 0x66, u32).
    pub prog_bright: u32,
    /// Bible `ProgDim` (offset 0x6A, u32).
    pub prog_dim: u32,
    /// Bible `ProgOutline` (offset 0x6E, u32).
    pub prog_outline: u32,
    /// Bible `Button1x & y` to `Button6x & y` (offset 0x72, 6 points): main
    /// menu buttons.
    pub button: [Point; 6],
    /// Bible `FloatingMap` (offset 0x8A, u32): map and escort menu border.
    pub floating_map: u32,
    /// Bible `ListText` (offset 0x8E, u32).
    pub list_text: u32,
    /// Bible `ListBkgnd` (offset 0x92, u32).
    pub list_bkgnd: u32,
    /// Bible `ListHilite` (offset 0x96, u32).
    pub list_hilite: u32,
    /// Bible `EscortHilite` (offset 0x9A, u32).
    pub escort_hilite: u32,
    /// Bible `ButtonFont` (offset 0x9E, 64-byte C string).
    #[br(parse_with = fixed_c_string::<_, 64>)]
    pub button_font: MacString,
    /// Bible `ButtonFontSz` (offset 0xDE, i16).
    pub button_font_sz: i16,
    /// Bible `LogoX & Y` (offset 0xE0, point).
    pub logo: Point,
    /// Bible `RolloverX & Y` (offset 0xE4, point).
    pub rollover: Point,
    /// Bible `Slide1x & y` to `Slide3x & y` (offset 0xE8, 3 points).
    pub slide: [Point; 3],
}

impl Record for Colors {
    const TYPE: ResType = ResType::new([b'c', 0x9A, b'l', b'r']);
    const SIZE: Option<usize> = Some(244);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::buf;

    #[test]
    fn fields_sit_at_their_bible_offsets() {
        let colr: Colors = buf::<Colors>()
            .u32(0x00, 0x00FF_FFFF)
            .u32(0x04, 0x0080_8080)
            .u32(0x08, 0x0026_2626)
            .bytes(0x0C, b"Geneva\0")
            .i16(0x4C, 9)
            .u32(0x4E, 0x00FF_0000)
            .u32(0x52, 0x0080_0000)
            .u32(0x56, 0x00FF_0001)
            .u32(0x5A, 0x0040_4040)
            .i16(0x5E, 280)
            .i16(0x60, -100)
            .i16(0x62, 290)
            .i16(0x64, 100)
            .u32(0x66, 1)
            .u32(0x6A, 2)
            .u32(0x6E, 3)
            .i16(0x72, 349)
            .i16(0x74, 400)
            .i16(0x86, 580)
            .i16(0x88, 528)
            .u32(0x8A, 4)
            .u32(0x8E, 5)
            .u32(0x92, 6)
            .u32(0x96, 7)
            .u32(0x9A, 8)
            .bytes(0x9E, b"Charcoal\0")
            .i16(0xDE, 12)
            .i16(0xE0, 191)
            .i16(0xE2, 162)
            .i16(0xE4, 444)
            .i16(0xE6, 465)
            .i16(0xE8, 343)
            .i16(0xEA, 399)
            .i16(0xF0, 337)
            .i16(0xF2, 526)
            .decode();
        let buttons = [colr.button_up, colr.button_down, colr.button_grey];
        assert_eq!(buttons, [0x00FF_FFFF, 0x0080_8080, 0x0026_2626]);
        assert_eq!(colr.menu_font.as_str(), "Geneva");
        assert_eq!(colr.menu_font_size, 9);
        assert_eq!(
            (colr.menu_color1, colr.menu_color2),
            (0x00FF_0000, 0x0080_0000)
        );
        assert_eq!(
            (colr.grid_bright, colr.grid_dim),
            (0x00FF_0001, 0x0040_4040)
        );
        assert_eq!(
            colr.progress_bar,
            Rect {
                top: 280,
                left: -100,
                bottom: 290,
                right: 100,
            }
        );
        assert_eq!(
            [colr.prog_bright, colr.prog_dim, colr.prog_outline],
            [1, 2, 3]
        );
        assert_eq!(colr.button[0], Point { x: 349, y: 400 });
        assert_eq!(colr.button[5], Point { x: 580, y: 528 });
        let lists = [
            colr.floating_map,
            colr.list_text,
            colr.list_bkgnd,
            colr.list_hilite,
            colr.escort_hilite,
        ];
        assert_eq!(lists, [4, 5, 6, 7, 8]);
        assert_eq!(colr.button_font.as_str(), "Charcoal");
        assert_eq!(colr.button_font_sz, 12);
        assert_eq!(colr.logo, Point { x: 191, y: 162 });
        assert_eq!(colr.rollover, Point { x: 444, y: 465 });
        assert_eq!(colr.slide[0], Point { x: 343, y: 399 });
        assert_eq!(colr.slide[2], Point { x: 337, y: 526 });
    }
}
