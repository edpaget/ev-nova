//! [`FlightView`]'s hail over the [`FlightSim`] port: the comm key, the
//! comm dialog's presses and close, the escort dialog a fleet escort
//! opens in its place, and the offer a person makes in its place, each
//! held for the router. What a ship answers is the flight's
//! (`nova_sim::Session::hail_key`, `nova_sim::Session::answer`,
//! `nova_sim::Session::haggle`).

use nova_sim::escort_dialog::{EscortButton, EscortDialogView};
use nova_sim::flight_frame::{Answer, FlightCommand, ScreenAnswer, ScreenAsk};
use nova_sim::{Haggle, HailView, Hailed, VenueOffer};

use crate::Key;
use crate::flight::sim_port::FlightSim;

use super::{FlightView, MESSAGE_TICKS, hail_refusal_message};

impl<C, S: FlightSim> FlightView<C, S> {
    /// Hails the target ([`FlightCommand::Hail`]), Option held as Alt is,
    /// once the flight takes the comm key ([`FlightView::send_gated`]), or
    /// says why not: a hail answered is held for the router
    /// ([`FlightView::take_hail`]), the escort dialog a fleet escort opens
    /// in its place ([`FlightView::take_escort_dialog`]) and a mission
    /// offered in its place ([`FlightView::take_offer`](FlightView#method.take_offer)),
    /// each letting go of the flight keys; one accepted at once shows
    /// nothing, and so does a navigation stellar that answers, as its comm
    /// dialog is not built. A refusal's beep is the flight's, drained with
    /// its sounds.
    pub(super) fn hail(&mut self) {
        let option = self.held.contains(&Key::Alt);
        match self.send_gated(FlightCommand::Hail { option }) {
            Answer::Hailed(Hailed::Comm(view)) => {
                self.pending_hail = Some(view);
                self.held.clear();
            }
            Answer::Hailed(Hailed::Escort(view)) => {
                self.pending_escort_dialog = Some(view);
                self.held.clear();
            }
            Answer::Hailed(Hailed::Offer(offer)) => self.hold_offer(offer),
            Answer::HailRefused(refusal) => {
                if let Some(text) = hail_refusal_message(refusal) {
                    self.say_for(text, MESSAGE_TICKS);
                }
            }
            // The planet comm dialog (`_DoPlanetCommDialog`) is not built
            // yet: rdm:task/planet-domination opens it on a stellar's
            // answer. A hail the flight refused does nothing.
            _ => {}
        }
    }

    /// Holds `offer`'s prompt for the router, letting go of the flight
    /// keys; a mission accepted at once shows nothing here, its texts
    /// queued in the flight, its before-movie first, for the router to
    /// show over flight.
    pub(super) fn hold_offer(&mut self, offer: VenueOffer) {
        if let VenueOffer::Prompt(prompt) = offer {
            self.pending_offer = Some(prompt);
            self.held.clear();
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
