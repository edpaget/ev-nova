//! The fight's tests of the turret aims it records for the session
//! ([`turret_aim`](crate::combat::turret_aim),
//! [`Combat::take_turret_aims`]): a turret's aim from its exit before its
//! inaccuracy, a turreted beam's bearing, none for a fixed gun or a
//! turret that holds fire, and no change to the events a volley raises.

use super::*;
use crate::combat::aim::BLIND_REAR;
use crate::combat::turret_aim::TurretAim;

const A: ShipRef = ShipRef::Npc(NpcId(1));

/// Ship A at the centre facing north, armed with `record`, aimed at B off
/// its nose to the right and ahead.
fn shooters(record: WeaponRecord) -> [Ship; 2] {
    [
        Ship::at(1, 0.0, 0.0).armed(record).facing(0.0).targeting(B),
        Ship::at(2, 60.0, -90.0),
    ]
}

/// A turret (4) 9 degrees inaccurate: the test's chance draws it 8 off.
fn inaccurate_turret() -> WeaponRecord {
    WeaponRecord {
        inaccuracy: 9,
        ..turret(4)
    }
}

#[test]
fn a_turret_records_its_aim_from_its_exit_before_its_inaccuracy() {
    let mut combat = Combat::default();
    let mut ships = shooters(inaccurate_turret());
    phase::fire(&mut combat, &mut ships, A, &NovaDisable);
    let shot = *combat.shots().last().expect("a shot");
    let aims = combat.take_turret_aims();
    assert_eq!(aims.len(), 1, "{aims:?}");
    assert_eq!(aims[0].ship, A);
    assert_ne!(aims[0].heading, 0.0, "aimed off the nose");
    // `NeverFires` draws `Rand(18) − 9` as 8: the shot leaves 8 degrees
    // off the recorded aim, which is the aim before it.
    assert_eq!(shot.heading, normalized(aims[0].heading + 8.0));
}

#[test]
fn a_true_turrets_aim_is_its_shots_heading() {
    let mut combat = Combat::default();
    let mut ships = shooters(turret(4));
    phase::fire(&mut combat, &mut ships, A, &NovaDisable);
    let shot = *combat.shots().last().expect("a shot");
    assert_eq!(
        combat.take_turret_aims(),
        vec![TurretAim {
            ship: A,
            heading: shot.heading,
        }]
    );
}

#[test]
fn a_fixed_gun_records_no_aim_and_nor_does_a_turret_holding_fire() {
    let mut combat = Combat::default();
    let mut ships = shooters(gun());
    phase::fire(&mut combat, &mut ships, A, &NovaDisable);
    assert!(!combat.shots().is_empty(), "the gun fired");
    assert_eq!(combat.take_turret_aims(), vec![]);

    // B dead astern of a turret blind to the rear.
    let blind = WeaponRecord {
        flags: BLIND_REAR,
        ..turret(4)
    };
    let mut combat = Combat::default();
    let mut ships = [
        Ship::at(1, 0.0, 0.0).armed(blind).facing(0.0).targeting(B),
        Ship::at(2, 0.0, 100.0),
    ];
    phase::fire(&mut combat, &mut ships, A, &NovaDisable);
    assert!(combat.shots().is_empty(), "the turret held fire");
    assert_eq!(combat.take_turret_aims(), vec![]);
}

#[test]
fn a_turreted_beam_records_its_bearing_as_launched() {
    let mut combat = Combat::default();
    let mut ships = shooters(WeaponRecord {
        guidance: 3,
        count: 10,
        beam_length: 300,
        ..weapon(142)
    });
    phase::handle_npcs(&mut combat, &mut ships, &NovaDisable);
    assert_eq!(combat.beams().len(), 1, "the beam fired");
    let aims = combat.take_turret_aims();
    let bearing = f32::from(trig::intercept_angle(
        ships[0].state.position,
        ships[1].state.position,
    ));
    assert_eq!(
        aims,
        vec![TurretAim {
            ship: A,
            heading: bearing,
        }]
    );
}

#[test]
fn taking_the_aims_empties_them() {
    let mut combat = Combat::default();
    let mut ships = shooters(turret(4));
    phase::fire(&mut combat, &mut ships, A, &NovaDisable);
    assert_eq!(combat.take_turret_aims().len(), 1);
    assert_eq!(combat.take_turret_aims(), vec![]);
}

#[test]
fn a_turret_volley_raises_only_its_firing() {
    let mut combat = Combat::default();
    let mut ships = shooters(turret(4));
    phase::fire(&mut combat, &mut ships, A, &NovaDisable);
    let events = combat.take_events();
    assert!(
        matches!(
            events[..],
            [CombatEvent::Fired { ship: A, weapon: WeaponId(130), at, .. }]
                if at == ships[0].state.position
        ),
        "{events:?}"
    );
}
