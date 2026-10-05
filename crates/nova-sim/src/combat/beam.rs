//! Beams: fired from a ship, held on it for their life, and hitting the
//! nearest ship along them every tick.
//!
//! The original (`_SpawnBeam` and `_HandleBeams` in the `EV Nova`
//! executable) keeps a beam for its weapon's `Count` ticks, following its
//! firer, and every tick damages the nearest hittable ship within its
//! `BeamLength` plus the ship's hit radius, inside a cone about its
//! heading. The simulation keeps that reach and swaps the cone for the
//! beam's segment against each ship's circle: deterministic, and what the
//! drawn beam shows. Which ships it can hit is a shot's rule
//! ([`Target::hittable_by`]).
//!
//! A beam (0) is held along its firer's heading, plus the offset it left
//! at. A turreted beam (3) is held along the bearing to its target every
//! tick, and ends once the target is gone or breaking up. A point-defence
//! beam (10) is held on its missile, ends once the missile is gone, and
//! hits no ship.

use super::ShipRef;
use super::aim::bearing;
use super::projectile::{Shot, ShotId, Target, contact};
use super::weapon::WeaponSpec;
use crate::flight::{ShipState, facing, normalized, shortest_turn};
use crate::geometry::Vec2;

/// What a beam is held on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Aiming {
    /// Its firer's heading, plus its offset: a beam (0).
    Ahead,
    /// This ship's bearing: a turreted beam (3).
    Ship(ShipRef),
    /// This shot's bearing: a point-defence beam (10).
    Shot(ShotId),
}

/// A beam, held on its firer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Beam {
    /// Its weapon.
    pub weapon: WeaponSpec,
    /// The ship that fired it.
    pub firer: ShipRef,
    /// The firer's fleet.
    pub fleet: ShipRef,
    /// How far off the firer's heading it points, in degrees, when it is
    /// held [`Aiming::Ahead`].
    pub offset: f32,
    /// What it is held on.
    pub aiming: Aiming,
    /// Where it starts: the firer's position.
    pub start: Vec2,
    /// Where it ends, its length along its heading.
    pub end: Vec2,
    /// The ticks it has lasted.
    pub age: u32,
}

impl Beam {
    /// A beam of `weapon` fired by `firer` of `fleet`, flying `from`,
    /// along `heading`.
    #[must_use]
    pub fn launch(
        weapon: WeaponSpec,
        firer: ShipRef,
        fleet: ShipRef,
        from: &ShipState,
        heading: f32,
        aiming: Aiming,
    ) -> Self {
        let mut beam = Self {
            weapon,
            firer,
            fleet,
            offset: shortest_turn(from.heading, heading),
            aiming,
            start: Vec2::ZERO,
            end: Vec2::ZERO,
            age: 0,
        };
        beam.point(from.position, heading);
        beam
    }

    /// Puts the beam on its firer, now `at`, held on what it aims at among
    /// `targets` and `shots`; false, and left where it was, when that is
    /// gone (see the module docs).
    pub fn follow(&mut self, at: &ShipState, targets: &[Target], shots: &[Shot]) -> bool {
        let toward = match self.aiming {
            Aiming::Ahead => None,
            Aiming::Ship(ship) => {
                let target = targets
                    .iter()
                    .find(|target| target.ship == ship && target.condition.hittable());
                match target {
                    Some(target) => Some(target.position),
                    None => return false,
                }
            }
            Aiming::Shot(id) => match shots.iter().find(|shot| shot.id == id) {
                Some(shot) => Some(shot.position),
                None => return false,
            },
        };
        let heading = toward.map_or(normalized(at.heading + self.offset), |toward| {
            bearing(at.position, toward)
        });
        self.point(at.position, heading);
        true
    }

    /// Points the beam from `start` along `heading`.
    fn point(&mut self, start: Vec2, heading: f32) {
        self.start = start;
        self.end = start + facing(heading) * self.weapon.beam_length;
    }

    /// The nearest of `targets` it can hit that it reaches, and where it
    /// meets it.
    #[must_use]
    pub fn hit(&self, targets: &[Target]) -> Option<(ShipRef, Vec2)> {
        if let Aiming::Shot(_) = self.aiming {
            return None;
        }
        targets
            .iter()
            .filter(|target| target.hittable_by(self.firer, self.fleet))
            .filter_map(|target| {
                let t = contact(self.start, self.end, target.position, target.radius)?;
                Some((t, target.ship))
            })
            .min_by(|(a, _), (b, _)| a.total_cmp(b))
            .map(|(t, ship)| (ship, self.start + (self.end - self.start) * t))
    }

    /// Counts a tick of its life.
    pub fn age(&mut self) {
        self.age += 1;
    }

