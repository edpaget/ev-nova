//! The spaceport's play keys on touch
//! (rdm:phase/key-bindings/phase-16-touch-pointer-gaps): the spaceport
//! hosts [`PlayKeys`] over whatever it shows (the spaceport itself, a
//! screen open over it, an offer, the mission info screen, the player
//! info screen), so a tap on Map, Pilot or Missions reaches the screen in
//! front as the touch device's press and release of the galaxy map,
//! player info or mission info key, and opens what that key opens there.
//! The strip hides while a mission text or its movie is in front, as
//! those take every input, while the spaceport can't be shown, and while
//! whoever holds the spaceport covers it ([`Screen::set_covered`](crate::screen::Screen::set_covered): the
//! course map over it, or a dialog of the app's).
//!
//! The spaceport also tells the screen open over it whether it is covered
//! ([`SpaceportView::settle_cover`]), so that screen's own touch aid (the
//! trade center's, the outfitter's or the bar's race's toggle) shows and
//! stays armed only while it is in front.

use super::{Port, SpaceportView};
use crate::ui::touch_aids::PlayKeys;
use crate::ui::touch_aids::host::{self, AidHost, TouchAid};

/// The strip over the spaceport's dialog, clear of it.
pub(super) fn play_keys(port: &Result<Port, String>) -> PlayKeys {
    PlayKeys::classic(port.as_ref().ok().map(|port| port.dialog.bounds()))
}

impl AidHost for SpaceportView {
    type Aid = PlayKeys;

    fn aid(&self) -> &PlayKeys {
        &self.play_keys
    }

    fn aid_mut(&mut self) -> &mut PlayKeys {
        &mut self.play_keys
    }

    fn aid_live(&self) -> bool {
        self.port.is_ok()
            && self
                .dialogs
                .as_ref()
                .is_none_or(|dialogs| dialogs.text().is_none() && !dialogs.playing())
    }
}

impl SpaceportView {
    /// [`Screen::set_covered`](crate::screen::Screen::set_covered): whether whoever holds the spaceport covers
    /// it. Its play keys hide under it, and so does whatever the screen
    /// open hosts.
    pub(super) fn cover_aids(&mut self, covered: bool) {
        host::cover(self, covered);
        self.settle_cover();
    }

    /// Tells the screen open whether anything covers it: whatever covers
    /// the spaceport, or what the spaceport shows over it (a mission text
    /// or its movie, the mission info or player info screen, or the
    /// spaceport's own offer). Run after each input and tick, and by each
    /// call that opens one of those; whoever holds the spaceport tells it
    /// after each of its own inputs and ticks too, which settles it again.
    pub(super) fn settle_cover(&mut self) {
        let covered = self.play_keys.covered()
            || self.mission_dialogs_open()
            || self.player_info.is_some()
            || self
                .missioning
                .as_ref()
                .is_some_and(|missioning| missioning.landing.dialog().is_some());
        if let Some(open) = &mut self.open {
            open.screen_mut().set_covered(covered);
        }
    }

    /// Its play keys' strip: where each control is and whether it shows.
    #[must_use]
    pub fn play_keys(&self) -> &PlayKeys {
        &self.play_keys
    }
}
