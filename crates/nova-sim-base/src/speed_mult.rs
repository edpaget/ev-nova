//! The frame's time gain, the original's `_gSpeedMult`, and its ramp,
//! `_HandleTimeAdjustment`.
//!
//! `_gSpeedMult` is a double at 0x188b48 (behind *0x3b511c, in `__common`,
//! so 0.0 at launch) that every timed pass multiplies its motion and its
//! timers by. [`SpeedMult`] is that value.
//!
//! # The ramp
//!
//! `_HandleTimeAdjustment` (@0x33332-0x3349b), called once per pass of
//! `_PlayGame`'s loop (@0x45b28) before the frame's work, so both of double
//! time's passes run at one gain, measures the frame's time as `new`, in
//! frames of 33.3 ms (`ms` × 0.03, @0xdd5d8), and, when 0.01 (@0xdd098) ≤
//! `new` < 10.0 (@0xdd65c), sets the gain to (3.0 (@0xdd490) × old + `new`)
//! × 0.25 (@0xdd130) in doubles (@0x333ed-0x33424). A frame 334 ms or more
//! after the last keeps the gain where it stood.
//!
//! The engine steps a fixed 1/30 s (`clock::TICK`), so each step is a
//! frame with `new` = 1.0 and [`TimeAdjustment::frame`] gives
//! (3 × gain + 1) / 4. The gain never rises above 1.0. A gap the original measures
//! as 334 ms or more (a landing, a dialog, the main menu, the map, the
//! pause) is a [`TimeAdjustment::hold`]: the next frame keeps the gain.
//! So after a zero the frames run at 0 (held), 0.25, 0.4375, 0.578125, …,
//! 1 − 0.75^(n−1). A dialog dismissed in under 334 ms gives the original
//! `new` = `ms` × 0.03, so its first frame runs at `ms` × 0.0075; the
//! engine holds instead, as its frames are not timed. The original's gain
//! above 1.0 (a slow frame) is the engine's catch-up steps.
//!
//! In doubles the ramp never reaches 1.0: it settles one ulp below, at
//! 1 − 2^-52, where 3 × gain + 1 rounds to 4 − 2^-51 and its quarter is
//! the gain again ([`SpeedMult::SETTLED`]). The engine keeps that, as the
//! original does.
//!
//! # Float timers
//!
//! A pass that keeps its timer as a float (a stellar's sprite timer at
//! `spöb` +0x490) adds the gain in doubles and narrows the sum back to a
//! float (@0x2e810-0x2e825). The narrowing is what makes the settled
//! gain a whole frame: 0 + (1 − 2^-52) rounds to 1.0, so an `AnimDelay`
//! of 1 steps every frame. A double timer would stay just below each
//! threshold and step a frame late every time. It also rounds away the
//! last of the ramp's shortfall, which a double timer would carry into
//! the threshold a long delay tests. [`SpeedMult::added_to_float`] is
//! that add, so such a timer, though a double, steps on the frames the
//! original's float does. From [`FLOAT_WHOLE_FRAMES`] (2^24) on a float
//! stops counting, where the original's timer would never reach a
//! threshold that wide (`AnimDelay` × `Frame0Bias` reaches 2^30); the
//! engine's keeps the double sum and counts on, a float's width not
//! replicated.
//!
//! # Its readers
//!
//! Each pass that reads `_gSpeedMult` takes the gain as a value, never
//! from a global: [`SpeedMult::times`] for a product the original passes
//! on as a float (an acceleration, a strength), [`SpeedMult::add_times`]
//! for a field that gains a scaled step in doubles, narrowed back to a
//! float. `_HandlePlayer`'s, `_AdjustInertialessShipVelocity`'s and
//! `_HandleGravity`'s readers (rdm:phase/play-feel-8/phase-21-handle-player-reads-the-gain)
//! are here in [`flight`](crate::flight), [`inertialess`](crate::inertialess),
//! [`player_turn`](crate::player_turn), [`ionization`](crate::ionization),
//! [`fuel`](crate::fuel), [`afterburner`](crate::afterburner) and
//! [`gravity`](crate::gravity); in `nova_sim_combat`, through the pass's
//! `Field::speed_mult` and `StellarHazards::speed_mult` (the player's
//! regeneration, reloads, repair inhibit, sabotage clock, self-destruct
//! count and cloak upkeep); and in `nova_sim`'s session, through
//! `Session::speed_mult`. `_HandleShip`'s readers
//! (rdm:phase/play-feel-8/phase-22-handle-ship-reads-the-gain), an NPC's
//! move, turn, thrust, ion block, fuel and the shared flight functions,
//! are here in [`flight`](crate::flight), [`inertialess`](crate::inertialess),
//! [`ionization`](crate::ionization) and [`fuel`](crate::fuel); in
//! `nova_sim_ai`'s traffic, through the pass's `World::speed_mult`, the
//! fly half's `FlyPass` (its end's AI waits and fuel) and each step's
//! `AtStep::gain` (the cloak's shield upkeep, the bank, the warp-out, the
//! stray provocation and the AI wait, a float its `ai_wait` module
//! counts down); in `nova_sim_combat`, through the pass's
//! `Field::speed_mult` (an NPC's reloads and regeneration); and in
//! `nova_sim_port`'s `npc_escape`, the crew pods' interval.
//! `_HandleShot`'s, `_HandleShotGuidance`'s and `_SpawnShot`'s readers
//! (rdm:phase/play-feel-8/phase-23-shots-read-the-gain), a shot's float
//! life, move, homing delay and turn, missile lock, spin, fade and decay
//! and the interference roll at launch, are in `nova_sim_combat`, through
//! the pass's `Field::speed_mult`, which each visit's
//! `Guide::speed_mult` carries to the shot's own rules
//! (`combat::projectile`, `combat::spin_timer`, `combat::shot_fade` and
//! `combat::shot_decay`), and here in [`flight`](crate::flight)'s
//! `moved`. `_HandleShipDisplay`'s readers
//! (rdm:phase/play-feel-8/phase-24-ship-display-reads-the-gain), each
//! ship's fold, animation and alt timers, its lights' timer and triangle
//! step, its weapon glow, its shield flash and its cloak fade, are in
//! `nova_sim_combat`, through each display's `Moment::gain`
//! (`combat::ship_frames`, `combat::ship_lights`) and `Cloak::step`'s
//! gain; in `nova_sim_ai`'s traffic, through `traffic::ship_display`'s
//! `display` and `Traffic::step_fades`; in the view, through the pass's
//! `PassReport::speed_mult` (the flash and the glow); and in `nova_sim`'s
//! session, through `Session::speed_mult` at each ship's display and
//! each `_PrepareAllSprites`. `_LowLevelAIHandler`'s,
//! `_HighLevelAIHandler`'s, `_AIDispatch`'s and `_AIMaintainFormation`'s
//! readers (rdm:phase/play-feel-8/phase-25-ai-reads-the-gain) are in
//! `nova_sim_ai`'s traffic: the dispatch, through the pass's
//! `World::speed_mult` (an own jump's stage, the pods' waits, the regroup
//! draw and the drift); the low level, through `Readings::speed_mult`
//! (the formation nudge, the dock pull, an inertialess plunderer's match,
//! the dock creep and the mining steps); and the think, through
//! `Surroundings::speed_mult` (the lost-sight timer and the mining fire's
//! hold); and in `nova_sim`'s session, through `Session::speed_mult` in
//! each think run's `World`. The readers of `_HandleExplods`,
//! `_HandleSmoke`, `_HandleBoxes`, `_HandleAsteroids`,
//! `_HandleEscapePods`, both bracket routines, `_HandleStellarWeapons`,
//! `_HandleReinforcements` and `_ShipCanSelfRepair`
//! (rdm:phase/play-feel-8/phase-26-effects-read-the-gain) are in
//! `nova_sim_combat`, the stellar guns' reload through the pass's
//! `Field::speed_mult`, and the asteroids' drift and spin
//! (`Asteroids::step`) and the self-repair draw
//! (`self_repair::repair_odds`) as values; in `nova_sim_ai`, the boxes'
//! drift, spin and life (`CargoBoxes::step`), the fly-ins
//! (`FlyIn::step`, `FlyIn::expand`) and the reinforcement timer
//! (`reinforcements::tick`), as values; in the view, through the pass's
//! `PassReport::speed_mult` (the explosions, the smoke and the escape
//! pods); and in `nova_sim`'s session, through `Session::speed_mult`,
//! and `Session::next_speed_mult` for a fresh fly-in's first draw.
//!
//! # Its writers
//!
//! - [`TimeAdjustment::frame`]: `_HandleTimeAdjustment` @0x33424.
//! - [`TimeAdjustment::switched_in`]: `_SwitchedIn` @0x513e1-0x513ec, 1.0
//!   wherever the game is.
//! - [`TimeAdjustment::zero`]: 0.0 at `_PlayerLandOnStellar`'s tail
//!   @0x6375e-0x63769 (after `_DoPortDialog`), `_HandlePlayer` after the
//!   player and mission info dialogs (@0x70357, @0x704e8), and the escort,
//!   comm and planet comm dialogs' closes (@0x95693, @0x9690e, @0x97af2).
//! - [`TimeAdjustment::zero_unheld`]: the same `_HandlePlayer` writes
//!   after an info dialog whose own guard returned at once (the screen
//!   faded, a jump, breaking up): no dialog lasted, so the next frame's
//!   `new` is about 1.0 and it ramps from 0.0 to 0.25.
//! - [`TimeAdjustment::entered`]: game entry finds the gain the last game
//!   left, as no `_HandleTimeAdjustment` runs before `_PlayGame`'s loop.
//!
//! # Left out of `_HandleTimeAdjustment`
//!
//! - Its fast-frame yield (@0x3336e-0x333a9), one `_IterateInGameEventLoop`
//!   for a frame of 9 ms or less: the engine's app polls events between
//!   every frame.
//! - `_lastEventLoopTime` (@0x333a9): nothing reads it.
//! - `_aiComplexity` (@0x3343f-0x33491), 1 to 16 by the gain: nothing reads
//!   it.
//! - `new` < 0.01 and the 32-bit microsecond wrap: the engine's step is
//!   never under a millisecond, and the wrap is an integer width.
//! - `new` from measured time, apart from the 334 ms hold: the engine's
//!   step model, above.
//!
//! No `PlaySound` or `_beepSnd` is made.

