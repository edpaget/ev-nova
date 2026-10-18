//! The kira adapter: plays the core's commands on an audio device.
//!
//! [`KiraAudio`] is thin: each [`AudioCommand`] becomes one or two kira
//! calls, with the `snd ` resources read through the [`SoundBank`] port
//! and decoded once each by `nova_data`'s decoder. Its pure parts (PCM to
//! kira frames, volume to decibels, the sound cache) are tested on their
//! own, and the adapter as a whole on kira's device-free `MockBackend`.
//! Only [`KiraAudio::open`] opens a real device (through cpal).
//!
//! Every sound played once (an effect, the mission channel's, one played
//! unless playing) takes a voice in one [`VoiceList`], as the original's
//! sound library keeps them: the list decides where it goes, or that it
//! isn't played, and which voice it evicts, and the adapter stops the
//! evicted sound at once. A voice leaves the list the moment it is halted,
//! evicted or stopped, and is let go once kira says it has stopped. The
//! engine loop and the music are not voices. [`KiraAudio::with_voices`]
//! sets how many voices there are.
//!
//! A sound that is missing or cannot be decoded is reported once, as a
//! warning, and is silent from then on; so is music that cannot be
//! decoded. Warnings go to stderr unless [`KiraAudio::with_warnings`] says
//! otherwise.

use std::collections::HashMap;
use std::io::{self, Cursor, Write};
use std::num::NonZeroUsize;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use ::kira::backend::Backend;
use ::kira::sound::static_sound::{StaticSoundData, StaticSoundHandle, StaticSoundSettings};
use ::kira::sound::streaming::{StreamingSoundData, StreamingSoundHandle};
use ::kira::sound::{FromFileError, PlaybackState};
use ::kira::{AudioManager, AudioManagerSettings, Decibels, DefaultBackend, Frame, Tween};
use nova_data::sound::{Pcm, decode_snd};
use nova_data::{GameData, SoundId};
use nova_rsrc::ResType;

use crate::port::{Audio, AudioCommand, Priority, Volume};
use crate::voices::{ORIGINAL_VOICES, Placement, Voice, VoiceList};

/// Why the audio device could not be opened.
pub use ::kira::backend::cpal::Error as OpenError;

/// The `snd ` resource type.
const SND: ResType = ResType::new(*b"snd ");

/// Where the adapter reads sounds from.
pub trait SoundBank {
    /// The raw bytes of `snd ` `id`, if there is one.
    fn snd(&self, id: SoundId) -> Option<Vec<u8>>;
}

/// The game data's `snd ` resources.
impl SoundBank for GameData {
    fn snd(&self, id: SoundId) -> Option<Vec<u8>> {
        Some(self.resource(SND, id.0)?.resource.data().to_vec())
    }
}

/// A shared bank is a bank.
impl<T: SoundBank + ?Sized> SoundBank for Rc<T> {
    fn snd(&self, id: SoundId) -> Option<Vec<u8>> {
        (**self).snd(id)
    }
}

/// `pcm`'s samples as kira frames, -1.0 to just under 1.0: a mono sample
/// goes to both channels, and stereo samples stay in pairs.
#[must_use]
pub fn frames(pcm: &Pcm) -> Vec<Frame> {
    let scale = |sample: i16| f32::from(sample) / 32768.0;
    match pcm.channels() {
        1 => pcm
            .samples()
            .iter()
            .map(|&sample| Frame::from_mono(scale(sample)))
            .collect(),
        _ => pcm
            .samples()
            .as_chunks::<2>()
            .0
            .iter()
            .map(|&[left, right]| Frame::new(scale(left), scale(right)))
            .collect(),
    }
}

/// `pcm` as kira sound data, at its sample rate to the nearest hertz.
#[must_use]
pub fn sound_data(pcm: &Pcm) -> StaticSoundData {
    StaticSoundData {
        sample_rate: pcm.sample_rate().nearest_hz(),
        frames: frames(pcm).into(),
        settings: StaticSoundSettings::default(),
        slice: None,
    }
}

/// A linear `volume` in decibels: 20 log10 of it, and silence for 0.
#[must_use]
pub fn decibels(volume: Volume) -> Decibels {
    let amplitude = volume.amplitude();
    if amplitude > 0.0 {
        Decibels(20.0 * amplitude.log10())
    } else {
        Decibels::SILENCE
    }
}

/// Each `snd ` decoded once, or remembered as unplayable.
#[derive(Default)]
pub struct SoundCache {
    sounds: HashMap<SoundId, Option<StaticSoundData>>,
}

impl SoundCache {
    /// `snd ` `id` from `bank`, decoded the first time it is asked for. A
    /// sound that is missing, does not decode or has no samples (which
    /// kira cannot loop) is reported to `warnings` the first time, and is
    /// `None` every time.
    pub fn get(
        &mut self,
        bank: &dyn SoundBank,
        id: SoundId,
        warnings: &mut dyn Write,
    ) -> Option<StaticSoundData> {
        self.sounds
            .entry(id)
            .or_insert_with(|| {
                let Some(bytes) = bank.snd(id) else {
                    warn(warnings, &format!("no snd {} to play", id.0));
                    return None;
                };
                match decode_snd(&bytes).map(|pcm| sound_data(&pcm)) {
                    Ok(data) if !data.frames.is_empty() => Some(data),
                    Ok(_) => {
                        warn(warnings, &format!("snd {} has no samples to play", id.0));
                        None
                    }
                    Err(error) => {
                        warn(warnings, &format!("cannot play snd {}: {error}", id.0));
                        None
                    }
                }
            })
            .clone()
    }
}

/// Writes `message` to `warnings` as one of nova's own lines. A warning
/// that cannot be written is dropped: there is nowhere else to say so.
fn warn(warnings: &mut dyn Write, message: &str) {
    let _ = writeln!(warnings, "nova: {message}");
}

/// Which command started a voice.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Role {
    /// [`AudioCommand::Play`].
    Effect,
    /// [`AudioCommand::PlayExclusive`]: the one mission channel's.
    Mission,
    /// [`AudioCommand::PlayUnlessPlaying`].
    Single,
}

