//! A stellar's animation: which frame of its sheet shows, stepped tick by
//! tick, as the original's sprite tick steps it every play frame
//! (`_HandleStellarSprites` @0x2e77b-0x2ee35, stellar-fidelity phase 1).
//!
//! A stellar animates when the frame count it animates over (its
//! counted sheet's) is above 1, it is not a hypergate, and its destroyed
//! state is the one its flags name: a stellar marked to animate only when
//! destroyed (`Flags2` 0x0080) animates only while destroyed, and any
//! other only while not (@0x2e784-0x2e7bb, @0x2e802). Nothing is
//! destroyed in a viewer. A stellar that does not animate shows frame 0
//! (@0x2ee20), its timer and frames frozen. A hypergate (`Flags2` 0x1000,
//! @0x2e832) takes its own branch, which its session steps
//! (`nova_sim::gate_sprite`, system-traffic-fidelity phase 7): it draws
//! that frame (`SceneStellar::gate_frame`), and frame 0 where no session
//! runs, as a released, closed gate shows.
//!
//! Each tick an animating stellar's timer gains one tick of 1/30 s (the
//! original's `_gSpeedMult`, 1.0 a frame at 30 frames a second,
//! @0x2e810-0x2e825). Once it reaches `AnimDelay`, or `AnimDelay` x
//! `Frame0Bias` while the current frame is 0 and the bias is above 1
//! (@0x2ec4b-0x2ec83), the timer starts again and the frames advance. A
//! threshold of 0 or less is met every tick: an `AnimDelay` of 0 or less
//! advances every tick, as nothing in the original gates on it. The
//! product is kept wide where the original's 16-bit `imul` wraps.
//!
//! How the frames advance (@0x2ec9b-0x2edf4):
//!
//! - Plainly, the current frame takes the next, and the next becomes the
//!   one after it modulo the counted frame count.
//! - `Flags2` 0x0001 (alternate) shows frame 0 between every other frame:
//!   from any other frame the current goes back to 0, the next kept; from
//!   frame 0 it takes the next, and the next becomes the one after it,
//!   its wrap skipping frame 0: 0, 1, 0, 2, 0, 3, 0, 1, ...
//! - `Flags2` 0x0002 (random) picks each next frame at random on the
//!   [`Chance`] the caller gives: uniform over the counted frames but the
//!   one just made current, and but frame 0 as well under 0x0001. With no
//!   frame left to pick (two frames under both), where the original draws
//!   forever, the next frame stays.
//!
//! Then the sheet drawn shows the current frame modulo its own frame
//! count (`_SWSetCurrentFrameIndex` @0xa90d4-0xa90e9). Between advances
//! the sheet drawn keeps the frame it shows; a swap to another sheet
//! keeps it while it is in range, and shows frame 0 otherwise
//! (`_SWCloneFramesFromMaster` @0xac504-0xac532). A frozen slot steps its
//! frames and draws its random picks, while the frame shown stays.

use std::num::NonZeroU16;

use nova_sim::Chance;
use nova_sim::gate::HYPERGATE;
use nova_sim::gate_sprite::{ALTERNATE, ONLY_WHEN_DESTROYED, RANDOM};

use super::catalog::AnimationData;

/// Whether a stellar with these fields, `destroyed` or not, animates over
/// `counted` frames by the sequential branch: never a hypergate, whose
/// frames its session steps.
#[must_use]
pub fn is_animated(data: AnimationData, destroyed: bool, counted: NonZeroU16) -> bool {
    counted.get() > 1
        && data.flags2 & HYPERGATE == 0
        && (data.flags2 & ONLY_WHEN_DESTROYED != 0) == destroyed
}

/// A stellar's animation state: its current and next frame, its timer,
/// and the frame its sheet drawn shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Frames {
    /// The current frame, in the counted sheet's frames.
    current: u16,
    /// The frame after it.
    next: u16,
    /// Ticks since the last advance.
    timer: u64,
    /// The frame of the sheet drawn that shows.
    shown: u16,
}

impl Default for Frames {
    fn default() -> Self {
        Self::new()
    }
}

impl Frames {
    /// A stellar's state the first time a flight shows it: frame 0
    /// current and shown, frame 1 next, the timer at 0, as a newly made
    /// sprite slot sets it (`_HandleStellarSystemVisibility`
    /// @0x331e2-0x331fb; `_InitObjects` @0x1c8c5-0x1c8d1 sets the frame
    /// and timer at launch). A stellar shown again resumes the state it
    /// was left in (`KeptFrames`, stellar-fidelity phase 12).
    #[must_use]
    pub const fn new() -> Self {
        Self {
            current: 0,
            next: 1,
            timer: 0,
            shown: 0,
        }
    }

