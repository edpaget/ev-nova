//! Aiming: which way a weapon fires, and at what, from its guidance and
//! its firer's target.
//!
//! The values are the original's (`_FirePlayerWeapon`, `_SpawnShot`,
//! `_CalcInterceptAngle`, `_CalcLeadAngle` and `_TurretBlindSpot` in the
//! `EV Nova` executable):
//!
//! - The bearing to a ship is the plain heading from the firer to it.
//! - The lead angle aims where the target will be when a shot at `Speed`
//!   gets there: with t the distance over the shot's speed, at the
//!   target's position plus its velocity relative to the firer's, times t.
//!   A shot of no speed aims at the bearing.
//! - A turret is blind in the sectors its weapon's `Flags` or its ship's
//!   `Flags` mark ([`blind`]): the front up to [`FRONT_ARC`] off the nose
//!   (0x1000), the sides up to [`SIDE_ARC`] (0x2000), and the rear beyond
//!   (0x4000). 3 and 4 always test them (`_FirePlayerWeapon` @0x629fe,
//!   @0x62a94). 7 and 8 test them as the reading [`aim`] is given says
//!   ([`RuleKey::QuadrantTurretBlindSpots`](crate::RuleKey::QuadrantTurretBlindSpots)):
//!   by the engine, as `_FirePlayerWeapon` and `_FireAIShipWeapon` fire
//!   them, never (@0x62aeb-0x62c38); by the other reading, as
//!   `_AIFireTurret` picks them (@0x80d0b) and the Bible has it ("Turreted
//!   weapon has a blind spot"), as 3 and 4 do. The fight passes the key's
//!   reading for the player's trigger, and the testing reading for an
//!   escort's formation trigger, whose turrets the original's AI picks.
//! - What each guidance fires ([`aim`]):
//!   - -1, 0, 5 and 6 fire along the ship's heading, at nothing.
//!   - 1 fires along the heading at the target, when there is one, and
//!     without one too; so does a fighter bay (99 carrying a ship), whose
//!     fighter leaves along the heading with the firer's target.
//!   - 3 (a turreted beam) fires along the bearing, and 4 (a turret) at
//!     the lead angle, only with a target not in a blind spot.
//!   - 7 fires at the lead angle at a target within [`FRONT_ARC`] of the
//!     nose, and otherwise (no target, or one out of the arc) straight
//!     ahead at nothing; one testing blind spots fires nothing at a target
//!     in its arc but in a blind spot.
//!   - 8 fires at the lead angle at a target within [`FRONT_ARC`] of the
//!     tail, and otherwise not at all.
//!   - 9 and 10 never fire on the trigger: only point defence fires them
//!     (see [`defence`](super::defence)).
//!   - A turret that does not fire spends nothing.
//! - Whether a turret fires, by its blind spots and arcs, is judged from
//!   the ship's centre; the lead angle and the bearing it fires along are
//!   measured once, from where the shot leaves, its exit point or the
//!   centre ([`Pointing`]).

use super::ShipRef;
use super::hull::HullSpec;
use super::projectile::Target;
use super::weapon::{Guidance, WeaponSpec};
use crate::flight::{ShipState, heading_of, shortest_turn};
use crate::geometry::Vec2;
use crate::rulebook::RuleSource;

/// How far off the nose, in degrees, the front sector reaches; a
/// front-quadrant turret's arc, and a rear-quadrant turret's off the tail.
pub const FRONT_ARC: f32 = 45.0;
/// How far off the nose, in degrees, the side sectors reach.
pub const SIDE_ARC: f32 = 135.0;
/// `Flags`: a turret blind in front.
pub const BLIND_FRONT: u16 = 0x1000;
/// `Flags`: a turret blind to the sides.
pub const BLIND_SIDES: u16 = 0x2000;
/// `Flags`: a turret blind to the rear.
pub const BLIND_REAR: u16 = 0x4000;

/// Which way a weapon fires, and at what.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Aim {
    /// Which way it points, from wherever it leaves, before its
    /// inaccuracy.
    pub pointing: Pointing,
    /// The ship it fires at, if any.
    pub target: Option<ShipRef>,
}

