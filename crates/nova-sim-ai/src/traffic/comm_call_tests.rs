//! Tests of the decisions' calls on the message line, as the traffic
//! gathers them ([`Traffic::take_comm_calls`]), and their place beside
//! the scans in slot order ([`Traffic::take_on_line`]).

use std::collections::{BTreeMap, BTreeSet};

use super::{Leave, Traffic, World};
use crate::ai::{Behaviour, CommCall, Decision, Goal, Surroundings};
use crate::chance::Chance;
use crate::combat::ShipRef;
use crate::stats::ShipStats;
use crate::traffic::fixture::npc;
use crate::traffic::npc::{Npc, NpcId};
use crate::traffic::on_line::OnLine;
use crate::traffic::table::SpawnTable;
use nova_sim_base::fixture::{Draws, FAST};

/// Each NPC calls as its entry says, idle; one with none calls nothing.
#[derive(Debug)]
struct Calls(BTreeMap<NpcId, CommCall>);

impl Behaviour for Calls {
    fn decide(&self, _npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
        Goal::Idle
    }

    fn decision(&self, npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Decision {
        Decision {
            comm_call: self.0.get(&npc.id).copied(),
            ..Decision::default()
        }
    }
}

/// Traffic of `count` NPCs at rest, each deciding on every tick.
fn traffic(count: u32) -> Traffic {
    let mut traffic = Traffic::with_interval(1);
    traffic.npcs = (0..count)
        .map(|id| npc(id, ShipStats::new(FAST, &[])))
        .collect();
    traffic
}

/// NPC 0 and 2 calling, 1 not.
fn calls() -> Calls {
    Calls(BTreeMap::from([
        (NpcId(2), CommCall::PreScan),
        (NpcId(0), CommCall::Distress),
    ]))
}

#[test]
fn the_ticks_calls_are_taken_in_npc_order_once() {
    let mut traffic = traffic(3);
    traffic.tick_in(&calls(), World::new(&[]), &[], &mut Draws::of(&[]));
    assert_eq!(
        traffic.take_comm_calls(),
        [
            (NpcId(0), CommCall::Distress),
            (NpcId(2), CommCall::PreScan)
        ]
    );
    assert_eq!(traffic.take_comm_calls(), [], "taken");
}

/// NPC 0 calls for help and for reinforcements, NPC 1 for reinforcements
/// alone.
#[derive(Debug)]
struct Reinforced;

impl Behaviour for Reinforced {
    fn decide(&self, _npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
        Goal::Idle
    }

    fn decision(&self, npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Decision {
        Decision {
            comm_call: (npc.id == NpcId(0)).then_some(CommCall::Distress),
            calls_reinforcements: npc.id <= NpcId(1),
            ..Decision::default()
        }
    }
}

#[test]
fn a_call_for_reinforcements_follows_the_thinks_own_call() {
    let mut traffic = traffic(3);
    traffic.tick_in(&Reinforced, World::new(&[]), &[], &mut Draws::of(&[]));
    assert_eq!(
        traffic.take_comm_calls(),
        [
            (NpcId(0), CommCall::Distress),
            (NpcId(0), CommCall::Reinforcements),
            (NpcId(1), CommCall::Reinforcements),
        ]
    );
}

#[test]
fn a_ship_whose_think_is_not_due_calls_nothing() {
    let mut traffic = traffic(3);
    traffic.interval = 3;
    traffic.ticks = 2;
    traffic.tick_in(&calls(), World::new(&[]), &[], &mut Draws::of(&[]));
    assert_eq!(traffic.take_comm_calls(), [(NpcId(2), CommCall::PreScan)]);
}

#[test]
fn entering_or_going_on_into_another_system_drops_the_calls() {
    let called = || {
        let mut traffic = traffic(3);
        traffic.tick_in(&calls(), World::new(&[]), &[], &mut Draws::of(&[]));
        traffic
    };
    let mut entered = called();
    entered.enter(SpawnTable::default(), &mut Draws::of(&[]));
    assert_eq!(entered.take_comm_calls(), []);
    let mut left = called();
    left.retarget(SpawnTable::default(), &BTreeSet::new(), Leave::Dropped);
    assert_eq!(left.take_comm_calls(), []);
    assert_eq!(called().take_comm_calls().len(), 2, "kept otherwise");
}

/// `traffic` one tick on, every NPC calling as `calls` says and going
/// idle, the player at the centre, where the NPCs are.
fn tick_by_the_player(traffic: &mut Traffic, calls: &Calls) {
    let world = World {
        player: Some(crate::ai::fixture::player(0.0, 0.0)),
        ..World::new(&[])
    };
    traffic.tick_in(calls, world, &[], &mut Draws::of(&[]));
}

/// Traffic of `count` NPCs at rest by the player, deciding on every tick,
/// those of `inspecting` inspecting the player.
fn inspecting(count: u32, inspecting: &[u32]) -> Traffic {
    let mut traffic = traffic(count);
    for &id in inspecting {
        traffic.npcs[id as usize].goal = Goal::Inspect(ShipRef::Player);
    }
    traffic
}

#[test]
fn a_scan_by_an_earlier_slot_holds_a_later_slots_call() {
    let mut traffic = inspecting(2, &[0]);
    tick_by_the_player(
        &mut traffic,
        &Calls(BTreeMap::from([(NpcId(1), CommCall::Distress)])),
    );
    assert!(traffic.holds_line_work());
    assert_eq!(traffic.take_comm_calls(), [], "held behind the scan");
    assert!(traffic.holds_line_work(), "still held");
    assert_eq!(
        traffic.take_on_line(),
        [
            OnLine::Scan(NpcId(0)),
            OnLine::Call(NpcId(1), CommCall::Distress)
        ]
    );
    assert!(!traffic.holds_line_work(), "taken");
    assert_eq!(traffic.take_on_line(), [], "taken");
}

#[test]
fn a_call_by_an_earlier_slot_is_taken_ahead_of_a_later_slots_scan() {
    let mut traffic = inspecting(2, &[1]);
    tick_by_the_player(
        &mut traffic,
        &Calls(BTreeMap::from([(NpcId(0), CommCall::Distress)])),
    );
    assert_eq!(traffic.take_comm_calls(), [(NpcId(0), CommCall::Distress)]);
    assert_eq!(traffic.take_on_line(), [OnLine::Scan(NpcId(1))]);
}

#[test]
fn one_ships_call_comes_before_its_own_scan() {
    let mut traffic = inspecting(1, &[0]);
    tick_by_the_player(
        &mut traffic,
        &Calls(BTreeMap::from([(NpcId(0), CommCall::Distress)])),
    );
    assert_eq!(traffic.take_comm_calls(), [(NpcId(0), CommCall::Distress)]);
    assert!(traffic.holds_line_work(), "the scan");
    assert_eq!(traffic.take_on_line(), [OnLine::Scan(NpcId(0))]);
}

#[test]
fn with_no_scan_no_line_work_is_held() {
    let mut traffic = traffic(3);
    traffic.tick_in(&calls(), World::new(&[]), &[], &mut Draws::of(&[]));
    assert!(traffic.holds_line_work(), "the calls, not yet taken");
    assert_eq!(traffic.take_comm_calls().len(), 2);
    assert!(!traffic.holds_line_work());
    assert_eq!(traffic.take_on_line(), []);
}