    /// The frame of the sheet drawn that shows.
    #[must_use]
    pub fn shown(&self) -> u16 {
        self.shown
    }

    /// Shows `frame` of the sheet drawn: the frame a hypergate's session
    /// gave it, kept as its slot freezes (`SystemScene::set_frozen`).
    pub fn hold(&mut self, frame: u16) {
        self.shown = frame;
    }

    /// A swap to another sheet drawn, of `drawn` frames: the frame shown
    /// is kept while it is in range, and is frame 0 otherwise.
    pub fn swap(&mut self, drawn: NonZeroU16) {
        if self.shown >= drawn.get() {
            self.shown = 0;
        }
    }

    /// One tick for a stellar with these fields, `destroyed` or not,
    /// animating over `counted` frames and drawing a sheet of `drawn`, a
    /// random next frame (`Flags2` 0x0002) drawn on `chance`.
    pub fn step(
        &mut self,
        data: AnimationData,
        destroyed: bool,
        counted: NonZeroU16,
        drawn: NonZeroU16,
        chance: &mut dyn Chance,
    ) {
        self.step_by(1, data, destroyed, counted, drawn, chance);
    }

    /// One pass at `gain` ticks, as [`Frames::step`] runs one at 1: the
    /// timer takes the gain (`_gSpeedMult`, @0x2e810-0x2e825) and the
    /// threshold is tested whatever it is, so at gain 0 a delay of 0, or a
    /// timer already there, still advances.
    pub fn step_by(
        &mut self,
        gain: u8,
        data: AnimationData,
        destroyed: bool,
        counted: NonZeroU16,
        drawn: NonZeroU16,
        chance: &mut dyn Chance,
    ) {
        match self.advance(gain, data, destroyed, counted, chance) {
            None => self.shown = 0,
            Some(true) => self.shown = self.current % drawn.get(),
            Some(false) => {}
        }
    }

    /// One tick for a stellar whose sprite slot shows a frame frozen
    /// (`Session::frame_frozen`, stellar-fidelity phase 8): its current
    /// and next frames and its timer step as [`Frames::step`] steps them,
    /// random draws included, but the frame shown stays, whether it
    /// animates or not, as every `_SWSetCurrentFrameIndex` the sprite tick
    /// calls refuses a slot whose frames count none (@0x2ee20-0x2ee30,
    /// @0xa90e7-0xa90ee).
    pub fn step_frozen(
        &mut self,
        data: AnimationData,
        destroyed: bool,
        counted: NonZeroU16,
        chance: &mut dyn Chance,
    ) {
        self.step_frozen_by(1, data, destroyed, counted, chance);
    }

    /// One pass at `gain` ticks for a stellar whose sprite slot shows a
    /// frame frozen, as [`Frames::step_frozen`] runs one at 1 (see
    /// [`Frames::step_by`]).
    pub fn step_frozen_by(
        &mut self,
        gain: u8,
        data: AnimationData,
        destroyed: bool,
        counted: NonZeroU16,
        chance: &mut dyn Chance,
    ) {
        let _ = self.advance(gain, data, destroyed, counted, chance);
    }

    /// One pass of `gain` ticks on the timer and the current and next
    /// frames over `counted`: `None` when the stellar does not animate,
    /// else whether the current frame advanced.
    fn advance(
        &mut self,
        gain: u8,
        data: AnimationData,
        destroyed: bool,
        counted: NonZeroU16,
        chance: &mut dyn Chance,
    ) -> Option<bool> {
        if !is_animated(data, destroyed, counted) {
            return None;
        }
        self.timer += u64::from(gain);
        // Wide, where the original's 16-bit `imul ax` wraps (@0x2ec62);
        // a threshold of 0 or less is met at once.
        let delay = i64::from(data.delay);
        let threshold = if self.current == 0 && data.frame0_bias > 1 {
            delay * i64::from(data.frame0_bias)
        } else {
            delay
        };
        if i128::from(self.timer) < i128::from(threshold) {
            return Some(false);
        }
        self.timer = 0;
        let alternate = data.flags2 & ALTERNATE != 0;
        if alternate && self.current != 0 {
            // Back to frame 0, the next frame kept (@0x2eca5-0x2ecad).
            self.current = 0;
            return Some(true);
        }
        self.current = self.next;
        self.next = if data.flags2 & RANDOM != 0 {
            self.pick(counted, alternate, chance)
        } else {
            // Below `counted`, which is a u16.
            let after = ((u32::from(self.next) + 1) % u32::from(counted.get())) as u16;
            // Alternating, the wrap skips frame 0 (@0x2ed2e-0x2ed63).
            if alternate && after == 0 { 1 } else { after }
        };
        Some(true)
    }

