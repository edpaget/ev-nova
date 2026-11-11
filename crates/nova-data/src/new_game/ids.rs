//! Nova IDs for the bible's ids: one pure allocation, per kind.
//!
//! Each kind has its range: [`GOVT_IDS`] for factions, [`SYSTEM_IDS`] and
//! [`STELLAR_IDS`] for systems and stellars, [`SHIP_IDS`] for ships,
//! [`OUTFIT_IDS`] for outfits, kits and the bays' fighters,
//! [`WEAPON_IDS`] for the outfits that are
//! weapons, [`CHARACTER_IDS`] for starts, [`MISSION_IDS`] for the
//! `take_point` missions' blockades ([`super::blockades`]),
//! [`COMMODITY_SLOTS`] then [`JUNK_IDS`] for goods ([`super::goods`]), and
//! [`WEAPON_SOUND_IDS`] for the sounds weapons fire ([`super::sounds`]). The placeholder traffic
//! `düde` is [`CIVILIAN_DUDE`] and the one `ïntf` [`DEFAULT_INTERFACE`];
//! neither depends on the data.
//!
//! The holders' guard `düde`s number after [`CIVILIAN_DUDE`]
//! ([`super::blockades`]), and the fleets' after the last guard, in fleet
//! id order, within [`DUDE_IDS`] ([`super::traffic`]). Both are unpinned
//! and kept out of [`IdTable`]: no saved pilot holds a `düde` ID (a
//! blockade's guards are saved by their `mïsn`), so a fleet renumbered
//! when a new holder's guards come in ahead of it changes nothing a pilot
//! keeps.
//!
//! # Pinning
//!
//! A saved pilot holds its system, stellar, ship, escorts and missions
//! (a blockade's `mïsn` among them) by Nova ID,
//! so the IDs must not move as the data grows. The data folder's ID
//! table, `nova-ids.json` ([`super::pins`]), pins them:
//!
//! - A record the table names keeps the ID it gives.
//! - Every other record is numbered in its kind's id order (the order the
//!   loader's maps give), from one above the highest ID the table holds in
//!   its range, or from 128, the first ID Nova gives data, when it holds
//!   none ([`after_pins`], [`allocate_pinned`]).
//! - Two kinds share a range. The jump points' gates share
//!   [`STELLAR_IDS`]: the unpinned stellars come first, then the unpinned
//!   gates in (system id, point id) order. The mod kits and the bays'
//!   fighters share [`OUTFIT_IDS`]: the unpinned outfits, then the
//!   unpinned kits in kit id order, then the unpinned fighters in bay id
//!   order ([`super::munitions`]). "The highest ID the table holds" counts
//!   every sharing kind's pins.
//! - An entry whose record has left the folder is retired: it stays in
//!   the table, so its ID, and every ID below the highest, is never handed
//!   out again.
//! - Goods fill the standard commodity slots first, then `jünk` IDs: each
//!   good the table pins in `commodities` keeps its slot, and each in
//!   `junk` its `jünk`; every other good takes the next free slot after
//!   the highest pinned one, in id order, and those left over the next
//!   `jünk` IDs after the highest pinned one. A slot holds a pilot's
//!   cargo as `Commodity(slot)`, so a pinned good keeps meaning the same
//!   good in a saved hold.
//! - A record past its range's end gets no ID, and the caller leaves it
//!   out.
//!
//! With no table, or an empty one, nothing is pinned: every kind numbers
//! from 128, and the gates and kits come after the last stellar and
//! outfit, and the fighters after the last kit. Loading never writes the table; `nova-dump --pin-ids
//! <GAME_DATA>` does, an authoring step to run whenever a record is
//! added.

use std::collections::BTreeMap;
use std::ops::RangeInclusive;

use nova_galaxy_format::id::{Id, PointRef};

/// The `gövt` IDs Nova gives data: 128 to 383.
pub const GOVT_IDS: RangeInclusive<i16> = 128..=383;

