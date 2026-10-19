//! The player's settings, all of them, kept in one settings file.
//!
//! - [`GameSettings`]: the sound settings ([`AudioSettings`]), whether
//!   hyperspace jumps play their white fades, whether `dësc` movies play,
//!   whether the game runs fullscreen, and how ([`FullscreenSetting`]),
//!   and the Scaling chosen ([`ScalingSetting`]).
//! - [`SettingsKeeper`]: the rules. It reads the settings through a
//!   [`SettingsStore`] (falling back to the defaults when there are none
//!   or they cannot be used), and saves each change. It is the only
//!   writer of the file, so saving one setting never drops another.
//! - [`game_settings`]: the keeper the game starts with, if it has
//!   somewhere to save.
//!
//! The file is JSON, with the keys `sound`, `music`, `effects_volume`,
//! `music_volume`, `hyperspace_effects`, `quicktime_movies`,
//! `show_map_borders`, `fullscreen`, `fullscreen_kind`, `frame_upscaler`
//! and `nova_files`; files saved before the last seven existed load
//! unchanged, with the hyperspace effects, the movies and the map's
//! borders on, the game windowed and, when fullscreen, borderless, the
//! world drawn natively, and no `Nova Files` remembered.
//!
//! `nova_files` is the `Nova Files` directory the player picked when the
//! game found none (`crate::startup`), a string, absent (or `null`) when
//! none is remembered. A path whose name is not UTF-8 is never saved.
//!
//! `frame_upscaler` holds the player's Scaling, under the key phase 6 named
//! when it held only the per-frame upscaler: `"native"`,
//! `"sharp_bilinear"`, `"metalfx_spatial"` (macOS's MetalFX, which a
//! device without it doesn't offer: the app then draws natively and says
//! so once at startup) or `"super_xbr"` (sprites enlarged as they load).
//! Files phase 6 saved load unchanged, their per-frame choice kept. A name
//! this game does not have is kept as it is and saved back unchanged; the
//! app draws natively and says so once at startup.
//!
//! `fullscreen_kind` is `"borderless"`, or an exclusive mode with the
//! monitor it was chosen on: `{"exclusive": {"width": 1920, "height":
//! 1080, "refresh_millihertz": 60000, "monitor": "DELL U2720Q"}}`, the
//! monitor `null` when the platform names none. It is kept while the game
//! is windowed, so the fullscreen shortcut goes back to it.
//!
//! `show_map_borders` is the original's `ShowMapBorders`: whether the
//! galaxy map shades government borders, on when absent (`_LoadPrefs`
//! @0xced57-0xced93). The Preferences dialog never carries it, as
//! `_SavePrefs` never writes it: the map's close saves it
//! (`nova_view::flight::FlightView::take_map_borders_save`).

use std::path::PathBuf;

use nova_audio::{AudioSettings, SettingsStore, Volume};
use nova_view::{FullscreenKind, Prefs, Scaling, VideoMode};
use serde::{Deserialize, Serialize};

/// The Scaling chosen, as saved: its name, kept as it was read even when
/// this game has no scaling of that name, so a choice made by another
/// build, or on a device that has it, is never overwritten by loading it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScalingSetting(String);

impl ScalingSetting {
    /// The name saved.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.0
    }

    /// The scaling the name names, if this game has it.
    #[must_use]
    pub fn known(&self) -> Option<Scaling> {
        match self.0.as_str() {
            "native" => Some(Scaling::Native),
            "sharp_bilinear" => Some(Scaling::SharpBilinear),
            "metalfx_spatial" => Some(Scaling::MetalFxSpatial),
            "super_xbr" => Some(Scaling::SuperXbr),
            _ => None,
        }
    }
}

impl Default for ScalingSetting {
    /// Native.
    fn default() -> Self {
        Self::from(Scaling::Native)
    }
}

impl From<Scaling> for ScalingSetting {
    /// The scaling's name: a stable key of the settings file. The
    /// per-frame ones keep the names phase 6 gave them.
    fn from(scaling: Scaling) -> Self {
        let name = match scaling {
            Scaling::Native => "native",
            Scaling::SharpBilinear => "sharp_bilinear",
            Scaling::MetalFxSpatial => "metalfx_spatial",
            Scaling::SuperXbr => "super_xbr",
        };
        Self(name.to_owned())
    }
}

