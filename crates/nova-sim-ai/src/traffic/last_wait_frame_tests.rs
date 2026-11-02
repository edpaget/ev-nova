//! Tests of a hit on the frame a ship's gate wait runs out
//! ([`last_wait_frame`](super::last_wait_frame)): `_AIMakeShipAttackPlayer`
//! (@0x89d3e) and `_DoGoodSamaritan` (@0x82b82-0x82b97) write goal 4 over
//! goal 0x15 until the pass's dispatch turns it into goal 8
//! (@0x8dd61-0x8dd93), so a hit before the dispatch, or in double time's
//! second pass, holds the ship; one after it lets it burst on.

#![allow(clippy::float_cmp)]

use super::emerge::exit_speed;
use super::npc::{Mode, Npc, NpcId};
use super::{Traffic, World};
use crate::ai::{Behaviour, Goal, Reaction, Surroundings};
use crate::catalog::StellarId;
use crate::chance::{Chance, NeverFires};
use crate::combat::{ShipRef, Strike};
use crate::geometry::Vec2;
use crate::stats::ShipStats;
use nova_sim_base::fixture::FAST;
use std::collections::BTreeSet;

/// The gate the ships come out of.
const GATE: StellarId = StellarId(300);

/// Every ship keeps its goal; the ship hit turns on the ship that hit
/// it, and every other ship, a Good Samaritan, attacks the player.
#[derive(Debug)]
struct Answering;

impl Behaviour for Answering {
    fn decide(&self, npc: &Npc, _: &Surroundings, _: &mut dyn Chance) -> Goal {
        npc.goal
    }

    fn target(&self, npc: &Npc, _: &Surroundings) -> Option<ShipRef> {
        npc.target
    }

    fn react(&self, npc: &Npc, strike: &Strike, _: &Surroundings) -> Reaction {
        if strike.ship == ShipRef::Npc(npc.id) {
            Reaction {
                target: strike.by,
                provoked: strike.damage,
                goal: Some(Goal::Attack(ShipRef::Player)),
            }
        } else {
            Reaction {
                target: Some(ShipRef::Player),
                provoked: 0.0,
                goal: Some(Goal::Attack(ShipRef::Player)),
            }
        }
    }
}

/// NPC 0, whose wait at its gate has just run out, drifting right, and
/// NPC 1, flying, far off.
fn traffic() -> Traffic {
    let mut traffic = Traffic::with_interval(1);
    let mut out = crate::traffic::fixture::npc(0, ShipStats::new(FAST, &[]));
    out.mode = Mode::LeavingGate { tick: 0 };
    out.emerges_from = Some(GATE);
    out.state.velocity = Vec2::new(1.0, 0.0);
    traffic.add_npc(out);
    let mut other = crate::traffic::fixture::npc(0, ShipStats::new(FAST, &[]));
    other.state.position = Vec2::new(5000.0, 0.0);
    traffic.add_npc(other);
    traffic
}

fn of(traffic: &Traffic, id: u32) -> &Npc {
    traffic
        .npcs()
        .iter()
        .find(|npc| npc.id == NpcId(id))
        .expect("there")
}

/// The player's hit on NPC `ship`.
fn hit(ship: u32) -> Strike {
    crate::combat::fixture::strike(ShipRef::Npc(NpcId(ship)), ShipRef::Player)
}

/// Asserts NPC 0 was held at its gate on the wait's last frame and its
/// fly half then ended the hold: flying, attacking the player, out of
/// its gate, owing its burst.
fn assert_held_and_released(traffic: &Traffic, why: &str) {
    let held = of(traffic, 0);
    assert_eq!(
        held.mode,
        Mode::Flying,
        "{why}: never LeavingGate {{ tick: 1 }}"
    );
    assert_eq!(held.goal, Goal::Attack(ShipRef::Player), "{why}");
    assert_eq!(held.emerges_from, None, "{why}");
    assert_eq!(held.owed_burst, Some(exit_speed(held.led_out, 0)), "{why}");
}

/// Asserts NPC 0 burst on: its first burst tick, its goal kept.
fn assert_bursting(traffic: &Traffic, why: &str) {
    let out = of(traffic, 0);
    assert_eq!(out.mode, Mode::LeavingGate { tick: 1 }, "{why}");
    assert_eq!(out.goal, Goal::Idle, "{why}: the glide path undoes goal 4");
    assert_eq!(out.target, Some(ShipRef::Player), "{why}");
}

#[test]
fn the_players_hit_before_the_dispatch_holds_a_ship_whose_wait_just_ran_out() {
    // A collision pass's hit (@0x43e70-0x44015) finds goal 0x15 still.
    let mut traffic = traffic();
    traffic.tick_in(&Answering, World::new(&[]), &[hit(0)], &mut NeverFires);
    assert_held_and_released(&traffic, "held");
}

