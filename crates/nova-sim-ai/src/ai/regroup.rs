//! Goal 0x13, the regroup pause: what a ship of a fleet whose lead was
//! lost does until it takes up its new lead's orders
//! ([`Goal::Regroup`]).
//!
//! **Who pauses.** `_AICheckParentExists` (@0x8926a-0x89501) writes goal
//! 0x13 to every follower of a lost lead but the successor, its substate
//! (+0xc8ca) kept, while its warp stage +0x50 is strictly below 0.0
//! (@0x8949d-0x894a6), and to a disabled follower with no intact fellow,
//! substate 0 (@0x894b1-0x894f3). The traffic's succession pass writes it
//! ([`succession`](crate::traffic::succession)).
//!
//! **Which substates reach it.** Only sub 0, or sub 4 or 0xd on the jump
//! path, on any data. The check runs for an escort alone (AI type above 4
//! with a parent, @0x440d8, @0x440f4), which rests at +0x50 = 0: every
//! negative write leaves one of those substates by the time the check
//! reads it.
//!
//! - `_EscortAI`'s Defend, Attack and formation writes (@0x83c70, @0x83d67,
//!   @0x83ead) leave goal 4 or 0xa with +0x4c -1.0, so the high level's
//!   hold zeroes +0x50 in the same dispatch (@0x8d513-0x8d52e).
//! - Its lost parent and the check's own writes (@0x83974, @0x8937c,
//!   @0x894ec) leave no parent: no longer a follower.
//! - Sub 0xd's give-up (@0x85ba5) runs after the hold and keeps sub 0xd,
//!   or, a high level on, goal 0, sub 0 (@0x8df31-0x8df49, @0x8deb9-0x8ded1).
//! - The jump path's cancels (@0x9011e, @0x9019c) write goal 0, sub 0, and
//!   last past the hold only under a wait, in which no high level writes
//!   a substate.
//! - The emerge (@0x89555) writes goal 0x15, sub 0 under a wait that skips
//!   the hold; a hit there keeps sub 0 (@0x89d3e).
//! - The disabled cancel (@0x8fce9) stays only on the jump path, in sub 4
//!   or 0xd: off it the reset drops the parent.
//! - A ship gliding in (-999.0) takes the write, overwritten in the same
//!   frame by the glide path (@0x8fcf5-0x8fd18): no pause.
//!
//! So subs 1, 9, 0xb and 0xc, a fight's and goal 5's never reach it.
//!
//! **What it flies** ([`Kept`]). `_AIDispatch` draws its release first
//! (@0x8fe73-0x8febb, [`Traffic::release_regroups`](crate::traffic::Traffic::release_regroups));
//! held, it skips the disabled reset, the type think and
//! `_HighLevelAIHandler` (@0x9002d → @0x900c6 → @0x901ea): only
//! `_LowLevelAIHandler` runs, flying the substate it kept. The engine has
//! no substate apart from the goal, so the goal keeps one in [`Kept`]
//! ([`Kept::of`]), and the fly half flies [`Goal::flown`] in its place
//! ([`Npc::flown_goal`]):
//!
//! - sub 0 moves on at its velocity with no thrust or turn (@0x8521e-0x85236,
//!   [`Kept::Coast`]), and fires `_AIEscortFireUnprovoked`;
//! - sub 0xd follows its new lead's jump on the jump path, its release
//!   undrawn ([`Kept::FollowsJump`]). Under a lead at +0x50 ≤ 1.0 it takes
//!   the lead's whole heading and velocity, `_AIMaintainFormation`
//!   (ship, 0) and the lead's glow, with no thrust (@0x859df-0x85a4a);
//! - sub 4 turns outward on its own jump, on the jump path too, and
//!   never leaves: the hold, with goal 0x13 at its head, zeroes the stage
//!   it starts each frame (@0x8585b-0x859c5, @0x8d52e), and
//!   `_AIIsShipJumping` reads goal 0x13, so the warp-out zeroes or restamps
//!   it (@0x34a27, @0x34a31) ([`Kept::OwnJump`]).
//!
//! Subs 4 and 0xd fire nothing: neither block calls an `_AIFire*`, and the
//! low level clears +0xba (@0x851f6).
//!
//! **Its readers.** Besides the pause, only `_AIIssueEscortOrders` reads
//! goal 0x13 (@0x89aa9, exempt from the all-Return override,
//! [`escort_orders`](crate::ai::escort_orders)) and the debug
//! abbreviations (@0x81654, @0x817e0). Every other reader of the goal
//! leaves 0x13 to its default: it is no attack, no fight, not in form and
//! not spared as a threat.
//!
//! Nothing here plays a sound or draws a random number.

use crate::ai::Goal;
use crate::traffic::npc::Npc;
use crate::traffic::warp_out::WarpGoal;

/// The substate a ship in the regroup pause keeps flying (see the module
/// docs).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kept {
    /// Sub 0: it moves on at its velocity with no thrust or turn.
    Coast,
    /// Sub 0xd, its [`WarpGoal::Follow`] kept: the jump path flies it.
    FollowsJump,
    /// Sub 4, its [`WarpGoal::Own`] kept: the jump path flies it.
    OwnJump,
}

impl Kept {
    /// The substate `npc` keeps as it pauses (see the module docs): a ship
    /// already paused keeps its own; else its jump's sub 4 or 0xd, or sub
    /// 0 off a jump, whatever its goal.
    #[must_use]
    pub fn of(npc: &Npc) -> Self {
        if let Goal::Regroup { kept } = npc.goal {
            return kept;
        }
        match npc.warp.goal {
            Some(WarpGoal::Follow(_)) => Self::FollowsJump,
            Some(WarpGoal::Own) => Self::OwnJump,
            None => Self::Coast,
        }
    }
}

