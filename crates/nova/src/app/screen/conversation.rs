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
//! none left, when the visit's news opens, once (see the `story_log`
//! module). An accept the flight refused for room is told on the
//! page it was decided on. The spaceport dropped (a take-off, the pilot
//! put away) drops it with it. What each conversation plays is the
//! session's.
//!
//! **The bar.** Under the new game's reading of
//! [`RuleKey::BarStory`](nova_sim::RuleKey::BarStory), the bar is given
//! the story it hosts as it opens ([`FlightView::bar_story`]), and a slot
//! clicked opens its scene the same way
//! ([`AppScreen::open_bar_conversation`], [`FlightView::open_bar_scene`]),
//! over the bar. Its close goes through the same close: the next
//! conversation due opens if any (one the bar's conversation made hold),
//! the news doesn't open twice in a visit, and the bar open is given its
//! story afresh, so a decided offer leaves its slots.

use nova_view::flight::FlightView;
use nova_view::spaceport::{ConversationChoice, ConversationScreen};
use nova_view::{DrawList, Input, Screen};

use super::{AppScreen, ENTERED};

impl AppScreen {
    /// Opens the landed visit's next conversation, if it has one and the
    /// router has metrics to lay it out with, replacing any open one.
    /// With none left, the visit's news opens, once (see the `story_log`
    /// module); a first page's accept refused for room is told on it.
    pub(super) fn open_conversation(&mut self) {
        self.conversation = None;
        let Some(metrics) = self.metrics() else {
            return;
        };
        let page = self.flight.as_mut().and_then(FlightView::open_conversation);
        self.conversation = page.map(|page| ConversationScreen::new(page, metrics));
        if self.conversation.is_some() {
            self.tell_room_refusals();
        } else {
            self.open_news_once();
        }
    }

    /// Opens the scene at `index` of the bar's story over the bar, if
    /// flight opens it and the router has metrics to lay it out with,
    /// replacing any open conversation; its first page's accept refused
    /// for room is told on it (see the module docs).
    pub(super) fn open_bar_conversation(&mut self, index: usize) {
        let Some(metrics) = self.metrics() else {
            return;
        };
        let page = self
            .flight
            .as_mut()
            .and_then(|flight| flight.open_bar_scene(index));
        if let Some(page) = page {
            self.conversation = Some(ConversationScreen::new(page, metrics));
            self.tell_room_refusals();
        }
    }

    /// Gives the bar open its story afresh, as flight answers it now.
    fn refresh_bar_story(&mut self) {
        if let (Some(spaceport), Some(flight)) = (&mut self.spaceport, &mut self.flight)
            && spaceport.open_bar().is_some()
        {
            spaceport.set_bar_story(flight.bar_story());
        }
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
                self.tell_room_refusals();
            }
            Some(ConversationChoice::Close) => {
                self.flight.as_mut().expect(ENTERED).close_conversation();
                self.open_conversation();
                self.refresh_bar_story();
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
