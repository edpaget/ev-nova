//! The tests of `_AIDispatch`'s loop as the traffic thinks it
//! ([`Traffic::think_run`], and [`Traffic::think_holding`] driving one run
//! a berth): each slot reads what the slots below it wrote this frame,
//! and the slots above it as their last frame left them
//! (@0x44270-0x442ae). The fight between the runs is the session's
//! (`nova_sim::Session::dispatch_npcs`), which its tests pin.

use std::cell::RefCell;

use super::gone::Freed;
use super::{Traffic, World};
use crate::ai::{Behaviour, GateWait, Goal, Surroundings};
use crate::catalog::{WeaponId, WeaponRecord};
use crate::chance::Chance;
use crate::cloak::Cloak;
use crate::combat::armament::{Armament, LauncherRounds, Trigger};
use crate::combat::hull::ESCAPE_POD_SHIP;
use crate::combat::weapon::WeaponSpec;
use crate::combat::{NpcId, ShipRef};
use crate::stats::ShipStats;
use crate::traffic::fixture;
use crate::traffic::npc::{Mode, Npc};
use nova_sim_base::fixture::Draws;
use nova_sim_catalog::fixture::weapon;

/// What [`Marking`] has every thinking NPC fire.
const MARKED: Trigger = Trigger {
    primary: true,
    secondary: None,
    only: None,
    bays: false,
    bays_after: false,
    point_defence: true,
    keep: Some(WeaponId(128)),
};

/// Attacks the player, fires [`MARKED`] at it, and holds a gate wait
/// that drops its target: each NPC that thinks is marked by it.
#[derive(Debug, Default)]
struct Marking(RefCell<Vec<NpcId>>);

impl Behaviour for Marking {
    fn decide(&self, npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
        self.0.borrow_mut().push(npc.id);
        Goal::Attack(ShipRef::Player)
    }

    fn trigger(&self, _npc: &Npc, _around: &Surroundings) -> Trigger {
        MARKED
    }

    fn target(&self, _npc: &Npc, _around: &Surroundings) -> Option<ShipRef> {
        Some(ShipRef::Player)
    }

    fn gate_wait(&self, _npc: &Npc, _around: &Surroundings) -> GateWait {
        GateWait {
            trigger: MARKED,
            drops_target: true,
        }
    }
}

/// A carrier whose bay is ready with two fighters.
fn carrier(id: u32) -> Npc {
    let bay = WeaponRecord {
        guidance: 99,
        ammo_type: 144,
        flags: 0x0002,
        reload: 60,
        ..weapon(149)
    };
    Npc {
        armament: Armament::new([(WeaponSpec::new(&bay), 1)]),
        rounds: LauncherRounds::from([(WeaponId(149), 2)]),
        ..fixture::npc(id, ShipStats::default())
    }
}

/// The goal-4 launch with the bay as the pick.
const LAUNCHING: Trigger = Trigger {
    primary: false,
    secondary: None,
    only: Some(WeaponId(149)),
    bays: true,
    bays_after: false,
    point_defence: false,
    keep: None,
};

/// NPC `id` of `traffic`.
fn find(traffic: &Traffic, id: NpcId) -> &Npc {
    traffic
        .npcs()
        .iter()
        .find(|npc| npc.id == id)
        .expect("there")
}

/// A world whose spawns leave `reserve` slots free.
fn world(reserve: u32) -> World<'static> {
    World {
        spawn_reserve: reserve,
        ..World::new(&[])
    }
}

