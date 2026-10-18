//! The lost-sight search through the traffic (`_HighLevelAIHandler`
//! @0x8d6ef-0x8d8ae; see `nova_sim::ai::lost_sight`): a warship whose
//! target cloaks stops and holds its attack for its search, 100-199
//! ticks, then drops the target, and takes the fight up again if the
//! target reappears first; a trader whose target cloaks leaves, or stops
//! when it can't jump.

use std::collections::BTreeSet;

use nova_sim::catalog::{GovtId, GovtRecord};
use nova_sim::cloak::{CLOAK_RETREATING, Cloak, FADE_FULL};
use nova_sim::combat::ShipRef;
use nova_sim::combat::armament::Armament;
use nova_sim::combat::weapon::WeaponSpec;
use nova_sim::geometry::Vec2;
use nova_sim::stats::ShipStats;
use nova_sim::traffic::npc::AiType;
use nova_sim::{Goal, Governments, NovaAi, Npc, NpcId, Rulebook, Traffic, World};
use nova_sim_base::fixture::{Draws, FAST};
use nova_sim_catalog::fixture::{govt, weapon};

/// The hunter, the traffic's first ship.
const HUNTER: ShipRef = ShipRef::Npc(NpcId(0));
/// Its quarry, the second.
const QUARRY: ShipRef = ShipRef::Npc(NpcId(1));

/// The hunters' government and their quarry's, at war.
fn govts() -> Governments {
    Governments::new([
        GovtRecord {
            enemies: [1, -1, -1, -1],
            classes: [0, -1, -1, -1],
            ..govt(128)
        },
        GovtRecord {
            enemies: [0, -1, -1, -1],
            classes: [1, -1, -1, -1],
            ..govt(129)
        },
    ])
}

/// A hunter of `ai_type` at the centre, attacking NPC 2, and NPC 2, its
/// quarry, 400 above, fleeing it cloaked: it has a cloak that its ship
/// type turns on while it retreats, fully faded in.
fn traffic(ai_type: AiType) -> Traffic {
    let mut hunter = nova_sim::traffic::fixture::npc(1, ShipStats::new(FAST, &[]));
    hunter.ai_type = ai_type;
    hunter.govt = Some(GovtId(128));
    hunter.goal = Goal::Attack(QUARRY);
    hunter.target = Some(QUARRY);
    hunter.provoked = 1.0;
    hunter.armament = Armament::new([(WeaponSpec::new(&weapon(127)), 1)]);
    hunter.hull.strength = 100.0;
    hunter.aggression = 2;
    let mut quarry = nova_sim::traffic::fixture::npc(2, ShipStats::new(FAST, &[]));
    quarry.govt = Some(GovtId(129));
    quarry.hull.strength = 100.0;
    quarry.state.position = Vec2::new(0.0, -400.0);
    quarry.goal = Goal::Flee(HUNTER);
    quarry.target = Some(HUNTER);
    quarry.provoked = 1.0;
    quarry.cloak_gear.device = true;
    quarry.cloak_gear.flags2 = CLOAK_RETREATING;
    quarry.cloak = Cloak {
        fade: FADE_FULL,
        direction: 0,
    };
    let mut traffic = Traffic::with_interval(1);
    assert_eq!(ShipRef::Npc(traffic.add_npc(hunter)), HUNTER);
    assert_eq!(ShipRef::Npc(traffic.add_npc(quarry)), QUARRY);
    traffic
}

/// `traffic` one tick on, the quarry kept in place.
fn tick(traffic: &mut Traffic, govts: &Governments, draws: &mut Draws) {
    let ai = NovaAi::from_rulebook(&Rulebook::default());
    let world = World {
        govts,
        ..World::new(&[])
    };
    let still = BTreeSet::from([NpcId(1)]);
    traffic.tick_holding(&ai, world, &[], &still, draws);
}

fn hunter(traffic: &Traffic) -> &Npc {
    &traffic.npcs()[0]
}

#[test]
fn a_warship_whose_target_cloaks_holds_for_its_search_then_drops_it() {
    let govts = govts();
    let mut traffic = traffic(AiType::Warship);
    let mut draws = Draws::of(&[0; 4]);
    for tick_index in 0..100 {
        tick(&mut traffic, &govts, &mut draws);
        let hunter = hunter(&traffic);
        assert_eq!(hunter.goal, Goal::Attack(QUARRY), "tick {tick_index}");
        assert_eq!(hunter.lost_sight, Some(100 - tick_index));
        assert_eq!(hunter.state.position, Vec2::ZERO, "it does not close in");
    }
    tick(&mut traffic, &govts, &mut draws);
    let hunter = hunter(&traffic);
    assert_eq!(hunter.goal, Goal::Idle, "the 101st tick");
    assert_eq!(hunter.target, None);
    assert_eq!(hunter.lost_sight, None);
    assert_eq!(draws.asked, [100], "one roll of the search's length");
}

#[test]
fn a_warship_whose_target_reappears_mid_search_resumes_the_attack() {
    let govts = govts();
    let mut traffic = traffic(AiType::Warship);
    let mut draws = Draws::of(&[0; 4]);
    for _ in 0..10 {
        tick(&mut traffic, &govts, &mut draws);
    }
    assert_eq!(hunter(&traffic).lost_sight, Some(91));
    traffic.npcs_mut()[1].cloak_gear.device = false;
    let mut ticks = 0;
    while hunter(&traffic).lost_sight.is_some() {
        tick(&mut traffic, &govts, &mut draws);
        ticks += 1;
        assert!(ticks < 60, "it never saw its target again");
    }
    let seen = hunter(&traffic);
    assert_eq!(seen.goal, Goal::Attack(QUARRY));
    assert_eq!(seen.target, Some(QUARRY));
    assert!(seen.pursuit.frame.is_some(), "its pursuit runs again");
    for _ in 0..30 {
        tick(&mut traffic, &govts, &mut draws);
    }
    let fighting = hunter(&traffic);
    assert_eq!(fighting.goal, Goal::Attack(QUARRY));
    assert!(
        fighting.state.position.y < 0.0,
        "it closes on its target, above: {:?}",
        fighting.state.position
    );
}

/// The goal a brave trader of class `Fuel` `class_fuel` takes on its
/// first tick once its target cloaks.
fn brave_trader_with(class_fuel: i16) -> Goal {
    let govts = govts();
    let mut traffic = traffic(AiType::BraveTrader);
    traffic.npcs_mut()[0].class_fuel = class_fuel;
    tick(&mut traffic, &govts, &mut Draws::of(&[]));
    let hunter = hunter(&traffic);
    assert_eq!(hunter.lost_sight, None, "a trader's timer is untouched");
    hunter.goal
}

#[test]
fn a_brave_trader_whose_target_cloaks_leaves_or_stops() {
    assert_eq!(brave_trader_with(100), Goal::JumpOut);
    assert_eq!(brave_trader_with(99), Goal::Wait, "it can't jump");
}
