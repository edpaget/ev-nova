//! `shïp`: ship classes.
//!
//! Layout: field names and meanings from the EV Nova Bible ("The shïp
//! resource"); byte order and widths from the ResForge Nova template
//! (`TMPB` 518), which sums to the 1860 bytes of every stock record. The
//! Bible's eight weapon slots and eight default items are each stored as two
//! blocks of four, and later fields were appended after the original block,
//! so the byte order differs from the Bible's description order.

use binrw::BinRead;
use nova_rsrc::ResType;
use serde::{Deserialize, Serialize};

use crate::decode::Record;
use crate::wire::flags::{Flags16, Flags64};
use crate::wire::id::{OutfitId, ShipId, WeaponId, id, ids};
use crate::wire::raw::RawArray;
use crate::wire::string::{MacString, fixed_c_string};

/// A ship class. The resource name is the class name.
#[derive(BinRead, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[br(big)]
pub struct Ship {
    /// Bible `Holds` (offset 0x00, i16): cargo tons; negative forbids mass
    /// expansions.
    pub holds: i16,
    /// Bible `Shield` (offset 0x02, i16): shield strength; negative means 5x
    /// the absolute value.
    pub shield: i16,
    /// Bible `Accel` (offset 0x04, i16): acceleration; 300 is average.
    pub accel: i16,
    /// Bible `Speed` (offset 0x06, i16): top speed; 300 is average.
    pub speed: i16,
    /// Bible `Maneuver` (offset 0x08, i16): turn rate; 10 is about 30°/s.
    pub maneuver: i16,
    /// Bible `Fuel` (offset 0x0A, i16): fuel capacity; 100 is one jump.
    pub fuel: i16,
    /// Bible `FreeMass` (offset 0x0C, i16): free expansion space in tons.
    pub free_mass: i16,
    /// Bible `Armor` (offset 0x0E, i16): armour strength.
    pub armor: i16,
    /// Bible `ShieldRech` (offset 0x10, i16): shield points x1000 per frame.
    pub shield_rech: i16,
    /// Bible `WeapType` 1-4 (offset 0x12, 4 x i16): stock `wëap` IDs; -1 or
    /// 0 for none.
    #[br(map = ids::<WeaponId, 4>)]
    pub weap_type1_4: [Option<WeaponId>; 4],
    /// Bible `WeapCount` 1-4 (offset 0x1A, 4 x i16).
    pub weap_count1_4: [i16; 4],
    /// Bible `AmmoLoad` 1-4 (offset 0x22, 4 x i16).
    pub ammo_load1_4: [i16; 4],
    /// Bible `MaxGun` (offset 0x2A, i16): maximum fixed guns.
    pub max_gun: i16,
    /// Bible `MaxTur` (offset 0x2C, i16): maximum turrets.
    pub max_tur: i16,
    /// Bible `TechLevel` (offset 0x2E, i16).
    pub tech_level: i16,
    /// Bible `Cost` (offset 0x30, i32): purchase price in credits.
    pub cost: i32,
    /// Bible `DeathDelay` (offset 0x34, i16): frames spent breaking up.
    pub death_delay: i16,
    /// Bible `ArmorRech` (offset 0x36, i16): armour points x1000 per frame.
    pub armor_rech: i16,
    /// Bible `Explode1` (offset 0x38, i16): explosion type while breaking
    /// up, or -1.
    pub explode1: i16,
    /// Bible `Explode2` (offset 0x3A, i16): encoded final explosion type:
    /// 0-63, +1000 for extra small explosions, -1 for none.
    pub explode2: i16,
    /// Bible `DispWeight` (offset 0x3C, i16): shipyard ordering.
    pub disp_weight: i16,
    /// Bible `Mass` (offset 0x3E, i16): tons.
    pub mass: i16,
    /// Bible `Length` (offset 0x40, i16): metres (display only).
    pub length: i16,
    /// Bible `InherentAI` (offset 0x42, i16): AI type 1-4 when escorting.
    pub inherent_ai: i16,
    /// Bible `Crew` (offset 0x44, i16).
    pub crew: i16,
    /// Bible `Strength` (offset 0x46, i16): relative strength for combat
    /// odds.
    pub strength: i16,
    /// Bible `InherentGovt` (offset 0x48, i16): encoded: -1 none, 128-383
    /// `gövt` ID, +1000 attributes only, +2000 combat only.
    pub inherent_govt: i16,
    /// Bible `Flags` (offset 0x4A, u16).
    pub flags: Flags16,
    /// Bible `PodCount` (offset 0x4C, i16): decorative escape pods.
    pub pod_count: i16,
    /// Bible `DefaultItems` 1-4 (offset 0x4E, 4 x i16): `oütf` IDs given to
    /// the player; -1 for none.
    #[br(map = ids::<OutfitId, 4>)]
    pub default_items1_4: [Option<OutfitId>; 4],
    /// Bible `ItemCount` 1-4 (offset 0x56, 4 x i16).
    pub item_count1_4: [i16; 4],
    /// Bible `FuelRegen` (offset 0x5E, i16): inherent fuel regeneration.
    pub fuel_regen: i16,
    /// Bible `SkillVar` (offset 0x60, i16): pilot skill variance, percent.
    pub skill_var: i16,
    /// Bible `Flags2` (offset 0x62, u16).
    pub flags2: Flags16,
    /// Bible `Contribute` pair (offset 0x64, u64).
    pub contribute: Flags64,
    /// Bible `Availability` (offset 0x6C, 255-byte C string): control-bit
    /// test expression.
    #[br(parse_with = fixed_c_string::<_, 255>)]
    pub availability: MacString,
    /// Bible `AppearOn` (offset 0x16B, 255-byte C string): control-bit test
    /// expression.
    #[br(parse_with = fixed_c_string::<_, 255>)]
    pub appear_on: MacString,
    /// Bible `OnPurchase` (offset 0x26A, 256-byte C string): control-bit set
    /// expression.
    #[br(parse_with = fixed_c_string::<_, 256>)]
    pub on_purchase: MacString,
    /// Bible `Deionize` (offset 0x36A, i16): ion dissipation rate.
    pub deionize: i16,
    /// Bible `IonizeMax` (offset 0x36C, i16): fully-ionized charge.
    pub ionize_max: i16,
    /// Bible `KeyCarried` (offset 0x36E, i16): key carried `shïp` ID.
    #[br(map = id::<ShipId>)]
    pub key_carried: Option<ShipId>,
    /// Bible `DefaultItms2` 5-8 (offset 0x370, 4 x i16): more default `oütf`
    /// IDs.
    #[br(map = ids::<OutfitId, 4>)]
    pub default_items5_8: [Option<OutfitId>; 4],
    /// Bible `ItemCount` 5-8 (offset 0x378, 4 x i16).
    pub item_count5_8: [i16; 4],
    /// Bible `Require` pair (offset 0x380, u64).
    pub require: Flags64,
    /// Bible `BuyRandom` (offset 0x388, i16): percent chance for sale.
    pub buy_random: i16,
    /// Bible `HireRandom` (offset 0x38A, i16): percent chance for hire.
    pub hire_random: i16,
    /// Offset 0x38C, 68 bytes: undocumented (the Bible is silent; the
    /// template marks them unused).
    pub unknown_0x38c: RawArray<68>,
    /// Bible `OnCapture` (offset 0x3D0, 255-byte C string): control-bit set
    /// expression.
    #[br(parse_with = fixed_c_string::<_, 255>)]
    pub on_capture: MacString,
    /// Bible `OnRetire` (offset 0x4CF, 255-byte C string): control-bit set
    /// expression.
    #[br(parse_with = fixed_c_string::<_, 255>)]
    pub on_retire: MacString,
    /// Bible `ShortName` (offset 0x5CE, 64-byte C string): shipyard name.
    #[br(parse_with = fixed_c_string::<_, 64>)]
    pub short_name: MacString,
    /// Bible `CommName` (offset 0x60E, 32-byte C string): name when hailed.
    #[br(parse_with = fixed_c_string::<_, 32>)]
    pub comm_name: MacString,
    /// Bible `Long Name` (offset 0x62E, 128-byte C string).
    #[br(parse_with = fixed_c_string::<_, 128>)]
    pub long_name: MacString,
    /// Bible `MovieFile` (offset 0x6AE, 32-byte C string): shipyard movie.
    #[br(parse_with = fixed_c_string::<_, 32>)]
    pub movie_file: MacString,
    /// Bible `WeapType` 5-8 (offset 0x6CE, 4 x i16): more stock `wëap` IDs.
    #[br(map = ids::<WeaponId, 4>)]
    pub weap_type5_8: [Option<WeaponId>; 4],
    /// Bible `WeapCount` 5-8 (offset 0x6D6, 4 x i16).
    pub weap_count5_8: [i16; 4],
    /// Bible `AmmoLoad` 5-8 (offset 0x6DE, 4 x i16).
    pub ammo_load5_8: [i16; 4],
    /// Bible `Subtitle` (offset 0x6E6, 64-byte C string): target display
    /// subtitle.
    #[br(parse_with = fixed_c_string::<_, 64>)]
    pub subtitle: MacString,
    /// Bible `Flags3` (offset 0x726, u16).
    pub flags3: Flags16,
    /// Bible `UpgradeTo` (offset 0x728, i16): escort upgrade `shïp` ID; 0 or
    /// -1 if none.
    #[br(map = id::<ShipId>)]
    pub upgrade_to: Option<ShipId>,
    /// Bible `EscUpgrdCost` (offset 0x72A, i32).
    pub esc_upgrd_cost: i32,
    /// Bible `EscSellValue` (offset 0x72E, i32): 0 or less means 10% of
    /// cost.
    pub esc_sell_value: i32,
    /// Bible `EscortType` (offset 0x732, i16): -1 auto, 0 fighter, 1
    /// medium, 2 warship, 3 freighter.
    pub escort_type: i16,
    /// Offset 0x734, 16 bytes: undocumented (the Bible is silent; the
    /// template marks them unused).
    pub unknown_0x734: RawArray<16>,
}