#[test]
fn the_players_hit_after_the_dispatch_lets_it_burst_on() {
    // `_HandleShot`'s hit (@0x44385) finds goal 8.
    let mut traffic = traffic();
    let world = World::new(&[]);
    traffic.think_holding(&Answering, world, &[], &mut NeverFires);
    traffic.answer_in_turn(
        &Answering,
        &world,
        &mut [hit(0)],
        &mut NeverFires,
        &mut |_, _| {},
    );
    assert_eq!(of(&traffic, 0).mode, Mode::LeavingGate { tick: 0 });
    traffic.fly_holding(world, &BTreeSet::new(), &mut NeverFires);
    assert_bursting(&traffic, "after the dispatch");
}

#[test]
fn a_good_samaritans_goal_holds_it_before_the_dispatch_and_not_after() {
    // `_DoGoodSamaritan`'s goal 4 (@0x82b82-0x82b97).
    let mut traffic = traffic();
    traffic.tick_in(&Answering, World::new(&[]), &[hit(1)], &mut NeverFires);
    let held = of(&traffic, 0);
    assert_eq!(held.mode, Mode::Flying, "held, then released");
    assert_eq!(held.goal, Goal::Attack(ShipRef::Player));
    assert_eq!(held.emerges_from, None);
    let mut traffic = self::traffic();
    let world = World::new(&[]);
    traffic.think_holding(&Answering, world, &[], &mut NeverFires);
    traffic.answer_in_turn(
        &Answering,
        &world,
        &mut [hit(1)],
        &mut NeverFires,
        &mut |_, _| {},
    );
    traffic.fly_holding(world, &BTreeSet::new(), &mut NeverFires);
    assert_bursting(&traffic, "after the dispatch");
}

#[test]
fn double_times_second_pass_neither_starts_its_burst_nor_lets_a_hit_pass() {
    // `_DoPlayGameWork(0)` runs no dispatch (@0x4402a): goal 0x15 stays.
    let mut traffic = traffic();
    let world = World::new(&[]);
    let at = of(&traffic, 0).state.position;
    traffic.move_in(world, &mut NeverFires);
    let waiting = of(&traffic, 0);
    assert_eq!(waiting.mode, Mode::LeavingGate { tick: 0 }, "no burst yet");
    assert_eq!(waiting.emerges_from, Some(GATE));
    assert_eq!(waiting.state.position, at + Vec2::new(1.0, 0.0), "drifting");
    let mut hit_in_it = traffic.clone();
    hit_in_it.answer_in_turn(
        &Answering,
        &world,
        &mut [hit(0)],
        &mut NeverFires,
        &mut |_, _| {},
    );
    let held = of(&hit_in_it, 0);
    assert_eq!(held.mode, Mode::HeldAtGate { ticks_left: 1 });
    assert_eq!(held.goal, Goal::Attack(ShipRef::Player));
    traffic.tick_in(&Answering, world, &[], &mut NeverFires);
    assert_eq!(
        of(&traffic, 0).mode,
        Mode::LeavingGate { tick: 1 },
        "the next advance pass bursts it"
    );
}

#[test]
fn a_ship_whose_wait_runs_out_in_the_ship_loop_is_held_by_a_later_hit_in_it() {
    // Its wait runs out in its own `_HandleShip` (@0x3501f-0x35048), after
    // the dispatch: goal 0x15 until the next one, so a later slot's
    // shockwave holds it, and so do the beams' hits after the loop.
    let mut traffic = traffic();
    traffic.npcs[0].mode = Mode::Emerging { ticks_left: 1 };
    let world = World::new(&[]);
    traffic.think_holding(&Answering, world, &[], &mut NeverFires);
    let mut pass = traffic.begin_fly(&world, &BTreeSet::new(), super::FlyHalf::Advance);
    traffic.fly_npc(&mut pass, NpcId(0), &world, &mut NeverFires);
    assert_eq!(of(&traffic, 0).mode, Mode::LeavingGate { tick: 0 });
    let mut held = traffic.clone();
    held.answer_in_turn(
        &Answering,
        &world,
        &mut [hit(0)],
        &mut NeverFires,
        &mut |_, _| {},
    );
    assert_eq!(of(&held, 0).mode, Mode::HeldAtGate { ticks_left: 1 });
    traffic.fly_npc(&mut pass, NpcId(1), &world, &mut NeverFires);
    traffic.end_fly(pass);
    traffic.answer_in_turn(
        &Answering,
        &world,
        &mut [hit(0)],
        &mut NeverFires,
        &mut |_, _| {},
    );
    assert_eq!(of(&traffic, 0).mode, Mode::HeldAtGate { ticks_left: 1 });
}

