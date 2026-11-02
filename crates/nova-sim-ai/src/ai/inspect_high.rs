//! Goal 7's high level for any ship that keeps an inspection
//! (`_HighLevelAIHandler`'s state-7 block @0x8f11e-0x8f3c5 in the `EV
//! Nova` executable), as the engine folds it after the think:
//! [`after_think`].
//!
//! `_AIDispatch` runs the high level after every type's think (@0x901e5),
//! and the state-7 block reads only the ship's own fields and the slot
//! +0x70 names, never its AI type. So an inspection a trader or warship
//! keeps, a lost lead's that the succession copied
//! ([`succession`](crate::traffic::succession), `_AICheckParentExists`
//! @0x893b0-0x893e1), is flown and ended as the interceptor's is. In
//! order:
//!
//! - **Under its AI wait** the handler returns first (@0x8d4b9-0x8d4d2), so
//!   nothing is written. The block's own +0x4c test (@0x8f141) can't be
//!   reached behind that gate.
//! - **Its ship gone**: the slot test makes +0x70 -1 (@0x8d4e4-0x8d50d),
//!   and the block writes goal 0 (@0x8f134 → @0x8f3bc); a slot gone in the
//!   block gives +0x70 -1 and goal 0 too (@0x8f160 → @0x8f3a5). Its +0x6c
//!   is kept.
//! - **+0x70 of 0** sets `_scanActive` (@0x8f16b-0x8f170), which nothing
//!   reads; nothing is built for it.
//! - **Beyond [`INSPECT_REACH`] on either axis** it takes sub 9
//!   (@0x8f1a3-0x8f1ed), the approach the traffic flies by the goal.
//! - **Within reach of an NPC**: goal 0 and +0x70 -1, its +0x6c kept
//!   (@0x8f391).
//! - **Within reach of the player**: +0x6c, +0x70, goal and sub written 0
//!   or -1, then `_ScanPlayer` (@0x8f366-0x8f38a), the scan
//!   [`Traffic`](crate::traffic::Traffic)'s `take_goals` queues on the goal
//!   change, with its +0x6c -1, whatever the type. The nag ship's branch
//!   (+0xc8d0 = 0x3ff, @0x8f229-0x8f364, with `_ST_PlaySound` @0x8f2cf and
//!   `_DisplayComm` @0x8f32c) is the shareware's alone.
//!
//! The fold reads the slot alone, neither sight nor whether the ship is
//! [`live`](Surroundings::live): the block approaches a ship breaking up as
//! any other. It folds only an inspection the think kept, as a fresh pick
//! within reach is the interceptor's own think's
//! ([`interceptor`](super::interceptor)). It plays no sound in the
//! engine's reach and draws nothing.

#[cfg(doc)]
use crate::ai::interceptor::INSPECT_REACH;
use crate::ai::interceptor::within_reach;
use crate::ai::travel::TravelWrite;
use crate::ai::{Decision, Goal, Surroundings};
use crate::traffic::npc::Npc;

