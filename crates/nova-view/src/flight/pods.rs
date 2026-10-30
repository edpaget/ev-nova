//! The NPCs' escape pods: the decorative sprites a dying ship's crew and
//! person eject, from the session's
//! [`CombatEvent::PodEjected`](nova_sim::CombatEvent::PodEjected)s, as
//! `_SpawnEscapePod` (@0x45547-0x456cc) and `_HandleEscapePods`
//! (@0x39e48-0x3a063) show them in the `EV Nova` executable.
//!
//! - **The spawn** ([`EscapePods::spawn`]): the pod takes the ship's
//!   position and velocity, then `_Accel(Rand(360), (Rand(10) + 10) x
//!   0.1)` on top (0.1 at 0xdd110), a push of 1.0 to 1.9 pixels a tick in a
//!   direction of its own, and lives `Rand(100) + 150` ticks. It faces its
//!   velocity: frame `trunc(_CalcInterceptAngle(0, v x 100) x frames /
//!   360)` (100 at 0xdd060, 360 at 0xdd0f8) of shän 895's sheet, the
//!   escape pod's ([`ESCAPE_POD_SHIP`](nova_sim::ESCAPE_POD_SHIP)), of its
//!   `FramesPer` rotations, and keeps that frame. It takes its spawn
//!   frame's step at once, as `_HandleEscapePods` runs after `_HandleShip`
//!   in the frame (`_DoPlayGameWork` @0x44483, @0x44513).
//! - **The pool.** Under the original's caps
//!   ([`RuleKey::CombatCaps`](nova_sim::RuleKey::CombatCaps)'s engine
//!   reading, [`Scene::caps`](super::effects::Scene::caps)) there are
//!   [`MAX_PODS`] slots, a pod taking the first whose life is below 0
//!   (@0x45564-0x456bf), and one more is not kept; by default nothing is
//!   capped, and the pool is unbounded but still first-free. `_boxLayer`
//!   draws the pods in slot order (`_InitSprites` adds them once, and
//!   nothing moves them), so a new pod in a freed lower slot draws under
//!   an older one in a higher slot ([`EscapePods::pods`],
//!   [`EscapePods::draw`]). Either way the pod is heard ([`Effects`](super::effects::Effects)
//!   plays `snd ` 372 for each event), as the sound is `_HandleShip`'s.
//! - **Each step** ([`EscapePods::step`]): it drifts by its velocity and a
//!   tick comes off its life. It is held, unseen, at a life of 0, and its
//!   slot is free once the life is below 0.
//! - **The fade** ([`EscapePods::draw`]): opaque at a life of 32 or more
//!   (32.0 at 0xdd654), at alpha life/32 over its last 32 ticks, and not
//!   drawn at 0 or less. `_HandleEscapePods` sets the sprite's destination
//!   factor +0xa8 to `trunc(32 - life)`, its channel factors left at 0
//!   (`_SWCreateSprite` @0xa8d56-0xa8d71), so its draw proc
//!   (`_BlitPixieRLETranslucentDrawProc`, 0xbb3fc) gives `(src x life) >>
//!   5 | (dst x (32 - life)) >> 5` per channel: the OR crossfade,
//!   [`Blend::OrCrossfade`](crate::draw::Blend::OrCrossfade) at alpha
//!   life/32 ([`Pod::tint`]). An opaque pod is drawn plainly, as k = 32
//!   is the same copy.
//! - **Leaving the system** clears them ([`EscapePods::clear`]), as
//!   `_killShots` frees them on landing, a gate and the arrival
//!   (@0x39e75-0x39e84), and the view clears its effects then.
//!
//! The pods are drawn in their sprite layer, `_boxLayer` 0x3b53ec, after
//! the cargo boxes (`_InitSprites` @0x1fc71-0x1fd57, added @0x1fcfb):
//! above the disabled ships and below the smoke, the shots and the flying
//! ships ([`SpriteLayer::Boxes`](nova_sim::sprite_layer::SpriteLayer::Boxes)).
//!
//! A pod is murked at its position, seen from the player's ship
//! ([`murk`](super::murk); `_HandleSpriteMurkiness` @0x39f90-0x39fbc,
//! rdm:phase/play-feel-4/phase-17-murk-other-sprites), on top of its fade:
//! the murk writes the sprite's +0xb0/+0xb4 and the fade +0xa8, so the
//! two compose.
//!
//! The player's wrap carries them ([`EscapePods::carry`]), its velocity
//! kept.

