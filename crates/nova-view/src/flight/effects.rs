//! The fight's explosions, debris and sounds, from the session's
//! [`CombatEvent`]s, as the original shows them (`_LoadObjectData`
//! @0x7d02b, `_HandleExplods` @0x2f5d2, `_CreateExplosion` @0x3f698,
//! `_HandleShipDisplay` @0x2d7ef-0x2dee7 and `_SpawnParticles` @0x44b70
//! in the `EV Nova` executable).
//!
//! - An explosion is a frame of its `bööm`'s sheet, drawn centred where it
//!   went off: it never moves. Each step its frame advances by the
//!   `bööm`'s `FrameAdvance` / 100, and it ends once the frame is past the
//!   last. A delayed one is hidden while its delay counts down by the same
//!   advance a step. One whose sheet cannot be read is heard and lasts a
//!   frame, unseen.
//! - Under the original's caps
//!   ([`RuleKey::CombatCaps`](nova_sim::RuleKey::CombatCaps)'s engine
//!   reading, [`Scene::caps`]; by default nothing is capped) at most
//!   [`MAX_EXPLOSIONS`] explosions are kept: one more is dropped, though
//!   still heard, as `_CreateExplosion` plays its sound whether
//!   `_SpawnExplod` found it a slot or not (@0x3f91a). The extras take
//!   their slots first, so a size of 163 or more fills the pool alone and
//!   the main one is lost, heard. At most [`MAX_PARTICLES`] particles are
//!   kept, of a burst or of debris, one more dropped; and a particle of
//!   no life is never moved, drawn or freed, holding its count of the
//!   pool until it is cleared (`_MoveParticles` @0xc6134-0xc613a).
//! - An explosion of a code of 1000 or more, with a size, first scatters
//!   `trunc(0.04 x size)` explosions of `bööm` 129 within a quarter of the
//!   size of it, delayed 4-11, and `trunc(0.16 x size)` of `bööm` 128
//!   within half the size, delayed 8-23 ([`Effects::explode`]). Only the
//!   main one sounds. A weapon's explosion is of its `BlastRadius`, as
//!   the fight raises it; every other is of none.
//! - A ship breaking up, while more than 2 ticks are left, sets off its
//!   `Explode1` about it at random, more often as its end nears, a quarter
//!   of them heard ([`Effects::step`]).
//! - A destroyed ship goes off in its `Explode2`, with its size, and,
//!   under [`RuleKey::ShipDebris`](nova_sim::RuleKey::ShipDebris)'s other
//!   reading, the default, though the original scatters none
//!   ([`Scene::debris`]), [`DEBRIS_COUNT`] particles of debris, by the
//!   original's particle model: each in a
//!   random direction at [`DEBRIS_SPEED`] ± [`DEBRIS_SPEED_VARIATION`] %,
//!   living 15-45 ticks, and carried on with the ship's velocity.
//! - A weapon firing sounds its `snd ` from where it was fired (its ship,
//!   or the shot releasing sub-munitions), and an explosion its own, each
//!   heard from the player ([`CombatSound`]), unless the fight raised it
//!   with no sound: an NPC's pick sounds once a call, on its first copy
//!   ([`FireSound`](nova_sim::FireSound)). Each carries the original's
//!   priority, which the audio side's voice list plays by: the
//!   [`FireSound`](nova_sim::FireSound)'s for a weapon,
//!   [`EXPLOSION_PRIORITY`] for an explosion.
//! - A weapon whose sound loops (`Flags` 0x0010), fired by a function that
//!   tests it ([`FireSound::unless_playing`](nova_sim::FireSound)), waits
//!   while its sound plays, as
//!   [`RuleKey::LoopedSoundWait`](nova_sim::RuleKey::LoopedSoundWait)
//!   says ([`Scene::looped`]). By the engine's reading, the default
//!   ([`LoopedSounds::Anywhere`]), its sound goes out marked
//!   [`CombatSound::until_stopped`], and the audio side skips it while
//!   that `snd ` plays on any voice, from any ship, as the original's
//!   `ST_IsSoundPlaying` does for the player, NPCs, point defence and
//!   sub-munitions alike. By the other reading
//!   ([`LoopedSounds::PerFirer`]) each ship keeps its own: it isn't heard
//!   again for its length in steps ([`WeaponLook::sound_ticks`]) after it
//!   was heard, so two ships' chainguns are each heard, from where each
//!   is, and a looped weapon whose sound has no known length is heard
//!   every shot. Either way it plays out when firing stops, and is never
//!   stopped. A fighter bay's launch never waits: `_AILaunchFighter` tests
//!   nothing.
//! - A missile shot down by point defence, and a shot fizzling out at the
//!   end of its life ([`CombatEvent::Fizzled`]), go off as explosion type
//!   0 ([`TYPE_ZERO`]), unheard, as `_SpawnExplod` plays nothing.
//! - A burst of particles ([`CombatEvent::Particles`]: a shot's or a
//!   beam's hit on a ship, a shot's on an asteroid or a stellar, an
//!   asteroid destroyed) is `_SpawnParticles`
//!   (@0x44b70-0x44d4e) as a plain particle effect: each of its pieces
//!   leaves where it went off, or `_Accel(Rand(360), Rand(spread × 100)
//!   × 0.01)` off it with a spread, along `Rand(360)` at its speed,
//!   varied as `speed × (100 + Rand(2v + 1) − v) / 100` for a variation
//!   `v` of 1-99, and lives `min + Rand(max − min + 1)` ticks, or `min`
//!   when `min` is at or above `max`; it takes on no source's velocity,
//!   and is a dot of the burst's `00RRGGBB` colour.
//! - Every particle, a burst's, a trail's or debris, is drawn opaque and
//!   then fades over its last 32 ticks of life, at `life`/32
//!   ([`particle_alpha`]), as `_DrawParticlesOffscreen` blends it
//!   (`_Trans16Bit(colour, dst, life)` @0xc5c26-0xc5c95). The
//!   original's argument after the colour, which `_NewParticle` stores
//!   (+0x24, @0xc50de), is only ever tested against 0x20 by the drawing
//!   (@0xc5a61, @0xc5b2b), and both its paths draw the same: it changes
//!   nothing, so a burst carries none.
//!
//! - A dying NPC's escape pod ([`CombatEvent::PodEjected`]) is heard as
//!   `snd ` 372 from where its ship is
//!   ([`ESCAPE_POD_SOUND`](nova_sim::sound::ESCAPE_POD_SOUND)), whether
//!   or not the pool keeps it, and set adrift ([`pods`](super::pods)),
//!   under the caps at most [`MAX_PODS`](super::pods::MAX_PODS) of them.
//!
//! [`WeaponLook::sound_ticks`]: super::catalog::WeaponLook::sound_ticks
//!
//! The effects draw on their own [`Chance`], never on the simulation's.

use std::collections::BTreeMap;

use nova_sim::combat::ParticleBurst;
use nova_sim::combat::weapon::Explosion as Blast;
use nova_sim::{Chance, CombatEvent, ShipRef, Vec2};

use super::catalog::{BoomId, Looks, WeaponId};
use super::pods::{EscapePods, Pod};
use super::weapons::{LOOPED_SOUND, translucent};
use crate::sound::CombatSound;
use crate::system::camera::Camera;
use crate::{Color, DrawList, ImageKey, Point};

/// The ticks of life over which a particle fades out
/// (`_DrawParticlesOffscreen` @0xc5c26-0xc5c95).
pub const PARTICLE_FADE: u32 = 32;

/// The alpha of a particle with `life` ticks left, as
/// `_DrawParticlesOffscreen` blends it (@0xc5c26-0xc5c95,
/// `_Trans16Bit(colour, dst, life)` @0xc2c2c): opaque above
/// [`PARTICLE_FADE`] ticks, then `life`/32, rounded.
#[must_use]
pub fn particle_alpha(life: u32) -> u8 {
    let shown = life.min(PARTICLE_FADE);
    u8::try_from((shown * 255 + PARTICLE_FADE / 2) / PARTICLE_FADE).unwrap_or(u8::MAX)
}

/// The most explosions shown at once under the caps (`_SpawnExplod`
/// @0x3f615-0x3f68e).
pub const MAX_EXPLOSIONS: usize = 32;
/// The most particles kept at once under the caps: the pool
/// `_SetupSpriteWorld` makes (`_InitParticles(0x400, 0)` @0x22170-0x2217f),
/// a particle more dropped (`_NewParticle` @0xc5054, @0xc5092).
pub const MAX_PARTICLES: usize = 1024;
/// The small extra explosions, `bööm` 129.
pub const SMALL_EXTRA: BoomId = BoomId(129);
/// The large extra explosions, `bööm` 128.
pub const LARGE_EXTRA: BoomId = BoomId(128);
/// `_SpawnExplod`'s explosion type 0, `bööm` 128: a missile shot down by
/// point defence, and a shot fizzling out.
pub const TYPE_ZERO: BoomId = BoomId(128);
/// Small extras per unit of size (the double @0xdd700).
pub const SMALL_PER_SIZE: f64 = 0.04;
/// Large extras per unit of size (the double @0xdd710).
pub const LARGE_PER_SIZE: f64 = 0.16;
/// How many pieces of debris a destroyed ship scatters: a placeholder.
pub const DEBRIS_COUNT: u32 = 16;
/// Debris' speed, in pixels a tick: a placeholder.
pub const DEBRIS_SPEED: f32 = 2.0;
/// How far, in percent, a piece's speed varies either way: a placeholder.
pub const DEBRIS_SPEED_VARIATION: u32 = 50;
/// A piece's shortest life, in ticks: a placeholder.
pub const DEBRIS_LIFE_MIN: u32 = 15;
/// A piece's longest life, in ticks: a placeholder.
pub const DEBRIS_LIFE_MAX: u32 = 45;
/// Debris' colour: a placeholder.
pub const DEBRIS_COLOR: Color = Color::rgba(0xA0, 0xA0, 0xA0, 255);
/// A piece of debris' size: a dot a pixel across.
pub const DEBRIS_SIZE: f32 = 1.0;
/// How far a breakup explosion goes off from the ship's centre, per pixel
/// of the ship's sprite width (the double @0xdd130).
pub const BREAKUP_SPREAD: f32 = 0.25;