/// `decision`, `npc`'s think among `around`, with goal 7's high level
/// folded in when it kept an inspection (see the module docs): a ship gone
/// or within [`INSPECT_REACH`] ends it, [`Goal::Idle`] with its target
/// cleared and its travel stellar as it was. Any other decision stands.
#[must_use]
pub fn after_think(npc: &Npc, decision: Decision, around: &Surroundings) -> Decision {
    let Goal::Inspect(ship) = decision.goal else {
        return decision;
    };
    if decision.goal != npc.goal || npc.high_level_waits_under(decision.goal) {
        return decision;
    }
    let ends = around
        .state_of(ship)
        .is_none_or(|state| within_reach(state.position - npc.state.position));
    if !ends {
        return decision;
    }
    Decision {
        goal: Goal::Idle,
        clears_target: true,
        travel: TravelWrite::Keep,
        ..decision
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::fixture::{TRADERS, around, govts, n, ship, sites};
    use crate::ai::interceptor::INSPECT_REACH;
    use crate::catalog::StellarId;
    use crate::combat::ShipRef;
    use crate::combat::hull::Condition;
    use crate::traffic::npc::AiType;

    /// A wimpy trader (NPC 1) at the centre keeping an inspection of
    /// `inspected`, a trader (NPC 2) at (`x`, 0) when `here`, the player
    /// at (`x`, 0) too.
    fn folded(inspected: ShipRef, x: f32, here: bool, tune: impl Fn(&mut Npc)) -> Decision {
        let mut trader = ship(1, TRADERS, AiType::WimpyTrader, 0.0, 0.0);
        trader.goal = Goal::Inspect(inspected);
        trader.target = Some(inspected);
        tune(&mut trader);
        let other = ship(2, TRADERS, AiType::WimpyTrader, x, 0.0);
        let npcs = if here {
            vec![trader, other]
        } else {
            vec![trader]
        };
        let govts = govts(0, 0);
        let sites = sites();
        let around = around(&sites, &npcs, &govts, (x, 0.0), 0);
        let thought = Decision {
            goal: Goal::Inspect(inspected),
            travel: TravelWrite::Set(StellarId(128)),
            ..Decision::default()
        };
        after_think(&npcs[0], thought, &around)
    }

    fn ended(decision: Decision) -> bool {
        decision.goal == Goal::Idle
            && decision.clears_target
            && decision.travel == TravelWrite::Keep
    }

    #[test]
    fn a_kept_inspection_of_a_ship_gone_ends_keeping_the_travel_stellar() {
        // @0x8f134 → @0x8f3bc, @0x8f160 → @0x8f3a5.
        assert!(ended(folded(n(2), 500.0, false, |_| {})));
    }

    #[test]
    fn a_kept_inspection_ends_within_reach_on_both_axes_of_an_npc_or_the_player() {
        // @0x8f1a3-0x8f1ed, then @0x8f391 or the scan @0x8f366-0x8f38a.
        for inspected in [n(2), ShipRef::Player] {
            assert!(
                ended(folded(inspected, INSPECT_REACH, true, |_| {})),
                "{inspected:?}"
            );
            let beyond = folded(inspected, INSPECT_REACH + 1.0, true, |_| {});
            assert_eq!(
                beyond.goal,
                Goal::Inspect(inspected),
                "{inspected:?}: sub 9"
            );
            assert!(!beyond.clears_target);
            assert_eq!(beyond.travel, TravelWrite::Set(StellarId(128)));
        }
    }

    #[test]
    fn the_fold_reads_the_slot_alone_so_a_ship_breaking_up_is_approached() {
        let mut trader = ship(1, TRADERS, AiType::WimpyTrader, 0.0, 0.0);
        trader.goal = Goal::Inspect(n(2));
        let mut dying = ship(2, TRADERS, AiType::WimpyTrader, 500.0, 0.0);
        dying.condition = Condition::Dying {
            ticks_left: 3,
            starting: false,
        };
        let npcs = [trader, dying];
        let govts = govts(0, 0);
        let sites = sites();
        let around = around(&sites, &npcs, &govts, (0.0, -3000.0), 0);
        let thought = Decision {
            goal: Goal::Inspect(n(2)),
            ..Decision::default()
        };
        assert_eq!(after_think(&npcs[0], thought, &around), thought);
    }

    #[test]
    fn under_the_ai_wait_nothing_is_written() {
        // The handler returns first (@0x8d4b9-0x8d4d2).
        let waiting = folded(n(2), 10.0, true, |npc| npc.ai_wait = 30);
        assert_eq!(waiting.goal, Goal::Inspect(n(2)));
        assert!(!waiting.clears_target);
    }

    #[test]
    fn through_nova_ai_a_trader_or_warship_keeping_an_inspection_within_reach_idles() {
        // The think keeps goal 7 (@0x8b06c, @0x8b835), the state-7 block
        // ends it within reach of an NPC: goal 0, +0x70 -1, +0x6c kept
        // (@0x8f391).
        use crate::ai::{Behaviour, NovaAi};
        use nova_sim_base::fixture::Draws;
        for ai_type in [AiType::WimpyTrader, AiType::BraveTrader, AiType::Warship] {
            let mut keeper = ship(1, TRADERS, ai_type, 0.0, 0.0);
            keeper.goal = Goal::Inspect(n(2));
            keeper.target = Some(n(2));
            keeper.travel = Some(StellarId(128));
            let npcs = [keeper, ship(2, TRADERS, AiType::WimpyTrader, 60.0, -60.0)];
            let govts = govts(0, 0);
            let sites = sites();
            let around = around(&sites, &npcs, &govts, (0.0, -3000.0), 0);
            let decision = NovaAi::default().decision(&npcs[0], &around, &mut Draws::of(&[0; 4]));
            assert_eq!(
                (decision.goal, decision.clears_target, decision.travel),
                (Goal::Idle, true, TravelWrite::Keep),
                "{ai_type:?}"
            );
        }
    }

    #[test]
    fn a_fresh_pick_or_any_other_goal_is_left_alone() {
        let fresh = folded(n(2), 10.0, true, |npc| npc.goal = Goal::Idle);
        assert_eq!(fresh.goal, Goal::Inspect(n(2)), "the think's own");
        let govts = govts(0, 0);
        let sites = sites();
        let npcs = [ship(1, TRADERS, AiType::WimpyTrader, 0.0, 0.0)];
        let around = around(&sites, &npcs, &govts, (0.0, 10.0), 0);
        let other = Decision {
            goal: Goal::Attack(ShipRef::Player),
            ..Decision::default()
        };
        assert_eq!(after_think(&npcs[0], other, &around), other);
    }
}
