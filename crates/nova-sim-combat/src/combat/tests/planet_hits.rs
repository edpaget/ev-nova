//! The fight's planet-type rules (stellar-fidelity phase 2, see the
//! [`planet_type`](crate::combat::planet_type) module): a planet-type
//! weapon's shots, blasts and beams against planet-type and ordinary
//! ships.

use super::*;
use crate::combat::planet_type::{BeamPlanetFlag, PlanetBlasts};
use crate::combat::weapon::DETONATES;

/// `ship`, its class planet-type.
fn planet(ship: Ship) -> Ship {
    let mut ship = ship;
    ship.hull.planet_type = true;
    ship
}

/// Whether `ship` has taken anything off its 10 shield and 30 armour.
fn hurt(ship: &Ship) -> bool {
    left(ship) != (10.0, 30.0)
}

/// A fires `record` once to the right at B, 100 pixels off, with D 50
/// pixels behind B; then the fight goes on for 12 ticks. Whether B and D
/// were hurt.
fn through(
    record: WeaponRecord,
    b: impl Fn(Ship) -> Ship,
    d: impl Fn(Ship) -> Ship,
) -> (bool, bool) {
    through_in(Combat::default(), record, b, d)
}

/// As [`through`], in `combat`.
fn through_in(
    mut combat: Combat,
    record: WeaponRecord,
    b: impl Fn(Ship) -> Ship,
    d: impl Fn(Ship) -> Ship,
) -> (bool, bool) {
    let mut ships = [
        Ship::at(1, 0.0, 0.0).armed(record),
        b(Ship::at(2, 100.0, 0.0)),
        d(Ship::at(4, 150.0, 0.0)),
    ];
    for _ in 0..12 {
        tick(&mut combat, &mut ships, &NovaDisable);
        ships[0].trigger = QUIET;
    }
    (hurt(&ships[1]), hurt(&ships[2]))
}

#[test]
fn a_planet_type_shot_passes_through_an_ordinary_ship_unspent_and_hits_a_planet_type_one() {
    assert_eq!(
        through(planet_gun(), |ship| ship, planet),
        (false, true),
        "through B to D"
    );
    assert_eq!(
        through(planet_gun(), planet, |ship| ship),
        (true, false),
        "B, planet-type, takes it"
    );
}

#[test]
fn a_planet_type_shot_passes_an_ordinary_ship_to_strike_the_stellar_behind_it() {
    let mut stellars = [stellar_at(100.0, 0.0, 20.0, 100)];
    let (_, ships, _, strikes) = shoot_once(
        planet_gun(),
        vec![Ship::at(2, 70.0, 0.0)],
        &mut stellars,
        &Arsenal::default(),
        8,
    );
    assert!(!hurt(&ships[1]), "passed through");
    assert_eq!(strikes, [strike(14, false)]);
}

#[test]
fn an_ordinary_shot_passes_through_a_planet_type_ship_unspent() {
    assert_eq!(through(blaster(), planet, |ship| ship), (false, true));
    assert_eq!(through(blaster(), |ship| ship, planet), (true, false));
}

#[test]
fn an_ordinary_shot_passes_through_the_player_when_its_class_is_planet_type() {
    // B fires at the player in its path, C behind it.
    let mut combat = Combat::default();
    let player = Ship {
        id: ShipRef::Player,
        fleet: ShipRef::Player,
        slot: Slot::Player,
        ..Ship::at(0, 100.0, 0.0)
    };
    let mut ships = [
        planet(player),
        Ship::at(2, 0.0, 0.0).armed(blaster()),
        Ship::at(3, 150.0, 0.0),
    ];
    for _ in 0..12 {
        tick(&mut combat, &mut ships, &NovaDisable);
        ships[1].trigger = QUIET;
    }
    assert!(!hurt(&ships[0]), "the player, planet-type, passed");
    assert!(hurt(&ships[2]), "C behind it");
}

#[test]
fn an_ordinary_ship_does_not_set_off_a_planet_type_shots_proximity_fuse() {
    // As `a_proximity_hit_releases_its_submunitions_at_the_ship_hit`: B
    // sits within the fuse's reach of the shell's second move.
    let fuse_struck = |b: fn(Ship) -> Ship| {
        let (shell, arsenal) = cluster(0x0020 | PLANET_TYPE);
        let fused = WeaponRecord {
            prox_radius: 20,
            ..shell
        };
        let mut combat = Combat::default();
        let mut ships = [
            Ship::at(1, 0.0, 0.0).armed(fused),
            b(Ship::at(2, 40.0, 20.0)),
        ];
        for _ in 0..4 {
            tick_with(&mut combat, &mut ships, &arsenal, Rules::default());
            ships[0].trigger = QUIET;
        }
        hurt(&ships[1])
    };
    assert!(!fuse_struck(|ship| ship), "an ordinary ship");
    assert!(fuse_struck(planet), "a planet-type ship");
}

