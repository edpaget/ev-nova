//! Tests of a warship provoked by an ally as the traffic's tick runs it
//! (see [`held_attack`](crate::ai::held_attack)): the attack block drops
//! the attack to goal 0 holding the ally (@0x8e2d2), the frame ends with
//! it held, and the next think takes it again (@0x8b843), so the warship
//! coasts on sub 0, firing nothing, until the ally goes. These pin the
//! wiring of `holds_target` to `Npc::target` to the next think, not a rule.
#![allow(clippy::float_cmp)]

use std::collections::BTreeSet;

use super::{Traffic, World};
use crate::ai::fixture::{POLICE, TRADERS, govts, n, player, ship};
use crate::ai::{Goal, NovaAi};
use crate::combat::armament::Trigger;
use crate::geometry::Vec2;
use crate::traffic::npc::{AiType, Npc};
use nova_sim_base::fixture::Ranged;

/// NPC 1, a police warship at the centre drifting right, provoked by the
/// allied trader NPC 2 it targets.
fn provoked() -> Npc {
    let mut npc = ship(1, POLICE, AiType::Warship, 0.0, 0.0);
    npc.berth = 1;
    npc.provoked = 5.0;
    npc.target = Some(n(2));
    npc.state.velocity = Vec2::new(1.0, 0.0);
    npc
}

/// NPC 2, an allied trader 500 below, thinking nothing.
fn trader() -> Npc {
    let mut npc = ship(2, TRADERS, AiType::WimpyTrader, 0.0, 500.0);
    npc.berth = 2;
    npc.no_ai = true;
    npc
}

/// `traffic` one tick on by Nova's AI, the player far off.
fn tick(traffic: &mut Traffic) {
    let govts = govts(0, 0);
    let world = World {
        player: Some(player(0.0, 90_000.0)),
        govts: &govts,
        ..World::new(&[])
    };
    traffic.tick_holding(
        &NovaAi::default(),
        world,
        &[],
        &BTreeSet::new(),
        &mut Ranged::of(&[]),
    );
}

#[test]
fn a_warship_provoked_by_an_ally_coasts_on_holding_it_until_it_goes_then_picks_again() {
    let mut traffic = Traffic::with_interval(1);
    traffic.npcs = vec![provoked(), trader()];
    for frame in 0..3 {
        let before = traffic.npcs()[0].state;
        tick(&mut traffic);
        let warship = &traffic.npcs()[0];
        assert_eq!(
            (warship.goal, warship.target),
            (Goal::Idle, Some(n(2))),
            "frame {frame}"
        );
        assert_eq!(warship.provoked, 5.0, "frame {frame}: +0x96 kept");
        assert_eq!(warship.trigger, Trigger::default(), "frame {frame}");
        assert_eq!(
            warship.state.velocity, before.velocity,
            "frame {frame}: no thrust"
        );
        assert_eq!(
            warship.state.position,
            before.position + before.velocity,
            "frame {frame}: it moves on at its velocity"
        );
    }
    traffic.npcs.truncate(1);
    tick(&mut traffic);
    let warship = &traffic.npcs()[0];
    assert_eq!(
        (warship.goal, warship.target),
        (Goal::Idle, None),
        "the ally gone: the drop (@0x8bc4e-0x8bcaf)"
    );
    tick(&mut traffic);
    assert_ne!(traffic.npcs()[0].goal, Goal::Idle, "it picks again");
}