/// The `sÿst` IDs Nova gives data: 128 to 2175.
pub const SYSTEM_IDS: RangeInclusive<i16> = 128..=2175;

/// The `spöb` IDs Nova gives data: 128 to 2175.
pub const STELLAR_IDS: RangeInclusive<i16> = 128..=2175;

/// The `shïp` IDs Nova gives data: 128 to 895.
pub const SHIP_IDS: RangeInclusive<i16> = 128..=895;

/// The `oütf` IDs Nova gives data: 128 to 639, the range the `G` and `D`
/// set operators act on.
pub const OUTFIT_IDS: RangeInclusive<i16> = 128..=639;

/// The `wëap` IDs Nova gives data: 128 to 383.
pub const WEAPON_IDS: RangeInclusive<i16> = 128..=383;

/// The `snd ` IDs a weapon's sound takes: 200 to 299. A weapon fires
/// `snd ` 200 plus its `wëap`'s `Sound` (nova-view's
/// `flight::data::FIRST_WEAPON_SOUND`), and these are the weapon handles
/// below the first explosion's, so no fixed cue shares one
/// (`nova_sim::sound::WEAPON_SOUNDS` is the engine's own).
pub const WEAPON_SOUND_IDS: RangeInclusive<i16> = 200..=299;

/// The `chär` IDs the new game's starts take: 128 to 255.
pub const CHARACTER_IDS: RangeInclusive<i16> = 128..=255;

/// The `mïsn` IDs Nova gives data: 128 to 1127, the range the `S`, `A`
/// and `F` set operators act on.
pub const MISSION_IDS: RangeInclusive<i16> = 128..=1127;

/// The standard commodity slots a good can take: 0 (food, in stock
/// terms) to 5 (equipment). The engine always has all six
/// (`nova_sim_state::market::COMMODITIES`).
pub const COMMODITY_SLOTS: RangeInclusive<i16> = 0..=5;

/// The `jünk` IDs Nova gives data: 128 to 255, whose scoop contents are
/// 1000 to 1127.
pub const JUNK_IDS: RangeInclusive<i16> = 128..=255;

/// The `düde` IDs a system's `DudeTypes` reads as a `düde`: 128 to 639.
/// Any value above them is unused.
pub const DUDE_IDS: RangeInclusive<i16> = 128..=639;

/// The placeholder traffic `düde`, "Civilian traffic", flown in each
/// system that names no traffic of its own (see [`super::fleet`]).
pub const CIVILIAN_DUDE: i16 = 128;

/// The one `ïntf`, the placeholder status bar
/// ([`super::records::default_interface`]): the one a ship of no
/// government, or of a `gövt` whose `Interface` is below 128, shows
/// (nova-view's `hud::DEFAULT_INTERFACE`).
pub const DEFAULT_INTERFACE: i16 = 128;

/// The `dësc` holding the About text, `game.md`'s body
/// ([`super::descriptions`]): the one the menu bar's About and the main
/// menu's About button show (`nova`'s `ABOUT_TEXT`).
pub const ABOUT_TEXT: i16 = 32767;

