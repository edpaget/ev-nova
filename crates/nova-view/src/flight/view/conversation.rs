//! [`FlightView`]'s conversation over the [`FlightSim`] port: the landed
//! stellar's conversation opened, a choice taken on its page, and closed.
//! Each sends its [`FlightCommand::Converse`] and hands back the page the
//! flight answers, `None` when it refuses (no conversation here, a choice
//! it doesn't offer, or a flight that never started). What a
//! conversation plays is nova-sim's (`session/apply/conversation.rs`).

use nova_sim::ConversationPage;
use nova_sim::flight_frame::{Answer, Converse, FlightCommand};

use crate::flight::sim_port::FlightSim;

use super::FlightView;

impl<C, S: FlightSim> FlightView<C, S> {
    /// Opens the landed stellar's conversation from its knot
    /// ([`Converse::Open`]): its first page, or `None` where the stellar
    /// names none.
    pub fn open_conversation(&mut self) -> Option<ConversationPage> {
        self.converse(Converse::Open)
    }

    /// Takes the choice at `index` on the open conversation's page
    /// ([`Converse::Choose`]): the next page, or `None` when the flight
    /// refuses it.
    pub fn choose_in_conversation(&mut self, index: usize) -> Option<ConversationPage> {
        self.converse(Converse::Choose(index))
    }

    /// Closes the open conversation ([`Converse::Close`]): whether one was
    /// open.
    pub fn close_conversation(&mut self) -> bool {
        !self
            .send(FlightCommand::Converse(Converse::Close))
            .is_refused()
    }

    fn converse(&mut self, converse: Converse) -> Option<ConversationPage> {
        match self.send(FlightCommand::Converse(converse)) {
            Answer::Conversation(page) => Some(page),
            _ => None,
        }
    }
}
