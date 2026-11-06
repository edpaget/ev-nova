//! Tests of a miner hit for no damage as the traffic's tick runs it (see
//! [`miner`](crate::ai::miner)): `_DamageShip` sets +0x70 on any hit by a
//! ship (@0x3bcf6), and `_MinerAI` writes it -1 only on its stop, its
//! leaves and disabled, so a scooping miner keeps its attacker, which the
//! threat tests read; a mining one keeps it until order 0x14 fires at
//! asteroid slot 0, which writes it -1 for any type (@0x8843a). These pin
//! the wiring of the miner's `clears_target`, its target step and the
//! fire's -1 to the tick, not a rule.

use super::{Traffic, World};
use crate::ai::Goal;
use crate::ai::fixture::{PIRATES, TRADERS, govts, n, player, ship, sites};
use crate::ai::miner::{MINES, SCOOPS};
use crate::ai::nova::NovaAi;
use crate::cargo_box::CargoBox;
use crate::combat::fixture::strike;
use crate::combat::{Nominal, ShipRef, Strike};
use crate::geometry::Vec2;
use crate::traffic::npc::{AiType, Npc, NpcId};
use nova_sim_base::fixture::Draws;
use nova_sim_combat::asteroid::Asteroid;

const ATTACKER: NpcId = NpcId(2);

/// NPC 2's answered hit on NPC 1 for no damage, adding no provocation.
fn glancing() -> Strike {
    Strike {
        damage: 0.0,
        nominal: Nominal::default(),
        ..strike(ShipRef::Npc(NpcId(1)), ShipRef::Npc(ATTACKER))
    }
}

/// A leaderless scooping miner (1) at the centre with a 20-ton hold
/// holding `tons`, on `goal`, and a pirate (2) out of its reach; deciding
/// every tick.
fn scene(goal: Goal, tons: u32) -> Traffic {
    let mut miner = ship(1, TRADERS, AiType::WimpyTrader, 0.0, 0.0);
    miner.berth = 1;
    miner.miner = SCOOPS;
    miner.can_scoop = true;
    miner.holds = 20;
    miner.cargo.add(0, tons);
    miner.goal = goal;
    let mut pirate = ship(2, PIRATES, AiType::Warship, 6000.0, 0.0);
    pirate.berth = 2;
    let mut traffic = Traffic::with_interval(1);
    traffic.npcs = vec![miner, pirate];
    traffic
}

/// A scoopable box 500 to the right.
fn scoopable() -> CargoBox {
    CargoBox {
        position: Vec2::new(500.0, 0.0),
        velocity: Vec2::ZERO,
        life: 100.0,
        frame: 0.0,
        spin: 0,
        graphic: 1,
        scoopable: true,
        contents: Some(2),
    }
}

/// Slot 0, at rest at the origin.
fn rock() -> Asteroid {
    Asteroid {
        position: Vec2::ZERO,
        velocity: Vec2::ZERO,
        frame: 0.0,
        spin: 0.0,
        strength: 30,
        kind: 0,
    }
}

/// A leaderless `MINES` miner (1) in [`Goal::Mine`] at (`x`, `y`) heading
/// `heading`, its class turn 2 (a mining box of 120), and a pirate (2)
/// out of its reach; deciding every tick.
fn mining_at(x: f32, y: f32, heading: f32) -> Traffic {
    let mut traffic = scene(Goal::Mine, 0);
    let miner = &mut traffic.npcs[0];
    miner.miner = MINES;
    miner.can_scoop = false;
    miner.hull.class_turn = 2.0;
    miner.state.position = Vec2::new(x, y);
    miner.state.heading = heading;
    traffic
}

/// Ticks `traffic` once, taking `strikes`, the player far below, a
/// scoopable box in space and slot 0 at the origin.
fn tick(traffic: &mut Traffic, strikes: &[Strike]) {
    let govts = govts(0, 0);
    let sites = sites();
    let boxes = [scoopable()];
    let world = World {
        player: Some(player(0.0, -8000.0)),
        govts: &govts,
        boxes: &boxes,
        mining_rock: Some(rock()),
        ..World::new(&sites)
    };
    traffic.tick_in(&NovaAi::default(), world, strikes, &mut Draws::of(&[0; 16]));
}

fn miner(traffic: &Traffic) -> &Npc {
    &traffic.npcs[0]
}

#[test]
fn a_scooping_miner_hit_for_no_damage_keeps_its_attacker_and_threatens_it() {
    // The scoop writes no +0x70 (@0x8b33b), nor does the high level's
    // order 0x15 (@0x8faf3-0x8fafd, @0x88459-0x884f0).
    let mut traffic = scene(Goal::Scoop, 0);
    tick(&mut traffic, &[glancing()]);
    tick(&mut traffic, &[]);
    assert_eq!(miner(&traffic).goal, Goal::Scoop);
    assert_eq!(miner(&traffic).target, Some(n(2)));
    assert!(miner(&traffic).threatens_npc_among(ATTACKER, &traffic.npcs));
}

#[test]
fn a_full_hold_miners_leave_clears_a_hits_target() {
    // The full-hold leave writes +0x70 -1 (@0x8b31e).
    let mut traffic = scene(Goal::Scoop, 20);
    tick(&mut traffic, &[glancing()]);
    assert_eq!(miner(&traffic).goal, Goal::JumpOut);
    assert_eq!(miner(&traffic).target, None);
}

#[test]
fn a_mining_miner_keeps_a_hits_target_outside_its_box_and_loses_it_as_it_fires() {
    // Order 0x13, the chase outside the box, writes no +0x70; order 0x14
    // writes -1 on each frame it fires at slot 0 (@0x8843a).
    let mut chasing = mining_at(400.0, 0.0, 270.0);
    tick(&mut chasing, &[glancing()]);
    tick(&mut chasing, &[]);
    assert_eq!(miner(&chasing).goal, Goal::Mine);
    assert_eq!(miner(&chasing).target, Some(n(2)), "outside the box");
    let mut firing = mining_at(100.0, 100.0, 314.0);
    tick(&mut firing, &[glancing()]);
    assert_eq!(miner(&firing).goal, Goal::Mine);
    assert_eq!(miner(&firing).target, None, "fired at slot 0");
}
