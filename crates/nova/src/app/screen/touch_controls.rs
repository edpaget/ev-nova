//! The router's side of flight's touch controls
//! (rdm:phase/key-bindings/phase-15-touch-flight-controls): it claims a
//! touch for flight only while flight shows with nothing over it, hands
//! every claimed finger to flight's overlay, draws the overlay over
//! flight, and runs what the overlay raises back through its own input
//! from the top ([`AppScreen::feed_touch_controls`]), as it does a key.
//!
//! The router reads two flight commands in front of flight, Pause (which
//! leaves flight) and Double Time, and its readers must see each release
//! too; a command raised inside flight would reach neither. So each
//! input the overlay raises passes the away gate, the live input, the
//! holds, Double Time and flight's Pause before it reaches flight, or
//! whatever is over flight by then.
//!
//! The feed runs after every input, after the keys or the pointer are
//! let go of, and after every tick: each call that can make the overlay
//! raise something. It ends: a fed input can only make the overlay
//! release what it held, which raises nothing more.

use nova_view::{DrawList, Input, Point, Screen, ScreenAction, Showing};

use super::AppScreen;

impl AppScreen {
    /// Whether a touch starting at `at` is flight's overlay's: flight
    /// shows with nothing over it, and its overlay claims the point.
    pub(super) fn claims_touch_control(&self, at: Point) -> bool {
        self.showing() == Showing::Flight
            && self
                .flight
                .as_ref()
                .is_some_and(|flight| flight.claims_touch(at))
    }

    /// Hands a claimed finger's touch to flight's overlay, whatever shows,
    /// so a finger that lifts after its own press opened a dialog still
    /// reaches the overlay.
    pub(super) fn touch_to_flight(&mut self, input: &Input) {
        if let Some(flight) = &mut self.flight {
            flight.input(input);
        }
    }

    /// Draws flight's overlay over flight with nothing over it.
    pub(super) fn draw_touch_controls(&self, list: &mut DrawList) {
        if self.showing() == Showing::Flight
            && let Some(flight) = &self.flight
        {
            flight.draw_touch_controls(list);
        }
    }

    /// Runs what flight's overlay raised back through the router's input
    /// from the top, as a key's, until it raises nothing more: a quit any
    /// of them asks for, or nothing.
    pub(super) fn feed_touch_controls(&mut self) -> ScreenAction {
        let mut action = ScreenAction::None;
        loop {
            let queued = self
                .flight
                .as_mut()
                .map(nova_view::flight::FlightView::take_touch_controls)
                .unwrap_or_default();
            if queued.is_empty() {
                return action;
            }
            for input in queued {
                if self.input(&input) == ScreenAction::Quit {
                    action = ScreenAction::Quit;
                }
            }
        }
    }
}