use std::num::NonZeroU16;

use nova_sim::player_wrap::PlayerWrap;
use nova_sim::trig::intercept_angle;
use nova_sim::{Chance, Vec2};

use super::catalog::Looks;
use super::murk::{murk_tint, sprite_murk};
use super::slot_pool::SlotPool;
use crate::system::camera::Camera;
use crate::{Color, DrawList, Point};

/// Moves the place `at` if `wrap` reaches it.
pub(super) fn carry_point(wrap: &PlayerWrap, at: &mut Point) {
    let mut place = Vec2::new(at.x, at.y);
    if wrap.carry(&mut place) {
        *at = Point::new(place.x, place.y);
    }
}

/// The most pods kept at once under the caps: `_podData`'s 32 entries
/// (`cmpl $0x20` @0x456bc).
pub const MAX_PODS: usize = 32;
/// The life, in ticks, from which a pod is drawn opaque (32.0 at 0xdd654).
pub const POD_FADE: i32 = 32;
/// A pod's least life (`Rand(100) + 150`, @0x4556e-0x45584).
pub const POD_LIFE_MIN: u32 = 150;
/// How many lives a pod can roll above [`POD_LIFE_MIN`].
pub const POD_LIFE_ROLL: u32 = 100;

/// An escape pod adrift.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pod {
    /// Where it is, in world units.
    pub at: Point,
    /// How far it moves a tick.
    pub velocity: Point,
    /// The ticks it has left.
    pub life: i32,
    /// The frame of shän 895's sheet it shows.
    pub frame: u16,
}

impl Pod {
    /// How it is drawn: white at its alpha, or none at a life of 0 or less.
    #[must_use]
    pub fn tint(&self) -> Option<Color> {
        match self.life {
            ..=0 => None,
            life if life >= POD_FADE => Some(Color::WHITE),
            life => Some(Color::rgba(
                255,
                255,
                255,
                ((life * 255 + POD_FADE / 2) / POD_FADE) as u8,
            )),
        }
    }

    /// One step: it drifts by its velocity, and a tick comes off its life.
    fn step(&mut self) {
        self.at = Point::new(self.at.x + self.velocity.x, self.at.y + self.velocity.y);
        self.life -= 1;
    }
}

/// The frame of a sheet of `frames` rotations a pod moving at `velocity`
/// faces: the original's whole-degree heading of its velocity, times the
/// rotations in integers, then divided by 360 (@0x45676-0x4569d).
#[must_use]
pub fn pod_frame(velocity: Point, frames: NonZeroU16) -> u16 {
    let towards = Vec2::new(velocity.x * 100.0, velocity.y * 100.0);
    let angle = i32::from(intercept_angle(Vec2::ZERO, towards));
    let frame = angle * i32::from(frames.get()) / 360;
    u16::try_from(frame).unwrap_or(0) % frames.get()
}

/// The escape pods adrift.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct EscapePods {
    pods: SlotPool<Pod>,
}

impl EscapePods {
    /// Sets a pod adrift from a ship at `at` moving at `velocity`, drawing
    /// on `chance`, unless the `caps` hold the pool full; its sheet's
    /// rotations from `looks` (one without a sheet faces frame 0). It
    /// takes its first step at once.
    pub fn spawn(
        &mut self,
        at: Point,
        velocity: Point,
        caps: bool,
        looks: &Looks,
        chance: &mut dyn Chance,
    ) {
        let cap = caps.then_some(MAX_PODS);
        if cap.is_some_and(|cap| self.pods.len() >= cap) {
            return;
        }
        let life = POD_LIFE_MIN + chance.below(POD_LIFE_ROLL);
        let heading = chance.below(360) as f32;
        let push = (chance.below(10) + 10) as f32 * 0.1;
        let (sin, cos) = heading.to_radians().sin_cos();
        let velocity = Point::new(velocity.x + sin * push, velocity.y - cos * push);
        let frame = looks
            .pod_sheet()
            .map_or(0, |sheet| pod_frame(velocity, sheet.frames));
        let mut pod = Pod {
            at,
            velocity,
            life: i32::try_from(life).unwrap_or(i32::MAX),
            frame,
        };
        pod.step();
        self.pods.insert(pod, cap);
    }

