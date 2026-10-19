//! A ship's hull in combat: how big a target it is, when it is disabled,
//! and how it dies.
//!
//! - Its hit radius is a third of its `shän`'s `BaseXSize`
//!   ([`HIT_RADIUS_PER_SIZE`]): the half of the original beam test's
//!   0.66 x sprite width (`_HandleBeams`) that a beam's reach adds. The
//!   original tests shots against sprite masks, which the simulation does
//!   not have. A ship type without a `shän` is a placeholder
//!   [`DEFAULT_HIT_RADIUS`] across.
//! - Its board reach, how far off its centre the player can board it, is
//!   half its `shän`'s `BaseXSize` on the x axis and half its `BaseYSize`
//!   on the y axis ([`BOARD_REACH_PER_SIZE`], 0.5 @0xdd128 x the sprite's
//!   width and height, `_GetSpriteXSize` @0x26d4 and `_GetSpriteYSize`
//!   @0x2703, in `_HandlePlayerBoardAttempt`). Those read the loaded
//!   sprite's frame, which is the `shän`'s sizes for every stock ship; a
//!   missing or non-positive size reads as [`FALLBACK_SPRITE_SIZE`], as
//!   they return for a sprite without a frame (@0x26fc, @0x272b).
//! - A ship is disabled as a [`DisableRule`] says. Nova's,
//!   [`NovaDisable`], is `_IsDisabled`'s.
//! - Its `Flags` 0x1000, 0x2000 and 0x4000 ([`BLIND_SPOTS`]) blind its
//!   turrets in front, to the sides and to the rear, as a weapon's own do
//!   (`_TurretBlindSpot`, which reads them from the `shïp`).
//! - Once its armour is gone it breaks up (its `Explode1` going off),
//!   counting down a timer of its `DeathDelay`, three times it for the
//!   player ([`PLAYER_BREAKUP_FACTOR`]), to [`EXPLOSION_BAND`], then is
//!   destroyed (its `Explode2`); see [`Breakup`] for a timer that never
//!   rises above 0. A
//!   ship of [`DEATH_SIZE_MASS`] tons or more dies in an explosion sized
//!   by its mass ([`HullSpec::death_size`], `_HandleShipDisplay`
//!   @0x2d7ef-0x2dee7), which scatters the `Explode2`'s extra explosions
//!   when it has them and reaches as far as its shockwave (see
//!   [`shockwave`](super::shockwave)); the Bible's huge explosion for a
//!   `DeathDelay` of 60 or more is not what the executable does.
//! - Its `Flags` 0x0400 ([`PLANET_TYPE`]) makes it a planet-type ship: an
//!   NPC's missile flies at it only from a planet-type weapon, and at any
//!   other ship only from any other weapon (`_AIFireMissile`
//!   @0x812b4-0x812fb, see `nova_sim::ai::fire`). It dies in an
//!   explosion of no size and makes no shockwave (`_HandleShipDisplay`
//!   tests the flag @0x2d90d), and nothing pushes it, neither a hit nor
//!   a shockwave (`_DamageShip` @0x3aacb), nor holds it in a tractor beam
//!   (`_HandleBeams` @0x31210). The original's explosion size for one is a stale stack
//!   value (copied @0x2d900 from a local only 0x2d968 writes); the engine
//!   reads it as none.
//! - Its turret miss class ([`HullSpec::miss_class`], [`miss_class`]) is
//!   80 for a Fighter-class ship, 90 for a Medium one and 100 for any
//!   other, from its escort class: its turret and point-defence shots roll
//!   1 to it, and a roll above it misses it while it is intact (see
//!   [`sides`](super::sides)).
//! - Its `KeyCarried` ([`HullSpec::key_carried`]) is the ship a weapon of
//!   `Flags2` 0x0080 needs aboard (`_WeaponHasAmmo` @0xba07-0xba85, see
//!   [`Armament::has_ammo`](super::armament::Armament::has_ammo)); none
//!   below a `shïp` ID, as the original keeps -1 below 128 (@0x7a64b-0x7a67d).

use std::fmt::Debug;

use super::exits::ExitPoints;
use super::ship_frames::FrameSpec;
use super::weapon::Explosion;
use crate::bay::FIRST_SHIP;
use crate::catalog::{EscortClass, HullRecord, ShipId};
use crate::geometry::Vec2;
use crate::gravity::STELLAR_IMMUNE_HULL;
use crate::handling::SPEED_PER_PIXEL_PER_TICK;
use crate::reserves::Gauge;
use crate::rulebook::RuleSource;

/// The escape pod's ship class, `shïp` 895 "Escape Pod": the original's
/// class index 0x2ff (the ID less 128), which `_EnterGameFromMainScreen`
/// tests (@0x1a4c5) to spare a strict pilot who ejected in it. Only the
/// pod branch of the player's eject sets it (@0x6f64a), but data may name
/// it for any NPC, and a hail counts a target of it mute (@0x62145). It
/// never breaks up (`_HandleShipDisplay` @0x2d803), and it and every class
/// after it are never damaged ([`immune`]).
pub const ESCAPE_POD_SHIP: ShipId = ShipId(895);

/// Whether ships of class `ship` take no damage: the [`ESCAPE_POD_SHIP`]
/// and every class after it, as `_DamageShip`'s prologue returns at once
/// for a class index above 0x2fe (`cmpw $0x2fe, 0x76` @0x3a87c-0x3a882),
/// before any of its damage, convictions or kill credits.
#[must_use]
pub fn immune(ship: ShipId) -> bool {
    ship >= ESCAPE_POD_SHIP
}

/// Whether ships of class `ship` never break up: the [`ESCAPE_POD_SHIP`]
/// alone, as `_HandleShipDisplay` skips its dying block for class 0x2ff
/// (@0x2d803).
#[must_use]
pub fn never_breaks_up(ship: ShipId) -> bool {
    ship == ESCAPE_POD_SHIP
}

