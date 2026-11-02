//! A pirate that loses its lead picks or skips its idle block's pick by
//! the travel stellar its fleet left, through the traffic and Nova's AI
//! ([`travel`](crate::ai::travel), the read @0x8c472-0x8c477): a successor
//! under its lead's government reads the lead's (`_AICheckParentExists`
//! @0x893b0-0x893e1), and a follower left alone its own, the lead's slot,
//! until the disabled reset clears it (@0x8feda-0x8ff13).

use crate::ai::{Behaviour, Goal, NovaAi, Surroundings};
use crate::bay::Carrier;
use crate::catalog::{GovtId, GovtRecord, LandingSite, StellarId};
use crate::chance::Chance;
use crate::combat::ShipRef;
use crate::combat::hull::Condition;
use crate::geometry::Vec2;
use crate::govt::{Governments, PLUNDERS};
use crate::stats::ShipStats;
use crate::traffic::fixture::npc;
use crate::traffic::npc::{AiType, Npc, NpcId};
use crate::traffic::regroup::RELEASE_DRAW;
use crate::traffic::{Traffic, World};
use nova_sim_base::fixture::{Draws, FAST};
use nova_sim_catalog::fixture::{govt, planet};

/// Every ship keeps its goal.
#[derive(Debug)]
struct Keep;

impl Behaviour for Keep {
    fn decide(&self, npc: &Npc, _: &Surroundings, _: &mut dyn Chance) -> Goal {
        npc.goal
    }
}

/// The stellar they may pick, far from them.
const STELLAR: StellarId = StellarId(128);
/// The pirates' government.
const PIRATES: GovtId = GovtId(128);
/// The lead, the traffic's first ship.
const LEAD: NpcId = NpcId(0);
/// Its follower, the second.
const FOLLOWER: NpcId = NpcId(1);
/// A pirate warship of class mass `mass` and `InherentAI` 3 at the centre.
fn pirate(mass: i16) -> Npc {
    let mut ship = npc(0, ShipStats::new(FAST, &[]));
    ship.govt = Some(PIRATES);
    ship.ai_type = AiType::Warship;
    ship.hull.inherent_ai = 3;
    ship.hull.signed_mass = mass;
    ship
}

/// [`pirate`] led by `lead`, keeping formation on it.
fn led_by(lead: NpcId, mass: i16) -> Npc {
    Npc {
        leader: Some(lead),
        goal: Goal::Follow { lead, guard: None },
        ..pirate(mass)
    }
}

/// A fleet of the lead, holding with its travel stellar `lead_set`, and a
/// follower with its own `own`: the lead's slot, or -1 under Hold.
fn fleet(lead_set: bool, own: bool) -> Traffic {
    let mut traffic = Traffic::with_interval(1);
    traffic.add_npc(Npc {
        goal: Goal::Hold,
        travel: lead_set.then_some(StellarId(140)),
        ..pirate(500)
    });
    traffic.add_npc(Npc {
        travel: own.then_some(StellarId(128)),
        ..led_by(LEAD, 300)
    });
    traffic
}

/// `traffic` a tick on by Nova's AI, among [`pirates`]' government and the
/// one stellar, a regroup pause released or held as `release` says.
fn tick_by(traffic: &mut Traffic, release: bool) {
    let sites = sites();
    let govts = pirates();
    let world = World {
        govts: &govts,
        ..World::new(&sites)
    };
    let mut draws = Draws::default().keyed(RELEASE_DRAW, u32::from(!release));
    traffic.tick_in(&NovaAi::default(), world, &[], &mut draws);
}

/// [`tick_by`], releasing any pause.
fn tick(traffic: &mut Traffic) {
    tick_by(traffic, true);
}

fn sites() -> [LandingSite; 1] {
    [planet(STELLAR.0, 800.0, 800.0)]
}

/// The government, which plunders.
fn pirates() -> Governments {
    Governments::new([GovtRecord {
        flags: PLUNDERS,
        ..govt(PIRATES.0)
    }])
}

fn of(traffic: &Traffic, id: NpcId) -> &Npc {
    traffic
        .npcs()
        .iter()
        .find(|npc| npc.id == id)
        .expect("in the system")
}

fn of_mut(traffic: &mut Traffic, id: NpcId) -> &mut Npc {
    traffic
        .npcs_mut()
        .iter_mut()
        .find(|npc| npc.id == id)
        .expect("in the system")
}

