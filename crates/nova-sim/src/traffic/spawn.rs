//! Which NPCs appear, and where: a system's initial population, the
//! arrivals over time, and the fleets, rolled on the [`Chance`] port.
//!
//! The rolls are the original's (`_SetupShipsInSystem`, `_EnterMoreShips`,
//! `_HyperShipSpawn`, `_SpawnFleet`, `_HyperSpawnFleet` and
//! `_RandomShipSpawn` in the `EV Nova` executable), each written as
//! "`below(n) < k` fires":
//!
//! - **Setup** ([`initial`]) makes `AvgShips` passes. Each first rolls
//!   for a person ([`PersonRules::roll`](crate::person::PersonRules::roll),
//!   see [`person`](crate::person)): a person who may appear is placed as
//!   a `düde` ship is, an empty roll brings nothing that pass, and no
//!   person goes on to a `LinkSyst` fleet (`Rand(7) == 0`, [`link_fleet`]),
//!   else one `düde` ship, placed in the system already: x and y each
//!   `Rand(1500) - 750`, heading `Rand(360)`, at rest. Then each of the
//!   system's Person slots naming a person alive whose `ActiveOn` holds is
//!   rolled ([`PersonRules::listed`](crate::person::PersonRules::listed)),
//!   and brings its person, placed so, unless one of its name is there.
//! - **Arrivals** ([`arrivals`]), every tick while the system holds fewer
//!   NPCs than `AvgShips`, draw `Rand(500)`. On 1, when the system's
//!   `DudeTypes` name fleets, `Rand(100) + 1` at or below the sum of their
//!   `% Prob` brings one in, picked by `% Prob`; otherwise it falls
//!   through to 0's case. On 0, when the system has a `düde`, the person
//!   roll (a person who may appear, its government not derelict, jumps
//!   in; an empty roll brings nothing), else a `LinkSyst` fleet
//!   (`Rand(7) == 0`), else one `düde` ship jumps in.
//! - **A `LinkSyst` fleet** ([`link_fleet`]) draws `Rand(256)`, `flët`
//!   `128 + slot`, and comes only when that fleet's `LinkSyst` matches.
//! - **A fleet** ([`fleet`]) is its lead, with the fleet's government and
//!   its ship's `InherentAI`, and `Min + Rand(Max - Min + 1)` of each
//!   escort type, each at the lead's place plus `Rand(300) - 150` each way,
//!   with its heading and velocity, its own ship's `InherentAI`, escorting
//!   the lead. Fleets always come by hyperspace, at setup too.
//! - **A `düde` ship** is a `düde` picked by weight, then a ship by its
//!   `Probability`; its AI is the `düde`'s `AIType` when above 0, or else
//!   its ship's `InherentAI`, and its government the `düde`'s.
//! - **By hyperspace** ([`hyperspace_entry`]) a ship starts at angle
//!   `Rand(360)`, [`jump_in_distance`] from the centre, heading in at
//!   [`JUMP_IN_SPEED`]. A ship with no fuel capacity is not brought in by
//!   hyperspace; nor is a fleet whose lead has none.
//!
//! - **Aggression** ([`aggression`]): every ship spawned draws
//!   `Rand(3) ^ 2` last, after its place (@0x3c55b, @0x3cb04): 2, 3 or 0.
//!   It decides how far a warship hunts a player wanted by its
//!   government, and when it retreats (see [`ai`](crate::ai)). A person
//!   draws none: its `Aggress` gives it ([`aggression_of`]).
//!
//! A ship type with no record spawns nothing, a `Max` below `Min` gives
//! `Min`, a negative count none, and a draw is never asked over 0.

use std::collections::BTreeSet;

use crate::catalog::{FleetId, GovtId, PersonId, ShipId};
use crate::chance::Chance;
use crate::flight::{ShipState, facing, heading_of};
use crate::geometry::Vec2;
use crate::person::{PersonRoll, PersonWorld, aggression_of};
use crate::traffic::npc::AiType;
use crate::traffic::table::{ShipKind, SpawnDude, SpawnTable, pick};

/// `Rand(7) == 0`: a person, rather than anything else, by the engine
/// (see [`person`](crate::person)).
pub const PERSON_ODDS: u32 = 7;
/// `Rand(7) == 0`: a `LinkSyst` fleet, rather than a `düde` ship.
pub const FLEET_ODDS: u32 = 7;
/// `Rand(500)`: 0 or 1 brings an arrival, about one every 8 s.
pub const ARRIVAL_ODDS: u32 = 500;
/// `Rand(100) + 1`: a percentage roll.
pub const PERCENT: u32 = 100;
/// `Rand(256)`: the fleet slot a `LinkSyst` roll lands on.
pub const FLEET_SLOTS: u32 = 256;
/// The `flët` ID of slot 0.
pub const FIRST_FLEET_ID: i16 = 128;
/// `Rand(1500) - 750`: where in the system a ship is placed at setup.
pub const SPAWN_SPREAD: u32 = 1500;
/// `Rand(300) - 150`: how far from its lead an escort starts.
pub const ESCORT_SPREAD: u32 = 300;
/// `Rand(360)`: a heading, or an angle round the centre.
pub const DEGREES: u32 = 360;
/// How many ticks a ship glides in from hyperspace (`0x2b`).
pub const JUMP_IN_TICKS: u32 = 43;
/// Its speed on the glide's first tick, in pixels a tick.
pub const JUMP_IN_SPEED: f32 = 50.0;
/// How much slower each tick of the glide is than the last.
pub const JUMP_IN_SLOWING: f32 = 1.165;
/// How far from the centre the glide ends.
pub const JUMP_IN_END: f32 = 1000.0;
/// `Rand(3)`: the draw a ship's aggression is made from.
pub const AGGRESSION_DRAW: u32 = 3;
/// What the aggression draw is combined with, bit by bit, by exclusive
/// or.
const AGGRESSION_XOR: u32 = 2;

/// A ship to add to the system.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NewShip {
    /// Its ship type.
    pub ship: ShipId,
    /// Its government.
    pub govt: Option<GovtId>,
    /// How it behaves.
    pub ai_type: AiType,
    /// The index, among the ships added with it, of the lead it escorts.
    pub lead: Option<usize>,
    /// Where it starts.
    pub state: ShipState,
    /// Whether it glides in from hyperspace.
    pub jumping_in: bool,
    /// How aggressive it is: 0, 2 or 3.
    pub aggression: u8,
    /// Its `düde`'s `Booty` flags; none for a fleet's ship, which has no
    /// `düde`.
    pub booty: u16,
    /// Its `düde`'s `InfoTypes` flags, what it says when hailed; none for
    /// a fleet's ship.
    pub info_types: u16,
    /// The person flying it, if any.
    pub person: Option<PersonId>,
}

/// The persons' side of a system's spawning: their world, and the names
/// of the persons in the system, which grow as persons spawn.
#[derive(Clone, Debug)]
pub struct PersonDraw<'a> {
    /// The rules, the persons gone and the control bits.
    pub world: PersonWorld<'a>,
    /// The names of the persons in the system.
    pub here: BTreeSet<String>,
}

impl<'a> PersonDraw<'a> {
    /// The draw in `world`, with no person in the system.
    #[must_use]
    pub fn new(world: PersonWorld<'a>) -> Self {
        Self {
            world,
            here: BTreeSet::new(),
        }
    }

