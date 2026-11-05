//! Tests of the other reading of `RuleKey::DeparturesThroughGates` through
//! the traffic's tick: a ship with nowhere left to go enters a gate where
//! the engine would jump it out (`ai::idle_block::leave_or_wait`), and as
//! a lead it still releases its followers to that gate as it reaches the
//! gate's box (`_HighLevelAIHandler` @0x8dc94-0x8dd58).

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
