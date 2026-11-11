//! The mapping from a loaded [`Galaxy`] to records, one small function
//! per kind over the neutral records of [`super::records`]: factions,
//! systems and stellars here, a faction's relations in
//! [`super::relations`], the jump points' gates in [`super::gates`],
//! ships, the civilian traffic and starts in [`super::fleet`], the fleets
//! and a system's traffic in [`super::traffic`], outfits and weapons in
//! [`super::outfits`], what launchers fire and bays launch in
//! [`super::munitions`], mod kits in [`super::kits`], a faction's law and
//! the goods contraband for it in [`super::law`], which yards sell what
//! in [`super::yards`], the `take_point`
//! missions' guards and blockades in [`super::blockades`], goods and
//! a stellar's prices in [`super::goods`], the weapons' sound IDs and the
//! explosions' `bööm`s in [`super::sounds`], and the bodies read as
//! descriptions in [`super::descriptions`]. The module docs of
//! [`super`] list what maps to what, and each problem.

use std::collections::BTreeMap;
use std::ops::RangeInclusive;

use nova_galaxy_format::id::Id;
use nova_galaxy_format::load::{Doc, Galaxy, Problem};
use nova_galaxy_format::model::{Faction, Position, Stellar as BibleStellar};

use super::fleet;
use super::ids::{
    DEFAULT_INTERFACE, GOVT_IDS, IdTable, OUTFIT_IDS, STELLAR_IDS, SYSTEM_IDS, after_pins,
    allocate_pinned,
};
use super::kits::{self, FitBits, YARD_TECH_LAWLESS, YARD_TECH_LEGAL};
use super::law::{self, GovtLaw};
use super::munitions;
use super::outfits::{self, Fitted};
use super::records::{
    ANY_RECORD, clamp_i16, default_interface, neutral_govt, neutral_stellar, neutral_system, rgb,
    stellar_flags,
};
use super::relations::{self, Relations};
use super::yards::{self, SPECIAL_TECH_SLOTS, Sales};
use super::{blockades, descriptions, gates, goods, hazards, sounds, strings, traffic};
use crate::decode::Entry;
use crate::records::govt::Govt;
use crate::records::stellar::Stellar;
use crate::records::system::System;
use crate::registry::AnyRecord;
use crate::store::Fittings;
use crate::wire::flags::{Flags16, Flags32};
use crate::wire::id::{GovtId, OutfitId, ShipId};

/// What [`map`] made of a galaxy.
#[derive(Debug)]
pub struct Mapped {
    /// Every record, for [`crate::store::GameData::from_decoded`].
    pub records: Vec<Entry<AnyRecord>>,
    /// Each record's Nova ID, by its bible id.
    pub ids: IdTable,
    /// What couldn't be mapped as written, each against its file.
    pub problems: Vec<Problem>,
    /// Each ship's chain hardpoints, each kit's attention, hidden cargo
    /// and hardpoints, and each law-keeping faction's own scan rate, for
    /// the store.
    pub fittings: Fittings,
}

/// Maps `galaxy`'s factions, systems, stellars, goods, jump points, ships,
/// starts, outfits, mod kits and `take_point` blockades to `gövt`, `sÿst`,
/// `spöb`, `STR#`, `jünk`, `shïp`, `shän`, `düde`, `chär`, `oütf`,
/// `wëap` and `mïsn` records, and their fittings; the bodies of its
/// stellars, ships, outfits and kits, and of `game.md`, to `dësc`s
/// ([`super::descriptions`]);
/// plus the placeholder `ïntf` status bar, which a flight frame always
/// needs, and the engine's own `STR#` lists ([`super::strings`]) at each ID
/// no good's list holds, so even an empty galaxy maps them. Nothing is
/// pinned: see [`map_pinned`].
#[must_use]
pub fn map(galaxy: &Galaxy) -> Mapped {
    map_pinned(galaxy, &IdTable::default())
}

