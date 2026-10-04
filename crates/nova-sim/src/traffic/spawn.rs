//! Which NPCs appear, and where: a system's initial population, the
//! arrivals over time, and the fleets, rolled on the [`Chance`] port.
//!
//! The rolls are the original's (`_SetupShipsInSystem`, `_EnterMoreShips`,
//! `_HyperShipSpawn`, `_SpawnFleet`, `_HyperSpawnFleet` and
//! `_RandomShipSpawn` in the `EV Nova` executable), each written as
//! "`below(n) < k` fires":
//!
//! - **Setup** ([`initial`]) makes `AvgShips` passes. Each draws a person
//!   (`Rand(7) == 0`; persons come later, so it spawns nothing), else a
//!   `LinkSyst` fleet (`Rand(7) == 0`, [`link_fleet`]), else one `düde`
//!   ship, placed in the system already: x and y each `Rand(1500) - 750`,
//!   heading `Rand(360)`, at rest.
//! - **Arrivals** ([`arrivals`]), every tick while the system holds fewer
//!   NPCs than `AvgShips`, draw `Rand(500)`. On 1, when the system's
//!   `DudeTypes` name fleets, `Rand(100) + 1` at or below the sum of their
//!   `% Prob` brings one in, picked by `% Prob`; otherwise it falls
//!   through to 0's case. On 0, when the system has a `düde`, a person
//!   (`Rand(7) == 0`, nothing), else a `LinkSyst` fleet (`Rand(7) == 0`),
//!   else one `düde` ship jumps in.
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
//! A ship type with no record spawns nothing, a `Max` below `Min` gives
//! `Min`, a negative count none, and a draw is never asked over 0.

use crate::catalog::{FleetId, GovtId, ShipId};
use crate::chance::Chance;
use crate::flight::{ShipState, facing, heading_of};
use crate::geometry::Vec2;
use crate::traffic::npc::AiType;
use crate::traffic::table::{ShipKind, SpawnTable, pick};

/// `Rand(7) == 0`: a person, rather than anything else.
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

/// A system's initial population (see the module docs).
pub fn initial(table: &SpawnTable, chance: &mut (impl Chance + ?Sized)) -> Vec<NewShip> {
    let mut out = Vec::new();
    for _ in 0..table.avg_ships {
        if chance.below(PERSON_ODDS) < 1 {
            continue;
        }
        if chance.below(FLEET_ODDS) < 1 {
            link_fleet(table, chance, &mut out);
            continue;
        }
        if let Some((ship, govt, ai_type, _)) = dude_ship(table, chance) {
            out.push(NewShip {
                ship,
                govt,
                ai_type,
                lead: None,
                state: in_system(chance),
                jumping_in: false,
            });
        }
    }
    out
}

