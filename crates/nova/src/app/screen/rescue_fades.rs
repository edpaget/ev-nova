//! [`AppScreen`]'s glue for the escape pod's rescue on screen. The
//! decisions are the view's: flight's rescue screen
//! (`nova_view::flight::rescue_screen`, run by [`FlightView`]) fades the
//! pod out and back, holds the rescue's `dësc` and texts until their
//! stages and says when the screen is faded; the dialogs' faded-text
//! stage (`nova_view::ui::faded_texts`) fades a text in from black. This
//! glue forwards each piece to the other:
//!
//! - flight gives the rescue's `dësc` once its black is over, and the
//!   router shows it unfaded ([`AppScreen::after_flight_tick`]); flight
//!   doesn't tick while it is open, so its next tick is the dialog's
//!   close, and one that is empty or can't be read shows nothing, so the
//!   next tick follows at once;
//! - flight gives the texts the rescue queued once its second fade is
//!   over, and they are queued over flight on the faded screen, as any
//!   text over flight reads [`FlightView::screen_faded`]
//!   (`AppScreen::over_flight`);
//! - a text that fades the screen in tells flight
//!   ([`AppScreen::tell_faded_in`]), so no closing fade follows;
//! - while the black still covers a text (a plain before-movie plays
//!   under it), flight's cover is drawn over the dialogs
//!   ([`AppScreen::draw_cover_over_texts`]);
//! - what was pressed under the rescue's fades goes to what shows next
//!   ([`AppScreen::replay_rescue_input`]).

#[cfg(doc)]
use nova_view::flight::FlightView;

use super::AppScreen;

impl AppScreen {
    /// Routes what was pressed under the rescue's fades
    /// ([`FlightView::take_rescue_replay`]) to what shows now: the
    /// rescue's `dësc`, its first text, or flight. It runs once a tick's
    /// texts are shown, so they are open to take it, as the original's
    /// queued events reach the next event loop.
    pub(super) fn replay_rescue_input(&mut self) {
        let Some(replay) = self
            .flight
            .as_mut()
            .map(nova_view::flight::FlightView::take_rescue_replay)
        else {
            return;
        };
        for input in replay {
            let _ = self.routed_input(&input);
        }
    }
}
