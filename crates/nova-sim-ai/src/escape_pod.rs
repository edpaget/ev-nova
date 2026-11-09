//! The NPCs' escape pods: the sprites a dying ship's crew and person
//! eject (`CombatEvent::PodEjected`), as `_SpawnEscapePod`
//! (@0x45547-0x456cc) and `_HandleEscapePods` (@0x39e48-0x3a063) keep them
//! in the `EV Nova` executable. They live in the simulation, as a
//! scooping ship can take one for box slot 0 (see [`scoop`](crate::scoop));
//! where and how each is drawn is the view's (`nova_view::flight::pods`).
//!
//! - **The spawn** ([`EscapePods::spawn`]): the pod takes the ship's
//!   position and velocity, then `_Accel(Rand(360), (Rand(10) + 10) x
//!   0.1)` on top (0.1 at 0xdd110), a push of 1.0 to 1.9 pixels a tick in a
//!   direction of its own, and lives `Rand(100) + 150` ticks. It faces its
//!   velocity: frame `trunc(_CalcInterceptAngle(0, v x 100) x frames /
//!   360)` (100 at 0xdd060, 360 at 0xdd0f8; [`pod_frame`]) of shän 895's
//!   sheet, the escape pod's (`ESCAPE_POD_SHIP`), of its `FramesPer`
//!   rotations, and keeps that frame. It is shown (`_SWSetSpriteVisible`
//!   @0x455ef). It takes its spawn frame's step at once, as
//!   `_HandleEscapePods` runs after `_HandleShip` in the frame
//!   (`_DoPlayGameWork` @0x44483, @0x44513).
//! - **The pool.** Under the original's caps (`RuleKey::CombatCaps`'
//!   engine reading) there are [`MAX_PODS`] slots, a pod taking the first
//!   whose life is below 0 (@0x45564-0x456bf), and one more is not kept;
//!   by default nothing is capped, and the pool is unbounded but still
//!   first-free ([`SlotPool`]). `_boxLayer` draws and collides the pods in
//!   slot order (`_InitSprites` adds them once, and nothing moves them), so
//!   a new pod in a freed lower slot comes before an older one in a higher
//!   slot ([`EscapePods::pods`]).
//! - **Each step** ([`EscapePods::step`]): it drifts by its velocity × the
//!   frame's gain and the gain comes off its life (`_gSpeedMult` @0x39e92,
//!   @0x39ebe, @0x39fcc), each in doubles. At a life of 0 or less (or a NaN
//!   one) the step hides it (`ucomiss 0, life` / `jbe`,
//!   @0x3a02d-0x3a046), and nothing shows it again. Its slot is free once
//!   the life is below 0: `_SpawnEscapePod` takes it (`ucomiss life, 0` /
//!   `jbe` @0x45564-0x45568), and until then `_HandleEscapePods` only
//!   hides it again (`ucomiss 0, life` / `jb` @0x39e67-0x39e6f to
//!   @0x3a039). The engine frees a NaN life too, which the original's
//!   spawn would skip (`jbe` on unordered); no life reaches NaN, as each
//!   starts whole and loses a finite gain. The
//!   original's frame spawns (`_HandleShip` @0x44483) before it steps
//!   (@0x44513), so a pod that falls below 0 is drawn hidden that frame
//!   and its slot is free from the next frame's spawns. The engine steps
//!   before its pass's spawns, so the step frees the pods already below 0
//!   first, then steps the rest: a pod that falls below 0 stays, hidden,
//!   through the spawns after the step, and the next pass's step frees it.
//!   A spawn's first step takes the spawning pass's gain.
//! - **The scoop** hides a pod a scooping ship takes for box slot 0
//!   (`_SWSetSpriteVisible(pod, 0)` @0x3712e, [`EscapePods::hide`]). Its
//!   life runs on, so it holds its slot until its life is below 0.
//! - **Leaving the system** clears them ([`EscapePods::clear`]), as
//!   `_killShots` frees them on landing, a gate and the arrival
//!   (@0x39e75-0x39e84).
//! - **The player's wrap** carries them ([`EscapePods::carry`],
//!   `_WrapPlayerPosition` @0x6822f-0x682b1), their velocity kept.
//!
//! The pod's sound, `snd ` 372, is `_HandleShip`'s (@0x351fa, @0x35300),
//! played whether or not the pool keeps the pod; the view plays it for
//! each `PodEjected`.

