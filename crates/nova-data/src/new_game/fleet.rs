//! Hulls as `shïp`s and size-only `shän`s, the placeholder traffic `düde`,
//! and starts as `chär`s.
//!
//! The bible has no traffic, no start date and no Nova-style stats, so
//! some values are placeholders, each a constant here:
//!
//! - **Hulls** ([`ship`]). Only ships with no `base` and no `mods` map;
//!   variants wait for their mod kits (`play-the-sample` phase 3). The
//!   bible's `speed` is Nova's (speed/100 pixels a tick), `shields`,
//!   `hull` and `cargo` are its `Shield`, `Armor` and `Holds`. Its
//!   acceleration is its speed and its turn rate a quarter of its
//!   `handling` ([`HANDLING_PER_TURN`]), both placeholders, and every hull
//!   holds [`HULL_FUEL`] and a crew of one. Outfits aren't mapped, so the
//!   hulls are unarmed.
//! - **Size** ([`ship_anim`]). A hull's `shän` sets only `BaseXSize`
//!   (the beam) and `BaseYSize` (the length), in pixels at
//!   [`PX_PER_METRE`]: the simulation collides and hovers at that size.
//!   It names no sprite, so the view draws the hull as its crossed box
//!   until `phase-4-draw-flat-outlines` draws outlines.
//! - **Traffic** ([`civilian_traffic`]). One `düde`,
//!   [`CIVILIAN_DUDE`], "Civilian traffic":
//!   every hull of `style: civilian`, at even odds, flown as wimpy traders
//!   of no government. Every system names it in `DudeTypes[0]` at 100%,
//!   with [`TRAFFIC_SHIPS`] ships on average. Factions' warships and
//!   fleets wait for the bible to author traffic.
//! - **Starts** ([`character`]). Each start is a `chär` in its hull, in
//!   its system, with its cash, on [`START_DATE`].

use std::collections::BTreeMap;

use nova_galaxy_format::fleet::{Ship as BibleShip, Style};
use nova_galaxy_format::id::Id;
use nova_galaxy_format::load::{Doc, Galaxy, Problem};
use nova_galaxy_format::model::Start;

use super::ids::{CHARACTER_IDS, CIVILIAN_DUDE, IdTable, SHIP_IDS};
use super::map::{absent, number};
use super::records::{neutral_character, neutral_dude, neutral_ship, neutral_ship_anim};
use crate::decode::Entry;
use crate::records::character::Character;
use crate::records::dude::Dude;
use crate::records::ship::Ship;
use crate::records::ship_anim::ShipAnim;
use crate::registry::AnyRecord;
use crate::wire::id::{ShipId, SystemId};

/// Pixels a metre of a hull's `size` takes (provisional: the scale is a
/// guess, kept here alone).
pub const PX_PER_METRE: f64 = 1.0;

/// Every hull's fuel: 300, three jumps, a placeholder above the 99 the
/// engine's reading needs for an NPC to jump out where a ruleset lets it.
pub const HULL_FUEL: i16 = 300;

/// How much `handling` makes a degree of turn rate: a hull's `Maneuver`
/// is its `handling` / 4 (a placeholder).
pub const HANDLING_PER_TURN: u32 = 4;

/// How many ships each system's traffic holds on average (`AvgShips`, a
/// placeholder).
pub const TRAFFIC_SHIPS: i16 = 4;

/// The AI type the civilian traffic flies as: 1, a wimpy trader.
const WIMPY_TRADER: i16 = 1;

/// The date every start begins on, as (day, month, year): 1 January 1200,
/// a placeholder until the bible dates its starts.
pub const START_DATE: (i16, i16, i16) = (1, 1, 1200);

/// The ships of `galaxy` that are hulls: no `base` and no `mods`.
pub fn hulls(galaxy: &Galaxy) -> impl Iterator<Item = (&Id, &Doc<BibleShip>)> {
    galaxy
        .ships
        .iter()
        .filter(|(_, doc)| doc.data.base.is_none() && doc.data.mods.is_empty())
}

/// Numbers the hulls of `galaxy` within [`SHIP_IDS`].
pub fn number_hulls(galaxy: &Galaxy, problems: &mut Vec<Problem>) -> BTreeMap<Id, i16> {
    number(hulls(galaxy), SHIP_IDS, "shïp", problems)
}

/// `stat` as a record's word: none is 0, and one past `i16` the most.
fn word(stat: Option<u32>) -> i16 {
    stat.map_or(0, |stat| i16::try_from(stat).unwrap_or(i16::MAX))
}

/// The `shïp` of the hull `doc`, numbered `nova` (see the module docs).
#[must_use]
pub fn ship(nova: i16, doc: &Doc<BibleShip>) -> Entry<AnyRecord> {
    let stats = doc.data.stats;
    let speed = word(stats.speed);
    let ship = Ship {
        speed,
        accel: speed,
        maneuver: word(stats.handling.map(|handling| handling / HANDLING_PER_TURN)),
        shield: word(stats.shields),
        armor: word(stats.hull),
        holds: word(stats.cargo),
        fuel: HULL_FUEL,
        crew: 1,
        ..neutral_ship()
    };
    Entry {
        id: nova,
        name: Some(doc.data.name.clone()),
        record: AnyRecord::Ship(Box::new(ship)),
    }
}

