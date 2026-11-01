//! Tests that a ship whose stored sub 1 stands through an AI wait is cut by
//! sub 1 under it: the high level writes no sub while +0x4c runs
//! (@0x8d4b9-0x8d4d2) and the low level runs sub 1 on its kept value
//! (@0x8536d), while `_HandleShip` skips the turn and thrust
//! (@0x34a54-0x34a72). An inertial ship on the soft brake within its retro
//! heading is cut × 0.94 (@0x85557-0x85588), off it only goal 9 is
//! (@0x85592-0x855e1); below 0.35 on both axes it settles × 0.95 and sub 0
//! is stored (@0x855e3-0x85624).
#![allow(clippy::float_cmp)]

use super::autopilot::{self, Beside, Outcome, Readings, TopSpeeds};
use super::npc::Npc;
use crate::ai::Goal;
use crate::flight::{FlightRules, ShipState, drag_by};
use crate::geometry::Vec2;
use crate::hail::Help;
use crate::handling::ShipFields;
use crate::stats::{INERTIALESS_HULL, ShipStats};
use crate::traffic::{Traffic, World};
use nova_sim_base::fixture::{Draws, FAST};
use std::collections::{BTreeMap, BTreeSet};

/// [`FAST`] with an inertialess hull.
const RAVEN: ShipFields = ShipFields {
    flags2: INERTIALESS_HULL,
    ..FAST
};

/// A ship of `fields` on `goal` moving at `velocity` facing `heading`, 2.0
/// along it, its AI wait running and sub 1 stored.
fn waiting(fields: ShipFields, goal: Goal, velocity: Vec2, heading: f32) -> Npc {
    Npc {
        state: ShipState {
            position: Vec2::new(10.0, 20.0),
            velocity,
            heading,
        },
        goal,
        speed: 2.0,
        ai_wait: 5,
        stopping: true,
        ..super::fixture::npc(1, ShipStats::new(fields, &[]))
    }
}

/// Flies `npc` one tick at its own top speed beside nothing, under the
/// original's rules.
fn fly(npc: &mut Npc) -> Outcome {
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
        FlightRules::default(),
        Readings::default(),
        &weapons,
        &mut Draws::of(&[]),
    )
}

/// `npc` and its twin with no sub 1 stored, each flown a tick.
fn flown_with_and_without(mut npc: Npc) -> (Npc, Npc) {
    let mut uncut = Npc {
        stopping: false,
        ..npc.clone()
    };
    assert_eq!(fly(&mut npc), Outcome::Flying);
    fly(&mut uncut);
    (npc, uncut)
}

/// Asserts `npc` was cut `by` before it moved, with no turn: its velocity
/// and speed, from `start` and 2.0.
fn cut(npc: &Npc, start: &ShipState, by: f64) {
    let velocity = drag_by(start.velocity, by);
    assert_eq!(npc.state.velocity, velocity, "{by}");
    #[allow(clippy::cast_possible_truncation)]
    let speed = (2.0_f64 * by) as f32;
    assert_eq!(npc.speed, speed, "{by}");
    assert_eq!(npc.state.position, start.position + velocity, "then moved");
    assert_eq!(npc.state.heading, start.heading, "no turn");
}

#[test]
fn an_inertial_ship_on_the_soft_brake_within_its_retro_heading_is_cut_by_0_94() {
    // Moving up at 1, its retro heading is 180.
    let ship = waiting(FAST, Goal::Wait, Vec2::new(0.0, -1.0), 180.0);
    let start = ship.state;
    let (ship, _) = flown_with_and_without(ship);
    cut(&ship, &start, 0.94);
    assert!(ship.stopping, "sub 1 stands");
}

#[test]
fn turned_away_it_moves_on_uncut_but_in_goal_9() {
    let ship = waiting(FAST, Goal::Wait, Vec2::new(0.0, -1.0), 90.0);
    let start = ship.state;
    let (ship, uncut) = flown_with_and_without(ship);
    assert_eq!((ship.state, ship.speed), (uncut.state, uncut.speed));
    assert_eq!(ship.state.position, start.position + start.velocity);
    assert!(ship.stopping);
    // Goal 9, the refuel (+0xc8c8 @0x85595), is cut off it too.
    let refuelling = waiting(FAST, Goal::Assist(Help::Refuel), start.velocity, 90.0);
    let (refuelling, _) = flown_with_and_without(refuelling);
    cut(&refuelling, &start, 0.94);
}

#[test]
fn on_the_full_brake_it_moves_on_uncut() {
    // 1.75 or more on an axis: the full brake only writes +0x30 (@0x85510).
    let ship = waiting(FAST, Goal::Wait, Vec2::new(0.0, -1.75), 180.0);
    let (ship, uncut) = flown_with_and_without(ship);
    assert_eq!((ship.state, ship.speed), (uncut.state, uncut.speed));
    assert!(ship.stopping);
}

#[test]
fn an_inertialess_ship_moves_on_uncut() {
    // Its brake writes +0x30 alone (@0x853d7-0x853fe).
    let ship = waiting(RAVEN, Goal::Wait, Vec2::new(0.0, -2.0), 0.0);
    let (ship, uncut) = flown_with_and_without(ship);
    assert_eq!((ship.state, ship.speed), (uncut.state, uncut.speed));
    assert!(ship.stopping);
}

#[test]
fn below_0_35_it_settles_once_by_0_95_and_stores_sub_0() {
    for fields in [FAST, RAVEN] {
        let ship = waiting(fields, Goal::Wait, Vec2::new(0.0, -0.2), 180.0);
        let start = ship.state;
        let (ship, _) = flown_with_and_without(ship);
        if fields == FAST {
            cut(&ship, &start, 0.95);
        } else {
            #[allow(clippy::cast_possible_truncation)]
            let speed = (2.0_f64 * 0.95) as f32;
            assert_eq!(ship.speed, speed);
        }
        assert!(!ship.stopping, "the settle's sub 0 (@0x85624)");
        // Uncut on the ticks after, sub 0 stored.
        let (after, uncut) = flown_with_and_without(ship);
        assert_eq!((after.state, after.speed), (uncut.state, uncut.speed));
        assert!(!after.stopping);
    }
}

#[test]
fn the_traffic_leaves_a_waiting_ships_sub_1_for_its_low_level_to_settle() {
    let mut traffic = Traffic::with_interval(1);
    let ship = waiting(FAST, Goal::Wait, Vec2::new(0.0, -0.2), 180.0);
    let start = ship.state;
    traffic.add_npc(ship);
    traffic.fly_holding(World::new(&[]), &BTreeSet::new(), &mut Draws::of(&[]));
    let ship = &traffic.npcs()[0];
    assert_eq!(
        ship.state.velocity,
        drag_by(start.velocity, 0.95),
        "settled"
    );
    assert!(!ship.stopping, "the settle's sub 0");
}
