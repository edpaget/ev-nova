//! [`FlightView`]'s board and plunder over the [`FlightSim`] port: the
//! Board key's latch and its attempt, what a boarding opens or says, and
//! the plunder dialog's presses. What boarding does and what a press takes
//! are the flight's (`nova_sim::Session::board`,
//! `nova_sim::Session::plunder`). The board's and the plunder's names are
//! worded by [`Words`], which the concrete callers hand in
//! ([`FlightView::WORDS`]).

use std::time::Duration;

use nova_sim::flight_frame::{Answer, FlightCommand};
use nova_sim::{Boarding, PlunderView, Take, Taken};

use crate::flight::sim_port::FlightSim;
use crate::time::sixtieths;
use crate::ui::PlunderShown;

use super::words::Words;
use super::{
    BOARD_LINE_TICKS, BOARD_REFUSAL_TICKS, FlightView, GRANT_TICKS, MESSAGE_TICKS, REPELLED,
    board_refusal_message, grant_message, plunder_message,
};

impl<C, S: FlightSim> FlightView<C, S> {
    /// Boards the target ([`FlightCommand::Board`]), or says why not: a
    /// boarding that opens is held for the router
    /// ([`FlightView::take_boarding`]), and a mission offered in its place
    /// too ([`FlightView::take_offer`](FlightView#method.take_offer)), each
    /// letting go of the flight keys. A failed mission's ship boarded for
    /// plunder says its pickup or assist line first
    /// ([`FlightEvent::BoardLine`](nova_sim::flight_frame::FlightEvent::BoardLine)),
    /// and a grant its retrieval
    /// ([`FlightEvent::Grant`](nova_sim::flight_frame::FlightEvent::Grant)),
    /// each in `words`. Whether the attempt sets the board latch
    /// ([`BoardRefusal::latches`](nova_sim::board::BoardRefusal::latches);
    /// every board that passes does, and one the flight refused does not).
    pub(super) fn board(&mut self, words: Words<Self>) -> bool {
        let answer = self.send(FlightCommand::Board);
        if let Some(line) = self.board_line_drained() {
            let said = (words.board_line)(self, line);
            self.say_for(said, BOARD_LINE_TICKS);
        }
        match answer {
            Answer::Boarded(boarding) => {
                self.boarded(boarding, words);
                true
            }
            Answer::BoardRefused(refusal) => {
                if let Some(text) = board_refusal_message(refusal) {
                    self.say_for(text, BOARD_REFUSAL_TICKS);
                }
                refusal.latches()
            }
            _ => false,
        }
    }

    /// What a board that passed the checks shows, in `words`.
    fn boarded(&mut self, boarding: Boarding, words: Words<Self>) {
        match boarding {
            Boarding::Opened(_) => {
                self.pending_boarding = true;
                self.held.clear();
                if let Some(granted) = self.grant_drained() {
                    let (name, plural) = (words.outfit_names)(self, granted.outfit);
                    let said = grant_message(granted.count, &name, &plural);
                    self.say_for(said, GRANT_TICKS);
                }
            }
            Boarding::Repelled => self.say_for(REPELLED, MESSAGE_TICKS),
            Boarding::Offered(offer) => self.hold_offer(offer),
            Boarding::MissionShip(Some(line)) => {
                let said = (words.board_line)(self, line);
                self.say_for(said, BOARD_LINE_TICKS);
            }
            // Its line is the flight's message (`FlightEvent::Message`).
            Boarding::MissionShip(None) | Boarding::Recovered(_) => {}
        }
    }

    /// One frame of the Board key, with B down or not, as `_HandlePlayer`
    /// runs it (@0x6a98b-0x6a9a8) on [`FlightView::board_latch`]: with B
    /// down and the latch clear, a board attempt
    /// ([`FlightView::board`]), which sets the latch unless it found no
    /// target or the player cloaked (`*0xdee18` @0x6504c, after
    /// @0x65031 and @0x6503f); with B up, the latch cleared. So B held
    /// with no target, or cloaked, boards on the first frame a target is
    /// taken or the cloak drops, and any other outcome waits for B's
    /// release.
    ///
    /// The Board key's gate ([`FlightCommand::gate`]) is asked here,
    /// before the latch moves, as J's frame asks its own
    /// (`FlightView::jump_frame`): a refused gate leaves the latch as it
    /// was, B down or up, and [`FlightView::board`] then sends with no
    /// gate of its own.
    ///
    /// [`FlightCommand::gate`]: nova_sim::flight_frame::FlightCommand::gate
    pub(super) fn board_frame(&mut self, down: bool, words: Words<Self>) {
        if !self.takes_command(&FlightCommand::Board) {
            return;
        }
        if !down {
            self.board_latch = false;
        } else if !self.board_latch {
            self.board_latch = self.board(words);
        }
    }

