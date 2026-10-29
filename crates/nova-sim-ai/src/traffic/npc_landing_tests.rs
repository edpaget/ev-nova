//! An NPC's landing through the traffic, as `_HighLevelAIHandler` state 1
//! lands it and its wait ends (@0x8d8b8-0x8db5a, the countdown
//! `_HandleShip` @0x35025-0x35048; see [`land_flight`](crate::traffic::land_flight)):
//! a wimpy trader flies to the system's planet and lands inside its box,
//! stays in the system, still, for its wait of 300 to 499 ticks, then
//! takes a new goal from the idle block, which finds it has visited a
//! stellar here ([`idle_block`](crate::ai::idle_block)): it leaves by hyperspace
//! with a fuel tank, and stops and waits (state 6) with none.

use crate::Rulebook;
use crate::ai::{Goal, NovaAi};
use crate::catalog::{LandingSite, StellarId};
use crate::geometry::Vec2;
use crate::rulebook::RuleSource;
use crate::stats::ShipStats;
use crate::traffic::Traffic;
use crate::traffic::npc::{AiType, Npc};
use nova_sim_base::fixture::{Draws, FAST};
use nova_sim_catalog::fixture::planet;

/// The system's one planet, 600 above the centre.
fn site() -> LandingSite {
    planet(128, 0.0, -600.0)
}

/// What became of a wimpy trader whose class's tank holds `fuel` (its
/// `shïp` `Fuel`), idling at the centre at rest, its landing's wait 300 +
/// `stay`.
struct Landing {
    /// The tick it landed on, counting from 1.
    landed_on: u32,
    /// The ship as it landed.
    landed: Npc,
    /// The ticks it stayed where it landed, the landing's included.
    still_for: u32,
    /// The goal it flies once it moves off, or holds once its wait is out.
    then: Goal,
}

fn land(fuel: i16, stay: u32) -> Landing {
    let sites = [site()];
    let mut npc = crate::traffic::fixture::npc(1, ShipStats::new(FAST, &[]));
    npc.ai_type = AiType::WimpyTrader;
    npc.class_fuel = fuel;
    npc.reserves.fuel.max = f32::from(fuel);
    npc.reserves.fuel.now = f32::from(fuel);
    let mut traffic = Traffic::with_interval(1);
    let id = traffic.add_npc(npc);
    let ai = NovaAi::from_rulebook(&Rulebook::default());
    let chance = || Draws::of(&[]).keyed(200, stay);
    let ship = |traffic: &Traffic| -> Npc {
        let found = traffic.npcs().iter().find(|npc| npc.id == id);
        found.expect("in the system").clone()
    };
    let mut landed = None;
    for tick in 1..=3000 {
        traffic.tick(&ai, &sites, &mut chance());
        assert_eq!(traffic.departed(), [], "it stays");
        let now = ship(&traffic);
        if now.ai_wait > 0 {
            landed = Some((tick, now));
            break;
        }
        assert_eq!(now.goal, Goal::Land(StellarId(128)), "{tick}");
    }
    let (landed_on, landed) = landed.expect("it lands");
    let mut still_for = 1;
    loop {
        traffic.tick(&ai, &sites, &mut chance());
        let now = ship(&traffic);
        if now.state != landed.state || now.ai_wait == 0 && now.goal == Goal::Wait {
            let then = now.goal;
            if now.state == landed.state {
                still_for += 1;
            }
            return Landing {
                landed_on,
                landed,
                still_for,
                then,
            };
        }
        still_for += 1;
        assert!(still_for <= 600, "it never moved off");
    }
}

#[test]
fn a_trader_lands_in_its_box_stays_still_for_its_wait_then_jumps_out() {
    for (stay, ticks) in [(0, 300), (199, 499)] {
        let landing = land(300, stay);
        assert!(landing.landed_on > 1, "it flew there");
        let at = landing.landed.state;
        assert_eq!(at.velocity, Vec2::ZERO, "stopped");
        let turn = landing.landed.turn_rate();
        assert!(
            crate::landing::npc_over(&site(), at.position, turn, RuleSource::Engine),
            "in its box: {at:?}"
        );
        assert_eq!(landing.landed.visited, Some(StellarId(128)));
        assert_eq!(landing.landed.goal, Goal::Idle, "goal 0");
        assert_eq!(landing.still_for, ticks, "{stay}");
        assert_eq!(landing.then, Goal::JumpOut, "it has visited a stellar here");
    }
}

#[test]
fn a_trader_with_no_fuel_lands_then_stops_and_waits() {
    let landing = land(0, 0);
    assert_eq!(landing.landed.visited, Some(StellarId(128)));
    assert_eq!(landing.then, Goal::Wait, "state 6");
}
