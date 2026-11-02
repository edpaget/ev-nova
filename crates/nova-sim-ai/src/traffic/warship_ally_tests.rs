//! A warship's goal 4 on an ally as the traffic's tick flies it with
//! Nova's AI deciding ([`held_attack`]): the provoked branch writes goal 4
//! on the ally that hit it at each think (`_WarshipAI` @0x8bb0c-0x8bb44),
//! and goal 4's ally drop idles it again, keeping +0x70 (@0x8e277-0x8e2db),
//! so the goal-0 block searches nothing (@0x8b843-0x8b860) until the
//! ally's slot goes.
//!
//! [`held_attack`]: crate::ai::held_attack
#![allow(clippy::float_cmp)]

use super::{Traffic, World};
use crate::ai::fixture::{PIRATES, POLICE, TRADERS, govts, n, player, ship};
use crate::ai::{Goal, NovaAi};
use crate::combat::fixture::strike;
use crate::combat::hull::Condition;
use crate::govt::Governments;
use crate::traffic::npc::{AiType, Npc, NpcId};
use nova_sim_base::fixture::Draws;

/// A world of `govts` in a traders' system where the player, far below
/// the centre, is of good record.
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

#[test]
fn a_warship_hit_by_an_ally_idles_on_it_until_the_ally_leaves() {
    // The hit sets +0x70 and +0x96 (`_DamageShip` @0x3bcc0-0x3bcf6). A
    // pirate attacking the warship, the threat its search would take, is
    // in view throughout.
    let govts = govts(0, 0);
    let mut traffic = Traffic::with_interval(1);
    let mut ally = ship(0, TRADERS, AiType::WimpyTrader, 0.0, 300.0);
    ally.no_ai = true;
    let ally = traffic.add_npc_at(ally, 2);
    let cop = traffic.add_npc_at(ship(0, POLICE, AiType::Warship, 0.0, 0.0), 1);
    let mut pirate = ship(0, PIRATES, AiType::Warship, 0.0, -400.0);
    pirate.no_ai = true;
    pirate.goal = Goal::Attack(n(cop.0));
    pirate.target = Some(n(cop.0));
    let pirate = traffic.add_npc_at(pirate, 3);
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
    assert_eq!(of(&traffic, cop).goal, Goal::Attack(n(pirate.0)));
}
