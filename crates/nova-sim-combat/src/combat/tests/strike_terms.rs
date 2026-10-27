//! The fight's tests of what each strike carries for `_DamageShip`'s
//! answer flag (rdm weapons-fidelity phase 26, see
//! `nova_sim::ai::answer_flag`): its nominal damage, the args +0x14 and
//! +0x18 as `_HandleShipHit` passes them, and its weight, arg +0x34: the
//! weapon's `Reload` for a shot's direct hit, its expiry blast and the
//! missile lock, 1 for a beam, none for a blast on a hit, an asteroid's
//! blast and a shockwave.

use super::rock_blasts::shot_into_rock;
use super::*;

/// What each of `strikes` carries for the answer flag: its nominal
/// damage, its weight, and whether it was answered.
fn terms(strikes: &[Strike]) -> Vec<(Nominal, f32, bool)> {
    strikes
        .iter()
        .map(|strike| (strike.nominal, strike.weight, strike.answered))
        .collect()
}

const fn nominal(mass: f32, energy: f32) -> Nominal {
    Nominal { mass, energy }
}

/// The strikes of the collision passes among `ships` once A has fired
/// in the ship loop and `_HandleShot` has flown its shot `moves` times.
fn struck_after(combat: &mut Combat, ships: &mut [Ship], moves: usize) -> Vec<Strike> {
    phase::handle_npcs(combat, ships, &NovaDisable);
    ships[0].trigger = QUIET;
    for _ in 0..moves {
        phase::fly_shots(combat, ships, &NovaDisable);
    }
    assert_eq!(combat.take_strikes(), [], "none in flight");
    phase::test_hits(combat, ships, &NovaDisable);
    combat.take_strikes()
}

#[test]
fn a_shots_direct_hit_carries_its_weapons_damage_and_reload() {
    let mut combat = Combat::default();
    let gun = WeaponRecord {
        reload: 7,
        ..blaster()
    };
    let mut ships = [
        Ship::at(1, 0.0, 0.0).armed(gun).targeting(B),
        Ship::at(2, 100.0, 0.0),
    ];
    // Its sixth move, from (75, 0) to (90, 0), meets B.
    let strikes = struck_after(&mut combat, &mut ships, 6);
    assert_eq!(terms(&strikes), [(nominal(4.0, 10.0), 7.0, false)]);
}

#[test]
fn a_shots_direct_hit_carries_its_decayed_damage() {
    let mut combat = Combat::default();
    let mut ships = [
        Ship::at(1, 0.0, 0.0).armed(slug(0)).targeting(B),
        Ship::at(2, 95.0, 0.0),
    ];
    // Its eighth move, from (70, 0) to (80, 0), meets B.
    let strikes = struck_after(&mut combat, &mut ships, 8);
    let [(Nominal { mass, energy }, weight, _)] = terms(&strikes)[..] else {
        panic!("one strike: {strikes:?}");
    };
    assert!(mass < 10.0, "{mass}");
    assert_eq!((mass - energy, weight), (3.0, 1000.0));
}

#[test]
fn a_beam_weighs_1_whatever_its_reload() {
    let mut combat = Combat::default();
    let mut target = Ship::at(2, 100.0, 0.0);
    target.reserves.shield = Gauge::full(100.0);
    let mut ships = [Ship::at(1, 0.0, 0.0).armed(laser()).targeting(B), target];
    phase::handle_npcs(&mut combat, &mut ships, &NovaDisable);
    phase::close_tick(&mut combat, &mut ships, &NovaDisable);
    assert_eq!(
        terms(&combat.take_strikes()),
        [(nominal(0.0, 5.0), 1.0, false)]
    );
}

#[test]
fn the_missile_lock_weighs_its_reload_and_carries_no_damage() {
    let mut ships = [player_with(missile()), far_b()];
    let mut combat = Combat::default();
    phase::fire_player(&mut combat, &mut ships, &NovaDisable);
    ships[0].trigger = QUIET;
    // The lock lands in `_HandleShot` as the missile's age passes 30.
    let mut strikes = combat.take_strikes();
    for _ in 0..40 {
        phase::fly_shots(&mut combat, &mut ships, &NovaDisable);
        strikes.extend(combat.take_strikes());
    }
    assert_eq!(terms(&strikes), [(nominal(0.0, 0.0), 1000.0, false)]);
    assert!(strikes[0].lock);
}

#[test]
fn a_blast_on_a_hit_weighs_nothing_and_an_expiry_blast_its_reload() {
    let shell = WeaponRecord {
        explod_type: 0,
        blast_radius: 30,
        reload: 1000,
        ..blaster()
    };
    let mut combat = Combat::default();
    let mut ships = [
        Ship::at(1, 0.0, 0.0).armed(shell).targeting(B),
        Ship::at(2, 100.0, 0.0),
        Ship::at(3, 100.0, 25.0),
    ];
    // Its sixth move, from (75, 0) to (90, 0), meets B.
    let strikes = struck_after(&mut combat, &mut ships, 6);
    assert_eq!(
        terms(&strikes),
        [
            (nominal(4.0, 10.0), 1000.0, false),
            (nominal(4.0, 10.0), 0.0, false)
        ],
        "the hit, then the blast"
    );
    let flak = WeaponRecord {
        count: 3,
        speed: 1000,
        explod_type: 2,
        blast_radius: 40,
        flags: 0x8000,
        reload: 9,
        ..blaster()
    };
    let mut combat = Combat::default();
    let mut ships = [Ship::at(1, 0.0, 0.0).armed(flak), Ship::at(3, 30.0, 35.0)];
    phase::handle_npcs(&mut combat, &mut ships, &NovaDisable);
    // Its third move ends at (30, 0), and the visit after ends its life.
    for _ in 0..4 {
        phase::fly_shots(&mut combat, &mut ships, &NovaDisable);
    }
    let strikes = combat.take_strikes();
    assert_eq!(terms(&strikes), [(nominal(4.0, 10.0), 9.0, false)]);
    assert!(!strikes[0].direct);
}

#[test]
fn a_shockwave_carries_its_damage_both_ways_and_weighs_nothing() {
    let mut combat = Combat::default();
    let mut ships = [
        wreck(WRECK, 200.0, 0.0, 0.0),
        holding(1, 20.0, 0.0, 0.0, 300.0, 300.0),
    ];
    // The wreck's final explosion, at its display in the ship loop.
    phase::handle_npcs(&mut combat, &mut ships, &NovaDisable);
    assert_eq!(
        terms(&combat.take_strikes()),
        [(nominal(32.0, 32.0), 0.0, false)]
    );
}

#[test]
fn an_asteroids_blast_weighs_nothing() {
    let mut planet = Ship::at(4, 100.0, -25.0);
    planet.hull.planet_type = true;
    let others = vec![Ship::at(2, 110.0, 20.0), Ship::at(3, 100.0, 60.0), planet];
    let mut combat = Combat::default();
    shot_into_rock(&mut combat, ShipRef::Player, blasting(0, 0), others);
    let strikes = combat.take_strikes();
    assert_eq!(
        terms(&strikes)
            .into_iter()
            .map(|(_, weight, answered)| (weight, answered))
            .collect::<Vec<_>>(),
        [(0.0, false)]
    );
}
