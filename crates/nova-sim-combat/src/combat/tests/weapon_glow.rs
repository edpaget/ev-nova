//! The fight's tests of the weapon glow (`Flags2` 0x0200, rdm
//! weapons-fidelity phase 24): ship +0xc8e8 = 32.0, set by
//! `_FirePlayerWeapon` (@0x62c78-0x62c98) and `_FireAIShipWeapon`
//! (@0x88dc0-0x88de4) once a call that fired something, and by
//! `_HandleBeams` for the firer of each live beam on the advance pass
//! (@0x30b1f-0x30b37), but a point-defence beam's at a missile
//! (@0x309ab-0x309bb).

use super::*;
use crate::combat::weapon::GLOWS;

/// `record` of the weapon glow.
fn glowing(record: WeaponRecord) -> WeaponRecord {
    WeaponRecord {
        flags2: record.flags2 | GLOWS,
        ..record
    }
}

/// The glows among `events`: each ship's, and whether it is late.
fn glows(events: &[CombatEvent]) -> Vec<(ShipRef, bool)> {
    events
        .iter()
        .filter_map(|event| match *event {
            CombatEvent::WeaponGlow { ship, late } => Some((ship, late)),
            _ => None,
        })
        .collect()
}

/// A blaster of three copies firing together.
fn triple(record: WeaponRecord) -> Armament {
    Armament::new([(WeaponSpec::new(&record), 3)])
}

fn simultaneous(record: WeaponRecord) -> WeaponRecord {
    WeaponRecord {
        flags: record.flags | crate::combat::weapon::SIMULTANEOUS,
        ..record
    }
}

#[test]
fn a_glow_weapon_loads_its_flags2_0x0200() {
    assert!(WeaponSpec::new(&glowing(blaster())).glows());
    assert!(!WeaponSpec::new(&blaster()).glows());
    assert_eq!(GLOWS, 0x0200);
}

#[test]
fn the_player_firing_a_glow_weapon_glows_once_a_call_that_frame() {
    let mut combat = Combat::default();
    let mut player = player_with(simultaneous(glowing(blaster())));
    player.armament = triple(simultaneous(glowing(blaster())));
    let mut ships = [player, Ship::at(2, 200.0, 0.0)];
    tick(&mut combat, &mut ships, &NovaDisable);
    let events = combat.take_events();
    assert_eq!(fired(&events, ShipRef::Player), 3, "three copies");
    assert_eq!(glows(&events), [(ShipRef::Player, false)], "one glow");
}

#[test]
fn an_npcs_pick_of_several_copies_glows_once_a_call() {
    let mut combat = Combat::default();
    let mut firer = Ship::at(1, 0.0, 0.0).picking(128).targeting(B).facing(90.0);
    firer.armament = triple(simultaneous(glowing(blaster())));
    let mut ships = [firer, Ship::at(2, 200.0, 0.0)];
    tick(&mut combat, &mut ships, &NovaDisable);
    let events = combat.take_events();
    assert_eq!(fired(&events, A), 3, "three copies");
    assert_eq!(glows(&events), [(A, false)], "one glow");
}

#[test]
fn a_weapon_without_the_flag_never_glows() {
    let mut combat = Combat::default();
    let mut ships = [
        player_with(blaster()),
        Ship::at(1, 0.0, 50.0).armed(blaster()).picking(128),
        Ship::at(2, 200.0, 0.0),
    ];
    tick(&mut combat, &mut ships, &NovaDisable);
    let events = combat.take_events();
    assert!(fired(&events, ShipRef::Player) > 0 && fired(&events, A) > 0);
    assert_eq!(glows(&events), []);
}

#[test]
fn a_call_that_fires_nothing_never_glows() {
    // Reloading: the second call of a slow gun.
    let slow = glowing(WeaponRecord {
        reload: 1000,
        ..blaster()
    });
    let mut combat = Combat::default();
    let mut ships = [player_with(slow), Ship::at(2, 200.0, 0.0)];
    tick(&mut combat, &mut ships, &NovaDisable);
    assert_eq!(glows(&combat.take_events()), [(ShipRef::Player, false)]);
    tick(&mut combat, &mut ships, &NovaDisable);
    let events = combat.take_events();
    assert_eq!(fired(&events, ShipRef::Player), 0, "reloading");
    assert_eq!(glows(&events), [], "reloading");
    // Out of ammo: a launcher of no rounds, the player's and an NPC's.
    let launcher = glowing(WeaponRecord {
        ammo_type: 0,
        ..blaster()
    });
    let mut combat = Combat::default();
    let mut ships = [
        player_with(launcher),
        Ship::at(1, 0.0, 50.0).armed(launcher).picking(128),
        Ship::at(2, 200.0, 0.0),
    ];
    tick(&mut combat, &mut ships, &NovaDisable);
    let events = combat.take_events();
    assert_eq!(fired(&events, ShipRef::Player) + fired(&events, A), 0);
    assert_eq!(glows(&events), [], "out of ammo");
}

#[test]
fn point_defence_glows_never_when_it_fires() {
    let mut combat = Combat::default();
    let mut ships = missile_attack(glowing(quad()));
    let mut seen = Vec::new();
    let mut defended = false;
    for _ in 0..30 {
        tick(&mut combat, &mut ships, &NovaDisable);
        let events = combat.take_events();
        defended |= fired(&events, A) > 0;
        seen.extend(glows(&events));
    }
    assert!(defended, "point defence fired");
    assert_eq!(seen, []);
}

