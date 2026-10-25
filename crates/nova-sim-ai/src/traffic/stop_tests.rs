//! Tests that a ship the autopilot stops comes to rest from any start
//! speed, not only a whole number of thrusts: every goal that flies sub 1
//! (`_LowLevelAIHandler` @0x8536d-0x85633) brakes until it is not
//! [`inertialess::moving`], below 0.35 on both axes, then settles by 0.95
//! a tick (@0x855e3-0x85624); a sniper (sub 0xe) by its own
//! [`snipe::moving`] and [`snipe::DAMPING`] (@0x870bf-0x8728e).
#![allow(clippy::float_cmp)]

use super::autopilot::{self, Beside, Outcome, Readings, TopSpeeds};
use super::inertialess;
use super::npc::Npc;
use super::snipe;
use crate::ai::Goal;
use crate::catalog::StellarId;
use crate::combat::ShipRef;
use crate::flight::{AT_REST_SPEED, FlightRules, ShipState, shortest_turn};
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

fn at(vx: f32, vy: f32, heading: f32) -> ShipState {
    ShipState {
        position: Vec2::ZERO,
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

/// Asserts `speed` isn't a whole number of `step`s.
fn not_whole(speed: f32, step: f32) {
    let steps = speed / step;
    assert!((steps - steps.round()).abs() > 0.05, "{speed} / {step}");
}

/// Sub 1's settle of `velocity`: × 0.95 in doubles (@0x855e3-0x8560b).
#[allow(clippy::cast_possible_truncation)]
fn settled(velocity: Vec2) -> Vec2 {
    let cut = |axis: f32| (f64::from(axis) * 0.95) as f32;
    Vec2::new(cut(velocity.x), cut(velocity.y))
}

#[test]
fn an_inertial_ship_stopping_from_a_speed_not_a_whole_number_of_thrusts_comes_to_rest() {
    for goal in [
        Goal::Idle,
        Goal::Wait,
        Goal::CountWait,
        Goal::Land(StellarId(999)),
    ] {
        let mut stopping = ship(FAST, goal, at(0.13, -1.87, 10.0), 0.0);
        not_whole(
            stopping.state.velocity.length(),
            stopping.stats.handling.accel,
        );
        for _ in 0..400 {
            assert_eq!(fly(&mut stopping, None), Outcome::Flying);
        }
        let state = stopping.state;
        assert!(!inertialess::moving(state.velocity), "{goal:?}: {state:?}");
        assert!(
            state.velocity.length() <= AT_REST_SPEED,
            "{goal:?}: {state:?}"
        );
        // A further tick settles it, with no thrust or turn.
        fly(&mut stopping, None);
        assert_eq!(stopping.state.velocity, settled(state.velocity), "{goal:?}");
        assert_eq!(stopping.state.heading, state.heading.trunc(), "{goal:?}");
    }
}

#[test]
fn an_inertialess_ship_stopping_from_a_speed_not_a_whole_number_of_brakes_stops_moving() {
    let mut stopping = ship(RAVEN, Goal::Idle, at(0.0, -2.95, 0.0), 2.95);
    // Its brake is half a tick's thrust (@0x853f6).
    not_whole(stopping.speed, stopping.stats.handling.accel * 0.5);
    for _ in 0..100 {
        fly(&mut stopping, None);
    }
    assert!(
        !inertialess::moving(stopping.state.velocity),
        "{stopping:?}"
    );
    // Not moving, it brakes no more: it keeps its speed, the settle's
    // +0x48 × 0.95 (@0x8560d-0x8561f) being left to
    // ship-handling-fidelity-followups phase-2-ai-drags-scale-the-npc-speed.
    // Its velocity's × 0.95 is overwritten by `adjust_velocity` the same
    // tick, so heading × speed is what shows.
    let speed = stopping.speed;
    assert!(speed > 0.0);
    for _ in 0..10 {
        fly(&mut stopping, None);
        assert_eq!(stopping.speed, speed);
    }
}

/// A sniper at the centre and its target in sight 300 to its left: once
/// it isn't [`snipe::moving`] it slows by [`snipe::DAMPING`] a tick, never
/// thrusting, and turns to within its turn of the table's bearing.
fn sniper_comes_to_rest_and_faces(mut sniper: Npc, inertial: bool) {
    let target = ShipState {
        position: Vec2::new(-300.0, 0.0),
        ..ShipState::default()
    };
    let mut rested = None;
    for tick in 0..400 {
        let before = sniper.state;
        let speed = sniper.speed;
        fly(&mut sniper, Some(&target));
        if !snipe::moving(&before) {
            rested.get_or_insert(tick);
            if inertial {
                assert_eq!(sniper.state.velocity, before.velocity * snipe::DAMPING);
            } else {
                assert_eq!(sniper.speed, speed, "no brake at rest");
            }
        }
    }
    assert!(rested.is_some(), "{sniper:?}");
    let bearing = trig::intercept_angle(sniper.state.position, target.position);
    assert!(
        shortest_turn(sniper.state.heading, f32::from(bearing)).abs()
            <= sniper.stats.handling.turn_rate,
        "{:?} {bearing}",
        sniper.state
    );
}

#[test]
fn an_inertial_sniper_from_a_speed_not_a_whole_number_of_thrusts_rests_and_faces_its_target() {
    let sniper = ship(FAST, Goal::Snipe(ShipRef::Player), at(0.0, -2.93, 0.0), 0.0);
    not_whole(2.93, sniper.stats.handling.accel);
    sniper_comes_to_rest_and_faces(sniper, true);
}

#[test]
fn an_inertialess_sniper_from_a_speed_not_a_whole_number_of_brakes_rests_and_faces_its_target() {
    // Sub 0xe's inertialess brake takes a whole tick's thrust off.
    let sniper = ship(
        RAVEN,
        Goal::Snipe(ShipRef::Player),
        at(0.0, -2.95, 0.0),
        2.95,
    );
    not_whole(2.95, sniper.stats.handling.accel);
    sniper_comes_to_rest_and_faces(sniper, false);
}