use std::time::Duration;

/// The shortest gap between frames that `_HandleTimeAdjustment` holds the
/// gain over: `new` = `ms` × 0.03 reaches 10.0 (@0xdd65c) at 334 whole
/// milliseconds (see the module docs).
pub const HELD_GAP: Duration = Duration::from_millis(334);

/// Whether a gap of `gap` without a frame holds the next one's gain
/// ([`TimeAdjustment::hold`]): [`HELD_GAP`] or more.
#[must_use]
pub fn holds_after(gap: Duration) -> bool {
    gap >= HELD_GAP
}

/// The weight of the old gain in a frame's ramp, the double 3.0 @0xdd490.
const OLD_WEIGHT: f64 = 3.0;

/// The share each frame's sum keeps, the double 0.25 @0xdd130.
const QUARTER: f64 = 0.25;

/// A frame's new time at the engine's fixed step, in the original's frames
/// of 33.3 ms: one.
const STEP_FRAMES: f64 = 1.0;

/// The frame's time gain, `_gSpeedMult`: what each timed pass multiplies
/// its motion and timers by (see the module docs).
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct SpeedMult(f64);

impl SpeedMult {
    /// A full frame's gain, 1.0, as `_SwitchedIn` sets it.
    pub const FULL: Self = Self(1.0);

