//! Tests of a landed NPC's stay through the traffic's tick: it lands as
//! `_HighLevelAIHandler` state 1 ends (@0x8dadc-0x8db5a), keeps its slot
//! (+0xb8 stays set), waits at the stellar while `_HandleShip` counts its
//! wait down (@0x35025-0x35048) and skips its turn and thrust
//! (@0x34a54-0x34a72), and flies the goal its think gives once the wait
//! is out. Its slot still counts toward the system's `AvgShips`
//! (`_EnterMoreShips` @0x43b14-0x43b5c), and its berth stays taken.

use super::tests::{placed, table};
use super::{Traffic, spawn};
use crate::ai::{Behaviour, Goal, Surroundings};
use crate::catalog::StellarId;
use crate::chance::Chance;
use crate::flight::ShipState;
use crate::geometry::Vec2;
use crate::traffic::npc::Npc;
use nova_sim_base::fixture::Draws;
use nova_sim_catalog::fixture::planet;

/// Lands on stellar 140 until it has visited a stellar, then leaves by
/// hyperspace, as a trader's idle block sends a ship that has landed
/// (`ai::idle_block::visited_here`).
#[derive(Debug)]
struct LandThenLeave;

impl Behaviour for LandThenLeave {
    fn decide(&self, npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
        if npc.visited.is_some() {
            Goal::JumpOut
        } else {
            Goal::Land(StellarId(140))
        }
    }
}

/// Traffic deciding every tick, entered into a system of `AvgShips` 1 with
/// its one ship at the centre, at rest.
fn one_ship() -> Traffic {
    let mut traffic = Traffic::with_interval(1);
    traffic.enter(table(1), &mut Draws::of(&placed(750, 750)));
    let npc = &mut traffic.npcs[0];
    npc.state.velocity = Vec2::ZERO;
    assert_eq!(npc.state.position, Vec2::ZERO);
    traffic
}

/// The one NPC of `traffic`.
fn the_ship(traffic: &Traffic) -> &Npc {
    assert_eq!(traffic.npcs().len(), 1, "{:?}", traffic.departed());
    &traffic.npcs()[0]
}

#[test]
fn a_landing_npc_stays_still_through_its_wait_then_takes_its_new_goal() {
    // Planet 140 under the ship, which lands on its first tick.
    let sites = [planet(140, 0.0, 0.0)];
    let mut traffic = one_ship();
    let id = traffic.npcs[0].id;
    // A stay of 300 + `Rand(200)` = 302.
    traffic.tick(&LandThenLeave, &sites, &mut Draws::of(&[]).keyed(200, 2));
    assert_eq!(traffic.departed(), [], "it stays in the system");
    let landed: ShipState = the_ship(&traffic).state;
    assert_eq!(the_ship(&traffic).id, id);
    assert_eq!(the_ship(&traffic).visited, Some(StellarId(140)));
    assert_eq!(the_ship(&traffic).ai_wait, 301, "counted once");
    assert_eq!(landed.velocity, Vec2::ZERO);
    for left in (1..=301).rev() {
        assert_eq!(the_ship(&traffic).ai_wait, left);
        traffic.tick(&LandThenLeave, &sites, &mut Draws::of(&[]));
        let ship = the_ship(&traffic);
        assert_eq!(ship.goal, Goal::JumpOut, "its think runs through the wait");
        assert_eq!(ship.state, landed, "{left}: still");
        assert_eq!(ship.ai_wait, left - 1);
        assert_eq!(traffic.departed(), []);
    }
    // Its wait out, it begins to leave: sub 3's run out from the centre.
    traffic.tick(&LandThenLeave, &sites, &mut Draws::of(&[]));
    assert_ne!(the_ship(&traffic).state, landed, "it moves off");
}

#[test]
fn a_waiting_landed_ship_counts_toward_avg_ships_and_keeps_its_berth() {
    let sites = [planet(140, 0.0, 0.0)];
    let mut traffic = one_ship();
    let berth = traffic.npcs[0].berth;
    let free = traffic.free_berth(0);
    assert_ne!(free, Some(berth));
    traffic.tick(&LandThenLeave, &sites, &mut Draws::of(&[]));
    assert!(the_ship(&traffic).wait_running(), "landed and waiting");
    for _ in 0..3 {
        // The draws that bring an arrival into an empty system.
        let mut chance = Draws::of(&[0, 6, 6, 0, 0, 90]);
        traffic.tick(&LandThenLeave, &sites, &mut chance);
        assert!(!chance.asked.contains(&spawn::ARRIVAL_ODDS), "no roll");
        assert_eq!(the_ship(&traffic).berth, berth);
        assert_eq!(traffic.berths_taken(), 1);
        assert_eq!(traffic.free_berth(0), free, "its berth stays taken");
    }
    // The same draws bring one into the system once it is empty.
    let mut empty = one_ship();
    let id = empty.npcs[0].id;
    empty.remove(id);
    let mut chance = Draws::of(&[0, 6, 6, 0, 0, 90]);
    empty.tick(&LandThenLeave, &sites, &mut chance);
    assert_eq!(chance.asked.first(), Some(&spawn::ARRIVAL_ODDS));
    assert_eq!(empty.npcs().len(), 1, "an arrival");
}