use std::num::NonZeroU16;

use crate::chance::Chance;
use crate::flight::moved;
use crate::geometry::Vec2;
use crate::player_wrap::PlayerWrap;
use crate::slot_pool::SlotPool;
use crate::speed_mult::SpeedMult;
use crate::trig::intercept_angle;

/// The most pods kept at once under the caps: `_podData`'s 32 entries
/// (`cmpl $0x20` @0x456bc).
pub const MAX_PODS: usize = 32;
/// A pod's least life (`Rand(100) + 150`, @0x4556e-0x45584).
pub const POD_LIFE_MIN: u32 = 150;
/// How many lives a pod can roll above [`POD_LIFE_MIN`].
pub const POD_LIFE_ROLL: u32 = 100;

/// An escape pod adrift.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pod {
    /// Where it is, in pixels from the system's centre.
    pub position: Vec2,
    /// How far it moves a pass at full gain: the move is this × the
    /// frame's gain.
    pub velocity: Vec2,
    /// The ticks it has left, a float as the original keeps it (+0x14):
    /// each pass takes the frame's gain off it.
    pub life: f32,
    /// The frame of shän 895's sheet it shows.
    pub frame: u16,
    /// Whether its sprite is shown: from its spawn until a step leaves
    /// its life at 0 or less, or a scoop hides it, and never again.
    pub shown: bool,
}

impl Pod {
    /// One step at the frame's `gain`: it drifts by its velocity × the
    /// gain, and the gain comes off its life, each added in doubles
    /// (`_HandleEscapePods` @0x39e8d-0x39ee0, @0x39fcc-0x39fe5); then a
    /// life of 0 or less hides it (@0x3a02d-0x3a046).
    fn step(&mut self, gain: SpeedMult) {
        self.position = moved(self.position, self.velocity, gain);
        self.life = gain.add_times(self.life, -1.0);
        // `jbe`: a NaN life hides it too.
        #[allow(clippy::neg_cmp_op_on_partial_ord)]
        if !(self.life > 0.0) {
            self.shown = false;
        }
    }
}

/// The frame of a sheet of `frames` rotations a pod moving at `velocity`
/// faces: the original's whole-degree heading of its velocity, times the
/// rotations in integers, then divided by 360 (@0x45676-0x4569d).
#[must_use]
pub fn pod_frame(velocity: Vec2, frames: NonZeroU16) -> u16 {
    let towards = velocity * 100.0;
    let angle = i32::from(intercept_angle(Vec2::ZERO, towards));
    let frame = angle * i32::from(frames.get()) / 360;
    u16::try_from(frame).unwrap_or(0) % frames.get()
}

/// The escape pods adrift, by slot (see the module docs).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct EscapePods {
    pods: SlotPool<Pod>,
}

impl EscapePods {
    /// The pods `pods`, in slots 0 on: for nova-sim's scoop tests, which
    /// place a pod exactly.
    #[cfg(any(test, feature = "fixture"))]
    #[must_use]
    pub fn adrift(pods: &[Pod]) -> Self {
        let mut adrift = Self::default();
        for &pod in pods {
            adrift.pods.insert(pod, None);
        }
        adrift
    }

