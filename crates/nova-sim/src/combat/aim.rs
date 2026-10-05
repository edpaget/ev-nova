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
//!   (0x4000). The original's player checks them for 3 and 4 only; here
//!   every turret does, as the original's AI does.
//! - What each guidance fires ([`aim`]):
//!   - -1, 0, 5 and 6 fire along the ship's heading, at nothing.
//!   - 1 fires along the heading at the target, when there is one, and
//!     without one too; so does a fighter bay (99 carrying a ship), whose
//!     fighter leaves along the heading with the firer's target.
//!   - 3 (a turreted beam) fires along the bearing, and 4 (a turret) at
//!     the lead angle, only with a target not in a blind spot.
//!   - 7 fires at the lead angle at a target within [`FRONT_ARC`] of the
//!     nose, and otherwise (no target, or one out of the arc) straight
//!     ahead at nothing.
//!   - 8 fires at the lead angle at a target within [`FRONT_ARC`] of the
//!     tail, and otherwise not at all.
//!   - 9 and 10 never fire on the trigger: only point defence fires them
//!     (see [`defence`](super::defence)).
//!   - A turret that does not fire spends nothing.

use super::ShipRef;
use super::hull::HullSpec;
use super::projectile::Target;
use super::weapon::{Guidance, WeaponSpec};
use crate::flight::{ShipState, heading_of, shortest_turn};
use crate::geometry::Vec2;

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
    /// The heading it fires along, in degrees, before its inaccuracy.
    pub heading: f32,
    /// The ship it fires at, if any.
    pub target: Option<ShipRef>,
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
/// fires at all (see the module docs).
#[must_use]
pub fn aim(
    spec: &WeaponSpec,
    firer: &ShipState,
    hull: &HullSpec,
    target: Option<&Target>,
) -> Option<Aim> {
    let ahead = Aim {
        heading: firer.heading,
        target: None,
    };
    let blind_spots = spec.flags | hull.blind_spots;
    // The target, its bearing's angle off the nose, and whether it is in a
    // blind spot.
    let sighted = target.map(|target| {
        let off = angle_off(firer.heading, bearing(firer.position, target.position));
        (target, off, blind(blind_spots, off))
    });
    let led = |target: &Target| Aim {
        heading: lead(firer, target, spec.speed),
        target: Some(target.ship),
    };
    match spec.guidance {
        Guidance::Homing | Guidance::FighterBay => Some(Aim {
            target: target.map(|target| target.ship),
            ..ahead
        }),
        Guidance::TurretBeam => sighted
            .filter(|&(_, _, blind)| !blind)
            .map(|(target, _, _)| Aim {
                heading: bearing(firer.position, target.position),
                target: Some(target.ship),
            }),
        Guidance::Turret => sighted
            .filter(|&(_, _, blind)| !blind)
            .map(|(target, _, _)| led(target)),
        Guidance::FrontTurret => match sighted {
            Some((target, off, blind)) if off <= FRONT_ARC => (!blind).then(|| led(target)),
            _ => Some(ahead),
        },
        Guidance::RearTurret => sighted
            .filter(|&(_, off, blind)| off >= 180.0 - FRONT_ARC && !blind)
            .map(|(target, _, _)| led(target)),
        Guidance::PointDefence | Guidance::PointDefenceBeam => None,
        Guidance::Unguided
        | Guidance::Beam
        | Guidance::FreefallBomb
        | Guidance::Rocket
        | Guidance::Other(_) => Some(ahead),
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
            fleet: TARGET,
            position: Vec2::new(x, y),
            velocity: Vec2::new(vx, vy),
            radius: 10.0,
            condition: Condition::Intact,
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
    /// of the centre, 100 pixels up, crossing to the right.
    fn aimed(spec: &WeaponSpec, heading: f32, hull_blind: u16, with_target: bool) -> Option<Aim> {
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
        )
    }

    fn led() -> f32 {
        20.0_f32.atan2(100.0).to_degrees()
    }

    #[test]
    fn unguided_weapons_beams_bombs_and_rockets_fire_ahead_at_nothing() {
        for raw in [-1, 0, 5, 6] {
            for with_target in [false, true] {
                assert_eq!(
                    aimed(&spec(raw, 0), 30.0, 0, with_target),
                    Some(Aim {
                        heading: 30.0,
                        target: None
                    }),
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
            Some(Aim {
                heading: 30.0,
                target: Some(TARGET)
            }),
            "no blind spot"
        );
        assert_eq!(
            aimed(&bay, 200.0, 0, false),
            Some(Aim {
                heading: 200.0,
                target: None
            })
        );
    }

    #[test]
    fn a_homing_weapon_fires_ahead_at_its_target_or_without_one() {
        let homing = spec(1, 0);
        assert_eq!(
            aimed(&homing, 30.0, 0, true),
            Some(Aim {
                heading: 30.0,
                target: Some(TARGET)
            })
        );
        assert_eq!(
            aimed(&homing, 30.0, 0, false),
            Some(Aim {
                heading: 30.0,
                target: None
            })
        );
    }

    #[test]
    fn a_turret_fires_at_the_lead_angle_and_a_turreted_beam_along_the_bearing() {
        let turret = aimed(&spec(4, 0), 180.0, 0, true).expect("fires");
        assert!(close(turret.heading, led()), "{turret:?}");
        assert_eq!(turret.target, Some(TARGET));
        assert_eq!(
            aimed(&spec(3, 0), 180.0, 0, true),
            Some(Aim {
                heading: 0.0,
                target: Some(TARGET)
            })
        );
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
        for raw in [3, 4, 8] {
            // The target is astern of a ship facing down.
            let rear_blind = spec(raw, BLIND_REAR);
            assert_eq!(aimed(&rear_blind, 180.0, 0, true), None, "{raw}");
            assert_eq!(aimed(&spec(raw, 0), 180.0, BLIND_REAR, true), None, "{raw}");
            assert_eq!(
                aimed(&rear_blind, 180.0, BLIND_REAR, true),
                None,
                "{raw}: both"
            );
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
        assert!(close(at_45.heading, led()), "{at_45:?}");
        assert_eq!(at_45.target, Some(TARGET));
        let at_315 = aimed(&front, 315.0, 0, true).expect("fires");
        assert!(close(at_315.heading, led()), "the other side: {at_315:?}");
        for (heading, with_target) in [(46.0, true), (314.0, true), (0.0, false)] {
            assert_eq!(
                aimed(&front, heading, 0, with_target),
                Some(Aim {
                    heading,
                    target: None
                }),
                "{heading} {with_target}"
            );
        }
        assert_eq!(aimed(&spec(7, BLIND_FRONT), 0.0, 0, true), None, "blind");
    }

    #[test]
    fn a_rear_quadrant_turret_fires_only_at_a_target_within_45_degrees_of_the_tail() {
        let rear = spec(8, 0);
        for heading in [135.0, 180.0, 225.0] {
            let fired = aimed(&rear, heading, 0, true).expect("fires");
            assert!(close(fired.heading, led()), "{heading}: {fired:?}");
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
