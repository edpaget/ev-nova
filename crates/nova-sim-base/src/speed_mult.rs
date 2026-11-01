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
//! # Its writers
//!
//! - [`TimeAdjustment::frame`]: `_HandleTimeAdjustment` @0x33424.
//! - [`TimeAdjustment::switched_in`]: `_SwitchedIn` @0x513e1-0x513ec, 1.0
//!   wherever the game is.
//! - [`TimeAdjustment::zero`]: 0.0 at `_PlayerLandOnStellar`'s tail
//!   @0x6375e-0x63769 (after `_DoPortDialog`), `_HandlePlayer` after the
//!   player and mission info dialogs (@0x70357, @0x704e8), and the escort,
//!   comm and planet comm dialogs' closes (@0x95693, @0x9690e, @0x97af2).
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
}

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
    fn a_full_gain_is_the_default_and_reads_as_a_float() {
        assert_eq!(SpeedMult::default(), SpeedMult::FULL);
        assert_eq!(SpeedMult::new(0.25).as_f32(), 0.25_f32);
        assert_eq!(SpeedMult::new(0.25).get(), 0.25);
    }
}
