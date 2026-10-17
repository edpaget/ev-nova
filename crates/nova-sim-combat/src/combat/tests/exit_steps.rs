//! The fight's tests of what steps a ship's exit words (rdm
//! weapons-fidelity phase 18): a beam of `ExitType` -1 steps the fold set
//! (+0xc8fc) as `_SpawnBeam` indexes its cursors with no range check
//! (@0x450fa-0x4523d), a hull with no exit points still steps its cursors
//! as `_ModifyShotStartPosition` and `_SpawnBeam` do, and point defence of
//! `Flags3` 0x0010 takes the exit nearest its missile.

use super::*;

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
        tick(&mut combat, &mut ships, &NovaDisable);
        let fired = defence_shots(&combat);
        assert_eq!(fired.len(), 1, "{source:?}: {:?}", combat.shots());
        let (at, heading) = fired[0];
        assert!(close_to(at, Vec2::new(40.0, 0.0)), "{source:?}: {at:?}");
        let bearing = aim::bearing(at, Vec2::new(150.0, 100.0));
        assert!((heading - bearing).abs() < 1e-3, "{source:?}: {heading}");
        assert_eq!(
            ships[0].armament.exit_cursors().next(ExitKind::Turret),
            0,
            "{source:?}: on from the nearest, 3 + 1"
        );
    }
}
