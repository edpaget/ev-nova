//! [`FlightView`]'s readouts of the new game's storylines over the
//! [`FlightSim`] port: the news heard in the system the ship is in
//! ([`ScreenAsk::HeardNews`]), and the offers' accepts the flight refused
//! for room ([`FlightEvent::RoomRefused`]), each taken once. What the
//! session answers and raises is nova-sim's (`session/story_screens.rs`,
//! `session/storylines.rs`); the router words and shows them.
//!
//! [`FlightEvent::RoomRefused`]: nova_sim::flight_frame::FlightEvent::RoomRefused

use nova_sim::flight_frame::{Names, RoomRefusal, ScreenAnswer, ScreenAsk};
use nova_sim::storylines::news::HeardNews;

use crate::flight::sim_port::FlightSim;

use super::FlightView;

impl<C, S: FlightSim> FlightView<C, S> {
    /// The names fixed for the flight's life, as its opening gave them:
    /// what the router words the story log with.
    #[must_use]
    pub fn names(&self) -> &Names {
        &self.names
    }

    /// The news heard in the system the ship is in
    /// ([`ScreenAsk::HeardNews`]), by release day then id: none for a
    /// flight with nothing to answer.
    pub fn heard_news(&mut self) -> Vec<HeardNews> {
        match self.sim.screen(ScreenAsk::HeardNews) {
            ScreenAnswer::HeardNews(news) => news,
            _ => Vec::new(),
        }
    }

    /// The offers' accepts the flight refused for room since they were
    /// last taken, in order; taking them leaves none.
    pub fn take_room_refusals(&mut self) -> Vec<RoomRefusal> {
        self.drain().take_room_refusals()
    }
}
