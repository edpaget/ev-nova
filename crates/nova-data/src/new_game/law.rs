//! A faction's law (`law`) and the goods it finds (a good's
//! `contraband`), mapped to the `gövt` and `jünk` fields the engine's
//! scans already read, so the stock scans need no new rule:
//!
//! - **The penalty and the fine.** A faction that keeps the law (not
//!   `lawless`) has `SmugPenalty` its `law.smuggling`, else
//!   [`LAW_SMUGGLE_PENALTY`], and `ScanFine` its `law.fine`, else
//!   [`LAW_SCAN_FINE`], each held within `i16` with a [`Problem`] when
//!   it is outside. A fine is credits: the `gövt`'s negative form, a
//!   percent of the player's cash, is never written.
//! - **The scan rate.** A law-keeping faction's `law.scan_rate` goes to
//!   the store's fittings as written ([`scan_rates`],
//!   `crate::Fittings::scan_rates`), since no `gövt` field holds it: the
//!   dock scan reads it beside the kits' attention it scales, and caps
//!   the odds (`nova_sim::dock_scan`). A faction that names none has no
//!   entry and takes the dock scan's default.
//! - **The contraband bits.** Every law-keeping faction some mapped
//!   `jünk`'s good names in its `contraband` gets a `ScanMask` bit of its
//!   own, in faction id order, from [`FIRST_CONTRABAND_BIT`] up
//!   ([`contraband_bits`]). Bit 0 stays the lawless kit's
//!   ([`LAWLESS_KIT_SCAN`]), so a kit and a good never share a bit, and
//!   so at most [`CONTRABAND_FACTIONS`] factions can be named. Its
//!   `gövt`'s `ScanMask` is [`LAWLESS_KIT_SCAN`] or'd with its bit
//!   ([`govt_law`]), and each `jünk`'s is the or of its good's factions'
//!   bits ([`junk_scan_mask`]). A scan finds a `jünk` or an outfit when
//!   its `ScanMask` shares a bit with the scanning government's, so a
//!   good is found by the factions it names and by no other.
//!
//! Only a special good (`jünk`) can be contraband: the engine's scans
//! read the `jünk` and outfit `ScanMask`s, and a standard commodity has
//! none. A good with a `contraband` list is numbered among the special
//! goods (`super::goods`).
//!
//! # Problems
//!
//! - A lawless faction that names any `law` word: it keeps no law, so
//!   its `law` is left out.
//! - A `smuggling` or `fine` outside `i16`: held at the nearest.
//! - A special good's `contraband` name that is no faction (one whose
//!   file failed to load isn't reported again), a lawless faction, or a
//!   faction past the [`CONTRABAND_FACTIONS`]th: left out, against the
//!   good's file. A name given twice in one list counts once. A good
//!   pinned to a standard slot has its names left out unchecked
//!   ([`super::goods`]).

use std::collections::{BTreeMap, BTreeSet};

use nova_galaxy_format::goods::Good;
use nova_galaxy_format::id::Id;
use nova_galaxy_format::load::{Doc, Galaxy, Problem};
use nova_galaxy_format::model::{Faction, Law};

use super::ids::IdTable;
use super::kits::{LAW_SCAN_FINE, LAW_SMUGGLE_PENALTY, LAWLESS_KIT_SCAN};
use super::map::absent;
use super::relations::held_i16;
use crate::wire::id::GovtId;

/// The `ScanMask` bit of the first faction named contraband: the bit
/// above [`LAWLESS_KIT_SCAN`].
pub const FIRST_CONTRABAND_BIT: u16 = LAWLESS_KIT_SCAN << 1;

/// The most factions the goods can name contraband: one `ScanMask` bit
/// each, above [`LAWLESS_KIT_SCAN`]'s.
pub const CONTRABAND_FACTIONS: usize = 15;

/// What a faction's law sets on its `gövt`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GovtLaw {
    /// `SmugPenalty`.
    pub smug_penalty: i16,
    /// `ScanFine`.
    pub scan_fine: i16,
    /// `ScanMask`.
    pub scan_mask: u16,
}