/// The simulation's `v` as a view point.
fn point(v: Vec2) -> Point {
    Point::new(v.x, v.y)
}

/// `at` moved by (`x`, `y`).
fn offset(at: Point, x: f64, y: f64) -> Point {
    Point::new(at.x + x as f32, at.y + y as f32)
}

/// `at` moved `distance` along `heading` degrees, as `_Accel` pushes.
fn offset_along(at: Point, heading: f32, distance: f32) -> Point {
    let (sin, cos) = heading.to_radians().sin_cos();
    Point::new(at.x + sin * distance, at.y - cos * distance)
}

/// A `00RRGGBB` colour, opaque.
fn particle_color(rgb: u32) -> Color {
    let [_, r, g, b] = rgb.to_be_bytes();
    Color::rgba(r, g, b, 255)
}

/// The priority an explosion's sound plays at (`_CreateExplosion`'s
/// `_PlaySoundDistance(.., 6, ..)` @0x3f8f1, played @0x3f91a), a breakup's
/// too (@0x2decf).
pub const EXPLOSION_PRIORITY: u16 = 6;

/// The priority an escape pod's sound plays at, not looped:
/// `_HandleShip`'s `_PlaySoundDistance(-1, snd, 5, ..)` for a person's pod
/// (@0x351fa) and each crew pod (@0x35300).
pub const POD_PRIORITY: u16 = 5;

/// `sound`, heard by a player at `player` from `at`, at `priority`, and
/// skipped while it plays `until_stopped`.
fn heard(
    sound: nova_sim::SoundId,
    at: Point,
    player: Point,
    priority: u16,
    until_stopped: bool,
) -> CombatSound {
    CombatSound {
        sound,
        offset: (
            (at.x - player.x).round() as i32,
            (at.y - player.y).round() as i32,
        ),
        priority,
        until_stopped,
    }
}

/// `sound`, an explosion's, heard by a player at `player` from `at`.
fn explosion_heard(sound: nova_sim::SoundId, at: Point, player: Point) -> CombatSound {
    heard(sound, at, player, EXPLOSION_PRIORITY, false)
}

/// Where a looped weapon's sound waits, as
/// [`RuleKey::LoopedSoundWait`](nova_sim::RuleKey::LoopedSoundWait) says
/// (see the module docs).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LoopedSounds {
    /// While its `snd ` plays on any voice, from any ship: the audio side
    /// decides, from [`CombatSound::until_stopped`]. The engine's reading.
    #[default]
    Anywhere,
    /// For its own length, each firer's weapon on its own. The other
    /// reading.
    PerFirer,
}

/// An explosion going off.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Explosion {
    /// Its `bööm`.
    pub boom: BoomId,
    /// Where it is, in world units.
    pub at: Point,
    /// The frame it is on, fractional.
    pub frame: f32,
    /// How long it is hidden yet, in frames.
    pub delay: f32,
}

/// A particle: a piece of debris, or of a burst.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Particle {
    /// Where it is, in world units.
    pub at: Point,
    /// How far it moves a tick.
    pub velocity: Point,
    /// The ticks it has left.
    pub life: u32,
    /// Its colour.
    pub color: Color,
}

/// Where a step's events are heard from, and how they are shown.
#[derive(Clone, Copy, Debug)]
pub struct Scene {
    /// Where the player is: sounds are heard from here.
    pub player: Point,
    /// Whether a destroyed ship scatters debris, as
    /// [`RuleKey::ShipDebris`](nova_sim::RuleKey::ShipDebris)'s other
    /// reading, the default, has it; the original scatters none.
    pub debris: bool,
    /// Whether the original's explosion and particle pools cap them, as
    /// [`RuleKey::CombatCaps`](nova_sim::RuleKey::CombatCaps)'s engine
    /// reading has it; by default, not.
    pub caps: bool,
    /// Where a looped weapon's sound waits.
    pub looped: LoopedSounds,
}

/// A ship breaking up.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Dying {
    /// Where it is, in world units.
    pub at: Point,
    /// Its sprite's width.
    pub sprite_width: f32,
    /// Its `Explode1`, if any.
    pub explosion: Option<Blast>,
    /// Its death timer: the original's countdown, exploding at 2 or
    /// below; 0 while it is unset.
    pub ticks_left: u32,
    /// Whether the timer is unset, or its display step set it on this
    /// tick.
    pub starting: bool,
}

/// The fight's explosions, debris and sounds.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Effects {
    explosions: Vec<Explosion>,
    particles: Vec<Particle>,
    sounds: Vec<CombatSound>,
    /// Each looped weapon sound still playing, by firer and weapon, with
    /// the steps it has left, under [`LoopedSounds::PerFirer`].
    playing: BTreeMap<(ShipRef, WeaponId), u32>,
    /// The particles made with no life under the caps: never moved, drawn
    /// or freed, each holds its count of the pool until it is cleared
    /// (see the module docs).
    lifeless: usize,
    /// The NPCs' escape pods adrift (see [`pods`](super::pods)).
    pods: EscapePods,
}

impl Effects {
    /// Shows `event` in `scene`, its looks from `looks`, drawing on
    /// `chance` (see the module docs).
    pub fn apply(
        &mut self,
        event: &CombatEvent,
        scene: &Scene,
        looks: &Looks,
        chance: &mut dyn Chance,
    ) {
        match *event {
            CombatEvent::Fired {
                ship,
                weapon,
                at,
                sound: Some(fire),
            } => {
                let Some((sound, look)) = looks
                    .weapon(weapon)
                    .and_then(|look| Some((look.sound?, look)))
                else {
                    return;
                };
                let waits = look.flags & LOOPED_SOUND != 0 && fire.unless_playing;
                let until_stopped = match scene.looped {
                    LoopedSounds::Anywhere => waits,
                    LoopedSounds::PerFirer => {
                        if waits && let Some(ticks) = look.sound_ticks {
                            if self.playing.contains_key(&(ship, weapon)) {
                                return;
                            }
                            self.playing.insert((ship, weapon), ticks);
                        }
                        false
                    }
                };
                let priority = u16::from(fire.priority);
                let heard = heard(sound, point(at), scene.player, priority, until_stopped);
                self.sounds.push(heard);
            }
            CombatEvent::ShotDown { at } | CombatEvent::Fizzled { at } => {
                self.spawn(TYPE_ZERO, point(at), 0.0, looks, scene.caps);
            }
            CombatEvent::Exploded {
                at,
                explosion,
                size,
            } => {
                self.explode(explosion, point(at), size, scene, looks, chance);
            }
            // A `Y`'s: the session raised its sound as it went off.
            CombatEvent::ExplodedHeard { at, explosion } => {
                self.go_off(explosion, point(at), 0.0, scene, looks, chance);
            }
            CombatEvent::Destroyed {
                at,
                velocity,
                explosion,
                size,
                ..
            } => {
                if let Some(explosion) = explosion {
                    self.explode(explosion, point(at), size, scene, looks, chance);
                }
                if scene.debris {
                    self.scatter(point(at), point(velocity), scene.caps, chance);
                }
            }
            CombatEvent::Particles(burst) => self.spray(&burst, scene.caps, chance),
            // `_HandleShip` plays the pod's sound whether or not
            // `_SpawnEscapePod` found it a slot (@0x351fa, @0x35300).
            CombatEvent::PodEjected { at, velocity, .. } => {
                let at = point(at);
                self.sounds.push(heard(
                    nova_sim::sound::ESCAPE_POD_SOUND,
                    at,
                    scene.player,
                    POD_PRIORITY,
                    false,
                ));
                self.pods
                    .spawn(at, point(velocity), scene.caps, looks, chance);
            }
            // A hit's shield flash is the flight view's
            // ([`ShieldFlash`](super::flash::ShieldFlash)), and so is the
            // weapon glow.
            CombatEvent::Fired { sound: None, .. }
            | CombatEvent::Disabled { .. }
            | CombatEvent::BreakingUp { .. }
            | CombatEvent::Hit { .. }
            | CombatEvent::WeaponGlow { .. } => {}
        }
    }

    /// Throws off `burst`'s particles, under the `caps` or not (see the
    /// module docs): a fight's burst, or a shot's trail
    /// ([`trails`](super::trails)).
    pub fn spray(&mut self, burst: &ParticleBurst, caps: bool, chance: &mut dyn Chance) {
        let at = point(burst.at);
        let color = particle_color(burst.color);
        let (least, most) = burst.life;
        for _ in 0..burst.count {
            let heading = chance.below(360) as f32;
            let speed = if (1..=99).contains(&burst.variation) {
                let v = burst.variation;
                let percent = 100 + chance.below(2 * v + 1);
                burst.speed * (percent as f32 - v as f32) / 100.0
            } else {
                burst.speed
            };
            let life = if least >= most {
                least
            } else {
                least + chance.below(most - least + 1)
            };
            let start = if burst.spread > 0.0 {
                let angle = chance.below(360) as f32;
                let reach = (burst.spread * 100.0) as u32;
                let off = chance.below(reach.max(1)) as f32 * 0.01;
                offset_along(at, angle, off)
            } else {
                at
            };
            let piece = Particle {
                at: start,
                velocity: offset_along(Point::new(0.0, 0.0), heading, speed),
                life,
                color,
            };
            self.keep(piece, caps);
        }
    }

