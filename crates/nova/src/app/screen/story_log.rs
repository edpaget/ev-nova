//! [`AppScreen`]'s story log: the new game's own read-only panel
//! ([`StoryLogScreen`]) for its news and its active storyline missions,
//! opened over the spaceport or
//! over the paused flight. It is one of the router's overlays: while it
//! is open it takes every input in front of what is under it, flight
//! stays paused under it, and it is drawn over it.
//!
//! **The news** is offered once a visit, as the landed visit's last page:
//! when the last conversation has closed (or there was none), it opens if
//! the system the ship is in has heard anything the pilot hasn't read
//! ([`FlightView::heard_news`], each item with its read flag), and lists
//! everything heard there, read items too. The flag that keeps the offer
//! to once a visit resets as the spaceport opens; what the pilot has read
//! is the session's, kept with the pilot and saved.
//! A dock scan's find needs no page here: a press that scans never lands,
//! and the message line keeps the find over a denial (`FlightView::land`).
//!
//! **Reading.** The player's close of a news log (Return, Enter, Escape,
//! or a press off its scroll arrows) marks read every item it lists,
//! drawn or reached only by scrolling ([`FlightView::read_news`]), however
//! it opened. A log the router drops unclosed marks nothing, so it opens
//! again at the next landing.
//!
//! **The bar's news.** Under the new game's reading of
//! [`RuleKey::BarStory`](nova_sim::RuleKey::BarStory), the bar's Holovid
//! opens the news log on the news its story lists, the news heard in the
//! system, read or not, saying so when nothing has been heard
//! ([`AppScreen::open_bar_news`]). It beeps as it opens and closes, as the
//! stock news does, and closes back to the bar. The landing's news stays
//! silent.
//!
//! **The missions.** The mission info key, in flight or in port, opens the
//! missions log in place of the stock mission info screen when the
//! flight's answer lists storyline missions ([`MissionInfo::story`]), so
//! it opens without the interface file's `DLOG` 1012. Its close over
//! flight tells flight, as the stock screen's does
//! ([`FlightView::close_info_dialog`]).
//!
//! **The map.** The missions log's map key asks for its selected
//! mission's map ([`StoryLogScreen::take_map_request`]), and the router
//! opens flight's course map for it ([`MapOpener::StoryLog`], marked by the
//! session for that row) with the log left open under it, as mission
//! info stays open under its map. Over flight it opens as a mission
//! dialog's map does there (`AppScreen::open_flight_dialog_map`): the
//! dialog map is in front of the log among the overlays, and the log is
//! drawn under it. In port it opens the same way, the spaceport's pointer
//! cancelled and its keys released first, as its dialogs' maps do: the
//! log is no overlay while the map is open over the spaceport, so the
//! map takes the input through the spaceport's, and the log is drawn
//! between the spaceport and the map. Either way the map's close
//! (Escape, Return, M) leaves the log in front again, and its own close
//! runs as before.
//!
//! **The refusal.** After a conversation's first page and after each
//! choice, an accept the flight refused for room
//! ([`FlightView::take_room_refusals`]) is told on that page
//! ([`ConversationScreen::set_notice`](nova_view::spaceport::ConversationScreen::set_notice)).
//!
//! **Scrolling.** A log longer than its panel scrolls by the router's Up
//! and Down (the log reads the router's bindings), by its arrows, and by
//! the wheel under [`RuleKey::DialogTextWheel`](nova_sim::RuleKey::DialogTextWheel)'s
//! other reading only, as the router drops every wheel under the
//! engine's (`AppScreen::route`).
//!
//! A router with no metrics shows none of it. The spaceport dropped (a
//! take-off, the pilot put away, the window closed) and flight dropped
//! drop it, unclosed.
//!
//! [`FlightView::heard_news`]: nova_view::flight::FlightView::heard_news
//! [`FlightView::read_news`]: nova_view::flight::FlightView::read_news
//! [`FlightView::close_info_dialog`]: nova_view::flight::FlightView::close_info_dialog
//! [`FlightView::take_room_refusals`]: nova_view::flight::FlightView::take_room_refusals

