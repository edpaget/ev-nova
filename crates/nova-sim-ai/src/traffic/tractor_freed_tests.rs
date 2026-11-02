//! Tests of a tractor hold whose holder's slot was freed (see
//! [`tractor_hold`](crate::traffic::tractor_hold)): `_HandleShip`'s pull
//! reads the holder's slot (+0x20/+0x24, @0x3401f-0x34032) with no +0xb8
//! test and its release tests the slot after the flight
//! (@0x353ba-0x35449), so a ship whose holder's slot was freed is pulled
//! once more by the slot's record, then released, at its next
//! `_HandleShip`; and a ship whose holder's berth was taken before its
//! step is held by the newcomer.
#![allow(clippy::float_cmp)]

use std::collections::BTreeSet;

use super::fly_pass::FlyHalf;
use super::{Traffic, World};
use crate::catalog::StellarId;
use crate::combat::ShipRef;
use crate::combat::hull::Condition;
use crate::combat::impact::TractorLock;
use crate::flight::DISABLED_DRAG;
use crate::geometry::Vec2;
use crate::stats::ShipStats;
use crate::traffic::fixture::npc;
use crate::traffic::npc::{Mode, Npc, NpcId};
use nova_sim_base::chance::NeverFires;
use nova_sim_base::fixture::{Draws, FAST};

/// NPC `id` moving at `velocity`.
fn moving(id: u32, velocity: Vec2) -> Npc {
    let mut ship = npc(id, ShipStats::new(FAST, &[]));
    ship.state.velocity = velocity;
    ship
}

/// A ship disabled so it drifts, at `velocity`, held by `holder`.
fn held_by(holder: NpcId, velocity: Vec2) -> Npc {
    let mut ship = moving(0, velocity);
    ship.condition = Condition::Disabled;
    ship.tractor = Some(TractorLock::by(ShipRef::Npc(holder)));
    ship
}

/// A world of no stellars whose tractor pulls by 0.5.
fn pulling() -> World<'static> {
    World {
        tractor_step: 0.5,
        ..World::new(&[])
    }
}

/// An advance pass's fly half of `traffic`.
fn advance(traffic: &mut Traffic) {
    traffic.fly_holding(pulling(), &BTreeSet::new(), &mut Draws::of(&[]));
}

/// Double time's second pass of `traffic`.
fn again(traffic: &mut Traffic) {
    traffic.move_in(pulling(), &mut Draws::of(&[]));
}

fn ship(traffic: &Traffic, id: NpcId) -> &Npc {
    traffic
        .npcs()
        .iter()
        .find(|npc| npc.id == id)
        .expect("there")
}

/// A traffic of a holder at berth 3 moving at (1, 0) and a ship at berth
/// `berth` it holds, moving at (5, 0), in slot order: the traffic, the
/// holder and the held ship.
fn holding(berth: u32) -> (Traffic, NpcId, NpcId) {
    let mut traffic = Traffic::new();
    let below = berth < 3;
    let holder = NpcId(traffic.next_id + u32::from(below));
    let ships = [
        (moving(0, Vec2::new(1.0, 0.0)), 3),
        (held_by(holder, Vec2::new(5.0, 0.0)), berth),
    ];
    let mut ids = [NpcId(0); 2];
    for at in if below { [1, 0] } else { [0, 1] } {
        let (ship, berth) = ships[at].clone();
        ids[at] = traffic.add_npc_at(ship, berth);
    }
    assert_eq!(ids[0], holder);
    (traffic, ids[0], ids[1])
}

/// Asserts that `held` was pulled toward (1, 0) by a step, then drifted,
/// and let go: pulled by the record and released.
fn pulled_and_released(traffic: &Traffic, held: NpcId, why: &str) {
    let ship = ship(traffic, held);
    assert_eq!(
        ship.state.velocity,
        Vec2::new(4.5, 0.0) * DISABLED_DRAG,
        "{why}: pulled by the record"
    );
    assert_eq!(ship.tractor, None, "{why}: released");
}

#[test]
fn a_ship_held_by_a_holder_killed_after_the_ship_loop_is_pulled_by_its_record_then_released_at_the_next_pass()
 {
    // A deadly stellar's kill (@0x39dac) frees the slot after the ship
    // loop; the next pass is double time's second when there is one, else
    // the next advance pass.
    for (pass, why) in [
        (again as fn(&mut Traffic), "the second pass"),
        (advance, "the next advance pass"),
    ] {
        let (mut traffic, holder, held) = holding(5);
        traffic.remove_killed(holder);
        pass(&mut traffic);
        pulled_and_released(&traffic, held, why);
    }
}

#[test]
fn a_ship_held_by_a_person_swapped_on_the_hail_is_pulled_by_its_record_then_released() {
    // The hail's swap frees the slot in a dialog (@0x62520), with no
    // `_HandleShip` before the next pass.
    let (mut traffic, holder, held) = holding(5);
    traffic.vacate(holder);
    again(&mut traffic);
    pulled_and_released(&traffic, held, "after the dialog");
}

/// `traffic`'s ship `id` entering a gate, its wait ending on its next
/// flight.
fn entering(traffic: &mut Traffic, id: NpcId) {
    let entrant = traffic.npcs_mut().iter_mut().find(|npc| npc.id == id);
    entrant.expect("there").mode = Mode::EnteringGate {
        gate: StellarId(130),
        ticks_left: 1.0,
    };
}

