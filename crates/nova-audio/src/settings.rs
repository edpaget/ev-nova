//! The player's sound settings, and where settings are saved.
//!
//! - [`AudioSettings`]: sound and music on or off, and their volumes, and
//!   how they show in the Preferences dialog.
//! - [`SettingsStore`]: the port the saved settings are read from and
//!   written to, as text. The game's settings keeper, which owns the
//!   format and every setting in it, is `nova`'s.

use std::io;

use nova_view::sound::{DEFAULT_LEVEL, MAX_LEVEL, SoundPrefs};

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
    /// Sound and music on, both at [`DEFAULT_LEVEL`]: music at half and
    /// effects at a quarter of full, so they agree with
    /// [`SoundPrefs::default`].
    fn default() -> Self {
        Self {
            sound: true,
            music: true,
            effects_volume: effects_level_volume(DEFAULT_LEVEL),
            music_volume: music_level_volume(DEFAULT_LEVEL),
        }
    }
}

/// The level, 0..=[`MAX_LEVEL`], as a fraction of `of`: the eighths of
/// music, the sixteenths of effects.
fn fraction(level: u8, of: f32) -> Volume {
    Volume::new(f32::from(level.min(MAX_LEVEL)) / of)
}

/// The nearest whole number of `of`ths in `volume`, no higher than
/// [`MAX_LEVEL`].
fn nearest(volume: Volume, of: f32) -> u8 {
    // A volume is 0..=1, so this is 0..=of, and `of` is at most 16.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let level = (volume.amplitude() * of).round() as u8;
    level.min(MAX_LEVEL)
}

/// The music's volume at the Preferences dialog's `level`: level/8, so
/// [`MAX_LEVEL`] is full; a level above the loudest is full.
///
/// `_HandleIntroMusic` @0x61d68 sets the music movie's volume to
/// `min(level x 32, 0x100)` (@0x61dcc-0x61de7), and QuickTime's movie
/// volume is linear 8.8 fixed point with 0x100 full.
///
/// `_StartIntroMusic` @0x61c29 starts the movie at `min(level x 48,
/// 0x100)` (@0x61c3d-0x61c57), but `_MusicThread` @0x61e10 calls
/// `_HandleIntroMusic(1)` straight after (@0x61e29), which skips its
/// 40-tick gate and sets level x 32 at once. That louder first pass is
/// over well inside one tick, before any frame is drawn, so it isn't
/// modelled.
#[must_use]
pub fn music_level_volume(level: u8) -> Volume {
    fraction(level, 8.0)
}

/// The effects' volume at the Preferences dialog's `level`: level/16, so
/// [`MAX_LEVEL`] is half of the sample (-6 dB); a level above the loudest
/// is half.
///
/// `_AdjustSoundVolume` @0x5efb sets `gSoundVolume = clamp(level x 8, 0,
/// 0x100)`, which every effect plays at (or less, with distance).
/// The `AmbrosiaTools` framework's `ST_PlaySoundParam` caps a channel's volume at 0x80
/// (@0xe3e12-0xe3e2b), and `ADPCM_Mixer` multiplies each sample by it and
/// shifts right by 7 (@0xe7017-0xe702c), so 0x80 is unity and level x 8
/// is level/16 of the sample.
#[must_use]
pub fn effects_level_volume(level: u8) -> Volume {
    fraction(level, 16.0)
}

/// The music level nearest `volume`, the inverse of
/// [`music_level_volume`].
#[must_use]
pub fn music_volume_level(volume: Volume) -> u8 {
    nearest(volume, 8.0)
}

/// The effects level nearest `volume`, the inverse of
/// [`effects_level_volume`]; a volume above the top level's is at the top
/// level.
#[must_use]
pub fn effects_volume_level(volume: Volume) -> u8 {
    nearest(volume, 16.0)
}

impl AudioSettings {
    /// The settings as the Preferences dialog shows them.
    #[must_use]
    pub fn prefs(self) -> SoundPrefs {
        SoundPrefs {
            sound: self.sound,
            music: self.music,
            effects_level: effects_volume_level(self.effects_volume),
            music_level: music_volume_level(self.music_volume),
        }
    }

    /// The settings `prefs` choose, from these: each volume already at its
    /// chosen level is kept as it is, so a volume between two levels (from
    /// a settings file) only moves when its level is changed.
    #[must_use]
    pub fn with_prefs(self, prefs: SoundPrefs) -> Self {
        let at =
            |volume: Volume, level: u8, to_level: fn(Volume) -> u8, to_volume: fn(u8) -> Volume| {
                if to_level(volume) == level {
                    volume
                } else {
                    to_volume(level)
                }
            };
        Self {
            sound: prefs.sound,
            music: prefs.music,
            effects_volume: at(
                self.effects_volume,
                prefs.effects_level,
                effects_volume_level,
                effects_level_volume,
            ),
            music_volume: at(
                self.music_volume,
                prefs.music_level,
                music_volume_level,
                music_level_volume,
            ),
        }
    }

