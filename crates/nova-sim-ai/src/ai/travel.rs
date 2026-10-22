//! A pirate's travel stellar: the original's +0x6c, which
//! `_PirateWarshipAI`'s copy of the idle block reads (@0x8c472-0x8c477 in
//! the `EV Nova` executable, [`idle_block::pirate_idle`]): while it is -1 a
//! pirate with nothing to fight picks a stellar; while it is set it leaves
//! or waits without picking. The think writes nothing to it first, so a
//! pirate keeps it through its fights and the drops that end them.
//!
//! The engine keeps it as [`Npc::travel_set`], whether it is set, and a
//! decision says how its think writes it ([`Decision::travel`],
//! [`TravelWrite`]); the traffic [`take`]s the write as it takes the goal.
//! It reads as set ([`is_set`]) while the flag is, while the ship is bound
//! for a stellar or a gate (goals 1 and 0x14 carry it), or while it comes
//! out of a gate ([`Npc::emerges_from`], the emerge's write @0x8954a).
//!
//! Its writes a pirate passes through:
//!
//! - **Set**: the pick (@0x8c495; [`take`] sets it as a decision leaves a
//!   `Land` or `EnterGate` goal), and a plunder of a disabled ship, the
//!   victim's slot (`_HighLevelAIHandler` @0x8e8c9).
//! - **Cleared** (-1): a spawn (`_InitObjects` @0x1c3c1, `_RandomShipSpawn`
//!   @0x3c444, `_GenericRandomShipSpawn` @0x3c9e2); the count wait
//!   (@0x8c70d, see [`board_wait`](crate::traffic::board_wait)); the plunder
//!   (@0x8c7ae); the dry drop (@0x8c887); a plunder of an intact ship, on
//!   every frame (@0x8ea50); the lost-sight search of a ship above the
//!   traders (@0x8d833, see [`lost_sight`](super::lost_sight)); the end of a
//!   refuel (@0x8f6b6-0x8f6ce) or a repair (@0x8f717-0x8f732,
//!   `_LowLevelAIHandler` @0x86f09-0x86f31) and a plunder's give-up
//!   (@0x8e88f-0x8e8b9); a capture's drop of the ships that target its
//!   victim (`_AIPlunderShipContents` @0x84872-0x8488d); and the player's
//!   boarding's or capture's drop of the ships after its ship
//!   (`_AIShipBoardedByPlayer` @0x897e6-0x8980f, `_AIShipCapturedByPlayer`
//!   @0x896c5-0x896fa); and every victim the player's hit turns on it
//!   (`_AIMakeShipAttackPlayer` @0x89d47, from `_DamageShip` @0x3be69;
//!   the traffic's reaction), an assister included; and the end of a
//!   held ship's burst (`_AIMakeShipStopJumping` @0x82eaa, see
//!   [`held_burst`](crate::traffic::held_burst)). The
//!   `_AIMakeShipStopRefuelling` that follows it (@0x3bec7, its -1
//!   @0x833ba) never acts, as it needs goal 9 or 0xf and finds goal 4.
//! - **Kept**: the head drop (@0x8c32a-0x8c330), the disabled-warship drop
//!   (@0x8c839-0x8c842), an attack, and every goal the think keeps.
//!
//! The emerge's gate is folded in only when read: the burst start's -1
//! (@0x8dd93) clears [`Npc::emerges_from`] already, and a ship emerges only
//! as it spawns, when the flag is clear.
//!
//! Not written, as each would clear a flag already clear: the count wait's
//! end (@0x8d486), the drift (@0x8eb8b) and a plunder whose target has gone
//! (@0x8eb45). Every path into them has cleared it: the count wait, the
//! plunder, or a plunder's every-frame write followed by the head drop.
//!
//! Nothing here plays a sound. +0x6c is 16-bit there; not replicated.
//!
//! Left out (see the phase body of
//! rdm:phase/combat-ai-fidelity/phase-20-left-out-of-phase-12): an NPC
//! escort's +0x6c, its leader's slot, and `_AIDispatch`'s disabled-ship
//! reset (@0x8feda-0x8ff13), which
//! rdm:phase/combat-ai-fidelity-cross-roadmap/phase-4-escort-travel-stellar-into-pirate-think
//! owns; and the session's writes that replace a `Land` goal without a decision, which
//! [`take`] never sees.

use crate::ai::Goal;
#[cfg(doc)]
use crate::ai::{Decision, idle_block};
use crate::traffic::npc::Npc;

/// How a think writes its ship's travel stellar ([`Decision::travel`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TravelWrite {
    /// It writes none: the flag is kept, or set by a stellar goal (see
    /// [`take`]).
    #[default]
    Keep,
    /// It writes a slot: the flag is set.
    Set,
    /// It writes -1: the flag is cleared.
    Clear,
}

/// Whether `npc`'s travel stellar (+0x6c) is set, so a pirate's idle block
/// picks none (see the module docs).
#[must_use]
pub fn is_set(npc: &Npc) -> bool {
    npc.travel_set || stellar_goal(npc.goal) || npc.emerges_from.is_some()
}

