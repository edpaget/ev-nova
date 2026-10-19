//! The fight's tests of what steps a ship's exit words (rdm
//! weapons-fidelity phase 18): a beam of `ExitType` -1 steps the fold set
//! (+0xc8fc) as `_SpawnBeam` indexes its cursors with no range check
//! (@0x450fa-0x4523d), a hull with no exit points still steps its cursors
//! as `_ModifyShotStartPosition` and `_SpawnBeam` do, and point defence of
//! `Flags3` 0x0010 takes the exit nearest its missile.

use super::*;
use crate::combat::exits::StrayWord;

/// A beam of `ExitType` -1 firing ahead for 10 ticks, 300 pixels long.
fn stray_beam() -> WeaponRecord {
    WeaponRecord {
        exit_type: -1,
        ..exit_beam(0)
    }
}

/// Beam exits 30 to either side, so a beam from one is plain to see.
fn beam_exits() -> HullSpec {
    exits_at(3, [(30, 0), (-30, 0), (30, 0), (-30, 0)])
}

/// The beam `ship`, facing up at the centre, fires in a tick of `combat`,
/// and the ship after it.
fn fired(combat: &mut Combat, ship: Ship) -> (Beam, Ship) {
    let mut ships = [ship, Ship::at(2, 0.0, -100.0)];
    tick(combat, &mut ships, &NovaDisable);
    let beam = *combat
        .beams()
        .iter()
        .find(|beam| beam.firer == ships[0].id)
        .expect("a beam");
    let [ship, _] = ships;
    (beam, ship)
}

#[test]
fn a_beam_of_exit_type_minus_1_steps_the_fold_set_and_starts_at_the_centre() {
    let nearest = WeaponRecord {
        flags3: crate::combat::weapon::NEAREST_EXIT,
        ..stray_beam()
    };
    for (record, hull, what) in [
        (stray_beam(), beam_exits(), "beam exits"),
        (nearest, beam_exits(), "nearest exit, with a target"),
        (stray_beam(), HullSpec::default(), "no exits"),
    ] {
        let mut combat = Combat::default();
        let ship = Ship::at(1, 0.0, 0.0)
            .armed(record)
            .facing(0.0)
            .targeting(B)
            .of(hull);
        let (beam, ship) = fired(&mut combat, ship);
        assert!(close_to(beam.start, Vec2::ZERO), "{what}: {beam:?}");
        assert_eq!((beam.kind, beam.exit), (None, None), "{what}");
        assert_eq!(ship.armament.fold_set(), 1, "{what}");
        assert_eq!(
            *ship.armament.exit_cursors(),
            ExitCursors::default(),
            "{what}: no cursor moved"
        );
    }
}

#[test]
fn a_beam_of_an_exit_type_out_of_range_but_minus_1_steps_nothing_by_the_engine() {
    for exit_type in [-2, 4, 6] {
        for (hull, what) in [
            (beam_exits(), "beam exits"),
            (HullSpec::default(), "no exits"),
        ] {
            let mut combat = Combat::default();
            combat.set_stray_beam_exit(RuleSource::Engine);
            let record = WeaponRecord {
                exit_type,
                ..exit_beam(0)
            };
            let ship = Ship::at(1, 0.0, 0.0)
                .armed(record)
                .facing(0.0)
                .targeting(B)
                .of(hull);
            let (beam, ship) = fired(&mut combat, ship);
            let what = format!("ExitType {exit_type}, {what}");
            assert!(close_to(beam.start, Vec2::ZERO), "{what}: {beam:?}");
            assert_eq!((beam.kind, beam.exit), (None, None), "{what}");
            assert_eq!(ship.armament.fold_set(), 0, "{what}");
            assert_eq!(
                *ship.armament.exit_cursors(),
                ExitCursors::default(),
                "{what}: no cursor moved"
            );
        }
    }
}

#[test]
fn a_fold_set_steps_round_from_3_to_0_beam_by_beam() {
    let mut combat = Combat::default();
    let mut ships = [Ship::at(1, 0.0, 0.0)
        .armed(WeaponRecord {
            count: 1,
            ..stray_beam()
        })
        .facing(0.0)];
    let mut seen = Vec::new();
    for _ in 0..4 {
        tick(&mut combat, &mut ships, &NovaDisable);
        seen.push(ships[0].armament.fold_set());
    }
    assert_eq!(seen, [1, 2, 3, 0]);
}

#[test]
fn by_the_other_reading_a_stray_beam_steps_nothing() {
    let mut combat = Combat::default();
    combat.set_stray_beam_exit(RuleSource::Bible);
    let ship = Ship::at(1, 0.0, 0.0)
        .armed(stray_beam())
        .facing(0.0)
        .of(beam_exits());
    let (beam, ship) = fired(&mut combat, ship);
    assert!(close_to(beam.start, Vec2::ZERO), "{beam:?}");
    assert_eq!(ship.armament.fold_set(), 0);
}

#[test]
fn a_stray_beam_the_caps_refuse_steps_nothing() {
    let mut combat = Combat {
        beams: held_beams(MAX_BEAMS),
        ..Combat::default()
    };
    combat.set_combat_caps(RuleSource::Engine);
    let mut ships = [Ship::at(1, 0.0, 0.0).armed(stray_beam()).facing(0.0)];
    tick(&mut combat, &mut ships, &NovaDisable);
    assert_eq!(ships[0].armament.fold_set(), 0);
}

#[test]
fn a_gun_from_a_hull_with_no_exits_fires_from_the_centre_and_steps_its_cursor() {
    let mut combat = Combat::default();
    let mut ships = [Ship::at(1, 50.0, 50.0).armed(gun()).facing(0.0)];
    for next in [1, 2] {
        let from = next_start(&mut combat, &mut ships);
        assert!(close_to(from, Vec2::new(50.0, 50.0)), "{from:?}");
        assert_eq!(ships[0].armament.exit_cursors().next(ExitKind::Gun), next);
    }
}

