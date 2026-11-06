//! [`AppScreen`]'s conversation: the landed visit's conversations, each
//! opened over the spaceport in turn, a page at a time.
//!
//! As the spaceport opens on landing, the router asks flight for the
//! visit's next conversation ([`FlightView::open_conversation`]): a
//! mission's completion or offer, or the stellar's own, which the session
//! picks. If there is one, the router shows it in a
//! [`ConversationScreen`], laid out with the router's metrics (a router
//! given none shows no conversation). While it is open it takes every
//! input the spaceport would, and is drawn over it. Each choice goes back
//! to flight ([`FlightView::choose_in_conversation`]), and the next page
//! shows; the ended page's close goes back too
//! ([`FlightView::close_conversation`]), and the router asks for the next
//! conversation, showing it, or dropping the screen once the session has
//! none left. The spaceport dropped (a take-off, the pilot put away) drops
//! it with it. What each conversation plays is the session's.

use nova_view::flight::FlightView;
use nova_view::spaceport::{ConversationChoice, ConversationScreen};
use nova_view::{DrawList, Input, Screen};

use super::{AppScreen, ENTERED};

impl AppScreen {
    /// Opens the landed visit's next conversation, if it has one and the
    /// router has metrics to lay it out with, replacing any open one.
    pub(super) fn open_conversation(&mut self) {
        self.conversation = None;
        let Some(metrics) = self.metrics() else {
            return;
        };
        let page = self.flight.as_mut().and_then(FlightView::open_conversation);
        self.conversation = page.map(|page| ConversationScreen::new(page, metrics));
    }

    /// Gives `input` to the open conversation, acting on its choice or
    /// close: whether one was open to take it.
    pub(super) fn conversation_input(&mut self, input: &Input) -> bool {
        let Some(screen) = &mut self.conversation else {
            return false;
        };
        screen.input(input);
        match screen.take_choice() {
            Some(ConversationChoice::Choose(index)) => {
                let flight = self.flight.as_mut().expect(ENTERED);
                if let Some(page) = flight.choose_in_conversation(index) {
                    screen.set_page(page);
                }
            }
            Some(ConversationChoice::Close) => {
                self.flight.as_mut().expect(ENTERED).close_conversation();
                self.open_conversation();
            }
            None => {}
        }
        true
    }

    /// Draws the open conversation, if any, over what is drawn.
    pub(super) fn draw_conversation(&self, list: &mut DrawList) {
        if let Some(screen) = &self.conversation {
            screen.draw(list);
        }
    }
}
