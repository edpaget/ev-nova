//! The fight's tests of where a shot starts
//! ([`muzzle`](crate::combat::muzzle)): at its exit point, the engine's
//! reading, and its weapon's muzzle reach ahead of it, the other reading
//! of [`RuleKey::MountedMuzzles`](crate::RuleKey::MountedMuzzles), along
//! its aim for a turret and along the hull's angle for a fixed gun. The
//! exit taken, the shot's heading and its velocity are the same under
//! both; a beam and a fighter keep starting where they did.
//!
//! Every expected start is built from the exit points' own placement
//! ([`ExitPoints::offset`]) and the muzzle's ([`offset`]), not retyped.

use super::*;
use crate::combat::exits::ExitKind;
use crate::combat::muzzle::offset;
use crate::rulebook::{RuleKey, Rulebook};

/// The reach every weapon here has, in pixels.
const REACH: i16 = 6;

/// Guns 3 either side of the nose, 10 ahead, and a turret 2 aft on the
/// centre line, of 36 frames, with no compression and no z.
fn hull() -> HullSpec {
    let mut raw = ShipExits {
        up_compress: (100, 100),
        down_compress: (100, 100),
        frames: 36,
        ..ShipExits::default()
    };
    raw.points[0] = NOSE_GUNS;
    raw.points[1] = [(0, -2); 4];
    hull_of(raw)
}

/// `record` with a muzzle [`REACH`] ahead.
fn reaching(record: WeaponRecord) -> WeaponRecord {
    WeaponRecord {
        muzzle_reach: Some(REACH),
        ..record
    }
}

/// A turret (guidance 4) firing from the turret exits, reaching.
fn turret_gun() -> WeaponRecord {
    reaching(WeaponRecord {
        exit_type: 1,
        ..turret(4)
    })
}

/// A fixed gun firing from the gun exits, reaching.
fn fixed_gun() -> WeaponRecord {
    reaching(gun())
}

/// A fight whose shots start as `reading` says.
fn fight(reading: RuleSource) -> Combat {
    let mut combat = Combat::default();
    combat.set_mounted_muzzles(reading);
    combat
}

const READINGS: [RuleSource; 2] = [RuleSource::Engine, RuleSource::Bible];

/// The newest shot after `ships[0]` fires in its own phase.
fn fired(combat: &mut Combat, ships: &mut [Ship]) -> Shot {
    let firer = ships[0].id;
    phase::fire(combat, ships, firer, &NovaDisable);
    *combat.shots().last().expect("a shot")
}

/// Ship A at the centre, facing and last drawn facing `heading`, armed
/// with `record` and aimed at B, off its nose to the right and ahead.
fn shooters(record: WeaponRecord, heading: f32) -> [Ship; 2] {
    [
        Ship::at(1, 0.0, 0.0)
            .armed(record)
            .facing(heading)
            .drawn_at(heading)
            .targeting(B)
            .of(hull()),
        Ship::at(2, 60.0, -90.0),
    ]
}

/// The angle `ship`'s hull stands at in `combat`, and where its exit
/// `index` of `kind` stands from its centre, as the exit points place it.
fn exit_of(combat: &Combat, ship: &Ship, kind: ExitKind, index: usize) -> (f32, Vec2) {
    let exits = ship.hull.exits.as_ref().expect("exit points");
    let heading = ship.drawn_heading.unwrap_or(ship.state.heading);
    let angle = exits.hull_angle(heading, combat.exit_rules.turn);
    let at = ship.state.position + exits.offset(angle, kind, index, combat.exit_rules);
    (angle, at)
}

#[test]
fn a_turret_shot_starts_at_its_exit_by_the_engine_and_its_reach_along_its_aim_ahead() {
    let record = turret_gun();
    let spec = WeaponSpec::new(&record);
    let mut shots = Vec::new();
    for reading in READINGS {
        let mut combat = fight(reading);
        let mut ships = shooters(record, 0.0);
        let shot = fired(&mut combat, &mut ships);
        let (angle, exit) = exit_of(&combat, &ships[0], ExitKind::Turret, 0);
        let ahead = offset(reading.into(), &spec, angle, || shot.heading);
        assert!(
            close_to(start(&shot), exit + ahead),
            "{reading:?}: {:?} not {:?}",
            start(&shot),
            exit + ahead
        );
        shots.push((shot, exit));
    }
    let [(engine, exit), (ahead, _)] = shots[..] else {
        unreachable!()
    };
    assert!(close_to(start(&engine), exit), "by the engine, at the exit");
    let moved = start(&ahead) - exit;
    assert!(
        (moved.length() - f32::from(REACH)).abs() < 1e-3,
        "{moved:?}"
    );
    // Off the nose, along the aim, not the hull.
    assert!(
        close_to(
            moved,
            crate::flight::facing(ahead.heading) * f32::from(REACH)
        ),
        "{moved:?}"
    );
    assert_ne!(ahead.heading, 0.0, "aimed off the nose");
    assert_eq!(
        ahead.heading, engine.heading,
        "aimed from the exit both ways"
    );
    assert_eq!(ahead.velocity, engine.velocity);
}

