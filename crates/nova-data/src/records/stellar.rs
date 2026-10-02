//! `spöb`: stellar objects (planets and stations).
//!
//! Layout: field names and meanings from the EV Nova Bible ("The spöb
//! resource"); byte order and widths from the ResForge Nova template
//! (`TMPB` 520), which sums to the 1118 bytes of every stock record. The
//! Bible's eight `SpecialTech` fields are stored as three near the start and
//! five near the end.

use binrw::BinRead;
use nova_rsrc::ResType;
use serde::{Deserialize, Serialize};

use crate::decode::Record;
use crate::wire::flags::{Flags16, Flags32};
use crate::wire::id::{DudeId, GovtId, StellarId, WeaponId, id, ids};
use crate::wire::raw::RawArray;
use crate::wire::string::{MacString, fixed_c_string};

/// A stellar object.
#[derive(BinRead, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[br(big)]
pub struct Stellar {
    /// Bible `xPos` (offset 0x00, i16): X position in the system.
    pub x_pos: i16,
    /// Bible `yPos` (offset 0x02, i16): Y position in the system.
    pub y_pos: i16,
    /// Bible `Type` (offset 0x04, i16): graphic, 0-255 (`spïn` 1000+).
    pub graphic_type: i16,
    /// Bible `Flags` (offset 0x06, u32): services and commodity price
    /// levels.
    pub flags: Flags32,
    /// Bible `Tribute` (offset 0x0A, i16): daily tribute when dominated; -1
    /// or 0 for the default.
    pub tribute: i16,
    /// Bible `TechLevel` (offset 0x0C, i16): base tech level.
    pub tech_level: i16,
    /// Bible `SpecialTech` 1-3 (offset 0x0E, 3 x i16): exact-match tech
    /// levels.
    pub special_tech1_3: [i16; 3],
    /// Bible `Govt` (offset 0x14, i16): controlling `gövt` ID, or -1 for
    /// independent.
    #[br(map = id::<GovtId>)]
    pub govt: Option<GovtId>,
    /// Bible `MinStatus` (offset 0x16, i16): legal record below which
    /// landing is refused; -32767 ignores it.
    pub min_status: i16,
    /// Bible `CustPicID` (offset 0x18, i16): encoded: 128 and up is a
    /// landscape `PICT` ID; below 128 none (hypergates reuse it).
    pub cust_pic_id: i16,
    /// Bible `CustSndID` (offset 0x1A, i16): encoded: ambient `snd ` ID or
    /// -1 for none (hypergates and wormholes reuse it as an angle).
    pub cust_snd_id: i16,
    /// Bible `DefenseDude` (offset 0x1C, i16): defense fleet `düde` ID, or
    /// -1 for none.
    #[br(map = id::<DudeId>)]
    pub defense_dude: Option<DudeId>,
    /// Bible `DefCount` (offset 0x1E, i16): encoded defense fleet size (above
    /// 1000, ships launch in waves).
    pub def_count: i16,
    /// Bible `Flags2` (offset 0x20, u16): animation, hypergate and other
    /// flags.
    pub flags2: Flags16,
    /// Bible `AnimDelay` (offset 0x22, i16): frame delay in 30ths of a
    /// second.
    pub anim_delay: i16,
    /// Bible `Frame0Bias` (offset 0x24, i16): display-time multiplier for
    /// the first frame.
    pub frame0_bias: i16,
    /// Bible `HyperLink1-8` (offset 0x26, 8 x i16): linked hypergate or
    /// wormhole `spöb` IDs; 0 or -1 if unused.
    #[br(map = ids::<StellarId, 8>)]
    pub hyper_link: [Option<StellarId>; 8],
    /// Bible `OnDominate` (offset 0x36, 255-byte C string): control-bit set
    /// expression.
    #[br(parse_with = fixed_c_string::<_, 255>)]
    pub on_dominate: MacString,
    /// Bible `OnRelease` (offset 0x135, 255-byte C string): control-bit set
    /// expression.
    #[br(parse_with = fixed_c_string::<_, 255>)]
    pub on_release: MacString,
    /// Bible `Fee` (offset 0x234, i32): landing fee.
    pub fee: i32,
    /// Bible `Gravity` (offset 0x238, i16): pull (positive) or push
    /// (negative).
    pub gravity: i16,
    /// Bible `Weapon` (offset 0x23A, i16): `wëap` ID of the stellar's
    /// weapon; 0 or -1 for none.
    #[br(map = id::<WeaponId>)]
    pub weapon: Option<WeaponId>,
    /// Bible `Strength` (offset 0x23C, i32): damage it takes to destroy; 0
    /// or -1 for invincible.
    pub strength: i32,
    /// Bible `DeadType` (offset 0x240, i16): graphic when destroyed, or -1.
    pub dead_type: i16,
    /// Bible `DeadTime` (offset 0x242, i16): days until it regenerates; -1
    /// for never.
    pub dead_time: i16,
    /// Bible `ExplodType` (offset 0x244, i16): encoded explosion type: 0-63,
    /// +1000 for extra small explosions, -1 for none.
    pub explod_type: i16,
    /// Bible `OnDestroy` (offset 0x246, 255-byte C string): control-bit set
    /// expression.
    #[br(parse_with = fixed_c_string::<_, 255>)]
    pub on_destroy: MacString,
    /// Bible `OnRegen` (offset 0x345, 255-byte C string): control-bit set
    /// expression.
    #[br(parse_with = fixed_c_string::<_, 255>)]
    pub on_regen: MacString,
    /// Bible `SpecialTech` 4-8 (offset 0x444, 5 x i16): exact-match tech
    /// levels.
    pub special_tech4_8: [i16; 5],
    /// Offset 0x44E, 16 bytes: undocumented (the Bible is silent; the
    /// template marks them unused).
    pub unknown_0x44e: RawArray<16>,
}