/// The Nova ID of each record that has one, by its bible id, per kind.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct IdTable {
    /// Each faction's `gövt`.
    pub factions: BTreeMap<Id, i16>,
    /// Each system's `sÿst`.
    pub systems: BTreeMap<Id, i16>,
    /// Each stellar's `spöb`.
    pub stellars: BTreeMap<Id, i16>,
    /// Each jump point's hypergate `spöb`, by `system/point`.
    pub gates: BTreeMap<PointRef, i16>,
    /// Each ship's `shïp` and `shän`, a hull's or a variant's.
    pub ships: BTreeMap<Id, i16>,
    /// Each start's `chär`.
    pub starts: BTreeMap<Id, i16>,
    /// Each outfit's `oütf`.
    pub outfits: BTreeMap<Id, i16>,
    /// Each outfit's `wëap`, for the outfits that are weapons, by the
    /// outfit's id.
    pub weapons: BTreeMap<Id, i16>,
    /// Each mod kit's `oütf`.
    pub kits: BTreeMap<Id, i16>,
    /// Each fighter bay's fighters `oütf`, the rounds of its `wëap`, by
    /// the bay outfit's id ([`super::munitions`]).
    pub fighters: BTreeMap<Id, i16>,
    /// Each `take_point` mission's blockade `mïsn`, by the mission's id
    /// ([`super::blockades`]).
    pub blockades: BTreeMap<Id, i16>,
    /// Each good's standard commodity slot, for the goods that take one
    /// ([`COMMODITY_SLOTS`], [`super::goods`]).
    pub commodities: BTreeMap<Id, i16>,
    /// Each good's `jünk`, for the goods past the standard slots
    /// ([`JUNK_IDS`]).
    pub junk: BTreeMap<Id, i16>,
    /// Each sound's `snd `, for the sounds a weapon fires
    /// ([`WEAPON_SOUND_IDS`], [`super::sounds`]), by the sound's id.
    pub sounds: BTreeMap<Id, i16>,
}

/// Numbers `ids`, in the order given, from the start of `range`: the
/// table of those that fit, and those past its end, in order.
pub fn allocate<'a>(
    ids: impl IntoIterator<Item = &'a Id>,
    range: RangeInclusive<i16>,
) -> (BTreeMap<Id, i16>, Vec<&'a Id>) {
    let mut free = range;
    allocate_pinned(ids, &BTreeMap::new(), &mut free)
}

/// The IDs of `range` left for unpinned records: those above every one
/// of `pinned`, which lie within it, or the whole range with none.
#[must_use]
pub fn after_pins<'a>(
    range: RangeInclusive<i16>,
    pinned: impl IntoIterator<Item = &'a i16>,
) -> RangeInclusive<i16> {
    let start = pinned
        .into_iter()
        .max()
        .map_or(*range.start(), |&highest| highest + 1);
    start..=*range.end()
}