    /// The persons of `table` who may appear, `arriving` by hyperspace or
    /// not, in ascending ID (see [`person`](crate::person)).
    #[must_use]
    pub fn eligible(&self, table: &SpawnTable, arriving: bool) -> Vec<PersonId> {
        table
            .persons
            .iter()
            .filter(|(id, person)| {
                let record = &person.record;
                person.linked
                    && record.ai_type > 0
                    && !self.world.gone.contains(id)
                    // Control bits: every `ActiveOn` holds until
                    // missions-and-storylines brings them.
                    && record.active_on.holds(|test| self.world.control_bits.allows(test))
                    && !(arriving && person.derelict)
                    && !self.world.in_fleet(**id)
                    && !self.named_here(table, &record.name)
            })
            .map(|(&id, _)| id)
            .collect()
    }

    /// Whether a person named `name` is in the system already, the
    /// persons of the player's fleet, known to `table`, among them.
    fn named_here(&self, table: &SpawnTable, name: &str) -> bool {
        self.here.contains(name)
            || self
                .world
                .fleet
                .iter()
                .filter_map(|escort| table.persons.get(&escort.person?))
                .any(|person| person.record.name == name)
    }

    /// The person roll among those of `table` who may appear.
    fn roll(
        &self,
        table: &SpawnTable,
        arriving: bool,
        chance: &mut (impl Chance + ?Sized),
    ) -> PersonRoll {
        let eligible = self.eligible(table, arriving);
        self.world.rules.roll(&eligible, &mut &mut *chance)
    }

    /// Person `id` of `table`, now in the system, starting at `state`.
    fn spawn(&mut self, table: &SpawnTable, id: PersonId, state: ShipState) -> Option<NewShip> {
        let person = table.persons.get(&id)?;
        let record = &person.record;
        self.here.insert(record.name.clone());
        Some(NewShip {
            ship: record.ship?,
            govt: record.govt,
            ai_type: AiType::from_raw(record.ai_type),
            lead: None,
            state,
            jumping_in: false,
            aggression: aggression_of(record.aggress),
            booty: 0,
            info_types: 0,
            person: Some(id),
        })
    }
}

/// How fast a ship glides on tick `k` (from 0) of its jump in.
#[must_use]
pub fn glide_speed(k: u32) -> f32 {
    JUMP_IN_SLOWING.mul_add(-(k as f32), JUMP_IN_SPEED)
}

/// How far from the centre a ship starts its jump in: the glide's end
/// plus the whole glide, about 2098 pixels.
#[must_use]
pub fn jump_in_distance() -> f32 {
    JUMP_IN_END + (0..JUMP_IN_TICKS).map(glide_speed).sum::<f32>()
}

/// A system's initial population, its persons drawn as `draw` says (see
/// the module docs).
pub fn initial(
    table: &SpawnTable,
    draw: &mut PersonDraw,
    chance: &mut (impl Chance + ?Sized),
) -> Vec<NewShip> {
    let mut out = Vec::new();
    for _ in 0..table.avg_ships {
        match draw.roll(table, false, chance) {
            PersonRoll::Person(id) => {
                let state = in_system(chance);
                out.extend(draw.spawn(table, id, state));
                continue;
            }
            PersonRoll::Empty => continue,
            PersonRoll::None => {}
        }
        if chance.below(FLEET_ODDS) < 1 {
            link_fleet(table, chance, &mut out);
            continue;
        }
        if let Some((ship, dude, ai_type, _)) = dude_ship(table, chance) {
            let state = in_system(chance);
            out.push(NewShip {
                ship,
                govt: dude.govt,
                ai_type,
                lead: None,
                state,
                jumping_in: false,
                aggression: aggression(chance),
                booty: dude.booty,
                info_types: dude.info_types,
                person: None,
            });
        }
    }
    for &(id, prob) in &table.person_slots {
        let Some(person) = table.persons.get(&id) else {
            continue;
        };
        let world = draw.world;
        if world.gone.contains(&id)
            || world.in_fleet(id)
            || !person
                .record
                .active_on
                .holds(|test| world.control_bits.allows(test))
        {
            continue;
        }
        if world.rules.listed(prob, &mut &mut *chance)
            && !draw.named_here(table, &person.record.name)
        {
            let state = in_system(chance);
            out.extend(draw.spawn(table, id, state));
        }
    }
    out
}

/// The ships that arrive this tick in a system holding `present` NPCs,
/// its persons drawn as `draw` says (see the module docs).
pub fn arrivals(
    table: &SpawnTable,
    present: usize,
    draw: &mut PersonDraw,
    chance: &mut (impl Chance + ?Sized),
) -> Vec<NewShip> {
    let mut out = Vec::new();
    if present >= table.avg_ships as usize {
        return out;
    }
    match chance.below(ARRIVAL_ODDS) {
        1 if named_fleet(table, chance, &mut out) => {}
        0 | 1 => hyper_ship(table, draw, chance, &mut out),
        _ => {}
    }
    out
}

/// Rolls for one of the fleets the system's `DudeTypes` name, and brings
/// it in, added to `out`; whether the roll fired.
fn named_fleet(
    table: &SpawnTable,
    chance: &mut (impl Chance + ?Sized),
    out: &mut Vec<NewShip>,
) -> bool {
    if table.dude_fleets.is_empty() {
        return false;
    }
    let odds: u32 = table.dude_fleets.iter().map(|&(_, prob)| prob).sum();
    if chance.below(PERCENT) + 1 > odds {
        return false;
    }
    if let Some(id) = pick(&table.dude_fleets, chance) {
        fleet(table, id, chance, out);
    }
    true
}

/// `_HyperShipSpawn`: when the system has a `düde`, a person (drawn as
/// `draw` says), a `LinkSyst` fleet or a `düde` ship jumps in, added to
/// `out`.
fn hyper_ship(
    table: &SpawnTable,
    draw: &mut PersonDraw,
    chance: &mut (impl Chance + ?Sized),
    out: &mut Vec<NewShip>,
) {
    if table.dudes.is_empty() {
        return;
    }
    match draw.roll(table, true, chance) {
        PersonRoll::Person(id) => {
            if table
                .persons
                .get(&id)
                .is_some_and(|person| can_jump(&person.kind))
            {
                let state = hyperspace_entry(chance);
                out.extend(draw.spawn(table, id, state).map(|ship| NewShip {
                    jumping_in: true,
                    ..ship
                }));
            }
            return;
        }
        PersonRoll::Empty => return,
        PersonRoll::None => {}
    }
    if chance.below(FLEET_ODDS) < 1 {
        link_fleet(table, chance, out);
        return;
    }
    if let Some((ship, dude, ai_type, kind)) = dude_ship(table, chance)
        && can_jump(kind)
    {
        let state = hyperspace_entry(chance);
        out.push(NewShip {
            ship,
            govt: dude.govt,
            ai_type,
            lead: None,
            state,
            jumping_in: true,
            aggression: aggression(chance),
            booty: dude.booty,
            info_types: dude.info_types,
            person: None,
        });
    }
}