#[test]
fn a_run_leaves_every_ship_outside_it_as_it_was() {
    let mut traffic = Traffic::new();
    // Cloaked with no cloak: a think would uncloak it.
    let flying = Npc {
        cloak: Cloak {
            fade: 20.0,
            direction: 0,
        },
        ..fixture::npc(0, ShipStats::default())
    };
    traffic.add_npc_at(flying, 1);
    let held = Npc {
        mode: Mode::HeldAtGate { ticks_left: 5 },
        target: Some(ShipRef::Player),
        ..fixture::npc(0, ShipStats::default())
    };
    traffic.add_npc_at(held, 3);
    // Coasting with a kept launch: settled again with no room, it
    // would lose it.
    let coasting = Npc {
        mode: Mode::Coasting { ticks_left: 5 },
        trigger: LAUNCHING,
        ..carrier(0)
    };
    traffic.add_npc_at(coasting, 4);
    let pod = Npc {
        ship: ESCAPE_POD_SHIP,
        mode: Mode::Adrift { ticks_left: 5 },
        ..fixture::npc(0, ShipStats::default())
    };
    traffic.add_npc_at(pod, 5);
    let fighter = Npc {
        cloak: Cloak {
            fade: 20.0,
            direction: 0,
        },
        ..fixture::npc(0, ShipStats::default())
    };
    let fighter = traffic.add_npc_at(fighter, 2);
    let others: Vec<Npc> = traffic
        .npcs()
        .iter()
        .filter(|npc| npc.id != fighter)
        .cloned()
        .collect();
    let marking = Marking::default();
    // No berth free below 64 - 63.
    traffic.think_run(
        &marking,
        world(63),
        |npc| npc.id == fighter,
        &crate::traffic::NoBay::new(),
        &mut Draws::of(&[]),
    );
    assert_eq!(marking.0.into_inner(), [fighter], "it alone decides");
    let after: Vec<Npc> = traffic
        .npcs()
        .iter()
        .filter(|npc| npc.id != fighter)
        .cloned()
        .collect();
    assert_eq!(after, others);
    let thought = find(&traffic, fighter);
    assert_eq!(thought.goal, Goal::Attack(ShipRef::Player));
    assert_eq!(thought.target, Some(ShipRef::Player));
    assert_eq!(thought.trigger, MARKED);
    assert_eq!(thought.pick, Some(WeaponId(128)));
    assert_eq!(thought.cloak.direction, -1, "its cloak think");
}

/// Keeps each goal, and asks for the goal-4 launch, recording each
/// NPC's room as it asks.
#[derive(Debug, Default)]
struct Launching(RefCell<Vec<(NpcId, bool)>>);

impl Behaviour for Launching {
    fn decide(&self, npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
        npc.goal
    }

    fn trigger(&self, npc: &Npc, around: &Surroundings) -> Trigger {
        self.0.borrow_mut().push((npc.id, around.fighter_room));
        LAUNCHING
    }
}

#[test]
fn each_run_reads_the_berths_free_as_it_starts() {
    // Below 64 - 61 = 3, berths 1 and 2 are free.
    let mut traffic = Traffic::new();
    let low = traffic.add_npc_at(carrier(0), 3);
    let high = traffic.add_npc_at(carrier(0), 5);
    let launching = Launching::default();
    let run = |traffic: &mut Traffic, id| {
        traffic.think_run(
            &launching,
            world(61),
            |npc| npc.id == id,
            &crate::traffic::NoBay::new(),
            &mut Draws::of(&[]),
        );
    };
    run(&mut traffic, low);
    // Its fighter takes berth 1, the session's work between the runs.
    traffic.add_npc_at(fixture::npc(0, ShipStats::default()), 1);
    run(&mut traffic, high);
    assert_eq!(find(&traffic, high).trigger, LAUNCHING, "berth 2 is free");
    traffic.add_npc_at(fixture::npc(0, ShipStats::default()), 2);
    run(&mut traffic, high);
    assert_eq!(
        launching.0.into_inner(),
        [(low, true), (high, true), (high, false)]
    );
    assert_eq!(
        find(&traffic, high).trigger,
        Trigger {
            only: Some(WeaponId(149)),
            ..Trigger::default()
        },
        "its launch cleared, its pick kept"
    );
}

/// Writes `new` as the goal of each NPC that decides, recording what it
/// read of the goal of the other ship there.
#[derive(Debug)]
struct Reading {
    new: Goal,
    reads: RefCell<Vec<(NpcId, Goal)>>,
}

impl Reading {
    fn new() -> Self {
        Self {
            new: Goal::JumpOut,
            reads: RefCell::default(),
        }
    }
}

impl Behaviour for Reading {
    fn decide(&self, npc: &Npc, around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
        let other = around.npcs.iter().find(|other| other.id != npc.id);
        let read = other.expect("two ships").goal;
        self.reads.borrow_mut().push((npc.id, read));
        self.new
    }
}

