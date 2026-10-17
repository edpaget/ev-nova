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
//!
//! **Where the pod is taken** ([`escape_stellar`], `_EscapeStellarSearch`
//! @0xd28f). A depth-first search from the system the player died in,
//! which is marked visited first, so its own stellars are never tested.
//! Each system searched (`_RecursiveEscapeStellarSearch` @0xd0fe) is
//! marked visited, then works through its raw links (`Con1-Con16`, +0xa,
//! not resolved by place) in order: first it tests the stellars of each
//! link that is open ([`EscapeMap::open`]) and not yet visited, and returns
//! the first that qualifies ([`EscapeMap::stellars`]); only then does it
//! search each such link in turn (@0xd223-0xd27a). It finds nothing when
//! no open system it reaches has a qualifying stellar.

use std::collections::{BTreeMap, BTreeSet};

use crate::catalog::{
    LandingSite, OutfitId, OutfitRecord, ShipId, ShipRecord, StellarId, SystemId, WeaponId,
};
use crate::combat::weapon::WeaponSpec;
use crate::exploration::GATES;
use crate::landing::{StellarFlags, landable, open_by_record};
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

/// The galaxy as the escape search reads it: a port the session adapts.
pub trait EscapeMap {
    /// `system`'s raw links (`Con1-Con16`, +0xa), in slot order, repeats
    /// and all; none for a system not in the galaxy.
    fn raw_links(&self, system: SystemId) -> Vec<SystemId>;
    /// Whether `system` is open to the search: in the galaxy, visible
    /// (+0x1eb, @0xd159), known (+0x1ec, @0xd166, and again @0xd250 for
    /// the recursion) and explored (+0x90 > 0, @0xd173).
    fn open(&self, system: SystemId) -> bool;
    /// `system`'s stellars in its nav slots (+0x2a), in order, each with
    /// whether it qualifies: active (+0x44, @0xd1b2), landable
    /// (`_StellarIsLandable` @0xd1bb), not a hypergate or wormhole
    /// (`Flags2` & 0x3000, @0xd1cd), with a shipyard (`Flags` 0x08,
    /// @0xd1d8), and open by the player's record in `system`
    /// (@0xd1de-0xd1fc).
    fn stellars(&self, system: SystemId) -> Vec<(StellarId, bool)>;
}

/// Whether `site`, overlaid, `active` (+0x44: its owning system visible),
/// qualifies for the escape pod with the player's legal `record` in its
/// system (see [`EscapeMap::stellars`]).
#[must_use]
pub fn qualifies(site: &LandingSite, active: bool, record: i16) -> bool {
    active
        && landable(site.flags, site.destroyed)
        && site.flags2 & GATES == 0
        && site.flags & StellarFlags::SHIPYARD != 0
        && open_by_record(site.min_status, record)
}

/// Where the escape pod is taken from `from` (see the module docs): the
/// first qualifying stellar the search finds, with the system it was
/// found in; none when it finds none.
pub fn escape_stellar(from: SystemId, map: &impl EscapeMap) -> Option<(StellarId, SystemId)> {
    let mut visited = BTreeSet::from([from]);
    search(from, map, &mut visited)
}

