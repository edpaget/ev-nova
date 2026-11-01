//! The target a deciding NPC's fire reads: the one the frame ends with.
//! `_AIDispatch` runs the high level and then the low level in the same
//! dispatch (@0x901e5-0x901f4), so the low level's fire reads +0x70 as
//! the high level left it. A drop that clears the target is in place
//! before the trigger (the traffic's goal step writes it); one that holds
//! it ([`Decision::holds_target`](crate::ai::Decision::holds_target), the
//! ally drop @0x8e2d2) is handed to the trigger here, so sub 0's
//! `_AIEscortFireUnprovoked` (@0x8521e-0x85236) fires at it, and drops it
//! in the same dispatch when it is down (+0x70 = -1, @0x80e9d).

use crate::ai::{Behaviour, Goal, Surroundings};
use crate::combat::ShipRef;
use crate::combat::armament::Trigger;
use crate::traffic::npc::Npc;

/// What `npc`, deciding this tick, fires among `around` as `behaviour`
/// says, and the target the frame ends with, its decision's last word
/// being `last_word` ([`Decision::last_target`](crate::ai::Decision::last_target)):
/// a cleared target stands, and a held one unless, on a goal-0 frame, sub
/// 0's unprovoked fire drops it ([`gate_wait`](Behaviour::gate_wait)'s,
/// which that fire is); with neither, the target step's answer. A target
/// step that works its answer out afresh (a defender's think, a mission
/// hunt) would refill a cleared or held one.
// `Decision::last_target`'s own shape: none for no word, then the word.
#[allow(clippy::option_option)]
pub(crate) fn decided_command(
    behaviour: &(impl Behaviour + ?Sized),
    npc: &Npc,
    last_word: Option<Option<ShipRef>>,
    around: &Surroundings,
) -> (Trigger, Option<ShipRef>) {
    match last_word {
        Some(Some(held)) => {
            let framed = Npc {
                target: Some(held),
                ..npc.clone()
            };
            let dropped =
                framed.goal == Goal::Idle && behaviour.gate_wait(&framed, around).drops_target;
            (
                behaviour.trigger(&framed, around),
                (!dropped).then_some(held),
            )
        }
        Some(None) => (behaviour.trigger(npc, around), None),
        None => (
            behaviour.trigger(npc, around),
            behaviour.target(npc, around),
        ),
    }
}
