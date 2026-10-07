//! `përs`: AI personalities the player can meet.
//!
//! Layout: field names and meanings from the EV Nova Bible ("The përs
//! resource"); byte order and widths from the ResForge Nova template
//! (`TMPB` 514), which sums to the 400 bytes of every stock record. The
//! Bible lists eight weapon slots, but the record holds four. The 64 bytes
//! at 0x13A, not in the Bible, are the person's subtitle.

use binrw::BinRead;
use nova_rsrc::ResType;
use serde::{Deserialize, Serialize};

use crate::decode::Record;
use crate::wire::flags::Flags16;
use crate::wire::id::{GovtId, MissionId, PictId, ShipId, WeaponId, id, ids};
use crate::wire::raw::RawArray;
use crate::wire::string::{MacString, fixed_c_string};

/// A person. The resource name is the person's name.
#[derive(BinRead, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[br(big)]
pub struct Person {
    /// Bible `LinkSyst` (offset 0x00, i16): encoded: -1 any, 128-2175
    /// `sÿst` ID, 9999+/15000+/20000+/25000+ government-relative.
    pub link_syst: i16,
    /// Bible `Govt` (offset 0x02, i16): `gövt` ID, or -1 for independent.
    #[br(map = id::<GovtId>)]
    pub govt: Option<GovtId>,
    /// Bible `AI Type` (offset 0x04, i16): 1-4.
    pub ai_type: i16,
    /// Bible `Aggress` (offset 0x06, i16): 1 (close) to 3 (far).
    pub aggress: i16,
    /// Bible `Coward` (offset 0x08, i16): shield percent at which to flee.
    pub coward: i16,
    /// Bible `ShipType` (offset 0x0A, i16): `shïp` ID.
    #[br(map = id::<ShipId>)]
    pub ship_type: Option<ShipId>,
    /// Bible `WeapType` (offset 0x0C, 4 x i16): extra `wëap` IDs; -1 or 0
    /// for none.
    #[br(map = ids::<WeaponId, 4>)]
    pub weap_type: [Option<WeaponId>; 4],
    /// Bible `WeapCount` (offset 0x14, 4 x i16).
    pub weap_count: [i16; 4],
    /// Bible `AmmoLoad` (offset 0x1C, 4 x i16).
    pub ammo_load: [i16; 4],
    /// Bible `Credits` (offset 0x24, i32).
    pub credits: i32,
    /// Bible `ShieldMod` (offset 0x28, i16): shield percent; negative is
    /// invincible.
    pub shield_mod: i16,
    /// Bible `HailPict` (offset 0x2A, i16): `PICT` ID for the hail dialog.
    #[br(map = id::<PictId>)]
    pub hail_pict: Option<PictId>,
    /// Bible `CommQuote` (offset 0x2C, i16): entry in `STR#` 7100. An index,
    /// not a resource ID.
    pub comm_quote: i16,
    /// Bible `HailQuote` (offset 0x2E, i16): entry in `STR#` 7101. An index,
    /// not a resource ID.
    pub hail_quote: i16,
    /// Bible `LinkMission` (offset 0x30, i16): `mïsn` ID offered when
    /// boarded or hailed.
    #[br(map = id::<MissionId>)]
    pub link_mission: Option<MissionId>,
    /// Bible `Flags` (offset 0x32, u16).
    pub flags: Flags16,
    /// Bible `ActiveOn` (offset 0x34, 256-byte C string): control-bit test
    /// expression.
    #[br(parse_with = fixed_c_string::<_, 256>)]
    pub active_on: MacString,
    /// Bible `GrantClass` (offset 0x134, i16): outfit `ItemClass` granted
    /// when boarded; 0 or -1 if unused.
    pub grant_class: i16,
    /// Bible `GrantCount` (offset 0x136, i16): maximum items granted.
    pub grant_count: i16,
    /// Bible `GrantProb` (offset 0x138, i16): percent chance of a grant.
    pub grant_prob: i16,
    /// Offset 0x13A, 64-byte C string: the person's subtitle, which the
    /// target panel shows under its name. Not in the Bible; the template
    /// labels it a subtitle, and the engine copies it for the target panel
    /// (`_LoadObjectData` @0x7c4f6, drawn by `_DrawStatusTarg` @0x4b860).
    #[br(parse_with = fixed_c_string::<_, 64>)]
    pub subtitle: MacString,
    /// Bible `Color` (offset 0x17A, u32): ship paint, `00RRGGBB`; 0 for
    /// none.
    pub color: u32,
    /// Bible `Flags2` (offset 0x17E, u16).
    pub flags2: Flags16,
    /// Offset 0x180, 16 bytes: undocumented (the Bible is silent; the
    /// template marks them unused).
    pub unknown_0x180: RawArray<16>,
}

