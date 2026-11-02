//! A coasting ship and a ship entering a gate deciding through their
//! waits, as the traffic's tick runs them ([`waiting_think`]): the type
//! think runs whatever the wait (`_AIDispatch` @0x8ff18-0x90016), the
//! high level stops at its head (@0x8d4b9-0x8d4d2), and `_HandleShip`
//! holds steering, thrust and fire (@0x34a54-0x34a72).
//!
//! [`waiting_think`]: super::waiting_think
#![allow(clippy::float_cmp)]

use super::autopilot::Outcome;
use super::board_wait::BOARD_WAIT;
use super::npc::{AiType, Mode, Npc, NpcId};
use super::{Traffic, World};
use crate::ai::fixture::{PIRATES, TRADERS, govts, n, ship};
use crate::ai::{Behaviour, Decision, Goal, NovaAi, Surroundings, fire};
use crate::catalog::{GovtRecord, LandingSite, StellarId};
use crate::chance::{Chance, NeverFires};
use crate::combat::ShipRef;
use crate::combat::hull::Condition;
use crate::geometry::Vec2;
use crate::govt::{Governments, PLUNDERS};
use nova_sim_catalog::fixture::{govt, planet};

const GATE: StellarId = StellarId(300);

/// Hypergate 300 far off the fight, 100 wide.
fn gate() -> LandingSite {
    LandingSite {
        flags2: crate::gate::HYPERGATE,
        ..planet(GATE.0, 0.0, 0.0)
    }
}

/// A world among `sites` of `govts`, the player far off.
fn world<'a>(sites: &'a [LandingSite], govts: &'a Governments) -> World<'a> {
    World {
        govts,
        player: Some(crate::ai::fixture::player(0.0, -5000.0)),
        system_govt: Some(TRADERS),
        ..World::new(sites)
    }
}

fn of(traffic: &Traffic, id: NpcId) -> &Npc {
    traffic
        .npcs()
        .iter()
        .find(|npc| npc.id == id)
        .expect("in the system")
}

/// A traffic of a pirate warship at the centre in `mode`, of `goal`,
/// moving by `velocity` on heading 10, and a trader, the pirates'
/// enemy, with no AI, 200 above it: the warship and the trader.
fn warship_and_enemy(mode: Mode, goal: Goal, velocity: Vec2) -> (Traffic, NpcId, NpcId) {
    let mut traffic = Traffic::with_interval(1);
    let mut trader = ship(0, TRADERS, AiType::WimpyTrader, 0.0, 200.0);
    trader.no_ai = true;
    let trader = traffic.add_npc_at(trader, 2);
    let mut warship = ship(0, PIRATES, AiType::Warship, 0.0, 0.0);
    warship.mode = mode;
    warship.goal = goal;
    warship.state.velocity = velocity;
    warship.state.heading = 10.0;
    let warship = traffic.add_npc_at(warship, 1);
    (traffic, warship, trader)
}

/// Asserts `id` holds its fire through its wait: its trigger, as
/// `_HandleShip` fires it, holds neither gun nor launcher.
fn assert_fire_held(traffic: &Traffic, id: NpcId) {
    let trigger = fire::gated(of(traffic, id));
    assert!(
        !trigger.primary && trigger.secondary.is_none() && trigger.only.is_none(),
        "{trigger:?}"
    );
}

#[test]
fn a_coasting_warship_takes_an_enemy_in_sight_as_its_target_through_its_wait() {
    let govts = govts(0, 0);
    let velocity = Vec2::new(4.0, -3.0);
    let (mut traffic, warship, trader) =
        warship_and_enemy(Mode::Coasting { ticks_left: 20 }, Goal::Idle, velocity);
    let start = of(&traffic, warship).state;
    traffic.tick_in(&NovaAi::default(), world(&[], &govts), &[], &mut NeverFires);
    let ship = of(&traffic, warship);
    assert_eq!(ship.goal, Goal::Attack(n(trader.0)), "_WarshipAI @0x8b875");
    assert_eq!(ship.target, Some(n(trader.0)));
    assert_eq!(ship.mode, Mode::Coasting { ticks_left: 19 });
    assert_eq!(
        ship.state.position,
        start.position + velocity,
        "course held"
    );
    assert_eq!(ship.state.velocity, velocity);
    assert_eq!(ship.state.heading, start.heading);
    assert_fire_held(&traffic, warship);
    assert_eq!(ship.pursuit.frame, None, "no high level, no fight sub");
    for _ in 0..19 {
        traffic.tick_in(&NovaAi::default(), world(&[], &govts), &[], &mut NeverFires);
    }
    let ship = of(&traffic, warship);
    assert_eq!(ship.mode, Mode::Flying, "its wait ran out");
    assert_eq!(ship.goal, Goal::Attack(n(trader.0)));
    traffic.tick_in(&NovaAi::default(), world(&[], &govts), &[], &mut NeverFires);
    let ship = of(&traffic, warship);
    assert!(ship.pursuit.frame.is_some(), "it flies the attack");
    assert_ne!(ship.state.heading, start.heading, "under its own controls");
}

