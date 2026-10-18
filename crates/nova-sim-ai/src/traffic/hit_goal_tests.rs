//! Tests of the goal the player's hit gives its victim as the traffic
//! answers strikes ([`Traffic::answer`]): `_AIMakeShipAttackPlayer`'s
//! goal 4 (@0x3be66-0x3be69, @0x89d3e), set on the victim alone, whatever
//! its goal, by every AI type's reaction (see
//! [`react::victim`](crate::ai::react::victim)).

#![allow(clippy::float_cmp)]

use super::{Traffic, World};
use crate::ai::nova::NovaAi;
use crate::ai::pursuit::{Kept, Pursuit};
use crate::ai::{Goal, Help};
use crate::bay::Carrier;
use crate::combat::fixture::strike;
use crate::combat::{ShipRef, Strike};
use crate::geometry::Vec2;
use crate::stats::ShipStats;
use crate::traffic::fixture::npc;
use crate::traffic::npc::{AiType, Npc, NpcId};
use nova_sim_base::fixture::FAST;

fn n(id: u32) -> ShipRef {
    ShipRef::Npc(NpcId(id))
}

/// NPC `id` of `ai_type` at (`x`, 0), with `goal`.
fn ship(id: u32, ai_type: AiType, x: f32, goal: Goal) -> Npc {
    let mut ship = npc(id, ShipStats::new(FAST, &[]));
    ship.ai_type = ai_type;
    ship.state.position = Vec2::new(x, 0.0);
    ship.goal = goal;
    ship
}

/// `npcs` once [`NovaAi`]'s traffic has answered `strikes`, the player
/// 400 below.
fn answered(npcs: Vec<Npc>, strikes: &[Strike]) -> Vec<Npc> {
    let mut traffic = Traffic::with_interval(1);
    traffic.npcs = npcs;
    let world = World {
        player: Some(crate::ai::fixture::player(0.0, 400.0)),
        ..World::new(&[])
    };
    traffic.answer(&NovaAi::default(), &world, strikes);
    traffic.npcs
}

#[test]
fn a_flying_trader_the_player_hits_turns_on_the_player() {
    for ai_type in [
        AiType::WimpyTrader,
        AiType::BraveTrader,
        AiType::Warship,
        AiType::Interceptor,
    ] {
        let ships = answered(
            vec![ship(
                1,
                ai_type,
                0.0,
                Goal::Land(crate::catalog::StellarId(128)),
            )],
            &[strike(n(1), ShipRef::Player)],
        );
        assert_eq!(ships[0].goal, Goal::Attack(ShipRef::Player), "{ai_type:?}");
        assert_eq!(ships[0].target, Some(ShipRef::Player), "{ai_type:?}");
    }
    let ships = answered(
        vec![ship(1, AiType::WimpyTrader, 0.0, Goal::JumpOut)],
        &[strike(n(1), n(2))],
    );
    assert_eq!(ships[0].goal, Goal::JumpOut, "an NPC's hit sets no goal");
    assert_eq!(ships[0].target, Some(n(2)));
}

#[test]
fn a_ship_assisting_the_player_that_the_player_hits_turns_on_it() {
    let ships = answered(
        vec![ship(
            1,
            AiType::WimpyTrader,
            0.0,
            Goal::Assist(Help::Refuel),
        )],
        &[strike(n(1), ShipRef::Player)],
    );
    assert_eq!(ships[0].goal, Goal::Attack(ShipRef::Player));
}

#[test]
fn a_ship_already_attacking_keeps_its_pursuit_as_it_turns_on_the_player() {
    // `_AIMakeShipAttackPlayer` clears only a jump's sub-goal (4 or 0xd
    // under goals 2, 3 and 0xb, @0x89cf2-0x89d35): a rush (0x11) stays.
    let mut fighter = ship(1, AiType::Warship, 0.0, Goal::Attack(n(2)));
    fighter.pursuit = Pursuit {
        kept: Some(Kept::Rush),
        frame: None,
    };
    let ships = answered(
        vec![fighter, ship(2, AiType::WimpyTrader, 3000.0, Goal::Idle)],
        &[strike(n(1), ShipRef::Player)],
    );
    assert_eq!(ships[0].goal, Goal::Attack(ShipRef::Player));
    assert_eq!(ships[0].pursuit.kept, Some(Kept::Rush));
}

#[test]
fn an_npc_carriers_fighter_the_player_hits_turns_on_it_and_its_carrier_takes_the_target() {
    let carrier = ship(1, AiType::Warship, 0.0, Goal::Attack(n(3)));
    let mut fighter = ship(2, AiType::Interceptor, 50.0, Goal::Attack(n(3)));
    fighter.leader = Some(NpcId(1));
    fighter.carrier = Some(Carrier {
        ship: n(1),
        window: 100.0,
        reach: 40.0,
    });
    let quarry = ship(3, AiType::WimpyTrader, 3000.0, Goal::Idle);
    let ships = answered(
        vec![carrier, fighter, quarry],
        &[strike(n(2), ShipRef::Player)],
    );
    assert_eq!(ships[1].goal, Goal::Attack(ShipRef::Player), "the fighter");
    assert_eq!(ships[1].target, Some(ShipRef::Player));
    assert_eq!(ships[1].provoked, 12.0);
    assert_eq!(
        ships[0].target,
        Some(ShipRef::Player),
        "its carrier, the lead"
    );
    assert_eq!(ships[0].provoked, 12.0);
    assert_eq!(ships[0].goal, Goal::Attack(n(3)), "the lead takes no goal");
}
