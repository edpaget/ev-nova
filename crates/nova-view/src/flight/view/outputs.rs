//! [`FlightView`]'s one-shot outputs taken over the [`FlightSim`] port:
//! the sounds and diagnostics the app takes each frame, and the flags
//! the router asks after, each drained from the flight into the inbox
//! (see the `inbox` module) and taken from it.

use crate::flight::sim_port::FlightSim;
use crate::{Diagnostic, Sound};

use super::FlightView;

impl<C, S: FlightSim> FlightView<C, S> {
    /// Whether the hold after the player's explosion ran out since this
    /// was last asked ([`FlightEvent::HoldExpired`]): the seam where the
    /// router may show its dialogs over the stopped flight before it
    /// begins the fade. True once.
    ///
    /// [`FlightEvent::HoldExpired`]: nova_sim::flight_frame::FlightEvent::HoldExpired
    pub fn take_hold_expired(&mut self) -> bool {
        self.drain().take_hold_expired()
    }

    /// The escape pod's rescue's `dësc`, once, after the rescue
    /// ([`FlightEvent::RescueDesc`]): the router shows it over flight.
    ///
    /// [`FlightEvent::RescueDesc`]: nova_sim::flight_frame::FlightEvent::RescueDesc
    pub fn take_rescue_desc(&mut self) -> Option<i16> {
        self.drain().take_rescue_desc()
    }

    /// Whether the pilot should be saved ([`FlightEvent::SaveDue`]);
    /// taking it clears it.
    ///
    /// [`FlightEvent::SaveDue`]: nova_sim::flight_frame::FlightEvent::SaveDue
    pub fn take_save_due(&mut self) -> bool {
        self.drain().take_save_due()
    }

    /// The flight's sounds (thrust, landing, taking off and jumping),
    /// then the interface's (the escort menu's, the floating map's zooms,
    /// the player info refused, the course map's beeps), then the fight's:
    /// what [`Screen::take_sounds`](crate::Screen::take_sounds) gives.
    pub(super) fn drained_sounds(&mut self) -> Vec<Sound> {
        let mut sounds: Vec<Sound> = self
            .drain()
            .take_sounds()
            .into_iter()
            .map(Sound::Sim)
            .collect();
        sounds.extend(self.ui_sounds.drain(..).map(Sound::Ui));
        sounds.extend(self.effects.take_sounds().into_iter().map(Sound::Combat));
        sounds
    }

    /// What could not be read of the looks, then the flight's
    /// diagnostics, each once: what
    /// [`Screen::take_diagnostics`](crate::Screen::take_diagnostics) gives.
    pub(super) fn drained_diagnostics(&mut self) -> Vec<Diagnostic> {
        let mut diagnostics = std::mem::take(&mut self.unread_looks);
        let sim = self.drain().take_diagnostics().into_iter();
        diagnostics.extend(sim.map(Diagnostic::Sim));
        diagnostics
    }
}
