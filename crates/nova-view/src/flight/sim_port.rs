//! The [`FlightSim`] port: what the flight view needs of the flight it
//! shows, in the view's own terms. A frame to draw from
//! ([`FlightSim::frame`]), a fixed step ([`FlightSim::step`]), the
//! player's commands ([`FlightSim::apply`]) and what the flight raised
//! for the view to act on once ([`FlightSim::drain`]). The values it
//! carries are `nova_sim`'s ([`flight_frame`](nova_sim::flight_frame)),
//! so the session can build them.
//!
//! [`SessionFlight`](super::sim_adapter::SessionFlight) is the adapter
//! over a real `nova_sim::Session`. The port is the strangler's seam:
//! the view still reaches the session itself in the
//! `FlightView<C, SessionFlight<C>>` blocks, and later work moves that
//! code across the port a slice at a time, growing the port and its
//! values as it goes.
//!
//! The three supertraits, [`MapSource`], [`CommSource`] and
//! [`PortSource`], are empty for now: the course map's, the comm's and
//! the spaceport's questions will go there, each method defaulting to
//! "nothing open", so a sim that has none of them open needs no code.

use nova_sim::flight_frame::{
    Answer, FlightCommand, FlightEvent, FlightFrame, StepInput, StepReport,
};

/// The flight the view shows, as the view needs it (see the module
/// docs).
pub trait FlightSim: MapSource + CommSource + PortSource {
    /// Fills `into` with what the view draws now, writing every field, so
    /// a frame kept from an earlier fill shows nothing stale.
    fn frame(&self, into: &mut FlightFrame);

    /// Advances the flight one fixed step, given `input`, and reports what
    /// the view needs of it.
    fn step(&mut self, input: &StepInput) -> StepReport;

    /// Runs the player's `command` and answers it:
    /// [`Answer::Refused`] when it did nothing, and then no frame changes.
    fn apply(&mut self, command: FlightCommand) -> Answer;

    /// Moves what the flight raised since the last drain onto the end of
    /// `into`, leaving none to drain again.
    fn drain(&mut self, into: &mut Vec<FlightEvent>);
}

/// The course map's questions of the flight. None yet: each method will
/// default to "nothing open".
pub trait MapSource {}

/// The comm's questions of the flight: hails and their answers. None yet:
/// each method will default to "nothing open".
pub trait CommSource {}

/// The spaceport's questions of the flight. None yet: each method will
/// default to "nothing open".
pub trait PortSource {}
