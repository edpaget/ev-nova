//! The player's self-destruct countdown, `_HandlePlayer`'s block Z
//! (@0x6fdb4-0x702bf), as a [`SelfDestruct`] stepped once a tick with
//! whether the chord (Command and the `-` key) is held.
//!
//! The count is `_destructCount` (a float at *0x3b53a8): -1.0 while idle,
//! here `None`. It is never saved; `_ResetPlayer` (@0x1d764) idles it, so
//! a new pilot, a death's end, the escape pod's rescue and a pilot open do
//! ([`SelfDestruct::reset`]). The frame's `mult` is one tick at the
//! engine's fixed rate, so the count is whole ticks and a `u32` matches
//! the float exactly.
//!
//! One step reads, in the original's order:
//!
//! - **The start** (@0x6fde1-0x6febe): the chord held, the count below 0
//!   (idle), and the player not dying (`_IsDying`, -0x2ca, read at the top
//!   of `_HandlePlayer`). It beats [`Beat::Started`] and sets the count to
//!   [`START`]. A dying player's start is silent: nothing is played or
//!   said.
//! - **Running** (count above 0, @0x6fecc): not gated on dying, so a
//!   countdown goes on while the ship breaks up. The start's own tick falls
//!   through to it, so that tick already leaves 149.
//!   - Held (@0x6ff0c-0x700b0): the count drops by one, and a count that
//!     is a multiple of [`BEAT_EVERY`] and at most [`BEAT_FROM`] beats
//!     [`Beat::Count`] with its seconds left (`count / 30`).
//!   - Released (@0x700b5-0x70174): [`Beat::Cancelled`], and the count
//!     idles.
//! - **The end** (@0x7018f-0x702b9): a count that was running, now at most
//!   [`END`] with the chord still held, beats [`Beat::Destruct`] and idles.
//!   With the chord held throughout, that is the 149th tick counting the
//!   press.
//!
//! No count is both a multiple of 30 and at most 1, so a step beats at
//! most once. What each beat plays, says and does is the session's (see
//! `nova_sim::Session::hold_self_destruct`).
//!
//! ## Left out
//!
//! - The no-decrement branch for a count below 1.0 and a fractional `mult`
//!   (@0x6ff18-0x6ff1b): unreachable in whole ticks.
//! - The `(short)` of the modulo test (@0x6ff36): an integer width.

/// The count a start sets (0x43160000, @0x6febe).
pub const START: u32 = 150;

/// The count beats every this many ticks (the divide by 30 @0x6ff3b).
pub const BEAT_EVERY: u32 = 30;

/// The highest count that beats (120.0, 0xdd970 @0x6ff64).
pub const BEAT_FROM: u32 = 120;

/// The count at or below which a held countdown ends (1.0, 0xdd068
/// @0x7017a).
pub const END: u32 = 1;

/// What one step of the countdown did, for the session to play and say.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Beat {
    /// The countdown began (`snd ` 153, "Self-destruct sequence
    /// initiated.").
    Started,
    /// A countdown line, `seconds` left (`snd ` 152, "Self-destruct in N
    /// seconds.").
    Count {
        /// The seconds left, `count / 30`: 4, 3, 2 then 1.
        seconds: u32,
    },
    /// The chord was let go while counting (`snd ` 151, "Self-destruct
    /// sequence cancelled.").
    Cancelled,
    /// The countdown ran out with the chord held: the player and the ships
    /// docked with it are destroyed (`snd ` 151, "Have a nice day.").
    Destruct,
}

/// `_destructCount`: idle, or the ticks left (see the module docs).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SelfDestruct {
    count: Option<u32>,
}

impl SelfDestruct {
    /// The ticks left, or `None` while idle.
    #[must_use]
    pub fn count(&self) -> Option<u32> {
        self.count
    }

    /// A countdown with `count` ticks left, for tests that start from a
    /// pass of it.
    #[cfg(any(test, feature = "fixture"))]
    #[must_use]
    pub fn at(count: u32) -> Self {
        Self { count: Some(count) }
    }

    /// `_ResetPlayer`'s part (@0x1d764): the count idles.
    pub fn reset(&mut self) {
        self.count = None;
    }

    /// Whether a pass with the chord held sets `_keyPressedThisFrame`:
    /// the start (@0x6fe02) and the destruct (@0x701ca), and none of the
    /// passes between them. `dying` is as [`SelfDestruct::step`] reads it.
    #[must_use]
    pub fn chord_fades(&self, dying: bool) -> bool {
        match self.count {
            None => !dying,
            Some(count) => count - 1 <= END,
        }
    }

