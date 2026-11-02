//! What the low level of a ship whose high level is skipped reads of a
//! gone ship, in the `EV Nova` executable: `_LowLevelAIHandler` reads its
//! target (+0x70) with no +0xb8 test (@0x851da-0x88726), and only the
//! high level's slot test (@0x8d4e4-0x8d50d) clears it before the low
//! level runs. With the AI wait running (@0x8d4b9-0x8d4d2) or in the
//! regroup pause (`_AIDispatch` @0x9002d → @0x900c6 → @0x901ea) no high
//! level runs, so the low level steers at the freed slot's record until
//! `_HandleShip`'s let-go clears +0x70 (@0x33865-0x33888).
//!
//! [`SkippedReads`] notes, at each such ship's think, the record of the
//! gone ship it still holds ([`GoneShip::held_by`]); the fly half reads
//! it where the ship's flight would read the gone ship
//! ([`fly_pass`](super::fly_pass)). The note is taken at the think, so
//! the let-go before the flight in the ship loop
//! (`nova_sim::Session::ship_loop`) doesn't clear it, as the original's
//! low level ran before that `_HandleShip`.
//!
//! What it changes on screen (see the phase's step-1 findings):
//!
//! - **The regroup pause** has no AI wait, but none of the substates that
//!   reach it reads +0x70 ([`regroup`](crate::ai::regroup): sub 0's coast
//!   and the jump path's subs 4 and 0xd), so the note changes nothing
//!   drawn there.
//! - **The AI wait** holds the turn, thrust and fire (`_HandleShip`
//!   @0x3443f-0x34454, @0x34a54-0x34a72; the autopilot's position step),
//!   and the next unwaited frame's low level rewrites +0x68 and +0x30
//!   before they are used, so the note changes nothing drawn there. A
//!   docking's velocity, which its sub writes directly, reads its nav,
//!   not +0x70 (see [`repair_dock`](super::repair_dock)).
//!
//! Nothing here plays a sound or draws a random number.

use std::collections::BTreeMap;

use super::gone::GoneShip;
use super::npc::{Npc, NpcId};
use super::regroup;
use crate::flight::ShipState;

/// Whether `npc`'s high level is skipped this think: its AI wait runs
/// ([`Npc::high_level_waits`]) or it is in the regroup pause.
#[must_use]
pub fn skips(npc: &Npc) -> bool {
    npc.high_level_waits() || regroup::paused(npc)
}

/// The gone ship each ship whose high level was skipped held at its think
/// this tick, by its record (see the module docs).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SkippedReads(BTreeMap<NpcId, (NpcId, ShipState)>);

impl SkippedReads {
    /// Notes, for each of `npcs` that `picked` picks (one flag an NPC)
    /// and whose high level is skipped ([`skips`]), the record among `gone`
    /// that it holds, if any.
    pub fn note(&mut self, npcs: &[Npc], picked: &[bool], gone: &[GoneShip]) {
        for (npc, _) in npcs.iter().zip(picked).filter(|(npc, p)| **p && skips(npc)) {
            if let Some(held) = gone.iter().find(|gone| gone.held_by(npc)) {
                self.0.insert(npc.id, (held.ship.id, held.ship.state));
            }
        }
    }

    /// Where `reader`'s low level reads gone ship `ship`, by the record it
    /// held at its think; none for a ship it didn't hold.
    #[must_use]
    pub fn state_of(&self, reader: NpcId, ship: NpcId) -> Option<ShipState> {
        self.0
            .get(&reader)
            .filter(|(held, _)| *held == ship)
            .map(|&(_, state)| state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::Goal;
    use crate::ai::regroup::Kept;
    use crate::combat::ShipRef;
    use crate::geometry::Vec2;
    use crate::stats::ShipStats;
    use crate::traffic::fixture::npc;
    use crate::traffic::gone::Freed;

    /// A ship in slot `berth` targeting NPC 7, its goal `goal` and its AI
    /// wait `ai_wait`.
    fn holder(id: u32, berth: u32, goal: Goal, ai_wait: u32) -> Npc {
        Npc {
            berth,
            goal,
            ai_wait: crate::traffic::ai_wait::ticks(ai_wait),
            target: Some(ShipRef::Npc(NpcId(7))),
            ..npc(id, ShipStats::default())
        }
    }

    /// NPC 7's record, slot 5, freed in the ship pass at (300, 400).
    fn gone() -> GoneShip {
        let mut ship = npc(7, ShipStats::default());
        ship.berth = 5;
        ship.state.position = Vec2::new(300.0, 400.0);
        GoneShip {
            ship,
            freed: Freed::InShipPass,
        }
    }

    #[test]
    fn a_skipped_high_level_is_an_ai_wait_or_the_regroup_pause() {
        let attack = Goal::Attack(ShipRef::Npc(NpcId(7)));
        assert!(skips(&holder(1, 1, attack, 5)), "the AI wait");
        assert!(skips(&holder(1, 1, Goal::Regroup { kept: Kept::Coast }, 0)));
        assert!(!skips(&holder(1, 1, attack, 0)));
        assert!(!skips(&holder(1, 1, Goal::Drift, 5)), "goal 0xe runs it");
    }

    #[test]
    fn a_ship_whose_high_level_is_skipped_reads_the_record_it_holds() {
        let attack = Goal::Attack(ShipRef::Npc(NpcId(7)));
        let paused = Goal::Regroup { kept: Kept::Coast };
        let npcs = [
            holder(1, 1, attack, 5),
            holder(2, 2, paused, 0),
            holder(3, 3, attack, 0),
            holder(4, 6, paused, 0),
            holder(5, 4, paused, 0),
        ];
        let mut reads = SkippedReads::default();
        reads.note(&npcs, &[true, true, true, true, false], &[gone()]);
        let at = Some(Vec2::new(300.0, 400.0));
        let read = |reader| reads.state_of(NpcId(reader), NpcId(7)).map(|s| s.position);
        assert_eq!(read(1), at, "waiting");
        assert_eq!(read(2), at, "paused");
        assert_eq!(read(3), None, "its high level runs");
        assert_eq!(read(4), None, "above the slot: let go");
        assert_eq!(read(5), None, "not picked");
        assert_eq!(reads.state_of(NpcId(1), NpcId(8)), None, "another ship");
    }
}
