//! The player's preferences as the Preferences dialog shows them, which a
//! screen reports through [`Screen::take_prefs`](crate::Screen::take_prefs).

use crate::sound::SoundPrefs;
use crate::video_mode::FullscreenKind;

/// The player's preferences, as the Preferences dialog shows them: the
/// sound preferences, whether a hyperspace jump plays its white fades, and
/// whether a `dësc`'s movie plays, and how the game runs fullscreen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Prefs {
    /// Sound effects and music, on or off, and their volumes.
    pub sound: SoundPrefs,
    /// Whether hyperspace jumps fade to white and back, the original's
    /// Hyperspace Effects preference.
    pub hyperspace_effects: bool,
    /// Whether a `dësc`'s QuickTime movie plays, the original's QuickTime
    /// Movies preference (`*0x3b501c + 7`, set meaning off: `_PlayQTMovie`
    /// @0x5597c).
    pub quicktime_movies: bool,
    /// How the game runs fullscreen: borderless, or in an exclusive
    /// display mode the dialog's picker chose. Not the original's: its
    /// dialog has no such control.
    pub fullscreen: FullscreenKind,
}

impl Default for Prefs {
    /// The default sound preferences, with the hyperspace effects and the
    /// movies on, as a fresh install of the original has them: it
    /// registers `HyperspaceEffects` and `QuickTimeMovies` as NO
    /// (`_LoadPrefs` @0xce8ac, @0xce8ee), and a clear byte means on; and
    /// fullscreen borderless.
    fn default() -> Self {
        Self {
            sound: SoundPrefs::default(),
            hyperspace_effects: true,
            quicktime_movies: true,
            fullscreen: FullscreenKind::Borderless,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_is_the_default_sound_with_the_hyperspace_effects_and_movies_on() {
        assert_eq!(
            Prefs::default(),
            Prefs {
                sound: SoundPrefs::default(),
                hyperspace_effects: true,
                quicktime_movies: true,
                fullscreen: FullscreenKind::Borderless,
            }
        );
    }
}
