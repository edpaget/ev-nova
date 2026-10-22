//! The rulebook's planet-type readings reach the session's fight
//! (stellar-fidelity phase 2; see `nova_sim::combat::planet_type`).

use crate::catalog::{HullRecord, StellarId, StellarLife, StockWeapon, WeaponId, WeaponRecord};
use crate::chance::NeverFires;
use crate::combat::Rules;
use crate::combat::armament::{MOD_WEAPON, Trigger};
use crate::combat::weapon::{DETONATES, PLANET_TYPE};
use crate::control::{Bit, Script};
use crate::rulebook::{RuleKey, RuleSource, Rulebook};
use crate::session::Session;
use crate::session::tests::{FIRE, armed, facing_an_npc};
use crate::testkit::{FakePilotCatalog, catalog, hull, life, outfit, weapon};

/// `catalog`'s session by `rules`, its NPC 100 pixels above the player
/// and planet-type, at rest.
fn with_a_planet_type_npc(catalog: &crate::testkit::FakePilotCatalog, rules: Rulebook) -> Session {
    let mut session = facing_an_npc(catalog, 180).with_rules(rules);
    session.space.traffic.npcs_mut()[0].hull.planet_type = true;
    session
}

/// Whether the NPC's shield is still full.
fn unhurt(session: &Session) -> bool {
    let shield = session.npcs()[0].reserves.shield;
    shield.now >= shield.max
}

#[test]
fn the_planet_blasts_rule_decides_whether_a_planet_type_expiry_blasts_a_planet_type_ship() {
    // The player's planet-type flak detonates 80 pixels up, short of the
    // NPC 100 up, which its 60-pixel blast reaches.
    let mut catalog = armed();
    catalog.weapons[0].count = 4;
    catalog.weapons[0].blast_radius = 60;
    catalog.weapons[0].flags = DETONATES;
    catalog.weapons[0].flags2 = PLANET_TYPE;
    let blasted = |source| fired_on(&catalog, RuleKey::PlanetBlasts, source);
    assert!(!blasted(RuleSource::Engine), "@0x35fb9");
    assert!(blasted(RuleSource::Bible));
}

/// Whether the player, firing `catalog`'s weapon once at its planet-type
/// NPC by `source` of `key`, hurts it over the next 8 ticks.
fn fired_on(catalog: &crate::testkit::FakePilotCatalog, key: RuleKey, source: RuleSource) -> bool {
    let rules = Rulebook::default().with_override(key, source);
    let mut session = with_a_planet_type_npc(catalog, rules);
    session.hold_trigger(FIRE);
    session.tick_combat(Rules::default(), &mut NeverFires);
    session.hold_trigger(Trigger::default());
    for _ in 0..8 {
        session.tick_combat(Rules::default(), &mut NeverFires);
    }
    !unhurt(&session)
}

#[test]
fn the_beam_planet_flag_rule_decides_which_bit_lets_a_beam_hit_a_planet_type_ship() {
    // The player's beam, of `Flags2` 0x0400 only, reaches the NPC.
    let mut catalog = armed();
    catalog.weapons[0].guidance = 0;
    catalog.weapons[0].beam_length = 200;
    catalog.weapons[0].count = 3;
    catalog.weapons[0].flags2 = PLANET_TYPE;
    let hit = |source| fired_on(&catalog, RuleKey::BeamPlanetFlag, source);
    assert!(!hit(RuleSource::Engine), "@0x30d34-0x30d82 reads Flags");
    assert!(hit(RuleSource::Bible));
}

// Beams on stellars.

/// The catalog with 128 back after 2 days, running `^b5` as it is shot
/// down, and the player's ship, starting over 128 (of `Strength` 100),
/// carrying a planet-type beam held 30 ticks, of 30 mass and 20 energy
/// damage a tick.
fn beaming() -> FakePilotCatalog {
    let beam = WeaponRecord {
        guidance: 0,
        beam_length: 200,
        count: 30,
        reload: 100,
        mass_dmg: 30,
        energy_dmg: 20,
        flags2: PLANET_TYPE,
        explod_type: 2,
        ..weapon(128)
    };
    FakePilotCatalog {
        weapons: vec![beam],
        hulls: vec![HullRecord {
            weapons: vec![StockWeapon {
                weapon: WeaponId(128),
                count: 1,
                ammo: 0,
            }],
            ..hull(128)
        }],
        outfits: vec![outfit(250, &[(MOD_WEAPON, 128)])],
        ..catalog().living([StellarLife {
            on_destroy: Script::parse("^b5"),
            ..life(128, 2)
        }])
    }
}

/// Whether, by `source` of [`RuleKey::BeamStellars`], the player's beam
/// held over 128 for 8 ticks shoots it down, and whether its `OnDestroy`
/// ran an odd number of times.
fn beamed_down(source: RuleSource) -> (bool, bool) {
    let catalog = beaming();
    let rules = Rulebook::default().with_override(RuleKey::BeamStellars, source);
    let mut session = Session::start(&catalog).expect("starts").with_rules(rules);
    session.hold_trigger(FIRE);
    session.tick_combat(Rules::default(), &mut NeverFires);
    session.hold_trigger(Trigger::default());
    for _ in 0..8 {
        session.tick_combat(Rules::default(), &mut NeverFires);
    }
    let destroyed = session
        .galaxy()
        .overlay()
        .stellar(StellarId(128))
        .destroyed
        .is_some();
    let ran = session.control_bit(Bit::new(5).expect("a bit"));
    (destroyed, ran)
}

#[test]
fn the_beam_stellars_rule_decides_whether_a_planet_type_beam_shoots_a_stellar_down() {
    assert_eq!(
        beamed_down(RuleSource::Engine),
        (false, false),
        "@0x30295-0x31e1f"
    );
    assert_eq!(
        beamed_down(RuleSource::Bible),
        (true, true),
        "shot down by its third tick, its OnDestroy run once"
    );
}
