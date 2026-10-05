//! Shots: launched from a ship, flown a tick at a time, and checked
//! against the ships they pass.
//!
//! The values are the original's (`_SpawnShot`, `_HandleShotGuidance`,
//! `_HandleShot` and `_ShotCanHitShip` in the `EV Nova` executable):
//!
//! - A shot starts at its firer's position with its firer's velocity (a
//!   freefall bomb with [`BOMB_LAUNCH`] of it), heading as it was aimed;
//!   any but a bomb or a rocket then adds its speed along its heading. A
//!   homing shot never carries its firer's velocity.
//! - Each tick an unguided shot keeps its velocity. A rocket's velocity
//!   becomes [`ROCKET_KEEP`] of itself and [`ROCKET_PUSH`] of its top
//!   speed along its heading, so it closes on its top speed. A bomb keeps
//!   its velocity and turns its heading [`BOMB_TURN`] a tick towards it.
//! - A homing shot flies at its speed along its heading every tick. Once
//!   it has flown [`HOMING_DELAY`] ticks it steers at its target, turning
//!   `GuidedTurn` a tick the short way while the target's bearing is
//!   further off than that, and keeping its heading otherwise. Once its
//!   target is gone (destroyed, landed or jumped out) or breaking up, it is
//!   lost: it flies straight on and hits nothing. One fired with no target
//!   flies straight.
//! - A shot lives its weapon's `Count` ticks, then detonates if its weapon
//!   says so (`Flags` 0x8000) and otherwise vanishes.
//! - It hits the first ship ([`Target`]) it passes within the ship's hit
//!   radius and its weapon's `ProxRadius`, swept from where it was to where
//!   it is so that a fast shot cannot pass through a ship between ticks. A
//!   shot cannot hit its firer, a ship of its firer's fleet, or a ship
//!   breaking up or destroyed; a disabled ship it can. A homing shot hits
//!   only its target, unless its weapon's proximity fuse is set off by
//!   other ships (`Flags2` 0x0008), and nothing once lost.

use super::ShipRef;
use super::aim::bearing;
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
/// How many ticks a homing shot flies straight before it steers.
pub const HOMING_DELAY: u32 = 15;

/// A ship a shot or beam might hit.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Target {
    /// Which ship.
    pub ship: ShipRef,
    /// Its fleet: the lead it escorts, or itself.
    pub fleet: ShipRef,
    /// Where it is.
    pub position: Vec2,
    /// How it is moving.
    pub velocity: Vec2,
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
    // A segment of no length (`a` none) starting outside, or a line that
    // misses (a negative discriminant), makes `t` NaN, which is in no
    // range.
    let t = (-b - discriminant.sqrt()) / (2.0 * a);
    (0.0..=1.0).contains(&t).then_some(t)
}

/// A shot's number in its fight, never reused.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ShotId(pub u32);

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
    /// Its number in its fight.
    pub id: ShotId,
    /// The ship it was fired at, if any: a homing shot's quarry.
    pub target: Option<ShipRef>,
    /// Whether a homing shot has lost its target: it flies straight on
    /// and hits nothing.
    pub lost: bool,
    /// What point defence must still take off it before the next hit
    /// destroys it.
    pub durability: f32,
    /// How many releases of sub-munitions it is from a ship's firing: 0
    /// for a shot a ship fired.
    pub generation: u32,
}

impl Shot {
    /// A shot of `weapon` launched by `firer` of `fleet`, flying `from`
    /// along `heading`.
    #[must_use]
    pub fn launch(
        weapon: WeaponSpec,
        firer: ShipRef,
        fleet: ShipRef,
        from: &ShipState,
        heading: f32,
    ) -> Self {
        let heading = normalized(heading);
        let velocity = match weapon.guidance {
            Guidance::FreefallBomb => from.velocity * BOMB_LAUNCH,
            Guidance::Rocket => from.velocity,
            Guidance::Homing => facing(heading) * weapon.speed,
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
            id: ShotId::default(),
            target: None,
            lost: false,
            durability: weapon.durability,
            generation: 0,
        }
    }

