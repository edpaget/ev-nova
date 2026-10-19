//! Tests of a pirate turned out of its count wait (goal 0x16,
//! [`Goal::CountWait`]) as the traffic answers strikes
//! ([`Traffic::answer`]). The original keeps the count in the AI wait
//! (+0x4c), apart from the goal, so a new goal keeps it:
//!
//! - the player's hit (`_DamageShip` @0x3be69, `_AIMakeShipAttackPlayer`:
//!   goal 4) cuts it to 20 (@0x3be6e-0x3be7c);
//! - a Good Samaritan's call (`_DoGoodSamaritan` @0x82b82-0x82b97: goal 4,
//!   target the attacker) leaves it whole.
//!
//! While it runs, `_HighLevelAIHandler` returns before goal 4's block
//! (@0x8d4b9-0x8d4d2) and `_HandleShip` holds the ship's turn, thrust and
//! fire (@0x3443f-0x34454, @0x34a54-0x34a72), goal 4 not being exempt as
//! 0x16 is. So it sits still for what is left, then attacks.

use super::{Traffic, World};
use crate::ai::fire;
use crate::ai::pirate::PirateWarship;
use crate::ai::pursuit::Pursuit;
use crate::ai::{Behaviour, Decision, Goal, Reaction, Surroundings, react};
use crate::chance::Chance;
use crate::combat::armament::Trigger;
use crate::combat::fixture::strike;
use crate::combat::{NpcId, ShipRef, Strike};
use crate::stats::ShipStats;
use crate::traffic::fixture::{count_waiting, npc};
use crate::traffic::npc::Npc;
use nova_sim_base::fixture::{Draws, FAST};

fn n(id: u32) -> ShipRef {
    ShipRef::Npc(NpcId(id))
}

/// A pirate stand-in: it keeps whatever goal it holds (as
/// `_PirateWarshipAI` returns on goal 0x16, @0x8c2ee, and keeps attacking
/// the player), always pulls its primary trigger, and targets its goal's
/// quarry. It answers a strike on itself as its victim, and the player's
/// strike on another ship as a Good Samaritan.
#[derive(Debug)]
struct Turned;

impl Behaviour for Turned {
    fn decide(&self, npc: &Npc, _: &Surroundings, _: &mut dyn Chance) -> Goal {
        npc.goal
    }

    fn decision(&self, npc: &Npc, _: &Surroundings, _: &mut dyn Chance) -> Decision {
        Decision {
            goal: npc.goal,
            ..Decision::default()
        }
    }

    fn trigger(&self, _: &Npc, _: &Surroundings) -> Trigger {
        Trigger {
            primary: true,
            ..Trigger::default()
        }
    }

    fn target(&self, npc: &Npc, _: &Surroundings) -> Option<ShipRef> {
        npc.goal.quarry()
    }

    fn react(&self, npc: &Npc, strike: &Strike, around: &Surroundings) -> Reaction {
        if strike.ship == ShipRef::Npc(npc.id) {
            return react::victim(npc, strike, around);
        }
        Reaction {
            target: Some(ShipRef::Player),
            provoked: 0.0,
            goal: Some(Goal::Attack(ShipRef::Player)),
        }
    }
}

/// The player, 400 below the centre.
fn world() -> World<'static> {
    World {
        player: Some(crate::ai::fixture::player(0.0, 400.0)),
        ..World::new(&[])
    }
}

/// NPC 1, a pirate at rest at the centre with `ticks` of its count wait
/// left, its primary trigger pulled.
fn pirate(ticks: u32) -> Npc {
    let mut pirate = npc(1, ShipStats::new(FAST, &[]));
    count_waiting(&mut pirate, ticks);
    pirate.trigger = Trigger {
        primary: true,
        ..Trigger::default()
    };
    pirate
}