impl From<String> for ScalingSetting {
    /// The setting `name` is saved as, known or not.
    fn from(name: String) -> Self {
        Self(name)
    }
}

/// How the game runs fullscreen, as saved.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum FullscreenSetting {
    /// Borderless, at the desktop's own mode.
    #[default]
    Borderless,
    /// In `mode`, exclusively, on the monitor named `monitor` (`None` when
    /// the platform named none).
    Exclusive {
        /// The display mode.
        mode: VideoMode,
        /// The monitor's name, as the platform gave it.
        monitor: Option<String>,
    },
}

impl FullscreenSetting {
    /// The kind, without the monitor.
    #[must_use]
    pub fn kind(&self) -> FullscreenKind {
        match self {
            Self::Borderless => FullscreenKind::Borderless,
            Self::Exclusive { mode, .. } => FullscreenKind::Exclusive(*mode),
        }
    }
}

/// Every setting the player can change.
#[derive(Clone, Debug, PartialEq)]
#[allow(clippy::struct_excessive_bools)] // One field per on-off setting.
pub struct GameSettings {
    /// Whether sound effects and music play, and how loud.
    pub audio: AudioSettings,
    /// Whether hyperspace jumps fade to white and back: the original's
    /// Hyperspace Effects preference.
    pub hyperspace_effects: bool,
    /// Whether the game runs fullscreen rather than in a window.
    pub fullscreen: bool,
    /// How the game runs fullscreen when it does, kept while it is
    /// windowed.
    pub fullscreen_kind: FullscreenSetting,
    /// Whether a `dësc`'s movie plays: the original's QuickTime Movies
    /// preference.
    pub quicktime_movies: bool,
    /// Whether the galaxy map shades government borders: the original's
    /// `ShowMapBorders`.
    pub map_borders: bool,
    /// The Scaling the player chose, as saved, under the key phase 6
    /// named for the frame upscaler it then held.
    pub frame_upscaler: ScalingSetting,
    /// The `Nova Files` directory the player picked when the game found
    /// none, tried before the usual places on the next launch.
    pub nova_files: Option<PathBuf>,
}

impl Default for GameSettings {
    /// The default sound settings, with the hyperspace effects, the
    /// movies and the map's borders on, in a window, the world drawn
    /// natively.
    fn default() -> Self {
        Self {
            audio: AudioSettings::default(),
            hyperspace_effects: true,
            fullscreen: false,
            fullscreen_kind: FullscreenSetting::Borderless,
            quicktime_movies: true,
            map_borders: true,
            frame_upscaler: ScalingSetting::default(),
            nova_files: None,
        }
    }
}

impl GameSettings {
    /// The settings as the Preferences dialog shows them, the fullscreen
    /// kind without its monitor, and the Scaling stored, or native when
    /// this game has none of its name.
    #[must_use]
    pub fn prefs(&self) -> Prefs {
        Prefs {
            sound: self.audio.prefs(),
            hyperspace_effects: self.hyperspace_effects,
            quicktime_movies: self.quicktime_movies,
            fullscreen: self.fullscreen_kind.kind(),
            scaling: self.frame_upscaler.known().unwrap_or_default(),
        }
    }

    /// The settings `prefs` choose, from these, keeping a volume already
    /// at its chosen level as [`AudioSettings::with_prefs`] does, and the
    /// settings the prefs don't set: the map's borders, fullscreen, both
    /// whether and how, the Scaling and the remembered `Nova Files`. A fullscreen kind or a scaling
    /// the dialog chooses is the app's to apply and save, as it applies
    /// it, so a change of another preference never overwrites a stored
    /// scaling this device could not use.
    #[must_use]
    pub fn with_prefs(self, prefs: Prefs) -> Self {
        Self {
            audio: self.audio.with_prefs(prefs.sound),
            hyperspace_effects: prefs.hyperspace_effects,
            quicktime_movies: prefs.quicktime_movies,
            ..self
        }
    }
}

