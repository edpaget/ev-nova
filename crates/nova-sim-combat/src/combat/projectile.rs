//! Shots: launched from a ship, flown a tick at a time, and checked
//! against the ships they pass.
//!
//! The values are the original's (`_SpawnShot`, `_HandleShotGuidance`,
//! `_HandleShot` and `_ShotCanHitShip` in the `EV Nova` executable):
//!
//! - A shot starts at its firer's position with its firer's velocity,
//!   heading as it was aimed, and adds its speed along its heading. A
//!   homing shot never carries its firer's velocity. Every push along a
//!   heading, at launch and in flight, is `_Accel`'s along the whole degree
//!   `trunc(heading)` ([`trig::accel`]; `_SpawnShot` @0x3eeea,
//!   `_HandleShotGuidance` @0x32502, @0x326f1, @0x3296b). A bomb or a rocket
//!   goes as [`BombLaunch`] says: an NPC's as any other shot by the
//!   engine, the player's, and every ship's by the Bible, with no speed
//!   added, a freefall bomb with [`BOMB_LAUNCH`] of the velocity.
//! - Each tick an unguided shot keeps its velocity. A rocket's velocity
//!   becomes [`ROCKET_KEEP`] of itself and [`ROCKET_PUSH`] of its top
//!   speed along its heading, so it closes on its top speed
//!   ([`rocket_push`]). A bomb keeps
//!   its velocity and turns its heading a whole degree ([`BOMB_TURN`]) a
//!   tick towards the table's bearing of it while its truncated heading
//!   is off it, towards up at rest (see `Shot::weathervane`).
//! - A homing shot flies at its speed along its heading every tick. Once
//!   it has flown [`HOMING_DELAY`] ticks it steers at its target, turning
//!   `GuidedTurn` a tick while the table's whole-degree bearing of it is
//!   further off its truncated heading than the truncated turn, the way
//!   [`homing_turn`] says, and keeping its heading otherwise, a target
//!   breaking up included; the heading is then taken round once
//!   ([`wrap_turned`]). One fired with no target flies straight.
//!   Before it turns, `_HandleShotGuidance` (@0x320dc-0x32aa4) tests it,
//!   in this order, every tick:
//!   - **Jamming** ([`jamming`](super::jamming)): it rolled each jamming
//!     type as it was launched (`_SpawnShot` @0x3f051-0x3f0a7); its target
//!     jams it when, for a type rolled above none, the target's jamming is
//!     above 100 less the roll. Jammed, it stops turning, or under `Seeker`
//!     0x0010 turns away, and under 0x8000 turns on its firer at one in
//!     [`JAMMED_TURNS_ON_FIRER`]. It is not [`Jam`]-jammed: it still hits
//!     its target, counts as aimed at it and locks.
//!   - **Sight**: when its firer cannot see the target
//!     ([`cloak::seen`]: cloaked with no scanner within 200 pixels and no
//!     escort's cloak, or coming out of a gate), it stops turning, and
//!     under 0x8000 turns on its firer at one in
//!     [`BLINDED_TURNS_ON_FIRER`].
//!   - **`Seeker` 0x4000**: within [`LOCK_BOX`] of its target on each
//!     axis, it loses its lock when the table's bearing of the target
//!     less its heading, truncated, is more than [`LOCK_CONE`] off ahead,
//!     as [`LockAngle`] measures it.
//!
//!   A shot turned on its firer is aimed at it and launched by no ship
//!   ([`Source::Other`]). On the tick of a retarget or a lost lock it
//!   still turns at the bearing it read before the tests.
//! - A homing shot of `Seeker` 0x0002 that began its tick not jammed then
//!   looks for an asteroid at one in 10, from its first tick: one near and
//!   ahead locks it ([`Jam::Asteroid`]), and from the next tick it steers
//!   at that asteroid's slot, hitting no ship and aimed at none (see
//!   [`asteroid_seek`](super::asteroid_seek)).
//! - A homing shot of `Seeker` 0x0008 is confused by sensor interference
//!   ([`Jam::Interfered`]) when, as it is launched, `below(100) + 1` is at
//!   most the player's system's `Interference` (`_SpawnShot`
//!   @0x3eab2-0x3eb66). Confused, it flies straight for its
//!   [`HOMING_DELAY`] ticks, then turns `GuidedTurn` a tick, back for 150
//!   frames of the frame counter and on for 150 ([`CIRCLE_FRAMES`]),
//!   whatever its target; it hits nothing, and under 0x8000 clears and
//!   turns on its firer at one in [`BLINDED_TURNS_ON_FIRER`] each tick.
//!   Neither function plays a sound.
//! - Any shot but point defence whose ship ([`Shot::aimed_at`]) is gone
//!   (destroyed, or out of the fight as it landed or jumped) is lost
//!   ([`Jam::Lost`]) before it next moves: it flies on, aimed at nothing,
//!   and hits no ship for the rest of its life (`_HandleShot`
//!   @0x355e0-0x35621). A homing one then flies straight.
//! - A shot lives its weapon's `Count` ticks, then detonates if its weapon
//!   says so (`Flags` 0x8000) and otherwise vanishes, a lost one too.
//! - It hits the first ship ([`Target`]) it touches, within the ship's hit
//!   radius, swept from where it was to where it is so that a fast shot
//!   cannot pass through a ship between ticks ([`Shot::touched`]), or
//!   else, with a proximity fuse, the first ship in slot order within its
//!   reach at the end of the move, `ProxRadius` and a third of the ship's
//!   sprite width in whole pixels ([`Shot::fused`]), as
//!   [`RuleKey::ProximityFuse`](crate::RuleKey::ProximityFuse) says: its
//!   other reading sweeps the fuse along the move as the touch is.
//!   `ProxSafety` holds a touch until its age reaches it and the fuse
//!   until its age is past it ([`Shot::hit`]). The fight tests a move in
//!   the next tick's hit pass (see [`Combat`](super::Combat)), and never
//!   its last. A point-defence shot (9) touches none, and meets a ship
//!   only by its proximity fuse. Which ships it can hit is
//!   [`sides`](super::sides)' rule, the
//!   original's `_ShotCanHitShip`: never its firer, the ships of its
//!   firer's government or under its firer's leaders, the escape pod or a
//!   destroyed ship, among others, but a disabled or breaking-up one; a
//!   homing shot only its target, unless its weapon's proximity fuse is
//!   set off by other ships (`Flags2` 0x0008), and nothing once lost.

use super::ShipRef;
use super::damage::Blast;
use super::hull::Condition;
use super::jamming::{JAMMING_TYPES, Jamming};
pub use super::shot_layer::ShotLayer;
use super::sides::{Sides, Ties};
use super::weapon::{
    Guidance, LOSES_LOCK_OFF_AHEAD, SENSOR_INTERFERENCE, TURNS_AWAY_JAMMED, TURNS_ON_FIRER,
    WeaponSpec,
};
use crate::catalog::{GovtId, ShipId};
use crate::chance::Chance;
use crate::cloak::{self, Observer};
use crate::flight::{ShipState, normalized};
use crate::geometry::Vec2;
use crate::rulebook::RuleSource;
use crate::ship_layer::Slot;
use crate::trig;

/// A freefall bomb's share of its firer's velocity at launch.
pub const BOMB_LAUNCH: f32 = 0.8;
/// The share of its velocity a rocket keeps each tick.
pub const ROCKET_KEEP: f32 = 0.95;
/// The share of its top speed a rocket gains each tick.
pub const ROCKET_PUSH: f32 = 0.05;
/// [`ROCKET_KEEP`] as the original multiplies it, in hundredths (0xdd678).
const ROCKET_KEEP_PER_CENT: f32 = 95.0;
/// [`ROCKET_PUSH`] as the original multiplies it, in hundredths (0xdd67c).
const ROCKET_PUSH_PER_CENT: f32 = 5.0;
/// How far a bomb turns into the wind each tick, in degrees.
pub const BOMB_TURN: f32 = 1.0;
/// How many ticks a homing shot flies straight before it steers.
pub const HOMING_DELAY: u32 = 15;
/// The odds, one in this, that a jammed `Seeker` 0x8000 shot turns on its
/// firer (`_Rand(500)` @0x32250).
pub const JAMMED_TURNS_ON_FIRER: u32 = 500;
/// The odds, one in this, that a `Seeker` 0x8000 shot whose firer cannot
/// see its target, or that sensor interference confuses, turns on its
/// firer (`_Rand(1000)` @0x322fa, @0x32735).
pub const BLINDED_TURNS_ON_FIRER: u32 = 1000;
/// How near, on each axis, a `Seeker` 0x4000 shot's target must be for
/// the shot to lose its lock (±249, @0x3237d-0x3239d).
pub const LOCK_BOX: f32 = 249.0;
/// How far off its heading, in degrees, a `Seeker` 0x4000 shot keeps its
/// lock (@0x323e7).
pub const LOCK_CONE: f32 = 45.0;
/// The frames of an interfered shot's circle, back and on (300,
/// @0x3265b).
pub const CIRCLE_FRAMES: i32 = 300;
/// The last frame of the circle an interfered shot turns back on (149,
/// @0x32663).
pub const CIRCLE_BACK: i32 = 149;
/// The range sensor interference rolls in at launch: interfered when
/// `below(INTERFERENCE_ROLL) + 1` is at most the system's `Interference`
/// (`_SpawnShot` @0x3ead4, 100 at `_gSpeedMult` 1.0).
pub const INTERFERENCE_ROLL: u32 = 100;
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
    /// Its sprite's width, in pixels, as `_GetMasterShipSpriteXSize`
    /// reads it, which a beam's cone sizes it by
    /// ([`cone`](super::beam::cone)): its `shän`'s `BaseXSize`, which every
    /// stock sheet's frame width equals, or
    /// [`NO_SPRITE_WIDTH`](super::beam::NO_SPRITE_WIDTH) with none.
    pub sprite_x: f32,
    /// Whether it has a sprite sheet: a proximity fuse sizes a ship of
    /// none [`NO_SHEET_WIDTH`] wide, as `_GetSpriteXSize` reads it
    /// ([`Target::fuse_width`]).
    pub sheet: bool,
    /// How it is holding up.
    pub condition: Condition,
    /// Whether `_IsDisabled` holds for it now (@0x2ce6-0x2e0d): held
    /// disabled, not intact, or its armour under the line its hull's
    /// `TOUGH` bit picks. The collision walk refreshes it after every hit
    /// it lands, so the miss rule reads the armour the pass's earlier hits
    /// left, while [`Target::condition`] settles after the pass.
    pub disabled: bool,
    /// Its hull's turret miss class
    /// ([`HullSpec::miss_class`](super::hull::HullSpec::miss_class)): a
    /// shot whose roll is above it misses it while it is intact.
    pub miss_class: u32,
    /// Its ship slot ([`Fighter::slot`](super::Fighter::slot)): a seeking
    /// sub-munition that finds two ships equally near takes the one in the
    /// lower slot, as `_FindNearestValidShotTarget` walks the ship table
    /// (@0x6abd-0x6ace; see [`submunition`](super::submunition)).
    pub slot: crate::ship_layer::Slot,
}

impl Target {
    /// Its sprite's width as a proximity fuse reads it, `_GetSpriteXSize`
    /// of its sprite (@0x26d4-0x2702): its current frame's width, which
    /// is its [`Target::sprite_x`] for every stock sheet, or
    /// [`NO_SHEET_WIDTH`] with no sheet.
    #[must_use]
    pub fn fuse_width(&self) -> f32 {
        if self.sheet {
            self.sprite_x
        } else {
            NO_SHEET_WIDTH
        }
    }
}

/// The width `_GetSpriteXSize` gives a sprite of no frames (@0x26fc).
pub const NO_SHEET_WIDTH: f32 = 32.0;

/// The share of a ship's sprite width a proximity fuse reaches past its
/// `ProxRadius` (the double 0.333 @0xdd0c8).
pub const FUSE_SPRITE_SHARE: f64 = 0.333;

/// How a proximity fuse is set off by a ship, as the rulebook's
/// [`RuleKey::ProximityFuse`](crate::RuleKey::ProximityFuse) says.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ProximityFuse {
    /// The original's, the engine's reading and the default: a point test
    /// at the shot's end point, ship by ship in slot order
    /// (`_CheckShotProximities` @0x37549-0x376a5; see [`Shot::fused`]).
    #[default]
    Point,
    /// Swept along the move against each ship's hit radius plus
    /// `ProxRadius`, the ship met first along it taking the hit, where
    /// the move entered its reach: the engine's fuse before
    /// weapons-fidelity phase 16, the other reading.
    Swept,
}