/// The hit radius of a ship type without a `shän`, in pixels: a
/// placeholder.
pub const DEFAULT_HIT_RADIUS: f32 = 16.0;
/// A ship's hit radius per pixel of its `shän`'s `BaseXSize`.
pub const HIT_RADIUS_PER_SIZE: f32 = 0.33;
/// The size, in pixels, `_GetSpriteXSize` and `_GetSpriteYSize` give a
/// sprite without a usable frame (@0x26fc, @0x272b): the board reach of a
/// ship without a `shän` size is half of it on that axis.
pub const FALLBACK_SPRITE_SIZE: f32 = 32.0;
/// A ship's board reach per pixel of its `shän`'s `BaseXSize` (on the x
/// axis) and `BaseYSize` (on the y axis).
pub const BOARD_REACH_PER_SIZE: f32 = 0.5;
/// The `shïp` `Flags` bit that disables it at [`TOUGH_DISABLE_PERCENT`]
/// of its armour instead of [`DISABLE_PERCENT`].
pub const TOUGH: u16 = 0x0010;
/// The `shïp` `Flags` bit that shows its armour on the target panel
/// while its shields are down (`_DrawStatusTarg` @0x4bbaa; see
/// `nova_sim::target_panel`).
pub const SHOWS_ARMOR: u16 = 0x0100;
/// The `shïp` `Flags` bit that hides the target panel's status line
/// (`_DrawStatusTarg` @0x4b941).
pub const HIDES_STATUS: u16 = 0x0200;
/// The percentage of its armour below which a ship is disabled.
pub const DISABLE_PERCENT: f64 = 33.333;
/// The percentage for a ship type with [`TOUGH`] set.
pub const TOUGH_DISABLE_PERCENT: f64 = 10.0;
/// The `shïp` `Flags` bits that blind its turrets, as a `wëap`'s do.
pub const BLIND_SPOTS: u16 = 0x7000;
/// The `shïp` `Flags` bit of a planet-type ship.
pub const PLANET_TYPE: u16 = 0x0400;
/// The `shïp` `Flags2` bit of a ship point defence fires on: "can be
/// fired on by point defense" (class +0x9ea, `_HandleShipPointDefense`
/// @0x3951d; see [`defence`](super::defence)).
pub const PD_TARGET: u16 = 0x0008;
/// The `shïp` `Flags2` bit of a ship that retreats once its weapons run
/// out of ammunition (class +0x9ea, `_WarshipAI` @0x8bf50,
/// `_InterceptorAI` @0x8d358; see `nova_sim::ai::out_of_ammo`).
pub const RETREATS_DRY: u16 = 0x0080;
/// The `shïp` `Flags2` bit of a ship that stands off: it keeps beyond
/// 0.85 of its longest weapon's reach and backs away inside the dogfight
/// box (class +0x9ea, `_HighLevelAIHandler` @0x8e48e and @0x8e6b7; see
/// `nova_sim::ai::stand_off`).
pub const STANDS_OFF: u16 = 0x0002;
/// The `shïp` `Flags3` bit of a ship whose turret shots (4) are drawn
/// over the ships, in 0x3b5308 (class +0x9ec, filled @0x7a204; `_SpawnShot`
/// @0x3f101; see [`ShotLayer`](super::shot_layer::ShotLayer)).
pub const TURRET_SHOTS_OVER: u16 = 0x0040;
/// The `shïp` `Flags` bit of a class whose AI ships always have an
/// afterburner, unless it is planet-type (class +0x9e8, `_AIHasAfterburner`
/// @0x6559; see `nova_sim::ai::afterburner`).
pub const AFTERBURNER_ALWAYS: u16 = 0x0040;
/// The `shïp` `Flags` bit of a class whose AI ships have an afterburner by
/// the player's combat rating (`_AIHasAfterburner` @0x655d-0x6589).
pub const AFTERBURNER_BY_RATING: u16 = 0x0020;
/// The `shïp` `Cost` that hides a ship of the type (@0x2b7f3).
pub const HIDDEN_COST: i32 = -1;
/// The mass, in tons, from which a ship's death explosion has a size.
pub const DEATH_SIZE_MASS: f32 = 100.0;
/// The death explosion's size per ton (the double @0xdd588).
pub const DEATH_SIZE_PER_TON: f32 = 0.075;
/// The death explosion's size before the mass (the double @0xdd590).
pub const DEATH_SIZE_BASE: f32 = 50.0;

/// The turret miss class of a Fighter-class ship (`_LoadObjectData`
/// @0x7a4c4).
pub const FIGHTER_MISS_CLASS: u32 = 80;
/// The turret miss class of a Medium ship (@0x7a4d1).
pub const MEDIUM_MISS_CLASS: u32 = 90;
/// The turret miss class of any heavier class, a Freighter's too
/// (@0x7a4d6).
pub const HEAVY_MISS_CLASS: u32 = 100;

/// The turret miss class of a ship of escort class `class`, as
/// `_LoadObjectData` sets class +0xa10 from its escort type +0xe
/// (@0x7a4bb-0x7a4de): [`FIGHTER_MISS_CLASS`], [`MEDIUM_MISS_CLASS`], or
/// [`HEAVY_MISS_CLASS`] for a Warship or a Freighter. Nothing else writes
/// it, so it is never 0 or less, and `_SpawnShot`'s `Rand(100) + 1` for a
/// class of 0 or less (@0x3e924-0x3e93c) is never reached.
#[must_use]
pub const fn miss_class(class: EscortClass) -> u32 {
    match class {
        EscortClass::Fighter => FIGHTER_MISS_CLASS,
        EscortClass::Medium => MEDIUM_MISS_CLASS,
        EscortClass::Warship | EscortClass::Freighter => HEAVY_MISS_CLASS,
    }
}

