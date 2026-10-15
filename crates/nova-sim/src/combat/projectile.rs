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
//!   further off than that, and keeping its heading otherwise, a target
//!   breaking up included. One fired with no target flies straight.
//! - Any shot but point defence whose ship ([`Shot::aimed_at`]) is gone
//!   (destroyed, or out of the fight as it landed or jumped) is lost
//!   ([`Jam::Lost`]) before it next moves: it flies on, aimed at nothing,
//!   and hits no ship for the rest of its life (`_HandleShot`
//!   @0x355e0-0x35621). A homing one then flies straight.
//! - A shot lives its weapon's `Count` ticks, then detonates if its weapon
//!   says so (`Flags` 0x8000) and otherwise vanishes, a lost one too.
//! - It hits the first ship ([`Target`]) it touches, within the ship's hit
//!   radius, or else, with a proximity fuse, the first it passes within
//!   that and its weapon's `ProxRadius` ([`Fuse`]), swept from where it was
//!   to where it is so that a fast shot cannot pass through a ship between
//!   ticks; `ProxSafety` holds a touch until its age reaches it and the
//!   fuse until its age is past it ([`Shot::hit`]). Its last move is never
//!   tested. Which ships it can hit is [`sides`](super::sides)' rule, the
//!   original's `_ShotCanHitShip`: never its firer, the ships of its
//!   firer's government or under its firer's leaders, the escape pod or a
//!   destroyed ship, among others, but a disabled or breaking-up one; a
//!   homing shot only its target, unless its weapon's proximity fuse is
//!   set off by other ships (`Flags2` 0x0008), and nothing once lost.

use super::ShipRef;
use super::aim::bearing;
use super::damage::Blast;
use super::hull::Condition;
use super::sides::{Sides, Ties};
use super::weapon::{Guidance, WeaponSpec};
use crate::catalog::{GovtId, ShipId};
use crate::flight::{ShipState, facing, heading_of, normalized, shortest_turn};
use crate::geometry::Vec2;
use crate::rulebook::RuleSource;

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
/// The age a player's homing missile locks on its target past (the 30.0
/// @0xdd638, tested "> 30" @0x357ab).
pub const LOCK_AGE: u32 = 30;

/// A ship a shot or beam might hit.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Target {
    /// Which ship.
    pub ship: ShipRef,
    /// Its ship type.
    pub ship_type: ShipId,
    /// Its fleet: the lead it escorts, or itself, which point defence
    /// reads.
    pub fleet: ShipRef,
    /// Its leader, government and `düde`, which the hit rules read.
    pub ties: Ties,
    /// Where it is.
    pub position: Vec2,
    /// How it is moving.
    pub velocity: Vec2,
    /// Its hit radius.
    pub radius: f32,
    /// How it is holding up.
    pub condition: Condition,
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

/// How a shot met a ship.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fuse {
    /// It touched the ship: the original's sprite collision passes, which
    /// release no sub-munitions (`_ShipCollisionHandler` passes
    /// `spawnSubs` 0, @0x37095).
    Contact,
    /// It came within its proximity fuse's reach of the ship:
    /// `_CheckShotProximities`, which releases its sub-munitions
    /// (`spawnSubs` 1, @0x37678).
    Proximity,
}

/// Where and how a shot met the ship it hits ([`Shot::hit`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Meeting {
    /// The ship.
    pub ship: ShipRef,
    /// Where on its move it met it.
    pub at: Vec2,
    /// How.
    pub fuse: Fuse,
}

/// How a shot is jammed off its ship: the original's shot +0x30, which
/// `_ShotCanHitShip` (@0x447f7) and `_CalculateTargetedDamage` read as
/// "jammed" whenever it is not 0.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Jam {
    /// Not jammed (0).
    #[default]
    Clear,
    /// Lost, its ship gone (998, `_HandleShot` @0x3561b).
    Lost,
}

/// How often a decaying shot loses a point of its direct hit's mass and
/// energy damage, as the rulebook's
/// [`RuleKey::ShotDecay`](crate::RuleKey::ShotDecay) says.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DecayStep {
    /// The original's: every `Decay` + 1 ticks, as `_HandleShot` adds a
    /// tick to the shot's count (+0x18) and takes a step (+0x32) only
    /// once the count is above `Decay`, starting again from none
    /// (@0x35eb2-0x35efc).
    #[default]
    EveryDecayPlusOne,
    /// The Bible's: "every time this number of frames goes by", every
    /// `Decay` ticks.
    EveryDecay,
}

impl From<RuleSource> for DecayStep {
    fn from(source: RuleSource) -> Self {
        match source {
            RuleSource::Engine => Self::EveryDecayPlusOne,
            RuleSource::Bible => Self::EveryDecay,
        }
    }
}

/// A shot's number in its fight, never reused.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ShotId(pub u32);

