//! A homing shot confused by sensor interference (jam 999, `_SpawnShot`
//! @0x3eb43) still hits its target: `_ShotCanHitShip` rejects only jam
//! 998 (@0x447f7), and neither `_ShipCollisionHandler` (@0x37002) nor
//! `_CheckShotProximities` (@0x375b2) tests the jam itself.

use super::collision_walk::{BESIDE_C, INTO_C, fused, walk};
use super::shot_slots::{handle_in, seeking_shell, spent_shell, unguided_sub};
use super::*;
use crate::combat::weapon::PROXIMITY_BY_OTHERS;

/// A's interfered `Seeker` 0x0008 missile of `record`, fired at C in a
/// field of `Interference` 100, its last move from `from` to `to`, among
/// A, a ship B in the way and C at (0, -100): the strikes the collision
/// passes make.
fn interfered_into_c(
    record: WeaponRecord,
    (from, to): (Vec2, Vec2),
) -> Vec<(ShipRef, Option<ShipRef>)> {
    let mut combat = Combat::default();
    let mut ships = [
        Ship::at(1, -300.0, 0.0)
            .armed(WeaponRecord {
                seeker: SENSOR_INTERFERENCE,
                ..record
            })
            .targeting(C),
        Ship::at(2, 0.0, -40.0),
        Ship::at(3, 0.0, -100.0),
    ];
    let field = Field {
        interference: 100,
        ..Field::default()
    };
    handle_npcs_in_field(
        &mut combat,
        &mut ships,
        (&Arsenal::default(), field),
        &mut NeverFires,
    );
    let shot = &mut combat.shots_mut()[0];
    assert_eq!((shot.jam, shot.target), (Jam::Interfered, Some(C)));
    shot.age = 1;
    shot.moved_from = Some(from);
    shot.position = to;
    walk(&mut combat, &mut ships)
}

#[test]
fn an_interfered_missile_hits_its_target_by_contact() {
    assert_eq!(interfered_into_c(missile(), INTO_C), [(C, Some(A))]);
}

#[test]
fn an_interfered_missile_hits_its_target_by_its_fuse() {
    assert_eq!(
        interfered_into_c(fused(missile()), BESIDE_C),
        [(C, Some(A))]
    );
}

#[test]
fn an_interfered_missile_passes_through_a_ship_it_is_not_aimed_at() {
    // Through B at (0, -40) and on: homing, it may hit only C.
    let through_b = (Vec2::new(0.0, -20.0), Vec2::new(0.0, -45.0));
    assert_eq!(interfered_into_c(missile(), through_b), []);
}

#[test]
fn an_interfered_seeking_sub_munition_finds_and_hits_a_ship() {
    // A homing `Seeker` 0x0008 sub-munition, released in a field of
    // `Interference` 100, rolls jam 999 as `_SpawnShot` seats it
    // (@0x3eb43), seeks the nearest ship, B, and strikes it on contact:
    // `_ShotCanHitShip` rejects only jam 998 (@0x447f7).
    let homing = WeaponRecord {
        guidance: 1,
        seeker: SENSOR_INTERFERENCE,
        flags2: PROXIMITY_BY_OTHERS,
        ..unguided_sub()
    };
    let (shell, arsenal) = seeking_shell(1, homing);
    let near = Ship {
        hull: HullSpec {
            miss_class: 100,
            ..HullSpec::default()
        },
        ..Ship::at(2, 0.0, 350.0)
    };
    let mut ships = [Ship::at(1, -5000.0, 0.0), near];
    let mut combat = Combat::default();
    combat.put_in_flight(Shot {
        aimed_at: Some(B),
        ..spent_shell(shell)
    });
    let field = Field {
        interference: 100,
        ..Field::default()
    };
    handle_in(&mut combat, &mut ships, &arsenal, field);
    let sub = &mut combat.shots_mut()[0];
    assert_eq!((sub.jam, sub.target), (Jam::Interfered, Some(B)));
    sub.age = 1;
    sub.moved_from = Some(Vec2::new(0.0, 330.0));
    sub.position = Vec2::new(0.0, 348.0);
    assert_eq!(walk(&mut combat, &mut ships), [(B, Some(A))]);
}
