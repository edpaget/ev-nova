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
//!   frame, unseen. At most [`MAX_EXPLOSIONS`] are kept: one more is
//!   dropped.
//! - An explosion of a code of 1000 or more, with a size, first scatters
//!   `trunc(0.04 x size)` explosions of `bööm` 129 within a quarter of the
//!   size of it, delayed 4-11, and `trunc(0.16 x size)` of `bööm` 128
//!   within half the size, delayed 8-23 ([`Effects::explode`]). Only the
//!   main one sounds. A weapon's explosion has no size.
//! - A ship breaking up, while more than 2 ticks are left, sets off its
//!   `Explode1` about it at random, more often as its end nears, a quarter
//!   of them heard ([`Effects::step`]).
//! - A destroyed ship goes off in its `Explode2`, with its size, and, as
//!   the phase asks though the original scatters none, [`DEBRIS_COUNT`]
//!   particles of debris, by the original's particle model: each in a
//!   random direction at [`DEBRIS_SPEED`] ± [`DEBRIS_SPEED_VARIATION`] %,
//!   living 15-45 ticks, and carried on with the ship's velocity.
//! - A weapon firing sounds its `snd `, and an explosion its own, each
//!   heard from where it happened ([`CombatSound`]). A weapon whose sound
//!   loops while its beam lasts (`Flags` 0x0010) is heard once a beam: not
//!   again while a beam of it from the same ship was already live before
//!   the step, which the caller says ([`Scene::beams_before`]).
//!
//! The effects draw on their own [`Chance`], never on the simulation's.

use nova_sim::combat::weapon::Explosion as Blast;
use nova_sim::{Chance, CombatEvent, ShipRef, Vec2};

use super::catalog::{BoomId, Looks, WeaponId};
use super::weapons::{LOOPED_SOUND, translucent};
use crate::sound::CombatSound;
use crate::system::camera::Camera;
use crate::{Color, DrawList, ImageKey, Point};

/// The most explosions shown at once.
pub const MAX_EXPLOSIONS: usize = 32;
/// The small extra explosions, `bööm` 129.
pub const SMALL_EXTRA: BoomId = BoomId(129);
/// The large extra explosions, `bööm` 128.
pub const LARGE_EXTRA: BoomId = BoomId(128);
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

/// `sound`, heard by a player at `player` from `at`.
fn heard(sound: nova_sim::SoundId, at: Point, player: Point) -> CombatSound {
    CombatSound {
        sound,
        offset: (
            (at.x - player.x).round() as i32,
            (at.y - player.y).round() as i32,
        ),
    }
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

/// A piece of debris.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Particle {
    /// Where it is, in world units.
    pub at: Point,
    /// How far it moves a tick.
    pub velocity: Point,
    /// The ticks it has left.
    pub life: u32,
}

/// Where a step's events are placed and heard from.
#[derive(Clone, Copy, Debug)]
pub struct Scene<'a> {
    /// Where the player is: sounds are heard from here.
    pub player: Point,
    /// Where each ship is.
    pub ships: &'a [(ShipRef, Point)],
    /// The beams live before the step: each firer and its weapon.
    pub beams_before: &'a [(ShipRef, WeaponId)],
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
    /// The ticks before it is destroyed.
    pub ticks_left: u32,
}