impl From<RuleSource> for ProximityFuse {
    fn from(source: RuleSource) -> Self {
        match source {
            RuleSource::Engine => Self::Point,
            RuleSource::Bible => Self::Swept,
        }
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
    /// Confused by sensor interference (999, `_SpawnShot` @0x3eb43): it
    /// circles (`_HandleShotGuidance` @0x32613-0x32769).
    Interfered,
    /// Locked onto an asteroid (1, `_HandleShotGuidance` @0x32919): the
    /// slot it steers at (+0x2a), none once it found the slot empty
    /// (@0x327b5). See [`asteroid_seek`](super::asteroid_seek).
    Asteroid(Option<usize>),
}

/// What homing guidance reads of a ship besides its [`Target`], as the
/// tick began: how strongly it jams (`_ShipECM`) and what
/// `_ShipVisibleToShip` reads of it, as the target or as the firer looking
/// for it. A ship with none is [`Sight::of`] it.
// Each flag is its own fact about the ship, read on its own.
#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sight {
    /// Which ship.
    pub ship: ShipRef,
    /// Its jamming.
    pub jamming: Jamming,
    /// Whether `_IsDisabled` holds of it: disabled, breaking up or held
    /// disabled. It then jams nothing.
    pub disabled: bool,
    /// Whether it is cloaked.
    pub cloaked: bool,
    /// Whether it waits in the gate it comes out of, unseen.
    pub emerging: bool,
    /// Whether it has a cloak scanner on screen.
    pub scanner: bool,
    /// Whether it has a cloak (`_HasCloak`), which lets it see a cloaked
    /// leader.
    pub has_cloak: bool,
}

impl Sight {
    /// `ship` jamming nothing, intact, uncloaked, in the open, with no
    /// scanner and no cloak.
    #[must_use]
    pub fn of(ship: ShipRef) -> Self {
        Self {
            ship,
            jamming: Jamming::NONE,
            disabled: false,
            cloaked: false,
            emerging: false,
            scanner: false,
            has_cloak: false,
        }
    }
}

/// How a `Seeker` 0x4000 shot measures how far its target is off its
/// heading, and a `Seeker` 0x0002 shot how far an asteroid is
/// ([`asteroid_seek`](super::asteroid_seek)), as the rulebook's
/// [`RuleKey::SeekerLockAngle`](crate::RuleKey::SeekerLockAngle) says.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LockAngle {
    /// The short way round: the default, a recorded departure from the
    /// original.
    #[default]
    Shortest,
    /// The truncated difference, absolute and mod 360 with no fold
    /// (@0x323bb-0x323e7, and @0x325c8-0x325f3 for an asteroid): the
    /// engine's reading.
    Plain,
}

impl From<RuleSource> for LockAngle {
    fn from(source: RuleSource) -> Self {
        match source {
            RuleSource::Engine => Self::Plain,
            RuleSource::Bible => Self::Shortest,
        }
    }
}

impl LockAngle {
    /// How far, in degrees, a truncated `angle` from a heading to a
    /// bearing is off ahead.
    #[must_use]
    pub fn off_ahead(self, angle: f32) -> f32 {
        let plain = angle.abs() % 360.0;
        match self {
            Self::Plain => plain,
            Self::Shortest => plain.min(360.0 - plain),
        }
    }
}

/// What a shot's guidance reads in a tick: every ship in the fight, what
/// it reads of each beyond that ([`Sight`]), the asteroids, the frame
/// counter (`_frameCounter` as `_DoPlayGameWork` sees it), and how a lock
/// is measured.
#[derive(Clone, Copy, Debug)]
pub struct Guide<'a> {
    /// Every ship in the fight.
    pub targets: &'a [Target],
    /// Where each asteroid slot's asteroid is, by slot, none for a free
    /// slot: what a `Seeker` 0x0002 shot locks onto and a locked one
    /// steers at ([`asteroid_seek`](super::asteroid_seek)).
    pub asteroids: &'a [Option<Vec2>],
    /// What guidance reads of each beyond its [`Target`].
    pub sights: &'a [Sight],
    /// The frame counter.
    pub frame: i32,
    /// How a `Seeker` 0x4000 lock is measured.
    pub lock_angle: LockAngle,
    /// Whether a bomb's heading is wrapped after it turns into the wind
    /// ([`RuleKey::BombHeadingWrap`](crate::RuleKey::BombHeadingWrap)): by
    /// the engine, never.
    pub bomb_heading_wrap: RuleSource,
}

impl<'a> Guide<'a> {
    /// `targets`, with no sights and no asteroids, at frame 0, by the
    /// default lock angle and the engine's unwrapped bomb heading.
    #[must_use]
    pub fn among(targets: &'a [Target]) -> Self {
        Self {
            targets,
            asteroids: &[],
            sights: &[],
            frame: 0,
            lock_angle: LockAngle::default(),
            bomb_heading_wrap: RuleSource::Engine,
        }
    }

    /// What it reads of `ship`: its sight, or [`Sight::of`] it with none.
    fn sight(&self, ship: ShipRef) -> Sight {
        self.sights
            .iter()
            .find(|sight| sight.ship == ship)
            .copied()
            .unwrap_or_else(|| Sight::of(ship))
    }
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

/// Which of `circles`, each an item with its centre and reach, the segment
/// from `from` to `to` meets first, and where: the one of least
/// [`contact`], the earlier of a tie; `from` itself for one it starts
/// within.
pub(super) fn nearest_contact<T>(
    from: Vec2,
    to: Vec2,
    circles: impl IntoIterator<Item = (T, Vec2, f32)>,
) -> Option<(T, Vec2)> {
    circles
        .into_iter()
        .filter_map(|(item, centre, reach)| Some((contact(from, to, centre, reach)?, item)))
        .min_by(|(a, _), (b, _)| a.total_cmp(b))
        .map(|(t, item)| (item, from + (to - from) * t))
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
    /// Its turret miss roll (the original's shot +0x42), 1 to its firer's
    /// [`miss_class`](super::hull::HullSpec::miss_class): it misses an
    /// intact ship of a lower class (see [`sides`](super::sides)). Only a
    /// ship's turret (4) or point-defence (9) shot rolls (`_SpawnShot`
    /// @0x3e89e-0x3e963); any other shot carries none, the original's -1,
    /// and never misses. A sub-munition, which writes none, carries the
    /// roll its slot's last ship-fired shot left: see
    /// [`submunition`](super::submunition).
    pub miss_roll: Option<u32>,
    /// The sprite layer it is in, as `_SpawnShot` puts it
    /// ([`ShotLayer::spawned`]), or [`ShotLayer::Shots`] for point defence
    /// fired at a ship.
    pub layer: ShotLayer,
    /// Whether its firer's class has `shïp` `Flags3` 0x0040 as it fired
    /// ([`HullSpec::turret_shots_over`](super::hull::HullSpec::turret_shots_over)),
    /// which a sub-munition's layer falls back on once the firer has left
    /// the fight (see [`submunition`](super::submunition)).
    pub turret_over: bool,
    /// Its rolls against each jamming type (the original's +0x38-+0x3e),
    /// drawn as it is launched ([`jamming::roll`](super::jamming::roll)).
    pub jam_rolls: [u16; JAMMING_TYPES],
    /// Where its last move began, which the next tick's hit pass tests
    /// that move from (see [`Combat`](super::Combat)): none before its
    /// first move, as the original's new shot has no sprite shown for
    /// the collision passes to test (`_SpawnShot` never shows it; the
    /// walks skip a hidden sprite @0xacba3, @0xacbc6).
    pub moved_from: Option<Vec2>,
    /// The original's shot slot it holds
    /// ([`ShotSlots`](super::shot_slots::ShotSlots)), which
    /// [`Combat`](super::Combat) gives it as it puts it in flight: the
    /// shots in flight are kept in slot order.
    pub slot: usize,
    /// The spin timer it started on (the original's +0x1c as it took its
    /// slot, which `_SpawnShot` never writes), in passes: its slot's
    /// leftover ([`ShotSlots::spin`](super::shot_slots::ShotSlots::spin)),
    /// which [`Combat`](super::Combat) gives it as it puts it in flight.
    /// The view counts its frame steps from it
    /// ([`spin_timer::steps`](super::spin_timer::steps)).
    pub spin_timer: u32,
    /// The ship slot its firer fired from, which a sub-munition takes from
    /// its parent (the original's source slot, shot +0x2c): none for a
    /// shot no ship fired. As it spawns, a sub-munition reads whatever
    /// ship holds that slot then (see [`submunition`](super::submunition)).
    pub firer_slot: Option<Slot>,
}

/// How a freefall bomb (5) or a rocket (6) leaves its ship. `_SpawnShot`
/// gives the player's bomb 0.8 of the ship's velocity (@0x3e73a-0x3e783)
/// and adds `Speed` along the heading only for a weapon of another
/// guidance or a ship other than the player (`-0x41`, source above 0,
/// @0x3ede8-0x3ee0a), so an NPC's bomb or rocket, an escort's too, leaves
/// faster than the Bible says.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BombLaunch {
    /// The player's, and every ship's by the Bible ("launched at 80% of
    /// the ship's current velocity"; "launched straight ahead, accelerates
    /// to its maximum velocity"): a bomb [`BOMB_LAUNCH`] of the ship's
    /// velocity, a rocket the ship's, neither sent broadside.
    Dropped,
    /// An NPC's in the original: the ship's velocity and the weapon's
    /// `Speed` along the heading, broadside or not, as any other guidance.
    Thrown,
}

impl BombLaunch {
    /// How an `npc`'s, or else the player's, bombs and rockets leave, as
    /// the rulebook's
    /// [`RuleKey::BombRocketLaunch`](crate::RuleKey::BombRocketLaunch) reads
    /// `source`.
    #[must_use]
    pub fn of(npc: bool, source: RuleSource) -> Self {
        if npc && source == RuleSource::Engine {
            Self::Thrown
        } else {
            Self::Dropped
        }
    }
}

/// The velocity a shot of `weapon` leaves a ship flying `from` with, sent
/// along `heading`, a bomb or a rocket as `launch` says ([`BombLaunch`]):
/// a homing missile its `Speed` along `heading`, and any other, a thrown
/// bomb or rocket too, its `Speed` along `heading` on top of the ship's
/// velocity. A broadside shot is sent to the side this way, facing ahead
/// (see [`exits`](super::exits)). The push is `_Accel`'s along the whole
/// degree `trunc(heading)` ([`trig::accel`], @0x3eeea-0x3eef3).
#[must_use]
pub fn launch_velocity(
    weapon: &WeaponSpec,
    from: &ShipState,
    heading: f32,
    launch: BombLaunch,
) -> Vec2 {
    match (weapon.guidance, launch) {
        (Guidance::FreefallBomb, BombLaunch::Dropped) => from.velocity * BOMB_LAUNCH,
        (Guidance::Rocket, BombLaunch::Dropped) => from.velocity,
        (Guidance::Homing, _) => trig::accel(trig::whole(heading), weapon.speed),
        _ => from.velocity + trig::accel(trig::whole(heading), weapon.speed),
    }
}

/// A rocket's velocity after a tick's push (`_HandleShotGuidance`
/// @0x32940-0x329dc): `_Accel(trunc(heading), speed)` from rest, then
/// `(velocity × 95 + push × 5)` in `f32` (0xdd678, 0xdd67c), times 0.01 in
/// doubles (0xdd098), rounded to `f32`: [`ROCKET_KEEP`] of the velocity
/// and [`ROCKET_PUSH`] of the push.
#[must_use]
#[allow(clippy::cast_possible_truncation)]
pub fn rocket_push(velocity: Vec2, heading: f32, speed: f32) -> Vec2 {
    let push = trig::accel(trig::whole(heading), speed);
    let mix = |v: f32, p: f32| {
        (f64::from(v * ROCKET_KEEP_PER_CENT + p * ROCKET_PUSH_PER_CENT) * 0.01) as f32
    };
    Vec2::new(mix(velocity.x, push.x), mix(velocity.y, push.y))
}

/// A homing shot's heading after its turn of `turn` at the whole-degree
/// `wanted` (`_HandleShotGuidance` @0x323f3-0x3248d): it turns only while
/// `|_AngularDifference(wanted, trunc(heading))|` is above `|trunc(turn)|`,
/// back when `wanted − trunc(heading)`, taken into 0..359 in 16 bits, is
/// above 180, and on otherwise, half round included. A negative turn, a
/// jammed 0x0010 shot's, turns the other way. The original multiplies the
/// turn by `_gSpeedMult`, 1.0 here, in doubles, which rounds as the `f32`
/// sum does.
#[must_use]
pub fn homing_turn(heading: f32, wanted: i16, turn: f32) -> f32 {
    let facing = trig::whole(heading);
    let off = i32::from(trig::angular_difference(wanted, facing)).abs();
    if off <= i32::from(trig::whole(turn)).abs() {
        return heading;
    }
    let mut ahead = i32::from(wanted) - i32::from(facing);
    if ahead >= 360 {
        ahead -= 360;
    }
    if ahead <= -1 {
        ahead += 360;
    }
    if ahead > 180 {
        heading - turn
    } else {
        heading + turn
    }
}

