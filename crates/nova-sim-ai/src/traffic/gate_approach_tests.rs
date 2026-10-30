//! A ship entering a hypergate or wormhole outside the gate's box, flown
//! through [`autopilot::fly`]: `_HighLevelAIHandler` state 20 sets sub 2
//! there (@0x8dc42-0x8dc55), so by the engine's reading of
//! [`RuleKey::NpcLanding`](crate::RuleKey::NpcLanding) it flies
//! [`approach_flight::toward`] as a landing does, whose turn, window and
//! cap that module's tests pin; by the other reading it flies the engine's
//! braking approach. See [`gate_entry`](crate::traffic::gate_entry).

#![allow(clippy::float_cmp)]

use super::autopilot::{self, Beside, Outcome, Readings, TopSpeeds};
use super::held_burst::{self, Word};
use super::inertialess::Inertia;
use super::npc::{Mode, Npc};
use super::{approach_flight, gate_entry};
use crate::ai::Goal;
use crate::catalog::{LandingSite, StellarId};
use crate::flight::{FlightRules, ShipState};
use crate::geometry::Vec2;
use crate::handling::ShipFields;
use crate::rulebook::RuleSource;
use crate::stats::{INERTIALESS_HULL, ShipStats};
use nova_sim_base::fixture::{Draws, FAST};
use nova_sim_base::speed_cap::SpeedCap;
use nova_sim_catalog::fixture::planet;
use std::collections::BTreeMap;

/// The gate: a hypergate at (500, 0), 100 wide, so a reach of 25.
const GATE: StellarId = StellarId(300);

fn gate() -> LandingSite {
    LandingSite {
        flags2: crate::gate::HYPERGATE,
        ..planet(300, 500.0, 0.0)
    }
}

/// [`FAST`] with an inertialess hull.
const RAVEN: ShipFields = ShipFields {
    flags2: INERTIALESS_HULL,
    ..FAST
};

/// A ship of `fields` entering the gate, `(dx, dy)` off it, moving `(vx,
/// vy)`, facing `heading`.
fn entering(fields: ShipFields, (dx, dy): (f32, f32), (vx, vy): (f32, f32), heading: f32) -> Npc {
    Npc {
        state: ShipState {
            position: gate().position + Vec2::new(dx, dy),
            velocity: Vec2::new(vx, vy),
            heading,
        },
        goal: Goal::EnterGate(GATE),
        ..super::fixture::npc(1, ShipStats::new(fields, &[]))
    }
}

/// Flies `npc` a tick toward the gate under `flight`, by `reading` of
/// `npc_landing`.
fn fly_by(npc: &mut Npc, flight: FlightRules, reading: RuleSource) -> Outcome {
    let speeds = TopSpeeds::own_of(npc);
    autopilot::fly(
        npc,
        &[gate()],
        &BTreeMap::new(),
        None,
        false,
        Beside::default(),
        speeds,
        flight,
        Readings {
            npc_landing: reading,
            ..Readings::default()
        },
        &crate::ai::NO_WEAPONS,
        &mut Draws::of(&[]),
    )
}

/// Flies `npc` a tick by the original's rules and readings.
fn fly(npc: &mut Npc) -> Outcome {
    fly_by(npc, FlightRules::default(), RuleSource::Engine)
}

/// `npc`'s twin flown a tick of sub 2's approach to the gate under
/// `flight`, as its hull says.
fn approached(npc: &Npc, flight: FlightRules) -> Npc {
    let mut twin = npc.clone();
    let handling = twin.flying_handling(TopSpeeds::own_of(&twin).own, flight);
    let inertia = Inertia::of(&twin, None, false, flight);
    let (controls, capped) = approach_flight::toward(&mut twin.state, gate().position, &handling);
    held_burst::step(&mut twin, &capped, controls, flight, inertia, Word::Writes);
    twin
}

#[test]
fn outside_the_box_it_flies_sub_2s_approach() {
    let circle = FlightRules {
        speed_cap: SpeedCap::Circle,
        ..FlightRules::default()
    };
    for fields in [FAST, RAVEN] {
        for flight in [FlightRules::default(), circle] {
            // Just outside the box on each axis, within 500 and beyond it,
            // facing it, off it, and fast.
            for (off, v, heading) in [
                ((26.0, 0.0), (0.0, 0.0), 270.0),
                ((0.0, -26.0), (0.0, 0.0), 0.0),
                ((-300.0, 40.0), (8.0, 0.0), 90.0),
                ((-300.0, 0.0), (0.0, 3.0), 200.0),
                ((-3000.0, 0.0), (0.0, 0.0), 90.0),
            ] {
                let mut npc = entering(fields, off, v, heading);
                npc.speed = v.0.hypot(v.1);
                let twin = approached(&npc, flight);
                assert_eq!(
                    fly_by(&mut npc, flight, RuleSource::Engine),
                    Outcome::Flying
                );
                assert_eq!(npc, twin, "{off:?} {v:?} {heading}");
                assert_eq!(npc.mode, Mode::Flying, "{off:?}");
            }
        }
    }
}

#[test]
fn closing_fast_it_never_brakes_and_still_enters_in_16_ticks() {
    // At 8 a tick from 300 out its stopping distance is far past what is
    // left, but sub 2 has no brake.
    let mut npc = entering(FAST, (-300.0, 0.0), (8.0, 0.0), 90.0);
    let mut ticks = 0;
    while npc.mode == Mode::Flying {
        assert_eq!(fly(&mut npc), Outcome::Flying);
        if npc.mode == Mode::Flying {
            assert_eq!(npc.state.heading, 90.0, "no retro turn");
            assert_eq!(npc.state.velocity.x, 8.0, "no brake");
        }
        ticks += 1;
        assert!(ticks < 100, "{:?}", npc.state);
    }
    assert_eq!(npc.state.velocity, Vec2::ZERO, "stopped dead in the box");
    // The tick it reached the box was the first of 16.
    for _ in 2..gate_entry::ENTRY_TICKS {
        assert_eq!(fly(&mut npc), Outcome::Flying);
    }
    assert_eq!(fly(&mut npc), Outcome::EnteredGate(GATE));
}

#[test]
fn its_first_thrust_within_500_drops_an_owed_burst_and_flies_capped() {
    // Owing a burst it thrusts by its capped top speed, which writes +0x34.
    let mut npc = entering(FAST, (-300.0, 0.0), (0.0, 0.0), 90.0);
    npc.owed_burst = Some(-30.0);
    let twin = approached(&npc, FlightRules::default());
    fly(&mut npc);
    assert_eq!(npc.owed_burst, None, "sub 2 writes +0x34");
    assert_eq!(npc, twin);
    // From rest, its speed at the quarter cap: thrust no further.
    let top = npc.own_handling().max_speed;
    let mut at_cap = entering(FAST, (-300.0, 0.0), (top * 0.25, 0.0), 90.0);
    fly(&mut at_cap);
    assert_eq!(at_cap.state.velocity.x, top * 0.25, "capped at a quarter");
}

#[test]
fn by_the_other_reading_closing_fast_it_brakes_by_its_stopping_distance() {
    let mut npc = entering(FAST, (-300.0, 0.0), (8.0, 0.0), 90.0);
    fly_by(&mut npc, FlightRules::default(), RuleSource::Bible);
    assert_eq!(npc.state.heading, 93.0, "turning to its retro heading");
    assert_eq!(npc.mode, Mode::Flying);
}
