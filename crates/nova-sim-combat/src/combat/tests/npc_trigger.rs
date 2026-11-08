//! The fight's tests of what an NPC's trigger fires (rdm combat-and-flight
//! phase 24): its pick (`Trigger::only`, `_FireAIShipWeapon`), and never
//! its primary or secondary keys, which are the player's. No AI holds an
//! NPC's primary or secondary, and the original has no such path: an
//! NPC's guns fire only as `_AIFireWeapon`/`_AIFireTurret` pick them.

use super::*;
use crate::combat::weapon::SECONDARY;

/// NPC 1 at the origin, facing B at (200, 0), armed with `armament`,
/// holding `trigger`.
fn holding(armament: Armament, trigger: Trigger) -> [Ship; 2] {
    let mut firer = Ship::at(1, 0.0, 0.0).targeting(B).facing(90.0);
    firer.armament = armament;
    firer.trigger = trigger;
    [firer, Ship::at(2, 200.0, 0.0)]
}

/// A blaster that, once fired, reloads.
fn reloading() -> WeaponRecord {
    WeaponRecord {
        reload: 100,
        ..blaster()
    }
}

/// The weapons whose shots `combat` holds.
fn shot_weapons(combat: &Combat) -> Vec<WeaponId> {
    combat.shots().iter().map(|shot| shot.weapon.id).collect()
}

#[test]
fn an_npcs_primary_trigger_fires_nothing() {
    let mut combat = Combat::default();
    let trigger = Trigger {
        primary: true,
        point_defence: false,
        ..QUIET
    };
    let mut ships = holding(Armament::new([(WeaponSpec::new(&reloading()), 1)]), trigger);
    phase::handle_npcs(&mut combat, &mut ships, &NovaDisable);
    assert_eq!(combat.shots(), [], "no shot");
    assert_eq!(fired(&combat.take_events(), A), 0, "no Fired");
    assert_eq!(
        ships[0].armament.mounts()[0].reload,
        0.0,
        "reload untouched"
    );
}

#[test]
fn an_npcs_secondary_trigger_fires_nothing() {
    let mut combat = Combat::default();
    let secondary = WeaponRecord {
        flags: SECONDARY,
        ..reloading()
    };
    let trigger = Trigger {
        secondary: Some(secondary.id),
        point_defence: false,
        ..QUIET
    };
    let mut ships = holding(Armament::new([(WeaponSpec::new(&secondary), 1)]), trigger);
    ships[0].rounds = LauncherRounds::from([(secondary.id, 5)]);
    phase::handle_npcs(&mut combat, &mut ships, &NovaDisable);
    assert_eq!(combat.shots(), [], "no shot");
    assert_eq!(fired(&combat.take_events(), A), 0, "no Fired");
    assert_eq!(
        ships[0].armament.mounts()[0].reload,
        0.0,
        "reload untouched"
    );
}

#[test]
fn an_npc_fires_only_its_pick_whatever_its_primary_says() {
    let mut combat = Combat::default();
    let a = reloading();
    let b = WeaponRecord {
        id: WeaponId(129),
        ..reloading()
    };
    let armament = Armament::new([(WeaponSpec::new(&a), 1), (WeaponSpec::new(&b), 1)]);
    let trigger = Trigger {
        primary: true,
        only: Some(b.id),
        point_defence: false,
        ..QUIET
    };
    let mut ships = holding(armament, trigger);
    phase::handle_npcs(&mut combat, &mut ships, &NovaDisable);
    assert_eq!(shot_weapons(&combat), [b.id], "the pick alone, once");
    assert_eq!(ships[0].armament.mounts()[0].reload, 0.0, "a untouched");
}