/// The law of `faction`, whose contraband bit is `bit` (see the module
/// docs): none for a lawless faction, a [`Problem`] if it names a `law`.
pub(super) fn govt_law(
    faction: &Doc<Faction>,
    bit: Option<u16>,
    problems: &mut Vec<Problem>,
) -> GovtLaw {
    let data = &faction.data;
    if data.lawless {
        if data.law != Law::default() {
            problems.push(Problem::new(
                &faction.path,
                "it is lawless, so its `law` is left out",
            ));
        }
        return GovtLaw::default();
    }
    let mut held = |value: Option<u32>, default: i16, what: &str| {
        value.map_or(default, |value| {
            held_i16(value.into(), what, &faction.path, problems)
        })
    };
    GovtLaw {
        smug_penalty: held(
            data.law.smuggling.map(u32::from),
            LAW_SMUGGLE_PENALTY,
            "law.smuggling",
        ),
        scan_fine: held(data.law.fine, LAW_SCAN_FINE, "law.fine"),
        scan_mask: LAWLESS_KIT_SCAN | bit.unwrap_or(0),
    }
}

/// Each law-keeping faction's own scan rate, by its `gövt`, as written:
/// only for a faction that names one.
pub(super) fn scan_rates(galaxy: &Galaxy, factions: &BTreeMap<Id, i16>) -> BTreeMap<GovtId, u32> {
    factions
        .iter()
        .filter_map(|(id, &nova)| {
            let data = &galaxy.factions[id].data;
            let rate = data.law.scan_rate.filter(|_| !data.lawless)?;
            Some((GovtId(nova), rate))
        })
        .collect()
}

/// Each faction's contraband bit, by its bible id: every law-keeping
/// mapped faction the good of some `jünk` in `ids` names, the first
/// [`CONTRABAND_FACTIONS`] in id order, from [`FIRST_CONTRABAND_BIT`].
pub(super) fn contraband_bits(galaxy: &Galaxy, ids: &IdTable) -> BTreeMap<Id, u16> {
    let named: BTreeSet<&Id> = (ids.junk.keys())
        .flat_map(|good| &galaxy.goods[good].data.contraband)
        .filter(|faction| scans(galaxy, ids, faction))
        .collect();
    named
        .into_iter()
        .take(CONTRABAND_FACTIONS)
        .zip(0..)
        .map(|(faction, n)| (faction.clone(), FIRST_CONTRABAND_BIT << n))
        .collect()
}

/// The `ScanMask` of `good`'s `jünk`: the or of the `bits` of the
/// factions its `contraband` names. Each name left out is a problem
/// against its file (see the module docs).
pub(super) fn junk_scan_mask(
    galaxy: &Galaxy,
    ids: &IdTable,
    bits: &BTreeMap<Id, u16>,
    good: &Doc<Good>,
    problems: &mut Vec<Problem>,
) -> u16 {
    let mut mask = 0;
    let mut seen: BTreeSet<&Id> = BTreeSet::new();
    for faction in &good.data.contraband {
        if !seen.insert(faction) {
            continue;
        }
        if let Some(bit) = bits.get(faction) {
            mask |= bit;
            continue;
        }
        let why = if !ids.factions.contains_key(faction) {
            absent(galaxy, &galaxy.factions, "factions", faction).then_some("no faction")
        } else if galaxy.factions[faction].data.lawless {
            Some("lawless, so it scans nobody")
        } else {
            Some("past the 15th faction named contraband")
        };
        if let Some(why) = why {
            problems.push(Problem::new(
                &good.path,
                format!("its contraband name `{faction}`, {why}; it is left out"),
            ));
        }
    }
    mask
}

/// Whether `faction` is a mapped faction that keeps the law, so its
/// scans can find a good.
fn scans(galaxy: &Galaxy, ids: &IdTable, faction: &Id) -> bool {
    ids.factions.contains_key(faction)
        && galaxy
            .factions
            .get(faction)
            .is_some_and(|doc| !doc.data.lawless)
}

#[cfg(test)]
mod tests;
