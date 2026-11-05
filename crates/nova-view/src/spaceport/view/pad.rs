//! The spaceport with the pad (rdm:phase/key-bindings/phase-9-gamepad-spaceport).
//!
//! - **Who tells whom.** The app tells the spaceport the surroundings
//!   (the router in `nova`, as it builds the spaceport and on each
//!   change). The spaceport keeps them ([`Told`]) and tells every screen it
//!   holds, not only the one in front: its own dialog, the screen open
//!   over it, the mission texts and the info beneath, the landing's offer
//!   and the player info screen. It tells each screen it opens as it opens
//!   it, and those that open others pass them on in turn (the bar, the
//!   venues' offers, the modal hosts, the shipyard's panel).
//! - **The surroundings are no player input.** They are caught at the top
//!   of its input, before the routing, the close keys, the wait replay
//!   and the gamble's settling, so nothing else runs on them.
//! - **The port's targets** are its dialog's: the service buttons, Leave
//!   and the landscape (item 5), a click area. The pad's focus starts on
//!   Leave, the default, so South at once takes off, as Return does.
//! - **The landscape.** South's press on it plays the ambient sound, as a
//!   press on it does (`_PortFilter` returns item 5 on the mouseDown
//!   @0x4f2cc; `_DoPortDialog` plays it @0x5fe59).
//!
//! The original moves no focus in the spaceport (`_MyModalDialog`
//! @0x59954 never calls `DialogSelect`; no port filter moves a focus), so
//! all of this is a new feature with no `Rulebook` key.

use crate::input::{Device, Input};
use crate::keymap::{Command, UiCommand};
use crate::screen::Screen;
use crate::sound::Sound;
use crate::ui::pad_focus::from_pad;

use super::{LANDSCAPE_ITEM, SpaceportView};

impl SpaceportView {
    /// The device the player used last, as the app last told it; the
    /// keyboard before any.
    #[must_use]
    pub fn device(&self) -> Device {
        self.told.device()
    }

    /// Keeps `input` if it is the surroundings, and tells every screen
    /// the spaceport holds; whether it was.
    pub(super) fn follow_surroundings(&mut self, input: &Input) -> bool {
        if !self.told.keep(input) {
            return false;
        }
        self.keys.follow(input);
        if let Ok(port) = &mut self.port {
            port.dialog.input(input);
        }
        if let Some(open) = &mut self.open {
            open.screen_mut().input(input);
        }
        if let Some(dialogs) = &mut self.dialogs {
            dialogs.input(input);
        }
        if let Some(missioning) = &mut self.missioning {
            missioning.landing.input(input, &mut self.sounds);
        }
        if let Some(screen) = &mut self.player_info {
            screen.input(input);
        }
        true
    }

    /// Tells the screen just opened over the spaceport the surroundings.
    pub(super) fn tell_open(&mut self) {
        if let Some(open) = &mut self.open {
            self.told.tell(open.screen_mut());
        }
    }

    /// Tells the player info screen just opened the surroundings.
    pub(super) fn tell_player_info(&mut self) {
        if let Some(screen) = &mut self.player_info {
            self.told.tell(screen);
        }
    }

    /// South's press on the landscape, where the pad's highlight is,
    /// plays the ambient sound, as a press on it does.
    pub(super) fn pad_press_landscape(&mut self, input: &Input, fresh: Option<Command>) {
        if !from_pad(input) || fresh != Some(Command::Ui(UiCommand::Activate)) {
            return;
        }
        if let (Ok(port), Some(ambient)) = (&self.port, &self.ambient)
            && port.dialog.pad_highlight() == Some(LANDSCAPE_ITEM)
        {
            self.sounds.extend(ambient.pressed().map(Sound::Ui));
        }
    }
}