/// The fight's explosions, debris and sounds.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Effects {
    explosions: Vec<Explosion>,
    debris: Vec<Particle>,
    sounds: Vec<CombatSound>,
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
            CombatEvent::Fired { ship, weapon, .. } => {
                let Some(look) = looks.weapon(weapon) else {
                    return;
                };
                let looping =
                    look.flags & LOOPED_SOUND != 0 && scene.beams_before.contains(&(ship, weapon));
                let firer = scene.ships.iter().find(|(id, _)| *id == ship);
                if let (Some(sound), Some(&(_, at)), false) = (look.sound, firer, looping) {
                    self.sounds.push(heard(sound, at, scene.player));
                }
            }
            CombatEvent::Exploded { at, explosion } => {
                self.explode(explosion, point(at), 0.0, scene.player, looks, chance);
            }
            CombatEvent::Destroyed {
                at,
                velocity,
                explosion,
                size,
                ..
            } => {
                if let Some(explosion) = explosion {
                    self.explode(explosion, point(at), size, scene.player, looks, chance);
                }
                self.scatter(point(at), point(velocity), chance);
            }
            CombatEvent::Disabled { .. }
            | CombatEvent::BreakingUp { .. }
            | CombatEvent::ShotDown { .. } => {}
        }
    }

    /// Scatters [`DEBRIS_COUNT`] pieces of debris from `at`, carried on
    /// with `velocity`.
    fn scatter(&mut self, at: Point, velocity: Point, chance: &mut dyn Chance) {
        let variation = DEBRIS_SPEED_VARIATION;
        for _ in 0..DEBRIS_COUNT {
            let heading = chance.below(360) as f32;
            let percent = 100 + chance.below(2 * variation + 1);
            let speed = DEBRIS_SPEED * (percent as f32 - variation as f32) / 100.0;
            let life = DEBRIS_LIFE_MIN + chance.below(DEBRIS_LIFE_MAX - DEBRIS_LIFE_MIN + 1);
            let (sin, cos) = heading.to_radians().sin_cos();
            self.debris.push(Particle {
                at,
                velocity: Point::new(velocity.x + sin * speed, velocity.y - cos * speed),
                life,
            });
        }
    }

    /// Sets off `boom` at `at`, hidden for `delay`, unless the pool is full
    /// or its look was not read; whether it went off.
    fn spawn(&mut self, boom: BoomId, at: Point, delay: f32, looks: &Looks) -> bool {
        let spawned = self.explosions.len() < MAX_EXPLOSIONS && looks.boom(boom).is_some();
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

    /// Sets off `explosion` at `at` with `size`, heard from `player` (see
    /// the module docs).
    pub fn explode(
        &mut self,
        explosion: Blast,
        at: Point,
        size: f32,
        player: Point,
        looks: &Looks,
        chance: &mut dyn Chance,
    ) {
        if explosion.extra {
            let size = f64::from(size);
            let small = (SMALL_PER_SIZE * size) as u32;
            let small_reach = (0.5 * size) as u32;
            for _ in 0..small {
                let off =
                    |chance: &mut dyn Chance| f64::from(chance.below(small_reach)) - 0.25 * size;
                let (x, y) = (off(chance), off(chance));
                let delay = chance.below(8) + 4;
                self.spawn(SMALL_EXTRA, offset(at, x, y), delay as f32, looks);
            }
            let large = (LARGE_PER_SIZE * size) as u32;
            let large_reach = size as u32;
            for _ in 0..large {
                let off =
                    |chance: &mut dyn Chance| f64::from(chance.below(large_reach)) - 0.5 * size;
                let (x, y) = (off(chance), off(chance));
                let delay = chance.below(16) + 8;
                self.spawn(LARGE_EXTRA, offset(at, x, y), delay as f32, looks);
            }
        }
        if self.spawn(explosion.boom, at, 0.0, looks)
            && let Some(sound) = looks.boom(explosion.boom).and_then(|look| look.sound)
        {
            self.sounds.push(heard(sound, at, player));
        }
    }

    /// Advances the explosions and debris a step, then sets off the
    /// explosions of the ships `dying`, heard from `player`.
    pub fn step(&mut self, dying: &[Dying], player: Point, looks: &Looks, chance: &mut dyn Chance) {
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
        self.debris.retain_mut(|piece| {
            piece.at = Point::new(piece.at.x + piece.velocity.x, piece.at.y + piece.velocity.y);
            piece.life = piece.life.saturating_sub(1);
            piece.life > 0
        });
        for ship in dying {
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
            let spawned = self.spawn(explosion.boom, at, 0.0, looks);
            let audible = chance.below(4) == 0;
            if spawned
                && audible
                && let Some(sound) = looks.boom(explosion.boom).and_then(|look| look.sound)
            {
                self.sounds.push(heard(sound, at, player));
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
        for piece in &self.debris {
            list.dot(camera.world_to_screen(piece.at), DEBRIS_SIZE, DEBRIS_COLOR);
        }
    }

    /// The explosions going off.
    #[must_use]
    pub fn explosions(&self) -> &[Explosion] {
        &self.explosions
    }

    /// The debris flying.
    #[must_use]
    pub fn debris(&self) -> &[Particle] {
        &self.debris
    }

    /// The sounds made since they were last taken; taking them empties the
    /// list.
    pub fn take_sounds(&mut self) -> Vec<CombatSound> {
        std::mem::take(&mut self.sounds)
    }

    /// Clears everything: the explosions, the debris and the sounds.
    pub fn clear(&mut self) {
        self.explosions.clear();
        self.debris.clear();
        self.sounds.clear();
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::collections::{BTreeMap, VecDeque};
    use std::num::NonZeroU16;

    use nova_sim::{NpcId, SoundId};

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
                image_id,
                frames: NonZeroU16::new(frames).expect("non-zero"),
            }),
            advance,
            sound: sound.map(SoundId),
        }
    }

    /// `bööm` 128 (3 frames, silent), 129 (2 frames, silent), 130 (4
    /// frames, `snd ` 302), 135 (3 frames at 0.3 a step, `snd ` 300),
    /// 142 (no sheet, `snd ` 303); 140 cannot be read; weapon 128 sounds `snd ` 208, weapon 146 loops `snd ` 210, and
    /// weapon 147 is silent.
    fn looks() -> Looks {
        let weapon = |sound: Option<i16>, flags| {
            Ok(WeaponLook {
                sound: sound.map(SoundId),
                flags,
                ..WeaponLook::default()
            })
        };
        Looks {
            weapons: BTreeMap::from([
                (WeaponId(128), weapon(Some(208), 0)),
                (WeaponId(146), weapon(Some(210), LOOPED_SOUND)),
                (WeaponId(147), weapon(None, 0)),
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
        effects.explode(
            blast(boom, false),
            where_,
            0.0,
            PLAYER,
            &looks(),
            &mut Script::default(),
        );
    }

    fn frames(effects: &Effects) -> Vec<f32> {
        effects.explosions().iter().map(|e| e.frame).collect()
    }

    fn steps(effects: &mut Effects, n: u32) {
        for _ in 0..n {
            effects.step(&[], PLAYER, &looks(), &mut Script::default());
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
        assert_eq!(effects.debris(), []);
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
        assert_eq!(
            effects.take_sounds(),
            [CombatSound {
                sound: SoundId(303),
                offset: (3, 4),
            }]
        );
        assert_eq!(frames(&effects), [0.0]);
        let mut list = DrawList::new();
        effects.draw(&mut list, &Camera::centred_on(PLAYER), &looks());
        assert_eq!(list, DrawList::new());
        steps(&mut effects, 1);
        assert_eq!(effects.explosions(), [], "past its one frame");
    }

    #[test]
    fn at_most_32_explosions_go_off_at_once() {
        let mut effects = Effects::default();
        for i in 0..33 {
            exploded(&mut effects, 130, at(i as f32, 0.0));
        }
        assert_eq!(effects.explosions().len(), 32);
        assert_eq!(
            effects.explosions()[31].at,
            at(31.0, 0.0),
            "the 33rd dropped"
        );
        assert_eq!(effects.take_sounds().len(), 32);
        assert_eq!(MAX_EXPLOSIONS, 32);
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
            PLAYER,
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
            PLAYER,
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
            [CombatSound {
                sound: SoundId(302),
                offset: (1000, 500)
            }],
            "only the main one sounds"
        );
        let mut first = Effects::default();
        let mut chance = Script::of(&[0, 0, 0]);
        first.explode(
            blast(130, true),
            at(1000.0, 500.0),
            100.0,
            PLAYER,
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
            PLAYER,
            &looks(),
            &mut chance,
        );
        effects.explode(
            blast(130, false),
            at(0.0, 0.0),
            100.0,
            PLAYER,
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
        }
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
            effects.step(&[dying(ticks_left)], PLAYER, &looks(), &mut chance);
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
        effects.step(&[ship, ship], at(100.0, 0.0), &looks(), &mut chance);
        assert_eq!(chance.asked, [2, 24, 24, 4, 2, 24, 24, 4]);
        let explosions = effects.explosions();
        assert_eq!(explosions.len(), 2);
        assert_eq!(explosions[0].at, at(88.0, 211.0));
        assert_eq!(explosions[1].at, at(93.0, 193.0));
        assert_eq!((explosions[0].frame, explosions[0].delay), (0.0, 0.0));
        assert_eq!(
            effects.take_sounds(),
            [CombatSound {
                sound: SoundId(302),
                offset: (-12, 211)
            }],
            "the first heard, the second not"
        );
        assert_eq!(BREAKUP_SPREAD, 0.25);
    }

    #[test]
    fn a_narrow_ship_breaks_up_within_a_pixel() {
        let mut effects = Effects::default();
        let mut chance = Script::of(&[0, 0, 1]);
        let narrow = Dying {
            sprite_width: 3.0,
            ..dying(10)
        };
        effects.step(&[narrow], PLAYER, &looks(), &mut chance);
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
            PLAYER,
            &looks(),
            &mut chance,
        );
        assert!(chance.asked.is_empty(), "{:?}", chance.asked);
        assert_eq!(effects.explosions(), []);
    }

    /// The scene: the player at the origin and NPC 1 at (300, -400), with
    /// `beams` live before the step.
    fn scene(beams: &[(ShipRef, WeaponId)]) -> Scene<'_> {
        const SHIPS: [(ShipRef, Point); 2] = [
            (ShipRef::Player, PLAYER),
            (ShipRef::Npc(NpcId(1)), Point::new(300.0, -400.0)),
        ];
        Scene {
            player: PLAYER,
            ships: &SHIPS,
            beams_before: beams,
        }
    }

    fn fired(ship: ShipRef, weapon: i16) -> CombatEvent {
        CombatEvent::Fired {
            ship,
            weapon: WeaponId(weapon),
            at: Vec2::ZERO,
        }
    }

    const NPC: ShipRef = ShipRef::Npc(NpcId(1));

    #[test]
    fn a_weapon_fired_sounds_from_its_firer() {
        let mut effects = Effects::default();
        let mut chance = Script::default();
        for event in [
            fired(ShipRef::Player, 128),
            fired(NPC, 128),
            fired(NPC, 147),
            fired(ShipRef::Npc(NpcId(9)), 128),
            fired(NPC, 199),
        ] {
            effects.apply(&event, &scene(&[]), &looks(), &mut chance);
        }
        assert_eq!(
            effects.take_sounds(),
            [
                CombatSound {
                    sound: SoundId(208),
                    offset: (0, 0)
                },
                CombatSound {
                    sound: SoundId(208),
                    offset: (300, -400)
                }
            ],
            "silent, gone, or unread: unheard"
        );
        assert_eq!(effects.take_sounds(), [], "taken");
        assert_eq!(effects.explosions(), []);
    }

    #[test]
    fn a_sound_is_heard_from_where_the_player_is() {
        let ships = [(ShipRef::Player, at(10.0, 20.0)), (NPC, at(300.0, -400.0))];
        let scene = Scene {
            player: at(10.0, 20.0),
            ships: &ships,
            beams_before: &[],
        };
        let mut effects = Effects::default();
        effects.apply(&fired(NPC, 128), &scene, &looks(), &mut Script::default());
        assert_eq!(
            effects.take_sounds(),
            [CombatSound {
                sound: SoundId(208),
                offset: (290, -420)
            }]
        );
    }

    #[test]
    fn a_looped_weapon_is_heard_once_a_beam_and_not_while_it_was_already_live() {
        let looped = |live: &[(ShipRef, WeaponId)]| {
            let mut effects = Effects::default();
            effects.apply(
                &fired(NPC, 146),
                &scene(live),
                &looks(),
                &mut Script::default(),
            );
            effects.take_sounds().len()
        };
        assert_eq!(looped(&[]), 1, "a new beam");
        assert_eq!(looped(&[(NPC, WeaponId(146))]), 0, "already live");
        assert_eq!(
            looped(&[(ShipRef::Player, WeaponId(146))]),
            1,
            "another ship's"
        );
        assert_eq!(looped(&[(NPC, WeaponId(128))]), 1, "another weapon's");
        let plain = |live: &[(ShipRef, WeaponId)]| {
            let mut effects = Effects::default();
            effects.apply(
                &fired(NPC, 128),
                &scene(live),
                &looks(),
                &mut Script::default(),
            );
            effects.take_sounds().len()
        };
        assert_eq!(plain(&[(NPC, WeaponId(128))]), 1, "not looped: every shot");
    }

    #[test]
    fn a_weapons_explosion_goes_off_and_sounds_once() {
        let mut effects = Effects::default();
        let mut chance = Script::default();
        let event = CombatEvent::Exploded {
            at: Vec2::new(30.0, 40.0),
            explosion: blast(130, true),
        };
        effects.apply(&event, &scene(&[]), &looks(), &mut chance);
        assert_eq!(effects.explosions().len(), 1, "no size, no extras");
        assert_eq!(effects.explosions()[0].at, at(30.0, 40.0));
        assert_eq!(
            effects.take_sounds(),
            [CombatSound {
                sound: SoundId(302),
                offset: (30, 40)
            }]
        );
        assert!(chance.asked.is_empty(), "{:?}", chance.asked);
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
            effects.apply(&event, &scene(&[]), &looks(), &mut chance);
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
            &scene(&[]),
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
        assert_eq!(effects.debris().len(), 16);
        assert_eq!(
            (DEBRIS_COUNT, DEBRIS_SPEED, DEBRIS_SPEED_VARIATION),
            (16, 2.0, 50)
        );
        assert_eq!((DEBRIS_LIFE_MIN, DEBRIS_LIFE_MAX), (15, 45));
    }

    #[test]
    fn debris_drifts_with_the_ships_velocity_plus_its_own_and_expires() {
        let mut effects = Effects::default();
        // Straight up at 2 x (100 + 0 - 50)%: 1 pixel a tick, 15 ticks;
        // then right at 2 x 150%: 3 pixels a tick, 45 ticks.
        let mut draws = vec![0, 0, 0, 90, 100, 30];
        draws.extend([0, 50, 0].repeat(14));
        let mut chance = Script::of(&draws);
        effects.apply(&destroyed(None, 0.0), &scene(&[]), &looks(), &mut chance);
        assert_eq!(effects.explosions(), [], "no Explode2");
        let first = effects.debris()[0];
        assert_eq!(first.at, at(200.0, 100.0));
        assert!((first.velocity.x - 3.0).abs() < 1e-5, "{first:?}");
        assert!((first.velocity.y + 2.0).abs() < 1e-5, "{first:?}");
        assert_eq!(first.life, 15);
        let second = effects.debris()[1];
        assert!((second.velocity.x - 6.0).abs() < 1e-5, "{second:?}");
        assert!((second.velocity.y + 1.0).abs() < 1e-5, "{second:?}");
        assert_eq!(second.life, 45);
        let third = effects.debris()[2];
        assert!((third.velocity.y + 3.0).abs() < 1e-5, "up at 2: {third:?}");
        steps(&mut effects, 1);
        let moved = effects.debris()[0];
        assert!((moved.at.x - 203.0).abs() < 1e-4 && (moved.at.y - 98.0).abs() < 1e-4);
        assert_eq!(moved.life, 14);
        steps(&mut effects, 14);
        assert_eq!(effects.debris().len(), 1, "the first expired");
        steps(&mut effects, 30);
        assert_eq!(effects.debris(), []);
    }

    #[test]
    fn explosions_are_drawn_see_through_and_debris_as_dots() {
        let mut effects = Effects::default();
        exploded(&mut effects, 130, at(10.0, 20.0));
        steps(&mut effects, 2);
        let mut chance = Script::of(&[0, 0, 0]);
        effects.apply(&destroyed(None, 0.0), &scene(&[]), &looks(), &mut chance);
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
            }
        );
        assert_eq!(commands.len(), 17);
        assert_eq!(
            commands[1],
            DrawCommand::Dot {
                center: camera.world_to_screen(at(200.0, 100.0)),
                size: DEBRIS_SIZE,
                color: DEBRIS_COLOR,
            }
        );
    }

    #[test]
    fn clearing_takes_everything_away() {
        let mut effects = Effects::default();
        exploded(&mut effects, 130, at(0.0, 0.0));
        effects.apply(
            &destroyed(None, 0.0),
            &scene(&[]),
            &looks(),
            &mut Script::default(),
        );
        effects.clear();
        assert_eq!(effects, Effects::default());
    }
}
