//! Flight's side of its on-screen touch controls
//! ([`crate::flight::touch_controls`]): the touches it claims, the inputs
//! the overlay raises, drained for the router to run back through its
//! input path as it does a key's ([`FlightView::take_touch_controls`]),
//! when the overlay shows, and drawing it.
//!
//! The overlay shows while touch was used last and flight flies live
//! ([`FlightView::flies_live`]): no map, movie, rescue screen or failure,
//! and a fate that still takes commands. Flight keeps the overlay told
//! after every input and tick; flight no longer live lets go of
//! everything the overlay held, as a dialog opening over flight does
//! ([`FlightView::let_go_of_input`]), so nothing stays held under the
//! map, a movie or a dialog.

use nova_sim::Takes;

use super::FlightView;
use crate::draw::DrawList;
use crate::flight::sim_port::FlightSim;
use crate::flight::touch_controls::draw;
use crate::geometry::Point;
use crate::input::{Input, Touch};

impl<C, S: FlightSim> FlightView<C, S> {
    /// Whether flight flies live, for the touch controls: no map, movie,
    /// rescue screen or failure over it, and a fate that still takes
    /// commands.
    pub(super) fn flies_live(&self) -> bool {
        !self.map_open
            && self.movie.is_none()
            && self.rescue.is_none()
            && self.failure.is_none()
            && !self.frame.hold_expired
            && self.frame.fate.row().input != Takes::Nothing
    }

    /// Tells the overlay the view's size, whether flight flies live and
    /// whether the escort menu shows: after every input and tick.
    pub(super) fn sync_touch_controls(&mut self) {
        self.touch_controls.set_view(self.view);
        self.touch_controls
            .set_escort_menu(self.escort_menu.is_open());
        self.touch_controls.set_live(self.flies_live());
    }

    /// Hands a claimed finger's touch to the overlay.
    pub(super) fn touch_input(&mut self, touch: Touch) {
        self.touch_controls.touch(touch);
    }

    /// Whether a touch starting at `at` is the overlay's.
    #[must_use]
    pub(super) fn claims_touch_control(&self, at: Point) -> bool {
        self.flies_live() && self.touch_controls.claims(at)
    }

    /// What the overlay raised since the last call, in order: the router
    /// runs each back through its own input path from the top, as it does
    /// a key's.
    pub fn take_touch_controls(&mut self) -> Vec<Input> {
        self.touch_controls.take_queued()
    }

    /// Draws the overlay, while it shows: the router draws it over flight
    /// with nothing over it.
    pub fn draw_touch_controls(&self, list: &mut DrawList) {
        if self.flies_live() {
            draw::draw(&self.touch_controls, list);
        }
    }
}
