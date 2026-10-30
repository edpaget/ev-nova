//! The traffic in a system that lists a stellar it doesn't show, one
//! another visible system owns: `_SelectRandomStellarDest` walks the
//! system's `NavDefs` (@0x806bd-0x8072e) with no owner or active test,
//! and `_HighLevelAIHandler` states 1 and 20 read the stellar's record
//! (@0x8d8d7, @0x8db8a), so an NPC takes such a stellar as its
//! destination, lands on it in its box, and at a gate enters it within a
//! quarter of its `Type` master's width, which for a gate the player's
//! system doesn't own is the unloaded master's 150
//! ([`gate_entry`](crate::traffic::gate_entry)). The traffic reads the
//! listing from [`World::listed`].

use super::World;
use crate::Rulebook;
use crate::ai::{Goal, NovaAi};
use crate::catalog::{LandingSite, StellarId};
use crate::flight::ShipState;
use crate::geometry::Vec2;
use crate::landing::TypeWidth;
use crate::rulebook::RuleSource;
use crate::stats::ShipStats;
use crate::traffic::Traffic;
use crate::traffic::npc::{AiType, Mode, Npc, NpcId};
use nova_sim_base::fixture::{FAST, Ranged};
use nova_sim_catalog::fixture::planet;
use std::collections::BTreeMap;

/// The stellar the system lists but doesn't show.
const UNSHOWN: StellarId = StellarId(140);

/// The system's stellars: 128, shown, 600 below the centre, and
/// [`UNSHOWN`], 600 above it.
fn listed() -> [LandingSite; 2] {
    [planet(128, 0.0, 600.0), planet(140, 0.0, -600.0)]
}

/// A wimpy trader at the centre, at rest, flying `goal`.
fn trader(goal: Goal) -> Npc {
    let mut npc = crate::traffic::fixture::npc(1, ShipStats::new(FAST, &[]));
    npc.ai_type = AiType::WimpyTrader;
    npc.goal = goal;
    npc
}

/// The ship `id` among `traffic`'s.
fn ship(traffic: &Traffic, id: NpcId) -> Npc {
    let found = traffic.npcs().iter().find(|npc| npc.id == id);
    found.expect("in the system").clone()
}

/// `npc` flown in a world showing `shown` of `listed`, each tick's draw
/// below 2 giving `pick`, until it lands: the ship as it landed.
fn landed(npc: Npc, shown: &[LandingSite], listed: &[LandingSite], pick: u32) -> Npc {
    let mut traffic = Traffic::with_interval(1);
    let id = traffic.add_npc(npc);
    let ai = NovaAi::from_rulebook(&Rulebook::default());
    let world = World {
        listed,
        ..World::new(shown)
    };
    for _ in 0..3000 {
        traffic.tick_in(&ai, world, &[], &mut Ranged::of(&[(2, pick)]));
        assert_eq!(traffic.departed(), [], "it stays");
        let now = ship(&traffic, id);
        if now.ai_wait > 0 {
            return now;
        }
    }
    panic!("it never landed");
}

#[test]
fn an_npc_bound_for_a_stellar_listed_but_not_shown_lands_on_it_in_its_box() {
    let listed = listed();
    let at = landed(trader(Goal::Land(UNSHOWN)), &listed[..1], &listed, 0);
    assert_eq!(at.visited, Some(UNSHOWN));
    let turn = at.turn_rate();
    assert!(
        crate::landing::npc_over(&listed[1], at.state.position, turn, RuleSource::Engine),
        "in its box: {:?}",
        at.state
    );
}

#[test]
fn an_idle_npc_takes_a_stellar_listed_but_not_shown_and_lands_on_it() {
    let listed = listed();
    let at = landed(trader(Goal::Idle), &listed[..1], &listed, 1);
    assert_eq!(at.visited, Some(UNSHOWN), "the second of two candidates");
    let shown_only = landed(trader(Goal::Idle), &listed[..1], &listed[..1], 1);
    assert_eq!(shown_only.visited, Some(StellarId(128)), "one candidate");
}

/// A hypergate the system lists but doesn't show, at (500, 0).
fn gate() -> LandingSite {
    LandingSite {
        flags2: crate::gate::HYPERGATE,
        ..planet(300, 500.0, 0.0)
    }
}

/// Whether a ship `(dx, dy)` off the unshown gate, at rest, its master
/// not loaded, starts to enter it on its first tick through the traffic.
fn enters_from(dx: f32, dy: f32) -> bool {
    let listed = [planet(128, 0.0, 600.0), gate()];
    let widths = BTreeMap::from([(StellarId(300), TypeWidth::Unloaded)]);
    let world = World {
        listed: &listed,
        type_widths: &widths,
        ..World::new(&listed[..1])
    };
    let mut npc = trader(Goal::EnterGate(StellarId(300)));
    npc.state = ShipState {
        position: gate().position + Vec2::new(dx, dy),
        velocity: Vec2::ZERO,
        heading: 0.0,
    };
    let mut traffic = Traffic::with_interval(1);
    let id = traffic.add_npc(npc);
    let ai = NovaAi::from_rulebook(&Rulebook::default());
    traffic.tick_in(&ai, world, &[], &mut Ranged::of(&[]));
    matches!(ship(&traffic, id).mode, Mode::EnteringGate { .. })
}

#[test]
fn an_npc_enters_a_gate_listed_but_not_shown_within_37_on_each_axis() {
    // 150 / 4 = 37, inclusive (@0x8dbdf-0x8dbfa, @0x8dc5a-0x8dc94).
    for (dx, dy) in [
        (37.0, 0.0),
        (-37.0, 0.0),
        (0.0, 37.0),
        (0.0, -37.0),
        (37.0, 37.0),
    ] {
        assert!(enters_from(dx, dy), "({dx}, {dy})");
    }
    for (dx, dy) in [(38.0, 0.0), (0.0, 38.0), (0.0, -38.0)] {
        assert!(!enters_from(dx, dy), "({dx}, {dy})");
    }
}