/// A `düde` ship: a `düde` by weight, a ship of it by `Probability`, and
/// the `düde`, its AI and the ship's kind; `None` when either has no
/// record.
fn dude_ship<'a>(
    table: &'a SpawnTable,
    chance: &mut (impl Chance + ?Sized),
) -> Option<(ShipId, &'a SpawnDude, AiType, &'a ShipKind)> {
    let dude = table.dude_records.get(&pick(&table.dudes, chance)?)?;
    let ship = pick(&dude.ships, chance)?;
    let kind = table.ships.get(&ship)?;
    let ai_type = if dude.ai_type > 0 {
        dude.ai_type
    } else {
        kind.inherent_ai
    };
    Some((ship, dude, AiType::from_raw(ai_type), kind))
}

/// The `LinkSyst` fleet roll: one of 256 slots, and the fleet there, if
/// its `LinkSyst` matches, added to `out`.
pub fn link_fleet(table: &SpawnTable, chance: &mut (impl Chance + ?Sized), out: &mut Vec<NewShip>) {
    let slot = chance.below(FLEET_SLOTS) as i16;
    let id = FleetId(FIRST_FLEET_ID + slot);
    if table.link_fleets.contains(&id) {
        fleet(table, id, chance, out);
    }
}

/// Fleet `id` jumping in, added to `out`: its lead, then its escorts.
pub fn fleet(
    table: &SpawnTable,
    id: FleetId,
    chance: &mut (impl Chance + ?Sized),
    out: &mut Vec<NewShip>,
) {
    let Some(record) = table.fleets.get(&id) else {
        return;
    };
    let Some((lead_ship, kind)) = record
        .lead
        .and_then(|ship| Some((ship, table.ships.get(&ship)?)))
    else {
        return;
    };
    if !can_jump(kind) {
        return;
    }
    let lead = out.len();
    let state = hyperspace_entry(chance);
    out.push(NewShip {
        ship: lead_ship,
        govt: record.govt,
        ai_type: AiType::from_raw(kind.inherent_ai),
        lead: None,
        state,
        jumping_in: true,
        aggression: aggression(chance),
        booty: 0,
        info_types: 0,
        person: None,
    });
    for escort in &record.escorts {
        let count = escort_count(escort.min, escort.max, chance);
        let Some(kind) = table.ships.get(&escort.ship).filter(|kind| can_jump(kind)) else {
            continue;
        };
        for _ in 0..count {
            let placed = escort_offset(&state, chance);
            out.push(NewShip {
                ship: escort.ship,
                govt: record.govt,
                ai_type: AiType::from_raw(kind.inherent_ai),
                lead: Some(lead),
                state: placed,
                jumping_in: true,
                aggression: aggression(chance),
                booty: 0,
                info_types: 0,
                person: None,
            });
        }
    }
}

/// How many of an escort type with `min` and `max` come:
/// `Min + Rand(Max - Min + 1)`, `min` when `max` is not above it, and
/// none below none.
pub fn escort_count(min: i16, max: i16, chance: &mut (impl Chance + ?Sized)) -> u32 {
    let min = u32::try_from(min).unwrap_or(0);
    let max = u32::try_from(max).unwrap_or(0);
    if max <= min {
        return min;
    }
    min + chance.below(max - min + 1)
}

/// Where a ship placed at setup starts: somewhere within 750 pixels of
/// the centre each way, at rest, facing anywhere.
pub fn in_system(chance: &mut (impl Chance + ?Sized)) -> ShipState {
    let x = spread(SPAWN_SPREAD, chance);
    let y = spread(SPAWN_SPREAD, chance);
    ShipState {
        position: Vec2::new(x, y),
        velocity: Vec2::ZERO,
        heading: chance.below(DEGREES) as f32,
    }
}

/// Where a ship jumping in starts: [`jump_in_distance`] from the centre
/// at an angle drawn, facing the centre and moving in at
/// [`JUMP_IN_SPEED`].
pub fn hyperspace_entry(chance: &mut (impl Chance + ?Sized)) -> ShipState {
    let out = facing(chance.below(DEGREES) as f32);
    let heading = heading_of(Vec2::ZERO - out);
    ShipState {
        position: out * jump_in_distance(),
        velocity: facing(heading) * JUMP_IN_SPEED,
        heading,
    }
}

/// Where an escort of a lead at `lead` starts: within 150 pixels of it
/// each way, with its heading and velocity.
pub fn escort_offset(lead: &ShipState, chance: &mut (impl Chance + ?Sized)) -> ShipState {
    let x = spread(ESCORT_SPREAD, chance);
    let y = spread(ESCORT_SPREAD, chance);
    ShipState {
        position: lead.position + Vec2::new(x, y),
        ..*lead
    }
}

/// A ship's aggression: `Rand(3) ^ 2`.
pub fn aggression(chance: &mut (impl Chance + ?Sized)) -> u8 {
    u8::try_from(chance.below(AGGRESSION_DRAW) ^ AGGRESSION_XOR).unwrap_or(0)
}

/// A draw of `Rand(spread) - spread / 2`.
fn spread(spread: u32, chance: &mut (impl Chance + ?Sized)) -> f32 {
    chance.below(spread) as f32 - (spread / 2) as f32
}

