//! Tests of the attack block's drops as the traffic's tick runs them (see
//! [`attack_drop`](crate::ai::attack_drop)): a cleared target -1 (@0x8e717,
//! @0x8e393) stands to the end of the frame for every thinker, so the
//! next think finds no target, and a held one stands the same way.
#![allow(clippy::float_cmp)]

use crate::ai::hopeless_flee::HopelessFlee;
use std::collections::BTreeSet;

use super::{Traffic, World};
use crate::ai::fixture::{PIRATES, TRADERS, n, player, ship};
use crate::ai::{Goal, NovaAi};
use crate::catalog::{StellarId, WeaponRecord};
use crate::combat::ShipRef;
use crate::combat::armament::Armament;
use crate::combat::hull::Condition;
use crate::combat::weapon::WeaponSpec;
use crate::escort::{EscortDuty, EscortOrder, FormationSlot};
use crate::traffic::ShipBehav;
use crate::traffic::npc::{AiType, MissionMark, Npc};
use nova_sim_base::fixture::Ranged;
use nova_sim_catalog::fixture::weapon;

const BREAKING_UP: Condition = Condition::Dying {
    ticks_left: 100,
    starting: false,
};

/// NPC 2, an escort of the player breaking up 500 to the right.
fn led_wreck() -> Npc {
    let mut wreck = ship(2, TRADERS, AiType::Warship, 500.0, 0.0);
    wreck.berth = 2;
    wreck.escort = Some(EscortDuty {
        formation: FormationSlot::default(),
        order: None,
    });
    wreck.condition = BREAKING_UP;
    wreck
}

/// NPC 1, defending Earth at the centre, attacking `led_wreck`.
fn defender() -> Npc {
    let mut npc = ship(1, TRADERS, AiType::Warship, 0.0, 0.0);
    npc.berth = 1;
    npc.defends = Some(StellarId(128));
    npc.goal = Goal::Attack(n(2));
    npc.target = Some(n(2));
    npc
}

/// Traffic of `npcs`, each thinking every tick.
fn traffic_of(npcs: Vec<Npc>) -> Traffic {
    let mut traffic = Traffic::with_interval(1);
    traffic.npcs = npcs;
    traffic
}

