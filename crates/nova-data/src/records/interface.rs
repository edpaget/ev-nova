//! `ïntf`: status bar layout.
//!
//! Layout: field names and meanings from the EV Nova Bible ("The ïntf
//! resource"); byte order and widths from the ResForge Nova template
//! (`TMPB` 508), which sums to the 166 bytes of every stock record. Colours
//! are `00RRGGBB`; areas are rectangles.

use binrw::BinRead;
use nova_rsrc::ResType;
use serde::{Deserialize, Serialize};

use crate::decode::Record;
use crate::wire::geometry::Rect;
use crate::wire::string::{MacString, fixed_c_string};

/// A status bar layout.
#[derive(BinRead, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[br(big)]
pub struct Interface {
    /// Bible `BrightText` (offset 0x00, u32).
    pub bright_text: u32,
    /// Bible `DimText` (offset 0x04, u32).
    pub dim_text: u32,
    /// Bible `RadarArea` (offset 0x08, rect).
    pub radar_area: Rect,
    /// Bible `BrightRadar` (offset 0x10, u32).
    pub bright_radar: u32,
    /// Bible `DimRadar` (offset 0x14, u32).
    pub dim_radar: u32,
    /// Bible `ShieldArea` (offset 0x18, rect).
    pub shield_area: Rect,
    /// Bible `ShieldColor` (offset 0x20, u32).
    pub shield_color: u32,
    /// Bible `ArmorArea` (offset 0x24, rect).
    pub armor_area: Rect,
    /// Bible `ArmorColor` (offset 0x2C, u32).
    pub armor_color: u32,
    /// Bible `FuelArea` (offset 0x30, rect).
    pub fuel_area: Rect,
    /// Bible `FuelFull` (offset 0x38, u32).
    pub fuel_full: u32,
    /// Bible `FuelPartial` (offset 0x3C, u32).
    pub fuel_partial: u32,
    /// Bible `NavArea` (offset 0x40, rect).
    pub nav_area: Rect,
    /// Bible `WeapArea` (offset 0x48, rect).
    pub weap_area: Rect,
    /// Bible `TargArea` (offset 0x50, rect).
    pub targ_area: Rect,
    /// Bible `CargoArea` (offset 0x58, rect).
    pub cargo_area: Rect,
    /// Bible `StatusFont` (offset 0x60, 64-byte C string).
    #[br(parse_with = fixed_c_string::<_, 64>)]
    pub status_font: MacString,
    /// Bible `StatFontSize` (offset 0xA0, i16).
    pub stat_font_size: i16,
    /// Bible `SubtitleSize` (offset 0xA2, i16).
    pub subtitle_size: i16,
    /// Bible `StatusBkgnd` (offset 0xA4, i16): background `PICT` ID; values
    /// below 128 mean 128, so this is kept raw.
    pub status_bkgnd: i16,
}

impl Record for Interface {
    const TYPE: ResType = ResType::new([0x95, b'n', b't', b'f']);
    const SIZE: Option<usize> = Some(166);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::buf;

    fn rect(top: i16, left: i16, bottom: i16, right: i16) -> Rect {
        Rect {
            top,
            left,
            bottom,
            right,
        }
    }

    #[test]
    fn fields_sit_at_their_bible_offsets() {
        let mut b = buf::<Interface>()
            .u32(0x00, 0x00FF_FFFF)
            .u32(0x04, 0x0080_8080)
            .u32(0x10, 1)
            .u32(0x14, 2)
            .u32(0x20, 3)
            .u32(0x2C, 4)
            .u32(0x38, 5)
            .u32(0x3C, 6)
            .bytes(0x60, b"Geneva\0")
            .i16(0xA0, 12)
            .i16(0xA2, 10)
            .i16(0xA4, 700);
        for (i, offset) in [0x08, 0x18, 0x24, 0x30, 0x40, 0x48, 0x50, 0x58]
            .into_iter()
            .enumerate()
        {
            let n = i as i16 * 10;
            b = b
                .i16(offset, n)
                .i16(offset + 2, n + 1)
                .i16(offset + 4, n + 2)
                .i16(offset + 6, n + 3);
        }
        let intf: Interface = b.decode();
        assert_eq!(
            (intf.bright_text, intf.dim_text),
            (0x00FF_FFFF, 0x0080_8080)
        );
        assert_eq!(intf.radar_area, rect(0, 1, 2, 3));
        assert_eq!((intf.bright_radar, intf.dim_radar), (1, 2));
        assert_eq!(intf.shield_area, rect(10, 11, 12, 13));
        assert_eq!(intf.shield_color, 3);
        assert_eq!(intf.armor_area, rect(20, 21, 22, 23));
        assert_eq!(intf.armor_color, 4);
        assert_eq!(intf.fuel_area, rect(30, 31, 32, 33));
        assert_eq!((intf.fuel_full, intf.fuel_partial), (5, 6));
        assert_eq!(intf.nav_area, rect(40, 41, 42, 43));
        assert_eq!(intf.weap_area, rect(50, 51, 52, 53));
        assert_eq!(intf.targ_area, rect(60, 61, 62, 63));
        assert_eq!(intf.cargo_area, rect(70, 71, 72, 73));
        assert_eq!(intf.status_font.as_str(), "Geneva");
        assert_eq!((intf.stat_font_size, intf.subtitle_size), (12, 10));
        assert_eq!(intf.status_bkgnd, 700);
    }
}
