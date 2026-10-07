//! The player's sound settings, and where settings are saved.
//!
//! - [`AudioSettings`]: sound and music on or off, and their volumes, and
//!   how they show in the Preferences dialog.
//! - [`SettingsStore`]: the port the saved settings are read from and
//!   written to, as text. The game's settings keeper, which owns the
//!   format and every setting in it, is `nova`'s.

use std::io;

use nova_view::sound::{MAX_LEVEL, SoundPrefs};

use crate::port::Volume;

/// Whether sound effects and music play, and how loud.
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

/// The volume at the Preferences dialog's `level`, 0 (silent) through
/// [`MAX_LEVEL`] (full), in equal steps; a level above the loudest is
/// full.
#[must_use]
pub fn level_volume(level: u8) -> Volume {
    Volume::new(f32::from(level.min(MAX_LEVEL)) / f32::from(MAX_LEVEL))
}

/// The Preferences dialog's level nearest `volume`.
#[must_use]
pub fn volume_level(volume: Volume) -> u8 {
    // A volume is 0..=1, so the level is 0..=MAX_LEVEL.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let level = (volume.amplitude() * f32::from(MAX_LEVEL)).round() as u8;
    level
}

impl AudioSettings {
    /// The settings as the Preferences dialog shows them.
    #[must_use]
    pub fn prefs(self) -> SoundPrefs {
        SoundPrefs {
            sound: self.sound,
            music: self.music,
            effects_level: volume_level(self.effects_volume),
            music_level: volume_level(self.music_volume),
        }
    }

    /// The settings `prefs` choose, from these: each volume already at its
    /// chosen level is kept as it is, so a volume between two levels (from
    /// a settings file) only moves when its level is changed.
    #[must_use]
    pub fn with_prefs(self, prefs: SoundPrefs) -> Self {
        let at = |volume: Volume, level: u8| {
            if volume_level(volume) == level {
                volume
            } else {
                level_volume(level)
            }
        };
        Self {
            sound: prefs.sound,
            music: prefs.music,
            effects_volume: at(self.effects_volume, prefs.effects_level),
            music_volume: at(self.music_volume, prefs.music_level),
        }
    }
}

/// Where the settings are saved: text in, text out. The format is the
/// keeper's.
pub trait SettingsStore {
    /// The saved text, or `None` when nothing has been saved.
    ///
    /// # Errors
    ///
    /// When the saved text is there but cannot be read.
    fn read(&mut self) -> io::Result<Option<String>>;

    /// Saves `text` in place of whatever was saved.
    ///
    /// # Errors
    ///
    /// When it cannot be saved.
    fn write(&mut self, text: &str) -> io::Result<()>;

    /// Where the settings are saved, for the player to find them: a file's
    /// path, say.
    fn location(&self) -> String;
}

/// A boxed store is a store.
impl SettingsStore for Box<dyn SettingsStore> {
    fn read(&mut self) -> io::Result<Option<String>> {
        (**self).read()
    }

    fn write(&mut self, text: &str) -> io::Result<()> {
        (**self).write(text)
    }

    fn location(&self) -> String {
        (**self).location()
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::recording::MemorySettings;

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

    fn quiet() -> AudioSettings {
        AudioSettings {
            sound: false,
            music: true,
            effects_volume: Volume::new(0.25),
            music_volume: Volume::new(0.5),
        }
    }

    #[test]
    fn levels_are_equal_steps_from_silent_to_full() {
        let volumes: Vec<f32> = (0..=MAX_LEVEL)
            .map(|level| level_volume(level).amplitude())
            .collect();
        let steps: Vec<f32> = (0..=7).map(|n| n as f32 / 7.0).collect();
        assert_eq!(volumes, steps);
        assert_eq!(level_volume(0), Volume::SILENT);
        assert_eq!(level_volume(7), Volume::FULL);
        assert_eq!(level_volume(8), Volume::FULL, "clamped");
        assert_eq!(level_volume(u8::MAX), Volume::FULL, "clamped");
    }

    #[test]
    fn a_volume_is_at_its_nearest_level_and_levels_round_trip() {
        for level in 0..=MAX_LEVEL {
            assert_eq!(volume_level(level_volume(level)), level);
        }
        assert_eq!(volume_level(Volume::FULL), 7);
        assert_eq!(volume_level(Volume::SILENT), 0);
        assert_eq!(volume_level(Volume::new(0.5)), 4, "3.5 rounds up");
        assert_eq!(volume_level(Volume::new(0.49)), 3);
        assert_eq!(volume_level(Volume::new(0.08)), 1);
        assert_eq!(volume_level(Volume::new(0.07)), 0);
    }

    #[test]
    fn the_settings_show_as_prefs() {
        assert_eq!(AudioSettings::default().prefs(), SoundPrefs::default());
        assert_eq!(
            quiet().prefs(),
            SoundPrefs {
                sound: false,
                music: true,
                effects_level: 2,
                music_level: 4,
            }
        );
    }

    #[test]
    fn prefs_change_the_settings_keeping_volumes_already_at_their_level() {
        let prefs = SoundPrefs {
            sound: true,
            music: false,
            effects_level: 2,
            music_level: 4,
        };
        assert_eq!(
            quiet().with_prefs(prefs),
            AudioSettings {
                sound: true,
                music: false,
                ..quiet()
            },
            "0.25 and 0.5 are at levels 2 and 4 already"
        );
        let louder = SoundPrefs {
            effects_level: 3,
            music_level: 0,
            ..prefs
        };
        let changed = quiet().with_prefs(louder);
        assert_eq!(changed.effects_volume, level_volume(3));
        assert_eq!(changed.music_volume, Volume::SILENT);
        assert_eq!(changed.prefs(), louder);
    }

    #[test]
    fn a_boxed_store_is_where_its_store_is() {
        let boxed: Box<dyn SettingsStore> = Box::new(MemorySettings::new());
        assert_eq!(boxed.location(), "memory");
    }

    #[test]
    fn a_boxed_store_forwards_reads_and_writes() {
        let store = MemorySettings::holding("old");
        let mut boxed: Box<dyn SettingsStore> = Box::new(store.clone());
        assert_eq!(boxed.read().expect("reads").as_deref(), Some("old"));
        boxed.write("new").expect("writes");
        assert_eq!(store.text().as_deref(), Some("new"));
    }
}
