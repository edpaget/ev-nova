//! Tests of a tractor hold read live at the held ship's own step (see
//! [`tractor_hold`](crate::traffic::tractor_hold)): `_HandleShip`'s pull
//! reads the holder's slot as it stands (+0x20/+0x24, @0x3401f-0x34032)
//! and its release tests the slot's +0xb8, `_IsDisabled` and `_IsDying`
//! (@0x353e2-0x3540c) after the flight, so a holder in a lower slot is
//! read as its own flight left it, one in a higher slot as the last pass
//! left it, and the player as it stands at the held ship's step.
#![allow(clippy::float_cmp)]

use std::collections::BTreeSet;

use super::fly_pass::FlyHalf;
use super::{Traffic, World};
use crate::combat::ShipRef;
use crate::combat::hull::Condition;
use crate::combat::impact::TractorLock;
use crate::flight::DISABLED_DRAG;
use crate::geometry::Vec2;
use crate::stats::ShipStats;
use crate::traffic::fixture::npc;
use crate::traffic::npc::{Npc, NpcId};
use nova_sim_base::chance::NeverFires;
use nova_sim_base::fixture::{Draws, FAST};

const HELD: NpcId = NpcId(1);
const HOLDER: NpcId = NpcId(2);

/// A pull step large enough that every pull here snaps the held ship to
/// its holder's velocity (@0x3403a-0x3413d), so the velocity it ends at
/// names the holder's velocity the pull read.
const SNAP: f32 = 1000.0;

/// NPC `id` moving at `velocity`.
fn moving(id: u32, velocity: Vec2) -> Npc {
    let mut ship = npc(id, ShipStats::new(FAST, &[]));
    ship.state.velocity = velocity;
    ship
}

/// NPC 1, disabled so it drifts, at `velocity`, held by `holder`.
fn held_by(holder: ShipRef, velocity: Vec2) -> Npc {
    let mut ship = moving(HELD.0, velocity);
    ship.condition = Condition::Disabled;
    ship.tractor = Some(TractorLock::by(holder));
    ship
}

/// NPC 2, intact, inertialess and moving across its heading, so its own
/// flight changes its velocity (`_AdjustInertialessShipVelocity`
/// @0x3425f).
fn holder() -> Npc {
    let mut ship = moving(HOLDER.0, Vec2::new(3.0, -2.0));
    ship.stats.inertialess_hull = true;
    ship
}

/// A traffic of `npcs`, in that order.
fn traffic_of(npcs: Vec<Npc>) -> Traffic {
    let mut traffic = Traffic::new();
    traffic.npcs = npcs;
    traffic
}

/// A world of no stellars whose tractor pulls by `step`.
fn pulling(step: f32) -> World<'static> {
    World {
        tractor_step: step,
        ..World::new(&[])
    }
}

fn ship(traffic: &Traffic, id: NpcId) -> &Npc {
    (traffic.npcs.iter())
        .find(|npc| npc.id == id)
        .expect("there")
}

/// `velocity` as a disabled ship's drift leaves it after its move.
fn drifted(velocity: Vec2) -> Vec2 {
    velocity * DISABLED_DRAG
}

/// The holder's velocity after its own flight in a pass alone.
fn holder_moved() -> Vec2 {
    let mut alone = traffic_of(vec![holder()]);
    alone.move_in(pulling(SNAP), &mut Draws::of(&[]));
    ship(&alone, HOLDER).state.velocity
}

#[test]
fn a_held_npc_after_its_holder_reads_the_holders_moved_velocity() {
    let before = holder().state.velocity;
    let moved = holder_moved();
    assert_ne!(moved, before, "the holder's flight changes its velocity");
    let mut traffic = traffic_of(vec![
        holder(),
        held_by(ShipRef::Npc(HOLDER), Vec2::new(5.0, 0.0)),
    ]);
    traffic.move_in(pulling(SNAP), &mut Draws::of(&[]));
    assert_eq!(ship(&traffic, HOLDER).state.velocity, moved);
    let held = ship(&traffic, HELD);
    assert_ne!(drifted(moved), drifted(before));
    assert_eq!(held.state.velocity, drifted(moved), "pulled to the moved");
    assert!(held.tractor.is_some(), "kept by an intact holder");
}

