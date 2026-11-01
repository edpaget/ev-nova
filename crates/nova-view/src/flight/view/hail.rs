//! [`FlightView`]'s hail over the [`FlightSim`] port: the comm key, the
//! comm dialog's presses and close, the escort dialog a fleet escort
//! opens in its place, the offer a person makes in its place, and the
//! planet comm dialog a stellar that answers opens, each held for the
//! router. What a ship or a stellar answers is the flight's
//! (`nova_sim::Session::hail_key`, `nova_sim::Session::answer`,
//! `nova_sim::Session::haggle`, `nova_sim::Session::press_planet_comm`,
//! `nova_sim::Session::haggle_bribe`).

use nova_sim::escort_dialog::{EscortButton, EscortDialogView};
use nova_sim::flight_frame::{Answer, FadedPress, FlightCommand, ScreenAnswer, ScreenAsk};
use nova_sim::planet_comm::{PlanetCommItem, PlanetCommView};
use nova_sim::{Haggle, HailView, Hailed, VenueOffer};

use crate::Key;
use crate::flight::sim_port::FlightSim;

use super::{FlightView, MESSAGE_TICKS, hail_refusal_message};

impl<C, S: FlightSim> FlightView<C, S> {
    /// Hails the target ([`FlightCommand::Hail`]), Option held as Alt is,
    /// once the flight takes the comm key ([`FlightView::send_gated`]), or
    /// says why not: a hail answered is held for the router
    /// ([`FlightView::take_hail`]), the escort dialog a fleet escort opens
    /// in its place ([`FlightView::take_escort_dialog`]), a mission
    /// offered in its place ([`FlightView::take_offer`](FlightView#method.take_offer))
    /// and the planet comm dialog a navigation stellar that answers opens
    /// ([`FlightView::take_planet_comm`]), each letting go of the flight
    /// keys, as `_SetAllKeyCmdFlags` does after the dialog (@0x62067); a
    /// mission accepted at once shows nothing. A refusal's beep is the
    /// flight's, drained with its sounds.
    pub(super) fn hail(&mut self) {
        let option = self.held.contains(&Key::Alt);
        let answer = self.send_gated(FlightCommand::Hail { option });
        self.take_hail_answer(answer);
    }

    /// The comm key read on the pass a jump collapsed under the white
    /// ([`FadedPress::Hail`]): whether it latched the keys after it, the
    /// dialog's guard refusing it (`_SetAllKeyCmdFlags` after it, letting
    /// go of the latched keys) or an offer opening. A refusal and an
    /// offer are answered as [`FlightView::hail`]'s.
    pub(super) fn hail_faded(&mut self) -> bool {
        let option = self.held.contains(&Key::Alt);
        let answer = self.send_gated(FlightCommand::PressFaded(FadedPress::Hail { option }));
        let faded = answer == Answer::Hailed(Hailed::Faded);
        if faded {
            self.keep_unlatched_keys();
        }
        self.take_hail_answer(answer);
        faded || self.hail_opened()
    }

    /// Holds what a hail's `answer` opened, or says why it was refused.
    fn take_hail_answer(&mut self, answer: Answer) {
        match answer {
            Answer::Hailed(Hailed::Comm(view)) => {
                self.pending_hail = Some(view);
                self.let_go_of_input();
            }
            Answer::Hailed(Hailed::Escort(view)) => {
                self.pending_escort_dialog = Some(view);
                self.let_go_of_input();
            }
            Answer::Hailed(Hailed::Stellar(view)) => {
                self.pending_planet_comm = Some(view);
                self.let_go_of_input();
            }
            Answer::Hailed(Hailed::Offer(offer)) => self.hold_offer(offer),
            Answer::HailRefused(refusal) => {
                if let Some(text) = hail_refusal_message(refusal) {
                    self.say_for(text, MESSAGE_TICKS);
                }
            }
            // A hail the flight refused does nothing.
            _ => {}
        }
    }

    /// Whether a hail left a dialog for the router, whichever kind: the
    /// keys were let go ([`FlightView::hail`]) and the dialog takes Y's
    /// release, so Y mustn't be tracked as held behind it.
    pub(super) fn hail_opened(&self) -> bool {
        self.pending_hail.is_some()
            || self.pending_escort_dialog.is_some()
            || self.pending_planet_comm.is_some()
            || self.pending_offer.is_some()
    }

