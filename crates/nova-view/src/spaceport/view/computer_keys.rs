//! Where the keys the mission computer held back during a press go once
//! it leaves or accepts (see the `mission_keys` module for the
//! original's paths, with addresses in `EV Nova.app`'s `EV Nova`, i386).
//!
//! - Left by a key or a click among them, the computer's rest reaches the
//!   spaceport, as Leave disposes of the dialog with no flush (@0xa40c8)
//!   and `_DoPortDialog`'s loop goes back to `_PortFilter`
//!   (@0x60035-0x6004e): on the input that let go, or on the tick that
//!   ran a cancelled press's keys.
//! - After an accept, the rest waits for whoever flies the ship to answer
//!   it ([`SpaceportView::set_mission_computer`]): a text the answer
//!   shows flushes it (@0xa1bb2, @0x5ae67); a mission taken re-enters the
//!   computer, whose reopen flushes it (@0xa3a1c), unless nothing is left
//!   to offer, when the computer closes silently (@0xa380a-0xa3817) and
//!   the rest reaches the spaceport; full slots return with no text
//!   (@0xa1b44-0xa1bbe), and the rest runs on in the computer.
//!
//! The asks the rest raises once the accept is answered are served at the
//! next input, as those a tick's replay raises are
//! (rdm:phase/flight-controls-2/phase-5-what-flight-controls-phase-15-left-out).

use nova_sim_frame::{ComputerRefusal, MissionComputer};

use super::{Open, SpaceportView};
use crate::input::Input;

impl SpaceportView {
    /// After an accept, the mission computer open shows `computer`; or,
    /// refused, it closes with nothing said, as the original's re-entry
    /// does (`_reenterMissionDialog` @0xa380a, @0xa3910). `taken` says
    /// whether the accept took the mission. The keys a press held back
    /// after the accept go where the module docs say.
    pub fn set_mission_computer(
        &mut self,
        computer: Result<MissionComputer, ComputerRefusal>,
        taken: bool,
    ) {
        let Some(Open::Computer(screen)) = &mut self.open else {
            return;
        };
        let rest = screen.take_after_accept();
        let reopened = computer.is_ok();
        match computer {
            Ok(computer) => screen.set_computer(computer),
            Err(_) => self.close_open(),
        }
        if self.mission_dialogs_open() || (reopened && taken) {
            return;
        }
        self.route_left(&rest);
    }

    /// Tells the mission computer open, if it is, whether the mission
    /// info screen lists a mission
    /// ([`MissionComputerScreen::set_missions_listed`](crate::spaceport::missions::MissionComputerScreen::set_missions_listed)):
    /// whoever flies the ship says so as the computer opens and after an
    /// accept; an abort says so itself
    /// ([`SpaceportView::set_mission_info`]).
    pub fn set_computer_missions_listed(&mut self, listed: bool) {
        if let Some(Open::Computer(screen)) = &mut self.open {
            screen.set_missions_listed(listed);
        }
    }

    /// What the mission computer's replay left after the key or click that
    /// left it, for [`SpaceportView::route_left`]: only a leave leaves a
    /// rest there.
    pub(super) fn take_left_computers_rest(&mut self) -> Vec<Input> {
        match &mut self.open {
            Some(Open::Computer(screen)) => screen.take_unclaimed(),
            _ => Vec::new(),
        }
    }

    /// Runs `inputs`, left by a screen that closed, through the spaceport's
    /// routing, in order.
    pub(super) fn route_left(&mut self, inputs: &[Input]) {
        for input in inputs {
            self.route_input(input);
        }
    }

    /// Closes the mission computer if a tick left it, handing its rest to
    /// the spaceport, unless mission texts are queued over it.
    pub(super) fn close_left_computer(&mut self) {
        if !matches!(&self.open, Some(Open::Computer(screen)) if screen.closed())
            || self.texts_queued()
        {
            return;
        }
        let left = self.take_left_computers_rest();
        self.close_open();
        self.route_left(&left);
    }
}
