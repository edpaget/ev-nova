//! Shots: launched from a ship, flown a tick at a time, and checked
//! against the ships they pass.
//!
//! The values are the original's (`_SpawnShot`, `_HandleShotGuidance`,
//! `_HandleShot` and `_ShotCanHitShip` in the `EV Nova` executable):
//!
//! - A shot starts at its firer's position with its firer's velocity (a
//!   freefall bomb with [`BOMB_LAUNCH`] of it), heading the firer's way
//!   plus its launch's offset; any but a bomb or a rocket then adds its
//!   speed along its heading.
//! - Each tick an unguided shot keeps its velocity. A rocket's velocity
//!   becomes [`ROCKET_KEEP`] of itself and [`ROCKET_PUSH`] of its top
//!   speed along its heading, so it closes on its top speed. A bomb keeps
//!   its velocity and turns its heading [`BOMB_TURN`] a tick towards it.
//! - A shot lives its weapon's `Count` ticks, then detonates if its weapon
//!   says so (`Flags` 0x8000) and otherwise vanishes.
//! - It hits the first ship ([`Target`]) it passes within the ship's hit
//!   radius and its weapon's `ProxRadius`, swept from where it was to where
//!   it is so that a fast shot cannot pass through a ship between ticks. A
//!   shot cannot hit its firer, a ship of its firer's fleet, or a ship
//!   breaking up or destroyed; a disabled ship it can.

use super::ShipRef;
use super::damage::Blast;
use super::hull::Condition;
use super::weapon::{Guidance, WeaponSpec};
use crate::flight::{ShipState, facing, heading_of, normalized, shortest_turn};
use crate::geometry::Vec2;

/// A freefall bomb's share of its firer's velocity at launch.
pub const BOMB_LAUNCH: f32 = 0.8;
/// The share of its velocity a rocket keeps each tick.
pub const ROCKET_KEEP: f32 = 0.95;
/// The share of its top speed a rocket gains each tick.
pub const ROCKET_PUSH: f32 = 0.05;
/// How far a bomb turns into the wind each tick, in degrees.
pub const BOMB_TURN: f32 = 1.0;

/// A ship a shot or beam might hit.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Target {
    /// Which ship.
    pub ship: ShipRef,
    /// Its fleet: the lead it escorts, or itself.
    pub fleet: ShipRef,
    /// Where it is.
    pub position: Vec2,
    /// Its hit radius.
    pub radius: f32,
    /// How it is holding up.
    pub condition: Condition,
}

impl Target {
    /// Whether a shot fired by `firer` of `fleet` can hit it.
    #[must_use]
    pub fn hittable_by(&self, firer: ShipRef, fleet: ShipRef) -> bool {
        self.ship != firer && self.fleet != fleet && self.condition.hittable()
    }
}

/// How far along the segment from `from` to `to`, as a fraction, it first
/// comes within `radius` of `centre`: 0 when it starts within, `None`
/// when it never does.
#[must_use]
pub fn contact(from: Vec2, to: Vec2, centre: Vec2, radius: f32) -> Option<f32> {
    let along = to - from;
    let off = from - centre;
    let c = off.x * off.x + off.y * off.y - radius * radius;
    if c <= 0.0 {
        return Some(0.0);
    }
    let a = along.x * along.x + along.y * along.y;
    let b = 2.0 * (off.x * along.x + off.y * along.y);
    let discriminant = b * b - 4.0 * a * c;
    if a == 0.0 || discriminant < 0.0 {
        return None;
    }
    let t = (-b - discriminant.sqrt()) / (2.0 * a);
    (0.0..=1.0).contains(&t).then_some(t)
}

/// A shot in flight.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shot {
    /// Its weapon.
    pub weapon: WeaponSpec,
    /// The ship that fired it.
    pub firer: ShipRef,
    /// The firer's fleet.
    pub fleet: ShipRef,
    /// Where it is.
    pub position: Vec2,
    /// How far it moves each tick.
    pub velocity: Vec2,
    /// Which way it faces, in degrees.
    pub heading: f32,
    /// The ticks it has flown.
    pub age: u32,
}

impl Shot {
    /// A shot of `weapon` launched by `firer` of `fleet`, flying `from`,
    /// `offset` degrees off its heading.
    #[must_use]
    pub fn launch(
        weapon: WeaponSpec,
        firer: ShipRef,
        fleet: ShipRef,
        from: &ShipState,
        offset: f32,
    ) -> Self {
        let heading = normalized(from.heading + offset);
        let velocity = match weapon.guidance {
            Guidance::FreefallBomb => from.velocity * BOMB_LAUNCH,
            Guidance::Rocket => from.velocity,
            _ => from.velocity + facing(heading) * weapon.speed,
        };
        Self {
            weapon,
            firer,
            fleet,
            position: from.position,
            velocity,
            heading,
            age: 0,
        }
    }

