//! The player's sound settings: what they are, and keeping them across
//! runs.
//!
//! - [`AudioSettings`]: sound and music on or off, and their volumes.
//! - [`SettingsStore`]: the port the saved settings are read from and
//!   written to, as text.
//! - [`SettingsKeeper`]: the rules. It reads the settings through a store
//!   (falling back to the defaults when there are none or they cannot be
//!   used), and saves each change.

use std::io;

use nova_view::sound::{MAX_LEVEL, SoundPrefs};
use serde::{Deserialize, Serialize};

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

/// The settings as saved: JSON, every field optional (a missing one takes
/// its default). Other fields are the game's other settings: kept, and
/// saved back as they were.
#[derive(Serialize, Deserialize)]
#[serde(default)]
struct Saved {
    sound: bool,
    music: bool,
    effects_volume: f32,
    music_volume: f32,
    #[serde(flatten)]
    others: Others,
}

/// The fields of the settings file that are not the sound's.
type Others = serde_json::Map<String, serde_json::Value>;

impl Default for Saved {
    fn default() -> Self {
        Self::from(AudioSettings::default())
    }
}

impl From<AudioSettings> for Saved {
    fn from(settings: AudioSettings) -> Self {
        Self {
            sound: settings.sound,
            music: settings.music,
            effects_volume: settings.effects_volume.amplitude(),
            music_volume: settings.music_volume.amplitude(),
            others: Others::new(),
        }
    }
}

impl From<Saved> for AudioSettings {
    /// Volumes out of range are clamped.
    fn from(saved: Saved) -> Self {
        Self {
            sound: saved.sound,
            music: saved.music,
            effects_volume: Volume::new(saved.effects_volume),
            music_volume: Volume::new(saved.music_volume),
        }
    }
}

/// Keeps the settings: reads them through its store when it opens, and
/// saves each change. It reports problems as messages for its caller to
/// show; it never prints.
#[derive(Debug)]
pub struct SettingsKeeper<S: SettingsStore> {
    store: S,
    settings: AudioSettings,
    others: Others,
}

impl<S: SettingsStore> SettingsKeeper<S> {
    /// Opens the settings saved in `store`, with a warning when they could
    /// not be used.
    ///
    /// Nothing saved gives the defaults, silently. Text that cannot be
    /// read, or is not settings (not JSON, or a field of the wrong type),
    /// gives the defaults and a warning naming the store's location. A
    /// missing field takes its default, and volumes are clamped to silent
    /// through full. Other fields, the game's other settings, are saved
    /// back with each change.
    pub fn open(mut store: S) -> (Self, Option<String>) {
        let mut others = Others::new();
        let (settings, warning) = match store.read() {
            Ok(None) => (AudioSettings::default(), None),
            Ok(Some(text)) => match serde_json::from_str::<Saved>(&text) {
                Ok(mut saved) => {
                    others = std::mem::take(&mut saved.others);
                    (AudioSettings::from(saved), None)
                }
                Err(error) => (
                    AudioSettings::default(),
                    Some(format!(
                        "nova: the saved settings in {} are not usable ({error}); \
                         using the defaults, and the next change will replace them",
                        store.location()
                    )),
                ),
            },
            Err(error) => (
                AudioSettings::default(),
                Some(format!(
                    "nova: cannot read the saved settings in {} ({error}); using the defaults",
                    store.location()
                )),
            ),
        };
        (
            Self {
                store,
                settings,
                others,
            },
            warning,
        )
    }

    /// The settings.
    #[must_use]
    pub fn settings(&self) -> AudioSettings {
        self.settings
    }

    /// The store they are saved in.
    #[must_use]
    pub fn store(&self) -> &S {
        &self.store
    }

