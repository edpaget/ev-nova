//! The head of `_HighLevelAIHandler`, which every type's think reaches
//! (`_AIDispatch` @0x901e5 in the `EV Nova` executable).
//!
//! The think runs first on every path that reaches the handler
//! (@0x8ff43-0x90016), so what the head reads is what the think left:
//!
//! - **The wait exit** ([`waits_out`], [`skips`], @0x8d4b9-0x8d4d2): an AI
//!   wait above 0 (+0x4c, [`Npc::wait_running`], which also reads the wait
//!   the engine keeps in a gate mode's count) returns at once to @0x8fb47,
//!   unless the goal is 0xa ([`Goal::flies_in_form`]) or 0xe
//!   ([`Goal::Drift`], tests @0x8d4c8, @0x8d4ce). A think that ended the
//!   wait has written +0x4c first ([`Decision::ends_wait`]); the traffic
//!   ends it later, so the head reads the word. Nothing that follows runs
//!   for a ship that waits out: no mission hunt, no lost sight, no flee
//!   drop, no attack block and no call for reinforcements.
//! - **The slot test** ([`slot_test`], @0x8d4e4-0x8d50d): a target whose
//!   +0xb8 is clear becomes -1, for every goal, before the mission hunt and
//!   the lost-sight block. Goal 4 on target -1 then ends at @0x8e263 →
//!   @0x8e731: goal 0, and +0x96 stays. A flight's end is
//!   [`flight_ended`](crate::traffic::provocation::flight_ended)'s, as the
//!   flee block (@0x8dff6) comes after the hunt.
//!
//! The head has no sound and no draw.

use crate::ai::{Decision, Goal, Surroundings};
use crate::traffic::npc::Npc;

/// Whether `npc`'s wait stops the handler at its head, its think having
/// left `goal` (@0x8d4b9-0x8d4d2; see the module docs).
#[must_use]
pub fn waits_out(npc: &Npc, goal: Goal) -> bool {
    npc.wait_running() && !goal.flies_in_form() && goal != Goal::Drift
}

/// Whether the handler is skipped after `think`: `npc` [`waits_out`], and
/// the think didn't end its wait ([`Decision::ends_wait`]).
#[must_use]
pub fn skips(npc: &Npc, think: &Decision) -> bool {
    waits_out(npc, think.goal) && !think.ends_wait
}

