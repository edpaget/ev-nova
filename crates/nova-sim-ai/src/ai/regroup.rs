//! Goal 0x13, the regroup pause: what a ship of a fleet whose lead was
//! lost does until it takes up its new lead's orders
//! ([`Goal::Regroup`]).
//!
//! **Who pauses.** `_AICheckParentExists` (@0x8926a-0x89501) writes goal
//! 0x13 to every follower of a lost lead but the successor, its substate
//! (+0xc8ca) kept, while its warp stage +0x50 is below 0.0 (@0x8949d-0x894a6),
//! and to a disabled follower with no intact fellow, substate 0
//! (@0x894b1-0x894f3). The traffic's succession pass writes it
//! ([`succession`](crate::traffic::succession)). An escort rests at 0, so
//! it takes the pause only once its jump gave its stage up; its sub 4 or
//! 0xd is then kept, and the jump path runs on it every frame
//! ([`jump_with_leader`](crate::traffic::jump_with_leader)).
//!
//! **What it flies** ([`Kept`]). `_AIDispatch` draws its release first
//! (@0x8fe73-0x8febb, [`Traffic::release_regroups`](crate::traffic::Traffic::release_regroups)); held,
//! it skips the disabled reset, the type think and `_HighLevelAIHandler`
//! (@0x9002d → @0x900c6 → @0x901ea): only `_LowLevelAIHandler` runs, flying
//! the substate it kept with that substate's own fire. The engine has no
//! substate apart from the goal, so the goal keeps one in [`Kept`]
//! ([`Kept::of`]), and the fly half flies [`Goal::flown`] in its place
//! ([`Npc::flown_goal`]):
//!
//! - a formation substate (the original's subs 9, 0xb and 0xc of goal 0xa)
//!   keeps formation on its lead, now the successor, on the slot the
//!   formation pass gives it, firing its turret at its guard
//!   ([`Kept::Formation`]);
//! - a fight's substate (goal 4's) keeps flying its fight at its target
//!   ([`Kept::Attack`], [`Kept::Snipe`]), firing its passes but launching
//!   no fighter through the high level (@0x8e6fa);
//! - sub 0 (a hold, a drift or an assist, which fly with no thrust or
//!   turn) coasts ([`Kept::Coast`]);
//! - sub 1 and anything else (a wait, a count wait, an idle, a dock) stops
//!   ([`Kept::Stop`]).
//!
//! Every kept substate but a fight's fires `_AIEscortFireUnprovoked`
//! (subs 0, 1, 9, 0xb and 0xc, AI type above 4 only).
//!
//! **Its readers.** Besides the pause, only `_AIIssueEscortOrders` reads
//! goal 0x13 (@0x89aa9, exempt from the all-Return override,
//! [`escort_orders`](crate::ai::escort_orders)) and the debug
//! abbreviations (@0x81654, @0x817e0). Every other reader of the goal
//! leaves 0x13 to its default: it is no attack, no fight, not in form and
//! not spared as a threat.
//!
//! The kept substates where the engine's flight differs from the
//! original's (no re-pick; subs 9 and 0xb steering at the lost lead's
//! +0x6c; a returning fighter's docking at its lost carrier) are
//! rdm:phase/escort-fleet-fidelity-followups/phase-9-left-out-of-leader-succession's.
//! Nothing here plays a sound or draws a random number.

use crate::ai::Goal;
use crate::combat::ShipRef;
use crate::traffic::npc::{Npc, NpcId};

/// The substate a ship in the regroup pause keeps flying (see the module
/// docs).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kept {
    /// Sub 0: it moves on at its velocity with no thrust or turn.
    Coast,
    /// Sub 1: it stops.
    Stop,
    /// Goal 0xa's substates: it keeps formation on its lead, firing its
    /// turret at `guard`.
    Formation {
        /// The ship its turrets fire at, if any.
        guard: Option<ShipRef>,
    },
    /// Goal 4's fight at this ship.
    Attack(ShipRef),
    /// Goal 4's stand-off fight at this ship.
    Snipe(ShipRef),
}

impl Kept {
    /// The substate a ship in `goal` keeps as it pauses (see the module
    /// docs): a formation's, a fight's, a coast for a hold, a drift or an
    /// assist, a stop for the rest; a ship already paused keeps its own.
    #[must_use]
    pub fn of(goal: Goal) -> Self {
        match goal {
            Goal::Follow { guard, .. } | Goal::Formation { guard } => Self::Formation { guard },
            Goal::Attack(ship) => Self::Attack(ship),
            Goal::Snipe(ship) => Self::Snipe(ship),
            Goal::Hold | Goal::Drift | Goal::Assist(_) => Self::Coast,
            Goal::Regroup { kept } => kept,
            Goal::Idle
            | Goal::Wait
            | Goal::Land(_)
            | Goal::EnterGate(_)
            | Goal::JumpOut
            | Goal::Flee(_)
            | Goal::Inspect(_)
            | Goal::Dock(_)
            // No miner pauses: only a led ship regroups, and a led ship
            // never thinks as a miner.
            | Goal::Mine
            | Goal::Scoop
            | Goal::Plunder { .. }
            | Goal::CountWait => Self::Stop,
        }
    }
}

