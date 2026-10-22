//! The fight's tests of the rating stretch (rdm cloaking-fidelity phase 6):
//! an NPC whose target is the player reloads its pick stretched by the
//! player's combat rating in the [`Field`] (`_FireAIShipWeapon`
//! @0x88fb3-0x8901b), and one targeting anyone else does not.

use super::*;
use crate::combat::ai_fire::PlayerRating;

/// A rating that stretches by x1.75: 0 against a yardstick of 1.
const NEW_PILOT: PlayerRating = PlayerRating {
    rating: 0,
    yardstick: 1,
};

/// A gun reloading in 20 ticks.
fn slow_gun() -> WeaponRecord {
    WeaponRecord {
        reload: 20,
        ..blaster()
    }
}

/// The reload left on NPC 1's pick after one tick in `field`, firing at
/// `target`, among B and the player.
fn reload_after_firing_at(target: ShipRef, field: Field) -> f32 {
    let mut combat = Combat::default();
    let mut ships = [
        Ship::at(1, 0.0, 0.0)
            .armed(slow_gun())
            .picking(128)
            .targeting(target),
        Ship::at(2, 100.0, 0.0),
        Ship {
            id: ShipRef::Player,
            ..Ship::at(9, 0.0, 100.0)
        },
    ];
    tick_in(
        &mut combat,
        &mut ships,
        (&Arsenal::default(), field),
        &mut NeverFires,
    );
    ships[0].armament.mounts()[0].reload
}

fn rated(player_rating: PlayerRating) -> Field {
    Field {
        player_rating,
        ..Field::default()
    }
}

#[test]
fn an_npc_targeting_the_player_reloads_stretched_by_the_fields_rating() {
    let stretched = reload_after_firing_at(ShipRef::Player, rated(NEW_PILOT));
    let plain = reload_after_firing_at(ShipRef::Player, Field::default());
    assert!(plain > 0.0, "it fired: {plain}");
    assert_eq!(stretched - plain, 15.0, "{stretched} against {plain}");
}

#[test]
fn an_npc_targeting_another_ship_reloads_unstretched() {
    let at_b = reload_after_firing_at(B, rated(NEW_PILOT));
    let plain = reload_after_firing_at(B, Field::default());
    assert!(plain > 0.0, "it fired: {plain}");
    assert_eq!(at_b, plain);
}

#[test]
fn a_default_field_stretches_nothing() {
    assert_eq!(Field::default().player_rating, PlayerRating::default());
    assert_eq!(PlayerRating::default().stretch(20.0), 20.0);
}