impl Goal {
    /// The goal whose flight a ship in this goal flies: for the regroup
    /// pause its kept substate's (a coast or a kept follow as a hold, its
    /// own jump as a jump out); any other goal flies itself.
    #[must_use]
    pub fn flown(self) -> Self {
        match self {
            Self::Regroup {
                kept: Kept::Coast | Kept::FollowsJump,
            } => Self::Hold,
            Self::Regroup {
                kept: Kept::OwnJump,
            } => Self::JumpOut,
            goal => goal,
        }
    }
}

impl Npc {
    /// The goal its flight flies ([`Goal::flown`]).
    #[must_use]
    pub fn flown_goal(&self) -> Goal {
        self.goal.flown()
    }

    /// Runs `fly` on it with its goal the one it flies
    /// ([`Npc::flown_goal`]), so every read of the flight finds its kept
    /// substate's goal, then takes its own goal back, unless the flight
    /// wrote another: what the fly half does for each ship
    /// ([`fly_pass`](crate::traffic::fly_pass)).
    pub fn fly_as_flown<R>(&mut self, fly: impl FnOnce(&mut Self) -> R) -> R {
        let goal = self.goal;
        let flown = self.flown_goal();
        self.goal = flown;
        let flew = fly(self);
        if self.goal == flown {
            self.goal = goal;
        }
        flew
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::Help;
    use crate::catalog::StellarId;
    use crate::combat::ShipRef;
    use crate::stats::ShipStats;
    use crate::traffic::fixture::npc;
    use crate::traffic::npc::NpcId;

    const T: ShipRef = ShipRef::Npc(NpcId(7));
    const LEAD: NpcId = NpcId(3);
    const ALL: [Kept; 3] = [Kept::Coast, Kept::FollowsJump, Kept::OwnJump];

    /// A ship in `goal`, on the jump `warp` says.
    fn in_goal(goal: Goal, warp: Option<WarpGoal>) -> Npc {
        let mut ship = npc(1, ShipStats::default());
        ship.goal = goal;
        ship.warp.goal = warp;
        ship
    }

    #[test]
    fn off_its_jump_a_ship_keeps_sub_0_whatever_its_goal() {
        let guard = Some(T);
        let plunder = Goal::Plunder {
            ship: T,
            hopeless: false,
        };
        for goal in [
            Goal::Follow { lead: LEAD, guard },
            Goal::Formation { guard },
            Goal::Attack(T),
            Goal::Snipe(T),
            Goal::Hold,
            Goal::Drift,
            Goal::Assist(Help::Repair),
            Goal::Assist(Help::Refuel),
            Goal::Idle,
            Goal::Wait,
            Goal::Land(StellarId(128)),
            Goal::EnterGate(StellarId(300)),
            Goal::JumpOut,
            Goal::Flee(T),
            Goal::Inspect(T),
            Goal::Dock(T),
            Goal::Mine,
            Goal::Scoop,
            plunder,
            Goal::CountWait,
        ] {
            assert_eq!(Kept::of(&in_goal(goal, None)), Kept::Coast, "{goal:?}");
        }
    }

    #[test]
    fn on_its_jump_a_ship_keeps_its_jumps_substate() {
        for goal in [Goal::Idle, Goal::Attack(T), Goal::JumpOut] {
            assert_eq!(
                Kept::of(&in_goal(goal, Some(WarpGoal::Own))),
                Kept::OwnJump,
                "{goal:?}"
            );
            assert_eq!(
                Kept::of(&in_goal(goal, Some(WarpGoal::Follow(T)))),
                Kept::FollowsJump,
                "{goal:?}"
            );
        }
    }

    #[test]
    fn a_paused_ship_keeps_its_own_whatever_its_warp_says() {
        for kept in ALL {
            let paused = Goal::Regroup { kept };
            for warp in [None, Some(WarpGoal::Own), Some(WarpGoal::Follow(T))] {
                assert_eq!(Kept::of(&in_goal(paused, warp)), kept, "{warp:?}");
            }
        }
    }

    #[test]
    fn a_paused_ship_flies_its_kept_substate() {
        for (kept, flown) in [
            (Kept::Coast, Goal::Hold),
            (Kept::FollowsJump, Goal::Hold),
            (Kept::OwnJump, Goal::JumpOut),
        ] {
            assert_eq!(Goal::Regroup { kept }.flown(), flown, "{kept:?}");
            let ship = in_goal(Goal::Regroup { kept }, None);
            assert_eq!(ship.flown_goal(), flown, "{kept:?}");
        }
        for goal in [Goal::Idle, Goal::Attack(T), Goal::Hold] {
            assert_eq!(goal.flown(), goal);
        }
    }

    #[test]
    fn a_flight_reads_the_flown_goal_and_leaves_the_pause_as_it_was() {
        let paused = Goal::Regroup {
            kept: Kept::OwnJump,
        };
        let mut ship = in_goal(paused, None);
        assert_eq!(ship.fly_as_flown(|ship| ship.goal), Goal::JumpOut);
        assert_eq!(ship.goal, paused);
        ship.fly_as_flown(|ship| ship.goal = Goal::Idle);
        assert_eq!(ship.goal, Goal::Idle, "a goal the flight wrote stands");
    }

    #[test]
    fn the_pause_is_no_attack_no_fight_not_in_form_and_not_spared_as_a_threat() {
        for kept in ALL {
            let goal = Goal::Regroup { kept };
            assert_eq!(goal.attacking(), None);
            assert!(!goal.fights());
            assert!(!goal.flies_in_form());
            assert!(!goal.spares_player_threat());
            assert!(!goal.scooping());
        }
    }
}
