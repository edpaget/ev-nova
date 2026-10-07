//! The player's preferences as the Preferences dialog shows them, which a
//! screen reports through [`Screen::take_prefs`](crate::Screen::take_prefs).

use crate::sound::SoundPrefs;

/// The player's preferences, as the Preferences dialog shows them: the
/// sound preferences, and whether a hyperspace jump plays its white fades.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Prefs {
    /// Sound effects and music, on or off, and their volumes.
    pub sound: SoundPrefs,
    /// Whether hyperspace jumps fade to white and back, the original's
    /// Hyperspace Effects preference.
    pub hyperspace_effects: bool,
}

impl Default for Prefs {
    /// The default sound preferences, with the hyperspace effects on, as a
    /// fresh install of the original has them.
    fn default() -> Self {
        Self {
            sound: SoundPrefs::default(),
            hyperspace_effects: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_is_the_default_sound_with_the_hyperspace_effects_on() {
        assert_eq!(
            Prefs::default(),
            Prefs {
                sound: SoundPrefs::default(),
                hyperspace_effects: true,
            }
        );
    }
}
