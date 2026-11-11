//! The game's sound: the audio core over the device that opened, and the
//! music that loaded, each with a warning when it did not. The settings
//! are [`settings`](crate::settings)'.
//!
//! The music is a value, not a port: start-up reads it once
//! ([`game_music`]) and hands its bytes to the audio device's adapter as
//! it opens (`nova_audio::KiraAudio::open`), which streams them when the
//! audio core starts the music. The stock game's is `Nova Music.mp3`; the
//! new game's is the track its `game.md` names, or none.

use std::fmt::Display;
use std::num::NonZeroUsize;
use std::path::Path;

use nova_audio::voices::ORIGINAL_VOICES;
use nova_audio::{Audio, AudioCore};
use nova_data::GameData;
use nova_data::music::{MusicError, open_music};
use nova_sim::{RuleKey, RuleSource, Rulebook};

use crate::ruleset::Ruleset;

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

/// How many voices the audio device plays at once, as `rulebook`'s
/// [`RuleKey::SoundVoices`] says: the original's 8 by the engine's
/// reading, or as many as are asked for (`None`) by the other, the
/// default.
#[must_use]
pub fn voice_limit(rulebook: &Rulebook) -> Option<NonZeroUsize> {
    match rulebook.source_for(RuleKey::SoundVoices) {
        RuleSource::Engine => Some(ORIGINAL_VOICES),
        RuleSource::Bible => None,
    }
}

/// The music `ruleset`'s game plays, the bytes start-up hands the audio
/// device unchanged, and a warning to print, if any.
///
/// The stock game reads `Nova Music.mp3` in its `Nova Files`, `dir`, as
/// it always has ([`music_warning`]), whatever `data` holds. The new game
/// plays the soundtrack its data names ([`GameData::soundtrack`]) and
/// never looks for `Nova Music.mp3` in its folder: naming none, it plays
/// none and says nothing, and a track named that didn't load was reported
/// with the load's problems, so it isn't warned of again.
#[must_use]
pub fn game_music(
    ruleset: Ruleset,
    data: &GameData,
    dir: &Path,
) -> (Option<Vec<u8>>, Option<String>) {
    match ruleset {
        Ruleset::Stock => music_warning(open_music(dir)),
        Ruleset::NewGame => (data.soundtrack().map(<[u8]>::to_vec), None),
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

#[cfg(test)]
mod tests {
    use std::io;
    use std::path::PathBuf;
    use std::sync::Arc;

    use nova_audio::recording::RecordingAudio;
    use nova_audio::{AudioCommand, AudioSettings};
    use nova_data::music::music_path;
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
        let jump = SimSound::JumpBegan { rate: 0x1_0000 };
        let cue = jump.cue().expect("the warp-up plays");
        core.update(Some(Showing::MainMenu), &[Sound::Sim(jump)]);
        assert_eq!(
            *log.borrow(),
            [
                AudioCommand::StartMusic {
                    volume: AudioSettings::default().music_volume
                },
                AudioCommand::Play {
                    sound: cue.sound,
                    volume: AudioSettings::default().effects_volume,
                    priority: nova_audio::Priority::new(cue.priority),
                },
            ]
        );
    }

    #[test]
    fn the_voices_are_unlimited_by_default_and_the_originals_8_by_the_engine() {
        assert_eq!(voice_limit(&Rulebook::default()), None);
        let engine = Rulebook::default().with_override(RuleKey::SoundVoices, RuleSource::Engine);
        assert_eq!(voice_limit(&engine), NonZeroUsize::new(8));
        assert_eq!(
            voice_limit(&Rulebook::new(RuleSource::Engine)),
            NonZeroUsize::new(8)
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

    /// A temporary `Nova Files` holding a `Nova Music.mp3` of `bytes`.
    fn stock_dir(bytes: &[u8]) -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("a temporary directory");
        std::fs::write(music_path(dir.path()), bytes).expect("writes the music");
        dir
    }

    fn with_soundtrack(bytes: &[u8]) -> GameData {
        GameData::default().with_soundtrack(Arc::from(bytes))
    }

    #[test]
    fn the_new_game_plays_its_soundtrack_and_never_the_stock_file() {
        let dir = stock_dir(b"stock");
        assert_eq!(
            game_music(Ruleset::NewGame, &with_soundtrack(b"theme"), dir.path()),
            (Some(b"theme".to_vec()), None)
        );
    }

    #[test]
    fn the_new_game_naming_no_music_plays_none_and_says_nothing() {
        let stocked = stock_dir(b"stock");
        let empty = tempfile::tempdir().expect("a temporary directory");
        for dir in [stocked.path(), empty.path()] {
            assert_eq!(
                game_music(Ruleset::NewGame, &GameData::default(), dir),
                (None, None)
            );
        }
    }

    #[test]
    fn the_stock_game_plays_its_music_file_whatever_the_store_holds() {
        let dir = stock_dir(b"stock");
        for data in [GameData::default(), with_soundtrack(b"theme")] {
            assert_eq!(
                game_music(Ruleset::Stock, &data, dir.path()),
                (Some(b"stock".to_vec()), None)
            );
        }
    }

    #[test]
    fn the_stock_game_without_its_music_file_warns_as_before() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        assert_eq!(
            game_music(Ruleset::Stock, &with_soundtrack(b"theme"), dir.path()),
            (
                None,
                Some(format!(
                    "nova: playing without music: no music at {}",
                    music_path(dir.path()).display()
                ))
            )
        );
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
}