#[test]
fn a_beam_from_a_hull_with_no_exits_starts_at_the_centre_and_steps_its_cursor() {
    let mut combat = Combat::default();
    let ship = Ship::at(1, 0.0, 0.0).armed(exit_beam(0)).facing(0.0);
    let (beam, ship) = fired(&mut combat, ship);
    assert!(close_to(beam.start, Vec2::ZERO), "{beam:?}");
    assert_eq!((beam.kind, beam.exit), (None, None));
    assert_eq!(ship.armament.exit_cursors().next(ExitKind::Beam), 1);
    assert_eq!(ship.armament.fold_set(), 0);
}

/// [`defended`] with its point defence of `Flags3` 0x0010.
fn defended_nearest() -> [Ship; 2] {
    let [defender, attacker] = defended();
    let nearest = WeaponRecord {
        flags3: crate::combat::weapon::NEAREST_EXIT,
        ..exit_defence()
    };
    [defender.armed(nearest), attacker]
}

#[test]
fn nearest_exit_point_defence_fires_from_the_exit_nearest_its_missile() {
    // The turret exits are 10, 20, 30 and 40 to the right, the missile
    // coming from (150, 100): exit 3 is nearest, not the cursor's next.
    for source in [RuleSource::Engine, RuleSource::Bible] {
        let mut combat = Combat::default();
        combat.set_pd_exit_step(source);
        let mut ships = defended_nearest();
        missile_in_flight(&mut combat, &mut ships);
        tick(&mut combat, &mut ships, &NovaDisable);
        let fired = defence_shots(&combat);
        assert_eq!(fired.len(), 1, "{source:?}: {:?}", combat.shots());
        let (at, heading) = fired[0];
        assert!(close_to(at, Vec2::new(40.0, 0.0)), "{source:?}: {at:?}");
        let bearing = crate::trig::intercept_angle(at, Vec2::new(150.0, 100.0));
        assert_eq!(heading, f32::from(bearing), "{source:?}");
        assert_eq!(
            ships[0].armament.exit_cursors().next(ExitKind::Turret),
            0,
            "{source:?}: on from the nearest, 3 + 1"
        );
    }
}

#[test]
fn an_armament_takes_carried_cursors_and_fold_set_keeping_its_weapons() {
    let built = Armament::new([(WeaponSpec::new(&gun()), 2)]);
    assert_eq!(built.fold_set(), 0, "0 when built");
    let mut cursors = ExitCursors::default();
    cursors.step(ExitKind::Turret);
    let carried = built.clone().with_exit_cursors(cursors).with_fold_set(3);
    assert_eq!(*carried.exit_cursors(), cursors);
    assert_eq!(carried.fold_set(), 3);
    assert_eq!(carried.mounts(), built.mounts(), "the same weapons");
}

/// A beam of `ExitType` 5, which lands on +0xc908.
fn pod_beam() -> WeaponRecord {
    WeaponRecord {
        exit_type: 5,
        ..exit_beam(0)
    }
}

/// `ship` as the player.
fn as_player(mut ship: Ship) -> Ship {
    ship.id = ShipRef::Player;
    ship.fleet = ShipRef::Player;
    ship
}

#[test]
fn a_beam_of_exit_type_5_raises_a_pod_step_for_its_firer_by_the_engine() {
    for (ship, firer) in [
        (Ship::at(1, 0.0, 0.0), ShipRef::Npc(NpcId(1))),
        (as_player(Ship::at(1, 0.0, 0.0)), ShipRef::Player),
    ] {
        let mut combat = Combat::default();
        combat.set_stray_beam_exit(RuleSource::Engine);
        let ship = ship.armed(pod_beam()).facing(0.0).of(beam_exits());
        let (beam, ship) = fired(&mut combat, ship);
        assert!(close_to(beam.start, Vec2::ZERO), "{firer:?}: {beam:?}");
        assert_eq!(ship.armament.fold_set(), 0, "{firer:?}");
        assert_eq!(
            combat.take_stray_steps(),
            [(firer, StrayWord::Pods)],
            "{firer:?}"
        );
        assert_eq!(combat.take_stray_steps(), [], "taking empties the list");
    }
}

#[test]
fn by_the_other_reading_a_beam_of_exit_type_5_raises_no_pod_step() {
    let mut combat = Combat::default();
    combat.set_stray_beam_exit(RuleSource::Bible);
    let ship = Ship::at(1, 0.0, 0.0).armed(pod_beam()).facing(0.0);
    fired(&mut combat, ship);
    assert_eq!(combat.take_stray_steps(), []);
}

#[test]
fn a_beam_of_exit_type_5_the_caps_refuse_raises_no_pod_step() {
    let mut combat = Combat {
        beams: held_beams(MAX_BEAMS),
        ..Combat::default()
    };
    combat.set_combat_caps(RuleSource::Engine);
    let mut ships = [Ship::at(1, 0.0, 0.0).armed(pod_beam()).facing(0.0)];
    tick(&mut combat, &mut ships, &NovaDisable);
    assert_eq!(combat.take_stray_steps(), []);
}

#[test]
fn a_beam_of_exit_type_minus_1_raises_no_stray_step() {
    let mut combat = Combat::default();
    let ship = Ship::at(1, 0.0, 0.0).armed(stray_beam()).facing(0.0);
    let (_, ship) = fired(&mut combat, ship);
    assert_eq!(ship.armament.fold_set(), 1);
    assert_eq!(combat.take_stray_steps(), []);
}
