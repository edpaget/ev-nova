//! The ship a trader (AI type 2 or below, `_WimpyTraderAI` and
//! `_BraveTraderAI` in the `EV Nova` executable) targets: the +0x70 it
//! holds through the goals whose blocks never write it, and its fight's
//! quarry otherwise.
//!
//! `_DamageShip` sets +0x70 to the attacker on any hit by a ship
//! (@0x3bcf0-0x3bcf6) and adds provocation only for damage above 0
//! (@0x3bcc0-0x3bcf0), so an NPC's hit for no damage leaves a trader
//! holding a target it has no reason to fight. Nothing that runs in its
//! leave, landing, gate entry, stop or idle state writes +0x70 again:
//!
//! - the idle block (goal 0, @0x8b06c, @0x8b4d6) ends in a landing
//!   (goal 1, @0x8b0f7-0x8b0fd, @0x8b567) or a stop (goal 6, @0x8b1a3,
//!   @0x8b60d) with +0x70 untouched, and in a leave (goal 2) with +0x70 -1
//!   (@0x8b190, @0x8b5fa; [`idle_leave_clears`]);
//! - unprovoked, every other goal reaches the provoked test (@0x8b1ac,
//!   @0x8b616), which writes nothing;
//! - `_HighLevelAIHandler`'s goal-1 block (@0x8d8b8-0x8db5f) has no +0x70
//!   write, neither for a gate stellar's goal 0x14 (@0x8d8f5) nor a
//!   landing's completion (@0x8dadc-0x8db5a); the goal-0x14 block
//!   (@0x8db62-0x8dd5e) writes it only on the followers it releases
//!   (@0x8dd20); the goal-2 block (@0x8dd9c-0x8de5e) writes none.
//!
//! So the target stands until the handler's slot test clears a gone slot
//! (@0x8d4e4-0x8d50d: one breaking up is still active). The original's
//! +0x70 is a slot number, so one refilled names the newcomer; the engine
//! keeps ids, which can't replicate that.
//!
//! A held target is read by the threat tests ([`Npc::fought`]) and by the
//! warp hold (@0x8d535-0x8d56b,
//! [`warp_out::hold_stage`](crate::traffic::warp_out::hold_stage)), which
//! turns a leaving trader whose warp is under way to flee it.

use crate::ai::{Goal, Surroundings};
use crate::combat::ShipRef;
use crate::combat::hull::Condition;
use crate::traffic::npc::Npc;

/// The ship a trader targets among `around`: its goal's quarry, but
/// stopped ([`Goal::Wait`], state 6), idle ([`Goal::Idle`], state 0),
/// leaving ([`Goal::JumpOut`], state 2), landing ([`Goal::Land`], state 1)
/// or entering a gate ([`Goal::EnterGate`], state 0x14) the target it
/// holds, until its slot is gone (see the module docs). A stopped trader
/// fires nothing at it (state 6 flies sub 1, @0x8f101), but the threat
/// tests read it ([`Npc::threatens_player_or_led`],
/// [`Npc::threatens_npc_among`], [`Npc::threatens_player_extended`]).
///
/// A stop keeps the trader's provocation as well (see
/// [`provocation`](crate::traffic::provocation)), and
/// [`provoked_by`](crate::ai::provoked_by) reads this target, so a stopped
/// trader with a kept target and provocation turns on it again, as the
/// original's provoked branch does from state 6 (@0x8b1ac, @0x8b616). The
/// lost-sight stop keeps it too (@0x8d803-0x8d80f; see
/// [`lost_sight`](crate::ai::lost_sight)).
#[must_use]
pub fn trader_target(npc: &Npc, around: &Surroundings) -> Option<ShipRef> {
    match npc.goal {
        Goal::Wait | Goal::Idle | Goal::JumpOut | Goal::Land(_) | Goal::EnterGate(_) => npc
            .target
            .filter(|&ship| around.condition_of(ship).is_some_and(Condition::active)),
        goal => goal.quarry(),
    }
}