/// `decision` through the handler's slot test among `around`: its goal 4 on
/// a ship not [`active`](Surroundings::active) is [`Goal::Idle`] with no
/// target (@0x8d4e4-0x8d50d, then @0x8e263 → @0x8e731). +0x96 stays. It
/// takes no flight: the test writes only target -1, and a flight's +0x96 0
/// belongs to the flee block, after the hunt.
#[must_use]
pub fn slot_test(decision: Decision, around: &Surroundings) -> Decision {
    match decision.goal.attacking() {
        Some(ship) if !around.active(ship) => Decision {
            goal: Goal::Idle,
            clears_target: true,
            ..decision
        },
        _ => decision,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::CommCall;
    use crate::ai::fixture::{PIRATES, TRADERS, around, govts, n, ship, sites};
    use crate::catalog::StellarId;
    use crate::combat::ShipRef;
    use crate::combat::hull::Condition;
    use crate::traffic::npc::{AiType, Mode, NpcId};

    fn of(goal: Goal) -> Decision {
        Decision {
            goal,
            ..Decision::default()
        }
    }

    fn fighter() -> Npc {
        ship(1, PIRATES, AiType::Warship, 0.0, 0.0)
    }

    /// A goal the wait exit stops.
    const ATTACK: Goal = Goal::Attack(ShipRef::Npc(NpcId(2)));
    /// The goals it lets through: 0xa's three, and 0xe.
    const FORM: [Goal; 4] = [
        Goal::Follow {
            lead: NpcId(9),
            guard: None,
        },
        Goal::Formation { guard: None },
        Goal::Hold,
        Goal::Drift,
    ];

    #[test]
    fn a_running_ai_wait_skips_the_handler_unless_the_think_ended_it() {
        let waiting = Npc {
            ai_wait: 3,
            ..fighter()
        };
        assert!(skips(&waiting, &of(ATTACK)), "@0x8d4d2");
        let ended = Decision {
            ends_wait: true,
            ..of(ATTACK)
        };
        assert!(!skips(&waiting, &ended), "@0x83d62 first");
        assert!(!skips(&fighter(), &of(ATTACK)), "no wait");
    }

    #[test]
    fn a_wait_kept_in_the_gate_hold_skips_the_handler_unless_the_think_ended_it() {
        let held = Npc {
            mode: Mode::HeldAtGate { ticks_left: 20 },
            ..fighter()
        };
        assert!(skips(&held, &of(ATTACK)), "@0x8d4d2");
        let ended = Decision {
            ends_wait: true,
            ..of(ATTACK)
        };
        assert!(!skips(&held, &ended), "@0x83d62 first");
    }

    #[test]
    fn goals_0xa_and_0xe_pass_the_wait_exit_whatever_the_wait() {
        let waiting = Npc {
            ai_wait: 3,
            ..fighter()
        };
        let held = Npc {
            mode: Mode::HeldAtGate { ticks_left: 20 },
            ..fighter()
        };
        for goal in FORM {
            assert!(!waits_out(&waiting, goal), "{goal:?}");
            assert!(!waits_out(&held, goal), "{goal:?}, held");
            assert!(!skips(&waiting, &of(goal)), "{goal:?}");
        }
        for goal in [
            ATTACK,
            Goal::Idle,
            Goal::Land(StellarId(128)),
            Goal::Wait,
            Goal::CountWait,
        ] {
            assert!(waits_out(&waiting, goal), "{goal:?}");
        }
    }

    /// `decision` after the slot test, NPC 2 in `condition`.
    fn tested(decision: Decision, condition: Condition) -> Decision {
        let mut other = ship(2, TRADERS, AiType::Warship, 100.0, 0.0);
        other.condition = condition;
        let npcs = [fighter(), other];
        let govts = govts(0, 0);
        let sites = sites();
        let around = around(&sites, &npcs, &govts, (0.0, -500.0), 0);
        slot_test(decision, &around)
    }

    const DYING: Condition = Condition::Dying {
        ticks_left: 3,
        starting: false,
    };

    #[test]
    fn goal_4_on_a_destroyed_or_gone_ship_ends_on_goal_0_with_target_minus_1() {
        for goal in [Goal::Attack(n(2)), Goal::Snipe(n(2))] {
            assert_eq!(
                tested(of(goal), Condition::Destroyed),
                Decision {
                    goal: Goal::Idle,
                    clears_target: true,
                    ..Decision::default()
                },
                "{goal:?}, destroyed"
            );
        }
        for goal in [Goal::Attack(n(9)), Goal::Snipe(n(9))] {
            assert_eq!(
                tested(of(goal), Condition::Intact),
                Decision {
                    goal: Goal::Idle,
                    clears_target: true,
                    ..Decision::default()
                },
                "{goal:?}, gone"
            );
        }
    }

    #[test]
    fn the_slot_test_writes_the_goal_and_target_alone() {
        let thought = Decision {
            provokes: true,
            comm_call: Some(CommCall::Distress),
            calls_reinforcements: true,
            forgets_provocation: true,
            ..of(Goal::Attack(n(9)))
        };
        assert_eq!(
            tested(thought, Condition::Intact),
            Decision {
                goal: Goal::Idle,
                clears_target: true,
                ..thought
            }
        );
    }

    #[test]
    fn a_ship_in_its_slot_and_a_flight_are_left_alone() {
        for condition in [Condition::Intact, Condition::Disabled, DYING] {
            let attack = of(Goal::Attack(n(2)));
            assert_eq!(tested(attack, condition), attack, "{condition:?}");
        }
        for goal in [Goal::Flee(n(9)), Goal::Flee(n(2))] {
            let flight = of(goal);
            assert_eq!(tested(flight, Condition::Destroyed), flight, "{goal:?}");
        }
    }
}