#[test]
fn a_warship_entering_a_gate_takes_an_enemy_in_sight_and_still_leaves() {
    let govts = govts(0, 0);
    let sites = [gate()];
    let entering = Mode::EnteringGate {
        gate: GATE,
        ticks_left: 4,
    };
    let (mut traffic, warship, trader) =
        warship_and_enemy(entering, Goal::EnterGate(GATE), Vec2::ZERO);
    traffic.tick_in(
        &NovaAi::default(),
        world(&sites, &govts),
        &[],
        &mut NeverFires,
    );
    let ship = of(&traffic, warship);
    // Goal 0x14's target section (@0x8bb56-0x8bba2) writes the goal
    // alone: the fade reads sub 0x17 (@0x89508), so it runs on.
    assert_eq!(ship.goal, Goal::Attack(n(trader.0)));
    assert_eq!(ship.target, Some(n(trader.0)));
    assert_eq!(ship.state.position, Vec2::ZERO);
    assert_eq!(ship.state.velocity, Vec2::ZERO);
    assert_fire_held(&traffic, warship);
    assert!(ship.entering_gate(), "{:?}", ship.mode);
    let site = gate();
    assert!(!ship.approaches_gate(GATE, site.position, 25.0), "goal 4");
    let mut left = Vec::new();
    for _ in 0..3 {
        traffic.tick_in(
            &NovaAi::default(),
            world(&sites, &govts),
            &[],
            &mut NeverFires,
        );
        left.extend(traffic.departed().iter().copied());
    }
    assert_eq!(left, [(warship, Outcome::EnteredGate(GATE))]);
}

/// The traders' government and the pirates', which plunders.
fn plunderers() -> Governments {
    Governments::new([
        govt(TRADERS.0),
        GovtRecord {
            flags: PLUNDERS,
            ..govt(PIRATES.0)
        },
    ])
}

#[test]
fn a_pirate_entering_a_gate_that_finds_a_ship_to_plunder_stays() {
    // `_AIFindShipToPlunder` writes goal 0xd and sub 0 (@0x7fe98-0x7fea5):
    // the fade reads the sub alone, so the entry ends and the ship stays,
    // held still while +0x4c runs on.
    let govts = plunderers();
    let sites = [gate()];
    let mut traffic = Traffic::with_interval(1);
    let mut victim = ship(0, TRADERS, AiType::WimpyTrader, 0.0, 300.0);
    victim.condition = Condition::Disabled;
    victim.hull.crew = 1;
    victim.no_ai = true;
    let victim = traffic.add_npc_at(victim, 5);
    let mut pirate = ship(0, PIRATES, AiType::Warship, 0.0, 0.0);
    pirate.goal = Goal::EnterGate(GATE);
    pirate.mode = Mode::EnteringGate {
        gate: GATE,
        ticks_left: 4,
    };
    let pirate = traffic.add_npc_at(pirate, 1);
    traffic.tick_in(
        &NovaAi::default(),
        world(&sites, &govts),
        &[],
        &mut NeverFires,
    );
    let ship = of(&traffic, pirate);
    assert_eq!(
        ship.goal,
        Goal::Plunder {
            ship: n(victim.0),
            hopeless: false
        }
    );
    assert_eq!(ship.mode, Mode::Coasting { ticks_left: 3 });
    for _ in 0..3 {
        assert_eq!(of(&traffic, pirate).state.position, Vec2::ZERO, "held");
        traffic.tick_in(
            &NovaAi::default(),
            world(&sites, &govts),
            &[],
            &mut NeverFires,
        );
        assert_eq!(traffic.departed(), [], "it never enters");
    }
    assert_eq!(of(&traffic, pirate).mode, Mode::Flying);
    traffic.tick_in(
        &NovaAi::default(),
        world(&sites, &govts),
        &[],
        &mut NeverFires,
    );
    assert_ne!(
        of(&traffic, pirate).state.position,
        Vec2::ZERO,
        "it flies to its victim"
    );
}