    /// Holds `offer`'s prompt for the router, letting go of the flight
    /// keys; a mission accepted at once shows nothing here, its texts
    /// queued in the flight, its before-movie first, for the router to
    /// show over flight.
    pub(super) fn hold_offer(&mut self, offer: VenueOffer) {
        if let VenueOffer::Prompt(prompt) = offer {
            self.pending_offer = Some(prompt);
            self.let_go_of_input();
        }
    }

    /// The hail just answered, once: the router takes it to open the comm
    /// dialog.
    pub fn take_hail(&mut self) -> Option<HailView> {
        self.pending_hail.take()
    }

    /// The escort dialog just opened, once: the router takes it to show
    /// the dialog.
    pub fn take_escort_dialog(&mut self) -> Option<EscortDialogView> {
        self.pending_escort_dialog.take()
    }

    /// The planet comm dialog just opened, once: the router takes it to
    /// show the dialog.
    pub fn take_planet_comm(&mut self) -> Option<PlanetCommView> {
        self.pending_planet_comm.take()
    }

    /// Presses `item` in the planet comm dialog
    /// ([`FlightCommand::PressPlanetComm`]): what it shows after, or none
    /// once it closes. A close refreshes the HUD, as the original's
    /// `_RedrawStatus(1)` does (@0x97b3d); a press the flight refused, with
    /// no flight to press, does nothing.
    pub fn press_planet_comm(&mut self, item: PlanetCommItem) -> Option<PlanetCommView> {
        self.planet_comm_answer(FlightCommand::PressPlanetComm(item))
    }

    /// Makes `choice` in the planet comm dialog's bribe haggle
    /// ([`FlightCommand::HaggleBribe`]): what the dialog shows after, or
    /// none once a bribe paid closes it, refreshing the HUD as a close
    /// does ([`FlightView::press_planet_comm`]).
    pub fn haggle_bribe(&mut self, choice: Haggle) -> Option<PlanetCommView> {
        self.planet_comm_answer(FlightCommand::HaggleBribe(choice))
    }

    /// Sends `command`, a planet comm dialog's, and gives what the dialog
    /// shows after, refreshing the HUD when it closed.
    fn planet_comm_answer(&mut self, command: FlightCommand) -> Option<PlanetCommView> {
        let Answer::PlanetComm(view) = self.send(command) else {
            return None;
        };
        if view.is_none() {
            self.redraw_status();
        }
        view
    }

    /// Presses `button` in the escort dialog
    /// ([`FlightCommand::PressEscort`]): what it shows after, or none once
    /// it closes. A close refreshes the HUD, as a hail's does
    /// ([`FlightView::hang_up`]); a press the flight refused, with no
    /// flight to press, does nothing.
    pub fn press_escort(&mut self, button: EscortButton) -> Option<EscortDialogView> {
        let Answer::EscortDialog(view) = self.send(FlightCommand::PressEscort(button)) else {
            return None;
        };
        if view.is_none() {
            self.redraw_status();
        }
        view
    }

    /// The comm dialog's contents while a hail is under way
    /// ([`ScreenAsk::Hail`]), asked after the haggle dialog closes: none
    /// with no hail under way, or for a flight with nothing to answer.
    pub fn hailing(&mut self) -> Option<HailView> {
        match self.sim.screen(ScreenAsk::Hail) {
            ScreenAnswer::Hail(view) => Some(*view),
            _ => None,
        }
    }

    /// Presses the `pick`th option listed ([`FlightCommand::AnswerHail`]):
    /// the comm dialog's contents after, none with no hail under way.
    pub fn answer(&mut self, pick: usize) -> Option<HailView> {
        match self.send(FlightCommand::AnswerHail(pick)) {
            Answer::Hail(view) => view,
            _ => None,
        }
    }

    /// Makes `choice` in the haggle dialog ([`FlightCommand::Haggle`]):
    /// the comm dialog's contents after, none with no hail under way.
    pub fn haggle(&mut self, choice: Haggle) -> Option<HailView> {
        match self.send(FlightCommand::Haggle(choice)) {
            Answer::Hail(view) => view,
            _ => None,
        }
    }

    /// Ends the hail under way, if any ([`FlightCommand::HangUp`]), and
    /// refreshes the HUD as the original's comm dialog does on closing
    /// ([`FlightView::redraw_status`]).
    pub fn hang_up(&mut self) {
        self.send(FlightCommand::HangUp);
        self.redraw_status();
    }
}