/// [`map`], with each record `pins` names keeping its pinned ID, and each
/// other numbered after the highest pin of its range
/// ([`super::ids`]).
#[must_use]
pub fn map_pinned(galaxy: &Galaxy, pins: &IdTable) -> Mapped {
    let mut problems = Vec::new();
    let (ids, fit) = number_all(galaxy, pins, &mut problems);
    let sales = yards::sales(galaxy, &ids, &mut problems);
    let civilian = fleet::civilian_traffic(galaxy, &ids.ships);
    let guards = blockades::number_guards(galaxy, &ids);
    let fleets = traffic::fleets(galaxy, &ids, &guards, &mut problems);
    let mut records = Vec::new();
    let relations = relations::relations(galaxy, &ids.factions, &mut problems);
    let contraband = law::contraband_bits(galaxy, &ids);
    for (id, &nova) in &ids.factions {
        let doc = &galaxy.factions[id];
        let law = law::govt_law(doc, contraband.get(id).copied(), &mut problems);
        records.push(faction(nova, doc, &relations[id], law));
    }
    for (id, &nova) in &ids.systems {
        let doc = &galaxy.systems[id];
        let slots = traffic::slots(galaxy, &fleets.ids, civilian.is_some(), doc, &mut problems);
        records.push(system(galaxy, &ids, id, nova, slots, &mut problems));
    }
    for (id, &nova) in &ids.stellars {
        let doc = &galaxy.stellars[id];
        records.push(stellar(galaxy, &ids, &sales, nova, doc, &mut problems));
    }
    records.extend(descriptions::descriptions(galaxy, &ids));
    records.extend(goods::standard_strings(galaxy, &ids, &mut problems));
    records.extend(goods::junk(galaxy, &ids, &contraband, &mut problems));
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
    let mut fittings = fittings(galaxy, &ids, &fit, &sales, &mut records, &mut problems);
    fittings.scan_rates = law::scan_rates(galaxy, &ids.factions);
    records.extend(civilian);
    records.extend(sounds::booms(galaxy));
    for (faction, &nova) in &guards {
        records.push(blockades::guard(galaxy, &ids, faction, nova));
    }
    records.extend(fleets.records);
    for (mission, &nova) in &ids.blockades {
        records.extend(blockades::blockade(
            galaxy,
            &ids,
            &guards,
            mission,
            nova,
            &mut problems,
        ));
    }
    for (id, &nova) in &ids.starts {
        records.push(fleet::character(&ids, nova, &galaxy.starts[id]));
    }
    records.push(Entry {
        id: DEFAULT_INTERFACE,
        name: Some("Default status bar".to_owned()),
        record: AnyRecord::Interface(Box::new(default_interface())),
    });
    strings::fill(&mut records);
    Mapped {
        records,
        ids,
        problems,
        fittings,
    }
}

/// Every record's ID under `pins`, and the kits' fit bits, which number
/// them.
fn number_all(galaxy: &Galaxy, pins: &IdTable, problems: &mut Vec<Problem>) -> (IdTable, FitBits) {
    let mut free = after_pins(GOVT_IDS, pins.factions.values());
    let factions = number(
        &galaxy.factions,
        &pins.factions,
        &mut free,
        "gövt",
        problems,
    );
    let mut free = after_pins(SYSTEM_IDS, pins.systems.values());
    let systems = number(&galaxy.systems, &pins.systems, &mut free, "sÿst", problems);
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
    let mut free = after_pins(
        STELLAR_IDS,
        pins.stellars.values().chain(pins.gates.values()),
    );
    let stellars = number(placed, &pins.stellars, &mut free, "spöb", problems);
    let gates = gates::number(galaxy, &systems, &pins.gates, &mut free, problems);
    let ships = fleet::number_ships(galaxy, &pins.ships, problems);
    let mut free = after_pins(
        OUTFIT_IDS,
        (pins.outfits.values())
            .chain(pins.kits.values())
            .chain(pins.fighters.values()),
    );
    let outfits = outfits::number_outfits(galaxy, &pins.outfits, &mut free, problems);
    let weapons = outfits::number_weapons(galaxy, &ships, &pins.weapons, problems);
    let sounds = sounds::number(galaxy, &weapons, &pins.sounds, problems);
    let fit = FitBits::of(galaxy, problems);
    let kits = kits::number_kits(galaxy, &fit, &pins.kits, &mut free, problems);
    let fighters =
        munitions::number_fighters(galaxy, &weapons, &pins.fighters, &mut free, problems);
    let blockades = blockades::number_blockades(galaxy, &gates, &pins.blockades, problems);
    let (commodities, junk) = goods::number_goods(galaxy, pins, problems);
    let mut ids = IdTable {
        factions,
        systems,
        stellars,
        gates,
        ships,
        starts: BTreeMap::new(),
        outfits,
        weapons,
        kits,
        fighters,
        blockades,
        commodities,
        junk,
        sounds,
    };
    ids.starts = fleet::number_starts(galaxy, &ids, &pins.starts, problems);
    (ids, fit)
}