/// A voice's hold on its sound: kira's handle, the level it was played
/// at, and the command that started it.
struct Held {
    handle: StaticSoundHandle,
    level: Decibels,
    role: Role,
}

/// Whether `voice`'s sound has not stopped.
fn sounding(voice: &Voice<Held>) -> bool {
    voice.handle.handle.state() != PlaybackState::Stopped
}

/// Stops `voices`' sounds at once.
fn stop_all(voices: impl IntoIterator<Item = Voice<Held>>) {
    for mut voice in voices {
        voice.handle.handle.stop(Tween::default());
    }
}

/// Plays the core's commands through a kira [`AudioManager`].
pub struct KiraAudio<B: Backend = DefaultBackend> {
    manager: AudioManager<B>,
    bank: Box<dyn SoundBank>,
    cache: SoundCache,
    /// The soundtrack's bytes, until they turn out not to decode.
    music: Option<Arc<[u8]>>,
    warnings: Box<dyn Write>,
    /// The sounds played once, each on its voice, until each has stopped
    /// or is halted, evicted or stopped.
    voices: VoiceList<Held>,
    /// The engine loop last started, stopped or not.
    engine: Option<StaticSoundHandle>,
    /// The level last applied to the engine loop.
    engine_level: Option<Decibels>,
    /// The music last started, stopped or not.
    track: Option<StreamingSoundHandle<FromFileError>>,
    /// The level last applied to the music.
    music_level: Option<Decibels>,
    /// Whether the music last started was stopped or faded out by a
    /// command, so its stopping is not its end.
    music_stopped: bool,
}

impl KiraAudio<DefaultBackend> {
    /// Opens the default audio device, playing sounds from `bank` and
    /// `music`, the soundtrack's bytes, if there are any.
    pub fn open(bank: impl SoundBank + 'static, music: Option<Vec<u8>>) -> Result<Self, OpenError> {
        Ok(Self::with_manager(
            AudioManager::new(AudioManagerSettings::default())?,
            bank,
            music,
        ))
    }
}

impl<B: Backend> KiraAudio<B> {
    /// Plays through `manager`, sounds from `bank` and `music`, the
    /// soundtrack's bytes, if there are any. Warnings go to stderr.
    pub fn with_manager(
        manager: AudioManager<B>,
        bank: impl SoundBank + 'static,
        music: Option<Vec<u8>>,
    ) -> Self {
        Self {
            manager,
            bank: Box::new(bank),
            cache: SoundCache::default(),
            music: music.map(Into::into),
            warnings: Box::new(io::stderr()),
            voices: VoiceList::new(Some(ORIGINAL_VOICES)),
            engine: None,
            engine_level: None,
            track: None,
            music_level: None,
            music_stopped: false,
        }
    }

    /// The adapter with its warnings going to `warnings`.
    #[must_use]
    pub fn with_warnings(self, warnings: impl Write + 'static) -> Self {
        Self {
            warnings: Box::new(warnings),
            ..self
        }
    }

    /// The adapter with `limit` voices, or as many as are asked for
    /// (`None`), in place of the original's 8 ([`ORIGINAL_VOICES`]); set
    /// before anything plays.
    #[must_use]
    pub fn with_voices(self, limit: Option<NonZeroUsize>) -> Self {
        Self {
            voices: VoiceList::new(limit),
            ..self
        }
    }

    /// The state of each one-shot effect still on a voice, in the voice
    /// list's order.
    #[must_use]
    pub fn effect_states(&self) -> Vec<PlaybackState> {
        self.held(Role::Effect)
            .map(|held| held.handle.state())
            .collect()
    }

    /// The level each one-shot effect still on a voice was played at.
    #[must_use]
    pub fn effect_levels(&self) -> Vec<Decibels> {
        self.held(Role::Effect).map(|held| held.level).collect()
    }

    /// The state of the mission sound on its voice, if one is.
    #[must_use]
    pub fn mission_state(&self) -> Option<PlaybackState> {
        self.held(Role::Mission)
            .next()
            .map(|held| held.handle.state())
    }

    /// The state of `sound` played unless it was playing, if it is on a
    /// voice.
    #[must_use]
    pub fn single_state(&self, sound: SoundId) -> Option<PlaybackState> {
        self.voices
            .iter()
            .find(|voice| voice.sound == sound && voice.handle.role == Role::Single)
            .map(|voice| voice.handle.handle.state())
    }

    /// The `snd ` on each voice, first the one that matters most.
    #[must_use]
    pub fn voice_sounds(&self) -> Vec<SoundId> {
        self.voices.iter().map(|voice| voice.sound).collect()
    }

    /// The voices started by `role`'s command, in order.
    fn held(&self, role: Role) -> impl Iterator<Item = &Held> {
        self.voices
            .iter()
            .map(|voice| &voice.handle)
            .filter(move |held| held.role == role)
    }

    /// The state of the engine loop last started, if any.
    #[must_use]
    pub fn loop_state(&self) -> Option<PlaybackState> {
        self.engine.as_ref().map(StaticSoundHandle::state)
    }

    /// The level last applied to the engine loop, if any.
    #[must_use]
    pub fn loop_level(&self) -> Option<Decibels> {
        self.engine_level
    }

    /// The state of the music last started, if any.
    #[must_use]
    pub fn music_state(&self) -> Option<PlaybackState> {
        self.track.as_ref().map(StreamingSoundHandle::state)
    }

    /// The level last applied to the music, if any.
    #[must_use]
    pub fn music_level(&self) -> Option<Decibels> {
        self.music_level
    }
}

impl<B: Backend> KiraAudio<B> {
    /// `sound` decoded, if it can be played.
    fn sound(&mut self, sound: SoundId) -> Option<StaticSoundData> {
        self.cache.get(&*self.bank, sound, &mut *self.warnings)
    }

    /// Plays `sound` once at `volume` on a voice, by `priority`, started by
    /// `role`'s command: the voices that have stopped are let go, then the
    /// list places it, or it isn't played, and the voice it evicts is
    /// stopped. A sound that cannot be played takes no voice, nor does one
    /// kira has no room for.
    fn start(&mut self, sound: SoundId, volume: Volume, priority: Priority, role: Role) {
        self.voices.retain(sounding);
        let Placement::At(index) = self.voices.place(priority, volume) else {
            return;
        };
        let level = decibels(volume);
        let Some(data) = self.sound(sound) else {
            return;
        };
        let Ok(handle) = self.manager.play(data.volume(level)) else {
            return;
        };
        let voice = Voice {
            sound,
            priority,
            volume,
            handle: Held {
                handle,
                level,
                role,
            },
        };
        stop_all(self.voices.insert(index, voice));
    }

