//! The sound port, [`SoundBank`]: where the adapter reads the sound each
//! `snd ` ID plays, as a [`Clip`].
//!
//! A clip is a `snd ` resource's bytes ([`Clip::Snd`]), a WAV file's bytes
//! ([`Clip::Wav`]), or silence on purpose ([`Clip::Silent`]). A bank that
//! has nothing for an ID answers `None`: the sound is missing, which the
//! adapter warns of once (`nova: no snd N to play`). A silent clip is no
//! warning: the data fills that cue with nothing.
//!
//! [`SoundBank::clip`] is provided: by default it is the `snd ` resource
//! of the ID ([`SoundBank::snd`]), so a bank that knows only resources
//! (the stock game's) serves them unchanged. [`pcm`] decodes a clip with
//! `nova_data`'s decoders, into the PCM the adapter plays.
//!
//! The banks:
//!
//! - the game data ([`GameData`]): its `snd ` resources, or, for a new
//!   game's store, its sound files (`GameData::sound_files`), which hold
//!   only what the data fills, every other ID silent;
//! - a new game's sound files ([`SoundFiles`]) on their own;
//! - a shared bank (`Rc<T>`), which forwards both methods, as `main`
//!   hands the adapter `Rc<GameData>`.
//!
//! With the `recording` feature (and in this crate's tests),
//! `recording::MemoryBank` is the port's fake, and
//! `recording::check_bank_contract` is the check every bank passes: the
//! stock bank and the new game's here, and the fake itself.

use std::fmt;
use std::rc::Rc;

use nova_data::sound::{Pcm, SoundError, WavError, decode_snd, decode_wav};
use nova_data::{GameData, SoundFiles, SoundId};
use nova_rsrc::ResType;

/// The `snd ` resource type.
pub(crate) const SND: ResType = ResType::new(*b"snd ");

/// What one `snd ` ID plays.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Clip {
    /// A `snd ` resource's bytes, decoded by `nova_data::sound::decode_snd`.
    Snd(Vec<u8>),
    /// A WAV file's bytes, decoded by `nova_data::sound::decode_wav`.
    Wav(Vec<u8>),
    /// Nothing, on purpose: the data leaves the sound empty.
    Silent,
}

/// Where the adapter reads sounds from (see the module docs).
pub trait SoundBank {
    /// The raw bytes of `snd ` `id`, if there is one.
    fn snd(&self, id: SoundId) -> Option<Vec<u8>>;

    /// What `id` plays: `None` when the bank has nothing for it, which is
    /// a missing sound. By default, the `snd ` resource ([`Self::snd`]).
    fn clip(&self, id: SoundId) -> Option<Clip> {
        self.snd(id).map(Clip::Snd)
    }
}

/// The game data's `snd ` resources; or, when it has sound files (a new
/// game's store), those, and silence for every ID they don't fill.
impl SoundBank for GameData {
    fn snd(&self, id: SoundId) -> Option<Vec<u8>> {
        Some(self.resource(SND, id.0)?.resource.data().to_vec())
    }

    fn clip(&self, id: SoundId) -> Option<Clip> {
        match self.sound_files() {
            Some(files) => files.clip(id),
            None => self.snd(id).map(Clip::Snd),
        }
    }
}

/// A new game's clips: each a WAV file, and every ID they don't fill
/// silent. They hold no `snd ` resource.
impl SoundBank for SoundFiles {
    fn snd(&self, _: SoundId) -> Option<Vec<u8>> {
        None
    }

    fn clip(&self, id: SoundId) -> Option<Clip> {
        Some(
            self.get(id)
                .map_or(Clip::Silent, |bytes| Clip::Wav(bytes.to_vec())),
        )
    }
}

/// A shared bank is a bank: both methods go to the shared one.
impl<T: SoundBank + ?Sized> SoundBank for Rc<T> {
    fn snd(&self, id: SoundId) -> Option<Vec<u8>> {
        (**self).snd(id)
    }

    fn clip(&self, id: SoundId) -> Option<Clip> {
        (**self).clip(id)
    }
}

/// Why a clip doesn't decode: its decoder's error, whose text is the
/// error's own.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ClipError {
    /// A `snd ` resource that doesn't decode.
    Snd(SoundError),
    /// A WAV file that doesn't decode.
    Wav(WavError),
}