/// Pushes the `shïp`s and `shän`s of `ids`' ships, the `oütf`s and
/// `wëap`s of its outfits, sold as `sales` says, and the `oütf`s of its
/// kits onto `records`, and gives their fittings.
fn fittings(
    galaxy: &Galaxy,
    ids: &IdTable,
    fit: &FitBits,
    sales: &Sales,
    records: &mut Vec<Entry<AnyRecord>>,
    problems: &mut Vec<Problem>,
) -> Fittings {
    let mut fittings = Fittings::default();
    let munitions = munitions::munitions(galaxy, ids, problems);
    for (id, &nova) in &ids.ships {
        let sold = sales.ships.get(id).copied();
        let mut ship = fleet::ship(galaxy, ids, fit, id, nova, sold, problems);
        if let AnyRecord::Ship(record) = &mut ship.record {
            munitions.arm(id, record);
        }
        records.push(ship);
        records.extend(fleet::ship_anim(galaxy, id, nova, problems));
        let spots = fleet::chain_spots(galaxy, id);
        if !spots.is_empty() {
            fittings.hulls.insert(ShipId(nova), spots);
        }
    }
    for (id, &nova) in &ids.outfits {
        let doc = &galaxy.outfits[id];
        let weapon = ids.weapons.get(id).copied();
        let sold = sales.outfits.get(id).copied();
        let fitted = match (weapon, munitions.ammunition.get(id)) {
            (Some(weapon), _) => Fitted::Weapon(weapon),
            (None, Some(&owner)) => Fitted::Rounds(owner),
            (None, None) => Fitted::Nothing,
        };
        records.push(outfits::outfit(nova, doc, fitted, sold));
        let ammo = munitions.ammo_types.get(id).copied();
        let sound = (doc.data.sound.as_ref()).and_then(|sound| ids.sounds.get(sound).copied());
        records.extend(weapon.map(|weapon| outfits::weapon(weapon, doc, ammo, sound)));
    }
    for (bay, &nova) in &ids.fighters {
        records.push(munitions::fighters(galaxy, ids, bay, nova));
    }
    for (id, &nova) in &ids.kits {
        let doc = &galaxy.mods[id];
        let require = fit.required(id).unwrap_or_default();
        records.push(kits::kit(
            galaxy,
            &ids.outfits,
            nova,
            doc,
            require,
            problems,
        ));
        fittings
            .kits
            .insert(OutfitId(nova), kits::kit_fittings(&doc.data));
    }
    fittings
}

/// Numbers `docs` in id order: each `pins` names keeps its pin, and each
/// other takes the next of `free`. Each `free` has no ID left for is
/// reported, as `what`, against its file.
pub(super) fn number<'a, T: 'a>(
    docs: impl IntoIterator<Item = (&'a Id, &'a Doc<T>)>,
    pins: &BTreeMap<Id, i16>,
    free: &mut RangeInclusive<i16>,
    what: &str,
    problems: &mut Vec<Problem>,
) -> BTreeMap<Id, i16> {
    let last = *free.end();
    let docs: BTreeMap<&Id, &Doc<T>> = docs.into_iter().collect();
    let (table, past) = allocate_pinned(docs.keys().copied(), pins, free);
    for id in past {
        problems.push(Problem::new(
            &docs[id].path,
            format!("it is past the last {what} ID, {last}; it is left out"),
        ));
    }
    table
}

/// Whether `galaxy` holds nothing at `dir/id`, so a reference to it is
/// reported: it neither loaded into `loaded` nor failed to load, and
/// `dir` itself didn't fail to list.
pub(super) fn absent<T>(galaxy: &Galaxy, loaded: &BTreeMap<Id, T>, dir: &str, id: &Id) -> bool {
    !loaded.contains_key(id) && !galaxy.is_unreadable(dir, id)
}

