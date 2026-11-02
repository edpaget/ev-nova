//! An interceptor's inspection a hit redirects, as the traffic's tick
//! takes it with the interceptor's own AI deciding (see
//! [`interceptor`](crate::ai::interceptor)'s "A hit on an inspection"):
//! `_DamageShip` writes the attacker into +0x70 (@0x3bcf6), so the state-7
//! block flies to the attacker and lets it go within reach (@0x8f391),
//! scanning no player it was inspecting, and nothing turns it on the
//! attacker.
#![allow(clippy::float_cmp)]

use super::{Traffic, World};
use crate::ai::Goal;
use crate::ai::fixture::{NEUTRAL, POLICE, govts, n, player, ship};
use crate::ai::interceptor::Interceptor;
use crate::combat::ShipRef;
use crate::geometry::Vec2;
use crate::govt::Governments;
use crate::traffic::npc::{AiType, Npc, NpcId};
use nova_sim_base::fixture::Draws;

/// A police interceptor at the centre (berth 1) inspecting `inspected`,
/// after a hit by a neutral ship with no AI of its own (berth 2) at
/// (`x`, `y`) left it targeting the attacker, provoked by 3: the traffic,
/// the interceptor and the attacker.
fn scene(inspected: impl Fn(NpcId) -> ShipRef, x: f32, y: f32) -> (Traffic, NpcId, NpcId) {
    let mut traffic = Traffic::with_interval(1);
    let mut attacker = ship(0, NEUTRAL, AiType::Warship, x, y);
    attacker.no_ai = true;
    let attacker = traffic.add_npc_at(attacker, 2);
    let mut cop = ship(0, POLICE, AiType::Interceptor, 0.0, 0.0);
    cop.goal = Goal::Inspect(inspected(attacker));
    cop.target = Some(n(attacker.0));
    cop.provoked = 3.0;
    let cop = traffic.add_npc_at(cop, 1);
    (traffic, cop, attacker)
}

/// `traffic` a tick on by the interceptor's AI among `govts`, the player
/// at `at`, every draw 0.
fn tick(traffic: &mut Traffic, govts: &Governments, at: (f32, f32)) {
    let world = World {
        govts,
        player: Some(player(at.0, at.1)),
        ..World::new(&[])
    };
    traffic.tick_in(
        &Interceptor::default(),
        world,
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
fn an_inspection_of_the_player_moved_by_a_hit_scans_nothing() {
    // The reach reads +0x70, the attacker (@0x8f11e), not the player 50
    // off.
    let govts = govts(0, 0);
    let (mut traffic, cop, attacker) = scene(|_| ShipRef::Player, 0.0, 800.0);
    tick(&mut traffic, &govts, (50.0, 0.0));
    assert_eq!(traffic.take_on_line(), []);
    let after = of(&traffic, cop);
    assert_eq!(after.goal, Goal::Inspect(n(attacker.0)));
    assert_eq!(after.target, None);
}

#[test]
fn the_moved_inspection_holds_on_the_next_tick_and_lets_the_attacker_go_within_reach() {
    let govts = govts(0, 0);
    let (mut traffic, cop, attacker) = scene(|_| ShipRef::Player, 0.0, 800.0);
    let far = (0.0, -1000.0);
    tick(&mut traffic, &govts, far);
    tick(&mut traffic, &govts, far);
    assert_eq!(
        of(&traffic, cop).goal,
        Goal::Inspect(n(attacker.0)),
        "no flip back to the player"
    );
    // The attacker within reach: the state-7 block lets it go
    // (@0x8f391), writing no +0x96, and nothing turns it on the attacker.
    let at = of(&traffic, cop).state.position;
    for npc in &mut traffic.npcs {
        if npc.id == attacker {
            npc.state.position = at + Vec2::new(40.0, 40.0);
        }
    }
    for _ in 0..2 {
        tick(&mut traffic, &govts, far);
        let after = of(&traffic, cop);
        assert_ne!(after.goal, Goal::Attack(n(attacker.0)), "{:?}", after.goal);
        assert_eq!(after.provoked, 3.0, "+0x96 kept");
    }
}
