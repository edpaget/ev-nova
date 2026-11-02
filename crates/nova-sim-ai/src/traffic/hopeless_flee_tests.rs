//! The first frame of a trader's hopeless flee as the traffic's tick
//! flies it with Nova's AI deciding ([`hopeless_flee`]): goal 4's high
//! level writes goal 3 with sub 5 (@0x8e622-0x8e62e) after goal 3's block,
//! so that frame flies flee close whatever the reach (@0x85c62-0x85e17),
//! and the state-4 tail launches its fighters (@0x8e6fa) and calls for
//! reinforcements (@0x8e70d); the next frame's goal 3 block runs out
//! beyond 250 with no call of its own (@0x8e0e1-0x8e21a).
//!
//! [`hopeless_flee`]: crate::ai::hopeless_flee
#![allow(clippy::float_cmp)]

use super::on_line::OnLine;
use super::{Traffic, World};
use crate::ai::fire::Manoeuvre;
use crate::ai::fixture::{PIRATES, TRADERS, govts, n, ship};
use crate::ai::hopeless_flee::HopelessFlee;
use crate::ai::{CommCall, Goal, NovaAi};
use crate::catalog::{WeaponId, WeaponRecord};
use crate::combat::armament::{Armament, LauncherRounds};
use crate::combat::weapon::WeaponSpec;
use crate::geometry::Vec2;
use crate::govt::Governments;
use crate::traffic::npc::{AiType, Npc, NpcId};
use nova_sim_base::fixture::Draws;
use nova_sim_catalog::fixture::weapon;

/// `traffic` a tick on by Nova's AI among `govts`, every draw 0.
fn tick(traffic: &mut Traffic, govts: &Governments) {
    let world = World {
        govts,
        ..World::new(&[])
    };
    traffic.tick_in(&NovaAi::default(), world, &[], &mut Draws::of(&[0; 64]));
}

fn of(traffic: &Traffic, id: NpcId) -> &Npc {
    traffic
        .npcs()
        .iter()
        .find(|npc| npc.id == id)
        .expect("in the system")
}

/// Whether `line` holds `id`'s call for reinforcements.
fn calls(line: &[OnLine], id: NpcId) -> bool {
    line.iter().any(
        |work| matches!(work, OnLine::Call(caller, CommCall::Reinforcements, _) if *caller == id),
    )
}

#[test]
fn a_trader_fleeing_a_hopeless_chase_flies_flee_close_its_first_tick_then_runs_out() {
    let govts = govts(0, 0);
    let mut traffic = Traffic::with_interval(1);
    let mut pirate = ship(0, PIRATES, AiType::Warship, 0.0, 600.0);
    pirate.no_ai = true;
    let pirate = traffic.add_npc_at(pirate, 2);
    let bay = WeaponRecord {
        guidance: 99,
        ammo_type: 144,
        flags: 0x0002,
        reload: 60,
        ..weapon(149)
    };
    let mut trader = ship(0, TRADERS, AiType::WimpyTrader, 0.0, 0.0);
    trader.hull.mass = 100.0;
    trader.goal = Goal::Attack(n(pirate.0));
    trader.target = Some(n(pirate.0));
    trader.armament = Armament::new([(WeaponSpec::new(&bay), 1)]);
    trader.rounds = LauncherRounds::from([(WeaponId(149), 2)]);
    let trader = traffic.add_npc_at(trader, 1);
    if let Some(quarry) = traffic.npcs_mut().iter_mut().find(|npc| npc.id == pirate) {
        quarry.goal = Goal::Flee(n(trader.0));
        quarry.state.velocity = Vec2::new(0.0, 1.0);
    }
    traffic.take_on_line();

    tick(&mut traffic, &govts);
    let line = traffic.take_on_line();
    let first = of(&traffic, trader);
    assert_eq!(first.goal, Goal::Flee(n(pirate.0)));
    assert_eq!(first.fled_hopeless, Some(HopelessFlee::Attack));
    let bodies: Vec<Manoeuvre> = first
        .pursuit
        .frame
        .iter()
        .flat_map(|frame| frame.iter().map(|body| body.manoeuvre))
        .collect();
    assert_eq!(bodies, [Manoeuvre::Flee], "flee close, 600 off");
    assert!(first.trigger.bays, "the state-4 tail's launch (@0x8e6fa)");
    assert!(calls(&line, trader), "the state-4 tail's call: {line:?}");

    tick(&mut traffic, &govts);
    let line = traffic.take_on_line();
    let second = of(&traffic, trader);
    assert_eq!(second.goal, Goal::Flee(n(pirate.0)));
    assert_eq!(second.fled_hopeless, None);
    assert!(
        second
            .pursuit
            .frame
            .is_some_and(|frame| frame.last().is_none()),
        "it runs out beyond 250, no body"
    );
    assert!(!second.trigger.bays);
    assert!(!calls(&line, trader), "goal 3's call needs 250: {line:?}");
}
