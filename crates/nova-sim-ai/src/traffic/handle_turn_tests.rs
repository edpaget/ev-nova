//! Tests of `_HandleShip`'s turn (@0x34424-0x3458e) for the NPC flights
//! the autopilot flies ([`autopilot::fly`]): every one wants a
//! whole-degree heading (+0x68) and is turned to it by
//! [`flight::turn_to`](crate::flight::turn_to), by its turn while more
//! than that off in whole degrees and onto it otherwise; one that wants
//! none has its heading truncated, the low level's default (@0x8522d).
//! While its AI wait runs the turn is skipped, but for a count wait
//! (@0x3443f-0x34454).
#![allow(clippy::float_cmp)]

use super::autopilot::{self, Beside, Outcome, Readings, TopSpeeds};
use super::npc::Npc;
use crate::ai::Goal;
use crate::ai::fire::Manoeuvre;
use crate::ai::pursuit::{Body, Frame};
use crate::catalog::{LandingSite, StellarId};
use crate::combat::ShipRef;
use crate::combat::aim::bearing;
use crate::flight::{AT_REST_SPEED, FlightRules, ShipState};
use crate::geometry::Vec2;
use crate::hail::Help;
use crate::handling::ShipFields;
use crate::stats::{INERTIALESS_HULL, ShipStats};
use crate::trig;
use nova_sim_base::fixture::{Draws, FAST};
use nova_sim_catalog::fixture::planet;
use std::collections::BTreeMap;

/// [`FAST`] with an inertialess hull.
const RAVEN: ShipFields = ShipFields {
    flags2: INERTIALESS_HULL,
    ..FAST
};

fn at(x: f32, y: f32, vx: f32, vy: f32, heading: f32) -> ShipState {
    ShipState {
        position: Vec2::new(x, y),
        velocity: Vec2::new(vx, vy),
        heading,
    }
}

/// An NPC of `fields` on `goal` at `start`, holding `speed` along its
/// heading.
fn ship(fields: ShipFields, goal: Goal, start: ShipState, speed: f32) -> Npc {
    Npc {
        state: start,
        goal,
        speed,
        ..super::fixture::npc(1, ShipStats::new(fields, &[]))
    }
}

/// Flies `npc` one tick at its own top speed beside `other`, under the
/// original's rules.
fn fly(npc: &mut Npc, other: Option<&ShipState>) -> Outcome {
    fly_among(npc, &[], other)
}

/// [`fly`] among `sites`.
fn fly_among(npc: &mut Npc, sites: &[LandingSite], other: Option<&ShipState>) -> Outcome {
    let speeds = TopSpeeds::own_of(npc);
    let weapons = crate::ai::fixture::records_of([&*npc]);
    autopilot::fly(
        npc,
        sites,
        &BTreeMap::new(),
        other,
        false,
        Beside::default(),
        speeds,
        FlightRules::default(),
        Readings::default(),
        &weapons,
        &mut Draws::of(&[]),
    )
}

#[test]
fn a_moving_inertialess_sniper_braking_or_bursting_has_its_heading_truncated() {
    // Sub 0xe's inertialess brake writes no +0x68 (@0x8710f-0x8713a), so
    // `_HandleShip` truncates the heading, whether the brake or an owed
    // burst flies it.
    let target = at(500.0, 0.0, 0.0, 0.0, 0.0);
    for owed in [None, Some(30.0)] {
        let mut sniper = ship(
            RAVEN,
            Goal::Snipe(ShipRef::Player),
            at(0.0, 0.0, 0.0, -3.0, 40.6),
            3.0,
        );
        sniper.owed_burst = owed;
        assert_eq!(fly(&mut sniper, Some(&target)), Outcome::Flying);
        assert_eq!(sniper.state.heading, 40.0, "owing {owed:?}");
    }
}

// The fight's bodies (subs 5, 6, 7, 0x10, 0x11 and 0x12), each writing
// +0x68 to its body's heading.

