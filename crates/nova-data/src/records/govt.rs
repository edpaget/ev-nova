//! `gövt`: governments.
//!
//! Layout: field names and meanings from the EV Nova Bible ("The gövt
//! resource"); byte order and widths from the ResForge Nova template
//! (`TMPB` 507), which sums to the 192 bytes of every stock record. The
//! Bible describes the fields in a different order from the bytes; the
//! template's order is the byte order.

use binrw::BinRead;
use nova_rsrc::ResType;
use serde::{Deserialize, Serialize};

use crate::decode::Record;
use crate::wire::flags::{Flags16, Flags64};
use crate::wire::raw::RawArray;
use crate::wire::string::{MacString, fixed_c_string};

/// A government.
#[derive(BinRead, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[br(big)]
pub struct Govt {
    /// Bible `VoiceType` (offset 0x00, i16): encoded voice set: 0-7,
    /// +1000 odd sounds only, +2000 even only, -1 silent.
    pub voice_type: i16,
    /// Bible `Flags` (offset 0x02, u16).
    pub flags: Flags16,
    /// Bible `Flags2` (offset 0x04, u16).
    pub flags2: Flags16,
    /// Bible `ScanFine` (offset 0x06, i16): fine for illegal cargo; negative
    /// is a percentage of the player's cash.
    pub scan_fine: i16,
    /// Bible `CrimeTol` (offset 0x08, i16): evilness tolerated before
    /// warships attack.
    pub crime_tol: i16,
    /// Bible `SmugPenalty` (offset 0x0A, i16).
    pub smug_penalty: i16,
    /// Bible `DisabPenalty` (offset 0x0C, i16).
    pub disab_penalty: i16,
    /// Bible `BoardPenalty` (offset 0x0E, i16).
    pub board_penalty: i16,
    /// Bible `KillPenalty` (offset 0x10, i16).
    pub kill_penalty: i16,
    /// Bible `ShootPenalty` (offset 0x12, i16): currently ignored by the
    /// engine.
    pub shoot_penalty: i16,
    /// Bible `InitialRec` (offset 0x14, i16): starting legal record.
    pub initial_rec: i16,
    /// Bible `MaxOdds` (offset 0x16, i16): highest combat odds considered
    /// favourable; 100 is 1-to-1.
    pub max_odds: i16,
    /// Bible `Class1-Class4` (offset 0x18, 4 x i16): government classes; -1
    /// if unused. Class numbers, not resource IDs.
    pub class: [i16; 4],
    /// Bible `Ally1-Ally4` (offset 0x20, 4 x i16): allied classes; -1 if
    /// unused.
    pub ally: [i16; 4],
    /// Bible `Enemy1-Enemy4` (offset 0x28, 4 x i16): enemy classes; -1 if
    /// unused.
    pub enemy: [i16; 4],
    /// Bible `SkillMult` (offset 0x30, i16): pilot skill multiplier, 100 is
    /// stock.
    pub skill_mult: i16,
    /// Bible `ScanMask` (offset 0x32, u16): matched against mission and
    /// item scan masks.
    pub scan_mask: Flags16,
    /// Bible `CommName` (offset 0x34, 16-byte C string): name shown when
    /// hailed.
    #[br(parse_with = fixed_c_string::<_, 16>)]
    pub comm_name: MacString,
    /// Bible `TargetCode` (offset 0x44, 16-byte C string): name on the
    /// target display.
    #[br(parse_with = fixed_c_string::<_, 16>)]
    pub target_code: MacString,
    /// Bible `Require` pair (offset 0x54, u64): 64 bits needed to land on
    /// this government's stellars.
    pub require: Flags64,
    /// Bible `InhJam1-4` (offset 0x5C, 4 x i16): inherent jamming per type,
    /// 0-100%.
    pub inh_jam: [i16; 4],
    /// Bible `MediumName` (offset 0x64, 64-byte C string).
    #[br(parse_with = fixed_c_string::<_, 64>)]
    pub medium_name: MacString,
    /// Bible `Color` (offset 0xA4, u32): theme colour, `00RRGGBB`.
    pub color: u32,
    /// Bible `ShipColor` (offset 0xA8, u32): ship paint, `00RRGGBB`; 0 for
    /// none.
    pub ship_color: u32,
    /// Bible `Interface` (offset 0xAC, i16): `ïntf` ID; values below 128
    /// mean 128, so this is kept raw.
    pub interface: i16,
    /// Bible `NewsPic` (offset 0xAE, i16): news background `PICT` ID; values
    /// below 128 are ignored, so this is kept raw.
    pub news_pic: i16,
    /// Offset 0xB0, 16 bytes: undocumented (the Bible is silent; the
    /// template marks them unused).
    pub unknown_0xb0: RawArray<16>,
}

impl Record for Govt {
    const TYPE: ResType = ResType::new([b'g', 0x9A, b'v', b't']);
    const SIZE: Option<usize> = Some(192);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::buf;

    #[test]
    fn fields_sit_at_their_bible_offsets() {
        let govt: Govt = buf::<Govt>()
            .i16(0x00, 1003)
            .u16(0x02, 0x8001)
            .u16(0x04, 0x0012)
            .i16(0x06, -5)
            .i16(0x08, 6)
            .i16(0x0A, 1)
            .i16(0x0C, 2)
            .i16(0x0E, 3)
            .i16(0x10, 4)
            .i16(0x12, 5)
            .i16(0x14, -100)
            .i16(0x16, 200)
            .i16(0x18, 7)
            .i16(0x20, 8)
            .i16(0x28, 9)
            .i16(0x30, 150)
            .u16(0x32, 0x0004)
            .bytes(0x34, b"Federation\0")
            .bytes(0x44, b"Fed.\0")
            .u64(0x54, 0x8000_0000_0000_0001)
            .i16(0x5C, 25)
            .bytes(0x64, b"the Federation\0")
            .u32(0xA4, 0x002C_2CAF)
            .u32(0xA8, 0x00FF_0000)
            .i16(0xAC, 129)
            .i16(0xAE, 9001)
            .bytes(0xB0, &[0x33; 16])
            .decode();
        assert_eq!(govt.voice_type, 1003);
        assert_eq!(
            (govt.flags, govt.flags2),
            (Flags16(0x8001), Flags16(0x0012))
        );
        assert_eq!((govt.scan_fine, govt.crime_tol), (-5, 6));
        assert_eq!(
            [
                govt.smug_penalty,
                govt.disab_penalty,
                govt.board_penalty,
                govt.kill_penalty,
                govt.shoot_penalty
            ],
            [1, 2, 3, 4, 5]
        );
        assert_eq!((govt.initial_rec, govt.max_odds), (-100, 200));
        assert_eq!((govt.class[0], govt.ally[0], govt.enemy[0]), (7, 8, 9));
        assert_eq!(govt.skill_mult, 150);
        assert_eq!(govt.scan_mask, Flags16(0x0004));
        assert_eq!(govt.comm_name.as_str(), "Federation");
        assert_eq!(govt.target_code.as_str(), "Fed.");
        assert_eq!(govt.require, Flags64(0x8000_0000_0000_0001));
        assert_eq!(govt.inh_jam[0], 25);
        assert_eq!(govt.medium_name.as_str(), "the Federation");
        assert_eq!((govt.color, govt.ship_color), (0x002C_2CAF, 0x00FF_0000));
        assert_eq!((govt.interface, govt.news_pic), (129, 9001));
        assert_eq!(govt.unknown_0xb0, RawArray([0x33; 16]));
    }
}