    /// No time at all, 0.0, as a landing's tail or a dialog's close sets
    /// it.
    pub const ZERO: Self = Self(0.0);

    /// The gain the ramp settles at after a zero, 1 − 2^-52, one ulp
    /// below [`FULL`](Self::FULL), where [`TimeAdjustment::frame`] gives
    /// it again (see the module docs' "The ramp"). A float timer takes it
    /// as a whole frame ([`added_to_float`](Self::added_to_float)).
    pub const SETTLED: Self = Self(1.0 - f64::EPSILON);

    /// The gain `value`.
    #[must_use]
    pub const fn new(value: f64) -> Self {
        Self(value)
    }

    /// The gain as a double, as the passes read it.
    #[must_use]
    pub const fn get(self) -> f64 {
        self.0
    }

    /// The gain narrowed to a float, as a pass that adds it to a float
    /// field in single precision reads it.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub const fn as_f32(self) -> f32 {
        self.0 as f32
    }

    /// `x` × the gain, a product the original works in doubles and passes
    /// on as a float (an acceleration handed to `_AdjustedAccel`, a
    /// strength to `_ApplyGravity`): `(double)x × g`, narrowed.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn times(self, x: f32) -> f32 {
        (f64::from(x) * self.0) as f32
    }

    /// `value` after it gains `by` × the gain, as the original adds a
    /// scaled step to a float field (a position, a heading, a gauge, a
    /// timer): `(double)value + (double)by × g`, narrowed. At
    /// [`FULL`](Self::FULL) it is the float sum `value + by`, as one
    /// double add narrowed rounds as the float add does.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn add_times(self, value: f32, by: f32) -> f32 {
        (f64::from(value) + f64::from(by) * self.0) as f32
    }

    /// `timer` after it takes the gain, as a pass adds it to a timer the
    /// original keeps as a float: the sum in doubles, narrowed back to a
    /// float (see the module docs' "Float timers"). From
    /// [`FLOAT_WHOLE_FRAMES`] on, where a float no longer counts whole
    /// frames, the timer keeps the double sum.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn added_to_float(self, timer: f64) -> f64 {
        let sum = timer + self.0;
        let narrowed = f64::from(sum as f32);
        if narrowed.abs() < FLOAT_WHOLE_FRAMES {
            narrowed
        } else {
            sum
        }
    }
}