    /// These settings with the effects' volume no louder than the top
    /// level's, [`effects_level_volume`]`(`[`MAX_LEVEL`]`)`: settings saved
    /// when the levels were sevenths of full may hold more. The music's
    /// top level is full, so it needs no cap.
    #[must_use]
    pub fn capped(self) -> Self {
        let top = effects_level_volume(MAX_LEVEL);
        Self {
            effects_volume: Volume::new(self.effects_volume.amplitude().min(top.amplitude())),
            ..self
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
    fn sound_and_music_start_on_at_the_normal_level() {
        assert_eq!(
            AudioSettings::default(),
            AudioSettings {
                sound: true,
                music: true,
                effects_volume: Volume::new(0.25),
                music_volume: Volume::new(0.5),
            }
        );
        assert_eq!(
            AudioSettings::default().effects_volume,
            effects_level_volume(DEFAULT_LEVEL)
        );
        assert_eq!(
            AudioSettings::default().music_volume,
            music_level_volume(DEFAULT_LEVEL)
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
    fn music_levels_are_eighths() {
        let volumes: Vec<f32> = (0..=MAX_LEVEL)
            .map(|level| music_level_volume(level).amplitude())
            .collect();
        let steps: Vec<f32> = (0..=8).map(|n| n as f32 / 8.0).collect();
        assert_eq!(volumes, steps);
        assert_eq!(music_level_volume(0), Volume::SILENT);
        assert_eq!(music_level_volume(8), Volume::FULL);
        assert_eq!(music_level_volume(9), Volume::FULL, "clamped");
        assert_eq!(music_level_volume(u8::MAX), Volume::FULL, "clamped");
    }

    #[test]
    fn effects_levels_are_sixteenths() {
        let volumes: Vec<f32> = (0..=MAX_LEVEL)
            .map(|level| effects_level_volume(level).amplitude())
            .collect();
        let steps: Vec<f32> = (0..=8).map(|n| n as f32 / 16.0).collect();
        assert_eq!(volumes, steps);
        assert_eq!(effects_level_volume(0), Volume::SILENT);
        assert_eq!(effects_level_volume(8), Volume::new(0.5));
        assert_eq!(effects_level_volume(9), Volume::new(0.5), "clamped");
        assert_eq!(effects_level_volume(u8::MAX), Volume::new(0.5), "clamped");
    }

    #[test]
    fn a_music_volume_is_at_its_nearest_level_and_levels_round_trip() {
        for level in 0..=MAX_LEVEL {
            assert_eq!(music_volume_level(music_level_volume(level)), level);
        }
        assert_eq!(music_volume_level(Volume::FULL), 8);
        assert_eq!(music_volume_level(Volume::SILENT), 0);
        assert_eq!(music_volume_level(Volume::new(0.25)), 2);
        assert_eq!(music_volume_level(Volume::new(0.5625)), 5, "4.5 rounds up");
        assert_eq!(music_volume_level(Volume::new(0.56)), 4);
        assert_eq!(music_volume_level(Volume::new(0.07)), 1);
        assert_eq!(music_volume_level(Volume::new(0.06)), 0);
    }

    #[test]
    fn an_effects_volume_is_at_its_nearest_level_and_levels_round_trip() {
        for level in 0..=MAX_LEVEL {
            assert_eq!(effects_volume_level(effects_level_volume(level)), level);
        }
        assert_eq!(effects_volume_level(Volume::FULL), 8, "clamped");
        assert_eq!(effects_volume_level(Volume::new(0.75)), 8, "clamped");
        assert_eq!(effects_volume_level(Volume::SILENT), 0);
        assert_eq!(effects_volume_level(Volume::new(0.25)), 4);
        assert_eq!(
            effects_volume_level(Volume::new(0.28125)),
            5,
            "4.5 rounds up"
        );
        assert_eq!(effects_volume_level(Volume::new(0.28)), 4);
        assert_eq!(effects_volume_level(Volume::new(0.04)), 1);
        assert_eq!(effects_volume_level(Volume::new(0.03)), 0);
    }

    #[test]
    fn capped_settings_hold_no_effects_volume_above_the_top_level() {
        let loud = AudioSettings {
            effects_volume: Volume::FULL,
            music_volume: Volume::FULL,
            ..quiet()
        };
        assert_eq!(
            loud.capped(),
            AudioSettings {
                effects_volume: Volume::new(0.5),
                ..loud
            },
            "the music's top level is full"
        );
        let edge = AudioSettings {
            effects_volume: Volume::new(0.5),
            ..quiet()
        };
        assert_eq!(edge.capped(), edge);
        assert_eq!(quiet().capped(), quiet());
    }

    #[test]
    fn the_settings_show_as_prefs() {
        assert_eq!(AudioSettings::default().prefs(), SoundPrefs::default());
        assert_eq!(
            quiet().prefs(),
            SoundPrefs {
                sound: false,
                music: true,
                effects_level: 4,
                music_level: 4,
            }
        );
    }

    #[test]
    fn prefs_change_the_settings_keeping_volumes_already_at_their_level() {
        let prefs = SoundPrefs {
            sound: true,
            music: false,
            effects_level: 4,
            music_level: 4,
        };
        assert_eq!(
            quiet().with_prefs(prefs),
            AudioSettings {
                sound: true,
                music: false,
                ..quiet()
            },
            "0.25 and 0.5 are at level 4 already"
        );
        let louder = SoundPrefs {
            effects_level: 3,
            music_level: 0,
            ..prefs
        };
        let changed = quiet().with_prefs(louder);
        assert_eq!(changed.effects_volume, effects_level_volume(3));
        assert_eq!(changed.music_volume, Volume::SILENT);
        assert_eq!(changed.prefs(), louder);
        let near = AudioSettings {
            effects_volume: Volume::new(0.26),
            music_volume: Volume::new(0.51),
            ..quiet()
        };
        assert_eq!(
            near.with_prefs(prefs),
            AudioSettings {
                sound: true,
                music: false,
                ..near
            },
            "each channel's level is read on its own scale"
        );
        let up = near.with_prefs(SoundPrefs {
            effects_level: 5,
            music_level: 5,
            ..prefs
        });
        assert_eq!(up.effects_volume, effects_level_volume(5));
        assert_eq!(up.music_volume, music_level_volume(5));
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
