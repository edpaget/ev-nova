//! The game's sound: the audio core over the device that opened, the
//! music that loaded and the settings that were saved, each with a
//! warning when it did not.

use std::fmt::Display;

use nova_audio::{Audio, AudioCore, AudioSettings, SettingsKeeper, SettingsStore};
use nova_data::music::MusicError;

/// The audio core playing the original's sounds through `device`, the
/// result of opening the audio device, and a warning to print, if any. A
/// device that did not open gives no core: the game runs silently.
pub fn game_audio<A: Audio + 'static, E: Display>(
    device: Result<A, E>,
) -> (Option<AudioCore<Box<dyn Audio>>>, Option<String>) {
    match device {
        Ok(audio) => (Some(AudioCore::new(Box::new(audio))), None),
        Err(error) => (None, Some(format!("nova: running without sound: {error}"))),
    }
}

/// The music's bytes from `music`, the result of reading the soundtrack,
/// and a warning to print, if any: without it the game plays no music.
#[must_use]
pub fn music_warning(music: Result<Vec<u8>, MusicError>) -> (Option<Vec<u8>>, Option<String>) {
    match music {
        Ok(bytes) => (Some(bytes), None),
        Err(error) => (None, Some(format!("nova: playing without music: {error}"))),
    }
}

/// The settings keeper the game saves through.
pub type GameSettings = SettingsKeeper<Box<dyn SettingsStore>>;

/// What the settings saved in `store` give: their keeper, the settings
/// the game starts with, and a warning to print, if any. With no store
/// (nowhere to save them), the game starts with the default settings and
/// does not save them.
#[must_use]
pub fn game_settings(
    store: Option<Box<dyn SettingsStore>>,
) -> (Option<GameSettings>, AudioSettings, Option<String>) {
    match store {
        Some(store) => {
            let (keeper, warning) = SettingsKeeper::open(store);
            let settings = keeper.settings();
            (Some(keeper), settings, warning)
        }
        None => (
            None,
            AudioSettings::default(),
            Some("nova: settings will not be saved: no home or config directory is set".to_owned()),
        ),
    }
}

#[cfg(test)]
mod tests {
    use std::io;
    use std::path::PathBuf;

    use nova_audio::recording::{MemorySettings, RecordingAudio};
    use nova_audio::{AudioCommand, Volume};
    use nova_view::Showing;
    use nova_view::sound::{SimSound, Sound};

    use super::*;

    #[test]
    fn a_device_that_opened_plays_the_originals_sounds() {
        let audio = RecordingAudio::new();
        let log = audio.log();
        let (core, warning) = game_audio::<_, String>(Ok(audio));
        assert_eq!(warning, None);
        let mut core = core.expect("a core");
        core.update(Some(Showing::GalaxyMap), &[Sound::Sim(SimSound::JumpBegan)]);
        assert_eq!(
            *log.borrow(),
            [
                AudioCommand::StartMusic {
                    volume: Volume::FULL
                },
                AudioCommand::Play {
                    sound: nova_data::SoundId(128),
                    volume: Volume::FULL
                },
            ]
        );
    }

    #[test]
    fn a_device_that_did_not_open_runs_silently_with_a_warning() {
        let (core, warning) =
            game_audio::<RecordingAudio, _>(Err("Cannot find the default audio output device"));
        assert!(core.is_none());
        assert_eq!(
            warning.as_deref(),
            Some("nova: running without sound: Cannot find the default audio output device")
        );
    }

    fn path() -> PathBuf {
        PathBuf::from("/Nova Files/Nova Music.mp3")
    }

    #[test]
    fn loaded_music_is_kept_without_a_warning() {
        assert_eq!(
            music_warning(Ok(b"mp3".to_vec())),
            (Some(b"mp3".to_vec()), None)
        );
    }

    #[test]
    fn missing_or_unreadable_music_plays_none_with_a_warning() {
        let (music, warning) = music_warning(Err(MusicError::Missing { path: path() }));
        assert_eq!(music, None);
        assert_eq!(
            warning.as_deref(),
            Some("nova: playing without music: no music at /Nova Files/Nova Music.mp3")
        );
        let unreadable = MusicError::Io {
            path: path(),
            source: io::Error::other("disk on fire"),
        };
        let (music, warning) = music_warning(Err(unreadable));
        assert_eq!(music, None);
        assert_eq!(
            warning.as_deref(),
            Some(
                "nova: playing without music: reading the music \
                 /Nova Files/Nova Music.mp3: disk on fire"
            )
        );
    }

    #[test]
    fn saved_settings_are_kept_and_started_with() {
        let store = MemorySettings::holding(r#"{"music": false, "effects_volume": 0.5}"#);
        let (keeper, settings, warning) = game_settings(Some(Box::new(store.clone())));
        let expected = AudioSettings {
            music: false,
            effects_volume: Volume::new(0.5),
            ..AudioSettings::default()
        };
        assert_eq!((settings, warning), (expected, None));
        let mut keeper = keeper.expect("a keeper");
        assert_eq!(keeper.settings(), expected);
        keeper.change(AudioSettings::default()).expect("saves");
        assert_eq!(store.writes(), 1, "through the store");
    }

    #[test]
    fn unusable_settings_start_with_the_defaults_and_a_warning() {
        let store = MemorySettings::holding("not json");
        let (keeper, settings, warning) = game_settings(Some(Box::new(store)));
        assert!(keeper.is_some(), "a change can still be saved");
        assert_eq!(settings, AudioSettings::default());
        assert!(warning.is_some_and(|warning| warning.starts_with("nova: the saved settings")),);
    }

    #[test]
    fn with_nowhere_to_save_them_the_defaults_are_not_saved() {
        let (keeper, settings, warning) = game_settings(None);
        assert!(keeper.is_none());
        assert_eq!(settings, AudioSettings::default());
        assert_eq!(
            warning.as_deref(),
            Some("nova: settings will not be saved: no home or config directory is set")
        );
    }
}