/// Two ships at berths 5 then 2, out of order, and their numbers lowest
/// berth first.
fn two_ships() -> (Traffic, NpcId, NpcId) {
    let mut traffic = Traffic::new();
    let high = traffic.add_npc_at(fixture::npc(0, ShipStats::default()), 5);
    let low = traffic.add_npc_at(fixture::npc(0, ShipStats::default()), 2);
    (traffic, low, high)
}

#[test]
fn think_holding_stops_at_every_berth() {
    let (mut traffic, low, high) = two_ships();
    let reading = Reading::new();
    traffic.think_holding(&reading, World::new(&[]), &[], &mut Draws::of(&[]));
    assert_eq!(
        reading.reads.into_inner(),
        [(low, Goal::Idle), (high, Goal::JumpOut)],
        "a higher berth reads a lower one's new goal, not the other way"
    );
}

#[test]
fn a_later_run_reads_the_goals_a_lower_run_wrote() {
    let (mut traffic, low, high) = two_ships();
    let reading = Reading::new();
    let world = World::new(&[]);
    let mut chance = Draws::of(&[]);
    traffic.think_run(
        &reading,
        world,
        |npc| npc.id == low,
        &crate::traffic::NoBay::new(),
        &mut chance,
    );
    traffic.think_run(
        &reading,
        world,
        |npc| npc.id == high,
        &crate::traffic::NoBay::new(),
        &mut chance,
    );
    assert_eq!(
        reading.reads.into_inner(),
        [(low, Goal::Idle), (high, Goal::JumpOut)]
    );
}

#[test]
fn a_pods_wait_counts_down_only_in_its_own_run() {
    // The pod's test is the first thing in its own `_AIDispatch`
    // (@0x8fb62-0x8fb68).
    let mut traffic = Traffic::new();
    let pod = Npc {
        ship: ESCAPE_POD_SHIP,
        mode: Mode::Adrift { ticks_left: 1 },
        ..fixture::npc(0, ShipStats::default())
    };
    let pod = traffic.add_npc_at(pod, 3);
    let other = traffic.add_npc_at(fixture::npc(0, ShipStats::default()), 1);
    let world = World::new(&[]);
    let mut chance = Draws::of(&[]);
    let keep = Marking::default();
    traffic.think_run(
        &keep,
        world,
        |npc| npc.id == other,
        &crate::traffic::NoBay::new(),
        &mut chance,
    );
    assert_eq!(find(&traffic, pod).mode, Mode::Adrift { ticks_left: 1 });
    traffic.think_run(
        &keep,
        world,
        |npc| npc.id == pod,
        &crate::traffic::NoBay::new(),
        &mut chance,
    );
    assert_eq!(find(&traffic, pod).mode, Mode::Adrift { ticks_left: 0 });
    assert_eq!(traffic.free_berths(0), 61, "held by both");
    traffic.think_run(
        &keep,
        world,
        |npc| npc.id == pod,
        &crate::traffic::NoBay::new(),
        &mut chance,
    );
    assert!(traffic.npcs().iter().all(|npc| npc.id != pod), "taken out");
    assert_eq!(traffic.free_berths(0), 62, "free at once");
}

/// Records the gone ships each decision reads, keeping its goal.
#[derive(Debug, Default)]
struct ReadingGone(RefCell<Vec<usize>>);

impl Behaviour for ReadingGone {
    fn decide(&self, npc: &Npc, around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
        self.0.borrow_mut().push(around.gone.len());
        npc.goal
    }
}

#[test]
fn every_run_of_a_tick_reads_the_gone_ships_and_close_think_forgets_them() {
    let (mut traffic, low, high) = two_ships();
    let ship = fixture::npc(9, ShipStats::default());
    traffic.gone.record(ship, Freed::InDispatch);
    let reading = ReadingGone::default();
    let world = World::new(&[]);
    let mut chance = Draws::of(&[]);
    traffic.think_run(
        &reading,
        world,
        |npc| npc.id == low,
        &crate::traffic::NoBay::new(),
        &mut chance,
    );
    traffic.think_run(
        &reading,
        world,
        |npc| npc.id == high,
        &crate::traffic::NoBay::new(),
        &mut chance,
    );
    assert_eq!(traffic.gone.ships().len(), 1, "held to the tick's end");
    traffic.close_think();
    assert_eq!(traffic.gone.ships(), []);
    assert_eq!(reading.0.into_inner(), [1, 1]);
}