    /// Keeps `piece`, unless the `caps` hold the pool full: then it is
    /// dropped, as `_NewParticle` drops one once 1024 are counted
    /// (@0xc5054-0xc5056, @0xc5092-0xc5094). Under them a piece of no life
    /// is counted and kept nowhere: `_NewParticle` counts it, and
    /// `_MoveParticles` skips a life of none or less, never moving it or
    /// counting it off (@0xc6134-0xc613a), and the draw skips it
    /// (`_DrawParticlesOffscreen` @0xc5a20-0xc5a27), until the pool is
    /// cleared ([`Effects::clear`], `_ClearParticles`).
    fn keep(&mut self, piece: Particle, caps: bool) {
        if caps && self.particles.len() + self.lifeless >= MAX_PARTICLES {
            return;
        }
        if caps && piece.life == 0 {
            self.lifeless += 1;
        } else {
            self.particles.push(piece);
        }
    }

    /// Scatters [`DEBRIS_COUNT`] pieces of debris from `at`, carried on
    /// with `velocity`, under the `caps` or not.
    fn scatter(&mut self, at: Point, velocity: Point, caps: bool, chance: &mut dyn Chance) {
        let variation = DEBRIS_SPEED_VARIATION;
        for _ in 0..DEBRIS_COUNT {
            let heading = chance.below(360) as f32;
            let percent = 100 + chance.below(2 * variation + 1);
            let speed = DEBRIS_SPEED * (percent as f32 - variation as f32) / 100.0;
            let life = DEBRIS_LIFE_MIN + chance.below(DEBRIS_LIFE_MAX - DEBRIS_LIFE_MIN + 1);
            let (sin, cos) = heading.to_radians().sin_cos();
            let piece = Particle {
                at,
                velocity: Point::new(velocity.x + sin * speed, velocity.y - cos * speed),
                life,
                color: DEBRIS_COLOR,
            };
            self.keep(piece, caps);
        }
    }

    /// Sets off `boom` at `at`, hidden for `delay`, unless the `caps` hold
    /// the pool full or its look was not read; whether it went off.
    fn spawn(&mut self, boom: BoomId, at: Point, delay: f32, looks: &Looks, caps: bool) -> bool {
        let room = !caps || self.explosions.len() < MAX_EXPLOSIONS;
        let spawned = room && looks.boom(boom).is_some();
        if spawned {
            self.explosions.push(Explosion {
                boom,
                at,
                frame: 0.0,
                delay,
            });
        }
        spawned
    }

    /// Sets off `explosion` at `at` with `size`, heard from the player in
    /// `scene`, under its caps or not (see the module docs).
    pub fn explode(
        &mut self,
        explosion: Blast,
        at: Point,
        size: f32,
        scene: &Scene,
        looks: &Looks,
        chance: &mut dyn Chance,
    ) {
        self.go_off(explosion, at, size, scene, looks, chance);
        // Heard whether or not the pool had room for it (`_CreateExplosion`
        // @0x3f91a).
        if let Some(sound) = looks.boom(explosion.boom).and_then(|look| look.sound) {
            self.sounds.push(explosion_heard(sound, at, scene.player));
        }
    }

    /// Sets off `explosion` at `at` with `size` as [`Effects::explode`]
    /// does, unheard.
    fn go_off(
        &mut self,
        explosion: Blast,
        at: Point,
        size: f32,
        scene: &Scene,
        looks: &Looks,
        chance: &mut dyn Chance,
    ) {
        let caps = scene.caps;
        if explosion.extra {
            let size = f64::from(size);
            let small = (SMALL_PER_SIZE * size) as u32;
            let small_reach = (0.5 * size) as u32;
            for _ in 0..small {
                let off =
                    |chance: &mut dyn Chance| f64::from(chance.below(small_reach)) - 0.25 * size;
                let (x, y) = (off(chance), off(chance));
                let delay = chance.below(8) + 4;
                self.spawn(SMALL_EXTRA, offset(at, x, y), delay as f32, looks, caps);
            }
            let large = (LARGE_PER_SIZE * size) as u32;
            let large_reach = size as u32;
            for _ in 0..large {
                let off =
                    |chance: &mut dyn Chance| f64::from(chance.below(large_reach)) - 0.5 * size;
                let (x, y) = (off(chance), off(chance));
                let delay = chance.below(16) + 8;
                self.spawn(LARGE_EXTRA, offset(at, x, y), delay as f32, looks, caps);
            }
        }
        self.spawn(explosion.boom, at, 0.0, looks, caps);
    }

    /// Advances the explosions and debris a step, and the looped sounds
    /// playing, then sets off the explosions of the ships `dying`, but
    /// for those on their breakup's first tick, heard from the player in
    /// `scene`, under its caps or not.
    pub fn step(&mut self, dying: &[Dying], scene: &Scene, looks: &Looks, chance: &mut dyn Chance) {
        let player = scene.player;
        self.pods.step();
        self.playing.retain(|_, left| {
            *left = left.saturating_sub(1);
            *left > 0
        });
        self.explosions.retain_mut(|explosion| {
            let Some(look) = looks.boom(explosion.boom) else {
                return false;
            };
            if explosion.delay > 0.0 {
                explosion.delay -= look.advance;
            } else {
                explosion.frame += look.advance;
            }
            let frames = look.sheet.as_ref().map_or(1, |sheet| sheet.frames.get());
            (explosion.frame as u32) < u32::from(frames)
        });
        self.particles.retain_mut(|piece| {
            piece.at = Point::new(piece.at.x + piece.velocity.x, piece.at.y + piece.velocity.y);
            piece.life = piece.life.saturating_sub(1);
            piece.life > 0
        });
        // The original never reaches its puff branch (0x2dd01) on the
        // frame it sets the timer: both its paths jump past it to 0x2def0.
        for ship in dying.iter().filter(|ship| !ship.starting) {
            let Some(explosion) = ship.explosion.filter(|_| ship.ticks_left > 2) else {
                continue;
            };
            let odds = match ship.ticks_left {
                ..20 => 1,
                20..40 => 2,
                40..60 => 4,
                _ => 8,
            };
            if chance.below(odds) != 0 {
                continue;
            }
            let half = ((BREAKUP_SPREAD * ship.sprite_width) as u32).max(1);
            let mut off = || chance.below(2 * half) as f32 - half as f32;
            let (x, y) = (off(), off());
            let at = Point::new(ship.at.x + x, ship.at.y + y);
            // `_CreateExplosion(.., Rand(4) == 0)` @0x2decf: heard whether
            // or not the pool had room for it.
            self.spawn(explosion.boom, at, 0.0, looks, scene.caps);
            let audible = chance.below(4) == 0;
            if audible && let Some(sound) = looks.boom(explosion.boom).and_then(|look| look.sound) {
                self.sounds.push(explosion_heard(sound, at, player));
            }
        }
    }

    /// Draws the explosions shown, as `looks` says, and the debris, where
    /// `camera` shows them.
    pub fn draw(&self, list: &mut DrawList, camera: &Camera, looks: &Looks) {
        for explosion in self.explosions.iter().filter(|e| e.delay <= 0.0) {
            if let Some(Ok(sheet)) = looks.boom(explosion.boom).map(|look| &look.sheet) {
                let frame = ImageKey::sprite(sheet.image_id, explosion.frame as u16);
                list.sprite(frame, camera.world_to_screen(explosion.at), translucent());
            }
        }
        for piece in &self.particles {
            let color = Color {
                a: particle_alpha(piece.life),
                ..piece.color
            };
            list.dot(camera.world_to_screen(piece.at), DEBRIS_SIZE, color);
        }
    }

    /// Draws the escape pods adrift, as `looks` says, where `camera` shows
    /// them, in their place among the sprites (see [`pods`](super::pods)).
    pub fn draw_pods(&self, list: &mut DrawList, camera: &Camera, looks: &Looks) {
        self.pods.draw(list, camera, looks);
    }

    /// The escape pods adrift.
    #[must_use]
    pub fn pods(&self) -> &[Pod] {
        self.pods.pods()
    }

    /// The explosions going off.
    #[must_use]
    pub fn explosions(&self) -> &[Explosion] {
        &self.explosions
    }

    /// The particles flying: debris and bursts.
    #[must_use]
    pub fn particles(&self) -> &[Particle] {
        &self.particles
    }

    /// The sounds made since they were last taken; taking them empties the
    /// list.
    pub fn take_sounds(&mut self) -> Vec<CombatSound> {
        std::mem::take(&mut self.sounds)
    }