impl Record for Stellar {
    const TYPE: ResType = ResType::new([b's', b'p', 0x9A, b'b']);
    const SIZE: Option<usize> = Some(1118);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::buf;

    #[test]
    fn fields_sit_at_their_bible_offsets() {
        let spob: Stellar = buf::<Stellar>()
            .i16(0x00, -300)
            .i16(0x02, 240)
            .i16(0x04, 12)
            .u32(0x06, 0x1020_0047)
            .i16(0x0A, -1)
            .i16(0x0C, 5)
            .i16(0x0E, 31)
            .i16(0x12, 33)
            .i16(0x14, 129)
            .i16(0x16, -32767)
            .i16(0x18, 10_000)
            .i16(0x1A, 300)
            .i16(0x1C, 140)
            .i16(0x1E, 1082)
            .u16(0x20, 0x3001)
            .i16(0x22, 4)
            .i16(0x24, 2)
            .i16(0x26, 400)
            .i16(0x28, -1)
            .bytes(0x36, b"b100\0")
            .bytes(0x135, b"!b100\0")
            .i32(0x234, 5000)
            .i16(0x238, -20)
            .i16(0x23A, 140)
            .i32(0x23C, 70_000)
            .i16(0x240, 3)
            .i16(0x242, -1)
            .i16(0x244, 1003)
            .bytes(0x246, b"b9\0")
            .bytes(0x345, b"!b9\0")
            .i16(0x444, 40)
            .i16(0x44C, 44)
            .bytes(0x44E, &[0x55; 16])
            .decode();
        assert_eq!((spob.x_pos, spob.y_pos, spob.graphic_type), (-300, 240, 12));
        assert_eq!(spob.flags, Flags32(0x1020_0047));
        assert_eq!((spob.tribute, spob.tech_level), (-1, 5));
        assert_eq!(spob.special_tech1_3, [31, 0, 33]);
        assert_eq!(spob.govt, Some(GovtId(129)));
        assert_eq!(spob.min_status, -32767);
        assert_eq!((spob.cust_pic_id, spob.cust_snd_id), (10_000, 300));
        assert_eq!(spob.defense_dude, Some(DudeId(140)));
        assert_eq!(spob.def_count, 1082);
        assert_eq!(spob.flags2, Flags16(0x3001));
        assert_eq!((spob.anim_delay, spob.frame0_bias), (4, 2));
        assert_eq!(spob.hyper_link[0], Some(StellarId(400)));
        assert_eq!(spob.hyper_link[1], None);
        assert_eq!(spob.on_dominate.as_str(), "b100");
        assert_eq!(spob.on_release.as_str(), "!b100");
        assert_eq!(spob.fee, 5000);
        assert_eq!(spob.gravity, -20);
        assert_eq!(spob.weapon, Some(WeaponId(140)));
        assert_eq!(spob.strength, 70_000);
        assert_eq!(
            (spob.dead_type, spob.dead_time, spob.explod_type),
            (3, -1, 1003)
        );
        assert_eq!(spob.on_destroy.as_str(), "b9");
        assert_eq!(spob.on_regen.as_str(), "!b9");
        assert_eq!(spob.special_tech4_8, [40, 0, 0, 0, 44]);
        assert_eq!(spob.unknown_0x44e, RawArray([0x55; 16]));
    }

    #[test]
    fn no_govt_dude_or_weapon_is_none() {
        let spob: Stellar = buf::<Stellar>()
            .i16(0x14, -1)
            .i16(0x1C, -1)
            .i16(0x23A, -1)
            .decode();
        assert_eq!(
            (spob.govt, spob.defense_dude, spob.weapon),
            (None, None, None)
        );
    }
}