/// Every manoeuvre the fight flies by `autopilot::fight`'s own turn.
const BODIES: [Manoeuvre; 6] = [
    Manoeuvre::Approach,
    Manoeuvre::Dogfight,
    Manoeuvre::Flee,
    Manoeuvre::Swarm,
    Manoeuvre::BreakOff,
    Manoeuvre::Rush,
];

/// A ship of `fields` attacking the player far above it, facing
/// `heading` and turning `rate` degrees a tick, its frame's last body
/// `manoeuvre` wanting heading 90: its heading after one tick.
fn fought(fields: ShipFields, manoeuvre: Manoeuvre, heading: f32, rate: f32) -> f32 {
    let target = at(0.0, -3000.0, 0.0, 0.0, 0.0);
    let goal = Goal::Attack(ShipRef::Player);
    let mut fighter = ship(fields, goal, at(0.0, 0.0, 0.0, 0.0, heading), 0.0);
    fighter.stats.handling.turn_rate = rate;
    fighter.pursuit.frame = Some(Frame {
        goal,
        bodies: [
            Some(Body {
                manoeuvre,
                heading: 90,
            }),
            None,
        ],
        speed_cap: None,
    });
    assert_eq!(fly(&mut fighter, Some(&target)), Outcome::Flying);
    fighter.state.heading
}

#[test]
fn a_fighter_within_its_turn_of_its_bodys_heading_is_put_on_it() {
    // 88.6 is 2 whole degrees off 90, within 3 (@0x34581-0x3458e).
    for fields in [FAST, RAVEN] {
        for manoeuvre in BODIES {
            for heading in [88.6, 92.9] {
                assert_eq!(
                    fought(fields, manoeuvre, heading, 3.0),
                    90.0,
                    "{manoeuvre:?} from {heading}, flags2 {:#x}",
                    fields.flags2
                );
            }
        }
    }
}

#[test]
fn a_fighter_measures_its_gap_to_its_bodys_heading_in_whole_degrees() {
    // 88.1 is 1.9 off 90, within 1.95, but 88 is 2 whole degrees off: it
    // turns by its rate.
    for fields in [FAST, RAVEN] {
        for manoeuvre in BODIES {
            let heading = fought(fields, manoeuvre, 88.1, 1.95);
            assert!(
                (heading - 90.05).abs() < 1e-4,
                "{manoeuvre:?}, flags2 {:#x}: {heading}",
                fields.flags2
            );
        }
    }
}

// Sub 8, a fighter docking inside its dock window.

/// A fighter of [`FAST`] turning `rate` degrees a tick, docking with the
/// player at the centre from `position`, facing `heading`, its dock
/// window 100 and the player's sprite 40 across.
fn docking(position: Vec2, heading: f32, rate: f32) -> Npc {
    let mut fighter = ship(
        FAST,
        Goal::Dock(ShipRef::Player),
        at(position.x, position.y, 0.0, 0.0, heading),
        0.0,
    );
    fighter.stats.handling.turn_rate = rate;
    fighter.carrier = Some(crate::bay::Carrier {
        ship: ShipRef::Player,
        window: 100.0,
        reach: 40.0,
    });
    fighter
}

#[test]
fn a_docking_fighter_within_its_turn_of_the_tables_bearing_is_put_on_it() {
    // The table puts the carrier at 134 from (-60, -60), where the float
    // bearing is 135 (`_CalcInterceptAngle` @0x87e81-0x87e9d).
    let carrier = at(0.0, 0.0, 0.0, 0.0, 0.0);
    let from = Vec2::new(-60.0, -60.0);
    assert_eq!(trig::intercept_angle(from, carrier.position), 134);
    assert_eq!(bearing(from, carrier.position), 135.0);
    for heading in [135.4, 131.9] {
        let mut fighter = docking(from, heading, 3.0);
        assert_eq!(fly(&mut fighter, Some(&carrier)), Outcome::Flying);
        assert_eq!(fighter.state.heading, 134.0, "from {heading}");
    }
}