    /// What is on board the ship just boarded, named in `words`, once:
    /// the router takes it to open the plunder dialog.
    pub(super) fn take_boarding_with(&mut self, words: Words<Self>) -> Option<PlunderShown> {
        if !std::mem::take(&mut self.pending_boarding) {
            return None;
        }
        self.plunder_shown_with(words)
    }

    /// What is on board the ship being boarded, while the plunder dialog
    /// is open ([`CommSource::boarding`](crate::flight::sim_port::CommSource::boarding)).
    #[must_use]
    pub fn boarding(&self) -> Option<PlunderView> {
        self.sim.boarding()
    }

    /// What the plunder dialog shows while it is open:
    /// [`FlightView::boarding`] with its cargo's good and its ammunition's
    /// outfit named in `words`, as the plunder's messages name them.
    pub(super) fn plunder_shown_with(&self, words: Words<Self>) -> Option<PlunderShown> {
        let view = self.boarding()?;
        let (good, outfit) = (words.plunder_names)(
            self,
            view.cargo.map(|(good, _)| good),
            view.ammo.map(|(outfit, _)| outfit),
        );
        Some(PlunderShown { view, good, outfit })
    }

    /// Presses `take` in the plunder dialog with the dialog open for
    /// `held`, real time ([`FlightCommand::Plunder`]), and says what it did
    /// in `words`. The target panel is taken afresh, as `_HandlePlayer`
    /// redraws it once the dialog closes (@0x6eb97, @0x6eda6). A press
    /// that leaves the dialog open ends the loop's pass with `_HandleComm`
    /// (@0x949a1): the message line counts down one tick, a take's own
    /// line after `_DisplayComm` sets it, so it shows 239, the flight's
    /// line with it ([`FlightCommand::CommTicked`], after the line it
    /// sets), and the floating map is hidden once more than 500 ticks have
    /// passed since it was shown, `TickCount` moving on under the dialog
    /// (@0x30262-0x30285). A press the flight refused, with no boarding
    /// under way, takes nothing.
    pub(super) fn plunder_held_with(
        &mut self,
        take: Take,
        held: Duration,
        words: Words<Self>,
    ) -> Taken {
        let (taken, open) = match self.send(FlightCommand::Plunder(take)) {
            Answer::Plundered { taken, open } => (taken, open),
            _ => (Taken::Nothing, false),
        };
        let (good, outfit) = match taken {
            Taken::Cargo { good, .. } => (words.plunder_names)(self, Some(good), None),
            Taken::Ammo { outfit, .. } => (words.plunder_names)(self, None, Some(outfit)),
            _ => (None, None),
        };
        if let Some(text) = plunder_message(taken, good.as_deref(), outfit.as_deref()) {
            self.say_for(text, MESSAGE_TICKS);
            // The flight's pass counted the line it had; the take's own
            // comes first in the original (`_DisplayComm`, then
            // `_HandleComm`).
            if open {
                self.send(FlightCommand::CommTicked);
            }
        }
        if open {
            if let Some(message) = &mut self.message {
                message.ticks = message.ticks.saturating_sub(1);
            }
            self.floating.expire(sixtieths(self.elapsed + held));
        }
        self.retake_target();
        taken
    }

    /// Lets the boarding under way go with no dialog shown and no sound
    /// ([`FlightCommand::LetGoOfBoarding`]): the router's way out when it
    /// cannot lay the plunder dialog out.
    pub fn let_go_of_boarding(&mut self) {
        self.send(FlightCommand::LetGoOfBoarding);
    }
}