/// Whether a trader's think of `thought` is its idle block's leave, which
/// clears +0x70 (`_WimpyTraderAI` @0x8b190, `_BraveTraderAI` @0x8b5fa):
/// a ship on goal 0 that thinks goal 2. A leave it already held is kept,
/// its target with it, and the idle block's landing and stop write none.
#[must_use]
pub fn idle_leave_clears(npc: &Npc, thought: Goal) -> bool {
    npc.goal == Goal::Idle && thought == Goal::JumpOut
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::fixture::{PIRATES, TRADERS, around, govts, n, ship, sites};
    use crate::catalog::StellarId;
    use crate::traffic::npc::AiType;

    /// The goals whose blocks never write a trader's +0x70.
    fn keeping() -> [Goal; 5] {
        [
            Goal::JumpOut,
            Goal::Land(StellarId(128)),
            Goal::EnterGate(StellarId(129)),
            Goal::Wait,
            Goal::Idle,
        ]
    }

    #[test]
    fn a_trader_keeps_a_hits_target_while_it_leaves_lands_enters_a_gate_waits_or_idles() {
        // The landing @0x8b0fd and the stop @0x8b1a3 leave it, the
        // provoked test @0x8b1ac writes nothing unprovoked, and the goal-1
        // (@0x8d8f5, @0x8dadc), 0x14 and 2 blocks of the high level none.
        let govts = govts(0, 0);
        let sites = sites();
        for goal in keeping() {
            let mut trader = ship(1, TRADERS, AiType::WimpyTrader, 0.0, 0.0);
            trader.goal = goal;
            trader.target = Some(n(2));
            let npcs = [trader.clone(), ship(2, PIRATES, AiType::Warship, 50.0, 0.0)];
            let around = around(&sites, &npcs, &govts, (0.0, -100.0), 0);
            assert_eq!(trader_target(&trader, &around), Some(n(2)), "{goal:?}");
        }
    }

    #[test]
    fn a_trader_lets_go_of_a_kept_target_whose_slot_is_gone() {
        // The slot test, @0x8d4e4-0x8d50d.
        let govts = govts(0, 0);
        let sites = sites();
        for goal in keeping() {
            let mut trader = ship(1, TRADERS, AiType::WimpyTrader, 0.0, 0.0);
            trader.goal = goal;
            trader.target = Some(n(2));
            let alone = [trader.clone()];
            let around_alone = around(&sites, &alone, &govts, (0.0, -100.0), 0);
            assert_eq!(trader_target(&trader, &around_alone), None, "{goal:?} gone");
            let mut gone = ship(2, PIRATES, AiType::Warship, 50.0, 0.0);
            gone.condition = Condition::Destroyed;
            let npcs = [trader.clone(), gone];
            let around = around(&sites, &npcs, &govts, (0.0, -100.0), 0);
            assert_eq!(trader_target(&trader, &around), None, "{goal:?} inactive");
        }
    }

    #[test]
    fn a_fighting_trader_targets_its_quarry() {
        let govts = govts(0, 0);
        let sites = sites();
        for goal in [Goal::Flee(n(3)), Goal::Attack(n(3))] {
            let mut trader = ship(1, TRADERS, AiType::BraveTrader, 0.0, 0.0);
            trader.goal = goal;
            trader.target = Some(n(2));
            let npcs = [
                trader.clone(),
                ship(2, PIRATES, AiType::Warship, 50.0, 0.0),
                ship(3, PIRATES, AiType::Warship, 60.0, 0.0),
            ];
            let around = around(&sites, &npcs, &govts, (0.0, -100.0), 0);
            assert_eq!(trader_target(&trader, &around), Some(n(3)), "{goal:?}");
        }
    }

    #[test]
    fn the_idle_blocks_leave_clears_the_target() {
        // @0x8b190, @0x8b5fa.
        let mut trader = ship(1, TRADERS, AiType::WimpyTrader, 0.0, 0.0);
        trader.goal = Goal::Idle;
        assert!(idle_leave_clears(&trader, Goal::JumpOut));
        assert!(!idle_leave_clears(&trader, Goal::Land(StellarId(128))));
        assert!(!idle_leave_clears(&trader, Goal::Wait));
        trader.goal = Goal::JumpOut;
        assert!(!idle_leave_clears(&trader, Goal::JumpOut), "a kept leave");
    }
}