/// What launched a shot, as `_ShotCanHitShip` reads its source slot
/// (+0x2c).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    /// A ship, its [`Shot::firer`]: here the copy of its ties taken as it
    /// fired, which the hit rules read once the firer has left the fight
    /// (see [`sides`](super::sides)).
    Ship(Ties),
    /// Something that is not a ship, a source slot above 0x3f such as a
    /// stellar's: such a shot hits only the ship it is aimed at
    /// ([`Shot::aimed_at`]), or any ship when it is aimed at none. Its
    /// [`Shot::firer`] is whatever its launcher put there, which the hit
    /// rules never read.
    Other,
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
    /// What launched it: a ship, with a copy of its ties as it fired, or
    /// something that is not a ship.
    pub source: Source,
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
    /// The ship its damage counts as aimed at (the original's shot +0x2a,
    /// `_SpawnShot`'s second argument): its firer's target, whatever its
    /// guidance; none for a front-quadrant turret firing ahead or for
    /// point defence, and a sub-munition's own target. See
    /// [`Combat::targeted_damage`](super::Combat::targeted_damage).
    pub aimed_at: Option<ShipRef>,
    /// Whether it is jammed off its ship (the original's shot +0x30): a
    /// shot jammed any way hits no ship, and its damage counts as aimed
    /// at none.
    pub jam: Jam,
    /// What point defence must still take off it before the next hit
    /// destroys it.
    pub durability: f32,
    /// How many releases of sub-munitions it is from a ship's firing: 0
    /// for a shot a ship fired.
    pub generation: u32,
    /// Whether it never destroys the ships it hits, directly or by its
    /// blast (the original's shot +0x44): its weapon's
    /// [`disable_only`](WeaponSpec::disable_only), set at launch
    /// (`_SpawnShot` @0x3e7e2), a sub-munition's from its own weapon.
    pub disable_only: bool,
}