impl Goal {
    /// The goal whose flight a ship in this goal, led by `leader`, flies:
    /// for the regroup pause its kept substate's (a coast as a hold, a
    /// stop as a wait, a formation on `leader`, or a hold with none, as
    /// goal 0xa with no leader flies, a fight as itself); any other goal
    /// flies itself.
    #[must_use]
    pub fn flown(self, leader: Option<NpcId>) -> Self {
        let Self::Regroup { kept } = self else {
            return self;
        };
        match kept {
            Kept::Coast => Self::Hold,
            Kept::Stop => Self::Wait,
            Kept::Formation { guard } => {
                leader.map_or(Self::Hold, |lead| Self::Follow { lead, guard })
            }
            Kept::Attack(ship) => Self::Attack(ship),
            Kept::Snipe(ship) => Self::Snipe(ship),
        }
    }
}

impl Npc {
    /// The goal its flight flies ([`Goal::flown`] under its leader).
    #[must_use]
    pub fn flown_goal(&self) -> Goal {
        self.goal.flown(self.leader)
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

    const T: ShipRef = ShipRef::Npc(NpcId(7));
    const LEAD: NpcId = NpcId(3);

    #[test]
    fn each_goal_keeps_the_substate_its_flight_had() {
        let guard = Some(T);
        let plunder = Goal::Plunder {
            ship: T,
            hopeless: false,
        };
        for (goal, kept) in [
            (
                Goal::Follow { lead: LEAD, guard },
                Kept::Formation { guard },
            ),
            (Goal::Formation { guard }, Kept::Formation { guard }),
            (Goal::Attack(T), Kept::Attack(T)),
            (Goal::Snipe(T), Kept::Snipe(T)),
            (Goal::Hold, Kept::Coast),
            (Goal::Drift, Kept::Coast),
            (Goal::Assist(Help::Repair), Kept::Coast),
            (Goal::Assist(Help::Refuel), Kept::Coast),
            (Goal::Idle, Kept::Stop),
            (Goal::Wait, Kept::Stop),
            (Goal::Land(StellarId(128)), Kept::Stop),
            (Goal::EnterGate(StellarId(300)), Kept::Stop),
            (Goal::JumpOut, Kept::Stop),
            (Goal::Flee(T), Kept::Stop),
            (Goal::Inspect(T), Kept::Stop),
            (Goal::Dock(T), Kept::Stop),
            (Goal::Mine, Kept::Stop),
            (plunder, Kept::Stop),
            (Goal::CountWait, Kept::Stop),
            (
                Goal::Regroup {
                    kept: Kept::Attack(T),
                },
                Kept::Attack(T),
            ),
        ] {
            assert_eq!(Kept::of(goal), kept, "{goal:?}");
        }
    }

    #[test]
    fn a_paused_ship_flies_its_kept_substate() {
        let regroup = |kept| Goal::Regroup { kept };
        for (kept, led, alone) in [
            (Kept::Coast, Goal::Hold, Goal::Hold),
            (Kept::Stop, Goal::Wait, Goal::Wait),
            (
                Kept::Formation { guard: Some(T) },
                Goal::Follow {
                    lead: LEAD,
                    guard: Some(T),
                },
                Goal::Hold,
            ),
            (Kept::Attack(T), Goal::Attack(T), Goal::Attack(T)),
            (Kept::Snipe(T), Goal::Snipe(T), Goal::Snipe(T)),
        ] {
            assert_eq!(regroup(kept).flown(Some(LEAD)), led, "{kept:?}");
            assert_eq!(regroup(kept).flown(None), alone, "{kept:?}");
        }
        for goal in [Goal::Idle, Goal::Attack(T), Goal::Hold] {
            assert_eq!(goal.flown(Some(LEAD)), goal);
        }
    }

    #[test]
    fn an_npc_flies_its_goal_under_its_leader() {
        let mut npc = crate::traffic::fixture::npc(1, crate::stats::ShipStats::default());
        npc.goal = Goal::Regroup {
            kept: Kept::Formation { guard: None },
        };
        assert_eq!(npc.flown_goal(), Goal::Hold);
        npc.leader = Some(LEAD);
        assert_eq!(
            npc.flown_goal(),
            Goal::Follow {
                lead: LEAD,
                guard: None
            }
        );
    }

    #[test]
    fn a_flight_reads_the_flown_goal_and_leaves_the_pause_as_it_was() {
        let mut npc = crate::traffic::fixture::npc(1, crate::stats::ShipStats::default());
        npc.leader = Some(LEAD);
        let paused = Goal::Regroup {
            kept: Kept::Formation { guard: Some(T) },
        };
        npc.goal = paused;
        let read = npc.fly_as_flown(|npc| npc.goal);
        assert_eq!(
            read,
            Goal::Follow {
                lead: LEAD,
                guard: Some(T)
            }
        );
        assert_eq!(npc.goal, paused);
        npc.fly_as_flown(|npc| npc.goal = Goal::JumpOut);
        assert_eq!(npc.goal, Goal::JumpOut, "a goal the flight wrote stands");
    }

    #[test]
    fn the_pause_is_no_attack_no_fight_not_in_form_and_not_spared_as_a_threat() {
        for kept in [
            Kept::Attack(T),
            Kept::Formation { guard: Some(T) },
            Kept::Coast,
        ] {
            let goal = Goal::Regroup { kept };
            assert_eq!(goal.attacking(), None);
            assert!(!goal.fights());
            assert!(!goal.flies_in_form());
            assert!(!goal.spares_player_threat());
            assert!(!goal.scooping());
        }
    }
}
