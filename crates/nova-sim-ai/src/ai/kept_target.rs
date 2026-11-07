//! The target (+0x70) a ship keeps through a goal whose think and high
//! level write none, until `_HighLevelAIHandler`'s slot test clears a
//! gone slot (@0x8d4e4-0x8d50d in the `EV Nova` executable): a ship
//! breaking up is still active, so it is kept.
//!
//! The original's +0x70 is a slot index, so a slot that refills names the
//! newcomer; the engine keeps ids, which can't replicate that. Under the
//! wait exit (@0x8d4b9-0x8d4d2) the slot test is skipped, and a gone
//! slot's +0x70 stays; this lets go of it at once, for the same reason.
//!
//! Its readers are the target steps of the types whose thinks keep it:
//! the traders' ([`trader_target`](super::trader_target)) and the miner's
//! ([`miner`](super::miner)).

use crate::ai::Surroundings;
use crate::combat::ShipRef;
use crate::traffic::npc::Npc;

/// The target `npc` holds while its slot is active among `around`
/// ([`Surroundings::active`]); none once the ship is gone or its slot
/// inactive (see the module docs).
#[must_use]
pub fn kept(npc: &Npc, around: &Surroundings) -> Option<ShipRef> {
    npc.target.filter(|&ship| around.active(ship))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::fixture::{PIRATES, TRADERS, around, govts, n, ship, sites};
    use crate::combat::hull::Condition;
    use crate::traffic::npc::AiType;

    /// NPC 1 holding NPC 2, and NPC 2 in `condition`.
    fn kept_with(condition: Condition) -> Option<ShipRef> {
        let govts = govts(0, 0);
        let sites = sites();
        let mut holder = ship(1, TRADERS, AiType::WimpyTrader, 0.0, 0.0);
        holder.target = Some(n(2));
        let mut held = ship(2, PIRATES, AiType::Warship, 50.0, 0.0);
        held.condition = condition;
        let npcs = [holder.clone(), held];
        let around = around(&sites, &npcs, &govts, (0.0, -100.0), 0);
        kept(&holder, &around)
    }

    #[test]
    fn a_target_whose_slot_is_active_is_kept_breaking_up_too() {
        assert_eq!(kept_with(Condition::Intact), Some(n(2)));
        assert_eq!(kept_with(Condition::Disabled), Some(n(2)));
        let breaking_up = Condition::Dying {
            ticks_left: 9,
            starting: false,
        };
        assert_eq!(kept_with(breaking_up), Some(n(2)), "@0x8d4e4-0x8d50d");
    }

    #[test]
    fn a_target_gone_or_inactive_is_let_go_and_none_stays_none() {
        assert_eq!(kept_with(Condition::Destroyed), None, "inactive");
        let govts = govts(0, 0);
        let sites = sites();
        let mut holder = ship(1, TRADERS, AiType::WimpyTrader, 0.0, 0.0);
        holder.target = Some(n(2));
        let alone = [holder.clone()];
        let around_alone = around(&sites, &alone, &govts, (0.0, -100.0), 0);
        assert_eq!(kept(&holder, &around_alone), None, "gone");
        holder.target = None;
        assert_eq!(kept(&holder, &around_alone), None, "none held");
    }
}
