//! The fight's tests of an NPC's readers of the frame's gain
//! ([`Field::speed_mult`], `_HandleShip`'s `_gSpeedMult`): its reloads
//! (@0x338b3) and its shield and armour regeneration (@0x345da,
//! @0x3464c) take the gain's share of a frame at its step
//! ([`Combat::step_npc`]).

use super::*;
use crate::speed_mult::SpeedMult;

const HALF: SpeedMult = SpeedMult::new(0.5);

/// Runs NPC 1's step among `ships` at `gain`.
fn stepped(combat: &mut Combat, ships: &mut [Ship], gain: SpeedMult) {
    let (arsenal, govts) = (Arsenal::default(), Governments::default());
    let arena = Arena {
        field: Field {
            speed_mult: gain,
            ..Field::default()
        },
        ..phase::arena(&arsenal, &govts, Rules::default())
    };
    phase::run_in(
        combat,
        ships,
        arena,
        &mut NeverFires,
        |combat, fighters, arena, chance| {
            combat.step_npc(fighters, arena, ShipRef::Npc(NpcId(1)), chance);
        },
    );
}

/// NPC 1, its shield at 5 of 10 and its armour at 20 of 30,
/// regenerating 2 and 1 a frame.
fn regenerating() -> Ship {
    let mut npc = Ship::at(1, 0.0, 0.0);
    npc.reserves.shield = Gauge {
        now: 5.0,
        max: 10.0,
    };
    npc.reserves.armor = Gauge {
        now: 20.0,
        max: 30.0,
    };
    npc.shield_regen = 2.0;
    npc.armor_regen = 1.0;
    npc
}

/// `ship`'s shield and armour after its step at `gain`.
fn regenerated(ship: Ship, gain: SpeedMult) -> (f32, f32) {
    let mut ships = [ship];
    stepped(&mut Combat::default(), &mut ships, gain);
    (ships[0].reserves.shield.now, ships[0].reserves.armor.now)
}

#[test]
fn at_half_gain_an_npc_regenerates_half_its_rates() {
    assert_eq!(regenerated(regenerating(), HALF), (6.0, 20.5));
    assert_eq!(regenerated(regenerating(), SpeedMult::FULL), (7.0, 21.0));
    assert_eq!(regenerated(regenerating(), SpeedMult::ZERO), (5.0, 20.0));
}

#[test]
fn at_half_gain_an_npcs_regeneration_still_stops_at_full_and_holds_off_while_disabled() {
    let mut nearly = regenerating();
    nearly.reserves.shield.now = 9.5;
    assert_eq!(regenerated(nearly, HALF).0, 10.0);
    let mut disabled = regenerating();
    disabled.condition = Condition::Disabled;
    assert_eq!(regenerated(disabled, HALF), (5.0, 20.0));
}

/// A blaster reloading in 4 frames.
fn slow_gun() -> WeaponRecord {
    WeaponRecord {
        reload: 4,
        ..blaster()
    }
}

/// NPC 1's reload after its step at `gain` on each of `passes` passes,
/// its gun held reloading from 4, its ion charge `ion` of 100.
fn reloads(gun: WeaponRecord, gain: SpeedMult, passes: usize, ion: f32) -> Vec<f32> {
    let mut combat = Combat::default();
    let mut npc = Ship::at(1, 0.0, 0.0).armed(gun);
    npc.ion_capacity = 100.0;
    npc.ion.charge = ion;
    npc.armament.hold(WeaponId(128));
    let mut ships = [npc];
    (0..passes)
        .map(|_| {
            stepped(&mut combat, &mut ships, gain);
            ships[0].armament.mounts()[0].reload
        })
        .collect()
}

#[test]
fn at_half_gain_an_npcs_reload_falls_half_a_frame_a_pass() {
    assert_eq!(reloads(slow_gun(), HALF, 3, 0.0), [3.5, 3.0, 2.5]);
    assert_eq!(
        reloads(slow_gun(), SpeedMult::FULL, 3, 0.0),
        [3.0, 2.0, 1.0]
    );
}

#[test]
fn at_half_gain_an_ionized_npcs_hold_still_lands_on_the_held_reload() {
    let held = WeaponRecord {
        seeker: ion::CANT_FIRE_IONIZED,
        ..slow_gun()
    };
    assert_eq!(
        reloads(held, HALF, 2, 100.0),
        [ion::HELD_RELOAD, ion::HELD_RELOAD]
    );
}