    /// One tick of block Z, with the chord `held` and the player `dying`
    /// as of the top of `_HandlePlayer` (see the module docs).
    pub fn step(&mut self, held: bool, dying: bool) -> Option<Beat> {
        let mut beat = None;
        if held && self.count.is_none() && !dying {
            beat = Some(Beat::Started);
            self.count = Some(START);
        }
        let Some(count) = self.count else {
            return beat;
        };
        if !held {
            self.count = None;
            return Some(Beat::Cancelled);
        }
        let count = count - 1;
        self.count = Some(count);
        if count % BEAT_EVERY == 0 && count <= BEAT_FROM {
            beat = Some(Beat::Count {
                seconds: count / BEAT_EVERY,
            });
        }
        if count <= END {
            self.count = None;
            beat = Some(Beat::Destruct);
        }
        beat
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Steps `ticks` times with the chord held and the player well,
    /// returning each tick's beat, tick 1 first.
    fn hold(destruct: &mut SelfDestruct, ticks: u32) -> Vec<Option<Beat>> {
        (0..ticks).map(|_| destruct.step(true, false)).collect()
    }

    #[test]
    fn an_idle_hold_starts_it_and_the_same_tick_leaves_149() {
        let mut destruct = SelfDestruct::default();
        assert_eq!(destruct.step(true, false), Some(Beat::Started));
        assert_eq!(destruct.count(), Some(149));
    }

    #[test]
    fn a_dying_hold_starts_nothing_and_beats_nothing() {
        let mut destruct = SelfDestruct::default();
        for _ in 0..200 {
            assert_eq!(destruct.step(true, true), None);
        }
        assert_eq!(destruct.count(), None);
    }

    #[test]
    fn holding_beats_four_three_two_one_on_ticks_30_60_90_120_and_nothing_else() {
        let mut destruct = SelfDestruct::default();
        let beats = hold(&mut destruct, 148);
        let counted: Vec<(usize, Beat)> = beats
            .iter()
            .enumerate()
            .skip(1)
            .filter_map(|(i, b)| b.map(|b| (i + 1, b)))
            .collect();
        assert_eq!(
            counted,
            vec![
                (30, Beat::Count { seconds: 4 }),
                (60, Beat::Count { seconds: 3 }),
                (90, Beat::Count { seconds: 2 }),
                (120, Beat::Count { seconds: 1 }),
            ]
        );
        assert_eq!(destruct.count(), Some(2));
    }

    #[test]
    fn holding_destructs_on_tick_149_and_goes_idle() {
        let mut destruct = SelfDestruct::default();
        let beats = hold(&mut destruct, 149);
        assert_eq!(beats[148], Some(Beat::Destruct));
        assert_eq!(destruct.count(), None);
        // A tick later the held chord starts it again, as the original's
        // idle -1.0 does.
        assert_eq!(destruct.step(true, false), Some(Beat::Started));
    }

    #[test]
    fn a_release_while_running_cancels_it_and_goes_idle() {
        let mut destruct = SelfDestruct::default();
        hold(&mut destruct, 40);
        assert_eq!(destruct.step(false, false), Some(Beat::Cancelled));
        assert_eq!(destruct.count(), None);
        assert_eq!(destruct.step(false, false), None);
    }

    #[test]
    fn a_running_count_goes_on_while_dying_and_a_dying_release_still_cancels() {
        let mut destruct = SelfDestruct::default();
        hold(&mut destruct, 10);
        assert_eq!(destruct.step(true, true), None);
        assert_eq!(destruct.count(), Some(139));
        let mut last = None;
        for _ in 0..138 {
            last = destruct.step(true, true);
        }
        assert_eq!(last, Some(Beat::Destruct));

        let mut destruct = SelfDestruct::default();
        hold(&mut destruct, 10);
        assert_eq!(destruct.step(false, true), Some(Beat::Cancelled));
        assert_eq!(destruct.count(), None);
    }

    #[test]
    fn an_idle_release_does_nothing() {
        let mut destruct = SelfDestruct::default();
        assert_eq!(destruct.step(false, false), None);
        assert_eq!(destruct.count(), None);
    }

    #[test]
    fn reset_idles_it() {
        let mut destruct = SelfDestruct::default();
        hold(&mut destruct, 50);
        destruct.reset();
        assert_eq!(destruct.count(), None);
    }

    #[test]
    fn after_the_destruct_a_held_chord_while_dying_does_not_restart_it() {
        let mut destruct = SelfDestruct::default();
        hold(&mut destruct, 149);
        for _ in 0..200 {
            assert_eq!(destruct.step(true, true), None);
        }
        assert_eq!(destruct.count(), None);
    }

    #[test]
    fn the_chord_fades_the_cursor_exactly_on_the_passes_that_start_or_destruct() {
        let mut running = SelfDestruct::default();
        hold(&mut running, 10);
        for count in [
            SelfDestruct::default(),
            running,
            SelfDestruct::at(END + 1),
            SelfDestruct::at(END + 2),
        ] {
            for dying in [false, true] {
                let beat = { count }.step(true, dying);
                assert_eq!(
                    count.chord_fades(dying),
                    matches!(beat, Some(Beat::Started | Beat::Destruct)),
                    "{count:?} dying {dying}"
                );
            }
        }
        assert!(SelfDestruct::default().chord_fades(false), "the start");
        assert!(!SelfDestruct::default().chord_fades(true), "a dying start");
        assert!(SelfDestruct::at(END + 1).chord_fades(true), "the destruct");
        assert!(!running.chord_fades(false), "running");
    }

    #[test]
    fn at_sets_the_count_a_step_reads() {
        let mut destruct = SelfDestruct::at(END + 1);
        assert_eq!(destruct.count(), Some(END + 1));
        assert_eq!(destruct.step(true, false), Some(Beat::Destruct));
    }

    #[test]
    fn the_constants_match_the_original() {
        assert_eq!((START, BEAT_EVERY, BEAT_FROM, END), (150, 30, 120, 1));
    }
}
