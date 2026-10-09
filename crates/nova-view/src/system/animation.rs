//! A stellar's animation: which frame of its sheet shows, stepped tick by
//! tick, as the original's sprite tick steps it every play frame
//! (`_HandleStellarSprites` @0x2e77b-0x2ee35, story-runtime phase 12).
//!
//! A stellar animates when the frame count it animates over (its
//! counted sheet's) is above 1, its `AnimDelay` is above 0, and its
//! destroyed state is the one its flags name: a stellar marked to animate
//! only when destroyed (`Flags2` 0x0080) animates only while destroyed,
//! and any other only while not (@0x2e784-0x2e7bb, @0x2e802). Nothing is
//! destroyed in a viewer. A stellar that does not animate shows frame 0
//! (@0x2ee20), its timer and frames frozen.
//!
//! Each tick an animating stellar's timer gains one tick of 1/30 s (the
//! original's `_gSpeedMult`, 1.0 a frame at 30 frames a second,
//! @0x2e810-0x2e825). Once it reaches `AnimDelay`, or `AnimDelay` x
//! `Frame0Bias` while the current frame is 0 and the bias is above 1
//! (@0x2ec4b-0x2ec83), the timer starts again, the current frame takes
//! the next, the next becomes the one after it modulo the counted frame
//! count (@0x2ed7f, @0x2eddc-0x2edf4), and the sheet drawn shows the
//! current frame modulo its own frame count (`_SWSetCurrentFrameIndex`
//! @0xa90d4-0xa90e9). Between advances the sheet drawn keeps the frame it
//! shows; a swap to another sheet keeps it while it is in range, and
//! shows frame 0 otherwise (`_SWCloneFramesFromMaster` @0xac504-0xac532).
//!
//! The `AnimDelay` of 0 or less that the original advances every frame
//! with, and `Flags2` 0x0001 and 0x0002, are story-runtime phase 21
//! (stellar-animation-flags); hypergates are phase 22
//! (hypergate-animation).

use std::num::NonZeroU16;

use super::catalog::AnimationData;

