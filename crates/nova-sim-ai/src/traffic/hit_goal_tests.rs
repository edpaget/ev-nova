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

const TYPES: [AiType; 4] = [
    AiType::WimpyTrader,
    AiType::BraveTrader,
    AiType::Warship,
    AiType::Interceptor,
];

/// `traffic` a tick on under [`NovaAi`], answering `strikes`, the player
/// at the centre with 40 fuel of 300.
fn ticked(traffic: &mut Traffic, strikes: &[Strike]) {
    let mut player = crate::ai::fixture::player(0.0, 0.0);
    player.reserves.fuel = crate::reserves::Gauge {
        now: 40.0,
        max: 300.0,
    };
    let world = World {
        player: Some(player),
        ..World::new(&[])
    };
    traffic.tick_in(
        &NovaAi::default(),
        world,
        strikes,
        &mut nova_sim_base::fixture::Draws::of(&[0; 64]),
    );
}

/// NPC 1 of `ai_type` refuelling the player from (`x`, 0), its class
/// turn 3, and NPC 2, a wimpy trader at (3000, 0).
fn refuelling(ai_type: AiType, x: f32) -> Traffic {
    let mut helper = ship(1, ai_type, x, Goal::Assist(Help::Refuel));
    helper.target = Some(ShipRef::Player);
    helper.hull.class_turn = 3.0;
    let mut other = ship(2, AiType::WimpyTrader, 3000.0, Goal::Idle);
    other.berth = 2;
    let mut traffic = Traffic::with_interval(1);
    traffic.npcs = vec![helper, other];
    traffic
}

#[test]
fn an_npcs_hit_on_an_assister_keeps_its_goal_and_takes_the_attacker_by_ai_type() {
    for ai_type in TYPES {
        let mut traffic = refuelling(ai_type, 1000.0);
        ticked(&mut traffic, &[strike(n(1), n(2))]);
        let helper = &traffic.npcs()[0];
        assert_eq!(helper.goal, Goal::Assist(Help::Refuel), "{ai_type:?}");
        assert_eq!(helper.target, Some(n(2)), "{ai_type:?}: @0x3bcf6");
        assert_eq!(helper.provoked, 12.0, "{ai_type:?}");
        assert_eq!(
            helper.docking.map(|docking| docking.ship),
            Some(n(2)),
            "{ai_type:?}: it closes on its attacker"
        );
    }
    let mut miner = refuelling(AiType::WimpyTrader, 1000.0);
    miner.npcs[0].miner = crate::ai::miner::SCOOPS;
    ticked(&mut miner, &[strike(n(1), n(2))]);
    assert!(
        !matches!(miner.npcs()[0].goal, Goal::Assist(_)),
        "a leaderless miner's think drops the help: {:?}",
        miner.npcs()[0].goal
    );
}

#[test]
fn the_players_hit_clears_its_victims_travel_stellar() {
    let mut pirate = ship(1, AiType::Warship, 0.0, Goal::Attack(n(3)));
    pirate.travel_set = true;
    let mut refueller = ship(2, AiType::WimpyTrader, 100.0, Goal::Assist(Help::Refuel));
    refueller.travel_set = true;
    let mut struck_by_npc = ship(3, AiType::WimpyTrader, 3000.0, Goal::Idle);
    struck_by_npc.travel_set = true;
    let ships = answered(
        vec![pirate, refueller, struck_by_npc],
        &[
            strike(n(1), ShipRef::Player),
            strike(n(2), ShipRef::Player),
            strike(n(3), n(1)),
        ],
    );
    assert!(!ships[0].travel_set, "+0x6c -1, @0x89d47");
    assert!(!ships[1].travel_set, "+0x6c -1, @0x89d47");
    assert!(ships[2].travel_set, "an NPC's hit keeps it");
}

#[test]
fn the_players_hit_turns_an_assister_on_the_player_by_any_ai_type() {
    for ai_type in TYPES {
        let traffic = refuelling(ai_type, 10.0);
        let turned = answered(traffic.npcs, &[strike(n(1), ShipRef::Player)]);
        assert_eq!(turned[0].goal, Goal::Attack(ShipRef::Player), "{ai_type:?}");
        assert_eq!(turned[0].target, Some(ShipRef::Player), "{ai_type:?}");
        // Through a tick, its type then thinks on goal 4 (a wimpy trader
        // flees): no refuel either way.
        let mut traffic = refuelling(ai_type, 10.0);
        ticked(&mut traffic, &[strike(n(1), ShipRef::Player)]);
        assert!(
            !matches!(traffic.npcs()[0].goal, Goal::Assist(_)),
            "{ai_type:?}"
        );
        assert_eq!(traffic.take_fuel_given(), 0.0, "{ai_type:?}");
        ticked(&mut traffic, &[]);
        assert_eq!(traffic.take_fuel_given(), 0.0, "{ai_type:?}: next tick");
    }
}
