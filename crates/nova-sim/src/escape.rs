//! The player's escape: what lets it eject from a disabled or dying ship,
//! and where its escape pod is taken (`_HandlePlayer`'s eject block
//! 0x6f22c-0x6f96f, its pod and rescue 0x68cc0-0x6923c).
//!
//! **What the eject needs.** An owned outfit of `ModType` 11, the escape
//! pod ([`has_escape_pod`], `_HasEscapePod` @0xb83d), or an escape ship
//! ([`escape_ship_type`]). Auto-eject needs an owned outfit of `ModType`
//! 20 too ([`has_auto_eject`], `_HasAutoEject` @0x3a14), and arms once the
//! breakup's countdown is low enough ([`auto_eject_armed`]). Each outfit
//! test looks only at outfits with a count above 0, and at all four of
//! an outfit's `ModType`s (@0xb881-0xb89a, @0x3a39-0x3a59).
//!
//! **The escape ship** (`_HasEscapeShip` @0x386e, `_EscapeShipType`
//! @0x38d0). The first `wëap`, in ID order, that is a fighter bay
//! (`Guidance` 99, @0x38ec), whose rounds the player holds (+0xd0 > 0,
//! @0x38f3; the fighters aboard), and whose fighter `shïp` has `Flags`
//! 0x8000 (the sign bit of `shïp` +0x9e8, `jns` @0x3916). Whether the bay
//! itself is still mounted is not asked. The player becomes that class,
//! its shield, armour and fuel each [`ESCAPE_FLOOR`] + a roll below
//! [`ESCAPE_ROLL`] percent of the class's own (0xdd098, @0x6f4d0-0x6f583).

use std::collections::BTreeMap;

use crate::catalog::{OutfitId, OutfitRecord, ShipId, ShipRecord, WeaponId};
use crate::combat::weapon::WeaponSpec;
use crate::outfitter::outfit_mods;

/// The `ModType` of an escape pod (`cmpw $0xb` @0xb88e; stock `oütf` 188).
pub const MOD_ESCAPE_POD: i16 = 11;
/// The `ModType` of auto-eject (`cmpw $0x14` @0x3a46; stock `oütf` 187).
pub const MOD_AUTO_EJECT: i16 = 20;
/// The `shïp` `Flags` bit of a fighter the player can escape in (the
/// sign bit of +0x9e8, @0x38b1, @0x3916).
pub const ESCAPE_SHIP_FLAG: u16 = 0x8000;
/// The floor of an escape ship's reserves, in percent of its class's
/// (`_HandlePlayer` @0x6f4d0-0x6f583).
pub const ESCAPE_FLOOR: u32 = 50;
/// How many percent above [`ESCAPE_FLOOR`] an escape ship's reserves can
/// roll: `Rand(30)`, so 50-79%.
pub const ESCAPE_ROLL: u32 = 30;
/// The auto-eject's floor, in ticks of the breakup's countdown: it arms
/// at or below 30.0 (0xdd084, @0x6f27d-0x6f286) whatever the `DeathDelay`.
pub const AUTO_EJECT_FLOOR: u32 = 30;
/// The `dësc` the rescue shows: "After several weeks of sensory
/// deprivation…" (@0x68e80-0x68eac).
pub const RESCUE_DESC: i16 = 13999;

/// Whether the outfits `owned` (of `records`) hold an escape pod: an
/// outfit with a count above 0 and a `ModType` [`MOD_ESCAPE_POD`] in any
/// of its four slots.
#[must_use]
pub fn has_escape_pod(owned: &BTreeMap<OutfitId, u16>, records: &[OutfitRecord]) -> bool {
    has_mod(owned, records, MOD_ESCAPE_POD)
}

/// Whether the outfits `owned` (of `records`) hold auto-eject: an outfit
/// with a count above 0 and a `ModType` [`MOD_AUTO_EJECT`] in any of its
/// four slots.
#[must_use]
pub fn has_auto_eject(owned: &BTreeMap<OutfitId, u16>, records: &[OutfitRecord]) -> bool {
    has_mod(owned, records, MOD_AUTO_EJECT)
}

/// Whether an outfit `owned` with a count above 0 has a mod of
/// `mod_type`.
fn has_mod(owned: &BTreeMap<OutfitId, u16>, records: &[OutfitRecord], mod_type: i16) -> bool {
    outfit_mods(owned, records)
        .iter()
        .any(|m| m.count > 0 && m.mod_type == mod_type)
}

/// The escape ship the player can eject into, if any: of `weapons`, in
/// `wëap` order, the first fighter bay whose `rounds` (the fighters
/// aboard) are above 0 and whose fighter's record among `ships` has
/// [`ESCAPE_SHIP_FLAG`] (see the module docs). A fighter with no record
/// has no flags.
pub fn escape_ship_type<'a>(
    weapons: impl IntoIterator<Item = &'a WeaponSpec>,
    rounds: impl Fn(WeaponId) -> u32,
    ships: &[ShipRecord],
) -> Option<ShipId> {
    weapons
        .into_iter()
        .filter(|weapon| rounds(weapon.id) > 0)
        .filter_map(|weapon| weapon.carried)
        .find(|&fighter| {
            ships
                .iter()
                .find(|record| record.id == fighter)
                .is_some_and(|record| record.flags & ESCAPE_SHIP_FLAG != 0)
        })
}

