//! The frame's work on the message line in slot order: each think's
//! calls, then its scan of the player, then what its pirate or repairer
//! did and its refuel note, one slot after another (`_DoPlayGameWork`
//! @0x44270-0x442ae walks slots 1 to 63 in ascending order, the slot
//! [`Npc::berth`](crate::traffic::npc::Npc::berth) models).
//!
//! Within a slot, `_AIDispatch` runs the type AI (whose calls are
//! `_AICallForHelp` @0x82ffb, `_InterceptorAI`'s pre-scan hail
//! @0x8ce4b-0x8cff5 and `_AICallForReinforcements`) before
//! `_HighLevelAIHandler` @0x901e5 (whose `_ScanPlayer` runs from
//! @0x8f11e-0x8f38a, and whose goal 9 says the refuel note, `_DisplayComm`
//! @0x8f6a1), then tail-jumps to `_LowLevelAIHandler` @0x901f4 (the repair
//! note @0x86ef4, the plunder @0x86f7f, whose lines are `_DisplayComm`
//! @0x84791 and @0x84941, and the dock start @0x86fc7). A think raises at
//! most one of the scan, the refuel note, the repair, the plunder and the
//! dock, as each ends the goal that leads to it.
//!
//! Each line sets `_commCount` at once (`_DisplayComm` @0x9024b), so a
//! later slot's call reads it (≤ 0 free @0x8308c, < 0 idle @0x8cecd). The
//! session settles everything queued in the pass, after the AI loop
//! ([`LineQueue::take_all`]).
//!
//! The engine's NPCs are kept in the order they came, not by slot, so the
//! queue keeps its work in order of (frame, berth), stable among equal
//! keys: a ship's own work stays in the order it was queued, and work held
//! from an earlier frame stays ahead of a later frame's.
//!
//! Each call carries the odds against its caller as its think read them
//! ([`Npc::odds`](crate::traffic::npc::Npc::odds)): the original calls
//! inside the think, before the frame's odds pass (see
//! [`odds_cache`](crate::traffic::odds_cache)), and the engine answers
//! after it, when the pass may have refreshed the caller or marked it
//! stale.

use crate::ai::CommCall;
use crate::traffic::npc::NpcId;
use crate::traffic::plunder::PirateEvent;
use nova_sim_catalog::catalog::ShipId;

/// One piece of a think's work on the message line.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum OnLine {
    /// A call the think made ([`CommCall`]), with the odds against the
    /// caller its think read.
    Call(NpcId, CommCall, f32),
    /// The NPC stopped inspecting the player within reach of it: it scans
    /// the player.
    Scan(NpcId),
    /// What a pirate or a repairer did ([`PirateEvent`]): a dock with the
    /// player, a plunder of the player or of an NPC, a repair.
    Pirate(PirateEvent),
    /// The NPC, of the ship type, finished refuelling the player: its
    /// note.
    RefuelDone(NpcId, ShipId),
}

/// The frame's work on the line in slot order (see the module docs).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LineQueue {
    /// The frame now being queued.
    frame: u64,
    /// The work, each with the frame and berth it was queued at, in order
    /// of the two.
    work: Vec<(u64, u32, OnLine)>,
}

impl LineQueue {
    /// Starts queueing the next frame's work: whatever is still held stays
    /// ahead of it.
    pub fn next_frame(&mut self) {
        self.frame += 1;
    }

    /// Queues `work` at `berth` in the frame now being queued, after
    /// everything queued at a lower berth or at the same one.
    pub fn push(&mut self, berth: u32, work: OnLine) {
        let key = (self.frame, berth);
        let at = self
            .work
            .partition_point(|&(frame, other, _)| (frame, other) <= key);
        self.work.insert(at, (self.frame, berth, work));
    }

    /// Queues the think of NPC `id` at `berth`, which read the `odds`
    /// against it: its `calls` in order, then its scan when it `scanned`.
    pub fn push_think(
        &mut self,
        berth: u32,
        id: NpcId,
        calls: impl IntoIterator<Item = CommCall>,
        odds: f32,
        scanned: bool,
    ) {
        for call in calls {
            self.push(berth, OnLine::Call(id, call, odds));
        }
        if scanned {
            self.push(berth, OnLine::Scan(id));
        }
    }

    /// Everything queued, in order, taken.
    pub fn take_all(&mut self) -> Vec<OnLine> {
        std::mem::take(&mut self.work)
            .into_iter()
            .map(|(_, _, work)| work)
            .collect()
    }

