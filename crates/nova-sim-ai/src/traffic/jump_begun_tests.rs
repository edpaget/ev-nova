//! Tests that a ship whose own jump begins at its slot runs sub 4 there
//! and flies its warp-out at its step that frame: `_HighLevelAIHandler`
//! state 2 (@0x8dd99-0x8de57) or state 3's far branch (@0x8e20f) writes
//! sub 4, the low level's sub 4 (@0x8585b-0x859c5) runs in the same
//! `_AIDispatch`, and `_HandleShip`'s warp-out (@0x346c0-0x34a54) turns it
//! toward +0x68 that frame.
#![allow(clippy::float_cmp)]

use std::cell::RefCell;
use std::collections::BTreeMap;

use super::npc::{AiType, Npc, NpcId};
use super::warp_out::WarpGoal;
use super::{Traffic, World};
use crate::ai::fixture::{PIRATES, TRADERS, ship};
use crate::ai::{Behaviour, Goal, Surroundings};
use crate::chance::{Chance, NeverFires};
use crate::combat::ShipRef;
use crate::geometry::Vec2;
use crate::speed_mult::SpeedMult;
use crate::trig::intercept_angle;

/// Every ship keeps its goal; each think records the warp stage of every
/// ship it reads.
#[derive(Debug, Default)]
struct Watch {
    stages: RefCell<BTreeMap<NpcId, BTreeMap<NpcId, f32>>>,
}

impl Behaviour for Watch {
    fn decide(&self, npc: &Npc, around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
        let stages = (around.npcs.iter())
            .map(|other| (other.id, other.warp.stage))
            .collect();
        self.stages.borrow_mut().insert(npc.id, stages);
        npc.goal
    }
}

fn of(traffic: &Traffic, id: NpcId) -> &Npc {
    (traffic.npcs().iter())
        .find(|npc| npc.id == id)
        .expect("there")
}

/// A world whose warp sound runs 600 Mac ticks, so a jump's clock takes a
/// while to run out.
fn world() -> World<'static> {
    let mut world = World::new(&[]);
    world.warp.durations = [600, 600];
    world
}

/// A ship in slot `berth` leaving (goal 2), at rest 2000 east of the
/// centre facing up, beyond the jump distance: its high level writes
/// sub 4 (@0x8de57).
fn leaving(traffic: &mut Traffic, berth: u32) -> NpcId {
    let mut runner = ship(0, TRADERS, AiType::WimpyTrader, 2000.0, 0.0);
    runner.goal = Goal::JumpOut;
    runner.state.heading = 0.0;
    traffic.add_npc_at(runner, berth)
}

#[test]
fn a_ship_whose_jump_begins_at_its_slot_runs_sub_4_there() {
    let mut traffic = Traffic::with_interval(1);
    let runner = leaving(&mut traffic, 3);
    traffic.think_holding(&Watch::default(), world(), &[], &mut NeverFires);
    let ship = of(&traffic, runner);
    assert_eq!(ship.warp.goal, Some(WarpGoal::Own));
    assert_eq!(
        ship.warp.aim,
        intercept_angle(Vec2::ZERO, ship.state.position),
        "+0x68 (@0x85892-0x8589e)"
    );
    assert_eq!(
        ship.warp.stage,
        SpeedMult::FULL.add_times(1.0, 1.0),
        "1.0, then the gain (@0x858a4-0x858f9)"
    );
    assert!(
        !traffic.slot_outcomes.contains_key(&runner),
        "its step flies its warp-out"
    );
}

#[test]
fn its_warp_out_flies_on_the_frame_it_begins() {
    let mut traffic = Traffic::with_interval(1);
    let runner = leaving(&mut traffic, 3);
    let turn = {
        let ship = of(&traffic, runner);
        ship.own_handling().turn_rate
    };
    traffic.tick_in(&Watch::default(), world(), &[], &mut NeverFires);
    let ship = of(&traffic, runner);
    assert_eq!(
        ship.state.heading, turn,
        "turned toward 90 (@0x34706-0x34713)"
    );
}

#[test]
fn a_fleeing_ship_beyond_reach_that_can_jump_begins_and_flies_its_jump_that_frame() {
    // State 3's far branch writes sub 4 the same way (@0x8e20f).
    let mut traffic = Traffic::with_interval(1);
    let attacker = traffic.add_npc_at(ship(0, PIRATES, AiType::Warship, -3000.0, 0.0), 1);
    let mut runner = ship(0, TRADERS, AiType::WimpyTrader, 2000.0, 0.0);
    runner.goal = Goal::Flee(ShipRef::Npc(attacker));
    runner.target = Some(ShipRef::Npc(attacker));
    runner.state.heading = 0.0;
    let turn = runner.own_handling().turn_rate;
    let runner = traffic.add_npc_at(runner, 3);
    traffic.tick_in(&Watch::default(), world(), &[], &mut NeverFires);
    let ship = of(&traffic, runner);
    assert_eq!(
        ship.goal,
        Goal::Flee(ShipRef::Npc(attacker)),
        "still fleeing"
    );
    assert_eq!(ship.warp.goal, Some(WarpGoal::Own));
    assert_eq!(ship.warp.stage, SpeedMult::FULL.add_times(1.0, 1.0));
    assert_eq!(ship.state.heading, turn);
}

#[test]
fn a_higher_slots_think_reads_the_sub_4_writes_that_tick() {
    let mut traffic = Traffic::with_interval(1);
    let below = traffic.add_npc_at(ship(0, PIRATES, AiType::Warship, 0.0, 0.0), 2);
    let runner = leaving(&mut traffic, 3);
    let above = traffic.add_npc_at(ship(0, PIRATES, AiType::Warship, 0.0, 0.0), 6);
    let watch = Watch::default();
    traffic.think_holding(&watch, world(), &[], &mut NeverFires);
    let read = |id| watch.stages.borrow()[&id][&runner];
    assert_eq!(read(below), 0.0, "its run came first");
    assert_eq!(read(above), 2.0, "after sub 4 at its slot");
}

#[test]
fn a_jump_whose_warp_is_done_at_once_leaves_at_its_slot() {
    // A warp sound of no length: sub 4's clock test (@0x858fe-0x8595e)
    // sends it out at the slot its jump begins (@0x859ae-0x859bf).
    let mut traffic = Traffic::with_interval(1);
    let runner = leaving(&mut traffic, 3);
    traffic.think_holding(&Watch::default(), World::new(&[]), &[], &mut NeverFires);
    assert!(traffic.npcs().iter().all(|npc| npc.id != runner), "gone");
    assert!(
        traffic
            .left_in_dispatch
            .iter()
            .any(|left| left.id == runner),
        "freed in the AI loop"
    );
}
