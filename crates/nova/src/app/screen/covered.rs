//! What the router shows over a side covers it
//! (rdm:phase/key-bindings/phase-16-touch-pointer-gaps): after each input,
//! tick and menu command the router tells the spaceport and flight
//! whether something of its own is in front of them
//! ([`Screen::set_covered`]), so a touch aid of theirs (the spaceport's
//! play keys, the toggle of the screen open over it, the course map's
//! Plot Course) shows, takes taps and stays armed only while its screen
//! is in front.
//!
//! - **The spaceport** is covered by any overlay open, and by flight's
//!   course map over it.
//! - **Flight**, and so its course map, is covered by any overlay in front
//!   of it. A mission dialog's map over flight is drawn in front of the
//!   dialogs it opened from, so only an overlay the router puts in front
//!   of that map ([`AppScreen::overlay_in_front`]) covers it.
//! - **The galaxy side** (the developer's viewer, and its Find) is covered
//!   by any overlay open.
//! - **Each open overlay** is covered unless it is the one in front, so a
//!   text dialog under another (New Pilot, the christening, the capture
//!   naming) asks for no text entry and draws no on-screen keyboard
//!   (rdm:phase/key-bindings/phase-17-text-entry-port). A mission dialog's
//!   map is told through flight.
//!
//! What a covered aid or text dialog does, and how the spaceport and
//! flight pass the word on, is `nova_view`'s (`ui::touch_aids::host`,
//! `ui::entry_keyboard`).

use nova_view::Screen;

use super::AppScreen;
use super::routing::Overlay;

impl AppScreen {
    /// Tells the spaceport, flight, the galaxy side and each open overlay
    /// whether anything the router shows covers them (see the module
    /// docs).
    pub(super) fn tell_covered(&mut self) {
        let front = self.overlay_in_front();
        let spaceport = front.is_some() || self.port_map_open();
        let flight = front.is_some_and(|overlay| overlay != Overlay::DialogMap);
        if let Some(view) = &mut self.spaceport {
            view.set_covered(spaceport);
        }
        if let Some(view) = &mut self.flight {
            view.set_covered(flight);
        }
        self.galaxy.set_covered(front.is_some());
        for overlay in self.overlays_opened() {
            if let Some(screen) = self.overlay_screen_mut(overlay) {
                screen.set_covered(front != Some(overlay));
            }
        }
    }
}