impl Record for Ship {
    const TYPE: ResType = ResType::new([b's', b'h', 0x95, b'p']);
    const SIZE: Option<usize> = Some(1860);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::buf;

    #[test]
    fn performance_and_weapon_fields_sit_at_their_bible_offsets() {
        let ship: Ship = buf::<Ship>()
            .i16(0x00, -100)
            .i16(0x02, 300)
            .i16(0x04, 350)
            .i16(0x06, 400)
            .i16(0x08, 10)
            .i16(0x0A, 300)
            .i16(0x0C, 20)
            .i16(0x0E, 150)
            .i16(0x10, 1000)
            .i16(0x12, 128)
            .i16(0x14, -1)
            .i16(0x1A, 2)
            .i16(0x22, 30)
            .i16(0x2A, 4)
            .i16(0x2C, 1)
            .i16(0x2E, 3)
            .i32(0x30, 10_000)
            .i16(0x34, 60)
            .i16(0x36, 500)
            .i16(0x38, 1)
            .i16(0x3A, 1002)
            .i16(0x3C, 9)
            .i16(0x3E, 33)
            .i16(0x40, 20)
            .i16(0x42, 4)
            .i16(0x44, 2)
            .i16(0x46, 7)
            .i16(0x48, 1129)
            .u16(0x4A, 0x8001)
            .i16(0x4C, 2)
            .decode();
        assert_eq!(
            [
                ship.holds,
                ship.shield,
                ship.accel,
                ship.speed,
                ship.maneuver
            ],
            [-100, 300, 350, 400, 10]
        );
        assert_eq!(
            [ship.fuel, ship.free_mass, ship.armor, ship.shield_rech],
            [300, 20, 150, 1000]
        );
        assert_eq!(ship.weap_type1_4[0], Some(WeaponId(128)));
        assert_eq!(ship.weap_type1_4[1], None);
        assert_eq!((ship.weap_count1_4[0], ship.ammo_load1_4[0]), (2, 30));
        assert_eq!((ship.max_gun, ship.max_tur, ship.tech_level), (4, 1, 3));
        assert_eq!(ship.cost, 10_000);
        assert_eq!((ship.death_delay, ship.armor_rech), (60, 500));
        assert_eq!((ship.explode1, ship.explode2), (1, 1002));
        assert_eq!((ship.disp_weight, ship.mass, ship.length), (9, 33, 20));
        assert_eq!((ship.inherent_ai, ship.crew, ship.strength), (4, 2, 7));
        assert_eq!(ship.inherent_govt, 1129);
        assert_eq!(ship.flags, Flags16(0x8001));
        assert_eq!(ship.pod_count, 2);
    }