    /// Flies the shot one tick, and gives where it was.
    pub fn step(&mut self) -> Vec2 {
        match self.weapon.guidance {
            Guidance::Rocket => {
                self.velocity = self.velocity * ROCKET_KEEP
                    + facing(self.heading) * (self.weapon.speed * ROCKET_PUSH);
            }
            Guidance::FreefallBomb if self.velocity != Vec2::ZERO => {
                let off = shortest_turn(self.heading, heading_of(self.velocity));
                self.heading = normalized(self.heading + off.clamp(-BOMB_TURN, BOMB_TURN));
            }
            _ => {}
        }
        let was = self.position;
        self.position = self.position + self.velocity;
        self.age += 1;
        was
    }

    /// Whether it has flown its life.
    #[must_use]
    pub fn expired(&self) -> bool {
        self.age >= self.weapon.lifetime
    }

    /// The first of `targets` it can hit that it met on its way here from
    /// `was`, if any, and where it met it.
    #[must_use]
    pub fn hit(&self, was: Vec2, targets: &[Target]) -> Option<(ShipRef, Vec2)> {
        targets
            .iter()
            .filter(|target| target.hittable_by(self.firer, self.fleet))
            .filter_map(|target| {
                let reach = target.radius + self.weapon.prox_radius;
                Some((
                    contact(was, self.position, target.position, reach)?,
                    target.ship,
                ))
            })
            .min_by(|(a, _), (b, _)| a.total_cmp(b))
            .map(|(t, ship)| (ship, was + (self.position - was) * t))
    }

