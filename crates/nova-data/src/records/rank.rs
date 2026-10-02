//! `ränk`: ranks the player can hold.
//!
//! Layout: field names and meanings from the EV Nova Bible ("ränk
//! resource"); byte order and widths from the ResForge Nova template
//! (`TMPB` 515), which sums to the 152 bytes of every stock record. The
//! Bible describes the fields in a different order from the bytes; the
//! template's order is the byte order.

use binrw::BinRead;
use nova_rsrc::ResType;
use serde::{Deserialize, Serialize};

use crate::decode::Record;
use crate::wire::flags::{Flags16, Flags64};
use crate::wire::id::{GovtId, id};
use crate::wire::string::{MacString, fixed_c_string};

/// A rank. The resource name is the rank's full name.
#[derive(BinRead, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[br(big)]
pub struct Rank {
    /// Bible `Weight` (offset 0x00, i16): importance relative to other
    /// active ranks.
    pub weight: i16,
    /// Bible `AffilGovt` (offset 0x02, i16): affiliated `gövt` ID.
    #[br(map = id::<GovtId>)]
    pub affil_govt: Option<GovtId>,
    /// Bible `PriceMod` (offset 0x04, i16): price percentage at the
    /// government's stellars.
    pub price_mod: i16,
    /// Bible `Salary` (offset 0x06, i32): credits per day.
    pub salary: i32,
    /// Bible `SalaryCap` (offset 0x0A, i32): cash above which the salary
    /// stops; 0 or -1 if unused.
    pub salary_cap: i32,
    /// Bible `Contribute` (offset 0x0E, u64): 64 contribute bits while
    /// active.
    pub contribute: Flags64,
    /// Bible `Flags` (offset 0x16, u16).
    pub flags: Flags16,
    /// Bible `ConvName` (offset 0x18, 64-byte C string): name used in
    /// conversation (`<PRK>`).
    #[br(parse_with = fixed_c_string::<_, 64>)]
    pub conv_name: MacString,
    /// Bible `ShortName` (offset 0x58, 64-byte C string): short name.
    #[br(parse_with = fixed_c_string::<_, 64>)]
    pub short_name: MacString,
}

impl Record for Rank {
    const TYPE: ResType = ResType::new([b'r', 0x8A, b'n', b'k']);
    const SIZE: Option<usize> = Some(152);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::buf;

    #[test]
    fn fields_sit_at_their_bible_offsets() {
        let rank: Rank = buf::<Rank>()
            .i16(0x00, 3)
            .i16(0x02, 136)
            .i16(0x04, 85)
            .i32(0x06, 200)
            .i32(0x0A, 1_000_000)
            .u64(0x0E, 0x0000_0001_0000_0002)
            .u16(0x16, 0x0F08)
            .bytes(0x18, b"Commander\0")
            .bytes(0x58, &[b'C'; 64])
            .decode();
        assert_eq!(rank.weight, 3);
        assert_eq!(rank.affil_govt, Some(GovtId(136)));
        assert_eq!(rank.price_mod, 85);
        assert_eq!((rank.salary, rank.salary_cap), (200, 1_000_000));
        assert_eq!(rank.contribute, Flags64(0x0000_0001_0000_0002));
        assert_eq!(rank.flags, Flags16(0x0F08));
        assert_eq!(rank.conv_name.as_str(), "Commander");
        assert_eq!(rank.short_name.as_str(), "C".repeat(64));
    }

    #[test]
    fn unaffiliated_rank_has_no_govt() {
        let rank: Rank = buf::<Rank>().i16(0x02, -1).decode();
        assert_eq!(rank.affil_govt, None);
    }
}
