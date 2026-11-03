//! The player's self-destruct countdown, `_HandlePlayer`'s block Z
//! (@0x6fdb4-0x702bf), as a [`SelfDestruct`] stepped once a pass with
//! whether the chord (Command and the `-` key) is held, at the frame's
//! gain.
//!
//! The count is `_destructCount` (a float at *0x3b53a8): -1.0 while idle,
//! here `None`. It is never saved; `_ResetPlayer` (@0x1d764) idles it, so
//! a new pilot, a death's end, the escape pod's rescue and a pilot open do
//! ([`SelfDestruct::reset`]). It drops by the frame's gain, `_gSpeedMult`
//! (a [`SpeedMult`]), in doubles and narrowed (@0x6ff1d-0x6ff2e), so at a
//! full gain it counts whole frames and at a lower one it sits on each
//! whole number for longer.
//!
//! One step reads, in the original's order:
//!
//! - **The start** (@0x6fde1-0x6febe): the chord held, the count below 0
//!   (idle), and the player not dying (`_IsDying`, -0x2ca, read at the top
//!   of `_HandlePlayer`). It beats [`Beat::Started`] and sets the count to
//!   [`START`]. A dying player's start is silent: nothing is played or
//!   said.
//! - **Running** (count above 0, @0x6fecc): not gated on dying, so a
//!   countdown goes on while the ship breaks up. The start's own pass falls
//!   through to it, so at a full gain that pass already leaves 149.
//!   - Held (@0x6ff0c-0x700b0): the count drops by the gain, and a count
//!     whose whole number (`cvttss2si` @0x6ff32) is a multiple of
//!     [`BEAT_EVERY`] and that is at most [`BEAT_FROM`] (a float compare,
//!     @0x6ff64) beats [`Beat::Count`] with its seconds left, the whole
//!     number of the count over 30.0 (`divss` @0x6ffde, `cvttss2si`
//!     @0x6ffe6). The test reads the count each pass, so a count that
//!     sits on 90.x, 60.x, 30.x or 0.x for several passes (a gain below
//!     one) beats on each; 120 beats once, as 120.x is above 120.0.
//!   - Released (@0x700b5-0x70174): [`Beat::Cancelled`], and the count
//!     idles.
//! - **The end** (@0x7018f-0x702b9): a count that was running, now at most
//!   [`END`] with the chord still held, beats [`Beat::Destruct`] and idles.
//!   With the chord held throughout at a full gain, that is the 149th pass
//!   counting the press. A count the gain left between none and one
//!   (above, say, after a ramp) has beaten a count of 0 seconds on the
//!   same pass, as the original plays and says both.
//!
//! What each beat plays, says and does is the session's (see
//! `nova_sim::Session::hold_self_destruct`).
//!
//! ## Left out
//!
//! - The no-decrement branch for a count below 1.0 (@0x6ff18-0x6ff1b): no
//!   running count enters a pass below 1.0. The start sets 150.0
//!   (@0x6febe), and the end test (1.0 ≥ count, @0x7018f-0x70197) sets
//!   every running count it finds at or below 1.0 to -1.0 (@0x702b9) on
//!   the pass that leaves it there, whether the chord is held or not.
//! - The `(short)` of the modulo test (@0x6ff36): an integer width.

use crate::speed_mult::SpeedMult;

/// The count a start sets (0x43160000, @0x6febe).
pub const START: f32 = 150.0;

/// The count beats every this many frames (the divide by 30 @0x6ff3b).
pub const BEAT_EVERY: i32 = 30;

/// What a beat's count is divided by for its seconds (30.0, 0xdd084
/// @0x6ffde).
const SECONDS: f32 = 30.0;

/// The highest count that beats (120.0, 0xdd970 @0x6ff64).
pub const BEAT_FROM: f32 = 120.0;

/// The count at or below which a held countdown ends (1.0, 0xdd068
/// @0x7017a).
pub const END: f32 = 1.0;

