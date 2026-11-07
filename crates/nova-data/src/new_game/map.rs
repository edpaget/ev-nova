//! The mapping from a loaded [`Galaxy`] to records, one small function
//! per kind over the neutral records of [`super::records`]: factions,
//! systems and stellars here, the jump points' gates in [`super::gates`],
//! and hulls, traffic and starts in [`super::fleet`]. The module docs of
//! [`super`] list what maps to what, and each problem.

use std::collections::BTreeMap;
use std::ops::RangeInclusive;

use nova_galaxy_format::id::Id;
use nova_galaxy_format::load::{Doc, Galaxy, Problem};
use nova_galaxy_format::model::{Faction, Position, Stellar as BibleStellar};

use super::fleet::{self, TRAFFIC_SHIPS};
use super::gates;
use super::ids::{CIVILIAN_DUDE, GOVT_IDS, IdTable, STELLAR_IDS, SYSTEM_IDS, allocate};
use super::records::{
    clamp_i16, neutral_govt, neutral_stellar, neutral_system, rgb, stellar_flags, text_desc,
};
use crate::decode::Entry;
use crate::records::govt::Govt;
use crate::records::stellar::Stellar;
use crate::records::system::System;
use crate::registry::AnyRecord;
use crate::wire::flags::Flags32;
use crate::wire::id::GovtId;

/// What [`map`] made of a galaxy.
#[derive(Debug)]
pub struct Mapped {
    /// Every record, for [`crate::store::GameData::from_decoded`].
    pub records: Vec<Entry<AnyRecord>>,
    /// Each record's Nova ID, by its bible id.
    pub ids: IdTable,
    /// What couldn't be mapped as written, each against its file.
    pub problems: Vec<Problem>,
}

/// Maps `galaxy`'s factions, systems, stellars, jump points, hulls and
/// starts to `gövt`, `sÿst`, `spöb`, `dësc`, `shïp`, `shän`, `düde` and
/// `chär` records.
#[must_use]
pub fn map(galaxy: &Galaxy) -> Mapped {
    let mut problems = Vec::new();
    let factions = number(&galaxy.factions, GOVT_IDS, "gövt", &mut problems);
    let systems = number(&galaxy.systems, SYSTEM_IDS, "sÿst", &mut problems);
    let placed: BTreeMap<&Id, &Doc<BibleStellar>> = galaxy
        .stellars
        .iter()
        .filter(|(_, doc)| {
            let system = &doc.data.system;
            let has_record = systems.contains_key(system);
            if !has_record && absent(galaxy, &galaxy.systems, "systems", system) {
                problems.push(Problem::new(
                    &doc.path,
                    format!("its system `{system}` is not in systems/; it is left out"),
                ));
            }
            has_record
        })
        .collect();
    let stellars = number(placed, STELLAR_IDS, "spöb", &mut problems);
    let gates = gates::number(galaxy, &systems, &stellars, &mut problems);
    let ships = fleet::number_hulls(galaxy, &mut problems);
    let mut ids = IdTable {
        factions,
        systems,
        stellars,
        gates,
        ships,
        starts: BTreeMap::new(),
    };
    ids.starts = fleet::number_starts(galaxy, &ids, &mut problems);
    let traffic = fleet::civilian_traffic(galaxy, &ids.ships);
    let mut records = Vec::new();
    for (id, &nova) in &ids.factions {
        records.push(faction(nova, &galaxy.factions[id]));
    }
    for (id, &nova) in &ids.systems {
        records.push(system(
            galaxy,
            &ids,
            id,
            nova,
            traffic.is_some(),
            &mut problems,
        ));
    }
    for (id, &nova) in &ids.stellars {
        let doc = &galaxy.stellars[id];
        records.push(stellar(galaxy, &ids, nova, doc, &mut problems));
        let body = doc.body.trim();
        if !body.is_empty() {
            records.push(Entry {
                id: nova,
                name: Some(doc.data.name.clone()),
                record: AnyRecord::Desc(Box::new(text_desc(body))),
            });
        }
    }
    for (at, &nova) in &ids.gates {
        let doc = &galaxy.systems[&at.system()];
        if let Some(point) = doc.data.jump_points.iter().find(|p| p.id == at.point()) {
            records.push(gates::gate(
                galaxy,
                &ids,
                &doc.path,
                point,
                nova,
                &mut problems,
            ));
        }
    }
    for (id, &nova) in &ids.ships {
        let doc = &galaxy.ships[id];
        records.push(fleet::ship(nova, doc));
        records.extend(fleet::ship_anim(nova, doc));
    }
    records.extend(traffic);
    for (id, &nova) in &ids.starts {
        records.push(fleet::character(&ids, nova, &galaxy.starts[id]));
    }
    Mapped {
        records,
        ids,
        problems,
    }
}

