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
//! - +0x50 below -900.0 (0xdda4c) takes the glide-in path and returns
//!   (@0x8fcf5-0x8fd7e): a ship gliding in from hyperspace, or bursting out
//!   of a gate past its first tick ([`Mode::JumpingIn`],
//!   [`Mode::LeavingGate`]);
//! - sub 4 or 0xd, its own jump or its leader's, takes the jump path
//!   (@0x8fd83-0x8fd9a → @0x900d5): any [`Warp::goal`](crate::traffic::warp_out::Warp).
//!
//! A ship waiting in its gate (goal 0x15) passes them: its skip comes
//! after the reset (@0x8ff18). So does a disabled ship. Nothing here plays
//! a sound or draws a random number.

use crate::traffic::npc::{Mode, Npc};
use crate::traffic::pod_flight;

/// Whether `npc`'s dispatch passes the gates (see the module docs).
#[must_use]
pub fn reaches_think(npc: &Npc) -> bool {
    passes_entry_gates(npc) && npc.warp.goal.is_none()
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

/// Whether `npc`'s dispatch passes the gates before the jump path's test:
/// not without AI, not an escape pod, not on the glide-in path.
fn passes_entry_gates(npc: &Npc) -> bool {
    !pod_flight::is_pod(npc)
        && !npc.no_ai
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
    use crate::stats::ShipStats;
    use crate::traffic::npc::NpcId;
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