    /// A random next frame drawn on `chance`: uniform over the `counted`
    /// frames but the current one, and but frame 0 when `alternate`, as
    /// the original's `_Rand(count)` drawn again while refused gives it
    /// (@0x2ece5-0x2ed1e, @0x2ed95-0x2edda). One draw over the frames
    /// allowed picks from the same set with the same odds, and always
    /// ends. When none is allowed (two frames, alternating), where the
    /// original draws forever, the next frame stays as it was.
    fn pick(&self, counted: NonZeroU16, alternate: bool, chance: &mut dyn Chance) -> u16 {
        let floor = u16::from(alternate);
        let count = counted.get();
        let skips_current = (floor..count).contains(&self.current);
        let allowed = count - floor - u16::from(skips_current);
        if allowed == 0 {
            return self.next;
        }
        // Below `allowed`, a u16; a source drawing out of range takes the
        // last frame allowed.
        let index = chance.below(u32::from(allowed)).min(u32::from(allowed) - 1) as u16;
        let frame = floor + index;
        if skips_current && frame >= self.current {
            frame + 1
        } else {
            frame
        }
    }

    /// The take-off's reset (`_HandlePlayerDockRequest` @0x675ce-0x675fa,
    /// play-feel-4 phase 14): the current and next frames to 0, the timer
    /// kept, then shown as the rebuild that follows shows it (@0x67653,
    /// [`Frames::reshow`]): frame 0, the current frame 0 modulo any sheet,
    /// and graphic type 0's frame 0 alike.
    pub fn take_off_reset(&mut self) {
        self.current = 0;
        self.next = 0;
        self.shown = 0;
    }

    /// A stellar's state the first time a flight shows it after a
    /// take-off's reset: frame 0 current and shown, frame 0 next, the
    /// timer at 0, as the reset leaves the `spöb` record `_InitObjects`
    /// set (@0x1c8c5-0x1c8d1).
    #[must_use]
    pub const fn after_reset() -> Self {
        Self {
            current: 0,
            next: 0,
            timer: 0,
            shown: 0,
        }
    }