    /// Changes the settings to `settings` and saves them, when they differ
    /// from the settings kept.
    ///
    /// # Errors
    ///
    /// A message saying why they could not be saved. The new settings are
    /// kept all the same.
    pub fn change(&mut self, settings: AudioSettings) -> Result<(), String> {
        if settings == self.settings {
            return Ok(());
        }
        self.settings = settings;
        let saved = Saved {
            others: self.others.clone(),
            ..Saved::from(settings)
        };
        let text = serde_json::to_string_pretty(&saved).expect("plain values always serialise");
        self.store
            .write(&text)
            .map_err(|error| format!("nova: cannot save the settings: {error}"))
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
    fn nothing_saved_is_the_defaults_without_a_warning() {
        let (keeper, warning) = SettingsKeeper::open(MemorySettings::new());
        assert_eq!(keeper.settings(), AudioSettings::default());
        assert_eq!(warning, None);
    }

    #[test]
    fn unusable_text_is_the_defaults_with_a_warning() {
        for text in [
            "not json",
            "",
            "[1, 2]",
            r#"{"sound": "yes"}"#,
            r#"{"music_volume": "loud"}"#,
            r#"{"sound": false"#,
        ] {
            let (keeper, warning) = SettingsKeeper::open(MemorySettings::holding(text));
            assert_eq!(keeper.settings(), AudioSettings::default(), "{text}");
            let warning = warning.expect("a warning");
            assert!(
                warning.starts_with("nova: the saved settings in memory are not usable (")
                    && warning
                        .ends_with("); using the defaults, and the next change will replace them"),
                "{text}: {warning}"
            );
        }
    }

    #[test]
    fn a_read_error_is_the_defaults_with_a_warning() {
        let store = MemorySettings::holding(r#"{"sound": false}"#);
        store.fail_reads(true);
        let (keeper, warning) = SettingsKeeper::open(store);
        assert_eq!(keeper.settings(), AudioSettings::default());
        assert_eq!(
            warning.as_deref(),
            Some(
                "nova: cannot read the saved settings in memory (the disk is unreadable); \
                 using the defaults"
            )
        );
    }

    #[test]
    fn a_boxed_store_is_where_its_store_is() {
        let boxed: Box<dyn SettingsStore> = Box::new(MemorySettings::new());
        assert_eq!(boxed.location(), "memory");
    }

    #[test]
    fn missing_fields_take_their_defaults_and_other_fields_leave_the_sound_alone() {
        let text = r#"{"music": false, "effects_volume": 0.5, "theme": "dark"}"#;
        let (keeper, warning) = SettingsKeeper::open(MemorySettings::holding(text));
        assert_eq!(warning, None);
        assert_eq!(
            keeper.settings(),
            AudioSettings {
                music: false,
                effects_volume: Volume::new(0.5),
                ..AudioSettings::default()
            }
        );
        let (keeper, _) = SettingsKeeper::open(MemorySettings::holding("{}"));
        assert_eq!(keeper.settings(), AudioSettings::default());
    }

    #[test]
    fn volumes_out_of_range_are_clamped() {
        let text = r#"{"effects_volume": 3.5, "music_volume": -1}"#;
        let (keeper, _) = SettingsKeeper::open(MemorySettings::holding(text));
        assert_eq!(keeper.settings().effects_volume, Volume::FULL);
        assert_eq!(keeper.settings().music_volume, Volume::SILENT);
    }

    #[test]
    fn a_change_is_saved_and_read_back_after_a_restart() {
        let store = MemorySettings::new();
        let (mut keeper, _) = SettingsKeeper::open(store.clone());
        assert_eq!(keeper.change(quiet()), Ok(()));
        assert_eq!(keeper.settings(), quiet());
        assert_eq!(store.writes(), 1);
        let (restarted, warning) = SettingsKeeper::open(store.clone());
        assert_eq!(warning, None);
        assert_eq!(restarted.settings(), quiet());
        let saved: serde_json::Value =
            serde_json::from_str(&store.text().expect("saved")).expect("JSON");
        assert_eq!(
            saved,
            serde_json::json!({
                "sound": false,
                "music": true,
                "effects_volume": 0.25,
                "music_volume": 0.5,
            })
        );
    }

    #[test]
    fn a_change_keeps_the_other_settings_saved_beside_the_sounds() {
        let text = r#"{"music": false, "crime_gains": "bible", "theme": {"dark": true}}"#;
        let store = MemorySettings::holding(text);
        let (mut keeper, _) = SettingsKeeper::open(store.clone());
        keeper.change(quiet()).expect("saves");
        let saved: serde_json::Value =
            serde_json::from_str(&store.text().expect("saved")).expect("JSON");
        assert_eq!(
            saved,
            serde_json::json!({
                "sound": false,
                "music": true,
                "effects_volume": 0.25,
                "music_volume": 0.5,
                "crime_gains": "bible",
                "theme": {"dark": true},
            })
        );
        let unusable = MemorySettings::holding(r#"{"sound": "yes", "crime_gains": "bible"}"#);
        let (mut keeper, _) = SettingsKeeper::open(unusable.clone());
        keeper.change(quiet()).expect("saves");
        let saved: serde_json::Value =
            serde_json::from_str(&unusable.text().expect("saved")).expect("JSON");
        assert_eq!(saved.get("crime_gains"), None, "unusable text is replaced");
    }

    #[test]
    fn an_unchanged_value_is_not_saved() {
        let store = MemorySettings::new();
        let (mut keeper, _) = SettingsKeeper::open(store.clone());
        assert_eq!(keeper.change(AudioSettings::default()), Ok(()));
        assert_eq!(store.writes(), 0);
        keeper.change(quiet()).expect("saves");
        keeper.change(quiet()).expect("nothing to save");
        assert_eq!(store.writes(), 1);
    }

    #[test]
    fn a_failed_save_says_why_and_keeps_the_new_settings() {
        let store = MemorySettings::new();
        let (mut keeper, _) = SettingsKeeper::open(store.clone());
        store.fail_writes(true);
        assert_eq!(
            keeper.change(quiet()),
            Err("nova: cannot save the settings: the disk is full".to_owned())
        );
        assert_eq!(keeper.settings(), quiet());
        assert_eq!(store.text(), None);
        assert_eq!(keeper.store().writes(), 0);
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