    /// Carries each pod within `wrap`'s reach: `_WrapPlayerPosition`'s loop
    /// of `_podData` (@0x6822f-0x682b1), which leaves its velocity.
    pub fn carry(&mut self, wrap: &PlayerWrap) {
        for pod in self.pods.iter_mut() {
            carry_point(wrap, &mut pod.at);
        }
    }

    /// Steps every pod, then frees those whose life is below 0.
    pub fn step(&mut self) {
        self.pods.retain_mut(|pod| {
            pod.step();
            pod.life >= 0
        });
    }

    /// Draws each pod shown, as `looks` says, where `camera` shows it,
    /// murked as seen from the player's ship at `viewer` in a system of
    /// effective murk `murk` ([`sprite_murk`]), as `_HandleEscapePods`
    /// murks it at its position (@0x39f90-0x39fbc) before its fade.
    pub fn draw(
        &self,
        list: &mut DrawList,
        camera: &Camera,
        looks: &Looks,
        viewer: Vec2,
        murk: u8,
    ) {
        let Some(sheet) = looks.pod_sheet() else {
            return;
        };
        for pod in self.pods.iter() {
            let Some(tint) = pod.tint() else {
                continue;
            };
            let level = sprite_murk(Vec2::new(pod.at.x, pod.at.y), viewer, murk);
            let tint = murk_tint(tint, level);
            let frame = sheet.image.frame(pod.frame);
            let at = camera.world_to_screen(pod.at);
            if pod.life >= POD_FADE {
                list.sprite(frame, at, tint);
            } else {
                list.crossfade_sprite(frame, at, tint);
            }
        }
    }

    /// The pods adrift, the unseen ones at a life of 0 too, in slot order
    /// (see the module docs).
    #[must_use]
    pub fn pods(&self) -> Vec<Pod> {
        self.pods.to_vec()
    }

