//! Tests of a trader hit for no damage as the traffic's tick runs it (see
//! [`trader_target`](crate::ai::trader_target)): `_DamageShip` sets +0x70
//! on any hit by a ship (@0x3bcf6) and adds no provocation for none
//! (@0x3bcc0-0x3bcf0), and the trader's leave, landing and stop keep the
//! target, which the warp hold and the threat tests read.

use super::{Traffic, World};
use crate::ai::fixture::{PIRATES, TRADERS, govts, player, ship, sites};
use crate::ai::nova::NovaAi;
use crate::ai::{CommCall, Goal};
use crate::catalog::StellarId;
use crate::combat::fixture::strike;
use crate::combat::{Nominal, ShipRef, Strike};
use crate::traffic::fixture::calls_on_line;
use crate::traffic::npc::{AiType, Npc, NpcId};
use nova_sim_base::fixture::Draws;

const ATTACKER: NpcId = NpcId(2);

/// NPC 2's hit on NPC 1 for no damage.
fn glancing() -> Strike {
    Strike {
        damage: 0.0,
        nominal: Nominal::default(),
        ..strike(ShipRef::Npc(NpcId(1)), ShipRef::Npc(ATTACKER))
    }
}

/// A trader of `ai_type` (1) at the centre on `goal`, and a pirate (2)
/// out of its reach.
fn scene(ai_type: AiType, goal: Goal) -> Traffic {
    let mut trader = ship(1, TRADERS, ai_type, 0.0, 0.0);
    trader.goal = goal;
    trader.visited = Some(StellarId(128));
    let mut traffic = Traffic::with_interval(1);
    traffic.npcs = vec![trader, ship(2, PIRATES, AiType::Warship, 6000.0, 0.0)];
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

fn trader(traffic: &Traffic) -> &Npc {
    &traffic.npcs[0]
}

#[test]
fn a_leaving_trader_hit_for_no_damage_keeps_its_attacker_as_target() {
    // The goal-2 block writes no +0x70 (@0x8dd9c-0x8de5e).
    for ai_type in [AiType::WimpyTrader, AiType::BraveTrader] {
        let mut traffic = scene(ai_type, Goal::JumpOut);
        tick(&mut traffic, &[glancing()]);
        tick(&mut traffic, &[]);
        assert_eq!(trader(&traffic).goal, Goal::JumpOut, "{ai_type:?}");
        assert_eq!(
            trader(&traffic).target,
            Some(ShipRef::Npc(ATTACKER)),
            "{ai_type:?}"
        );
    }
}

#[test]
fn a_trader_hit_for_no_damage_with_its_warp_under_way_flees_its_attacker_and_calls_for_help() {
    // The hold gives a ship under way goal 3 with a target (@0x8d557
    // -0x8d56b), and the next think's kept goal 3 calls for help
    // (`_WimpyTraderAI` @0x8b1f5, `_BraveTraderAI` @0x8b6cc, and the
    // brave trader's reinforcements @0x8b6dc).
    for (ai_type, calls) in [
        (AiType::WimpyTrader, vec![CommCall::Distress]),
        (
            AiType::BraveTrader,
            vec![CommCall::Distress, CommCall::Reinforcements],
        ),
    ] {
        let mut traffic = scene(ai_type, Goal::JumpOut);
        tick(&mut traffic, &[glancing()]);
        traffic.take_on_line();
        traffic.npcs[0].begin_own_jump();
        traffic.npcs[0].warp.stage = 3.0;
        tick(&mut traffic, &[]);
        assert_eq!(
            trader(&traffic).goal,
            Goal::Flee(ShipRef::Npc(ATTACKER)),
            "{ai_type:?}"
        );
        traffic.take_on_line();
        tick(&mut traffic, &[]);
        let called: Vec<CommCall> = calls_on_line(&traffic.take_on_line())
            .into_iter()
            .filter(|&(id, ..)| id == NpcId(1))
            .map(|(_, call, _)| call)
            .collect();
        assert_eq!(called, calls, "{ai_type:?}");
    }
}

#[test]
fn a_landing_trader_hit_for_no_damage_threatens_its_attacker() {
    // The goal-1 block writes no +0x70 (@0x8d8b8-0x8db5f), and the
    // threat tests spare no goal 1 (@0x7f536-0x7f56b, @0x82024-0x82055).
    for ai_type in [AiType::WimpyTrader, AiType::BraveTrader] {
        let mut traffic = scene(ai_type, Goal::Land(StellarId(128)));
        tick(&mut traffic, &[glancing()]);
        tick(&mut traffic, &[]);
        assert_eq!(
            trader(&traffic).goal,
            Goal::Land(StellarId(128)),
            "{ai_type:?}"
        );
        assert!(
            trader(&traffic).threatens_npc_among(ATTACKER, &traffic.npcs),
            "{ai_type:?}"
        );
    }
}
