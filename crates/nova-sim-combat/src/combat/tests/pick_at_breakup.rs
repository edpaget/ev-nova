//! An NPC's pick at a ship breaking up (rdm escort-fleet-fidelity-followups
//! phase 15): `_FireAIShipWeapon` (@0x8873d-0x89269) reads of its target
//! only whether there is one (+0x70 against -1), its position and whether
//! it is slot 0, never `_IsDying`, so a pick at a ship breaking up spawns
//! as at one intact. A destroyed ship's slot isn't active, and it stays no
//! target.

use super::*;

/// Breaking up, its death timer running.
const BREAKING_UP: Condition = Condition::Dying {
    ticks_left: 9,
    starting: false,
};

/// A turreted beam (3) reaching 300 pixels.
fn turret_beam() -> WeaponRecord {
    WeaponRecord {
        guidance: 3,
        count: 10,
        beam_length: 300,
        energy_dmg: 1,
        ..weapon(142)
    }
}

/// The fight after A, facing up, picks `record` once at B at (`x`, `y`)
/// in `condition`.
fn picked(record: WeaponRecord, x: f32, y: f32, condition: Condition) -> Combat {
    let id = record.id.0;
    let mut combat = Combat::default();
    let mut target = Ship::at(2, x, y);
    target.condition = condition;
    let mut ships = [
        Ship::at(1, 0.0, 0.0)
            .facing(0.0)
            .armed(record)
            .picking(id)
            .targeting(B),
        target,
    ];
    phase::handle_npcs(&mut combat, &mut ships, &NovaDisable);
    combat
}

/// A's shots in `combat`, each with its heading and target.
fn shots(combat: &Combat) -> Vec<(f32, Option<ShipRef>)> {
    combat
        .shots()
        .iter()
        .filter(|shot| shot.firer == A)
        .map(|shot| (shot.heading.round(), shot.target))
        .collect()
}

#[test]
fn a_turret_pick_at_a_ship_breaking_up_fires_at_it_as_at_one_intact() {
    // Turret (4) astern, rear turret (8) astern, homing (1) to the right.
    for (guidance, x, y) in [(4, 0.0, 150.0), (8, 0.0, 150.0), (1, 150.0, 0.0)] {
        let intact = shots(&picked(turret(guidance), x, y, Condition::Intact));
        assert_eq!(intact.len(), 1, "{guidance}: {intact:?}");
        let dying = shots(&picked(turret(guidance), x, y, BREAKING_UP));
        assert_eq!(dying, intact, "guidance {guidance}");
    }
}

#[test]
fn a_turreted_beam_pick_at_a_ship_breaking_up_holds_on_it() {
    let combat = picked(turret_beam(), 0.0, 100.0, BREAKING_UP);
    let beams: Vec<Aiming> = combat.beams().iter().map(|beam| beam.aiming).collect();
    assert_eq!(beams, [Aiming::Ship(B)]);
}

#[test]
fn a_front_turret_pick_at_a_ship_breaking_up_ahead_fires_at_it() {
    // B ahead and to the right, clear of the arc's seam at 0, so at it
    // isn't along the heading.
    let intact = shots(&picked(turret(7), 30.0, -100.0, Condition::Intact));
    assert_eq!(intact.len(), 1);
    assert_ne!(intact[0].0, 0.0, "led at B: {intact:?}");
    let dying = shots(&picked(turret(7), 30.0, -100.0, BREAKING_UP));
    assert_eq!(dying, intact);
}

#[test]
fn a_pick_at_a_destroyed_ship_fires_as_at_no_target() {
    for guidance in [4, 8, 1] {
        let combat = picked(turret(guidance), 0.0, 150.0, Condition::Destroyed);
        assert_eq!(shots(&combat), [], "guidance {guidance}");
    }
    let combat = picked(turret_beam(), 0.0, 100.0, Condition::Destroyed);
    assert_eq!(combat.beams(), []);
    let ahead = shots(&picked(turret(7), 30.0, -100.0, Condition::Destroyed));
    assert_eq!(ahead, [(0.0, None)], "along the heading at nothing");
}