/// The ships that arrive this tick in a system holding `present` NPCs
/// (see the module docs).
pub fn arrivals(
    table: &SpawnTable,
    present: usize,
    chance: &mut (impl Chance + ?Sized),
) -> Vec<NewShip> {
    let mut out = Vec::new();
    if present >= table.avg_ships as usize {
        return out;
    }
    match chance.below(ARRIVAL_ODDS) {
        1 if named_fleet(table, chance, &mut out) => {}
        0 | 1 => hyper_ship(table, chance, &mut out),
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

/// `_HyperShipSpawn`: when the system has a `düde`, a person, a
/// `LinkSyst` fleet or a `düde` ship jumps in, added to `out`.
fn hyper_ship(table: &SpawnTable, chance: &mut (impl Chance + ?Sized), out: &mut Vec<NewShip>) {
    if table.dudes.is_empty() || chance.below(PERSON_ODDS) < 1 {
        return;
    }
    if chance.below(FLEET_ODDS) < 1 {
        link_fleet(table, chance, out);
        return;
    }
    if let Some((ship, govt, ai_type, kind)) = dude_ship(table, chance)
        && can_jump(kind)
    {
        out.push(NewShip {
            ship,
            govt,
            ai_type,
            lead: None,
            state: hyperspace_entry(chance),
            jumping_in: true,
        });
    }
}

/// A `düde` ship: a `düde` by weight, a ship of it by `Probability`, and
/// its government, AI and kind; `None` when either has no record.
fn dude_ship<'a>(
    table: &'a SpawnTable,
    chance: &mut (impl Chance + ?Sized),
) -> Option<(ShipId, Option<GovtId>, AiType, &'a ShipKind)> {
    let dude = table.dude_records.get(&pick(&table.dudes, chance)?)?;
    let ship = pick(&dude.ships, chance)?;
    let kind = table.ships.get(&ship)?;
    let ai_type = if dude.ai_type > 0 {
        dude.ai_type
    } else {
        kind.inherent_ai
    };
    Some((ship, dude.govt, AiType::from_raw(ai_type), kind))
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
    });
    for escort in &record.escorts {
        let count = escort_count(escort.min, escort.max, chance);
        let Some(kind) = table.ships.get(&escort.ship).filter(|kind| can_jump(kind)) else {
            continue;
        };
        for _ in 0..count {
            out.push(NewShip {
                ship: escort.ship,
                govt: record.govt,
                ai_type: AiType::from_raw(kind.inherent_ai),
                lead: Some(lead),
                state: escort_offset(&state, chance),
                jumping_in: true,
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
    use crate::handling::ShipFields;
    use crate::stats::ShipStats;
    use crate::testkit::{Draws, FAST};
    use crate::traffic::table::SpawnDude;

    fn kind(inherent_ai: i16, fields: ShipFields) -> ShipKind {
        ShipKind {
            stats: ShipStats::new(fields, &[]),
            inherent_ai,
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
            appear_on: String::new(),
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
        assert_eq!(initial(&one_pass(), &mut chance), []);
        assert_eq!(chance.asked, [7]);
    }

    #[test]
    fn a_setup_pass_drawing_a_fleet_rolls_the_linksyst_slot() {
        // Not a person, a fleet, slot 13 (fleet 141), at angle 0, its two
        // escorts at (-150, -150) and (149, 149) from the lead.
        let mut chance = Draws::of(&[6, 0, 13, 0, 0, 0, 299, 299]);
        let ships = initial(&one_pass(), &mut chance);
        assert_eq!(chance.asked, [7, 7, 256, 360, 300, 300, 300, 300]);
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
        assert_eq!(initial(&one_pass(), &mut chance).len(), 1);
        assert_eq!(chance.asked, [7, 7, 100, 100, 1500, 1500, 360]);
        let mut chance = Draws::of(&[0, 1, 1, 0, 0, 0]);
        assert_eq!(arrivals(&table(), 0, &mut chance).len(), 1);
        assert_eq!(chance.asked, [500, 7, 7, 100, 100, 360]);
    }

    #[test]
    fn a_setup_pass_drawing_a_dude_ship_places_it_in_the_system_at_rest() {
        // Not a person, not a fleet: düde 128, ship 200, at (-750, 749)
        // facing 90.
        let mut chance = Draws::of(&[6, 6, 0, 0, 0, 1499, 90]);
        let ships = initial(&one_pass(), &mut chance);
        assert_eq!(chance.asked, [7, 7, 100, 100, 1500, 1500, 360]);
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
        assert_eq!(initial(&three, &mut persons), []);
        assert_eq!(persons.asked, [7, 7, 7]);
        assert_eq!(initial(&three, &mut crate::NeverFires).len(), 3);
        let none = SpawnTable {
            avg_ships: 0,
            ..table()
        };
        let mut nothing = Draws::of(&[]);
        assert_eq!(initial(&none, &mut nothing), []);
        assert!(nothing.asked.is_empty());
    }

    #[test]
    fn a_dude_with_an_ai_type_gives_it_and_one_without_gives_the_ships() {
        let mut table = one_pass();
        let ship_201 = [6, 6, 0, 99, 0, 0, 0];
        let ships = initial(&table, &mut Draws::of(&ship_201));
        assert_eq!(ships[0].ai_type, AiType::Warship, "201's InherentAI");
        table
            .dude_records
            .get_mut(&DudeId(128))
            .expect("düde")
            .ai_type = 4;
        let ships = initial(&table, &mut Draws::of(&ship_201));
        assert_eq!(ships[0].ai_type, AiType::Interceptor, "the düde's AIType");
        table
            .dude_records
            .get_mut(&DudeId(128))
            .expect("düde")
            .ai_type = 1;
        let ships = initial(&table, &mut Draws::of(&ship_201));
        assert_eq!(ships[0].ai_type, AiType::WimpyTrader, "1 is above 0");
    }

    #[test]
    fn a_ship_type_without_a_record_or_a_dude_without_one_spawns_nothing() {
        let mut table = one_pass();
        table.ships.remove(&ShipId(200));
        let mut chance = Draws::of(&[6, 6, 0, 0]);
        assert_eq!(initial(&table, &mut chance), []);
        assert_eq!(chance.asked, [7, 7, 100, 100], "no placement drawn");
        table.dude_records.clear();
        let mut chance = Draws::of(&[6, 6, 0]);
        assert_eq!(initial(&table, &mut chance), []);
        assert_eq!(chance.asked, [7, 7, 100]);
    }

    // Arrivals.

    #[test]
    fn no_ship_arrives_in_a_system_holding_avg_ships() {
        let mut chance = Draws::of(&[0]);
        assert_eq!(arrivals(&table(), 2, &mut chance), []);
        assert_eq!(arrivals(&table(), 3, &mut chance), []);
        assert!(chance.asked.is_empty(), "nothing drawn");
    }

    #[test]
    fn two_in_500_ticks_bring_an_arrival() {
        let mut chance = Draws::of(&[2]);
        assert_eq!(arrivals(&table(), 1, &mut chance), []);
        assert_eq!(chance.asked, [500]);
        let mut chance = Draws::of(&[499]);
        assert_eq!(arrivals(&table(), 0, &mut chance), []);
    }

    #[test]
    fn on_0_a_dude_ship_jumps_in() {
        // 0, not a person, not a fleet, düde 128, ship 201, at angle 90.
        let mut chance = Draws::of(&[0, 6, 6, 0, 99, 90]);
        let ships = arrivals(&table(), 1, &mut chance);
        assert_eq!(chance.asked, [500, 7, 7, 100, 100, 360]);
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
        assert_eq!(arrivals(&table(), 0, &mut person), []);
        assert_eq!(person.asked, [500, 7]);
        let mut linked = Draws::of(&[0, 6, 0, 13]);
        let ships = arrivals(&table(), 0, &mut linked);
        assert_eq!(&linked.asked[..4], [500, 7, 7, 256]);
        assert_eq!(ships.len(), 3, "fleet 141: its lead and two escorts");
        let mut missed = Draws::of(&[0, 6, 0, 14]);
        assert_eq!(arrivals(&table(), 0, &mut missed), []);
        assert_eq!(missed.asked, [500, 7, 7, 256]);
    }

    #[test]
    fn on_1_a_named_fleet_comes_when_the_percentage_roll_is_within_its_odds() {
        // 1, then Rand(100) + 1 = 30, at the fleets' 30: fleet 140, at
        // angle 0, with Rand(3) = 2, so three escorts.
        let mut chance = Draws::of(&[1, 29, 0, 0, 2]);
        let ships = arrivals(&table(), 0, &mut chance);
        assert_eq!(&chance.asked[..5], [500, 100, 30, 360, 3]);
        assert_eq!(chance.asked.len(), 5 + 3 * 2, "each escort's offset");
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
        assert_eq!(arrivals(&table(), 0, &mut chance), []);
        assert_eq!(chance.asked, [500, 100, 7]);
        // Through to a düde ship.
        let mut chance = Draws::of(&[1, 30, 6, 6, 0, 0, 0]);
        let ships = arrivals(&table(), 0, &mut chance);
        assert_eq!(chance.asked, [500, 100, 7, 7, 100, 100, 360]);
        assert_eq!(ships.len(), 1);
    }

    #[test]
    fn on_1_without_named_fleets_it_goes_straight_to_0s_case() {
        let table = SpawnTable {
            dude_fleets: Vec::new(),
            ..table()
        };
        let mut chance = Draws::of(&[1, 0]);
        assert_eq!(arrivals(&table, 0, &mut chance), []);
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
        assert_eq!(arrivals(&table, 0, &mut chance), []);
        assert_eq!(chance.asked, [500]);
        let mut chance = Draws::of(&[1, 30]);
        assert_eq!(arrivals(&table, 0, &mut chance), []);
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
        assert_eq!(arrivals(&table, 0, &mut chance), []);
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
        assert_eq!(chance.asked, [360]);
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
        let mut out = vec![initial(&one_pass(), &mut crate::NeverFires)[0]];
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
}
