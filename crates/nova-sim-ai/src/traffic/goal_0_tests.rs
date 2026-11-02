//! Tests that a ship in goal 0 coasts on sub 0, as `_HighLevelAIHandler`'s
//! tail (@0x8fb09-0x8fb13) and `_LowLevelAIHandler`'s head
//! (@0x851da-0x85236) have it: no thrust (+0x30 = 0, @0x851f1), its own
//! whole heading (+0x68 = trunc(+0x44), @0x85228-0x8522d), moving on at its
//! velocity, and keeping what it owes of a burst (`_HandleShip` bursts only
//! on a +0x30 not 0, @0x34a84). The gone-target flights that stand in for a
//! goal-0 frame coast too.
#![allow(clippy::float_cmp)]

use super::autopilot::{self, Beside, Outcome, Readings, TopSpeeds};
use super::npc::Npc;
use super::ship_display::Thrust;
use crate::ai::Goal;
use crate::combat::ShipRef;
use crate::flight::{FlightRules, FormationFlight, ShipState};
use crate::geometry::Vec2;
use crate::handling::ShipFields;
use crate::stats::{INERTIALESS_HULL, ShipStats};
use nova_sim_base::fixture::{Draws, FAST};
use std::collections::BTreeMap;

/// [`FAST`] with an inertialess hull.
const RAVEN: ShipFields = ShipFields {
    flags2: INERTIALESS_HULL,
    ..FAST
};

/// An NPC of `fields` on `goal` moving (4, -3) on heading 10.7, its drive
/// last recorded as a thrust: an inertial ship holding no speed along its
/// heading, an inertialess one ([`RAVEN`]) 5.
fn ship(fields: ShipFields, goal: Goal) -> Npc {
    let mut npc = Npc {
        state: ShipState {
            position: Vec2::new(100.0, -50.0),
            velocity: Vec2::new(4.0, -3.0),
            heading: 10.7,
        },
        goal,
        speed: if fields == RAVEN { 5.0 } else { 0.0 },
        ..super::fixture::npc(1, ShipStats::new(fields, &[]))
    };
    npc.display.drive.thrust = Thrust::Plain;
    npc
}

/// Flies `npc` one tick at its own top speed beside nothing, under
/// `flight`.
fn fly_by(npc: &mut Npc, flight: FlightRules) -> Outcome {
    let speeds = TopSpeeds::own_of(npc);
    let weapons = crate::ai::fixture::records_of([&*npc]);
    autopilot::fly(
        npc,
        &[],
        &BTreeMap::new(),
        None,
        false,
        Beside::default(),
        speeds,
        flight,
        Readings::default(),
        &weapons,
        &mut Draws::of(&[]),
    )
}

/// Asserts a ship of `fields` on `goal` ([`ship`]), owing `owed` of a
/// burst, coasts as a held ship (sub 0, [`Goal::Hold`]) does, tick by
/// tick, under the original's rules: no thrust, its heading truncated, its
/// speed along its heading and what it owes kept, never left in sub 1,
/// and, inertial, its velocity (an inertialess ship's follows heading ×
/// speed).
fn coasts_as_sub_0(fields: ShipFields, goal: Goal, owed: Option<f32>) {
    let mut ship = Npc {
        owed_burst: owed,
        ..ship(fields, goal)
    };
    let mut held = Npc {
        goal: Goal::Hold,
        ..ship.clone()
    };
    let flight = FlightRules::default();
    let start = ship.state;
    let speed = ship.speed;
    for tick in 0..20 {
        assert_eq!(fly_by(&mut ship, flight), Outcome::Flying);
        fly_by(&mut held, flight);
        assert_eq!(ship.state, held.state, "{goal:?} {tick}: as sub 0 flies");
        assert_eq!(ship.speed, held.speed, "{goal:?} {tick}");
        assert_eq!(ship.state.heading, 10.0, "{goal:?} {tick}: truncated");
        assert_eq!(ship.speed, speed, "{goal:?} {tick}: no brake");
        assert_eq!(ship.owed_burst, owed, "{goal:?} {tick}: kept, unburst");
        assert_eq!(ship.display.drive.thrust, Thrust::None, "{goal:?} {tick}");
        assert!(!ship.stopping, "{goal:?} {tick}: no sub 1");
        if fields != RAVEN {
            assert_eq!(ship.state.velocity, start.velocity, "{goal:?} {tick}");
        }
    }
    if fields != RAVEN {
        let moved = start.position + start.velocity * 20.0;
        let off = (ship.state.position - moved).length();
        assert!(off < 1e-3, "{goal:?} {:?}: it moves on", ship.state);
    }
}

#[test]
fn an_idle_ship_coasts_on_its_velocity_with_no_thrust_on_its_whole_heading() {
    for fields in [FAST, RAVEN] {
        coasts_as_sub_0(fields, Goal::Idle, None);
    }
}

#[test]
fn an_idle_ship_owing_a_burst_keeps_it_unburst() {
    for fields in [FAST, RAVEN] {
        coasts_as_sub_0(fields, Goal::Idle, Some(-30.0));
    }
}

#[test]
fn a_fight_whose_target_is_gone_coasts_as_goal_0_does() {
    // The flee block's -1 (@0x8dff6 → @0x8e228), the attack block's gone
    // target (@0x8e3e6 → @0x8e714) and goal 0xd's (@0x8e85d → @0x8eb3c)
    // all write goal 0, and the tail sub 0.
    for goal in [
        Goal::Attack(ShipRef::Player),
        Goal::Flee(ShipRef::Player),
        Goal::Plunder {
            ship: ShipRef::Player,
            hopeless: false,
        },
    ] {
        for fields in [FAST, RAVEN] {
            coasts_as_sub_0(fields, goal, None);
        }
    }
}

#[test]
fn by_the_originals_flight_an_inspector_whose_ship_is_gone_coasts() {
    // Goal 7's gone let-go (@0x8f158 → @0x8f3a5-0x8f3ae) is goal 0.
    for fields in [FAST, RAVEN] {
        coasts_as_sub_0(fields, Goal::Inspect(ShipRef::Player), None);
    }
}

#[test]
fn by_the_earlier_flight_an_inspector_whose_ship_is_gone_still_brakes() {
    let earlier = FlightRules {
        formation_flight: FormationFlight::Earlier,
        ..FlightRules::default()
    };
    let mut inspector = ship(FAST, Goal::Inspect(ShipRef::Player));
    for _ in 0..200 {
        fly_by(&mut inspector, earlier);
    }
    assert!(
        inspector.state.velocity.length() < 0.5,
        "its follow's brake: {:?}",
        inspector.state
    );
}
