//! A ship dropping an attack on a ship breaking up, as
//! `_HighLevelAIHandler`'s state-4 block does after every type's think
//! (`_AIDispatch` @0x901e5, @0x8e3df-0x8e3f6 → @0x8e714-0x8e726 in the
//! `EV Nova` executable).
//!
//! The block runs on the goal the think and the mission hunt leave: goal 4
//! ([`Goal::attacking`], an attack or a snipe, whatever the ship's type).
//! Its target's slot still active (+0xb8 @0x8e3df) and `_IsDying` true
//! (@0x8e3ef), it writes target -1 (@0x8e717), +0x96 0 (@0x8e71d) and goal
//! 0 (@0x8e726), and no substate, so a kept rush or break-off carries over
//! ([`Decision::keeps_pursuit`]). It skips the fighter launch (@0x8e6fa)
//! and the call for reinforcements (@0x8e70d), and
//! `_LowLevelAIHandler`'s passes then find no target. So nothing fires at
//! a ship breaking up but a ship that doesn't reach the block:
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
use crate::traffic::npc::Npc;

/// Whether the block drops an attack `goal` of `npc` among `around` (see
/// the module docs); `ends_wait` as [`Decision::ends_wait`].
#[must_use]
pub fn drops(npc: &Npc, goal: Goal, ends_wait: bool, around: &Surroundings) -> bool {
    (!npc.wait_running() || ends_wait)
        && goal
            .attacking()
            .is_some_and(|ship| around.breaking_up(ship))
}

/// `decision` with the block's drop applied (see the module docs).
#[must_use]
pub fn with_attack_drop(npc: &Npc, decision: Decision, around: &Surroundings) -> Decision {
    if !drops(npc, decision.goal, decision.ends_wait, around) {
        return decision;
    }
    Decision {
        goal: Goal::Idle,
        clears_target: true,
        forgets_provocation: true,
        keeps_pursuit: true,
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
    use crate::traffic::npc::{AiType, Mode};

    const DYING: Condition = Condition::Dying {
        ticks_left: 3,
        starting: false,
    };

    /// `decision`, of a ship at the centre, after the block, NPC 2 in
    /// `condition` and the player 500 up.
    fn after(wait: u32, decision: Decision, condition: Condition) -> Decision {
        let mut attacker = ship(1, PIRATES, AiType::Warship, 0.0, 0.0);
        attacker.ai_wait = wait;
        let mut other = ship(2, TRADERS, AiType::Warship, 100.0, 0.0);
        other.condition = condition;
        let npcs = [attacker, other];
        let govts = govts(0, 0);
        let sites = sites();
        let around = around(&sites, &npcs, &govts, (0.0, -500.0), 0);
        with_attack_drop(&npcs[0], decision, &around)
    }

    fn of(goal: Goal) -> Decision {
        Decision {
            goal,
            ..Decision::default()
        }
    }

    #[test]
    fn an_attack_or_snipe_on_a_ship_breaking_up_ends_keeping_its_substate() {
        for goal in [Goal::Attack(n(2)), Goal::Snipe(n(2))] {
            assert_eq!(
                after(0, of(goal), DYING),
                Decision {
                    goal: Goal::Idle,
                    clears_target: true,
                    forgets_provocation: true,
                    keeps_pursuit: true,
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
        let dropped = after(0, thought, DYING);
        assert_eq!(dropped.goal, Goal::Idle);
        assert_eq!(
            (dropped.turns_on_player, dropped.provokes, dropped.comm_call),
            (true, true, Some(CommCall::Distress))
        );
        assert!(dropped.calls_reinforcements, "the think's own call");
    }

    #[test]
    fn a_target_not_breaking_up_is_kept() {
        for condition in [Condition::Intact, Condition::Disabled, Condition::Destroyed] {
            let attack = of(Goal::Attack(n(2)));
            assert_eq!(after(0, attack, condition), attack, "{condition:?}");
        }
        let gone = of(Goal::Attack(n(9)));
        assert_eq!(after(0, gone, DYING), gone, "@0x8e731's");
        let player = of(Goal::Attack(ShipRef::Player));
        assert_eq!(after(0, player, DYING), player, "the player stands");
    }

    #[test]
    fn only_goal_4_is_dropped() {
        for goal in [
            Goal::Idle,
            Goal::Flee(n(2)),
            Goal::Inspect(n(2)),
            Goal::Assist(Help::Refuel),
        ] {
            assert_eq!(after(0, of(goal), DYING), of(goal), "{goal:?}");
        }
    }

    #[test]
    fn a_running_ai_wait_skips_the_handler_unless_the_think_ended_it() {
        let attack = of(Goal::Attack(n(2)));
        assert_eq!(after(3, attack, DYING), attack, "@0x8d4d2");
        let ended = Decision {
            ends_wait: true,
            ..attack
        };
        assert_eq!(after(3, ended, DYING).goal, Goal::Idle, "@0x83d62 first");
    }

    #[test]
    fn a_wait_kept_in_the_gate_hold_skips_the_handler_unless_the_think_ended_it() {
        let mut held = ship(1, PIRATES, AiType::Warship, 0.0, 0.0);
        held.mode = Mode::HeldAtGate { ticks_left: 20 };
        let mut other = ship(2, TRADERS, AiType::Warship, 100.0, 0.0);
        other.condition = DYING;
        let npcs = [held, other];
        let govts = govts(0, 0);
        let sites = sites();
        let around = around(&sites, &npcs, &govts, (0.0, -500.0), 0);
        let attack = Goal::Attack(n(2));
        assert!(!drops(&npcs[0], attack, false, &around), "@0x8d4d2");
        assert!(drops(&npcs[0], attack, true, &around), "@0x83d62 first");
    }
}