    /// Plays `sound` on the mission channel, unless the last sound played
    /// there is still on its voice and has not stopped.
    fn play_exclusive(&mut self, sound: SoundId, volume: Volume, priority: Priority) {
        let playing = self
            .voices
            .any(|voice| voice.handle.role == Role::Mission && sounding(voice));
        if !playing {
            self.start(sound, volume, priority, Role::Mission);
        }
    }

    /// Plays `sound`, unless it plays on any voice, however it was started
    /// (`ST_IsSoundPlaying`).
    fn play_unless_playing(&mut self, sound: SoundId, volume: Volume, priority: Priority) {
        if !self.playing(sound) {
            self.start(sound, volume, priority, Role::Single);
        }
    }

    /// Stops the engine loop, if it is playing.
    fn stop_loop(&mut self) {
        if let Some(engine) = &mut self.engine {
            engine.stop(Tween::default());
        }
    }

    /// Loops `sound` from end to end at `level`, in place of the engine
    /// loop before.
    fn start_loop(&mut self, sound: SoundId, level: Decibels) {
        self.stop_loop();
        self.engine = None;
        if let Some(data) = self.sound(sound)
            && let Ok(handle) = self.manager.play(data.loop_region(..).volume(level))
        {
            self.engine = Some(handle);
            self.engine_level = Some(level);
        }
    }

    /// Stops the music, if it is playing, over `fade`.
    fn stop_music(&mut self, fade: Duration) {
        if let Some(track) = &mut self.track {
            track.stop(Tween {
                duration: fade,
                ..Tween::default()
            });
            self.music_stopped = true;
        }
    }

    /// Streams the soundtrack from the top, once, at `level`: it plays to
    /// its end and stops. Music that does not decode is reported once and
    /// never tried again.
    fn start_music(&mut self, level: Decibels) {
        self.stop_music(Tween::default().duration);
        self.track = None;
        self.music_stopped = false;
        let Some(bytes) = &self.music else {
            return;
        };
        match StreamingSoundData::from_cursor(Cursor::new(Arc::clone(bytes))) {
            Ok(data) => {
                if let Ok(track) = self.manager.play(data.volume(level)) {
                    self.track = Some(track);
                    self.music_level = Some(level);
                }
            }
            Err(error) => {
                warn(
                    &mut *self.warnings,
                    &format!("cannot play the music: {error}"),
                );
                self.music = None;
            }
        }
    }
}

impl<B: Backend> Audio for KiraAudio<B> {
    fn run(&mut self, command: AudioCommand) {
        match command {
            AudioCommand::Play {
                sound,
                volume,
                priority,
            } => self.start(sound, volume, priority, Role::Effect),
            AudioCommand::PlayExclusive {
                sound,
                volume,
                priority,
            } => self.play_exclusive(sound, volume, priority),
            AudioCommand::PlayUnlessPlaying {
                sound,
                volume,
                priority,
            } => self.play_unless_playing(sound, volume, priority),
            AudioCommand::StartLoop { sound, volume } => self.start_loop(sound, decibels(volume)),
            AudioCommand::SetLoopVolume(volume) => {
                let level = decibels(volume);
                self.engine_level = Some(level);
                if let Some(engine) = &mut self.engine {
                    engine.set_volume(level, Tween::default());
                }
            }
            AudioCommand::StopLoop => self.stop_loop(),
            AudioCommand::StartMusic { volume } => self.start_music(decibels(volume)),
            AudioCommand::SetMusicVolume(volume) => {
                let level = decibels(volume);
                self.music_level = Some(level);
                if let Some(track) = &mut self.track {
                    track.set_volume(level, Tween::default());
                }
            }
            AudioCommand::StopMusic => self.stop_music(Tween::default().duration),
            AudioCommand::FadeOutMusic(fade) => self.stop_music(fade),
            AudioCommand::Halt { sound } => stop_all(self.voices.remove_sound(sound)),
            AudioCommand::StopEffects => {
                stop_all(self.voices.clear());
                self.stop_loop();
            }
        }
    }

    fn music_ended(&self) -> bool {
        !self.music_stopped
            && self
                .track
                .as_ref()
                .is_some_and(|track| track.state() == PlaybackState::Stopped)
    }

