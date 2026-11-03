//! The fight's tests of the player's readers of the frame's gain
//! ([`Field::speed_mult`], `_HandlePlayer`'s `_gSpeedMult`): its shield
//! and armour regeneration (@0x6cfc6, @0x6d063) and its reloads
//! (@0x6ae46) take the gain's share of a frame.

use super::*;
use crate::speed_mult::SpeedMult;

/// The arena of the advance pass by the default rules at `gain`.
fn at_gain<'a>(arsenal: &'a Arsenal, govts: &'a Governments, gain: SpeedMult) -> Arena<'a> {
    Arena {
        field: Field {
            speed_mult: gain,
            ..Field::default()
        },
        ..phase::arena(arsenal, govts, Rules::default())
    }
}

/// Runs `phase` among `ships` at `gain`.
fn run_at(
    combat: &mut Combat,
    ships: &mut [Ship],
    gain: SpeedMult,
    phase: impl FnOnce(&mut Combat, &mut [Fighter], Arena<'_>, &mut dyn Chance),
) {
    let (arsenal, govts) = (Arsenal::default(), Governments::default());
    phase::run_in(
        combat,
        ships,
        at_gain(&arsenal, &govts, gain),
        &mut NeverFires,
        phase,
    );
}

/// The player, its shield at 5 of 10 and its armour at 20 of 30,
/// regenerating 2 and 1 a frame.
fn regenerating() -> Ship {
    let mut player = Ship {
        id: ShipRef::Player,
        fleet: ShipRef::Player,
        slot: Slot::Player,
        ..Ship::at(0, 0.0, 0.0)
    };
    player.reserves.shield = Gauge {
        now: 5.0,
        max: 10.0,
    };
    player.reserves.armor = Gauge {
        now: 20.0,
        max: 30.0,
    };
    player.shield_regen = 2.0;
    player.armor_regen = 1.0;
    player
}

/// The player's shield and armour after its regeneration at `gain`.
fn regenerated(ship: Ship, gain: SpeedMult) -> (f32, f32) {
    let mut combat = Combat::default();
    let mut ships = [ship];
    run_at(
        &mut combat,
        &mut ships,
        gain,
        |combat, fighters, arena, _| {
            combat.regenerate_player(fighters, arena);
        },
    );
    (ships[0].reserves.shield.now, ships[0].reserves.armor.now)
}

#[test]
fn at_half_gain_the_player_regenerates_half_its_rates() {
    assert_eq!(
        regenerated(regenerating(), SpeedMult::new(0.5)),
        (6.0, 20.5)
    );
    assert_eq!(regenerated(regenerating(), SpeedMult::FULL), (7.0, 21.0));
    assert_eq!(regenerated(regenerating(), SpeedMult::ZERO), (5.0, 20.0));
}

#[test]
fn at_half_gain_the_regeneration_still_stops_at_full_and_holds_off_while_disabled() {
    let mut nearly = regenerating();
    nearly.reserves.shield.now = 9.5;
    assert_eq!(regenerated(nearly, SpeedMult::new(0.5)).0, 10.0);
    let mut disabled = regenerating();
    disabled.condition = Condition::Disabled;
    assert_eq!(regenerated(disabled, SpeedMult::new(0.5)), (5.0, 20.0));
}

/// A blaster reloading in 4 frames.
fn slow_gun() -> WeaponRecord {
    WeaponRecord {
        reload: 4,
        ..blaster()
    }
}

/// The player's reload after its trigger at `gain` on each of `passes`
/// passes, the gun held reloading from 4.
fn reloads(gun: WeaponRecord, gain: SpeedMult, passes: usize, ion: f32) -> Vec<f32> {
    let mut combat = Combat::default();
    let mut player = Ship {
        id: ShipRef::Player,
        fleet: ShipRef::Player,
        slot: Slot::Player,
        ..Ship::at(0, 0.0, 0.0)
    }
    .armed(gun);
    player.ion_capacity = 100.0;
    player.ion.charge = ion;
    player.armament.hold(WeaponId(128));
    let mut ships = [player];
    (0..passes)
        .map(|_| {
            run_at(
                &mut combat,
                &mut ships,
                gain,
                |combat, fighters, arena, chance| {
                    combat.fire_player(fighters, arena, chance);
                },
            );
            ships[0].armament.mounts()[0].reload
        })
        .collect()
}

#[test]
fn at_half_gain_the_players_reload_falls_half_a_frame_a_pass() {
    assert_eq!(
        reloads(slow_gun(), SpeedMult::new(0.5), 4, 0.0),
        [3.5, 3.0, 2.5, 2.0]
    );
    assert_eq!(
        reloads(slow_gun(), SpeedMult::FULL, 4, 0.0),
        [3.0, 2.0, 1.0, 0.0]
    );
}

#[test]
fn at_half_gain_an_ionized_hold_still_lands_on_the_held_reload() {
    let held = WeaponRecord {
        seeker: ion::CANT_FIRE_IONIZED,
        ..slow_gun()
    };
    assert_eq!(
        reloads(held, SpeedMult::new(0.5), 2, 100.0),
        [ion::HELD_RELOAD, ion::HELD_RELOAD]
    );
}

#[test]
fn a_default_field_runs_at_full_gain() {
    assert_eq!(Field::default().speed_mult, SpeedMult::FULL);
}
