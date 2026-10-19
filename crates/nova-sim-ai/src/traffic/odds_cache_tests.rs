//! Tests of the odds cache in the traffic (see [`odds_cache`]): the
//! spawns' marks, the pass after the thinks, and the berths it keeps.

#![allow(clippy::float_cmp)]

use std::collections::{BTreeMap, BTreeSet};

use super::odds_cache::{self, STALE};
use super::tests::{placed, table};
use super::{Leave, Traffic, World};
use crate::ai::{Behaviour, Goal, Surroundings};
use crate::catalog::{GovtId, SystemId};
use crate::chance::Chance;
use crate::combat::ShipRef;
use crate::stats::ShipStats;
use crate::traffic::fixture::npc;
use crate::traffic::npc::{Npc, NpcId};
use crate::traffic::table::SpawnTable;
use nova_sim_base::fixture::{Draws, FAST};

/// Each NPC listed takes its goal; the others keep theirs.
#[derive(Debug)]
struct Turns(BTreeMap<NpcId, Goal>);

impl Behaviour for Turns {
    fn decide(&self, npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
        self.0.get(&npc.id).copied().unwrap_or(npc.goal)
    }
}

/// Every NPC keeps its goal.
fn keeping() -> Turns {
    Turns(BTreeMap::new())
}

/// Traffic deciding every tick, with an NPC of each `strength` added in
/// turn (berths 1, 2, ...), each of its own government, idle and at rest.
fn traffic(strengths: &[f32]) -> Traffic {
    let mut traffic = Traffic::with_interval(1);
    for (&strength, govt) in strengths.iter().zip(140..) {
        let mut ship = npc(0, ShipStats::new(FAST, &[]));
        ship.hull.strength = strength;
        ship.govt = Some(GovtId(govt));
        traffic.add_npc(ship);
    }
    traffic
}

/// NPC `id` of `traffic`.
fn ship(traffic: &Traffic, id: u32) -> &Npc {
    traffic
        .npcs()
        .iter()
        .find(|npc| npc.id == NpcId(id))
        .expect("in the system")
}

/// NPC `id` of `traffic` attacks NPC `target`.
fn attack(traffic: &mut Traffic, id: u32, target: u32) {
    if let Some(npc) = traffic.npcs.iter_mut().find(|npc| npc.id == NpcId(id)) {
        npc.goal = Goal::Attack(ShipRef::Npc(NpcId(target)));
    }
}

/// `traffic` one tick on as `behaviour` decides, alone in the system.
fn tick(traffic: &mut Traffic, behaviour: &Turns) {
    traffic.tick_in(behaviour, World::new(&[]), &[], &mut Draws::of(&[]));
}

/// Whether `odds` is the stale marker.
fn stale(odds: f32) -> bool {
    odds.to_bits() == STALE.to_bits()
}

/// Whether every NPC's odds are its berth's.
fn mirrored(traffic: &Traffic) -> bool {
    traffic
        .npcs()
        .iter()
        .all(|npc| npc.odds.to_bits() == traffic.berth_odds(npc.berth).to_bits())
}

#[test]
fn a_ship_added_reads_stale_and_so_does_its_berth() {
    let traffic = traffic(&[100.0, 50.0]);
    for npc in traffic.npcs() {
        assert!(stale(npc.odds));
        assert!(stale(traffic.berth_odds(npc.berth)));
    }
    let mut placed_at = Traffic::new();
    placed_at.add_npc_at(npc(0, ShipStats::new(FAST, &[])), 9);
    assert!(stale(placed_at.berth_odds(9)));
}

#[test]
fn a_ship_the_table_places_reads_stale_and_so_does_its_berth() {
    let mut traffic = Traffic::new();
    traffic.enter(table(1), &mut Draws::of(&placed(750, 750)));
    let placed = &traffic.npcs()[0];
    assert!(stale(placed.odds));
    assert!(stale(traffic.berth_odds(placed.berth)));
}

#[test]
fn a_tick_refreshes_the_first_stale_ship_after_the_ticks_goals() {
    let mut traffic = traffic(&[100.0, 50.0]);
    let turned = Turns(BTreeMap::from([(
        NpcId(1),
        Goal::Attack(ShipRef::Npc(NpcId(0))),
    )]));
    tick(&mut traffic, &turned);
    assert_eq!(
        ship(&traffic, 0).odds,
        0.5,
        "its attacker of strength 50 counts, turned on it this tick"
    );
    assert!(stale(ship(&traffic, 1).odds), "marked");
    assert!(mirrored(&traffic));
}