/// `traffic` one tick on by Nova's AI, the player as `side` says.
fn tick(traffic: &mut Traffic, side: &crate::ai::PlayerSide) {
    let world = World {
        player: Some(*side),
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

/// `traffic`'s NPC 1, its goal and target.
fn first(traffic: &Traffic) -> (Goal, Option<ShipRef>) {
    let npc = &traffic.npcs()[0];
    (npc.goal, npc.target)
}

#[test]
fn a_defender_whose_led_target_breaks_up_attacks_the_nearer_player_on_its_next_think() {
    // The block writes target -1 and goal 0 (@0x8e717, @0x8e726), so the
    // next `_DefenseShipAI` finds no target and attacks the candidate
    // (@0x7f944-0x7f971).
    let mut traffic = traffic_of(vec![defender(), led_wreck()]);
    let near = player(100.0, 0.0);
    tick(&mut traffic, &near);
    assert_eq!(first(&traffic), (Goal::Idle, None), "@0x8e717, @0x8e726");
    tick(&mut traffic, &near);
    assert_eq!(
        first(&traffic),
        (Goal::Attack(ShipRef::Player), Some(ShipRef::Player)),
        "@0x7f944-0x7f971"
    );
}

/// Drops to goal 0 holding NPC 9, its target step answering none.
#[derive(Debug)]
struct Deciding(crate::ai::Decision);

impl crate::ai::Behaviour for Deciding {
    fn decide(
        &self,
        _npc: &Npc,
        _around: &crate::ai::Surroundings,
        _chance: &mut dyn crate::chance::Chance,
    ) -> Goal {
        self.0.goal
    }

    fn decision(
        &self,
        _npc: &Npc,
        _around: &crate::ai::Surroundings,
        _chance: &mut dyn crate::chance::Chance,
    ) -> crate::ai::Decision {
        self.0
    }
}

/// `traffic` one tick on, each NPC deciding `decision`, its target step
/// answering none, the player far off.
fn tick_deciding(traffic: &mut Traffic, decision: crate::ai::Decision) {
    let world = World {
        player: Some(player(0.0, 90_000.0)),
        ..World::new(&[])
    };
    traffic.tick_holding(
        &Deciding(decision),
        world,
        &[],
        &BTreeSet::new(),
        &mut Ranged::of(&[]),
    );
}

#[test]
fn a_held_target_ends_the_frame_over_the_target_steps_answer() {
    let mut traffic = traffic_of(vec![ship(1, PIRATES, AiType::Warship, 0.0, 0.0)]);
    // A ship the slot test keeps (@0x8d4e4-0x8d50d): the player.
    let held = crate::ai::Decision {
        holds_target: Some(ShipRef::Player),
        ..crate::ai::Decision::default()
    };
    tick_deciding(&mut traffic, held);
    assert_eq!(first(&traffic), (Goal::Idle, Some(ShipRef::Player)));
}

#[test]
fn each_decision_writes_whether_its_flight_is_a_hopeless_chase() {
    let mut traffic = traffic_of(vec![ship(1, PIRATES, AiType::BraveTrader, 0.0, 0.0)]);
    let hopeless = crate::ai::Decision {
        goal: Goal::Flee(ShipRef::Player),
        hopeless_flee: Some(HopelessFlee::Attack),
        ..crate::ai::Decision::default()
    };
    tick_deciding(&mut traffic, hopeless);
    assert_eq!(traffic.npcs()[0].fled_hopeless, Some(HopelessFlee::Attack));
    tick_deciding(
        &mut traffic,
        crate::ai::Decision {
            hopeless_flee: None,
            ..hopeless
        },
    );
    assert_eq!(
        traffic.npcs()[0].fled_hopeless,
        None,
        "the flee block's own"
    );
}

#[test]
fn a_kept_rush_doesnt_survive_a_drop() {
    // The drop writes goal 0 (@0x8e726), and the handler's tail writes sub
    // 0 on every goal-0 frame (@0x8fb09-0x8fb13), so no pursuit carries
    // over to the next fight.
    let mut rushing = defender();
    rushing.pursuit = crate::ai::pursuit::Pursuit {
        kept: Some(crate::ai::pursuit::Kept::Rush),
        frame: None,
    };
    let mut traffic = traffic_of(vec![rushing, led_wreck()]);
    tick(&mut traffic, &player(0.0, 90_000.0));
    assert_eq!(first(&traffic), (Goal::Idle, None));
    assert_eq!(traffic.npcs()[0].pursuit.kept, None, "@0x8fb13");
}

#[test]
fn a_defender_whose_wreck_is_the_nearest_candidate_takes_it_and_drops_it_each_think() {
    // Far from the player the wreck is the candidate again, and the block
    // drops it again: the target -1 holds at each frame's end.
    let mut traffic = traffic_of(vec![defender(), led_wreck()]);
    for _ in 0..3 {
        tick(&mut traffic, &player(0.0, 90_000.0));
        assert_eq!(first(&traffic), (Goal::Idle, None));
    }
}

#[test]
fn a_mission_hunter_ends_the_frame_with_no_target_on_a_player_breaking_up() {
    // `NovaAi`'s target step answers the hunt, not the goal, as the
    // defender's answers its think: the block's -1 stands over it.
    let mut hunter = ship(1, PIRATES, AiType::Warship, 0.0, 0.0);
    hunter.mission = Some(MissionMark {
        order: None,
        slot: 1,
        behav: ShipBehav::from_code(0),
        formation: None,
    });
    // A destroying weapon, or it gives up on a player breaking up
    // (@0x8d684-0x8d6e9).
    hunter.armament = Armament::new([(
        WeaponSpec::new(&WeaponRecord {
            speed: 1000,
            count: 30,
            mass_dmg: 5,
            ..weapon(141)
        }),
        1,
    )]);
    let mut traffic = traffic_of(vec![hunter]);
    let mut side = player(100.0, 0.0);
    side.condition = BREAKING_UP;
    tick(&mut traffic, &side);
    assert_eq!(first(&traffic), (Goal::Idle, None), "@0x8e717, @0x8e726");
    side.condition = Condition::Destroyed;
    tick(&mut traffic, &side);
    assert_eq!(
        first(&traffic),
        (Goal::Idle, None),
        "destroyed, the hunt's target 0 meets @0x8e3e6 → @0x8e714"
    );
}

/// NPC 1, the player's escort at the centre on `order`.
fn players_escort(order: EscortOrder) -> Npc {
    let mut escort = ship(1, TRADERS, AiType::Warship, 0.0, 0.0);
    escort.berth = 1;
    escort.escort = Some(EscortDuty {
        formation: FormationSlot::default(),
        order: Some(order),
    });
    escort
}

/// NPC 1, a mission escort at the centre on `order` whose special ship is
/// of `ShipBehav` `behav`.
fn mission_escort(order: EscortOrder, behav: i16) -> Npc {
    let mut escort = ship(1, TRADERS, AiType::Warship, 0.0, 0.0);
    escort.berth = 1;
    escort.mission = Some(MissionMark {
        order: Some(order),
        slot: 1,
        behav: ShipBehav::from_code(behav),
        formation: Some(FormationSlot::default()),
    });
    escort
}

#[test]
fn an_escort_the_player_hits_turns_on_him_and_the_block_drops_it_that_frame() {
    // `_AIMakeShipAttackPlayer` from the hit writes goal 4, target 0
    // (@0x89d3e-0x89d4d); `_EscortAI` keeps it by its order, and the block
    // drops it that dispatch: +0x9a 0 is +0x70 0 (@0x8e306 → @0x8e381).
    let escorts = [
        players_escort(EscortOrder::Defend),
        players_escort(EscortOrder::Attack),
        mission_escort(EscortOrder::Attack, 1),
    ];
    for escort in escorts {
        let mut traffic = traffic_of(vec![escort]);
        let side = player(100.0, 0.0);
        let world = World {
            player: Some(side),
            ..World::new(&[])
        };
        let hit = crate::combat::fixture::strike(n(1), ShipRef::Player);
        traffic.answer(&NovaAi::default(), &world, &[hit]);
        assert_eq!(
            first(&traffic),
            (Goal::Attack(ShipRef::Player), Some(ShipRef::Player)),
            "the hit turns it"
        );
        for tick_no in 0..5 {
            tick(&mut traffic, &side);
            let (goal, target) = first(&traffic);
            assert_ne!(goal, Goal::Attack(ShipRef::Player), "tick {tick_no}");
            assert_ne!(target, Some(ShipRef::Player), "tick {tick_no}");
        }
    }
}

#[test]
fn a_hunted_captured_mission_escort_ends_every_tick_dropped() {
    // The hunt turns it on the player each pass (@0x8d6b7), and the block
    // drops that at once (@0x8e306 → @0x8e381).
    let mut traffic = traffic_of(vec![mission_escort(EscortOrder::Attack, 0)]);
    let side = player(100.0, 0.0);
    for tick_no in 0..40 {
        tick(&mut traffic, &side);
        assert_eq!(first(&traffic), (Goal::Idle, None), "tick {tick_no}");
    }
}

#[test]
fn a_hired_escort_on_attack_drops_a_fellow_escort_every_tick_dying_or_not() {
    // The fellow's +0x9a is its own (@0x8e322 → @0x8e81d).
    for condition in [Condition::Intact, BREAKING_UP] {
        let mut fellow = ship(2, PIRATES, AiType::Warship, 100.0, 0.0);
        fellow.berth = 2;
        fellow.escort = Some(EscortDuty {
            formation: FormationSlot::default(),
            order: None,
        });
        fellow.condition = condition;
        fellow.no_ai = true;
        // Its stored word Attack: the validity step reads it before the
        // think rewrites it (@0x83ac8), and keeps a disabled target.
        let escort = Npc {
            order_word: crate::fleet::OrderWord::of(Some(EscortOrder::Attack)),
            ..players_escort(EscortOrder::Attack)
        };
        let mut traffic = traffic_of(vec![escort, fellow]);
        for tick_no in 0..3 {
            traffic.npcs[0].goal = Goal::Attack(n(2));
            traffic.npcs[0].target = Some(n(2));
            traffic.npcs[0].provoked = 5.0;
            tick(&mut traffic, &player(0.0, 90_000.0));
            assert_eq!(
                first(&traffic),
                (Goal::Idle, None),
                "{condition:?} {tick_no}"
            );
            assert_eq!(traffic.npcs()[0].provoked, 5.0, "no +0x96 write");
        }
    }
}

#[test]
fn a_brave_trader_provoked_by_an_ally_stays_on_it_on_goal_0() {
    // The think attacks its provoker within 1250 (@0x8b616), and the ally
    // drop writes goal and sub 0 alone (@0x8e2d2): +0x70 and +0x96 stay,
    // so it thinks the same again. The world holds no government records,
    // so its provoker is of its own government, which `_GovtAllies` holds
    // allied (@0x4e4e).
    let mut trader = ship(1, TRADERS, AiType::BraveTrader, 0.0, 0.0);
    trader.berth = 1;
    trader.provoked = 10.0;
    trader.target = Some(n(2));
    let mut police = ship(2, TRADERS, AiType::Warship, 300.0, 0.0);
    police.berth = 2;
    police.no_ai = true;
    let mut traffic = traffic_of(vec![trader, police]);
    for tick_no in 0..5 {
        tick(&mut traffic, &player(0.0, 90_000.0));
        assert_eq!(first(&traffic), (Goal::Idle, Some(n(2))), "tick {tick_no}");
        assert_eq!(traffic.npcs()[0].provoked, 10.0, "tick {tick_no}");
    }
}

#[test]
fn a_traders_hopeless_flight_flies_close_beyond_250_launching_and_calling() {
    // The think attacks its provoker 300 off (@0x8b616); the block finds
    // the chase hopeless and writes goal 3, sub 5 (@0x8e625-0x8e62e), then
    // launches and calls (@0x8e6eb-0x8e70d).
    let mut trader = ship(1, TRADERS, AiType::BraveTrader, 0.0, 0.0);
    trader.berth = 1;
    trader.provoked = 10.0;
    trader.target = Some(n(2));
    trader.hull.mass = 100.0;
    let mut runner = ship(2, PIRATES, AiType::Warship, 0.0, -300.0);
    runner.berth = 2;
    runner.goal = Goal::Flee(n(1));
    runner.state.velocity = crate::geometry::Vec2::new(0.0, -1.0);
    runner.no_ai = true;
    let mut traffic = traffic_of(vec![trader, runner]);
    tick(&mut traffic, &player(0.0, 90_000.0));
    let fleeing = &traffic.npcs()[0];
    assert_eq!(fleeing.goal, Goal::Flee(n(2)));
    assert_eq!(fleeing.fled_hopeless, Some(HopelessFlee::Attack));
    let last = fleeing
        .pursuit
        .frame
        .and_then(|frame| frame.last())
        .map(|body| body.manoeuvre);
    assert_eq!(
        last,
        Some(crate::ai::fire::Manoeuvre::Flee),
        "sub 5, 300 off"
    );
    assert!(fleeing.trigger.bays, "@0x8e6fa");
    let calls: Vec<crate::ai::CommCall> =
        crate::traffic::fixture::calls_on_line(&traffic.take_on_line())
            .into_iter()
            .map(|(_, call, _)| call)
            .collect();
    assert!(
        calls.contains(&crate::ai::CommCall::Reinforcements),
        "@0x8e70d: {calls:?}"
    );
}