/// The frames a float timer counts one by one, 2^24: a float's whole
/// numbers run out above it, so a float at 2^24 that takes a frame stays
/// there.
pub const FLOAT_WHOLE_FRAMES: f64 = 16_777_216.0;

impl Default for SpeedMult {
    /// [`SpeedMult::FULL`]: the launch's resume event runs `_SwitchedIn`
    /// (`_DoMainScreen` @0x1b33c) before any game.
    fn default() -> Self {
        Self::FULL
    }
}

/// The ramp's state, `_HandleTimeAdjustment`'s: the gain and whether the
/// next frame is held (see the module docs).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TimeAdjustment {
    /// The gain the last frame ran at, or a writer set.
    gain: SpeedMult,
    /// Whether the next frame keeps the gain: a gap of 334 ms or more
    /// came before it.
    held: bool,
}

impl TimeAdjustment {
    /// The ramp at `gain`, the next frame not held.
    #[must_use]
    pub const fn at(gain: SpeedMult) -> Self {
        Self { gain, held: false }
    }

    /// Runs `_HandleTimeAdjustment` for one engine step and gives the
    /// frame's gain: held, the gain stays and the hold clears; otherwise it
    /// becomes (3 × gain + 1) × 0.25, in doubles.
    pub fn frame(&mut self) -> SpeedMult {
        self.gain = self.next();
        self.held = false;
        self.gain
    }

    /// The gain the next [`frame`](Self::frame) gives, which the view
    /// steps the stellars by between steps.
    #[must_use]
    pub fn next(self) -> SpeedMult {
        if self.held {
            self.gain
        } else {
            SpeedMult((OLD_WEIGHT * self.gain.0 + STEP_FRAMES) * QUARTER)
        }
    }

    /// The gain as it stands: the last frame's, or a writer's since.
    #[must_use]
    pub const fn gain(self) -> SpeedMult {
        self.gain
    }

    /// Sets the gain to 0.0 and holds the next frame: a landing's tail or
    /// a dialog's close, which the player spent at least 334 ms in.
    pub fn zero(&mut self) {
        self.gain = SpeedMult::ZERO;
        self.held = true;
    }

    /// Sets the gain to 0.0 and leaves the next frame unheld: a press a
    /// dialog's own guard refused before anything opened, so only a
    /// frame's time passes before the next frame, which ramps from 0.0. A
    /// hold already set (a gap before the press) stays.
    pub fn zero_unheld(&mut self) {
        self.gain = SpeedMult::ZERO;
    }

    /// Sets the gain to 1.0 and holds the next frame: `_SwitchedIn`, after
    /// the app was in the background.
    pub fn switched_in(&mut self) {
        self.gain = SpeedMult::FULL;
        self.held = true;
    }

    /// Holds the next frame alone: a gap of 334 ms or more between frames.
    pub fn hold(&mut self) {
        self.held = true;
    }

    /// Game entry at `gain`, the one the last game left (or the
    /// switch-in's): the next frame is held, as the main menu lasts at
    /// least 334 ms.
    pub fn entered(&mut self, gain: SpeedMult) {
        self.gain = gain;
        self.held = true;
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)] // the ramp's first gains are exact in binary
mod tests {
    use super::*;

