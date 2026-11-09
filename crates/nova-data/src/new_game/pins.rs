//! The ID table, `nova-ids.json` at the data folder's root: the Nova ID
//! each record keeps, so a pilot saved at it finds it again after the data
//! grows ([`super::ids`] has the numbering rules).
//!
//! [`read`] gives the loader the table through the same [`Source`] the
//! content comes from; loading never writes it. [`pin`] is the writer,
//! which `nova-dump --pin-ids <GAME_DATA>` runs: it merges a load's
//! [`IdTable`] into the previous table and gives the text to write.
//!
//! # Format
//!
//! One JSON object, holding one object per [`IdTable`] kind ([`KINDS`]),
//! each mapping a bible id (`system/point` for `gates`, the outfit's id
//! for `weapons`) to its ID. A kind left out pins nothing. The writer
//! writes every kind, pretty-printed with sorted keys and a trailing
//! newline, so a diff shows just the added lines.
//!
//! # Problems
//!
//! Each is reported against [`FILE`], and the game goes on:
//!
//! - A table that doesn't parse, or names a kind not in [`KINDS`]: the
//!   whole table is ignored ([`TableError`]), and every record is
//!   unpinned. The writer refuses to replace such a table.
//! - An entry whose key isn't a bible id (or, for `gates`, a
//!   `system/point`), or whose ID is outside its kind's range: the entry
//!   is ignored.
//! - Two entries of one range with one ID (a stellar and a gate, or an
//!   outfit and a kit, included): the first, in allocation order (kind,
//!   then key), keeps it, and the other is ignored, so its record is
//!   unpinned.
//! - A table that leaves out some mapped record: one problem, counting
//!   them and naming the first, which says to run the pin command.
//!
//! An entry the reader ignores is dropped by the merge, which takes the
//! load's ID for its record instead, and [`pin`] names each one in
//! [`Pinned::dropped`]. Dropping one never frees an ID: a malformed key
//! names no record, an ID out of range is no ID, and a duplicate's ID
//! stays with the entry that kept it. But a record whose entry is dropped
//! moves, so a pilot saved at its old ID would find something else there:
//! `nova-dump --pin-ids` refuses to write such a merge, and the author
//! fixes the table by hand, keeping the rule that no pin is ever dropped
//! or reused.

#[cfg(test)]
mod tests;

use std::collections::BTreeMap;
use std::fmt::Display;
use std::ops::RangeInclusive;
use std::path::Path;

use nova_galaxy_format::id::{Id, PointRef};
use nova_galaxy_format::load::Problem;
use nova_galaxy_format::source::Source;

use super::ids::{
    CHARACTER_IDS, GOVT_IDS, IdTable, OUTFIT_IDS, SHIP_IDS, STELLAR_IDS, SYSTEM_IDS, WEAPON_IDS,
};

/// The table's path in the data folder.
pub const FILE: &str = "nova-ids.json";

/// The table's kinds, in allocation order: the stellars before the gates
/// and the outfits before the kits, which share their ranges.
pub const KINDS: [&str; 9] = [
    "factions", "systems", "stellars", "gates", "ships", "starts", "outfits", "kits", "weapons",
];

/// Why a whole table is ignored, and why the writer won't replace it.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum TableError {
    /// It isn't a JSON object of objects of whole numbers.
    #[error("it doesn't parse as a table of IDs ({0})")]
    Unparsable(String),
    /// It names a kind not in [`KINDS`].
    #[error("it names the kind `{0}`, which isn't one of {kinds}", kinds = KINDS.join(", "))]
    UnknownKind(String),
}

/// A table that parsed: its pins, and why each entry left out of them was.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Table {
    /// The entries that pin a record.
    pub pins: IdTable,
    /// One message per entry ignored.
    pub problems: Vec<String>,
}

/// What [`pin`] made of a load.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pinned {
    /// The merged table: the previous one's pins, retired ones included,
    /// and the load's IDs for everything else.
    pub table: IdTable,
    /// The table as the file's text.
    pub text: String,
    /// How many of the load's records the previous table didn't pin.
    pub added: usize,
    /// One message per entry of the previous table the reader ignored,
    /// which the merged table drops, giving its record the load's ID: the
    /// [`Table::problems`] of the previous table. `nova-dump --pin-ids`
    /// refuses to write while any is here.
    pub dropped: Vec<String>,
}