/// One `_RecursiveEscapeStellarSearch` from `system`.
fn search(
    system: SystemId,
    map: &impl EscapeMap,
    visited: &mut BTreeSet<SystemId>,
) -> Option<(StellarId, SystemId)> {
    visited.insert(system);
    let links = map.raw_links(system);
    for &link in &links {
        if map.open(link) && !visited.contains(&link) {
            let found = map.stellars(link).into_iter().find(|&(_, ok)| ok);
            if let Some((stellar, _)) = found {
                return Some((stellar, link));
            }
        }
    }
    for &link in &links {
        if map.open(link)
            && !visited.contains(&link)
            && let Some(found) = search(link, map, visited)
        {
            return Some(found);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bay::FIGHTER_BAY;
    use crate::catalog::WeaponRecord;
    use crate::handling::ShipFields;
    use nova_sim_catalog::fixture::{outfit, ship, weapon};

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

    /// A system of a constructed galaxy: its links, whether it is open,
    /// and its stellars with whether each qualifies.
    type System = (Vec<i16>, bool, Vec<(i16, bool)>);

    /// A constructed galaxy.
    #[derive(Default)]
    struct Map {
        systems: BTreeMap<i16, System>,
    }

    impl Map {
        fn with(mut self, id: i16, links: &[i16], open: bool, stellars: &[(i16, bool)]) -> Self {
            self.systems
                .insert(id, (links.to_vec(), open, stellars.to_vec()));
            self
        }
    }

    impl EscapeMap for Map {
        fn raw_links(&self, system: SystemId) -> Vec<SystemId> {
            self.systems
                .get(&system.0)
                .map(|(links, ..)| links.iter().map(|&id| SystemId(id)).collect())
                .unwrap_or_default()
        }

        fn open(&self, system: SystemId) -> bool {
            self.systems
                .get(&system.0)
                .is_some_and(|(_, open, _)| *open)
        }

        fn stellars(&self, system: SystemId) -> Vec<(StellarId, bool)> {
            self.systems
                .get(&system.0)
                .map(|(.., stellars)| {
                    stellars
                        .iter()
                        .map(|&(id, ok)| (StellarId(id), ok))
                        .collect()
                })
                .unwrap_or_default()
        }
    }

    fn found(map: &Map, from: i16) -> Option<(i16, i16)> {
        escape_stellar(SystemId(from), map).map(|(stellar, system)| (stellar.0, system.0))
    }

    #[test]
    fn never_picks_a_stellar_in_the_start_system() {
        let map = Map::default()
            .with(128, &[129, 128], true, &[(200, true)])
            .with(129, &[128], true, &[]);
        assert_eq!(found(&map, 128), None);
    }

    #[test]
    fn takes_the_first_qualifying_stellar_of_the_first_qualifying_neighbour_before_recursing() {
        let map = Map::default()
            .with(128, &[129, 130], true, &[])
            .with(129, &[131], true, &[(201, false)])
            .with(130, &[], true, &[(202, false), (203, true), (204, true)])
            .with(131, &[], true, &[(205, true)]);
        assert_eq!(
            found(&map, 128),
            Some((203, 130)),
            "130 before 129's link 131"
        );
        let deeper = map.with(130, &[], true, &[]);
        assert_eq!(found(&deeper, 128), Some((205, 131)), "then depth first");
    }

    #[test]
    fn skips_a_system_that_is_not_visible_or_not_explored() {
        let map = Map::default()
            .with(128, &[129, 130], true, &[])
            .with(129, &[131], false, &[(201, true)])
            .with(130, &[], true, &[(202, true)])
            .with(131, &[], true, &[(203, true)]);
        assert_eq!(found(&map, 128), Some((202, 130)));
        let closed = map.with(130, &[], false, &[(202, true)]);
        assert_eq!(found(&closed, 128), None, "nor searched through");
    }

    #[test]
    fn finds_nothing_and_returns_none() {
        let map = Map::default()
            .with(128, &[129], true, &[])
            .with(129, &[128, 130], true, &[(201, false)])
            .with(130, &[129], true, &[]);
        assert_eq!(found(&map, 128), None);
        assert_eq!(found(&Map::default(), 128), None, "no galaxy");
    }

    #[test]
    fn follows_raw_links_including_repeats_and_ignores_missing_systems() {
        let map = Map::default()
            .with(128, &[999, 129, 129, 130], true, &[])
            .with(129, &[131], true, &[])
            .with(130, &[], true, &[(202, true)])
            .with(131, &[], true, &[(203, true)]);
        assert_eq!(
            found(&map, 128),
            Some((202, 130)),
            "130 tested before recursing"
        );
        let looped = Map::default()
            .with(128, &[129], true, &[])
            .with(129, &[129, 128, 129, 130], true, &[])
            .with(130, &[], true, &[(204, true)]);
        assert_eq!(found(&looped, 128), Some((204, 130)));
    }

    #[test]
    fn skips_an_inactive_unlandable_gate_wormhole_shipyardless_or_min_status_barred_stellar() {
        use crate::catalog::LandingSite;
        use crate::landing::{ANY_RECORD, NEVER_BY_RECORD, StellarFlags};
        let yard = LandingSite {
            flags: StellarFlags::CAN_LAND | StellarFlags::SHIPYARD,
            ..nova_sim_catalog::fixture::planet(128, 0.0, 0.0)
        };
        let record = 10;
        assert!(qualifies(&yard, true, record), "the control");
        assert!(!qualifies(&yard, false, record), "inactive");
        let unlandable = LandingSite {
            flags: StellarFlags::SHIPYARD,
            ..yard
        };
        assert!(!qualifies(&unlandable, true, record), "unlandable");
        let destroyed = LandingSite {
            destroyed: true,
            ..yard
        };
        assert!(!qualifies(&destroyed, true, record), "destroyed");
        for gate in [0x1000, 0x2000] {
            let gate = LandingSite {
                flags2: gate,
                ..yard
            };
            assert!(!qualifies(&gate, true, record), "{:#x}", gate.flags2);
        }
        let shipyardless = LandingSite {
            flags: StellarFlags::CAN_LAND,
            ..yard
        };
        assert!(!qualifies(&shipyardless, true, record), "no shipyard");
        let barred = |min_status| LandingSite { min_status, ..yard };
        assert!(qualifies(&barred(record), true, record), "at MinStatus");
        assert!(!qualifies(&barred(record + 1), true, record), "below it");
        assert!(!qualifies(&barred(NEVER_BY_RECORD), true, i16::MAX));
        assert!(qualifies(&barred(ANY_RECORD), true, i16::MIN));
    }
}