/// A ship type's hull, in the simulation's units.
// Each flag is one of the ship type's `Flags` bits, read on its own.
#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HullSpec {
    /// How near, in pixels, a shot must pass its centre to hit it.
    pub hit_radius: f32,
    /// Whether it is disabled at [`TOUGH_DISABLE_PERCENT`].
    pub tough: bool,
    /// Whether the target panel shows its armour while its shields are
    /// down: its `Flags` [`SHOWS_ARMOR`].
    pub shows_armor: bool,
    /// Whether the target panel hides its status line: its `Flags`
    /// [`HIDES_STATUS`].
    pub hides_status: bool,
    /// Ticks it breaks up for before it is destroyed.
    pub death_delay: u32,
    /// Its `DeathDelay` as the data gives it, negative too: the escape
    /// pods' and the escape ship's tests compare the death timer with half
    /// of it (see `nova_sim::npc_escape`), while the breakup floors it
    /// ([`HullSpec::death_delay`]).
    pub signed_death_delay: i32,
    /// The crew pods a dying ship of the type ejects, its `PodCount`
    /// (class +0xa20); none below none.
    pub pod_count: u16,
    /// The explosion as it breaks up, its `Explode1`.
    pub breakup: Option<Explosion>,
    /// The explosion that destroys it, its `Explode2`.
    pub explosion: Option<Explosion>,
    /// Its mass, in tons.
    pub mass: f32,
    /// Where its turrets cannot fire: its `Flags` 0x1000 (front), 0x2000
    /// (sides) and 0x4000 (rear), as a weapon's `Flags` mark them.
    pub blind_spots: u16,
    /// How strong it counts in a fight, its `Strength`; none below none.
    pub strength: f32,
    /// How far off its centre, in pixels, the player can board it: `x` on
    /// the x axis and `y` on the y axis.
    pub board_reach: Vec2,
    /// Its sprite's width, its `shän`'s `BaseXSize`, in pixels; none
    /// without a sprite. A formation is spaced by it (see
    /// `nova_sim::escort::spacing`).
    pub sprite_size: Option<f32>,
    /// Its sprite's height, its `shän`'s `BaseYSize`, in pixels; none
    /// without a sprite. Whether it is on screen reads it with the width
    /// (see [`cloak::on_screen`](crate::cloak::on_screen)).
    pub sprite_height: Option<f32>,
    /// Whether it is a planet-type ship, its `Flags` [`PLANET_TYPE`]: it
    /// dies with no shockwave, and no hit, shockwave or tractor beam
    /// pushes or holds it.
    pub planet_type: bool,
    /// Whether point defence fires on it, its `Flags2` [`PD_TARGET`],
    /// when it fights the defender or its leader (see
    /// [`defence::choose_ship`](super::defence::choose_ship)).
    pub pd_target: bool,
    /// Whether it retreats once its weapons run out of ammunition, its
    /// `Flags2` [`RETREATS_DRY`] (see `nova_sim::ai::out_of_ammo`).
    pub retreats_dry: bool,
    /// Whether it stands off from its target at its weapons' reach, its
    /// `Flags2` [`STANDS_OFF`] (see `nova_sim::ai::stand_off`).
    pub stands_off: bool,
    /// The ship a weapon of `Flags2` 0x0080 needs aboard, its
    /// `KeyCarried`; none below a `shïp` ID.
    pub key_carried: Option<ShipId>,
    /// Its class's top speed, in pixels a tick: its `Speed` alone, none of
    /// its outfits' (the original's class +0x40, `_LoadObjectData`
    /// @0x7a0db-0x7a0ef), as a hopeless chase compares them (see
    /// `nova_sim::ai::odds`); none below none.
    pub class_speed: f32,
    /// How far its class's stock weapons reach, as `_AIInGunRange` asks
    /// with no weapon named (@0x7fa2d-0x7faa8, see
    /// `nova_sim::ai::fire::in_reach`): the longest reach
    /// among its stock slots of a count above none and a raw guidance of 8
    /// or less, a beam (0 and 3) its length and 32, any other its range
    /// ([`WeaponSpec::range`](super::weapon::WeaponSpec::range)) and 32,
    /// truncated; none when it has no such weapon. Its person's and
    /// outfits' weapons, its point defence and its bays never count. An
    /// [`Arsenal`](super::armament::Arsenal) sets it.
    pub stock_reach: Option<f32>,
    /// Whether it is hidden: its `Cost` is -1, and `_HandleShipDisplay`
    /// hides a ship of such a type and leaves its slot's layer as it was
    /// (@0x2b7f3 → @0x2cc6b; see
    /// [`ship_layer`](crate::ship_layer)).
    pub hidden: bool,
    /// Its weapon exit points, from its `shän`; none without one, and its
    /// shots and beams leave from its centre (see
    /// [`exits`](super::exits)).
    pub exits: Option<ExitPoints>,
    /// Its `shän`'s display: what picks the frames it is drawn on and
    /// whether it unfolds to fire (see
    /// [`ship_frames`](super::ship_frames)); [`FrameSpec::NONE`] without
    /// a `shän`.
    pub frames: FrameSpec,
    /// Its turret miss class (the original's class +0xa10,
    /// [`miss_class`]): the range a turret or point-defence shot it fires
    /// rolls in, and the highest roll that hits it while it is intact (see
    /// [`sides`](super::sides)).
    pub miss_class: u32,
    /// Whether its turret shots (4) are drawn over the ships: its `Flags3`
    /// [`TURRET_SHOTS_OVER`] (see
    /// [`ShotLayer::spawned`](super::shot_layer::ShotLayer::spawned)).
    pub turret_shots_over: bool,
    /// Whether neither gravity nor a deadly stellar touches it: its
    /// `Flags3` [`STELLAR_IMMUNE_HULL`] (class +0x9ec; see
    /// [`resists_gravity`](crate::gravity::resists_gravity) and
    /// [`resists_deadly`](crate::deadly_stellar::resists_deadly)).
    pub stellar_immune: bool,
    /// Its `Flags` afterburner bits, [`AFTERBURNER_ALWAYS`] and
    /// [`AFTERBURNER_BY_RATING`], which decide with [`HullSpec::planet_type`]
    /// whether an AI ship of the type has an afterburner (see
    /// `nova_sim::ai::afterburner`).
    pub afterburner: u16,
    /// Its class's `InherentAI` (the original's class +0x12), which a
    /// dogfighter's break-off reads (see `nova_sim::ai::pursuit`). A
    /// [`HullRecord`] doesn't carry it: the spawn table sets it from the
    /// `shïp` (`nova_sim::traffic::table::kind`), and it is none
    /// otherwise.
    pub inherent_ai: i16,
    /// Its class's `Crew` (the original's class +0x9f2), which a pirate's
    /// plunder tests read (see `nova_sim::ai::pirate`): it plunders only a
    /// ship whose class has a crew above none. Like
    /// [`inherent_ai`](Self::inherent_ai), the spawn table sets it from
    /// the `shïp`, and the session for the player's ship; none otherwise.
    pub crew: i16,
    /// Its class's turn, the original's class +0x38: its `Maneuver` × the
    /// double 0.1 as a float (`_LoadObjectData` @0x7a0db-0x7a10b, see
    /// [`class_turn`]). A pirate's docking box reads it (see
    /// `nova_sim::traffic::plunder_dock`). Like [`crew`](Self::crew), the
    /// spawn table sets it; none otherwise.
    pub class_turn: f32,
}

