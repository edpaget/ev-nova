//! The fight's tests of its phases' order (rdm weapons-fidelity phase 28):
//! `_DoPlayGameWork` runs the collision passes, then `_AIDispatch` (the
//! NPCs' point defence), then `_HandleShot` (the shots' ends of life,
//! moves, locks and holds), then `_HandleShip` (each NPC's reload and
//! pick), then `_HandleBeams`.

use super::*;

/// The tick numbers, from 1, on which `ship` was struck over `ticks`
/// ticks of `ships`.
fn struck_on(ships: &mut [Ship], ship: ShipRef, ticks: u32) -> Vec<u32> {
    let mut combat = Combat::default();
    let mut on = Vec::new();
    for tick_no in 1..=ticks {
        tick(&mut combat, ships, &NovaDisable);
        if combat
            .take_strikes()
            .iter()
            .any(|strike| strike.ship == ship)
        {
            on.push(tick_no);
        }
    }
    on
}

#[test]
fn an_npcs_shot_first_moves_in_the_tick_after_it_is_fired() {
    // `_HandleShip` fires the pick (@0x34ffd) after `_HandleShot` has
    // moved the shots (@0x44385), so its shot first moves in the next
    // frame's `_HandleShot`.
    let mut combat = Combat::default();
    let mut ships = [Ship::at(1, 0.0, 0.0).armed(blaster())];
    tick(&mut combat, &mut ships, &NovaDisable);
    let fired = combat.shots()[0];
    assert_eq!(fired.position, Vec2::ZERO, "at its launch point");
    assert_eq!(fired.moved_from, None, "not moved");
    ships[0].trigger = QUIET;
    tick(&mut combat, &mut ships, &NovaDisable);
    let moved = combat.shots()[0];
    assert!((moved.position.x - 15.0).abs() < 1e-4, "{moved:?}");
    assert_eq!(moved.moved_from, Some(Vec2::ZERO), "moved once");
}

#[test]
fn an_npcs_shot_is_first_tested_two_ticks_after_it_is_fired() {
    // Fired on tick 1, moved on tick 2, its move tested by tick 3's
    // collision passes.
    let mut ships = [
        Ship::at(1, 0.0, 0.0).armed(blaster()),
        Ship::at(2, 15.0, 0.0),
    ];
    let on = struck_on(&mut ships, B, 3);
    assert_eq!(on.first(), Some(&3), "{on:?}");
}

/// A one-at-a-time weapon (`Flags3` 0x0004) of Reload 3, 10 pixels a tick
/// for 10 ticks.
fn single() -> WeaponRecord {
    WeaponRecord {
        reload: 3,
        count: 10,
        speed: 1000,
        flags3: 0x0004,
        ..weapon(128)
    }
}

#[test]
fn an_npcs_one_at_a_time_weapon_fires_again_reload_ticks_after_its_shots_last_move() {
    // `_HandleShot` holds the reload at 3 on the shot's every move
    // (@0x35c09), then `_HandleShip` counts it down (@0x33896) before the
    // pick (@0x34ffd): fired on tick 0, the shot moves on ticks 1 to 10,
    // the 10th move spending it, and the reload reads 2, 1 then 0 on
    // ticks 10 to 12.
    let firer = || Ship::at(1, 0.0, 0.0).armed(single());
    assert_eq!(
        fire_ticks(&mut [firer(), Ship::at(2, 900.0, 0.0)], 25),
        [0, 12, 24],
        "Reload ticks after its last move"
    );
    // It moves onto B, 60 pixels off, on tick 5, and the collision passes
    // take it on tick 6, before `_HandleShot` could hold the reload again.
    assert_eq!(
        fire_ticks(&mut [firer(), Ship::at(2, 60.0, 0.0)], 9),
        [0, 7],
        "Reload ticks after its last move, onto its victim"
    );
}

#[test]
fn the_players_reload_counts_down_after_its_trigger() {
    // `_HandlePlayer` counts its reloads down (@0x6ae25-0x6ae61) after
    // its trigger (@0x6ab55) and before `_HandleShot` holds them: its
    // shot moves on the tick it is fired, ticks 0 to 9, and the reload
    // reads 2, 1 then 0 after the triggers of ticks 10 to 12.
    let player = Ship {
        id: ShipRef::Player,
        fleet: ShipRef::Player,
        ..Ship::at(1, 0.0, 0.0).armed(single())
    };
    assert_eq!(
        fire_ticks(&mut [player, Ship::at(2, 900.0, 0.0)], 14),
        [0, 13]
    );
    let every = Ship {
        id: ShipRef::Player,
        fleet: ShipRef::Player,
        ..Ship::at(1, 0.0, 0.0).armed(WeaponRecord {
            flags3: 0,
            ..single()
        })
    };
    assert_eq!(
        fire_ticks(&mut [every, Ship::at(2, 900.0, 0.0)], 7),
        [0, 3, 6],
        "every Reload ticks"
    );
}
