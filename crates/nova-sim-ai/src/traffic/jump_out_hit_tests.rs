//! Tests of what a hit on a ship jumping out, or on a ship whose NPC
//! leader is jumping out, leaves once the traffic has weighed and
//! answered it ([`Traffic::weigh_strikes`], [`Traffic::answer`]):
//! `_DamageShip`'s +0x50 tests (the entry gate @0x3b467-0x3b46e, the
//! leader's @0x3bab7-0x3bae4) skip the whole reaction block, so the
//! victim, its lead and the Good Samaritans take nothing, and no person
//! holds a grudge, under either reading of [`World::arrival_hits`]. See
//! [`answer_flag`](crate::ai::answer_flag).

#![allow(clippy::float_cmp)]

use super::{Traffic, World};
use crate::ai::Goal;
use crate::ai::fixture::{PIRATES, POLICE, TRADERS, govts, player, ship};
use crate::ai::nova::NovaAi;
use crate::combat::fixture::strike;
use crate::combat::{ShipRef, Strike};
use crate::rulebook::RuleSource;
use crate::traffic::npc::{AiType, Mode, Npc, NpcId};
use nova_sim_base::fixture::Draws;

const P: ShipRef = ShipRef::Player;

fn n(id: u32) -> ShipRef {
    ShipRef::Npc(NpcId(id))
}

/// Both readings of [`World::arrival_hits`].
const READINGS: [RuleSource; 2] = [RuleSource::Engine, RuleSource::Bible];

/// The traffic once [`NovaAi`]'s has weighed `strikes` and answered
/// them, as the session does, by `arrival_hits`, the player 400 below.
fn answered_with(arrival_hits: RuleSource, npcs: Vec<Npc>, strikes: &[Strike]) -> Traffic {
    let govts = govts(0, 0);
    let mut traffic = Traffic::with_interval(1);
    traffic.npcs = npcs;
    let world = World {
        player: Some(player(0.0, 400.0)),
        govts: &govts,
        arrival_hits,
        ..World::new(&[])
    };
    let mut strikes = strikes.to_vec();
    traffic.weigh_strikes(&world, &mut strikes, &mut Draws::of(&[]));
    traffic.answer(&NovaAi::default(), &world, &strikes);
    traffic
}

/// `npc` on its own jump out at `stage` (+0x50), or flying as it was at
/// none.
fn at_stage(mut npc: Npc, stage: f32) -> Npc {
    if stage > 0.0 {
        npc.begin_own_jump();
        npc.warp.stage = stage;
    }
    npc
}

/// A trader victim (1) at the centre, at `victim_stage`, led by a
/// warship lead (2) at `lead_stage`; an allied police warship (3), a
/// Good Samaritan for the player's hit, and a pirate (4).
fn scene(victim_stage: f32, lead_stage: f32) -> Vec<Npc> {
    let mut victim = ship(1, TRADERS, AiType::WimpyTrader, 0.0, 0.0);
    victim.leader = Some(NpcId(2));
    vec![
        at_stage(victim, victim_stage),
        at_stage(ship(2, TRADERS, AiType::Warship, 200.0, 0.0), lead_stage),
        ship(3, POLICE, AiType::Warship, 3000.0, 0.0),
        ship(4, PIRATES, AiType::Warship, 500.0, 0.0),
    ]
}

/// Asserts that answering `hit` in the scene of `stages` left every
/// ship as it was and queued no grudge, by `arrival_hits`.
fn ignored(arrival_hits: RuleSource, stages: (f32, f32), hit: Strike) {
    let before = scene(stages.0, stages.1);
    let mut traffic = answered_with(arrival_hits, before.clone(), &[hit]);
    let why = format!("{arrival_hits:?} {stages:?} {hit:?}");
    for (after, before) in traffic.npcs.iter().zip(&before) {
        assert_eq!(after.target, before.target, "{why}: {:?}", after.id);
        assert_eq!(after.goal, before.goal, "{why}: {:?}", after.id);
        assert_eq!(after.provoked, before.provoked, "{why}: {:?}", after.id);
    }
    assert_eq!(traffic.take_grudges(), [], "{why}");
}

/// Asserts that answering the player's hit in the scene of `stages`
/// turns the victim, its lead and the Good Samaritan on the player and
/// queues the victim's grudge, by `arrival_hits`: the scene would react.
fn turns_on_player(arrival_hits: RuleSource, stages: (f32, f32)) {
    let mut traffic = answered_with(arrival_hits, scene(stages.0, stages.1), &[strike(n(1), P)]);
    let why = format!("{arrival_hits:?} {stages:?}");
    assert_eq!(traffic.npcs[0].target, Some(P), "{why}: the victim");
    assert_eq!(traffic.npcs[1].target, Some(P), "{why}: its lead");
    assert_eq!(
        traffic.npcs[2].goal,
        Goal::Attack(P),
        "{why}: the Samaritan"
    );
    assert_eq!(traffic.take_grudges(), [NpcId(1)], "{why}");
}

#[test]
fn a_ship_jumping_out_ignores_the_players_hit_under_either_arrival_reading() {
    for arrival_hits in READINGS {
        ignored(arrival_hits, (3.0, 0.0), strike(n(1), P));
        ignored(arrival_hits, (3.0, 0.0), strike(n(1), n(4)));
        turns_on_player(arrival_hits, (0.0, 0.0));
    }
}

#[test]
fn a_ship_whose_leader_jumps_out_ignores_an_npcs_hit_under_either_arrival_reading() {
    for arrival_hits in READINGS {
        ignored(arrival_hits, (0.0, 3.0), strike(n(1), n(4)));
        ignored(arrival_hits, (0.0, 3.0), strike(n(1), P));
        let traffic = answered_with(arrival_hits, scene(0.0, 0.0), &[strike(n(1), n(4))]);
        assert_eq!(
            traffic.npcs[0].target,
            Some(n(4)),
            "{arrival_hits:?}: its leader not jumping, it answers"
        );
    }
}

#[test]
fn an_arriving_ship_is_not_caught_by_the_jump_out_gate() {
    // An arrival never writes +0x50 (it spawns at 0.0), so the gate lets
    // its hit through under both readings; only `ArrivalHits` decides
    // whether it then reacts.
    for (arrival_hits, reacts) in [(RuleSource::Engine, true), (RuleSource::Bible, false)] {
        let mut victim = ship(1, TRADERS, AiType::WimpyTrader, 0.0, 0.0);
        victim.mode = Mode::JumpingIn { tick: 0 };
        let govts = govts(0, 0);
        let world = World {
            player: Some(player(0.0, 400.0)),
            govts: &govts,
            arrival_hits,
            ..World::new(&[])
        };
        let mut strikes = [strike(n(1), P)];
        let mut traffic = Traffic::with_interval(1);
        traffic.npcs = vec![victim];
        traffic.weigh_strikes(&world, &mut strikes, &mut Draws::of(&[]));
        assert!(strikes[0].answered, "{arrival_hits:?}: weighed answered");
        traffic.answer(&NovaAi::default(), &world, &strikes);
        assert_eq!(
            traffic.npcs[0].target,
            reacts.then_some(P),
            "{arrival_hits:?}"
        );
    }
}
