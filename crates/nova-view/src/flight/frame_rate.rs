//! The frame-rate readout's meter: the original's `_showFPS`, its
//! `_fpsCounter` and its `_lastFPSUpdateTime`, counted and timed as
//! `_PlayGame` does (@0x45bcd-0x45d57) and toggled as `_HandlePlayer`'s
//! frame-rate key does (@0x6e454-0x6e4bd).
//!
//! The original's loop pass is one game step and one draw; the engine
//! draws apart from its fixed steps, so [`FrameRate::frame`] counts the
//! frames drawn, which is what the readout measures. Its clock is the
//! wall time the app hands each tick ([`FrameRate::elapse`]), in Mac
//! ticks of 1/60 s, `TickCount()`'s unit. The original's 16-bit counter
//! and 32-bit tick wrap are not replicated.

use std::time::Duration;

use crate::time::sixtieths;

/// `STR#` 2002 #282: the readout's first line after it is turned on
/// (`GetIndString` @0x45c19).
pub const CALCULATING_FPS: &str = "Calculating FPS...";
/// `STR#` 2002 #283: what follows the number in the readout's line
/// (`_StrcatIndCString` @0x45ced).
pub const FPS: &str = "fps";

/// How long the readout waits between lines, in Mac ticks: a line comes
/// once `TickCount()` is past the last plus 120, 2 s (`jbe` @0x45bf6).
const PERIOD: u128 = 120;

/// How many ticks [`Readout::Calculating`] shows for (`_DisplayComm`
/// @0x45d2e, 0x64).
const CALCULATING_TICKS: u16 = 100;

/// How many ticks a [`Readout::Fps`] line shows for (0x3e7).
const FPS_TICKS: u16 = 999;

/// The frame-rate readout's state, which outlives the flights it is
/// shown over.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FrameRate {
    /// Whether the readout is shown (`_showFPS`).
    shown: bool,
    /// When the readout last said a line, or followed the clock while
    /// hidden (`_lastFPSUpdateTime`).
    last: Last,
    /// The frames drawn since (`_fpsCounter`).
    count: u32,
    /// The wall time summed from every tick.
    clock: Duration,
}

/// The original's `_lastFPSUpdateTime`, its 0 sentinel named.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Last {
    /// Just turned on or switched in (0): the next frame says
    /// [`Readout::Calculating`].
    #[default]
    Calculating,
    /// The Mac tick of the last line, or of the last frame while hidden.
    At(u128),
}

/// What a press of the frame-rate key did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Toggled {
    /// It turned the readout on: the message line is free at once
    /// (`_commCount` := 0).
    On,
    /// It turned the readout off: the message line shows an empty line
    /// for 1 tick (`_commCount` := 1).
    Off,
}

/// A line the readout says.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Readout {
    /// "Calculating FPS...", the first line after the readout is turned
    /// on or the app comes back to the front.
    Calculating,
    /// The frames drawn a second since the last line, truncated toward
    /// zero (`cvttsd2si` @0x45cac).
    Fps(u32),
}

impl Readout {
    /// The line's words: [`CALCULATING_FPS`], or the number, a space and
    /// [`FPS`] (`NumToString`, @0x45ced).
    #[must_use]
    pub fn text(self) -> String {
        match self {
            Self::Calculating => CALCULATING_FPS.to_owned(),
            Self::Fps(n) => format!("{n} {FPS}"),
        }
    }

    /// How many ticks the line shows for: 100 for
    /// [`Readout::Calculating`], 999 for a number.
    #[must_use]
    pub fn ticks(self) -> u16 {
        match self {
            Self::Calculating => CALCULATING_TICKS,
            Self::Fps(_) => FPS_TICKS,
        }
    }
}

impl FrameRate {
    /// A meter built with the readout `shown` or not, as a game entered
    /// from the main screen finds it ([`FrameRate::entered`]).
    #[must_use]
    pub fn with_shown(shown: bool) -> Self {
        let mut meter = Self {
            shown,
            ..Self::default()
        };
        meter.entered();
        meter
    }

    /// Whether the readout is shown.
    #[must_use]
    pub fn shown(&self) -> bool {
        self.shown
    }

    /// Moves the clock on by `dt`, every tick, whatever runs.
    pub fn elapse(&mut self, dt: Duration) {
        self.clock += dt;
    }

    /// The clock, in Mac ticks.
    fn now(&self) -> u128 {
        sixtieths(self.clock)
    }

    /// A press of the frame-rate key (@0x6e473-0x6e4b3): turning it on
    /// starts the count afresh, with "Calculating FPS..." next.
    pub fn toggle(&mut self) -> Toggled {
        self.shown = !self.shown;
        if self.shown {
            self.last = Last::Calculating;
            self.count = 0;
            Toggled::On
        } else {
            Toggled::Off
        }
    }

    /// One frame drawn: counts it (@0x45bd8), and says a line when the
    /// readout is shown and more than 2 s have passed since the last
    /// (@0x45bf6), starting the count afresh. While hidden, the last line
    /// follows the clock (@0x45d4c-0x45d57).
    pub fn frame(&mut self) -> Option<Readout> {
        self.count += 1;
        let now = self.now();
        if !self.shown {
            self.last = Last::At(now);
            return None;
        }
        let readout = match self.last {
            Last::Calculating => Readout::Calculating,
            Last::At(last) if now > last + PERIOD => {
                let fps = u128::from(self.count) * 60 / (now - last);
                Readout::Fps(u32::try_from(fps).unwrap_or(u32::MAX))
            }
            Last::At(_) => return None,
        };
        self.last = Last::At(now);
        self.count = 0;
        Some(readout)
    }

