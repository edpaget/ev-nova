//! Tests of the other reading of `RuleKey::DeparturesThroughGates` through
//! the traffic's tick: a ship with nowhere left to go enters a gate where
//! the engine would jump it out (`ai::idle_block::leave_or_wait`), and as
//! a lead it still releases its followers to that gate as it reaches the
//! gate's box (`_HighLevelAIHandler` @0x8dc94-0x8dd58). Under that reading,
//! the world's reading of `RuleKey::GateFuel` reaches the ship's decision:
//! short of its fare it jumps rather than enter the gate.

use super::tests::populated;
use super::{Outcome, World};
use crate::ai::{Behaviour, Goal, Surroundings, idle};
use crate::catalog::{LandingSite, StellarId};
use crate::chance::Chance;
use crate::rulebook::RuleSource;
use crate::traffic::npc::{Mode, Npc, NpcId};
use nova_sim_base::fixture::Draws;
use nova_sim_catalog::fixture::planet;

/// The idle block for a ship with no lead; a follower keeps its goal.
#[derive(Debug)]
struct IdleLead;

impl Behaviour for IdleLead {
    fn decide(&self, npc: &Npc, around: &Surroundings, chance: &mut dyn Chance) -> Goal {
        if npc.leader.is_some() {
            npc.goal
        } else {
            idle(npc, around, chance)
        }
    }
}

/// Hypergate 300 at the centre.
const GATE: StellarId = StellarId(300);

fn gate() -> LandingSite {
    LandingSite {
        flags2: crate::gate::HYPERGATE,
        ..planet(300, 0.0, 0.0)
    }
}

#[test]
fn through_gates_a_lead_out_of_the_only_gate_enters_it_and_releases_its_followers() {
    // NPC 0 came out of gate 300, the system's only stellar, and leads 1.
    let mut traffic = populated(2, 1);
    let lead = NpcId(0);
    traffic.npcs[0].visited = Some(GATE);
    traffic.npcs[1].leader = Some(lead);
    traffic.npcs[1].goal = Goal::Follow { lead, guard: None };
    let sites = [gate()];
    let world = World {
        departures_through_gates: RuleSource::Bible,
        ..World::new(&sites)
    };
    traffic.tick_in(&IdleLead, world, &[], &mut Draws::of(&[]));
    assert!(
        matches!(traffic.npcs[0].mode, Mode::EnteringGate { gate, .. } if gate == GATE),
        "{:?}",
        traffic.npcs[0].mode
    );
    assert_eq!(traffic.npcs[1].leader, None, "released");
    assert_eq!(traffic.npcs[1].goal, Goal::EnterGate(GATE));
    let mut left = Vec::new();
    for _ in 0..40 {
        traffic.tick_in(&IdleLead, world, &[], &mut Draws::of(&[]));
        left.extend(traffic.departed().iter().copied());
    }
    assert_eq!(
        left,
        [
            (NpcId(0), Outcome::EnteredGate(GATE)),
            (NpcId(1), Outcome::EnteredGate(GATE))
        ]
    );
}

#[test]
fn by_the_engine_the_same_lead_jumps_out() {
    let mut traffic = populated(1, 1);
    traffic.npcs[0].visited = Some(GATE);
    let sites = [gate()];
    traffic.tick_in(&IdleLead, World::new(&sites), &[], &mut Draws::of(&[]));
    assert_eq!(traffic.npcs[0].goal, Goal::JumpOut);
}

// The traffic's `World` hands its reading of `RuleKey::GateFuel` on to the
// `Surroundings` its NPCs decide with (`World::around`).

/// How a lone ship out of the only gate, leaving through gates, with a hull
/// whose gate fare is 150 and holding its fare less `short`, leaves over the
/// neighbouring tests' 41 ticks as `gate_fuel` reads: its goal after the
/// first tick, whether it ever entered the gate, and its departures.
fn leaving_short_of_fare_by(
    short: f32,
    gate_fuel: RuleSource,
) -> (Goal, bool, Vec<(NpcId, Outcome)>) {
    let mut traffic = populated(1, 1);
    let npc = &mut traffic.npcs[0];
    npc.visited = Some(GATE);
    npc.stats.gate_fuel = Some(150.0);
    npc.reserves.fuel.now = crate::gate_fuel::fare(&npc.stats) - short;
    let sites = [gate()];
    let world = World {
        departures_through_gates: RuleSource::Bible,
        gate_fuel,
        ..World::new(&sites)
    };
    let mut goal = None;
    let mut entered = false;
    let mut left = Vec::new();
    for _ in 0..41 {
        traffic.tick_in(&IdleLead, world, &[], &mut Draws::of(&[]));
        if goal.is_none() {
            goal = Some(traffic.npcs[0].goal);
        }
        entered |= traffic
            .npcs
            .iter()
            .any(|npc| matches!(npc.mode, Mode::EnteringGate { .. }));
        left.extend(traffic.departed().iter().copied());
    }
    (goal.expect("ticked"), entered, left)
}

#[test]
fn by_the_worlds_spending_reading_a_ship_short_of_its_fare_jumps_rather_than_enter_the_gate() {
    let (goal, entered, left) = leaving_short_of_fare_by(1.0, RuleSource::Bible);
    assert_eq!(goal, Goal::JumpOut);
    assert!(!entered, "never enters the gate");
    assert!(
        !left.contains(&(NpcId(0), Outcome::EnteredGate(GATE))),
        "{left:?}"
    );
}

#[test]
fn with_its_fare_or_by_the_worlds_engine_reading_the_same_ship_enters_the_gate() {
    for (short, gate_fuel) in [(0.0, RuleSource::Bible), (1.0, RuleSource::Engine)] {
        let (goal, entered, left) = leaving_short_of_fare_by(short, gate_fuel);
        assert_eq!(goal, Goal::EnterGate(GATE), "{short} by {gate_fuel:?}");
        assert!(entered, "{short} by {gate_fuel:?}");
        assert_eq!(
            left,
            [(NpcId(0), Outcome::EnteredGate(GATE))],
            "{short} by {gate_fuel:?}"
        );
    }
}
