//! The target a deciding NPC's fire reads: the one the frame ends with.
//! `_AIDispatch` runs the high level and then the low level in the same
//! dispatch (@0x901e5-0x901f4), so the low level's fire reads +0x70 as
//! the high level left it. A drop that clears the target is in place
//! before the trigger (the traffic's goal step writes it); one that holds
//! it ([`Decision::holds_target`](crate::ai::Decision::holds_target), the
//! ally drop @0x8e2d2) is handed to the trigger here, so sub 0's
//! `_AIEscortFireUnprovoked` (@0x8521e-0x85236) fires at it.

use crate::ai::{Behaviour, Surroundings};
use crate::combat::ShipRef;
use crate::combat::armament::Trigger;
use crate::traffic::npc::Npc;

/// `behaviour`'s trigger for `npc` among `around`, reading the target its
/// decision `held` in place of the one `npc` had.
pub(crate) fn held_trigger(
    behaviour: &(impl Behaviour + ?Sized),
    npc: &Npc,
    held: Option<ShipRef>,
    around: &Surroundings,
) -> Trigger {
    match held {
        Some(held) => {
            let framed = Npc {
                target: Some(held),
                ..npc.clone()
            };
            behaviour.trigger(&framed, around)
        }
        None => behaviour.trigger(npc, around),
    }
}