    /// Its blast where it is, hitting `direct` (or nothing, as it
    /// detonates).
    #[must_use]
    pub fn blast(&self, direct: Option<ShipRef>) -> Blast {
        Blast {
            at: self.position,
            radius: self.weapon.blast_radius,
            direct,
            firer: self.firer,
            spares_player: self.weapon.spares_player(),
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::catalog::WeaponRecord;
    use crate::testkit::weapon;
    use crate::traffic::npc::NpcId;

    const FIRER: ShipRef = ShipRef::Npc(NpcId(1));

    fn spec(guidance: i16, speed: i16, count: i16) -> WeaponSpec {
        WeaponSpec::new(&WeaponRecord {
            count,
            guidance,
            speed,
            ..weapon(128)
        })
    }

    /// A ship at (`x`, `y`) moving (`vx`, `vy`), facing `heading`.
    fn state(x: f32, y: f32, vx: f32, vy: f32, heading: f32) -> ShipState {
        ShipState {
            position: Vec2::new(x, y),
            velocity: Vec2::new(vx, vy),
            heading,
        }
    }

    fn close(a: Vec2, b: Vec2) -> bool {
        (a - b).length() < 1e-4
    }

    #[test]
    fn an_unguided_shot_starts_with_its_firers_velocity_plus_its_speed_ahead() {
        let shot = Shot::launch(
            spec(-1, 1500, 13),
            FIRER,
            FIRER,
            &state(10.0, 20.0, 1.0, -2.0, 90.0),
            0.0,
        );
        assert_eq!(shot.position, Vec2::new(10.0, 20.0));
        assert!(close(shot.velocity, Vec2::new(16.0, -2.0)), "{shot:?}");
        assert_eq!((shot.heading, shot.age), (90.0, 0));
        assert_eq!((shot.firer, shot.fleet), (FIRER, FIRER));
    }

    #[test]
    fn a_launch_offset_turns_the_shot_and_wraps() {
        let shot = Shot::launch(
            spec(-1, 100, 1),
            FIRER,
            FIRER,
            &state(0.0, 0.0, 0.0, 0.0, 355.0),
            9.0,
        );
        assert!((shot.heading - 4.0).abs() < 1e-4, "{shot:?}");
        assert!(close(shot.velocity, facing(4.0)), "{shot:?}");
        let back = Shot::launch(
            spec(-1, 100, 1),
            FIRER,
            FIRER,
            &state(0.0, 0.0, 0.0, 0.0, 5.0),
            -9.0,
        );
        assert!((back.heading - 356.0).abs() < 1e-4, "{back:?}");
    }

    #[test]
    fn an_unguided_shot_flies_straight_for_its_count_then_expires() {
        let mut shot = Shot::launch(
            spec(-1, 1500, 13),
            FIRER,
            FIRER,
            &state(0.0, 0.0, 0.0, 0.0, 0.0),
            0.0,
        );
        for tick in 1..=13 {
            assert!(!shot.expired(), "{tick}");
            let was = shot.position;
            assert_eq!(shot.step(), was);
            assert_eq!(shot.age, tick);
        }
        assert!(shot.expired());
        assert!(close(shot.position, Vec2::new(0.0, -195.0)), "{shot:?}");
        assert!(close(shot.velocity, Vec2::new(0.0, -15.0)));
    }

    #[test]
    fn a_rocket_starts_at_its_firers_velocity_and_closes_on_its_top_speed() {
        let mut rocket = Shot::launch(
            spec(6, 1000, 120),
            FIRER,
            FIRER,
            &state(0.0, 0.0, 0.0, 0.0, 90.0),
            0.0,
        );
        assert_eq!(rocket.velocity, Vec2::ZERO);
        rocket.step();
        assert!(close(rocket.velocity, Vec2::new(0.5, 0.0)), "{rocket:?}");
        assert!(
            close(rocket.position, Vec2::new(0.5, 0.0)),
            "moved after speeding up"
        );
        rocket.step();
        assert!(close(rocket.velocity, Vec2::new(0.975, 0.0)), "{rocket:?}");
        for _ in 0..200 {
            rocket.step();
        }
        assert!((rocket.velocity.x - 10.0).abs() < 1e-3, "{rocket:?}");
        let moving = Shot::launch(
            spec(6, 1000, 120),
            FIRER,
            FIRER,
            &state(0.0, 0.0, 2.0, 0.0, 0.0),
            0.0,
        );
        assert_eq!(moving.velocity, Vec2::new(2.0, 0.0));
        assert_eq!((ROCKET_KEEP, ROCKET_PUSH), (0.95, 0.05));
    }

    #[test]
    fn a_bomb_starts_at_four_fifths_of_its_firers_velocity_and_weathervanes() {
        let mut bomb = Shot::launch(
            spec(5, 1000, 60),
            FIRER,
            FIRER,
            &state(0.0, 0.0, 5.0, 0.0, 0.0),
            0.0,
        );
        assert!(close(bomb.velocity, Vec2::new(4.0, 0.0)), "{bomb:?}");
        assert_eq!(BOMB_LAUNCH, 0.8);
        bomb.step();
        assert!((bomb.heading - 1.0).abs() < 1e-4, "{bomb:?}");
        assert!(close(bomb.velocity, Vec2::new(4.0, 0.0)), "kept");
        assert!(close(bomb.position, Vec2::new(4.0, 0.0)));
        for _ in 0..100 {
            bomb.step();
        }
        assert!(
            (bomb.heading - 90.0).abs() < 1e-3,
            "into the wind: {bomb:?}"
        );
        let mut left = Shot::launch(
            spec(5, 1000, 60),
            FIRER,
            FIRER,
            &state(0.0, 0.0, -5.0, 0.0, 0.0),
            0.0,
        );
        left.step();
        assert!((left.heading - 359.0).abs() < 1e-3, "{left:?}");
        let mut still = Shot::launch(
            spec(5, 1000, 60),
            FIRER,
            FIRER,
            &state(0.0, 0.0, 0.0, 0.0, 30.0),
            0.0,
        );
        still.step();
        assert_eq!(still.heading, 30.0, "no wind");
        assert_eq!(BOMB_TURN, 1.0);
    }

    #[test]
    fn a_shots_blast_is_where_it_is() {
        let record = WeaponRecord {
            blast_radius: 90,
            flags: 0x8100,
            ..weapon(140)
        };
        let shot = Shot {
            position: Vec2::new(3.0, 4.0),
            ..Shot::launch(
                WeaponSpec::new(&record),
                FIRER,
                FIRER,
                &ShipState::default(),
                0.0,
            )
        };
        assert_eq!(
            shot.blast(None),
            Blast {
                at: Vec2::new(3.0, 4.0),
                radius: 90.0,
                direct: None,
                firer: FIRER,
                spares_player: true,
            }
        );
        assert_eq!(
            shot.blast(Some(ShipRef::Player)).direct,
            Some(ShipRef::Player)
        );
    }

    fn target(id: u32, x: f32, y: f32, condition: Condition) -> Target {
        Target {
            ship: ShipRef::Npc(NpcId(id)),
            fleet: ShipRef::Npc(NpcId(id)),
            position: Vec2::new(x, y),
            radius: 5.0,
            condition,
        }
    }

    /// A shot from (0, 0) to (100, 0) this tick.
    fn passing(prox: i16) -> Shot {
        let record = WeaponRecord {
            speed: 10_000,
            count: 10,
            prox_radius: prox,
            ..weapon(231)
        };
        let mut shot = Shot::launch(
            WeaponSpec::new(&record),
            FIRER,
            FIRER,
            &state(0.0, 0.0, 0.0, 0.0, 90.0),
            0.0,
        );
        shot.step();
        shot
    }

    #[test]
    fn a_fast_shot_hits_a_ship_it_passes_between_ticks() {
        let shot = passing(0);
        assert!(close(shot.position, Vec2::new(100.0, 0.0)));
        let hit = |targets: &[Target]| shot.hit(Vec2::ZERO, targets).map(|(ship, _)| ship);
        assert_eq!(
            hit(&[target(2, 50.0, 4.0, Condition::Intact)]),
            Some(ShipRef::Npc(NpcId(2)))
        );
        assert_eq!(
            hit(&[target(2, 50.0, 5.5, Condition::Intact)]),
            None,
            "wide"
        );
        assert_eq!(
            hit(&[target(2, 110.0, 0.0, Condition::Intact)]),
            None,
            "ahead"
        );
        assert_eq!(
            hit(&[target(2, -10.0, 0.0, Condition::Intact)]),
            None,
            "behind"
        );
        assert_eq!(
            hit(&[
                target(2, 80.0, 0.0, Condition::Intact),
                target(3, 30.0, 0.0, Condition::Intact),
                target(4, 60.0, 0.0, Condition::Intact),
            ]),
            Some(ShipRef::Npc(NpcId(3))),
            "the first met"
        );
        assert_eq!(hit(&[]), None);
    }

    #[test]
    fn a_proximity_fuse_widens_the_hit() {
        let shot = passing(3);
        let (ship, at) = shot
            .hit(Vec2::ZERO, &[target(2, 50.0, 7.9, Condition::Intact)])
            .expect("a hit");
        assert_eq!(ship, ShipRef::Npc(NpcId(2)));
        assert!(
            at.y.abs() < 1e-3 && at.x > 48.5 && at.x < 49.0,
            "met on its way: {at:?}"
        );
        assert_eq!(
            shot.hit(Vec2::ZERO, &[target(2, 50.0, 8.1, Condition::Intact)]),
            None
        );
    }

    #[test]
    fn a_shot_never_hits_its_firer_or_its_fleet() {
        let shot = passing(0);
        let firer = Target {
            ship: FIRER,
            fleet: ShipRef::Npc(NpcId(7)),
            ..target(1, 50.0, 0.0, Condition::Intact)
        };
        let mate = Target {
            fleet: FIRER,
            ..target(2, 40.0, 0.0, Condition::Intact)
        };
        assert_eq!(shot.hit(Vec2::ZERO, &[firer, mate]), None);
        let escort_of_the_same_lead = Shot {
            fleet: ShipRef::Npc(NpcId(9)),
            ..shot
        };
        let other_escort = Target {
            fleet: ShipRef::Npc(NpcId(9)),
            ..target(3, 30.0, 0.0, Condition::Intact)
        };
        assert_eq!(
            escort_of_the_same_lead.hit(Vec2::ZERO, &[other_escort]),
            None
        );
        let (ship, at) = shot
            .hit(Vec2::ZERO, &[other_escort])
            .expect("another fleet");
        assert_eq!(ship, ShipRef::Npc(NpcId(3)));
        assert!(close(at, Vec2::new(25.0, 0.0)), "met at its edge: {at:?}");
    }

    #[test]
    fn a_disabled_ship_can_be_hit_and_a_dying_or_destroyed_one_cannot() {
        let shot = passing(0);
        let hit = |condition| {
            shot.hit(Vec2::ZERO, &[target(2, 50.0, 0.0, condition)])
                .map(|(ship, _)| ship)
        };
        assert_eq!(hit(Condition::Disabled), Some(ShipRef::Npc(NpcId(2))));
        assert_eq!(hit(Condition::Dying { ticks_left: 3 }), None);
        assert_eq!(hit(Condition::Destroyed), None);
    }

    #[test]
    fn a_segment_meets_a_circle_where_it_first_enters() {
        let at = |from: (f32, f32), to: (f32, f32), r| {
            contact(
                Vec2::new(from.0, from.1),
                Vec2::new(to.0, to.1),
                Vec2::new(10.0, 0.0),
                r,
            )
        };
        assert_eq!(at((0.0, 0.0), (20.0, 0.0), 5.0), Some(0.25));
        assert_eq!(at((9.0, 0.0), (20.0, 0.0), 5.0), Some(0.0), "starts within");
        assert_eq!(at((0.0, 0.0), (4.0, 0.0), 5.0), None, "short of it");
        assert_eq!(at((0.0, 0.0), (5.0, 0.0), 5.0), Some(1.0), "just reaches");
        assert_eq!(at((0.0, 6.0), (20.0, 6.0), 5.0), None, "beside it");
        assert_eq!(at((0.0, 0.0), (0.0, 0.0), 5.0), None, "still, outside");
        assert_eq!(at((20.0, 0.0), (30.0, 0.0), 5.0), None, "moving away");
    }
}