impl fmt::Display for ClipError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Snd(error) => error.fmt(f),
            Self::Wav(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for ClipError {}

/// `clip` decoded: its PCM, or `None` for silence.
///
/// # Errors
///
/// [`ClipError`] with the decoder's error when its bytes don't decode.
pub fn pcm(clip: &Clip) -> Result<Option<Pcm>, ClipError> {
    match clip {
        Clip::Snd(bytes) => decode_snd(bytes).map(Some).map_err(ClipError::Snd),
        Clip::Wav(bytes) => decode_wav(bytes).map(Some).map_err(ClipError::Wav),
        Clip::Silent => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::Path;
    use std::sync::Arc;

    use nova_data::sound::fixture::{Header, SndBuilder, SndFormat, wav};
    use nova_data::store::fixture::{DATA_DIR, one_file_at};
    use nova_rsrc::fixture::ForkBuilder;

    use super::*;

    /// An 8-bit mono `snd ` of `samples` at 11127 Hz.
    fn snd(samples: Vec<u8>) -> Vec<u8> {
        let header = Header::Standard {
            rate: 0x2B77_0000,
            loop_points: (0, 0),
            base_note: 60,
            samples,
        };
        SndBuilder::new(SndFormat::Two, header).bytes()
    }

    /// A store of one file holding `snd ` 600.
    fn stock() -> GameData {
        let fork = ForkBuilder::new()
            .resource(SND, 600, Some(b"Menu button down"), &snd(vec![1, 2]))
            .build()
            .bytes;
        one_file_at(Path::new(DATA_DIR).join("Nova Sounds"), fork).load()
    }

    fn files(entries: &[(i16, &[u8])]) -> SoundFiles {
        SoundFiles::new(
            entries
                .iter()
                .map(|&(id, bytes)| (SoundId(id), Arc::from(bytes)))
                .collect::<BTreeMap<_, _>>(),
        )
    }

    /// A bank that knows only resources: `snd ` 1.
    struct OnlySnd;

    impl SoundBank for OnlySnd {
        fn snd(&self, id: SoundId) -> Option<Vec<u8>> {
            (id == SoundId(1)).then(|| b"bytes".to_vec())
        }
    }

    #[test]
    fn the_default_clip_is_the_snd_resource() {
        assert_eq!(OnlySnd.clip(SoundId(1)), Some(Clip::Snd(b"bytes".to_vec())));
        assert_eq!(OnlySnd.clip(SoundId(2)), None, "missing, not silent");
    }

    #[test]
    fn a_stock_store_s_clip_is_its_resource_or_missing() {
        let data = stock();
        assert_eq!(data.clip(SoundId(600)), Some(Clip::Snd(snd(vec![1, 2]))));
        assert_eq!(data.clip(SoundId(601)), None);
        assert_eq!(GameData::default().clip(SoundId(600)), None);
    }

    #[test]
    fn a_store_with_sound_files_plays_only_them_and_is_silent_elsewhere() {
        let clip = wav(1, 8000, 16, &[1, 2]);
        let data = stock().with_sound_files(files(&[(200, &clip)]));
        assert_eq!(data.clip(SoundId(200)), Some(Clip::Wav(clip.clone())));
        assert_eq!(
            data.clip(SoundId(600)),
            Some(Clip::Silent),
            "its resource isn't read"
        );
        assert_eq!(data.clip(SoundId(601)), Some(Clip::Silent));
        let alone = files(&[(200, &clip)]);
        assert_eq!(alone.snd(SoundId(200)), None, "no resources");
        assert_eq!(alone.clip(SoundId(200)), Some(Clip::Wav(clip)));
        assert_eq!(alone.clip(SoundId(153)), Some(Clip::Silent));
    }

    #[test]
    fn a_shared_bank_forwards_both_methods() {
        let clip = wav(1, 8000, 16, &[1, 2]);
        let shared = Rc::new(GameData::default().with_sound_files(files(&[(200, &clip)])));
        assert_eq!(shared.clip(SoundId(200)), Some(Clip::Wav(clip)));
        assert_eq!(shared.clip(SoundId(201)), Some(Clip::Silent));
        assert_eq!(shared.snd(SoundId(200)), None);
        let stock = Rc::new(stock());
        assert_eq!(stock.snd(SoundId(600)), Some(snd(vec![1, 2])));
        assert_eq!(stock.clip(SoundId(600)), Some(Clip::Snd(snd(vec![1, 2]))));
    }

    #[test]
    fn pcm_decodes_each_kind_of_clip_and_silence_is_none() {
        let from_snd = pcm(&Clip::Snd(snd(vec![0x80, 0xC0]))).expect("decodes");
        assert_eq!(from_snd, Some(decode_snd(&snd(vec![0x80, 0xC0])).unwrap()));
        let bytes = wav(2, 22_050, 16, &[1, -1, 2, -2]);
        let from_wav = pcm(&Clip::Wav(bytes.clone())).expect("decodes");
        assert_eq!(from_wav, Some(decode_wav(&bytes).unwrap()));
        assert_eq!(pcm(&Clip::Silent), Ok(None));
    }

    #[test]
    fn a_clip_that_does_not_decode_gives_its_decoder_s_error_and_text() {
        let broken_snd = b"\0\x07 not a snd".to_vec();
        let snd_error = decode_snd(&broken_snd).expect_err("broken");
        let error = pcm(&Clip::Snd(broken_snd)).expect_err("broken");
        assert_eq!(error, ClipError::Snd(snd_error.clone()));
        assert_eq!(error.to_string(), snd_error.to_string());
        let wav_error = decode_wav(b"RIFF").expect_err("broken");
        let error = pcm(&Clip::Wav(b"RIFF".to_vec())).expect_err("broken");
        assert_eq!(error, ClipError::Wav(wav_error.clone()));
        assert_eq!(error.to_string(), wav_error.to_string());
        assert!(
            pcm(&Clip::Wav(snd(vec![0x80]))).is_err(),
            "a snd is no WAV file"
        );
    }

    // The contract, over each real bank.

    use crate::recording::check_bank_contract;

    #[test]
    fn the_stock_bank_and_its_shared_one_keep_the_contract() {
        let data = stock();
        assert_eq!(
            check_bank_contract(&data, SoundId(600), SoundId(601)),
            Ok(())
        );
        let shared = Rc::new(data);
        assert_eq!(
            check_bank_contract(&shared, SoundId(600), SoundId(601)),
            Ok(())
        );
    }

    #[test]
    fn the_new_game_s_bank_and_its_shared_one_keep_the_contract() {
        let clip = wav(1, 8000, 16, &[1, -1, 2]);
        let sounds = files(&[(200, &clip), (151, &clip)]);
        assert_eq!(
            check_bank_contract(&sounds, SoundId(200), SoundId(153)),
            Ok(())
        );
        let data = GameData::default().with_sound_files(sounds);
        assert_eq!(
            check_bank_contract(&data, SoundId(151), SoundId(153)),
            Ok(())
        );
        let shared = Rc::new(data);
        assert_eq!(
            check_bank_contract(&shared, SoundId(200), SoundId(153)),
            Ok(())
        );
    }
}