    /// A rebuild point: the sheet drawn, of `drawn` frames, shows the
    /// current frame, modulo its frame count (@0x33096); or frame 0 when
    /// it is graphic type 0's, which fills an empty slot (@0x332d0).
    pub fn reshow(&mut self, drawn: NonZeroU16, type_zero: bool) {
        self.shown = if type_zero {
            0
        } else {
            self.current % drawn.get()
        };
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use nova_sim::NeverFires;

    use super::*;
    use crate::flight::SharedChance;

    fn count(n: u16) -> NonZeroU16 {
        NonZeroU16::new(n).expect("non-zero")
    }

    fn data(delay: i16, frame0_bias: i16) -> AnimationData {
        AnimationData {
            delay,
            frame0_bias,
            flags2: 0,
        }
    }

    /// The frame shown after 0, 1, ... `len - 1` ticks of a whole stellar
    /// with `data`, counting `counted` frames and drawing `drawn`.
    fn shown_over(data: AnimationData, counted: u16, drawn: u16, len: usize) -> Vec<u16> {
        let mut frames = Frames::new();
        let mut shown = Vec::new();
        for _ in 0..len {
            shown.push(frames.shown());
            frames.step(data, false, count(counted), count(drawn), &mut NeverFires);
        }
        shown
    }

    /// The frame shown after 0, 1, ... `len - 1` ticks of a whole stellar
    /// with `data` over a sheet of `frames`.
    fn plain(data: AnimationData, frames: u16, len: usize) -> Vec<u16> {
        shown_over(data, frames, frames, len)
    }

    #[test]
    fn a_wormhole_shows_each_of_its_32_frames_for_2_ticks_and_wraps() {
        let wormhole = plain(data(2, -1), 32, 64 * 1000 + 6);
        assert_eq!(wormhole[..5], [0, 0, 1, 1, 2]);
        assert_eq!(wormhole[62..67], [31, 31, 0, 0, 1]);
        assert_eq!(wormhole[64 * 1000 + 5], 2);
    }

    #[test]
    fn a_bias_above_1_holds_frame_0_that_many_times_as_long() {
        assert_eq!(
            plain(data(2, 3), 4, 14),
            [0, 0, 0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 0, 0]
        );
        // A bias of 1, 0 or below shows frame 0 for one delay.
        for bias in [1, 0, -1] {
            assert_eq!(
                plain(data(2, bias), 4, 9),
                [0, 0, 1, 1, 2, 2, 3, 3, 0],
                "{bias}"
            );
        }
    }

    #[test]
    fn a_delay_of_1_steps_every_tick() {
        assert_eq!(plain(data(1, 0), 3, 7), [0, 1, 2, 0, 1, 2, 0]);
        assert_eq!(plain(data(1, 2), 2, 7), [0, 0, 1, 0, 0, 1, 0]);
    }

    /// `data`, alternating with frame 0 (`Flags2` 0x0001).
    fn alt(delay: i16, frame0_bias: i16) -> AnimationData {
        AnimationData {
            flags2: ALTERNATE,
            ..data(delay, frame0_bias)
        }
    }

    #[test]
    fn alternate_shows_frame_0_between_every_other_frame() {
        assert_eq!(plain(alt(1, 0), 4, 9), [0, 1, 0, 2, 0, 3, 0, 1, 0]);
        assert_eq!(plain(alt(1, 0), 2, 5), [0, 1, 0, 1, 0]);
        assert_eq!(
            plain(alt(1, 2), 4, 12),
            [0, 0, 1, 0, 0, 2, 0, 0, 3, 0, 0, 1]
        );
        assert_eq!(plain(alt(2, 0), 3, 9), [0, 0, 1, 1, 0, 0, 2, 2, 0]);
    }

    #[test]
    fn alternate_leaves_next_alone_on_its_way_to_0() {
        let mut frames = Frames::new();
        let mut states = Vec::new();
        for _ in 0..7 {
            frames.step(alt(1, 0), false, count(4), count(4), &mut NeverFires);
            states.push((frames.current, frames.next));
        }
        assert_eq!(
            states,
            [(1, 2), (0, 2), (2, 3), (0, 3), (3, 1), (0, 1), (1, 2)]
        );
    }

    /// Whether a stellar with `data`, `destroyed` or not, over `frames`
    /// frames, stays on frame 0 for 100 ticks, and is not animated.
    fn stays_on_frame_0(data: AnimationData, destroyed: bool, frames: u16) {
        assert!(!is_animated(data, destroyed, count(frames)), "{data:?}");
        let mut state = Frames::new();
        for _ in 0..100 {
            state.step(
                data,
                destroyed,
                count(frames),
                count(frames),
                &mut Draws::none(),
            );
            assert_eq!(state.shown(), 0, "{data:?}");
        }
    }

    /// Scripted draws: each `below(n)` records `n` and takes the next
    /// scripted value, and panics once the script runs out, so a test
    /// with no script shows that nothing was drawn.
    #[derive(Debug, Default)]
    struct Draws {
        draws: VecDeque<u32>,
        asked: Vec<u32>,
    }

    impl Draws {
        fn new(draws: &[u32]) -> Self {
            Self {
                draws: draws.iter().copied().collect(),
                asked: Vec::new(),
            }
        }

        fn none() -> Self {
            Self::default()
        }
    }

    impl Chance for Draws {
        fn fires(&mut self, percent: u8) -> bool {
            panic!("asked whether {percent}% fires");
        }

        fn below(&mut self, n: u32) -> u32 {
            self.asked.push(n);
            self.draws
                .pop_front()
                .unwrap_or_else(|| panic!("an unscripted below({n})"))
        }
    }

    /// The frame shown after 0, 1, ... `len - 1` ticks of a whole stellar
    /// with `data`, counting `counted` frames and drawing `drawn`, drawing
    /// on `chance`: `len - 1` steps, so a script holds only the draws the
    /// frames shown need.
    fn drawn_over(
        data: AnimationData,
        counted: u16,
        drawn: u16,
        len: usize,
        chance: &mut dyn Chance,
    ) -> Vec<u16> {
        let mut frames = Frames::new();
        let mut shown = vec![frames.shown()];
        for _ in 1..len {
            frames.step(data, false, count(counted), count(drawn), chance);
            shown.push(frames.shown());
        }
        shown
    }

    /// `data` with `flags2`.
    fn flagged(flags2: u16, delay: i16, frame0_bias: i16) -> AnimationData {
        AnimationData {
            flags2,
            ..data(delay, frame0_bias)
        }
    }

    #[test]
    fn random_picks_each_next_frame_among_the_counted_frames_but_the_current() {
        // From frame 1 the frames allowed are 0, 2 and 3; from 3, 0 to 2;
        // from 0, 1 to 3.
        let mut chance = Draws::new(&[2, 0, 1]);
        let random = flagged(RANDOM, 1, 0);
        assert_eq!(drawn_over(random, 4, 4, 4, &mut chance), [0, 1, 3, 0]);
        assert_eq!(chance.asked, [3, 3, 3]);
        // The range is the counted frames', whatever the sheet drawn.
        let mut chance = Draws::new(&[3, 0]);
        assert_eq!(drawn_over(random, 5, 3, 3, &mut chance), [0, 1, 1]);
        assert_eq!(chance.asked, [4, 4], "frame 4 of 3, then 0");
        // Every frame but the current can be picked.
        for (draw, next) in [(0, 0), (1, 2), (2, 3), (3, 4)] {
            let mut frames = Frames::new();
            let mut chance = Draws::new(&[draw]);
            frames.step(random, false, count(5), count(5), &mut chance);
            assert_eq!((frames.current, frames.next), (1, next), "{draw}");
        }
    }

    #[test]
    fn random_with_alternate_picks_among_1_to_count_less_1_but_the_current() {
        // From frame 1 the frames allowed are 2 and 3; from 2, 1 and 3.
        let mut chance = Draws::new(&[0, 1]);
        let both = flagged(ALTERNATE | RANDOM, 1, 0);
        assert_eq!(drawn_over(both, 4, 4, 5, &mut chance), [0, 1, 0, 2, 0]);
        assert_eq!(chance.asked, [2, 2], "no draw on the way back to 0");
        // From 3, 1 and 2.
        let mut chance = Draws::new(&[1, 0, 1]);
        assert_eq!(drawn_over(both, 4, 4, 6, &mut chance), [0, 1, 0, 3, 0, 1]);
        assert_eq!(chance.asked, [2, 2, 2]);
        // From frame 0 itself, every frame but 0 can be picked.
        for (draw, next) in [(0, 1), (1, 2), (2, 3)] {
            let mut frames = Frames {
                next: 0,
                ..Frames::new()
            };
            let mut chance = Draws::new(&[draw]);
            frames.step(both, false, count(4), count(4), &mut chance);
            assert_eq!((frames.current, frames.next), (0, next), "{draw}");
        }
    }

    #[test]
    fn random_with_alternate_over_2_frames_keeps_its_next_where_the_original_hangs() {
        let both = flagged(ALTERNATE | RANDOM, 1, 0);
        let mut chance = Draws::none();
        assert_eq!(drawn_over(both, 2, 2, 5, &mut chance), [0, 1, 0, 1, 0]);
        assert!(chance.asked.is_empty());
    }

    #[test]
    fn random_with_bias_holds_a_picked_frame_0() {
        // Frame 0 shows 3 ticks, 1 one, then the picked 0 three again.
        let mut chance = Draws::new(&[0, 1, 0]);
        let random = flagged(RANDOM, 1, 3);
        assert_eq!(
            drawn_over(random, 3, 3, 8, &mut chance),
            [0, 0, 0, 1, 0, 0, 0, 2]
        );
        assert_eq!(chance.asked, [2, 2, 2]);
    }

    #[test]
    fn a_random_stellar_steps_on_the_default_chance() {
        // The default draws the last outcome: the last frame allowed.
        let mut chance = SharedChance::default();
        let random = flagged(RANDOM, 1, 0);
        assert_eq!(drawn_over(random, 4, 4, 6, &mut chance), [0, 1, 3, 2, 3, 2]);
        let both = flagged(ALTERNATE | RANDOM, 1, 0);
        assert_eq!(
            drawn_over(both, 4, 4, 8, &mut chance),
            [0, 1, 0, 3, 0, 2, 0, 3]
        );
    }

    /// Draws `n` for every `below(n)`, one past the last outcome.
    struct OutOfRange;

    impl Chance for OutOfRange {
        fn fires(&mut self, _percent: u8) -> bool {
            false
        }

        fn below(&mut self, n: u32) -> u32 {
            n
        }
    }

    #[test]
    fn a_draw_out_of_range_takes_the_last_frame_allowed() {
        let random = flagged(RANDOM, 1, 0);
        assert_eq!(drawn_over(random, 4, 4, 4, &mut OutOfRange), [0, 1, 3, 2]);
    }

    #[test]
    fn a_frozen_random_stellar_draws_while_its_frame_shown_stays() {
        let random = flagged(RANDOM, 1, 0);
        let mut frames = Frames::new();
        let mut chance = Draws::new(&[1, 0]);
        frames.step_frozen(random, false, count(4), &mut chance);
        assert_eq!((frames.current, frames.next, frames.shown()), (1, 2, 0));
        frames.step_frozen(random, false, count(4), &mut chance);
        assert_eq!((frames.current, frames.next, frames.shown()), (2, 0, 0));
        assert_eq!(chance.asked, [3, 3]);
    }

    #[test]
    fn an_uncounted_or_still_stellar_draws_nothing() {
        for flags2 in [RANDOM, ALTERNATE | RANDOM] {
            stays_on_frame_0(flagged(flags2, 1, 0), false, 1);
            stays_on_frame_0(flagged(flags2 | ONLY_WHEN_DESTROYED, 1, 0), false, 4);
            stays_on_frame_0(flagged(flags2 | HYPERGATE, 1, 0), false, 4);
            let mut frames = Frames::new();
            frames.step_frozen(flagged(flags2, 1, 0), false, count(1), &mut Draws::none());
            assert_eq!(frames, Frames::new());
        }
    }

    /// A delay of 2, animating only when destroyed or not as
    /// `only_when_destroyed` says.
    fn live(only_when_destroyed: bool) -> AnimationData {
        AnimationData {
            flags2: if only_when_destroyed {
                ONLY_WHEN_DESTROYED
            } else {
                0
            },
            ..data(2, -1)
        }
    }

    #[test]
    fn one_frame_a_hypergate_or_only_when_destroyed_stays_on_frame_0() {
        stays_on_frame_0(data(2, 0), false, 1);
        stays_on_frame_0(gate(0), false, 42);
        stays_on_frame_0(live(true), false, 32);
    }

    /// A hypergate (`Flags2` 0x1000 with 0x0200, as every stock gate's)
    /// with an `AnimDelay` of `delay`.
    fn gate(delay: i16) -> AnimationData {
        AnimationData {
            flags2: HYPERGATE | 0x0200,
            ..data(delay, 0)
        }
    }

    #[test]
    fn a_delay_of_0_or_less_advances_every_tick_as_the_original_does() {
        assert_eq!(plain(data(0, 0), 3, 7), [0, 1, 2, 0, 1, 2, 0]);
        assert_eq!(plain(data(-1, 0), 3, 7), [0, 1, 2, 0, 1, 2, 0]);
        // A bias times a delay of 0 is still 0.
        assert_eq!(plain(data(0, 3), 3, 7), [0, 1, 2, 0, 1, 2, 0]);
        assert_eq!(plain(data(-2, 3), 3, 4), [0, 1, 2, 0]);
        assert!(is_animated(data(0, 0), false, count(42)));
        assert!(is_animated(data(i16::MIN, 0), false, count(2)));
    }

    #[test]
    fn a_hypergate_never_takes_the_sequential_branch() {
        for delay in [0, 2, -1] {
            stays_on_frame_0(gate(delay), false, 42);
            stays_on_frame_0(
                AnimationData {
                    flags2: HYPERGATE,
                    ..data(delay, 0)
                },
                false,
                42,
            );
        }
    }

    #[test]
    fn a_stellar_animates_only_while_its_destroyed_state_is_the_one_flags2_names() {
        assert!(is_animated(live(false), false, count(32)), "whole");
        stays_on_frame_0(live(false), true, 32);
        stays_on_frame_0(live(true), false, 32);
        assert!(is_animated(live(true), true, count(32)), "destroyed");
        let mut dead = Frames::new();
        let mut shown = Vec::new();
        for _ in 0..5 {
            shown.push(dead.shown());
            dead.step(live(true), true, count(32), count(32), &mut NeverFires);
        }
        assert_eq!(shown, [0, 0, 1, 1, 2]);
        // One frame stays on frame 0 whatever the state; no delay
        // animates every tick.
        stays_on_frame_0(live(true), true, 1);
        let no_delay = AnimationData {
            delay: 0,
            ..live(true)
        };
        let mut dead = Frames::new();
        let mut shown = Vec::new();
        for _ in 0..5 {
            shown.push(dead.shown());
            dead.step(no_delay, true, count(4), count(4), &mut NeverFires);
        }
        assert_eq!(shown, [0, 1, 2, 3, 0], "animates every tick");
        stays_on_frame_0(no_delay, false, 4);
    }

    #[test]
    fn the_largest_fields_do_not_overflow() {
        let slow = data(i16::MAX, i16::MAX);
        let most = count(u16::MAX);
        assert!(is_animated(slow, false, most));
        let first = u64::from(i16::MAX.unsigned_abs()).pow(2);
        let mut frames = Frames {
            timer: first - 2,
            ..Frames::new()
        };
        frames.step(slow, false, most, most, &mut NeverFires);
        assert_eq!(frames.shown(), 0, "a tick short");
        frames.step(slow, false, most, most, &mut NeverFires);
        assert_eq!(frames.shown(), 1);
        let mut last = Frames {
            next: u16::MAX - 1,
            timer: u64::from(i16::MAX.unsigned_abs()) - 1,
            ..frames
        };
        last.step(slow, false, most, most, &mut NeverFires);
        assert_eq!((last.current, last.next), (u16::MAX - 1, 0), "wraps");
    }

    #[test]
    fn a_stellar_that_stops_animating_shows_frame_0_and_resumes_from_its_own_frame() {
        let (whole, four) = (data(2, 0), count(4));
        let mut frames = Frames::new();
        for _ in 0..5 {
            frames.step(whole, false, four, four, &mut NeverFires);
        }
        assert_eq!(frames.shown(), 2, "two advances and a tick");
        for _ in 0..7 {
            frames.step(whole, true, four, four, &mut NeverFires);
            assert_eq!(frames.shown(), 0, "still while destroyed");
        }
        frames.step(whole, false, four, four, &mut NeverFires);
        assert_eq!(frames.shown(), 3, "the timer kept its tick");
        frames.step(whole, false, four, four, &mut NeverFires);
        assert_eq!(frames.shown(), 3);
        frames.step(whole, false, four, four, &mut NeverFires);
        assert_eq!(frames.shown(), 0);
    }

    #[test]
    fn a_swap_keeps_the_frame_shown_in_range_and_shows_frame_0_otherwise() {
        let (every_tick, four) = (data(1, 0), count(4));
        let mut frames = Frames::new();
        frames.step(every_tick, false, four, four, &mut NeverFires);
        frames.step(every_tick, false, four, four, &mut NeverFires);
        assert_eq!(frames.shown(), 2);
        frames.swap(count(3));
        assert_eq!(frames.shown(), 2, "in range");
        frames.swap(count(2));
        assert_eq!(frames.shown(), 0, "out of range");
        frames.step(every_tick, false, four, count(2), &mut NeverFires);
        assert_eq!(frames.shown(), 1, "frame 3 of 2 at the next advance");
    }

    #[test]
    fn more_frames_counted_than_drawn_wrap_past_the_end() {
        assert_eq!(plain(data(1, 0), 4, 1), [0]);
        assert_eq!(shown_over(data(1, 0), 4, 3, 8), [0, 1, 2, 0, 0, 1, 2, 0]);
        // The wrapped frame shows for the delay, not the bias.
        assert_eq!(
            shown_over(data(1, 3), 4, 3, 10),
            [0, 0, 0, 1, 2, 0, 0, 0, 0, 1]
        );
    }

    #[test]
    fn fewer_frames_counted_than_drawn_leave_the_rest_unshown() {
        assert_eq!(shown_over(data(1, 0), 2, 4, 6), [0, 1, 0, 1, 0, 1]);
        assert_eq!(shown_over(data(1, 0), 1, 4, 6), [0; 6], "one counted");
    }

    #[test]
    fn a_rebuild_point_shows_the_current_frame_or_0_for_graphic_type_0() {
        let (every_tick, eight) = (data(1, 0), count(8));
        let mut frames = Frames::new();
        for _ in 0..5 {
            frames.step(every_tick, false, eight, eight, &mut NeverFires);
        }
        frames.swap(count(3));
        assert_eq!(frames.shown(), 0, "frame 5 is out of range");
        frames.reshow(count(3), false);
        assert_eq!(frames.shown(), 2, "frame 5 of 3");
        frames.reshow(count(3), true);
        assert_eq!(frames.shown(), 0);
    }

    #[test]
    fn a_frozen_stellar_keeps_the_frame_shown_while_its_current_and_next_advance() {
        let (every_tick, four) = (data(1, 0), count(4));
        let mut frames = Frames::new();
        frames.step(every_tick, false, four, four, &mut NeverFires);
        frames.step(every_tick, false, four, four, &mut NeverFires);
        assert_eq!(frames.shown(), 2);
        frames.step_frozen(every_tick, false, four, &mut NeverFires);
        assert_eq!((frames.current, frames.next, frames.shown()), (3, 0, 2));
        frames.step_frozen(every_tick, false, four, &mut NeverFires);
        assert_eq!((frames.current, frames.next, frames.shown()), (0, 1, 2));
        // Not animating, it still keeps the frame shown, not frame 0.
        for _ in 0..3 {
            frames.step_frozen(every_tick, true, four, &mut NeverFires);
        }
        assert_eq!((frames.current, frames.shown()), (0, 2));
        // A swap after the freeze keeps an in-range frame; the next
        // advance shows the current one.
        frames.swap(count(3));
        assert_eq!(frames.shown(), 2);
        frames.step(every_tick, false, four, count(3), &mut NeverFires);
        assert_eq!(frames.shown(), 1);
    }

    #[test]
    fn a_take_off_reset_zeroes_the_frames_keeps_the_timer_and_shows_frame_0() {
        let (every_2, four) = (data(2, 0), count(4));
        let mut frames = Frames::new();
        for _ in 0..5 {
            frames.step(every_2, false, four, four, &mut NeverFires);
        }
        assert_eq!(
            (frames.current, frames.next, frames.timer, frames.shown()),
            (2, 3, 1, 2)
        );
        frames.take_off_reset();
        assert_eq!(
            (frames.current, frames.next, frames.timer, frames.shown()),
            (0, 0, 1, 0)
        );
        // The timer carries on: one tick reaches the delay, and the
        // current takes next 0; then frame 1 two ticks on.
        let mut shown = Vec::new();
        for _ in 0..3 {
            frames.step(every_2, false, four, four, &mut NeverFires);
            shown.push(frames.shown());
        }
        assert_eq!(shown, [0, 0, 1]);
    }

    #[test]
    fn at_gain_0_the_timer_stands_and_the_frames_stay() {
        let (every_2, four) = (data(2, 0), count(4));
        let mut frames = Frames::new();
        frames.step_by(0, every_2, false, four, four, &mut NeverFires);
        assert_eq!(frames, Frames::new());
    }

    #[test]
    fn at_gain_0_a_delay_of_0_still_advances() {
        let (no_delay, four) = (data(0, 0), count(4));
        let mut frames = Frames::new();
        frames.step_by(0, no_delay, false, four, four, &mut NeverFires);
        assert_eq!(frames.shown(), 1);
    }

    #[test]
    fn at_gain_0_a_timer_past_the_threshold_fires() {
        let (every_2, four) = (data(2, 0), count(4));
        let mut frames = Frames {
            timer: 3,
            ..Frames::new()
        };
        frames.step_by(0, every_2, false, four, four, &mut NeverFires);
        assert_eq!((frames.current, frames.timer), (1, 0));
    }

    #[test]
    fn at_gain_0_a_frozen_stellar_tests_its_threshold_with_its_frame_kept() {
        let (every_2, four) = (data(2, 0), count(4));
        let mut frames = Frames {
            timer: 2,
            ..Frames::new()
        };
        frames.step_frozen_by(0, every_2, false, four, &mut NeverFires);
        assert_eq!((frames.current, frames.timer, frames.shown()), (1, 0, 0));
        let before = frames;
        frames.step_frozen_by(0, every_2, false, four, &mut NeverFires);
        assert_eq!(frames, before);
    }

    #[test]
    fn gain_1_is_a_step() {
        let (every_2, four) = (data(2, 0), count(4));
        let (mut by, mut plain) = (Frames::new(), Frames::new());
        for tick in 0..9 {
            by.step_by(1, every_2, false, four, four, &mut NeverFires);
            plain.step(every_2, false, four, four, &mut NeverFires);
            assert_eq!(by, plain, "{tick}");
        }
    }

    #[test]
    fn a_stellar_first_shown_after_a_reset_has_next_0_and_its_timer_0() {
        let afresh = Frames::after_reset();
        assert_eq!(
            (afresh.current, afresh.next, afresh.timer, afresh.shown()),
            (0, 0, 0, 0)
        );
    }
}
