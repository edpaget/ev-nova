//! [`FlightView`]'s readouts of the new game's storylines over the
//! [`FlightSim`] port: the offers' accepts the flight refused for room
//! ([`FlightEvent::RoomRefused`]), each taken once. What the session
//! raises is nova-sim's (`session/storylines.rs`); the router words and
//! shows them.
//!
//! [`FlightEvent::RoomRefused`]: nova_sim::flight_frame::FlightEvent::RoomRefused

use nova_sim::flight_frame::RoomRefusal;

use crate::flight::sim_port::FlightSim;

use super::FlightView;

impl<C, S: FlightSim> FlightView<C, S> {
    /// The offers' accepts the flight refused for room since they were
    /// last taken, in order; taking them leaves none.
    pub fn take_room_refusals(&mut self) -> Vec<RoomRefusal> {
        self.drain().take_room_refusals()
    }
}
