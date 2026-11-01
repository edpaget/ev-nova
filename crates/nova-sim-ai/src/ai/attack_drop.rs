//! A ship dropping an attack on a ship breaking up, as
//! `_HighLevelAIHandler`'s state-4 block does after every type's think
//! (`_AIDispatch` @0x901e5, @0x8e3df-0x8e3f6 → @0x8e714-0x8e726 in the
//! `EV Nova` executable).
//!
//! The block runs on the goal the think and the mission hunt leave: goal 4
//! ([`Goal::attacking`], an attack or a snipe, whatever the ship's type).
//! Its target's slot still active (+0xb8 @0x8e3df) and `_IsDying` true
//! (@0x8e3ef), it writes target -1 (@0x8e717), +0x96 0 (@0x8e71d) and goal
//! 0 (@0x8e726), and the handler's tail then writes sub 0
//! (@0x8fb09-0x8fb13), so no pursuit carries over. Its +0x96 0 comes after
//! the think, so it
//! stands over a word's 1 (@0x8cb9d) on a player breaking up. It skips the
//! fighter launch (@0x8e6fa) and the call for reinforcements (@0x8e70d),
//! and `_LowLevelAIHandler`'s passes then find no target. So nothing fires
//! at a ship breaking up but a ship that doesn't reach the block:
//!
//! - One whose AI wait runs (+0x4c above 0, [`Npc::wait_running`], which
//!   also reads the wait the engine keeps in a gate mode's count): the
//!   handler returns at once (@0x8d4b9-0x8d4d2), though a think that ended
//!   the wait ([`Decision::ends_wait`]) wrote it first.
//! - One in the regroup pause ([`regroup`](crate::ai::regroup)), which
//!   runs no high level.
//! - One that can't see its target: the lost-sight block ends the handler
//!   first (@0x8d85a, @0x8d8a6, [`lost_sight`](crate::ai::lost_sight)).
//!
//! The target -1 stands to the end of the frame for every thinker, a
//! target step that works its answer out afresh included
//! ([`Decision::clears_target`]), so the ship's next think finds no target.
//!
//! A plunder (goal 0xd) has its own block, and the pirate's think drops a
//! victim breaking up before it (@0x8c315-0x8c330).
//!
//! The other exits from the block's head:
//!
//! - A target gone from its slot never reaches @0x8e3e6: the handler's
//!   slot test has made it -1 (@0x8d4e4-0x8d50d), and an attack on -1 ends
//!   at @0x8e731 with its provocation
//!   ([`fight_on_gone`](crate::traffic::provocation::fight_on_gone)).
//! - The ally and fleet drops come before the slot test (@0x8e2d2,
//!   @0x8e381, @0x8e7af, @0x8e820), and write goal 0 with no +0x96. Only
//!   the zeroed provocation of a dying ship of the attacker's own fleet
//!   differs here.
//!
//! The warship's and the pirate's thinks drop such a target themselves,
//! with no +0x96 (`_WarshipAI` @0x8bc67-0x8bca9, `_PirateWarshipAI`
//! @0x8c315-0x8c330), so goal 4 doesn't reach this block for them. No
//! sound and no draw.

use crate::ai::{Decision, Goal, Surroundings};
#[cfg(doc)]
use crate::traffic::npc::Npc;

/// `decision` with the block's drop applied (see the module docs).
#[must_use]
pub fn with_attack_drop(decision: Decision, around: &Surroundings) -> Decision {
    if !decision
        .goal
        .attacking()
        .is_some_and(|ship| around.breaking_up(ship))
    {
        return decision;
    }
    Decision {
        goal: Goal::Idle,
        provokes: false,
        clears_target: true,
        forgets_provocation: true,
        ..decision
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::fixture::{PIRATES, TRADERS, around, govts, n, ship, sites};
    use crate::ai::{CommCall, Help};
    use crate::combat::ShipRef;
    use crate::combat::hull::Condition;
    use crate::traffic::npc::AiType;

    const DYING: Condition = Condition::Dying {
        ticks_left: 3,
        starting: false,
    };

    /// `decision`, of a ship at the centre, after the block, NPC 2 in
    /// `condition` and the player 500 up.
    fn after(decision: Decision, condition: Condition) -> Decision {
        let attacker = ship(1, PIRATES, AiType::Warship, 0.0, 0.0);
        let mut other = ship(2, TRADERS, AiType::Warship, 100.0, 0.0);
        other.condition = condition;
        let npcs = [attacker, other];
        let govts = govts(0, 0);
        let sites = sites();
        let around = around(&sites, &npcs, &govts, (0.0, -500.0), 0);
        with_attack_drop(decision, &around)
    }

    fn of(goal: Goal) -> Decision {
        Decision {
            goal,
            ..Decision::default()
        }
    }

    #[test]
    fn an_attack_or_snipe_on_a_ship_breaking_up_ends_on_goal_0() {
        for goal in [Goal::Attack(n(2)), Goal::Snipe(n(2))] {
            assert_eq!(
                after(of(goal), DYING),
                Decision {
                    goal: Goal::Idle,
                    clears_target: true,
                    forgets_provocation: true,
                    ..Decision::default()
                },
                "{goal:?}"
            );
        }
    }

    #[test]
    fn the_thinks_other_words_stand_for_they_came_first() {
        let thought = Decision {
            turns_on_player: true,
            provokes: true,
            comm_call: Some(CommCall::Distress),
            calls_reinforcements: true,
            ..of(Goal::Attack(n(2)))
        };
        let dropped = after(thought, DYING);
        assert_eq!(dropped.goal, Goal::Idle);
        assert_eq!(
            (dropped.turns_on_player, dropped.comm_call),
            (true, Some(CommCall::Distress))
        );
        assert!(dropped.calls_reinforcements, "the think's own call");
    }

    #[test]
    fn its_zero_stands_over_a_word_s_1() {
        // The word's +0x96 1 (@0x8cb9d) is written in the think, and the
        // block's 0 (@0x8e71d) after it.
        let worded = Decision {
            provokes: true,
            ..of(Goal::Attack(n(2)))
        };
        assert!(!after(worded, DYING).provokes, "@0x8e71d");
    }

    #[test]
    fn a_target_not_breaking_up_is_kept() {
        for condition in [Condition::Intact, Condition::Disabled, Condition::Destroyed] {
            let attack = of(Goal::Attack(n(2)));
            assert_eq!(after(attack, condition), attack, "{condition:?}");
        }
        let gone = of(Goal::Attack(n(9)));
        assert_eq!(after(gone, DYING), gone, "@0x8e731's");
        let player = of(Goal::Attack(ShipRef::Player));
        assert_eq!(after(player, DYING), player, "the player stands");
    }

    #[test]
    fn only_goal_4_is_dropped() {
        for goal in [
            Goal::Idle,
            Goal::Flee(n(2)),
            Goal::Inspect(n(2)),
            Goal::Assist(Help::Refuel),
            Goal::Plunder {
                ship: n(2),
                hopeless: false,
            },
        ] {
            assert_eq!(after(of(goal), DYING), of(goal), "{goal:?}");
        }
    }
}
