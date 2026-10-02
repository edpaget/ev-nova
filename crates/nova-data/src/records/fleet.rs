//! `flët`: fleets of ships that appear together.
//!
//! Layout: field names and meanings from the EV Nova Bible ("The flët
//! resource"); byte order and widths from the ResForge Nova template
//! (`TMPB` 506), which sums to the 306 bytes of every stock record.

use binrw::BinRead;
use nova_rsrc::ResType;
use serde::{Deserialize, Serialize};

use crate::decode::Record;
use crate::wire::flags::Flags16;
use crate::wire::id::{GovtId, ShipId, StrListId, id, ids};
use crate::wire::raw::RawArray;
use crate::wire::string::{MacString, fixed_c_string};

/// A fleet.
#[derive(BinRead, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[br(big)]
pub struct Fleet {
    /// Bible `LeadShipType` (offset 0x00, i16): flagship `shïp` ID.
    #[br(map = id::<ShipId>)]
    pub lead_ship_type: Option<ShipId>,
    /// Bible `EscortType` x4 (offset 0x02, 4 x i16): escort `shïp` IDs.
    #[br(map = ids::<ShipId, 4>)]
    pub escort_type: [Option<ShipId>; 4],
    /// Bible `Min` x4 (offset 0x0A, 4 x i16): minimum escorts of each type.
    pub min: [i16; 4],
    /// Bible `Max` x4 (offset 0x12, 4 x i16): maximum escorts of each type.
    pub max: [i16; 4],
    /// Bible `Govt` (offset 0x1A, i16): `gövt` ID, or -1 for none.
    #[br(map = id::<GovtId>)]
    pub govt: Option<GovtId>,
    /// Bible `LinkSyst` (offset 0x1C, i16): encoded: -1 any, 128-2175
    /// `sÿst` ID, 10000+/15000+/20000+/25000+ government-relative.
    pub link_syst: i16,
    /// Bible `AppearOn` (offset 0x1E, 256-byte C string): control-bit test
    /// expression.
    #[br(parse_with = fixed_c_string::<_, 256>)]
    pub appear_on: MacString,
    /// Bible `Quote` (offset 0x11E, i16): `STR#` ID of arrival quotes.
    #[br(map = id::<StrListId>)]
    pub quote: Option<StrListId>,
    /// Bible `Flags` (offset 0x120, u16).
    pub flags: Flags16,
    /// Offset 0x122, 16 bytes: undocumented (the Bible is silent; the
    /// template marks them unused).
    pub unknown_0x122: RawArray<16>,
}

impl Record for Fleet {
    const TYPE: ResType = ResType::new([b'f', b'l', 0x91, b't']);
    const SIZE: Option<usize> = Some(306);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::buf;

    #[test]
    fn fields_sit_at_their_bible_offsets() {
        let flet: Fleet = buf::<Fleet>()
            .i16(0x00, 200)
            .i16(0x02, 201)
            .i16(0x04, -1)
            .i16(0x0A, 1)
            .i16(0x12, 3)
            .i16(0x1A, -1)
            .i16(0x1C, 10_130)
            .bytes(0x1E, b"b42\0")
            .i16(0x11E, 7600)
            .u16(0x120, 0x0001)
            .bytes(0x122, &[0x12; 16])
            .decode();
        assert_eq!(flet.lead_ship_type, Some(ShipId(200)));
        assert_eq!(flet.escort_type[0], Some(ShipId(201)));
        assert_eq!(flet.escort_type[1], None);
        assert_eq!((flet.min[0], flet.max[0]), (1, 3));
        assert_eq!(flet.govt, None);
        assert_eq!(flet.link_syst, 10_130);
        assert_eq!(flet.appear_on.as_str(), "b42");
        assert_eq!(flet.quote, Some(StrListId(7600)));
        assert_eq!(flet.flags, Flags16(0x0001));
        assert_eq!(flet.unknown_0x122, RawArray([0x12; 16]));
    }
}