/// Parses the table `bytes`.
///
/// # Errors
///
/// [`TableError`] when the whole table is ignored.
pub fn parse(bytes: &[u8]) -> Result<Table, TableError> {
    let mut raw: BTreeMap<String, BTreeMap<String, i64>> =
        serde_json::from_slice(bytes).map_err(|error| TableError::Unparsable(error.to_string()))?;
    if let Some(kind) = raw.keys().find(|kind| !KINDS.contains(&kind.as_str())) {
        return Err(TableError::UnknownKind(kind.clone()));
    }
    let mut held = Held::default();
    let mut problems = Vec::new();
    let mut take = |kind: usize| {
        let entries = raw.remove(KINDS[kind]).unwrap_or_default();
        held.read(kind, entries, &mut problems)
    };
    let pins = IdTable {
        factions: keyed(take(0)),
        systems: keyed(take(1)),
        stellars: keyed(take(2)),
        gates: keyed(take(3)),
        ships: keyed(take(4)),
        starts: keyed(take(5)),
        outfits: keyed(take(6)),
        kits: keyed(take(7)),
        weapons: keyed(take(8)),
    };
    Ok(Table { pins, problems })
}

/// Each ID read so far, by range, with the `kind/key` holding it.
#[derive(Default)]
struct Held(BTreeMap<(usize, i16), String>);

impl Held {
    /// The entries of the `kind`th kind whose key spells a key of it and
    /// whose ID is in its range and not held yet; each other's message
    /// goes onto `problems`.
    fn read(
        &mut self,
        kind: usize,
        entries: BTreeMap<String, i64>,
        problems: &mut Vec<String>,
    ) -> Vec<(String, i16)> {
        let name = KINDS[kind];
        let range = RANGES[kind].clone();
        let mut kept = Vec::new();
        for (key, nova) in entries {
            let spelled = if name == "gates" {
                PointRef::try_from(key.clone()).map(drop)
            } else {
                Id::try_from(key.clone()).map(drop)
            };
            if let Err(error) = spelled {
                problems.push(format!("its {name} entry {error}; it is ignored"));
                continue;
            }
            let Some(nova) = i16::try_from(nova).ok().filter(|nova| range.contains(nova)) else {
                problems.push(format!(
                    "its {name} entry `{key}` is {nova}, outside {} to {}; it is ignored",
                    range.start(),
                    range.end()
                ));
                continue;
            };
            let slot = (SHARED[kind], nova);
            if let Some(holder) = self.0.get(&slot) {
                problems.push(format!(
                    "its {name} entry `{key}` is {nova}, which `{holder}` holds; it is ignored"
                ));
                continue;
            }
            self.0.insert(slot, format!("{name}/{key}"));
            kept.push((key, nova));
        }
        kept
    }
}

/// The range of each of [`KINDS`].
const RANGES: [RangeInclusive<i16>; 9] = [
    GOVT_IDS,
    SYSTEM_IDS,
    STELLAR_IDS,
    STELLAR_IDS,
    SHIP_IDS,
    CHARACTER_IDS,
    OUTFIT_IDS,
    OUTFIT_IDS,
    WEAPON_IDS,
];

/// Which range each of [`KINDS`] numbers in, by the first kind of it: the
/// gates share the stellars', and the kits the outfits'.
const SHARED: [usize; 9] = [0, 1, 2, 2, 4, 5, 6, 6, 8];

/// `kept`, whose keys are spelled right, as a kind's table.
fn keyed<K: TryFrom<String> + Ord>(kept: Vec<(String, i16)>) -> BTreeMap<K, i16> {
    kept.into_iter()
        .filter_map(|(key, nova)| Some((K::try_from(key).ok()?, nova)))
        .collect()
}

/// Every entry of `table`, as `(kind, key, ID)`, in [`KINDS`] order, then
/// by key.
fn entries(table: &IdTable) -> Vec<(&'static str, String, i16)> {
    fn of<'a, K: Display>(
        kind: &'static str,
        map: &'a BTreeMap<K, i16>,
    ) -> impl Iterator<Item = (&'static str, String, i16)> + 'a {
        map.iter()
            .map(move |(key, &nova)| (kind, key.to_string(), nova))
    }
    of("factions", &table.factions)
        .chain(of("systems", &table.systems))
        .chain(of("stellars", &table.stellars))
        .chain(of("gates", &table.gates))
        .chain(of("ships", &table.ships))
        .chain(of("starts", &table.starts))
        .chain(of("outfits", &table.outfits))
        .chain(of("kits", &table.kits))
        .chain(of("weapons", &table.weapons))
        .collect()
}