/// `metres` in whole pixels at [`PX_PER_METRE`], held within `i16`.
fn pixels(metres: f64) -> i16 {
    let px = (metres * PX_PER_METRE).round();
    // Held within `i16` first, so the cast can't truncate.
    #[allow(clippy::cast_possible_truncation)]
    let px = px.clamp(f64::from(i16::MIN), f64::from(i16::MAX)) as i16;
    px
}

/// The size-only `shän` of the hull `doc`, numbered `nova`; none for a hull
/// with no `size`, which the simulation collides as its default disc.
#[must_use]
pub fn ship_anim(nova: i16, doc: &Doc<BibleShip>) -> Option<Entry<AnyRecord>> {
    let size = doc.data.size?;
    let anim = ShipAnim {
        base_x_size: pixels(size.beam),
        base_y_size: pixels(size.length),
        ..neutral_ship_anim()
    };
    Some(Entry {
        id: nova,
        name: Some(doc.data.name.clone()),
        record: AnyRecord::ShipAnim(Box::new(anim)),
    })
}

/// The civilian traffic `düde`: the first 16 civilian hulls of `ships`, in
/// ID order, at even odds; none without a civilian hull.
#[must_use]
pub fn civilian_traffic(galaxy: &Galaxy, ships: &BTreeMap<Id, i16>) -> Option<Entry<AnyRecord>> {
    let civilian: Vec<i16> = ships
        .iter()
        .filter(|(ship, _)| galaxy.ships[*ship].data.style == Some(Style::Civilian))
        .map(|(_, &nova)| nova)
        .take(16)
        .collect();
    if civilian.is_empty() {
        return None;
    }
    let odds = i16::try_from(100 / civilian.len()).unwrap_or(1).max(1);
    let mut dude = Dude {
        ai_type: WIMPY_TRADER,
        ..neutral_dude()
    };
    for (slot, &nova) in civilian.iter().enumerate() {
        dude.ship_type[slot] = Some(ShipId(nova));
        dude.probability[slot] = odds;
    }
    Some(Entry {
        id: CIVILIAN_DUDE,
        name: Some("Civilian traffic".to_owned()),
        record: AnyRecord::Dude(Box::new(dude)),
    })
}

/// The starts of `galaxy` whose ship is a mapped hull and whose system has
/// a record, numbered within [`CHARACTER_IDS`]. Each other is reported
/// and left out, but not for a file that failed to load or a record left
/// out for its ID.
pub fn number_starts(
    galaxy: &Galaxy,
    ids: &IdTable,
    problems: &mut Vec<Problem>,
) -> BTreeMap<Id, i16> {
    let mapped: Vec<(&Id, &Doc<Start>)> = galaxy
        .starts
        .iter()
        .filter(|(_, doc)| {
            let start = &doc.data;
            let has_ship = ids.ships.contains_key(&start.ship);
            let a_hull_left_out = hulls(galaxy).any(|(hull, _)| *hull == start.ship);
            if !has_ship
                && !a_hull_left_out
                && !galaxy
                    .unreadable
                    .contains(&format!("ships/{}.md", start.ship))
            {
                problems.push(Problem::new(
                    &doc.path,
                    format!(
                        "its ship `{}` is no hull in ships/; it is left out",
                        start.ship
                    ),
                ));
            }
            let has_system = ids.systems.contains_key(&start.system);
            if !has_system && absent(galaxy, &galaxy.systems, "systems", &start.system) {
                problems.push(Problem::new(
                    &doc.path,
                    format!(
                        "its system `{}` is not in systems/; it is left out",
                        start.system
                    ),
                ));
            }
            has_ship && has_system
        })
        .collect();
    number(mapped, CHARACTER_IDS, "chär", problems)
}

/// The `chär` of the start `doc`, numbered `nova` (see the module docs).
#[must_use]
pub fn character(ids: &IdTable, nova: i16, doc: &Doc<Start>) -> Entry<AnyRecord> {
    let start = &doc.data;
    let mut system = [None; 4];
    system[0] = ids.systems.get(&start.system).copied().map(SystemId);
    let (day, month, year) = START_DATE;
    let character = Character {
        cash: i32::try_from(start.cash).unwrap_or(i32::MAX),
        ship_type: ids.ships.get(&start.ship).copied().map(ShipId),
        system,
        unknown_0x134: day,
        unknown_0x136: month,
        start_year: year,
        ..neutral_character()
    };
    Entry {
        id: nova,
        name: Some(start.name.clone()),
        record: AnyRecord::Character(Box::new(character)),
    }
}