impl Aim {
    /// Fired along `heading`, in degrees, at `target`.
    #[must_use]
    pub const fn along(heading: f32, target: Option<ShipRef>) -> Self {
        Self {
            pointing: Pointing::Along(heading),
            target,
        }
    }
}

/// Which way a weapon points from where it leaves its ship: the ship's
/// centre, or the exit point it fires from
/// ([`exits`](super::exits)). The fight works the heading out once it
/// knows that origin ([`Pointing::heading_from`]), as `_SpawnShot` leads
/// from the shot's exit (@0x3ecdc) and `_HandleShipPointDefense` aims at
/// its missile from it (`_CalcInterceptAngle` @0x39790).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Pointing {
    /// Along this heading, in degrees, wherever it leaves from.
    Along(f32),
    /// At this point: the bearing to it from where it leaves.
    At(Vec2),
    /// At `target`'s lead angle for a shot of `speed` pixels a tick, from
    /// where it leaves ([`lead`]).
    Leading {
        /// The ship it leads, as it was when it was aimed at.
        target: Target,
        /// The shot's speed.
        speed: f32,
    },
}

impl Pointing {
    /// The heading it points along from `origin`, before any inaccuracy.
    #[must_use]
    pub fn heading_from(&self, origin: &ShipState) -> f32 {
        match self {
            Self::Along(heading) => *heading,
            Self::At(point) => bearing(origin.position, *point),
            Self::Leading { target, speed } => lead(origin, target, *speed),
        }
    }
}

/// The heading from `from` to `to`; none for a point on top of `from`.
#[must_use]
pub fn bearing(from: Vec2, to: Vec2) -> f32 {
    let off = to - from;
    if off == Vec2::ZERO {
        0.0
    } else {
        heading_of(off)
    }
}

/// How far, in degrees from 0 to 180, `bearing` is off `heading`.
#[must_use]
pub fn angle_off(heading: f32, bearing: f32) -> f32 {
    shortest_turn(heading, bearing).abs()
}

/// How far `bearing` is off a quadrant turret's `reference` (its heading
/// for a front turret, 7, or `(heading + 180) mod 360` for a rear one,
/// 8), as the rulebook's
/// [`RuleKey::QuadrantTurretArc`](crate::RuleKey::QuadrantTurretArc)
/// entry `arc` says: by the engine, made absolute and taken mod 360 with
/// no fold past 180 (`_AIFireTurret` @0x80c3d-0x80cf3, `_FireAIShipWeapon`
/// @0x88ac4-0x88b7f), so a reference near 0 holds fire across the seam;
/// by the other reading, folded ([`angle_off`]).
#[must_use]
pub fn quadrant_off(reference: f32, bearing: f32, arc: RuleSource) -> f32 {
    match arc {
        RuleSource::Engine => (bearing - reference).abs() % 360.0,
        RuleSource::Bible => angle_off(reference, bearing),
    }
}

/// A quadrant turret's reference: the heading for a front turret, and
/// `(heading + 180) mod 360` for a rear one.
#[must_use]
pub fn quadrant_reference(guidance: Guidance, heading: f32) -> f32 {
    if guidance == Guidance::RearTurret {
        (heading + 180.0) % 360.0
    } else {
        heading
    }
}

/// Whether a turret blind where `blind_spots` say is blind `off` degrees
/// off the nose (see the module docs).
#[must_use]
pub fn blind(blind_spots: u16, off: f32) -> bool {
    let sector = if off <= FRONT_ARC {
        BLIND_FRONT
    } else if off <= SIDE_ARC {
        BLIND_SIDES
    } else {
        BLIND_REAR
    };
    blind_spots & sector != 0
}

/// The lead angle from `firer` to `target` for a shot of `speed` pixels a
/// tick (see the module docs).
#[must_use]
pub fn lead(firer: &ShipState, target: &Target, speed: f32) -> f32 {
    if speed <= 0.0 {
        return bearing(firer.position, target.position);
    }
    let ticks = (target.position - firer.position).length() / speed;
    let ahead = target.position + (target.velocity - firer.velocity) * ticks;
    bearing(firer.position, ahead)
}