/// Decides the goal it holds, its think writing `decision` over it.
#[derive(Debug)]
struct Deciding(Decision);

impl Behaviour for Deciding {
    fn decide(&self, _: &Npc, _: &Surroundings, _: &mut dyn Chance) -> Goal {
        self.0.goal
    }

    fn decision(&self, _: &Npc, _: &Surroundings, _: &mut dyn Chance) -> Decision {
        self.0
    }

    fn target(&self, npc: &Npc, _: &Surroundings) -> Option<ShipRef> {
        npc.goal.quarry()
    }
}

/// A traffic of a ship at (0, 0), still, in `mode`, of `goal`, with the
/// player 300 up.
fn waiting(mode: Mode, goal: Goal) -> (Traffic, NpcId) {
    let mut traffic = Traffic::with_interval(1);
    let mut npc = ship(0, PIRATES, AiType::Warship, 0.0, 0.0);
    npc.mode = mode;
    npc.goal = goal;
    let id = traffic.add_npc_at(npc, 1);
    (traffic, id)
}

fn player_world(sites: &[LandingSite]) -> World<'_> {
    World {
        player: Some(crate::ai::fixture::player(0.0, 300.0)),
        ..World::new(sites)
    }
}

#[test]
fn a_think_ending_a_coasting_or_entering_ships_wait_flies_it_that_tick() {
    // `_EscortAI`'s +0x4c = -1.0 (@0x83c73): the high level runs in the
    // same dispatch, and the attack block writes its own sub.
    let attack = Decision {
        goal: Goal::Attack(ShipRef::Player),
        ends_wait: true,
        ..Decision::default()
    };
    let sites = [gate()];
    for (mode, goal) in [
        (Mode::Coasting { ticks_left: 30 }, Goal::Idle),
        (
            Mode::EnteringGate {
                gate: GATE,
                ticks_left: 4,
            },
            Goal::EnterGate(GATE),
        ),
    ] {
        let (mut traffic, id) = waiting(mode, goal);
        for _ in 0..6 {
            traffic.tick_in(
                &Deciding(attack),
                player_world(&sites),
                &[],
                &mut NeverFires,
            );
            assert_eq!(traffic.departed(), [], "{mode:?}");
        }
        let ship = of(&traffic, id);
        assert_eq!(ship.mode, Mode::Flying, "{mode:?}");
        assert!(ship.pursuit.frame.is_some(), "{mode:?}: its fight frame");
        assert_ne!(ship.state.heading, 0.0, "{mode:?}: it turns to the player");
    }
}

#[test]
fn a_pirate_turned_out_of_its_entry_into_the_count_wait_waits_120_once() {
    // Goal 0x16, sub 0 and +0x4c = 120.0 over the entry's wait
    // (@0x8c6f5-0x8c71c), through the traffic's own order of the writes.
    let count = Decision {
        goal: Goal::CountWait,
        writes_sub: true,
        ..Decision::default()
    };
    let sites = [gate()];
    let (mut traffic, id) = waiting(
        Mode::EnteringGate {
            gate: GATE,
            ticks_left: 4,
        },
        Goal::EnterGate(GATE),
    );
    traffic.think_holding(&Deciding(count), player_world(&sites), &[], &mut NeverFires);
    let ship = of(&traffic, id);
    assert_eq!((ship.mode, ship.ai_wait), (Mode::Flying, BOARD_WAIT));
    assert_eq!(ship.goal, Goal::CountWait);
    traffic.fly_holding(
        player_world(&sites),
        &std::collections::BTreeSet::new(),
        &mut NeverFires,
    );
    for _ in 0..10 {
        traffic.tick_in(&Deciding(count), player_world(&sites), &[], &mut NeverFires);
        assert_eq!(traffic.departed(), []);
    }
    let ship = of(&traffic, id);
    assert_eq!(ship.ai_wait, BOARD_WAIT - 11, "one wait, counting down");
    assert_eq!(ship.mode, Mode::Flying);
}