/// A class's turn, the original's class +0x38, from its `Maneuver`: the
/// whole `Maneuver` × the double 0.1 (0xdd110), stored as a float
/// (`_LoadObjectData` @0x7a0db-0x7a10b).
#[must_use]
pub fn class_turn(maneuver: i16) -> f32 {
    // A float, as the original stores it.
    #[allow(clippy::cast_possible_truncation)]
    let turn = (f64::from(maneuver) * 0.1) as f32;
    turn
}

impl Default for HullSpec {
    /// A placeholder hull: [`DEFAULT_HIT_RADIUS`] across, gone at once,
    /// with no explosions.
    fn default() -> Self {
        Self {
            hit_radius: DEFAULT_HIT_RADIUS,
            tough: false,
            shows_armor: false,
            hides_status: false,
            death_delay: 0,
            signed_death_delay: 0,
            pod_count: 0,
            breakup: None,
            explosion: None,
            mass: 0.0,
            blind_spots: 0,
            strength: 0.0,
            board_reach: Vec2::new(
                FALLBACK_SPRITE_SIZE * BOARD_REACH_PER_SIZE,
                FALLBACK_SPRITE_SIZE * BOARD_REACH_PER_SIZE,
            ),
            sprite_size: None,
            sprite_height: None,
            planet_type: false,
            pd_target: false,
            retreats_dry: false,
            stands_off: false,
            key_carried: None,
            class_speed: 0.0,
            stock_reach: None,
            hidden: false,
            exits: None,
            frames: FrameSpec::NONE,
            miss_class: miss_class(EscortClass::Freighter),
            turret_shots_over: false,
            stellar_immune: false,
            afterburner: 0,
            inherent_ai: 0,
            crew: 0,
            class_turn: 0.0,
        }
    }
}

impl HullSpec {
    /// The hull `record` describes (see the module docs).
    #[must_use]
    pub fn new(record: &HullRecord) -> Self {
        let size = record.size.filter(|&size| size > 0).map(f32::from);
        let hit_radius = size.map_or(DEFAULT_HIT_RADIUS, |size| size * HIT_RADIUS_PER_SIZE);
        let height = record.height.filter(|&height| height > 0).map(f32::from);
        let board_reach = Vec2::new(
            size.unwrap_or(FALLBACK_SPRITE_SIZE) * BOARD_REACH_PER_SIZE,
            height.unwrap_or(FALLBACK_SPRITE_SIZE) * BOARD_REACH_PER_SIZE,
        );
        let frames = FrameSpec::new(record.display.as_ref(), record.exits.as_ref());
        Self {
            hit_radius,
            tough: record.flags & TOUGH != 0,
            shows_armor: record.flags & SHOWS_ARMOR != 0,
            hides_status: record.flags & HIDES_STATUS != 0,
            death_delay: u32::try_from(record.death_delay).unwrap_or(0),
            signed_death_delay: i32::from(record.death_delay),
            pod_count: u16::try_from(record.pod_count).unwrap_or(0),
            breakup: Explosion::decode(record.explode1),
            explosion: Explosion::decode(record.explode2),
            mass: f32::from(record.mass.max(0)),
            blind_spots: record.flags & BLIND_SPOTS,
            strength: f32::from(record.strength.max(0)),
            board_reach,
            sprite_size: size,
            sprite_height: height,
            planet_type: record.flags & PLANET_TYPE != 0,
            pd_target: record.flags2 & PD_TARGET != 0,
            retreats_dry: record.flags2 & RETREATS_DRY != 0,
            stands_off: record.flags2 & STANDS_OFF != 0,
            key_carried: record.key_carried.filter(|ship| ship.0 >= FIRST_SHIP),
            class_speed: f32::from(record.speed.max(0)) / SPEED_PER_PIXEL_PER_TICK,
            stock_reach: None,
            hidden: record.cost == HIDDEN_COST,
            exits: record
                .exits
                .as_ref()
                .and_then(ExitPoints::new)
                .map(|exits| exits.with_angle_table(frames.table.is_some())),
            frames,
            miss_class: miss_class(record.escort_class),
            turret_shots_over: record.flags3 & TURRET_SHOTS_OVER != 0,
            stellar_immune: record.flags3 & STELLAR_IMMUNE_HULL != 0,
            afterburner: record.flags & (AFTERBURNER_ALWAYS | AFTERBURNER_BY_RATING),
            inherent_ai: 0,
            crew: 0,
            class_turn: 0.0,
        }
    }

    /// The size of the explosion it dies in: `trunc(mass x 0.075 + 50)`
    /// for a ship of [`DEATH_SIZE_MASS`] tons or more, and none for a
    /// lighter one or a planet-type one ([`HullSpec::planet_type`]). It is
    /// also the reach of the shockwave the explosion makes (see
    /// [`shockwave`](super::shockwave)).
    #[must_use]
    pub fn death_size(&self) -> f32 {
        if self.mass >= DEATH_SIZE_MASS && !self.planet_type {
            self.mass
                .mul_add(DEATH_SIZE_PER_TON, DEATH_SIZE_BASE)
                .trunc()
        } else {
            0.0
        }
    }
}

/// How a ship is holding up.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Condition {
    /// Flying and fighting.
    #[default]
    Intact,
    /// Disabled: it drifts, and cannot fire, land or jump. Further damage
    /// can still destroy it.
    Disabled,
    /// Its armour is gone, and it breaks up, counting down the original's
    /// death timer (field 0x3c) to its final explosion (see
    /// [`Combat`](super::Combat)'s step 9).
    Dying {
        /// The death timer: its `DeathDelay`, times
        /// [`PLAYER_BREAKUP_FACTOR`] for the player, as set on the
        /// breakup's first tick, less a tick for each tick since. The ship
        /// explodes on the tick it comes down to [`EXPLOSION_BAND`] or
        /// below.
        ticks_left: u32,
        /// Whether the timer was set on this tick: the original's "dying,
        /// timer not yet set" frame, on which `_HandleShipDisplay` sets it
        /// (@0x2d834-0x2d850) and does nothing else. False from the next
        /// tick on; under [`Breakup::Stalls`], true again on every tick
        /// the timer is set again. Phase 9's eject reads it, as
        /// `_HandlePlayer` (@0x6f248-0x6f286) runs before
        /// `_HandleShipDisplay` sets the timer.
        starting: bool,
    },
    /// Destroyed.
    Destroyed,
}

