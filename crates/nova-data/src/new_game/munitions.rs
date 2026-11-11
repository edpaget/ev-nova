//! What a launcher fires and what a bay launches: a launcher's
//! `ammunition`, a bay's `fighters`, the fighters' own `oütf`s and the
//! problems of the three words a launcher and a bay read (`guidance`,
//! `ammunition` and `fighters`). [`super::outfits`] maps the `wëap`s and
//! `oütf`s from what this works out, and [`Munitions::arm`] loads each
//! `shïp`.
//!
//! **A launcher's ammunition.** A launcher that is a weapon and names an
//! `ammunition` outfit fires its rounds when that outfit has an `oütf` and
//! is no weapon itself. The rounds are those of one `wëap`, the owner: the
//! lowest-ID `wëap` among the launchers naming that ammunition, so two
//! launchers naming one ammunition share its rounds, as stock launchers
//! sharing an `AmmoType` do. Each such launcher's `AmmoType` is the owner
//! less [`FIRST_AMMO_WEAPON`], and the ammunition's `oütf`'s first mod is
//! [`MOD_AMMO`] naming the owner (`nova_sim::combat::armament`: the rounds
//! of an outfit whose first mod is `MOD_AMMO`, @0xcab1; a launcher's
//! `AmmoType` n spends the rounds of `wëap` 128 + n).
//!
//! **A bay's fighters.** A bay naming `fighters` whose `hull` is a mapped
//! ship carrying no bay with `fighters` of its own ([`launched_hull`]) is a
//! weapon: its `AmmoType` is the hull's `shïp`, which with `Guidance` 99
//! makes it a fighter bay launching that hull, its rounds its own slot
//! (`_WeaponHasAmmo` @0xba87-0xbaa1). Its rounds are a fighters `oütf`
//! ([`fighters`]) numbered in [`OUTFIT_IDS`](super::ids::OUTFIT_IDS) after
//! the kits ([`number_fighters`]), keyed by the bay's id: named for the
//! hull, first mod [`MOD_AMMO`] naming the bay's `wëap`, of no cost, never
//! sold and with no limit. The hull is marked a fighter (`EscortType` 0),
//! so the engine groups, orders and recalls it as one
//! (`nova_sim::EscortClass::of`).
//!
//! **Loads.** Each `shïp` slot of a launcher whose ammunition resolved,
//! or of a bay launching a hull, carries `load` rounds a hardpoint: its
//! `AmmoLoad` is `load` times its `WeapCount`, held within `i16`
//! ([`Munitions::arm`]). The player's ship bought turns that load into
//! owned outfits of its ammunition, and an NPC's class flies with it
//! (`nova_sim::combat::armament`, `_DoShipyardDialog` @0x5ef3e-0x5ef56,
//! `_RandomShipSpawn` @0x3c7de-0x3c848).
//!
//! **Problems**, each against the outfit's file; the record still maps as
//! the rest of its text says:
//!
//! - a `guidance` other than `unguided` or an `ammunition` on an outfit
//!   that isn't a launcher, or `fighters` on one that isn't a bay, is
//!   ignored;
//! - a launcher of no damage, which is no weapon, ignores its `guidance`
//!   and its `ammunition`;
//! - a launcher whose ammunition isn't in `outfits/`, or is a weapon,
//!   fires without ammunition;
//! - a bay whose fighters' hull isn't in `ships/`, or carries a bay with
//!   `fighters` of its own, launches nothing.
//!
//! A hull or an ammunition left out for its chain or its ID is reported
//! already and isn't reported again.

use std::collections::{BTreeMap, BTreeSet};
use std::ops::RangeInclusive;

use nova_galaxy_format::fleet::{Guidance, Mount, Outfit as BibleOutfit, resolve};
use nova_galaxy_format::id::Id;
use nova_galaxy_format::load::{Galaxy, Problem};

use super::fleet::saturated;
use super::ids::IdTable;
use super::map::{absent, number};
use super::outfits::{MOD_AMMO, is_weapon};
use super::records::named_outfit;
use crate::decode::Entry;
use crate::records::outfit::Outfit;
use crate::records::ship::Ship;
use crate::registry::AnyRecord;

/// The `wëap` whose rounds an `AmmoType` of 0 spends: a launcher's
/// `AmmoType` is its rounds' owner less this
/// (`nova_sim::combat::weapon::FIRST_AMMO_WEAPON`).
pub const FIRST_AMMO_WEAPON: i16 = 128;