/// `id` destroyed and gone, its record kept.
fn destroy(traffic: &mut Traffic, id: NpcId) {
    of_mut(traffic, id).condition = Condition::Destroyed;
    traffic.remove_destroyed(id);
}

#[test]
fn a_pirate_successor_of_a_lead_with_no_stellar_picks_one() {
    let mut traffic = fleet(false, true);
    destroy(&mut traffic, LEAD);
    tick(&mut traffic);
    let successor = of(&traffic, FOLLOWER);
    assert_eq!(successor.leader, None);
    assert_eq!(successor.goal, Goal::Land(STELLAR));
}

#[test]
fn a_pirate_successor_of_a_lead_bound_for_one_leaves_without_picking() {
    let mut traffic = fleet(true, false);
    destroy(&mut traffic, LEAD);
    tick(&mut traffic);
    let successor = of(&traffic, FOLLOWER);
    assert_eq!(successor.leader, None);
    assert_eq!(successor.goal, Goal::JumpOut);
}

#[test]
fn a_pirate_successor_of_an_escort_lead_leaves_without_picking() {
    // The escort holds its own lead's slot from its think (@0x83f09), the
    // fighter it launched takes that over as its successor, with goal 0xa
    // and no leader, a hold (@0x893b0-0x893e1), and its pirate's think
    // reads it (@0x8c472-0x8c477).
    const ESCORT: NpcId = NpcId(1);
    const FIGHTER: NpcId = NpcId(2);
    let mut traffic = Traffic::with_interval(1);
    traffic.add_npc(pirate(900));
    traffic.add_npc(led_by(LEAD, 500));
    let mut fighter = Npc {
        carrier: Some(Carrier {
            ship: ShipRef::Npc(ESCORT),
            window: 100.0,
            reach: 40.0,
        }),
        ..led_by(ESCORT, 100)
    };
    fighter.state.position = Vec2::new(500.0, 500.0);
    traffic.add_npc(fighter);
    tick(&mut traffic);
    assert!(
        of(&traffic, ESCORT).travel_set(),
        "formation writes its slot"
    );
    destroy(&mut traffic, ESCORT);
    tick(&mut traffic);
    let fighter = of(&traffic, FIGHTER);
    assert_eq!(fighter.leader, None);
    assert_eq!(fighter.carrier, None);
    assert_eq!(fighter.goal, Goal::JumpOut);
}

/// A lost lead's one follower, disabled, left alone in the regroup pause
/// with its own travel stellar (@0x894b1-0x894f3), held there a tick.
fn left_alone() -> Traffic {
    let mut traffic = fleet(false, true);
    of_mut(&mut traffic, FOLLOWER).condition = Condition::Disabled;
    destroy(&mut traffic, LEAD);
    tick_by(&mut traffic, false);
    let ship = of(&traffic, FOLLOWER);
    assert!(matches!(ship.goal, Goal::Regroup { .. }), "{:?}", ship.goal);
    assert_eq!(ship.leader, None);
    assert!(ship.travel_set(), "paused, it skips the reset");
    traffic
}

#[test]
fn a_follower_left_alone_and_whole_by_its_release_leaves_without_picking() {
    let mut traffic = left_alone();
    of_mut(&mut traffic, FOLLOWER).condition = Condition::Intact;
    tick(&mut traffic);
    assert_eq!(of(&traffic, FOLLOWER).goal, Goal::JumpOut);
}

#[test]
fn one_still_disabled_at_its_release_picks_afresh_once_whole() {
    let mut traffic = left_alone();
    tick(&mut traffic);
    assert!(!of(&traffic, FOLLOWER).travel_set(), "the reset (@0x8fee9)");
    of_mut(&mut traffic, FOLLOWER).condition = Condition::Intact;
    tick(&mut traffic);
    assert_eq!(of(&traffic, FOLLOWER).goal, Goal::Land(STELLAR));
}

#[test]
fn a_follower_released_at_its_leads_gate_holds_the_gate() {
    // +0x6c the lead's, its gate (@0x8dd30).
    const GATE: StellarId = StellarId(129);
    let sites = [LandingSite {
        flags2: crate::gate::HYPERGATE,
        ..planet(GATE.0, 0.0, 0.0)
    }];
    let mut traffic = Traffic::with_interval(1);
    traffic.add_npc(Npc {
        goal: Goal::EnterGate(GATE),
        ..pirate(500)
    });
    let mut follower = led_by(LEAD, 300);
    follower.state.position = Vec2::new(0.0, 500.0);
    traffic.add_npc(follower);
    traffic.tick(&Keep, &sites, &mut Draws::of(&[]));
    let released = of(&traffic, FOLLOWER);
    assert_eq!(released.goal, Goal::EnterGate(GATE));
    assert_eq!(released.travel, Some(GATE));
}