/// Numbers `docs` in id order from the start of `range`, reporting each
/// past its end, as `what`, against its file.
pub(super) fn number<'a, T: 'a>(
    docs: impl IntoIterator<Item = (&'a Id, &'a Doc<T>)>,
    range: RangeInclusive<i16>,
    what: &str,
    problems: &mut Vec<Problem>,
) -> BTreeMap<Id, i16> {
    let last = *range.end();
    let docs: BTreeMap<&Id, &Doc<T>> = docs.into_iter().collect();
    let (table, past) = allocate(docs.keys().copied(), range);
    for id in past {
        problems.push(Problem::new(
            &docs[id].path,
            format!("it is past the last {what} ID, {last}; it is left out"),
        ));
    }
    table
}

/// Whether `galaxy` holds nothing at `dir/id`, so a reference to it is
/// reported: it neither loaded into `loaded` nor failed to load.
pub(super) fn absent<T>(galaxy: &Galaxy, loaded: &BTreeMap<Id, T>, dir: &str, id: &Id) -> bool {
    !loaded.contains_key(id) && !galaxy.unreadable.contains(&format!("{dir}/{id}.md"))
}

/// The `gövt` of `faction`, its colour as both colours.
fn faction(nova: i16, faction: &Doc<Faction>) -> Entry<AnyRecord> {
    let color = rgb(&faction.data.color);
    let govt = Govt {
        color,
        ship_color: color,
        ..neutral_govt()
    };
    Entry {
        id: nova,
        name: Some(faction.data.name.clone()),
        record: AnyRecord::Govt(Box::new(govt)),
    }
}

/// The `sÿst` of the system `id`: its stellars and gates in `nav_def`
/// ([`gates::nav_def`]), and, with the civilian `traffic`, it in
/// `DudeTypes[0]` at 100% with [`TRAFFIC_SHIPS`] ships on average.
fn system(
    galaxy: &Galaxy,
    ids: &IdTable,
    id: &Id,
    nova: i16,
    traffic: bool,
    problems: &mut Vec<Problem>,
) -> Entry<AnyRecord> {
    let doc = &galaxy.systems[id];
    let (x_pos, y_pos) = position(doc.data.map, "map position", &doc.path, problems);
    let nav_def = gates::nav_def(galaxy, ids, id, &doc.path, problems);
    let mut dude_types = [-1; 8];
    let mut prob = [0; 8];
    let mut avg_ships = 0;
    if traffic {
        dude_types[0] = CIVILIAN_DUDE;
        prob[0] = 100;
        avg_ships = TRAFFIC_SHIPS;
    }
    let system = System {
        x_pos,
        y_pos,
        nav_def,
        dude_types,
        prob,
        avg_ships,
        govt: govt(
            galaxy,
            ids,
            doc.data.government.as_ref(),
            &doc.path,
            problems,
        ),
        ..neutral_system()
    };
    Entry {
        id: nova,
        name: Some(doc.data.name.clone()),
        record: AnyRecord::System(Box::new(system)),
    }
}

/// The `spöb` of the stellar `doc`.
fn stellar(
    galaxy: &Galaxy,
    ids: &IdTable,
    nova: i16,
    doc: &Doc<BibleStellar>,
    problems: &mut Vec<Problem>,
) -> Entry<AnyRecord> {
    let data = &doc.data;
    let (x_pos, y_pos) = position(data.position, "position", &doc.path, problems);
    let stellar = Stellar {
        x_pos,
        y_pos,
        flags: Flags32(stellar_flags(data.kind, &data.services)),
        govt: govt(galaxy, ids, data.owner.as_ref(), &doc.path, problems),
        ..neutral_stellar()
    };
    Entry {
        id: nova,
        name: Some(data.name.clone()),
        record: AnyRecord::Stellar(Box::new(stellar)),
    }
}

/// The `gövt` of `faction`, named in `file`: none for none, and none,
/// reported, for a faction that isn't in `factions/`.
pub(super) fn govt(
    galaxy: &Galaxy,
    ids: &IdTable,
    faction: Option<&Id>,
    file: &str,
    problems: &mut Vec<Problem>,
) -> Option<GovtId> {
    let faction = faction?;
    if let Some(&nova) = ids.factions.get(faction) {
        return Some(GovtId(nova));
    }
    if absent(galaxy, &galaxy.factions, "factions", faction) {
        problems.push(Problem::new(
            file,
            format!("the faction `{faction}` is not in factions/; it has no government"),
        ));
    }
    None
}

/// `at` as a record's position, each number outside `i16` moved to the
/// nearest and reported, as `what`, against `file`.
pub(super) fn position(
    at: Position,
    what: &str,
    file: &str,
    problems: &mut Vec<Problem>,
) -> (i16, i16) {
    let (x, x_fits) = clamp_i16(at[0]);
    let (y, y_fits) = clamp_i16(at[1]);
    if !(x_fits && y_fits) {
        problems.push(Problem::new(
            file,
            format!(
                "its {what} [{}, {}] is outside {} to {}; it is moved to [{x}, {y}]",
                at[0],
                at[1],
                i16::MIN,
                i16::MAX
            ),
        ));
    }
    (x, y)
}
