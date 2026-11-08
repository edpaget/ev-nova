//! The mission info key, the shared play keys' Mission Info
//! ([`PlayCommand::MissionInfo`]), shared by every screen that asks for
//! the mission info screen
//! ([`MissionInfoScreen`](crate::spaceport::missions::MissionInfoScreen))
//! and by that screen, which it closes. It asks for the screen from the
//! bar, the spaceport, the mission computer, the trade center, the
//! outfitter, the shipyard and its Info panel, an offer, and flight. The
//! original reads it from the player's key settings (`_keyPrefs` +0x50,
//! default 0x22 I, `_loadKeys` @0xce625); I is free on every spaceport
//! screen (the shipyard's Info is Space, as `_ShipyardFilter` reads it
//! @0x60526) and in flight.

use crate::input::Key;
use crate::keymap::{PlayCommand, SharedBindings};

/// The key the mission info key's command,
/// [`PlayCommand::MissionInfo`](crate::keymap::PlayCommand::MissionInfo),
/// is bound to in `bindings`, if any: what an answered offer's close
/// waits on. A key wait compares the key itself (the phase's fixed keys).
pub(crate) fn mission_info_key(bindings: &SharedBindings) -> Option<Key> {
    bindings.with(|bindings| {
        bindings
            .keyboard
            .combo(PlayCommand::MissionInfo.into())
            .map(|combo| combo.key)
    })
}