/// Whether a ship of `kind` can come by hyperspace: it has fuel capacity.
fn can_jump(kind: &ShipKind) -> bool {
    kind.stats.fuel > 0.0
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use super::*;
    use crate::catalog::{DudeId, EscortRecord, FleetRecord};
    use crate::catalog::{PersonId, PersonRecord};
    use crate::combat::hull::Condition;
    use crate::control::{Test, TestExpr};
    use crate::handling::ShipFields;
    use crate::hire::ControlBits;
    use crate::person::{NovaPersons, PersonWorld};
    use crate::rulebook::{RuleKey, RuleSource, Rulebook};
    use crate::stats::ShipStats;
    use crate::testkit::{Draws, FAST, person};
    use crate::traffic::table::SpawnPerson;

    /// The person draw of a world without persons gone or grudging, by
    /// the engine.
    fn nobody() -> PersonDraw<'static> {
        PersonDraw::new(PersonWorld::NONE)
    }

    fn kind(inherent_ai: i16, fields: ShipFields) -> ShipKind {
        ShipKind {
            stats: ShipStats::new(fields, &[]),
            inherent_ai,
            ..ShipKind::default()
        }
    }

    fn fleet_record(id: i16, lead: i16, escorts: &[(i16, i16, i16)]) -> FleetRecord {
        FleetRecord {
            id: FleetId(id),
            lead: Some(ShipId(lead)),
            escorts: escorts
                .iter()
                .map(|&(ship, min, max)| EscortRecord {
                    ship: ShipId(ship),
                    min,
                    max,
                })
                .collect(),
            govt: Some(GovtId(131)),
            link_syst: -1,
            appear_on: Test::default(),
        }
    }

    /// Two ships on average. Düde 128 (weight 100, no `AIType`, govt 130)
    /// flies ship 200 (`InherentAI` 2) or 201 (3), half and half. Fleet
    /// 140, named at 30 %, is led by ship 201 with one to three of ship 202
    /// (4); fleet 141 (slot 13) links here, led by ship 200 with exactly
    /// two of ship 202. Ship 203 has no fuel tank.
    fn table() -> SpawnTable {
        let no_fuel = ShipFields { fuel: 0, ..FAST };
        SpawnTable {
            avg_ships: 2,
            dudes: vec![(DudeId(128), 100)],
            dude_records: BTreeMap::from([(
                DudeId(128),
                SpawnDude {
                    ai_type: 0,
                    govt: Some(GovtId(130)),
                    ships: vec![(ShipId(200), 50), (ShipId(201), 50)],
                    booty: 0x0041,
                    info_types: 0x4005,
                },
            )]),
            dude_fleets: vec![(FleetId(140), 30)],
            link_fleets: BTreeSet::from([FleetId(141)]),
            fleets: BTreeMap::from([
                (FleetId(140), fleet_record(140, 201, &[(202, 1, 3)])),
                (FleetId(141), fleet_record(141, 200, &[(202, 2, 2)])),
            ]),
            ships: BTreeMap::from([
                (ShipId(200), kind(2, FAST)),
                (ShipId(201), kind(3, FAST)),
                (ShipId(202), kind(4, FAST)),
                (ShipId(203), kind(1, no_fuel)),
            ]),
            persons: BTreeMap::new(),
            person_slots: Vec::new(),
        }
    }

    fn one_pass() -> SpawnTable {
        SpawnTable {
            avg_ships: 1,
            ..table()
        }
    }

    fn near(a: Vec2, b: Vec2) -> bool {
        (a - b).length() < 1e-3
    }

    #[test]
    fn the_named_values() {
        assert_eq!(
            [
                PERSON_ODDS,
                FLEET_ODDS,
                ARRIVAL_ODDS,
                PERCENT,
                FLEET_SLOTS,
                SPAWN_SPREAD,
                ESCORT_SPREAD,
                DEGREES,
                JUMP_IN_TICKS
            ],
            [7, 7, 500, 100, 256, 1500, 300, 360, 43]
        );
        assert_eq!(FIRST_FLEET_ID, 128);
        assert_eq!(
            (JUMP_IN_SPEED, JUMP_IN_SLOWING, JUMP_IN_END),
            (50.0, 1.165, 1000.0)
        );
    }

    #[test]
    fn the_glide_slows_each_tick_and_starts_about_2098_out() {
        assert_eq!(glide_speed(0), 50.0);
        assert_eq!(glide_speed(1), 50.0 - 1.165);
        assert!((glide_speed(42) - (50.0 - 1.165 * 42.0)).abs() < 1e-4);
        // 1000 + 43 x 50 - 1.165 x (0 + 1 + ... + 42).
        assert!(
            (jump_in_distance() - 2098.005).abs() < 0.01,
            "{}",
            jump_in_distance()
        );
    }

    // Setup.

    #[test]
    fn a_setup_pass_drawing_a_person_spawns_nothing() {
        let mut chance = Draws::of(&[0]);
        assert_eq!(initial(&one_pass(), &mut nobody(), &mut chance), []);
        assert_eq!(chance.asked, [7]);
    }

    #[test]
    fn a_setup_pass_drawing_a_fleet_rolls_the_linksyst_slot() {
        // Not a person, a fleet, slot 13 (fleet 141), at angle 0, its two
        // escorts at (-150, -150) and (149, 149) from the lead, each ship's
        // aggression drawn after its place.
        let mut chance = Draws::of(&[6, 0, 13, 0, 2, 0, 0, 2, 299, 299, 2]);
        let ships = initial(&one_pass(), &mut nobody(), &mut chance);
        assert_eq!(chance.asked, [7, 7, 256, 360, 3, 300, 300, 3, 300, 300, 3]);
        let lead = ships[0];
        assert_eq!(
            (lead.ship, lead.lead, lead.jumping_in),
            (ShipId(200), None, true)
        );
        assert!(near(
            lead.state.position,
            Vec2::new(0.0, -jump_in_distance())
        ));
        let escorts: Vec<_> = ships[1..]
            .iter()
            .map(|escort| {
                (
                    escort.ship,
                    escort.lead,
                    escort.state.position - lead.state.position,
                )
            })
            .collect();
        assert_eq!(escorts.len(), 2);
        assert_eq!((escorts[0].0, escorts[0].1), (ShipId(202), Some(0)));
        assert!(near(escorts[0].2, Vec2::new(-150.0, -150.0)), "{escorts:?}");
        assert!(near(escorts[1].2, Vec2::new(149.0, 149.0)), "{escorts:?}");
    }

    #[test]
    fn only_a_draw_of_0_in_7_is_a_person_or_a_fleet() {
        // 1 is neither: a düde ship.
        let mut chance = Draws::of(&[1, 1, 0, 0, 0, 0, 0]);
        assert_eq!(initial(&one_pass(), &mut nobody(), &mut chance).len(), 1);
        assert_eq!(chance.asked, [7, 7, 100, 100, 1500, 1500, 360, 3]);
        let mut chance = Draws::of(&[0, 1, 1, 0, 0, 0]);
        assert_eq!(arrivals(&table(), 0, &mut nobody(), &mut chance).len(), 1);
        assert_eq!(chance.asked, [500, 7, 7, 100, 100, 360, 3]);
    }

    #[test]
    fn a_setup_pass_drawing_a_dude_ship_places_it_in_the_system_at_rest() {
        // Not a person, not a fleet: düde 128, ship 200, at (-750, 749)
        // facing 90.
        let mut chance = Draws::of(&[6, 6, 0, 0, 0, 1499, 90]);
        let ships = initial(&one_pass(), &mut nobody(), &mut chance);
        assert_eq!(chance.asked, [7, 7, 100, 100, 1500, 1500, 360, 3]);
        assert_eq!(
            ships,
            [NewShip {
                ship: ShipId(200),
                govt: Some(GovtId(130)),
                ai_type: AiType::BraveTrader,
                lead: None,
                state: ShipState {
                    position: Vec2::new(-750.0, 749.0),
                    velocity: Vec2::ZERO,
                    heading: 90.0,
                },
                jumping_in: false,
                aggression: 0,
                booty: 0x0041,
                info_types: 0x4005,
                person: None,
            }]
        );
    }

    #[test]
    fn setup_makes_avg_ships_passes() {
        let mut persons = Draws::of(&[0, 0, 0]);
        let three = SpawnTable {
            avg_ships: 3,
            ..table()
        };
        assert_eq!(initial(&three, &mut nobody(), &mut persons), []);
        assert_eq!(persons.asked, [7, 7, 7]);
        assert_eq!(
            initial(&three, &mut nobody(), &mut crate::NeverFires).len(),
            3
        );
        let none = SpawnTable {
            avg_ships: 0,
            ..table()
        };
        let mut nothing = Draws::of(&[]);
        assert_eq!(initial(&none, &mut nobody(), &mut nothing), []);
        assert!(nothing.asked.is_empty());
    }

    #[test]
    fn a_dude_with_an_ai_type_gives_it_and_one_without_gives_the_ships() {
        let mut table = one_pass();
        let ship_201 = [6, 6, 0, 99, 0, 0, 0];
        let ships = initial(&table, &mut nobody(), &mut Draws::of(&ship_201));
        assert_eq!(ships[0].ai_type, AiType::Warship, "201's InherentAI");
        table
            .dude_records
            .get_mut(&DudeId(128))
            .expect("düde")
            .ai_type = 4;
        let ships = initial(&table, &mut nobody(), &mut Draws::of(&ship_201));
        assert_eq!(ships[0].ai_type, AiType::Interceptor, "the düde's AIType");
        table
            .dude_records
            .get_mut(&DudeId(128))
            .expect("düde")
            .ai_type = 1;
        let ships = initial(&table, &mut nobody(), &mut Draws::of(&ship_201));
        assert_eq!(ships[0].ai_type, AiType::WimpyTrader, "1 is above 0");
    }

    #[test]
    fn a_dude_ship_carries_its_dudes_booty() {
        let ship_201 = [6, 6, 0, 99, 0, 0, 0];
        let ships = initial(&one_pass(), &mut nobody(), &mut Draws::of(&ship_201));
        assert_eq!(ships[0].booty, 0x0041);
        let arrived = arrivals(
            &table(),
            1,
            &mut nobody(),
            &mut Draws::of(&[0, 6, 6, 0, 99, 90]),
        );
        assert_eq!(arrived[0].booty, 0x0041, "a düde ship jumping in too");
    }

    #[test]
    fn a_fleet_ship_has_no_booty() {
        let ships = arrivals(
            &table(),
            0,
            &mut nobody(),
            &mut Draws::of(&[1, 29, 0, 0, 0, 2]),
        );
        assert_eq!(ships.len(), 4);
        assert!(ships.iter().all(|ship| ship.booty == 0), "{ships:?}");
    }

    #[test]
    fn a_dude_ship_carries_its_dudes_info_types() {
        let ship_201 = [6, 6, 0, 99, 0, 0, 0];
        let ships = initial(&one_pass(), &mut nobody(), &mut Draws::of(&ship_201));
        assert_eq!(ships[0].info_types, 0x4005);
        let arrived = arrivals(
            &table(),
            1,
            &mut nobody(),
            &mut Draws::of(&[0, 6, 6, 0, 99, 90]),
        );
        assert_eq!(arrived[0].info_types, 0x4005, "a düde ship jumping in too");
    }

    #[test]
    fn a_fleet_ship_has_no_info_types() {
        let ships = arrivals(
            &table(),
            0,
            &mut nobody(),
            &mut Draws::of(&[1, 29, 0, 0, 0, 2]),
        );
        assert_eq!(ships.len(), 4);
        assert!(ships.iter().all(|ship| ship.info_types == 0), "{ships:?}");
    }

    #[test]
    fn a_ship_type_without_a_record_or_a_dude_without_one_spawns_nothing() {
        let mut table = one_pass();
        table.ships.remove(&ShipId(200));
        let mut chance = Draws::of(&[6, 6, 0, 0]);
        assert_eq!(initial(&table, &mut nobody(), &mut chance), []);
        assert_eq!(chance.asked, [7, 7, 100, 100], "no placement drawn");
        table.dude_records.clear();
        let mut chance = Draws::of(&[6, 6, 0]);
        assert_eq!(initial(&table, &mut nobody(), &mut chance), []);
        assert_eq!(chance.asked, [7, 7, 100]);
    }

    // Arrivals.

    #[test]
    fn no_ship_arrives_in_a_system_holding_avg_ships() {
        let mut chance = Draws::of(&[0]);
        assert_eq!(arrivals(&table(), 2, &mut nobody(), &mut chance), []);
        assert_eq!(arrivals(&table(), 3, &mut nobody(), &mut chance), []);
        assert!(chance.asked.is_empty(), "nothing drawn");
    }

    #[test]
    fn two_in_500_ticks_bring_an_arrival() {
        let mut chance = Draws::of(&[2]);
        assert_eq!(arrivals(&table(), 1, &mut nobody(), &mut chance), []);
        assert_eq!(chance.asked, [500]);
        let mut chance = Draws::of(&[499]);
        assert_eq!(arrivals(&table(), 0, &mut nobody(), &mut chance), []);
    }

    #[test]
    fn on_0_a_dude_ship_jumps_in() {
        // 0, not a person, not a fleet, düde 128, ship 201, at angle 90.
        let mut chance = Draws::of(&[0, 6, 6, 0, 99, 90]);
        let ships = arrivals(&table(), 1, &mut nobody(), &mut chance);
        assert_eq!(chance.asked, [500, 7, 7, 100, 100, 360, 3]);
        assert_eq!(ships.len(), 1);
        let ship = ships[0];
        assert_eq!(
            (
                ship.ship,
                ship.govt,
                ship.ai_type,
                ship.lead,
                ship.jumping_in
            ),
            (ShipId(201), Some(GovtId(130)), AiType::Warship, None, true)
        );
        assert!(near(
            ship.state.position,
            Vec2::new(jump_in_distance(), 0.0)
        ));
    }

    #[test]
    fn on_0_a_person_or_a_linksyst_fleet_may_come_instead() {
        let mut person = Draws::of(&[0, 0]);
        assert_eq!(arrivals(&table(), 0, &mut nobody(), &mut person), []);
        assert_eq!(person.asked, [500, 7]);
        let mut linked = Draws::of(&[0, 6, 0, 13]);
        let ships = arrivals(&table(), 0, &mut nobody(), &mut linked);
        assert_eq!(&linked.asked[..4], [500, 7, 7, 256]);
        assert_eq!(ships.len(), 3, "fleet 141: its lead and two escorts");
        let mut missed = Draws::of(&[0, 6, 0, 14]);
        assert_eq!(arrivals(&table(), 0, &mut nobody(), &mut missed), []);
        assert_eq!(missed.asked, [500, 7, 7, 256]);
    }

    #[test]
    fn on_1_a_named_fleet_comes_when_the_percentage_roll_is_within_its_odds() {
        // 1, then Rand(100) + 1 = 30, at the fleets' 30: fleet 140, at
        // angle 0, its lead's aggression, then Rand(3) = 2, so three
        // escorts.
        let mut chance = Draws::of(&[1, 29, 0, 0, 0, 2]);
        let ships = arrivals(&table(), 0, &mut nobody(), &mut chance);
        assert_eq!(&chance.asked[..6], [500, 100, 30, 360, 3, 3]);
        assert_eq!(
            chance.asked.len(),
            6 + 3 * 3,
            "each escort's offset and aggression"
        );
        assert_eq!(ships.len(), 4);
        assert_eq!(
            (ships[0].ship, ships[0].govt, ships[0].ai_type),
            (ShipId(201), Some(GovtId(131)), AiType::Warship),
            "the lead's own InherentAI, the fleet's govt"
        );
        for escort in &ships[1..] {
            assert_eq!(
                (escort.ship, escort.govt, escort.ai_type, escort.lead),
                (ShipId(202), Some(GovtId(131)), AiType::Interceptor, Some(0))
            );
            assert!(escort.jumping_in);
        }
    }

    #[test]
    fn on_1_a_percentage_roll_above_the_odds_falls_through_to_0s_case() {
        // Rand(100) + 1 = 31, above 30: falls through, and a person comes.
        let mut chance = Draws::of(&[1, 30, 0]);
        assert_eq!(arrivals(&table(), 0, &mut nobody(), &mut chance), []);
        assert_eq!(chance.asked, [500, 100, 7]);
        // Through to a düde ship.
        let mut chance = Draws::of(&[1, 30, 6, 6, 0, 0, 0]);
        let ships = arrivals(&table(), 0, &mut nobody(), &mut chance);
        assert_eq!(chance.asked, [500, 100, 7, 7, 100, 100, 360, 3]);
        assert_eq!(ships.len(), 1);
    }

    #[test]
    fn on_1_without_named_fleets_it_goes_straight_to_0s_case() {
        let table = SpawnTable {
            dude_fleets: Vec::new(),
            ..table()
        };
        let mut chance = Draws::of(&[1, 0]);
        assert_eq!(arrivals(&table, 0, &mut nobody(), &mut chance), []);
        assert_eq!(chance.asked, [500, 7]);
    }

    #[test]
    fn without_a_dude_nothing_comes_on_0() {
        let table = SpawnTable {
            dudes: Vec::new(),
            dude_records: BTreeMap::new(),
            ..table()
        };
        let mut chance = Draws::of(&[0]);
        assert_eq!(arrivals(&table, 0, &mut nobody(), &mut chance), []);
        assert_eq!(chance.asked, [500]);
        let mut chance = Draws::of(&[1, 30]);
        assert_eq!(arrivals(&table, 0, &mut nobody(), &mut chance), []);
        assert_eq!(chance.asked, [500, 100]);
    }

    #[test]
    fn a_ship_without_fuel_never_jumps_in() {
        let mut table = table();
        table
            .dude_records
            .get_mut(&DudeId(128))
            .expect("düde")
            .ships = vec![(ShipId(203), 1)];
        let mut chance = Draws::of(&[0, 6, 6, 0, 0]);
        assert_eq!(arrivals(&table, 0, &mut nobody(), &mut chance), []);
        assert_eq!(chance.asked, [500, 7, 7, 100, 1], "no placement drawn");
        // Nor a fleet it leads.
        table
            .fleets
            .insert(FleetId(141), fleet_record(141, 203, &[(202, 2, 2)]));
        let mut out = Vec::new();
        let mut chance = Draws::of(&[]);
        fleet(&table, FleetId(141), &mut chance, &mut out);
        assert_eq!(out, []);
        assert!(chance.asked.is_empty());
        // Nor such an escort, though its lead comes.
        table.fleets.insert(
            FleetId(141),
            fleet_record(141, 200, &[(203, 2, 2), (999, 1, 1)]),
        );
        let mut chance = Draws::of(&[]);
        fleet(&table, FleetId(141), &mut chance, &mut out);
        assert_eq!(out.len(), 1);
        assert_eq!(chance.asked, [360, 3]);
    }

    #[test]
    fn a_fleet_without_a_lead_or_a_record_does_not_come() {
        let mut table = table();
        let mut out = Vec::new();
        fleet(&table, FleetId(150), &mut Draws::of(&[]), &mut out);
        let mut leaderless = fleet_record(141, 200, &[(202, 2, 2)]);
        leaderless.lead = None;
        table.fleets.insert(FleetId(141), leaderless);
        fleet(&table, FleetId(141), &mut Draws::of(&[]), &mut out);
        assert_eq!(out, []);
    }

    // Fleets.

    #[test]
    fn the_linksyst_roll_hits_one_slot_in_256() {
        let mut out = Vec::new();
        let mut hit = Draws::of(&[13]);
        link_fleet(&table(), &mut hit, &mut out);
        assert_eq!(out.len(), 3, "fleet 141");
        assert_eq!(hit.asked[0], 256);
        let mut out = Vec::new();
        for slot in [12, 0, 255] {
            let mut miss = Draws::of(&[slot]);
            link_fleet(&table(), &mut miss, &mut out);
            assert_eq!(miss.asked, [256], "{slot}");
        }
        assert_eq!(out, [], "fleet 140 is named, not linked");
    }

    #[test]
    fn a_fleets_escorts_follow_the_index_of_their_lead_among_the_ships_added() {
        let mut out = vec![initial(&one_pass(), &mut nobody(), &mut crate::NeverFires)[0]];
        fleet(&table(), FleetId(141), &mut crate::NeverFires, &mut out);
        assert_eq!(out.len(), 4);
        assert_eq!(
            out.iter().map(|ship| ship.lead).collect::<Vec<_>>(),
            [None, None, Some(1), Some(1)]
        );
    }

    #[test]
    fn escort_counts_run_from_min_to_max() {
        for (draw, count) in [(0, 1), (1, 2), (2, 3)] {
            let mut chance = Draws::of(&[draw]);
            assert_eq!(escort_count(1, 3, &mut chance), count);
            assert_eq!(chance.asked, [3]);
        }
        let mut chance = Draws::of(&[]);
        assert_eq!(escort_count(2, 2, &mut chance), 2);
        assert_eq!(escort_count(3, 1, &mut chance), 3, "Max below Min");
        assert_eq!(escort_count(-1, -1, &mut chance), 0);
        assert_eq!(escort_count(-3, -5, &mut chance), 0);
        assert!(chance.asked.is_empty(), "no draw over one outcome or none");
        let mut chance = Draws::of(&[1]);
        assert_eq!(escort_count(-2, 1, &mut chance), 1, "none below none");
        assert_eq!(chance.asked, [2]);
    }

    // Aggression.

    #[test]
    fn a_ships_aggression_is_its_last_draw_of_3_xor_2() {
        for (draw, aggression) in [(0, 2), (1, 3), (2, 0)] {
            // A düde ship placed at setup.
            let mut chance = Draws::of(&[6, 6, 0, 0, 0, 0, 0, draw]);
            let ships = initial(&one_pass(), &mut nobody(), &mut chance);
            assert_eq!(chance.asked, [7, 7, 100, 100, 1500, 1500, 360, 3]);
            assert_eq!(ships[0].aggression, aggression, "{draw}");
            // One jumping in.
            let mut chance = Draws::of(&[0, 6, 6, 0, 0, 0, draw]);
            let ships = arrivals(&table(), 0, &mut nobody(), &mut chance);
            assert_eq!(chance.asked, [500, 7, 7, 100, 100, 360, 3]);
            assert_eq!(ships[0].aggression, aggression, "{draw}");
        }
        assert_eq!(AGGRESSION_DRAW, 3);
    }

    #[test]
    fn each_ship_of_a_fleet_draws_its_aggression_after_its_place() {
        // Fleet 141: its lead at angle 0 (aggression 3), then its two
        // escorts, each placed and then its aggression drawn (0, then 2).
        let mut out = Vec::new();
        let mut chance = Draws::of(&[0, 1, 150, 150, 2, 150, 150, 0]);
        fleet(&table(), FleetId(141), &mut chance, &mut out);
        assert_eq!(chance.asked, [360, 3, 300, 300, 3, 300, 300, 3]);
        assert_eq!(
            out.iter().map(|ship| ship.aggression).collect::<Vec<_>>(),
            [3, 0, 2]
        );
    }

    // Placement.

    #[test]
    fn a_ship_placed_at_setup_is_within_750_each_way_at_rest() {
        let state = in_system(&mut Draws::of(&[0, 0, 0]));
        assert_eq!(state.position, Vec2::new(-750.0, -750.0));
        assert_eq!((state.velocity, state.heading), (Vec2::ZERO, 0.0));
        let state = in_system(&mut Draws::of(&[1499, 750, 359]));
        assert_eq!(state.position, Vec2::new(749.0, 0.0));
        assert_eq!(state.heading, 359.0);
    }

    #[test]
    fn a_ship_jumping_in_starts_about_2098_out_heading_in_at_50() {
        for angle in [0, 90, 135, 359] {
            let state = hyperspace_entry(&mut Draws::of(&[angle]));
            let out = facing(angle as f32);
            assert!(
                near(state.position, out * jump_in_distance()),
                "{angle}: {state:?}"
            );
            assert!(
                near(state.velocity, out * -JUMP_IN_SPEED),
                "{angle}: {state:?}"
            );
            assert!(
                near(facing(state.heading), out * -1.0),
                "{angle}: {state:?}"
            );
            assert!((0.0..360.0).contains(&state.heading));
        }
        let mut chance = Draws::of(&[]);
        hyperspace_entry(&mut chance);
        assert_eq!(chance.asked, [360]);
    }

    #[test]
    fn an_escort_starts_within_150_of_its_lead_with_its_heading_and_velocity() {
        let lead = ShipState {
            position: Vec2::new(100.0, -200.0),
            velocity: Vec2::new(3.0, 4.0),
            heading: 37.0,
        };
        let state = escort_offset(&lead, &mut Draws::of(&[0, 299]));
        assert_eq!(state.position, Vec2::new(-50.0, -51.0));
        assert_eq!(
            (state.velocity, state.heading),
            (lead.velocity, lead.heading)
        );
        let state = escort_offset(&lead, &mut Draws::of(&[150, 150]));
        assert_eq!(state.position, lead.position);
    }

    // Persons.

    /// Person `id` flying ship 201, linked here or not, of `aggress` and
    /// government 128.
    fn spawn_person(id: i16, linked: bool, aggress: i16) -> SpawnPerson {
        let kind = kind(3, FAST);
        SpawnPerson {
            record: PersonRecord {
                govt: Some(GovtId(128)),
                aggress,
                ..person(id, 201)
            },
            linked,
            reserves: kind.stats.full(),
            kind,
            condition: Condition::Intact,
            derelict: false,
        }
    }

    /// [`one_pass`] with person 510 (`Aggress` 4) linked here, and 600,
    /// unlinked, in the first Person slot at 50 %.
    fn peopled() -> SpawnTable {
        SpawnTable {
            persons: BTreeMap::from([
                (PersonId(510), spawn_person(510, true, 4)),
                (PersonId(600), spawn_person(600, false, 1)),
            ]),
            person_slots: vec![(PersonId(600), 50)],
            ..one_pass()
        }
    }

    /// Refuses every test that reads its bit.
    #[derive(Debug)]
    struct Refusing(u16);

    impl ControlBits for Refusing {
        fn allows(&self, test: &TestExpr) -> bool {
            !test.reads().iter().any(|bit| bit.get() == self.0)
        }
    }

    /// The setup of `table` in `world`, drawn from `draws`, and the
    /// bounds asked.
    fn set_up(table: &SpawnTable, world: PersonWorld, draws: &[u32]) -> (Vec<NewShip>, Vec<u32>) {
        let mut chance = Draws::of(draws);
        let ships = initial(table, &mut PersonDraw::new(world), &mut chance);
        (ships, chance.asked)
    }

    fn persons_of(ships: &[NewShip]) -> Vec<Option<i16>> {
        ships
            .iter()
            .map(|ship| ship.person.map(|id| id.0))
            .collect()
    }

    #[test]
    fn a_setup_pass_rolling_a_person_places_it_with_its_own_aggression() {
        let (ships, asked) = set_up(&peopled(), PersonWorld::NONE, &[0, 382, 0, 1499, 90, 99]);
        assert_eq!(
            asked,
            [7, 1022, 1500, 1500, 360, 100],
            "no aggression drawn"
        );
        assert_eq!(
            ships,
            [NewShip {
                ship: ShipId(201),
                govt: Some(GovtId(128)),
                ai_type: AiType::Warship,
                lead: None,
                state: ShipState {
                    position: Vec2::new(-750.0, 749.0),
                    velocity: Vec2::ZERO,
                    heading: 90.0,
                },
                jumping_in: false,
                aggression: 4,
                booty: 0,
                info_types: 0,
                person: Some(PersonId(510)),
            }]
        );
    }

    #[test]
    fn a_person_roll_that_misses_brings_nothing_that_pass() {
        let two = SpawnTable {
            avg_ships: 2,
            ..peopled()
        };
        let (ships, asked) = set_up(
            &two,
            PersonWorld::NONE,
            &[0, 381, 6, 6, 0, 0, 0, 0, 0, 0, 99],
        );
        assert_eq!(&asked[..2], [7, 1022], "the first pass: nothing");
        assert_eq!(&asked[2..4], [7, 7], "the next pass rolls as usual");
        assert_eq!(persons_of(&ships), [None]);
    }

    #[test]
    fn a_person_gone_refused_by_its_active_on_or_of_no_ai_type_does_not_come() {
        let gone = BTreeSet::from([PersonId(510)]);
        let world = PersonWorld {
            gone: &gone,
            ..PersonWorld::NONE
        };
        assert_eq!(set_up(&peopled(), world, &[0, 99]).1, [7, 100], "empty");
        let mut table = peopled();
        table
            .persons
            .get_mut(&PersonId(510))
            .expect("there")
            .record
            .active_on = Test::parse("b8");
        let refusing = Refusing(8);
        let world = PersonWorld {
            control_bits: &refusing,
            ..PersonWorld::NONE
        };
        assert_eq!(set_up(&table, world, &[0, 99]).1, [7, 100]);
        let mut table = peopled();
        table
            .persons
            .get_mut(&PersonId(510))
            .expect("there")
            .record
            .ai_type = 0;
        assert_eq!(set_up(&table, PersonWorld::NONE, &[0, 99]).1, [7, 100]);
    }

    #[test]
    fn a_person_whose_name_is_in_the_system_does_not_come_again() {
        let mut draw = PersonDraw::new(PersonWorld::NONE);
        draw.here.insert("Person 510".to_owned());
        assert_eq!(draw.eligible(&peopled(), false), []);
        let two = SpawnTable {
            avg_ships: 2,
            ..peopled()
        };
        // The first pass brings 510; the second's roll lands on it again.
        let (ships, asked) = set_up(&two, PersonWorld::NONE, &[0, 382, 0, 0, 0, 0, 99]);
        assert_eq!(persons_of(&ships), [Some(510)]);
        assert_eq!(&asked[5..], [7, 100], "nobody may come: no second draw");
    }

    #[test]
    fn a_derelict_person_may_come_at_setup_but_never_jumps_in() {
        let mut table = peopled();
        table
            .persons
            .get_mut(&PersonId(510))
            .expect("there")
            .derelict = true;
        let draw = PersonDraw::new(PersonWorld::NONE);
        assert_eq!(draw.eligible(&table, false), [PersonId(510)]);
        assert_eq!(draw.eligible(&table, true), []);
        assert_eq!(draw.eligible(&peopled(), true), [PersonId(510)]);
    }

    #[test]
    fn after_the_passes_each_person_slot_is_rolled_and_brings_its_person() {
        let none = SpawnTable {
            avg_ships: 0,
            ..peopled()
        };
        let (ships, asked) = set_up(&none, PersonWorld::NONE, &[49, 0, 0, 0]);
        assert_eq!(asked, [100, 1500, 1500, 360]);
        assert_eq!(persons_of(&ships), [Some(600)], "linked or not");
        assert_eq!(ships[0].aggression, 1);
        assert!(!ships[0].jumping_in);
        let (ships, asked) = set_up(&none, PersonWorld::NONE, &[50]);
        assert_eq!((ships.len(), asked), (0, vec![100]));
    }

    #[test]
    fn a_slot_naming_a_person_gone_refused_or_without_a_ship_draws_nothing() {
        let none = SpawnTable {
            avg_ships: 0,
            ..peopled()
        };
        let gone = BTreeSet::from([PersonId(600)]);
        let world = PersonWorld {
            gone: &gone,
            ..PersonWorld::NONE
        };
        assert_eq!(set_up(&none, world, &[]), (vec![], vec![]));
        let mut table = none.clone();
        table
            .persons
            .get_mut(&PersonId(600))
            .expect("there")
            .record
            .active_on = Test::parse("b9");
        let refusing = Refusing(9);
        let world = PersonWorld {
            control_bits: &refusing,
            ..PersonWorld::NONE
        };
        assert_eq!(set_up(&table, world, &[]), (vec![], vec![]));
        let shipless = SpawnTable {
            person_slots: vec![(PersonId(605), 100)],
            ..none
        };
        assert_eq!(set_up(&shipless, PersonWorld::NONE, &[]), (vec![], vec![]));
    }

    #[test]
    fn a_slot_person_already_present_draws_and_does_not_come() {
        let none = SpawnTable {
            avg_ships: 0,
            ..peopled()
        };
        let mut draw = PersonDraw::new(PersonWorld::NONE);
        draw.here.insert("Person 600".to_owned());
        let mut chance = Draws::of(&[0]);
        assert_eq!(initial(&none, &mut draw, &mut chance), []);
        assert_eq!(chance.asked, [100]);
    }

    #[test]
    fn a_person_arriving_jumps_in_from_hyperspace() {
        let mut draw = PersonDraw::new(PersonWorld::NONE);
        let mut chance = Draws::of(&[0, 0, 382, 90]);
        let ships = arrivals(&peopled(), 0, &mut draw, &mut chance);
        assert_eq!(chance.asked, [500, 7, 1022, 360], "no aggression drawn");
        assert_eq!(persons_of(&ships), [Some(510)]);
        assert!(ships[0].jumping_in);
        assert_eq!(ships[0].aggression, 4);
        assert!(near(
            ships[0].state.position,
            Vec2::new(jump_in_distance(), 0.0)
        ));
        assert!(draw.here.contains("Person 510"), "now here");
        let mut chance = Draws::of(&[0, 0, 381]);
        assert_eq!(arrivals(&peopled(), 0, &mut nobody(), &mut chance), []);
        assert_eq!(chance.asked, [500, 7, 1022], "nothing arrives");
    }

    fn by(key: RuleKey) -> NovaPersons {
        NovaPersons::from_rulebook(&Rulebook::default().with_override(key, RuleSource::Bible))
    }

    #[test]
    fn by_the_bibles_odds_a_pass_brings_a_person_5_times_in_100_or_goes_on() {
        let rules = by(RuleKey::PersonOdds);
        let world = PersonWorld {
            rules: &rules,
            ..PersonWorld::NONE
        };
        let (ships, asked) = set_up(&peopled(), world, &[4, 0, 0, 0, 99]);
        assert_eq!(asked, [100, 1500, 1500, 360, 100]);
        assert_eq!(persons_of(&ships), [Some(510)]);
        let (ships, asked) = set_up(&one_pass(), world, &[4, 6, 0, 0, 0, 0, 0, 0]);
        assert_eq!(
            asked,
            [100, 7, 100, 100, 1500, 1500, 360, 3],
            "on to a düde ship"
        );
        assert_eq!(persons_of(&ships), [None]);
    }

    #[test]
    fn by_the_bibles_slots_a_slot_brings_its_person_without_a_draw() {
        let rules = by(RuleKey::SystemPersons);
        let world = PersonWorld {
            rules: &rules,
            ..PersonWorld::NONE
        };
        let table = SpawnTable {
            avg_ships: 0,
            person_slots: vec![(PersonId(600), 1)],
            ..peopled()
        };
        let (ships, asked) = set_up(&table, world, &[0, 0, 0]);
        assert_eq!(asked, [1500, 1500, 360]);
        assert_eq!(persons_of(&ships), [Some(600)]);
    }

    /// The player's escorts that are persons `ids`.
    fn fleet_of(ids: &[i16]) -> Vec<crate::pilot::Escort> {
        ids.iter()
            .map(|&id| crate::pilot::Escort {
                ship: ShipId(201),
                reserves: kind(3, FAST).stats.full(),
                order: None,
                carried: false,
                wage: None,
                person: Some(PersonId(id)),
            })
            .collect()
    }

    #[test]
    fn a_person_in_the_players_fleet_is_never_rolled_nor_slotted() {
        let mut table = peopled();
        table
            .persons
            .insert(PersonId(511), spawn_person(511, true, 2));
        let fleet = fleet_of(&[510]);
        let world = PersonWorld {
            fleet: &fleet,
            ..PersonWorld::NONE
        };
        assert_eq!(
            PersonDraw::new(world).eligible(&table, false),
            [PersonId(511)]
        );
        let (ships, asked) = set_up(&table, world, &[0, 382, 99]);
        assert_eq!(asked, [7, 1022, 100], "landing on it is empty");
        assert_eq!(ships, []);
        let (ships, _) = set_up(&table, world, &[0, 383, 0, 0, 0, 99]);
        assert_eq!(persons_of(&ships), [Some(511)], "another comes");
        let slotted = fleet_of(&[600]);
        let world = PersonWorld {
            fleet: &slotted,
            ..PersonWorld::NONE
        };
        let none = SpawnTable {
            avg_ships: 0,
            ..peopled()
        };
        assert_eq!(set_up(&none, world, &[0]), (vec![], vec![]), "no draw");
    }

    #[test]
    fn a_person_of_the_name_of_one_in_the_players_fleet_does_not_come() {
        let mut table = peopled();
        let mut namesake = spawn_person(512, true, 2);
        namesake.record.name = "Person 600".to_owned();
        table.persons.insert(PersonId(512), namesake);
        let fleet = fleet_of(&[600]);
        let world = PersonWorld {
            fleet: &fleet,
            ..PersonWorld::NONE
        };
        assert_eq!(
            PersonDraw::new(world).eligible(&table, false),
            [PersonId(510)]
        );
        assert_eq!(
            PersonDraw::new(PersonWorld::NONE).eligible(&table, false),
            [PersonId(510), PersonId(512)],
            "with no fleet, as before"
        );
        let slot = SpawnTable {
            avg_ships: 0,
            person_slots: vec![(PersonId(512), 100)],
            ..table
        };
        let (ships, asked) = set_up(&slot, world, &[0]);
        assert_eq!((persons_of(&ships), asked), (vec![], vec![100]));
    }
}
