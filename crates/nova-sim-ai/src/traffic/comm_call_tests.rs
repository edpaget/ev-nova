//! Tests of the decisions' calls on the message line, as the traffic
//! gathers them ([`Traffic::take_comm_calls`]).

use std::collections::{BTreeMap, BTreeSet};

use super::{Leave, Traffic, World};
use crate::ai::{Behaviour, CommCall, Decision, Goal, Surroundings};
use crate::chance::Chance;
use crate::stats::ShipStats;
use crate::traffic::fixture::npc;
use crate::traffic::npc::{Npc, NpcId};
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