/// Whether a stellar with these fields, `destroyed` or not, animates over
/// `counted` frames.
#[must_use]
pub fn is_animated(data: AnimationData, destroyed: bool, counted: NonZeroU16) -> bool {
    counted.get() > 1 && data.delay > 0 && data.only_when_destroyed == destroyed
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
    /// A stellar's state as it is laid out: frame 0 current and shown,
    /// frame 1 next, the timer at 0 (`_InitObjects` @0x1c8b9-0x1c8d1).
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

    /// A swap to another sheet drawn, of `drawn` frames: the frame shown
    /// is kept while it is in range, and is frame 0 otherwise.
    pub fn swap(&mut self, drawn: NonZeroU16) {
        if self.shown >= drawn.get() {
            self.shown = 0;
        }
    }

    /// One tick for a stellar with these fields, `destroyed` or not,
    /// animating over `counted` frames and drawing a sheet of `drawn`.
    pub fn step(
        &mut self,
        data: AnimationData,
        destroyed: bool,
        counted: NonZeroU16,
        drawn: NonZeroU16,
    ) {
        if !is_animated(data, destroyed, counted) {
            self.shown = 0;
            return;
        }
        self.timer += 1;
        let delay = u64::from(data.delay.unsigned_abs());
        let threshold = if self.current == 0 && data.frame0_bias > 1 {
            delay * u64::from(data.frame0_bias.unsigned_abs())
        } else {
            delay
        };
        if self.timer >= threshold {
            self.timer = 0;
            self.current = self.next;
            // Below `counted`, which is a u16.
            self.next = ((u32::from(self.next) + 1) % u32::from(counted.get())) as u16;
            self.shown = self.current % drawn.get();
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
    use super::*;

    fn count(n: u16) -> NonZeroU16 {
        NonZeroU16::new(n).expect("non-zero")
    }

    fn data(delay: i16, frame0_bias: i16) -> AnimationData {
        AnimationData {
            delay,
            frame0_bias,
            only_when_destroyed: false,
        }
    }

    /// The frame shown after 0, 1, ... `len - 1` ticks of a whole stellar
    /// with `data`, counting `counted` frames and drawing `drawn`.
    fn shown_over(data: AnimationData, counted: u16, drawn: u16, len: usize) -> Vec<u16> {
        let mut frames = Frames::new();
        let mut shown = Vec::new();
        for _ in 0..len {
            shown.push(frames.shown());
            frames.step(data, false, count(counted), count(drawn));
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

    /// Whether a stellar with `data`, `destroyed` or not, over `frames`
    /// frames, stays on frame 0 for 100 ticks, and is not animated.
    fn stays_on_frame_0(data: AnimationData, destroyed: bool, frames: u16) {
        assert!(!is_animated(data, destroyed, count(frames)), "{data:?}");
        let mut state = Frames::new();
        for _ in 0..100 {
            state.step(data, destroyed, count(frames), count(frames));
            assert_eq!(state.shown(), 0, "{data:?}");
        }
    }

    /// A delay of 2, animating only when destroyed or not as
    /// `only_when_destroyed` says.
    fn live(only_when_destroyed: bool) -> AnimationData {
        AnimationData {
            only_when_destroyed,
            ..data(2, -1)
        }
    }

    #[test]
    fn one_frame_no_delay_or_only_when_destroyed_stays_on_frame_0() {
        stays_on_frame_0(data(2, 0), false, 1);
        // A Hypergate: 42 frames, but no delay.
        stays_on_frame_0(data(0, 0), false, 42);
        stays_on_frame_0(data(-1, 0), false, 42);
        stays_on_frame_0(live(true), false, 32);
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
            dead.step(live(true), true, count(32), count(32));
        }
        assert_eq!(shown, [0, 0, 1, 1, 2]);
        // No delay or one frame stays on frame 0 whatever the state.
        stays_on_frame_0(live(true), true, 1);
        let no_delay = AnimationData {
            delay: 0,
            ..live(true)
        };
        stays_on_frame_0(no_delay, true, 4);
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
        frames.step(slow, false, most, most);
        assert_eq!(frames.shown(), 0, "a tick short");
        frames.step(slow, false, most, most);
        assert_eq!(frames.shown(), 1);
        let mut last = Frames {
            next: u16::MAX - 1,
            timer: u64::from(i16::MAX.unsigned_abs()) - 1,
            ..frames
        };
        last.step(slow, false, most, most);
        assert_eq!((last.current, last.next), (u16::MAX - 1, 0), "wraps");
    }

    #[test]
    fn a_stellar_that_stops_animating_shows_frame_0_and_resumes_from_its_own_frame() {
        let (whole, four) = (data(2, 0), count(4));
        let mut frames = Frames::new();
        for _ in 0..5 {
            frames.step(whole, false, four, four);
        }
        assert_eq!(frames.shown(), 2, "two advances and a tick");
        for _ in 0..7 {
            frames.step(whole, true, four, four);
            assert_eq!(frames.shown(), 0, "still while destroyed");
        }
        frames.step(whole, false, four, four);
        assert_eq!(frames.shown(), 3, "the timer kept its tick");
        frames.step(whole, false, four, four);
        assert_eq!(frames.shown(), 3);
        frames.step(whole, false, four, four);
        assert_eq!(frames.shown(), 0);
    }

    #[test]
    fn a_swap_keeps_the_frame_shown_in_range_and_shows_frame_0_otherwise() {
        let (every_tick, four) = (data(1, 0), count(4));
        let mut frames = Frames::new();
        frames.step(every_tick, false, four, four);
        frames.step(every_tick, false, four, four);
        assert_eq!(frames.shown(), 2);
        frames.swap(count(3));
        assert_eq!(frames.shown(), 2, "in range");
        frames.swap(count(2));
        assert_eq!(frames.shown(), 0, "out of range");
        frames.step(every_tick, false, four, count(2));
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
            frames.step(every_tick, false, eight, eight);
        }
        frames.swap(count(3));
        assert_eq!(frames.shown(), 0, "frame 5 is out of range");
        frames.reshow(count(3), false);
        assert_eq!(frames.shown(), 2, "frame 5 of 3");
        frames.reshow(count(3), true);
        assert_eq!(frames.shown(), 0);
    }
}