#[test]
fn a_fixed_shot_starts_its_reach_along_the_hull_s_angle_under_either_hull_turn() {
    let record = fixed_gun();
    let spec = WeaponSpec::new(&record);
    // Heading 7 stands on frame 0 by the engine, and at 7 continuously.
    for turn in READINGS {
        let mut starts = Vec::new();
        for reading in READINGS {
            let mut combat = fight(reading);
            combat.set_continuous_headings(turn);
            let mut ships = shooters(record, 7.0);
            let shot = fired(&mut combat, &mut ships);
            let (angle, exit) = exit_of(&combat, &ships[0], ExitKind::Gun, 0);
            let ahead = offset(reading.into(), &spec, angle, || unreachable!());
            assert!(
                close_to(start(&shot), exit + ahead),
                "{turn:?} {reading:?}: {:?} not {:?}",
                start(&shot),
                exit + ahead
            );
            starts.push((start(&shot), angle, shot.heading));
        }
        let moved = starts[1].0 - starts[0].0;
        let along = crate::flight::facing(starts[1].1) * f32::from(REACH);
        assert!(close_to(moved, along), "{turn:?}: {moved:?} not {along:?}");
        assert_eq!(starts[0].2, starts[1].2, "{turn:?}: the same heading");
    }
}

#[test]
fn a_nearest_exit_weapon_takes_the_same_exit_under_both_readings() {
    let record = WeaponRecord {
        flags3: crate::combat::weapon::NEAREST_EXIT,
        ..fixed_gun()
    };
    let spec = WeaponSpec::new(&record);
    let mut backs = Vec::new();
    for reading in READINGS {
        let mut combat = fight(reading);
        let mut ships = shooters(record, 0.0);
        let shot = fired(&mut combat, &mut ships);
        let (angle, _) = exit_of(&combat, &ships[0], ExitKind::Gun, 0);
        backs.push(start(&shot) - offset(reading.into(), &spec, angle, || unreachable!()));
    }
    assert!(close_to(backs[0], backs[1]), "{backs:?}");
    // B is off to the right: the right-hand gun, 3 to starboard.
    let (_, right) = exit_of(
        &fight(RuleSource::Engine),
        &shooters(record, 0.0)[0],
        ExitKind::Gun,
        0,
    );
    assert!(close_to(backs[0], right), "{backs:?} not {right:?}");
}

#[test]
fn the_player_s_shot_starts_ahead_as_an_npc_s_does() {
    let record = fixed_gun();
    let player = |ships: [Ship; 2]| {
        let [npc, other] = ships;
        [
            Ship {
                id: ShipRef::Player,
                fleet: ShipRef::Player,
                ..npc
            },
            other,
        ]
    };
    for firer in [false, true] {
        let mut starts = Vec::new();
        for reading in READINGS {
            let mut combat = fight(reading);
            let ships = shooters(record, 90.0);
            let mut ships = if firer { player(ships) } else { ships };
            let shot = fired(&mut combat, &mut ships);
            starts.push(start(&shot));
        }
        let moved = starts[1] - starts[0];
        let along = crate::flight::facing(90.0) * f32::from(REACH);
        assert!(close_to(moved, along), "player {firer}: {moved:?}");
    }
}

#[test]
fn a_weapon_with_no_reach_starts_at_its_exit_under_both_readings() {
    for record in [
        gun(),
        WeaponRecord {
            exit_type: 1,
            ..turret(4)
        },
    ] {
        let starts: Vec<Vec2> = READINGS
            .map(|reading| {
                let mut combat = fight(reading);
                start(&fired(&mut combat, &mut shooters(record, 0.0)))
            })
            .into();
        assert!(close_to(starts[0], starts[1]), "{starts:?}");
    }
}

#[test]
fn a_beam_starts_at_its_exit_under_both_readings() {
    let beams: Vec<Beam> = READINGS
        .map(|reading| {
            let mut combat = fight(reading);
            let mut ships = [Ship::at(1, 0.0, 0.0)
                .armed(reaching(exit_beam(0)))
                .facing(0.0)
                .of(exits_at(3, [(0, 20); 4]))];
            phase::handle_npcs(&mut combat, &mut ships, &NovaDisable);
            phase::close_tick(&mut combat, &mut ships, &NovaDisable);
            combat.beams()[0]
        })
        .into();
    assert_eq!(beams[0], beams[1]);
    assert!(close_to(beams[0].start, Vec2::new(0.0, -20.0)), "{beams:?}");
}

#[test]
fn a_fighter_leaves_its_carrier_as_it_did_under_both_readings() {
    let sorties: Vec<Vec<Sortie>> = READINGS
        .map(|reading| {
            let mut combat = fight(reading);
            let mut ships = carrier();
            ships[0].armament = Armament::new([(WeaponSpec::new(&reaching(carrier_bay())), 1)]);
            phase::defend_npcs(&mut combat, &mut ships, &NovaDisable);
            combat.take_sorties()
        })
        .into();
    assert_eq!(sorties[0].len(), 1, "{sorties:?}");
    assert_eq!(sorties[0], sorties[1]);
}

#[test]
fn set_hull_rules_reads_mounted_muzzles() {
    let start_under = |rules: &Rulebook| {
        let mut combat = Combat::default();
        combat.set_hull_rules(rules);
        start(&fired(&mut combat, &mut shooters(fixed_gun(), 90.0)))
    };
    let ahead = Rulebook::default().with_override(RuleKey::MountedMuzzles, RuleSource::Bible);
    let moved = start_under(&ahead) - start_under(&Rulebook::default());
    let along = crate::flight::facing(90.0) * f32::from(REACH);
    assert!(close_to(moved, along), "{moved:?}");
}
