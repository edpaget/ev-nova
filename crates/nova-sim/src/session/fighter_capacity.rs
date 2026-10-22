//! Tests of a fighter's capacities in flight, by the rulebook's
//! [`RuleKey::FighterCapacity`]: a fighter, the player's or an NPC
//! carrier's, holds 1.333 times its class's shield and armour and
//! recharges its shield 1.333 times as fast, and the readers of its
//! capacities (the disable threshold, the self-repair) read that (see
//! `nova_sim_ai::traffic::fighter_capacity`).

#![allow(clippy::float_cmp)]

use super::Session;
use super::fighters::tests::{
    VIPER_FIELDS, carrier, carrying, earlier_escorts, fight, fleet_npcs, flying_by, launch, out,
    pilot, pirate, raiding_ai, vipers,
};
use crate::bay::fighter_capacity;
use crate::chance::NeverFires;
use crate::combat::Rules;
use crate::combat::hull::Condition;
use crate::reserves::Reserves;
use crate::rulebook::{RuleKey, RuleSource, Rulebook};
use crate::stats::ShipStats;
use crate::testkit::{Draws, FakePilotCatalog};
use crate::traffic::npc::Npc;

/// The Viper's class's shield and armour: 30 and 45.
const CLASS: (f32, f32) = (30.0, 45.0);
/// A fighter's by the engine: 30 and 45 × 1.333, taken in doubles.
const SCALED: (f32, f32) = (39.99, 59.985);

/// The rules by `capacity`'s reading of [`RuleKey::FighterCapacity`].
fn by(capacity: RuleSource) -> Rulebook {
    Rulebook::default().with_override(RuleKey::FighterCapacity, capacity)
}

/// The shield and armour gauges' (now, most).
fn gauges(npc: &Npc) -> [(f32, f32); 2] {
    let Reserves { shield, armor, .. } = npc.reserves;
    [(shield.now, shield.max), (armor.now, armor.max)]
}

/// Full gauges of (shield, armour).
fn full((shield, armor): (f32, f32)) -> [(f32, f32); 2] {
    [(shield, shield), (armor, armor)]
}

/// The player's session by `capacity`, two Vipers aboard, one launched.
fn players_launched(catalog: &FakePilotCatalog, capacity: RuleSource) -> Session {
    let mut session = flying_by(catalog, pilot(catalog, 2, Vec::new()), by(capacity));
    launch(&mut session);
    session
}

#[test]
fn the_players_launched_fighter_holds_1_333_times_its_classs_full_by_the_engine() {
    let catalog = carrying();
    assert_eq!(
        (
            ShipStats::new(VIPER_FIELDS, &[]).shield,
            ShipStats::new(VIPER_FIELDS, &[]).armor
        ),
        CLASS
    );
    let session = players_launched(&catalog, RuleSource::Engine);
    let fighter = &fleet_npcs(&session)[0];
    assert_eq!(gauges(fighter), full(SCALED));
    assert_eq!(
        (fighter.stats.shield, fighter.stats.armor),
        CLASS,
        "its stats its class's"
    );
    assert_eq!(
        session.pilot().escorts()[0].reserves,
        fighter.reserves,
        "its record"
    );
    let session = players_launched(&catalog, RuleSource::Bible);
    assert_eq!(gauges(&fleet_npcs(&session)[0]), full(CLASS));
}

#[test]
fn an_npc_carriers_launched_fighter_holds_1_333_times_its_classs_full_by_the_engine() {
    let catalog = carrying();
    for (capacity, held) in [(RuleSource::Engine, SCALED), (RuleSource::Bible, CLASS)] {
        let rules = earlier_escorts()
            .with_override(RuleKey::FighterLaunch, RuleSource::Engine)
            .with_override(RuleKey::FighterCapacity, capacity);
        let mut session = flying_by(&catalog, pilot(&catalog, 0, Vec::new()), rules);
        let lead = pirate(&mut session, 2000.0, 2000.0);
        carrier(&mut session, lead, 2);
        fight(&mut session, &catalog, &raiding_ai());
        let launched = vipers(&session);
        assert_eq!(launched.len(), 1, "{capacity:?}");
        assert_eq!(gauges(&launched[0]), full(held), "{capacity:?}");
    }
}

/// [`carrying`] with the Viper recharging its shield 0.125 a tick and its
/// armour 0.02.
fn recharging() -> FakePilotCatalog {
    let mut catalog = carrying();
    let fields = &mut catalog.ship_records[1].fields;
    fields.shield_rech = 125;
    fields.armor_rech = 20;
    catalog
}

