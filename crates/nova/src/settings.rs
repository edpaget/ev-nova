//! The player's settings, all of them, kept in one settings file.
//!
//! - [`GameSettings`]: the sound settings ([`AudioSettings`]) and whether
//!   hyperspace jumps play their white fades.
//! - [`SettingsKeeper`]: the rules. It reads the settings through a
//!   [`SettingsStore`] (falling back to the defaults when there are none
//!   or they cannot be used), and saves each change. It is the only
//!   writer of the file, so saving one setting never drops another.
//! - [`game_settings`]: the keeper the game starts with, if it has
//!   somewhere to save.
//!
//! The file is JSON, with the keys `sound`, `music`, `effects_volume`,
//! `music_volume` and `hyperspace_effects`; files saved before the last
//! one existed load unchanged, with the hyperspace effects on.

use nova_audio::{AudioSettings, SettingsStore, Volume};
use nova_view::Prefs;
use serde::{Deserialize, Serialize};

/// Every setting the player can change.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GameSettings {
    /// Whether sound effects and music play, and how loud.
    pub audio: AudioSettings,
    /// Whether hyperspace jumps fade to white and back: the original's
    /// Hyperspace Effects preference.
    pub hyperspace_effects: bool,
}

impl Default for GameSettings {
    /// The default sound settings, with the hyperspace effects on.
    fn default() -> Self {
        Self {
            audio: AudioSettings::default(),
            hyperspace_effects: true,
        }
    }
}

impl GameSettings {
    /// The settings as the Preferences dialog shows them.
    #[must_use]
    pub fn prefs(self) -> Prefs {
        Prefs {
            sound: self.audio.prefs(),
            hyperspace_effects: self.hyperspace_effects,
        }
    }

    /// The settings `prefs` choose, from these, keeping a volume already
    /// at its chosen level as [`AudioSettings::with_prefs`] does.
    #[must_use]
    pub fn with_prefs(self, prefs: Prefs) -> Self {
        Self {
            audio: self.audio.with_prefs(prefs.sound),
            hyperspace_effects: prefs.hyperspace_effects,
        }
    }
}

/// The settings as saved: JSON, every field optional (a missing one takes
/// its default). Other fields, such as the rules' `rules` and
/// `rule_overrides`, are kept and saved back as they were.
#[derive(Serialize, Deserialize)]
#[serde(default)]
struct Saved {
    sound: bool,
    music: bool,
    effects_volume: f32,
    music_volume: f32,
    hyperspace_effects: bool,
    #[serde(flatten)]
    others: Others,
}

/// The fields of the settings file that are not the keeper's own.
type Others = serde_json::Map<String, serde_json::Value>;

impl Default for Saved {
    fn default() -> Self {
        Self::from(GameSettings::default())
    }
}

impl From<GameSettings> for Saved {
    fn from(settings: GameSettings) -> Self {
        let audio = settings.audio;
        Self {
            sound: audio.sound,
            music: audio.music,
            effects_volume: audio.effects_volume.amplitude(),
            music_volume: audio.music_volume.amplitude(),
            hyperspace_effects: settings.hyperspace_effects,
            others: Others::new(),
        }
    }
}

impl From<Saved> for GameSettings {
    /// Volumes out of range are clamped.
    fn from(saved: Saved) -> Self {
        Self {
            audio: AudioSettings {
                sound: saved.sound,
                music: saved.music,
                effects_volume: Volume::new(saved.effects_volume),
                music_volume: Volume::new(saved.music_volume),
            },
            hyperspace_effects: saved.hyperspace_effects,
        }
    }
}

/// Keeps the settings: reads them through its store when it opens, and
/// saves each change. It reports problems as messages for its caller to
/// show; it never prints.
#[derive(Debug)]
pub struct SettingsKeeper<S: SettingsStore> {
    store: S,
    settings: GameSettings,
    others: Others,
}

