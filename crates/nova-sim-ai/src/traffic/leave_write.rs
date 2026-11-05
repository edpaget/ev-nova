//! The warp writes of a leave by hyperspace, goal 2, as
//! `_AIMakeShipLeave` makes them (@0x7e2b0-0x7e2e7 in the `EV Nova`
//! executable): goal +0xc8c8 = 2; `if 0 > +0x50` then +0x50 = 0, a floor
//! that raises a stage given up (-1.0) or a glide's -999.0 to 0 and keeps
//! a stage above 0; target +0x70 = -1; and +0xa4 = `TickCount`, a stamp of
//! the warp clock. It writes no sub-goal and plays no sound.
//! [`Npc::takes_leave_write`] makes the floor and the stamp; each caller
//! writes the goal and the target itself.
//!
//! **Its sites.** Every one is the routine or an inlined copy of it:
//!
//! | Site | Address | Engine port |
//! |---|---|---|
//! | `_AIMakeShipLeave` | @0x7e2b0-0x7e2e7, called @0x6234d, @0x6dcd6, @0x95502, @0x95652, @0x961eb, @0x968b7, @0x99b12 | the session's `hail::make_leave` |
//! | `_WimpyTraderAI` idle block | @0x8b179-0x8b19b | [`idle_block::leave_or_wait`](crate::ai::idle_block::leave_or_wait), [`idle_block::leaves`](crate::ai::idle_block::leaves) |
//! | `_MinerAI` scoop-full | @0x8b307-0x8b329, every think while full | [`miner`](crate::ai::miner) |
//! | `_MinerAI` shuttle | @0x8b428-0x8b44a | [`miner`](crate::ai::miner) |
//! | `_BraveTraderAI` idle block | @0x8b5e3-0x8b605 | [`idle_block::leaves`](crate::ai::idle_block::leaves) |
//! | `_WarshipAI` idle block | @0x8b995-0x8b9b7 | [`idle_block::leaves`](crate::ai::idle_block::leaves) |
//! | `_WarshipAI` second idle copy | @0x8bad9-0x8bafb | not modelled ([`idle_block`](crate::ai::idle_block)'s Left out) |
//! | `_WarshipAI` grounded leave | @0x8c0b4-0x8c0d6 | unreachable: its fuel test (@0x8c058-0x8c0b2) can't pass after the first failed ([`warship`](crate::ai::warship)'s `grounded`) |
//! | `_PirateWarshipAI` idle copy | @0x8c59b-0x8c5bd, every think it reaches the copy | [`pirate`](crate::ai::pirate) |
//! | `_InterceptorAI` nag-ship block | @0x8c98d-0x8c9b5, @0x8cab1-0x8cadb, gated on +0xc8d0 = 0x3ff (@0x8c8c8), which `_SpawnNagShip` alone writes (@0x61b5) | shareware alone, not built |
//! | `_InterceptorAI` patrol | @0x8d0c5-0x8d0f0 | [`idle_block::patrol`](crate::ai::idle_block::patrol), via [`interceptor`](crate::ai::interceptor) |
//! | `_HighLevelAIHandler` mission-hunt give-up | @0x8d6be-0x8d6e9 | [`NovaAi`](crate::ai::NovaAi) |
//! | `_HighLevelAIHandler` trader lost-sight leave | @0x8d7cd-0x8d7fe | [`lost_sight::after_think`](crate::ai::lost_sight::after_think) |
//! | `_HighLevelAIHandler` nag ship | @0x8f331-0x8f35e | shareware alone, not built |
//! | `_AIDispatch` mark cleared on a led mission ship | @0x8fc3a-0x8fc96 | the session's `mission_escorts`, through `make_leave` |
//!
//! The thinks' sites set [`Decision::leaves`](crate::ai::Decision::leaves),
//! and the traffic makes the write as it takes the goal; a think that
//! keeps a goal 2 it already held writes nothing. Other goal-2 writes make
//! neither the floor nor the stamp: `_EscortAI` @0x83c31 (+0x50 = 0
//! outright), the low level's detach @0x85adf, `_AICheckParentExists`
//! @0x89415, the goal-0xb push @0x8def0 and own jump @0x8df63, and the
//! high level's hold @0x8d56b.
//!
//! **Who reads them.** +0xa4 is read, for an NPC, by sub 4 (@0x858c7,
//! @0x85906) and sub 0xd (@0x85a63), each after its own stamp, made only
//! with +0x50 at or below 0 (@0x858a4-0x858b0), and by `_HandleShip`'s
//! warp-out with +0x50 above 0 (@0x347f0, @0x348ca). So the stamp shows
//! only on a stage under way at the write: sub 4 then keeps it, and the
//! ship goes a whole jump's time after the write, not on its older clock.
//! +0x50 below 0 is read by `_AICheckParentExists`'s pause (@0x8949d-0x894a6,
//! [`Npc::stage_below_zero`]) and by `_AIDispatch`'s glide test
//! (@0x8fcf5), so the floor shows on a given-up ship that later loses its
//! parent with a successor (it keeps leaving rather than pausing), and on
//! a glide in or a burst out (it ends there).
//!
//! **A ship waiting in its gate.** The emerge writes +0x50 = -1.0
//! (@0x89555), which the engine keeps in the gate-wait mode
//! ([`Mode::Emerging`](super::npc::Mode::Emerging),
//! [`Mode::HeldAtGate`](super::npc::Mode::HeldAtGate)) rather than in
//! [`Warp::stage`](super::warp_out::Warp::stage), which the high level's
//! hold zeroes each pass where the original's waiting ship runs no high
//! level. The floor over it sets
//! [`Warp::gate_wait_floored`](super::warp_out::Warp::gate_wait_floored),
//! which [`Npc::stage_below_zero`] honours, so a successor's pause passes
//! the ship by and it keeps leaving, as the original's +0x50 of 0 does. A
//! flag, not the -1.0 held in the stage: the pause is the one reader of a
//! gate wait's -1.0, and every reader of the stage would otherwise have to
//! learn to read a gate wait's -1.0 apart from a give-up.
//!
//! **Left out:**
//!
//! - `_WarshipAI`'s second idle copy's leave (@0x8bad9-0x8bafb), not
//!   modelled: rdm:phase/npc-ai-2/phase-3-leave-write-left-out.