#[test]
fn a_ship_below_a_gate_entrant_holding_it_is_pulled_by_its_record_and_released_at_the_next_pass() {
    // Its `_HandleShip` comes before the entrant's display frees the slot
    // (@0x2c078): pulled by the live holder and kept, then pulled by the
    // record at the next pass, and let go.
    let (mut traffic, holder, held) = holding(1);
    entering(&mut traffic, holder);
    advance(&mut traffic);
    assert!(traffic.npcs().iter().all(|npc| npc.id != holder), "entered");
    let ship = ship(&traffic, held);
    assert_eq!(ship.state.velocity, Vec2::new(4.5, 0.0) * DISABLED_DRAG);
    assert!(ship.tractor.is_some(), "kept on the entry's pass");
    let before = ship.state.velocity;
    again(&mut traffic);
    let ship = self::ship(&traffic, held);
    assert_eq!(
        ship.state.velocity,
        Vec2::new(before.x - 0.5, 0.0) * DISABLED_DRAG,
        "pulled by the record"
    );
    assert_eq!(ship.tractor, None, "released");
}

#[test]
fn a_ship_above_a_gate_entrant_holding_it_is_pulled_by_its_record_and_released_in_that_pass() {
    // The entrant's slot is freed at its own step of the ship loop
    // (@0x2c078), before a later slot's `_HandleShip` reads it.
    // Its record at rest, as the entry stops it, where the pass began with
    // it at (-2, 0).
    let (mut traffic, holder, held) = holding(5);
    entering(&mut traffic, holder);
    for npc in traffic.npcs_mut() {
        npc.state.velocity = if npc.id == holder {
            Vec2::new(-2.0, 0.0)
        } else {
            Vec2::new(-1.5, 0.0)
        };
    }
    advance(&mut traffic);
    let record = (traffic.gone.ships().iter()).find(|gone| gone.ship.id == holder);
    let velocity = record.expect("recorded").ship.state.velocity;
    assert_eq!(velocity, Vec2::ZERO, "the entrant's record");
    let ship = self::ship(&traffic, held);
    assert_eq!(
        ship.state.velocity,
        Vec2::new(-1.0, 0.0) * DISABLED_DRAG,
        "pulled toward the record"
    );
    assert_eq!(ship.tractor, None, "released in that pass");
}

#[test]
fn a_ship_whose_holders_berth_is_taken_before_its_step_is_held_and_pulled_by_the_newcomer() {
    // A spawn rewrites the slot and leaves every other ship's +0xc8dc
    // (`_GenericRandomShipSpawn` @0x3c8b1-0x3c90b): an arrival after a
    // deadly stellar's kill is the held ship's holder at its next step.
    let (mut traffic, holder, held) = holding(5);
    traffic.remove_killed(holder);
    let newcomer = traffic.add_npc_at(moving(0, Vec2::new(0.0, 2.0)), 3);
    again(&mut traffic);
    let ship = ship(&traffic, held);
    assert_eq!(
        ship.tractor.map(|lock| lock.holder),
        Some(ShipRef::Npc(newcomer))
    );
    assert_eq!(ship.state.velocity, Vec2::new(4.5, 0.5) * DISABLED_DRAG);
}

#[test]
fn a_hold_on_another_ship_is_kept_through_a_take() {
    let (mut traffic, holder, held) = holding(5);
    let other = traffic.add_npc_at(moving(0, Vec2::ZERO), 4);
    let ship = traffic.npcs_mut().iter_mut().find(|npc| npc.id == held);
    let lock = TractorLock {
        holder: ShipRef::Npc(other),
        ticks_since_touch: 3,
    };
    ship.expect("there").tractor = Some(lock);
    traffic.remove_killed(holder);
    traffic.add_npc_at(moving(0, Vec2::ZERO), 3);
    assert_eq!(self::ship(&traffic, held).tractor, Some(lock));
    let own = TractorLock {
        holder: ShipRef::Npc(held),
        ticks_since_touch: 3,
    };
    let ship = traffic.npcs_mut().iter_mut().find(|npc| npc.id == held);
    ship.expect("there").tractor = Some(own);
    traffic.remove_killed(other);
    traffic.add_npc_at(moving(0, Vec2::ZERO), 4);
    assert_eq!(self::ship(&traffic, held).tractor, Some(own), "a self-hold");
}

#[test]
fn a_ship_launched_in_the_loop_into_its_holders_freed_berth_holds_it_at_its_step() {
    // A wreck displayed earlier in the loop frees its berth (@0x2dee0); a
    // launch there (`_LaunchFighter` @0x354ea) is the holder a later slot's
    // `_HandleShip` reads.
    let mut traffic = Traffic::new();
    let mut wreck = moving(0, Vec2::ZERO);
    wreck.condition = Condition::Dying {
        ticks_left: 1,
        starting: false,
    };
    let wreck = traffic.add_npc_at(wreck, 1);
    let held = traffic.add_npc_at(held_by(wreck, Vec2::new(5.0, 0.0)), 5);
    let world = pulling();
    traffic.set_ship_loop(true);
    let mut pass = traffic.begin_fly(&world, &BTreeSet::new(), FlyHalf::Again);
    let index = traffic.npcs().iter().position(|npc| npc.id == wreck);
    traffic.npcs_mut()[index.expect("there")].condition = Condition::Destroyed;
    let launched = traffic.add_npc_at(moving(0, Vec2::new(1.0, 0.0)), 1);
    traffic.fly_npc(&mut pass, held, &world, &mut NeverFires);
    let ship = ship(&traffic, held);
    assert_eq!(
        ship.tractor.map(|lock| lock.holder),
        Some(ShipRef::Npc(launched)),
        "kept"
    );
    assert_eq!(ship.state.velocity, Vec2::new(4.5, 0.0) * DISABLED_DRAG);
}
