//! An exit `_HandlePlayer` settles in its pass: the jump's arrival and
//! the escape pod's rescue ([`Session::settle_exit`]).
//!
//! **The original.** Both exits take the player to another system inside
//! `_HandlePlayer` and leave it by its epilogue (@0x71113), on either of
//! double time's passes:
//!
//! - the arrival, on the pass the warp ends naturally (@0x6bbb8-0x6bbcd):
//!   the warp-out (@0x6bbe1-0x6bc08), then the arrival block
//!   (@0x6bc0d-0x6c620: the fuel @0x6be31, `_SetupShipsInSystem`
//!   @0x6c1f9, `_RefreshSAT` @0x6c605, `_RedrawComm` @0x6c611,
//!   `_DoEscortPayment` @0x6c620), then `jmp 0x71113` (@0x6c625);
//! - the rescue, on the pass the pod's countdown runs out (@0x68ddd-0x692ce:
//!   the fades, the `dësc`, `_AbortMission`, `_ResetPlayer` @0x68f2f,
//!   `_SetupShipsInSystem` @0x69114, `_RefreshSAT` @0x69214, the save),
//!   then `jmp 0x71113` (@0x692ce).
//!
//! The epilogue only restores the stack and returns, so the rest of
//! `_HandlePlayer` (the turn, the regeneration, the move-off and position
//! step, the death timer, the fuel's regeneration, the escort menu's
//! block and the class-order bookkeeping, the klaxon, the eject, the
//! self-destruct, the key block and the cloak key) is skipped, and
//! `_DoPlayGameWork` goes on from @0x43e1e among the new system: the
//! player's `_HandleShipDisplay` (@0x43e29), the collisions, on the
//! advance pass the AI over the ships `_SetupShipsInSystem` set up
//! (`_EnterMoreShips` @0x44237, `_AIDispatch` @0x4429d), then
//! `_HandleShot` (@0x44385) through `_HandleBeams` (@0x4453d). Neither
//! exit makes a sound call after its test (the arrival's warp-out
//! @0x6bc08 is its last).
//!
//! **Here.** Each pass ([`Session::advance_pass`], `Session::tick_again`)
//! calls [`Session::settle_exit`] after the player's move and the
//! fighters' warning (@0x6b3de), and before the pass's masks are loaded,
//! so the scoop reads the new system's sheets:
//!
//! 1. on a natural warp end the warp stage has returned with nothing
//!    after the test run (see the `warp_stage` module), and the exit is
//!    the arrival ([`Session::arrive`], which raises the halt and the
//!    warp-out); else a due rescue settles (`Session::rescue_if_due`);
//! 2. the wrap of the system left is dropped, and the pass's start is
//!    read again ([`PassStart::after_exit`]): the NPCs and where the
//!    player stands are the new system's, and the player's head is marked
//!    as left by the epilogue ([`PlayerHead::returned`]), so the rest of
//!    the pass ([`Session::pass_after_move`]) runs none of the display
//!    step's head nor the cloak key (see the `display_step` module);
//! 3. inside a frame's step ([`Outbox::in_step`], see the `step`
//!    module), the exit is recorded for the step's report
//!    ([`PassExit`]): the fight's events the pass raised before it, in
//!    the system left, which the view hears and draws nowhere, and how
//!    many comm notes came before it. A rescue there also holds the lines
//!    raised after it ([`StepRun::rescue_lines`]), as `_ResetPlayer`
//!    empties the message line (`_commCount` @0x1d8ff-0x1d904) and the
//!    view shows them after its rescue screen; on the second pass the
//!    lines it held before are kept apart for
//!    [`StepRun::second_pass_lines`].
//!
//! A pass run alone (`Session::game_work`, `Session::tick_again`)
//! settles its exit the same way and records none.
//!
//! Left out, filed as
//! rdm:phase/combat-and-flight-2/phase-10-handle-player-exit-left-outs: the
//! regeneration (@0x6cf86), the tribbles (@0x70827) and the hints
//! (@0x7093d), which ours runs before the move and so before the exit,
//! and the dialogs and the save the original settles inside the exits.
//!
//! [`Outbox::in_step`]: super::outbox::Outbox::in_step
//! [`PlayerHead::returned`]: super::game_work::PlayerHead::returned
//! [`StepRun::rescue_lines`]: crate::flight_frame::StepRun::rescue_lines
//! [`StepRun::second_pass_lines`]: crate::flight_frame::StepRun::second_pass_lines

use super::Session;
use super::game_work::PassStart;
use crate::catalog::{PilotCatalog, TrafficCatalog};
use crate::chance::Chance;
use crate::flight_frame::{PassExit, RaisedLine, ShipPlace};

/// Which exit `_HandlePlayer` took (see the module docs).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Exit {
    /// The jump's arrival (@0x6c625).
    Arrival,
    /// The escape pod's rescue (@0x692ce).
    Rescue,
}

/// An exit a pass of a frame's step settled, until the step takes it
/// (see the `step` module).
#[derive(Clone, Debug, PartialEq)]
pub(super) struct SettledExit {
    /// Which exit.
    pub(super) exit: Exit,
    /// Where it came, its `pass` 0 until the step sets it.
    pub(super) report: PassExit,
    /// The lines double time's second pass held before a rescue on it;
    /// `None` for an arrival and for a rescue on the advance pass.
    pub(super) held_before: Option<Vec<RaisedLine>>,
}

impl Session {
    /// Settles the exit due on this pass, if any, reading `catalog` and
    /// drawing on `chance`, and reads the pass's `start` again among the
    /// new system (see the module docs): which exit it was.
    pub(super) fn settle_exit(
        &mut self,
        start: &mut PassStart,
        catalog: &(impl PilotCatalog + TrafficCatalog),
        chance: &mut dyn Chance,
    ) -> Option<Exit> {
        let exit = if self.warp_end_due() {
            Exit::Arrival
        } else if self.ship.fate.rescue_due {
            Exit::Rescue
        } else {
            return None;
        };
        #[cfg(test)]
        for line in std::mem::take(&mut self.out.before_exit_lines) {
            self.space.raise_line(&mut self.out, line);
        }
        let in_step = self.out.in_step;
        let before = if in_step {
            self.take_combat_events()
        } else {
            Vec::new()
        };
        let comm = self.out.comm.len();
        match exit {
            Exit::Arrival => {
                self.arrive(catalog, &mut *chance);
            }
            Exit::Rescue => {
                self.rescue_if_due(catalog, &mut *chance);
            }
        }
        // The system left's wrap reaches no report.
        self.out.player_wrap = None;
        if in_step {
            let held_before = match exit {
                Exit::Arrival => None,
                Exit::Rescue => self.out.second_pass.replace(Vec::new()),
            };
            self.out.exit = Some(SettledExit {
                exit,
                report: PassExit {
                    pass: 0,
                    place: ShipPlace::from(*self.player()),
                    before,
                    comm,
                },
                held_before,
            });
        }
        *start = start.after_exit(self);
        Some(exit)
    }
}

#[cfg(test)]
mod tests;