#[test]
fn a_tick_refreshes_one_ship_and_the_next_tick_the_next() {
    let mut traffic = traffic(&[100.0, 50.0, 25.0]);
    attack(&mut traffic, 0, 1);
    attack(&mut traffic, 1, 0);
    tick(&mut traffic, &keeping());
    assert_eq!(ship(&traffic, 0).odds, 0.5);
    assert!(stale(ship(&traffic, 1).odds));
    assert!(stale(ship(&traffic, 2).odds));
    tick(&mut traffic, &keeping());
    assert_eq!(ship(&traffic, 0).odds, 0.5, "kept until its next round");
    assert_eq!(ship(&traffic, 1).odds, 2.0);
    assert!(stale(ship(&traffic, 2).odds));
    tick(&mut traffic, &keeping());
    assert_eq!(ship(&traffic, 2).odds, 0.0, "no enemy");
    assert!(stale(traffic.berth_odds(4)));
    assert!(mirrored(&traffic));
}

#[test]
fn a_ship_leaving_with_the_marker_passes_it_on() {
    let mut traffic = traffic(&[100.0, 50.0]);
    traffic.remove(NpcId(0));
    tick(&mut traffic, &keeping());
    assert_eq!(traffic.berth_odds(1), 0.0, "no ship there");
    assert_eq!(ship(&traffic, 1).odds, 0.0, "refreshed on the same tick");
    assert!(stale(traffic.berth_odds(3)));
}

#[test]
fn with_no_ship_left_every_berth_reads_zero_and_slot_one_is_marked() {
    let mut traffic = traffic(&[100.0]);
    traffic.remove(NpcId(0));
    tick(&mut traffic, &keeping());
    assert!((2..=odds_cache::LAST_SLOT).all(|berth| traffic.berth_odds(berth) == 0.0));
    assert!(stale(traffic.berth_odds(1)));
}

#[test]
fn a_ship_left_behind_reads_zero_once_the_walk_passes_its_berth_and_comes_back_so() {
    let (here, there) = (SystemId(130), SystemId(131));
    let mut traffic = traffic(&[100.0, 50.0]);
    traffic.retarget(
        SpawnTable::default(),
        &BTreeSet::from([NpcId(1)]),
        Leave::Held {
            from: here,
            to: there,
        },
    );
    traffic.refresh_odds(&World::new(&[]));
    assert_eq!(traffic.berth_odds(1), 0.0, "in another system");
    assert_eq!(ship(&traffic, 1).odds, 0.0, "the walk went on to it");
    traffic.retarget(
        SpawnTable::default(),
        &BTreeSet::from([NpcId(1)]),
        Leave::Held {
            from: there,
            to: here,
        },
    );
    assert_eq!(ship(&traffic, 0).odds, 0.0, "back, as the walk left it");
    assert!(mirrored(&traffic));
}

#[test]
fn an_in_flight_move_keeps_the_odds_of_the_ships_it_takes_along() {
    let mut traffic = traffic(&[100.0, 50.0]);
    attack(&mut traffic, 1, 0);
    tick(&mut traffic, &keeping());
    traffic.retarget(
        SpawnTable::default(),
        &BTreeSet::from([NpcId(0), NpcId(1)]),
        Leave::Dropped,
    );
    assert_eq!(ship(&traffic, 0).odds, 0.5, "not respawned");
    assert!(stale(ship(&traffic, 1).odds));
}

#[test]
fn a_ship_at_a_berth_past_the_last_slot_stays_stale() {
    let mut traffic = Traffic::with_interval(1);
    let id = traffic.add_npc_at(npc(0, ShipStats::new(FAST, &[])), 64);
    for _ in 0..3 {
        tick(&mut traffic, &keeping());
    }
    assert!(stale(ship(&traffic, id.0).odds));
}

#[test]
fn caching_odds_writes_the_ship_and_its_berth() {
    let mut traffic = traffic(&[100.0]);
    traffic.cache_odds(NpcId(0), 1.5);
    assert_eq!(ship(&traffic, 0).odds, 1.5);
    assert_eq!(traffic.berth_odds(1), 1.5);
    traffic.cache_odds(NpcId(9), 3.0);
    assert!(mirrored(&traffic), "a ship not here changes nothing");
}

#[test]
fn a_stale_berth_held_by_a_ship_with_no_ai_reads_zero_and_the_walk_goes_on_to_the_next_ship() {
    let mut traffic = traffic(&[100.0, 50.0, 25.0]);
    traffic.npcs[0].no_ai = true;
    attack(&mut traffic, 2, 1);
    traffic.refresh_odds(&World::new(&[]));
    assert_eq!(
        traffic.berth_odds(1),
        0.0,
        "zeroed, as AI type -1 is (@0x4431f)"
    );
    assert_eq!(ship(&traffic, 0).odds, 0.0);
    assert_eq!(ship(&traffic, 1).odds, 0.5, "worked out in the same pass");
    assert!(stale(ship(&traffic, 2).odds), "marked, and the walk stops");
    assert!(mirrored(&traffic));
}
