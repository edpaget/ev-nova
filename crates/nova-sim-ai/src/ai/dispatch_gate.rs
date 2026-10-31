//! `_AIDispatch`'s gates (@0x8fb52): whether a ship's dispatch reaches the
//! part of it past the escape pod's return, the glide-in path and the jump
//! path, where a fleet's leader issues its orders (@0x8fdb3,
//! [`escort_orders`](crate::ai::escort_orders)), the regroup pause draws
//! its release (@0x8fe73,
//! [`Traffic::release_regroups`](crate::traffic::Traffic::release_regroups))
//! and a disabled ship is reset (@0x8fece).
//!
//! In the dispatch's order:
//!
//! - `_DoPlayGameWork` dispatches only a ship with AI, of AI type above 0
//!   (@0x4427e, [`Npc::no_ai`]);
//! - an escape pod (class 0x2ff) flies its own block and returns
//!   (@0x8fb62-0x8fbba, [`pod_flight::is_pod`]);
//! - a `ShipBehav` 1 ship in the formation is led again (@0x8fc03-0x8fc22,
//!   [`led_again`]), which stops no one: the session does it each traffic
//!   tick (see `nova_sim`'s `dying_player_escorts` module);
//! - a formation parent's dispatch works its followers' places out
//!   (@0x8fcaf-0x8fcc3, [`reaches_formation_calc`]), so past those two
//!   gates alone: before the disabled test (@0x8fcc8) and the gates below;
//! - +0x50 below -900.0 (0xdda4c) takes the glide-in path and returns
//!   (@0x8fcf5-0x8fd7e): a ship gliding in from hyperspace, or bursting out
//!   of a gate past its first tick ([`Mode::JumpingIn`],
//!   [`Mode::LeavingGate`]);
//! - sub 4 or 0xd, its own jump or its leader's, takes the jump path
//!   (@0x8fd83-0x8fd9a → @0x900d5): any [`Warp::goal`](crate::traffic::warp_out::Warp).
//!
//! A ship waiting in its gate (goal 0x15) passes them: its skip comes
//! after the reset (@0x8ff18). So does a disabled ship. Past them,
//! `_EscortAI` is reached out of goals 0x15 and 0x16 alone
//! ([`reaches_escort_think`]). Nothing here plays a sound or draws a
//! random number.

use crate::traffic::ShipBehav;
use crate::traffic::npc::{Mode, Npc};
use crate::traffic::pod_flight;

/// Whether `npc`'s dispatch passes the gates (see the module docs).
#[must_use]
pub fn reaches_think(npc: &Npc) -> bool {
    passes_entry_gates(npc) && npc.warp.goal.is_none()
}

/// Whether `npc`'s dispatch reaches `_EscortAI`, which frees a ship whose
/// parent is dying (@0x83917-0x8397b): past the gates ([`reaches_think`]),
/// out of goal 0x15 (@0x8ff18, [`Npc::emerging`]) and out of goal 0x16
/// (@0x838ef, [`Npc::count_wait`]). Goals 9 and 0xf return there too, but
/// only under an NPC parent (@0x838e4-0x83911).
#[must_use]
pub fn reaches_escort_think(npc: &Npc) -> bool {
    reaches_think(npc) && !npc.emerging() && !npc.count_wait()
}

/// Whether the dispatch leads `npc` again each frame (@0x8fc03-0x8fc22): a
/// `ShipBehav` 1 ship in the formation. The re-lead also takes a ship not
/// disabled (@0x8fc0c), which its caller tests.
#[must_use]
pub fn led_again(npc: &Npc) -> bool {
    npc.mission
        .is_some_and(|mark| mark.behav == ShipBehav::EscortPlayer && mark.formation.is_some())
}

/// Whether `npc` is a mission escort no dispatch leads again ([`led_again`])
/// and whose think is `_EscortAI`'s: captured, of another `ShipBehav` than 1
/// or 2.
#[must_use]
pub fn not_led_again(npc: &Npc) -> bool {
    npc.mission_escort() && !led_again(npc) && !npc.attacks_stellars()
}