    fn frames(time: &mut TimeAdjustment, n: usize) -> Vec<f64> {
        (0..n).map(|_| time.frame().get()).collect()
    }

    #[test]
    fn a_frame_takes_three_quarters_of_the_old_gain_and_a_quarter_of_a_full_frame() {
        let mut time = TimeAdjustment::at(SpeedMult::ZERO);
        assert_eq!(frames(&mut time, 3), vec![0.25, 0.4375, 0.578_125]);
    }

    #[test]
    fn the_frame_after_a_zero_keeps_it() {
        let mut time = TimeAdjustment::default();
        time.zero();
        assert_eq!(frames(&mut time, 3), vec![0.0, 0.25, 0.4375]);
    }

    #[test]
    fn the_frame_after_an_unheld_zero_ramps_from_it() {
        let mut time = TimeAdjustment::default();
        time.zero_unheld();
        assert_eq!(time.gain(), SpeedMult::ZERO);
        assert_eq!(frames(&mut time, 3), vec![0.25, 0.4375, 0.578_125]);
    }

    #[test]
    fn an_unheld_zero_keeps_a_gap_already_held() {
        let mut time = TimeAdjustment::default();
        time.hold();
        time.zero_unheld();
        assert_eq!(frames(&mut time, 2), vec![0.0, 0.25]);
    }

    #[test]
    fn a_full_gain_stays_full() {
        let mut time = TimeAdjustment::default();
        assert_eq!(frames(&mut time, 3), vec![1.0, 1.0, 1.0]);
    }

    #[test]
    fn switched_in_sets_full_whatever_the_ramp() {
        let mut time = TimeAdjustment::default();
        time.zero();
        frames(&mut time, 2);
        time.switched_in();
        assert_eq!(time.gain(), SpeedMult::FULL);
        assert_eq!(frames(&mut time, 2), vec![1.0, 1.0]);
    }

    #[test]
    fn a_hold_keeps_a_ramp_where_it_stands() {
        let mut time = TimeAdjustment::default();
        time.zero();
        frames(&mut time, 2);
        time.hold();
        assert_eq!(frames(&mut time, 2), vec![0.25, 0.4375]);
    }

    #[test]
    fn the_gain_never_rises_above_full() {
        let mut time = TimeAdjustment::at(SpeedMult::ZERO);
        let gains = frames(&mut time, 200);
        assert!(gains.iter().all(|&gain| gain <= 1.0));
        assert!(gains.windows(2).all(|pair| pair[0] <= pair[1]));
    }

    #[test]
    fn the_ramp_settles_one_ulp_below_full_as_the_originals_doubles_do() {
        let mut time = TimeAdjustment::default();
        time.zero();
        let gains = frames(&mut time, 200);
        assert_eq!(gains[199], 1.0 - f64::EPSILON);
        assert_eq!(SpeedMult::new(gains[199]), SpeedMult::SETTLED);
        assert_eq!(time.frame(), SpeedMult::SETTLED, "it stays there");
    }

    #[test]
    fn a_float_timer_takes_the_settled_ramp_as_a_whole_frame() {
        let settled = SpeedMult::SETTLED;
        assert_eq!(settled.added_to_float(0.0), 1.0);
        assert_eq!(settled.added_to_float(1.0), 2.0);
        assert_eq!(settled.added_to_float(-1.0), -f64::EPSILON);
    }

    #[test]
    fn a_float_timer_keeps_what_a_float_holds_of_a_gain() {
        assert_eq!(SpeedMult::new(0.25).added_to_float(0.5), 0.75);
        assert_eq!(SpeedMult::ZERO.added_to_float(3.0), 3.0);
        let third = SpeedMult::new(1.0 / 3.0);
        assert_eq!(
            third.added_to_float(0.0),
            f64::from(SpeedMult::new(1.0 / 3.0).as_f32())
        );
        assert_ne!(third.added_to_float(0.0), 1.0 / 3.0);
    }

