//! The spaceport's play keys on touch
//! (rdm:phase/key-bindings/phase-16-touch-pointer-gaps): the spaceport
//! hosts [`PlayKeys`] over whatever it shows (the spaceport itself, a
//! screen open over it, an offer, the mission info screen, the player
//! info screen), so a tap on Map, Pilot or Missions reaches the screen in
//! front as the touch device's press and release of the galaxy map,
//! player info or mission info key, and opens what that key opens there.
//! The strip hides while a mission text or its movie is in front, as
//! those take every input, and while the spaceport can't be shown.

use super::{Port, SpaceportView};
use crate::ui::touch_aids::PlayKeys;
use crate::ui::touch_aids::host::AidHost;

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
    /// Its play keys' strip: where each control is and whether it shows.
    #[must_use]
    pub fn play_keys(&self) -> &PlayKeys {
        &self.play_keys
    }
}