/// What one step of the countdown did, for the session to play and say.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Beat {
    /// The countdown began (`snd ` 153, "Self-destruct sequence
    /// initiated.").
    Started,
    /// A countdown line, `seconds` left (`snd ` 152, "Self-destruct in N
    /// seconds.").
    Count {
        /// The seconds left, the whole number of `count / 30`: 4, 3, 2,
        /// 1, and 0 on a last pass the gain left below one.
        seconds: u32,
    },
    /// The chord was let go while counting (`snd ` 151, "Self-destruct
    /// sequence cancelled.").
    Cancelled,
    /// The countdown ran out with the chord held: the player and the ships
    /// docked with it are destroyed (`snd ` 151, "Have a nice day.").
    Destruct,
}

/// `_destructCount`: idle, or the frames left (see the module docs).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct SelfDestruct {
    count: Option<f32>,
}

impl SelfDestruct {
    /// The frames left, or `None` while idle.
    #[must_use]
    pub fn count(&self) -> Option<f32> {
        self.count
    }

    /// A countdown with `count` frames left, for tests that start from a
    /// pass of it.
    #[cfg(any(test, feature = "fixture"))]
    #[must_use]
    pub fn at(count: f32) -> Self {
        Self { count: Some(count) }
    }

    /// `_ResetPlayer`'s part (@0x1d764): the count idles.
    pub fn reset(&mut self) {
        self.count = None;
    }

    /// Whether a pass at `gain` with the chord held sets
    /// `_keyPressedThisFrame`: the start (@0x6fe02) and the destruct
    /// (@0x701ca), and none of the passes between them. `dying` is as
    /// [`SelfDestruct::step`] reads it.
    #[must_use]
    pub fn chord_fades(&self, dying: bool, gain: SpeedMult) -> bool {
        match self.count {
            None => !dying,
            Some(count) => dropped(count, gain) <= END,
        }
    }

    /// One pass of block Z at the frame's `gain`, with the chord `held`
    /// and the player `dying` as of the top of `_HandlePlayer` (see the
    /// module docs): the beats it made, in order.
    pub fn step(&mut self, held: bool, dying: bool, gain: SpeedMult) -> Vec<Beat> {
        let mut beats = Vec::new();
        if held && self.count.is_none() && !dying {
            beats.push(Beat::Started);
            self.count = Some(START);
        }
        let Some(count) = self.count else {
            return beats;
        };
        if !held {
            self.count = None;
            beats.push(Beat::Cancelled);
            return beats;
        }
        let count = dropped(count, gain);
        self.count = Some(count);
        if let Some(seconds) = beat_at(count) {
            beats.push(Beat::Count { seconds });
        }
        if count <= END {
            self.count = None;
            beats.push(Beat::Destruct);
        }
        beats
    }
}

/// A running `count` after a held pass at `gain`: down by the gain, in
/// doubles and narrowed (@0x6ff1d-0x6ff2e).
fn dropped(count: f32, gain: SpeedMult) -> f32 {
    gain.add_times(count, -1.0)
}

