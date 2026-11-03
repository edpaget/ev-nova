//! Tests of the frame's gain as the AI reads it, `_LowLevelAIHandler`'s,
//! `_HighLevelAIHandler`'s, `_AIDispatch`'s and `_AIMaintainFormation`'s
//! reads of `_gSpeedMult`: [`World::speed_mult`] reaching the think's
//! [`Surroundings::speed_mult`](crate::ai::Surroundings::speed_mult), the
//! low level's [`Readings::speed_mult`](super::autopilot::Readings::speed_mult)
//! and each dispatch row. Each test asserts only against the rule's own
//! function at the same gain. Kept out of `traffic/mod.rs`, which is
//! large.

#![allow(clippy::float_cmp)]

use super::{Traffic, World, formation_flight, plunder, pod_flight, warp_out};
use crate::ai::fixture::{TRADERS, ship};
use crate::ai::{Behaviour, Goal, Surroundings};
use crate::chance::Chance;
use crate::speed_mult::SpeedMult;
use crate::traffic::npc::{AiType, Mode, Npc};
use nova_sim_base::fixture::Draws;

const HALF: SpeedMult = SpeedMult::new(0.5);

/// A bare world at `gain`.
fn at(gain: SpeedMult) -> World<'static> {
    World {
        speed_mult: gain,
        ..World::new(&[])
    }
}

#[test]
fn a_worlds_surroundings_carry_its_gain() {
    assert_eq!(at(HALF).around(&[]).speed_mult, HALF);
    assert_eq!(at(SpeedMult::FULL).around(&[]).speed_mult, SpeedMult::FULL);
}

/// Every ship keeps its goal.
#[derive(Debug, Default)]
struct Keeps;

impl Behaviour for Keeps {
    fn decide(&self, npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
        npc.goal
    }
}

/// One think run of every ship in `traffic` at `gain`.
fn think_at(traffic: &mut Traffic, gain: SpeedMult) {
    let mut world = at(gain);
    world.warp.durations = [600, 600];
    traffic.think_holding(&Keeps, world, &[], &mut Draws::of(&[0; 8]));
}

#[test]
fn at_half_gain_an_own_jumpers_think_run_grows_its_stage_by_own_jump_at_half_gain() {
    let mut runner = ship(1, TRADERS, AiType::WimpyTrader, 2000.0, 0.0);
    runner.goal = Goal::JumpOut;
    runner.begin_own_jump();
    runner.warp.stage = 3.0;
    let mut traffic = Traffic::with_interval(1);
    let id = traffic.add_npc_at(runner, 1);
    let before = traffic.npcs()[0].clone();
    think_at(&mut traffic, HALF);
    let stage = |gain| {
        let mut npc = before.clone();
        warp_out::own_jump(&mut npc, 600, gain);
        npc.warp.stage
    };
    let npc = traffic
        .npcs()
        .iter()
        .find(|npc| npc.id == id)
        .expect("still out");
    assert_eq!(npc.warp.stage, stage(HALF));
    assert_ne!(npc.warp.stage, stage(SpeedMult::FULL));
}

#[test]
fn at_half_gain_a_keeping_escorts_think_run_nudges_it_by_maintain_at_half_gain() {
    let player = crate::ai::fixture::player(0.0, 0.0);
    let mut escort = ship(1, TRADERS, AiType::Warship, 0.0, 0.0);
    escort.goal = Goal::Formation { guard: None };
    escort.escort = Some(crate::escort::EscortDuty {
        formation: crate::escort::FormationSlot {
            slot: 2,
            ships: 3,
            spacing: 30.0,
        },
        order: None,
    });
    let slot = escort.slot_at(&player.state).expect("a slot");
    escort.state.position = slot + crate::geometry::Vec2::new(20.0, 0.0);
    escort.state.velocity = player.state.velocity;
    escort.state.heading = player.state.heading;
    let mut traffic = Traffic::with_interval(1);
    let id = traffic.add_npc_at(escort, 1);
    let before = traffic.npcs()[0].clone();
    let run = |gain| {
        let mut traffic = traffic.clone();
        let world = World {
            player: Some(player),
            ..at(gain)
        };
        traffic.think_holding(&Keeps, world, &[], &mut Draws::of(&[0; 8]));
        let npc = traffic.npcs().iter().find(|npc| npc.id == id);
        npc.expect("there").state.position
    };
    let mut maintained = before.clone();
    let flight = crate::flight::FlightRules::default();
    let handling = before.flying_handling(before.top_speed(1.0), flight);
    formation_flight::maintain(&mut maintained, slot, &handling, false, HALF);
    assert_eq!(run(HALF), maintained.state.position);
    assert_ne!(run(HALF), run(SpeedMult::FULL));
}

#[test]
fn at_half_gain_a_drifting_pirates_think_run_drifts_it_by_drift_at_half_gain() {
    let mut pirate = ship(1, TRADERS, AiType::Warship, 0.0, 0.0);
    pirate.goal = Goal::Drift;
    pirate.ai_wait = 5.0;
    let mut traffic = Traffic::with_interval(1);
    let id = traffic.add_npc_at(pirate, 1);
    let before = traffic.npcs().to_vec();
    let run = |gain| {
        let mut traffic = traffic.clone();
        think_at(&mut traffic, gain);
        let npc = traffic.npcs().iter().find(|npc| npc.id == id);
        npc.expect("there").state.position
    };
    let mut drifted = before;
    plunder::drift(&mut drifted, &[true], HALF);
    assert_eq!(run(HALF), drifted[0].state.position);
    assert_ne!(run(HALF), run(SpeedMult::FULL));
}

#[test]
fn at_half_gain_a_pods_think_run_counts_its_wait_by_counted_at_half_gain() {
    let mut pod = ship(1, TRADERS, AiType::WimpyTrader, 0.0, 0.0);
    pod.ship = crate::combat::hull::ESCAPE_POD_SHIP;
    pod.mode = Mode::Adrift { ticks_left: 2.0 };
    let mut traffic = Traffic::with_interval(1);
    let id = traffic.add_npc_at(pod, 1);
    let run = |gain| {
        let mut traffic = traffic.clone();
        think_at(&mut traffic, gain);
        let npc = traffic.npcs().iter().find(|npc| npc.id == id);
        npc.expect("there").mode
    };
    let wait = |gain| pod_flight::counted(2.0, gain).expect("kept");
    assert_eq!(
        run(HALF),
        Mode::Adrift {
            ticks_left: wait(HALF)
        }
    );
    assert_ne!(run(HALF), run(SpeedMult::FULL));
}