    #[test]
    fn item_text_and_escort_fields_sit_at_their_bible_offsets() {
        let ship: Ship = buf::<Ship>()
            .i16(0x4E, 200)
            .i16(0x50, -1)
            .i16(0x56, 3)
            .i16(0x5E, 60)
            .i16(0x60, 10)
            .u16(0x62, 0x4000)
            .u64(0x64, 0x0000_0000_0000_0010)
            .bytes(0x6C, b"b1\0")
            .bytes(0x16B, b"b2\0")
            .bytes(0x26A, b"b3\0")
            .i16(0x36A, 100)
            .i16(0x36C, 400)
            .i16(0x36E, 140)
            .i16(0x370, 201)
            .i16(0x378, 4)
            .u64(0x380, 0x8000_0000_0000_0000)
            .i16(0x388, 90)
            .i16(0x38A, 50)
            .bytes(0x38C, &[0x5A; 68])
            .bytes(0x3D0, b"b4\0")
            .bytes(0x4CF, b"b5\0")
            .bytes(0x5CE, b"Shuttle\0")
            .bytes(0x60E, b"shuttle\0")
            .bytes(0x62E, b"Shuttlecraft\0")
            .bytes(0x6AE, b"ship.mov\0")
            .i16(0x6CE, 130)
            .i16(0x6D6, 5)
            .i16(0x6DE, 50)
            .bytes(0x6E6, b"Light Transport\0")
            .u16(0x726, 0x0003)
            .i16(0x728, 129)
            .i32(0x72A, 25_000)
            .i32(0x72E, -1)
            .i16(0x732, 3)
            .bytes(0x734, &[0xA5; 16])
            .decode();
        assert_eq!(ship.default_items1_4[0], Some(OutfitId(200)));
        assert_eq!(ship.default_items1_4[1], None);
        assert_eq!(ship.item_count1_4[0], 3);
        assert_eq!((ship.fuel_regen, ship.skill_var), (60, 10));
        assert_eq!(ship.flags2, Flags16(0x4000));
        assert_eq!(ship.contribute, Flags64(0x10));
        assert_eq!(ship.availability.as_str(), "b1");
        assert_eq!(ship.appear_on.as_str(), "b2");
        assert_eq!(ship.on_purchase.as_str(), "b3");
        assert_eq!((ship.deionize, ship.ionize_max), (100, 400));
        assert_eq!(ship.key_carried, Some(ShipId(140)));
        assert_eq!(ship.default_items5_8[0], Some(OutfitId(201)));
        assert_eq!(ship.item_count5_8[0], 4);
        assert_eq!(ship.require, Flags64(0x8000_0000_0000_0000));
        assert_eq!((ship.buy_random, ship.hire_random), (90, 50));
        assert_eq!(ship.unknown_0x38c, RawArray([0x5A; 68]));
        assert_eq!(ship.on_capture.as_str(), "b4");
        assert_eq!(ship.on_retire.as_str(), "b5");
        assert_eq!(ship.short_name.as_str(), "Shuttle");
        assert_eq!(ship.comm_name.as_str(), "shuttle");
        assert_eq!(ship.long_name.as_str(), "Shuttlecraft");
        assert_eq!(ship.movie_file.as_str(), "ship.mov");
        assert_eq!(ship.weap_type5_8[0], Some(WeaponId(130)));
        assert_eq!((ship.weap_count5_8[0], ship.ammo_load5_8[0]), (5, 50));
        assert_eq!(ship.subtitle.as_str(), "Light Transport");
        assert_eq!(ship.flags3, Flags16(0x0003));
        assert_eq!(ship.upgrade_to, Some(ShipId(129)));
        assert_eq!((ship.esc_upgrd_cost, ship.esc_sell_value), (25_000, -1));
        assert_eq!(ship.escort_type, 3);
        assert_eq!(ship.unknown_0x734, RawArray([0xA5; 16]));
    }

    #[test]
    fn no_key_carried_or_upgrade_is_none() {
        let ship: Ship = buf::<Ship>().i16(0x36E, -1).i16(0x728, -1).decode();
        assert_eq!((ship.key_carried, ship.upgrade_to), (None, None));
    }
}