impl Condition {
    /// Whether it still stands: intact or disabled, neither breaking up
    /// nor destroyed. A homing shot loses a target that no longer stands,
    /// a turreted beam lets go of it, and the AI no longer aims at it.
    #[must_use]
    pub fn standing(self) -> bool {
        matches!(self, Self::Intact | Self::Disabled)
    }

    /// Whether its slot is active: anything but destroyed, breaking up
    /// included (the original's +0xb8). Shots, beams and blasts reach
    /// every active ship ([`sides`](super::sides)): `_ShotCanHitShip`,
    /// `_HandleBeams`' filter and the blast pass test the slot alone, never
    /// `_IsDying`.
    #[must_use]
    pub fn active(self) -> bool {
        self != Self::Destroyed
    }

    /// Whether it is disabled as the original's `_IsDisabled` reads it:
    /// disabled or breaking up. A pirate plunders only such a ship, and
    /// fires disable-only at one that is not.
    #[must_use]
    pub fn disabled(self) -> bool {
        matches!(self, Self::Disabled | Self::Dying { .. })
    }
}

/// What the player's `DeathDelay` is multiplied by as its breakup starts
/// (the 3.0 @0xdd664, applied @0x2d848 when +0x86 is 0, the player).
pub const PLAYER_BREAKUP_FACTOR: u32 = 3;
/// The death timer at or below which a breaking-up ship makes its final
/// explosion (the 2.0 @0xdd074, tested @0x2d85a).
pub const EXPLOSION_BAND: u32 = 2;

/// What becomes of a breakup whose death timer never rises above 0 by
/// the time `_HandleShipDisplay` checks it: a `DeathDelay` of 0 or less
/// for any ship, or of 1 for an NPC, whose timer `_HandleShip` takes back
/// to 0 before the check. No stock ship has one (the lowest stock
/// `DeathDelay` is 3).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Breakup {
    /// The ship explodes: on its breakup tick with no `DeathDelay`, on the
    /// tick after with 1. The default, and the other reading.
    #[default]
    Explodes,
    /// The original's: `_HandleShipDisplay` sets a timer that is not above
    /// 0 again every frame (@0x2d80d-0x2d83d), and the decrements skip one
    /// that is not (@0x6d7eb, @0x35088), so the ship breaks up for good.
    /// For the player, the game never ends.
    Stalls,
}

impl From<RuleSource> for Breakup {
    /// The engine's reading [`Breakup::Stalls`]; the other
    /// [`Breakup::Explodes`].
    fn from(source: RuleSource) -> Self {
        match source {
            RuleSource::Engine => Self::Stalls,
            RuleSource::Bible => Self::Explodes,
        }
    }
}

/// When a ship is disabled.
pub trait DisableRule: Debug {
    /// Whether a ship of `hull` with `armor` is disabled. The fight asks
    /// it only of a ship that holds some armour: one that holds none is
    /// never disabled.
    fn disabled(&self, armor: Gauge, hull: &HullSpec) -> bool;
}

/// Nova's rule, the default (`_IsDisabled`): a ship is disabled while its
/// armour is below [`DISABLE_PERCENT`] of what it holds, or
/// [`TOUGH_DISABLE_PERCENT`] for a [`TOUGH`] ship type.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NovaDisable;

