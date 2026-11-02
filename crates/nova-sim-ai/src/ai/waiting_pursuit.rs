//! A fight's low level under an AI wait: `_AIDispatch` calls
//! `_LowLevelAIHandler` (@0x901f4 in the `EV Nova` executable) after the
//! high level even when the high level returns at its head for the wait
//! (@0x8d4b9-0x8d4d2 → @0x8fb47, past the goal-0 tail's sub 0 @0x8fb09),
//! so the substate the last frame left (+0xc8ca) runs its block again with
//! no high level to rewrite it ([`under_wait`]).
//!
//! - **What shows.** `_HandleShip` skips steering, thrust and fire while
//!   +0x4c is above 0 (@0x34a54-0x34a72 → @0x3501f), so the fight
//!   substates' +0x30, +0x34 and +0x68 don't fly, and the next unwaited
//!   frame's low level rewrites +0x68 and +0x30 before they are used. Four
//!   writes do show: the +0xc8ca transitions, which choose the first
//!   unwaited frame's block (0x10 → 6 by the inertialess exit beyond 165 on
//!   an axis, @0x864c6-0x86513; 0x11 → 6, @0x86755-0x867b2; 6 → 0x10 or
//!   0x11, the joust and the rush entry; 7 → 0x11); the swarm branch's
//!   `_AIMaintainFormation(ship, 0)` nudge and glow copy in subs 5, 0x11
//!   and 7 (@0x85c87-0x85cbf, @0x8660b-0x8663d, @0x867e0-0x86818, the
//!   traffic's `swarm_low_level`); the turret pass's point defence; and the
//!   bays' launch, which [`fire::gated`](super::fire::gated) already
//!   passes.
//! - **The gates.** Each fight block needs +0x70 ≠ -1 and a ship not
//!   disabled (sub 0x10 @0x8641e-0x8642c; the others as `swarm_low_level`'s
//!   docs say). +0x70 is the goal's quarry, or the target it holds
//!   ([`Npc::slot_target`]); a target gone from the system runs nothing.
//!   The stored sub stands whatever the goal, as the head's return skips
//!   the goal-0 tail.
//! - **Which sub a wait holds** ([`held_sub`]): the engine keeps no
//!   +0xc8ca, so the sub is read from what the last frame left
//!   ([`Pursuit`]): a kept break-off or rush is held; with none kept, a
//!   last body of a break-off or a rush dropped back to dogfight (6), and
//!   any other fight body is its own sub. Sub 0 (the coast) and sub 0x17
//!   (a gate entry) show nothing: the ship holds still, and the engine
//!   keeps no frame for them. A ship running out (subs 1, 3 and 4) runs no
//!   body here; its non-fight subs' direct writes under a wait (sub 1's
//!   settle, sub 0x14, sub 0xe) are
//!   rdm:phase/combat-ai-fidelity-cross-roadmap-followups/phase-1-left-out-of-phase-19's.
//! - **When a fight sub is held.** Every +0x4c writer (`objdump`, every
//!   `mov ... 0x4c(%reg)`) was checked: the glide end and the count-wait
//!   entry write sub 0, the dock wait holds sub 0xf (@0x86fea), the
//!   capture's 150 (@0x8499c) goes to a victim the disabled reset cleared,
//!   and the landing stay (@0x8db3a, @0x8db5a), the warp follow (@0x85bc3,
//!   sub 0xd), the fleet spawn's `+= Rand(15)+5` (@0x4255e), the gate
//!   emerges (@0x67470, @0x675b7, @0x8954e) and the hit's cut to 20
//!   (@0x3be6e-0x3be7c, which only shortens a running wait) start no
//!   fight. So in stock play no wait begins holding sub 5, 6, 7, 0x10,
//!   0x11, 0x12 or 0xe, and the inertialess exit and the swarm branch under
//!   a wait show only from states a test constructs. The pursuit runs there
//!   all the same, as the original's low level does, so a constructed or
//!   future path behaves right.
//!
//! None of these blocks calls `PlaySound` or `_beepSnd`.

use crate::ai::fire::Manoeuvre;
use crate::ai::pursuit::{self, Kept, Pursuit, PursuitRules, Start};
use crate::ai::{Goal, Surroundings, handler_head};
use crate::chance::Chance;
use crate::traffic::npc::Npc;

/// The fight substate (+0xc8ca) `pursuit`, what the last frame left, holds
/// for the next frame (see the module docs): a kept break-off or rush; else
/// its last body's, a break-off or rush dropped back to dogfight. None for
/// no frame, or one with no body.
#[must_use]
pub(crate) fn held_sub(pursuit: &Pursuit) -> Option<Start> {
    match pursuit.kept {
        Some(Kept::BreakOff { heading }) => return Some(Start::BreakOff { heading }),
        Some(Kept::Rush) => return Some(Start::Sub(Manoeuvre::Rush)),
        None => {}
    }
    let body = pursuit.frame?.last()?;
    Some(Start::Sub(match body.manoeuvre {
        Manoeuvre::BreakOff | Manoeuvre::Rush => Manoeuvre::Dogfight,
        manoeuvre => manoeuvre,
    }))
}

