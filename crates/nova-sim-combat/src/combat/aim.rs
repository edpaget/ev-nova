//! Aiming: which way a weapon fires, and at what, from its guidance and
//! its firer's target.
//!
//! The values are the original's (`_FirePlayerWeapon`, `_SpawnShot`,
//! `_CalcInterceptAngle`, `_CalcLeadAngle` and `_TurretBlindSpot` in the
//! `EV Nova` executable):
//!
//! - A shot's heading is a whole degree (`_SpawnShot` @0x3e677-0x3eaa0):
//!   along the ship's heading truncated, at a point along the table's
//!   bearing to it (`_CalcInterceptAngle`, [`trig::intercept_angle`]), or
//!   at a ship's lead angle (`_CalcLeadAngle`, [`trig::lead_angle`]): where
//!   the target will be when a shot at `Speed` gets there, with t the
//!   distance over the shot's speed (a rocket's in its own way), at the
//!   target's position plus its velocity relative to the firer's, times t
//!   ([`Pointing::whole_from`]).
//! - A turret is blind in the sectors its weapon's `Flags` or its ship's
//!   `Flags` mark ([`blind_spot`], `_TurretBlindSpot` @0xb325): by the
//!   table's bearing to the target against the ship's truncated heading,
//!   the front up to [`FRONT_ARC`] whole degrees off the nose (0x1000), the
//!   sides up to [`SIDE_ARC`] (0x2000), and the rear beyond (0x4000). 3 and
//!   4 always test them (`_FirePlayerWeapon` @0x629fe, @0x62a94). 7 and 8 test them as the reading [`aim`] is given says
//!   ([`RuleKey::QuadrantTurretBlindSpots`](crate::RuleKey::QuadrantTurretBlindSpots)):
//!   by the engine, as `_FirePlayerWeapon` and `_FireAIShipWeapon` fire
//!   them, never (@0x62aeb-0x62c38); by the other reading, as
//!   `_AIFireTurret` picks them (@0x80d0b) and the Bible has it ("Turreted
//!   weapon has a blind spot"), as 3 and 4 do. The fight passes the key's
//!   reading for the player's trigger, and the testing reading for an
//!   NPC's primary trigger (which no AI sets now: an escort's formation
//!   fire is its AI's pick, tested as `_AIFireTurret` picks it).
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
//!   - The player's arcs are [`player_quadrant_off`]'s, folded; an NPC's
//!     are [`quadrant_off`]'s, by the rulebook's reading.
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
use crate::trig::{self, LeadShot, angular_difference, whole};

/// How far off the nose, in whole degrees, the front sector reaches; a
/// front-quadrant turret's arc, and a rear-quadrant turret's off the tail
/// (`cmpw $0x2d`, @0xb389, @0x62bc1, @0x88b83).
pub const FRONT_ARC: i16 = 45;
/// How far off the nose, in whole degrees, the side sectors reach
/// (`cmpw $0x87`, @0xb3cc).
pub const SIDE_ARC: i16 = 135;
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
    /// At `target`'s lead angle for `shot`, from where it leaves
    /// ([`trig::lead_angle`]).
    Leading {
        /// The ship it leads, as it was when it was aimed at.
        target: Target,
        /// The shot `_CalcLeadAngle` leads with, by its weapon's guidance
        /// ([`WeaponSpec::lead_shot`]); none for the table bearing.
        shot: Option<LeadShot>,
    },
}

impl Pointing {
    /// The whole-degree heading a shot pointing so leaves `origin` along,
    /// before any inaccuracy, as `_SpawnShot` sets it: the ship's heading
    /// truncated (`cvttss2si` @0x3e677), the table's bearing to a point
    /// (`_CalcInterceptAngle`, as point defence aims at a missile,
    /// @0x39790), or the lead angle from where it leaves (`_CalcLeadAngle`
    /// @0x3ecdc, @0x397c6), the firer's velocity `origin`'s.
    #[must_use]
    pub fn whole_from(&self, origin: &ShipState) -> i16 {
        match self {
            Self::Along(heading) => whole(*heading),
            Self::At(point) => trig::intercept_angle(origin.position, *point),
            Self::Leading { target, shot } => trig::lead_angle(
                origin.position,
                origin.velocity,
                (target.position, target.velocity),
                *shot,
            ),
        }
    }

