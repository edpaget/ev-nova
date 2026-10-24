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
use crate::combat::ShipRef;
use crate::combat::aim::bearing;
use crate::flight::{FlightRules, ShipState};
use crate::geometry::Vec2;
use crate::handling::ShipFields;
use crate::stats::{INERTIALESS_HULL, ShipStats};
use crate::trig;
use nova_sim_base::fixture::{Draws, FAST};
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
    let speeds = TopSpeeds::own_of(npc);
    let weapons = crate::ai::fixture::records_of([&*npc]);
    autopilot::fly(
        npc,
        &[],
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
