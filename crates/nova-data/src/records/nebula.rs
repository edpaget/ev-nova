//! `nëbu`: nebula images on the star map.
//!
//! Layout: field names and meanings from the EV Nova Bible ("The nëbu
//! resource"); byte order and widths from the ResForge Nova template
//! (`TMPB` 511), which sums to the 534 bytes of every stock record.

use binrw::BinRead;
use nova_rsrc::ResType;
use serde::{Deserialize, Serialize};

use crate::decode::Record;
use crate::wire::raw::RawArray;
use crate::wire::string::{MacString, fixed_c_string};

/// A nebula.
#[derive(BinRead, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[br(big)]
pub struct Nebula {
    /// Bible `XPos` (offset 0x00, i16): left edge on the map.
    pub x_pos: i16,
    /// Bible `YPos` (offset 0x02, i16): top edge on the map.
    pub y_pos: i16,
    /// Bible `XSize` (offset 0x04, i16): width at normal zoom.
    pub x_size: i16,
    /// Bible `YSize` (offset 0x06, i16): height at normal zoom.
    pub y_size: i16,
    /// Bible `ActiveOn` (offset 0x08, 255-byte C string): control-bit test
    /// expression.
    #[br(parse_with = fixed_c_string::<_, 255>)]
    pub active_on: MacString,
    /// Bible `OnExplore` (offset 0x107, 255-byte C string): control-bit set
    /// expression.
    #[br(parse_with = fixed_c_string::<_, 255>)]
    pub on_explore: MacString,
    /// Offset 0x206, 16 bytes: undocumented (the Bible is silent; the
    /// template marks them unused).
    pub unknown_0x206: RawArray<16>,
}

impl Record for Nebula {
    const TYPE: ResType = ResType::new([b'n', 0x91, b'b', b'u']);
    const SIZE: Option<usize> = Some(534);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::buf;

    #[test]
    fn fields_sit_at_their_bible_offsets() {
        let nebu: Nebula = buf::<Nebula>()
            .i16(0, -400)
            .i16(2, 120)
            .i16(4, 300)
            .i16(6, 200)
            .bytes(8, b"b5\0")
            .bytes(0x107, b"b6 b7\0")
            .bytes(0x206, &[0x77; 16])
            .decode();
        assert_eq!(
            (nebu.x_pos, nebu.y_pos, nebu.x_size, nebu.y_size),
            (-400, 120, 300, 200)
        );
        assert_eq!(nebu.active_on.as_str(), "b5");
        assert_eq!(nebu.on_explore.as_str(), "b6 b7");
        assert_eq!(nebu.unknown_0x206, RawArray([0x77; 16]));
    }
}