/// Which way `spec` fires from `firer`, of `hull`, at its `target`, if it
/// fires at all, its quadrant turrets testing their blind spots as
/// `quadrant_blind` says (see the module docs).
#[must_use]
pub fn aim(
    spec: &WeaponSpec,
    firer: &ShipState,
    hull: &HullSpec,
    target: Option<&Target>,
    quadrant_blind: RuleSource,
) -> Option<Aim> {
    let ahead = Aim::along(firer.heading, None);
    let blind_spots = spec.flags | hull.blind_spots;
    let quadrant = matches!(spec.guidance, Guidance::FrontTurret | Guidance::RearTurret);
    let tested = !quadrant || quadrant_blind == RuleSource::Bible;
    // The target, its bearing's angle off the nose, and whether it is in a
    // blind spot that holds the weapon.
    let sighted = target.map(|target| {
        let off = angle_off(firer.heading, bearing(firer.position, target.position));
        (target, off, tested && blind(blind_spots, off))
    });
    match spec.guidance {
        Guidance::Homing | Guidance::FighterBay => Some(Aim {
            target: target.map(|target| target.ship),
            ..ahead
        }),
        Guidance::TurretBeam => sighted
            .filter(|&(_, _, blind)| !blind)
            .map(|(target, _, _)| on_bearing(target)),
        Guidance::Turret => sighted
            .filter(|&(_, _, blind)| !blind)
            .map(|(target, _, _)| led(spec, target)),
        Guidance::FrontTurret => match sighted {
            Some((target, off, blind)) if off <= FRONT_ARC => (!blind).then(|| led(spec, target)),
            _ => Some(ahead),
        },
        Guidance::RearTurret => sighted
            .filter(|&(_, off, blind)| off >= 180.0 - FRONT_ARC && !blind)
            .map(|(target, _, _)| led(spec, target)),
        Guidance::PointDefence | Guidance::PointDefenceBeam => None,
        Guidance::Unguided
        | Guidance::Beam
        | Guidance::FreefallBomb
        | Guidance::Rocket
        | Guidance::Other(_) => Some(ahead),
    }
}

/// A turret of `spec` fired at `target`, leading it from where the shot
/// leaves (4, 7 and 8; `_SpawnShot` @0x3e685-0x3e6cf, @0x3ecdc).
#[must_use]
pub fn led(spec: &WeaponSpec, target: &Target) -> Aim {
    Aim {
        pointing: Pointing::Leading {
            target: *target,
            speed: spec.speed,
        },
        target: Some(target.ship),
    }
}