/// The `gövt` of `faction`, its name as its `CommName`, its colour as
/// both colours; its `SmugPenalty`, `ScanFine` and `ScanMask` from `law`
/// ([`law`]): when it keeps the law, its own or
/// [`kits::LAW_SMUGGLE_PENALTY`] and [`kits::LAW_SCAN_FINE`], and a
/// `ScanMask` of [`kits::LAWLESS_KIT_SCAN`] and its contraband bit, so
/// its scans find a
/// lawless kit and the goods contraband for it, and none when lawless;
/// its classes, allies, enemies, stance toward the player, starting
/// record and penalties from `relations` ([`relations`]).
fn faction(
    nova: i16,
    faction: &Doc<Faction>,
    relations: &Relations,
    law: GovtLaw,
) -> Entry<AnyRecord> {
    let color = rgb(&faction.data.color);
    let govt = Govt {
        color,
        ship_color: color,
        comm_name: faction.data.name.as_str().into(),
        flags: Flags16(relations.flags),
        class: relations.class,
        ally: relations.ally,
        enemy: relations.enemy,
        initial_rec: relations.initial_rec,
        shoot_penalty: relations.shoot_penalty,
        disab_penalty: relations.disab_penalty,
        kill_penalty: relations.kill_penalty,
        smug_penalty: law.smug_penalty,
        scan_fine: law.scan_fine,
        scan_mask: Flags16(law.scan_mask),
        ..neutral_govt()
    };
    Entry {
        id: nova,
        name: Some(faction.data.name.clone()),
        record: AnyRecord::Govt(Box::new(govt)),
    }
}

/// The `sÿst` of the system `id`: its stellars and gates in `nav_def`
/// ([`gates::nav_def`]), and its traffic `slots` ([`traffic::slots`]).
fn system(
    galaxy: &Galaxy,
    ids: &IdTable,
    id: &Id,
    nova: i16,
    slots: traffic::Slots,
    problems: &mut Vec<Problem>,
) -> Entry<AnyRecord> {
    let doc = &galaxy.systems[id];
    let (x_pos, y_pos) = position(doc.data.map, "map position", &doc.path, problems);
    let nav_def = gates::nav_def(galaxy, ids, id, &doc.path, problems);
    let hazards::Effects { murk, interference } = hazards::effects(&doc.data.hazards);
    let system = System {
        x_pos,
        y_pos,
        nav_def,
        dude_types: slots.dude_types,
        prob: slots.prob,
        avg_ships: slots.avg_ships,
        murk,
        interference,
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

/// The `spöb` of the stellar `doc`, its `Flags` its services' bits and
/// its standard goods' price levels ([`goods::price_flags`]): a law-keeping owner's lists legal
/// kits ([`YARD_TECH_LEGAL`]) and clears by record (`MinStatus` 0); a
/// lawless owner's, or one with none, lists every kit
/// ([`YARD_TECH_LAWLESS`]) and clears any record ([`ANY_RECORD`]). Its
/// `SpecialTech` slots are the tech numbers of what `sales` sells there
/// ([`yards`]), none for a yard selling nothing.
fn stellar(
    galaxy: &Galaxy,
    ids: &IdTable,
    sales: &Sales,
    nova: i16,
    doc: &Doc<BibleStellar>,
    problems: &mut Vec<Problem>,
) -> Entry<AnyRecord> {
    let data = &doc.data;
    let (x_pos, y_pos) = position(data.position, "position", &doc.path, problems);
    let govt = govt(galaxy, ids, data.owner.as_ref(), &doc.path, problems);
    let lawful = (data.owner.as_ref())
        .and_then(|owner| galaxy.factions.get(owner))
        .is_some_and(|owner| !owner.data.lawless)
        && govt.is_some();
    let special = sales
        .slots
        .get(&nova)
        .copied()
        .unwrap_or([-1; SPECIAL_TECH_SLOTS]);
    let stellar = Stellar {
        x_pos,
        y_pos,
        special_tech1_3: [special[0], special[1], special[2]],
        special_tech4_8: [special[3], special[4], special[5], special[6], special[7]],
        flags: Flags32(
            stellar_flags(data.kind, &data.services)
                | goods::price_flags(galaxy, ids, doc, problems),
        ),
        govt,
        tech_level: if lawful {
            YARD_TECH_LEGAL
        } else {
            YARD_TECH_LAWLESS
        },
        min_status: if lawful { 0 } else { ANY_RECORD },
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