    #[test]
    fn past_a_floats_whole_frames_the_timer_keeps_counting_in_doubles() {
        let wide = FLOAT_WHOLE_FRAMES;
        assert_eq!(SpeedMult::FULL.added_to_float(wide - 1.0), wide);
        assert_eq!(SpeedMult::FULL.added_to_float(wide), wide + 1.0);
        assert_eq!(SpeedMult::FULL.added_to_float(-wide - 4.0), -wide - 3.0);
        let widest = f64::from(i16::MAX).powi(2);
        assert_eq!(SpeedMult::FULL.added_to_float(widest - 2.0), widest - 1.0);
    }

    #[test]
    fn next_is_what_the_next_frame_gives() {
        let mut time = TimeAdjustment::default();
        time.zero();
        for _ in 0..4 {
            let next = time.next();
            assert_eq!(time.frame(), next);
        }
    }

    #[test]
    fn entered_holds_the_carried_gain_for_one_frame() {
        let mut time = TimeAdjustment::default();
        time.entered(SpeedMult::ZERO);
        assert_eq!(time.gain(), SpeedMult::ZERO);
        assert_eq!(frames(&mut time, 2), vec![0.0, 0.25]);
        time.entered(SpeedMult::FULL);
        assert_eq!(frames(&mut time, 2), vec![1.0, 1.0]);
    }

    #[test]
    fn a_gap_of_334_ms_or_more_holds_the_next_frame() {
        assert!(!holds_after(Duration::from_millis(333)));
        assert!(holds_after(Duration::from_millis(334)));
        assert!(holds_after(Duration::from_secs(5)));
    }

    #[test]
    fn times_is_the_product_in_doubles_narrowed() {
        assert_eq!(SpeedMult::new(0.5).times(0.3), 0.15);
        assert_eq!(SpeedMult::new(0.25).times(6.0), 1.5);
        let third = 1.0_f32 / 3.0;
        assert_eq!(
            SpeedMult::new(0.1).times(third),
            (f64::from(third) * 0.1) as f32
        );
    }

    #[test]
    fn add_times_adds_the_scaled_step_in_doubles_narrowed() {
        assert_eq!(SpeedMult::new(0.5).add_times(10.0, 3.0), 11.5);
        assert_eq!(SpeedMult::new(0.25).add_times(100.0, -1.0), 99.75);
        let big = 16_777_216.0_f32;
        assert_eq!(
            SpeedMult::new(0.75).add_times(big, 1.0),
            (f64::from(big) + 0.75) as f32
        );
    }

    #[test]
    fn at_full_gain_both_are_the_plain_float_arithmetic() {
        let values = [0.0_f32, 0.3, -1.25, 149.0, 1.0e7, 0.033_33, 6.1e-5];
        for &x in &values {
            assert_eq!(SpeedMult::FULL.times(x), x, "{x}");
            for &by in &values {
                assert_eq!(SpeedMult::FULL.add_times(x, by), x + by, "{x} + {by}");
            }
        }
    }

    #[test]
    fn at_no_gain_the_value_stays() {
        assert_eq!(SpeedMult::ZERO.times(7.5), 0.0);
        for x in [0.0_f32, 1.5, -300.0, 1.0e-3] {
            assert_eq!(SpeedMult::ZERO.add_times(x, 2.75), x);
        }
    }

    #[test]
    fn at_the_settled_gain_a_whole_timer_still_gains_a_whole_frame() {
        for x in [0.0_f32, 2.0, 29.0, 299.0, -1.0, 149.0] {
            assert_eq!(SpeedMult::SETTLED.add_times(x, -1.0), x - 1.0, "{x}");
        }
        for x in [0.0_f32, 1.0, 29.0, 299.0, -2.0, 149.0] {
            assert_eq!(SpeedMult::SETTLED.add_times(x, 1.0), x + 1.0, "{x}");
        }
        // Only a sum that lands on none keeps the ulp, as a float can.
        assert_eq!(
            SpeedMult::SETTLED.add_times(-1.0, 1.0),
            -f64::EPSILON as f32
        );
        assert_eq!(SpeedMult::SETTLED.add_times(1.0, -1.0), f64::EPSILON as f32);
    }

    #[test]
    fn a_full_gain_is_the_default_and_reads_as_a_float() {
        assert_eq!(SpeedMult::default(), SpeedMult::FULL);
        assert_eq!(SpeedMult::new(0.25).as_f32(), 0.25_f32);
        assert_eq!(SpeedMult::new(0.25).get(), 0.25);
    }
}
