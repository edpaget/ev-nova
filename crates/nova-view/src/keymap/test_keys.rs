//! The shared play keys' default keys, spelt as keys for the tests that
//! press them: [`KeyMap::default`](super::KeyMap) binds them, as the
//! keyboard module's `the_play_keys_defaults_are_m_p_and_i` pins.

use crate::Key;

/// Player Info's default key.
pub(crate) const PLAYER_INFO_KEY: Key = Key::KeyP;

/// Mission Info's default key.
pub(crate) const MISSION_INFO_KEY: Key = Key::KeyI;