    /// Flies the shot one tick, a homing one steering at its target among
    /// `targets`, and gives where it was.
    pub fn step(&mut self, targets: &[Target]) -> Vec2 {
        match self.weapon.guidance {
            Guidance::Homing => self.home(targets),
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

    /// Steers a homing shot a tick at its target among `targets`, which
    /// it loses once the target is gone or breaking up, and sets its
    /// velocity to its speed along its heading (see the module docs).
    fn home(&mut self, targets: &[Target]) {
        if let Some(ship) = self.target {
            let target = targets
                .iter()
                .find(|target| target.ship == ship && target.condition.hittable());
            match target {
                None => {
                    self.target = None;
                    self.lost = true;
                }
                Some(target) if self.age >= HOMING_DELAY => {
                    let wanted = bearing(self.position, target.position);
                    let off = shortest_turn(self.heading, wanted);
                    let turn = self.weapon.turn;
                    if off.abs() > turn {
                        self.heading = normalized(self.heading + turn.copysign(off));
                    }
                }
                Some(_) => {}
            }
        }
        self.velocity = facing(self.heading) * self.weapon.speed;
    }

    /// Whether it can hit `target`: never its firer, its fleet or a ship
    /// past hitting, and a homing shot only its own target (or, set off
    /// by other ships, any other), and nothing once lost.
    #[must_use]
    pub fn can_hit(&self, target: &Target) -> bool {
        let guided = match self.weapon.guidance {
            Guidance::Homing => {
                !self.lost
                    && (self.weapon.proximity_by_others() || self.target == Some(target.ship))
            }
            _ => true,
        };
        guided && target.hittable_by(self.firer, self.fleet)
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
            .filter(|target| self.can_hit(target))
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
            90.0,
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
            364.0,
        );
        assert!((shot.heading - 4.0).abs() < 1e-4, "{shot:?}");
        assert!(close(shot.velocity, facing(4.0)), "{shot:?}");
        let back = Shot::launch(
            spec(-1, 100, 1),
            FIRER,
            FIRER,
            &state(0.0, 0.0, 0.0, 0.0, 5.0),
            -4.0,
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
            assert_eq!(shot.step(&[]), was);
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
            90.0,
        );
        assert_eq!(rocket.velocity, Vec2::ZERO);
        rocket.step(&[]);
        assert!(close(rocket.velocity, Vec2::new(0.5, 0.0)), "{rocket:?}");
        assert!(
            close(rocket.position, Vec2::new(0.5, 0.0)),
            "moved after speeding up"
        );
        rocket.step(&[]);
        assert!(close(rocket.velocity, Vec2::new(0.975, 0.0)), "{rocket:?}");
        for _ in 0..200 {
            rocket.step(&[]);
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
        bomb.step(&[]);
        assert!((bomb.heading - 1.0).abs() < 1e-4, "{bomb:?}");
        assert!(close(bomb.velocity, Vec2::new(4.0, 0.0)), "kept");
        assert!(close(bomb.position, Vec2::new(4.0, 0.0)));
        for _ in 0..100 {
            bomb.step(&[]);
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
        left.step(&[]);
        assert!((left.heading - 359.0).abs() < 1e-3, "{left:?}");
        let mut still = Shot::launch(
            spec(5, 1000, 60),
            FIRER,
            FIRER,
            &state(0.0, 0.0, 0.0, 0.0, 30.0),
            30.0,
        );
        still.step(&[]);
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
            velocity: Vec2::ZERO,
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
            90.0,
        );
        shot.step(&[]);
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

    // Homing.

    const TARGET: ShipRef = ShipRef::Npc(NpcId(2));

    /// An IR Missile: 10 pixels a tick for 100 ticks, turning 7 degrees a
    /// tick, with `flags2`.
    fn missile(flags2: u16) -> WeaponSpec {
        WeaponSpec::new(&WeaponRecord {
            guidance: 1,
            speed: 1000,
            count: 100,
            guided_turn: 70,
            durability: 4,
            flags2,
            ..weapon(134)
        })
    }

    /// `missile` launched from the centre heading right at `target`.
    fn homing(weapon: WeaponSpec, target: Option<ShipRef>) -> Shot {
        Shot {
            target,
            ..Shot::launch(weapon, FIRER, FIRER, &state(0.0, 0.0, 0.0, 0.0, 90.0), 90.0)
        }
    }

    #[test]
    fn a_homing_shot_flies_straight_for_15_ticks_then_turns_its_turn_a_tick() {
        // The target is straight down, 90 degrees off its heading.
        let below = [target(2, 0.0, 1000.0, Condition::Intact)];
        let mut shot = homing(missile(0), Some(TARGET));
        for tick in 1..=15 {
            shot.step(&below);
            assert_eq!(shot.heading, 90.0, "{tick}");
        }
        assert!(close(shot.position, Vec2::new(150.0, 0.0)), "{shot:?}");
        shot.step(&below);
        assert!((shot.heading - 97.0).abs() < 1e-4, "{shot:?}");
        assert!(close(shot.velocity, facing(97.0) * 10.0), "{shot:?}");
        shot.step(&below);
        assert!((shot.heading - 104.0).abs() < 1e-4, "{shot:?}");
        let above = [target(2, 300.0, -1000.0, Condition::Intact)];
        let mut left = homing(missile(0), Some(TARGET));
        for _ in 0..16 {
            left.step(&above);
        }
        assert!(
            (left.heading - 83.0).abs() < 1e-4,
            "the short way: {left:?}"
        );
        assert_eq!(HOMING_DELAY, 15);
    }

    #[test]
    fn a_homing_shot_within_its_turn_of_the_bearing_keeps_its_heading() {
        // 500 pixels on from where it is on its 16th tick, 5 degrees up.
        let near_ahead = 5.0_f32.to_radians().tan() * 500.0;
        let ahead = [target(2, 650.0, -near_ahead, Condition::Intact)];
        let mut shot = homing(missile(0), Some(TARGET));
        for _ in 0..16 {
            shot.step(&ahead);
        }
        assert_eq!(shot.heading, 90.0, "not snapped to the bearing");
        // Straight up from where it is on its 16th tick: exactly 90
        // degrees off, its turn.
        let sharp = WeaponSpec {
            turn: 90.0,
            ..missile(0)
        };
        let above = [target(2, 150.0, -1000.0, Condition::Intact)];
        let mut shot = homing(sharp, Some(TARGET));
        for _ in 0..16 {
            shot.step(&above);
        }
        assert_eq!(shot.heading, 90.0, "exactly its turn off: kept");
    }

    #[test]
    fn a_homing_shot_flies_at_its_speed_whatever_its_firer_does() {
        let mut shot = Shot::launch(
            missile(0),
            FIRER,
            FIRER,
            &state(0.0, 0.0, 5.0, -3.0, 180.0),
            180.0,
        );
        assert!(close(shot.velocity, Vec2::new(0.0, 10.0)), "{shot:?}");
        shot.velocity = Vec2::new(4.0, 4.0);
        shot.step(&[]);
        assert!(close(shot.position, Vec2::new(0.0, 10.0)), "{shot:?}");
    }

    /// A homing shot that flew from (0, 0) to (100, 0) this tick, at
    /// `target`, past ship 3 at (30, 0) to ship 2 at (60, 0).
    fn passing_homing(flags2: u16, target: Option<ShipRef>) -> Shot {
        Shot {
            position: Vec2::new(100.0, 0.0),
            ..homing(missile(flags2), target)
        }
    }

    fn in_its_path() -> [Target; 2] {
        [
            target(3, 30.0, 0.0, Condition::Intact),
            target(2, 60.0, 0.0, Condition::Intact),
        ]
    }

    #[test]
    fn a_homing_shot_hits_its_target_and_passes_through_any_other() {
        let shot = passing_homing(0, Some(TARGET));
        assert_eq!(
            shot.hit(Vec2::ZERO, &in_its_path()).map(|(ship, _)| ship),
            Some(TARGET)
        );
        let at_three = passing_homing(0, Some(ShipRef::Npc(NpcId(3))));
        assert_eq!(
            at_three
                .hit(Vec2::ZERO, &in_its_path())
                .map(|(ship, _)| ship),
            Some(ShipRef::Npc(NpcId(3)))
        );
    }

    #[test]
    fn a_homing_shot_set_off_by_other_ships_hits_the_first_it_meets() {
        let shot = passing_homing(0x0008, Some(TARGET));
        assert_eq!(
            shot.hit(Vec2::ZERO, &in_its_path()).map(|(ship, _)| ship),
            Some(ShipRef::Npc(NpcId(3)))
        );
        let untargeted = passing_homing(0x0008, None);
        assert_eq!(
            untargeted
                .hit(Vec2::ZERO, &in_its_path())
                .map(|(ship, _)| ship),
            Some(ShipRef::Npc(NpcId(3)))
        );
    }

    #[test]
    fn a_homing_shot_without_a_target_flies_straight_and_hits_nothing() {
        let mut shot = homing(missile(0), None);
        for _ in 0..30 {
            shot.step(&in_its_path());
        }
        assert_eq!(shot.heading, 90.0);
        assert!(!shot.lost);
        assert_eq!(
            passing_homing(0, None).hit(Vec2::ZERO, &in_its_path()),
            None
        );
    }

    #[test]
    fn a_homing_shot_whose_target_is_gone_or_breaking_up_is_lost() {
        for gone in [
            vec![],
            vec![target(2, 0.0, 1000.0, Condition::Dying { ticks_left: 9 })],
            vec![target(2, 0.0, 1000.0, Condition::Destroyed)],
        ] {
            let mut shot = homing(missile(0), Some(TARGET));
            for _ in 0..20 {
                shot.step(&gone);
            }
            assert!(shot.lost, "{gone:?}");
            assert_eq!(shot.target, None, "{gone:?}");
            assert_eq!(shot.heading, 90.0, "flies straight on: {gone:?}");
            let back = [target(2, 0.0, 1000.0, Condition::Intact)];
            shot.step(&back);
            assert_eq!(shot.heading, 90.0, "lost for good: {gone:?}");
        }
        let lost = Shot {
            lost: true,
            ..passing_homing(0x0008, Some(TARGET))
        };
        assert_eq!(lost.hit(Vec2::ZERO, &in_its_path()), None, "hits nothing");
        let disabled = [target(2, 0.0, 1000.0, Condition::Disabled)];
        let mut chasing = homing(missile(0), Some(TARGET));
        for _ in 0..16 {
            chasing.step(&disabled);
        }
        assert!(
            !chasing.lost && chasing.heading > 90.0,
            "a disabled one is chased"
        );
    }

    #[test]
    fn a_shot_starts_unaimed_whole_and_of_the_first_generation() {
        let shot = homing(missile(0), None);
        assert_eq!(
            (shot.id, shot.lost, shot.durability, shot.generation),
            (ShotId(0), false, 4.0, 0)
        );
        let unguided = passing(0);
        assert_eq!((unguided.target, unguided.durability), (None, 0.0));
    }

    #[test]
    fn only_homing_shots_steer() {
        let below = [target(2, 0.0, 1000.0, Condition::Intact)];
        let mut turret = Shot {
            target: Some(TARGET),
            ..Shot::launch(
                spec(4, 1000, 100),
                FIRER,
                FIRER,
                &state(0.0, 0.0, 0.0, 0.0, 90.0),
                90.0,
            )
        };
        for _ in 0..20 {
            turret.step(&below);
        }
        assert_eq!(turret.heading, 90.0);
        assert!(!turret.lost);
        let passing_turret = Shot {
            position: Vec2::new(100.0, 0.0),
            ..turret
        };
        assert_eq!(
            passing_turret
                .hit(Vec2::ZERO, &in_its_path())
                .map(|(ship, _)| ship),
            Some(ShipRef::Npc(NpcId(3))),
            "any ship it meets, its target or not"
        );
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
        assert_eq!(
            at((10.0, 4.0), (30.0, 4.0), 5.0),
            Some(0.0),
            "within, off the axis"
        );
        assert_eq!(
            at((10.0, 6.0), (30.0, 6.0), 5.0),
            None,
            "beside it, off the axis"
        );
        assert_eq!(
            at((10.0, -20.0), (10.0, 20.0), 5.0),
            Some(0.375),
            "across it"
        );
    }
}