    /// Whether a voice plays `sound` and has not stopped, whichever
    /// command started it.
    fn playing(&self, sound: SoundId) -> bool {
        self.voices
            .any(|voice| voice.sound == sound && sounding(voice))
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::cell::RefCell;
    use std::path::Path;

    use ::kira::backend::mock::{MockBackend, MockBackendSettings};
    use nova_data::sound::fixture::{Header, SndBuilder, SndFormat};
    use nova_data::store::fs::{DirLister, EntryKind, Listing};
    use nova_rsrc::fixture::ForkBuilder;
    use nova_rsrc::{Fork, ForkReader};

    use super::*;

    /// 11127 Hz, the stock standard headers' rate, in 16.16.
    const RATE: u32 = 0x2B77_0000;

    /// An 8-bit mono `snd ` of `samples`.
    fn mono(samples: Vec<u8>) -> Vec<u8> {
        let header = Header::Standard {
            rate: RATE,
            loop_points: (0, 0),
            base_note: 60,
            samples,
        };
        SndBuilder::new(SndFormat::Two, header).bytes()
    }

    /// A 16-bit stereo `snd ` of `samples`, left and right interleaved.
    fn stereo(samples: &[i16]) -> Vec<u8> {
        let header = Header::Extended {
            channels: 2,
            rate: 0xAC44_0000,
            sample_size: 16,
            data: samples.iter().flat_map(|s| s.to_be_bytes()).collect(),
        };
        SndBuilder::new(SndFormat::One, header).bytes()
    }

    fn pcm(snd: &[u8]) -> Pcm {
        decode_snd(snd).expect("decodes")
    }

    // The pure parts.

    #[test]
    fn mono_samples_go_to_both_channels_scaled_to_one() {
        // 0x80 is silence, 0x00 the lowest and 0xC0 half way up.
        let frames = frames(&pcm(&mono(vec![0x80, 0x00, 0xC0, 0xFF])));
        assert_eq!(
            frames,
            [
                Frame::new(0.0, 0.0),
                Frame::new(-1.0, -1.0),
                Frame::new(0.5, 0.5),
                Frame::new(32512.0 / 32768.0, 32512.0 / 32768.0),
            ]
        );
    }

    #[test]
    fn stereo_samples_stay_in_pairs() {
        let frames = frames(&pcm(&stereo(&[i16::MIN, i16::MAX, 16384, -8192])));
        assert_eq!(
            frames,
            [Frame::new(-1.0, 32767.0 / 32768.0), Frame::new(0.5, -0.25),]
        );
    }

    #[test]
    fn sound_data_is_the_frames_at_the_nearest_whole_rate() {
        let data = sound_data(&pcm(&mono(vec![0x80, 0xC0])));
        assert_eq!(data.sample_rate, 11_127);
        assert_eq!(
            data.frames[..],
            [Frame::new(0.0, 0.0), Frame::new(0.5, 0.5)]
        );
        assert_eq!(data.slice, None);
        let stereo = sound_data(&pcm(&stereo(&[0, 0])));
        assert_eq!(stereo.sample_rate, 44_100);
    }

    #[test]
    fn volumes_become_decibels_with_zero_silent() {
        assert_eq!(decibels(Volume::FULL), Decibels::IDENTITY);
        assert_eq!(decibels(Volume::SILENT), Decibels::SILENCE);
        assert_eq!(decibels(Volume::new(0.1)), Decibels(-20.0));
        assert!((decibels(Volume::new(0.5)).0 + 6.0206).abs() < 1e-4);
        assert_eq!(decibels(Volume::new(0.001)), Decibels(-60.0));
        assert!(decibels(Volume::new(0.0001)).0 < -60.0, "quieter still");
    }

    /// One data file, `/data/Nova Sounds`, holding a fork.
    struct OneFile(Vec<u8>);

    impl DirLister for OneFile {
        fn list(&self, _dir: &Path) -> io::Result<Vec<Listing>> {
            Ok(vec![Listing {
                name: "Nova Sounds".into(),
                kind: EntryKind::File,
            }])
        }
    }

    impl ForkReader for OneFile {
        fn read_fork(&self, _path: &Path, fork: Fork) -> io::Result<Option<Vec<u8>>> {
            Ok((fork == Fork::Data).then(|| self.0.clone()))
        }
    }

    #[test]
    fn the_game_datas_bank_is_its_snd_resources() {
        let fork = ForkBuilder::new()
            .resource(SND, 600, Some(b"Menu button down"), &mono(vec![1, 2]))
            .resource(ResType::new(*b"PICT"), 601, None, b"not a sound")
            .build()
            .bytes;
        let file = OneFile(fork);
        let data = GameData::load(&file, &file, Path::new("/data"), None).expect("opens");
        assert_eq!(data.snd(SoundId(600)), Some(mono(vec![1, 2])));
        assert_eq!(data.snd(SoundId(601)), None, "a PICT, not a snd");
        assert_eq!(data.snd(SoundId(602)), None);
        let shared = Rc::new(data);
        assert_eq!(shared.snd(SoundId(600)), Some(mono(vec![1, 2])));
        assert_eq!(shared.snd(SoundId(602)), None);
    }

    /// A bank of canned sounds that counts what it is asked for.
    #[derive(Default)]
    struct FakeBank {
        sounds: Vec<(SoundId, Vec<u8>)>,
        asked: RefCell<Vec<SoundId>>,
    }

    impl SoundBank for FakeBank {
        fn snd(&self, id: SoundId) -> Option<Vec<u8>> {
            self.asked.borrow_mut().push(id);
            self.sounds
                .iter()
                .find(|(sound, _)| *sound == id)
                .map(|(_, bytes)| bytes.clone())
        }
    }

    /// A shared buffer the warnings go to.
    #[derive(Clone, Default)]
    struct Warnings(Rc<RefCell<Vec<u8>>>);

    impl Warnings {
        fn text(&self) -> String {
            String::from_utf8(self.0.borrow().clone()).expect("UTF-8")
        }
    }

    impl Write for Warnings {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.0.borrow_mut().extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    /// 1 is a sound, 2 a broken one; nothing else is there.
    fn bank() -> FakeBank {
        FakeBank {
            sounds: vec![
                (SoundId(1), mono(vec![0x80; 11_127])),
                (SoundId(2), b"\0\x07 not a snd".to_vec()),
            ],
            asked: RefCell::default(),
        }
    }

    #[test]
    fn the_cache_decodes_each_sound_once() {
        let (bank, mut cache, mut warnings) = (bank(), SoundCache::default(), Warnings::default());
        let first = cache.get(&bank, SoundId(1), &mut warnings).expect("plays");
        assert_eq!(first, sound_data(&pcm(&mono(vec![0x80; 11_127]))));
        let again = cache.get(&bank, SoundId(1), &mut warnings).expect("plays");
        assert_eq!(again, first);
        assert_eq!(*bank.asked.borrow(), [SoundId(1)], "read once");
        assert_eq!(warnings.text(), "");
    }

    #[test]
    fn a_missing_or_broken_sound_warns_once_and_stays_silent() {
        let (bank, mut cache, mut warnings) = (bank(), SoundCache::default(), Warnings::default());
        assert_eq!(cache.get(&bank, SoundId(3), &mut warnings), None);
        assert_eq!(warnings.text(), "nova: no snd 3 to play\n");
        assert_eq!(cache.get(&bank, SoundId(3), &mut warnings), None);
        assert_eq!(cache.get(&bank, SoundId(2), &mut warnings), None);
        let error = decode_snd(b"\0\x07 not a snd").expect_err("broken");
        assert_eq!(
            warnings.text(),
            format!("nova: no snd 3 to play\nnova: cannot play snd 2: {error}\n")
        );
        assert_eq!(cache.get(&bank, SoundId(2), &mut warnings), None);
        assert_eq!(*bank.asked.borrow(), [SoundId(3), SoundId(2)], "once each");
        assert_eq!(warnings.text().lines().count(), 2);
    }

    #[test]
    fn a_sound_with_no_samples_warns_once_and_stays_silent() {
        // Looping it would spin kira's mixer forever.
        let bank = FakeBank {
            sounds: vec![(SoundId(4), mono(Vec::new()))],
            ..FakeBank::default()
        };
        let (mut cache, mut warnings) = (SoundCache::default(), Warnings::default());
        assert_eq!(cache.get(&bank, SoundId(4), &mut warnings), None);
        assert_eq!(cache.get(&bank, SoundId(4), &mut warnings), None);
        assert_eq!(warnings.text(), "nova: snd 4 has no samples to play\n");
    }

    // The adapter, on kira's device-free backend.

    /// Silent MPEG-1 Layer III frames: 128 kbit/s at 44.1 kHz, joint
    /// stereo, so each is 417 bytes (144 x 128000 / 44100), its 4-byte
    /// header followed by zeroed side information and main data.
    fn silent_mp3(frames: usize) -> Vec<u8> {
        let mut frame = vec![0; 417];
        frame[..4].copy_from_slice(&[0xFF, 0xFB, 0x90, 0x64]);
        frame.repeat(frames)
    }

    type Adapter = KiraAudio<MockBackend>;

    /// An adapter over the mock backend at 48 kHz playing `bank()` and
    /// `music`, its warnings kept.
    fn adapter(music: Option<Vec<u8>>) -> (Adapter, Warnings) {
        let settings = AudioManagerSettings {
            backend_settings: MockBackendSettings {
                sample_rate: 48_000,
            },
            ..AudioManagerSettings::default()
        };
        let manager = AudioManager::<MockBackend>::new(settings).expect("a mock never fails");
        let warnings = Warnings::default();
        let audio = KiraAudio::with_manager(manager, bank(), music).with_warnings(warnings.clone());
        (audio, warnings)
    }

    /// Runs `command`, then lets the backend take it in.
    fn run(audio: &mut Adapter, command: AudioCommand) {
        audio.run(command);
        let backend = audio.manager.backend_mut();
        backend.on_start_processing();
        backend.process();
    }

    fn play(id: i16, volume: f32) -> AudioCommand {
        play_at(id, volume, 1)
    }

    fn play_at(id: i16, volume: f32, priority: u16) -> AudioCommand {
        AudioCommand::Play {
            sound: SoundId(id),
            volume: Volume::new(volume),
            priority: Priority::new(priority),
        }
    }

    /// Processes `audio` for a few dozen milliseconds: long enough for a
    /// stopped sound's fade to end and kira to let it go, too short for a
    /// second-long sound to end.
    fn settle(audio: &mut Adapter) {
        for _ in 0..20 {
            let backend = audio.manager.backend_mut();
            backend.on_start_processing();
            backend.process();
        }
    }

    /// How many sounds kira holds.
    fn sounds_held(audio: &mut Adapter) -> usize {
        audio.manager.main_track().num_sounds()
    }

    fn stopped(state: Option<PlaybackState>) -> bool {
        matches!(
            state,
            Some(PlaybackState::Stopping | PlaybackState::Stopped)
        )
    }

    #[test]
    fn playing_adds_an_effect_at_the_commands_volume() {
        let (mut audio, warnings) = adapter(None);
        assert_eq!(audio.effect_states(), []);
        run(&mut audio, play(1, 0.5));
        assert_eq!(audio.effect_states(), [PlaybackState::Playing]);
        run(&mut audio, play(1, 1.0));
        assert_eq!(
            audio.effect_states(),
            [PlaybackState::Playing, PlaybackState::Playing]
        );
        assert_eq!(
            audio.effect_levels(),
            [Decibels::IDENTITY, decibels(Volume::new(0.5))],
            "the louder first, as the voice list keeps them"
        );
        assert_eq!(warnings.text(), "");
    }

    #[test]
    fn finished_effects_are_let_go() {
        let short = FakeBank {
            sounds: vec![(SoundId(1), mono(vec![0x80; 2]))],
            ..FakeBank::default()
        };
        let (audio, _) = adapter(None);
        let mut audio = KiraAudio::with_manager(audio.manager, short, None);
        run(&mut audio, play(1, 1.0));
        assert_eq!(audio.effect_states(), [PlaybackState::Stopped], "over");
        run(&mut audio, play(1, 1.0));
        assert_eq!(audio.effect_states().len(), 1, "the finished one let go");
    }

    #[test]
    fn a_played_sound_is_playing_until_it_is_over() {
        let (mut audio, _) = adapter(None);
        assert!(!audio.playing(SoundId(1)), "never played");
        run(&mut audio, play(1, 1.0));
        assert!(audio.playing(SoundId(1)));
        assert!(!audio.playing(SoundId(3)), "another snd");

        let short = FakeBank {
            sounds: vec![(SoundId(1), mono(vec![0x80; 2]))],
            ..FakeBank::default()
        };
        let mut audio = KiraAudio::with_manager(audio.manager, short, None);
        run(&mut audio, play(1, 1.0));
        assert!(!audio.playing(SoundId(1)), "over");
        // Started, before the backend has played any of it.
        audio.run(play(1, 1.0));
        assert!(audio.playing(SoundId(1)), "again");
    }

    #[test]
    fn a_sound_on_any_voice_counts_as_playing_however_it_was_started() {
        let mission = FakeBank {
            sounds: vec![
                (SoundId(1), mono(vec![0x80; 11_127])),
                (SoundId(5), mono(vec![0x80; 11_127])),
                (SoundId(6), mono(vec![0x80; 11_127])),
            ],
            ..FakeBank::default()
        };
        let (audio, _) = adapter(None);
        let mut audio = KiraAudio::with_manager(audio.manager, mission, None);
        run(&mut audio, exclusive(5));
        assert!(audio.playing(SoundId(5)), "the mission channel's");
        run(&mut audio, unless_playing(6));
        assert!(audio.playing(SoundId(6)), "one played unless playing");
        run(&mut audio, start_loop(1, 1.0));
        assert!(!audio.playing(SoundId(1)), "the engine loop is no voice");
        run(&mut audio, play(3, 1.0));
        assert!(!audio.playing(SoundId(3)), "a missing snd never plays");
    }

    #[test]
    fn a_sound_played_unless_playing_waits_on_the_same_snd_played_as_an_effect() {
        let (mut audio, _) = adapter(None);
        run(&mut audio, play(1, 1.0));
        run(&mut audio, unless_playing(1));
        assert_eq!(audio.single_state(SoundId(1)), None, "skipped");
        assert_eq!(sounds_held(&mut audio), 1);
    }

    /// An adapter of the original's 8 voices, each sound a second long.
    fn eight_voices() -> Adapter {
        let (audio, _) = adapter(None);
        audio.with_voices(Some(ORIGINAL_VOICES))
    }

    #[test]
    fn a_full_list_refuses_a_sound_that_beats_no_voice() {
        let mut audio = eight_voices();
        for _ in 0..8 {
            run(&mut audio, play_at(1, 0.5, 5));
        }
        run(&mut audio, play_at(1, 1.0, 4));
        assert_eq!(audio.effect_states(), [PlaybackState::Playing; 8]);
        assert_eq!(sounds_held(&mut audio), 8, "nothing started");
        run(&mut audio, play_at(1, 0.25, 9));
        assert_eq!(sounds_held(&mut audio), 8, "louder voices: quieter refused");
    }

    #[test]
    fn a_full_list_evicts_its_last_voice_and_stops_it() {
        let mut audio = eight_voices();
        for _ in 0..7 {
            run(&mut audio, play_at(1, 0.5, 5));
        }
        run(&mut audio, play_at(1, 0.25, 3));
        run(&mut audio, play_at(1, 0.5, 4));
        let levels = audio.effect_levels();
        assert_eq!(levels.len(), 8);
        assert_eq!(levels[7], decibels(Volume::new(0.5)), "the priority 3 gone");
        assert!(!levels.contains(&decibels(Volume::new(0.25))));
        assert_eq!(audio.effect_states(), [PlaybackState::Playing; 8]);
        assert_eq!(sounds_held(&mut audio), 9, "the evicted one fading");
        settle(&mut audio);
        assert_eq!(sounds_held(&mut audio), 8, "stopped and let go");
    }

    #[test]
    fn a_halted_voice_frees_its_place_at_once() {
        let five = FakeBank {
            sounds: vec![
                (SoundId(1), mono(vec![0x80; 11_127])),
                (SoundId(5), mono(vec![0x80; 11_127])),
            ],
            ..FakeBank::default()
        };
        let (audio, _) = adapter(None);
        let mut audio =
            KiraAudio::with_manager(audio.manager, five, None).with_voices(Some(ORIGINAL_VOICES));
        for _ in 0..8 {
            run(&mut audio, play_at(5, 1.0, 9));
        }
        run(&mut audio, halt(5));
        assert_eq!(audio.voice_sounds(), [], "out of the list before its fade");
        run(&mut audio, play_at(1, 0.1, 1));
        assert_eq!(audio.voice_sounds(), [SoundId(1)]);
    }

    #[test]
    fn without_a_limit_a_ninth_sound_plays_and_none_is_evicted() {
        let (audio, _) = adapter(None);
        let mut audio = audio.with_voices(None);
        for _ in 0..9 {
            run(&mut audio, play_at(1, 1.0, 50));
        }
        run(&mut audio, play_at(1, 0.0, 1));
        assert_eq!(audio.effect_states(), [PlaybackState::Playing; 10]);
        settle(&mut audio);
        assert_eq!(sounds_held(&mut audio), 10);
    }

    #[test]
    fn the_adapter_starts_with_the_originals_eight_voices() {
        let (mut audio, _) = adapter(None);
        for _ in 0..9 {
            run(&mut audio, play_at(1, 1.0, 5));
        }
        assert_eq!(audio.effect_states().len(), 8);
    }

    #[test]
    fn a_missing_sound_warns_once_and_plays_nothing() {
        let (mut audio, warnings) = adapter(None);
        run(&mut audio, play(3, 1.0));
        assert_eq!(audio.effect_states(), []);
        assert_eq!(warnings.text(), "nova: no snd 3 to play\n");
        run(&mut audio, play(3, 1.0));
        run(&mut audio, play(2, 1.0));
        run(&mut audio, play(2, 1.0));
        assert_eq!(audio.effect_states(), []);
        assert_eq!(warnings.text().lines().count(), 2, "{}", warnings.text());
    }

    fn start_loop(id: i16, volume: f32) -> AudioCommand {
        AudioCommand::StartLoop {
            sound: SoundId(id),
            volume: Volume::new(volume),
        }
    }

    #[test]
    fn the_engine_loops_until_stopped() {
        let (mut audio, _) = adapter(None);
        assert_eq!((audio.loop_state(), audio.loop_level()), (None, None));
        run(&mut audio, start_loop(1, 0.5));
        assert_eq!(audio.loop_state(), Some(PlaybackState::Playing));
        assert_eq!(audio.loop_level(), Some(decibels(Volume::new(0.5))));
        // A second's worth of a second-long sound: it loops round.
        for _ in 0..400 {
            run(&mut audio, AudioCommand::SetLoopVolume(Volume::new(0.25)));
        }
        assert_eq!(audio.loop_state(), Some(PlaybackState::Playing), "looped");
        assert_eq!(audio.loop_level(), Some(decibels(Volume::new(0.25))));
        run(&mut audio, AudioCommand::StopLoop);
        assert!(stopped(audio.loop_state()), "{:?}", audio.loop_state());
        assert_eq!(audio.effect_states(), [], "not an effect");
    }

    #[test]
    fn starting_the_engine_again_stops_the_one_before() {
        let (mut audio, _) = adapter(None);
        run(&mut audio, start_loop(1, 1.0));
        run(&mut audio, start_loop(1, 1.0));
        assert_eq!(audio.loop_state(), Some(PlaybackState::Playing));
        run(&mut audio, AudioCommand::StopLoop);
        assert!(stopped(audio.loop_state()));
        run(&mut audio, start_loop(3, 1.0));
        assert_eq!(audio.loop_state(), None, "nothing to loop");
    }

    #[test]
    fn stopping_the_effects_stops_every_one_and_the_engine() {
        let (mut audio, _) = adapter(None);
        run(&mut audio, play(1, 1.0));
        run(&mut audio, play(1, 1.0));
        run(&mut audio, start_loop(1, 1.0));
        run(&mut audio, AudioCommand::StopEffects);
        assert_eq!(audio.effect_states(), [], "off their voices");
        assert!(stopped(audio.loop_state()), "{:?}", audio.loop_state());
        settle(&mut audio);
        assert_eq!(sounds_held(&mut audio), 0, "every one stopped");
    }

    fn exclusive(id: i16) -> AudioCommand {
        AudioCommand::PlayExclusive {
            sound: SoundId(id),
            volume: Volume::FULL,
            priority: Priority::new(1),
        }
    }

    #[test]
    fn a_mission_sound_plays_alone_on_its_channel() {
        let (mut audio, warnings) = adapter(None);
        assert_eq!(audio.mission_state(), None);
        run(&mut audio, exclusive(1));
        assert_eq!(audio.mission_state(), Some(PlaybackState::Playing));
        assert_eq!(audio.effect_states(), [], "not an effect");
        run(&mut audio, exclusive(3));
        assert_eq!(warnings.text(), "", "dropped, not even read");
        assert_eq!(audio.mission_state(), Some(PlaybackState::Playing));
        run(&mut audio, play(1, 1.0));
        assert_eq!(
            audio.effect_states(),
            [PlaybackState::Playing],
            "others play"
        );
    }

    #[test]
    fn once_the_mission_sound_is_over_the_next_plays() {
        let short = FakeBank {
            sounds: vec![(SoundId(1), mono(vec![0x80; 2]))],
            ..FakeBank::default()
        };
        let (audio, _) = adapter(None);
        let warnings = Warnings::default();
        let mut audio =
            KiraAudio::with_manager(audio.manager, short, None).with_warnings(warnings.clone());
        run(&mut audio, exclusive(1));
        assert_eq!(audio.mission_state(), Some(PlaybackState::Stopped), "over");
        run(&mut audio, exclusive(3));
        assert_eq!(warnings.text(), "nova: no snd 3 to play\n", "tried");
    }

    #[test]
    fn stopping_the_effects_stops_the_mission_sound_and_frees_its_channel() {
        let (mut audio, warnings) = adapter(None);
        run(&mut audio, exclusive(1));
        run(&mut audio, AudioCommand::StopEffects);
        assert_eq!(audio.mission_state(), None, "off its voice");
        settle(&mut audio);
        assert_eq!(sounds_held(&mut audio), 0, "stopped");
        run(&mut audio, exclusive(3));
        assert_eq!(warnings.text(), "nova: no snd 3 to play\n");
    }

    fn unless_playing(id: i16) -> AudioCommand {
        AudioCommand::PlayUnlessPlaying {
            sound: SoundId(id),
            volume: Volume::new(0.5),
            priority: Priority::new(1),
        }
    }

    #[test]
    fn a_sound_still_playing_is_not_started_again() {
        let (mut audio, warnings) = adapter(None);
        assert_eq!(audio.single_state(SoundId(1)), None);
        run(&mut audio, unless_playing(1));
        assert_eq!(audio.single_state(SoundId(1)), Some(PlaybackState::Playing));
        assert_eq!(audio.effect_states(), [], "not an effect");
        assert_eq!(audio.manager.main_track().num_sounds(), 1);
        run(&mut audio, unless_playing(1));
        assert_eq!(audio.manager.main_track().num_sounds(), 1, "dropped");
        run(&mut audio, unless_playing(3));
        assert_eq!(
            warnings.text(),
            "nova: no snd 3 to play\n",
            "another is tried"
        );
        assert_eq!(audio.single_state(SoundId(1)), Some(PlaybackState::Playing));
        run(&mut audio, exclusive(1));
        run(&mut audio, play(1, 1.0));
        assert_eq!(
            audio.manager.main_track().num_sounds(),
            3,
            "the others play"
        );
    }

    #[test]
    fn once_it_is_over_it_plays_again() {
        let short = FakeBank {
            sounds: vec![(SoundId(1), mono(vec![0x80; 2]))],
            ..FakeBank::default()
        };
        let (audio, _) = adapter(None);
        let mut audio = KiraAudio::with_manager(audio.manager, short, None);
        run(&mut audio, unless_playing(1));
        assert_eq!(
            audio.single_state(SoundId(1)),
            Some(PlaybackState::Stopped),
            "over"
        );
        // Started, before the backend has played any of it.
        audio.run(unless_playing(1));
        assert_eq!(
            audio.single_state(SoundId(1)),
            Some(PlaybackState::Playing),
            "again"
        );
    }

    #[test]
    fn stopping_the_effects_stops_it() {
        let (mut audio, _) = adapter(None);
        run(&mut audio, unless_playing(1));
        run(&mut audio, AudioCommand::StopEffects);
        assert_eq!(audio.single_state(SoundId(1)), None, "off its voice");
        settle(&mut audio);
        assert_eq!(sounds_held(&mut audio), 0, "stopped");
    }

    fn halt(id: i16) -> AudioCommand {
        AudioCommand::Halt { sound: SoundId(id) }
    }

    #[test]
    fn halt_stops_every_playing_instance_of_that_sound_only() {
        let sounds = FakeBank {
            sounds: [1, 5, 6]
                .map(|id| (SoundId(id), mono(vec![0x80; 11_127])))
                .to_vec(),
            ..FakeBank::default()
        };
        let (audio, _) = adapter(None);
        let warnings = Warnings::default();
        let mut audio =
            KiraAudio::with_manager(audio.manager, sounds, None).with_warnings(warnings.clone());
        run(&mut audio, play(1, 1.0));
        run(&mut audio, play(1, 1.0));
        run(&mut audio, play(5, 1.0));
        run(&mut audio, unless_playing(6));
        run(&mut audio, exclusive(5));
        run(&mut audio, start_loop(1, 1.0));
        run(&mut audio, halt(7));
        let mut sorted = audio.voice_sounds();
        sorted.sort_by_key(|sound| sound.0);
        assert_eq!(
            sorted,
            [1, 1, 5, 5, 6].map(SoundId),
            "halting a sound not playing changes nothing"
        );
        assert_eq!(warnings.text(), "", "nor reads it");
        run(&mut audio, halt(1));
        let mut left = audio.voice_sounds();
        left.sort_by_key(|sound| sound.0);
        assert_eq!(left, [5, 5, 6].map(SoundId), "every 1 off its voice");
        assert_eq!(
            audio.loop_state(),
            Some(PlaybackState::Playing),
            "the engine is not an effect it halts"
        );
        settle(&mut audio);
        assert_eq!(
            sounds_held(&mut audio),
            4,
            "the 1s stopped: 5, 6, 5 and the loop"
        );
        run(&mut audio, halt(5));
        assert_eq!(audio.voice_sounds(), [SoundId(6)]);
        assert_eq!(audio.mission_state(), None, "the mission's 5 too");
        assert_eq!(audio.single_state(SoundId(6)), Some(PlaybackState::Playing));
        assert_eq!(audio.loop_state(), Some(PlaybackState::Playing));
        settle(&mut audio);
        assert_eq!(sounds_held(&mut audio), 2);
    }

    #[test]
    fn halt_leaves_the_music_alone() {
        let (mut audio, _) = adapter(Some(silent_mp3(40)));
        run(&mut audio, start_music(1.0));
        run(&mut audio, halt(1));
        assert_eq!(audio.music_state(), Some(PlaybackState::Playing));
    }

    fn start_music(volume: f32) -> AudioCommand {
        AudioCommand::StartMusic {
            volume: Volume::new(volume),
        }
    }

    #[test]
    fn the_music_plays_until_stopped() {
        let (mut audio, warnings) = adapter(Some(silent_mp3(40)));
        assert_eq!((audio.music_state(), audio.music_level()), (None, None));
        run(&mut audio, start_music(0.5));
        assert_eq!(audio.music_state(), Some(PlaybackState::Playing));
        assert_eq!(audio.music_level(), Some(decibels(Volume::new(0.5))));
        run(&mut audio, AudioCommand::SetMusicVolume(Volume::new(0.25)));
        assert_eq!(audio.music_level(), Some(decibels(Volume::new(0.25))));
        run(&mut audio, AudioCommand::StopMusic);
        assert!(stopped(audio.music_state()), "{:?}", audio.music_state());
        run(&mut audio, start_music(1.0));
        assert_eq!(audio.music_state(), Some(PlaybackState::Playing), "again");
        assert_eq!(warnings.text(), "");
    }

    /// Processes `audio` until the music has stopped, or gives up after
    /// a few seconds of wall time: the soundtrack decodes on its own
    /// thread, which the processing may have to wait for.
    fn process_until_the_music_stops(audio: &mut Adapter) {
        let give_up = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while audio.music_state() != Some(PlaybackState::Stopped)
            && std::time::Instant::now() < give_up
        {
            let backend = audio.manager.backend_mut();
            backend.on_start_processing();
            backend.process();
            std::thread::sleep(std::time::Duration::from_micros(200));
        }
    }

    #[test]
    fn the_music_plays_once_to_its_end_and_is_not_looped() {
        let (mut audio, _) = adapter(Some(silent_mp3(40)));
        run(&mut audio, start_music(1.0));
        assert!(!audio.music_ended(), "still playing");
        process_until_the_music_stops(&mut audio);
        assert_eq!(audio.music_state(), Some(PlaybackState::Stopped));
        assert!(audio.music_ended());
        run(&mut audio, start_music(1.0));
        assert!(!audio.music_ended(), "started again");
    }

    #[test]
    fn restarted_music_counts_as_ended_when_it_plays_out() {
        let (mut audio, _) = adapter(Some(silent_mp3(40)));
        run(&mut audio, start_music(1.0));
        run(&mut audio, AudioCommand::StopMusic);
        run(&mut audio, start_music(1.0));
        process_until_the_music_stops(&mut audio);
        assert_eq!(audio.music_state(), Some(PlaybackState::Stopped));
        assert!(audio.music_ended(), "the restart played to its end");
    }

    #[test]
    fn fading_the_music_out_stops_it_once_the_fade_is_over() {
        let (mut audio, _) = adapter(Some(silent_mp3(400)));
        run(&mut audio, start_music(1.0));
        run(
            &mut audio,
            AudioCommand::FadeOutMusic(std::time::Duration::from_millis(20)),
        );
        assert_eq!(audio.music_state(), Some(PlaybackState::Stopping));
        process_until_the_music_stops(&mut audio);
        assert_eq!(audio.music_state(), Some(PlaybackState::Stopped));
        assert!(!audio.music_ended(), "faded out, not played to its end");
    }

    #[test]
    fn stopped_music_has_not_ended() {
        let (mut audio, _) = adapter(Some(silent_mp3(40)));
        assert!(!audio.music_ended(), "never started");
        run(&mut audio, start_music(1.0));
        run(&mut audio, AudioCommand::StopMusic);
        process_until_the_music_stops(&mut audio);
        assert!(!audio.music_ended());
    }

    #[test]
    fn without_music_nothing_plays_and_nothing_warns() {
        let (mut audio, warnings) = adapter(None);
        run(&mut audio, start_music(1.0));
        run(&mut audio, AudioCommand::SetMusicVolume(Volume::FULL));
        run(&mut audio, AudioCommand::StopMusic);
        run(
            &mut audio,
            AudioCommand::FadeOutMusic(std::time::Duration::ZERO),
        );
        assert_eq!(audio.music_state(), None);
        assert!(!audio.music_ended());
        assert_eq!(warnings.text(), "");
    }

    #[test]
    fn music_that_does_not_decode_warns_once_and_stays_silent() {
        let (mut audio, warnings) = adapter(Some(b"not an mp3 at all".to_vec()));
        run(&mut audio, start_music(1.0));
        assert_eq!(audio.music_state(), None);
        let text = warnings.text();
        assert!(text.starts_with("nova: cannot play the music: "), "{text}");
        assert_eq!(text.lines().count(), 1, "{text}");
        run(&mut audio, start_music(1.0));
        assert_eq!(audio.music_state(), None);
        assert_eq!(warnings.text(), text, "once");
    }
}
