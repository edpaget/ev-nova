//! The sounds screens make, as events: what happened, not what to play.
//! The audio side decides which `snd ` each one plays, if any.
//!
//! - [`UiSound`]: the interface's own, a button pressed and released.
//! - [`SimSound`]: the flight session's, re-exported from `nova_sim`.
//! - [`Sound`]: either, as a screen reports it through
//!   [`Screen::take_sounds`](crate::Screen::take_sounds).
//! - [`SoundPrefs`]: the player's sound preferences as the Preferences
//!   dialog shows them, part of the [`Prefs`](crate::Prefs) a screen
//!   reports through [`Screen::take_prefs`](crate::Screen::take_prefs).

pub use nova_sim::SimSound;

/// A sound the interface makes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiSound {
    /// A button was pressed: the pointer went down on it.
    ButtonDown,
    /// A pressed button popped back up: the pointer was let go, on the
    /// button or off it.
    ButtonUp,
}

/// A sound a screen reports.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sound {
    /// From the flight session.
    Sim(SimSound),
    /// From the interface.
    Ui(UiSound),
}

/// The loudest volume level; levels run from 0 (silent) to this.
pub const MAX_LEVEL: u8 = 7;

/// The player's sound preferences, as the Preferences dialog shows them:
/// sound effects and music on or off, and each one's volume level, 0
/// through [`MAX_LEVEL`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SoundPrefs {
    /// Whether sound effects play.
    pub sound: bool,
    /// Whether music plays.
    pub music: bool,
    /// The sound effects' volume level.
    pub effects_level: u8,
    /// The music's volume level.
    pub music_level: u8,
}

impl Default for SoundPrefs {
    /// Sound and music on, both at the loudest level.
    fn default() -> Self {
        Self {
            sound: true,
            music: true,
            effects_level: MAX_LEVEL,
            music_level: MAX_LEVEL,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_is_everything_on_at_the_loudest_level() {
        assert_eq!(MAX_LEVEL, 7);
        assert_eq!(
            SoundPrefs::default(),
            SoundPrefs {
                sound: true,
                music: true,
                effects_level: 7,
                music_level: 7,
            }
        );
    }
}