/// The seconds `count` beats, if it beats (@0x6ff32-0x6ffe6): its whole
/// number a multiple of [`BEAT_EVERY`] and the count at most
/// [`BEAT_FROM`].
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn beat_at(count: f32) -> Option<u32> {
    let whole = count as i32;
    (whole % BEAT_EVERY == 0 && count <= BEAT_FROM).then(|| (count / SECONDS) as u32)
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    /// A full frame's gain, as every pass of the integer days ran at.
    const G: SpeedMult = SpeedMult::FULL;

    /// The one beat a pass made, if any.
    fn one(beats: &[Beat]) -> Option<Beat> {
        assert!(beats.len() <= 1, "{beats:?}");
        beats.first().copied()
    }

    /// Steps `ticks` times with the chord held and the player well,
    /// returning each tick's beat, tick 1 first.
    fn hold(destruct: &mut SelfDestruct, ticks: u32) -> Vec<Option<Beat>> {
        (0..ticks)
            .map(|_| one(&destruct.step(true, false, G)))
            .collect()
    }

    #[test]
    fn an_idle_hold_starts_it_and_the_same_tick_leaves_149() {
        let mut destruct = SelfDestruct::default();
        assert_eq!(one(&destruct.step(true, false, G)), Some(Beat::Started));
        assert_eq!(destruct.count(), Some(149.0));
    }

    #[test]
    fn a_dying_hold_starts_nothing_and_beats_nothing() {
        let mut destruct = SelfDestruct::default();
        for _ in 0..200 {
            assert_eq!(one(&destruct.step(true, true, G)), None);
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
        assert_eq!(destruct.count(), Some(2.0));
    }

    #[test]
    fn holding_destructs_on_tick_149_and_goes_idle() {
        let mut destruct = SelfDestruct::default();
        let beats = hold(&mut destruct, 149);
        assert_eq!(beats[148], Some(Beat::Destruct));
        assert_eq!(destruct.count(), None);
        // A tick later the held chord starts it again, as the original's
        // idle -1.0 does.
        assert_eq!(one(&destruct.step(true, false, G)), Some(Beat::Started));
    }

    #[test]
    fn a_release_while_running_cancels_it_and_goes_idle() {
        let mut destruct = SelfDestruct::default();
        hold(&mut destruct, 40);
        assert_eq!(one(&destruct.step(false, false, G)), Some(Beat::Cancelled));
        assert_eq!(destruct.count(), None);
        assert_eq!(one(&destruct.step(false, false, G)), None);
    }

    #[test]
    fn a_running_count_goes_on_while_dying_and_a_dying_release_still_cancels() {
        let mut destruct = SelfDestruct::default();
        hold(&mut destruct, 10);
        assert_eq!(one(&destruct.step(true, true, G)), None);
        assert_eq!(destruct.count(), Some(139.0));
        let mut last = None;
        for _ in 0..138 {
            last = one(&destruct.step(true, true, G));
        }
        assert_eq!(last, Some(Beat::Destruct));

        let mut destruct = SelfDestruct::default();
        hold(&mut destruct, 10);
        assert_eq!(one(&destruct.step(false, true, G)), Some(Beat::Cancelled));
        assert_eq!(destruct.count(), None);
    }

    #[test]
    fn an_idle_release_does_nothing() {
        let mut destruct = SelfDestruct::default();
        assert_eq!(one(&destruct.step(false, false, G)), None);
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
            assert_eq!(one(&destruct.step(true, true, G)), None);
        }
        assert_eq!(destruct.count(), None);
    }

    #[test]
    fn the_chord_fades_the_cursor_exactly_on_the_passes_that_start_or_destruct() {
        let mut running = SelfDestruct::default();
        hold(&mut running, 10);
        for gain in [
            G,
            SpeedMult::new(0.25),
            SpeedMult::new(0.6),
            SpeedMult::ZERO,
        ] {
            for count in [
                SelfDestruct::default(),
                running,
                SelfDestruct::at(END + 1.0),
                SelfDestruct::at(END + 2.0),
                SelfDestruct::at(END + 0.5),
                SelfDestruct::at(END + 0.25),
            ] {
                for dying in [false, true] {
                    let beats = { count }.step(true, dying, gain);
                    assert_eq!(
                        count.chord_fades(dying, gain),
                        beats
                            .iter()
                            .any(|beat| matches!(beat, Beat::Started | Beat::Destruct)),
                        "{count:?} dying {dying} at {gain:?}"
                    );
                }
            }
        }
        assert!(SelfDestruct::default().chord_fades(false, G), "the start");
        assert!(
            !SelfDestruct::default().chord_fades(true, G),
            "a dying start"
        );
        assert!(
            SelfDestruct::at(END + 1.0).chord_fades(true, G),
            "the destruct"
        );
        assert!(!running.chord_fades(false, G), "running");
        let half = SpeedMult::new(0.5);
        assert!(!SelfDestruct::at(END + 1.0).chord_fades(false, half));
        assert!(SelfDestruct::at(END + 0.5).chord_fades(false, half));
    }

    #[test]
    fn at_sets_the_count_a_step_reads() {
        let mut destruct = SelfDestruct::at(END + 1.0);
        assert_eq!(destruct.count(), Some(END + 1.0));
        assert_eq!(one(&destruct.step(true, false, G)), Some(Beat::Destruct));
    }

    #[test]
    fn the_constants_match_the_original() {
        assert_eq!((START, BEAT_EVERY, BEAT_FROM, END), (150.0, 30, 120.0, 1.0));
        assert_eq!(SECONDS, 30.0);
    }

    // The frame's gain.

    /// The beats of `passes` held passes at `gain` from `count`, each with
    /// the count it left.
    fn held_at(count: f32, gain: SpeedMult, passes: usize) -> Vec<(Option<f32>, Vec<Beat>)> {
        let mut destruct = SelfDestruct::at(count);
        (0..passes)
            .map(|_| {
                let beats = destruct.step(true, false, gain);
                (destruct.count(), beats)
            })
            .collect()
    }

    #[test]
    fn at_a_quarter_gain_the_count_drops_a_quarter_a_pass_and_each_pass_on_a_beat_beats() {
        let quarter = SpeedMult::new(0.25);
        let passes = held_at(91.0, quarter, 5);
        let counts: Vec<Option<f32>> = passes.iter().map(|(count, _)| *count).collect();
        assert_eq!(
            counts,
            [
                Some(90.75),
                Some(90.5),
                Some(90.25),
                Some(90.0),
                Some(89.75)
            ]
        );
        let beats: Vec<Vec<Beat>> = passes.into_iter().map(|(_, beats)| beats).collect();
        let three = vec![Beat::Count { seconds: 3 }];
        assert_eq!(
            beats,
            [three.clone(), three.clone(), three.clone(), three, vec![]]
        );
        for (from, seconds) in [(61.0, 2), (31.0, 1)] {
            let beats: Vec<Vec<Beat>> = held_at(from, quarter, 4)
                .into_iter()
                .map(|(_, beats)| beats)
                .collect();
            assert_eq!(beats, vec![vec![Beat::Count { seconds }]; 4], "from {from}");
        }
    }

    #[test]
    fn at_a_quarter_gain_120_beats_once_as_120_and_a_bit_is_past_it() {
        let beats: Vec<Vec<Beat>> = held_at(121.0, SpeedMult::new(0.25), 5)
            .into_iter()
            .map(|(_, beats)| beats)
            .collect();
        assert_eq!(
            beats,
            [
                vec![],
                vec![],
                vec![],
                vec![Beat::Count { seconds: 4 }],
                vec![]
            ]
        );
    }

    #[test]
    fn at_no_gain_a_count_on_a_beat_beats_again_and_does_not_move() {
        let passes = held_at(60.0, SpeedMult::ZERO, 3);
        for (count, beats) in passes {
            assert_eq!(count, Some(60.0));
            assert_eq!(beats, [Beat::Count { seconds: 2 }]);
        }
        let passes = held_at(59.0, SpeedMult::ZERO, 2);
        assert!(passes.iter().all(|(_, beats)| beats.is_empty()));
    }

    #[test]
    fn a_last_pass_the_gain_leaves_below_one_beats_none_seconds_then_destructs() {
        let passes = held_at(1.5, SpeedMult::new(0.75), 1);
        assert_eq!(
            passes,
            [(None, vec![Beat::Count { seconds: 0 }, Beat::Destruct])]
        );
        // On one exactly it only destructs.
        let passes = held_at(1.5, SpeedMult::new(0.5), 1);
        assert_eq!(passes, [(None, vec![Beat::Destruct])]);
    }

    #[test]
    fn the_count_drops_in_doubles() {
        let gain = SpeedMult::new(0.1);
        let passes = held_at(100.0, gain, 1);
        assert_eq!(passes[0].0, Some((100.0 - 0.1_f64) as f32));
    }

    #[test]
    fn at_the_settled_gain_the_count_runs_as_at_a_full_one() {
        let settled: Vec<Vec<Beat>> = held_at(149.0, SpeedMult::SETTLED, 148)
            .into_iter()
            .map(|(_, beats)| beats)
            .collect();
        let full: Vec<Vec<Beat>> = held_at(149.0, G, 148)
            .into_iter()
            .map(|(_, beats)| beats)
            .collect();
        assert_eq!(settled, full);
    }
}
