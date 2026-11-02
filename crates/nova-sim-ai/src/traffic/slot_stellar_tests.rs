//! The traffic flying a stellar a slot names: a ship's +0x6c read as a
//! stellar's table index has no bounds test (`_MinerAI` @0x8b36c-0x8b3ae,
//! goal 1 @0x8d8d7-0x8d8e8), so a slot names the stellar of ID 128 + slot,
//! a record that usually belongs to another system. Goal 1 flies to that
//! record's place (@0x8d90d-0x8d950) and lands there, and one with
//! `Flags2` 0x3000 hands off to goal 0x14 (@0x8d8ed-0x8d8f5), which is
//! entered as any gate is. The traffic reads those records from
//! [`World::slot_stellars`] ([`World::flown_among`]).

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

/// The system's one stellar, 128, 600 below the centre.
fn listed() -> [LandingSite; 1] {
    [planet(128, 0.0, 600.0)]
}

/// The stellar slot 12 names, another system's, 600 above the centre.
const FOREIGN: StellarId = StellarId(140);

/// The slot stellars: [`FOREIGN`], and a hypergate, 131, at (500, 0).
fn slot_stellars() -> [LandingSite; 2] {
    [
        planet(140, 0.0, -600.0),
        LandingSite {
            flags2: crate::gate::HYPERGATE,
            ..planet(131, 500.0, 0.0)
        },
    ]
}

/// A wimpy trader at the centre, at rest, flying `goal`.
fn trader(goal: Goal) -> Npc {
    let mut npc = crate::traffic::fixture::npc(1, ShipStats::new(FAST, &[]));
    npc.ai_type = AiType::WimpyTrader;
    npc.goal = goal;
    npc
}

/// The ship `id` among `traffic`'s.
fn ship(traffic: &Traffic, id: NpcId) -> Option<Npc> {
    traffic.npcs().iter().find(|npc| npc.id == id).cloned()
}

#[test]
fn a_world_flies_a_stellar_it_doesnt_list_among_the_slot_stellars() {
    let (listed, slots) = (listed(), slot_stellars());
    let world = World {
        slot_stellars: &slots,
        ..World::new(&listed)
    };
    let ids = |sites: &[LandingSite]| sites.iter().map(|site| site.id).collect::<Vec<_>>();
    assert_eq!(
        ids(world.flown_among(Goal::Land(FOREIGN))),
        [FOREIGN, StellarId(131)]
    );
    assert_eq!(
        ids(world.flown_among(Goal::EnterGate(StellarId(131)))),
        [FOREIGN, StellarId(131)]
    );
    assert_eq!(
        ids(world.flown_among(Goal::Land(StellarId(128)))),
        [StellarId(128)]
    );
    assert_eq!(ids(world.flown_among(Goal::Idle)), [StellarId(128)]);
}

#[test]
fn a_landing_outside_the_system_flies_to_that_stellars_place_and_lands() {
    let (listed, slots) = (listed(), slot_stellars());
    let world = World {
        slot_stellars: &slots,
        ..World::new(&listed)
    };
    let mut traffic = Traffic::with_interval(1);
    let id = traffic.add_npc(trader(Goal::Land(FOREIGN)));
    let ai = NovaAi::from_rulebook(&Rulebook::default());
    for _ in 0..3000 {
        traffic.tick_in(&ai, world, &[], &mut Ranged::of(&[]));
        assert_eq!(traffic.departed(), [], "it stays");
        let now = ship(&traffic, id).expect("here");
        if now.ai_wait > 0.0 {
            assert_eq!(now.visited, Some(FOREIGN));
            assert_eq!(now.travel, Some(FOREIGN), "+0x6c keeps it");
            let turn = now.turn_rate();
            assert!(
                crate::landing::npc_over(&slots[0], now.state.position, turn, RuleSource::Engine),
                "in its box: {:?}",
                now.state
            );
            return;
        }
    }
    panic!("it never landed");
}

#[test]
fn a_gate_outside_the_system_is_entered() {
    let (listed, slots) = (listed(), slot_stellars());
    let widths = BTreeMap::from([(StellarId(131), TypeWidth::Unloaded)]);
    let world = World {
        slot_stellars: &slots,
        type_widths: &widths,
        ..World::new(&listed)
    };
    let mut npc = trader(Goal::Land(StellarId(131)));
    npc.state = ShipState {
        position: slots[1].position + Vec2::new(20.0, 0.0),
        velocity: Vec2::ZERO,
        heading: 0.0,
    };
    let mut traffic = Traffic::with_interval(1);
    let id = traffic.add_npc(npc);
    let ai = NovaAi::from_rulebook(&Rulebook::default());
    traffic.tick_in(&ai, world, &[], &mut Ranged::of(&[]));
    let now = ship(&traffic, id).expect("here");
    assert_eq!(now.goal, Goal::EnterGate(StellarId(131)), "goal 0x14");
    assert!(
        matches!(now.mode, Mode::EnteringGate { .. }),
        "within the unloaded master's 37: {:?}",
        now.mode
    );
}

/// Every ship keeps its goal.
#[derive(Debug)]
struct KeepGoal;

impl crate::ai::Behaviour for KeepGoal {
    fn decide(
        &self,
        npc: &Npc,
        _: &crate::ai::Surroundings,
        _: &mut dyn crate::chance::Chance,
    ) -> Goal {
        npc.goal
    }
}

#[test]
fn a_stellar_with_no_record_brakes() {
    // A plug-in's gap in IDs 128 to 191 reads memory `NewPtr` left as it
    // was (@0x1b7cd-0x1b7d4), which can't be replicated: the ship brakes.
    let (listed, slots) = (listed(), slot_stellars());
    let world = World {
        slot_stellars: &slots,
        ..World::new(&listed)
    };
    let mut npc = trader(Goal::Land(StellarId(150)));
    npc.state.velocity = Vec2::new(2.0, 0.0);
    let mut traffic = Traffic::with_interval(1);
    let id = traffic.add_npc(npc);
    for _ in 0..200 {
        traffic.tick_in(&KeepGoal, world, &[], &mut Ranged::of(&[]));
    }
    let now = ship(&traffic, id).expect("here");
    assert_eq!(now.goal, Goal::Land(StellarId(150)));
    assert!(
        now.state.velocity.length() < 0.5,
        "turned and braked: {:?}",
        now.state
    );
}