/// `table` as the file's text.
#[must_use]
pub fn text(table: &IdTable) -> String {
    let mut raw: BTreeMap<&str, BTreeMap<String, i16>> =
        KINDS.iter().map(|&kind| (kind, BTreeMap::new())).collect();
    for (kind, key, nova) in entries(table) {
        raw.entry(kind).or_default().insert(key, nova);
    }
    let mut text = serde_json::to_string_pretty(&raw).expect("a map of maps of numbers prints");
    text.push('\n');
    text
}

/// Merges `loaded`, a load's IDs, into the `previous` table's bytes (none
/// when the folder has no table).
///
/// # Errors
///
/// [`TableError`] when `previous` is a table the loader ignores: writing
/// over it would lose its pins.
pub fn pin(previous: Option<&[u8]>, loaded: &IdTable) -> Result<Pinned, TableError> {
    let Table {
        pins: previous,
        problems: dropped,
    } = previous.map(parse).transpose()?.unwrap_or_default();
    let mut table = previous.clone();
    overlay(&mut table.factions, &loaded.factions);
    overlay(&mut table.systems, &loaded.systems);
    overlay(&mut table.stellars, &loaded.stellars);
    overlay(&mut table.gates, &loaded.gates);
    overlay(&mut table.ships, &loaded.ships);
    overlay(&mut table.starts, &loaded.starts);
    overlay(&mut table.outfits, &loaded.outfits);
    overlay(&mut table.kits, &loaded.kits);
    overlay(&mut table.weapons, &loaded.weapons);
    let before = entries(&previous);
    let added = entries(loaded)
        .into_iter()
        .filter(|entry| !before.contains(entry))
        .count();
    Ok(Pinned {
        text: text(&table),
        table,
        added,
        dropped,
    })
}

/// `into` with each of `from`'s entries in place of its own.
fn overlay<K: Ord + Clone>(into: &mut BTreeMap<K, i16>, from: &BTreeMap<K, i16>) {
    into.extend(from.iter().map(|(key, &nova)| (key.clone(), nova)));
}

/// The pins of the table in `source`, none when it has none or when it is
/// ignored, with each of its problems pushed onto `problems`.
pub fn read(source: &impl Source, problems: &mut Vec<Problem>) -> Option<IdTable> {
    let ignored = |why: &dyn Display| -> Problem {
        Problem::new(FILE, format!("{why}; every record is unpinned"))
    };
    let bytes = match source.bytes(FILE) {
        Ok(bytes) => bytes?,
        Err(error) => {
            problems.push(ignored(&format_args!("it can't be read ({error})")));
            return None;
        }
    };
    match parse(&bytes) {
        Ok(table) => {
            problems.extend(
                table
                    .problems
                    .into_iter()
                    .map(|line| Problem::new(FILE, line)),
            );
            Some(table.pins)
        }
        Err(error) => {
            problems.push(ignored(&error));
            None
        }
    }
}

/// The problem for the records of `loaded` that `pins` leaves out, for
/// the folder `dir`, if any are.
#[must_use]
pub fn unpinned(pins: &IdTable, loaded: &IdTable, dir: &Path) -> Option<Problem> {
    let pinned: Vec<_> = entries(pins)
        .into_iter()
        .map(|(kind, key, _)| (kind, key))
        .collect();
    let left_out: Vec<_> = entries(loaded)
        .into_iter()
        .filter(|(kind, key, _)| !pinned.contains(&(*kind, key.clone())))
        .collect();
    let (kind, key, _) = left_out.first()?;
    let count = match left_out.len() {
        1 => "1 record".to_owned(),
        n => format!("{n} records"),
    };
    Some(Problem::new(
        FILE,
        format!(
            "it leaves out {count}, the first `{kind}/{key}`; run `nova-dump --pin-ids {}` to pin them",
            dir.display()
        ),
    ))
}
