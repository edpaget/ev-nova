//! Tests of the skill each ship the traffic adds rolls
//! ([`random_skill_level`](crate::skill::random_skill_level)) and the
//! government's factor it takes, on every path into `Traffic::add`: the
//! setup's and the arrivals' `düde` ships (`_RandomShipSpawn`), a fleet's
//! lead and escorts and a person (`_HyperSpawnFleet`, `_SpawnPerson`,
//! both through `_GenericRandomShipSpawn`), and the mission and
//! auxiliary ships (`_MissionDudeSpawn`, `_SpecificDudeSpawn`).
#![allow(clippy::float_cmp)]

use std::collections::{BTreeMap, BTreeSet};

use super::spawn::{Entry, NewShip};
use super::table::{ShipKind, SpawnDude, SpawnPerson, SpawnTable};
use super::{Traffic, World};
use crate::catalog::{
    DudeId, EscortRecord, FleetId, FleetRecord, GovtId, GovtRecord, PersonId, PersonRecord, ShipId,
    SystemReinforcements,
};
use crate::combat::hull::Condition;
use crate::flight::ShipState;
use crate::govt::Governments;
use crate::stats::ShipStats;
use crate::traffic::npc::AiType;
use nova_sim_base::fixture::{Draws, FAST};

/// The class's `SkillVar`: its roll asks 75 sides, a range no other draw
/// of these spawns asks.
const SKILL_VAR: i16 = 37;
const SIDES: u32 = 75;

/// Draws as `draws` script them, but each skill roll gives `outcome`,
/// wherever it falls among them.
fn skilled(outcome: u32, draws: &[u32]) -> Draws {
    Draws::of(draws).keyed(SIDES, outcome)
}

/// The skill an outcome of the roll gives: `(outcome − 37 + 100) × 0.01`.
fn skill(outcome: u32) -> f32 {
    (f64::from(i32::try_from(outcome).expect("small") - 37 + 100) * 0.01) as f32
}

/// Government 140, of `SkillMult` 150; 141, of none.
fn govts() -> Governments {
    Governments::new([
        GovtRecord {
            skill_mult: 150,
            ..nova_sim_catalog::fixture::govt(140)
        },
        nova_sim_catalog::fixture::govt(141),
    ])
}

/// A system of `avg_ships` on average, all of düde 128's ship 200 of
/// government 140, which rolls its skill on [`SKILL_VAR`]; its
/// reinforcement fleet, 129 of government 140, ship 200 leading one more;
/// and person 150, flying ship 200, of government 140.
fn spawning(avg_ships: u32) -> SpawnTable {
    let kind = ShipKind {
        stats: ShipStats::new(FAST, &[]),
        skill_var: SKILL_VAR,
        ..ShipKind::default()
    };
    SpawnTable {
        avg_ships,
        dudes: vec![(DudeId(128), 100)],
        dude_records: BTreeMap::from([(
            DudeId(128),
            SpawnDude {
                ai_type: 1,
                govt: Some(GovtId(140)),
                ships: vec![(ShipId(200), 1)],
                booty: 0,
                info_types: 0,
            },
        )]),
        ships: BTreeMap::from([(ShipId(200), kind.clone())]),
        fleets: BTreeMap::from([(
            FleetId(129),
            FleetRecord {
                id: FleetId(129),
                lead: Some(ShipId(200)),
                escorts: vec![EscortRecord {
                    ship: ShipId(200),
                    min: 1,
                    max: 1,
                }],
                govt: Some(GovtId(140)),
                link_syst: -1,
                appear_on: crate::control::Test::default(),
                quote: None,
                flags: 0,
            },
        )]),
        reinforcements: Some(SystemReinforcements {
            fleet: FleetId(129),
            time: 900,
            interval: 2,
        }),
        persons: BTreeMap::from([(
            PersonId(150),
            SpawnPerson {
                record: PersonRecord {
                    govt: Some(GovtId(140)),
                    ..nova_sim_catalog::fixture::person(150, 200)
                },
                linked: false,
                kind,
                reserves: crate::reserves::Reserves::default(),
                condition: Condition::Intact,
                derelict: false,
            },
        )]),
        ..SpawnTable::default()
    }
}

/// A ship of type 200 of government `govt`, placed as it is.
fn ship(govt: Option<GovtId>) -> NewShip {
    NewShip {
        ship: ShipId(200),
        govt,
        ai_type: AiType::WimpyTrader,
        lead: None,
        state: ShipState::default(),
        entry: Entry::Placed,
        aggression: 0,
        booty: Some(0),
        info_types: 0,
        person: None,
        mission: None,
        aux: None,
        derelict: false,
        held: false,
        boarded: false,
        cloaked: false,
        quote: None,
        cargo: None,
    }
}