/// Whether `npc`'s dispatch takes the jump path (@0x8fd83-0x8fd9a →
/// @0x900d5): past the gates before it, on its own jump or its leader's
/// (sub 4 or 0xd). There it skips the leader's orders, the regroup draw,
/// the disabled reset and its type's think, and runs only the jump
/// cancels and the high and low levels
/// ([`jump_with_leader`](crate::traffic::jump_with_leader)).
#[must_use]
pub fn takes_jump_path(npc: &Npc) -> bool {
    passes_entry_gates(npc) && npc.warp.goal.is_some()
}

/// Whether `npc`'s dispatch reaches `_AICalcFormationPositions(ship, 0)`
/// (@0x8fcaf): not without AI, not an escape pod. A disabled or breaking-up
/// ship reaches it, as a ship gliding in does (see the module docs).
#[must_use]
pub fn reaches_formation_calc(npc: &Npc) -> bool {
    !npc.no_ai && !pod_flight::is_pod(npc)
}

/// Whether `npc`'s dispatch passes the gates before the jump path's test:
/// not without AI, not an escape pod, not on the glide-in path.
fn passes_entry_gates(npc: &Npc) -> bool {
    reaches_formation_calc(npc)
        && !matches!(
            npc.mode,
            Mode::JumpingIn { .. } | Mode::LeavingGate { tick: 1.. }
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat::ShipRef;
    use crate::combat::hull::Condition;
    use crate::escort::FormationSlot;
    use crate::stats::ShipStats;
    use crate::traffic::npc::{MissionMark, NpcId};
    use crate::traffic::warp_out::WarpGoal;

    fn ship() -> Npc {
        crate::traffic::fixture::npc(1, ShipStats::default())
    }

    #[test]
    fn a_ship_flying_disabled_or_waiting_in_its_gate_reaches_its_think() {
        assert!(reaches_think(&ship()));
        let mut disabled = ship();
        disabled.condition = Condition::Disabled;
        assert!(reaches_think(&disabled));
        for mode in [
            Mode::Emerging { ticks_left: 3 },
            Mode::HeldAtGate { ticks_left: 3 },
            Mode::Coasting { ticks_left: 3 },
            Mode::LeavingGate { tick: 0 },
            Mode::EnteringGate {
                gate: crate::catalog::StellarId(300),
                ticks_left: 3,
            },
        ] {
            assert!(reaches_think(&Npc { mode, ..ship() }), "{mode:?}");
        }
    }

    #[test]
    fn an_escape_pod_does_not() {
        let mut pod = ship();
        pod.ship = crate::combat::hull::ESCAPE_POD_SHIP;
        assert!(!reaches_think(&pod));
    }

    #[test]
    fn a_ship_without_ai_does_not() {
        let mut hulk = ship();
        hulk.no_ai = true;
        assert!(!reaches_think(&hulk));
    }

    #[test]
    fn a_ship_gliding_in_or_bursting_out_past_its_first_tick_does_not() {
        for mode in [Mode::JumpingIn { tick: 0 }, Mode::LeavingGate { tick: 1 }] {
            assert!(!reaches_think(&Npc { mode, ..ship() }), "{mode:?}");
        }
    }

    #[test]
    fn a_formation_calc_is_reached_past_the_ai_and_pod_gates_alone() {
        assert!(reaches_formation_calc(&ship()));
        let mut disabled = ship();
        disabled.condition = Condition::Disabled;
        assert!(reaches_formation_calc(&disabled));
        assert!(reaches_formation_calc(&Npc {
            mode: Mode::JumpingIn { tick: 0 },
            ..ship()
        }));
        let mut pod = ship();
        pod.ship = crate::combat::hull::ESCAPE_POD_SHIP;
        assert!(!reaches_formation_calc(&pod));
        let mut hulk = ship();
        hulk.no_ai = true;
        assert!(!reaches_formation_calc(&hulk));
    }

    #[test]
    fn a_ship_on_its_own_jump_or_its_leaders_does_not() {
        for goal in [WarpGoal::Own, WarpGoal::Follow(ShipRef::Npc(NpcId(9)))] {
            let mut jumping = ship();
            jumping.warp.goal = Some(goal);
            assert!(!reaches_think(&jumping), "{goal:?}");
            assert!(takes_jump_path(&jumping), "{goal:?}");
        }
        assert!(!takes_jump_path(&ship()), "no warp goal");
    }

    #[test]
    fn an_escort_think_is_reached_off_the_gate_the_glide_the_jump_and_a_count_wait() {
        assert!(reaches_escort_think(&ship()));
        for mode in [
            Mode::HeldAtGate { ticks_left: 3 },
            Mode::Coasting { ticks_left: 3 },
        ] {
            assert!(reaches_escort_think(&Npc { mode, ..ship() }), "{mode:?}");
        }
        for mode in [
            Mode::Emerging { ticks_left: 3 },
            Mode::LeavingGate { tick: 0 },
            Mode::LeavingGate { tick: 1 },
            Mode::JumpingIn { tick: 0 },
        ] {
            assert!(!reaches_escort_think(&Npc { mode, ..ship() }), "{mode:?}");
        }
        let mut jumping = ship();
        jumping.warp.goal = Some(WarpGoal::Own);
        assert!(!reaches_escort_think(&jumping));
        let counting = Npc {
            goal: crate::ai::Goal::CountWait,
            ..ship()
        };
        assert!(!reaches_escort_think(&counting));
    }

    /// A ship marked by a mission of `behav`, in the player's formation as
    /// `joined` says.
    fn marked(behav: ShipBehav, joined: bool) -> Npc {
        Npc {
            mission: Some(MissionMark {
                slot: 1,
                behav,
                formation: joined.then(FormationSlot::default),
                order: None,
            }),
            ..ship()
        }
    }

    #[test]
    fn the_dispatch_leads_again_only_a_behav_1_ship_in_the_formation() {
        assert!(led_again(&marked(ShipBehav::EscortPlayer, true)));
        assert!(
            !led_again(&marked(ShipBehav::EscortPlayer, false)),
            "not joined"
        );
        for behav in [
            ShipBehav::Own,
            ShipBehav::AttackPlayer,
            ShipBehav::AttackStellars,
        ] {
            assert!(!led_again(&marked(behav, true)), "{behav:?}");
        }
        assert!(!led_again(&ship()), "no mark");
    }

    #[test]
    fn a_mission_escort_let_go_keeps_its_mark_and_so_is_led_again() {
        let let_go = Npc {
            mission_let_go: true,
            ..marked(ShipBehav::EscortPlayer, true)
        };
        assert!(led_again(&let_go));
        assert!(!not_led_again(&let_go));
    }

    #[test]
    fn a_captured_mission_ship_is_not_led_again_but_a_behav_2_ship_keeps_its_own_think() {
        for behav in [ShipBehav::Own, ShipBehav::AttackPlayer] {
            assert!(not_led_again(&marked(behav, true)), "{behav:?}");
            assert!(
                !not_led_again(&marked(behav, false)),
                "{behav:?} not joined"
            );
        }
        assert!(!not_led_again(&marked(ShipBehav::EscortPlayer, true)));
        assert!(!not_led_again(&marked(ShipBehav::AttackStellars, true)));
        assert!(!not_led_again(&ship()), "no mark");
    }

    #[test]
    fn a_ship_failing_an_earlier_gate_takes_no_jump_path() {
        let jumping = |npc: Npc| Npc {
            warp: crate::traffic::warp_out::Warp {
                goal: Some(WarpGoal::Own),
                ..npc.warp
            },
            ..npc
        };
        let mut pod = jumping(ship());
        pod.ship = crate::combat::hull::ESCAPE_POD_SHIP;
        assert!(!takes_jump_path(&pod));
        let mut hulk = jumping(ship());
        hulk.no_ai = true;
        assert!(!takes_jump_path(&hulk));
        for mode in [Mode::JumpingIn { tick: 0 }, Mode::LeavingGate { tick: 1 }] {
            assert!(
                !takes_jump_path(&Npc {
                    mode,
                    ..jumping(ship())
                }),
                "{mode:?}"
            );
        }
        assert!(takes_jump_path(&Npc {
            mode: Mode::LeavingGate { tick: 0 },
            ..jumping(ship())
        }));
    }
}
