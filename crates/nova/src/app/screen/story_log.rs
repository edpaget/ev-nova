//! [`AppScreen`]'s story log: the new game's own read-only panel
//! ([`StoryLogScreen`]) for its news and its active storyline missions,
//! opened over the spaceport or
//! over the paused flight. It is one of the router's overlays: while it
//! is open it takes every input in front of what is under it, flight
//! stays paused under it, and it is drawn over it.
//!
//! **The news** opens once a visit, as the landed visit's last page: when
//! the last conversation has closed (or there was none) and the system
//! the ship is in has heard any ([`FlightView::heard_news`]). The flag
//! that keeps it to once a visit resets as the spaceport opens. A dock
//! scan's find needs no page here: a press that scans never lands, and
//! the message line keeps the find over a denial (`FlightView::land`).
//!
//! **The missions.** The mission info key, in flight or in port, opens the
//! missions log in place of the stock mission info screen when the
//! flight's answer lists storyline missions ([`MissionInfo::story`]), so
//! it opens without the interface file's `DLOG` 1012. Its close over
//! flight tells flight, as the stock screen's does
//! ([`FlightView::close_info_dialog`]).
//!
//! **The refusal.** After a conversation's first page and after each
//! choice, an accept the flight refused for room
//! ([`FlightView::take_room_refusals`]) is told on that page
//! ([`ConversationScreen::set_notice`](nova_view::spaceport::ConversationScreen::set_notice)).
//!
//! A router with no metrics shows none of it. The spaceport dropped (a
//! take-off, the pilot put away) and flight dropped drop it.
//!
//! [`FlightView::heard_news`]: nova_view::flight::FlightView::heard_news
//! [`FlightView::close_info_dialog`]: nova_view::flight::FlightView::close_info_dialog
//! [`FlightView::take_room_refusals`]: nova_view::flight::FlightView::take_room_refusals

use nova_sim::MissionInfo;
use nova_view::spaceport::conversation::room_refusal_text;
use nova_view::ui::story_log::{StoryLog, StoryLogScreen, missions_log, news_log};
use nova_view::{Input, Screen, ScreenAction};

use super::{AppScreen, ENTERED};

/// What comes after the open log closes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum AfterLog {
    /// The mission info key's log over flight: its close tells flight.
    InfoOverFlight,
    /// Nothing.
    Nothing,
}

/// The story log open, and what follows its close.
#[derive(Clone, Debug)]
pub(super) struct OpenLog {
    /// The screen.
    pub(super) screen: StoryLogScreen,
    /// What follows its close.
    pub(super) then: AfterLog,
}

impl AppScreen {
    /// Opens `log`, followed by `then`, if the router has metrics to lay
    /// it out with: whether it opened.
    fn open_story_log(&mut self, log: StoryLog, then: AfterLog) -> bool {
        let Some(metrics) = self.metrics() else {
            return false;
        };
        self.story_log = Some(OpenLog {
            screen: StoryLogScreen::new(log, metrics),
            then,
        });
        true
    }

    /// The landed visit's first pages, as the spaceport opens: its news
    /// not offered yet, and its conversations (see the module docs).
    pub(super) fn open_landed_pages(&mut self) {
        self.news_read = false;
        self.open_conversation();
    }

    /// Opens the news heard where the ship is, once a visit, when there is
    /// any (see the module docs).
    pub(super) fn open_news_once(&mut self) {
        if self.news_read {
            return;
        }
        self.news_read = true;
        let Some(flight) = self.flight.as_mut() else {
            return;
        };
        let news = flight.heard_news();
        if news.is_empty() {
            return;
        }
        let log = news_log(&news, flight.names());
        self.open_story_log(log, AfterLog::Nothing);
    }

    /// Opens the missions log on `info`'s storyline missions, when it
    /// lists any, followed by `then`: whether it opened.
    pub(super) fn open_missions_log(&mut self, info: &MissionInfo, then: AfterLog) -> bool {
        if info.story.is_empty() {
            return false;
        }
        let flight = self.flight.as_ref().expect(ENTERED);
        let log = missions_log(&info.story, flight.names());
        self.open_story_log(log, then)
    }

    /// Tells the open conversation's page the accept the flight last
    /// refused for room, if any.
    pub(super) fn tell_room_refusals(&mut self) {
        let Some(flight) = self.flight.as_mut() else {
            return;
        };
        let refused = flight.take_room_refusals();
        if let (Some(refusal), Some(screen)) = (refused.last(), &mut self.conversation) {
            screen.set_notice(Some(room_refusal_text(refusal)));
        }
    }

    /// The story log's input: it closes on Return, Escape or a click, and
    /// what follows its close runs (see the module docs).
    pub(super) fn story_log_input(&mut self, input: &Input) -> ScreenAction {
        let Some(open) = &mut self.story_log else {
            return ScreenAction::None;
        };
        open.screen.input(input);
        if !open.screen.take_closed() {
            return ScreenAction::None;
        }
        let then = open.then;
        self.story_log = None;
        match then {
            AfterLog::InfoOverFlight => {
                if let Some(flight) = &mut self.flight {
                    flight.close_info_dialog();
                }
            }
            AfterLog::Nothing => {}
        }
        ScreenAction::None
    }
}
