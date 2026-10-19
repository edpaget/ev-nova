//! Tests of the lost-sight search as the traffic's tick runs it (see
//! [`lost_sight`](crate::ai::lost_sight)): the timer each decision writes,
//! and the stop a searching ship flies in place of its fight.

use std::collections::BTreeSet;

use super::{Traffic, World};
use crate::ai::lost_sight::{LostSight, SearchTimer};
use crate::ai::pursuit::{Kept, Pursuit};
use crate::ai::{Behaviour, Decision, Goal, PlayerSide, Surroundings};
use crate::chance::Chance;
use crate::combat::ShipRef;
use crate::geometry::Vec2;
use crate::stats::ShipStats;
use crate::traffic::fixture::npc;
use crate::traffic::npc::Npc;
use crate::traffic::plunder_dock::{DockSub, Docking};
use nova_sim_base::fixture::{FAST, Ranged};

/// Every NPC takes this decision.
#[derive(Debug)]
struct Fixed(Decision);

impl Behaviour for Fixed {
    fn decide(&self, _npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
        self.0.goal
    }

    fn decision(&self, _npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Decision {
        self.0
    }
}

/// An attack on the player through the block, writing `lost_sight`.
fn attack(lost_sight: Option<LostSight>) -> Fixed {
    Fixed(Decision {
        goal: Goal::Attack(ShipRef::Player),
        lost_sight,
        ..Decision::default()
    })
}

/// The player at `at`, at rest facing down.
fn player_at(at: Vec2) -> PlayerSide {
    let mut side = crate::ai::fixture::player(at.x, at.y);
    side.state.heading = 180.0;
    side
}

/// Traffic of one afterburner attacker of the player at the centre
/// facing up, its timer `timer`, deciding every tick.
fn attacker(timer: SearchTimer) -> Traffic {
    let mut traffic = Traffic::with_interval(1);
    let mut ship = npc(1, ShipStats::new(FAST, &[]));
    ship.afterburner = true;
    ship.goal = Goal::Attack(ShipRef::Player);
    ship.lost_sight = timer;
    traffic.npcs = vec![ship];
    traffic
}

/// `traffic` one tick on, every NPC deciding as `behaviour` does, the
/// player up and a little left of the centre.
fn tick(traffic: &mut Traffic, behaviour: &Fixed) {
    let world = World {
        player: Some(player_at(Vec2::new(-100.0, -400.0))),
        ..World::new(&[])
    };
    traffic.tick_holding(
        behaviour,
        world,
        &[],
        &BTreeSet::new(),
        &mut Ranged::of(&[]),
    );
}

#[test]
fn a_searching_decision_sets_the_timer_and_seen_or_gave_up_clears_it() {
    let mut traffic = attacker(SearchTimer::Unset);
    tick(&mut traffic, &attack(Some(LostSight::Searching(150))));
    assert_eq!(traffic.npcs()[0].lost_sight, SearchTimer::Searching(150));
    tick(&mut traffic, &attack(Some(LostSight::Seen)));
    assert_eq!(traffic.npcs()[0].lost_sight, SearchTimer::Unset);
    let mut traffic = attacker(SearchTimer::Searching(1));
    let gave_up = Fixed(Decision {
        goal: Goal::Idle,
        clears_target: true,
        lost_sight: Some(LostSight::GaveUp),
        ..Decision::default()
    });
    tick(&mut traffic, &gave_up);
    assert_eq!(traffic.npcs()[0].lost_sight, SearchTimer::Unset);
}

#[test]
fn a_decision_without_the_block_leaves_the_timer_as_it_was() {
    let mut traffic = attacker(SearchTimer::Searching(40));
    let fleeing = Fixed(Decision {
        goal: Goal::Flee(ShipRef::Player),
        ..Decision::default()
    });
    tick(&mut traffic, &fleeing);
    assert_eq!(traffic.npcs()[0].lost_sight, SearchTimer::Searching(40));
}

#[test]
fn a_searching_ship_brakes_as_a_waiting_one_does() {
    let moving = |traffic: &mut Traffic| {
        traffic.npcs[0].state.velocity = Vec2::new(-1.5, 2.0);
    };
    let mut searching = attacker(SearchTimer::Searching(10));
    moving(&mut searching);
    tick(&mut searching, &attack(Some(LostSight::Searching(9))));
    let mut waiting = attacker(SearchTimer::Unset);
    moving(&mut waiting);
    let wait = Fixed(Decision {
        goal: Goal::Wait,
        ..Decision::default()
    });
    tick(&mut waiting, &wait);
    let (searched, waited) = (&searching.npcs()[0], &waiting.npcs()[0]);
    assert_eq!(searched.goal, Goal::Attack(ShipRef::Player));
    assert_eq!(searched.state, waited.state, "sub 1, a stop");
}

#[test]
fn a_search_drops_a_kept_pursuit_and_a_docking() {
    let mut traffic = attacker(SearchTimer::Searching(10));
    traffic.npcs[0].pursuit = Pursuit {
        kept: Some(Kept::Rush),
        frame: None,
    };
    traffic.npcs[0].docking = Some(Docking {
        ship: ShipRef::Player,
        sub: DockSub::Approach,
    });
    tick(&mut traffic, &attack(Some(LostSight::Searching(9))));
    let ship = &traffic.npcs()[0];
    assert_eq!(ship.pursuit, Pursuit::default());
    assert_eq!(ship.docking, None);
}

#[test]
fn a_ship_whose_target_is_seen_again_flies_its_fight() {
    let mut traffic = attacker(SearchTimer::Searching(10));
    tick(&mut traffic, &attack(Some(LostSight::Seen)));
    let ship = &traffic.npcs()[0];
    assert_eq!(ship.lost_sight, SearchTimer::Unset);
    assert_eq!(ship.pursuit.kept, Some(Kept::Rush), "its pursuit ran");
    assert!(ship.pursuit.frame.is_some(), "with this tick's frame");
}

#[test]
fn a_searching_ship_whose_ai_wait_runs_skips_its_high_level() {
    // The AI wait skips the whole handler (@0x8d4b9-0x8d4d2), the
    // search's stop with it: its pursuit and docking stand.
    let mut traffic = attacker(SearchTimer::Searching(10));
    let kept = Pursuit {
        kept: Some(Kept::Rush),
        frame: None,
    };
    let docking = Some(Docking {
        ship: ShipRef::Player,
        sub: DockSub::Approach,
    });
    traffic.npcs[0].pursuit = kept;
    traffic.npcs[0].docking = docking;
    traffic.npcs[0].ai_wait = 5;
    tick(&mut traffic, &attack(None));
    let ship = &traffic.npcs()[0];
    assert_eq!(ship.lost_sight, SearchTimer::Searching(10));
    assert_eq!(ship.pursuit, kept);
    assert_eq!(ship.docking, docking);
}

/// A ship of the fixture at berth `berth` of `traffic`, as a spawn brings
/// it in.
fn brought_in(traffic: &mut Traffic, berth: u32) -> SearchTimer {
    let id = traffic.add_npc_at(npc(1, ShipStats::new(FAST, &[])), berth);
    traffic
        .npcs()
        .iter()
        .find(|ship| ship.id == id)
        .map(|ship| ship.lost_sight)
        .expect("brought in")
}

#[test]
fn a_new_ship_takes_its_berths_last_search_timer() {
    let mut traffic = Traffic::with_interval(1);
    traffic.add_npc_at(npc(1, ShipStats::new(FAST, &[])), 4);
    traffic.npcs[0].goal = Goal::Attack(ShipRef::Player);
    tick(&mut traffic, &attack(Some(LostSight::Searching(40))));
    assert_eq!(traffic.npcs()[0].lost_sight, SearchTimer::Searching(40));
    traffic.npcs.clear();
    assert_eq!(
        brought_in(&mut traffic, 4),
        SearchTimer::Carried(40),
        "the count, with no search under way"
    );
    let mut seen = Traffic::with_interval(1);
    seen.add_npc_at(npc(1, ShipStats::new(FAST, &[])), 4);
    seen.npcs[0].goal = Goal::Attack(ShipRef::Player);
    tick(&mut seen, &attack(Some(LostSight::Seen)));
    seen.npcs.clear();
    assert_eq!(brought_in(&mut seen, 4), SearchTimer::Unset, "a seen -1");
}

#[test]
fn a_ship_in_a_fresh_berth_starts_with_a_zero_timer() {
    let mut traffic = Traffic::with_interval(1);
    assert_eq!(brought_in(&mut traffic, 9), SearchTimer::Carried(0));
    let id = traffic.add_npc(npc(1, ShipStats::new(FAST, &[])));
    let added = traffic.npcs().iter().find(|ship| ship.id == id);
    assert_eq!(
        added.map(|ship| ship.lost_sight),
        Some(SearchTimer::Carried(0))
    );
}

#[test]
fn by_the_other_reading_every_new_ship_starts_unset() {
    let mut traffic = Traffic::with_interval(1);
    traffic.set_reused_slot_search(crate::rulebook::RuleSource::Bible);
    assert_eq!(
        traffic.reused_slot_search(),
        crate::rulebook::RuleSource::Bible
    );
    traffic.add_npc_at(npc(1, ShipStats::new(FAST, &[])), 4);
    traffic.npcs[0].goal = Goal::Attack(ShipRef::Player);
    tick(&mut traffic, &attack(Some(LostSight::Searching(40))));
    traffic.npcs.clear();
    assert_eq!(brought_in(&mut traffic, 4), SearchTimer::Unset);
    assert_eq!(brought_in(&mut traffic, 9), SearchTimer::Unset);
}