    /// Sets a pod adrift from a ship at `at` moving at `velocity`, drawing
    /// on `chance`, unless the `caps` hold the pool full; it faces a frame
    /// of shän 895's `frames` rotations (frame 0 without them). It takes
    /// its first step at once, at the pass's `gain`.
    pub fn spawn(
        &mut self,
        at: Vec2,
        velocity: Vec2,
        gain: SpeedMult,
        caps: bool,
        frames: Option<NonZeroU16>,
        chance: &mut (impl Chance + ?Sized),
    ) {
        let cap = caps.then_some(MAX_PODS);
        if cap.is_some_and(|cap| self.pods.len() >= cap) {
            return;
        }
        let life = POD_LIFE_MIN + chance.below(POD_LIFE_ROLL);
        // Whole degrees and tenths, far inside f32's exact range.
        #[allow(clippy::cast_precision_loss)]
        let heading = chance.below(360) as f32;
        #[allow(clippy::cast_precision_loss)]
        let push = (chance.below(10) + 10) as f32 * 0.1;
        let (sin, cos) = heading.to_radians().sin_cos();
        let velocity = Vec2::new(velocity.x + sin * push, velocity.y - cos * push);
        let frame = frames.map_or(0, |frames| pod_frame(velocity, frames));
        #[allow(clippy::cast_precision_loss)]
        let mut pod = Pod {
            position: at,
            velocity,
            life: life as f32,
            frame,
            shown: true,
        };
        pod.step(gain);
        self.pods.insert(pod, cap);
    }

    /// Carries each pod within `wrap`'s reach: `_WrapPlayerPosition`'s loop
    /// of `_podData` (@0x6822f-0x682b1), which leaves its velocity.
    pub fn carry(&mut self, wrap: &PlayerWrap) {
        for pod in self.pods.iter_mut() {
            wrap.carry(&mut pod.position);
        }
    }

    /// Frees the pods whose life is already below 0 (a NaN one too), then
    /// steps the rest at the frame's `gain`. A pod this step takes below 0
    /// stays, hidden, until the next step frees it, so it holds its slot
    /// through the spawns that come after the step in the engine's pass,
    /// as the original's `_SpawnEscapePod` (@0x44483) runs before
    /// `_HandleEscapePods` (@0x44513) and frees only a life below 0
    /// (@0x45564-0x456bf). See the module docs.
    pub fn step(&mut self, gain: SpeedMult) {
        self.pods.retain_mut(|pod| pod.life >= 0.0);
        for pod in self.pods.iter_mut() {
            pod.step(gain);
        }
    }

    /// Hides the pod at `index` of [`EscapePods::pods`], as the scoop
    /// does (@0x3712e); its life runs on.
    pub fn hide(&mut self, index: usize) {
        if let Some(pod) = self.pods.iter_mut().nth(index) {
            pod.shown = false;
        }
    }

    /// The pods adrift, the hidden ones too, in slot order (see the
    /// module docs).
    #[must_use]
    pub fn pods(&self) -> &[Pod] {
        self.pods.as_slice()
    }