/// The `shïp` `EscortType` of a fighter (`nova_sim::EscortClass::of`).
pub const FIGHTER_ESCORT: i16 = 0;

/// The hull the bay `outfit` launches, if it launches one: it is a bay
/// naming `fighters` whose `hull` is in `ships` and carries no bay naming
/// `fighters` of its own.
#[must_use]
pub fn launched_hull<'a>(
    galaxy: &Galaxy,
    ships: &BTreeMap<Id, i16>,
    outfit: &'a BibleOutfit,
) -> Option<&'a Id> {
    let fighters = outfit
        .fighters
        .as_ref()
        .filter(|_| outfit.mount == Mount::Bay)?;
    let hull = &fighters.hull;
    (ships.contains_key(hull) && !carries_a_bay(galaxy, hull)).then_some(hull)
}

/// Whether the ship `hull`'s hardpoints, its chain's or its kits', carry a
/// bay naming `fighters`.
fn carries_a_bay(galaxy: &Galaxy, hull: &Id) -> bool {
    resolve(galaxy, hull).is_ok_and(|ship| {
        ship.hardpoints.iter().any(|placed| {
            (placed.hardpoint.outfit.as_ref())
                .and_then(|outfit| galaxy.outfits.get(outfit))
                .is_some_and(|outfit| {
                    outfit.data.mount == Mount::Bay && outfit.data.fighters.is_some()
                })
        })
    })
}

/// Numbers the fighters of each bay among `weapons` within
/// [`OUTFIT_IDS`](super::ids::OUTFIT_IDS), by the bay's id: each `pins`
/// names at its pin, and each other from `free`, after the kits.
pub fn number_fighters(
    galaxy: &Galaxy,
    weapons: &BTreeMap<Id, i16>,
    pins: &BTreeMap<Id, i16>,
    free: &mut RangeInclusive<i16>,
    problems: &mut Vec<Problem>,
) -> BTreeMap<Id, i16> {
    let bays = (galaxy.outfits.iter())
        .filter(|(id, doc)| doc.data.mount == Mount::Bay && weapons.contains_key(*id));
    number(bays, pins, free, "oütf", problems)
}

/// What the launchers fire and the bays launch, worked out once from the
/// galaxy and its IDs ([`munitions`]).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Munitions {
    /// Each launcher's and bay's `AmmoType`, by its outfit's id: a
    /// launcher's its rounds' owner less [`FIRST_AMMO_WEAPON`], a bay's
    /// its hull's `shïp`. A weapon not here fires without ammunition.
    pub ammo_types: BTreeMap<Id, i16>,
    /// Each ammunition outfit's owner, the `wëap` whose rounds it is, by
    /// the ammunition's id.
    pub ammunition: BTreeMap<Id, i16>,
    /// The rounds each hardpoint carrying a weapon comes with, by its
    /// `wëap`: a launcher's whose ammunition resolved, and a bay's.
    pub loads: BTreeMap<i16, u32>,
    /// The ships a bay launches.
    pub carried: BTreeSet<Id>,
}

impl Munitions {
    /// Gives `ship`, the `shïp` of the ship `id`, each stock slot's
    /// `AmmoLoad`, its weapon's load a hardpoint times its `WeapCount`
    /// held within `i16`, and marks it a fighter when a bay launches it.
    pub fn arm(&self, id: &Id, ship: &mut Ship) {
        let slots = (ship.weap_type1_4.iter())
            .chain(&ship.weap_type5_8)
            .zip(ship.weap_count1_4.iter().chain(&ship.weap_count5_8));
        let loads: Vec<i16> = slots
            .map(|(weapon, &count)| {
                let load = weapon.and_then(|weapon| self.loads.get(&weapon.0)).copied();
                saturated(i64::from(load.unwrap_or(0)) * i64::from(count))
            })
            .collect();
        ship.ammo_load1_4.copy_from_slice(&loads[..4]);
        ship.ammo_load5_8.copy_from_slice(&loads[4..]);
        if self.carried.contains(id) {
            ship.escort_type = FIGHTER_ESCORT;
        }
    }
}