// A successor of a lead of another AI type reads the +0x6c that lead's
// think left (`_AICheckParentExists` @0x893b0-0x893e1).

/// A stellar a lead held from before, far off.
const HELD: StellarId = StellarId(140);

/// `traffic` a tick on by Nova's AI among `sites` and the pirates'
/// government, any pause released.
fn tick_among(traffic: &mut Traffic, sites: &[LandingSite]) {
    let govts = pirates();
    let world = World {
        govts: &govts,
        ..World::new(sites)
    };
    let mut draws = Draws::default().keyed(RELEASE_DRAW, 0);
    traffic.tick_in(&NovaAi::default(), world, &[], &mut draws);
}

/// A system whose one stellar no ship may land on.
fn unlandable() -> [LandingSite; 1] {
    [LandingSite {
        flags: 0,
        ..sites()[0]
    }]
}

/// The lead, `lead` of the pirates' government holding [`HELD`], and its
/// pirate follower: the lead thinks a tick among `first`, then is
/// destroyed, and the follower takes over on the next tick among the
/// one stellar it may pick. The successor as it stands.
fn successor_after(lead: Npc, first: &[LandingSite]) -> Npc {
    let mut traffic = Traffic::with_interval(1);
    traffic.add_npc(Npc {
        travel: Some(HELD),
        ..lead
    });
    traffic.add_npc(led_by(LEAD, 300));
    tick_among(&mut traffic, first);
    destroy(&mut traffic, LEAD);
    tick(&mut traffic);
    let successor = of(&traffic, FOLLOWER).clone();
    assert_eq!(successor.leader, None);
    successor
}

/// [`pirate`] thinking as `ai_type`, of that type's class.
fn of_type(ai_type: AiType, inherent: i16) -> Npc {
    let mut ship = pirate(500);
    ship.ai_type = ai_type;
    ship.hull.inherent_ai = inherent;
    ship
}

#[test]
fn a_pirate_successor_of_a_trader_lead_whose_pick_failed_picks() {
    // The pick's -1 (`_WimpyTraderAI` @0x8b0d1, @0x8b0ef; `_BraveTraderAI`
    // @0x8b53b, @0x8b559).
    for (ai_type, inherent) in [(AiType::WimpyTrader, 1), (AiType::BraveTrader, 2)] {
        let successor = successor_after(of_type(ai_type, inherent), &unlandable());
        assert_eq!(successor.goal, Goal::Land(STELLAR), "{ai_type:?}");
    }
}

#[test]
fn a_pirate_successor_of_a_patrolling_interceptor_lead_leaves_without_picking() {
    // The patrol heads off (@0x8d132): the successor copies goal 1 and its
    // stellar, and its idle block reads it set (@0x8c472-0x8c477). The
    // lead has looked its follower over already (+0x90), so it patrols
    // rather than inspect it: a successor that copied goal 7 on itself
    // would end it within its own reach (@0x8f391).
    let lead = Npc {
        inspected: Some(ShipRef::Npc(FOLLOWER)),
        ..of_type(AiType::Interceptor, 4)
    };
    let successor = successor_after(lead, &sites());
    assert!(
        matches!(successor.goal, Goal::Land(_) | Goal::JumpOut),
        "{:?}",
        successor.goal
    );
    assert!(crate::ai::travel::is_set(&successor));
}

#[test]
fn a_pirate_successor_of_an_interceptor_lead_that_lost_sight_of_its_inspection_picks() {
    // The lost-sight tail's -1 (@0x8d3df).
    let mut traffic = Traffic::with_interval(1);
    traffic.add_npc(Npc {
        travel: Some(HELD),
        goal: Goal::Inspect(ShipRef::Npc(NpcId(2))),
        target: Some(ShipRef::Npc(NpcId(2))),
        ..of_type(AiType::Interceptor, 4)
    });
    traffic.add_npc(led_by(LEAD, 300));
    let mut hidden = crate::ai::fixture::cloaked(npc(0, ShipStats::new(FAST, &[])));
    hidden.state.position = Vec2::new(3000.0, 0.0);
    traffic.add_npc(hidden);
    tick_among(&mut traffic, &unlandable());
    assert_eq!(of(&traffic, LEAD).travel, None, "dropped for lost sight");
    destroy(&mut traffic, LEAD);
    tick(&mut traffic);
    assert_eq!(of(&traffic, FOLLOWER).goal, Goal::Land(STELLAR));
}

