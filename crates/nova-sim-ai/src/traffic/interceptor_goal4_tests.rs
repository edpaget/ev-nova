//! An interceptor's goal 4 as the traffic's tick flies it with Nova's AI
//! deciding ([`held_attack`]): a drop ends the attack with no search that
//! think, and the next think searches (`_InterceptorAI` @0x8cc6d-0x8cc7d,
//! @0x8d2fa-0x8d33a); the ally drop keeps its target, so a provoked
//! interceptor idles on the ally that hit it, searching nothing, until the
//! ally's slot goes (@0x8e277-0x8e2db, @0x8cc93-0x8ccc9).
//!
//! [`held_attack`]: crate::ai::held_attack
#![allow(clippy::float_cmp)]

use super::{Traffic, World};
use crate::ai::fixture::{PIRATES, POLICE, TRADERS, govts, n, player, ship};
use crate::ai::{Goal, NovaAi};
use crate::combat::ShipRef;
use crate::combat::fixture::strike;
use crate::combat::hull::Condition;
use crate::govt::Governments;
use crate::traffic::npc::{AiType, Npc, NpcId};
use nova_sim_base::fixture::Draws;

/// A world of `govts` in a traders' system where the player, 300 below
/// the centre, is wanted (record -10): hostile to the police.
fn world(govts: &Governments) -> World<'_> {
    World {
        govts,
        player: Some(player(0.0, -300.0)),
        system_govt: Some(TRADERS),
        record: -10,
        ..World::new(&[])
    }
}

/// `traffic` a tick on by Nova's AI among `govts`, every draw 0.
fn tick(traffic: &mut Traffic, govts: &Governments) {
    traffic.tick_in(
        &NovaAi::default(),
        world(govts),
        &[],
        &mut Draws::of(&[0; 64]),
    );
}

fn of(traffic: &Traffic, id: NpcId) -> &Npc {
    traffic
        .npcs()
        .iter()
        .find(|npc| npc.id == id)
        .expect("in the system")
}

/// A police interceptor at the centre (berth 1) holding `goal` on
/// `quarry` (berth 2), with no AI of its own, 300 above it: the traffic,
/// the interceptor and the quarry.
fn scene(quarry: Npc, goal: impl Fn(ShipRef) -> Goal) -> (Traffic, NpcId, NpcId) {
    let mut traffic = Traffic::with_interval(1);
    let mut quarry = quarry;
    quarry.no_ai = true;
    let quarry = traffic.add_npc_at(quarry, 2);
    let mut cop = ship(0, POLICE, AiType::Interceptor, 0.0, 0.0);
    cop.goal = goal(n(quarry.0));
    cop.target = cop.goal.quarry();
    let cop = traffic.add_npc_at(cop, 1);
    (traffic, cop, quarry)
}

#[test]
fn an_interceptor_whose_attack_a_drop_ends_idles_that_tick_and_searches_the_next() {
    // The disabled drop (@0x8d2fa-0x8d33a) behind the gate goal 4 closes
    // (@0x8cc6d-0x8cc7d): goal 0, +0x70 -1, no search; the next think's
    // search turns it on the wanted player.
    let govts = govts(0, 0);
    let mut pirate = ship(0, PIRATES, AiType::Warship, 0.0, 300.0);
    pirate.condition = Condition::Disabled;
    let (mut traffic, cop, _) = scene(pirate, Goal::Attack);
    tick(&mut traffic, &govts);
    let first = of(&traffic, cop);
    assert_eq!((first.goal, first.target), (Goal::Idle, None));
    tick(&mut traffic, &govts);
    let second = of(&traffic, cop);
    assert_eq!(second.goal, Goal::Attack(ShipRef::Player));
    assert_eq!(second.target, Some(ShipRef::Player));
}

#[test]
fn an_interceptor_hit_by_an_ally_idles_with_it_as_its_target_until_the_ally_leaves() {
    // The hit sets +0x70 and +0x96 (`_DamageShip` @0x3bcc0-0x3bcf6); the
    // provoked branch writes goal 4 on it each think (@0x8cbf6-0x8cc31)
    // and the ally drop idles it again, keeping +0x70 (@0x8e2d2). Once the
    // ally is gone the let-go clears +0x70, and the next think searches.
    let govts = govts(0, 0);
    let ally = ship(0, TRADERS, AiType::WimpyTrader, 0.0, 300.0);
    let (mut traffic, cop, ally) = scene(ally, |_| Goal::Idle);
    traffic.answer(
        &NovaAi::default(),
        &world(&govts),
        &[strike(n(cop.0), n(ally.0))],
    );
    assert_eq!(of(&traffic, cop).target, Some(n(ally.0)));
    assert!(of(&traffic, cop).provoked > 0.0);
    for at in 0..3 {
        tick(&mut traffic, &govts);
        let held = of(&traffic, cop);
        assert_eq!(
            (held.goal, held.target),
            (Goal::Idle, Some(n(ally.0))),
            "tick {at}"
        );
        assert!(held.provoked > 0.0, "tick {at}: the ally drop zeroes none");
    }
    if let Some(gone) = traffic.npcs_mut().iter_mut().find(|npc| npc.id == ally) {
        gone.condition = Condition::Destroyed;
    }
    traffic.remove_destroyed(ally);
    tick(&mut traffic, &govts);
    tick(&mut traffic, &govts);
    assert_eq!(of(&traffic, cop).goal, Goal::Attack(ShipRef::Player));
}
