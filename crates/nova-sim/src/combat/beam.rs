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

use super::ShipRef;
use super::projectile::{Target, contact};
use super::weapon::WeaponSpec;
use crate::flight::{ShipState, facing, normalized};
use crate::geometry::Vec2;

/// A beam, held on its firer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Beam {
    /// Its weapon.
    pub weapon: WeaponSpec,
    /// The ship that fired it.
    pub firer: ShipRef,
    /// The firer's fleet.
    pub fleet: ShipRef,
    /// How far off the firer's heading it points, in degrees.
    pub offset: f32,
    /// Where it starts: the firer's position.
    pub start: Vec2,
    /// Where it ends, its length along its heading.
    pub end: Vec2,
    /// The ticks it has lasted.
    pub age: u32,
}

impl Beam {
    /// A beam of `weapon` fired by `firer` of `fleet`, flying `from`,
    /// `offset` degrees off its heading.
    #[must_use]
    pub fn launch(
        weapon: WeaponSpec,
        firer: ShipRef,
        fleet: ShipRef,
        from: &ShipState,
        offset: f32,
    ) -> Self {
        let mut beam = Self {
            weapon,
            firer,
            fleet,
            offset,
            start: Vec2::ZERO,
            end: Vec2::ZERO,
            age: 0,
        };
        beam.follow(from);
        beam
    }

    /// Puts the beam on its firer, now `at`.
    pub fn follow(&mut self, at: &ShipState) {
        let heading = normalized(at.heading + self.offset);
        self.start = at.position;
        self.end = at.position + facing(heading) * self.weapon.beam_length;
    }

    /// The nearest of `targets` it can hit that it reaches, and where it
    /// meets it.
    #[must_use]
    pub fn hit(&self, targets: &[Target]) -> Option<(ShipRef, Vec2)> {
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
    use crate::testkit::weapon;
    use crate::traffic::npc::NpcId;

    const FIRER: ShipRef = ShipRef::Npc(NpcId(1));

    /// A 100-pixel beam lasting 3 ticks.
    fn laser() -> WeaponSpec {
        WeaponSpec::new(&WeaponRecord {
            guidance: 0,
            count: 3,
            beam_length: 100,
            ..weapon(146)
        })
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
        let mut beam = Beam::launch(laser(), FIRER, FIRER, &at(10.0, 20.0, 90.0), 0.0);
        assert_eq!(beam.start, Vec2::new(10.0, 20.0));
        assert!(close(beam.end, Vec2::new(110.0, 20.0)), "{beam:?}");
        for tick in 1..=3 {
            assert!(!beam.expired(), "{tick}");
            beam.age();
        }
        assert!(beam.expired());
        beam.follow(&at(-5.0, 0.0, 180.0));
        assert_eq!(beam.start, Vec2::new(-5.0, 0.0));
        assert!(close(beam.end, Vec2::new(-5.0, 100.0)), "{beam:?}");
        let offset = Beam::launch(laser(), FIRER, FIRER, &at(0.0, 0.0, 350.0), 100.0);
        assert!(close(offset.end, Vec2::new(100.0, 0.0)), "{offset:?}");
        assert_eq!(offset.offset, 100.0);
    }

    fn target(id: u32, x: f32, y: f32) -> Target {
        Target {
            ship: ShipRef::Npc(NpcId(id)),
            fleet: ShipRef::Npc(NpcId(id)),
            position: Vec2::new(x, y),
            radius: 8.0,
            condition: Condition::Intact,
        }
    }

    #[test]
    fn a_beam_hits_the_nearest_ship_it_reaches() {
        let beam = Beam::launch(laser(), FIRER, FIRER, &at(0.0, 0.0, 90.0), 0.0);
        let (ship, point) = beam
            .hit(&[
                target(2, 80.0, 0.0),
                target(3, 40.0, 3.0),
                target(4, 60.0, -1.0),
            ])
            .expect("a hit");
        assert_eq!(ship, ShipRef::Npc(NpcId(3)));
        assert!(point.x > 30.0 && point.x < 34.0, "{point:?}");
        assert_eq!(beam.hit(&[]), None);
    }

    #[test]
    fn a_beam_reaches_its_length_and_the_ships_radius_and_no_further() {
        let beam = Beam::launch(laser(), FIRER, FIRER, &at(0.0, 0.0, 90.0), 0.0);
        let one = |x, y| beam.hit(&[target(2, x, y)]).map(|(ship, _)| ship);
        assert_eq!(one(107.9, 0.0), Some(ShipRef::Npc(NpcId(2))));
        assert_eq!(one(108.1, 0.0), None, "beyond its length and the radius");
        assert_eq!(one(50.0, 7.9), Some(ShipRef::Npc(NpcId(2))));
        assert_eq!(one(50.0, 8.1), None, "beside it");
        assert_eq!(one(-9.0, 0.0), None, "behind it");
    }

    #[test]
    fn a_beam_never_hits_its_firer_its_fleet_or_the_dying() {
        let beam = Beam::launch(laser(), FIRER, FIRER, &at(0.0, 0.0, 90.0), 0.0);
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