    /// The app came back to the front (`_SwitchedIn` @0x513cc-0x513dc):
    /// "Calculating FPS..." shows again on the next frame.
    pub fn switched_in(&mut self) {
        self.last = Last::Calculating;
        self.count = 0;
    }

    /// A game entered from the main screen (`_EnterGameFromMainScreen`
    /// @0x1a3cc-0x1a3e1): the count starts afresh from now, so a readout
    /// shown says a number after 2 s, never "Calculating FPS...".
    pub fn entered(&mut self) {
        self.count = 0;
        self.last = Last::At(self.now());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nova_sim::mission::screens::mac_ticks;

    /// Moves `meter` on by `n` Mac ticks.
    fn wait(meter: &mut FrameRate, n: u32) {
        meter.elapse(mac_ticks(n));
    }

    /// `n` frames, each `tick` Mac ticks after the last, and the last
    /// line said.
    fn frames(meter: &mut FrameRate, n: u32, tick: u32) -> Option<Readout> {
        let mut said = None;
        for _ in 0..n {
            wait(meter, tick);
            if let Some(readout) = meter.frame() {
                said = Some(readout);
            }
        }
        said
    }

    #[test]
    fn toggling_flips_the_flag_and_on_starts_the_count_afresh() {
        let mut meter = FrameRate::default();
        assert!(!meter.shown());
        wait(&mut meter, 500);
        frames(&mut meter, 40, 1);
        assert_eq!(meter.toggle(), Toggled::On);
        assert!(meter.shown());
        assert_eq!(meter.count, 0);
        assert_eq!(meter.last, Last::Calculating);
        assert_eq!(meter.toggle(), Toggled::Off);
        assert!(!meter.shown());
    }

    #[test]
    fn the_first_frame_after_turning_it_on_says_calculating() {
        let mut meter = FrameRate::default();
        meter.toggle();
        assert_eq!(meter.frame(), Some(Readout::Calculating), "even at 0");
        let mut later = FrameRate::default();
        wait(&mut later, 5000);
        later.toggle();
        assert_eq!(later.frame(), Some(Readout::Calculating));
    }

    #[test]
    fn nothing_is_said_until_more_than_120_ticks_have_passed() {
        let mut meter = FrameRate::default();
        meter.toggle();
        meter.frame();
        wait(&mut meter, 119);
        assert_eq!(meter.frame(), None);
        wait(&mut meter, 1);
        assert_eq!(meter.frame(), None, "exactly 120 says nothing");
        wait(&mut meter, 1);
        assert!(matches!(meter.frame(), Some(Readout::Fps(_))), "121 does");
    }

    #[test]
    fn the_rate_is_frames_a_second_truncated_toward_zero() {
        let mut meter = FrameRate::default();
        meter.toggle();
        meter.frame();
        frames(&mut meter, 61, 1);
        wait(&mut meter, 60);
        // 62 frames over 121 ticks: 30.74 a second.
        assert_eq!(meter.frame(), Some(Readout::Fps(30)));
        assert_eq!(meter.count, 0, "the count starts afresh");
        assert_eq!(frames(&mut meter, 121, 1), Some(Readout::Fps(60)));
    }

    #[test]
    fn frames_are_counted_while_hidden_and_the_last_follows_the_clock() {
        let mut meter = FrameRate::default();
        frames(&mut meter, 3, 10);
        assert_eq!(meter.count, 3);
        assert_eq!(meter.last, Last::At(30));
    }

    #[test]
    fn a_game_entered_says_a_number_not_calculating() {
        let mut meter = FrameRate::default();
        meter.toggle();
        wait(&mut meter, 300);
        meter.frame();
        frames(&mut meter, 9, 1);
        meter.entered();
        assert_eq!(meter.count, 0);
        assert_eq!(frames(&mut meter, 121, 1), Some(Readout::Fps(60)));
        let mut built = FrameRate::with_shown(true);
        assert!(built.shown());
        assert_eq!(frames(&mut built, 120, 1), None, "no Calculating");
        assert_eq!(frames(&mut built, 1, 1), Some(Readout::Fps(60)));
        assert!(!FrameRate::with_shown(false).shown());
    }

    #[test]
    fn switching_in_brings_calculating_back() {
        let mut meter = FrameRate::with_shown(true);
        frames(&mut meter, 10, 1);
        meter.switched_in();
        assert_eq!(meter.count, 0);
        assert_eq!(meter.frame(), Some(Readout::Calculating));
    }

    #[test]
    fn a_readout_says_its_words_for_its_ticks() {
        assert_eq!(Readout::Calculating.text(), "Calculating FPS...");
        assert_eq!(Readout::Calculating.ticks(), 100);
        assert_eq!(Readout::Fps(31).text(), "31 fps");
        assert_eq!(Readout::Fps(31).ticks(), 999);
    }
}