/// What `galaxy`'s launchers fire and its bays launch, under `ids` (see
/// the module docs), each problem pushed onto `problems`.
pub fn munitions(galaxy: &Galaxy, ids: &IdTable, problems: &mut Vec<Problem>) -> Munitions {
    let mut munitions = Munitions::default();
    for (id, doc) in &galaxy.outfits {
        let data = &doc.data;
        let mut report = |why: String| problems.push(Problem::new(&doc.path, why));
        let named = [
            ("guidance", data.guidance != Guidance::Unguided),
            ("ammunition", data.ammunition.is_some()),
        ];
        for (word, _) in named.into_iter().filter(|&(_, named)| named) {
            if data.mount != Mount::Launcher {
                report(format!(
                    "its `{word}` is read only on a launcher; it is ignored"
                ));
            } else if !is_weapon(data) {
                report(format!(
                    "it does no damage, so it is no weapon; its `{word}` is ignored"
                ));
            }
        }
        let Some(fighters) = &data.fighters else {
            continue;
        };
        if data.mount != Mount::Bay {
            report("its `fighters` is read only on a bay; it is ignored".to_owned());
            continue;
        }
        let hull = &fighters.hull;
        let Some(&ship) = ids.ships.get(hull) else {
            if absent(galaxy, &galaxy.ships, "ships", hull) {
                report(format!(
                    "its fighters' hull `{hull}` is not in ships/; it launches nothing"
                ));
            }
            continue;
        };
        if carries_a_bay(galaxy, hull) {
            report(format!(
                "its fighters' hull `{hull}` carries a bay of its own; it launches nothing"
            ));
            continue;
        }
        if let Some(&weapon) = ids.weapons.get(id) {
            munitions.ammo_types.insert(id.clone(), ship);
            munitions.loads.insert(weapon, fighters.load);
            munitions.carried.insert(hull.clone());
        }
    }
    launchers(galaxy, ids, &mut munitions, problems);
    munitions
}

/// Adds the rounds of `galaxy`'s launchers to `munitions`: each
/// ammunition's owner, and each launcher's `AmmoType` and load, a
/// launcher whose ammunition doesn't resolve reported.
fn launchers(
    galaxy: &Galaxy,
    ids: &IdTable,
    munitions: &mut Munitions,
    problems: &mut Vec<Problem>,
) {
    let mut fed = Vec::new();
    for (id, &weapon) in &ids.weapons {
        let doc = &galaxy.outfits[id];
        let Some(ammunition) =
            (doc.data.ammunition.as_ref()).filter(|_| doc.data.mount == Mount::Launcher)
        else {
            continue;
        };
        let rounds = &ammunition.outfit;
        let report = |why: &str| {
            Problem::new(
                &doc.path,
                format!("its ammunition `{rounds}` {why}; it fires without ammunition"),
            )
        };
        match galaxy.outfits.get(rounds) {
            None => {
                if absent(galaxy, &galaxy.outfits, "outfits", rounds) {
                    problems.push(report("is not in outfits/"));
                }
            }
            Some(doc)
                if is_weapon(&doc.data)
                    || launched_hull(galaxy, &ids.ships, &doc.data).is_some() =>
            {
                problems.push(report("is a weapon"));
            }
            Some(_) if ids.outfits.contains_key(rounds) => {
                let owner = munitions.ammunition.entry(rounds.clone()).or_insert(weapon);
                *owner = (*owner).min(weapon);
                fed.push((id, weapon, rounds, ammunition.load));
            }
            Some(_) => {}
        }
    }
    for (id, weapon, rounds, load) in fed {
        let owner = munitions.ammunition[rounds];
        munitions
            .ammo_types
            .insert(id.clone(), owner - FIRST_AMMO_WEAPON);
        munitions.loads.insert(weapon, load);
    }
}

/// The fighters `oütf` of the bay `bay`, numbered `nova` (see the module
/// docs): named for its hull, the rounds of the bay's `wëap`.
#[must_use]
pub fn fighters(galaxy: &Galaxy, ids: &IdTable, bay: &Id, nova: i16) -> Entry<AnyRecord> {
    let hull = (galaxy.outfits[bay].data.fighters.as_ref())
        .and_then(|fighters| galaxy.ships.get(&fighters.hull))
        .map_or("", |ship| ship.data.name.as_str());
    let outfit = Outfit {
        mod_type: MOD_AMMO,
        mod_val: ids.weapons.get(bay).copied().unwrap_or(0),
        ..named_outfit(hull)
    };
    Entry {
        id: nova,
        name: Some(hull.to_owned()),
        record: AnyRecord::Outfit(Box::new(outfit)),
    }
}

#[cfg(test)]
mod tests;