/// The settings as saved: JSON, every field optional (a missing one takes
/// its default). Other fields, such as the rules' `rules`,
/// `rule_overrides` and `rule_values`, are kept and saved back as they
/// were.
#[derive(Serialize, Deserialize)]
#[serde(default)]
#[allow(clippy::struct_excessive_bools)] // One field per key of the file.
struct Saved {
    sound: bool,
    music: bool,
    effects_volume: f32,
    music_volume: f32,
    hyperspace_effects: bool,
    fullscreen: bool,
    fullscreen_kind: SavedKind,
    quicktime_movies: bool,
    show_map_borders: bool,
    frame_upscaler: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    nova_files: Option<String>,
    #[serde(flatten)]
    others: Others,
}

/// A [`FullscreenSetting`] as saved.
#[derive(Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum SavedKind {
    #[default]
    Borderless,
    Exclusive {
        width: u32,
        height: u32,
        refresh_millihertz: u32,
        monitor: Option<String>,
    },
}

impl From<FullscreenSetting> for SavedKind {
    fn from(setting: FullscreenSetting) -> Self {
        match setting {
            FullscreenSetting::Borderless => Self::Borderless,
            FullscreenSetting::Exclusive { mode, monitor } => Self::Exclusive {
                width: mode.width,
                height: mode.height,
                refresh_millihertz: mode.refresh_millihertz,
                monitor,
            },
        }
    }
}

impl From<SavedKind> for FullscreenSetting {
    fn from(saved: SavedKind) -> Self {
        match saved {
            SavedKind::Borderless => Self::Borderless,
            SavedKind::Exclusive {
                width,
                height,
                refresh_millihertz,
                monitor,
            } => Self::Exclusive {
                mode: VideoMode {
                    width,
                    height,
                    refresh_millihertz,
                },
                monitor,
            },
        }
    }
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
            fullscreen: settings.fullscreen,
            fullscreen_kind: settings.fullscreen_kind.into(),
            quicktime_movies: settings.quicktime_movies,
            show_map_borders: settings.map_borders,
            frame_upscaler: settings.frame_upscaler.0,
            // A name that isn't UTF-8 can't be a JSON string: it is left
            // out, and the next launch looks again.
            nova_files: settings
                .nova_files
                .and_then(|dir| dir.into_os_string().into_string().ok()),
            others: Others::new(),
        }
    }
}