    /// Whether no pod is adrift.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.pods.is_empty()
    }

    /// Frees every pod.
    pub fn clear(&mut self) {
        self.pods.clear();
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use nova_sim_base::fixture::Draws;

    /// shän 895's 36 rotations.
    fn frames() -> Option<NonZeroU16> {
        NonZeroU16::new(36)
    }

    fn spawned(draws: &[u32]) -> EscapePods {
        spawned_at(draws, SpeedMult::FULL)
    }

    /// A pod spawned as [`spawned`] does, its first step at `gain`.
    fn spawned_at(draws: &[u32], gain: SpeedMult) -> EscapePods {
        let mut pods = EscapePods::default();
        pods.spawn(
            Vec2::new(100.0, 50.0),
            Vec2::new(2.0, 0.0),
            gain,
            false,
            frames(),
            &mut Draws::of(draws),
        );
        pods
    }

    /// A shown pod of `life`, drifting at (2, -1).
    fn adrift(life: f32) -> EscapePods {
        EscapePods::adrift(&[Pod {
            position: Vec2::new(10.0, 20.0),
            velocity: Vec2::new(2.0, -1.0),
            life,
            frame: 0,
            shown: true,
        }])
    }

    #[test]
    fn a_pod_drifts_and_ages_by_the_gain() {
        let mut half = adrift(10.0);
        half.step(SpeedMult::new(0.5));
        let pod = half.pods()[0];
        assert_eq!(pod.position, Vec2::new(11.0, 19.5), "half its velocity");
        assert_eq!(pod.life, 9.5, "half a tick off its life");
        let mut held = adrift(10.0);
        held.step(SpeedMult::ZERO);
        assert_eq!(held.pods(), adrift(10.0).pods(), "held still at 0");
    }

    #[test]
    fn a_spawns_first_step_is_taken_at_the_gain() {
        // Pushed (5 + 10) x 0.1 = 1.5 along 90°: velocity (3.5, 0).
        let pod = spawned_at(&[20, 90, 5], SpeedMult::new(0.5)).pods()[0];
        assert!(
            (pod.position.x - 101.75).abs() < 1e-5,
            "half a step on: {pod:?}"
        );
        assert_eq!(pod.life, 169.5, "170 less half a tick");
    }

    #[test]
    fn at_half_gain_a_pod_is_hidden_at_0_and_freed_below_it() {
        let mut pods = adrift(1.0);
        pods.step(SpeedMult::new(0.5));
        assert_eq!(pods.pods()[0].life, 0.5);
        assert!(pods.pods()[0].shown, "shown at 0.5");
        pods.step(SpeedMult::new(0.5));
        assert_eq!(pods.pods()[0].life, 0.0);
        assert!(!pods.pods()[0].shown, "hidden at 0");
        pods.step(SpeedMult::new(0.5));
        assert_eq!(pods.pods()[0].life, -0.5, "held by the step it falls in");
        pods.step(SpeedMult::new(0.5));
        assert!(pods.pods().is_empty(), "freed below 0 by the next");
    }

    #[test]
    fn a_pod_a_step_hides_stays_hidden() {
        let mut pods = adrift(0.5);
        pods.step(SpeedMult::new(0.5));
        assert!(!pods.pods()[0].shown);
        pods.pods.iter_mut().for_each(|pod| pod.life = 10.0);
        pods.step(SpeedMult::FULL);
        assert!(!pods.pods()[0].shown, "never shown again");
    }

    #[test]
    fn a_nan_life_is_freed() {
        let mut nan = adrift(f32::NAN);
        nan.step(SpeedMult::FULL);
        assert!(nan.pods().is_empty(), "freed");
    }

    #[test]
    fn a_pod_leaves_its_ship_pushed_off_shown_and_takes_its_first_step() {
        // Life 150 + 20, pushed (5 + 10) x 0.1 = 1.5 along 90°.
        let pods = spawned(&[20, 90, 5]);
        let [pod] = pods.pods()[..] else {
            panic!("one pod");
        };
        assert!(pod.shown);
        assert_eq!(pod.life, 169.0, "170 less its first step");
        assert!((pod.velocity.x - 3.5).abs() < 1e-5, "{:?}", pod.velocity);
        assert!(pod.velocity.y.abs() < 1e-5);
        assert!((pod.position.x - 103.5).abs() < 1e-5, "one step on");
        assert_eq!(pod.frame, 9, "facing 90°: 9 of 36");
        let least = spawned(&[0, 0, 0]);
        assert_eq!(least.pods()[0].life, 149.0);
        assert!((least.pods()[0].velocity.y + 1.0).abs() < 1e-5, "1.0 up");
        let most = spawned(&[99, 180, 9]);
        assert_eq!(most.pods()[0].life, 248.0);
        assert!((most.pods()[0].velocity.y - 1.9).abs() < 1e-5, "1.9 down");
    }

    #[test]
    fn a_pod_draws_its_life_heading_and_push_over_their_ranges() {
        let mut draws = Draws::of(&[]);
        EscapePods::default().spawn(
            Vec2::ZERO,
            Vec2::ZERO,
            SpeedMult::FULL,
            false,
            frames(),
            &mut draws,
        );
        assert_eq!(draws.asked, [100, 360, 10]);
    }

    #[test]
    fn a_pod_without_its_sheets_rotations_faces_frame_0() {
        let mut pods = EscapePods::default();
        pods.spawn(
            Vec2::ZERO,
            Vec2::ZERO,
            SpeedMult::FULL,
            false,
            None,
            &mut Draws::of(&[20, 90, 5]),
        );
        assert_eq!(pods.pods()[0].frame, 0);
    }

    #[test]
    fn a_pod_drifts_by_its_velocity_each_step() {
        let mut pods = spawned(&[20, 90, 5]);
        let before = pods.pods()[0];
        pods.step(SpeedMult::FULL);
        let after = pods.pods()[0];
        assert!((after.position.x - before.position.x - 3.5).abs() < 1e-5);
        assert_eq!(after.position.y, before.position.y);
        assert_eq!(after.life, before.life - 1.0);
        // Pushed straight down: it drifts down too.
        let mut down = spawned(&[20, 180, 5]);
        let before = down.pods()[0];
        down.step(SpeedMult::FULL);
        let after = down.pods()[0];
        assert!(
            (after.position.y - before.position.y - 1.5).abs() < 1e-5,
            "{after:?}"
        );
        assert!((after.position.x - before.position.x - 2.0).abs() < 1e-5);
    }

    #[test]
    fn under_the_caps_a_33rd_pod_is_not_kept() {
        for (caps, kept) in [(true, MAX_PODS), (false, MAX_PODS + 1)] {
            let mut pods = EscapePods::default();
            for _ in 0..=MAX_PODS {
                pods.spawn(
                    Vec2::ZERO,
                    Vec2::ZERO,
                    SpeedMult::FULL,
                    caps,
                    frames(),
                    &mut Draws::of(&[]),
                );
            }
            assert_eq!(pods.pods().len(), kept, "caps {caps}");
        }
    }

    #[test]
    fn a_full_pool_under_the_caps_draws_nothing() {
        let mut pods = EscapePods::default();
        for _ in 0..MAX_PODS {
            pods.spawn(
                Vec2::ZERO,
                Vec2::ZERO,
                SpeedMult::FULL,
                true,
                frames(),
                &mut Draws::of(&[]),
            );
        }
        let mut draws = Draws::of(&[]);
        pods.spawn(
            Vec2::ZERO,
            Vec2::ZERO,
            SpeedMult::FULL,
            true,
            frames(),
            &mut draws,
        );
        assert!(draws.asked.is_empty());
    }

    #[test]
    fn a_held_pod_keeps_its_slot_until_its_life_is_below_0() {
        let mut pods = EscapePods::default();
        for _ in 0..MAX_PODS {
            pods.spawn(
                Vec2::ZERO,
                Vec2::ZERO,
                SpeedMult::FULL,
                true,
                frames(),
                &mut Draws::of(&[0]),
            );
        }
        for _ in 0..149 {
            pods.step(SpeedMult::FULL);
        }
        assert!(pods.pods().iter().all(|pod| pod.life == 0.0));
        pods.spawn(
            Vec2::ZERO,
            Vec2::ZERO,
            SpeedMult::FULL,
            true,
            frames(),
            &mut Draws::of(&[]),
        );
        assert_eq!(pods.pods().len(), MAX_PODS, "no slot free at 0");
        pods.step(SpeedMult::FULL);
        assert!(pods.pods().iter().all(|pod| pod.life == -1.0 && !pod.shown));
        pods.spawn(
            Vec2::ZERO,
            Vec2::ZERO,
            SpeedMult::FULL,
            true,
            frames(),
            &mut Draws::of(&[]),
        );
        assert_eq!(
            pods.pods().len(),
            MAX_PODS,
            "held through the spawns after the step that takes them below 0"
        );
        pods.step(SpeedMult::FULL);
        assert!(pods.pods().is_empty(), "all free below 0 at the next step");
    }

    #[test]
    fn a_new_pod_takes_a_freed_lower_slot_before_an_older_one() {
        let spawn = |pods: &mut EscapePods, x: f32, life: u32| {
            pods.spawn(
                Vec2::new(x, 0.0),
                Vec2::ZERO,
                SpeedMult::FULL,
                false,
                frames(),
                &mut Draws::of(&[life, 0, 0]),
            );
        };
        let mut pods = EscapePods::default();
        spawn(&mut pods, 10.0, 0);
        spawn(&mut pods, 20.0, 99);
        // The 150th step takes the first to -1; the 151st frees it.
        for _ in 0..151 {
            pods.step(SpeedMult::FULL);
        }
        let xs = |pods: &EscapePods| -> Vec<f32> {
            pods.pods().iter().map(|pod| pod.position.x).collect()
        };
        assert_eq!(xs(&pods), [20.0], "the first freed below 0");
        spawn(&mut pods, 30.0, 0);
        assert_eq!(xs(&pods), [30.0, 20.0], "in slot order");
    }

    #[test]
    fn hiding_a_pod_leaves_its_life_running_and_its_slot_held() {
        let mut pods = spawned(&[20, 90, 5]);
        pods.hide(0);
        pods.hide(5);
        let pod = pods.pods()[0];
        assert!(!pod.shown);
        pods.step(SpeedMult::FULL);
        assert_eq!(pods.pods()[0].life, pod.life - 1.0);
        assert!(!pods.pods()[0].shown, "a step shows it no more");
    }

    #[test]
    fn a_wrap_carries_the_pods_within_its_reach_and_leaves_their_velocities() {
        use crate::player_wrap::{WRAP_EDGE, WRAP_SHIFT};
        let player = Vec2::new(WRAP_EDGE + 10.0, 0.0);
        let wrap = PlayerWrap::of(player).expect("past the edge");
        let mut pods = EscapePods::default();
        for x in [player.x - 100.0, player.x - WRAP_EDGE * 2.0] {
            pods.spawn(
                Vec2::new(x, 40.0),
                Vec2::new(1.0, 0.0),
                SpeedMult::FULL,
                false,
                frames(),
                &mut Draws::of(&[0, 0]),
            );
        }
        let before = pods.pods().to_vec();
        pods.carry(&wrap);
        let after = pods.pods();
        assert_eq!(after[0].position.x, before[0].position.x - WRAP_SHIFT);
        assert_eq!(after[0].position.y, before[0].position.y);
        assert_eq!(after[0].velocity, before[0].velocity);
        assert_eq!(after[1], before[1]);
    }

    #[test]
    fn clearing_frees_every_pod() {
        let mut pods = spawned(&[20, 90, 5]);
        assert!(!pods.is_empty());
        pods.clear();
        assert!(pods.is_empty());
    }

    #[test]
    fn a_pod_faces_its_velocity_in_whole_degrees_of_its_sheet() {
        let frames = |n| NonZeroU16::new(n).expect("non-zero");
        assert_eq!(pod_frame(Vec2::new(0.0, -1.0), frames(36)), 0, "up");
        assert_eq!(pod_frame(Vec2::new(1.0, 0.0), frames(36)), 9, "right");
        assert_eq!(pod_frame(Vec2::new(0.0, 1.0), frames(36)), 18, "down");
        assert_eq!(pod_frame(Vec2::new(-1.0, 0.0), frames(36)), 27, "left");
        assert_eq!(pod_frame(Vec2::new(1.0, 0.0), frames(64)), 16);
        // The table's heading of (1, 1) is 134°: trunc(134 x 64 / 360) = 23.
        assert_eq!(intercept_angle(Vec2::ZERO, Vec2::new(100.0, 100.0)), 134);
        assert_eq!(pod_frame(Vec2::new(1.0, 1.0), frames(64)), 23);
        assert_eq!(
            pod_frame(Vec2::new(1.0, 1.0), frames(36)),
            13,
            "134 x 36 / 360"
        );
    }
}
