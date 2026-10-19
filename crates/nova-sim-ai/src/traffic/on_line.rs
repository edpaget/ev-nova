//! The frame's work on the message line in slot order: each think's
//! calls, then its scan of the player, as `_AIDispatch` runs the type AI
//! (whose calls are `_AICallForHelp` @0x82ffb, `_InterceptorAI`'s
//! pre-scan hail @0x8ce4b-0x8cff5 and `_AICallForReinforcements`) before
//! `_HighLevelAIHandler` @0x901e5 (whose `_ScanPlayer` runs from
//! @0x8f11e-0x8f38a), one slot after another (`_DoPlayGameWork`
//! @0x4429d).
//!
//! A scan's line sets `_commCount` at once (`_DisplayComm` @0x9024b), so
//! a later slot's call reads it (≤ 0 free @0x8308c, < 0 idle @0x8cecd).
//! The session answers the calls ahead of the frame's first scan with the
//! traffic tick ([`LineQueue::take_ahead_of_scans`]), and settles the
//! rest, scans and calls interleaved, in the combat tick
//! ([`LineQueue::take_all`]), where the scans' law is at hand.
//!
//! Each call carries the odds against its caller as its think read them
//! ([`Npc::odds`](crate::traffic::npc::Npc::odds)): the original calls
//! inside the think, before the frame's odds pass (see
//! [`odds_cache`](crate::traffic::odds_cache)), and the engine answers
//! after it, when the pass may have refreshed the caller or marked it
//! stale.

use crate::ai::CommCall;
use crate::traffic::npc::NpcId;

/// One piece of a think's work on the message line.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum OnLine {
    /// A call the think made ([`CommCall`]), with the odds against the
    /// caller its think read.
    Call(NpcId, CommCall, f32),
    /// The NPC stopped inspecting the player within reach of it: it scans
    /// the player.
    Scan(NpcId),
}

/// The frame's calls and scans in slot order (see the module docs).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LineQueue(Vec<OnLine>);

impl LineQueue {
    /// Queues NPC `id`'s think, which read the `odds` against it: its
    /// `calls` in order, then its scan when it `scanned`.
    pub fn push_think(
        &mut self,
        id: NpcId,
        calls: impl IntoIterator<Item = CommCall>,
        odds: f32,
        scanned: bool,
    ) {
        self.0
            .extend(calls.into_iter().map(|call| OnLine::Call(id, call, odds)));
        if scanned {
            self.0.push(OnLine::Scan(id));
        }
    }

    /// The calls queued ahead of the first scan, in order, each with the
    /// odds its think read, taken; the scan and everything after it stay
    /// queued.
    pub fn take_ahead_of_scans(&mut self) -> Vec<(NpcId, CommCall, f32)> {
        let ahead = self
            .0
            .iter()
            .position(|work| matches!(work, OnLine::Scan(_)))
            .unwrap_or(self.0.len());
        self.0
            .drain(..ahead)
            .filter_map(|work| match work {
                OnLine::Call(id, call, odds) => Some((id, call, odds)),
                OnLine::Scan(_) => None,
            })
            .collect()
    }

    /// Everything queued, in order, taken.
    pub fn take_all(&mut self) -> Vec<OnLine> {
        std::mem::take(&mut self.0)
    }

    /// Whether anything is queued.
    #[must_use]
    pub fn holds_any(&self) -> bool {
        !self.0.is_empty()
    }

    /// Drops everything queued.
    pub fn clear(&mut self) {
        self.0.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: NpcId = NpcId(0);
    const B: NpcId = NpcId(1);
    const C: NpcId = NpcId(2);
    const D: NpcId = NpcId(3);
    /// The odds a think read.
    const ODDS: f32 = 0.5;

    #[test]
    fn each_call_carries_the_odds_its_think_read() {
        let mut queue = LineQueue::default();
        queue.push_think(A, [CommCall::Reinforcements], 1.5, false);
        queue.push_think(B, [CommCall::Distress], -1.0, true);
        queue.push_think(C, [CommCall::Reinforcements], 0.25, false);
        assert_eq!(
            queue.take_ahead_of_scans(),
            [
                (A, CommCall::Reinforcements, 1.5),
                (B, CommCall::Distress, -1.0)
            ]
        );
        assert_eq!(
            queue.take_all(),
            [
                OnLine::Scan(B),
                OnLine::Call(C, CommCall::Reinforcements, 0.25)
            ]
        );
    }

    #[test]
    fn a_thinks_own_calls_come_before_its_scan() {
        let mut queue = LineQueue::default();
        queue.push_think(
            A,
            [CommCall::Distress, CommCall::Reinforcements],
            ODDS,
            true,
        );
        assert_eq!(
            queue.take_all(),
            [
                OnLine::Call(A, CommCall::Distress, ODDS),
                OnLine::Call(A, CommCall::Reinforcements, ODDS),
                OnLine::Scan(A),
            ]
        );
    }

    #[test]
    fn the_calls_ahead_of_the_first_scan_are_taken_and_the_rest_kept_in_order() {
        let mut queue = LineQueue::default();
        queue.push_think(A, [CommCall::Distress], ODDS, false);
        queue.push_think(B, [], ODDS, true);
        queue.push_think(C, [CommCall::PreScan], ODDS, false);
        queue.push_think(D, [], ODDS, true);
        assert_eq!(queue.take_ahead_of_scans(), [(A, CommCall::Distress, ODDS)]);
        assert!(queue.holds_any());
        assert_eq!(
            queue.take_all(),
            [
                OnLine::Scan(B),
                OnLine::Call(C, CommCall::PreScan, ODDS),
                OnLine::Scan(D),
            ]
        );
    }

    #[test]
    fn with_no_scan_every_call_is_taken_ahead() {
        let mut queue = LineQueue::default();
        queue.push_think(A, [CommCall::Distress], ODDS, false);
        queue.push_think(B, [], ODDS, false);
        queue.push_think(
            C,
            [CommCall::PreScan, CommCall::Reinforcements],
            ODDS,
            false,
        );
        assert_eq!(
            queue.take_ahead_of_scans(),
            [
                (A, CommCall::Distress, ODDS),
                (C, CommCall::PreScan, ODDS),
                (C, CommCall::Reinforcements, ODDS),
            ]
        );
        assert!(!queue.holds_any());
    }

    #[test]
    fn a_scan_first_leaves_nothing_ahead() {
        let mut queue = LineQueue::default();
        queue.push_think(A, [], ODDS, true);
        queue.push_think(B, [CommCall::Distress], ODDS, false);
        assert_eq!(queue.take_ahead_of_scans(), []);
        assert_eq!(
            queue.take_all(),
            [OnLine::Scan(A), OnLine::Call(B, CommCall::Distress, ODDS)]
        );
    }

    #[test]
    fn taking_all_empties_it() {
        let mut queue = LineQueue::default();
        queue.push_think(A, [CommCall::Distress], ODDS, true);
        assert!(queue.holds_any());
        assert_eq!(queue.take_all().len(), 2);
        assert!(!queue.holds_any());
        assert_eq!(queue.take_all(), []);
    }

    #[test]
    fn clearing_drops_everything() {
        let mut queue = LineQueue::default();
        queue.push_think(A, [CommCall::Distress], ODDS, true);
        queue.clear();
        assert!(!queue.holds_any());
        assert_eq!(queue.take_all(), []);
    }
}