impl DisableRule for NovaDisable {
    fn disabled(&self, armor: Gauge, hull: &HullSpec) -> bool {
        let percent = if hull.tough {
            TOUGH_DISABLE_PERCENT
        } else {
            DISABLE_PERCENT
        };
        f64::from(armor.now) * 100.0 < f64::from(armor.max) * percent
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::catalog::{BoomId, ShipDisplay, ShipExits, ShipId};
    use crate::combat::exits::ExitPoints;
    use nova_sim_catalog::fixture::hull;

    fn armor(now: f32, max: f32) -> Gauge {
        Gauge { now, max }
    }

    fn disabled(now: f32, max: f32, tough: bool) -> bool {
        let hull = HullSpec {
            tough,
            ..HullSpec::default()
        };
        NovaDisable.disabled(armor(now, max), &hull)
    }

    #[test]
    fn at_exactly_a_third_a_ship_is_not_disabled_and_just_below_it_is() {
        // 33.333% of 100,000 is 33,333.
        assert!(!disabled(33_333.0, 100_000.0, false));
        assert!(disabled(33_332.99, 100_000.0, false));
        assert!(disabled(33_332.0, 100_000.0, false));
        assert!(!disabled(45.0, 45.0, false));
        assert!(!disabled(15.0, 45.0, false), "15 is a third and more");
        assert!(disabled(14.99, 45.0, false));
        assert!(disabled(0.0, 45.0, false));
        assert!(disabled(-3.0, 45.0, false));
    }

    #[test]
    fn a_tough_ship_type_is_disabled_at_a_tenth() {
        assert!(!disabled(10.0, 100.0, true));
        assert!(disabled(9.999, 100.0, true));
        assert!(!disabled(14.99, 45.0, true), "a third does not disable it");
        assert_eq!((DISABLE_PERCENT, TOUGH_DISABLE_PERCENT), (33.333, 10.0));
    }

    #[test]
    fn a_ship_types_turret_shots_go_over_the_ships_with_flags3_0x0040() {
        let over = HullSpec::new(&HullRecord {
            flags3: 0x0140,
            ..hull(143)
        });
        assert!(over.turret_shots_over);
        let under = HullSpec::new(&HullRecord {
            flags3: 0x0100,
            ..hull(141)
        });
        assert!(!under.turret_shots_over);
        assert!(!HullSpec::default().turret_shots_over);
        assert_eq!(TURRET_SHOTS_OVER, 0x0040);
    }

    #[test]
    fn a_ship_type_is_stellar_immune_by_its_flags3_0x0020_alone() {
        let of = |flags3: u16| {
            HullSpec::new(&HullRecord {
                flags3,
                ..hull(140)
            })
            .stellar_immune
        };
        assert!(of(0x0020));
        assert!(of(0xFFFF));
        assert!(!of(0x0040), "the turret shots' bit");
        assert!(!of(0xFFDF), "every other bit");
        assert!(!HullSpec::default().stellar_immune);
    }

    #[test]
    fn a_ship_types_afterburner_bits_are_its_flags_0x0020_and_0x0040() {
        let of = |flags: u16| HullSpec::new(&HullRecord { flags, ..hull(140) }).afterburner;
        assert_eq!(of(0x0160), 0x0060);
        assert_eq!(of(0x0420), 0x0020);
        assert_eq!(of(0x0040), 0x0040);
        assert_eq!(of(0x0110), 0);
        assert_eq!(HullSpec::default().afterburner, 0);
        assert_eq!(
            (AFTERBURNER_ALWAYS, AFTERBURNER_BY_RATING),
            (0x0040, 0x0020)
        );
        assert_eq!(HullSpec::new(&hull(140)).inherent_ai, 0);
        assert_eq!(
            HullSpec::new(&hull(140)).crew,
            0,
            "a hull record carries no crew"
        );
    }

    #[test]
    fn a_ship_that_holds_no_armour_is_never_disabled() {
        assert!(!disabled(0.0, 0.0, false));
        assert!(!disabled(0.0, 0.0, true));
    }

    #[test]
    fn the_hit_radius_is_a_third_of_the_shäns_size() {
        let shuttle = HullSpec::new(&HullRecord {
            size: Some(24),
            ..hull(128)
        });
        assert!((shuttle.hit_radius - 7.92).abs() < 1e-5, "{shuttle:?}");
        assert_eq!(HIT_RADIUS_PER_SIZE, 0.33);
        for size in [None, Some(0), Some(-5)] {
            let placeholder = HullSpec::new(&HullRecord { size, ..hull(128) });
            assert_eq!(placeholder.hit_radius, DEFAULT_HIT_RADIUS, "{size:?}");
        }
        assert_eq!(DEFAULT_HIT_RADIUS, 16.0);
        assert_eq!(HullSpec::default().hit_radius, 16.0);
    }

    #[test]
    fn hull_spec_reads_exit_points_in_simulation_units() {
        let raw = ShipExits {
            points: [
                [(3, 10), (-3, 10), (3, 10), (-3, 10)],
                [(0, 0); 4],
                [(0, 0); 4],
                [(0, 0); 4],
            ],
            z: [[-2; 4], [0; 4], [0; 4], [0; 4]],
            up_compress: (100, 71),
            down_compress: (100, 71),
            frames: 36,
        };
        let shuttle = HullSpec::new(&HullRecord {
            exits: Some(raw),
            ..hull(128)
        });
        assert_eq!(shuttle.exits, ExitPoints::new(&raw));
        assert!(shuttle.exits.is_some());
        let without = HullSpec::new(&HullRecord {
            exits: None,
            ..hull(128)
        });
        assert_eq!(without.exits, None, "no shän");
        assert_eq!(
            HullSpec::default().exits,
            None,
            "a placeholder fires from its centre"
        );
    }

    #[test]
    fn hull_spec_reads_its_shäns_display_and_turns_its_exits_by_its_table() {
        let raw = ShipExits {
            frames: 8,
            up_compress: (100, 100),
            down_compress: (100, 50),
            ..ShipExits::default()
        };
        let display = ShipDisplay {
            flags: 0x01C2,
            base_sets: 6,
            anim_delay: 5,
            ..ShipDisplay::default()
        };
        let miner = HullSpec::new(&HullRecord {
            exits: Some(raw),
            display: Some(display),
            ..hull(379)
        });
        assert_eq!(miner.frames, FrameSpec::new(Some(&display), Some(&raw)));
        assert_eq!(miner.frames.unfolds_to_fire(), Some(6));
        let exits = miner.exits.expect("exits");
        assert_eq!(exits.drawn_frame(125.0), 3, "by the table");
        let plain = HullSpec::new(&HullRecord {
            exits: Some(raw),
            display: Some(ShipDisplay {
                flags: 0x00C2,
                ..display
            }),
            ..hull(379)
        });
        assert_eq!(
            plain.exits.expect("exits").drawn_frame(125.0),
            2,
            "no table"
        );
        assert_eq!(HullSpec::default().frames, FrameSpec::NONE);
        assert_eq!(HullSpec::new(&hull(128)).frames, FrameSpec::NONE, "no shän");
    }

    #[test]
    fn the_sprite_size_is_the_shäns_size() {
        let shuttle = HullSpec::new(&HullRecord {
            size: Some(24),
            ..hull(128)
        });
        assert_eq!(shuttle.sprite_size, Some(24.0));
        for size in [None, Some(0), Some(-5)] {
            let placeholder = HullSpec::new(&HullRecord { size, ..hull(128) });
            assert_eq!(placeholder.sprite_size, None, "{size:?}");
        }
        assert_eq!(HullSpec::default().sprite_size, None);
    }

    #[test]
    fn the_sprite_height_is_the_shäns_height() {
        let tall = HullSpec::new(&HullRecord {
            size: Some(24),
            height: Some(40),
            ..hull(128)
        });
        assert_eq!(tall.sprite_height, Some(40.0));
        assert_eq!(tall.sprite_size, Some(24.0));
        for height in [None, Some(0), Some(-5)] {
            let placeholder = HullSpec::new(&HullRecord {
                height,
                ..hull(128)
            });
            assert_eq!(placeholder.sprite_height, None, "{height:?}");
        }
        assert_eq!(HullSpec::default().sprite_height, None);
    }

    #[test]
    fn the_board_reach_is_half_the_shäns_size_on_each_axis() {
        let tall = HullSpec::new(&HullRecord {
            size: Some(24),
            height: Some(40),
            ..hull(128)
        });
        assert_eq!(tall.board_reach, Vec2::new(12.0, 20.0));
        let shuttle = HullSpec::new(&HullRecord {
            size: Some(25),
            height: Some(25),
            ..hull(128)
        });
        assert_eq!(shuttle.board_reach, Vec2::new(12.5, 12.5));
        assert_eq!(BOARD_REACH_PER_SIZE, 0.5);
        for bad in [None, Some(0), Some(-5)] {
            let no_width = HullSpec::new(&HullRecord {
                size: bad,
                height: Some(40),
                ..hull(128)
            });
            assert_eq!(no_width.board_reach, Vec2::new(16.0, 20.0), "{bad:?}");
            let no_height = HullSpec::new(&HullRecord {
                size: Some(24),
                height: bad,
                ..hull(128)
            });
            assert_eq!(no_height.board_reach, Vec2::new(12.0, 16.0), "{bad:?}");
        }
        assert_eq!(FALLBACK_SPRITE_SIZE, 32.0);
        assert_eq!(HullSpec::default().board_reach, Vec2::new(16.0, 16.0));
    }

    #[test]
    fn a_hull_reads_its_flag_delay_explosions_and_mass() {
        let spec = HullSpec::new(&HullRecord {
            flags: 0x0130,
            death_delay: 60,
            explode1: 4,
            explode2: 1005,
            mass: 120,
            strength: 325,
            ..hull(141)
        });
        assert_eq!(spec.strength, 325.0);
        assert_eq!(HullSpec::default().strength, 0.0);
        assert!(spec.tough);
        assert_eq!(spec.death_delay, 60);
        assert_eq!(
            spec.breakup,
            Some(Explosion {
                boom: BoomId(132),
                extra: false
            })
        );
        assert_eq!(
            spec.explosion,
            Some(Explosion {
                boom: BoomId(133),
                extra: true
            })
        );
        assert_eq!(spec.mass, 120.0);
        let trader = HullSpec::new(&HullRecord {
            flags: 0x0120,
            death_delay: -5,
            mass: -1,
            strength: -2,
            ..hull(128)
        });
        assert!(!trader.tough);
        assert_eq!((trader.death_delay, trader.mass), (0, 0.0));
        assert_eq!(trader.strength, 0.0, "none below none");
        assert_eq!((trader.breakup, trader.explosion), (None, None));
        assert_eq!(TOUGH, 0x0010);
    }

    #[test]
    fn a_hull_reads_its_pod_count_floored_and_its_signed_death_delay() {
        let liner = HullSpec::new(&HullRecord {
            death_delay: 100,
            pod_count: 10,
            ..hull(134)
        });
        assert_eq!((liner.pod_count, liner.signed_death_delay), (10, 100));
        let odd = HullSpec::new(&HullRecord {
            death_delay: -5,
            pod_count: -3,
            ..hull(128)
        });
        assert_eq!(odd.pod_count, 0, "none below none");
        assert_eq!(
            (odd.death_delay, odd.signed_death_delay),
            (0, -5),
            "the breakup's floored, the escape tests' raw"
        );
        assert_eq!(
            (
                HullSpec::default().pod_count,
                HullSpec::default().signed_death_delay
            ),
            (0, 0)
        );
    }

    #[test]
    fn a_hulls_turret_blind_spots_are_its_flags_0x7000() {
        let destroyer = HullSpec::new(&HullRecord {
            flags: 0x4130,
            ..hull(141)
        });
        assert_eq!(destroyer.blind_spots, 0x4000);
        let carrier = HullSpec::new(&HullRecord {
            flags: 0xffff,
            ..hull(222)
        });
        assert_eq!(carrier.blind_spots, 0x7000);
        assert_eq!(HullSpec::new(&hull(128)).blind_spots, 0);
        assert_eq!(HullSpec::default().blind_spots, 0);
        assert_eq!(BLIND_SPOTS, 0x7000);
    }

    #[test]
    fn a_hull_is_planet_type_with_its_flags_0x0400() {
        let planet = HullSpec::new(&HullRecord {
            flags: 0x0410,
            ..hull(141)
        });
        assert!(planet.planet_type);
        assert!(planet.tough, "the other bits still read");
        let ordinary = HullSpec::new(&HullRecord {
            flags: 0xfbff,
            ..hull(141)
        });
        assert!(!ordinary.planet_type);
        assert!(!HullSpec::default().planet_type);
        assert_eq!(PLANET_TYPE, 0x0400);
    }

    #[test]
    fn a_hull_point_defence_fires_on_has_its_flags2_0x0008() {
        let target = |flags2| {
            HullSpec::new(&HullRecord {
                flags2,
                ..hull(167)
            })
            .pd_target
        };
        assert!(target(0x0008));
        assert!(target(0x0089), "with other bits");
        assert!(!target(0xfff7), "every other bit");
        assert!(!target(0), "none");
        let flags = HullSpec::new(&HullRecord {
            flags: 0x0008,
            ..hull(167)
        });
        assert!(!flags.pd_target, "its Flags never");
        assert!(!HullSpec::default().pd_target);
        assert_eq!(PD_TARGET, 0x0008);
    }

    #[test]
    fn a_hull_that_retreats_out_of_ammunition_has_its_flags2_0x0080() {
        let retreats = |flags2| {
            HullSpec::new(&HullRecord {
                flags2,
                ..hull(167)
            })
            .retreats_dry
        };
        assert!(retreats(0x0080));
        assert!(retreats(0x0089), "with other bits");
        assert!(!retreats(0xff7f), "every other bit");
        assert!(!retreats(0), "none");
        let flags = HullSpec::new(&HullRecord {
            flags: 0x0080,
            ..hull(167)
        });
        assert!(!flags.retreats_dry, "its Flags never");
        assert!(!HullSpec::default().retreats_dry);
        assert_eq!(RETREATS_DRY, 0x0080);
    }

    #[test]
    fn a_hull_that_stands_off_has_its_flags2_0x0002() {
        let stands_off = |flags2| {
            HullSpec::new(&HullRecord {
                flags2,
                ..hull(167)
            })
            .stands_off
        };
        assert!(stands_off(0x0002));
        assert!(stands_off(0x0083), "with other bits");
        assert!(!stands_off(0xfffd), "every other bit");
        assert!(!stands_off(0), "none");
        let flags = HullSpec::new(&HullRecord {
            flags: 0x0002,
            ..hull(167)
        });
        assert!(!flags.stands_off, "its Flags never");
        assert!(!HullSpec::default().stands_off);
        assert_eq!(STANDS_OFF, 0x0002);
    }

    #[test]
    fn a_hull_is_hidden_when_its_cost_is_minus_one() {
        let hidden = HullSpec::new(&HullRecord {
            cost: -1,
            ..hull(141)
        });
        assert!(hidden.hidden);
        for cost in [0, -2, 5000] {
            let shown = HullSpec::new(&HullRecord { cost, ..hull(141) });
            assert!(!shown.hidden, "cost {cost}");
        }
        assert!(!HullSpec::default().hidden);
    }

    #[test]
    fn a_hulls_miss_class_is_80_90_or_100_by_its_escort_class() {
        for (escort_class, miss_class) in [
            (EscortClass::Fighter, 80),
            (EscortClass::Medium, 90),
            (EscortClass::Warship, 100),
            (EscortClass::Freighter, 100),
        ] {
            let spec = HullSpec::new(&HullRecord {
                escort_class,
                ..hull(141)
            });
            assert_eq!(spec.miss_class, miss_class, "{escort_class:?}");
        }
        assert_eq!(HullSpec::default().miss_class, 100);
    }

    #[test]
    fn a_hulls_key_carried_ship_is_none_below_128() {
        let carrier = |key| {
            HullSpec::new(&HullRecord {
                key_carried: key,
                ..hull(141)
            })
            .key_carried
        };
        assert_eq!(carrier(Some(ShipId(128))), Some(ShipId(128)));
        assert_eq!(carrier(Some(ShipId(300))), Some(ShipId(300)));
        assert_eq!(carrier(Some(ShipId(127))), None);
        assert_eq!(carrier(Some(ShipId(0))), None);
        assert_eq!(carrier(Some(ShipId(-5))), None);
        assert_eq!(carrier(None), None);
        assert_eq!(HullSpec::default().key_carried, None);
    }

    #[test]
    fn a_hulls_target_panel_flags_are_its_flags_0x0100_and_0x0200() {
        let armour = HullSpec::new(&HullRecord {
            flags: 0x0100,
            ..hull(128)
        });
        assert!(armour.shows_armor);
        assert!(!armour.hides_status);
        let hidden = HullSpec::new(&HullRecord {
            flags: 0x0200,
            ..hull(129)
        });
        assert!(!hidden.shows_armor);
        assert!(hidden.hides_status);
        let neither = HullSpec::new(&HullRecord {
            flags: !0x0300,
            ..hull(130)
        });
        assert!(!neither.shows_armor);
        assert!(!neither.hides_status);
        assert!(!HullSpec::default().shows_armor);
        assert!(!HullSpec::default().hides_status);
        assert_eq!((SHOWS_ARMOR, HIDES_STATUS), (0x0100, 0x0200));
    }

    #[test]
    fn a_class_turn_is_its_maneuver_times_a_tenth_stored_as_a_float() {
        assert_eq!(class_turn(30), 3.0);
        assert_eq!(class_turn(80), 8.0);
        // 3.3000000000000003 in doubles, stored as the float nearest.
        assert_eq!(class_turn(33), 3.3_f32);
        assert_eq!(class_turn(-5), -0.5);
        assert_eq!(HullSpec::default().class_turn, 0.0);
    }

    #[test]
    fn a_hulls_class_speed_is_its_speed_alone_in_pixels_a_tick() {
        let of = |speed| HullSpec::new(&HullRecord { speed, ..hull(128) }).class_speed;
        assert_eq!(of(350), 3.5);
        assert_eq!(of(0), 0.0);
        assert_eq!(of(-200), 0.0, "none below none");
        assert_eq!(HullSpec::default().class_speed, 0.0);
    }

    #[test]
    fn a_ship_of_100_tons_or_more_dies_in_an_explosion_sized_by_its_mass() {
        let of = |mass| {
            HullSpec {
                mass,
                ..HullSpec::default()
            }
            .death_size()
        };
        assert_eq!(of(0.0), 0.0);
        assert_eq!(of(99.0), 0.0);
        assert_eq!(of(99.99), 0.0);
        assert_eq!(of(100.0), 57.0, "57.5, truncated");
        assert_eq!(of(120.0), 59.0);
        assert_eq!(of(10_000.0), 800.0);
        assert_eq!(
            (DEATH_SIZE_MASS, DEATH_SIZE_PER_TON, DEATH_SIZE_BASE),
            (100.0, 0.075, 50.0)
        );
    }

    #[test]
    fn a_planet_type_ship_dies_in_an_explosion_of_no_size() {
        let planet = HullSpec {
            mass: 400.0,
            planet_type: true,
            ..HullSpec::default()
        };
        assert_eq!(planet.death_size(), 0.0);
        let ordinary = HullSpec {
            planet_type: false,
            ..planet
        };
        assert_eq!(ordinary.death_size(), 80.0);
    }

    #[test]
    fn intact_and_disabled_ships_stand_and_dying_or_destroyed_ones_do_not() {
        assert!(Condition::Intact.standing());
        assert!(Condition::Disabled.standing());
        for starting in [false, true] {
            assert!(
                !Condition::Dying {
                    ticks_left: 3,
                    starting
                }
                .standing()
            );
        }
        assert!(!Condition::Destroyed.standing());
        assert_eq!(Condition::default(), Condition::Intact);
    }

    #[test]
    fn every_ship_but_a_destroyed_one_is_active() {
        assert!(Condition::Intact.active());
        assert!(Condition::Disabled.active());
        for starting in [false, true] {
            assert!(
                Condition::Dying {
                    ticks_left: 3,
                    starting
                }
                .active()
            );
        }
        assert!(!Condition::Destroyed.active());
    }

    #[test]
    fn disabled_and_breaking_up_ships_are_disabled_and_intact_or_destroyed_ones_not() {
        assert!(!Condition::Intact.disabled());
        assert!(Condition::Disabled.disabled());
        for starting in [false, true] {
            assert!(
                Condition::Dying {
                    ticks_left: 3,
                    starting
                }
                .disabled()
            );
        }
        assert!(!Condition::Destroyed.disabled());
    }

    #[test]
    fn the_engine_reading_stalls_and_the_other_explodes() {
        assert_eq!(Breakup::from(RuleSource::Engine), Breakup::Stalls);
        assert_eq!(Breakup::from(RuleSource::Bible), Breakup::Explodes);
        assert_eq!(Breakup::default(), Breakup::Explodes);
        assert_eq!((PLAYER_BREAKUP_FACTOR, EXPLOSION_BAND), (3, 2));
    }
}
