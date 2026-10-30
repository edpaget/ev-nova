//! The keyboard access port: whether the player has asked the system for
//! every control to take the keyboard focus, which Preferences' focus ring
//! follows ([`AppScreen`](super::AppScreen) asks it each time Preferences
//! opens, so a change in the system's settings shows at the next opening).
//!
//! The original's Preferences is Cocoa, where `AppKit` puts every control
//! in the Tab loop only with full keyboard access on. The macOS adapter
//! reads that setting (`crate::platform`'s `AppKitKeyboardAccess`);
//! everywhere else [`NoKeyboardAccessSetting`] says there is none, and
//! Preferences keeps its focus ring.

use nova_view::ui::FocusRing;

/// Whether the player has turned on keyboard access to every control.
pub trait KeyboardAccess: std::fmt::Debug {
    /// `Some(true)` with full keyboard access on, `Some(false)` with it
    /// off, and `None` where the platform has no such setting.
    fn full_keyboard_access(&self) -> Option<bool>;
}

/// The platforms with no keyboard access setting: it answers `None`.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoKeyboardAccessSetting;

impl KeyboardAccess for NoKeyboardAccessSetting {
    fn full_keyboard_access(&self) -> Option<bool> {
        None
    }
}

/// Preferences' focus ring for the setting `access` answers: on with full
/// keyboard access on or with no such setting, off with it off.
#[must_use]
pub fn prefs_focus_ring(access: Option<bool>) -> FocusRing {
    match access {
        Some(false) => FocusRing::Off,
        Some(true) | None => FocusRing::Tab,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_focus_ring_is_off_only_with_full_keyboard_access_off() {
        assert_eq!(prefs_focus_ring(Some(true)), FocusRing::Tab);
        assert_eq!(prefs_focus_ring(None), FocusRing::Tab);
        assert_eq!(prefs_focus_ring(Some(false)), FocusRing::Off);
    }

    #[test]
    fn with_no_setting_there_is_no_answer() {
        assert_eq!(NoKeyboardAccessSetting.full_keyboard_access(), None);
    }
}
