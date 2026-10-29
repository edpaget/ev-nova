//! The idle block's visit through the traffic (`_AIMakeShipEmergeFromHyperGate`
//! @0x8955e-0x89574 and the idle block @0x8b0a3-0x8b0c8; see
//! [`idle_block`](crate::ai::idle_block)): a ship with a fuel tank that comes out of a
//! gate has visited it, so once its burst is over and it thinks, it jumps
//! out rather than heading for a stellar. One with no tank has visited
//! nothing, and heads for one.

use crate::Rulebook;
use crate::ai::{Goal, NovaAi};
use crate::catalog::{LandingSite, StellarId};
use crate::gate::HYPERGATE;
use crate::geometry::Vec2;
use crate::stats::ShipStats;
use crate::traffic::Traffic;
use crate::traffic::npc::{AiType, Mode};
use nova_sim_base::fixture::{Draws, FAST};
use nova_sim_catalog::fixture::planet;

const GATE: StellarId = StellarId(129);

/// A planet at the centre and a hypergate off to the side.
fn sites() -> [LandingSite; 2] {
    [
        planet(128, 0.0, 0.0),
        LandingSite {
            flags2: HYPERGATE,
            ..planet(GATE.0, 300.0, -400.0)
        },
    ]
}

/// The goal a wimpy trader whose tank holds `fuel_capacity` takes on its
/// first think after it comes out of the gate.
fn first_goal_out_of_the_gate(fuel_capacity: f32) -> Goal {
    let sites = sites();
    let mut npc = crate::traffic::fixture::npc(1, ShipStats::new(FAST, &[]));
    npc.ai_type = AiType::WimpyTrader;
    npc.reserves.fuel.max = fuel_capacity;
    npc.reserves.fuel.now = fuel_capacity;
    npc.state.position = Vec2::new(300.0, -400.0);
    npc.emerge_from_gate(GATE, 3);
    let mut traffic = Traffic::with_interval(1);
    traffic.add_npc(npc);
    let ai = NovaAi::from_rulebook(&Rulebook::default());
    for _ in 0..600 {
        traffic.tick(&ai, &sites, &mut Draws::of(&[0; 64]));
        let Some(npc) = traffic.npcs().first() else {
            panic!("it left before it thought: {:?}", traffic.departed());
        };
        if npc.mode == Mode::Flying && npc.goal != Goal::Idle {
            return npc.goal;
        }
    }
    panic!("it never thought");
}

#[test]
fn a_ship_with_a_tank_out_of_a_gate_has_visited_it_and_jumps_out() {
    assert_eq!(first_goal_out_of_the_gate(300.0), Goal::JumpOut);
}

#[test]
fn a_ship_with_no_tank_out_of_a_gate_heads_for_a_stellar() {
    assert!(
        matches!(
            first_goal_out_of_the_gate(0.0),
            Goal::Land(_) | Goal::EnterGate(_)
        ),
        "never visited"
    );
}