use super::npc::Npc;
use crate::rulebook::RuleSource;

impl Npc {
    /// `_AIMakeShipLeave`'s warp writes (@0x7e2b0-0x7e2e7) and their
    /// inlined copies (see the module docs): its +0x50 floored at 0, a
    /// stage above 0 kept, and its clock stamped (+0xa4 = `TickCount`).
    ///
    /// A glide in or a burst out (-999.0, [`Npc::jumping_in`]) is floored
    /// too: `_AIDispatch` no longer sends it to goal 8 (@0x8fcf5), so the
    /// glide ends there and goal 2's block writes its sub over sub 0xa. It
    /// flies on at the velocity it held, owing the rest of its burst by
    /// `burst`'s reading of
    /// [`RuleKey::HeldGateBurst`](crate::RuleKey::HeldGateBurst)
    /// ([`Npc::leaves_arrival`]), as [`Npc::follow_player_jump`] has it. A
    /// ship waiting in its gate leaves it, as any goal written over goal
    /// 0x15 does ([`Npc::leaves_gate_goal`]).
    ///
    /// The thinks' writes are taken in the same dispatch as the ship's own
    /// jump, after the clock moves on (`Traffic::open_dispatch`), so a
    /// stage under way runs from t = 0 that frame, as the original's does.
    /// A dialog's write (the session's `make_leave`) comes between frames,
    /// and the next frame's dispatch moves the clock on before the jump
    /// reads it: one tick behind the original's real-time clock, which
    /// crosses no boundary the player sees.
    pub fn takes_leave_write(&mut self, burst: RuleSource) {
        if self.stage_below_zero() {
            // The gate wait's -1.0, kept in the mode (see the module docs).
            self.warp.gate_wait_floored = true;
        }
        self.leaves_gate_goal();
        if self.jumping_in() {
            self.leaves_arrival(burst);
            self.emerges_from = None;
        }
        if self.warp.stage < 0.0 {
            self.warp.stage = 0.0;
        }
        self.warp.stamp();
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::catalog::StellarId;
    use crate::speed_mult::SpeedMult;
    use crate::stats::ShipStats;
    use crate::traffic::fixture::npc;
    use crate::traffic::npc::Mode;
    use crate::traffic::warp_out::{WarpGoal, own_jump};
    use crate::warp::warp_factor;

    fn ship() -> Npc {
        npc(
            1,
            ShipStats {
                warp_factor: warp_factor(0),
                ..ShipStats::default()
            },
        )
    }

    /// `ship` with its stage `stage`, `since` passes since its stamp.
    fn staged(stage: f32, since: u32) -> Npc {
        let mut npc = ship();
        npc.warp.stage = stage;
        npc.warp.since = since;
        npc
    }

    #[test]
    fn a_given_up_stage_is_floored_to_zero() {
        let mut npc = staged(-1.0, 9);
        assert!(npc.stage_below_zero());
        npc.takes_leave_write(RuleSource::Engine);
        assert_eq!(npc.warp.stage, 0.0, "`if 0 > +0x50` +0x50 = 0");
        assert!(!npc.stage_below_zero());
    }

    #[test]
    fn a_stage_under_way_is_kept_and_its_clock_restamped() {
        let mut npc = staged(3.0, 40);
        npc.takes_leave_write(RuleSource::Engine);
        assert_eq!(npc.warp.stage, 3.0, "a stage above 0 is kept");
        assert_eq!(npc.warp.since, 0, "+0xa4 = TickCount");
    }

    #[test]
    fn a_stage_at_zero_is_stamped() {
        let mut npc = staged(0.0, 7);
        npc.takes_leave_write(RuleSource::Engine);
        assert_eq!((npc.warp.stage, npc.warp.since), (0.0, 0));
    }

    #[test]
    fn a_glide_in_ends_owing_its_burst_by_each_reading() {
        for burst in [RuleSource::Engine, RuleSource::Bible] {
            for mode in [Mode::JumpingIn { tick: 4 }, Mode::LeavingGate { tick: 3 }] {
                let mut gliding = ship();
                gliding.mode = mode;
                gliding.emerges_from = Some(StellarId(310));
                let mut followed = gliding.clone();
                gliding.takes_leave_write(burst);
                followed.follow_player_jump(0.0, 0, burst);
                assert_eq!(gliding.mode, Mode::Flying, "{mode:?} {burst:?}");
                assert_eq!(gliding.mode, followed.mode, "{mode:?} {burst:?}");
                assert_eq!(
                    gliding.owed_burst, followed.owed_burst,
                    "{mode:?} {burst:?}"
                );
                assert_eq!(gliding.emerges_from, None, "{mode:?} {burst:?}");
            }
        }
    }

    #[test]
    fn a_ship_waiting_in_its_gate_is_held_there() {
        let mut waiting = ship();
        waiting.mode = Mode::Emerging { ticks_left: 12.0 };
        waiting.emerges_from = Some(StellarId(310));
        waiting.takes_leave_write(RuleSource::Engine);
        assert_eq!(waiting.mode, Mode::HeldAtGate { ticks_left: 12.0 });
        assert_eq!(waiting.emerges_from, None);
    }

    #[test]
    fn a_gate_waits_minus_one_is_floored_to_zero() {
        // The emerge's +0x50 = -1.0 (@0x89555), which the engine keeps in
        // the mode: the floor raises it to 0 there too.
        for mode in [
            Mode::Emerging { ticks_left: 12.0 },
            Mode::HeldAtGate { ticks_left: 12.0 },
            Mode::LeavingGate { tick: 0 },
        ] {
            let mut held = ship();
            held.mode = mode;
            assert!(held.stage_below_zero(), "{mode:?}");
            held.takes_leave_write(RuleSource::Engine);
            assert!(!held.stage_below_zero(), "{mode:?}");
            assert_eq!(held.warp.stage, 0.0, "{mode:?}");
        }
    }

    /// The passes `npc`'s own jump takes until it is gone, at a duration
    /// of 364 (280 Mac ticks at w 1.3: 140 ticks).
    fn passes_to_go(npc: &mut Npc) -> u32 {
        npc.warp.goal = Some(WarpGoal::Own);
        for pass in 1..=1000 {
            if own_jump(npc, 364, SpeedMult::FULL) {
                return pass;
            }
            npc.warp.tick_clock();
        }
        panic!("never gone");
    }

    #[test]
    fn a_stage_under_way_takes_a_whole_jump_from_the_write() {
        // Sub 4 keeps a stamp made above 0 (@0x858a4-0x858b0): the write's
        // restamp holds the ship a whole jump.
        let fresh = passes_to_go(&mut staged(0.0, 0));
        let mut stale = staged(2.0, 500);
        stale.takes_leave_write(RuleSource::Engine);
        assert_eq!(passes_to_go(&mut stale), fresh);
        assert_eq!(passes_to_go(&mut staged(2.0, 500)), 1, "unstamped, at once");
    }
}