#[test]
fn a_planet_type_shot_turned_on_its_ordinary_firer_passes_through_it() {
    let struck = |firer: fn(Ship) -> Ship| {
        let mut combat = Combat::default();
        let mut ships = [firer(Ship::at(3, 30.0, 0.0))];
        combat.put_in_flight(Shot {
            source: Source::Other,
            target: Some(C),
            aimed_at: Some(C),
            ..flown_by_c(planet_gun(), Vec2::new(15.0, 0.0))
        });
        for _ in 0..3 {
            tick(&mut combat, &mut ships, &NovaDisable);
        }
        hurt(&ships[0])
    };
    assert!(
        !struck(|ship| ship),
        "rule 8 holds for a shot no ship fired"
    );
    assert!(struck(planet), "a planet-type firer is hit");
}

// Blasts.

/// Ionizing flak of `flags2`, detonating with a 40-pixel blast where its
/// third move ends, at (30, 0).
fn flak(flags2: u16) -> WeaponRecord {
    WeaponRecord {
        count: 3,
        speed: 1000,
        explod_type: 2,
        blast_radius: 40,
        flags: DETONATES,
        flags2,
        ..ionizing(blaster(), 80, GREEN)
    }
}

/// A fires `record` once to the right among `others`, by `reading` of
/// [`PlanetBlasts`], for `ticks` ticks. The ships, and the events.
fn fired_among(
    record: WeaponRecord,
    others: Vec<Ship>,
    reading: RuleSource,
    ticks: u32,
) -> (Vec<Ship>, Vec<CombatEvent>) {
    let mut combat = Combat::default();
    combat.set_planet_blasts(PlanetBlasts::from(reading));
    let mut ships = vec![Ship::at(1, 0.0, 0.0).armed(record)];
    ships.extend(others);
    let mut events = Vec::new();
    for _ in 0..ticks {
        tick(&mut combat, &mut ships, &NovaDisable);
        events.extend(combat.take_events());
        ships[0].trigger = QUIET;
    }
    (ships, events)
}

/// Whether the expiry of [`flak`] of `flags2`, by `reading`, blasts an
/// ordinary ship and a planet-type one beside where it ends: each
/// whether it was hurt and whether it was ionized.
fn expiry_blasts(flags2: u16, reading: RuleSource) -> [(bool, bool); 2] {
    let others = vec![Ship::at(3, 30.0, 35.0), planet(Ship::at(4, 30.0, -35.0))];
    let (ships, events) = fired_among(flak(flags2), others, reading, 6);
    assert_eq!(explosions(&events).len(), 1, "it explodes: {events:?}");
    [
        (hurt(&ships[1]), ships[1].ion.charge > 0.0),
        (hurt(&ships[2]), ships[2].ion.charge > 0.0),
    ]
}

#[test]
fn a_planet_type_shots_expiry_explodes_but_blasts_and_ionizes_no_ship() {
    assert_eq!(
        expiry_blasts(PLANET_TYPE, RuleSource::Engine),
        [(false, false); 2],
        "@0x35fb9"
    );
}

#[test]
fn an_ordinary_shots_expiry_blasts_the_ordinary_ships_but_not_a_planet_type_one() {
    assert_eq!(
        expiry_blasts(0, RuleSource::Engine),
        [(true, true), (false, false)],
        "@0x3602e"
    );
}

/// Whether a 30-pixel blast shell of `flags2` hitting B at (100, 0), of
/// `b`, blasts C, ordinary, at (100, 25), and D, planet-type, at
/// (100, -25), by `reading`.
fn on_hit_blasts(flags2: u16, b: fn(Ship) -> Ship, reading: RuleSource) -> (bool, bool, bool) {
    let shell = WeaponRecord {
        explod_type: 0,
        blast_radius: 30,
        flags2,
        ..blaster()
    };
    let others = vec![
        b(Ship::at(2, 100.0, 0.0)),
        Ship::at(3, 100.0, 25.0),
        planet(Ship::at(4, 100.0, -25.0)),
    ];
    let (ships, _) = fired_among(shell, others, reading, 8);
    (hurt(&ships[1]), hurt(&ships[2]), hurt(&ships[3]))
}

#[test]
fn a_planet_type_shot_hitting_a_planet_type_ship_blasts_the_ordinary_ship_beside_it() {
    assert_eq!(
        on_hit_blasts(PLANET_TYPE, planet, RuleSource::Engine),
        (true, true, true),
        "`_HandleShipHit` tests neither bit"
    );
}

#[test]
fn an_ordinary_shots_hit_blast_reaches_a_planet_type_ship_beside_its_victim() {
    assert_eq!(
        on_hit_blasts(0, |ship| ship, RuleSource::Engine),
        (true, true, true)
    );
}

