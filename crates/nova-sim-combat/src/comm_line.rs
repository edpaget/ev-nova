//! The message line's countdown, `_commCount` (0x183c0c in the `EV Nova`
//! executable), as one [`CommLine`]: what the original's callers read to
//! know whether a message is showing.
//!
//! - **Set.** `_DisplayComm` (@0x90201) sets it to the ticks its caller
//!   passes (@0x9024b, [`CommLine::show`]); the ticks of each
//!   [`SimMessage`](crate::message::SimMessage) are
//!   [`SimMessage::ticks`](crate::message::SimMessage::ticks).
//! - **Counted down.** `_HandleComm` (@0x301ce-0x301e1), once a frame on
//!   the advance pass only (`_DoPlayGameWork` @0x4421e), decrements it
//!   while it is 0 or more, and paints the box empty at 0 or less
//!   ([`CommLine::tick`]). A line of N ticks counts N … 1, then 0 for one
//!   frame, then -1, where it stays.
//! - **Read.** The original compares it two ways:
//!   [`CommLine::free_for_calls`] (≤ 0) and [`CommLine::idle`] (< 0).
//! - **Cleared.** Return (`ackComm`) re-shows an empty line for 1 tick
//!   while the count is above 0 (`_HandlePlayer` @0x6dd22-0x6dd65,
//!   [`CommLine::acknowledge`]); the by-name clears re-show one for 1 tick
//!   whatever the count (@0x6b231-0x6b30d, @0x6dedb-0x6df29), which is
//!   [`CommLine::show`] of 1. `_ResetPlayer` (@0x1d904) and a landing
//!   (`_HandlePlayerDockRequest` @0x66d7f) set it to 0
//!   ([`CommLine::reset`]), where a new line starts too
//!   ([`CommLine::default`]). No jump, gate or arrival writes it.

/// The message line's countdown, `_commCount` (see the module docs).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CommLine {
    /// The count: the ticks left, 0 for the frame the box is painted
    /// empty, -1 once it is.
    count: i32,
}

impl CommLine {
    /// A message shows for `ticks`, as `_DisplayComm` sets the count
    /// (@0x9024b).
    pub fn show(&mut self, ticks: u16) {
        self.count = i32::from(ticks);
    }

    /// A frame's countdown, as `_HandleComm` has it (@0x301ce-0x301e1):
    /// one less while the count is 0 or more.
    pub fn tick(&mut self) {
        if self.count >= 0 {
            self.count -= 1;
        }
    }

    /// Return (`ackComm`): an empty line for 1 tick while a message
    /// shows, as `_HandlePlayer` @0x6dd22-0x6dd65 re-shows one only while
    /// the count is above 0.
    pub fn acknowledge(&mut self) {
        if self.count > 0 {
            self.count = 1;
        }
    }

    /// The count back at 0, as `_ResetPlayer` (@0x1d904) and a landing
    /// (`_HandlePlayerDockRequest` @0x66d7f) set it.
    pub fn reset(&mut self) {
        self.count = 0;
    }

    /// Whether the line is free for a call: the count is 0 or less, as
    /// `_AICallForHelp` (`jg` @0x8308c) and `_HandleShip`'s random
    /// hail-quote roll (@0x33f42-0x33f4b) read it.
    #[must_use]
    pub fn free_for_calls(self) -> bool {
        self.count <= 0
    }

    /// Whether the line is idle: the count is below 0, its box painted
    /// empty a frame ago or more, as `_InterceptorAI`'s pre-scan hail
    /// reads it (`jns` @0x8cecd).
    #[must_use]
    pub fn idle(self) -> bool {
        self.count < 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A line shown for `ticks`, then ticked `n` times.
    fn after(ticks: u16, n: usize) -> CommLine {
        let mut line = CommLine::default();
        line.show(ticks);
        for _ in 0..n {
            line.tick();
        }
        line
    }

    #[test]
    fn a_line_counts_down_to_0_then_to_minus_1_and_stays() {
        assert_eq!(after(3, 2), CommLine { count: 1 });
        assert_eq!(after(3, 3), CommLine { count: 0 });
        assert_eq!(after(3, 4), CommLine { count: -1 });
        assert_eq!(after(3, 9), CommLine { count: -1 }, "stays");
    }

    #[test]
    fn free_for_calls_is_0_or_less_and_idle_below_0() {
        let reads = |line: CommLine| (line.free_for_calls(), line.idle());
        assert_eq!(reads(after(3, 2)), (false, false), "showing");
        assert_eq!(reads(after(3, 3)), (true, false), "at 0");
        assert_eq!(reads(after(3, 4)), (true, true), "at -1");
    }

    #[test]
    fn a_new_line_starts_at_0() {
        assert_eq!(CommLine::default(), CommLine { count: 0 });
        let mut line = after(240, 0);
        line.reset();
        assert_eq!(line, CommLine { count: 0 });
    }

    #[test]
    fn return_shows_an_empty_line_for_a_tick_only_while_one_shows() {
        let mut shown = after(240, 10);
        shown.acknowledge();
        assert_eq!(shown, CommLine { count: 1 });
        for (ticks, n) in [(3, 3), (3, 4)] {
            let mut gone = after(ticks, n);
            let before = gone;
            gone.acknowledge();
            assert_eq!(gone, before, "count {before:?}");
        }
    }
}