// Goal 0x15's turn-out drops the target (@0x8dd7b).

/// Every ship keeps its goal and target, and a ship hit takes the ship
/// that hit it as its target and the damage as provocation, its goal
/// kept.
#[derive(Debug)]
struct Provoked;

impl Behaviour for Provoked {
    fn decide(&self, npc: &Npc, _: &Surroundings, _: &mut dyn Chance) -> Goal {
        npc.goal
    }

    fn target(&self, npc: &Npc, _: &Surroundings) -> Option<ShipRef> {
        npc.target
    }

    fn react(&self, npc: &Npc, strike: &Strike, _: &Surroundings) -> Reaction {
        if strike.ship == ShipRef::Npc(npc.id) {
            Reaction {
                target: strike.by,
                provoked: strike.damage,
                goal: None,
            }
        } else {
            Reaction::default()
        }
    }
}

/// NPC 1's hit on NPC `ship`.
fn npc_hit(ship: u32) -> Strike {
    crate::combat::fixture::strike(ShipRef::Npc(NpcId(ship)), ShipRef::Npc(NpcId(1)))
}

/// [`traffic`] with NPC 0 waiting `ticks_left` more ticks in its gate.
fn waiting(ticks_left: u32) -> Traffic {
    let mut traffic = traffic();
    traffic.npcs[0].mode = Mode::Emerging { ticks_left };
    traffic
}

#[test]
fn a_ship_hit_in_its_gate_keeps_its_target_until_the_dispatch_that_turns_it_out() {
    // +0x70 = -1 at the turn-out (@0x8dd7b); +0x96 is not written.
    let world = World::new(&[]);
    let mut traffic = waiting(2);
    traffic.tick_in(&Provoked, world, &[npc_hit(0)], &mut NeverFires);
    let hit = of(&traffic, 0);
    assert_eq!(hit.mode, Mode::Emerging { ticks_left: 1 });
    assert_eq!(hit.target, Some(ShipRef::Npc(NpcId(1))), "through the wait");
    assert_eq!(hit.provoked, 12.0);
    traffic.tick_in(&Provoked, world, &[], &mut NeverFires);
    let out = of(&traffic, 0);
    assert_eq!(out.mode, Mode::LeavingGate { tick: 0 });
    assert_eq!(
        out.target,
        Some(ShipRef::Npc(NpcId(1))),
        "before the dispatch"
    );
    traffic.think_holding(&Provoked, world, &[], &mut NeverFires);
    let turned = of(&traffic, 0);
    assert_eq!(turned.target, None, "the turn-out drops it");
    assert_eq!(turned.provoked, 12.0, "and keeps the provocation");
    traffic.fly_holding(world, &BTreeSet::new(), &mut NeverFires);
    assert_eq!(of(&traffic, 0).mode, Mode::LeavingGate { tick: 1 });
}

#[test]
fn a_hit_in_its_burst_gives_it_a_target_it_keeps() {
    let world = World::new(&[]);
    let mut traffic = traffic();
    traffic.tick_in(&Provoked, world, &[], &mut NeverFires);
    assert_eq!(of(&traffic, 0).mode, Mode::LeavingGate { tick: 1 });
    traffic.tick_in(&Provoked, world, &[npc_hit(0)], &mut NeverFires);
    for _ in 0..3 {
        traffic.tick_in(&Provoked, world, &[], &mut NeverFires);
        let bursting = of(&traffic, 0);
        assert!(matches!(bursting.mode, Mode::LeavingGate { tick: 2.. }));
        assert_eq!(bursting.target, Some(ShipRef::Npc(NpcId(1))));
    }
}

#[test]
fn a_ship_the_player_holds_on_its_last_wait_frame_keeps_its_target() {
    // Held before the dispatch (@0x89d3e): no goal 0x15 for it to turn out.
    let mut traffic = traffic();
    traffic.tick_in(&Answering, World::new(&[]), &[hit(0)], &mut NeverFires);
    let held = of(&traffic, 0);
    assert_eq!(held.goal, Goal::Attack(ShipRef::Player));
    assert_eq!(held.target, Some(ShipRef::Player));
}

#[test]
fn double_times_second_pass_keeps_the_target_until_the_next_dispatch() {
    let world = World::new(&[]);
    let mut traffic = traffic();
    traffic.npcs[0].target = Some(ShipRef::Npc(NpcId(1)));
    traffic.move_in(world, &mut NeverFires);
    assert_eq!(of(&traffic, 0).target, Some(ShipRef::Npc(NpcId(1))));
    traffic.think_holding(&Provoked, world, &[], &mut NeverFires);
    assert_eq!(of(&traffic, 0).target, None, "the advance pass's dispatch");
}
