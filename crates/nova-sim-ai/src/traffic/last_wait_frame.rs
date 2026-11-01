//! The frame a ship's gate wait runs out ([`Mode::LeavingGate`] at tick
//! 0): which hits hold it there.
//!
//! **The original.** `_HandleShip` counts the wait (+0x4c) down to 0
//! (@0x3501f-0x35048) with the ship still in goal 0x15. The high level
//! turns goal 0x15 into goal 8, sub 0xa, only in a dispatch
//! (`_HighLevelAIHandler` @0x8dd61-0x8dd93), and `_HandleShip` bursts
//! only on sub 0xa's thrust word (@0x34a7b-0x34b03 → @0x34bc0,
//! @0x8811e-0x88136). `_AIMakeShipAttackPlayer` writes goal 4 whatever
//! the goal (@0x89d3e), +0x6c -1 and target 0 (@0x89d47-0x89d4d), keeping
//! sub 0 for goal 0x15 (@0x89cf2-0x89d35); `_DoGoodSamaritan` writes the
//! same goal (@0x82b82-0x82b97). So, for a ship whose wait has run out:
//!
//! - a hit before the pass's dispatch (the collision passes, the player's
//!   display step) writes goal 4 over goal 0x15: it never bursts, and
//!   flies its goal from that dispatch, still owing its burst word
//!   ([`Npc::hold_at_gate`], [`World::held_burst`]);
//! - a hit after the dispatch (`_HandleShot`, the ship loop, the beams)
//!   finds goal 8, which the next dispatch's glide path puts back over
//!   the goal 4 (@0x8fcf5-0x8fd18): it bursts on;
//! - a ship whose wait runs out in the ship loop, after the dispatch, is
//!   in goal 0x15 until the next one, so a later hit in the pass holds it;
//! - double time's second pass runs no dispatch (@0x4402a), so the ship
//!   stays in goal 0x15 through it: a hit anywhere in it holds the ship,
//!   and with none its burst waits for the next advance pass
//!   ([`autopilot`](super::autopilot)'s second-pass wait).
//!
//! **Here.** The think half notes each ship at that frame as its
//! dispatch turns it out ([`Traffic::turn_out_of_waits`]), and the answer
//! to a strike holds a ship at that frame that is not among them
//! ([`in_turn`](super::in_turn)). The fly half's end forgets them.
//!
//! The first tick of a ship so held drifts (the one-tick
//! [`Mode::HeldAtGate`] of [`Npc::leaves_gate_goal`]) where the original
//! steers from that dispatch: the approximation the disabled reset and
//! the succession share, noted there.
//!
//! Nothing here plays a sound or draws a random number.
//!
//! [`Npc::hold_at_gate`]: super::npc::Npc::hold_at_gate
//! [`Npc::leaves_gate_goal`]: super::npc::Npc::leaves_gate_goal
//! [`World::held_burst`]: super::World::held_burst

use std::collections::BTreeSet;

use super::Traffic;
use super::npc::{Mode, NpcId};

/// The ships whose gate wait had run out as this pass's dispatch reached
/// them, which turned their goal 0x15 into goal 8 (see the module docs).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct TurnedOut(BTreeSet<NpcId>);

impl Traffic {
    /// Notes each ship whose wait at its gate has run out as this pass's
    /// dispatch turns it out of goal 0x15 (@0x8dd61-0x8dd93).
    pub(super) fn turn_out_of_waits(&mut self) {
        self.turned_out = TurnedOut(
            self.npcs
                .iter()
                .filter(|npc| npc.mode == Mode::LeavingGate { tick: 0 })
                .map(|npc| npc.id)
                .collect(),
        );
    }

    /// Whether this pass's dispatch turned NPC `id` out of its gate wait
    /// ([`Traffic::turn_out_of_waits`]).
    pub(super) fn turned_out(&self, id: NpcId) -> bool {
        self.turned_out.0.contains(&id)
    }

    /// Forgets the ships the pass's dispatch turned out, as its fly half
    /// ends.
    pub(super) fn forget_turned_out(&mut self) {
        self.turned_out = TurnedOut::default();
    }
}