/// Each NPC's skill and government factor, in order.
fn skills(traffic: &Traffic) -> Vec<(f32, f32)> {
    traffic
        .npcs()
        .iter()
        .map(|npc| (npc.skill, npc.skill_mult))
        .collect()
}

#[test]
fn a_duede_ship_the_setup_places_rolls_its_skill_and_takes_its_governments_factor() {
    let govts = govts();
    for outcome in [0, 74] {
        let mut traffic = Traffic::new();
        let mut chance = skilled(outcome, &[6, 6, 0, 0, 750, 750, 0, 2]);
        let world = World {
            govts: &govts,
            ..World::new(&[])
        };
        traffic.enter_in(spawning(1), world, &mut chance);
        assert_eq!(skills(&traffic), [(skill(outcome), 1.5)], "{outcome}");
        assert_eq!(chance.times_asked(SIDES), 1);
    }
    assert_eq!((skill(0), skill(74)), (0.63, 1.37));
}

#[test]
fn a_duede_ship_that_arrives_rolls_its_skill() {
    let govts = govts();
    let mut traffic = Traffic::new();
    traffic.table = spawning(1);
    let world = World {
        govts: &govts,
        ..World::new(&[])
    };
    let mut chance = skilled(74, &[0, 6, 6, 0, 0, 90]);
    traffic.enter_more_ships(world, &mut chance);
    assert_eq!(skills(&traffic), [(skill(74), 1.5)]);
}

#[test]
fn a_fleets_lead_and_escort_each_roll_their_skill() {
    let govts = govts();
    let mut traffic = Traffic::new();
    traffic.enter(spawning(0), &mut Draws::of(&[]));
    let world = World {
        govts: &govts,
        ..World::new(&[])
    };
    let mut chance = skilled(0, &[0]);
    traffic.bring_reinforcements(&world, &mut chance);
    assert_eq!(skills(&traffic), [(skill(0), 1.5), (skill(0), 1.5)]);
    assert_eq!(chance.times_asked(SIDES), 2);
}

#[test]
fn a_person_rolls_its_ships_skill_and_takes_its_governments_factor() {
    let govts = govts();
    let mut traffic = Traffic::new();
    traffic.table = spawning(0);
    let person = NewShip {
        person: Some(PersonId(150)),
        ..ship(Some(GovtId(140)))
    };
    let mut chance = skilled(74, &[]);
    traffic.add(
        vec![person],
        &BTreeSet::new(),
        None,
        None,
        &govts,
        &mut chance,
    );
    assert_eq!(skills(&traffic), [(skill(74), 1.5)]);
}

#[test]
fn a_mission_ship_set_up_with_the_system_rolls_its_skill() {
    let govts = govts();
    let mut traffic = Traffic::new();
    let world = World {
        govts: &govts,
        ..World::new(&[])
    };
    let mut chance = skilled(0, &[]);
    traffic.enter_with(
        spawning(0),
        world,
        vec![ship(Some(GovtId(140)))],
        &mut chance,
    );
    assert_eq!(skills(&traffic), [(skill(0), 1.5)]);
}

#[test]
fn a_mission_or_auxiliary_ship_brought_in_rolls_its_skill() {
    let govts = govts();
    let mut traffic = Traffic::new();
    traffic.enter(spawning(0), &mut Draws::of(&[]));
    let mut chance = skilled(74, &[]);
    let came = traffic.bring_in_within(
        vec![ship(Some(GovtId(140))), ship(None)],
        8,
        &govts,
        &mut chance,
    );
    assert_eq!(came, 2);
    assert_eq!(skills(&traffic), [(skill(74), 1.5), (skill(74), 1.0)]);
}

#[test]
fn a_class_of_no_skill_var_flies_at_1_and_rolls_nothing() {
    let mut table = spawning(0);
    table.ships.get_mut(&ShipId(200)).expect("there").skill_var = 0;
    let mut traffic = Traffic::new();
    traffic.enter(table, &mut Draws::of(&[]));
    let mut chance = skilled(0, &[]);
    traffic.bring_in_within(vec![ship(None)], 8, &Governments::empty(), &mut chance);
    assert_eq!(skills(&traffic), [(1.0, 1.0)]);
    assert_eq!(chance.times_asked(SIDES), 0);
}