#[test]
fn a_fighters_shield_recharges_1_333_times_as_fast_and_its_armour_at_its_classs_rate() {
    let catalog = recharging();
    for (capacity, shield) in [
        (
            RuleSource::Engine,
            fighter_capacity(0.125, RuleSource::Engine),
        ),
        (RuleSource::Bible, 0.125),
    ] {
        let mut session = players_launched(&catalog, capacity);
        let id = fleet_npcs(&session)[0].id;
        let fighter = session.space.npc_mut(id).expect("out");
        fighter.reserves.shield.now = 0.0;
        fighter.reserves.armor.now = 40.0;
        session.tick_combat(Rules::default(), &mut NeverFires);
        let fighter = session.space.npc(id).expect("out");
        assert_eq!(fighter.reserves.shield.now, shield, "{capacity:?}");
        assert_eq!(fighter.reserves.armor.now, 40.02, "{capacity:?}");
    }
    assert_eq!(fighter_capacity(0.125, RuleSource::Engine), 0.166_625);
}

#[test]
fn a_fighters_disable_threshold_reads_its_scaled_armour() {
    // A third of the class's armour is 15, a third of 59.985 is 19.995:
    // at 17 the engine's fighter is disabled and leaves the fleet.
    let catalog = carrying();
    for (capacity, disabled) in [(RuleSource::Engine, true), (RuleSource::Bible, false)] {
        let mut session = players_launched(&catalog, capacity);
        let id = fleet_npcs(&session)[0].id;
        session.space.npc_mut(id).expect("out").reserves.armor.now = 17.0;
        session.tick_combat(Rules::default(), &mut NeverFires);
        let condition = session.space.npc(id).expect("there").condition;
        assert_eq!(condition == Condition::Disabled, disabled, "{capacity:?}");
        assert_eq!(
            session.pilot().escorts().is_empty(),
            disabled,
            "{capacity:?}"
        );
    }
}

#[test]
fn a_carried_fighter_placed_in_the_system_holds_its_scaled_capacities_keeping_its_record() {
    let catalog = carrying();
    for (capacity, (shield, armor)) in [(RuleSource::Engine, SCALED), (RuleSource::Bible, CLASS)] {
        let mut session = flying_by(&catalog, pilot(&catalog, 0, vec![out()]), by(capacity));
        let placed = session.space.escorts.fleet[0].expect("placed");
        session.space.traffic.remove(placed);
        session.space.escorts.fleet[0] = None;
        let record = &mut session.pilot.escorts[0].reserves;
        record.shield.now = 20.0;
        record.armor.now = 40.0;
        let id = session
            .place_escort(0, &mut NeverFires)
            .expect("placed again");
        assert_eq!(
            gauges(session.space.npc(id).expect("placed")),
            [(20.0, shield), (40.0, armor)],
            "{capacity:?}"
        );
    }
}

#[test]
fn a_lost_fighter_repairing_itself_reaches_its_share_of_its_scaled_armour() {
    // Lost to the fleet disabled, it keeps its carrier (the original's AI
    // type 5, `_DamageShip` @0x3afdd clearing only its parent), so its
    // repair reads the scaled capacity (`_HandleShip` @0x33a73, @0x33aaa).
    let mut catalog = carrying();
    catalog.ship_records[1].defaults = vec![(crate::catalog::OutfitId(437), 1)];
    catalog.outfits.push(crate::testkit::outfit(
        437,
        &[(crate::self_repair::MOD_REPAIR, 0)],
    ));
    for (capacity, armor) in [(RuleSource::Engine, SCALED.1), (RuleSource::Bible, CLASS.1)] {
        let mut session = players_launched(&catalog, capacity);
        let id = fleet_npcs(&session)[0].id;
        assert!(session.space.npc(id).expect("out").self_repair);
        session.space.npc_mut(id).expect("out").reserves.armor.now = 1.0;
        session.tick_combat(Rules::default(), &mut NeverFires);
        assert!(session.pilot().escorts().is_empty(), "lost");
        session.self_repair(Rules::default(), &mut Draws::of(&[0]));
        let npc = session.space.npc(id).expect("there");
        assert_eq!(
            npc.reserves.armor.now,
            crate::self_repair::repaired_armor(armor, npc.hull.tough),
            "{capacity:?}"
        );
        assert_eq!(session.pilot().escorts().len(), 1, "rejoined");
        assert_eq!(gauges(npc)[1].1, armor, "{capacity:?}");
    }
}
