//! [`AppScreen`]'s texts over the jump's white: a text shown over flight
//! while the warp has lit the screen white shows on the faded screen, as
//! `_ShowTextDialog` shows it with `_screenFaded` set (`_FadeWhiteIn`
//! sets it, @0x5475). The decisions are the view's: the dialogs'
//! (`nova_view::ui::faded_texts`) and flight's
//! ([`FlightView::screen_faded`], [`FlightView::screen_faded_in`]). This
//! glue tells each what the other did:
//!
//! - the mission dialogs over flight, and the About text opened over it,
//!   are told the screen is faded as their texts are queued
//!   ([`AppScreen::white_faded`]);
//! - while the white still covers the screen (a plain before-movie
//!   plays under it), flight's white runs on, though flight is paused,
//!   and is drawn over the dialogs, as the original's display fade is
//!   asynchronous and covers the whole display;
//! - once a text has faded the screen in from black, flight is told, so
//!   the white is gone and the warp lights it again on its next pass.
//!
//! The router queues a text after the tick that queued it and reads the
//! white then, which is the white the original's flag shows: it lasts
//! until the pass after the arrival.

use std::time::Duration;

use nova_view::DrawList;
#[cfg(doc)]
use nova_view::flight::FlightView;
use nova_view::ui::mission_dialogs::MissionDialogs;

use super::{AppScreen, Side};

impl AppScreen {
    /// Whether flight shows with the jump's white leaving the screen
    /// faded ([`FlightView::screen_faded`]).
    pub(super) fn white_faded(&self) -> bool {
        self.side == Side::Flight
            && self
                .flight
                .as_ref()
                .is_some_and(nova_view::flight::FlightView::screen_faded)
    }

    /// Whether the white still covers the texts over flight: the mission
    /// dialogs' or the About text's.
    fn texts_under_white(&self) -> bool {
        self.side == Side::Flight
            && (self.missions.as_ref().is_some_and(MissionDialogs::faded)
                || self.about.as_ref().is_some_and(MissionDialogs::faded))
    }

    /// Tells flight once a text over it has faded the screen in
    /// ([`FlightView::screen_faded_in`]).
    pub(super) fn tell_faded_in(&mut self) {
        let missions = self
            .missions
            .as_mut()
            .is_some_and(MissionDialogs::take_faded_in);
        let about = self
            .about
            .as_mut()
            .is_some_and(MissionDialogs::take_faded_in);
        if (missions || about)
            && let Some(flight) = &mut self.flight
        {
            flight.screen_faded_in();
        }
    }

    /// Runs flight's white on by `dt` under the texts it still covers
    /// ([`FlightView::advance_white`]).
    pub(super) fn advance_white_under_texts(&mut self, dt: Duration) {
        if self.texts_under_white()
            && let Some(flight) = &mut self.flight
        {
            flight.advance_white(dt);
        }
    }

    /// Draws flight's white over the texts it still covers, over the whole
    /// widened view.
    pub(super) fn draw_white_over_texts(&self, list: &mut DrawList) {
        if self.texts_under_white()
            && let Some(effect) = self.flight.as_ref().and_then(|flight| flight.jump_effect())
        {
            effect.draw_fade(list, self.widened);
        }
    }
}
