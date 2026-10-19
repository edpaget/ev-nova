//! The fight's stamps of a ship's last shot (+0xa8), which a ship of
//! `shän` `Flags` 0x0080 refolds by (see
//! [`ship_frames`](crate::combat::ship_frames)): its pick's fire
//! (`_FireAIShipWeapon` @0x8874c) and each advance pass of a live beam of
//! its own (`_HandleBeams` @0x30b62-0x30b67).

use super::*;
use crate::combat::armament::{Fold, NEVER_FIRED};

/// NPC 1 at the centre facing up, its pick the beam 142 of 10 ticks.
fn beaming() -> Ship {
    let ship = Ship::at(1, 0.0, 0.0).armed(exit_beam(0)).facing(0.0);
    Ship {
        trigger: Trigger {
            primary: false,
            only: Some(WeaponId(142)),
            ..QUIET
        },
        ..ship
    }
}

#[test]
fn a_live_beam_stamps_its_firers_last_shot_each_advance_pass() {
    let mut combat = Combat::default();
    let mut ships = [beaming()];
    ships[0].armament = ships[0].armament.clone().with_fold(Fold::default());
    tick(&mut combat, &mut ships, &NovaDisable);
    assert_eq!(combat.beams().len(), 1, "a beam");
    assert_eq!(ships[0].armament.fold().since_fire, 0, "the pick stamps");
    ships[0].trigger = QUIET;
    ships[0].armament.tick_fire_clock();
    tick(&mut combat, &mut ships, &NovaDisable);
    assert_eq!(combat.beams().len(), 1, "still lit");
    assert_eq!(ships[0].armament.fold().since_fire, 0, "the beam stamps");
    for _ in 0..12 {
        ships[0].armament.tick_fire_clock();
        tick(&mut combat, &mut ships, &NovaDisable);
    }
    assert_eq!(combat.beams(), [], "out");
    ships[0].armament.tick_fire_clock();
    tick(&mut combat, &mut ships, &NovaDisable);
    assert_ne!(ships[0].armament.fold().since_fire, 0, "no beam, no stamp");
    assert_ne!(ships[0].armament.fold().since_fire, NEVER_FIRED);
}

#[test]
fn a_folded_npc_that_unfolds_to_fire_holds_its_pick() {
    let mut combat = Combat::default();
    let mut ship = beaming();
    ship.hull.frames.flags = 0x00C2;
    ship.hull.frames.base_sets = 6;
    ship.armament = ship.armament.clone().with_fold_set(5);
    let mut ships = [ship];
    tick(&mut combat, &mut ships, &NovaDisable);
    assert_eq!(combat.beams(), [], "folded: no beam");
    let fold = ships[0].armament.fold();
    assert_eq!((fold.set, fold.dir, fold.since_fire), (5, -1, 0));
    ships[0].armament.fold_mut().set = 0;
    tick(&mut combat, &mut ships, &NovaDisable);
    assert_eq!(combat.beams().len(), 1, "unfolded: it fires");
}
