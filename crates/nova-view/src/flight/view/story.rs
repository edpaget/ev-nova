//! [`FlightView`]'s readouts of the new game's storylines over the
//! [`FlightSim`] port: the news heard in the system the ship is in
//! ([`ScreenAsk::HeardNews`]), the story the landed stellar's bar hosts
//! ([`ScreenAsk::BarStory`]), and the offers' accepts the flight refused
//! for room ([`FlightEvent::RoomRefused`]), each taken once; and one
//! command, the news a closed log listed read ([`FlightCommand::ReadNews`]).
//! What the
//! session answers and raises is nova-sim's (`session/story_screens.rs`,
//! `session/bar_story.rs`, `session/storylines.rs`); the router words and
//! shows them.
//!
//! [`FlightEvent::RoomRefused`]: nova_sim::flight_frame::FlightEvent::RoomRefused

use nova_sim::flight_frame::{
    BarStory, FlightCommand, Names, RoomRefusal, ScreenAnswer, ScreenAsk,
};
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

    /// The story the landed stellar's bar hosts ([`ScreenAsk::BarStory`]):
    /// the scenes it lists and the news heard in the system, or `None`
    /// when the bar hosts none (the original's bar, in flight, or no bar
    /// here).
    pub fn bar_story(&mut self) -> Option<BarStory> {
        match self.sim.screen(ScreenAsk::BarStory) {
            ScreenAnswer::BarStory(story) => Some(story),
            _ => None,
        }
    }

    /// Marks read the news items a log the player closed listed, by id
    /// ([`FlightCommand::ReadNews`]): the session marks those the ship's
    /// system has heard (`session/apply/news.rs`).
    pub fn read_news(&mut self, ids: Vec<String>) {
        self.send(FlightCommand::ReadNews(ids));
    }

    /// The offers' accepts the flight refused for room since they were
    /// last taken, in order; taking them leaves none.
    pub fn take_room_refusals(&mut self) -> Vec<RoomRefusal> {
        self.drain().take_room_refusals()
    }
}