/// The settings keeper the game saves through.
pub type GameKeeper = SettingsKeeper<Box<dyn SettingsStore>>;

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
            Ok(None) => (GameSettings::default(), None),
            Ok(Some(text)) => match serde_json::from_str::<Saved>(&text) {
                Ok(mut saved) => {
                    others = std::mem::take(&mut saved.others);
                    (GameSettings::from(saved), None)
                }
                Err(error) => (
                    GameSettings::default(),
                    Some(format!(
                        "nova: the saved settings in {} are not usable ({error}); \
                         using the defaults, and the next change will replace them",
                        store.location()
                    )),
                ),
            },
            Err(error) => (
                GameSettings::default(),
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
    pub fn settings(&self) -> GameSettings {
        self.settings
    }

    /// The store they are saved in.
    #[must_use]
    pub fn store(&self) -> &S {
        &self.store
    }

    /// Changes the settings to `settings` and saves them, every one, when
    /// they differ from the settings kept.
    ///
    /// # Errors
    ///
    /// A message saying why they could not be saved. The new settings are
    /// kept all the same.
    pub fn change(&mut self, settings: GameSettings) -> Result<(), String> {
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

/// What the settings saved in `store` give: their keeper, the settings
/// the game starts with, and a warning to print, if any. With no store
/// (nowhere to save them), the game starts with the default settings and
/// does not save them.
#[must_use]
pub fn game_settings(
    store: Option<Box<dyn SettingsStore>>,
) -> (Option<GameKeeper>, GameSettings, Option<String>) {
    match store {
        Some(store) => {
            let (keeper, warning) = SettingsKeeper::open(store);
            let settings = keeper.settings();
            (Some(keeper), settings, warning)
        }
        None => (
            None,
            GameSettings::default(),
            Some("nova: settings will not be saved: no home or config directory is set".to_owned()),
        ),
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use nova_audio::recording::MemorySettings;
    use nova_audio::settings::level_volume;
    use nova_audio::{FileSettings, Volume};
    use nova_view::SoundPrefs;

    use super::*;

    fn quiet() -> AudioSettings {
        AudioSettings {
            sound: false,
            music: true,
            effects_volume: Volume::new(0.25),
            music_volume: Volume::new(0.5),
        }
    }

    fn plain(audio: AudioSettings) -> GameSettings {
        GameSettings {
            audio,
            hyperspace_effects: false,
        }
    }

    fn json(store: &MemorySettings) -> serde_json::Value {
        serde_json::from_str(&store.text().expect("saved")).expect("JSON")
    }

    #[test]
    fn hyperspace_effects_start_on() {
        assert_eq!(
            GameSettings::default(),
            GameSettings {
                audio: AudioSettings::default(),
                hyperspace_effects: true,
            }
        );
    }

    #[test]
    fn nothing_saved_is_the_defaults_without_a_warning() {
        let (keeper, warning) = SettingsKeeper::open(MemorySettings::new());
        assert_eq!(keeper.settings(), GameSettings::default());
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
            r#"{"hyperspace_effects": "no"}"#,
            r#"{"sound": false"#,
        ] {
            let (keeper, warning) = SettingsKeeper::open(MemorySettings::holding(text));
            assert_eq!(keeper.settings(), GameSettings::default(), "{text}");
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
        let store = MemorySettings::holding(r#"{"sound": false, "hyperspace_effects": false}"#);
        store.fail_reads(true);
        let (keeper, warning) = SettingsKeeper::open(store);
        assert_eq!(keeper.settings(), GameSettings::default());
        assert_eq!(
            warning.as_deref(),
            Some(
                "nova: cannot read the saved settings in memory (the disk is unreadable); \
                 using the defaults"
            )
        );
    }

    #[test]
    fn missing_fields_take_their_defaults_and_other_ones_leave_these_alone() {
        let text = r#"{"music": false, "effects_volume": 0.5, "theme": "dark"}"#;
        let (keeper, warning) = SettingsKeeper::open(MemorySettings::holding(text));
        assert_eq!(warning, None);
        assert_eq!(
            keeper.settings(),
            GameSettings {
                audio: AudioSettings {
                    music: false,
                    effects_volume: Volume::new(0.5),
                    ..AudioSettings::default()
                },
                hyperspace_effects: true,
            }
        );
        let (keeper, _) = SettingsKeeper::open(MemorySettings::holding("{}"));
        assert_eq!(keeper.settings(), GameSettings::default());
    }

    #[test]
    fn a_file_without_the_flag_has_effects_on() {
        let (keeper, warning) =
            SettingsKeeper::open(MemorySettings::holding(r#"{"sound": false}"#));
        assert_eq!(warning, None, "an old file still loads");
        assert_eq!(
            keeper.settings(),
            GameSettings {
                audio: AudioSettings {
                    sound: false,
                    ..AudioSettings::default()
                },
                hyperspace_effects: true,
            }
        );
    }

    #[test]
    fn volumes_out_of_range_are_clamped() {
        let text = r#"{"effects_volume": 3.5, "music_volume": -1}"#;
        let (keeper, _) = SettingsKeeper::open(MemorySettings::holding(text));
        assert_eq!(keeper.settings().audio.effects_volume, Volume::FULL);
        assert_eq!(keeper.settings().audio.music_volume, Volume::SILENT);
    }

    #[test]
    fn the_flag_is_saved_and_read_back_after_a_restart() {
        let store = MemorySettings::new();
        let (mut keeper, _) = SettingsKeeper::open(store.clone());
        assert_eq!(keeper.change(plain(quiet())), Ok(()));
        assert_eq!(keeper.settings(), plain(quiet()));
        assert_eq!(store.writes(), 1);
        let (restarted, warning) = SettingsKeeper::open(store.clone());
        assert_eq!(warning, None);
        assert_eq!(restarted.settings(), plain(quiet()));
        assert_eq!(
            json(&store),
            serde_json::json!({
                "sound": false,
                "music": true,
                "effects_volume": 0.25,
                "music_volume": 0.5,
                "hyperspace_effects": false,
            })
        );
    }

    #[test]
    fn saving_an_audio_change_keeps_the_flag() {
        let store = MemorySettings::holding(r#"{"hyperspace_effects": false}"#);
        let (mut keeper, _) = SettingsKeeper::open(store.clone());
        let settings = keeper.settings();
        assert!(!settings.hyperspace_effects);
        keeper
            .change(GameSettings {
                audio: quiet(),
                ..settings
            })
            .expect("saves");
        assert_eq!(json(&store)["hyperspace_effects"], false);
        assert_eq!(json(&store)["sound"], false);
    }

    #[test]
    fn a_change_keeps_the_other_settings_saved_beside_these() {
        let text = r#"{"music": false, "rules": "bible",
            "rule_overrides": {"crime_gains": "engine", "nested": {"deeper": [1, {"x": null}]}}}"#;
        let store = MemorySettings::holding(text);
        let (mut keeper, _) = SettingsKeeper::open(store.clone());
        keeper.change(plain(quiet())).expect("saves");
        assert_eq!(json(&store)["rules"], "bible");
        assert_eq!(
            json(&store)["rule_overrides"],
            serde_json::json!({"crime_gains": "engine", "nested": {"deeper": [1, {"x": null}]}})
        );
        assert_eq!(json(&store)["sound"], false);
        let unusable = MemorySettings::holding(r#"{"sound": "yes", "rules": "bible"}"#);
        let (mut keeper, _) = SettingsKeeper::open(unusable.clone());
        keeper.change(plain(quiet())).expect("saves");
        assert_eq!(
            json(&unusable).get("rules"),
            None,
            "unusable text is replaced"
        );
    }

    #[test]
    fn saving_the_flag_keeps_the_audio_settings() {
        let store = MemorySettings::new();
        let (mut keeper, _) = SettingsKeeper::open(store.clone());
        keeper
            .change(GameSettings {
                audio: quiet(),
                hyperspace_effects: true,
            })
            .expect("saves");
        keeper
            .change(GameSettings {
                hyperspace_effects: false,
                ..keeper.settings()
            })
            .expect("saves");
        let (restarted, _) = SettingsKeeper::open(store.clone());
        assert_eq!(restarted.settings(), plain(quiet()));
        assert_eq!(store.writes(), 2);
    }

    #[test]
    fn an_unchanged_value_is_not_saved() {
        let store = MemorySettings::new();
        let (mut keeper, _) = SettingsKeeper::open(store.clone());
        assert_eq!(keeper.change(GameSettings::default()), Ok(()));
        assert_eq!(store.writes(), 0);
        keeper.change(plain(quiet())).expect("saves");
        keeper.change(plain(quiet())).expect("nothing to save");
        assert_eq!(store.writes(), 1);
    }

    #[test]
    fn a_failed_save_says_why_and_keeps_the_new_settings() {
        let store = MemorySettings::new();
        let (mut keeper, _) = SettingsKeeper::open(store.clone());
        store.fail_writes(true);
        assert_eq!(
            keeper.change(plain(quiet())),
            Err("nova: cannot save the settings: the disk is full".to_owned())
        );
        assert_eq!(keeper.settings(), plain(quiet()));
        assert_eq!(store.text(), None);
        assert_eq!(keeper.store().writes(), 0);
    }

    #[test]
    fn prefs_round_trip() {
        let settings = plain(quiet());
        let prefs = settings.prefs();
        assert_eq!(
            prefs,
            Prefs {
                sound: quiet().prefs(),
                hyperspace_effects: false,
            }
        );
        assert_eq!(settings.with_prefs(prefs), settings, "between levels kept");
        let changed = settings.with_prefs(Prefs {
            sound: SoundPrefs {
                effects_level: 3,
                ..prefs.sound
            },
            hyperspace_effects: true,
        });
        assert_eq!(changed.audio.effects_volume, level_volume(3));
        assert_eq!(changed.audio.music_volume, Volume::new(0.5));
        assert!(changed.hyperspace_effects);
        assert_eq!(GameSettings::default().prefs(), Prefs::default());
    }

    #[test]
    fn the_keeper_over_the_real_file_keeps_both_parts_across_a_restart() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let path = dir.path().join("nova").join("settings.json");
        let (mut keeper, warning) = SettingsKeeper::open(FileSettings::new(&path));
        assert_eq!(
            (keeper.settings(), warning),
            (GameSettings::default(), None)
        );
        keeper.change(plain(quiet())).expect("saves");
        drop(keeper);
        let (restarted, warning) = SettingsKeeper::open(FileSettings::new(&path));
        assert_eq!((restarted.settings(), warning), (plain(quiet()), None));
    }

    #[test]
    fn an_unusable_file_is_named_in_the_warning() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let path = dir.path().join("settings.json");
        std::fs::write(&path, "not json").expect("writes");
        let (_, warning) = SettingsKeeper::open(FileSettings::new(&path));
        let warning = warning.expect("a warning");
        assert!(
            warning.starts_with(&format!(
                "nova: the saved settings in {} are not usable (",
                path.display()
            )),
            "{warning}"
        );
    }

    #[test]
    fn saved_settings_are_kept_and_started_with() {
        let store = MemorySettings::holding(
            r#"{"music": false, "effects_volume": 0.5, "hyperspace_effects": false}"#,
        );
        let (keeper, settings, warning) = game_settings(Some(Box::new(store.clone())));
        let expected = GameSettings {
            audio: AudioSettings {
                music: false,
                effects_volume: Volume::new(0.5),
                ..AudioSettings::default()
            },
            hyperspace_effects: false,
        };
        assert_eq!((settings, warning), (expected, None));
        let mut keeper: GameKeeper = keeper.expect("a keeper");
        assert_eq!(keeper.settings(), expected);
        keeper.change(GameSettings::default()).expect("saves");
        assert_eq!(store.writes(), 1, "through the store");
    }

    #[test]
    fn unusable_settings_start_with_the_defaults_and_a_warning() {
        let store = MemorySettings::holding("not json");
        let (keeper, settings, warning) = game_settings(Some(Box::new(store)));
        assert!(keeper.is_some(), "a change can still be saved");
        assert_eq!(settings, GameSettings::default());
        assert!(warning.is_some_and(|warning| warning.starts_with("nova: the saved settings")));
    }

    #[test]
    fn with_nowhere_to_save_them_the_defaults_are_not_saved() {
        let (keeper, settings, warning) = game_settings(None);
        assert!(keeper.is_none());
        assert_eq!(settings, GameSettings::default());
        assert_eq!(
            warning.as_deref(),
            Some("nova: settings will not be saved: no home or config directory is set")
        );
    }
}
