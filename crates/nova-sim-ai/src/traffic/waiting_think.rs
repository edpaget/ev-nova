//! A ship's think under its mode's wait: a ship holding its course after
//! its glide or burst ([`Mode::Coasting`]) or entering a gate
//! ([`Mode::EnteringGate`]) runs its type's think every frame of its wait,
//! as `_AIDispatch` runs it (in the `EV Nova` executable).
//!
//! - **The dispatch.** `_AIDispatch` resets a disabled ship (goal 0, sub 0,
//!   @0x8fed1-0x8ff13) and skips its type AI. For every other ship it
//!   skips the type AI only for goal 0x15 (`cmpw $0x15, 0xc8c8`
//!   @0x8ff18-0x8ff20 → @0x9001f). Then it calls `_AIVerifySwarmLeader` and
//!   `_AIFindSwarmLeader` (@0x8ff26-0x8ff35) and one type AI:
//!   `_DeathStarAI` @0x8ff43, `_DefenseShipAI` (+0x8c ≠ -1) @0x8ff5a,
//!   `_MinerAI` (class 0x9ec & 3, no leader) @0x8ff8d, `_WimpyTraderAI`
//!   @0x8ffab, `_BraveTraderAI` @0x8ffbb, `_PirateWarshipAI` (government
//!   flag 0x1000) @0x8fff0, `_WarshipAI` @0x8fffa, `_InterceptorAI`
//!   @0x9000a, `_EscortAI` @0x90016. No +0x4c test comes between the reset
//!   and the type AI: the dispatch's own +0x4c reads are the cloak think
//!   (@0x8fca1) and the fold step (@0x90063). After the type AI come
//!   `_HighLevelAIHandler` (@0x901e5) and the tail jump to
//!   `_LowLevelAIHandler` (@0x901f4). So a coasting ship (goal 0) and a ship
//!   entering a gate (goal 0x14) run their type AI every frame of their
//!   wait ([`decides_in`]), and only the high level stops at its head
//!   (@0x8d4b9-0x8d4d2 → @0x8fb47, past the goal-0 tail's sub 0 @0x8fb09).
//!   Their steering, thrust and fire stay held (`_HandleShip`
//!   @0x34a54-0x34a72).
//! - **The sub-goals the waits hold.** A coasting ship holds sub 0:
//!   `_AIMakeShipStopJumping` writes goal 0 and sub 0 (@0x82e96-0x82e9f),
//!   but for goals 9 and 0xf, which keep both (@0x82e84-0x82e94). A ship
//!   entering a gate holds sub 0x17 with a wait of 16 (@0x8dc5a-0x8dc94).
//!   `_AIShipIsEnteringHypergate` tests the sub alone (`cmpw $0x17,
//!   0xc8ca` @0x89508), and `_HandleShipDisplay` fades the ship by that
//!   test and frees it once +0x4c < 1.0 (@0x2c066-0x2c07f). So a goal
//!   written mid-entry doesn't stop the fade, and the ship still leaves. A
//!   sub other than 0x17 does ([`Npc::takes_sub_write`]): the ship stays,
//!   held still while +0x4c runs on, then flies its goal.
//! - **The type thinks' sub writes** a waiting ship reaches: `_WarshipAI`'s
//!   fully-dry drop (@0x8c108), `_PirateWarshipAI`'s count-wait entry
//!   (@0x8c6fe) and dry drop (@0x8c878), `_AIFindShipToPlunder`'s find
//!   (@0x7fea5), `_DeathStarAI`'s drop of goal 0x12 (@0x8c2b2) and
//!   `_EscortAI`'s lost parent (@0x8396b), each a
//!   [`Decision::writes_sub`], and `_EscortAI`'s inertialess detach (sub 4
//!   @0x83c3a, [`Npc::detach_for_own_jump`]). The traffic takes the sub
//!   write first among a think's writes of the mode, so a think that also
//!   ends the wait or enters the count wait leaves the ship flying,
//!   whatever order the original writes them in.
//! - **The count wait over a mode's wait** ([`Npc::takes_count_wait`]):
//!   `_PirateWarshipAI`'s entry writes goal 0x16, sub 0 and +0x4c = 120.0
//!   over whatever wait ran (@0x8c6f5-0x8c71c). So the ship has that one
//!   wait, and flies the count wait's own sub 1.
//! - **A wait ended by a think.** +0x4c = -1.0 (`_EscortAI`'s Defend,
//!   Attack and formation blocks @0x83c73, @0x83d6a, @0x83eb0, and the
//!   hail's `_AIMakeShipRefuelPlayer` and `_AIMakeShipRepairPlayer`
//!   @0x82de9, @0x82e20) or 0 (`_DefenseShipAI` @0x7f906) makes the high
//!   level run in the same dispatch. Each goal such a write comes with (4,
//!   0xa, 9, 0xf) has a high-level block that writes its own sub, so a ship
//!   entering a gate stays and flies its goal that frame, as a coasting one
//!   does ([`Npc::end_wait`]). The hail's two makers also write sub 0 first
//!   (@0x82de0, @0x82e17).
//!
//! None of these functions calls `PlaySound` or `_beepSnd`: this raises no
//! [`SimSound`](crate::SimSound).
//!
//! [`Decision::writes_sub`]: crate::ai::Decision::writes_sub

use crate::ai::pursuit::Pursuit;
use crate::rulebook::RuleSource;
use crate::traffic::board_wait::BOARD_WAIT;
use crate::traffic::npc::{Mode, Npc};