/// The ships struck when the player shoots `weapon` of `flags2`'s blast
/// at an asteroid at (100, 0), by `reading`, with B, ordinary, and D,
/// planet-type, in the blast's square.
fn asteroid_blasts(flags2: u16, reading: RuleSource) -> Vec<ShipRef> {
    let mut combat = Combat::default();
    combat.set_planet_blasts(PlanetBlasts::from(reading));
    let mut rocks = [rock(0, 100.0, 0.0)];
    let others = vec![Ship::at(2, 110.0, 20.0), planet(Ship::at(4, 100.0, -25.0))];
    shoot_rocks(
        &mut combat,
        ShipRef::Player,
        blasting(0, flags2),
        others,
        &mut rocks,
        8,
    );
    combat
        .take_strikes()
        .iter()
        .map(|strike| strike.ship)
        .collect()
}

#[test]
fn by_the_engine_an_asteroid_hits_blast_spares_planet_type_ships_and_weapons() {
    assert_eq!(asteroid_blasts(0, RuleSource::Engine), [B]);
    assert_eq!(asteroid_blasts(PLANET_TYPE, RuleSource::Engine), []);
}

#[test]
fn by_the_bible_every_blast_reaches_the_ships_of_its_weapons_planet_type() {
    let d = ShipRef::Npc(NpcId(4));
    assert_eq!(
        expiry_blasts(PLANET_TYPE, RuleSource::Bible),
        [(false, false), (true, true)]
    );
    assert_eq!(
        expiry_blasts(0, RuleSource::Bible),
        [(true, true), (false, false)]
    );
    assert_eq!(
        on_hit_blasts(PLANET_TYPE, planet, RuleSource::Bible),
        (true, false, true)
    );
    assert_eq!(
        on_hit_blasts(0, |ship| ship, RuleSource::Bible),
        (true, true, false)
    );
    assert_eq!(asteroid_blasts(0, RuleSource::Bible), [B]);
    assert_eq!(asteroid_blasts(PLANET_TYPE, RuleSource::Bible), [d]);
}

// Beams.

/// `wëap` `Flags` 0x0400, the "big smoke" bit, which the original's beam
/// filter reads as the planet-type bit (@0x30d34-0x30d82).
const BIG_SMOKE: u16 = 0x0400;

/// [`laser`] with `flags` and `flags2`.
fn beam_of(flags: u16, flags2: u16) -> WeaponRecord {
    WeaponRecord {
        flags,
        flags2,
        ..laser()
    }
}

/// [`through`] for a beam of `flags` and `flags2`, by `reading` of
/// [`BeamPlanetFlag`].
fn beam_through(
    (flags, flags2): (u16, u16),
    reading: RuleSource,
    b: fn(Ship) -> Ship,
    d: fn(Ship) -> Ship,
) -> (bool, bool) {
    let mut combat = Combat::default();
    combat.set_beam_planet_flag(BeamPlanetFlag::from(reading));
    through_in(combat, beam_of(flags, flags2), b, d)
}

/// Each of a beam of `bits` by `reading`: whether it hits an ordinary B
/// before D, a planet-type one; and whether it passes a planet-type B to
/// hit D, ordinary.
fn beam_hits(bits: (u16, u16), reading: RuleSource) -> [(bool, bool); 2] {
    [
        beam_through(bits, reading, |ship| ship, planet),
        beam_through(bits, reading, planet, |ship| ship),
    ]
}

/// An ordinary beam hits the ordinary B and passes the planet-type B to
/// the ordinary D behind it.
const ORDINARY_HITS: [(bool, bool); 2] = [(true, false), (false, true)];
/// A planet-type beam passes the ordinary B to the planet-type D behind
/// it, and hits the planet-type B.
const PLANET_HITS: [(bool, bool); 2] = [(false, true), (true, false)];

#[test]
fn by_the_engine_a_beam_matches_its_flags_0x0400_against_the_ships_planet_type() {
    let engine = RuleSource::Engine;
    assert_eq!(beam_hits((BIG_SMOKE, 0), engine), PLANET_HITS);
    assert_eq!(beam_hits((0, 0), engine), ORDINARY_HITS);
    assert_eq!(
        beam_hits((0, PLANET_TYPE), engine),
        ORDINARY_HITS,
        "Flags2 does nothing for a beam"
    );
    assert_eq!(beam_hits((BIG_SMOKE, PLANET_TYPE), engine), PLANET_HITS);
}

#[test]
fn by_the_bible_a_beam_matches_its_flags2_0x0400_against_the_ships_planet_type() {
    let bible = RuleSource::Bible;
    assert_eq!(beam_hits((0, PLANET_TYPE), bible), PLANET_HITS);
    assert_eq!(beam_hits((0, 0), bible), ORDINARY_HITS);
    assert_eq!(beam_hits((BIG_SMOKE, 0), bible), ORDINARY_HITS);
    assert_eq!(beam_hits((BIG_SMOKE, PLANET_TYPE), bible), PLANET_HITS);
}
