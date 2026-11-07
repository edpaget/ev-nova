//! Tests of an interceptor hit for no damage behind its closed gate as the
//! traffic's tick runs it (see [`held_attack`](crate::ai::held_attack)):
//! `_DamageShip` sets +0x70 on any hit by a ship (@0x3bcf6), and nothing
//! from @0x8d13f writes it in goal 2, so the leaving interceptor keeps the
//! attacker, which the threat tests and the warp hold read. These pin the
//! wiring of the keep to the tick, not a rule.

use super::{Traffic, World};
use crate::ai::Goal;
use crate::ai::fixture::{PIRATES, POLICE, govts, n, player, ship, sites};
use crate::ai::nova::NovaAi;
use crate::combat::fixture::strike;
use crate::combat::{Nominal, ShipRef, Strike};
use crate::traffic::npc::{AiType, Npc, NpcId};
use nova_sim_base::fixture::Draws;

const ATTACKER: NpcId = NpcId(2);

/// NPC 2's answered hit on NPC 1 for no damage, adding no provocation.
fn glancing() -> Strike {
    Strike {
        damage: 0.0,
        nominal: Nominal::default(),
        ..strike(ShipRef::Npc(NpcId(1)), ShipRef::Npc(ATTACKER))
    }
}

/// A police interceptor (1) at the centre leaving, and a pirate (2) out
/// of its reach; deciding every tick.
fn scene() -> Traffic {
    let mut cop = ship(1, POLICE, AiType::Interceptor, 0.0, 0.0);
    cop.berth = 1;
    cop.goal = Goal::JumpOut;
    let mut pirate = ship(2, PIRATES, AiType::Warship, 6000.0, 0.0);
    pirate.berth = 2;
    let mut traffic = Traffic::with_interval(1);
    traffic.npcs = vec![cop, pirate];
    traffic
}

/// Ticks `traffic` once, taking `strikes`, the player far below.
fn tick(traffic: &mut Traffic, strikes: &[Strike]) {
    let govts = govts(0, 0);
    let sites = sites();
    let world = World {
        player: Some(player(0.0, -8000.0)),
        govts: &govts,
        ..World::new(&sites)
    };
    traffic.tick_in(&NovaAi::default(), world, strikes, &mut Draws::of(&[0; 16]));
}

fn cop(traffic: &Traffic) -> &Npc {
    &traffic.npcs[0]
}

#[test]
fn a_leaving_interceptor_hit_for_no_damage_keeps_its_attacker_through_a_tick() {
    let mut traffic = scene();
    tick(&mut traffic, &[glancing()]);
    tick(&mut traffic, &[]);
    assert_eq!(cop(&traffic).goal, Goal::JumpOut);
    assert_eq!(cop(&traffic).target, Some(n(2)));
    assert!(cop(&traffic).threatens_npc_among(ATTACKER, &traffic.npcs));
}

#[test]
fn a_leaving_interceptor_hit_with_its_warp_under_way_flees_its_attacker() {
    // The hold gives a ship under way goal 3 with a target (@0x8d557
    // -0x8d56b).
    let mut traffic = scene();
    tick(&mut traffic, &[glancing()]);
    traffic.npcs[0].begin_own_jump();
    traffic.npcs[0].warp.stage = 3.0;
    tick(&mut traffic, &[]);
    assert_eq!(cop(&traffic).goal, Goal::Flee(n(2)));
}