    /// Frees every pod.
    pub fn clear(&mut self) {
        self.pods.clear();
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::collections::VecDeque;

    use super::*;
    use crate::DrawCommand;
    use crate::ImageKey;
    use crate::draw::Blend;
    use crate::flight::catalog::EffectSheet;

    /// Draws its script, then the last outcome.
    #[derive(Default)]
    struct Script(VecDeque<u32>);

    impl Chance for Script {
        fn fires(&mut self, _percent: u8) -> bool {
            false
        }

        fn below(&mut self, n: u32) -> u32 {
            let draw = self.0.pop_front().unwrap_or(n - 1);
            assert!(draw < n, "drew {draw} below {n}");
            draw
        }
    }

    fn script(draws: &[u32]) -> Script {
        Script(draws.iter().copied().collect())
    }

    /// The pod's sheet, `rlëD` 1095, of 36 rotations.
    fn looks() -> Looks {
        Looks {
            pod: Some(Ok(EffectSheet {
                image: crate::SheetImage::Rled(1095),
                frames: NonZeroU16::new(36).expect("non-zero"),
                frame_width: 0,
                frame_height: 0,
            })),
            ..Looks::default()
        }
    }

    fn spawned(draws: &[u32]) -> EscapePods {
        let mut pods = EscapePods::default();
        pods.spawn(
            Point::new(100.0, 50.0),
            Point::new(2.0, 0.0),
            false,
            &looks(),
            &mut script(draws),
        );
        pods
    }

    #[test]
    fn a_pod_leaves_its_ship_pushed_off_and_takes_its_first_step() {
        // Life 150 + 20, pushed (5 + 10) x 0.1 = 1.5 along 90°.
        let pods = spawned(&[20, 90, 5]);
        let [pod] = pods.pods()[..] else {
            panic!("one pod");
        };
        assert_eq!(pod.life, 169, "170 less its first step");
        assert!((pod.velocity.x - 3.5).abs() < 1e-5, "{:?}", pod.velocity);
        assert!(pod.velocity.y.abs() < 1e-5);
        assert!((pod.at.x - 103.5).abs() < 1e-5, "one step on");
        assert_eq!(pod.frame, 9, "facing 90°: 9 of 36");
        let least = spawned(&[0, 0, 0]);
        assert_eq!(least.pods()[0].life, 149);
        assert!((least.pods()[0].velocity.y + 1.0).abs() < 1e-5, "1.0 up");
        let most = spawned(&[99, 180, 9]);
        assert_eq!(most.pods()[0].life, 248);
        assert!((most.pods()[0].velocity.y - 1.9).abs() < 1e-5, "1.9 down");
    }

    #[test]
    fn a_pod_drifts_by_its_velocity_each_step() {
        let mut pods = spawned(&[20, 90, 5]);
        let before = pods.pods()[0];
        pods.step();
        let after = pods.pods()[0];
        assert!((after.at.x - before.at.x - 3.5).abs() < 1e-5);
        assert_eq!(after.at.y, before.at.y);
        assert_eq!(after.life, before.life - 1);
        // Pushed straight down: it drifts down too.
        let mut down = spawned(&[20, 180, 5]);
        let before = down.pods()[0];
        down.step();
        let after = down.pods()[0];
        assert!((after.at.y - before.at.y - 1.5).abs() < 1e-5, "{after:?}");
        assert!((after.at.x - before.at.x - 2.0).abs() < 1e-5);
    }

    #[test]
    fn a_pod_is_opaque_to_32_fades_over_its_last_32_and_is_hidden_then_freed() {
        // Life 150: 149 after its first step.
        let mut pods = spawned(&[0, 90, 0]);
        let mut alphas = Vec::new();
        while let Some(pod) = pods.pods().first() {
            alphas.push((pod.life, pod.tint().map(|tint| tint.a)));
            pods.step();
        }
        assert_eq!(alphas.first(), Some(&(149, Some(255))));
        let at = |life| alphas.iter().find(|(l, _)| *l == life).map(|(_, a)| *a);
        assert_eq!(at(32), Some(Some(255)), "opaque at 32");
        assert_eq!(at(31), Some(Some(247)), "31/32");
        assert_eq!(at(16), Some(Some(128)));
        assert_eq!(at(1), Some(Some(8)), "1/32");
        assert_eq!(at(0), Some(None), "held unseen at 0");
        assert_eq!(alphas.last(), Some(&(0, None)), "then freed");
        let fading: Vec<u8> = alphas
            .iter()
            .filter_map(|(life, alpha)| alpha.filter(|_| *life < 32))
            .collect();
        assert!(
            fading.windows(2).all(|pair| pair[0] > pair[1]),
            "{fading:?}"
        );
        assert_eq!(fading.len(), 31);
    }

    #[test]
    fn a_pod_is_drawn_on_its_frame_at_its_tint() {
        let pods = spawned(&[20, 90, 5]);
        let mut list = DrawList::new();
        pods.draw(
            &mut list,
            &Camera::centred_on(Point::default()),
            &looks(),
            Vec2::ZERO,
            0,
        );
        let drawn: Vec<_> = list
            .into_iter()
            .filter_map(|command| match command {
                DrawCommand::Sprite { image, tint, .. } => Some((*image, *tint)),
                _ => None,
            })
            .collect();
        assert_eq!(drawn, [(ImageKey::sprite(1095, 9), Color::WHITE)]);
        let mut unread = DrawList::new();
        pods.draw(
            &mut unread,
            &Camera::centred_on(Point::default()),
            &Looks::default(),
            Vec2::ZERO,
            0,
        );
        assert!(unread.into_iter().next().is_none(), "no sheet, no pod");
    }

    #[test]
    fn a_fading_pod_is_crossfaded_by_or_and_an_opaque_one_drawn_plainly() {
        let mut pods = spawned(&[20, 90, 5]);
        let blends = |pods: &EscapePods| -> Vec<(Color, Blend)> {
            let mut list = DrawList::new();
            pods.draw(
                &mut list,
                &Camera::centred_on(Point::default()),
                &looks(),
                Vec2::ZERO,
                0,
            );
            list.into_iter()
                .filter_map(|command| match command {
                    DrawCommand::Sprite { tint, blend, .. } => Some((*tint, *blend)),
                    _ => None,
                })
                .collect()
        };
        while pods.pods()[0].life > POD_FADE {
            pods.step();
        }
        assert_eq!(blends(&pods), [(Color::WHITE, Blend::Normal)], "at 32");
        pods.step();
        let tint = pods.pods()[0].tint().expect("shown");
        assert_eq!(pods.pods()[0].life, POD_FADE - 1);
        assert_eq!(blends(&pods), [(tint, Blend::OrCrossfade)], "at 31");
        assert_eq!(tint.a, 247, "31/32");
    }

    #[test]
    fn a_pod_far_from_the_viewer_in_murk_is_darkened_opaque_or_fading() {
        let mut pods = spawned(&[20, 90, 5]);
        let tints = |pods: &EscapePods, viewer: Vec2| -> Vec<(Color, Blend)> {
            let mut list = DrawList::new();
            let camera = Camera::centred_on(Point::default());
            pods.draw(&mut list, &camera, &looks(), viewer, 100);
            list.into_iter()
                .filter_map(|command| match command {
                    DrawCommand::Sprite { tint, blend, .. } => Some((*tint, *blend)),
                    _ => None,
                })
                .collect()
        };
        let pod = pods.pods()[0];
        let level = sprite_murk(Vec2::new(pod.at.x, pod.at.y), Vec2::ZERO, 100);
        assert!(level > 0, "murky out here: {level}");
        let opaque = murk_tint(Color::WHITE, level);
        assert_eq!(tints(&pods, Vec2::ZERO), [(opaque, Blend::Normal)]);
        let beside = Vec2::new(pod.at.x, pod.at.y);
        assert_eq!(
            tints(&pods, beside),
            [(Color::WHITE, Blend::Normal)],
            "untinted at the viewer"
        );
        while pods.pods()[0].life >= POD_FADE {
            pods.step();
        }
        let pod = pods.pods()[0];
        let level = sprite_murk(Vec2::new(pod.at.x, pod.at.y), Vec2::ZERO, 100);
        let fading = murk_tint(pod.tint().expect("shown"), level);
        assert_ne!(fading, pod.tint().expect("shown"));
        assert_eq!(tints(&pods, Vec2::ZERO), [(fading, Blend::OrCrossfade)]);
    }

    #[test]
    fn under_the_caps_a_33rd_pod_is_not_kept() {
        for (caps, kept) in [(true, MAX_PODS), (false, MAX_PODS + 1)] {
            let mut pods = EscapePods::default();
            for _ in 0..=MAX_PODS {
                pods.spawn(
                    Point::new(0.0, 0.0),
                    Point::new(0.0, 0.0),
                    caps,
                    &looks(),
                    &mut script(&[]),
                );
            }
            assert_eq!(pods.pods().len(), kept, "caps {caps}");
        }
    }

    #[test]
    fn a_held_pod_keeps_its_slot_until_its_life_is_below_0() {
        let mut pods = EscapePods::default();
        for _ in 0..MAX_PODS {
            pods.spawn(
                Point::new(0.0, 0.0),
                Point::new(0.0, 0.0),
                true,
                &looks(),
                &mut script(&[0]),
            );
        }
        for _ in 0..149 {
            pods.step();
        }
        assert!(pods.pods().iter().all(|pod| pod.life == 0));
        pods.spawn(
            Point::new(0.0, 0.0),
            Point::new(0.0, 0.0),
            true,
            &looks(),
            &mut script(&[]),
        );
        assert_eq!(pods.pods().len(), MAX_PODS, "no slot free at 0");
        pods.step();
        assert!(pods.pods().is_empty(), "all free below 0");
    }

    #[test]
    fn a_new_pod_takes_a_freed_lower_slot_and_draws_under_an_older_one() {
        let spawn = |pods: &mut EscapePods, x: f32, life: u32| {
            pods.spawn(
                Point::new(x, 0.0),
                Point::default(),
                false,
                &looks(),
                &mut script(&[life, 0, 0]),
            );
        };
        let mut pods = EscapePods::default();
        spawn(&mut pods, 10.0, 0);
        spawn(&mut pods, 20.0, 99);
        for _ in 0..150 {
            pods.step();
        }
        let xs = |pods: &EscapePods| -> Vec<f32> { pods.pods().iter().map(|p| p.at.x).collect() };
        assert_eq!(xs(&pods), [20.0], "the first freed below 0");
        spawn(&mut pods, 30.0, 0);
        assert_eq!(xs(&pods), [30.0, 20.0], "in slot order");
        let camera = Camera::centred_on(Point::default());
        let mut list = DrawList::new();
        pods.draw(&mut list, &camera, &looks(), Vec2::ZERO, 0);
        let drawn: Vec<f32> = list
            .into_iter()
            .filter_map(|command| match command {
                DrawCommand::Sprite { center, .. } => Some(center.x),
                _ => None,
            })
            .collect();
        let screen = |x| camera.world_to_screen(Point::new(x, 0.0)).x;
        assert_eq!(drawn, [screen(30.0), screen(20.0)], "the new one first");
    }

    #[test]
    fn a_wrap_carries_the_pods_within_its_reach_and_leaves_their_velocities() {
        use nova_sim::player_wrap::{WRAP_EDGE, WRAP_SHIFT};
        let player = Vec2::new(WRAP_EDGE + 10.0, 0.0);
        let wrap = PlayerWrap::of(player).expect("past the edge");
        let mut pods = EscapePods::default();
        for x in [player.x - 100.0, player.x - WRAP_EDGE - 5.0] {
            let at = Point::new(x, 40.0);
            pods.spawn(
                at,
                Point::new(1.0, 0.0),
                false,
                &looks(),
                &mut script(&[0, 0]),
            );
        }
        let before = pods.pods();
        pods.carry(&wrap);
        let after = pods.pods();
        assert_eq!(after[0].at.x, before[0].at.x - WRAP_SHIFT);
        assert_eq!(after[0].at.y, before[0].at.y);
        assert_eq!(after[0].velocity, before[0].velocity);
        assert_eq!(after[1], before[1]);
    }

    #[test]
    fn clearing_frees_every_pod() {
        let mut pods = spawned(&[20, 90, 5]);
        pods.clear();
        assert!(pods.pods().is_empty());
    }

    #[test]
    fn a_pod_faces_its_velocity_in_whole_degrees_of_its_sheet() {
        let frames = |n| NonZeroU16::new(n).expect("non-zero");
        assert_eq!(pod_frame(Point::new(0.0, -1.0), frames(36)), 0, "up");
        assert_eq!(pod_frame(Point::new(1.0, 0.0), frames(36)), 9, "right");
        assert_eq!(pod_frame(Point::new(0.0, 1.0), frames(36)), 18, "down");
        assert_eq!(pod_frame(Point::new(-1.0, 0.0), frames(36)), 27, "left");
        assert_eq!(pod_frame(Point::new(1.0, 0.0), frames(64)), 16);
        // The table's heading of (1, 1) is 134°: trunc(134 x 64 / 360) = 23.
        assert_eq!(intercept_angle(Vec2::ZERO, Vec2::new(100.0, 100.0)), 134);
        assert_eq!(pod_frame(Point::new(1.0, 1.0), frames(64)), 23);
        assert_eq!(
            pod_frame(Point::new(1.0, 1.0), frames(36)),
            13,
            "134 x 36 / 360"
        );
    }
}
