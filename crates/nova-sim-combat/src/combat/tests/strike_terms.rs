//! The fight's tests of what each strike carries for `_DamageShip`'s
//! answer flag (rdm weapons-fidelity phase 26, see
//! `nova_sim::ai::answer_flag`): its nominal damage, the args +0x14 and
//! +0x18 as `_HandleShipHit` passes them, and its weight, arg +0x34: the
//! weapon's `Reload` for a shot's direct hit, its expiry blast and the
//! missile lock, 1 for a beam, none for a blast on a hit, an asteroid's
//! blast and a shockwave.

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
    let strikes = first_strikes(&mut combat, &mut ships, &Arsenal::default());
    assert_eq!(terms(&strikes), [(nominal(4.0, 10.0), 7.0, false)]);
}

#[test]
fn a_shots_direct_hit_carries_its_decayed_damage() {
    let mut combat = Combat::default();
    let mut ships = [
        Ship::at(1, 0.0, 0.0).armed(slug(0)).targeting(B),
        Ship::at(2, 95.0, 0.0),
    ];
    let strikes = first_strikes(&mut combat, &mut ships, &Arsenal::default());
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
    tick(&mut combat, &mut ships, &NovaDisable);
    assert_eq!(
        terms(&combat.take_strikes()),
        [(nominal(0.0, 5.0), 1.0, false)]
    );
}

#[test]
fn the_missile_lock_weighs_its_reload_and_carries_no_damage() {
    let mut ships = [player_with(missile()), far_b()];
    let strikes: Vec<Strike> = strikes_over(&mut ships, &Arsenal::default(), 40)
        .into_iter()
        .map(|(_, strike)| strike)
        .collect();
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
    let strikes = first_strikes(&mut combat, &mut ships, &Arsenal::default());
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
    let strikes = first_strikes(&mut combat, &mut ships, &Arsenal::default());
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
    tick(&mut combat, &mut ships, &NovaDisable);
    assert_eq!(
        terms(&combat.take_strikes()),
        [(nominal(32.0, 32.0), 0.0, false)]
    );
}

#[test]
fn an_asteroids_blast_weighs_nothing() {
    let strikes = blasted(ShipRef::Player, blasting(0, 0));
    assert_eq!(
        terms(&strikes)
            .into_iter()
            .map(|(_, weight, answered)| (weight, answered))
            .collect::<Vec<_>>(),
        [(0.0, false)]
    );
}
