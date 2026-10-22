//! The fight's planet-type rules (stellar-fidelity phase 2, see the
//! [`planet_type`](crate::combat::planet_type) module): a planet-type
//! weapon's shots against planet-type and ordinary ships.

use super::*;

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
    let mut combat = Combat::default();
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