impl From<Saved> for GameSettings {
    /// Volumes are clamped to what the levels reach: silent through full,
    /// and the effects no louder than their top level's
    /// ([`AudioSettings::capped`]).
    fn from(saved: Saved) -> Self {
        Self {
            audio: AudioSettings {
                sound: saved.sound,
                music: saved.music,
                effects_volume: Volume::new(saved.effects_volume),
                music_volume: Volume::new(saved.music_volume),
            }
            .capped(),
            hyperspace_effects: saved.hyperspace_effects,
            fullscreen: saved.fullscreen,
            fullscreen_kind: saved.fullscreen_kind.into(),
            quicktime_movies: saved.quicktime_movies,
            map_borders: saved.show_map_borders,
            frame_upscaler: ScalingSetting(saved.frame_upscaler),
            nova_files: saved.nova_files.map(PathBuf::from),
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
    /// missing field takes its default, and volumes are clamped to what
    /// the levels reach. Other fields, the game's other settings, are saved
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
        self.settings.clone()
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
        self.settings = settings.clone();
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
    use nova_audio::settings::effects_level_volume;
    use nova_audio::{FileSettings, Volume};
    use nova_view::{FullscreenKind, Scaling, SoundPrefs, VideoMode};

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
            fullscreen: false,
            fullscreen_kind: FullscreenSetting::Borderless,
            quicktime_movies: true,
            map_borders: true,
            frame_upscaler: ScalingSetting::default(),
            nova_files: None,
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
                fullscreen: false,
                fullscreen_kind: FullscreenSetting::Borderless,
                quicktime_movies: true,
                map_borders: true,
                frame_upscaler: ScalingSetting::default(),
                nova_files: None,
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
            r#"{"fullscreen": "yes"}"#,
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
                fullscreen: false,
                fullscreen_kind: FullscreenSetting::Borderless,
                quicktime_movies: true,
                map_borders: true,
                frame_upscaler: ScalingSetting::default(),
                nova_files: None,
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
                fullscreen: false,
                fullscreen_kind: FullscreenSetting::Borderless,
                quicktime_movies: true,
                map_borders: true,
                frame_upscaler: ScalingSetting::default(),
                nova_files: None,
            }
        );
    }

    #[test]
    fn the_game_starts_windowed() {
        assert!(!GameSettings::default().fullscreen);
    }

    #[test]
    fn a_file_without_the_fullscreen_field_loads_windowed() {
        let (keeper, warning) =
            SettingsKeeper::open(MemorySettings::holding(r#"{"sound": false}"#));
        assert_eq!(warning, None);
        assert!(!keeper.settings().fullscreen);
    }

    #[test]
    fn fullscreen_is_saved_and_read_back_after_a_restart_both_ways() {
        let store = MemorySettings::new();
        let (mut keeper, _) = SettingsKeeper::open(store.clone());
        let fullscreen = GameSettings {
            fullscreen: true,
            ..GameSettings::default()
        };
        keeper.change(fullscreen.clone()).expect("saves");
        assert_eq!(json(&store)["fullscreen"], true);
        let (mut restarted, warning) = SettingsKeeper::open(store.clone());
        assert_eq!((restarted.settings(), warning), (fullscreen, None));
        restarted.change(GameSettings::default()).expect("saves");
        let (restarted, _) = SettingsKeeper::open(store.clone());
        assert!(!restarted.settings().fullscreen);
        assert_eq!(store.writes(), 2);
    }

    #[test]
    fn choosing_preferences_keeps_fullscreen() {
        let settings = GameSettings {
            fullscreen: true,
            ..plain(quiet())
        };
        assert!(settings.with_prefs(Prefs::default()).fullscreen);
        let exclusive = GameSettings {
            fullscreen: true,
            fullscreen_kind: exclusive_on(Some("DELL U2720Q")),
            ..plain(quiet())
        };
        for kind in [FullscreenKind::Borderless, FullscreenKind::Exclusive(qhd())] {
            let chosen = exclusive.clone().with_prefs(Prefs {
                fullscreen: kind,
                ..Prefs::default()
            });
            assert!(chosen.fullscreen, "{kind:?}");
            assert_eq!(
                chosen.fullscreen_kind, exclusive.fullscreen_kind,
                "{kind:?}"
            );
        }
    }

    fn fhd() -> VideoMode {
        VideoMode {
            width: 1920,
            height: 1080,
            refresh_millihertz: 59_940,
        }
    }

    fn qhd() -> VideoMode {
        VideoMode {
            width: 2560,
            height: 1440,
            refresh_millihertz: 144_000,
        }
    }

    fn exclusive_on(monitor: Option<&str>) -> FullscreenSetting {
        FullscreenSetting::Exclusive {
            mode: fhd(),
            monitor: monitor.map(str::to_owned),
        }
    }

    #[test]
    fn a_phase_one_file_loads_fullscreen_and_borderless() {
        let (keeper, warning) =
            SettingsKeeper::open(MemorySettings::holding(r#"{"fullscreen": true}"#));
        assert_eq!(warning, None);
        assert!(keeper.settings().fullscreen);
        assert_eq!(
            keeper.settings().fullscreen_kind,
            FullscreenSetting::Borderless
        );
        let (keeper, warning) = SettingsKeeper::open(MemorySettings::holding("{}"));
        assert_eq!(warning, None);
        assert!(!keeper.settings().fullscreen);
        assert_eq!(
            keeper.settings().fullscreen_kind,
            FullscreenSetting::Borderless
        );
        assert_eq!(
            GameSettings::default().fullscreen_kind,
            FullscreenSetting::Borderless
        );
    }

    #[test]
    fn an_exclusive_kind_is_saved_and_read_back_after_a_restart() {
        for monitor in [Some("DELL U2720Q"), None] {
            let store = MemorySettings::new();
            let (mut keeper, _) = SettingsKeeper::open(store.clone());
            let settings = GameSettings {
                fullscreen: true,
                fullscreen_kind: exclusive_on(monitor),
                ..GameSettings::default()
            };
            keeper.change(settings.clone()).expect("saves");
            assert_eq!(
                json(&store)["fullscreen_kind"],
                serde_json::json!({
                    "exclusive": {
                        "width": 1920,
                        "height": 1080,
                        "refresh_millihertz": 59_940,
                        "monitor": monitor,
                    }
                })
            );
            let (restarted, warning) = SettingsKeeper::open(store.clone());
            assert_eq!((restarted.settings(), warning), (settings, None));
        }
        let store = MemorySettings::new();
        let (mut keeper, _) = SettingsKeeper::open(store.clone());
        keeper
            .change(GameSettings {
                fullscreen: true,
                ..GameSettings::default()
            })
            .expect("saves");
        assert_eq!(json(&store)["fullscreen_kind"], "borderless");
    }

    #[test]
    fn a_fullscreen_kind_of_the_wrong_shape_is_not_usable() {
        for text in [
            r#"{"fullscreen_kind": 7}"#,
            r#"{"fullscreen_kind": "sideways"}"#,
            r#"{"fullscreen_kind": {"exclusive": {"width": 1920}}}"#,
        ] {
            let (keeper, warning) = SettingsKeeper::open(MemorySettings::holding(text));
            assert_eq!(keeper.settings(), GameSettings::default(), "{text}");
            assert!(
                warning.is_some_and(|warning| warning.contains("are not usable")),
                "{text}"
            );
        }
    }

    #[test]
    fn the_prefs_show_the_stored_kind_without_its_monitor() {
        let settings = GameSettings {
            fullscreen_kind: exclusive_on(Some("DELL U2720Q")),
            ..GameSettings::default()
        };
        assert_eq!(
            settings.prefs().fullscreen,
            FullscreenKind::Exclusive(fhd())
        );
        assert_eq!(
            GameSettings::default().prefs().fullscreen,
            FullscreenKind::Borderless
        );
        assert_eq!(exclusive_on(None).kind(), FullscreenKind::Exclusive(fhd()));
        assert_eq!(
            FullscreenSetting::Borderless.kind(),
            FullscreenKind::Borderless
        );
    }

    #[test]
    fn the_movies_flag_is_saved_and_a_file_without_it_has_them_on() {
        let (keeper, warning) =
            SettingsKeeper::open(MemorySettings::holding(r#"{"hyperspace_effects": false}"#));
        assert_eq!(warning, None);
        assert!(keeper.settings().quicktime_movies, "an old file");
        let store = MemorySettings::new();
        let (mut keeper, _) = SettingsKeeper::open(store.clone());
        keeper
            .change(GameSettings {
                quicktime_movies: false,
                ..GameSettings::default()
            })
            .expect("saves");
        assert_eq!(json(&store)["quicktime_movies"], false);
        let (restarted, _) = SettingsKeeper::open(store);
        assert!(!restarted.settings().quicktime_movies);
        assert!(!restarted.settings().prefs().quicktime_movies);
        let movies_on = GameSettings::default().with_prefs(Prefs {
            quicktime_movies: false,
            ..Prefs::default()
        });
        assert!(!movies_on.quicktime_movies, "the prefs choose it");
    }

    #[test]
    fn the_map_borders_start_on_and_a_file_without_them_has_them_on() {
        assert!(GameSettings::default().map_borders);
        let (keeper, warning) =
            SettingsKeeper::open(MemorySettings::holding(r#"{"hyperspace_effects": false}"#));
        assert_eq!(warning, None);
        assert!(
            keeper.settings().map_borders,
            "ShowMapBorders' default, YES"
        );
    }

    #[test]
    fn the_map_borders_off_are_saved_and_read_back() {
        let store = MemorySettings::new();
        let (mut keeper, _) = SettingsKeeper::open(store.clone());
        keeper
            .change(GameSettings {
                map_borders: false,
                ..GameSettings::default()
            })
            .expect("saves");
        assert_eq!(json(&store)["show_map_borders"], false);
        let (restarted, _) = SettingsKeeper::open(store);
        assert!(!restarted.settings().map_borders);
    }

    #[test]
    fn the_prefs_keep_the_map_borders() {
        let off = GameSettings {
            map_borders: false,
            ..GameSettings::default()
        };
        let changed = off.with_prefs(Prefs {
            hyperspace_effects: false,
            ..Prefs::default()
        });
        assert!(!changed.map_borders, "the Preferences dialog never has it");
        assert!(!changed.hyperspace_effects);
    }

    fn remembering(dir: &str) -> GameSettings {
        GameSettings {
            nova_files: Some(PathBuf::from(dir)),
            ..GameSettings::default()
        }
    }

    #[test]
    fn no_nova_files_are_remembered_at_first_or_in_a_file_without_them() {
        assert_eq!(GameSettings::default().nova_files, None);
        let (keeper, warning) =
            SettingsKeeper::open(MemorySettings::holding(r#"{"sound": false}"#));
        assert_eq!(warning, None);
        assert_eq!(keeper.settings().nova_files, None);
        let (keeper, warning) = SettingsKeeper::open(MemorySettings::holding(
            r#"{"sound": false, "nova_files": null}"#,
        ));
        assert_eq!(warning, None);
        assert_eq!(keeper.settings().nova_files, None);
    }

    #[test]
    fn remembered_nova_files_are_saved_as_a_string_and_read_back() {
        let store = MemorySettings::holding(r#"{"rules": "bible", "music": false}"#);
        let (mut keeper, _) = SettingsKeeper::open(store.clone());
        keeper
            .change(GameSettings {
                nova_files: Some(PathBuf::from("/Games/EV Nova/Nova Files")),
                ..keeper.settings()
            })
            .expect("saves");
        let saved = json(&store);
        assert_eq!(saved["nova_files"], "/Games/EV Nova/Nova Files");
        assert_eq!(saved["rules"], "bible", "the other keys survive");
        assert_eq!(saved["music"], false);
        let (restarted, _) = SettingsKeeper::open(store);
        assert_eq!(
            restarted.settings().nova_files,
            Some(PathBuf::from("/Games/EV Nova/Nova Files"))
        );
    }

    #[test]
    fn forgotten_nova_files_leave_the_key_out() {
        let store = MemorySettings::holding(r#"{"nova_files": "/gone", "rules": "bible"}"#);
        let (mut keeper, _) = SettingsKeeper::open(store.clone());
        assert_eq!(keeper.settings().nova_files, Some(PathBuf::from("/gone")));
        keeper
            .change(GameSettings {
                nova_files: None,
                ..keeper.settings()
            })
            .expect("saves");
        let saved = json(&store);
        assert_eq!(saved.get("nova_files"), None);
        assert_eq!(saved["rules"], "bible");
    }

    #[test]
    fn remembered_nova_files_of_the_wrong_type_are_not_usable() {
        let (keeper, warning) =
            SettingsKeeper::open(MemorySettings::holding(r#"{"nova_files": 3}"#));
        assert!(warning.is_some());
        assert_eq!(keeper.settings().nova_files, None);
    }

    #[test]
    fn the_prefs_keep_the_remembered_nova_files() {
        let changed = remembering("/Nova Files").with_prefs(Prefs {
            hyperspace_effects: false,
            ..Prefs::default()
        });
        assert_eq!(changed.nova_files, Some(PathBuf::from("/Nova Files")));
    }

    #[cfg(unix)]
    #[test]
    fn a_path_that_is_not_utf8_is_never_saved() {
        use std::os::unix::ffi::OsStrExt;
        let store = MemorySettings::new();
        let (mut keeper, _) = SettingsKeeper::open(store.clone());
        let dir = PathBuf::from(std::ffi::OsStr::from_bytes(b"/Nova \xff Files"));
        keeper
            .change(GameSettings {
                nova_files: Some(dir),
                map_borders: false,
                ..GameSettings::default()
            })
            .expect("saves without panicking");
        assert_eq!(json(&store).get("nova_files"), None);
    }

    fn upscaler_named(name: &str) -> ScalingSetting {
        ScalingSetting::from(name.to_owned())
    }

    #[test]
    fn each_scaling_is_saved_under_the_frame_upscaler_key_by_its_name_and_read_back() {
        for (scaling, name) in [
            (Scaling::Native, "native"),
            (Scaling::SharpBilinear, "sharp_bilinear"),
            (Scaling::MetalFxSpatial, "metalfx_spatial"),
            (Scaling::SuperXbr, "super_xbr"),
        ] {
            let store = MemorySettings::new();
            let (mut keeper, _) = SettingsKeeper::open(store.clone());
            let settings = GameSettings {
                frame_upscaler: ScalingSetting::from(scaling),
                hyperspace_effects: false,
                ..GameSettings::default()
            };
            keeper.change(settings.clone()).expect("saves");
            assert_eq!(json(&store)["frame_upscaler"], name);
            let (restarted, warning) = SettingsKeeper::open(store);
            assert_eq!((restarted.settings(), warning), (settings, None));
            assert_eq!(restarted.settings().frame_upscaler.known(), Some(scaling));
            assert_eq!(restarted.settings().frame_upscaler.name(), name);
        }
        assert_eq!(
            Scaling::ALL.map(|scaling| ScalingSetting::from(scaling).known()),
            Scaling::ALL.map(Some),
            "every scaling has a name"
        );
    }

    #[test]
    fn a_file_phase_6_saved_loads_its_per_frame_choice_unchanged() {
        for (name, scaling) in [
            ("native", Scaling::Native),
            ("sharp_bilinear", Scaling::SharpBilinear),
            ("metalfx_spatial", Scaling::MetalFxSpatial),
        ] {
            let text = format!(r#"{{"music": true, "frame_upscaler": "{name}"}}"#);
            let (keeper, warning) = SettingsKeeper::open(MemorySettings::holding(&text));
            assert_eq!(warning, None, "{name}");
            assert_eq!(keeper.settings().frame_upscaler.known(), Some(scaling));
            assert_eq!(keeper.settings().prefs().scaling, scaling);
        }
    }

    #[test]
    fn a_file_without_a_frame_upscaler_draws_natively() {
        let (keeper, warning) =
            SettingsKeeper::open(MemorySettings::holding(r#"{"sound": false}"#));
        assert_eq!(warning, None);
        assert_eq!(keeper.settings().frame_upscaler.name(), "native");
        assert_eq!(
            keeper.settings().frame_upscaler.known(),
            Some(Scaling::Native)
        );
        assert_eq!(
            GameSettings::default().frame_upscaler,
            upscaler_named("native")
        );
        assert_eq!(ScalingSetting::default(), upscaler_named("native"));
    }

    #[test]
    fn an_unknown_frame_upscaler_loads_and_is_saved_back_as_it_was() {
        for name in ["warp_drive", "hd_pack_v9"] {
            let text = format!(r#"{{"frame_upscaler": "{name}"}}"#);
            let store = MemorySettings::holding(&text);
            let (mut keeper, warning) = SettingsKeeper::open(store.clone());
            assert_eq!(warning, None, "the app warns, not the keeper");
            let settings = keeper.settings();
            assert_eq!(settings.frame_upscaler, upscaler_named(name));
            assert_eq!(settings.frame_upscaler.known(), None);
            assert_eq!(settings.prefs().scaling, Scaling::Native);
            keeper
                .change(settings.with_prefs(Prefs {
                    sound: SoundPrefs {
                        music: false,
                        ..Prefs::default().sound
                    },
                    ..Prefs::default()
                }))
                .expect("saves");
            assert_eq!(json(&store)["music"], false);
            assert_eq!(json(&store)["frame_upscaler"], name);
        }
    }

    #[test]
    fn a_frame_upscaler_of_the_wrong_type_is_not_usable() {
        let (keeper, warning) =
            SettingsKeeper::open(MemorySettings::holding(r#"{"frame_upscaler": 2}"#));
        assert_eq!(keeper.settings(), GameSettings::default());
        assert!(warning.is_some_and(|warning| warning.contains("are not usable")));
    }

    #[test]
    fn the_prefs_never_set_the_stored_frame_upscaler() {
        for stored in [
            "native",
            "sharp_bilinear",
            "metalfx_spatial",
            "super_xbr",
            "warp_drive",
        ] {
            let settings = GameSettings {
                frame_upscaler: upscaler_named(stored),
                ..GameSettings::default()
            };
            for scaling in Scaling::ALL {
                let chosen = settings.clone().with_prefs(Prefs {
                    scaling,
                    ..Prefs::default()
                });
                assert_eq!(chosen.frame_upscaler, upscaler_named(stored), "{scaling:?}");
            }
        }
    }

    #[test]
    fn the_prefs_show_the_stored_frame_upscaler_when_known_and_native_otherwise() {
        let shown = |name: &str| {
            GameSettings {
                frame_upscaler: upscaler_named(name),
                ..GameSettings::default()
            }
            .prefs()
            .scaling
        };
        assert_eq!(shown("sharp_bilinear"), Scaling::SharpBilinear);
        assert_eq!(shown("metalfx_spatial"), Scaling::MetalFxSpatial);
        assert_eq!(shown("super_xbr"), Scaling::SuperXbr);
        assert_eq!(shown("native"), Scaling::Native);
        assert_eq!(shown("warp_drive"), Scaling::Native);
        assert_eq!(shown(""), Scaling::Native);
    }

    #[test]
    fn volumes_out_of_range_are_clamped() {
        let text = r#"{"effects_volume": 3.5, "music_volume": -1}"#;
        let (keeper, _) = SettingsKeeper::open(MemorySettings::holding(text));
        assert_eq!(keeper.settings().audio.effects_volume, Volume::new(0.5));
        assert_eq!(keeper.settings().audio.music_volume, Volume::SILENT);
    }

    #[test]
    fn an_effects_volume_above_the_top_level_loads_at_the_top_level() {
        let text = r#"{"effects_volume": 1.0, "music_volume": 1.0}"#;
        let (keeper, warning) = SettingsKeeper::open(MemorySettings::holding(text));
        assert_eq!(warning, None);
        assert_eq!(keeper.settings().audio.effects_volume, Volume::new(0.5));
        assert_eq!(keeper.settings().audio.music_volume, Volume::FULL);
        assert_eq!(keeper.settings().audio.prefs().effects_level, 8);
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
                "fullscreen": false,
                "fullscreen_kind": "borderless",
                "quicktime_movies": true,
                "show_map_borders": true,
                "frame_upscaler": "native",
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
            "rule_overrides": {"crime_gains": "engine", "nested": {"deeper": [1, {"x": null}]}},
            "rule_values": {"thrust_divisor": 3000}}"#;
        let store = MemorySettings::holding(text);
        let (mut keeper, _) = SettingsKeeper::open(store.clone());
        keeper.change(plain(quiet())).expect("saves");
        assert_eq!(json(&store)["rules"], "bible");
        assert_eq!(
            json(&store)["rule_values"],
            serde_json::json!({"thrust_divisor": 3000})
        );
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
                fullscreen: false,
                fullscreen_kind: FullscreenSetting::Borderless,
                quicktime_movies: true,
                map_borders: true,
                frame_upscaler: ScalingSetting::default(),
                nova_files: None,
            })
            .expect("saves");
        keeper
            .change(GameSettings {
                hyperspace_effects: false,
                quicktime_movies: true,
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
                quicktime_movies: true,
                fullscreen: FullscreenKind::Borderless,
                scaling: Scaling::Native,
            }
        );
        assert_eq!(
            settings.clone().with_prefs(prefs),
            settings,
            "between levels kept"
        );
        let changed = settings.with_prefs(Prefs {
            sound: SoundPrefs {
                effects_level: 3,
                ..prefs.sound
            },
            hyperspace_effects: true,
            ..prefs
        });
        assert_eq!(changed.audio.effects_volume, effects_level_volume(3));
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
            fullscreen: false,
            fullscreen_kind: FullscreenSetting::Borderless,
            quicktime_movies: true,
            map_borders: true,
            frame_upscaler: ScalingSetting::default(),
            nova_files: None,
        };
        assert_eq!((settings, warning), (expected.clone(), None));
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
