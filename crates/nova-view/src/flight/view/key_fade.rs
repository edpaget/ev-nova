//! Which physical key fades the cursor, against what the kept frame says
//! fades it ([`FlightFrame::key_fades`](nova_sim::flight_frame::FlightFrame::key_fades)).
//! The gates are the session's (`nova_sim::key_fade`, which records where
//! each of `_HandlePlayer`'s writes of the byte sits); the keys are the
//! view's, in [`fade_key`].

use nova_sim::flight_frame::FadeKey;

use crate::Key;
use crate::flight::sim_port::FlightSim;
use crate::ui::mission_info::MISSION_INFO_KEY;
use crate::ui::player_info::PLAYER_INFO_KEY;

use super::{
    AFTERBURNER_KEY, BOARD_KEY, CLOAK_KEY, DESELECT_KEY, EJECT_KEY, FACE_TARGET_KEY, FIRE_KEY,
    FlightView, HAIL_KEY, HYPER_SELECT_KEY, HYPERSPACE_MODE_KEY, JETTISON_KEY, JUMP_KEY, LAND_KEY,
    MAP_KEY, MENU_CLOSE_KEY, NAV_OFF_KEY, NEAREST_KEY, SECONDARY_KEY, SELECT_KEY,
    SELF_DESTRUCT_KEY, TARGET_KEY,
};

/// The gate `key` sets the byte behind, with Command down or not, or
/// `None` for a key that never sets it: K, X and `-` are polled only with
/// Command down, and Alt, Shift, Command alone, E, F, D, V, C and F12 are
/// polled with no write. The stellar slot keys are
/// [`FlightView::slot_key_polled`]'s.
fn fade_key(key: Key, command: bool) -> Option<FadeKey> {
    let gate = match key {
        TARGET_KEY | NEAREST_KEY | HYPER_SELECT_KEY | SELECT_KEY | BOARD_KEY | HAIL_KEY
        | MAP_KEY | PLAYER_INFO_KEY | MENU_CLOSE_KEY | HYPERSPACE_MODE_KEY | NAV_OFF_KEY
        | FACE_TARGET_KEY => FadeKey::Free,
        Key::Up
        | Key::Down
        | Key::Left
        | Key::Right
        | AFTERBURNER_KEY
        | FIRE_KEY
        | JUMP_KEY
        | CLOAK_KEY => FadeKey::Helm,
        SECONDARY_KEY => FadeKey::Secondary,
        DESELECT_KEY => FadeKey::WeaponSafety,
        LAND_KEY => FadeKey::Land,
        MISSION_INFO_KEY => FadeKey::MissionInfo,
        JETTISON_KEY if command => FadeKey::Free,
        EJECT_KEY if command => FadeKey::Eject,
        SELF_DESTRUCT_KEY if command => FadeKey::SelfDestruct,
        _ => return None,
    };
    Some(gate)
}

impl<C, S: FlightSim> FlightView<C, S> {
    /// Whether `key`, pressed or held, fades the cursor on this frame: a
    /// stellar slot key while its block polls it, as it writes the byte
    /// behind no gate of its own (@0x69e0d), any other by [`fade_key`].
    pub(super) fn key_fades_cursor(&self, key: Key) -> bool {
        let fades = self.frame.key_fades;
        match self.slot_key_polled(key, false) {
            Some(polled) => polled && fades.fades(FadeKey::Free),
            None => fade_key(key, self.held.contains(&Key::Command))
                .is_some_and(|gate| fades.fades(gate)),
        }
    }

    /// Whether this frame's pass sets the byte with what is held: a jump
    /// or an auto-eject with no key at all, or any key held that fades.
    pub(super) fn cursor_keyed(&self) -> bool {
        self.frame.key_fades.unbidden() || self.held.iter().any(|&key| self.key_fades_cursor(key))
    }
}
