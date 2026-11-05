//! The mission info key ([`MISSION_INFO_KEY`]), shared by every screen
//! that asks for the mission info screen
//! ([`MissionInfoScreen`](crate::spaceport::missions::MissionInfoScreen))
//! and by that screen, which it closes.

use crate::input::{Input, Key};

/// The key that asks for the mission info screen from the bar, the
/// spaceport, the mission computer, the trade center, the outfitter, the
/// shipyard and its Info panel, an offer, and flight. The original reads
/// it from the player's key settings (`_keyPrefs` +0x50, default 0x22 I,
/// `_loadKeys` @0xce625); I is free on every spaceport screen (the
/// shipyard's Info is Space, as `_ShipyardFilter` reads it @0x60526) and
/// in flight. It is the default of the shared play keys' Mission Info
/// ([`PlayCommand::MissionInfo`](crate::keymap::PlayCommand::MissionInfo)),
/// which the key map binds; the spaceport's screens and the router compare
/// this until they read the key map
/// (rdm:phase/key-bindings/phase-2-command-key-map's steps 4 and 8).
pub const MISSION_INFO_KEY: Key = Key::KeyI;

/// Whether `input` is a fresh press of [`MISSION_INFO_KEY`], which asks
/// for the mission info screen where the original's filters read
/// `_keyPrefs` +0x50, and on that screen itself closes it
/// (`_MissionInfoFilter` @0x9d756-0x9d769).
#[must_use]
pub fn is_mission_info_press(input: &Input) -> bool {
    *input
        == Input::Key {
            key: MISSION_INFO_KEY,
            pressed: true,
            repeat: false,
        }
}
