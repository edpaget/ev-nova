//! The target a warship or interceptor holds as it thinks (+0x70), as
//! `_WarshipAI` (@0x8b729) and `_InterceptorAI` (@0x8c896) read it in the
//! `EV Nova` executable, and what their thinks do with a target they drop.
//! [`hunt`](crate::ai::warship::hunt) reads it through
//! [`Retreat::held_target`](crate::ai::warship::Retreat::held_target).
//!
//! The held target is [`Npc::target`] at the think: the target the last
//! frame ended with ([`Decision::last_target`](crate::ai::Decision::last_target),
//! else [`Behaviour::target`](crate::ai::Behaviour::target)'s answer), or a
//! hit's reaction since. The attack block's ally drop leaves a ship on goal
//! 0 holding its target (@0x8e2d2,
//! [`Decision::holds_target`](crate::ai::Decision::holds_target); see
//! [`attack_drop`](crate::ai::attack_drop)), and this is how the next
//! think takes it up again.
//!
//! - **A provoked ship** (+0x96 above 0; `_WarshipAI` @0x8bb0c-0x8bb44,
//!   `_InterceptorAI` @0x8cbf6-0x8cc31), in any goal but fleeing or
//!   inspecting, attacks the target it holds, as before; one its think
//!   drops ([`Held::Drop`]) ends the think below.
//! - **A warship holding a target unprovoked**, in [`Goal::Idle`] (state 0,
//!   @0x8b843-0x8b860) or travelling ([`Goal::Land`], [`Goal::EnterGate`]
//!   and [`Goal::JumpOut`], states 1, 0x14 and 2, @0x8bb6e-0x8bba2), with
//!   its warp stage not under way (+0x50 at or below 0), attacks it
//!   ([`Held::Attack`]), with no search: only +0x70 -1 calls
//!   `_SelectWarshipTarget` (@0x8b875, @0x8bb96).
//! - **An interceptor holding a target unprovoked**, in [`Goal::Idle`],
//!   [`Goal::Land`] or [`Goal::EnterGate`] with no AI wait running (+0x4c,
//!   @0x8cc86-0x8cc8d), searches nothing (@0x8cc93) and jumps past the
//!   inspection and the patrol (@0x8d13f → @0x8d14d), keeping its goal and
//!   the target ([`Held::Hold`]). One whose target is gone keeps its goal
//!   and loses the target ([`Held::Clear`]): the handler's slot test clears
//!   a freed slot's +0x70 after the think (@0x8d4e4-0x8d50d; see
//!   [`gone`](crate::traffic::gone)). Its out-of-ammo flight (@0x8d343-0x8d391)
//!   still runs on that goal ([`out_of_ammo::after_patrol`](crate::ai::out_of_ammo::after_patrol)),
//!   and a stop there drops the target with the provocation.
//! - **A dropped target** ([`Held::Drop`]): a held target, a provoker, the
//!   government's word on the player or a fresh pick that the think's own
//!   drop lets go (a warship's [`dropped`](crate::ai::target::dropped),
//!   @0x8bc4e-0x8bcaf; an interceptor's
//!   [`let_go`](crate::ai::target::let_go), @0x8d2fd-0x8d33a) leaves the
//!   ship in [`Goal::Idle`] with no target for that think, running no idle
//!   block (`_WarshipAI` @0x8bcb4 → @0x8bf8c) and no patrol
//!   (`_InterceptorAI`'s drop comes after it).
//!
//! So a warship or interceptor that takes an ally or a fleet-mate is
//! dropped by the attack block in the same dispatch, goal and sub 0, its
//! target and provocation kept, and its next think takes it again: it
//! coasts on sub 0, firing nothing, until the target dies, breaks up or
//! leaves.
//!
//! **Left out**, as the engine has nothing to model them with or they are
//! another phase's: the provoked branch's +0x50 gate (@0x8bb0c-0x8bb44,
//! @0x8cbf6-0x8cc31): a provoked ship whose warp is under way still turns
//! on its provoker here; and an interceptor whose AI wait runs, which the
//! original lets search nothing at all (@0x8cc86-0x8cc8d), and which still
//! searches here. The inspecting interceptor a hit provokes (state 7,
//! @0x8cc0d) is rdm:phase/play-feel-9/phase-6-inspection-redirected-by-a-hit.
//! The branches play no sound and make no draw.

