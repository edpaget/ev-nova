//! Tests of the decisions' calls on the message line, as the traffic
//! queues them, and their place beside the scans in slot order
//! ([`Traffic::take_on_line`]).

use std::collections::{BTreeMap, BTreeSet};

use super::{Leave, Traffic, World};
use crate::ai::{Behaviour, CommCall, Decision, Goal, Surroundings};
use crate::chance::Chance;
use crate::combat::ShipRef;
use crate::stats::ShipStats;
use crate::traffic::fixture::{calls_on_line, npc};
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
        calls_on_line(&traffic.take_on_line()),
        [
            (NpcId(0), CommCall::Distress, 0.0),
            (NpcId(2), CommCall::PreScan, 0.0)
        ]
    );
    assert_eq!(calls_on_line(&traffic.take_on_line()), [], "taken");
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
        calls_on_line(&traffic.take_on_line()),
        [
            (NpcId(0), CommCall::Distress, 0.0),
            (NpcId(0), CommCall::Reinforcements, 0.0),
            (NpcId(1), CommCall::Reinforcements, 0.0),
        ]
    );
}

#[test]
fn a_ship_whose_think_is_not_due_calls_nothing() {
    let mut traffic = traffic(3);
    traffic.interval = 3;
    traffic.ticks = 2;
    traffic.tick_in(&calls(), World::new(&[]), &[], &mut Draws::of(&[]));
    assert_eq!(
        calls_on_line(&traffic.take_on_line()),
        [(NpcId(2), CommCall::PreScan, 0.0)]
    );
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
    assert_eq!(entered.take_on_line(), []);
    let mut left = called();
    left.retarget(SpawnTable::default(), &BTreeSet::new(), Leave::Dropped);
    assert_eq!(left.take_on_line(), []);
    assert_eq!(called().take_on_line().len(), 2, "kept otherwise");
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
fn a_scan_by_an_earlier_slot_comes_before_a_later_slots_call() {
    let mut traffic = inspecting(2, &[0]);
    tick_by_the_player(
        &mut traffic,
        &Calls(BTreeMap::from([(NpcId(1), CommCall::Distress)])),
    );
    assert!(traffic.holds_line_work());
    assert_eq!(
        traffic.take_on_line(),
        [
            OnLine::Scan(NpcId(0)),
            OnLine::Call(NpcId(1), CommCall::Distress, 0.0)
        ]
    );
    assert!(!traffic.holds_line_work(), "taken");
    assert_eq!(traffic.take_on_line(), [], "taken");
}

#[test]
fn a_call_by_an_earlier_slot_comes_before_a_later_slots_scan() {
    let mut traffic = inspecting(2, &[1]);
    tick_by_the_player(
        &mut traffic,
        &Calls(BTreeMap::from([(NpcId(0), CommCall::Distress)])),
    );
    assert_eq!(
        traffic.take_on_line(),
        [
            OnLine::Call(NpcId(0), CommCall::Distress, 0.0),
            OnLine::Scan(NpcId(1))
        ]
    );
}

#[test]
fn one_ships_call_comes_before_its_own_scan() {
    let mut traffic = inspecting(1, &[0]);
    tick_by_the_player(
        &mut traffic,
        &Calls(BTreeMap::from([(NpcId(0), CommCall::Distress)])),
    );
    assert_eq!(
        traffic.take_on_line(),
        [
            OnLine::Call(NpcId(0), CommCall::Distress, 0.0),
            OnLine::Scan(NpcId(0))
        ]
    );
}

#[test]
fn the_calls_are_held_until_taken() {
    let mut traffic = traffic(3);
    traffic.tick_in(&calls(), World::new(&[]), &[], &mut Draws::of(&[]));
    assert!(traffic.holds_line_work(), "the calls, not yet taken");
    assert_eq!(traffic.take_on_line().len(), 2);
    assert!(!traffic.holds_line_work());
    assert_eq!(traffic.take_on_line(), []);
}

#[test]
fn a_ship_added_later_in_a_lower_berth_has_its_call_queued_first() {
    let mut traffic = Traffic::with_interval(1);
    let high = traffic.add_npc_at(npc(0, ShipStats::new(FAST, &[])), 5);
    let low = traffic.add_npc_at(npc(0, ShipStats::new(FAST, &[])), 2);
    let calls = Calls(BTreeMap::from([
        (high, CommCall::Distress),
        (low, CommCall::PreScan),
    ]));
    traffic.tick_in(&calls, World::new(&[]), &[], &mut Draws::of(&[]));
    let queued: Vec<(NpcId, CommCall)> = calls_on_line(&traffic.take_on_line())
        .into_iter()
        .map(|(id, call, _)| (id, call))
        .collect();
    assert_eq!(
        queued,
        [(low, CommCall::PreScan), (high, CommCall::Distress)],
        "berth 2's first"
    );
}

#[test]
fn a_frames_work_held_untaken_stays_ahead_of_the_next_frames() {
    let mut traffic = Traffic::with_interval(1);
    let high = traffic.add_npc_at(npc(0, ShipStats::new(FAST, &[])), 5);
    let low = traffic.add_npc_at(npc(0, ShipStats::new(FAST, &[])), 2);
    let first = Calls(BTreeMap::from([(high, CommCall::Distress)]));
    traffic.tick_in(&first, World::new(&[]), &[], &mut Draws::of(&[]));
    let second = Calls(BTreeMap::from([(low, CommCall::PreScan)]));
    traffic.tick_in(&second, World::new(&[]), &[], &mut Draws::of(&[]));
    let queued: Vec<NpcId> = calls_on_line(&traffic.take_on_line())
        .into_iter()
        .map(|(id, _, _)| id)
        .collect();
    assert_eq!(queued, [high, low], "the earlier frame's first");
}

#[test]
fn a_call_carries_the_odds_its_think_read_not_the_passs() {
    let mut traffic = Traffic::with_interval(1);
    for _ in 0..2 {
        traffic.add_npc(npc(0, ShipStats::new(FAST, &[])));
    }
    traffic.tick_in(&Reinforced, World::new(&[]), &[], &mut Draws::of(&[]));
    assert!(
        traffic.npcs()[0].odds.abs() < f32::EPSILON,
        "refreshed by the pass"
    );
    assert_eq!(
        calls_on_line(&traffic.take_on_line()),
        [
            (NpcId(0), CommCall::Distress, -1.0),
            (NpcId(0), CommCall::Reinforcements, -1.0),
            (NpcId(1), CommCall::Reinforcements, -1.0),
        ],
        "stale at their thinks"
    );
}

#[test]
fn work_queued_for_a_ship_gone_comes_after_the_frames_other_work() {
    let mut traffic = Traffic::with_interval(1);
    let here = traffic.add_npc_at(npc(0, ShipStats::new(FAST, &[])), 5);
    let gone = NpcId(99);
    traffic.queue_on_line(gone, OnLine::Scan(gone));
    traffic.queue_on_line(here, OnLine::Scan(here));
    assert_eq!(
        traffic.take_on_line(),
        [OnLine::Scan(here), OnLine::Scan(gone)],
        "at the berth of a ship here, after it for one gone"
    );
}