#[test]
fn a_docking_fighter_measures_its_gap_to_the_bearing_in_whole_degrees() {
    // 178.1 is 1.9 off 180, within 1.95, but 178 is 2 whole degrees off.
    let carrier = at(0.0, 0.0, 0.0, 0.0, 0.0);
    let mut fighter = docking(Vec2::new(0.0, -60.0), 178.1, 1.95);
    fly(&mut fighter, Some(&carrier));
    let heading = fighter.state.heading;
    assert!((heading - 180.05).abs() < 1e-4, "{heading}");
}

#[test]
fn a_docking_fighter_thrusts_strictly_within_its_turn_and_one_whole_degree() {
    // 1 degree a tick: 2 whole degrees off the bearing (180) is not less
    // than 2 (@0x87ea9-0x87ee4), 1 is; read before the turn.
    let carrier = at(0.0, 0.0, 0.0, 0.0, 0.0);
    let thrusts = |heading: f32| {
        let mut fighter = docking(Vec2::new(0.0, -60.0), heading, 1.0);
        fly(&mut fighter, Some(&carrier));
        fighter.state.velocity != Vec2::ZERO
    };
    // 178.9 is 178 in whole degrees, where its float 1.1 would thrust.
    for heading in [178.0, 178.9, 182.0, 182.9, 177.5] {
        assert!(!thrusts(heading), "{heading}");
    }
    for heading in [179.0, 179.5, 180.0, 181.9] {
        assert!(thrusts(heading), "{heading}");
    }
}

// The flights that want no heading.

#[test]
fn a_ship_holding_or_assisting_with_no_docking_has_its_heading_truncated() {
    // State 0 has no handler (@0x851f1) and sub 0xb with no target does
    // nothing (@0x87345-0x87351): +0x68 stays the low level's default.
    for goal in [Goal::Hold, Goal::Assist(Help::Refuel), Goal::Drift] {
        for fields in [FAST, RAVEN] {
            let mut coasting = ship(fields, goal, at(0.0, 0.0, 1.0, 0.0, 37.6), 1.0);
            assert_eq!(fly(&mut coasting, None), Outcome::Flying);
            assert_eq!(coasting.state.heading, 37.0, "{goal:?}");
        }
    }
}

// The engine's stand-ins: `autopilot::brake`, sub 1 for every goal the
// autopilot stops, and the velocity match its approach, rejoin and follow
// fly by. Each wants a whole-degree heading and `_HandleShip` turns it.

/// A ship of `fields` idling at the centre, moving at `velocity` and
/// facing `heading`, turning `rate` degrees a tick: its heading after a
/// tick's brake.
fn braked(fields: ShipFields, velocity: Vec2, heading: f32, rate: f32) -> f32 {
    let start = at(0.0, 0.0, velocity.x, velocity.y, heading);
    let mut idle = ship(fields, Goal::Idle, start, velocity.length());
    idle.stats.handling.turn_rate = rate;
    assert_eq!(fly(&mut idle, None), Outcome::Flying);
    idle.state.heading
}

#[test]
fn a_ship_braking_within_its_turn_of_its_whole_retro_heading_is_put_on_it() {
    // Moving at (1, 1) the table's retro heading is 314, the float's 315
    // (`snipe::retro`, as sub 1 reads it, @0x85440-0x8546e).
    assert_eq!(braked(FAST, Vec2::new(1.0, 1.0), 315.4, 3.0), 314.0);
    assert_eq!(braked(FAST, Vec2::new(0.0, -3.0), 178.6, 3.0), 180.0);
}