impl Shot {
    /// A shot of `weapon` launched by the ship `firer` of `fleet`, flying
    /// `from` along `heading`, tied to no one until it is given its
    /// firer's ties.
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
            source: Source::Ship(Ties::default()),
            position: from.position,
            velocity,
            heading,
            age: 0,
            id: ShotId::default(),
            target: None,
            aimed_at: None,
            jam: Jam::Clear,
            durability: weapon.durability,
            generation: 0,
            disable_only: weapon.disable_only(),
        }
    }

    /// Flies the shot one tick, a homing one steering at its target among
    /// `targets`, and gives where it was. First, a shot whose ship has
    /// gone from `targets` is lost (see the module docs).
    pub fn step(&mut self, targets: &[Target]) -> Vec2 {
        self.lose_a_gone_ship(targets);
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

    /// `_HandleShot`'s target rule (@0x355e0-0x35621), run before the
    /// shot's life is counted down and it moves: a shot not jammed, and
    /// not point defence, whose ship ([`Shot::aimed_at`], +0x2a) is not
    /// active among `targets` (destroyed, or out of the fight as it
    /// landed, jumped or was taken out: the slot's +0xb8) is lost, jam
    /// 998, and aimed at nothing from then on. A jammed shot keeps its
    /// ship. Point defence (guidance 9) is aimed at no ship every tick
    /// and is never lost. A ship breaking up is still active, so a shot
    /// at it is not lost.
    fn lose_a_gone_ship(&mut self, targets: &[Target]) {
        if self.weapon.guidance == Guidance::PointDefence {
            self.aimed_at = None;
            return;
        }
        if self.jam != Jam::Clear {
            return;
        }
        let Some(ship) = self.aimed_at else {
            return;
        };
        let active = targets
            .iter()
            .any(|target| target.ship == ship && target.condition.active());
        if !active {
            self.jam = Jam::Lost;
            self.aimed_at = None;
            self.target = None;
        }
    }

    /// Steers a homing shot a tick at its target among `targets`, while
    /// that is active, breaking up included (`_HandleShotGuidance` steers
    /// at any active slot), and sets its velocity to its speed along its
    /// heading (see the module docs).
    fn home(&mut self, targets: &[Target]) {
        let target = self.target.and_then(|ship| {
            targets
                .iter()
                .find(|target| target.ship == ship && target.condition.active())
        });
        if let Some(target) = target.filter(|_| self.age >= HOMING_DELAY) {
            let wanted = bearing(self.position, target.position);
            let off = shortest_turn(self.heading, wanted);
            let turn = self.weapon.turn;
            if off.abs() > turn {
                self.heading = normalized(self.heading + turn.copysign(off));
            }
        }
        self.velocity = facing(self.heading) * self.weapon.speed;
    }

    /// Its firer's government as it fired, or `None` for an independent
    /// ship, the player, or a shot no ship fired.
    #[must_use]
    pub fn govt(&self) -> Option<GovtId> {
        match self.source {
            Source::Ship(ties) => ties.govt,
            Source::Other => None,
        }
    }

    /// Whether it can hit `target`, among the fight's `sides`
    /// ([`Sides::shot_can_hit`]).
    #[must_use]
    pub fn can_hit(&self, target: &Target, sides: &Sides) -> bool {
        sides.shot_can_hit(self, target)
    }

    /// Whether it has flown its life.
    #[must_use]
    pub fn expired(&self) -> bool {
        self.age >= self.weapon.lifetime
    }

    /// The decay steps it has taken in the ticks it has flown, a step as
    /// `step` says: none for a weapon of no `Decay`. A sub-munition counts
    /// from its release, by its own weapon's `Decay`. Every tick of a
    /// shot's flight reaches the count (`_HandleShot` @0x35eb2), so it is
    /// read from its age.
    #[must_use]
    pub fn decay_steps(&self, step: DecayStep) -> u32 {
        let decay = self.weapon.decay;
        match step {
            _ if decay == 0 => 0,
            DecayStep::EveryDecayPlusOne => self.age / (decay + 1),
            DecayStep::EveryDecay => self.age / decay,
        }
    }

    /// Whether the tick it has just flown took it past [`LOCK_AGE`], as
    /// `_HandleShot` reads its age, `Count` less its life: once before the
    /// life is counted down (@0x3565b) and once after the move (@0x3578a).
    /// The life of a shot just spent is put at -1 between the two
    /// (@0x356c1), so its last move reads an age of its `Count` + 1, and a
    /// shot of `Count` 30 passes it on that move.
    #[must_use]
    pub fn passes_lock_age(&self) -> bool {
        let before = self.age.saturating_sub(1);
        let after = if self.expired() {
            self.weapon.lifetime + 1
        } else {
            self.age
        };
        before <= LOCK_AGE && after > LOCK_AGE
    }

    /// The first of the ships among `sides` it can hit that it met on its
    /// way here from `was`, if any, where it met it, and how: by contact
    /// within the ship's hit radius, the original's sprite collision
    /// passes (`_ShipCollisionHandler` @0x36f87-0x370a6), or else, with a
    /// proximity fuse, within the radius and its weapon's `ProxRadius`
    /// (`_CheckShotProximities` @0x37553-0x3768a), which runs after them,
    /// so a contact anywhere on the move is the hit. `ProxSafety` arms
    /// each: contact once its age has reached it (@0x37066-0x37086),
    /// proximity once its age is past it (@0x3720e-0x3721e).
    ///
    /// Each pass here takes the ship met first along the move; the
    /// original's contact pass goes in sprite-collision order and its
    /// proximity pass takes the first active ship in slot order at the
    /// shot's end point, which crosses no boundary within the contact pass
    /// and is weapons-fidelity phase 16's for the proximity pass.
    #[must_use]
    pub fn hit(&self, was: Vec2, sides: &Sides) -> Option<Meeting> {
        let first = |fuse: Fuse, prox: f32| {
            sides
                .targets
                .iter()
                .filter(|target| self.can_hit(target, sides))
                .filter_map(|target| {
                    let reach = target.radius + prox;
                    Some((
                        contact(was, self.position, target.position, reach)?,
                        target.ship,
                    ))
                })
                .min_by(|(a, _), (b, _)| a.total_cmp(b))
                .map(|(t, ship)| Meeting {
                    ship,
                    at: was + (self.position - was) * t,
                    fuse,
                })
        };
        let safety = self.weapon.prox_safety;
        let prox = self.weapon.prox_radius;
        let touched = (self.age >= safety)
            .then(|| first(Fuse::Contact, 0.0))
            .flatten();
        touched.or_else(|| {
            (self.age > safety && prox > 0.0)
                .then(|| first(Fuse::Proximity, prox))
                .flatten()
        })
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
    use crate::catalog::{GovtRecord, WeaponRecord};
    use crate::combat::sides::PassThrough;
    use crate::govt::{Governments, NEVER_ATTACKS_PLAYER, PLAYER_SHOTS_PASS};
    use crate::testkit::{among, govt, weapon};
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
    fn a_shot_is_disable_only_as_its_weapon_is() {
        let from = state(0.0, 0.0, 0.0, 0.0, 90.0);
        for (flags2, disable_only) in [(0x1000, true), (0, false)] {
            let weapon = WeaponSpec::new(&WeaponRecord {
                flags2,
                ..weapon(128)
            });
            let shot = Shot::launch(weapon, FIRER, FIRER, &from, 90.0);
            assert_eq!(shot.disable_only, disable_only, "{flags2:#x}");
        }
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
            ship_type: ShipId(128),
            fleet: ShipRef::Npc(NpcId(id)),
            ties: Ties::default(),
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
        let hit = |targets: &[Target]| shot.hit(Vec2::ZERO, &among(targets)).map(|met| met.ship);
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
        let Meeting { ship, at, .. } = shot
            .hit(
                Vec2::ZERO,
                &among(&[target(2, 50.0, 7.9, Condition::Intact)]),
            )
            .expect("a hit");
        assert_eq!(ship, ShipRef::Npc(NpcId(2)));
        assert!(
            at.y.abs() < 1e-3 && at.x > 48.5 && at.x < 49.0,
            "met on its way: {at:?}"
        );
        assert_eq!(
            shot.hit(
                Vec2::ZERO,
                &among(&[target(2, 50.0, 8.1, Condition::Intact)])
            ),
            None
        );
    }

    #[test]
    fn a_shot_within_a_ships_radius_hits_it_by_contact_and_within_its_reach_by_proximity() {
        let shot = passing(3);
        let met = |y| {
            shot.hit(Vec2::ZERO, &among(&[target(2, 50.0, y, Condition::Intact)]))
                .map(|met| met.fuse)
        };
        assert_eq!(met(4.0), Some(Fuse::Contact));
        assert_eq!(met(7.9), Some(Fuse::Proximity));
        assert_eq!(met(8.1), None);
        let no_fuse = passing(0);
        let on = no_fuse.hit(
            Vec2::ZERO,
            &among(&[target(2, 50.0, 4.0, Condition::Intact)]),
        );
        assert_eq!(on.map(|met| met.fuse), Some(Fuse::Contact));
    }

    #[test]
    fn a_contact_anywhere_on_the_move_comes_before_a_nearer_proximity_meeting() {
        // Ship 3 is within the fuse's reach early on; ship 2 sits on the
        // path further along.
        let shot = passing(3);
        let targets = [
            target(3, 20.0, 7.0, Condition::Intact),
            target(2, 80.0, 0.0, Condition::Intact),
        ];
        let met = shot.hit(Vec2::ZERO, &among(&targets)).expect("a hit");
        assert_eq!(
            (met.ship, met.fuse),
            (ShipRef::Npc(NpcId(2)), Fuse::Contact)
        );
        assert!(close(met.at, Vec2::new(75.0, 0.0)), "{met:?}");
        let alone = shot.hit(Vec2::ZERO, &among(&targets[..1])).expect("a hit");
        assert_eq!(
            (alone.ship, alone.fuse),
            (ShipRef::Npc(NpcId(3)), Fuse::Proximity)
        );
    }

    #[test]
    fn prox_safety_holds_contact_hits_until_the_age_reaches_it_and_proximity_hits_until_it_is_past()
    {
        let aged = |age, prox| {
            let mut shot = passing(prox);
            shot.weapon.prox_safety = 5;
            shot.age = age;
            shot
        };
        let on_path = [target(2, 50.0, 0.0, Condition::Intact)];
        let beside = [target(2, 50.0, 7.0, Condition::Intact)];
        let fuse = |shot: Shot, targets: &[Target]| {
            shot.hit(Vec2::ZERO, &among(targets)).map(|met| met.fuse)
        };
        assert_eq!(fuse(aged(4, 0), &on_path), None, "contact at 4");
        assert_eq!(fuse(aged(5, 0), &on_path), Some(Fuse::Contact));
        assert_eq!(fuse(aged(5, 3), &beside), None, "proximity at 5");
        assert_eq!(fuse(aged(6, 3), &beside), Some(Fuse::Proximity));
        assert_eq!(
            fuse(aged(4, 3), &[on_path[0], beside[0]]),
            None,
            "neither armed"
        );
    }

    #[test]
    fn a_shot_never_hits_its_firer() {
        let shot = passing(0);
        let firer = Target {
            ship: FIRER,
            ..target(1, 50.0, 0.0, Condition::Intact)
        };
        assert_eq!(shot.hit(Vec2::ZERO, &among(&[firer])), None);
        let Meeting { ship, at, .. } = shot
            .hit(
                Vec2::ZERO,
                &among(&[firer, target(3, 30.0, 0.0, Condition::Intact)]),
            )
            .expect("another ship");
        assert_eq!(ship, ShipRef::Npc(NpcId(3)));
        assert!(close(at, Vec2::new(25.0, 0.0)), "met at its edge: {at:?}");
    }

    // Who a shot may hit (`_ShotCanHitShip`).

    const PLAIN: GovtId = GovtId(140);
    const NEVER_ATTACKS: GovtId = GovtId(141);
    const SHOTS_PASS: GovtId = GovtId(142);
    const OTHER: GovtId = GovtId(143);

    /// Governments 140 and 143 with no flags, 141 that never attacks the
    /// player (`Flags` 0x0040), and 142 that the player's shots pass
    /// through (`Flags` 0x0008).
    fn govts() -> Governments {
        Governments::new([
            govt(140),
            GovtRecord {
                flags: NEVER_ATTACKS_PLAYER,
                ..govt(141)
            },
            GovtRecord {
                flags: PLAYER_SHOTS_PASS,
                ..govt(142)
            },
            govt(143),
        ])
    }

    fn npc(id: u32) -> ShipRef {
        ShipRef::Npc(NpcId(id))
    }

    fn led_by(leader: ShipRef) -> Ties {
        Ties {
            leader: Some(leader),
            ..Ties::default()
        }
    }

    fn of(govt: GovtId) -> Ties {
        Ties {
            govt: Some(govt),
            ..Ties::default()
        }
    }

    fn dude(booty: u16) -> Ties {
        Ties {
            booty: Some(booty),
            ..Ties::default()
        }
    }

    /// `ship`, tied as `ties`, `x` pixels along the shot's path.
    fn on_path(ship: ShipRef, x: f32, ties: Ties) -> Target {
        Target {
            ship,
            ties,
            ..target(0, x, 0.0, Condition::Intact)
        }
    }

    /// `ship`, tied as `ties`, well off the shot's path.
    fn aside(ship: ShipRef, ties: Ties) -> Target {
        Target {
            ship,
            ties,
            ..target(0, 0.0, 500.0, Condition::Intact)
        }
    }

    /// The shot from (0, 0) to (100, 0), fired by `firer`, tied as `ties`
    /// as it fired.
    fn fired_by(firer: ShipRef, ties: Ties) -> Shot {
        Shot {
            firer,
            source: Source::Ship(ties),
            ..passing(0)
        }
    }

    /// The first of `targets` `shot` hits, among `govts`, as `pass_through`
    /// says.
    fn first_by(
        shot: &Shot,
        targets: &[Target],
        govts: &Governments,
        pass_through: PassThrough,
    ) -> Option<ShipRef> {
        let sides = Sides {
            targets,
            govts,
            pass_through,
        };
        shot.hit(Vec2::ZERO, &sides).map(|met| met.ship)
    }

    /// The first of `targets` `shot` hits, by the original's reading.
    fn first(shot: &Shot, targets: &[Target]) -> Option<ShipRef> {
        first_by(shot, targets, &govts(), PassThrough::PlayerSide)
    }

    #[test]
    fn a_shot_passes_through_a_ship_of_its_firers_government() {
        let shot = fired_by(FIRER, of(PLAIN));
        let targets = [
            aside(FIRER, of(PLAIN)),
            on_path(npc(2), 30.0, of(PLAIN)),
            on_path(npc(3), 60.0, of(OTHER)),
        ];
        assert_eq!(first(&shot, &targets), Some(npc(3)));
    }

    #[test]
    fn ships_of_no_government_hit_each_other() {
        let shot = fired_by(FIRER, Ties::default());
        let targets = [
            aside(FIRER, Ties::default()),
            on_path(npc(2), 30.0, Ties::default()),
        ];
        assert_eq!(first(&shot, &targets), Some(npc(2)));
    }

    #[test]
    fn a_shot_passes_through_a_ship_under_its_firers_leader_or_root() {
        // The firer and ship 2 share lead 5; ship 3's leader 6 is led by
        // 5 too, two levels down.
        let shot = fired_by(FIRER, led_by(npc(5)));
        let mut targets = vec![
            aside(FIRER, led_by(npc(5))),
            aside(npc(6), led_by(npc(5))),
            on_path(npc(5), 10.0, Ties::default()),
            on_path(npc(2), 20.0, led_by(npc(5))),
            on_path(npc(3), 40.0, led_by(npc(6))),
            on_path(npc(4), 80.0, Ties::default()),
        ];
        assert_eq!(first(&shot, &targets), Some(npc(4)));
        targets.pop();
        assert_eq!(first(&shot, &targets), None);
    }

    #[test]
    fn player_side_fire_never_hits_an_escorts_fighter() {
        // Ship 2 is the fighter of escort 3, which is destroyed: the
        // shared root no longer holds, but the fighter rule still reads
        // its leader's leader.
        let escort = Target {
            condition: Condition::Destroyed,
            ..aside(npc(3), led_by(ShipRef::Player))
        };
        let fighter = on_path(npc(2), 30.0, led_by(npc(3)));
        let behind = on_path(npc(4), 60.0, Ties::default());
        let mut targets = vec![
            aside(ShipRef::Player, Ties::default()),
            aside(npc(7), led_by(ShipRef::Player)),
            escort,
            fighter,
            behind,
        ];
        let from_the_player = fired_by(ShipRef::Player, Ties::default());
        assert_eq!(first(&from_the_player, &targets), Some(npc(4)));
        let from_an_escort = fired_by(npc(7), led_by(ShipRef::Player));
        assert_eq!(first(&from_an_escort, &targets), Some(npc(4)));
        targets.push(aside(npc(9), Ties::default()));
        let from_traffic = fired_by(npc(9), Ties::default());
        assert_eq!(
            first(&from_traffic, &targets),
            Some(npc(2)),
            "not the player's side"
        );
    }

    #[test]
    fn a_player_side_firer_of_a_never_attacks_player_government_hits_nothing() {
        let ties = Ties {
            govt: Some(NEVER_ATTACKS),
            ..led_by(ShipRef::Player)
        };
        let targets = [aside(FIRER, ties), on_path(npc(2), 30.0, Ties::default())];
        assert_eq!(first(&fired_by(FIRER, ties), &targets), None);
        let plain = Ties {
            govt: Some(PLAIN),
            ..ties
        };
        let targets = [aside(FIRER, plain), targets[1]];
        assert_eq!(first(&fired_by(FIRER, plain), &targets), Some(npc(2)));
        let traffic = [aside(FIRER, of(NEVER_ATTACKS)), targets[1]];
        assert_eq!(
            first(&fired_by(FIRER, of(NEVER_ATTACKS)), &traffic),
            Some(npc(2)),
            "not the player's side"
        );
    }

    #[test]
    fn player_side_fire_passes_through_a_government_flagged_0x0008() {
        let targets = [
            aside(ShipRef::Player, Ties::default()),
            aside(npc(4), led_by(ShipRef::Player)),
            aside(npc(5), led_by(npc(4))),
            aside(FIRER, Ties::default()),
            on_path(npc(2), 30.0, of(SHOTS_PASS)),
            on_path(npc(3), 60.0, Ties::default()),
        ];
        for firer in [ShipRef::Player, npc(4), npc(5)] {
            let leader = targets.iter().find(|t| t.ship == firer).map(|t| t.ties);
            let shot = fired_by(firer, leader.unwrap_or_default());
            assert_eq!(first(&shot, &targets), Some(npc(3)), "{firer:?}");
        }
        let traffic = fired_by(FIRER, Ties::default());
        assert_eq!(first(&traffic, &targets), Some(npc(2)));
    }

    #[test]
    fn the_player_and_its_direct_escorts_pass_through_a_never_attacks_player_ship() {
        let targets = [
            aside(ShipRef::Player, Ties::default()),
            aside(npc(4), led_by(ShipRef::Player)),
            aside(npc(5), led_by(npc(4))),
            on_path(npc(2), 30.0, of(NEVER_ATTACKS)),
            on_path(npc(3), 60.0, Ties::default()),
        ];
        let from_the_player = fired_by(ShipRef::Player, Ties::default());
        assert_eq!(first(&from_the_player, &targets), Some(npc(3)));
        let from_an_escort = fired_by(npc(4), led_by(ShipRef::Player));
        assert_eq!(first(&from_an_escort, &targets), Some(npc(3)));
        let from_its_fighter = fired_by(npc(5), led_by(npc(4)));
        assert_eq!(
            first(&from_its_fighter, &targets),
            Some(npc(2)),
            "an escort's fighter is no direct escort"
        );
    }

    #[test]
    fn booty_0x0100_ships_and_player_side_fire_pass_through_each_other() {
        // 6i: player-side fire passes through a ship of such a düde.
        let targets = [
            aside(ShipRef::Player, Ties::default()),
            on_path(npc(2), 30.0, dude(0x0141)),
            on_path(npc(3), 60.0, dude(0x0041)),
        ];
        let from_the_player = fired_by(ShipRef::Player, Ties::default());
        assert_eq!(first(&from_the_player, &targets), Some(npc(3)));
        // 6j: such a ship's fire passes through the player and its
        // escorts.
        let allied = [
            aside(FIRER, dude(0x0100)),
            on_path(ShipRef::Player, 20.0, Ties::default()),
            on_path(npc(4), 40.0, led_by(ShipRef::Player)),
            on_path(npc(3), 60.0, Ties::default()),
        ];
        assert_eq!(first(&fired_by(FIRER, dude(0x0100)), &allied), Some(npc(3)));
        // Without a düde of its own, its leader's is read; with one of
        // Booty 0, it is not.
        let mut led = allied.to_vec();
        led[0] = aside(npc(6), dude(0x0100));
        led.push(aside(FIRER, led_by(npc(6))));
        let no_dude = fired_by(FIRER, led_by(npc(6)));
        assert_eq!(first(&no_dude, &led), Some(npc(3)), "the leader's düde");
        let own = Ties {
            booty: Some(0),
            ..led_by(npc(6))
        };
        let last = led.len() - 1;
        led[last] = aside(FIRER, own);
        let booty_0 = fired_by(FIRER, own);
        assert_eq!(
            first(&booty_0, &led),
            Some(ShipRef::Player),
            "its own düde of Booty 0"
        );
    }

    #[test]
    fn the_bibles_pass_through_counts_the_player_alone() {
        let govts = govts();
        let bible = |shot: &Shot, targets: &[Target]| {
            first_by(shot, targets, &govts, PassThrough::PlayerOnly)
        };
        let escort = led_by(ShipRef::Player);
        let side = [
            aside(ShipRef::Player, Ties::default()),
            aside(npc(4), escort),
            on_path(npc(2), 20.0, of(SHOTS_PASS)),
            on_path(npc(3), 40.0, of(NEVER_ATTACKS)),
            on_path(npc(5), 60.0, dude(0x0100)),
            on_path(npc(6), 80.0, Ties::default()),
        ];
        let from_the_player = fired_by(ShipRef::Player, Ties::default());
        assert_eq!(bible(&from_the_player, &side), Some(npc(6)), "all three");
        let from_an_escort = fired_by(npc(4), escort);
        assert_eq!(
            bible(&from_an_escort, &side),
            Some(npc(2)),
            "not the player"
        );
        let never_attacks = Ties {
            govt: Some(NEVER_ATTACKS),
            ..escort
        };
        let mut odd = side.to_vec();
        odd[1] = aside(npc(4), never_attacks);
        assert_eq!(
            bible(&fired_by(npc(4), never_attacks), &odd),
            Some(npc(2)),
            "no 6g"
        );
        // A Booty 0x0100 ship's own fire passes through the player, not
        // its escorts, and its leader's düde is not read.
        let at = [
            aside(FIRER, dude(0x0100)),
            on_path(npc(4), 20.0, escort),
            on_path(ShipRef::Player, 40.0, Ties::default()),
        ];
        assert_eq!(bible(&fired_by(FIRER, dude(0x0100)), &at), Some(npc(4)));
        assert_eq!(
            bible(&fired_by(FIRER, dude(0x0100)), &[at[0], at[2]]),
            None,
            "passes through the player"
        );
        let led = [
            aside(npc(7), dude(0x0100)),
            aside(FIRER, led_by(npc(7))),
            at[2],
        ];
        assert_eq!(
            bible(&fired_by(FIRER, led_by(npc(7))), &led),
            Some(ShipRef::Player)
        );
    }

    #[test]
    fn a_non_ship_shot_hits_only_its_target() {
        let shot = Shot {
            source: Source::Other,
            aimed_at: Some(npc(3)),
            ..passing(0)
        };
        let targets = [
            on_path(npc(2), 30.0, Ties::default()),
            on_path(npc(3), 60.0, Ties::default()),
        ];
        assert_eq!(first(&shot, &targets), Some(npc(3)));
    }

    #[test]
    fn a_non_ship_shot_with_no_target_may_hit_any_ship() {
        let shot = Shot {
            source: Source::Other,
            aimed_at: None,
            ..passing(0)
        };
        let targets = [
            on_path(npc(2), 30.0, of(NEVER_ATTACKS)),
            on_path(npc(3), 60.0, Ties::default()),
        ];
        assert_eq!(first(&shot, &targets), Some(npc(2)));
        assert_eq!(shot.govt(), None);
    }

    #[test]
    fn a_shot_passes_through_the_escape_pod_and_every_class_after_it() {
        let class = |ship, x, class| Target {
            ship_type: ShipId(class),
            ..on_path(ship, x, Ties::default())
        };
        let targets = [
            class(npc(2), 20.0, 895),
            class(npc(3), 40.0, 896),
            class(npc(4), 60.0, 894),
        ];
        assert_eq!(first(&passing(0), &targets), Some(npc(4)));
    }

    #[test]
    fn a_shot_hits_a_breaking_up_ship_and_not_a_destroyed_one() {
        let shot = passing(0);
        let hit = |condition| {
            shot.hit(Vec2::ZERO, &among(&[target(2, 50.0, 0.0, condition)]))
                .map(|met| met.ship)
        };
        assert_eq!(hit(Condition::Disabled), Some(ShipRef::Npc(NpcId(2))));
        assert_eq!(
            hit(Condition::Dying {
                ticks_left: 3,
                starting: false
            }),
            Some(ShipRef::Npc(NpcId(2)))
        );
        assert_eq!(hit(Condition::Destroyed), None);
    }

    #[test]
    fn the_firers_side_is_read_as_it_is_now_and_kept_once_it_is_gone() {
        // It fired as a ship of 140, and is of 143 now.
        let shot = fired_by(FIRER, of(PLAIN));
        let path = [
            on_path(npc(2), 30.0, of(OTHER)),
            on_path(npc(3), 60.0, of(PLAIN)),
        ];
        let now = [aside(FIRER, of(OTHER)), path[0], path[1]];
        assert_eq!(first(&shot, &now), Some(npc(3)), "as it is now");
        assert_eq!(first(&shot, &path), Some(npc(2)), "as it fired");
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
            shot.hit(Vec2::ZERO, &among(&in_its_path()))
                .map(|met| met.ship),
            Some(TARGET)
        );
        let at_three = passing_homing(0, Some(ShipRef::Npc(NpcId(3))));
        assert_eq!(
            at_three
                .hit(Vec2::ZERO, &among(&in_its_path()))
                .map(|met| met.ship),
            Some(ShipRef::Npc(NpcId(3)))
        );
    }

    #[test]
    fn a_homing_shot_set_off_by_other_ships_hits_the_first_it_meets() {
        let shot = passing_homing(0x0008, Some(TARGET));
        assert_eq!(
            shot.hit(Vec2::ZERO, &among(&in_its_path()))
                .map(|met| met.ship),
            Some(ShipRef::Npc(NpcId(3)))
        );
        let untargeted = passing_homing(0x0008, None);
        assert_eq!(
            untargeted
                .hit(Vec2::ZERO, &among(&in_its_path()))
                .map(|met| met.ship),
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
        assert_eq!(shot.jam, Jam::Clear);
        assert_eq!(
            passing_homing(0, None).hit(Vec2::ZERO, &among(&in_its_path())),
            None
        );
    }

    #[test]
    fn a_homing_shot_whose_target_is_gone_is_lost_and_one_breaking_up_is_chased() {
        for gone in [vec![], vec![target(2, 0.0, 1000.0, Condition::Destroyed)]] {
            let mut shot = homing(missile(0), Some(TARGET));
            shot.aimed_at = Some(TARGET);
            for _ in 0..20 {
                shot.step(&gone);
            }
            assert_eq!(shot.jam, Jam::Lost, "{gone:?}");
            assert_eq!((shot.target, shot.aimed_at), (None, None), "{gone:?}");
            assert_eq!(shot.heading, 90.0, "flies straight on: {gone:?}");
            let back = [target(2, 0.0, 1000.0, Condition::Intact)];
            shot.step(&back);
            assert_eq!(shot.heading, 90.0, "lost for good: {gone:?}");
        }
        let lost = Shot {
            jam: Jam::Lost,
            ..passing_homing(0x0008, Some(TARGET))
        };
        assert_eq!(
            lost.hit(Vec2::ZERO, &among(&in_its_path())),
            None,
            "hits nothing"
        );
        for condition in [
            Condition::Disabled,
            Condition::Dying {
                ticks_left: 9,
                starting: false,
            },
        ] {
            let quarry = [target(2, 0.0, 1000.0, condition)];
            let mut chasing = homing(missile(0), Some(TARGET));
            chasing.aimed_at = Some(TARGET);
            for _ in 0..16 {
                chasing.step(&quarry);
            }
            assert_eq!(chasing.jam, Jam::Clear, "{condition:?}");
            assert!(chasing.heading > 90.0, "chased: {condition:?}");
        }
    }

    /// A shot of `guidance` at (0, 0) moving 10 pixels a tick right,
    /// fired by its firer at ship 2, set off by any ship.
    fn aimed_at_two(guidance: i16) -> Shot {
        let mut weapon = WeaponSpec::new(&WeaponRecord {
            guidance,
            speed: 1000,
            count: 100,
            flags2: 0x0008,
            ..weapon(128)
        });
        weapon.speed = 10.0;
        Shot {
            velocity: Vec2::new(10.0, 0.0),
            target: Some(TARGET),
            aimed_at: Some(TARGET),
            ..Shot::launch(weapon, FIRER, FIRER, &state(0.0, 0.0, 0.0, 0.0, 90.0), 90.0)
        }
    }

    #[test]
    fn a_shot_of_every_guidance_but_point_defence_whose_ship_has_left_the_fight_is_lost_and_passes_through_the_ships_behind()
     {
        // Ship 3 sits in the path; ship 2, aimed at, is far off or gone.
        let behind = target(3, 5.0, 0.0, Condition::Intact);
        let aimed = target(2, 0.0, 900.0, Condition::Intact);
        for guidance in [-1, 4, 5, 6, 1] {
            let mut kept = aimed_at_two(guidance);
            let was = kept.position;
            kept.step(&[behind, aimed]);
            assert_eq!(kept.jam, Jam::Clear, "{guidance}");
            assert_eq!(kept.aimed_at, Some(TARGET), "{guidance}");
            assert_eq!(
                kept.hit(was, &among(&[behind, aimed])).map(|met| met.ship),
                Some(ShipRef::Npc(NpcId(3))),
                "{guidance}: aimed at a ship still there"
            );
            let mut lost = aimed_at_two(guidance);
            let was = lost.position;
            lost.step(&[behind]);
            assert_eq!(lost.jam, Jam::Lost, "{guidance}");
            assert_eq!((lost.aimed_at, lost.target), (None, None), "{guidance}");
            assert_eq!(lost.hit(was, &among(&[behind])), None, "{guidance}");
            for _ in 0..200 {
                let was = lost.position;
                lost.step(&[behind, aimed]);
                assert_eq!(lost.hit(was, &among(&[behind, aimed])), None);
            }
            assert_eq!(lost.jam, Jam::Lost, "{guidance}: for good");
        }
    }

    #[test]
    fn a_shot_aimed_at_nothing_is_never_lost() {
        let mut ahead = Shot {
            target: None,
            aimed_at: None,
            ..aimed_at_two(7)
        };
        ahead.step(&[]);
        assert_eq!(ahead.jam, Jam::Clear);
    }

    #[test]
    fn a_point_defence_shot_is_never_lost_and_is_aimed_at_no_ship() {
        let mut defence = aimed_at_two(9);
        defence.step(&[]);
        assert_eq!(defence.jam, Jam::Clear);
        assert_eq!(defence.aimed_at, None);
        assert_eq!(defence.target, Some(TARGET), "its quarry kept");
    }

    #[test]
    fn a_shot_whose_ship_is_breaking_up_stays_clear_and_one_destroyed_is_lost() {
        let dying = target(
            2,
            0.0,
            900.0,
            Condition::Dying {
                ticks_left: 5,
                starting: false,
            },
        );
        let mut shot = aimed_at_two(-1);
        shot.step(&[dying]);
        assert_eq!(shot.jam, Jam::Clear);
        let destroyed = Target {
            condition: Condition::Destroyed,
            ..dying
        };
        shot.step(&[destroyed]);
        assert_eq!(shot.jam, Jam::Lost);
    }

    #[test]
    fn a_shot_starts_unaimed_whole_clear_and_of_the_first_generation() {
        let shot = homing(missile(0), None);
        assert_eq!(
            (shot.id, shot.jam, shot.durability, shot.generation),
            (ShotId(0), Jam::Clear, 4.0, 0)
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
        assert_eq!(turret.jam, Jam::Clear);
        let passing_turret = Shot {
            position: Vec2::new(100.0, 0.0),
            ..turret
        };
        assert_eq!(
            passing_turret
                .hit(Vec2::ZERO, &among(&in_its_path()))
                .map(|met| met.ship),
            Some(ShipRef::Npc(NpcId(3))),
            "any ship it meets, its target or not"
        );
    }

    #[test]
    fn a_shot_takes_a_decay_step_every_decay_plus_one_ticks_and_by_the_bible_every_decay_ticks() {
        let decaying = WeaponSpec {
            decay: 2,
            ..spec(-1, 100, 20)
        };
        let mut shot = Shot::launch(decaying, FIRER, FIRER, &state(0.0, 0.0, 0.0, 0.0, 0.0), 0.0);
        let mut original = Vec::new();
        let mut bible = Vec::new();
        for _ in 1..=9 {
            shot.step(&[]);
            original.push(shot.decay_steps(DecayStep::EveryDecayPlusOne));
            bible.push(shot.decay_steps(DecayStep::EveryDecay));
        }
        assert_eq!(original, [0, 0, 1, 1, 1, 2, 2, 2, 3]);
        assert_eq!(bible, [0, 1, 1, 2, 2, 3, 3, 4, 4]);
        let steady = Shot {
            age: 9,
            ..Shot::launch(spec(-1, 100, 20), FIRER, FIRER, &ShipState::default(), 0.0)
        };
        for step in [DecayStep::EveryDecayPlusOne, DecayStep::EveryDecay] {
            assert_eq!(steady.decay_steps(step), 0, "no decay: {step:?}");
        }
        assert_eq!(DecayStep::default(), DecayStep::EveryDecayPlusOne);
        assert_eq!(DecayStep::from(RuleSource::Bible), DecayStep::EveryDecay);
        assert_eq!(
            DecayStep::from(RuleSource::Engine),
            DecayStep::EveryDecayPlusOne
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