    /// The pirate events queued, in order, taken; the rest stays queued.
    #[cfg(any(test, feature = "fixture"))]
    pub fn take_pirate_events(&mut self) -> Vec<PirateEvent> {
        let mut events = Vec::new();
        self.work.retain(|&(_, _, work)| match work {
            OnLine::Pirate(event) => {
                events.push(event);
                false
            }
            _ => true,
        });
        events
    }

    /// Whether anything is queued.
    #[must_use]
    pub fn holds_any(&self) -> bool {
        !self.work.is_empty()
    }

    /// Drops everything queued.
    pub fn clear(&mut self) {
        self.work.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traffic::plunder::PirateEvent;
    use nova_sim_catalog::catalog::ShipId;

    const A: NpcId = NpcId(0);
    const B: NpcId = NpcId(1);
    const C: NpcId = NpcId(2);
    /// The odds a think read.
    const ODDS: f32 = 0.5;

    fn call(id: NpcId) -> OnLine {
        OnLine::Call(id, CommCall::Distress, ODDS)
    }

    #[test]
    fn each_call_carries_the_odds_its_think_read() {
        let mut queue = LineQueue::default();
        queue.next_frame();
        queue.push_think(1, A, [CommCall::Reinforcements], 1.5, false);
        queue.push_think(2, B, [CommCall::Distress], -1.0, true);
        assert_eq!(
            queue.take_all(),
            [
                OnLine::Call(A, CommCall::Reinforcements, 1.5),
                OnLine::Call(B, CommCall::Distress, -1.0),
                OnLine::Scan(B),
            ]
        );
    }

    #[test]
    fn a_thinks_own_calls_come_before_its_scan() {
        let mut queue = LineQueue::default();
        queue.push_think(
            1,
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
    fn work_is_kept_in_berth_order_within_a_frame() {
        let mut queue = LineQueue::default();
        queue.next_frame();
        queue.push(3, call(C));
        queue.push(1, call(A));
        queue.push(2, call(B));
        assert_eq!(queue.take_all(), [call(A), call(B), call(C)]);
    }

    #[test]
    fn one_berths_work_keeps_the_order_it_was_queued_in() {
        let mut queue = LineQueue::default();
        queue.next_frame();
        queue.push(2, OnLine::Scan(B));
        queue.push(1, call(A));
        queue.push(2, call(B));
        queue.push(2, OnLine::Scan(C));
        assert_eq!(
            queue.take_all(),
            [call(A), OnLine::Scan(B), call(B), OnLine::Scan(C)]
        );
    }

    #[test]
    fn an_earlier_frames_held_work_stays_ahead_of_a_later_frames() {
        let mut queue = LineQueue::default();
        queue.next_frame();
        queue.push(5, call(C));
        queue.next_frame();
        queue.push(1, call(A));
        queue.push(9, call(B));
        assert_eq!(queue.take_all(), [call(C), call(A), call(B)]);
    }

    #[test]
    fn a_pirate_event_and_a_refuel_note_sort_among_the_calls_by_berth() {
        let plunder = OnLine::Pirate(PirateEvent::PlunderPlayer { pirate: B });
        let refuel = OnLine::RefuelDone(C, ShipId(128));
        let mut queue = LineQueue::default();
        queue.next_frame();
        queue.push_think(4, A, [CommCall::Distress], ODDS, false);
        queue.push(1, refuel);
        queue.push_think(6, A, [CommCall::PreScan], ODDS, false);
        queue.push(5, plunder);
        assert_eq!(
            queue.take_all(),
            [
                refuel,
                call(A),
                plunder,
                OnLine::Call(A, CommCall::PreScan, ODDS),
            ]
        );
    }

    #[test]
    fn the_pirate_events_alone_are_drained_in_order() {
        let docked = PirateEvent::PlayerDockedWith { pirate: B };
        let plunder = PirateEvent::PlunderPlayer { pirate: C };
        let mut queue = LineQueue::default();
        queue.push(3, OnLine::Pirate(plunder));
        queue.push(1, call(A));
        queue.push(2, OnLine::Pirate(docked));
        assert_eq!(queue.take_pirate_events(), [docked, plunder]);
        assert_eq!(queue.take_all(), [call(A)], "the rest kept");
    }

    #[test]
    fn taking_all_empties_it() {
        let mut queue = LineQueue::default();
        queue.push_think(1, A, [CommCall::Distress], ODDS, true);
        assert!(queue.holds_any());
        assert_eq!(queue.take_all().len(), 2);
        assert!(!queue.holds_any());
        assert_eq!(queue.take_all(), []);
    }

    #[test]
    fn clearing_drops_everything() {
        let mut queue = LineQueue::default();
        queue.push_think(1, A, [CommCall::Distress], ODDS, true);
        queue.clear();
        assert!(!queue.holds_any());
        assert_eq!(queue.take_all(), []);
    }
}