/// A turreted beam (3) fired at `target`, along the bearing to it.
#[must_use]
pub fn on_bearing(target: &Target) -> Aim {
    Aim {
        pointing: Pointing::At(target.position),
        target: Some(target.ship),
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::catalog::WeaponRecord;
    use crate::combat::hull::Condition;
    use crate::testkit::weapon;
    use crate::traffic::npc::NpcId;

    const TARGET: ShipRef = ShipRef::Npc(NpcId(2));

    /// A ship at the centre, at rest, facing `heading`.
    fn firer(heading: f32) -> ShipState {
        ShipState {
            heading,
            ..ShipState::default()
        }
    }

    /// The target at (`x`, `y`), moving (`vx`, `vy`).
    fn target(x: f32, y: f32, vx: f32, vy: f32) -> Target {
        Target {
            ship: TARGET,
            ship_type: crate::catalog::ShipId(128),
            fleet: TARGET,
            ties: crate::combat::sides::Ties::default(),
            position: Vec2::new(x, y),
            velocity: Vec2::new(vx, vy),
            radius: 10.0,
            sprite_x: 30.0,
            sheet: true,
            condition: Condition::Intact,
            miss_class: 100,
        }
    }

    /// A weapon of `guidance` with `flags`, 10 pixels a tick.
    fn spec(guidance: i16, flags: u16) -> WeaponSpec {
        WeaponSpec::new(&WeaponRecord {
            guidance,
            flags,
            speed: 1000,
            ..weapon(130)
        })
    }

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-3
    }

    #[test]
    fn the_bearing_is_the_heading_to_the_point() {
        assert_eq!(bearing(Vec2::ZERO, Vec2::new(0.0, -50.0)), 0.0);
        assert!(close(bearing(Vec2::ZERO, Vec2::new(50.0, 0.0)), 90.0));
        assert!(close(
            bearing(Vec2::new(10.0, 10.0), Vec2::new(10.0, 60.0)),
            180.0
        ));
        assert!(close(bearing(Vec2::ZERO, Vec2::new(-5.0, -5.0)), 315.0));
        assert_eq!(bearing(Vec2::new(3.0, 4.0), Vec2::new(3.0, 4.0)), 0.0);
    }

    #[test]
    fn only_turrets_4_7_and_8_lead() {
        let leads = |raw| {
            [0.0, 180.0].into_iter().any(|heading| {
                aimed(&spec(raw, 0), heading, 0, true)
                    .is_some_and(|aim| matches!(aim.pointing, Pointing::Leading { .. }))
            })
        };
        let leading: Vec<i16> = (-1..=10).filter(|&raw| leads(raw)).collect();
        assert_eq!(leading, [4, 7, 8]);
    }

    #[test]
    fn the_angle_off_is_the_short_way_either_side() {
        assert_eq!(angle_off(0.0, 45.0), 45.0);
        assert_eq!(angle_off(45.0, 0.0), 45.0);
        assert_eq!(angle_off(350.0, 10.0), 20.0);
        assert_eq!(angle_off(10.0, 350.0), 20.0);
        assert_eq!(angle_off(0.0, 180.0), 180.0);
        assert_eq!(angle_off(90.0, 90.0), 0.0);
    }

    #[test]
    fn each_blind_spot_covers_its_sector() {
        for (off, sector) in [
            (0.0, BLIND_FRONT),
            (45.0, BLIND_FRONT),
            (45.5, BLIND_SIDES),
            (135.0, BLIND_SIDES),
            (135.5, BLIND_REAR),
            (180.0, BLIND_REAR),
        ] {
            assert!(blind(sector, off), "{off}");
            assert!(!blind(0x7000 & !sector, off), "{off}: only its own sector");
            assert!(blind(0x7000, off), "{off}");
            assert!(!blind(0, off), "{off}");
        }
        assert_eq!((FRONT_ARC, SIDE_ARC), (45.0, 135.0));
        assert_eq!(
            (BLIND_FRONT, BLIND_SIDES, BLIND_REAR),
            (0x1000, 0x2000, 0x4000)
        );
    }

    #[test]
    fn the_lead_angle_at_a_resting_target_is_its_bearing() {
        let at_rest = target(100.0, -100.0, 0.0, 0.0);
        assert!(close(lead(&firer(0.0), &at_rest, 10.0), 45.0));
        assert!(close(lead(&firer(0.0), &at_rest, 0.0), 45.0), "no speed");
    }

    #[test]
    fn the_lead_angle_aims_ahead_of_a_crossing_target() {
        // 100 pixels at 10 a tick is 10 ticks, in which it crosses 20.
        let crossing = target(0.0, -100.0, 2.0, 0.0);
        let expected = 20.0_f32.atan2(100.0).to_degrees();
        assert!(close(lead(&firer(0.0), &crossing, 10.0), expected));
        assert_eq!(
            lead(&firer(0.0), &crossing, 0.0),
            0.0,
            "no speed: the bearing"
        );
        let alongside = ShipState {
            velocity: Vec2::new(2.0, 0.0),
            ..firer(0.0)
        };
        assert_eq!(
            lead(&alongside, &crossing, 10.0),
            0.0,
            "relative to the firer"
        );
        let slow = lead(&firer(0.0), &crossing, 5.0);
        assert!(close(slow, 40.0_f32.atan2(100.0).to_degrees()), "{slow}");
        let elsewhere = ShipState {
            position: Vec2::new(-300.0, 400.0),
            ..firer(0.0)
        };
        let ahead = target(-300.0, 300.0, 2.0, 0.0);
        assert!(
            close(lead(&elsewhere, &ahead, 10.0), expected),
            "from where it is"
        );
    }

    /// `spec` aimed from a ship facing `heading` at the target dead ahead
    /// of the centre, 100 pixels up, crossing to the right, its quadrant
    /// turrets testing no blind spot, as the engine's player fires them.
    fn aimed(spec: &WeaponSpec, heading: f32, hull_blind: u16, with_target: bool) -> Option<Aim> {
        aimed_by(spec, heading, hull_blind, with_target, RuleSource::Engine)
    }

    /// As [`aimed`], its quadrant turrets testing blind spots as
    /// `quadrant_blind` says.
    fn aimed_by(
        spec: &WeaponSpec,
        heading: f32,
        hull_blind: u16,
        with_target: bool,
        quadrant_blind: RuleSource,
    ) -> Option<Aim> {
        let hull = HullSpec {
            blind_spots: hull_blind,
            ..HullSpec::default()
        };
        let crossing = target(0.0, -100.0, 2.0, 0.0);
        aim(
            spec,
            &firer(heading),
            &hull,
            with_target.then_some(&crossing),
            quadrant_blind,
        )
    }

    fn led() -> f32 {
        20.0_f32.atan2(100.0).to_degrees()
    }

    /// The heading `aim` points along from the firer at the centre.
    fn pointed(aim: &Aim) -> f32 {
        aim.pointing.heading_from(&firer(0.0))
    }

    #[test]
    fn unguided_weapons_beams_bombs_and_rockets_fire_ahead_at_nothing() {
        for raw in [-1, 0, 5, 6] {
            for with_target in [false, true] {
                assert_eq!(
                    aimed(&spec(raw, 0), 30.0, 0, with_target),
                    Some(Aim::along(30.0, None)),
                    "{raw}"
                );
            }
        }
    }

    #[test]
    fn a_fighter_bay_launches_ahead_with_the_firers_target_or_without_one() {
        let bay = WeaponSpec::new(&WeaponRecord {
            guidance: 99,
            ammo_type: 144,
            ..weapon(149)
        });
        assert_eq!(
            aimed(&bay, 30.0, BLIND_FRONT, true),
            Some(Aim::along(30.0, Some(TARGET))),
            "no blind spot"
        );
        assert_eq!(aimed(&bay, 200.0, 0, false), Some(Aim::along(200.0, None)));
    }

    #[test]
    fn a_homing_weapon_fires_ahead_at_its_target_or_without_one() {
        let homing = spec(1, 0);
        assert_eq!(
            aimed(&homing, 30.0, 0, true),
            Some(Aim::along(30.0, Some(TARGET)))
        );
        assert_eq!(aimed(&homing, 30.0, 0, false), Some(Aim::along(30.0, None)));
    }

    #[test]
    fn a_turret_fires_at_the_lead_angle_and_a_turreted_beam_along_the_bearing() {
        let turret = aimed(&spec(4, 0), 180.0, 0, true).expect("fires");
        assert!(close(pointed(&turret), led()), "{turret:?}");
        assert_eq!(turret.target, Some(TARGET));
        let beam = aimed(&spec(3, 0), 180.0, 0, true).expect("fires");
        assert_eq!(beam.pointing, Pointing::At(Vec2::new(0.0, -100.0)));
        assert_eq!((pointed(&beam), beam.target), (0.0, Some(TARGET)));
        for raw in [3, 4] {
            assert_eq!(
                aimed(&spec(raw, 0), 180.0, 0, false),
                None,
                "{raw}: no target"
            );
        }
    }

    #[test]
    fn a_turret_does_not_fire_into_a_blind_spot_of_its_weapon_or_its_ship() {
        let tested = |raw, (flags, hull_blind)| {
            let readings = if raw == 8 {
                vec![RuleSource::Bible]
            } else {
                RuleSource::ALL.to_vec()
            };
            readings.into_iter().all(|reading| {
                aimed_by(&spec(raw, flags), 180.0, hull_blind, true, reading).is_none()
            })
        };
        for raw in [3, 4, 8] {
            // The target is astern of a ship facing down.
            assert!(tested(raw, (BLIND_REAR, 0)), "{raw}");
            assert!(tested(raw, (0, BLIND_REAR)), "{raw}");
            assert!(tested(raw, (BLIND_REAR, BLIND_REAR)), "{raw}: both");
        }
        for raw in [3, 4] {
            assert!(
                aimed(&spec(raw, BLIND_REAR), 90.0, 0, true).is_some(),
                "{raw}: abeam"
            );
            assert!(
                aimed(&spec(raw, 0), 90.0, BLIND_FRONT, true).is_some(),
                "{raw}"
            );
            assert_eq!(aimed(&spec(raw, BLIND_SIDES), 90.0, 0, true), None, "{raw}");
            assert_eq!(aimed(&spec(raw, 0), 0.0, BLIND_FRONT, true), None, "{raw}");
        }
    }

    #[test]
    fn a_front_quadrant_turret_leads_a_target_within_45_degrees_and_otherwise_fires_ahead() {
        let front = spec(7, 0);
        let at_45 = aimed(&front, 45.0, 0, true).expect("fires");
        assert!(close(pointed(&at_45), led()), "{at_45:?}");
        assert_eq!(at_45.target, Some(TARGET));
        let at_315 = aimed(&front, 315.0, 0, true).expect("fires");
        assert!(close(pointed(&at_315), led()), "the other side: {at_315:?}");
        for (heading, with_target) in [(46.0, true), (314.0, true), (0.0, false)] {
            assert_eq!(
                aimed(&front, heading, 0, with_target),
                Some(Aim::along(heading, None)),
                "{heading} {with_target}"
            );
        }
        let blind = aimed_by(&spec(7, BLIND_FRONT), 0.0, 0, true, RuleSource::Bible);
        assert_eq!(blind, None, "blind, by the other reading");
    }

    #[test]
    fn by_the_engine_a_quadrant_turret_tests_no_blind_spot() {
        let front = aimed(&spec(7, BLIND_FRONT), 0.0, 0, true).expect("fires");
        assert!(close(pointed(&front), led()), "{front:?}");
        let hull_blind = aimed(&spec(7, 0), 0.0, BLIND_FRONT, true).expect("fires");
        assert_eq!(hull_blind.target, Some(TARGET), "the hull's blind spot too");
        let rear = aimed(&spec(8, BLIND_REAR), 180.0, 0, true).expect("fires");
        assert!(close(pointed(&rear), led()), "{rear:?}");
        assert!(aimed(&spec(8, 0), 180.0, BLIND_REAR, true).is_some());
        for raw in [7, 8] {
            for (flags, hull_blind) in [(0x7000, 0), (0, 0x7000)] {
                let heading = if raw == 7 { 0.0 } else { 180.0 };
                assert_eq!(
                    aimed_by(
                        &spec(raw, flags),
                        heading,
                        hull_blind,
                        true,
                        RuleSource::Bible
                    ),
                    None,
                    "{raw}: the other reading"
                );
            }
        }
    }

    #[test]
    fn a_full_turret_tests_its_blind_spots_under_both_readings() {
        for raw in [3, 4] {
            for reading in RuleSource::ALL {
                assert_eq!(
                    aimed_by(&spec(raw, BLIND_FRONT), 0.0, 0, true, reading),
                    None,
                    "{raw} {reading:?}"
                );
                assert!(aimed_by(&spec(raw, BLIND_REAR), 0.0, 0, true, reading).is_some());
            }
        }
    }

    #[test]
    fn a_rear_quadrant_turret_fires_only_at_a_target_within_45_degrees_of_the_tail() {
        let rear = spec(8, 0);
        for heading in [135.0, 180.0, 225.0] {
            let fired = aimed(&rear, heading, 0, true).expect("fires");
            assert!(close(pointed(&fired), led()), "{heading}: {fired:?}");
            assert_eq!(fired.target, Some(TARGET));
        }
        for heading in [134.0, 226.0, 0.0] {
            assert_eq!(aimed(&rear, heading, 0, true), None, "{heading}");
        }
        assert_eq!(aimed(&rear, 180.0, 0, false), None, "no target");
    }

    #[test]
    fn point_defence_never_fires_on_the_trigger() {
        for raw in [9, 10] {
            for with_target in [false, true] {
                assert_eq!(aimed(&spec(raw, 0), 0.0, 0, with_target), None, "{raw}");
            }
        }
    }
}