use crate::ai::{Goal, Surroundings};
use crate::combat::ShipRef;
use crate::traffic::npc::Npc;

/// How a hunter's think reads the target it holds unprovoked (see the
/// module docs).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HeldTarget {
    /// A warship attacks it (`_WarshipAI` @0x8b843-0x8b860,
    /// @0x8bb6e-0x8bba2).
    Attacks,
    /// An interceptor keeps its goal holding it (`_InterceptorAI`
    /// @0x8cc93 → @0x8d13f).
    Holds,
}

/// What a hunter's think does with the target it holds (see the module
/// docs).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Held {
    /// It attacks it, through the hunt's retreats.
    Attack(ShipRef),
    /// Its think drops it: [`Goal::Idle`] with no target, no idle block.
    Drop,
    /// It keeps its goal holding it ([`HeldTarget::Holds`]).
    Hold(ShipRef),
    /// It keeps its goal, the target gone from its slot.
    Clear,
}

/// What `npc`'s think does, unprovoked, with `target`, the target it
/// holds, as `reading` says, `kept` saying whether its think's drop keeps
/// a ship; `None` when its goal reads no held target, and it searches.
#[must_use]
pub fn unprovoked(
    npc: &Npc,
    target: ShipRef,
    around: &Surroundings,
    reading: HeldTarget,
    kept: impl Fn(ShipRef) -> bool,
) -> Option<Held> {
    let travelling = matches!(npc.goal, Goal::Land(_) | Goal::EnterGate(_));
    match reading {
        HeldTarget::Attacks => {
            let reads = (npc.goal == Goal::Idle || travelling || npc.goal == Goal::JumpOut)
                && !npc.warp.under_way();
            reads.then(|| provoked(target, kept))
        }
        HeldTarget::Holds => {
            let reads = (npc.goal == Goal::Idle || travelling) && !npc.wait_running();
            reads.then(|| {
                if around.active(target) {
                    Held::Hold(target)
                } else {
                    Held::Clear
                }
            })
        }
    }
}

/// What a hunter's think does with `target`, the ship provoking it or one
/// it takes to attack, `kept` saying whether its think's drop keeps it: it
/// attacks it, or drops it.
#[must_use]
pub fn provoked(target: ShipRef, kept: impl Fn(ShipRef) -> bool) -> Held {
    if kept(target) {
        Held::Attack(target)
    } else {
        Held::Drop
    }
}

#[cfg(test)]
mod tests {
    use crate::ai::fixture::{NEUTRAL, POLICE, TRADERS, around, govts, n, ship, sites};
    use crate::ai::interceptor::Interceptor;
    use crate::ai::warship::Warship;
    use crate::ai::{Behaviour, Decision, Goal, NovaAi};
    use crate::catalog::StellarId;
    use crate::combat::hull::Condition;
    use crate::traffic::npc::{AiType, Npc};
    use nova_sim_base::fixture::Draws;

    const DYING: Condition = Condition::Dying {
        ticks_left: 3,
        starting: false,
    };

    /// A police ship of `ai_type` (NPC 1) at the centre, holding `target`
    /// in `goal`, unprovoked.
    fn holding(ai_type: AiType, goal: Goal, target: u32) -> Npc {
        Npc {
            goal,
            target: Some(n(target)),
            ..ship(1, POLICE, ai_type, 0.0, 0.0)
        }
    }

    /// An allied trader (NPC 3) 500 below.
    fn trader() -> Npc {
        ship(3, TRADERS, AiType::WimpyTrader, 0.0, 500.0)
    }

    /// `behaviour`'s decision for `npcs[0]` among `npcs`, the player far
    /// off and clean, and the draws it asked.
    fn decided_by(behaviour: &dyn Behaviour, npcs: &[Npc]) -> (Decision, Vec<u32>) {
        let (sites, govts) = (sites(), govts(0, 0));
        let around = around(&sites, npcs, &govts, (0.0, -5000.0), 0);
        let mut chance = Draws::of(&[]);
        let decision = behaviour.decision(&npcs[0], &around, &mut chance);
        (decision, chance.asked)
    }