/// The traffic of the pirate with `ticks` of its count wait left, deciding
/// every tick, once it has answered `strikes` by `behaviour`.
fn answered(ticks: u32, behaviour: &dyn Behaviour, strikes: &[Strike]) -> Traffic {
    let mut traffic = Traffic::with_interval(1);
    traffic.npcs = vec![pirate(ticks)];
    traffic.answer(behaviour, &world(), strikes);
    traffic
}

/// `traffic` a tick on by `behaviour`.
fn tick(traffic: &mut Traffic, behaviour: &dyn Behaviour) {
    traffic.tick_in(behaviour, world(), &[], &mut Draws::of(&[0; 8]));
}

/// Ticks `traffic`'s pirate, just turned on the player, through `held`
/// ticks of its wait, asserting it sits still, firing nothing and with no
/// fight, then once more, asserting it attacks.
fn holds_then_attacks(traffic: &mut Traffic, held: u32) {
    let pirate = &traffic.npcs()[0];
    assert_eq!(pirate.goal, Goal::Attack(ShipRef::Player));
    assert!(pirate.wait_running(), "the wait outlives goal 0x16");
    assert!(!pirate.count_wait(), "goal 4 is not exempt");
    let at = pirate.state;
    for tick_after in 1..=held {
        tick(traffic, &Turned);
        let pirate = &traffic.npcs()[0];
        assert_eq!(pirate.state, at, "still on tick {tick_after}");
        assert_eq!(
            pirate.pursuit,
            Pursuit::default(),
            "the high level returned on tick {tick_after}"
        );
        assert!(pirate.trigger.primary, "its AI pulls the trigger");
        if tick_after < held {
            assert!(
                !fire::gated(pirate).primary,
                "the gate holds its fire after tick {tick_after}"
            );
        }
    }
    assert_eq!(traffic.npcs()[0].ai_wait, 0, "the wait is out");
    tick(traffic, &Turned);
    let pirate = &traffic.npcs()[0];
    assert!(pirate.pursuit.frame.is_some(), "it flies its fight");
    assert!(fire::gated(pirate).primary, "and fires");
    assert_ne!(pirate.state, at, "it turns or thrusts toward the player");
    assert_eq!(pirate.goal, Goal::Attack(ShipRef::Player));
}

#[test]
fn a_pirate_the_player_hits_in_its_count_wait_holds_still_for_what_is_left_up_to_20_then_attacks() {
    for left in [120, 21, 20, 7] {
        let mut traffic = answered(left, &Turned, &[strike(n(1), ShipRef::Player)]);
        assert_eq!(traffic.npcs()[0].ai_wait, left.min(20), "cut from {left}");
        holds_then_attacks(&mut traffic, left.min(20));
    }
}

#[test]
fn a_pirate_a_good_samaritans_call_turns_in_its_count_wait_holds_still_for_its_whole_count() {
    let mut traffic = answered(120, &Turned, &[strike(n(2), ShipRef::Player)]);
    assert_eq!(traffic.npcs()[0].target, Some(ShipRef::Player));
    assert_eq!(traffic.npcs()[0].ai_wait, 120, "not hit: nothing cut");
    holds_then_attacks(&mut traffic, 120);
}

#[test]
fn the_pirate_warships_own_answer_keeps_its_count_wait() {
    let behaviour = PirateWarship::default();
    let mut traffic = answered(120, &behaviour, &[strike(n(1), ShipRef::Player)]);
    let pirate = &traffic.npcs()[0];
    assert_eq!(pirate.goal, Goal::Attack(ShipRef::Player));
    assert_eq!(pirate.target, Some(ShipRef::Player));
    assert_eq!(pirate.ai_wait, 20);
    let at = pirate.state;
    for tick_after in 1..=20 {
        assert!(traffic.npcs()[0].wait_running(), "tick {tick_after}");
        traffic.tick_in(&behaviour, world(), &[], &mut Draws::of(&[0; 8]));
        assert_eq!(traffic.npcs()[0].state, at, "still on tick {tick_after}");
    }
    assert!(!traffic.npcs()[0].wait_running(), "out after 20");
}