#[test]
fn a_pirate_successor_of_a_miner_lead_stopped_with_no_asteroids_picks() {
    // `_MinerAI`'s stop with no rock: +0x6c -1 (@0x8b2cb).
    let mut miner = of_type(AiType::WimpyTrader, 1);
    miner.miner = crate::ai::miner::MINES;
    let successor = successor_after(miner, &sites());
    assert_eq!(successor.goal, Goal::Land(STELLAR));
}

/// Two stellars: [`STELLAR`] near the centre, and [`HELD`] far off.
fn two_stellars() -> [LandingSite; 2] {
    [
        planet(STELLAR.0, 100.0, 0.0),
        planet(HELD.0, 3000.0, 3000.0),
    ]
}

/// [`led_by`] `lead`, a shuttling miner (`Flags3` 0x0002, no scoop).
fn miner_led_by(lead: NpcId) -> Npc {
    Npc {
        miner: crate::ai::miner::SCOOPS,
        ..led_by(lead, 300)
    }
}

/// A miner successor of `lead`, after the lead is destroyed and a tick
/// among `sites` with `slots` the slot stellars.
fn miner_successor_of(lead: Npc, sites: &[LandingSite], slots: &[LandingSite]) -> Npc {
    let mut traffic = Traffic::with_interval(1);
    traffic.add_npc(lead);
    traffic.add_npc(miner_led_by(LEAD));
    destroy(&mut traffic, LEAD);
    let govts = pirates();
    let world = World {
        govts: &govts,
        slot_stellars: slots,
        ..World::new(sites)
    };
    traffic.tick_in(&NovaAi::default(), world, &[], &mut Draws::default());
    of(&traffic, FOLLOWER).clone()
}

#[test]
fn a_miner_successor_lands_on_the_stellar_its_lead_held() {
    // `_MinerAI` reads the number (@0x8b36c), not the nearest.
    let lead = Npc {
        travel: Some(HELD),
        ..pirate(500)
    };
    let successor = miner_successor_of(lead, &two_stellars(), &[]);
    assert_eq!(successor.goal, Goal::Land(HELD));
}

#[test]
fn a_miner_successor_of_a_warship_lead_lands_where_that_lead_was_bound() {
    let lead = Npc {
        goal: Goal::Land(HELD),
        ..pirate(500)
    };
    let successor = miner_successor_of(lead, &two_stellars(), &[]);
    assert_eq!(successor.goal, Goal::Land(HELD));
    assert_eq!(successor.travel, Some(HELD));
}

#[test]
fn a_miner_successor_of_an_escort_lead_flies_to_the_stellar_its_slot_names() {
    // The escort holds its lead's slot, 6 (@0x83f09), which a miner reads
    // as stellar 134, another system's, flown to where it stands.
    const ESCORT: NpcId = NpcId(1);
    const MINER: NpcId = NpcId(2);
    let slots = [planet(134, -2000.0, 0.0)];
    let sites = two_stellars();
    let govts = pirates();
    let world = World {
        govts: &govts,
        slot_stellars: &slots,
        ..World::new(&sites)
    };
    let mut traffic = Traffic::with_interval(1);
    traffic.add_npc_at(pirate(900), 6);
    traffic.add_npc_at(led_by(LEAD, 500), 7);
    traffic.add_npc_at(miner_led_by(ESCORT), 8);
    traffic.tick_in(&NovaAi::default(), world, &[], &mut Draws::default());
    assert_eq!(
        of(&traffic, ESCORT).travel,
        Some(crate::ai::travel::of_slot(6)),
        "formation writes its slot"
    );
    destroy(&mut traffic, ESCORT);
    let before = of(&traffic, MINER).state.position;
    // It turns west first, at 3° a tick.
    for _ in 0..150 {
        traffic.tick_in(&NovaAi::default(), world, &[], &mut Draws::default());
    }
    let miner = of(&traffic, MINER);
    assert_eq!(miner.leader, None);
    assert_eq!(miner.goal, Goal::Land(StellarId(134)));
    assert!(
        miner.state.position.x < before.x - 100.0,
        "toward (-2000, 0): {:?}",
        miner.state
    );
}