impl Record for Person {
    const TYPE: ResType = ResType::new([b'p', 0x91, b'r', b's']);
    const SIZE: Option<usize> = Some(400);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::buf;

    #[test]
    fn fields_sit_at_their_bible_offsets() {
        let pers: Person = buf::<Person>()
            .i16(0x00, 10_128)
            .i16(0x02, 130)
            .i16(0x04, 3)
            .i16(0x06, 2)
            .i16(0x08, 25)
            .i16(0x0A, 140)
            .i16(0x0C, 150)
            .i16(0x0E, -1)
            .i16(0x14, 2)
            .i16(0x1C, 40)
            .i32(0x24, 50_000)
            .i16(0x28, 130)
            .i16(0x2A, 5000)
            .i16(0x2C, 3)
            .i16(0x2E, 4)
            .i16(0x30, 400)
            .u16(0x32, 0x0241)
            .bytes(0x34, b"b10 & !b11\0")
            .i16(0x134, 25)
            .i16(0x136, 1)
            .i16(0x138, 50)
            .bytes(0x13A, b"w00tWare\0")
            .u32(0x17A, 0x00FF_FFFF)
            .u16(0x17E, 0x0001)
            .bytes(0x180, &[0x21; 16])
            .decode();
        assert_eq!(pers.link_syst, 10_128);
        assert_eq!(pers.govt, Some(GovtId(130)));
        assert_eq!([pers.ai_type, pers.aggress, pers.coward], [3, 2, 25]);
        assert_eq!(pers.ship_type, Some(ShipId(140)));
        assert_eq!(pers.weap_type[0], Some(WeaponId(150)));
        assert_eq!(pers.weap_type[1], None);
        assert_eq!((pers.weap_count[0], pers.ammo_load[0]), (2, 40));
        assert_eq!((pers.credits, pers.shield_mod), (50_000, 130));
        assert_eq!(pers.hail_pict, Some(PictId(5000)));
        assert_eq!((pers.comm_quote, pers.hail_quote), (3, 4));
        assert_eq!(pers.link_mission, Some(MissionId(400)));
        assert_eq!(pers.flags, Flags16(0x0241));
        assert_eq!(pers.active_on.as_str(), "b10 & !b11");
        assert_eq!(
            [pers.grant_class, pers.grant_count, pers.grant_prob],
            [25, 1, 50]
        );
        assert_eq!(pers.subtitle.as_str(), "w00tWare");
        assert_eq!(pers.color, 0x00FF_FFFF);
        assert_eq!(pers.flags2, Flags16(0x0001));
        assert_eq!(pers.unknown_0x180, RawArray([0x21; 16]));
    }

    #[test]
    fn the_subtitle_is_named_in_the_json_and_uses_all_64_bytes_without_a_nul() {
        let pers: Person = buf::<Person>().bytes(0x13A, b"Top Gun\0").decode();
        let json = serde_json::to_value(&pers).expect("serializes");
        assert_eq!(json["subtitle"], "Top Gun");
        let full: Person = buf::<Person>().bytes(0x13A, &[b'x'; 64]).decode();
        assert_eq!(full.subtitle.len(), 64);
        assert_eq!(full.color, 0, "the next field is untouched");
    }

    #[test]
    fn unset_references_are_none() {
        let pers: Person = buf::<Person>()
            .i16(0x02, -1)
            .i16(0x2A, -1)
            .i16(0x30, -1)
            .decode();
        assert_eq!(
            (pers.govt, pers.hail_pict, pers.link_mission),
            (None, None, None)
        );
    }
}
