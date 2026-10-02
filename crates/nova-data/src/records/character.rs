//! `chär`: starting character templates.
//!
//! Layout: field names and meanings from the EV Nova Bible ("The chär
//! resource"); byte order and widths from the ResForge Nova template
//! (`TMPB` 501), which sums to the 362 bytes of the stock record. The Bible
//! documents only the starting year, so the two words before it stay raw.

use binrw::BinRead;
use nova_rsrc::ResType;
use serde::{Deserialize, Serialize};

use crate::decode::Record;
use crate::wire::flags::Flags16;
use crate::wire::id::{DescId, GovtId, PictId, ShipId, SystemId, id, ids};
use crate::wire::raw::RawArray;
use crate::wire::string::{MacString, fixed_c_string};

/// A character template offered when a new pilot is created.
#[derive(BinRead, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[br(big)]
pub struct Character {
    /// Bible `Cash` (offset 0x00, i32): starting credits.
    pub cash: i32,
    /// Bible `ShipType` (offset 0x04, i16): starting `shïp` ID.
    #[br(map = id::<ShipId>)]
    pub ship_type: Option<ShipId>,
    /// Bible `System1-4` (offset 0x06, 4 x i16): starting `sÿst` IDs; -1 if
    /// unused.
    #[br(map = ids::<SystemId, 4>)]
    pub system: [Option<SystemId>; 4],
    /// Bible `Govt1-4` (offset 0x0E, 4 x i16): `gövt` IDs with a starting
    /// legal status; -1 if unused.
    #[br(map = ids::<GovtId, 4>)]
    pub govt: [Option<GovtId>; 4],
    /// Bible `Status1-4` (offset 0x16, 4 x i16): legal status for each
    /// `govt`.
    pub status: [i16; 4],
    /// Bible `Kills` (offset 0x1E, i16): starting combat rating.
    pub kills: i16,
    /// Bible `IntroPict1-4` (offset 0x20, 4 x i16): intro `PICT` IDs; -1 if
    /// unused.
    #[br(map = ids::<PictId, 4>)]
    pub intro_pict: [Option<PictId>; 4],
    /// Bible `PictDelay1-4` (offset 0x28, 4 x i16): seconds per picture.
    pub pict_delay: [i16; 4],
    /// Bible `IntroTextID` (offset 0x30, i16): intro `dësc` ID.
    #[br(map = id::<DescId>)]
    pub intro_text_id: Option<DescId>,
    /// Bible `OnStart` (offset 0x32, 256-byte C string): control-bit set
    /// expression.
    #[br(parse_with = fixed_c_string::<_, 256>)]
    pub on_start: MacString,
    /// Bible `Flags` (offset 0x132, u16): 0x0001 marks the default.
    pub flags: Flags16,
    /// Offset 0x134, i16: undocumented in the Bible (the template labels it
    /// the starting day).
    pub unknown_0x134: i16,
    /// Offset 0x136, i16: undocumented in the Bible (the template labels it
    /// the starting month).
    pub unknown_0x136: i16,
    /// Bible `StartYear` (offset 0x138, i16).
    pub start_year: i16,
    /// Bible `DatePrefix` (offset 0x13A, 16-byte C string).
    #[br(parse_with = fixed_c_string::<_, 16>)]
    pub date_prefix: MacString,
    /// Bible `DateSuffix` (offset 0x14A, 16-byte C string).
    #[br(parse_with = fixed_c_string::<_, 16>)]
    pub date_suffix: MacString,
    /// Offset 0x15A, 16 bytes: undocumented (the Bible is silent; the
    /// template marks them unused).
    pub unknown_0x15a: RawArray<16>,
}

impl Record for Character {
    const TYPE: ResType = ResType::new([b'c', b'h', 0x8A, b'r']);
    const SIZE: Option<usize> = Some(362);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::buf;

    #[test]
    fn fields_sit_at_their_bible_offsets() {
        let chr: Character = buf::<Character>()
            .i32(0x00, 25_000)
            .i16(0x04, 128)
            .i16(0x06, 136)
            .i16(0x08, -1)
            .i16(0x0E, 129)
            .i16(0x16, 50)
            .i16(0x1E, 100)
            .i16(0x20, 8000)
            .i16(0x22, -1)
            .i16(0x28, 10)
            .i16(0x30, 128)
            .bytes(0x32, b"b1 b2\0")
            .u16(0x132, 0x0001)
            .i16(0x134, 23)
            .i16(0x136, 6)
            .i16(0x138, 1177)
            .bytes(0x13A, b"Year \0")
            .bytes(0x14A, b" NC\0")
            .bytes(0x15A, &[0x0F; 16])
            .decode();
        assert_eq!(chr.cash, 25_000);
        assert_eq!(chr.ship_type, Some(ShipId(128)));
        assert_eq!(chr.system[0], Some(SystemId(136)));
        assert_eq!(chr.system[1], None);
        assert_eq!(chr.govt[0], Some(GovtId(129)));
        assert_eq!(chr.status[0], 50);
        assert_eq!(chr.kills, 100);
        assert_eq!(chr.intro_pict[0], Some(PictId(8000)));
        assert_eq!(chr.intro_pict[1], None);
        assert_eq!(chr.pict_delay[0], 10);
        assert_eq!(chr.intro_text_id, Some(DescId(128)));
        assert_eq!(chr.on_start.as_str(), "b1 b2");
        assert_eq!(chr.flags, Flags16(0x0001));
        assert_eq!((chr.unknown_0x134, chr.unknown_0x136), (23, 6));
        assert_eq!(chr.start_year, 1177);
        assert_eq!(chr.date_prefix.as_str(), "Year ");
        assert_eq!(chr.date_suffix.as_str(), " NC");
        assert_eq!(chr.unknown_0x15a, RawArray([0x0F; 16]));
    }
}
