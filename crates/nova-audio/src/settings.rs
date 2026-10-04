//! The player's sound settings, as the core holds them.

use crate::port::Volume;

/// Whether sound effects and music play, and how loud. Plain state: the
/// Preferences dialog that changes it, and saving it, come later.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AudioSettings {
    /// Whether sound effects play.
    pub sound: bool,
    /// Whether music plays.
    pub music: bool,
    /// How loud sound effects are.
    pub effects_volume: Volume,
    /// How loud the music is.
    pub music_volume: Volume,
}

impl Default for AudioSettings {
    /// Sound and music on, both at full volume.
    fn default() -> Self {
        Self {
            sound: true,
            music: true,
            effects_volume: Volume::FULL,
            music_volume: Volume::FULL,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sound_and_music_start_on_at_full_volume() {
        assert_eq!(
            AudioSettings::default(),
            AudioSettings {
                sound: true,
                music: true,
                effects_volume: Volume::FULL,
                music_volume: Volume::FULL,
            }
        );
    }
}