    /// Clears everything: the explosions, the debris, the sounds, the
    /// looped sounds playing, the particles of no life the caps count, and
    /// the escape pods.
    pub fn clear(&mut self) {
        self.pods.clear();
        self.explosions.clear();
        self.particles.clear();
        self.sounds.clear();
        self.playing.clear();
        self.lifeless = 0;
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::collections::{BTreeMap, VecDeque};
    use std::num::NonZeroU16;

    use nova_sim::{FireSound, NpcId, SoundId};

    use super::*;
    use crate::DrawCommand;
    use crate::flight::catalog::{BoomLook, EffectSheet, WeaponLook};
    use crate::flight::weapons::LOOPED_SOUND;

    /// Draws its script, then the last outcome; records each `n` asked.
    #[derive(Default)]
    struct Script {
        draws: VecDeque<u32>,
        asked: Vec<u32>,
    }

    impl Script {
        fn of(draws: &[u32]) -> Self {
            Self {
                draws: draws.iter().copied().collect(),
                asked: Vec::new(),
            }
        }
    }

    impl Chance for Script {
        fn fires(&mut self, _percent: u8) -> bool {
            false
        }

        fn below(&mut self, n: u32) -> u32 {
            self.asked.push(n);
            let draw = self.draws.pop_front().unwrap_or(n - 1);
            assert!(draw < n, "drew {draw} below {n}");
            draw
        }
    }

    fn at(x: f32, y: f32) -> Point {
        Point::new(x, y)
    }

    fn boom_look(image_id: i16, frames: u16, advance: f32, sound: Option<i16>) -> BoomLook {
        BoomLook {
            sheet: Ok(EffectSheet {
                frame_width: 0,
                image_id,
                frames: NonZeroU16::new(frames).expect("non-zero"),
            }),
            advance,
            sound: sound.map(SoundId),
        }
    }

    /// `bööm` 128 (3 frames, silent), 129 (2 frames, silent), 130 (4
    /// frames, `snd ` 302), 135 (3 frames at 0.3 a step, `snd ` 300),
    /// 142 (no sheet, `snd ` 303); 140 cannot be read. Weapon 128 sounds
    /// `snd ` 208; 146, a beam, loops `snd ` 210 for 3 ticks; 147 is
    /// silent; 148 loops `snd ` 211 of no known length; and 155, a
    /// chaingun, loops `snd ` 205 for 14 ticks.
    fn looks() -> Looks {
        let weapon = |sound: Option<i16>, flags, sound_ticks| {
            Ok(WeaponLook {
                sound: sound.map(SoundId),
                flags,
                sound_ticks,
                ..WeaponLook::default()
            })
        };
        Looks {
            smokes: BTreeMap::default(),
            weapons: BTreeMap::from([
                (WeaponId(128), weapon(Some(208), 0, Some(5))),
                (WeaponId(146), weapon(Some(210), LOOPED_SOUND, Some(3))),
                (WeaponId(147), weapon(None, LOOPED_SOUND, Some(3))),
                (WeaponId(148), weapon(Some(211), LOOPED_SOUND, None)),
                (WeaponId(155), weapon(Some(205), LOOPED_SOUND, Some(14))),
            ]),
            booms: BTreeMap::from([
                (BoomId(128), Ok(boom_look(400, 3, 1.0, None))),
                (BoomId(129), Ok(boom_look(401, 2, 1.0, None))),
                (BoomId(130), Ok(boom_look(402, 4, 1.0, Some(302)))),
                (BoomId(135), Ok(boom_look(405, 3, 0.3, Some(300)))),
                (BoomId(140), Err("no spïn 410".to_owned())),
                (
                    BoomId(142),
                    Ok(BoomLook {
                        sheet: Err("no spïn 412".to_owned()),
                        advance: 1.0,
                        sound: Some(SoundId(303)),
                    }),
                ),
            ]),
            boxes: BTreeMap::new(),
            asteroids: BTreeMap::new(),
            cursor: None,
            pod: None,
            stars: None,
        }
    }

    fn blast(boom: i16, extra: bool) -> Blast {
        Blast {
            boom: BoomId(boom),
            extra,
        }
    }

    const PLAYER: Point = Point::new(0.0, 0.0);

    fn exploded(effects: &mut Effects, boom: i16, where_: Point) {
        exploded_in(effects, boom, where_, &scene());
    }

    /// `boom` set off at `where_` in `scene`.
    fn exploded_in(effects: &mut Effects, boom: i16, where_: Point, scene: &Scene) {
        effects.explode(
            blast(boom, false),
            where_,
            0.0,
            scene,
            &looks(),
            &mut Script::default(),
        );
    }

    fn frames(effects: &Effects) -> Vec<f32> {
        effects.explosions().iter().map(|e| e.frame).collect()
    }

    fn steps(effects: &mut Effects, n: u32) {
        for _ in 0..n {
            effects.step(&[], &scene(), &looks(), &mut Script::default());
        }
    }

    /// `looks` with the escape pods' sheet, `rlëD` 1095 of 36 rotations.
    fn pod_looks() -> Looks {
        Looks {
            pod: Some(Ok(EffectSheet {
                image_id: 1095,
                frames: NonZeroU16::new(36).expect("non-zero"),
                frame_width: 0,
            })),
            ..looks()
        }
    }

    /// A pod ejected at (`x`, 0) at rest.
    fn ejected(x: f32) -> CombatEvent {
        CombatEvent::PodEjected {
            ship: ShipRef::Npc(NpcId(1)),
            at: Vec2::new(x, 0.0),
            velocity: Vec2::ZERO,
        }
    }

    #[test]
    fn an_ejected_pod_is_heard_as_snd_372_and_set_adrift() {
        let mut effects = Effects::default();
        effects.apply(
            &ejected(30.0),
            &scene(),
            &pod_looks(),
            &mut Script::default(),
        );
        assert_eq!(
            effects.take_sounds(),
            [CombatSound {
                sound: SoundId(372),
                offset: (30, 0),
                priority: POD_PRIORITY,
                until_stopped: false,
            }]
        );
        assert_eq!(effects.pods().len(), 1);
        let life = effects.pods()[0].life;
        effects.step(&[], &scene(), &pod_looks(), &mut Script::default());
        assert_eq!(effects.pods()[0].life, life - 1, "it steps with the rest");
        let mut list = DrawList::new();
        effects.draw_pods(&mut list, &Camera::centred_on(PLAYER), &pod_looks());
        assert!(
            list.into_iter().any(|command| matches!(
                command,
                DrawCommand::Sprite { image, .. } if image.id == 1095
            )),
            "drawn from its sheet"
        );
        effects.clear();
        assert!(effects.pods().is_empty(), "cleared with the rest");
    }

    #[test]
    fn under_the_caps_a_33rd_pod_is_not_kept_but_is_heard() {
        for (caps, kept) in [(true, 32), (false, 33)] {
            let mut effects = Effects::default();
            let scene = Scene { caps, ..scene() };
            for _ in 0..33 {
                effects.apply(&ejected(0.0), &scene, &pod_looks(), &mut Script::default());
            }
            assert_eq!(effects.pods().len(), kept, "caps {caps}");
            assert_eq!(effects.take_sounds().len(), 33, "every one heard");
        }
    }

    #[test]
    fn an_explosion_advances_a_frame_a_step_where_it_went_off_and_ends_after_its_last() {
        let mut effects = Effects::default();
        exploded(&mut effects, 130, at(10.0, 20.0));
        assert_eq!(
            effects.explosions(),
            [Explosion {
                boom: BoomId(130),
                at: at(10.0, 20.0),
                frame: 0.0,
                delay: 0.0
            }]
        );
        let mut seen = Vec::new();
        for _ in 0..4 {
            steps(&mut effects, 1);
            seen.push(frames(&effects));
        }
        assert_eq!(seen, [vec![1.0], vec![2.0], vec![3.0], vec![]]);
        assert_eq!(effects.particles(), []);
    }

    #[test]
    fn a_slow_explosion_shows_a_frame_for_several_steps() {
        let mut effects = Effects::default();
        exploded(&mut effects, 135, at(0.0, 0.0));
        let mut shown = Vec::new();
        for _ in 0..10 {
            shown.push(effects.explosions().first().map(|e| e.frame as u16));
            steps(&mut effects, 1);
        }
        assert_eq!(
            shown,
            [
                Some(0),
                Some(0),
                Some(0),
                Some(0),
                Some(1),
                Some(1),
                Some(1),
                Some(2),
                Some(2),
                Some(2)
            ]
        );
        steps(&mut effects, 1);
        assert_eq!(effects.explosions(), [], "past its third frame");
    }

    #[test]
    fn an_explosion_without_a_look_is_not_set_off() {
        let mut effects = Effects::default();
        exploded(&mut effects, 140, at(0.0, 0.0));
        exploded(&mut effects, 141, at(0.0, 0.0));
        assert_eq!(effects.explosions(), []);
        assert_eq!(effects.take_sounds(), []);
    }

    #[test]
    fn an_explosion_without_a_sheet_sounds_for_a_frame_and_is_not_drawn() {
        let mut effects = Effects::default();
        exploded(&mut effects, 142, at(3.0, 4.0));
        assert_eq!(effects.take_sounds(), [explosion_sound(303, (3, 4))]);
        assert_eq!(frames(&effects), [0.0]);
        let mut list = DrawList::new();
        effects.draw(&mut list, &Camera::centred_on(PLAYER), &looks());
        assert_eq!(list, DrawList::new());
        steps(&mut effects, 1);
        assert_eq!(effects.explosions(), [], "past its one frame");
    }

    #[test]
    fn under_the_caps_at_most_32_explosions_go_off_at_once() {
        let mut effects = Effects::default();
        for i in 0..33 {
            exploded_in(&mut effects, 130, at(i as f32, 0.0), &capped());
        }
        assert_eq!(effects.explosions().len(), 32);
        assert_eq!(
            effects.explosions()[31].at,
            at(31.0, 0.0),
            "the 33rd dropped"
        );
        // `_CreateExplosion` plays the main one's sound whether or not
        // `_SpawnExplod` found it a slot (@0x3f91a).
        assert_eq!(effects.take_sounds().len(), 33, "the 33rd still heard");
        assert_eq!(MAX_EXPLOSIONS, 32);
    }

    #[test]
    fn uncapped_by_default_a_33rd_explosion_goes_off() {
        let mut effects = Effects::default();
        for i in 0..33 {
            exploded(&mut effects, 130, at(i as f32, 0.0));
        }
        assert_eq!(effects.explosions().len(), 33);
        let mut chance = Script::of(&[0, 0, 0, 0]);
        effects.step(&[dying(30)], &scene(), &looks(), &mut chance);
        assert_eq!(effects.explosions().len(), 34, "a breakup's too");
        let mut capped_too = effects.clone();
        let mut chance = Script::of(&[0, 0, 0, 0]);
        capped_too.step(&[dying(30)], &capped(), &looks(), &mut chance);
        assert_eq!(
            capped_too.explosions().len(),
            34,
            "none more under the caps"
        );
        let shot_down = CombatEvent::ShotDown { at: Vec2::ZERO };
        effects.apply(&shot_down, &scene(), &looks(), &mut Script::default());
        assert_eq!(effects.explosions().len(), 35);
        capped_too.apply(&shot_down, &capped(), &looks(), &mut Script::default());
        assert_eq!(capped_too.explosions().len(), 34);
    }

    #[test]
    fn under_the_caps_a_particle_past_1024_is_dropped_and_by_default_none_is() {
        let fill = |scene: &Scene| {
            let mut effects = Effects::default();
            let mut chance = Script::default();
            effects.apply(&burst(1000, 0, (10, 10), 0.0), scene, &looks(), &mut chance);
            effects.apply(&burst(30, 0, (10, 10), 0.0), scene, &looks(), &mut chance);
            effects
        };
        assert_eq!(fill(&scene()).particles().len(), 1030);
        let mut capped_fill = fill(&capped());
        assert_eq!(capped_fill.particles().len(), MAX_PARTICLES);
        let destroyed = CombatEvent::Destroyed {
            ship: NPC,
            ship_type: nova_sim::ShipId(128),
            at: Vec2::ZERO,
            velocity: Vec2::ZERO,
            explosion: None,
            size: 0.0,
        };
        capped_fill.apply(&destroyed, &capped(), &looks(), &mut Script::default());
        assert_eq!(
            capped_fill.particles().len(),
            MAX_PARTICLES,
            "no debris either"
        );
        let mut debris = fill(&scene());
        debris.apply(&destroyed, &scene(), &looks(), &mut Script::default());
        assert_eq!(debris.particles().len(), 1030 + DEBRIS_COUNT as usize);
        assert_eq!(MAX_PARTICLES, 1024);
    }

    #[test]
    fn under_the_caps_a_lifeless_particle_holds_its_count_unseen_until_cleared() {
        let mut effects = Effects::default();
        let mut chance = Script::default();
        effects.apply(
            &burst(1000, 0, (0, 0), 0.0),
            &capped(),
            &looks(),
            &mut chance,
        );
        assert_eq!(effects.particles(), [], "never kept to be drawn");
        effects.step(&[], &capped(), &looks(), &mut chance);
        effects.apply(
            &burst(30, 0, (10, 10), 0.0),
            &capped(),
            &looks(),
            &mut chance,
        );
        assert_eq!(effects.particles().len(), 24, "the pool's count held");
        effects.clear();
        effects.apply(
            &burst(30, 0, (10, 10), 0.0),
            &capped(),
            &looks(),
            &mut chance,
        );
        assert_eq!(effects.particles().len(), 30, "cleared");
        let mut uncapped = Effects::default();
        uncapped.apply(
            &burst(1000, 0, (0, 0), 0.0),
            &scene(),
            &looks(),
            &mut chance,
        );
        uncapped.step(&[], &scene(), &looks(), &mut chance);
        uncapped.apply(
            &burst(30, 0, (10, 10), 0.0),
            &scene(),
            &looks(),
            &mut chance,
        );
        assert_eq!(uncapped.particles().len(), 30, "gone a step on, by default");
    }

    #[test]
    fn a_delayed_explosion_is_hidden_until_its_delay_runs_out() {
        let mut effects = Effects::default();
        let mut chance = Script::of(&[10, 10, 0, 10, 10, 0]);
        // Size 25: one small extra (delay 4) first, then four large ones.
        effects.explode(
            blast(130, true),
            at(0.0, 0.0),
            25.0,
            &scene(),
            &looks(),
            &mut chance,
        );
        let small = effects.explosions()[0];
        assert_eq!(
            (small.boom, small.delay, small.frame),
            (SMALL_EXTRA, 4.0, 0.0)
        );
        let list = |effects: &Effects| {
            let mut list = DrawList::new();
            effects.draw(&mut list, &Camera::centred_on(PLAYER), &looks());
            list.iter()
                .filter(|c| matches!(c, DrawCommand::Sprite { image, .. } if image.id == 401))
                .count()
        };
        assert_eq!(list(&effects), 0, "hidden");
        steps(&mut effects, 3);
        assert_eq!(effects.explosions()[0].delay, 1.0);
        assert_eq!(list(&effects), 0, "still hidden");
        steps(&mut effects, 1);
        assert_eq!(
            (effects.explosions()[0].delay, effects.explosions()[0].frame),
            (0.0, 0.0)
        );
        assert_eq!(list(&effects), 1, "shown");
        steps(&mut effects, 1);
        assert_eq!(effects.explosions()[0].frame, 1.0);
        assert_eq!(effects.explosions()[0].at, small.at, "never moving");
    }

    #[test]
    fn an_extra_explosion_of_size_100_scatters_4_small_and_16_large_ones_first() {
        let mut effects = Effects::default();
        let mut chance = Script::default();
        effects.explode(
            blast(130, true),
            at(1000.0, 500.0),
            100.0,
            &scene(),
            &looks(),
            &mut chance,
        );
        let explosions = effects.explosions();
        assert_eq!(explosions.len(), 21);
        // Each small one draws x and y below 50 and its delay below 8; each
        // large one x and y below 100 and its delay below 16.
        let mut asked = [50, 50, 8].repeat(4);
        asked.extend([100, 100, 16].repeat(16));
        assert_eq!(chance.asked, asked);
        // The last draw (n - 1) puts each at its square's far corner.
        for small in &explosions[..4] {
            assert_eq!(small.boom, SMALL_EXTRA);
            assert_eq!(small.at, at(1024.0, 524.0));
            assert_eq!(small.delay, 11.0);
        }
        for large in &explosions[4..20] {
            assert_eq!(large.boom, LARGE_EXTRA);
            assert_eq!(large.at, at(1049.0, 549.0));
            assert_eq!(large.delay, 23.0);
        }
        assert_eq!(
            explosions[20],
            Explosion {
                boom: BoomId(130),
                at: at(1000.0, 500.0),
                frame: 0.0,
                delay: 0.0
            }
        );
        assert_eq!(
            effects.take_sounds(),
            [explosion_sound(302, (1000, 500))],
            "only the main one sounds"
        );
        let mut first = Effects::default();
        let mut chance = Script::of(&[0, 0, 0]);
        first.explode(
            blast(130, true),
            at(1000.0, 500.0),
            100.0,
            &scene(),
            &looks(),
            &mut chance,
        );
        assert_eq!(
            first.explosions()[0].at,
            at(975.0, 475.0),
            "the near corner"
        );
        assert_eq!(first.explosions()[0].delay, 4.0);
        assert_eq!((SMALL_PER_SIZE, LARGE_PER_SIZE), (0.04, 0.16));
    }

    #[test]
    fn an_extra_explosion_without_a_size_or_a_plain_one_scatters_none() {
        let mut effects = Effects::default();
        let mut chance = Script::default();
        effects.explode(
            blast(130, true),
            at(0.0, 0.0),
            0.0,
            &scene(),
            &looks(),
            &mut chance,
        );
        effects.explode(
            blast(130, false),
            at(0.0, 0.0),
            100.0,
            &scene(),
            &looks(),
            &mut chance,
        );
        assert_eq!(effects.explosions().len(), 2);
        assert!(chance.asked.is_empty(), "{:?}", chance.asked);
    }

    fn dying(ticks_left: u32) -> Dying {
        Dying {
            at: at(100.0, 200.0),
            sprite_width: 48.0,
            explosion: Some(blast(130, false)),
            ticks_left,
            starting: false,
        }
    }

    #[test]
    fn a_ship_on_its_breakups_first_tick_rolls_no_puff() {
        let mut effects = Effects::default();
        let mut chance = Script::default();
        let starting = Dying {
            starting: true,
            ..dying(30)
        };
        effects.step(&[starting], &scene(), &looks(), &mut chance);
        assert!(chance.asked.is_empty(), "{:?}", chance.asked);
        assert_eq!(effects.explosions(), []);
        effects.step(&[dying(30)], &scene(), &looks(), &mut chance);
        assert_eq!(chance.asked, [2], "from the tick after");
    }

    #[test]
    fn a_ship_breaking_up_explodes_more_often_as_its_end_nears() {
        for (ticks_left, odds) in [
            (70, 8),
            (60, 8),
            (59, 4),
            (50, 4),
            (40, 4),
            (39, 2),
            (30, 2),
            (20, 2),
            (19, 1),
            (10, 1),
            (3, 1),
        ] {
            let mut effects = Effects::default();
            let mut chance = Script::default();
            effects.step(&[dying(ticks_left)], &scene(), &looks(), &mut chance);
            let expected: &[u32] = if odds == 1 { &[1, 24, 24, 4] } else { &[odds] };
            assert_eq!(chance.asked, expected, "{ticks_left}");
        }
    }

    #[test]
    fn a_breakup_explosion_goes_off_about_the_ship_and_a_quarter_are_heard() {
        let mut effects = Effects::default();
        // h is a quarter of 48: 12; the offsets draw below 24, less 12.
        let mut chance = Script::of(&[0, 0, 23, 0, 0, 5, 5, 1]);
        let ship = dying(30);
        effects.step(
            &[ship, ship],
            &heard_at(at(100.0, 0.0)),
            &looks(),
            &mut chance,
        );
        assert_eq!(chance.asked, [2, 24, 24, 4, 2, 24, 24, 4]);
        let explosions = effects.explosions();
        assert_eq!(explosions.len(), 2);
        assert_eq!(explosions[0].at, at(88.0, 211.0));
        assert_eq!(explosions[1].at, at(93.0, 193.0));
        assert_eq!((explosions[0].frame, explosions[0].delay), (0.0, 0.0));
        assert_eq!(
            effects.take_sounds(),
            [explosion_sound(302, (-12, 211))],
            "the first heard, the second not"
        );
        assert_eq!(BREAKUP_SPREAD, 0.25);
    }

    #[test]
    fn a_breakup_explosion_with_no_room_is_still_heard() {
        let mut effects = Effects::default();
        for i in 0..32 {
            exploded_in(&mut effects, 129, at(i as f32, 0.0), &capped());
        }
        effects.take_sounds();
        let mut chance = Script::of(&[0, 0, 0, 0]);
        let scene = Scene {
            caps: true,
            ..heard_at(at(100.0, 0.0))
        };
        effects.step(&[dying(30)], &scene, &looks(), &mut chance);
        assert!(
            effects.explosions().iter().all(|e| e.boom == BoomId(129)),
            "no room"
        );
        assert_eq!(effects.take_sounds().len(), 1, "heard");
    }

    #[test]
    fn a_narrow_ship_breaks_up_within_a_pixel() {
        let mut effects = Effects::default();
        let mut chance = Script::of(&[0, 0, 1]);
        let narrow = Dying {
            sprite_width: 3.0,
            ..dying(10)
        };
        effects.step(&[narrow], &scene(), &looks(), &mut chance);
        assert_eq!(&chance.asked[..3], [1, 2, 2]);
        assert_eq!(effects.explosions()[0].at, at(99.0, 200.0));
    }

    #[test]
    fn nothing_goes_off_in_the_last_two_ticks_or_without_an_explode1() {
        let mut effects = Effects::default();
        let mut chance = Script::default();
        let quiet = Dying {
            explosion: None,
            ..dying(10)
        };
        effects.step(
            &[dying(2), dying(1), dying(0), quiet],
            &scene(),
            &looks(),
            &mut chance,
        );
        assert!(chance.asked.is_empty(), "{:?}", chance.asked);
        assert_eq!(effects.explosions(), []);
    }

    /// The scene: the player at the origin, debris scattered, nothing
    /// capped.
    /// Explosion `snd ` `id`, heard from `offset`.
    fn explosion_sound(id: i16, offset: (i32, i32)) -> CombatSound {
        CombatSound {
            sound: SoundId(id),
            offset,
            priority: EXPLOSION_PRIORITY,
            until_stopped: false,
        }
    }

    /// Weapon `snd ` `id` fired at an NPC pick's priority, 4, heard from
    /// `offset`.
    fn fire_sound(id: i16, offset: (i32, i32)) -> CombatSound {
        CombatSound {
            sound: SoundId(id),
            offset,
            priority: 4,
            until_stopped: false,
        }
    }

    fn scene() -> Scene {
        Scene {
            player: PLAYER,
            debris: true,
            caps: false,
            looped: LoopedSounds::Anywhere,
        }
    }

    /// The scene with each firer's looped weapon waiting on its own.
    fn per_firer() -> Scene {
        Scene {
            looped: LoopedSounds::PerFirer,
            ..scene()
        }
    }

    /// The scene under the caps.
    fn capped() -> Scene {
        Scene {
            caps: true,
            ..scene()
        }
    }

    /// The scene, its player at `player`.
    fn heard_at(player: Point) -> Scene {
        Scene { player, ..scene() }
    }

    /// Where NPC 1 is.
    const NPC_AT: Vec2 = Vec2::new(300.0, -400.0);

    /// `ship` firing `weapon` from where it is: the player at the origin,
    /// any NPC at [`NPC_AT`].
    fn fired(ship: ShipRef, weapon: i16) -> CombatEvent {
        CombatEvent::Fired {
            ship,
            weapon: WeaponId(weapon),
            at: if ship == ShipRef::Player {
                Vec2::ZERO
            } else {
                NPC_AT
            },
            sound: Some(FireSound::at(4)),
        }
    }

    const NPC: ShipRef = ShipRef::Npc(NpcId(1));

    #[test]
    fn a_fired_event_with_no_sound_plays_nothing() {
        let mut effects = Effects::default();
        let mut chance = Script::default();
        let pick = |sound| CombatEvent::Fired {
            ship: NPC,
            weapon: WeaponId(128),
            at: NPC_AT,
            sound,
        };
        let first = Some(FireSound::at(4));
        for event in [pick(first), pick(None), pick(None)] {
            effects.apply(&event, &scene(), &looks(), &mut chance);
        }
        assert_eq!(
            effects.take_sounds(),
            [fire_sound(208, (300, -400))],
            "once a call"
        );
    }

    #[test]
    fn a_weapon_fired_sounds_from_where_it_was_fired() {
        let mut effects = Effects::default();
        let mut chance = Script::default();
        for event in [
            fired(ShipRef::Player, 128),
            fired(NPC, 128),
            fired(NPC, 147),
            fired(NPC, 199),
            CombatEvent::Fired {
                ship: NPC,
                weapon: WeaponId(128),
                at: Vec2::new(-5.0, 7.0),
                sound: Some(FireSound::at(4)),
            },
        ] {
            effects.apply(&event, &scene(), &looks(), &mut chance);
        }
        assert_eq!(
            effects.take_sounds(),
            [
                fire_sound(208, (0, 0)),
                fire_sound(208, (300, -400)),
                fire_sound(208, (-5, 7))
            ],
            "silent or unread: unheard"
        );
        assert_eq!(effects.take_sounds(), [], "taken");
        assert_eq!(effects.explosions(), []);
    }

    #[test]
    fn a_sound_is_heard_from_where_the_player_is() {
        let scene = Scene {
            player: at(10.0, 20.0),
            ..scene()
        };
        let mut effects = Effects::default();
        effects.apply(&fired(NPC, 128), &scene, &looks(), &mut Script::default());
        assert_eq!(effects.take_sounds(), [fire_sound(208, (290, -420))]);
    }

    /// What `firers` firing `weapon` every step for `steps` steps in
    /// `scene` are heard as: each step's sounds, each firer's separately.
    fn heard_in(
        scene: &Scene,
        weapon: i16,
        firers: &[ShipRef],
        steps: u32,
    ) -> Vec<Vec<Vec<CombatSound>>> {
        let mut effects = Effects::default();
        let mut heard = vec![Vec::new(); firers.len()];
        for _ in 0..steps {
            effects.step(&[], scene, &looks(), &mut Script::default());
            for (n, &firer) in firers.iter().enumerate() {
                effects.apply(
                    &fired(firer, weapon),
                    scene,
                    &looks(),
                    &mut Script::default(),
                );
                heard[n].push(effects.take_sounds());
            }
        }
        heard
    }

    /// The steps of `steps` on which `firers` firing `weapon` every step
    /// are heard in `scene`, each firer's separately.
    fn heard_on_in(scene: &Scene, weapon: i16, firers: &[ShipRef], steps: u32) -> Vec<Vec<u32>> {
        heard_in(scene, weapon, firers, steps)
            .into_iter()
            .map(|by_step| {
                (0..steps)
                    .filter(|&step| !by_step[step as usize].is_empty())
                    .collect()
            })
            .collect()
    }

    /// The steps on which `firers` are heard, each firer's looped weapon
    /// waiting on its own.
    fn heard_on(weapon: i16, firers: &[ShipRef], steps: u32) -> Vec<Vec<u32>> {
        heard_on_in(&per_firer(), weapon, firers, steps)
    }

    #[test]
    fn by_default_a_looped_weapon_is_heard_every_shot_marked_to_wait_on_any_voice() {
        let heard = heard_in(&scene(), 155, &[NPC, ShipRef::Player], 3);
        for (firer, offset) in [(0, (300, -400)), (1, (0, 0))] {
            let looped = CombatSound {
                sound: SoundId(205),
                offset,
                priority: 4,
                until_stopped: true,
            };
            assert_eq!(heard[firer], vec![vec![looped]; 3], "firer {firer}");
        }
        let mut effects = Effects::default();
        effects.apply(&fired(NPC, 155), &scene(), &looks(), &mut Script::default());
        assert_eq!(
            effects,
            Effects {
                sounds: effects.sounds.clone(),
                ..Effects::default()
            },
            "nothing kept by firer"
        );
    }

    #[test]
    fn a_sound_not_looped_never_waits() {
        let heard = heard_in(&scene(), 128, &[NPC], 1);
        assert_eq!(heard, [[vec![fire_sound(208, (300, -400))]]]);
    }

    #[test]
    fn a_looped_sound_fired_with_no_test_never_waits() {
        for scene in [scene(), per_firer()] {
            let mut effects = Effects::default();
            let launch = CombatEvent::Fired {
                ship: NPC,
                weapon: WeaponId(155),
                at: NPC_AT,
                sound: Some(FireSound {
                    priority: 5,
                    unless_playing: false,
                }),
            };
            for _ in 0..2 {
                effects.apply(&launch, &scene, &looks(), &mut Script::default());
            }
            let launched = CombatSound {
                sound: SoundId(205),
                offset: (300, -400),
                priority: 5,
                until_stopped: false,
            };
            assert_eq!(effects.take_sounds(), [launched; 2], "{:?}", scene.looped);
        }
    }

    #[test]
    fn a_weapons_sound_carries_its_fire_sounds_priority() {
        let mut effects = Effects::default();
        let at_priority = |priority| CombatEvent::Fired {
            ship: NPC,
            weapon: WeaponId(128),
            at: NPC_AT,
            sound: Some(FireSound::at(priority)),
        };
        for priority in [3, 6] {
            effects.apply(
                &at_priority(priority),
                &scene(),
                &looks(),
                &mut Script::default(),
            );
        }
        let priorities: Vec<u16> = effects.take_sounds().iter().map(|s| s.priority).collect();
        assert_eq!(priorities, [3, 6]);
        assert_eq!(EXPLOSION_PRIORITY, 6);
    }

    #[test]
    fn a_looped_weapon_firing_every_step_is_heard_once_its_sound_has_played_out() {
        assert_eq!(heard_on(155, &[NPC], 30), [vec![0, 14, 28]]);
        assert_eq!(heard_on(146, &[NPC], 7), [vec![0, 3, 6]], "a looped beam");
        let heard = heard_in(&per_firer(), 155, &[NPC], 1);
        assert!(
            heard[0][0].iter().all(|sound| !sound.until_stopped),
            "{heard:?}"
        );
    }

    #[test]
    fn each_firer_hears_its_own_looped_weapon_on_its_own_schedule() {
        let scene = per_firer();
        let mut effects = Effects::default();
        let mut heard = Vec::new();
        for step in 0..20 {
            effects.step(&[], &scene, &looks(), &mut Script::default());
            if step % 14 == 0 {
                effects.apply(&fired(NPC, 155), &scene, &looks(), &mut Script::default());
            }
            if step >= 5 {
                effects.apply(
                    &fired(ShipRef::Player, 155),
                    &scene,
                    &looks(),
                    &mut Script::default(),
                );
            }
            heard.push(
                effects
                    .take_sounds()
                    .iter()
                    .map(|sound| sound.offset)
                    .collect::<Vec<_>>(),
            );
        }
        let on = |offset: (i32, i32)| -> Vec<usize> {
            (0..20)
                .filter(|&step| heard[step].contains(&offset))
                .collect()
        };
        assert_eq!(on((300, -400)), [0, 14]);
        assert_eq!(on((0, 0)), [5, 19]);
    }

    #[test]
    fn a_weapon_not_looped_or_of_no_known_length_is_heard_every_shot() {
        assert_eq!(heard_on(128, &[NPC], 4), [vec![0, 1, 2, 3]], "not looped");
        assert_eq!(heard_on(148, &[NPC], 4), [vec![0, 1, 2, 3]], "no length");
    }

    #[test]
    fn clearing_forgets_the_looped_sounds_playing() {
        let scene = per_firer();
        let mut effects = Effects::default();
        effects.apply(&fired(NPC, 155), &scene, &looks(), &mut Script::default());
        effects.clear();
        effects.apply(&fired(NPC, 155), &scene, &looks(), &mut Script::default());
        assert_eq!(effects.take_sounds().len(), 1, "heard afresh");
    }

    #[test]
    fn a_missile_shot_down_or_a_shot_fizzling_explodes_as_bööm_128_without_a_sound() {
        let at_ = Vec2::new(30.0, 40.0);
        for event in [
            CombatEvent::ShotDown { at: at_ },
            CombatEvent::Fizzled { at: at_ },
        ] {
            let mut effects = Effects::default();
            let mut chance = Script::default();
            effects.apply(&event, &scene(), &looks(), &mut chance);
            assert_eq!(
                effects.explosions(),
                [Explosion {
                    boom: BoomId(128),
                    at: at(30.0, 40.0),
                    frame: 0.0,
                    delay: 0.0
                }],
                "{event:?}"
            );
            assert_eq!(effects.take_sounds(), [], "{event:?}");
            assert!(chance.asked.is_empty());
        }
        assert_eq!(TYPE_ZERO, BoomId(128));
    }

    #[test]
    fn a_hit_shows_nothing_of_its_own() {
        let mut effects = Effects::default();
        let mut chance = Script::default();
        let event = CombatEvent::Hit {
            ship: NPC,
            weapon: Some(WeaponId(128)),
            at: Vec2::new(30.0, 40.0),
            passes_shields: false,
            late: false,
        };
        effects.apply(&event, &scene(), &looks(), &mut chance);
        assert_eq!(effects, Effects::default());
    }

    #[test]
    fn a_weapons_explosion_goes_off_and_sounds_once() {
        let mut effects = Effects::default();
        let mut chance = Script::default();
        let event = CombatEvent::Exploded {
            at: Vec2::new(30.0, 40.0),
            explosion: blast(130, true),
            size: 0.0,
        };
        effects.apply(&event, &scene(), &looks(), &mut chance);
        assert_eq!(effects.explosions().len(), 1, "no size, no extras");
        assert_eq!(effects.explosions()[0].at, at(30.0, 40.0));
        assert_eq!(effects.take_sounds(), [explosion_sound(302, (30, 40))]);
        assert!(chance.asked.is_empty(), "{:?}", chance.asked);
    }

    #[test]
    fn a_ys_explosion_goes_off_unheard_its_sound_the_sessions() {
        let mut effects = Effects::default();
        let mut chance = Script::default();
        let event = CombatEvent::ExplodedHeard {
            at: Vec2::new(30.0, 40.0),
            explosion: blast(130, true),
        };
        effects.apply(&event, &scene(), &looks(), &mut chance);
        assert_eq!(effects.explosions().len(), 1, "of size 0, no extras");
        assert_eq!(effects.explosions()[0].at, at(30.0, 40.0));
        assert_eq!(effects.take_sounds(), [], "the session sounded it");
    }

    /// Weapon explosion `explosion` of `size` at (30, 40), applied: the
    /// effects, and what the chance was asked.
    fn weapon_explosion(explosion: Blast, size: f32) -> (Effects, Vec<u32>) {
        let mut effects = Effects::default();
        let mut chance = Script::default();
        let event = CombatEvent::Exploded {
            at: Vec2::new(30.0, 40.0),
            explosion,
            size,
        };
        effects.apply(&event, &scene(), &looks(), &mut chance);
        (effects, chance.asked)
    }

    #[test]
    fn a_hellhounds_explosion_of_size_145_scatters_5_small_and_23_large_extras() {
        let (mut effects, asked) = weapon_explosion(blast(130, true), 145.0);
        let booms: Vec<BoomId> = effects.explosions().iter().map(|e| e.boom).collect();
        let mut expected = vec![SMALL_EXTRA; 5];
        expected.extend([LARGE_EXTRA; 23]);
        expected.push(BoomId(130));
        assert_eq!(booms, expected, "trunc(5.8) small, trunc(23.2) large");
        // Within half the size and delayed below 8, then within the size
        // and delayed below 16.
        let mut reaches = [72, 72, 8].repeat(5);
        reaches.extend([145, 145, 16].repeat(23));
        assert_eq!(asked, reaches);
        for small in &effects.explosions()[..5] {
            assert_eq!(small.at, at(30.0 + 71.0 - 36.25, 40.0 + 71.0 - 36.25));
            assert_eq!(small.delay, 11.0);
        }
        for large in &effects.explosions()[5..28] {
            assert_eq!(large.at, at(30.0 + 144.0 - 72.5, 40.0 + 144.0 - 72.5));
            assert_eq!(large.delay, 23.0);
        }
        assert_eq!(effects.explosions()[28].at, at(30.0, 40.0));
        assert_eq!(
            effects.take_sounds(),
            [explosion_sound(302, (30, 40))],
            "only the main one is heard"
        );
    }

    #[test]
    fn a_weapons_plain_explosion_or_one_of_no_size_scatters_none() {
        for (explosion, size) in [(blast(130, false), 145.0), (blast(130, true), 0.0)] {
            let (effects, asked) = weapon_explosion(explosion, size);
            assert_eq!(effects.explosions().len(), 1, "{explosion:?} {size}");
            assert!(asked.is_empty(), "{asked:?}");
        }
    }

    #[test]
    fn being_disabled_or_starting_to_break_up_shows_nothing_at_once() {
        let mut effects = Effects::default();
        let mut chance = Script::default();
        for event in [
            CombatEvent::Disabled { ship: NPC },
            CombatEvent::BreakingUp {
                ship: NPC,
                at: Vec2::ZERO,
                explosion: Some(blast(130, false)),
            },
        ] {
            effects.apply(&event, &scene(), &looks(), &mut chance);
        }
        assert_eq!(effects, Effects::default());
        assert!(chance.asked.is_empty(), "{:?}", chance.asked);
    }

    fn destroyed(explosion: Option<Blast>, size: f32) -> CombatEvent {
        CombatEvent::Destroyed {
            ship: NPC,
            ship_type: nova_sim::ShipId(129),
            at: Vec2::new(200.0, 100.0),
            velocity: Vec2::new(3.0, -1.0),
            explosion,
            size,
        }
    }

    #[test]
    fn a_destroyed_ship_explodes_with_its_size_and_scatters_debris() {
        let mut effects = Effects::default();
        let mut chance = Script::default();
        effects.apply(
            &destroyed(Some(blast(130, true)), 57.0),
            &scene(),
            &looks(),
            &mut chance,
        );
        // trunc(2.28) small and trunc(9.12) large extras, then the main.
        assert_eq!(effects.explosions().len(), 12);
        assert_eq!(effects.explosions()[11].boom, BoomId(130));
        assert_eq!(chance.asked[..3], [28, 28, 8]);
        assert_eq!(chance.asked[6..9], [57, 57, 16]);
        // Each piece draws its direction, its speed and its life.
        let debris = &chance.asked[33..];
        assert_eq!(debris, [360, 101, 31].repeat(16));
        assert_eq!(effects.particles().len(), 16);
        assert_eq!(
            (DEBRIS_COUNT, DEBRIS_SPEED, DEBRIS_SPEED_VARIATION),
            (16, 2.0, 50)
        );
        assert_eq!((DEBRIS_LIFE_MIN, DEBRIS_LIFE_MAX), (15, 45));
    }

    #[test]
    fn a_destroyed_ship_scatters_no_debris_when_the_scene_draws_none() {
        let mut effects = Effects::default();
        let mut chance = Script::default();
        let scene = Scene {
            debris: false,
            ..scene()
        };
        effects.apply(
            &destroyed(Some(blast(130, false)), 57.0),
            &scene,
            &looks(),
            &mut chance,
        );
        assert_eq!(effects.explosions().len(), 1, "its Explode2 still");
        assert_eq!(effects.particles(), []);
        assert!(chance.asked.is_empty(), "{:?}", chance.asked);
    }

    #[test]
    fn debris_drifts_with_the_ships_velocity_plus_its_own_and_expires() {
        let mut effects = Effects::default();
        // Straight up at 2 x (100 + 0 - 50)%: 1 pixel a tick, 15 ticks;
        // then right at 2 x 150%: 3 pixels a tick, 45 ticks.
        let mut draws = vec![0, 0, 0, 90, 100, 30];
        draws.extend([0, 50, 0].repeat(14));
        let mut chance = Script::of(&draws);
        effects.apply(&destroyed(None, 0.0), &scene(), &looks(), &mut chance);
        assert_eq!(effects.explosions(), [], "no Explode2");
        let first = effects.particles()[0];
        assert_eq!(first.at, at(200.0, 100.0));
        assert!((first.velocity.x - 3.0).abs() < 1e-5, "{first:?}");
        assert!((first.velocity.y + 2.0).abs() < 1e-5, "{first:?}");
        assert_eq!(first.life, 15);
        let second = effects.particles()[1];
        assert!((second.velocity.x - 6.0).abs() < 1e-5, "{second:?}");
        assert!((second.velocity.y + 1.0).abs() < 1e-5, "{second:?}");
        assert_eq!(second.life, 45);
        let third = effects.particles()[2];
        assert!((third.velocity.y + 3.0).abs() < 1e-5, "up at 2: {third:?}");
        steps(&mut effects, 1);
        let moved = effects.particles()[0];
        assert!((moved.at.x - 203.0).abs() < 1e-4 && (moved.at.y - 98.0).abs() < 1e-4);
        assert_eq!(moved.life, 14);
        steps(&mut effects, 14);
        assert_eq!(effects.particles().len(), 1, "the first expired");
        steps(&mut effects, 30);
        assert_eq!(effects.particles(), []);
    }

    #[test]
    fn explosions_are_drawn_see_through_and_debris_as_dots() {
        let mut effects = Effects::default();
        exploded(&mut effects, 130, at(10.0, 20.0));
        steps(&mut effects, 2);
        let mut chance = Script::of(&[0, 0, 0]);
        effects.apply(&destroyed(None, 0.0), &scene(), &looks(), &mut chance);
        let camera = Camera::centred_on(at(10.0, 20.0));
        let mut list = DrawList::new();
        effects.draw(&mut list, &camera, &looks());
        let commands: Vec<DrawCommand> = list.iter().cloned().collect();
        assert_eq!(
            commands[0],
            DrawCommand::Sprite {
                image: ImageKey::sprite(402, 2),
                center: camera.world_to_screen(at(10.0, 20.0)),
                tint: translucent(),
                blend: crate::Blend::Normal,
            }
        );
        assert_eq!(commands.len(), 17);
        assert_eq!(
            commands[1],
            DrawCommand::Dot {
                center: camera.world_to_screen(at(200.0, 100.0)),
                size: DEBRIS_SIZE,
                color: Color {
                    a: particle_alpha(15),
                    ..DEBRIS_COLOR
                },
            },
            "15 ticks of life: fading"
        );
    }

    #[test]
    fn a_particle_fades_over_its_last_32_ticks() {
        assert_eq!(particle_alpha(40), 255, "opaque above 32");
        assert_eq!(particle_alpha(33), 255);
        assert_eq!(particle_alpha(32), 255, "full at 32");
        assert_eq!(particle_alpha(16), 128, "half");
        assert_eq!(particle_alpha(1), 8, "a 32nd");
        assert_eq!(particle_alpha(0), 0);
        let mut effects = Effects::default();
        let mut chance = Script::of(&[0, 0, 0]);
        effects.apply(&burst(1, 0, (40, 40), 0.0), &scene(), &looks(), &mut chance);
        let alphas = |effects: &Effects| {
            let mut list = DrawList::new();
            effects.draw(&mut list, &Camera::centred_on(PLAYER), &looks());
            list.iter()
                .filter_map(|command| match *command {
                    DrawCommand::Dot { color, .. } => Some(color.a),
                    _ => None,
                })
                .collect::<Vec<u8>>()
        };
        assert_eq!(alphas(&effects), [255], "40 ticks left");
        steps(&mut effects, 24);
        assert_eq!(alphas(&effects), [128], "16 left");
        steps(&mut effects, 15);
        assert_eq!(alphas(&effects), [8], "1 left");
    }

    /// A burst of `count` at (10, 20), 2 pixels a tick varied by
    /// `variation` percent, living `life`, orange, spread over `spread`.
    fn burst(count: u32, variation: u32, life: (u32, u32), spread: f32) -> CombatEvent {
        CombatEvent::Particles(ParticleBurst {
            at: Vec2::new(10.0, 20.0),
            speed: 2.0,
            variation,
            life,
            color: 0x00FF_8000,
            count,
            spread,
        })
    }

    const ORANGE: Color = Color::rgba(0xFF, 0x80, 0x00, 255);

    #[test]
    fn a_burst_throws_off_its_count_from_where_it_went_off_in_its_colour() {
        let mut effects = Effects::default();
        // Each piece draws its direction, its speed and its life.
        let mut chance = Script::of(&[0, 0, 0, 90, 40, 7]);
        effects.apply(
            &burst(2, 20, (30, 37), 0.0),
            &scene(),
            &looks(),
            &mut chance,
        );
        assert_eq!(chance.asked, [360, 41, 8, 360, 41, 8]);
        let pieces = effects.particles();
        assert_eq!(pieces.len(), 2);
        assert_eq!(pieces[0].at, at(10.0, 20.0));
        assert!((pieces[0].velocity.y + 1.6).abs() < 1e-5, "{:?}", pieces[0]);
        assert!(pieces[0].velocity.x.abs() < 1e-5, "{:?}", pieces[0]);
        assert_eq!((pieces[0].life, pieces[0].color), (30, ORANGE));
        assert!((pieces[1].velocity.x - 2.4).abs() < 1e-5, "{:?}", pieces[1]);
        assert_eq!(pieces[1].life, 37);
        assert_eq!(effects.explosions(), []);
        assert_eq!(effects.take_sounds(), []);
    }

    #[test]
    fn a_burst_varies_no_speed_outside_1_to_99_percent_and_no_life_without_a_range() {
        for variation in [0, 100, 150] {
            let mut effects = Effects::default();
            let mut chance = Script::of(&[90]);
            effects.apply(
                &burst(1, variation, (5, 5), 0.0),
                &scene(),
                &looks(),
                &mut chance,
            );
            assert_eq!(chance.asked, [360], "{variation}");
            let piece = effects.particles()[0];
            assert!(
                (piece.velocity.x - 2.0).abs() < 1e-5,
                "{variation}: {piece:?}"
            );
            assert_eq!(piece.life, 5);
        }
        let mut effects = Effects::default();
        let mut chance = Script::default();
        effects.apply(&burst(1, 0, (9, 4), 0.0), &scene(), &looks(), &mut chance);
        assert_eq!(chance.asked, [360], "the least life above the most");
        assert_eq!(effects.particles()[0].life, 9);
    }

    #[test]
    fn a_spread_burst_starts_each_piece_up_to_its_spread_off() {
        let mut effects = Effects::default();
        // Its direction, speed and life, then its offset's direction and
        // hundredths of a pixel.
        let mut chance = Script::of(&[0, 0, 0, 90, 999]);
        effects.apply(
            &burst(1, 20, (30, 37), 10.0),
            &scene(),
            &looks(),
            &mut chance,
        );
        assert_eq!(chance.asked, [360, 41, 8, 360, 1000]);
        let piece = effects.particles()[0];
        assert!((piece.at.x - 19.99).abs() < 1e-3, "{piece:?}");
        assert!((piece.at.y - 20.0).abs() < 1e-3, "{piece:?}");
    }

    #[test]
    fn burst_pieces_move_by_their_own_velocity_and_expire() {
        let mut effects = Effects::default();
        let mut chance = Script::of(&[90]);
        effects.apply(&burst(1, 0, (2, 2), 0.0), &scene(), &looks(), &mut chance);
        steps(&mut effects, 1);
        let moved = effects.particles()[0];
        assert!((moved.at.x - 12.0).abs() < 1e-4, "{moved:?}");
        assert!((moved.at.y - 20.0).abs() < 1e-4, "{moved:?}");
        assert_eq!(moved.life, 1);
        steps(&mut effects, 1);
        assert_eq!(effects.particles(), []);
    }

    #[test]
    fn a_bursts_pieces_are_drawn_as_dots_of_its_colour_and_debris_of_its_own() {
        let mut effects = Effects::default();
        effects.apply(
            &burst(3, 20, (30, 37), 0.0),
            &scene(),
            &looks(),
            &mut Script::default(),
        );
        effects.apply(
            &destroyed(None, 0.0),
            &scene(),
            &looks(),
            &mut Script::default(),
        );
        let mut list = DrawList::new();
        effects.draw(&mut list, &Camera::centred_on(PLAYER), &looks());
        let colours: Vec<Color> = list
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Dot { color, .. } => Some(*color),
                _ => None,
            })
            .collect();
        let mut expected = vec![ORANGE; 3];
        expected.extend([DEBRIS_COLOR; 16]);
        assert_eq!(colours, expected);
    }

    #[test]
    fn clearing_takes_everything_away() {
        let mut effects = Effects::default();
        exploded(&mut effects, 130, at(0.0, 0.0));
        effects.apply(
            &destroyed(None, 0.0),
            &scene(),
            &looks(),
            &mut Script::default(),
        );
        effects.clear();
        assert_eq!(effects, Effects::default());
    }
}