    fn warship(npcs: &[Npc]) -> Decision {
        decided_by(&Warship::default(), npcs).0
    }

    #[test]
    fn an_unprovoked_warship_holding_an_allied_npc_attacks_it() {
        // State 0 with +0x70 not -1 takes state 4 at once (@0x8b843-0x8b860).
        let npcs = [holding(AiType::Warship, Goal::Idle, 3), trader()];
        assert_eq!(warship(&npcs).goal, Goal::Attack(n(3)));
    }

    #[test]
    fn a_travelling_warship_holding_a_target_attacks_it() {
        // States 1, 0x14 and 2 (@0x8bb6e-0x8bba2).
        let neutral = ship(3, NEUTRAL, AiType::Warship, 0.0, 500.0);
        for goal in [
            Goal::Land(StellarId(128)),
            Goal::EnterGate(StellarId(128)),
            Goal::JumpOut,
        ] {
            let npcs = [holding(AiType::Warship, goal, 3), neutral.clone()];
            assert_eq!(warship(&npcs).goal, Goal::Attack(n(3)), "{goal:?}");
        }
    }

    #[test]
    fn a_warship_whose_warp_is_under_way_never_attacks_the_target_it_holds() {
        // +0x50 above 0 (@0x8b843-0x8b860).
        let mut leaving = holding(AiType::Warship, Goal::JumpOut, 3);
        leaving.warp.stage = 1.0;
        let npcs = [leaving, ship(3, NEUTRAL, AiType::Warship, 0.0, 500.0)];
        assert_eq!(warship(&npcs).goal, Goal::JumpOut);
    }

    #[test]
    fn a_warship_holding_a_target_it_drops_ends_idle_with_no_target_and_no_landing() {
        // The drop (@0x8bc4e-0x8bcaf) jumps past the idle block (@0x8bcb4
        // → @0x8bf8c).
        let dropped = Decision {
            goal: Goal::Idle,
            clears_target: true,
            ..Decision::default()
        };
        let gone = [holding(AiType::Warship, Goal::Idle, 9)];
        assert_eq!(warship(&gone), dropped, "gone");
        let dying = [
            holding(AiType::Warship, Goal::Idle, 3),
            Npc {
                condition: DYING,
                ..trader()
            },
        ];
        assert_eq!(warship(&dying), dropped, "breaking up");
        let harmless = [
            holding(AiType::Warship, Goal::Land(StellarId(128)), 3),
            Npc {
                condition: Condition::Disabled,
                ..trader()
            },
        ];
        assert_eq!(
            warship(&harmless),
            dropped,
            "disabled, no destroying weapons"
        );
    }

    #[test]
    fn a_warship_provoked_by_a_ship_it_drops_ends_idle_with_no_target() {
        // The provoked branch (@0x8bb0c-0x8bb44) takes state 4, and the drop
        // lets it go.
        let mut provoked = holding(AiType::Warship, Goal::Idle, 3);
        provoked.provoked = 5.0;
        let npcs = [
            provoked,
            Npc {
                condition: Condition::Disabled,
                ..ship(3, NEUTRAL, AiType::Warship, 0.0, 500.0)
            },
        ];
        let decision = warship(&npcs);
        assert_eq!((decision.goal, decision.clears_target), (Goal::Idle, true));
    }

    fn interceptor(npcs: &[Npc]) -> (Decision, Vec<u32>) {
        decided_by(&Interceptor::default(), npcs)
    }

    #[test]
    fn an_unprovoked_interceptor_whose_held_target_is_gone_keeps_its_goal_with_no_target() {
        // The slot test clears a freed slot's +0x70 after the think
        // (@0x8d4e4-0x8d50d).
        let goal = Goal::Land(StellarId(128));
        let (decision, asked) = interceptor(&[holding(AiType::Interceptor, goal, 9)]);
        assert_eq!(
            (decision.goal, decision.clears_target, decision.holds_target),
            (goal, true, None)
        );
        assert_eq!(asked, Vec::<u32>::new(), "no inspection, no patrol");
    }