use nova_sim::MissionInfo;
use nova_view::flight::MapOpener;
use nova_view::sound::{Sound, UiSound};
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
    /// The ids of the news items it lists, which its close marks read:
    /// none for the missions log and for a log that has heard nothing.
    pub(super) news: Vec<String>,
}

impl AppScreen {
    /// Opens `log`, listing the news items `news`, followed by `then`, if
    /// the router has metrics to lay it out with: whether it opened.
    fn open_story_log(&mut self, log: StoryLog, then: AfterLog, news: Vec<String>) -> bool {
        let Some(metrics) = self.metrics() else {
            return false;
        };
        self.story_log = Some(OpenLog {
            screen: StoryLogScreen::new(log, metrics).with_bindings(self.bindings.clone()),
            then,
            news,
        });
        true
    }

    /// The landed visit's first pages, as the spaceport opens: its news
    /// not offered yet, and its conversations (see the module docs).
    pub(super) fn open_landed_pages(&mut self) {
        self.news_offered = false;
        self.open_conversation();
    }

    /// Opens the news heard where the ship is, once a visit, when any of
    /// it is unread, listing it all (see the module docs).
    pub(super) fn open_news_once(&mut self) {
        if self.news_offered {
            return;
        }
        self.news_offered = true;
        let Some(flight) = self.flight.as_mut() else {
            return;
        };
        let news = flight.heard_news();
        if !news.iter().any(|item| !item.read) {
            return;
        }
        let log = news_log(&news, flight.names());
        let ids = news.into_iter().map(|item| item.id).collect();
        self.open_story_log(log, AfterLog::Nothing, ids);
    }

    /// Opens the news log on the bar's story's news, with the stock news's
    /// beeps, over the bar (see the module docs).
    pub(super) fn open_bar_news(&mut self) {
        let (Some(spaceport), Some(flight)) = (&self.spaceport, &self.flight) else {
            return;
        };
        let Some(story) = spaceport.bar_story() else {
            return;
        };
        let log = news_log(&story.news, flight.names());
        let ids = story.news.iter().map(|item| item.id.clone()).collect();
        if self.open_story_log(log, AfterLog::Nothing, ids)
            && let Some(open) = &mut self.story_log
        {
            open.screen = open.screen.clone().with_sounds(
                Sound::Ui(UiSound::NewsOpened),
                Sound::Ui(UiSound::NewsClosed),
            );
        }
    }

    /// Opens the missions log on `info`'s storyline missions, when it
    /// lists any, followed by `then`: whether it opened.
    pub(super) fn open_missions_log(&mut self, info: &MissionInfo, then: AfterLog) -> bool {
        if info.story.is_empty() {
            return false;
        }
        let flight = self.flight.as_ref().expect(ENTERED);
        let log = missions_log(&info.story, flight.names());
        self.open_story_log(log, then, Vec::new())
    }

    /// Opens flight's course map for the missions log's row `row`, with
    /// the log left open under it (see the module docs), as a mission
    /// dialog's map opens over flight, the keys held let go
    /// (`AppScreen::open_flight_dialog_map`). Landed, the spaceport's
    /// pointer and keys are let go first, as its own dialogs' maps let
    /// them go; in flight there is no spaceport.
    fn open_story_map(&mut self, row: usize) {
        if let Some(spaceport) = &mut self.spaceport {
            spaceport.cancel_pointer();
            spaceport.release_keys();
        }
        self.open_flight_dialog_map(MapOpener::StoryLog(row));
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

    /// The story log's input: it closes on Return, Escape or a click, its
    /// close marks read the news it lists, and what follows its close runs
    /// (see the module docs).
    pub(super) fn story_log_input(&mut self, input: &Input) -> ScreenAction {
        let Some(open) = &mut self.story_log else {
            return ScreenAction::None;
        };
        open.screen.input(input);
        if let Some(row) = open.screen.take_map_request() {
            self.open_story_map(row);
            return ScreenAction::None;
        }
        if !open.screen.take_closed() {
            return ScreenAction::None;
        }
        // Its closing beep outlives it.
        self.sounds.extend(open.screen.take_sounds());
        let then = open.then;
        let news = std::mem::take(&mut open.news);
        self.story_log = None;
        if !news.is_empty()
            && let Some(flight) = &mut self.flight
        {
            flight.read_news(news);
        }
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