    /// Whether it has lasted its life.
    #[must_use]
    pub fn expired(&self) -> bool {
        self.age >= self.weapon.lifetime
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::catalog::WeaponRecord;
    use crate::combat::hull::Condition;
    use crate::combat::projectile::ShotId;
    use crate::testkit::weapon;
    use crate::traffic::npc::NpcId;

    const FIRER: ShipRef = ShipRef::Npc(NpcId(1));

    /// A 100-pixel beam lasting 3 ticks.
    fn laser_record() -> WeaponRecord {
        WeaponRecord {
            guidance: 0,
            count: 3,
            beam_length: 100,
            ..weapon(146)
        }
    }

    fn laser() -> WeaponSpec {
        WeaponSpec::new(&laser_record())
    }

    fn at(x: f32, y: f32, heading: f32) -> ShipState {
        ShipState {
            position: Vec2::new(x, y),
            velocity: Vec2::new(7.0, 7.0),
            heading,
        }
    }

    fn close(a: Vec2, b: Vec2) -> bool {
        (a - b).length() < 1e-3
    }

    #[test]
    fn a_beam_lasts_its_count_and_follows_its_firer_as_it_moves_and_turns() {
        let mut beam = Beam::launch(
            laser(),
            FIRER,
            FIRER,
            &at(10.0, 20.0, 90.0),
            90.0,
            Aiming::Ahead,
        );
        assert_eq!(beam.start, Vec2::new(10.0, 20.0));
        assert!(close(beam.end, Vec2::new(110.0, 20.0)), "{beam:?}");
        for tick in 1..=3 {
            assert!(!beam.expired(), "{tick}");
            beam.age();
        }
        assert!(beam.expired());
        assert!(beam.follow(&at(-5.0, 0.0, 180.0), &[], &[]));
        assert_eq!(beam.start, Vec2::new(-5.0, 0.0));
        assert!(close(beam.end, Vec2::new(-5.0, 100.0)), "{beam:?}");
        let offset = Beam::launch(
            laser(),
            FIRER,
            FIRER,
            &at(0.0, 0.0, 350.0),
            90.0,
            Aiming::Ahead,
        );
        assert!(close(offset.end, Vec2::new(100.0, 0.0)), "{offset:?}");
        assert_eq!(offset.offset, 100.0);
    }

    fn target(id: u32, x: f32, y: f32) -> Target {
        Target {
            ship: ShipRef::Npc(NpcId(id)),
            fleet: ShipRef::Npc(NpcId(id)),
            position: Vec2::new(x, y),
            velocity: Vec2::ZERO,
            radius: 8.0,
            condition: Condition::Intact,
        }
    }

    #[test]
    fn a_beam_hits_the_nearest_ship_it_reaches() {
        let beam = Beam::launch(
            laser(),
            FIRER,
            FIRER,
            &at(10.0, 5.0, 90.0),
            90.0,
            Aiming::Ahead,
        );
        let (ship, point) = beam
            .hit(&[
                target(2, 90.0, 5.0),
                target(3, 50.0, 8.0),
                target(4, 70.0, 4.0),
            ])
            .expect("a hit");
        assert_eq!(ship, ShipRef::Npc(NpcId(3)));
        assert!(point.x > 40.0 && point.x < 44.0, "{point:?}");
        assert!((point.y - 5.0).abs() < 1e-3, "on the beam: {point:?}");
        assert_eq!(beam.hit(&[]), None);
    }

    #[test]
    fn a_beam_reaches_its_length_and_the_ships_radius_and_no_further() {
        let beam = Beam::launch(
            laser(),
            FIRER,
            FIRER,
            &at(0.0, 0.0, 90.0),
            90.0,
            Aiming::Ahead,
        );
        let one = |x, y| beam.hit(&[target(2, x, y)]).map(|(ship, _)| ship);
        assert_eq!(one(107.9, 0.0), Some(ShipRef::Npc(NpcId(2))));
        assert_eq!(one(108.1, 0.0), None, "beyond its length and the radius");
        assert_eq!(one(50.0, 7.9), Some(ShipRef::Npc(NpcId(2))));
        assert_eq!(one(50.0, 8.1), None, "beside it");
        assert_eq!(one(-9.0, 0.0), None, "behind it");
    }

    const TARGET: ShipRef = ShipRef::Npc(NpcId(2));

    /// A turreted beam from (0, 0), first aimed straight up at the
    /// target.
    fn turreted() -> Beam {
        let spec = WeaponSpec::new(&WeaponRecord {
            guidance: 3,
            ..laser_record()
        });
        Beam::launch(
            spec,
            FIRER,
            FIRER,
            &at(0.0, 0.0, 90.0),
            3.0,
            Aiming::Ship(TARGET),
        )
    }

    #[test]
    fn a_turreted_beam_leaves_along_its_launch_then_re_aims_at_its_target_every_tick() {
        let mut beam = turreted();
        assert!(
            close(beam.end, facing(3.0) * 100.0),
            "as launched: {beam:?}"
        );
        for (x, y) in [(0.0, -50.0), (50.0, 0.0), (-30.0, 30.0)] {
            assert!(beam.follow(&at(0.0, 0.0, 90.0), &[target(2, x, y)], &[]));
            let along = Vec2::new(x, y) * (100.0 / Vec2::new(x, y).length());
            assert!(close(beam.end, along), "({x}, {y}): {beam:?}");
        }
        assert!(beam.follow(&at(10.0, 0.0, 0.0), &[target(2, 10.0, 50.0)], &[]));
        assert_eq!(beam.start, Vec2::new(10.0, 0.0));
        assert!(
            close(beam.end, Vec2::new(10.0, 100.0)),
            "from its firer: {beam:?}"
        );
    }

    #[test]
    fn a_turreted_beam_ends_once_its_target_is_gone_or_breaking_up() {
        let mut beam = turreted();
        assert!(!beam.follow(&at(0.0, 0.0, 90.0), &[target(3, 0.0, -50.0)], &[]));
        let dying = Target {
            condition: Condition::Dying { ticks_left: 3 },
            ..target(2, 0.0, -50.0)
        };
        assert!(!turreted().follow(&at(0.0, 0.0, 90.0), &[dying], &[]));
        let disabled = Target {
            condition: Condition::Disabled,
            ..target(2, 0.0, -50.0)
        };
        assert!(turreted().follow(&at(0.0, 0.0, 90.0), &[disabled], &[]));
    }

    /// A shot of the IR Missile at (`x`, `y`), numbered `id`.
    fn missile(id: u32, x: f32, y: f32) -> Shot {
        let spec = WeaponSpec::new(&WeaponRecord {
            guidance: 1,
            ..weapon(134)
        });
        Shot {
            id: ShotId(id),
            position: Vec2::new(x, y),
            ..Shot::launch(spec, TARGET, TARGET, &at(0.0, 0.0, 0.0), 0.0)
        }
    }

    #[test]
    fn a_point_defence_beam_follows_its_shot_and_hits_no_ship() {
        let spec = WeaponSpec::new(&WeaponRecord {
            guidance: 10,
            ..laser_record()
        });
        let mut beam = Beam::launch(
            spec,
            FIRER,
            FIRER,
            &at(0.0, 0.0, 0.0),
            90.0,
            Aiming::Shot(ShotId(5)),
        );
        assert!(close(beam.end, Vec2::new(100.0, 0.0)));
        let shots = [missile(4, 0.0, -40.0), missile(5, 0.0, 40.0)];
        assert!(beam.follow(&at(0.0, 0.0, 0.0), &[], &shots));
        assert!(
            close(beam.end, Vec2::new(0.0, 100.0)),
            "at shot 5: {beam:?}"
        );
        assert_eq!(beam.hit(&[target(2, 0.0, 50.0)]), None);
        assert!(
            !beam.follow(&at(0.0, 0.0, 0.0), &[], &shots[..1]),
            "its shot gone"
        );
    }

    #[test]
    fn a_beam_ahead_ignores_any_target() {
        let mut beam = Beam::launch(
            laser(),
            FIRER,
            FIRER,
            &at(0.0, 0.0, 90.0),
            80.0,
            Aiming::Ahead,
        );
        assert!(beam.follow(&at(0.0, 0.0, 0.0), &[target(2, 50.0, 0.0)], &[]));
        assert!(close(beam.end, facing(350.0) * 100.0), "{beam:?}");
    }

    #[test]
    fn a_beam_never_hits_its_firer_its_fleet_or_the_dying() {
        let beam = Beam::launch(
            laser(),
            FIRER,
            FIRER,
            &at(0.0, 0.0, 90.0),
            90.0,
            Aiming::Ahead,
        );
        let firer = Target {
            ship: FIRER,
            ..target(1, 0.0, 0.0)
        };
        let mate = Target {
            fleet: FIRER,
            ..target(2, 20.0, 0.0)
        };
        let dying = Target {
            condition: Condition::Dying { ticks_left: 1 },
            ..target(3, 30.0, 0.0)
        };
        assert_eq!(beam.hit(&[firer, mate, dying]), None);
        let disabled = Target {
            condition: Condition::Disabled,
            ..target(4, 50.0, 0.0)
        };
        assert_eq!(
            beam.hit(&[firer, disabled]).map(|(ship, _)| ship),
            Some(ShipRef::Npc(NpcId(4)))
        );
    }
}