/// Takes `write` into `npc`'s flag as it takes `goal`, before its goal is
/// replaced: set or cleared as `write` says, or else kept, and set by a
/// stellar goal it leaves or takes.
pub fn take(npc: &mut Npc, write: TravelWrite, goal: Goal) {
    npc.travel_set = match write {
        TravelWrite::Set => true,
        TravelWrite::Clear => false,
        TravelWrite::Keep => npc.travel_set || stellar_goal(npc.goal) || stellar_goal(goal),
    };
}

/// Whether `goal` carries a stellar in +0x6c: a landing (goal 1) or a gate
/// entered (0x14).
fn stellar_goal(goal: Goal) -> bool {
    matches!(goal, Goal::Land(_) | Goal::EnterGate(_))
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;
    use crate::ai::{Behaviour, Decision, Surroundings};
    use crate::catalog::StellarId;
    use crate::chance::Chance;
    use crate::combat::ShipRef;
    use crate::stats::ShipStats;
    use crate::traffic::Traffic;
    use crate::traffic::fixture::npc;
    use crate::traffic::npc::NpcId;
    use nova_sim_base::fixture::{Draws, FAST};

    const LAND: Goal = Goal::Land(StellarId(128));
    const GATE: Goal = Goal::EnterGate(StellarId(129));

    fn ship() -> Npc {
        npc(1, ShipStats::new(FAST, &[]))
    }

    fn holding(goal: Goal, travel_set: bool) -> Npc {
        Npc {
            goal,
            travel_set,
            ..ship()
        }
    }

    /// The flag `npc` holds once it takes `write` with `goal`.
    fn taken(mut npc: Npc, write: TravelWrite, goal: Goal) -> bool {
        take(&mut npc, write, goal);
        npc.travel_set
    }

    #[test]
    fn keep_holds_the_flag_and_a_stellar_goal_sets_it() {
        let keep = TravelWrite::Keep;
        assert!(!taken(holding(Goal::Idle, false), keep, Goal::JumpOut));
        assert!(taken(holding(Goal::Idle, true), keep, Goal::JumpOut));
        assert!(taken(holding(LAND, false), keep, Goal::Idle), "left");
        assert!(taken(holding(GATE, false), keep, Goal::Wait), "left");
        assert!(taken(holding(Goal::Idle, false), keep, LAND), "taken");
        assert!(taken(holding(Goal::Wait, false), keep, GATE), "taken");
    }

    #[test]
    fn set_and_clear_overrule_the_goal() {
        assert!(taken(
            holding(Goal::Idle, false),
            TravelWrite::Set,
            Goal::Idle
        ));
        assert!(!taken(holding(LAND, true), TravelWrite::Clear, LAND));
        assert!(!taken(holding(Goal::Idle, true), TravelWrite::Clear, GATE));
    }

    #[test]
    fn the_emerges_gate_and_a_stellar_goal_read_as_set() {
        assert!(!is_set(&ship()));
        assert!(is_set(&holding(Goal::Idle, true)));
        assert!(is_set(&holding(LAND, false)));
        assert!(is_set(&holding(GATE, false)));
        let emerging = Npc {
            emerges_from: Some(StellarId(129)),
            ..ship()
        };
        assert!(is_set(&emerging));
        assert!(!is_set(&holding(Goal::Wait, false)));
        assert!(!is_set(&holding(Goal::JumpOut, false)));
    }

    /// Gives its queued decisions in turn, then idles.
    #[derive(Debug)]
    struct Scripted(RefCell<Vec<Decision>>);

    impl Scripted {
        fn of(decisions: &[Decision]) -> Self {
            Self(RefCell::new(decisions.iter().rev().copied().collect()))
        }
    }

    impl Behaviour for Scripted {
        fn decide(&self, npc: &Npc, around: &Surroundings, chance: &mut dyn Chance) -> Goal {
            self.decision(npc, around, chance).goal
        }

        fn decision(&self, _: &Npc, _: &Surroundings, _: &mut dyn Chance) -> Decision {
            self.0.borrow_mut().pop().unwrap_or_default()
        }
    }

    fn of(goal: Goal) -> Decision {
        Decision {
            goal,
            ..Decision::default()
        }
    }

    /// The flag after each of `decisions`, one a tick, of a pirate in the
    /// traffic.
    fn flags(decisions: &[Decision]) -> Vec<bool> {
        let mut traffic = Traffic::with_interval(1);
        traffic.add_npc(ship());
        let script = Scripted::of(decisions);
        decisions
            .iter()
            .map(|_| {
                traffic.tick(&script, &[], &mut Draws::of(&[0; 8]));
                traffic.npcs()[0].travel_set
            })
            .collect()
    }

    #[test]
    fn in_traffic_a_pirate_bound_for_a_stellar_keeps_it_set_through_a_fight() {
        let fight = of(Goal::Attack(ShipRef::Npc(NpcId(2))));
        assert_eq!(
            flags(&[of(LAND), fight, of(Goal::Idle)]),
            [true, true, true],
            "set by the landing, kept through the fight and its drop"
        );
        assert_eq!(flags(&[fight, of(Goal::Idle)]), [false, false], "no pick");
        let cleared = Decision {
            travel: TravelWrite::Clear,
            ..of(Goal::Idle)
        };
        assert_eq!(flags(&[of(LAND), fight, cleared]), [true, true, false]);
    }
}