/// Whether a ship in `mode` makes the decisions due on the AI timer: flying,
/// held at its gate ([`Mode::HeldAtGate`], a goal other than 0x15), holding
/// its course ([`Mode::Coasting`]) or entering a gate
/// ([`Mode::EnteringGate`]), as `_AIDispatch` runs the type AI for every
/// goal but 0x15, whatever the wait (@0x8ff18-0x90016). A ship gliding in,
/// bursting out of a gate or waiting in one decides nothing, nor does a pod
/// adrift.
#[must_use]
pub(crate) fn decides_in(mode: Mode) -> bool {
    matches!(
        mode,
        Mode::Flying | Mode::HeldAtGate { .. } | Mode::Coasting { .. } | Mode::EnteringGate { .. }
    )
}

impl Npc {
    /// A think writes its sub-goal (+0xc8ca, [`Decision::writes_sub`]): a
    /// gate entry, which the fade reads by sub 0x17 alone, ends
    /// ([`Npc::ends_gate_entry`], the ship holding its course for the wait
    /// left), and the pursuit it kept goes, as no fight sub stands after
    /// the write.
    ///
    /// [`Decision::writes_sub`]: crate::ai::Decision::writes_sub
    pub fn takes_sub_write(&mut self) {
        self.ends_gate_entry();
        self.pursuit = Pursuit::default();
    }

    /// It enters the count wait over whatever wait its mode runs:
    /// `_PirateWarshipAI` writes sub 0 and +0x4c = 120.0 (@0x8c6fe,
    /// @0x8c71c). A gate entry ends ([`Npc::ends_gate_entry`]); a coast or a
    /// hold at its gate ends as [`Npc::end_wait`] ends them, the held ship
    /// owing its burst as `burst` reads
    /// [`RuleKey::HeldGateBurst`](crate::RuleKey::HeldGateBurst). Either way
    /// its AI wait is then [`BOARD_WAIT`], its one wait.
    pub(crate) fn takes_count_wait(&mut self, burst: RuleSource) {
        self.ends_gate_entry();
        self.end_wait(burst);
        self.ai_wait = BOARD_WAIT;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::Goal;
    use crate::ai::fire::Manoeuvre;
    use crate::ai::pursuit::{Body, Frame, Kept};
    use crate::catalog::StellarId;
    use crate::combat::ShipRef;
    use crate::stats::ShipStats;
    use crate::traffic::fixture::npc;

    const GATE: StellarId = StellarId(300);

    /// A pursuit kept from a rush, its frame a rush.
    fn rushing() -> Pursuit {
        Pursuit {
            kept: Some(Kept::Rush),
            frame: Some(Frame {
                goal: Goal::Attack(ShipRef::Player),
                bodies: [
                    Some(Body {
                        manoeuvre: Manoeuvre::Rush,
                        heading: 90,
                    }),
                    None,
                ],
                speed_cap: None,
            }),
        }
    }

    fn in_mode(mode: Mode) -> Npc {
        Npc {
            mode,
            pursuit: rushing(),
            ..npc(1, ShipStats::default())
        }
    }

    #[test]
    fn a_ship_flying_held_coasting_or_entering_decides() {
        for mode in [
            Mode::Flying,
            Mode::HeldAtGate { ticks_left: 3 },
            Mode::Coasting { ticks_left: 3 },
            Mode::EnteringGate {
                gate: GATE,
                ticks_left: 3,
            },
        ] {
            assert!(decides_in(mode), "{mode:?}");
        }
        for mode in [
            Mode::JumpingIn { tick: 3 },
            Mode::Emerging { ticks_left: 3 },
            Mode::LeavingGate { tick: 3 },
            Mode::Adrift { ticks_left: 3 },
        ] {
            assert!(!decides_in(mode), "{mode:?}: goal 8, 0x15 or a pod");
        }
    }

    #[test]
    fn a_sub_written_over_a_gate_entry_ends_it_and_drops_the_kept_pursuit() {
        let mut entering = in_mode(Mode::EnteringGate {
            gate: GATE,
            ticks_left: 7,
        });
        entering.takes_sub_write();
        assert_eq!(entering.mode, Mode::Coasting { ticks_left: 7 });
        assert_eq!(entering.pursuit, Pursuit::default());
        let mut coasting = in_mode(Mode::Coasting { ticks_left: 7 });
        coasting.takes_sub_write();
        assert_eq!(coasting.mode, Mode::Coasting { ticks_left: 7 });
        assert_eq!(coasting.pursuit, Pursuit::default());
    }

    #[test]
    fn the_count_wait_over_a_coasting_entering_or_held_ships_wait_is_its_only_wait() {
        for mode in [
            Mode::Coasting { ticks_left: 9 },
            Mode::EnteringGate {
                gate: GATE,
                ticks_left: 9,
            },
            Mode::HeldAtGate { ticks_left: 9 },
            Mode::Flying,
        ] {
            for (burst, owed) in [(RuleSource::Engine, Some(30.0)), (RuleSource::Bible, None)] {
                let mut ship = in_mode(mode);
                ship.takes_sub_write();
                ship.takes_count_wait(burst);
                assert_eq!(ship.mode, Mode::Flying, "{mode:?}");
                assert_eq!(ship.ai_wait, BOARD_WAIT, "{mode:?}");
                let owes = if matches!(mode, Mode::HeldAtGate { .. }) {
                    owed
                } else {
                    None
                };
                assert_eq!(ship.owed_burst, owes, "{mode:?} {burst:?}");
            }
        }
    }

    #[test]
    fn the_count_wait_ends_a_gate_entry_with_no_sub_write_before_it() {
        // The entry's sub 0 (@0x8c6fe) is the count wait's own.
        let mut entering = in_mode(Mode::EnteringGate {
            gate: GATE,
            ticks_left: 9,
        });
        entering.takes_count_wait(RuleSource::Engine);
        assert_eq!(
            (entering.mode, entering.ai_wait),
            (Mode::Flying, BOARD_WAIT)
        );
    }
}
