//! The odds cache through the traffic and the AI (`_DoPlayGameWork`'s
//! odds pass @0x442b0-0x44336 over +0xc914; see
//! `nova_sim::traffic::odds_cache`): an outnumbered warship of a
//! retreating government holds while its slot reads -1 from its spawn,
//! and flees on the first think after the pass reaches its berth, one
//! berth a tick; by the other reading of `odds_cache` every warship flees
//! on its first think.

use std::collections::BTreeSet;

use nova_sim::catalog::{GovtId, GovtRecord};
use nova_sim::combat::ShipRef;
use nova_sim::combat::armament::Armament;
use nova_sim::combat::weapon::WeaponSpec;
use nova_sim::govt::WARSHIPS_RETREAT;
use nova_sim::stats::ShipStats;
use nova_sim::traffic::npc::AiType;
use nova_sim::{
    Goal, Governments, NovaAi, Npc, NpcId, RuleKey, RuleSource, Rulebook, Traffic, World,
};
use nova_sim_base::fixture::{Draws, FAST};
use nova_sim_catalog::fixture::{govt, weapon};

/// The warships' government (class 0, retreating outnumbered, `MaxOdds`
/// 1) and the pirates' (class 1), at war.
fn govts() -> Governments {
    Governments::new([
        GovtRecord {
            flags: WARSHIPS_RETREAT,
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

/// A ship of `govt` and `strength`, attacking `quarry`, armed with a
/// harmless gun of unlimited ammunition.
fn ship(govt: i16, strength: f32, quarry: u32) -> Npc {
    let mut npc = nova_sim::traffic::fixture::npc(0, ShipStats::new(FAST, &[]));
    npc.ai_type = AiType::Warship;
    npc.govt = Some(GovtId(govt));
    npc.hull.strength = strength;
    npc.aggression = 3;
    npc.goal = Goal::Attack(ShipRef::Npc(NpcId(quarry)));
    npc.target = Some(ShipRef::Npc(NpcId(quarry)));
    npc.provoked = 1.0;
    npc.armament = Armament::new([(WeaponSpec::new(&weapon(127)), 1)]);
    npc
}

/// Two warships of strength 100 at berths 1 and 2, their shields below
/// half, each attacked by a pirate of 300 (berths 3 and 4): odds of 3
/// against each, over its `MaxOdds` of 1.
fn traffic() -> Traffic {
    let mut traffic = Traffic::with_interval(1);
    for (strength, govt, quarry) in [
        (100.0, 128, 2),
        (100.0, 128, 3),
        (300.0, 129, 0),
        (300.0, 129, 1),
    ] {
        let mut npc = ship(govt, strength, quarry);
        npc.reserves.shield.now = npc.reserves.shield.max * 0.4;
        traffic.add_npc(npc);
    }
    traffic
}

/// The tick on which each warship first flees, reading the odds as
/// `source` says, within 5 ticks; the ships held in place.
fn first_flights(source: RuleSource) -> [Option<u32>; 2] {
    let rules = Rulebook::default().with_override(RuleKey::OddsCache, source);
    let ai = NovaAi::from_rulebook(&rules);
    let govts = govts();
    let world = World {
        govts: &govts,
        odds_cache: rules.source_for(RuleKey::OddsCache),
        ..World::new(&[])
    };
    let still: BTreeSet<NpcId> = (0..4).map(NpcId).collect();
    let mut traffic = traffic();
    let mut draws = Draws::of(&[0; 256]);
    let mut fled = [None; 2];
    for tick in 1..=5 {
        traffic.tick_holding(&ai, world, &[], &still, &mut draws);
        for (warship, first) in fled.iter_mut().enumerate() {
            if first.is_none() && matches!(traffic.npcs()[warship].goal, Goal::Flee(_)) {
                *first = Some(tick);
            }
        }
    }
    fled
}

#[test]
fn by_the_engine_each_warship_flees_on_the_think_after_the_pass_reaches_its_berth() {
    assert_eq!(
        first_flights(RuleSource::Engine),
        [Some(2), Some(3)],
        "-1 at the first think; berth 1 refreshed on tick 1, berth 2 on tick 2"
    );
}

#[test]
fn by_the_other_reading_every_warship_flees_on_its_first_think() {
    assert_eq!(first_flights(RuleSource::Bible), [Some(1), Some(1)]);
}