/// A regression pin for the other direction: the held ship flies before
/// its holder, so it reads the holder as the last pass left it. It passed
/// before the hold read its holder live, as the snapshot it read then was
/// that velocity too; it fails if the hold reads a holder already flown
/// through the pass (checked by reading the holders from a copy of the
/// traffic flown through the pass, then restored).
#[test]
fn a_held_npc_before_its_holder_reads_the_holders_velocity_from_before_its_move() {
    let before = holder().state.velocity;
    assert_ne!(holder_moved(), before);
    let mut traffic = traffic_of(vec![
        held_by(ShipRef::Npc(HOLDER), Vec2::new(5.0, 0.0)),
        holder(),
    ]);
    traffic.move_in(pulling(SNAP), &mut Draws::of(&[]));
    let held = ship(&traffic, HELD);
    assert_eq!(
        held.state.velocity,
        drifted(before),
        "pulled to the unmoved"
    );
    assert!(held.tractor.is_some());
}

#[test]
fn a_held_npc_reads_the_players_velocity_as_the_world_holds_it_at_its_step() {
    // A lower wreck's shockwave (`_DamageShip` @0x2da89, at its display)
    // can push the player before a higher slot's `_HandleShip` reads it.
    let with_player = |velocity: Vec2| {
        let mut player = crate::ai::fixture::player(0.0, 0.0);
        player.state.velocity = velocity;
        World {
            player: Some(player),
            ..pulling(SNAP)
        }
    };
    let (began, pushed) = (Vec2::new(0.0, 2.0), Vec2::new(-3.0, 1.0));
    let mut traffic = traffic_of(vec![held_by(ShipRef::Player, Vec2::new(5.0, 0.0))]);
    let mut pass = traffic.begin_fly(&with_player(began), &BTreeSet::new(), FlyHalf::Again);
    traffic.fly_npc(&mut pass, HELD, &with_player(pushed), &mut NeverFires);
    traffic.end_fly(pass);
    let held = ship(&traffic, HELD);
    assert_eq!(held.state.velocity, drifted(pushed), "pulled to the pushed");
    assert!(held.tractor.is_some());
}

#[test]
fn a_held_npc_lets_go_of_a_holder_disabled_earlier_in_the_pass() {
    // The release tests the holder's slot live (`_IsDisabled` @0x353ee),
    // so a holder a lower slot's shockwave disabled lets go at once.
    let world = pulling(SNAP);
    let mut traffic = traffic_of(vec![
        held_by(ShipRef::Npc(HOLDER), Vec2::new(5.0, 0.0)),
        holder(),
    ]);
    let mut pass = traffic.begin_fly(&world, &BTreeSet::new(), FlyHalf::Again);
    let index = traffic.npcs.iter().position(|npc| npc.id == HOLDER);
    traffic.npcs[index.expect("there")].condition = Condition::Disabled;
    traffic.fly_npc(&mut pass, HELD, &world, &mut NeverFires);
    let held = ship(&traffic, HELD);
    assert_eq!(
        held.state.velocity,
        drifted(holder().state.velocity),
        "pulled once"
    );
    assert_eq!(held.tractor, None, "released in that pass");
}

#[test]
fn a_holder_taken_out_at_its_step_with_no_record_is_read_as_the_half_began() {
    // A mission ship that leaves as it lands, under `LandedLeaves`' other
    // reading, leaves the traffic at its step with no record (`Freed::by`
    // a landing is none): a later slot's hold reads it as the half began,
    // its velocity and intact, as no slot of the original is freed so.
    let world = pulling(SNAP);
    let mut traffic = traffic_of(vec![
        holder(),
        held_by(ShipRef::Npc(HOLDER), Vec2::new(5.0, 0.0)),
    ]);
    let mut pass = traffic.begin_fly(&world, &BTreeSet::new(), FlyHalf::Again);
    traffic.npcs.retain(|npc| npc.id != HOLDER);
    traffic.fly_npc(&mut pass, HELD, &world, &mut NeverFires);
    let held = ship(&traffic, HELD);
    assert_eq!(held.state.velocity, drifted(holder().state.velocity));
    assert!(held.tractor.is_some(), "kept by the intact holder");
}
