//! The game's sound: the audio core over the device that opened, and the
//! music that loaded, each with a warning when it did not. The settings
//! are [`settings`](crate::settings)'.

use std::fmt::Display;
use std::num::NonZeroUsize;

use nova_audio::voices::ORIGINAL_VOICES;
use nova_audio::{Audio, AudioCore};
use nova_data::music::MusicError;
use nova_sim::{RuleKey, RuleSource, Rulebook};

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

    use nova_audio::recording::RecordingAudio;
    use nova_audio::{AudioCommand, AudioSettings};
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