#[test]
fn a_ship_braking_measures_its_gap_to_its_retro_heading_in_whole_degrees() {
    // 178.1 is 1.9 off 180, within 1.95, but 178 is 2 whole degrees off.
    let heading = braked(FAST, Vec2::new(0.0, -3.0), 178.1, 1.95);
    assert!((heading - 180.05).abs() < 1e-4, "{heading}");
}

#[test]
fn a_ship_braking_at_rest_or_inertialess_has_its_heading_truncated() {
    let creeping = Vec2::new(0.0, AT_REST_SPEED);
    assert_eq!(braked(FAST, creeping, 37.6, 3.0), 37.0, "at rest");
    // The inertialess brake writes no +0x68 (@0x853d7-0x85403).
    assert_eq!(braked(RAVEN, Vec2::new(0.0, -3.0), 37.6, 3.0), 37.0);
}

#[test]
fn a_ship_braking_from_3_comes_to_rest_on_a_whole_degree() {
    let mut idle = ship(FAST, Goal::Idle, at(0.0, 0.0, 0.0, -3.0, 0.0), 0.0);
    let accel = idle.stats.handling.accel;
    for _ in 0..200 {
        fly(&mut idle, None);
    }
    assert!(
        idle.state.velocity.length() <= accel / 2.0,
        "{:?}",
        idle.state
    );
    assert_eq!(idle.state.heading, idle.state.heading.trunc());
}

/// A ship of `fields` landing on a planet far off at `planet`, at rest
/// at the centre facing `heading`, turning `rate` degrees a tick: its
/// heading after a tick of its approach.
fn landing(fields: ShipFields, planet_at: Vec2, heading: f32, rate: f32) -> f32 {
    let sites = [planet(140, planet_at.x, planet_at.y)];
    let start = at(0.0, 0.0, 0.0, 0.0, heading);
    let mut lander = ship(fields, Goal::Land(StellarId(140)), start, 0.0);
    lander.stats.handling.turn_rate = rate;
    assert_eq!(fly_among(&mut lander, &sites, None), Outcome::Flying);
    lander.state.heading
}

#[test]
fn a_ship_matching_a_velocity_within_its_turn_of_the_changes_table_heading_is_put_on_it() {
    // Wanting (1, 1) the table's heading is 134, the float's 135: the
    // table heading of the change × 100, as sub 1 reads its velocity.
    let down_right = Vec2::new(5000.0, 5000.0);
    for fields in [FAST, RAVEN] {
        assert_eq!(landing(fields, down_right, 135.4, 3.0), 134.0);
        assert_eq!(landing(fields, Vec2::new(0.0, -5000.0), 1.4, 3.0), 0.0);
    }
}

#[test]
fn a_ship_matching_a_velocity_measures_its_gap_in_whole_degrees() {
    // 358.1 is 1.9 off 0, within 1.95, but 358 is 2 whole degrees off.
    for fields in [FAST, RAVEN] {
        let heading = landing(fields, Vec2::new(0.0, -5000.0), 358.1, 1.95);
        assert!((heading - 0.05).abs() < 1e-4, "{heading}");
    }
}

#[test]
fn an_inspector_matching_its_leads_velocity_has_its_heading_truncated() {
    // Within its margin it wants the lead's velocity, which it has.
    let lead = at(50.0, 0.0, 1.0, 0.0, 90.0);
    let goal = Goal::Inspect(ShipRef::Player);
    let mut alongside = ship(FAST, goal, at(0.0, 0.0, 1.0, 0.0, 37.6), 0.0);
    assert_eq!(fly(&mut alongside, Some(&lead)), Outcome::Flying);
    assert_eq!(alongside.state.heading, 37.0);
    // Far off it closes, turned onto the change's heading, 0.
    let far = at(0.0, -3000.0, 0.0, 0.0, 0.0);
    let mut closing = ship(FAST, goal, at(0.0, 0.0, 0.0, 0.0, 1.4), 0.0);
    fly(&mut closing, Some(&far));
    assert_eq!(closing.state.heading, 0.0);
}