/// Whether auto-eject is armed with the breakup's countdown at `after`
/// (after this tick's decrement) on a ship of raw `DeathDelay`
/// `death_delay`: at or below half the `DeathDelay` (0.5 at 0xdd128,
/// `jae` @0x6f27b) or at or below [`AUTO_EJECT_FLOOR`] (@0x6f27d-0x6f286).
#[must_use]
pub fn auto_eject_armed(after: u32, death_delay: u32) -> bool {
    u64::from(after) * 2 <= u64::from(death_delay) || after <= AUTO_EJECT_FLOOR
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bay::FIGHTER_BAY;
    use crate::catalog::WeaponRecord;
    use crate::handling::ShipFields;
    use crate::testkit::{outfit, ship, weapon};

    fn owning(outfits: &[(i16, u16)]) -> BTreeMap<OutfitId, u16> {
        outfits.iter().map(|&(id, n)| (OutfitId(id), n)).collect()
    }

    #[test]
    fn the_pod_needs_mod_type_11_and_auto_eject_mod_type_20() {
        let records = [
            outfit(188, &[(MOD_ESCAPE_POD, 0)]),
            outfit(187, &[(MOD_AUTO_EJECT, 0)]),
            outfit(200, &[(1, 0), (2, 0), (3, 0), (MOD_ESCAPE_POD, 0)]),
            outfit(201, &[(1, 0), (2, 0), (3, 0), (MOD_AUTO_EJECT, 0)]),
            outfit(202, &[(12, 0), (19, 0), (21, 0)]),
        ];
        let pod = |owned: &[(i16, u16)]| has_escape_pod(&owning(owned), &records);
        let auto = |owned: &[(i16, u16)]| has_auto_eject(&owning(owned), &records);
        assert!(pod(&[(188, 1)]));
        assert!(!auto(&[(188, 1)]));
        assert!(auto(&[(187, 1)]));
        assert!(!pod(&[(187, 1)]));
        assert!(pod(&[(200, 2)]), "the fourth slot counts");
        assert!(auto(&[(201, 1)]), "the fourth slot counts");
        assert!(!pod(&[(188, 0)]) && !auto(&[(187, 0)]), "none owned");
        assert!(!pod(&[(202, 1)]) && !auto(&[(202, 1)]), "other mods");
        assert!(!pod(&[]) && !auto(&[]));
    }

    /// A fighter bay `id` launching `shïp` `fighter`.
    fn bay(id: i16, fighter: i16) -> WeaponSpec {
        WeaponSpec::new(&WeaponRecord {
            guidance: FIGHTER_BAY,
            ammo_type: fighter,
            ..weapon(id)
        })
    }

    /// `shïp` `id` with `Flags` `flags`.
    fn flagged(id: i16, flags: u16) -> ShipRecord {
        ShipRecord {
            flags,
            ..ship(id, ShipFields::default())
        }
    }

    #[test]
    fn an_escape_ship_is_the_first_flagged_fighter_bay_with_fighters_aboard() {
        let ships = [
            flagged(144, 0x8000),
            flagged(145, 0x8001),
            flagged(146, 0x7fff),
        ];
        let weapons = [bay(150, 146), bay(151, 144), bay(152, 145), bay(153, 147)];
        let full = |_: WeaponId| 2;
        assert_eq!(
            escape_ship_type(&weapons, full, &ships),
            Some(ShipId(144)),
            "the first flagged, by wëap order"
        );
        let aboard = |held: &'static [i16]| move |id: WeaponId| u32::from(held.contains(&id.0));
        assert_eq!(
            escape_ship_type(&weapons, aboard(&[152]), &ships),
            Some(ShipId(145)),
            "the first with fighters aboard"
        );
        assert_eq!(
            escape_ship_type(&weapons, aboard(&[150]), &ships),
            None,
            "no 0x8000"
        );
        assert_eq!(
            escape_ship_type(&weapons, aboard(&[153]), &ships),
            None,
            "no record"
        );
        assert_eq!(
            escape_ship_type(&weapons, |_| 0, &ships),
            None,
            "no fighters aboard"
        );
        let gun = WeaponSpec::new(&WeaponRecord {
            ammo_type: 144,
            ..weapon(140)
        });
        assert_eq!(escape_ship_type(&[gun], full, &ships), None, "not a bay");
    }

    #[test]
    fn auto_eject_arms_at_or_below_half_the_death_delay_or_30() {
        assert!(auto_eject_armed(50, 100));
        assert!(!auto_eject_armed(51, 100));
        assert!(auto_eject_armed(50, 101));
        assert!(!auto_eject_armed(51, 101));
        assert!(auto_eject_armed(30, 40));
        assert!(!auto_eject_armed(31, 40));
        assert!(auto_eject_armed(0, 0));
    }

    #[test]
    fn the_escape_constants_are_the_originals() {
        assert_eq!(
            (MOD_ESCAPE_POD, MOD_AUTO_EJECT, ESCAPE_SHIP_FLAG),
            (11, 20, 0x8000)
        );
        assert_eq!((ESCAPE_FLOOR, ESCAPE_ROLL, AUTO_EJECT_FLOOR), (50, 30, 30));
        assert_eq!(RESCUE_DESC, 13999);
    }
}