/// Numbers `keys` in the order given: each `pins` names keeps its pin,
/// and each other takes the next ID of `free`. The table of those
/// numbered, and the unpinned ones `free` had no ID left for, in order.
///
/// Two kinds that share a range number from one `free` in turn, so the
/// second's unpinned records come after the first's.
pub fn allocate_pinned<'a, K: Ord + Clone + 'a>(
    keys: impl IntoIterator<Item = &'a K>,
    pins: &BTreeMap<K, i16>,
    free: &mut RangeInclusive<i16>,
) -> (BTreeMap<K, i16>, Vec<&'a K>) {
    let mut table = BTreeMap::new();
    let mut past = Vec::new();
    for key in keys {
        match pins.get(key).copied().or_else(|| free.next()) {
            Some(nova) => {
                table.insert(key.clone(), nova);
            }
            None => past.push(key),
        }
    }
    (table, past)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(text: &str) -> Id {
        Id::try_from(text.to_owned()).expect("an id")
    }

    #[test]
    fn counts_up_from_the_range_s_start_in_the_order_given() {
        let ids = [id("alpha"), id("beta"), id("gamma")];
        let (table, past) = allocate(&ids, SYSTEM_IDS);
        assert_eq!(
            table.into_iter().collect::<Vec<_>>(),
            [(id("alpha"), 128), (id("beta"), 129), (id("gamma"), 130)]
        );
        assert!(past.is_empty());
    }

    #[test]
    fn ids_past_the_range_s_end_get_none() {
        let ids = [id("a"), id("b"), id("c"), id("d")];
        let (table, past) = allocate(&ids, 128..=129);
        assert_eq!(
            table.into_iter().collect::<Vec<_>>(),
            [(id("a"), 128), (id("b"), 129)]
        );
        assert_eq!(past, [&id("c"), &id("d")]);
    }

    fn pins(entries: &[(&str, i16)]) -> BTreeMap<Id, i16> {
        entries
            .iter()
            .map(|&(text, nova)| (id(text), nova))
            .collect()
    }

    #[test]
    fn a_pinned_id_keeps_its_id_whatever_its_sort_position() {
        let pinned = pins(&[("zeta", 128)]);
        let mut free = after_pins(SYSTEM_IDS, pinned.values());
        let ids = [id("alpha"), id("zeta")];
        let (table, past) = allocate_pinned(&ids, &pinned, &mut free);
        assert_eq!(
            table.into_iter().collect::<Vec<_>>(),
            [(id("alpha"), 129), (id("zeta"), 128)]
        );
        assert!(past.is_empty());
    }

    #[test]
    fn unpinned_ids_start_above_the_highest_pin_a_retired_one_included() {
        let pinned = pins(&[("beta", 130), ("gone", 140)]);
        let mut free = after_pins(SYSTEM_IDS, pinned.values());
        let ids = [id("alpha"), id("beta"), id("gamma")];
        let (table, _) = allocate_pinned(&ids, &pinned, &mut free);
        assert_eq!(
            table.into_iter().collect::<Vec<_>>(),
            [(id("alpha"), 141), (id("beta"), 130), (id("gamma"), 142)],
            "128, 129 and 131 to 139 are never handed out"
        );
    }

    #[test]
    fn no_pins_number_from_the_range_s_start() {
        assert_eq!(after_pins(SYSTEM_IDS, &[]), SYSTEM_IDS);
        assert_eq!(after_pins(128..=200, &[150, 140]), 151..=200);
    }

    #[test]
    fn unpinned_ids_past_the_range_s_end_get_none_and_pinned_ones_keep_theirs() {
        let pinned = pins(&[("b", 129)]);
        let mut free = after_pins(128..=129, pinned.values());
        let ids = [id("a"), id("b"), id("c")];
        let (table, past) = allocate_pinned(&ids, &pinned, &mut free);
        assert_eq!(table.into_iter().collect::<Vec<_>>(), [(id("b"), 129)]);
        assert_eq!(past, [&id("a"), &id("c")]);
    }

    #[test]
    fn a_shared_range_numbers_on_where_the_first_kind_stopped() {
        let mut free = after_pins(128..=200, &[]);
        let first = [id("a"), id("b")];
        let second = [id("c")];
        let (_, _) = allocate_pinned(&first, &BTreeMap::new(), &mut free);
        let (table, _) = allocate_pinned(&second, &BTreeMap::new(), &mut free);
        assert_eq!(table[&id("c")], 130);
    }

    #[test]
    fn the_ranges_are_nova_s() {
        assert_eq!(GOVT_IDS, 128..=383);
        assert_eq!(SYSTEM_IDS, 128..=2175);
        assert_eq!(STELLAR_IDS, 128..=2175);
        assert_eq!(SHIP_IDS, 128..=895);
        assert_eq!(OUTFIT_IDS, 128..=639);
        assert_eq!(WEAPON_IDS, 128..=383);
        assert_eq!(MISSION_IDS, 128..=1127);
        assert_eq!(COMMODITY_SLOTS, 0..=5);
        assert_eq!(JUNK_IDS, 128..=255);
        assert_eq!(WEAPON_SOUND_IDS, 200..=299);
    }

    #[test]
    fn a_full_range_numbers_its_last_id() {
        let ids: Vec<_> = (0..256).map(|n| id(&format!("f{n}"))).collect();
        let (table, past) = allocate(&ids, GOVT_IDS);
        assert_eq!(table.len(), 256);
        assert_eq!(table[&id("f255")], 383);
        assert!(past.is_empty());
        let more: Vec<_> = (0..257).map(|n| id(&format!("f{n}"))).collect();
        let (_, past) = allocate(&more, GOVT_IDS);
        assert_eq!(past, [&id("f256")]);
    }
}