#[test]
fn a_point_defence_beam_at_a_missile_glows_never_on_its_ticks() {
    let mut combat = Combat::default();
    let mut ships = missile_attack(glowing(pd_beam()));
    let mut seen = Vec::new();
    let mut beamed = false;
    for _ in 0..30 {
        tick(&mut combat, &mut ships, &NovaDisable);
        beamed |= !combat.beams().is_empty();
        seen.extend(glows(&combat.take_events()));
    }
    assert!(beamed, "point defence beamed");
    assert_eq!(seen, []);
}

#[test]
fn a_bay_launch_glows_never_in_the_fight() {
    let bay = glowing(carrier_bay());
    let mut combat = Combat::default();
    let mut ships = carrier();
    ships[0].armament = Armament::new([(WeaponSpec::new(&bay), 1)]);
    tick(&mut combat, &mut ships, &NovaDisable);
    assert_eq!(combat.take_sorties().len(), 1, "launched");
    assert_eq!(glows(&combat.take_events()), [], "_AILaunchFighter");

    // `_FirePlayerWeapon` reaches the glow from a bay only once
    // `_LaunchFighter` put the fighter in the system (@0x62c61-0x62c6e),
    // which the session decides: it lights the glow (`Combat::light_glow`).
    let mut combat = Combat::default();
    let [carrier, other] = carrier();
    let mut player = Ship {
        id: ShipRef::Player,
        fleet: ShipRef::Player,
        ..carrier
    };
    player.armament = Armament::new([(WeaponSpec::new(&bay), 1)]);
    let mut ships = [player, other];
    tick(&mut combat, &mut ships, &NovaDisable);
    assert_eq!(combat.take_sorties().len(), 1, "launched");
    assert_eq!(glows(&combat.take_events()), [], "the session's to light");
}

#[test]
fn lighting_a_glow_raises_it_that_frame() {
    let mut combat = Combat::default();
    combat.light_glow(ShipRef::Player);
    assert_eq!(glows(&combat.take_events()), [(ShipRef::Player, false)]);
}

#[test]
fn a_sub_munition_release_glows_never() {
    let (shell, _) = cluster(0);
    let sub = glowing(WeaponRecord {
        count: 50,
        speed: 500,
        ..weapon(148)
    });
    let arsenal = Arsenal::new(&[shell, sub], Vec::new());
    let mut combat = Combat::default();
    let mut ships = [player_with(shell), Ship::at(2, 0.0, 400.0)];
    let mut seen = Vec::new();
    for tick_no in 0..6 {
        tick_with(&mut combat, &mut ships, &arsenal, Rules::default());
        if tick_no == 0 {
            ships[0].trigger = QUIET;
        }
        seen.extend(glows(&combat.take_events()));
    }
    assert!(!sub_shots(&combat).is_empty(), "released");
    assert_eq!(seen, []);
}

#[test]
fn a_live_glow_beam_glows_its_firer_late_each_tick_including_its_last() {
    let mut combat = Combat::default();
    let mut ships = [
        Ship::at(1, 0.0, 0.0).armed(glowing(laser())).picking(146),
        Ship::at(2, 100.0, 0.0),
    ];
    let mut kept = Vec::new();
    let mut seen = Vec::new();
    for tick_no in 0..5 {
        tick(&mut combat, &mut ships, &NovaDisable);
        if tick_no == 0 {
            ships[0].trigger = QUIET;
        }
        kept.push(combat.beams().len());
        seen.push(glows(&combat.take_events()));
    }
    assert_eq!(kept, [1, 1, 1, 0, 0], "three ticks of beam");
    assert_eq!(
        seen,
        [
            vec![(A, false), (A, true)],
            vec![(A, true)],
            vec![(A, true)],
            vec![],
            vec![],
        ]
    );
}

#[test]
fn a_beam_glows_never_on_double_times_second_pass() {
    let mut combat = Combat::default();
    let mut ships = [
        Ship::at(1, 0.0, 0.0).armed(glowing(laser())).picking(146),
        Ship::at(2, 100.0, 0.0),
    ];
    tick(&mut combat, &mut ships, &NovaDisable);
    ships[0].trigger = QUIET;
    combat.take_events();
    tick_on(&mut combat, &mut ships, GamePass::Again);
    assert_eq!(combat.beams().len(), 1, "still live");
    assert_eq!(glows(&combat.take_events()), []);
}

#[test]
fn a_point_defence_beam_glows_on_its_ticks_at_a_ship_but_not_at_a_missile() {
    let from = ShipState::default();
    let beam = |record: WeaponRecord, aiming| {
        Beam::launch(WeaponSpec::new(&record), A, A, &from, aiming).glows()
    };
    let pd = glowing(pd_beam());
    assert!(beam(pd, Aiming::Ship(B)), "at a ship");
    assert!(!beam(pd, Aiming::Shot(ShotId(1))), "at a missile");
    assert!(beam(glowing(laser()), Aiming::Ahead));
    assert!(!beam(pd_beam(), Aiming::Ship(B)), "no flag");
}
