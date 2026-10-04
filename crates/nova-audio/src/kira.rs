//! The kira adapter: plays the core's commands on an audio device.
//!
//! [`KiraAudio`] is thin: each [`AudioCommand`] becomes one or two kira
//! calls, with the `snd ` resources read through the [`SoundBank`] port
//! and decoded once each by `nova_data`'s decoder. Its pure parts (PCM to
//! kira frames, volume to decibels, the sound cache) are tested on their
//! own, and the adapter as a whole on kira's device-free `MockBackend`.
//! Only [`KiraAudio::open`] opens a real device (through cpal).
//!
//! A sound that is missing or cannot be decoded is reported once, as a
//! warning, and is silent from then on; so is music that cannot be
//! decoded. Warnings go to stderr unless [`KiraAudio::with_warnings`] says
//! otherwise.

use std::collections::HashMap;
use std::io::{self, Cursor, Write};
use std::rc::Rc;
use std::sync::Arc;

use ::kira::backend::Backend;
use ::kira::sound::static_sound::{StaticSoundData, StaticSoundHandle, StaticSoundSettings};
use ::kira::sound::streaming::{StreamingSoundData, StreamingSoundHandle};
use ::kira::sound::{FromFileError, PlaybackState};
use ::kira::{AudioManager, AudioManagerSettings, Decibels, DefaultBackend, Frame, Tween};
use nova_data::sound::{Pcm, decode_snd};
use nova_data::{GameData, SoundId};
use nova_rsrc::ResType;

use crate::port::{Audio, AudioCommand, Volume};

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

/// Plays the core's commands through a kira [`AudioManager`].
pub struct KiraAudio<B: Backend = DefaultBackend> {
    manager: AudioManager<B>,
    bank: Box<dyn SoundBank>,
    cache: SoundCache,
    /// The soundtrack's bytes, until they turn out not to decode.
    music: Option<Arc<[u8]>>,
    warnings: Box<dyn Write>,
    /// The one-shot effects started, until they finish.
    effects: Vec<(StaticSoundHandle, Decibels)>,
    /// The engine loop last started, stopped or not.
    engine: Option<StaticSoundHandle>,
    /// The level last applied to the engine loop.
    engine_level: Option<Decibels>,
    /// The music last started, stopped or not.
    track: Option<StreamingSoundHandle<FromFileError>>,
    /// The level last applied to the music.
    music_level: Option<Decibels>,
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
            effects: Vec::new(),
            engine: None,
            engine_level: None,
            track: None,
            music_level: None,
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

    /// The state of each one-shot effect still kept, oldest first.
    #[must_use]
    pub fn effect_states(&self) -> Vec<PlaybackState> {
        self.effects
            .iter()
            .map(|(handle, _)| handle.state())
            .collect()
    }

    /// The level each one-shot effect still kept was played at.
    #[must_use]
    pub fn effect_levels(&self) -> Vec<Decibels> {
        self.effects.iter().map(|&(_, level)| level).collect()
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

    /// Plays `sound` once at `level`, letting go of the effects that have
    /// finished. More sounds at once than kira has room for are dropped.
    fn play(&mut self, sound: SoundId, level: Decibels) {
        self.effects
            .retain(|(handle, _)| handle.state() != PlaybackState::Stopped);
        if let Some(data) = self.sound(sound)
            && let Ok(handle) = self.manager.play(data.volume(level))
        {
            self.effects.push((handle, level));
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

    /// Stops the music, if it is playing.
    fn stop_music(&mut self) {
        if let Some(track) = &mut self.track {
            track.stop(Tween::default());
        }
    }

    /// Streams the soundtrack from the top, looping, at `level`. Music that
    /// does not decode is reported once and never tried again.
    fn start_music(&mut self, level: Decibels) {
        self.stop_music();
        self.track = None;
        let Some(bytes) = &self.music else {
            return;
        };
        match StreamingSoundData::from_cursor(Cursor::new(Arc::clone(bytes))) {
            Ok(data) => {
                if let Ok(track) = self.manager.play(data.loop_region(..).volume(level)) {
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
            AudioCommand::Play { sound, volume } => self.play(sound, decibels(volume)),
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
            AudioCommand::StopMusic => self.stop_music(),
            AudioCommand::StopEffects => {
                for (effect, _) in &mut self.effects {
                    effect.stop(Tween::default());
                }
                self.stop_loop();
            }
        }
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
        AudioCommand::Play {
            sound: SoundId(id),
            volume: Volume::new(volume),
        }
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
            [decibels(Volume::new(0.5)), Decibels::IDENTITY]
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
        let states = audio.effect_states();
        assert_eq!(states.len(), 2);
        assert!(states.iter().all(|&s| stopped(Some(s))), "{states:?}");
        assert!(stopped(audio.loop_state()), "{:?}", audio.loop_state());
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

    #[test]
    fn without_music_nothing_plays_and_nothing_warns() {
        let (mut audio, warnings) = adapter(None);
        run(&mut audio, start_music(1.0));
        run(&mut audio, AudioCommand::SetMusicVolume(Volume::FULL));
        run(&mut audio, AudioCommand::StopMusic);
        assert_eq!(audio.music_state(), None);
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
