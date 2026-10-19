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

use crate::ai::CommCall;
use crate::traffic::npc::NpcId;

/// One piece of a think's work on the message line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OnLine {
    /// A call the think made ([`CommCall`]).
    Call(NpcId, CommCall),
    /// The NPC stopped inspecting the player within reach of it: it scans
    /// the player.
    Scan(NpcId),
}

/// The frame's calls and scans in slot order (see the module docs).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LineQueue(Vec<OnLine>);

impl LineQueue {
    /// Queues NPC `id`'s think: its `calls` in order, then its scan when
    /// it `scanned`.
    pub fn push_think(
        &mut self,
        id: NpcId,
        calls: impl IntoIterator<Item = CommCall>,
        scanned: bool,
    ) {
        self.0
            .extend(calls.into_iter().map(|call| OnLine::Call(id, call)));
        if scanned {
            self.0.push(OnLine::Scan(id));
        }
    }

    /// The calls queued ahead of the first scan, in order, taken; the scan
    /// and everything after it stay queued.
    pub fn take_ahead_of_scans(&mut self) -> Vec<(NpcId, CommCall)> {
        let ahead = self
            .0
            .iter()
            .position(|work| matches!(work, OnLine::Scan(_)))
            .unwrap_or(self.0.len());
        self.0
            .drain(..ahead)
            .filter_map(|work| match work {
                OnLine::Call(id, call) => Some((id, call)),
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

    #[test]
    fn a_thinks_own_calls_come_before_its_scan() {
        let mut queue = LineQueue::default();
        queue.push_think(A, [CommCall::Distress, CommCall::Reinforcements], true);
        assert_eq!(
            queue.take_all(),
            [
                OnLine::Call(A, CommCall::Distress),
                OnLine::Call(A, CommCall::Reinforcements),
                OnLine::Scan(A),
            ]
        );
    }

    #[test]
    fn the_calls_ahead_of_the_first_scan_are_taken_and_the_rest_kept_in_order() {
        let mut queue = LineQueue::default();
        queue.push_think(A, [CommCall::Distress], false);
        queue.push_think(B, [], true);
        queue.push_think(C, [CommCall::PreScan], false);
        queue.push_think(D, [], true);
        assert_eq!(queue.take_ahead_of_scans(), [(A, CommCall::Distress)]);
        assert!(queue.holds_any());
        assert_eq!(
            queue.take_all(),
            [
                OnLine::Scan(B),
                OnLine::Call(C, CommCall::PreScan),
                OnLine::Scan(D),
            ]
        );
    }

    #[test]
    fn with_no_scan_every_call_is_taken_ahead() {
        let mut queue = LineQueue::default();
        queue.push_think(A, [CommCall::Distress], false);
        queue.push_think(B, [], false);
        queue.push_think(C, [CommCall::PreScan, CommCall::Reinforcements], false);
        assert_eq!(
            queue.take_ahead_of_scans(),
            [
                (A, CommCall::Distress),
                (C, CommCall::PreScan),
                (C, CommCall::Reinforcements),
            ]
        );
        assert!(!queue.holds_any());
    }

    #[test]
    fn a_scan_first_leaves_nothing_ahead() {
        let mut queue = LineQueue::default();
        queue.push_think(A, [], true);
        queue.push_think(B, [CommCall::Distress], false);
        assert_eq!(queue.take_ahead_of_scans(), []);
        assert_eq!(
            queue.take_all(),
            [OnLine::Scan(A), OnLine::Call(B, CommCall::Distress)]
        );
    }

    #[test]
    fn taking_all_empties_it() {
        let mut queue = LineQueue::default();
        queue.push_think(A, [CommCall::Distress], true);
        assert!(queue.holds_any());
        assert_eq!(queue.take_all().len(), 2);
        assert!(!queue.holds_any());
        assert_eq!(queue.take_all(), []);
    }

    #[test]
    fn clearing_drops_everything() {
        let mut queue = LineQueue::default();
        queue.push_think(A, [CommCall::Distress], true);
        queue.clear();
        assert!(!queue.holds_any());
        assert_eq!(queue.take_all(), []);
    }
}