    /// The heading it points along from `origin`, before any inaccuracy:
    /// along its heading as it is, which a fighter launched from a bay
    /// keeps, or [`Pointing::whole_from`]'s whole degree.
    #[must_use]
    pub fn heading_from(&self, origin: &ShipState) -> f32 {
        match self {
            Self::Along(heading) => *heading,
            _ => f32::from(self.whole_from(origin)),
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

/// How far the table's whole-degree `angle` to a target is off a
/// quadrant turret's arc, as an NPC's turret measures it, by the
/// rulebook's
/// [`RuleKey::QuadrantTurretArc`](crate::RuleKey::QuadrantTurretArc)
/// entry `arc` (`_AIFireTurret` @0x80c4d-0x80ce8, `_FireAIShipWeapon`
/// @0x88ac4-0x88b7f): by the engine, a front turret (7) takes
/// `|trunc(angle − heading)|` in `f32` and a rear one (8) `|angle −
/// (trunc(heading) + 180) mod 360|`, each mod 360 with no fold past 180,
/// so a heading near 0 holds fire across the seam; by the other reading,
/// folded as `_AngularDifference` folds.
#[must_use]
pub fn quadrant_off(guidance: Guidance, heading: f32, angle: i16, arc: RuleSource) -> i16 {
    let rear = (i32::from(whole(heading)) + 180) % 360;
    let plain = if guidance == Guidance::RearTurret {
        (i32::from(angle) - rear).abs() % 360
    } else {
        i32::from(whole(f32::from(angle) - heading)).abs() % 360
    };
    match arc {
        RuleSource::Engine => i16::try_from(plain).unwrap_or(i16::MAX),
        RuleSource::Bible => {
            let reference = if guidance == Guidance::RearTurret {
                i16::try_from(rear).unwrap_or_default()
            } else {
                whole(heading)
            };
            angular_difference(angle, reference).abs()
        }
    }
}

/// How far the table's whole-degree `angle` to a target is off the
/// player's quadrant turret's arc (`_FirePlayerWeapon` @0x62b13-0x62bc6):
/// `|_AngularDifference(angle, reference)|`, the reference the truncated
/// heading for a front turret (7) and `trunc(heading + 180) mod 360` for a
/// rear one (8), the add in `f32` (0xdd688).
#[must_use]
pub fn player_quadrant_off(guidance: Guidance, heading: f32, angle: i16) -> i16 {
    let reference = if guidance == Guidance::RearTurret {
        whole(heading + 180.0) % 360
    } else {
        whole(heading)
    };
    angular_difference(angle, reference).abs()
}

/// Whether a turret blind where `blind_spots` say, on a ship facing
/// `heading`, is blind to a target at the table's whole-degree `angle`
/// (`_TurretBlindSpot` @0xb325-0xb44b): `_AngularDifference` of the
/// truncated heading and the angle, folded inline (@0xb341-0xb387), up to
/// [`FRONT_ARC`] the front, up to [`SIDE_ARC`] the sides, and beyond it
/// the rear (see the module docs).
#[must_use]
pub fn blind_spot(blind_spots: u16, heading: f32, angle: i16) -> bool {
    let off = angular_difference(whole(heading), angle);
    let sector = if off <= FRONT_ARC {
        BLIND_FRONT
    } else if off <= SIDE_ARC {
        BLIND_SIDES
    } else {
        BLIND_REAR
    };
    blind_spots & sector != 0
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
    // The target, the table's angle to it, and whether it is in a blind
    // spot that holds the weapon (@0x629e3, @0x62a7b, @0x62b13).
    let sighted = target.map(|target| {
        let angle = trig::intercept_angle(firer.position, target.position);
        (
            target,
            angle,
            tested && blind_spot(blind_spots, firer.heading, angle),
        )
    });
    let off = |angle| player_quadrant_off(spec.guidance, firer.heading, angle);
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
            Some((target, angle, blind)) if off(angle) <= FRONT_ARC => {
                (!blind).then(|| led(spec, target))
            }
            _ => Some(ahead),
        },
        Guidance::RearTurret => sighted
            .filter(|&(_, angle, blind)| off(angle) <= FRONT_ARC && !blind)
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
            shot: spec.lead_shot(),
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
    use crate::combat::NpcId;
    use crate::combat::hull::Condition;
    use nova_sim_catalog::fixture::weapon;

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
            disabled: false,
            miss_class: 100,
            slot: crate::ship_layer::Slot::Berth(0),
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
    fn each_blind_spot_covers_its_sector_in_whole_degrees() {
        for (angle, sector) in [
            (0, BLIND_FRONT),
            (45, BLIND_FRONT),
            (46, BLIND_SIDES),
            (135, BLIND_SIDES),
            (136, BLIND_REAR),
            (180, BLIND_REAR),
            (315, BLIND_FRONT),
            (314, BLIND_SIDES),
        ] {
            assert!(blind_spot(sector, 0.0, angle), "{angle}");
            assert!(
                !blind_spot(0x7000 & !sector, 0.0, angle),
                "{angle}: only its own sector"
            );
            assert!(blind_spot(0x7000, 0.0, angle), "{angle}");
            assert!(!blind_spot(0, 0.0, angle), "{angle}");
        }
        assert_eq!((FRONT_ARC, SIDE_ARC), (45, 135));
        assert_eq!(
            (BLIND_FRONT, BLIND_SIDES, BLIND_REAR),
            (0x1000, 0x2000, 0x4000)
        );
    }

    #[test]
    fn a_blind_spot_reads_the_truncated_heading() {
        // 45.9 off by float, 45 by the truncated heading: the front.
        assert!(blind_spot(BLIND_FRONT, 45.9, 0));
        assert!(!blind_spot(BLIND_SIDES, 45.9, 0));
        assert!(blind_spot(BLIND_SIDES, 46.0, 0));
    }

    #[test]
    fn a_pointing_points_along_whole_degrees() {
        let at = ShipState {
            position: Vec2::new(10.0, 10.0),
            velocity: Vec2::new(1.0, 0.0),
            heading: 30.7,
        };
        assert_eq!(Pointing::Along(30.7).whole_from(&at), 30);
        assert_eq!(Pointing::Along(30.7).heading_from(&at), 30.7, "a fighter's");
        let corner = Vec2::new(20.0, 20.0);
        assert_eq!(Pointing::At(corner).whole_from(&at), 134, "the table's");
        assert_eq!(Pointing::At(corner).heading_from(&at), 134.0);
        let crossing = target(10.0, -90.0, 2.0, 0.0);
        let shot = Some(LeadShot {
            speed: 10.0,
            rocket: false,
        });
        let leading = Pointing::Leading {
            target: crossing,
            shot,
        };
        let expected = trig::lead_angle(
            at.position,
            at.velocity,
            (crossing.position, crossing.velocity),
            shot,
        );
        assert_eq!(leading.whole_from(&at), expected);
        assert_ne!(expected, 0, "led ahead");
        let plain = Pointing::Leading {
            target: crossing,
            shot: None,
        };
        assert_eq!(plain.whole_from(&at), 0, "no shot: the table bearing");
    }

    #[test]
    fn a_turret_leads_with_its_weapons_shot() {
        let crossing = target(0.0, -100.0, 2.0, 0.0);
        assert_eq!(
            super::led(&spec(4, 0), &crossing).pointing,
            Pointing::Leading {
                target: crossing,
                shot: spec(4, 0).lead_shot(),
            }
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

    /// The lead angle at the crossing target from the centre at 10 a tick.
    fn led() -> f32 {
        let crossing = target(0.0, -100.0, 2.0, 0.0);
        let shot = Some(LeadShot {
            speed: 10.0,
            rocket: false,
        });
        let at = (crossing.position, crossing.velocity);
        f32::from(trig::lead_angle(Vec2::ZERO, Vec2::ZERO, at, shot))
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
    fn a_turrets_blind_spot_is_read_by_the_tables_bearing_and_the_truncated_heading() {
        // The diagonal ahead and right is 45 off by float, the front, but
        // the table reads it 46, the side.
        let diagonal = target(100.0, -100.0, 0.0, 0.0);
        let at_diagonal = |raw, flags| {
            aim(
                &spec(raw, flags),
                &firer(0.0),
                &HullSpec::default(),
                Some(&diagonal),
                RuleSource::Engine,
            )
        };
        for raw in [3, 4] {
            assert!(at_diagonal(raw, BLIND_FRONT).is_some(), "{raw}: 46");
            assert_eq!(at_diagonal(raw, BLIND_SIDES), None, "{raw}: the side");
            // Dead ahead of a ship facing 45.9 is 45.9 off by float, the
            // side, but 45 off its truncated heading, the front.
            assert!(
                aimed(&spec(raw, BLIND_SIDES), 45.9, 0, true).is_some(),
                "{raw}: 45"
            );
            assert_eq!(
                aimed(&spec(raw, BLIND_FRONT), 45.9, 0, true),
                None,
                "{raw}: the front"
            );
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
    fn the_players_quadrant_arc_folds_the_table_angle_against_the_truncated_heading() {
        let (front, rear) = (Guidance::FrontTurret, Guidance::RearTurret);
        assert_eq!(player_quadrant_off(front, 10.9, 55), 45, "trunc 10");
        assert_eq!(player_quadrant_off(front, 10.0, 334), 36, "folded");
        assert_eq!(player_quadrant_off(rear, 90.5, 270), 0, "trunc(270.5)");
        assert_eq!(player_quadrant_off(rear, 270.0, 90), 0, "450 mod 360");
        assert_eq!(
            player_quadrant_off(rear, 359.5, 134),
            45,
            "trunc(539.5) mod 360"
        );
    }

    #[test]
    fn by_the_engine_an_npcs_quadrant_arc_is_the_plain_difference() {
        let (front, rear) = (Guidance::FrontTurret, Guidance::RearTurret);
        let engine = RuleSource::Engine;
        assert_eq!(quadrant_off(front, 90.0, 100, engine), 10);
        assert_eq!(quadrant_off(front, 100.0, 90, engine), 10);
        // trunc(100 − 89.5) = 10, where trunc first would give 11.
        assert_eq!(quadrant_off(front, 89.5, 100, engine), 10);
        assert_eq!(
            quadrant_off(front, 10.0, 350, engine),
            340,
            "no fold past 180"
        );
        assert_eq!(
            quadrant_off(rear, 90.9, 280, engine),
            10,
            "(90 + 180) mod 360"
        );
        assert_eq!(quadrant_off(rear, 90.0, 260, engine), 10, "under the tail");
        assert_eq!(
            quadrant_off(rear, 190.0, 350, engine),
            340,
            "10 off by the fold"
        );
        assert_eq!(quadrant_off(front, 10.0, 350, RuleSource::Bible), 20);
        assert_eq!(quadrant_off(rear, 190.0, 350, RuleSource::Bible), 20);
        assert_eq!(
            quadrant_off(front, 10.9, 55, RuleSource::Bible),
            45,
            "trunc 10"
        );
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
