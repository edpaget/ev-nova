//! A hunter's flight as the traffic's tick flies it with Nova's AI
//! deciding: `_WarshipAI` and `_InterceptorAI` keep goal 3 on +0x70
//! whatever that ship's state, searching nothing (@0x8bb34, @0x8be1a,
//! @0x8cc6d-0x8cc7d → @0x8d13f), and goal 3's block ends a flight from a
//! ship gone from its slot that think, with no search (@0x8dff6 →
//! @0x8e228-0x8e23d, [`flee_drop`]), as it ends one from a hitter whose
//! slot went though the goal's quarry stays. The next think searches.
//!
//! [`flee_drop`]: crate::ai::flee_drop
#![allow(clippy::float_cmp)]

use super::{Traffic, World};
use crate::ai::fixture::{NEUTRAL, PIRATES, TRADERS, govts, n, player, ship};
use crate::ai::{Goal, NovaAi};
use crate::combat::fixture::strike;
use crate::combat::hull::Condition;
use crate::govt::Governments;
use crate::traffic::npc::{AiType, Npc, NpcId};
use nova_sim_base::fixture::Draws;

/// A world of `govts` in a traders' system where the player, far below
/// the centre, is of good record: no hunter's quarry.
fn world(govts: &Governments) -> World<'_> {
    World {
        govts,
        player: Some(player(0.0, -3000.0)),
        system_govt: Some(TRADERS),
        record: 0,
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

/// A traders' hunter of `ai_type` at the centre (berth 1) fleeing
/// `attacker` (berth 2), provoked by it, with a pirate in view 400 below
/// (berth 3), neither of them with AI of its own: the traffic, the hunter,
/// the attacker and the pirate.
fn scene(ai_type: AiType, attacker: Npc) -> (Traffic, NpcId, NpcId, NpcId) {
    let mut traffic = Traffic::with_interval(1);
    let mut attacker = attacker;
    attacker.no_ai = true;
    let attacker = traffic.add_npc_at(attacker, 2);
    let mut pirate = ship(0, PIRATES, AiType::Warship, 0.0, -400.0);
    pirate.no_ai = true;
    let pirate = traffic.add_npc_at(pirate, 3);
    let mut runner = ship(0, TRADERS, ai_type, 0.0, 0.0);
    runner.goal = Goal::Flee(n(attacker.0));
    runner.target = Some(n(attacker.0));
    runner.provoked = 5.0;
    let runner = traffic.add_npc_at(runner, 1);
    (traffic, runner, attacker, pirate)
}

/// `ship` taken from `traffic`'s system.
fn remove(traffic: &mut Traffic, ship: NpcId) {
    if let Some(gone) = traffic.npcs_mut().iter_mut().find(|npc| npc.id == ship) {
        gone.condition = Condition::Destroyed;
    }
    traffic.remove_destroyed(ship);
}

#[test]
fn an_interceptor_whose_attacker_leaves_idles_that_think_and_searches_the_next() {
    let govts = govts(0, 0);
    let attacker = ship(0, PIRATES, AiType::Warship, 0.0, 300.0);
    let (mut traffic, runner, attacker, pirate) = scene(AiType::Interceptor, attacker);
    tick(&mut traffic, &govts);
    assert_eq!(of(&traffic, runner).goal, Goal::Flee(n(attacker.0)));
    remove(&mut traffic, attacker);
    tick(&mut traffic, &govts);
    let idle = of(&traffic, runner);
    assert_eq!(
        (idle.goal, idle.target, idle.provoked),
        (Goal::Idle, None, 0.0),
        "goal 0, sub 0, +0x96 0, no search (@0x8e228-0x8e23d)"
    );
    tick(&mut traffic, &govts);
    assert_eq!(of(&traffic, runner).goal, Goal::Attack(n(pirate.0)));
}

#[test]
fn a_warship_flees_a_ship_breaking_up_until_its_wreck_goes() {
    let govts = govts(0, 0);
    let mut attacker = ship(0, PIRATES, AiType::Warship, 0.0, 300.0);
    attacker.condition = Condition::Dying {
        ticks_left: 1000,
        starting: false,
    };
    let (mut traffic, runner, attacker, pirate) = scene(AiType::Warship, attacker);
    for at in 0..3 {
        tick(&mut traffic, &govts);
        let fleeing = of(&traffic, runner);
        assert_eq!(
            (fleeing.goal, fleeing.target),
            (Goal::Flee(n(attacker.0)), Some(n(attacker.0))),
            "tick {at}"
        );
        assert_eq!(fleeing.provoked, 5.0, "tick {at}");
    }
    remove(&mut traffic, attacker);
    tick(&mut traffic, &govts);
    let idle = of(&traffic, runner);
    assert_eq!(
        (idle.goal, idle.target, idle.provoked),
        (Goal::Idle, None, 0.0)
    );
    tick(&mut traffic, &govts);
    assert_eq!(of(&traffic, runner).goal, Goal::Attack(n(pirate.0)));
}

#[test]
fn a_hunter_whose_hitter_leaves_idles_that_think_though_the_ship_it_fled_stays() {
    // A hit rewrites +0x70 (`_DamageShip` @0x3bcf6) and keeps goal 3; the
    // hitter's slot going makes +0x70 -1 (@0x8d4e4-0x8d50d), and goal 3's
    // block then ends the flight on +0x70 alone, whatever the goal's
    // quarry (@0x8dff6 → @0x8e228-0x8e23d). The next think searches,
    // finding the pirate: the ship it fled is no enemy of it.
    let govts = govts(0, 0);
    for ai_type in [AiType::Warship, AiType::Interceptor] {
        let fled = ship(0, NEUTRAL, AiType::Warship, 0.0, 300.0);
        let (mut traffic, runner, fled, pirate) = scene(ai_type, fled);
        let mut hitter = ship(0, PIRATES, AiType::Warship, 300.0, 0.0);
        hitter.no_ai = true;
        let hitter = traffic.add_npc_at(hitter, 4);
        tick(&mut traffic, &govts);
        assert_eq!(
            of(&traffic, runner).goal,
            Goal::Flee(n(fled.0)),
            "{ai_type:?}"
        );
        traffic.answer(
            &NovaAi::default(),
            &world(&govts),
            &[strike(n(runner.0), n(hitter.0))],
        );
        let hit = of(&traffic, runner);
        assert_eq!(
            (hit.goal, hit.target),
            (Goal::Flee(n(fled.0)), Some(n(hitter.0))),
            "{ai_type:?}"
        );
        remove(&mut traffic, hitter);
        // The hunter's own step lets go of it before its next think
        // (`_HandleShip` @0x33865-0x33888).
        traffic.let_go_at_step(runner, true);
        assert_eq!(of(&traffic, runner).target, None, "{ai_type:?}");
        tick(&mut traffic, &govts);
        let idle = of(&traffic, runner);
        assert_eq!(
            (idle.goal, idle.target, idle.provoked),
            (Goal::Idle, None, 0.0),
            "goal 0, sub 0, +0x96 0, no search: {ai_type:?}"
        );
        tick(&mut traffic, &govts);
        assert_eq!(
            of(&traffic, runner).goal,
            Goal::Attack(n(pirate.0)),
            "{ai_type:?}"
        );
    }
}