    #[test]
    fn an_interceptor_out_of_ammunition_holding_a_target_stops_and_lets_it_go() {
        // The out-of-ammo flight (@0x8d343-0x8d391) runs on the goal it
        // keeps; with no ship to flee it stops, state 3 to 0, its
        // provocation zeroed (@0x8e228-0x8e23d).
        let dry = crate::ai::fixture::short_of_ammo(
            holding(AiType::Interceptor, Goal::Land(StellarId(128)), 3),
            crate::ai::fire::AmmoLeft::OnlyUnlimited,
            true,
        );
        let (decision, _) = interceptor(&[dry, trader()]);
        assert_eq!(
            (
                decision.goal,
                decision.clears_target,
                decision.holds_target,
                decision.forgets_provocation
            ),
            (Goal::Idle, true, None, true)
        );
    }

    #[test]
    fn an_unprovoked_interceptor_holding_an_ally_keeps_it_and_inspects_nothing() {
        // +0x70 not -1 searches nothing and jumps past the inspection and
        // the patrol (@0x8cc93 → @0x8d13f → @0x8d14d).
        for goal in [
            Goal::Idle,
            Goal::Land(StellarId(128)),
            Goal::EnterGate(StellarId(128)),
        ] {
            let npcs = [holding(AiType::Interceptor, goal, 3), trader()];
            let (decision, asked) = interceptor(&npcs);
            assert_eq!(
                (decision.goal, decision.holds_target),
                (goal, Some(n(3))),
                "{goal:?}"
            );
            assert_eq!(asked, Vec::<u32>::new(), "no inspection pick, {goal:?}");
        }
    }

    #[test]
    fn a_provoked_interceptor_attacks_the_ally_it_holds() {
        // Active and provoked, state 4 (@0x8ccb0-0x8cccf).
        let mut provoked = holding(AiType::Interceptor, Goal::Idle, 3);
        provoked.provoked = 5.0;
        let (decision, _) = interceptor(&[provoked, trader()]);
        assert_eq!(decision.goal, Goal::Attack(n(3)));
    }

    #[test]
    fn an_interceptor_provoked_by_a_ship_it_lets_go_ends_idle_with_no_target_or_patrol() {
        // The drop (@0x8d2fd-0x8d33a) jumps past the patrol.
        let mut provoked = holding(AiType::Interceptor, Goal::Idle, 3);
        provoked.provoked = 5.0;
        let downed = Npc {
            condition: Condition::Disabled,
            ..trader()
        };
        let (decision, asked) = interceptor(&[provoked, downed]);
        assert_eq!((decision.goal, decision.clears_target), (Goal::Idle, true));
        assert_eq!(asked, Vec::<u32>::new(), "no inspection, no patrol");
    }

    /// `npcs[0]`'s decision through Nova's AI.
    fn flown(npcs: &[Npc]) -> Decision {
        decided_by(&NovaAi::default(), npcs).0
    }

    /// `npc` as the frame `decision` ends leaves it: its goal, and its
    /// target held or cleared.
    fn after(npc: &Npc, decision: &Decision) -> Npc {
        Npc {
            goal: decision.goal,
            target: decision.last_target().unwrap_or(decision.goal.quarry()),
            ..npc.clone()
        }
    }

    #[test]
    fn a_hunter_provoked_by_an_ally_stays_idle_holding_it_think_after_think_until_it_goes() {
        for ai_type in [AiType::Warship, AiType::Interceptor] {
            let mut provoked = holding(ai_type, Goal::Idle, 3);
            provoked.provoked = 5.0;
            let mut npcs = [provoked, trader()];
            for think in 0..2 {
                let decision = flown(&npcs);
                assert_eq!(
                    (decision.goal, decision.holds_target),
                    (Goal::Idle, Some(n(3))),
                    "{ai_type:?}, think {think}: the block's ally drop (@0x8e2d2)"
                );
                assert!(!decision.forgets_provocation, "{ai_type:?}");
                npcs[0] = after(&npcs[0], &decision);
            }
            let gone = [npcs[0].clone()];
            let decision = flown(&gone);
            assert_eq!(
                (decision.goal, decision.clears_target),
                (Goal::Idle, true),
                "{ai_type:?}: gone"
            );
            let searching = [after(&gone[0], &decision)];
            assert_eq!(searching[0].target, None);
            assert_ne!(flown(&searching).goal, Goal::Idle, "{ai_type:?} searches");
        }
    }
}