/// A turned heading taken round once each way, as `_HandleShotGuidance`
/// wraps a homing or interfered shot's every tick (`≥ 360` → −360, then
/// `< 0` → +360; @0x32492-0x324d4, @0x32685-0x326c6).
#[must_use]
pub fn wrap_turned(heading: f32) -> f32 {
    let heading = if heading >= 360.0 {
        heading - 360.0
    } else {
        heading
    };
    if heading < 0.0 {
        heading + 360.0
    } else {
        heading
    }
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
        let velocity = launch_velocity(&weapon, from, heading, BombLaunch::Dropped);
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
            miss_roll: None,
            layer: ShotLayer::spawned(weapon.guidance, true, false),
            turret_over: false,
            jam_rolls: [0; JAMMING_TYPES],
            moved_from: None,
            slot: 0,
            spin_timer: 0,
            firer_slot: None,
        }
    }

    /// Flies the shot one tick, a homing one steering at its target among
    /// `guide`'s ships, its draws made on `chance`, and gives where it
    /// was, which it keeps as [`Shot::moved_from`]. First, a shot whose
    /// ship has gone from the fight is lost (see the module docs).
    pub fn step(&mut self, guide: &Guide, chance: &mut dyn Chance) -> Vec2 {
        let targets = guide.targets;
        self.lose_a_gone_ship(targets);
        match self.weapon.guidance {
            Guidance::Homing => self.home(guide, chance),
            Guidance::Rocket => {
                self.velocity = rocket_push(self.velocity, self.heading, self.weapon.speed);
            }
            Guidance::FreefallBomb => self.weathervane(guide.bomb_heading_wrap),
            _ => {}
        }
        let was = self.position;
        self.position = self.position + self.velocity;
        self.age += 1;
        self.moved_from = Some(was);
        was
    }

    /// Turns a bomb into the wind, as `_HandleShotGuidance`'s bomb tail
    /// does every tick before the shot moves (@0x329e3-0x32a98): its
    /// bearing is the table's whole-degree heading of its velocity
    /// (`_CalcInterceptAngle` from the origin, [`trig::intercept_angle`]),
    /// up (0) for a bomb at rest; the original scales the velocity by 1000
    /// first (@0xdd058), which changes no ratio the table reads. While its
    /// truncated heading is not the bearing (`_AngularDifference` @0x649e
    /// is 0 only then, @0x32a41-0x32a53), it turns [`BOMB_TURN`] a whole
    /// degree: back when the truncated difference from the heading to the
    /// bearing, taken into 0-359 in 16 bits (`≥ 360` → −360, `≤ −1` →
    /// +360, @0x32a62-0x32a7d), is above 180, and on otherwise, half round
    /// included. So it may pass its bearing within the degree (89.5
    /// towards 90 goes to 90.5) and then holds.
    ///
    /// By the engine (`wrap` [`RuleSource::Engine`]) the heading is never
    /// wrapped after the turn (@0x32a86-0x32a98), so a bomb that crosses
    /// up stops up to a degree and a half short of its bearing, below 0 or
    /// past 360 (0.5 towards 359 goes to -0.5, then -1.5, where
    /// `_AngularDifference(359, -1)` is 0), and is drawn on the end frame
    /// its frame index is clamped to. By the other reading
    /// ([`RuleKey::BombHeadingWrap`](crate::RuleKey::BombHeadingWrap)) it
    /// is wrapped into 0..360 after each turn.
    fn weathervane(&mut self, wrap: RuleSource) {
        let bearing = trig::intercept_angle(Vec2::ZERO, self.velocity);
        if trig::angular_difference(bearing, trig::whole(self.heading)) == 0 {
            return;
        }
        let mut off = i32::from(trig::whole(f32::from(bearing) - self.heading));
        if off >= 360 {
            off -= 360;
        }
        if off <= -1 {
            off += 360;
        }
        let turn = if off > 180 { -BOMB_TURN } else { BOMB_TURN };
        self.heading += turn;
        if wrap == RuleSource::Bible {
            self.heading = normalized(self.heading);
        }
    }

    /// `_HandleShot`'s target rule (@0x355e0-0x35621), run before the
    /// shot's life is counted down and it moves: a shot not jammed, and
    /// not point defence, whose ship ([`Shot::aimed_at`], +0x2a) is not
    /// active among `targets` (destroyed, or out of the fight as it
    /// landed, jumped or was taken out: the slot's +0xb8) is lost, jam
    /// 998, and aimed at nothing from then on. A jammed shot keeps its
    /// ship. Point defence (guidance 9) is aimed at no ship every tick
    /// and is never lost. A ship breaking up is still active, so a shot
    /// at it is not lost. A ship destroyed during a tick's resolution
    /// makes its shots lost as they next move, the next tick: the
    /// original's slot is cleared in `_HandleShipDisplay`, after
    /// `_HandleShot` has run that frame.
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

    /// Steers a homing shot a tick, as `_HandleShotGuidance` does
    /// (@0x320dc-0x32aa4; see the module docs): an interfered one circles
    /// ([`Shot::circle`]); one locked onto an asteroid steers at it
    /// ([`Shot::steer_at_asteroid`]); any other, past [`HOMING_DELAY`] ticks with its
    /// target active among `guide`'s ships, breaking up included, is
    /// tested in order for its target's jamming ([`Shot::jammed`]), its
    /// firer's sight of the target ([`Shot::blinded`]) and a `Seeker`
    /// 0x4000 lock ([`Shot::lock_holds`]), then turns at the bearing it
    /// read before them; then its velocity is its speed along its heading,
    /// and one that began the tick not jammed may lock onto an asteroid
    /// ([`Shot::seek_asteroid`]). Draws are made on `chance`.
    fn home(&mut self, guide: &Guide, chance: &mut dyn Chance) {
        match self.jam {
            Jam::Interfered => {
                self.circle(guide.frame, chance);
                return;
            }
            Jam::Asteroid(slot) => {
                self.steer_at_asteroid(slot, guide);
                return;
            }
            Jam::Clear | Jam::Lost => {}
        }
        let seeks = self.jam == Jam::Clear;
        let targets = guide.targets;
        let target = self.target.and_then(|ship| {
            targets
                .iter()
                .find(|target| target.ship == ship && target.condition.active())
        });
        if let Some(target) = target.filter(|_| self.age >= HOMING_DELAY) {
            let wanted = trig::intercept_angle(self.position, target.position);
            let mut turn = self.weapon.turn;
            self.jammed(target.ship, &mut turn, guide, chance);
            self.blinded(target, &mut turn, guide, chance);
            if !self.lock_holds(guide) {
                self.target = None;
                self.aimed_at = None;
            }
            self.heading = homing_turn(self.heading, wanted, turn);
        }
        self.heading = wrap_turned(self.heading);
        self.velocity = trig::accel(trig::whole(self.heading), self.weapon.speed);
        if seeks {
            self.seek_asteroid(guide, chance);
        }
    }

    /// The jam test (@0x321b7-0x32275): when `target`'s jamming jams the
    /// shot's rolls ([`jamming::jams`](super::jamming::jams)), a positive
    /// `turn` is turned away under `Seeker` 0x0010 ([`TURNS_AWAY_JAMMED`])
    /// and stopped otherwise, and under `Seeker` 0x8000
    /// ([`TURNS_ON_FIRER`]) a [`JAMMED_TURNS_ON_FIRER`] roll of none turns
    /// it on its firer ([`Shot::turn_on_firer`]). It sets no jam.
    fn jammed(&mut self, target: ShipRef, turn: &mut f32, guide: &Guide, chance: &mut dyn Chance) {
        let sight = guide.sight(target);
        if !super::jamming::jams(self.jam_rolls, sight.jamming, sight.disabled) {
            return;
        }
        if *turn > 0.0 {
            *turn = if self.weapon.seeks(TURNS_AWAY_JAMMED) {
                -*turn
            } else {
                0.0
            };
        }
        if self.weapon.seeks(TURNS_ON_FIRER) && chance.below(JAMMED_TURNS_ON_FIRER) == 0 {
            self.turn_on_firer();
        }
    }

    /// The cloak test (@0x32284-0x32335), for a shot a ship launched: when
    /// its firer does not see `target` ([`cloak::seen`]), the `turn` stops,
    /// and under `Seeker` 0x8000 a [`BLINDED_TURNS_ON_FIRER`] roll of none
    /// turns it on its firer. The firer looks from where it is, with its
    /// scanner, and as an escort of `target` with a cloak of its own; a
    /// firer no longer in the fight sees only a ship neither cloaked nor
    /// coming out of a gate (the original reads its stale slot,
    /// @0x322b4-0x322d8).
    fn blinded(&mut self, target: &Target, turn: &mut f32, guide: &Guide, chance: &mut dyn Chance) {
        if !matches!(self.source, Source::Ship(_)) {
            return;
        }
        let observer = guide
            .targets
            .iter()
            .find(|ship| ship.ship == self.firer)
            .map_or_else(Observer::default, |firer| {
                let sight = guide.sight(firer.ship);
                Observer {
                    at: firer.position,
                    scanner: sight.scanner,
                    cloaked_escort: firer.ties.leader == Some(target.ship) && sight.has_cloak,
                }
            });
        let seen = guide.sight(target.ship);
        if cloak::seen(seen.cloaked, target.position, seen.emerging, observer) {
            return;
        }
        *turn = 0.0;
        if self.weapon.seeks(TURNS_ON_FIRER) && chance.below(BLINDED_TURNS_ON_FIRER) == 0 {
            self.turn_on_firer();
        }
    }

    /// Whether a `Seeker` 0x4000 shot ([`LOSES_LOCK_OFF_AHEAD`]) keeps its
    /// lock on its target as the jam and cloak tests left it
    /// (@0x3233b-0x323ed): it loses it on a ship within [`LOCK_BOX`] on
    /// each axis, truncated, whose truncated bearing off its heading is
    /// above [`LOCK_CONE`] as `guide`'s [`LockAngle`] measures it. Any
    /// other shot, and one whose target is not in the fight, keeps it.
    fn lock_holds(&self, guide: &Guide) -> bool {
        if !self.weapon.seeks(LOSES_LOCK_OFF_AHEAD) {
            return true;
        }
        let Some(target) = self
            .target
            .and_then(|ship| guide.targets.iter().find(|target| target.ship == ship))
        else {
            return true;
        };
        let off = target.position - self.position;
        if off.x.trunc().abs() > LOCK_BOX || off.y.trunc().abs() > LOCK_BOX {
            return true;
        }
        let bearing = trig::intercept_angle(self.position, target.position);
        let angle = (f32::from(bearing) - self.heading).trunc();
        guide.lock_angle.off_ahead(angle) <= LOCK_CONE
    }

    /// Sensor interference's flight (@0x32613-0x32769): past
    /// [`HOMING_DELAY`] ticks it turns its turn a tick, back while the
    /// `frame` counter's truncated remainder by [`CIRCLE_FRAMES`] is at
    /// most [`CIRCLE_BACK`] and on otherwise, whatever its target; its
    /// velocity is its speed along its heading. Then, every tick, under
    /// `Seeker` 0x8000 a [`BLINDED_TURNS_ON_FIRER`] roll of none drawn on
    /// `chance` clears it, turned on its firer.
    fn circle(&mut self, frame: i32, chance: &mut dyn Chance) {
        if self.age >= HOMING_DELAY {
            let turn = self.weapon.turn;
            let back = frame % CIRCLE_FRAMES <= CIRCLE_BACK;
            self.heading = if back {
                self.heading - turn
            } else {
                self.heading + turn
            };
        }
        self.heading = wrap_turned(self.heading);
        self.velocity = trig::accel(trig::whole(self.heading), self.weapon.speed);
        if self.weapon.seeks(TURNS_ON_FIRER)
            && chance.below(BLINDED_TURNS_ON_FIRER) == 0
            && matches!(self.source, Source::Ship(_))
        {
            self.jam = Jam::Clear;
            self.turn_on_firer();
        }
    }

    /// Turns a shot a ship launched on its firer: its target and the ship
    /// its damage is aimed at become the firer (+0x2a), and it is launched
    /// by no ship from then on ([`Source::Other`], +0x2c = -1), so it can
    /// hit only its firer, holds no reload, locks nothing, and its hit
    /// and blast are no one's. A shot no ship launched is left alone.
    fn turn_on_firer(&mut self) {
        if matches!(self.source, Source::Ship(_)) {
            self.target = Some(self.firer);
            self.aimed_at = Some(self.firer);
            self.source = Source::Other;
        }
    }

    /// Draws what `_SpawnShot` rolls for a shot as it is launched, on
    /// `chance`, in a system of `interference`: its rolls against each
    /// jamming type ([`jamming::roll`](super::jamming::roll),
    /// @0x3f051-0x3f0a7), and for a homing shot of `Seeker` 0x0008
    /// ([`SENSOR_INTERFERENCE`]) whether it is [`Jam::Interfered`]:
    /// `below(`[`INTERFERENCE_ROLL`]`) + 1` at most the `interference`
    /// (@0x3eab2-0x3eb66). A sub-munition rolls its own weapon's.
    pub fn roll_at_launch(&mut self, interference: i16, chance: &mut dyn Chance) {
        if self.weapon.guidance == Guidance::Homing && self.weapon.seeks(SENSOR_INTERFERENCE) {
            let roll = i64::from(chance.below(INTERFERENCE_ROLL)) + 1;
            if roll <= i64::from(interference) {
                self.jam = Jam::Interfered;
            }
        }
        self.jam_rolls = super::jamming::roll(self.weapon.jam_vuln, chance);
    }

    /// The ship it is a hit by: its firer, or none for a shot no ship
    /// launched ([`Source::Other`]), as `_DamageShip` and the blast pass
    /// read the source slot -1 (see [`Strike::by`](super::Strike::by)).
    #[must_use]
    pub fn attacker(&self) -> Option<ShipRef> {
        match self.source {
            Source::Ship(_) => Some(self.firer),
            Source::Other => None,
        }
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

    /// Whether its proximity fuse is armed: it has flown past its
    /// weapon's `ProxSafety` (@0x371fc-0x3721e, @0x3720e-0x3721e).
    #[must_use]
    pub fn fuse_armed(&self) -> bool {
        self.age > self.weapon.prox_safety
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

    /// The ship among `sides` it hits on its move here from `was`, if
    /// any, where and how: the ship it touched ([`Shot::touched`]), or
    /// else one that set its proximity fuse off as `fuse` reads it
    /// ([`Shot::fused`]). `_CheckShotProximities` runs after the contact
    /// passes, so a contact anywhere on the move is the hit.
    #[must_use]
    pub fn hit(&self, was: Vec2, sides: &Sides, fuse: ProximityFuse) -> Option<Meeting> {
        self.touched(was, sides)
            .or_else(|| self.fused(was, sides, fuse))
    }

    /// The first of the ships among `sides` it can hit that it touched on
    /// its way here from `was`, within the ship's hit radius, and where:
    /// the original's sprite collision passes (`_ShipCollisionHandler`
    /// @0x36f87-0x370a6), swept so that a fast shot cannot pass through a
    /// ship between ticks. `ProxSafety` holds a touch until its age has
    /// reached it (@0x37066-0x37086). A shot in point defence's layer
    /// ([`ShotLayer::PointDefence`]) never touches a ship: that layer
    /// (0x3b53bc) is collided with the ships' only by
    /// `_ShipCollisionHandler`, which acts on a shot in the other shot
    /// layers alone (`_DoPlayGameWork` @0x43fdd-0x43ffa, @0x36fbd-0x36fd6).
    /// A point-defence shot fired at a ship is in the shot layer, and
    /// touches one.
    ///
    /// It takes the ship met first along the move, which [`Shot::hit`]
    /// asks. The hit pass itself walks the ships as the original's contact
    /// passes do, in their layers' order, a hidden ship not among them,
    /// each ship asking [`Shot::touches`]: of two ships a move touches,
    /// the one walked first takes the shot, wherever it is along the move.
    ///
    /// The engine tests a swept hit radius in place of the original's
    /// sprite rects and RLE masks (`_SWRadiusCollision`,
    /// `_SWRLECollision` @0x3700f-0x3704a).
    #[must_use]
    pub fn touched(&self, was: Vec2, sides: &Sides) -> Option<Meeting> {
        if !self.touches_ships() {
            return None;
        }
        self.first_along(was, sides, Fuse::Contact, 0.0)
    }

    /// Where on its move here from `was` it touched `target`, within its
    /// hit radius, whether or not it can hit it: one ship's contact, as
    /// the contact passes test each ship they walk ([`Shot::touched`]).
    /// None for a shot in point defence's layer, or before its
    /// `ProxSafety`.
    #[must_use]
    pub fn touches(&self, was: Vec2, target: &Target) -> Option<Vec2> {
        if !self.touches_ships() {
            return None;
        }
        let t = contact(was, self.position, target.position, target.radius)?;
        Some(was + (self.position - was) * t)
    }

    /// Whether it can touch a ship at all: it is not in point defence's
    /// layer, one no ship's collision proc acts on (see
    /// [`Shot::touched`]), and its age has reached its `ProxSafety`.
    fn touches_ships(&self) -> bool {
        self.layer != ShotLayer::PointDefence && self.age >= self.weapon.prox_safety
    }

    /// The ship among `sides` that sets its proximity fuse off, its move
    /// having come here from `was`, and where, as `fuse` reads it: none
    /// for a weapon of no `ProxRadius` (signed, `jle` @0x37560), nor
    /// until its age is past its `ProxSafety` ([`Shot::fuse_armed`],
    /// @0x3720e-0x3721e). `_CheckShotProximities` has no guidance test, so
    /// a point-defence shot is set off as any other.
    ///
    /// - [`ProximityFuse::Point`], the original's ship branch
    ///   (@0x37549-0x376a5): the first ship in `sides`' order, the
    ///   fight's slot order with the player first, that it can hit
    ///   (`_ShotCanHitShip` @0x375b2) and whose offset from the shot's
    ///   end point is within reach. The reach is trunc(`ProxRadius` + the
    ///   ship's [`Target::fuse_width`] × [`FUSE_SPRITE_SHARE`]), in
    ///   doubles (@0x375dd-0x37628); the offset is trunc(ship) −
    ///   trunc(shot) on each axis, each coordinate truncated
    ///   (@0x37634-0x3765e); it is met when reach² ≥ the offsets squared
    ///   and summed (@0x3766e-0x37670). The meeting is at the shot's end
    ///   point, which its blast and sub-munitions read (`_HandleShipHit`
    ///   @0x36aa9, @0x36d4b, @0x36f12).
    /// - [`ProximityFuse::Swept`]: the first ship its move met within the
    ///   ship's hit radius and `ProxRadius`, where it entered that reach.
    #[must_use]
    pub fn fused(&self, was: Vec2, sides: &Sides, fuse: ProximityFuse) -> Option<Meeting> {
        let prox = self.weapon.prox_radius;
        if !self.fuse_armed() || prox <= 0.0 {
            return None;
        }
        match fuse {
            ProximityFuse::Swept => self.first_along(was, sides, Fuse::Proximity, prox),
            ProximityFuse::Point => {
                let whole = |value: f32| value.trunc() as i64;
                sides
                    .targets
                    .iter()
                    .filter(|target| self.can_hit(target, sides))
                    .find(|target| {
                        let width = f64::from(target.fuse_width());
                        let reach = (width * FUSE_SPRITE_SHARE + f64::from(prox)).trunc() as i64;
                        let dx = whole(target.position.x) - whole(self.position.x);
                        let dy = whole(target.position.y) - whole(self.position.y);
                        reach * reach >= dx * dx + dy * dy
                    })
                    .map(|target| Meeting {
                        ship: target.ship,
                        at: self.position,
                        fuse: Fuse::Proximity,
                    })
            }
        }
    }

    /// The first of the ships among `sides` it can hit that its move here
    /// from `was` came within each one's hit radius and `prox` of, where,
    /// met by `fuse`.
    fn first_along(&self, was: Vec2, sides: &Sides, fuse: Fuse, prox: f32) -> Option<Meeting> {
        nearest_contact(
            was,
            self.position,
            sides
                .targets
                .iter()
                .filter(|target| self.can_hit(target, sides))
                .map(|target| (target.ship, target.position, target.radius + prox)),
        )
        .map(|(ship, at)| Meeting { ship, at, fuse })
    }

    /// Its blast where it is, hitting `direct` (or nothing, as it
    /// detonates).
    #[must_use]
    pub fn blast(&self, direct: Option<ShipRef>) -> Blast {
        Blast {
            at: self.position,
            radius: self.weapon.blast_radius,
            direct,
            firer: self.attacker(),
            spares_player: self.weapon.spares_player(),
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::catalog::{GovtRecord, WeaponRecord};
    use crate::chance::NeverFires;
    use crate::combat::NpcId;
    use crate::combat::fixture::among;
    use crate::combat::sides::PassThrough;
    use crate::combat::weapon::{LOSES_LOCK_OFF_AHEAD, TURNS_AWAY_JAMMED, TURNS_ON_FIRER};
    use crate::flight::facing;
    use crate::govt::{Governments, NEVER_ATTACKS_PLAYER, PLAYER_SHOTS_PASS};
    use nova_sim_base::fixture::Ranged;
    use nova_sim_catalog::fixture::{govt, weapon};

    const FIRER: ShipRef = ShipRef::Npc(NpcId(1));

    /// Flies `shot` a tick among `targets`, with no sights, at frame 0,
    /// every draw its last outcome.
    fn step(shot: &mut Shot, targets: &[Target]) -> Vec2 {
        shot.step(&Guide::among(targets), &mut NeverFires)
    }

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
    fn a_shots_fuse_is_armed_once_it_has_flown_past_its_prox_safety() {
        let mut shot = Shot::launch(
            WeaponSpec::new(&WeaponRecord {
                prox_safety: 3,
                ..weapon(128)
            }),
            FIRER,
            FIRER,
            &state(0.0, 0.0, 0.0, 0.0, 0.0),
            0.0,
        );
        for (age, armed) in [(0, false), (3, false), (4, true), (40, true)] {
            shot.age = age;
            assert_eq!(shot.fuse_armed(), armed, "{age}");
        }
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
            assert_eq!(step(&mut shot, &[]), was);
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
        step(&mut rocket, &[]);
        assert!(close(rocket.velocity, Vec2::new(0.5, 0.0)), "{rocket:?}");
        assert!(
            close(rocket.position, Vec2::new(0.5, 0.0)),
            "moved after speeding up"
        );
        step(&mut rocket, &[]);
        assert!(close(rocket.velocity, Vec2::new(0.975, 0.0)), "{rocket:?}");
        for _ in 0..200 {
            step(&mut rocket, &[]);
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
        step(&mut bomb, &[]);
        assert!((bomb.heading - 1.0).abs() < 1e-4, "{bomb:?}");
        assert!(close(bomb.velocity, Vec2::new(4.0, 0.0)), "kept");
        assert!(close(bomb.position, Vec2::new(4.0, 0.0)));
        for _ in 0..100 {
            step(&mut bomb, &[]);
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
        step(&mut left, &[]);
        assert_eq!(left.heading, -1.0, "unwrapped by the engine: {left:?}");
        assert_eq!(BOMB_TURN, 1.0);
    }

    /// A bomb facing `heading`, moving `velocity`, after `ticks` ticks.
    fn vaned(heading: f32, velocity: Vec2, ticks: u32) -> f32 {
        vaned_by(heading, velocity, ticks, RuleSource::Engine)
    }

    /// As [`vaned`], its heading wrapped as `wrap` reads
    /// [`RuleKey::BombHeadingWrap`](crate::RuleKey::BombHeadingWrap).
    fn vaned_by(heading: f32, velocity: Vec2, ticks: u32, wrap: RuleSource) -> f32 {
        let mut bomb = Shot {
            velocity,
            heading,
            ..Shot::launch(spec(5, 1000, 600), FIRER, FIRER, &ShipState::default(), 0.0)
        };
        let guide = Guide {
            bomb_heading_wrap: wrap,
            ..Guide::among(&[])
        };
        for _ in 0..ticks {
            bomb.step(&guide, &mut NeverFires);
        }
        bomb.heading
    }

    #[test]
    fn a_bomb_turns_a_whole_degree_while_its_truncated_heading_is_off_its_bearing() {
        let right = Vec2::new(4.0, 0.0);
        assert_eq!(
            vaned(9.5, Vec2::new(0.0, -4.0), 1),
            8.5,
            "bearing 0, the short way"
        );
        assert_eq!(
            vaned(89.5, right, 1),
            90.5,
            "past the bearing within its degree"
        );
        assert_eq!(vaned(89.5, right, 5), 90.5, "then it holds");
        assert_eq!(vaned(90.5, right, 3), 90.5, "on its degree already");
        assert_eq!(vaned(80.0, right, 3), 83.0);
    }

    #[test]
    fn a_stationary_bomb_turns_towards_up() {
        assert_eq!(vaned(30.0, Vec2::ZERO, 1), 29.0);
        assert_eq!(vaned(30.0, Vec2::ZERO, 30), 0.0);
        assert_eq!(vaned(300.0, Vec2::ZERO, 10), 310.0);
    }

    #[test]
    fn a_bomb_half_round_from_its_bearing_turns_clockwise() {
        assert_eq!(vaned(270.0, Vec2::new(4.0, 0.0), 1), 271.0, "180 off");
        assert_eq!(vaned(269.0, Vec2::new(4.0, 0.0), 1), 268.0, "181 off");
        assert_eq!(vaned(271.0, Vec2::new(4.0, 0.0), 1), 272.0, "179 off");
    }

    #[test]
    fn a_bomb_settles_on_the_tables_bearing() {
        // Up and right on the diagonal is the table's 46, not 45.
        assert_eq!(vaned(40.0, Vec2::new(1.0, -1.0), 10), 46.0);
        assert_eq!(vaned(50.0, Vec2::new(1.0, -1.0), 10), 46.0);
    }

    #[test]
    fn by_the_engine_a_bomb_crossing_up_stops_short_of_its_bearing_unwrapped() {
        // Its bearing is the table's 359.
        let left_of_up = Vec2::new(-1.0, -40.0);
        assert_eq!(trig::intercept_angle(Vec2::ZERO, left_of_up), 359);
        assert_eq!(vaned(0.5, left_of_up, 1), -0.5);
        assert_eq!(vaned(0.5, left_of_up, 2), -1.5);
        // `_AngularDifference(359, -1)` is 0: it holds there.
        assert_eq!(vaned(0.5, left_of_up, 6), -1.5, "then holds");
        assert_eq!(vaned(1.0, left_of_up, 1), 0.0);
        assert_eq!(vaned(0.0, left_of_up, 1), -1.0);
        // The other way: 359.5 towards 0 goes to 360.5 and holds.
        let up = Vec2::new(0.0, -4.0);
        assert_eq!(vaned(359.5, up, 1), 360.5);
        assert_eq!(vaned(359.5, up, 4), 360.5, "then holds");
    }

    #[test]
    fn an_unwrapped_bombs_16_bit_difference_is_taken_round_once_each_way() {
        let left_of_up = Vec2::new(-1.0, -40.0);
        // 359 - (-2.5) truncates to 361, taken to 1: on.
        assert_eq!(vaned(-2.5, left_of_up, 1), -1.5);
        // 359 - (-182.5) truncates to 541, taken to 181: back.
        assert_eq!(vaned(-182.5, left_of_up, 1), -183.5);
    }

    #[test]
    fn by_the_other_reading_a_bomb_crossing_up_wraps_and_settles_on_its_bearing() {
        let bible = RuleSource::Bible;
        let left_of_up = Vec2::new(-1.0, -40.0);
        assert_eq!(vaned_by(0.5, left_of_up, 1, bible), 359.5);
        assert_eq!(vaned_by(0.5, left_of_up, 4, bible), 359.5, "then holds");
        assert_eq!(vaned_by(0.0, left_of_up, 1, bible), 359.0);
        let up = Vec2::new(0.0, -4.0);
        assert!((vaned_by(359.5, up, 1, bible) - 0.5).abs() < 1e-4);
        assert!(
            (vaned_by(359.5, up, 4, bible) - 0.5).abs() < 1e-4,
            "then holds"
        );
    }

    #[test]
    fn the_guide_reads_the_engines_unwrapped_bomb_heading_by_default() {
        assert_eq!(Guide::among(&[]).bomb_heading_wrap, RuleSource::Engine);
    }

    #[test]
    fn a_thrown_bomb_or_rocket_adds_its_speed_and_a_dropped_one_does_not() {
        let from = state(0.0, 0.0, 1.0, 2.0, 90.0);
        let rocket = spec(6, 1000, 60);
        let bomb = spec(5, 1000, 60);
        let thrown = |weapon| launch_velocity(&weapon, &from, 90.0, BombLaunch::Thrown);
        let dropped = |weapon| launch_velocity(&weapon, &from, 90.0, BombLaunch::Dropped);
        assert!(
            close(thrown(rocket), Vec2::new(11.0, 2.0)),
            "{:?}",
            thrown(rocket)
        );
        assert!(
            close(thrown(bomb), Vec2::new(11.0, 2.0)),
            "{:?}",
            thrown(bomb)
        );
        assert_eq!(dropped(rocket), Vec2::new(1.0, 2.0));
        assert!(
            close(dropped(bomb), Vec2::new(0.8, 1.6)),
            "{:?}",
            dropped(bomb)
        );
        for guidance in [-1, 4] {
            let other = spec(guidance, 1000, 60);
            assert_eq!(thrown(other), dropped(other), "{guidance}");
            assert!(close(thrown(other), Vec2::new(11.0, 2.0)), "{guidance}");
        }
        let homing = spec(1, 1000, 60);
        assert!(
            close(thrown(homing), Vec2::new(10.0, 0.0)),
            "no ship velocity"
        );
        assert_eq!(thrown(homing), dropped(homing));
    }

    #[test]
    fn an_npcs_bomb_or_rocket_is_thrown_by_the_engine_and_any_other_dropped() {
        assert_eq!(BombLaunch::of(true, RuleSource::Engine), BombLaunch::Thrown);
        assert_eq!(BombLaunch::of(true, RuleSource::Bible), BombLaunch::Dropped);
        for source in RuleSource::ALL {
            assert_eq!(BombLaunch::of(false, source), BombLaunch::Dropped);
        }
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
                firer: Some(FIRER),
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
            sprite_x: 15.0,
            sheet: true,
            condition,
            disabled: condition != Condition::Intact,
            miss_class: 100,
            slot: crate::ship_layer::Slot::Berth(id),
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
        step(&mut shot, &[]);
        shot
    }

    #[test]
    fn a_fast_shot_hits_a_ship_it_passes_between_ticks() {
        let shot = passing(0);
        assert!(close(shot.position, Vec2::new(100.0, 0.0)));
        let hit = |targets: &[Target]| {
            shot.hit(Vec2::ZERO, &among(targets), ProximityFuse::Swept)
                .map(|met| met.ship)
        };
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
                ProximityFuse::Swept,
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
                &among(&[target(2, 50.0, 8.1, Condition::Intact)]),
                ProximityFuse::Swept,
            ),
            None
        );
    }

    #[test]
    fn a_shot_touches_a_ship_within_its_radius_alone_not_its_fuses_reach() {
        let shot = passing(3);
        let met =
            |was: Vec2, targets: &[Target]| shot.hit(was, &among(targets), ProximityFuse::Swept);
        let fused = [target(2, 50.0, 7.9, Condition::Intact)];
        assert_eq!(
            met(Vec2::ZERO, &fused).map(|met| met.fuse),
            Some(Fuse::Proximity),
            "the fuse alone"
        );
        let touched = [
            target(3, 80.0, 4.9, Condition::Intact),
            target(2, 50.0, 4.9, Condition::Intact),
        ];
        let first = met(Vec2::ZERO, &touched).expect("touched");
        assert_eq!(
            (first.ship, first.fuse),
            (ShipRef::Npc(NpcId(2)), Fuse::Contact),
            "the nearer along the way"
        );
        assert!(first.at.x > 48.0 && first.at.x < 50.0, "{first:?}");
        let from_20 = met(Vec2::new(20.0, 0.0), &touched).expect("touched");
        assert!(
            (from_20.at.x - first.at.x).abs() < 1e-3 && from_20.at.y.abs() < 1e-3,
            "met at the same point on a shorter way: {from_20:?}"
        );
    }

    #[test]
    fn a_shot_touches_one_ship_where_touched_meets_it_alone() {
        let shot = passing(3);
        let lone = target(2, 50.0, 4.9, Condition::Intact);
        let touched = shot.touched(Vec2::ZERO, &among(&[lone])).expect("touched");
        assert_eq!(shot.touches(Vec2::ZERO, &lone), Some(touched.at));
        // Behind a nearer ship it still touches: one ship's contact.
        assert!(
            shot.touches(Vec2::ZERO, &target(3, 80.0, 0.0, Condition::Intact))
                .is_some()
        );
        assert_eq!(
            shot.touches(Vec2::ZERO, &target(2, 50.0, 7.9, Condition::Intact)),
            None,
            "within its fuse's reach alone"
        );
    }

    #[test]
    fn a_shot_touches_no_ship_before_its_prox_safety() {
        let mut shot = passing(0);
        shot.weapon.prox_safety = 5;
        shot.age = 4;
        let on_path = target(2, 50.0, 0.0, Condition::Intact);
        assert_eq!(shot.touches(Vec2::ZERO, &on_path), None);
        shot.age = 5;
        assert!(shot.touches(Vec2::ZERO, &on_path).is_some());
    }

    #[test]
    fn a_shot_in_point_defences_layer_touches_no_ship() {
        let shot = Shot {
            layer: ShotLayer::PointDefence,
            ..passing(0)
        };
        assert_eq!(
            shot.touches(Vec2::ZERO, &target(2, 50.0, 0.0, Condition::Intact)),
            None
        );
    }

    #[test]
    fn a_shot_within_a_ships_radius_hits_it_by_contact_and_within_its_reach_by_proximity() {
        let shot = passing(3);
        let met = |y| {
            shot.hit(
                Vec2::ZERO,
                &among(&[target(2, 50.0, y, Condition::Intact)]),
                ProximityFuse::Swept,
            )
            .map(|met| met.fuse)
        };
        assert_eq!(met(4.0), Some(Fuse::Contact));
        assert_eq!(met(7.9), Some(Fuse::Proximity));
        assert_eq!(met(8.1), None);
        let no_fuse = passing(0);
        let on = no_fuse.hit(
            Vec2::ZERO,
            &among(&[target(2, 50.0, 4.0, Condition::Intact)]),
            ProximityFuse::Swept,
        );
        assert_eq!(on.map(|met| met.fuse), Some(Fuse::Contact));
    }

    #[test]
    fn a_shot_in_point_defences_layer_meets_a_ship_only_by_its_proximity_fuse() {
        let defence = |prox| {
            let mut shot = passing(prox);
            shot.weapon.guidance = Guidance::PointDefence;
            shot.layer = ShotLayer::PointDefence;
            shot
        };
        let through = [target(2, 50.0, 0.0, Condition::Intact)];
        let met = |shot: Shot| {
            shot.hit(Vec2::ZERO, &among(&through), ProximityFuse::Swept)
                .map(|met| (met.ship, met.fuse))
        };
        assert_eq!(met(defence(0)), None, "through its centre, no fuse");
        assert_eq!(
            met(defence(5)),
            Some((ShipRef::Npc(NpcId(2)), Fuse::Proximity)),
            "set off on its way"
        );
        assert_eq!(
            met(passing(0)),
            Some((ShipRef::Npc(NpcId(2)), Fuse::Contact)),
            "an unguided shot touches it"
        );
        let moved = Shot {
            layer: ShotLayer::Shots,
            ..defence(0)
        };
        assert_eq!(
            met(moved),
            Some((ShipRef::Npc(NpcId(2)), Fuse::Contact)),
            "moved to the shot layer, as point defence fired at a ship is"
        );
    }

    #[test]
    fn a_shot_is_launched_in_its_guidances_layer() {
        let layer = |guidance| {
            Shot::launch(
                spec(guidance, 1000, 10),
                FIRER,
                FIRER,
                &state(0.0, 0.0, 0.0, 0.0, 0.0),
                0.0,
            )
            .layer
        };
        assert_eq!(layer(9), ShotLayer::PointDefence);
        assert_eq!(layer(1), ShotLayer::Homing);
        for guidance in [-1, 0, 3, 4, 5, 6, 7, 8, 10] {
            assert_eq!(layer(guidance), ShotLayer::Shots, "{guidance}");
        }
        assert_eq!(ShotLayer::default(), ShotLayer::Shots);
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
        let met = shot
            .hit(Vec2::ZERO, &among(&targets), ProximityFuse::Swept)
            .expect("a hit");
        assert_eq!(
            (met.ship, met.fuse),
            (ShipRef::Npc(NpcId(2)), Fuse::Contact)
        );
        assert!(close(met.at, Vec2::new(75.0, 0.0)), "{met:?}");
        let alone = shot
            .hit(Vec2::ZERO, &among(&targets[..1]), ProximityFuse::Swept)
            .expect("a hit");
        assert_eq!(
            (alone.ship, alone.fuse),
            (ShipRef::Npc(NpcId(3)), Fuse::Proximity)
        );
    }

    /// Who `shot`, flown from the origin, sets its fuse off among
    /// `targets`, by `fuse`.
    fn fused_by(shot: &Shot, targets: &[Target], fuse: ProximityFuse) -> Option<ShipRef> {
        shot.hit(Vec2::ZERO, &among(targets), fuse)
            .map(|met| met.ship)
    }

    #[test]
    fn a_fused_shot_passing_a_ship_between_two_points_is_not_set_off() {
        // Beside the path halfway, out of its touch, within the swept
        // reach of 5 + 3, and 50 pixels from where the move ends.
        let beside = [target(2, 50.0, 7.0, Condition::Intact)];
        let shot = passing(3);
        assert_eq!(fused_by(&shot, &beside, ProximityFuse::Point), None);
        assert_eq!(
            fused_by(&shot, &beside, ProximityFuse::Swept),
            Some(ShipRef::Npc(NpcId(2))),
            "the other reading sweeps it"
        );
        assert_eq!(ProximityFuse::default(), ProximityFuse::Point);
        assert_eq!(
            ProximityFuse::from(RuleSource::Engine),
            ProximityFuse::Point
        );
        assert_eq!(ProximityFuse::from(RuleSource::Bible), ProximityFuse::Swept);
    }

    #[test]
    fn the_fuse_takes_the_first_ship_in_slot_order_within_reach_not_the_nearest() {
        // Both off the touch (radius 5) and within the end point's reach
        // of trunc(3 + 15 x 0.333) = 7: ship 3 at 6.5 to one side, the
        // player at 5.5 to the other, which the sweep meets first.
        let player = Target {
            ship: ShipRef::Player,
            fleet: ShipRef::Player,
            ..target(0, 100.0, -5.5, Condition::Intact)
        };
        let npc = target(3, 100.0, 6.5, Condition::Intact);
        let shot = passing(3);
        assert_eq!(
            fused_by(&shot, &[npc, player], ProximityFuse::Point),
            Some(ShipRef::Npc(NpcId(3))),
            "first in the order, not the nearest"
        );
        assert_eq!(
            fused_by(&shot, &[player, npc], ProximityFuse::Point),
            Some(ShipRef::Player),
            "the player's slot first"
        );
        assert_eq!(
            fused_by(&shot, &[npc, player], ProximityFuse::Swept),
            Some(ShipRef::Player),
            "the sweep's first met"
        );
        let firer = Target { ship: FIRER, ..npc };
        assert_eq!(
            fused_by(&shot, &[firer, player], ProximityFuse::Point),
            Some(ShipRef::Player),
            "only a ship it can hit"
        );
    }

    #[test]
    fn the_fuse_reaches_prox_radius_and_a_third_of_the_sprite_width_in_whole_pixels() {
        let shot = passing(3);
        let at = |x: f32, y: f32| target(2, x, y, Condition::Intact);
        let point = |shot: &Shot, ship: Target| fused_by(shot, &[ship], ProximityFuse::Point);
        // trunc(3 + 15 x 0.333) = 7, inclusive.
        assert!(point(&shot, at(100.0, 7.9)).is_some(), "7 off");
        assert!(point(&shot, at(100.0, 8.0)).is_none(), "8 off");
        assert!(
            point(&shot, at(100.0, -7.9)).is_some(),
            "7 off the other way"
        );
        assert!(point(&shot, at(104.0, 5.9)).is_some(), "4² + 5² ≤ 7²");
        assert!(point(&shot, at(105.0, 5.0)).is_none(), "5² + 5² > 7²");
        // Each coordinate is truncated before the difference: 107 − 99 is
        // 8, where the difference 7.6 would truncate to 7.
        let mut fractional = shot;
        fractional.position = Vec2::new(99.6, 0.0);
        assert!(point(&fractional, at(106.9, 0.0)).is_some(), "106 − 99");
        assert!(point(&fractional, at(107.2, 0.0)).is_none(), "107 − 99");
        // No sheet: 32 wide, trunc(3 + 10.656) = 13.
        let unsheeted = |y| Target {
            sheet: false,
            ..at(100.0, y)
        };
        assert!(point(&shot, unsheeted(13.9)).is_some(), "13 off");
        assert!(point(&shot, unsheeted(14.0)).is_none(), "14 off");
        assert_eq!(unsheeted(0.0).fuse_width(), NO_SHEET_WIDTH);
        assert_eq!(at(0.0, 0.0).fuse_width(), 15.0, "its sprite's width");
        // A wide sprite's third: trunc(3 + 60 x 0.333) = 22.
        let wide = Target {
            sprite_x: 60.0,
            ..at(100.0, 22.9)
        };
        assert!(point(&shot, wide).is_some(), "22 off");
        assert!(
            point(
                &shot,
                Target {
                    position: Vec2::new(100.0, 23.0),
                    ..wide
                }
            )
            .is_none(),
            "23 off"
        );
        assert!(point(&passing(0), at(100.0, 6.0)).is_none(), "no fuse");
        let mut unarmed = passing(3);
        unarmed.weapon.prox_safety = 1;
        assert!(point(&unarmed, at(100.0, 6.0)).is_none(), "not armed");
        unarmed.age = 2;
        assert!(point(&unarmed, at(100.0, 6.0)).is_some(), "armed");
    }

    #[test]
    fn the_fused_meeting_is_at_the_shots_end_point() {
        let shot = passing(3);
        let beside = [target(2, 100.0, 6.5, Condition::Intact)];
        let met = shot
            .hit(Vec2::ZERO, &among(&beside), ProximityFuse::Point)
            .expect("set off");
        assert_eq!(
            met,
            Meeting {
                ship: ShipRef::Npc(NpcId(2)),
                at: shot.position,
                fuse: Fuse::Proximity,
            }
        );
        let swept = shot
            .hit(Vec2::ZERO, &among(&beside), ProximityFuse::Swept)
            .expect("set off");
        assert!(swept.at.x < 96.0, "where it entered: {swept:?}");
        // A touch on the way still comes first, where it touched.
        let touched = [
            target(3, 50.0, 0.0, Condition::Intact),
            target(2, 100.0, 6.5, Condition::Intact),
        ];
        let met = shot
            .hit(Vec2::ZERO, &among(&touched), ProximityFuse::Point)
            .expect("touched");
        assert_eq!(
            (met.ship, met.fuse),
            (ShipRef::Npc(NpcId(3)), Fuse::Contact)
        );
        assert!(close(met.at, Vec2::new(45.0, 0.0)), "{met:?}");
    }

    #[test]
    fn a_move_keeps_where_it_began() {
        let mut shot = Shot::launch(
            spec(-1, 1000, 10),
            FIRER,
            FIRER,
            &state(3.0, 4.0, 0.0, 0.0, 90.0),
            90.0,
        );
        assert_eq!(shot.moved_from, None, "not yet moved");
        step(&mut shot, &[]);
        assert_eq!(shot.moved_from, Some(Vec2::new(3.0, 4.0)));
        step(&mut shot, &[]);
        assert!(close(shot.moved_from.expect("moved"), Vec2::new(13.0, 4.0)));
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
            shot.hit(Vec2::ZERO, &among(targets), ProximityFuse::Swept)
                .map(|met| met.fuse)
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
        assert_eq!(
            shot.hit(Vec2::ZERO, &among(&[firer]), ProximityFuse::Swept),
            None
        );
        let Meeting { ship, at, .. } = shot
            .hit(
                Vec2::ZERO,
                &among(&[firer, target(3, 30.0, 0.0, Condition::Intact)]),
                ProximityFuse::Swept,
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
            govts,
            pass_through,
            ..crate::combat::fixture::among(targets)
        };
        shot.hit(Vec2::ZERO, &sides, ProximityFuse::Swept)
            .map(|met| met.ship)
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
            shot.hit(
                Vec2::ZERO,
                &among(&[target(2, 50.0, 0.0, condition)]),
                ProximityFuse::Swept,
            )
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
            step(&mut shot, &below);
            assert_eq!(shot.heading, 90.0, "{tick}");
        }
        assert!(close(shot.position, Vec2::new(150.0, 0.0)), "{shot:?}");
        step(&mut shot, &below);
        assert!((shot.heading - 97.0).abs() < 1e-4, "{shot:?}");
        assert!(close(shot.velocity, facing(97.0) * 10.0), "{shot:?}");
        step(&mut shot, &below);
        assert!((shot.heading - 104.0).abs() < 1e-4, "{shot:?}");
        let above = [target(2, 300.0, -1000.0, Condition::Intact)];
        let mut left = homing(missile(0), Some(TARGET));
        for _ in 0..16 {
            step(&mut left, &above);
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
            step(&mut shot, &ahead);
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
            step(&mut shot, &above);
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
        step(&mut shot, &[]);
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
            shot.hit(Vec2::ZERO, &among(&in_its_path()), ProximityFuse::Swept)
                .map(|met| met.ship),
            Some(TARGET)
        );
        let at_three = passing_homing(0, Some(ShipRef::Npc(NpcId(3))));
        assert_eq!(
            at_three
                .hit(Vec2::ZERO, &among(&in_its_path()), ProximityFuse::Swept)
                .map(|met| met.ship),
            Some(ShipRef::Npc(NpcId(3)))
        );
    }

    #[test]
    fn a_homing_shot_set_off_by_other_ships_hits_the_first_it_meets() {
        let shot = passing_homing(0x0008, Some(TARGET));
        assert_eq!(
            shot.hit(Vec2::ZERO, &among(&in_its_path()), ProximityFuse::Swept)
                .map(|met| met.ship),
            Some(ShipRef::Npc(NpcId(3)))
        );
        let untargeted = passing_homing(0x0008, None);
        assert_eq!(
            untargeted
                .hit(Vec2::ZERO, &among(&in_its_path()), ProximityFuse::Swept)
                .map(|met| met.ship),
            Some(ShipRef::Npc(NpcId(3)))
        );
    }

    #[test]
    fn a_homing_shot_without_a_target_flies_straight_and_hits_nothing() {
        let mut shot = homing(missile(0), None);
        for _ in 0..30 {
            step(&mut shot, &in_its_path());
        }
        assert_eq!(shot.heading, 90.0);
        assert_eq!(shot.jam, Jam::Clear);
        assert_eq!(
            passing_homing(0, None).hit(Vec2::ZERO, &among(&in_its_path()), ProximityFuse::Swept),
            None
        );
    }

    #[test]
    fn a_homing_shot_whose_target_is_gone_is_lost_and_one_breaking_up_is_chased() {
        for gone in [vec![], vec![target(2, 0.0, 1000.0, Condition::Destroyed)]] {
            let mut shot = homing(missile(0), Some(TARGET));
            shot.aimed_at = Some(TARGET);
            for _ in 0..20 {
                step(&mut shot, &gone);
            }
            assert_eq!(shot.jam, Jam::Lost, "{gone:?}");
            assert_eq!((shot.target, shot.aimed_at), (None, None), "{gone:?}");
            assert_eq!(shot.heading, 90.0, "flies straight on: {gone:?}");
            let back = [target(2, 0.0, 1000.0, Condition::Intact)];
            step(&mut shot, &back);
            assert_eq!(shot.heading, 90.0, "lost for good: {gone:?}");
        }
        let lost = Shot {
            jam: Jam::Lost,
            ..passing_homing(0x0008, Some(TARGET))
        };
        assert_eq!(
            lost.hit(Vec2::ZERO, &among(&in_its_path()), ProximityFuse::Swept),
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
                step(&mut chasing, &quarry);
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
            step(&mut kept, &[behind, aimed]);
            assert_eq!(kept.jam, Jam::Clear, "{guidance}");
            assert_eq!(kept.aimed_at, Some(TARGET), "{guidance}");
            assert_eq!(
                kept.hit(was, &among(&[behind, aimed]), ProximityFuse::Swept)
                    .map(|met| met.ship),
                Some(ShipRef::Npc(NpcId(3))),
                "{guidance}: aimed at a ship still there"
            );
            let mut lost = aimed_at_two(guidance);
            let was = lost.position;
            step(&mut lost, &[behind]);
            assert_eq!(lost.jam, Jam::Lost, "{guidance}");
            assert_eq!((lost.aimed_at, lost.target), (None, None), "{guidance}");
            assert_eq!(
                lost.hit(was, &among(&[behind]), ProximityFuse::Swept),
                None,
                "{guidance}"
            );
            for _ in 0..200 {
                let was = lost.position;
                step(&mut lost, &[behind, aimed]);
                assert_eq!(
                    lost.hit(was, &among(&[behind, aimed]), ProximityFuse::Swept),
                    None
                );
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
        step(&mut ahead, &[]);
        assert_eq!(ahead.jam, Jam::Clear);
    }

    #[test]
    fn a_point_defence_shot_is_never_lost_and_is_aimed_at_no_ship() {
        let mut defence = aimed_at_two(9);
        step(&mut defence, &[]);
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
        step(&mut shot, &[dying]);
        assert_eq!(shot.jam, Jam::Clear);
        let destroyed = Target {
            condition: Condition::Destroyed,
            ..dying
        };
        step(&mut shot, &[destroyed]);
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
            step(&mut turret, &below);
        }
        assert_eq!(turret.heading, 90.0);
        assert_eq!(turret.jam, Jam::Clear);
        let passing_turret = Shot {
            position: Vec2::new(100.0, 0.0),
            ..turret
        };
        assert_eq!(
            passing_turret
                .hit(Vec2::ZERO, &among(&in_its_path()), ProximityFuse::Swept)
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
            step(&mut shot, &[]);
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

    /// An IR Missile with `seeker`, at `target`, fired by [`FIRER`].
    fn seeking(seeker: u16, target: Option<ShipRef>) -> Shot {
        let weapon = WeaponSpec {
            seeker,
            ..missile(0)
        };
        Shot {
            aimed_at: target,
            ..homing(weapon, target)
        }
    }

    /// `shot` flown `ticks` ticks among `targets` and `sights`, at
    /// `frame`, its draws on `chance`.
    fn guided(
        shot: &mut Shot,
        ticks: u32,
        (targets, sights): (&[Target], &[Sight]),
        chance: &mut dyn Chance,
    ) {
        let guide = Guide {
            sights,
            ..Guide::among(targets)
        };
        for _ in 0..ticks {
            shot.step(&guide, chance);
        }
    }

    /// The target straight down, 90 degrees off a missile heading right.
    fn below() -> Target {
        target(2, 0.0, 1000.0, Condition::Intact)
    }

    /// Ship 2 jamming `jamming`.
    fn jamming(jamming: [i16; 4]) -> Sight {
        Sight {
            jamming: Jamming(jamming),
            ..Sight::of(TARGET)
        }
    }

    #[test]
    fn a_jammed_missile_stops_turning_from_its_15th_tick_and_one_not_jammed_turns() {
        let jammer = [jamming([30, 0, 0, 0])];
        for (rolls, heading, why) in [
            ([71, 0, 0, 0], 90.0, "71 against 30"),
            ([70, 0, 0, 0], 97.0, "exactly 100 less 30 is not jammed"),
            ([0, 0, 0, 0], 97.0, "no roll"),
        ] {
            let mut shot = Shot {
                jam_rolls: rolls,
                ..seeking(0, Some(TARGET))
            };
            guided(&mut shot, 16, (&[below()], &jammer), &mut NeverFires);
            assert!((shot.heading - heading).abs() < 1e-4, "{why}: {shot:?}");
            assert_eq!(shot.jam, Jam::Clear, "{why}: jammed sets no jam");
            assert_eq!((shot.target, shot.aimed_at), (Some(TARGET), Some(TARGET)));
        }
        let mut over = Shot {
            jam_rolls: [101, 0, 0, 0],
            ..seeking(0, Some(TARGET))
        };
        guided(&mut over, 16, (&[below()], &[]), &mut NeverFires);
        assert_eq!(over.heading, 90.0, "a roll above 100 jams a target of none");
    }

    #[test]
    fn each_types_roll_is_tested_against_its_own_jamming() {
        let jammer = [jamming([100, 40, 0, 0])];
        let mut second = Shot {
            jam_rolls: [0, 61, 0, 0],
            ..seeking(0, Some(TARGET))
        };
        guided(&mut second, 16, (&[below()], &jammer), &mut NeverFires);
        assert_eq!(second.heading, 90.0);
        let mut short = Shot {
            jam_rolls: [0, 60, 0, 0],
            ..seeking(0, Some(TARGET))
        };
        guided(&mut short, 16, (&[below()], &jammer), &mut NeverFires);
        assert!((short.heading - 97.0).abs() < 1e-4, "type 1's 100 ignored");
    }

    #[test]
    fn a_jammed_0x0010_missile_turns_away_by_its_turn() {
        let jammer = [jamming([30, 0, 0, 0])];
        let mut shot = Shot {
            jam_rolls: [71, 0, 0, 0],
            ..seeking(TURNS_AWAY_JAMMED, Some(TARGET))
        };
        guided(&mut shot, 17, (&[below()], &jammer), &mut NeverFires);
        assert!((shot.heading - 76.0).abs() < 1e-4, "{shot:?}");
        assert!(close(shot.velocity, facing(76.0) * 10.0));
    }

    #[test]
    fn a_disabled_or_breaking_up_target_jams_nothing() {
        let disabled = [Sight {
            disabled: true,
            ..jamming([100, 100, 100, 100])
        }];
        let mut shot = Shot {
            jam_rolls: [100, 100, 100, 100],
            ..seeking(0, Some(TARGET))
        };
        guided(&mut shot, 16, (&[below()], &disabled), &mut NeverFires);
        assert!((shot.heading - 97.0).abs() < 1e-4, "{shot:?}");
    }

    #[test]
    fn the_jam_test_waits_its_15_ticks_and_asks_its_odds_only_when_jammed() {
        let jammer = [jamming([30, 0, 0, 0])];
        let mut chance = Ranged::of(&[(500, 0)]);
        let mut shot = Shot {
            jam_rolls: [71, 0, 0, 0],
            ..seeking(TURNS_ON_FIRER, Some(TARGET))
        };
        guided(&mut shot, 15, (&[below()], &jammer), &mut chance);
        assert!(!chance.was_asked(500), "before its 15 ticks");
        assert_eq!(shot.target, Some(TARGET));
        let mut clear = Shot {
            jam_rolls: [70, 0, 0, 0],
            ..seeking(TURNS_ON_FIRER, Some(TARGET))
        };
        let mut unjammed = Ranged::of(&[(500, 0)]);
        guided(&mut clear, 20, (&[below()], &jammer), &mut unjammed);
        assert!(!unjammed.was_asked(500), "not jammed");
        let mut plain = Shot {
            jam_rolls: [71, 0, 0, 0],
            ..seeking(0, Some(TARGET))
        };
        let mut never = Ranged::of(&[(500, 0)]);
        guided(&mut plain, 20, (&[below()], &jammer), &mut never);
        assert!(!never.was_asked(500), "no 0x8000");
        assert_eq!(plain.target, Some(TARGET));
    }

    #[test]
    fn a_jammed_0x8000_missile_turns_on_its_firer_at_1_in_500() {
        let jammer = [jamming([30, 0, 0, 0])];
        // The firer sits above the missile's 16th-tick position.
        let firer = Target {
            ship: FIRER,
            ..target(1, 160.0, -1000.0, Condition::Intact)
        };
        let ships = [below(), firer];
        let mut shot = Shot {
            jam_rolls: [71, 0, 0, 0],
            ..seeking(TURNS_ON_FIRER, Some(TARGET))
        };
        let mut chance = Ranged::of(&[(500, 0)]);
        guided(&mut shot, 16, (&ships, &jammer), &mut chance);
        assert!(chance.was_asked(500));
        assert_eq!(
            (shot.target, shot.aimed_at, shot.source),
            (Some(FIRER), Some(FIRER), Source::Other)
        );
        assert_eq!(shot.heading, 90.0, "its jammed turn, from the old bearing");
        guided(&mut shot, 1, (&ships, &jammer), &mut chance);
        assert!(
            (shot.heading - 83.0).abs() < 1e-4,
            "then at its firer: {shot:?}"
        );
        let mut kept = Shot {
            jam_rolls: [71, 0, 0, 0],
            ..seeking(TURNS_ON_FIRER, Some(TARGET))
        };
        guided(
            &mut kept,
            16,
            (&ships, &jammer),
            &mut Ranged::of(&[(500, 1)]),
        );
        assert_eq!(
            (kept.target, kept.source),
            (Some(TARGET), Source::Ship(Ties::default()))
        );
        let mut other = Shot {
            jam_rolls: [71, 0, 0, 0],
            source: Source::Other,
            ..seeking(TURNS_ON_FIRER, Some(TARGET))
        };
        guided(
            &mut other,
            16,
            (&ships, &jammer),
            &mut Ranged::of(&[(500, 0)]),
        );
        assert_eq!(other.target, Some(TARGET), "no ship launched it");
    }

    #[test]
    fn a_missile_turned_on_its_firer_hits_it_and_no_other() {
        let shot = Shot {
            target: Some(FIRER),
            aimed_at: Some(FIRER),
            source: Source::Other,
            ..passing_homing(0, Some(FIRER))
        };
        let firer = Target {
            ship: FIRER,
            ..target(1, 60.0, 0.0, Condition::Intact)
        };
        let path = [target(3, 30.0, 0.0, Condition::Intact), firer];
        assert_eq!(
            shot.hit(Vec2::ZERO, &among(&path), ProximityFuse::Swept)
                .map(|met| met.ship),
            Some(FIRER)
        );
    }

    /// Ship 2 cloaked.
    fn cloaked() -> Sight {
        Sight {
            cloaked: true,
            ..Sight::of(TARGET)
        }
    }

    #[test]
    fn a_missile_stops_turning_while_its_firer_cannot_see_its_target() {
        let mut shot = seeking(0, Some(TARGET));
        guided(&mut shot, 17, (&[below()], &[cloaked()]), &mut NeverFires);
        assert_eq!(shot.heading, 90.0, "cloaked, no scanner");
        assert!(close(shot.velocity, facing(90.0) * 10.0));
        guided(&mut shot, 1, (&[below()], &[]), &mut NeverFires);
        assert!((shot.heading - 97.0).abs() < 1e-4, "uncloaked: {shot:?}");
        let emerging = [Sight {
            emerging: true,
            ..Sight::of(TARGET)
        }];
        let mut gated = seeking(0, Some(TARGET));
        guided(&mut gated, 16, (&[below()], &emerging), &mut NeverFires);
        assert_eq!(gated.heading, 90.0, "coming out of a gate");
    }

    #[test]
    fn a_firer_with_a_cloak_scanner_sees_a_cloaked_target_within_200_pixels_on_each_axis() {
        let scanner = Sight {
            scanner: true,
            ..Sight::of(FIRER)
        };
        for (y, seen) in [(800.0, true), (799.0, false)] {
            let firer = Target {
                ship: FIRER,
                ..target(1, 200.0, y, Condition::Intact)
            };
            let mut shot = seeking(0, Some(TARGET));
            guided(
                &mut shot,
                16,
                (&[below(), firer], &[cloaked(), scanner]),
                &mut NeverFires,
            );
            assert_eq!(shot.heading > 90.0, seen, "{y}: {shot:?}");
        }
    }

    #[test]
    fn an_escort_with_a_cloak_sees_its_cloaked_leader() {
        let escort = Target {
            ship: FIRER,
            ties: Ties {
                leader: Some(TARGET),
                ..Ties::default()
            },
            ..target(1, 0.0, -5000.0, Condition::Intact)
        };
        for (has_cloak, seen) in [(true, true), (false, false)] {
            let gear = Sight {
                has_cloak,
                ..Sight::of(FIRER)
            };
            let mut shot = seeking(0, Some(TARGET));
            guided(
                &mut shot,
                16,
                (&[below(), escort], &[cloaked(), gear]),
                &mut NeverFires,
            );
            assert_eq!(shot.heading > 90.0, seen, "{has_cloak}");
        }
    }

    #[test]
    fn a_blinded_0x8000_missile_turns_on_its_firer_at_1_in_1000() {
        let mut shot = seeking(TURNS_ON_FIRER, Some(TARGET));
        let mut chance = Ranged::of(&[(1000, 0)]);
        guided(&mut shot, 16, (&[below()], &[cloaked()]), &mut chance);
        assert_eq!(
            (shot.target, shot.aimed_at, shot.source),
            (Some(FIRER), Some(FIRER), Source::Other)
        );
        assert_eq!(shot.heading, 90.0, "no turn that tick");
        let mut kept = seeking(TURNS_ON_FIRER, Some(TARGET));
        guided(
            &mut kept,
            16,
            (&[below()], &[cloaked()]),
            &mut Ranged::of(&[(1000, 1)]),
        );
        assert_eq!(kept.target, Some(TARGET));
        let mut seen = seeking(TURNS_ON_FIRER, Some(TARGET));
        let mut asked = Ranged::of(&[(1000, 0)]);
        guided(&mut seen, 16, (&[below()], &[]), &mut asked);
        assert!(!asked.was_asked(1000), "seen: no odds asked");
    }

    #[test]
    fn a_missile_its_jamming_turned_on_its_firer_skips_the_cloak_test_that_tick() {
        let blinded_jammer = [Sight {
            jamming: Jamming([30, 0, 0, 0]),
            ..cloaked()
        }];
        let mut shot = Shot {
            jam_rolls: [71, 0, 0, 0],
            ..seeking(TURNS_ON_FIRER, Some(TARGET))
        };
        let mut chance = Ranged::of(&[(500, 0), (1000, 0)]);
        guided(&mut shot, 16, (&[below()], &blinded_jammer), &mut chance);
        assert_eq!(shot.source, Source::Other);
        assert!(!chance.was_asked(1000), "{:?}", chance.asked);
    }

    #[test]
    fn an_unseen_target_stops_a_turn_away_too() {
        let blinded_jammer = [Sight {
            jamming: Jamming([30, 0, 0, 0]),
            ..cloaked()
        }];
        let mut shot = Shot {
            jam_rolls: [71, 0, 0, 0],
            ..seeking(TURNS_AWAY_JAMMED, Some(TARGET))
        };
        guided(
            &mut shot,
            16,
            (&[below()], &blinded_jammer),
            &mut NeverFires,
        );
        assert_eq!(shot.heading, 90.0);
    }

    /// A 0x4000 missile past its 15 ticks at (150, 0) heading `heading`,
    /// at ship 2 `at` off it.
    fn locked(heading: f32, seeker: u16) -> Shot {
        Shot {
            age: HOMING_DELAY,
            position: Vec2::new(150.0, 0.0),
            heading,
            ..seeking(LOSES_LOCK_OFF_AHEAD | seeker, Some(TARGET))
        }
    }

    /// Ship 2 at `bearing` from (150, 0), `distance` away.
    fn bearing_off(bearing: f32, distance: f32) -> Target {
        let at = Vec2::new(150.0, 0.0) + facing(bearing) * distance;
        target(2, at.x, at.y, Condition::Intact)
    }

    #[test]
    fn a_0x4000_missile_drops_its_lock_on_a_target_behind_it_in_the_box() {
        let behind = [target(2, -50.0, 0.0, Condition::Intact)];
        let mut shot = locked(90.0, 0);
        guided(&mut shot, 1, (&behind, &[]), &mut NeverFires);
        assert_eq!((shot.target, shot.aimed_at), (None, None));
        assert!(
            (shot.heading - 97.0).abs() < 1e-4,
            "it still turns at it that tick: {shot:?}"
        );
        guided(&mut shot, 5, (&behind, &[]), &mut NeverFires);
        assert!((shot.heading - 97.0).abs() < 1e-4, "then straight");
        let at = Shot {
            position: Vec2::new(-50.0, 0.0),
            ..shot
        };
        assert_eq!(
            at.hit(Vec2::new(-60.0, 0.0), &among(&behind), ProximityFuse::Swept),
            None
        );
        let mut young = Shot {
            age: HOMING_DELAY - 1,
            ..locked(90.0, 0)
        };
        guided(&mut young, 1, (&behind, &[]), &mut NeverFires);
        assert_eq!(young.target, Some(TARGET), "before its 15 ticks");
        let mut plain = Shot {
            weapon: missile(0),
            ..locked(90.0, 0)
        };
        guided(&mut plain, 1, (&behind, &[]), &mut NeverFires);
        assert_eq!(plain.target, Some(TARGET), "no 0x4000");
    }

    #[test]
    fn the_lock_box_is_249_pixels_on_each_axis_truncated() {
        for (x, y, kept) in [
            (150.0 - 250.0, 0.0, true),
            (150.0 - 249.0, 0.0, false),
            (150.0 - 249.9, 0.0, false),
            (150.0 + 249.9, 249.9, false),
            (150.0 - 100.0, -250.0, true),
            (150.0 - 100.0, 250.0, true),
        ] {
            let mut shot = locked(0.0, 0);
            let there = [target(2, x, y, Condition::Intact)];
            guided(&mut shot, 1, (&there, &[]), &mut NeverFires);
            assert_eq!(shot.target.is_some(), kept, "({x}, {y})");
        }
    }

    #[test]
    fn the_lock_holds_within_45_degrees_of_ahead() {
        // The table's bearings: 135.0 reads 134 and 135.5 reads 136 (no
        // whole degree reads 135 there), 45.0 reads 46 and 44.5 reads 44.
        for (bearing, kept) in [(135.0, true), (135.5, false), (45.0, true), (44.5, false)] {
            let mut shot = locked(90.0, 0);
            guided(
                &mut shot,
                1,
                (&[bearing_off(bearing, 100.0)], &[]),
                &mut NeverFires,
            );
            assert_eq!(shot.target.is_some(), kept, "{bearing}");
        }
    }

    #[test]
    fn by_the_engine_the_lock_angle_is_not_the_short_way_round() {
        let ahead = [bearing_off(5.0, 100.0)];
        for (lock_angle, kept) in [(LockAngle::Shortest, true), (LockAngle::Plain, false)] {
            let mut shot = locked(355.0, 0);
            let guide = Guide {
                lock_angle,
                ..Guide::among(&ahead)
            };
            shot.step(&guide, &mut NeverFires);
            assert_eq!(shot.target.is_some(), kept, "{lock_angle:?}");
        }
        assert_eq!(LockAngle::default(), LockAngle::Shortest);
        assert_eq!(LockAngle::from(RuleSource::Engine), LockAngle::Plain);
        assert_eq!(LockAngle::from(RuleSource::Bible), LockAngle::Shortest);
    }

    #[test]
    fn the_lock_is_tested_on_the_firer_a_jammed_missile_turned_on() {
        let jammer = [jamming([30, 0, 0, 0])];
        let firer = Target {
            ship: FIRER,
            ..target(1, -50.0, 0.0, Condition::Intact)
        };
        let mut shot = Shot {
            jam_rolls: [71, 0, 0, 0],
            ..locked(90.0, TURNS_ON_FIRER)
        };
        guided(
            &mut shot,
            1,
            (&[bearing_off(90.0, 100.0), firer], &jammer),
            &mut Ranged::of(&[(500, 0)]),
        );
        assert_eq!(
            (shot.target, shot.aimed_at, shot.source),
            (None, None, Source::Other)
        );
    }

    /// An interfered missile `age` ticks old.
    fn interfered(seeker: u16, age: u32) -> Shot {
        Shot {
            jam: Jam::Interfered,
            age,
            ..seeking(seeker, Some(TARGET))
        }
    }

    /// `shot` flown a tick at `frame` among `targets`, its draws on
    /// `chance`.
    fn at_frame(shot: &mut Shot, frame: i32, targets: &[Target], chance: &mut dyn Chance) {
        let guide = Guide {
            frame,
            ..Guide::among(targets)
        };
        shot.step(&guide, chance);
    }

    #[test]
    fn an_interfered_missile_flies_straight_for_15_ticks_then_circles_with_the_frame() {
        let mut young = interfered(0, 0);
        for frame in 0..15 {
            at_frame(&mut young, frame, &[below()], &mut NeverFires);
        }
        assert_eq!(young.heading, 90.0);
        for (frame, heading) in [
            (149, 83.0),
            (150, 97.0),
            (299, 97.0),
            (300, 83.0),
            (-14, 83.0),
            (-1, 83.0),
            (1024, 83.0),
            (0, 83.0),
            (151, 97.0),
        ] {
            let mut shot = interfered(0, HOMING_DELAY);
            at_frame(&mut shot, frame, &[below()], &mut NeverFires);
            assert!((shot.heading - heading).abs() < 1e-4, "{frame}: {shot:?}");
            assert!(close(shot.velocity, facing(heading) * 10.0), "{frame}");
            assert_eq!(shot.jam, Jam::Interfered, "{frame}");
        }
        let mut round = interfered(0, HOMING_DELAY);
        round.heading = 3.0;
        at_frame(&mut round, 0, &[below()], &mut NeverFires);
        assert!((round.heading - 356.0).abs() < 1e-4, "wrapped: {round:?}");
    }

    #[test]
    fn an_interfered_missile_hits_nothing_its_target_included() {
        let shot = Shot {
            jam: Jam::Interfered,
            ..passing_homing(0x0008, Some(TARGET))
        };
        assert_eq!(
            shot.hit(Vec2::ZERO, &among(&in_its_path()), ProximityFuse::Swept),
            None
        );
    }

    #[test]
    fn an_interfered_0x8000_missile_turns_on_its_firer_at_1_in_1000_from_launch() {
        for age in [0, HOMING_DELAY] {
            let mut shot = interfered(TURNS_ON_FIRER, age);
            let mut chance = Ranged::of(&[(1000, 0)]);
            at_frame(&mut shot, 0, &[below()], &mut chance);
            assert_eq!(
                (shot.jam, shot.target, shot.aimed_at, shot.source),
                (Jam::Clear, Some(FIRER), Some(FIRER), Source::Other),
                "{age}"
            );
        }
        let mut kept = interfered(TURNS_ON_FIRER, 3);
        at_frame(&mut kept, 0, &[below()], &mut Ranged::of(&[(1000, 1)]));
        assert_eq!(kept.jam, Jam::Interfered);
        let mut other = Shot {
            source: Source::Other,
            ..interfered(TURNS_ON_FIRER, 3)
        };
        at_frame(&mut other, 0, &[below()], &mut Ranged::of(&[(1000, 0)]));
        assert_eq!((other.jam, other.target), (Jam::Interfered, Some(TARGET)));
        let mut plain = interfered(0, 3);
        let mut never = Ranged::of(&[(1000, 0)]);
        at_frame(&mut plain, 0, &[below()], &mut never);
        assert!(!never.was_asked(1000));
    }

    #[test]
    fn a_shots_velocity_goes_along_its_truncated_heading() {
        let from = state(0.0, 0.0, 1.0, -2.0, 0.0);
        let blaster = spec(-1, 1500, 13);
        let velocity = launch_velocity(&blaster, &from, 45.9, BombLaunch::Thrown);
        assert_eq!(velocity, from.velocity + trig::accel(45, 15.0));
        let homing = spec(1, 1500, 13);
        let velocity = launch_velocity(&homing, &from, 359.99, BombLaunch::Thrown);
        assert_eq!(velocity, trig::accel(359, 15.0), "from rest");
        let shot = Shot::launch(blaster, FIRER, FIRER, &from, 45.9);
        assert_eq!(shot.velocity, from.velocity + trig::accel(45, 15.0));
    }

    #[test]
    fn a_rockets_push_goes_along_its_truncated_heading_rounded_as_the_original() {
        let mut rocket = Shot::launch(
            spec(6, 1000, 120),
            FIRER,
            FIRER,
            &state(0.0, 0.0, 3.0, 1.0, 0.0),
            45.7,
        );
        step(&mut rocket, &[]);
        let push = trig::accel(45, 10.0);
        let mixed = |v: f32, p: f32| (f64::from(v * 95.0 + p * 5.0) * 0.01) as f32;
        assert_eq!(
            rocket.velocity,
            Vec2::new(mixed(3.0, push.x), mixed(1.0, push.y))
        );
    }

    #[test]
    fn a_homing_shot_steers_at_the_tables_bearing_against_its_truncated_heading() {
        // Ship 2 is 91.43 degrees off; the table reads 91. A turn of 0.9
        // truncates to 0, so a whole degree off is enough to turn.
        let ahead = [target(2, 1000.0, 25.0, Condition::Intact)];
        let mut shot = Shot {
            age: HOMING_DELAY,
            heading: 90.9,
            ..homing(
                WeaponSpec {
                    turn: 0.9,
                    ..missile(0)
                },
                Some(TARGET),
            )
        };
        step(&mut shot, &ahead);
        assert!((shot.heading - 91.8).abs() < 1e-4, "{shot:?}");
        assert_eq!(shot.velocity, trig::accel(91, 10.0));
        // On the bearing's whole degree, it holds.
        let mut on = Shot {
            age: HOMING_DELAY,
            heading: 91.95,
            ..homing(missile(0), Some(TARGET))
        };
        step(&mut on, &ahead);
        assert_eq!(on.heading, 91.95);
        assert_eq!(on.velocity, trig::accel(91, 10.0));
    }

    #[test]
    fn a_homing_shot_turns_on_at_half_round_and_back_past_it() {
        // Straight left of a shot heading right: 180 off, turned on.
        let left = [target(2, -1000.0, 0.0, Condition::Intact)];
        let mut shot = Shot {
            age: HOMING_DELAY,
            ..homing(missile(0), Some(TARGET))
        };
        step(&mut shot, &left);
        assert!((shot.heading - 97.0).abs() < 1e-4, "{shot:?}");
        // 181 on (the table's 271 from 90) is back.
        let mut back = Shot {
            age: HOMING_DELAY,
            heading: 90.0,
            ..homing(missile(0), Some(TARGET))
        };
        let past = [target(2, -1000.0, -20.0, Condition::Intact)];
        assert_eq!(trig::intercept_angle(Vec2::ZERO, past[0].position), 271);
        step(&mut back, &past);
        assert!((back.heading - 83.0).abs() < 1e-4, "{back:?}");
    }

    #[test]
    fn a_homing_turn_takes_its_16_bit_difference_round_once_each_way() {
        // A heading below 0: 355 - (-6) is 361, taken to 1, so on; a turn
        // of 0.5 truncates to 0, under the difference's 1.
        assert_eq!(homing_turn(-6.0, 355, 0.5), -5.5);
        // 359 - (-182) is 541, taken to 181, so back.
        assert_eq!(homing_turn(-182.0, 359, 7.0), -189.0);
        // 10 - 350 is -340, taken to 20, so on.
        assert_eq!(homing_turn(350.0, 10, 7.0), 357.0);
        assert_eq!(wrap_turned(0.0), 0.0, "0 stays 0");
        assert_eq!(wrap_turned(360.0), 0.0);
        assert_eq!(wrap_turned(-1.0), 359.0);
    }

    #[test]
    fn a_homing_shots_turn_wraps_its_heading_once() {
        let right = [target(2, 1000.0, 0.0, Condition::Intact)];
        let mut shot = Shot {
            age: HOMING_DELAY,
            heading: 358.5,
            ..homing(missile(0), Some(TARGET))
        };
        step(&mut shot, &right);
        assert!((shot.heading - 5.5).abs() < 1e-4, "{shot:?}");
        assert_eq!(shot.velocity, trig::accel(5, 10.0));
    }

    #[test]
    fn the_lock_angle_is_the_tables_bearing_less_the_heading_truncated() {
        // 135.6 degrees by float, 136 by the table: 46 off a heading of 90.
        let near = [target(2, 290.0, 143.0, Condition::Intact)];
        let mut shot = locked(90.0, 0);
        step(&mut shot, &near);
        assert_eq!((shot.target, shot.aimed_at), (None, None), "lost");
    }

    #[test]
    fn an_interfered_missiles_velocity_goes_along_its_truncated_heading() {
        let mut shot = interfered(0, HOMING_DELAY);
        shot.heading = 90.5;
        at_frame(&mut shot, 0, &[below()], &mut NeverFires);
        assert!((shot.heading - 83.5).abs() < 1e-4, "{shot:?}");
        assert_eq!(shot.velocity, trig::accel(83, 10.0));
        let mut young = interfered(0, 3);
        young.heading = 45.5;
        at_frame(&mut young, 0, &[below()], &mut NeverFires);
        assert_eq!(young.velocity, trig::accel(45, 10.0), "before it circles");
    }

    #[test]
    fn the_nearest_contact_is_the_first_met_and_a_tie_goes_to_the_earlier() {
        let (was, now) = (Vec2::ZERO, Vec2::new(40.0, 0.0));
        let circle = |name, x, r| (name, Vec2::new(x, 0.0), r);
        assert_eq!(
            nearest_contact(was, now, [circle('b', 30.0, 5.0), circle('a', 20.0, 5.0)]),
            Some(('a', Vec2::new(15.0, 0.0))),
            "the nearest along the way, whatever the order"
        );
        assert_eq!(
            nearest_contact(was, now, [circle('b', 20.0, 5.0), circle('a', 20.0, 5.0)]),
            Some(('b', Vec2::new(15.0, 0.0))),
            "a tie goes to the earlier"
        );
        assert_eq!(
            nearest_contact(was, now, [circle('b', 30.0, 5.0), circle('a', 2.0, 5.0)]),
            Some(('a', was)),
            "one it starts within, met where it was"
        );
        assert_eq!(
            nearest_contact(was, now, [circle('a', 60.0, 5.0)]),
            None,
            "none on its way"
        );
    }
}