/// `npc`'s pursuit among `around` under its AI wait, by `rules`, rolling on
/// `chance` (see the module docs): its low-level chain
/// ([`pursuit::chain`]) from the sub it holds ([`held_sub`]) at the ship
/// +0x70 names, the goal's quarry or else the target it holds
/// ([`Npc::slot_target`]). None when the high level isn't skipped by the
/// wait (goal 0x16's block comes before the head's wait exit,
/// @0x8d462-0x8d4ac), or with no fight sub held, no target or the target
/// gone: no block runs.
#[must_use]
pub(crate) fn under_wait(
    npc: &Npc,
    around: &Surroundings,
    rules: PursuitRules,
    chance: &mut (impl Chance + ?Sized),
) -> Option<Pursuit> {
    if npc.goal == Goal::CountWait || !handler_head::waits_out(npc, npc.goal) {
        return None;
    }
    let start = held_sub(&npc.pursuit)?;
    let ship = npc.goal.quarry().or_else(|| npc.slot_target())?;
    let at = around.state_of(ship)?;
    Some(pursuit::chain(
        npc,
        around,
        rules,
        chance,
        (ship, at),
        start,
    ))
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::ai::PlayerSide;
    use crate::ai::fixture::player;
    use crate::ai::pursuit::{Body, Frame, STAY_BOX};
    use crate::combat::ShipRef;
    use crate::geometry::Vec2;
    use crate::stats::ShipStats;
    use crate::traffic::npc::NpcId;
    use nova_sim_base::fixture::{FAST, Ranged};

    const AT_PLAYER: Goal = Goal::Attack(ShipRef::Player);
    /// Where the fighter is.
    const AT: Vec2 = Vec2::new(300.0, 200.0);

    /// A frame for `goal` of `bodies`.
    fn frame(goal: Goal, bodies: &[Manoeuvre]) -> Frame {
        let mut slots = [None; 2];
        for (slot, &manoeuvre) in slots.iter_mut().zip(bodies) {
            *slot = Some(Body {
                manoeuvre,
                heading: 0,
            });
        }
        Frame {
            goal,
            bodies: slots,
            speed_cap: None,
        }
    }

    fn left(kept: Option<Kept>, bodies: Option<&[Manoeuvre]>) -> Pursuit {
        Pursuit {
            kept,
            frame: bodies.map(|bodies| frame(AT_PLAYER, bodies)),
        }
    }

    #[test]
    fn a_kept_break_off_or_rush_is_held() {
        let off = Kept::BreakOff { heading: 135 };
        assert_eq!(
            held_sub(&left(Some(off), Some(&[Manoeuvre::Dogfight]))),
            Some(Start::BreakOff { heading: 135 })
        );
        assert_eq!(
            held_sub(&left(Some(Kept::Rush), None)),
            Some(Start::Sub(Manoeuvre::Rush))
        );
    }

    #[test]
    fn a_last_break_off_or_rush_with_none_kept_dropped_back_to_dogfight() {
        for body in [Manoeuvre::BreakOff, Manoeuvre::Rush] {
            assert_eq!(
                held_sub(&left(None, Some(&[Manoeuvre::Dogfight, body]))),
                Some(Start::Sub(Manoeuvre::Dogfight)),
                "{body:?}"
            );
        }
    }

    #[test]
    fn any_other_last_body_is_its_own_sub() {
        for body in [
            Manoeuvre::Dogfight,
            Manoeuvre::Approach,
            Manoeuvre::Flee,
            Manoeuvre::Swarm,
            Manoeuvre::Snipe,
        ] {
            assert_eq!(
                held_sub(&left(None, Some(&[body]))),
                Some(Start::Sub(body)),
                "{body:?}"
            );
        }
    }

    #[test]
    fn no_frame_or_a_frame_of_no_body_holds_no_fight_sub() {
        assert_eq!(held_sub(&left(None, None)), None);
        assert_eq!(held_sub(&left(None, Some(&[]))), None, "running out");
    }

    /// A fighter at [`AT`] on `goal`, facing up, its AI wait running,
    /// holding `kept` from a frame of `bodies`, inertialess as `raven`
    /// says.
    fn waiting(goal: Goal, kept: Option<Kept>, bodies: &[Manoeuvre], raven: bool) -> Npc {
        let fields = crate::handling::ShipFields {
            flags2: if raven {
                crate::stats::INERTIALESS_HULL
            } else {
                0
            },
            ..FAST
        };
        let mut npc = crate::traffic::fixture::npc(1, ShipStats::new(fields, &[]));
        npc.goal = goal;
        npc.berth = 2;
        npc.state.position = AT;
        npc.ai_wait = 50;
        npc.pursuit = Pursuit {
            kept,
            frame: Some(frame(goal, bodies)),
        };
        npc
    }

    /// `npc`'s pursuit under its wait at the player `off` from [`AT`].
    fn run_at_player(npc: &Npc, off: Vec2) -> Option<Pursuit> {
        let npcs = [npc.clone()];
        let side: PlayerSide = player(AT.x + off.x, AT.y + off.y);
        let around = Surroundings {
            player: Some(side),
            ..Surroundings::new(&[], &npcs)
        };
        under_wait(npc, &around, PursuitRules::default(), &mut Ranged::of(&[]))
    }

    fn bodies(pursuit: &Pursuit) -> Vec<Manoeuvre> {
        pursuit
            .frame
            .map(|frame| frame.iter().map(|body| body.manoeuvre).collect())
            .unwrap_or_default()
    }

    const BREAKING: Option<Kept> = Some(Kept::BreakOff { heading: 135 });

    #[test]
    fn an_inertialess_ship_breaking_off_under_a_wait_drops_back_to_dogfight_beyond_165() {
        // @0x864c6-0x86513: the high level, which would keep 0x10 only
        // inside 165.0 (@0x8e42e-0x8e473), returned at its head.
        let raven = waiting(AT_PLAYER, BREAKING, &[Manoeuvre::BreakOff], true);
        let left = run_at_player(&raven, Vec2::new(200.0, 0.0)).expect("its block runs");
        assert_eq!(left.kept, None, "sub 6");
        assert_eq!(bodies(&left), [Manoeuvre::BreakOff]);
        let near = run_at_player(&raven, Vec2::new(160.0, 160.0)).expect("its block runs");
        assert_eq!(near.kept, BREAKING, "inside 165 on both axes");
        // An inertial one stays until it faces its break heading.
        let inertial = waiting(AT_PLAYER, BREAKING, &[Manoeuvre::BreakOff], false);
        let kept = run_at_player(&inertial, Vec2::new(200.0, 0.0)).expect("its block runs");
        assert_eq!(kept.kept, BREAKING, "facing up, not 135");
    }

    #[test]
    fn a_rush_held_under_a_wait_drops_back_inside_the_stay_box() {
        let rushing = waiting(AT_PLAYER, Some(Kept::Rush), &[Manoeuvre::Rush], false);
        let near = f32::from(u8::try_from(STAY_BOX - 5).expect("small"));
        let dropped = run_at_player(&rushing, Vec2::new(near, near)).expect("its block runs");
        assert_eq!(bodies(&dropped), [Manoeuvre::Rush]);
        assert_eq!(dropped.kept, None, "sub 6 (@0x86755-0x8679c)");
    }

    #[test]
    fn the_held_sub_runs_whatever_the_goal_at_the_target_it_holds() {
        // The head's return skips the goal-0 tail's sub 0 (@0x8fb09).
        let mut idle = waiting(Goal::Idle, BREAKING, &[Manoeuvre::BreakOff], true);
        idle.target = Some(ShipRef::Player);
        let left = run_at_player(&idle, Vec2::new(200.0, 0.0)).expect("its block runs");
        assert_eq!(left.kept, None);
        assert_eq!(left.frame.map(|frame| frame.goal), Some(Goal::Idle));
    }

    #[test]
    fn no_held_sub_no_target_or_no_wait_runs_nothing() {
        let unframed = Npc {
            pursuit: Pursuit::default(),
            ..waiting(AT_PLAYER, None, &[], true)
        };
        assert_eq!(run_at_player(&unframed, Vec2::new(200.0, 0.0)), None);
        let untargeted = waiting(Goal::Idle, BREAKING, &[Manoeuvre::BreakOff], true);
        assert_eq!(run_at_player(&untargeted, Vec2::new(200.0, 0.0)), None);
        let mut gone = waiting(AT_PLAYER, BREAKING, &[Manoeuvre::BreakOff], true);
        gone.goal = Goal::Attack(ShipRef::Npc(NpcId(9)));
        assert_eq!(run_at_player(&gone, Vec2::new(200.0, 0.0)), None);
        let unwaited = Npc {
            ai_wait: 0,
            ..waiting(AT_PLAYER, BREAKING, &[Manoeuvre::BreakOff], true)
        };
        assert_eq!(
            run_at_player(&unwaited, Vec2::new(200.0, 0.0)),
            None,
            "its high level runs"
        );
        let counting = Npc {
            goal: Goal::CountWait,
            target: Some(ShipRef::Player),
            ..waiting(AT_PLAYER, BREAKING, &[Manoeuvre::BreakOff], true)
        };
        assert_eq!(
            run_at_player(&counting, Vec2::new(200.0, 0.0)),
            None,
            "goal 0x16's block comes first (@0x8d462)"
        );
        let held = Npc {
            goal: Goal::Formation { guard: None },
            target: Some(ShipRef::Player),
            ..waiting(AT_PLAYER, BREAKING, &[Manoeuvre::BreakOff], true)
        };
        assert_eq!(
            run_at_player(&held, Vec2::new(200.0, 0.0)),
            None,
            "goal 0xa's high level runs through the wait (@0x8d4c8)"
        );
    }
}
