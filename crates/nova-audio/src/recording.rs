//! Port mocks, for tests: a recording [`Audio`] that plays nothing and
//! keeps every command it is given, [`MemorySettings`], a
//! [`SettingsStore`] that keeps the saved text in memory, and
//! [`MemoryBank`], a [`SoundBank`] of clips in memory, with
//! [`check_bank_contract`], the check every sound bank passes.
//!
//! Available to this crate's tests and, through the `recording` feature,
//! to other crates' tests. Their state is shared, so a test keeps a handle
//! to it after moving the mock into the app.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::io;
use std::rc::Rc;

use nova_data::SoundId;

use crate::bank::{Clip, SoundBank, pcm};
use crate::port::{Audio, AudioCommand};
use crate::settings::SettingsStore;

/// The commands an [`Audio`] was given, in order.
pub type AudioLog = Rc<RefCell<Vec<AudioCommand>>>;

/// Records every command; plays nothing.
#[derive(Clone, Debug, Default)]
pub struct RecordingAudio {
    log: AudioLog,
    music_ended: Rc<Cell<bool>>,
    /// The sounds that count as playing.
    playing: Rc<RefCell<Vec<SoundId>>>,
}

impl RecordingAudio {
    /// A mock with an empty log.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// A handle to the log, which sees every command given after it too.
    #[must_use]
    pub fn log(&self) -> AudioLog {
        Rc::clone(&self.log)
    }

    /// Makes the music last started count as played to its end, for this
    /// mock and its clones, until music is started again.
    pub fn end_music(&self) {
        self.music_ended.set(true);
    }

    /// Makes `sound` count as playing (`true`) or not, for this mock and
    /// its clones.
    pub fn set_playing(&self, sound: SoundId, playing: bool) {
        let mut sounds = self.playing.borrow_mut();
        sounds.retain(|&other| other != sound);
        if playing {
            sounds.push(sound);
        }
    }
}

impl Audio for RecordingAudio {
    fn run(&mut self, command: AudioCommand) {
        if matches!(command, AudioCommand::StartMusic { .. }) {
            self.music_ended.set(false);
        }
        self.log.borrow_mut().push(command);
    }

    fn music_ended(&self) -> bool {
        self.music_ended.get()
    }

    fn playing(&self, sound: SoundId) -> bool {
        self.playing.borrow().contains(&sound)
    }
}

/// A [`SettingsStore`] in memory. Clones share the saved text and the
/// failure switches, so a test can seed the store before the app takes a
/// clone, read back what was saved, or "restart" by opening a new keeper
/// over another clone.
#[derive(Clone, Debug, Default)]
pub struct MemorySettings {
    text: Rc<RefCell<Option<String>>>,
    writes: Rc<Cell<usize>>,
    fail_reads: Rc<Cell<bool>>,
    fail_writes: Rc<Cell<bool>>,
}

impl MemorySettings {
    /// A store with nothing saved.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// A store holding `text`, as if it had been saved.
    #[must_use]
    pub fn holding(text: &str) -> Self {
        let store = Self::new();
        *store.text.borrow_mut() = Some(text.to_owned());
        store
    }

    /// The saved text, if any.
    #[must_use]
    pub fn text(&self) -> Option<String> {
        self.text.borrow().clone()
    }

    /// How many writes have succeeded.
    #[must_use]
    pub fn writes(&self) -> usize {
        self.writes.get()
    }

    /// Makes every read fail (`true`) or succeed again.
    pub fn fail_reads(&self, fail: bool) {
        self.fail_reads.set(fail);
    }

    /// Makes every write fail (`true`), keeping the text saved before, or
    /// succeed again.
    pub fn fail_writes(&self, fail: bool) {
        self.fail_writes.set(fail);
    }
}

impl SettingsStore for MemorySettings {
    fn read(&mut self) -> io::Result<Option<String>> {
        if self.fail_reads.get() {
            return Err(io::Error::other("the disk is unreadable"));
        }
        Ok(self.text())
    }

    fn write(&mut self, text: &str) -> io::Result<()> {
        if self.fail_writes.get() {
            return Err(io::Error::other("the disk is full"));
        }
        *self.text.borrow_mut() = Some(text.to_owned());
        self.writes.set(self.writes.get() + 1);
        Ok(())
    }

    /// Always `memory`.
    fn location(&self) -> String {
        "memory".to_owned()
    }
}

/// A [`SoundBank`] of clips in memory, by ID. An ID it doesn't hold is
/// missing, as in the stock game's bank, or silent, as in a new game's,
/// when built [`MemoryBank::silent_otherwise`]. Its `snd ` resources are
/// the [`Clip::Snd`] clips it holds.
#[derive(Clone, Debug, Default)]
pub struct MemoryBank {
    clips: BTreeMap<SoundId, Clip>,
    silent_otherwise: bool,
}

impl MemoryBank {
    /// A bank holding nothing, every ID missing.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The bank with `clip` at `id`, in place of any it held there.
    #[must_use]
    pub fn with(mut self, id: SoundId, clip: Clip) -> Self {
        self.clips.insert(id, clip);
        self
    }

    /// The bank with every ID it doesn't hold silent, not missing.
    #[must_use]
    pub fn silent_otherwise(self) -> Self {
        Self {
            silent_otherwise: true,
            ..self
        }
    }
}

impl SoundBank for MemoryBank {
    fn snd(&self, id: SoundId) -> Option<Vec<u8>> {
        match self.clips.get(&id) {
            Some(Clip::Snd(bytes)) => Some(bytes.clone()),
            _ => None,
        }
    }

    fn clip(&self, id: SoundId) -> Option<Clip> {
        (self.clips.get(&id).cloned()).or_else(|| self.silent_otherwise.then_some(Clip::Silent))
    }
}

/// Checks that `bank` keeps the [`SoundBank`] contract at `held`, an ID it
/// plays, and `unheld`, one it doesn't:
///
/// - `held`'s clip isn't silent, and decodes ([`pcm`]) to at least one
///   frame of one or two channels at a rate above 0;
/// - `unheld`'s answer is missing or silent, never samples;
/// - wherever `snd` answers bytes, `clip` answers them as a
///   [`Clip::Snd`], so a bank that serves a resource plays it;
/// - asking again gives the same answers.
///
/// # Errors
///
/// What broke the contract first.
pub fn check_bank_contract(
    bank: &dyn SoundBank,
    held: SoundId,
    unheld: SoundId,
) -> Result<(), String> {
    let clip = bank
        .clip(held)
        .ok_or_else(|| format!("snd {} is missing", held.0))?;
    let decoded = pcm(&clip).map_err(|error| format!("snd {} doesn't decode: {error}", held.0))?;
    let decoded = decoded.ok_or_else(|| format!("snd {} is silent", held.0))?;
    if decoded.frames() == 0 {
        return Err(format!("snd {} has no frames", held.0));
    }
    if !matches!(decoded.channels(), 1 | 2) {
        return Err(format!(
            "snd {} has {} channels",
            held.0,
            decoded.channels()
        ));
    }
    if decoded.sample_rate().fixed() == 0 {
        return Err(format!("snd {} has no sample rate", held.0));
    }
    let quiet = bank.clip(unheld);
    if let Some(other) = quiet.as_ref().filter(|clip| **clip != Clip::Silent) {
        return Err(format!("snd {} plays {other:?}", unheld.0));
    }
    for (id, first) in [(held, Some(clip)), (unheld, quiet)] {
        let snd = bank.snd(id);
        if let Some(bytes) = snd.clone()
            && first != Some(Clip::Snd(bytes))
        {
            return Err(format!("snd {} serves a resource its clip drops", id.0));
        }
        if bank.clip(id) != first || bank.snd(id) != snd {
            return Err(format!("snd {} answers differently when asked again", id.0));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use nova_data::sound::fixture::wav;

    use super::*;

    const HELD: SoundId = SoundId(200);
    const UNHELD: SoundId = SoundId(153);

    fn clip() -> Clip {
        Clip::Wav(wav(1, 8000, 16, &[1, -1, 2]))
    }

    #[test]
    fn the_memory_bank_holds_its_clips_and_misses_or_silences_the_rest() {
        let snd = Clip::Snd(b"bytes".to_vec());
        let bank = MemoryBank::new()
            .with(HELD, clip())
            .with(SoundId(600), snd.clone());
        assert_eq!(bank.clip(HELD), Some(clip()));
        assert_eq!(bank.snd(HELD), None, "a WAV is no resource");
        assert_eq!(bank.clip(SoundId(600)), Some(snd));
        assert_eq!(bank.snd(SoundId(600)), Some(b"bytes".to_vec()));
        assert_eq!(bank.clip(UNHELD), None);
        let quiet = bank.silent_otherwise();
        assert_eq!(quiet.clip(UNHELD), Some(Clip::Silent));
        assert_eq!(quiet.clip(HELD), Some(clip()));
        let replaced = MemoryBank::new()
            .with(HELD, clip())
            .with(HELD, Clip::Silent);
        assert_eq!(replaced.clip(HELD), Some(Clip::Silent));
    }

    #[test]
    fn the_memory_bank_passes_the_contract() {
        let bank = MemoryBank::new().with(HELD, clip());
        assert_eq!(check_bank_contract(&bank, HELD, UNHELD), Ok(()));
        let quiet = bank.silent_otherwise();
        assert_eq!(check_bank_contract(&quiet, HELD, UNHELD), Ok(()));
        let snd = nova_data::sound::fixture::SndBuilder::new(
            nova_data::sound::fixture::SndFormat::Two,
            nova_data::sound::fixture::Header::Standard {
                rate: 0x2B77_0000,
                loop_points: (0, 0),
                base_note: 60,
                samples: vec![0x80, 0xC0],
            },
        )
        .bytes();
        let stock = MemoryBank::new().with(HELD, Clip::Snd(snd));
        assert_eq!(check_bank_contract(&stock, HELD, UNHELD), Ok(()));
    }

    /// The contract's verdict on `bank`, which must be a failure starting
    /// with `start`.
    fn fails(bank: &dyn SoundBank, start: &str) {
        let error = check_bank_contract(bank, HELD, UNHELD).expect_err("it breaks the contract");
        assert!(error.starts_with(start), "{error}");
    }

    #[test]
    fn a_bank_missing_silent_or_undecodable_at_its_held_id_fails() {
        fails(&MemoryBank::new(), "snd 200 is missing");
        fails(
            &MemoryBank::new().with(HELD, Clip::Silent),
            "snd 200 is silent",
        );
        fails(
            &MemoryBank::new().with(HELD, Clip::Wav(b"RIFF".to_vec())),
            "snd 200 doesn't decode",
        );
        fails(
            &MemoryBank::new().with(HELD, Clip::Wav(wav(1, 8000, 16, &[]))),
            "snd 200 has no frames",
        );
    }

    #[test]
    fn a_bank_playing_samples_at_its_unheld_id_fails() {
        let bank = MemoryBank::new().with(HELD, clip()).with(UNHELD, clip());
        fails(&bank, "snd 153 plays");
    }

    /// A bank whose `clip` doesn't serve its `snd `.
    struct Drops;

    impl SoundBank for Drops {
        fn snd(&self, _: SoundId) -> Option<Vec<u8>> {
            Some(b"a resource".to_vec())
        }

        fn clip(&self, id: SoundId) -> Option<Clip> {
            (id == HELD).then(clip)
        }
    }

    #[test]
    fn a_bank_whose_clip_drops_its_snd_fails() {
        fails(&Drops, "snd 200 serves a resource its clip drops");
    }

    /// A wrapper that forwards only `snd `, so its `clip` is the default.
    struct Unforwarded(MemoryBank);

    impl SoundBank for Unforwarded {
        fn snd(&self, id: SoundId) -> Option<Vec<u8>> {
            self.0.snd(id)
        }
    }

    #[test]
    fn a_wrapper_that_does_not_forward_clip_fails() {
        fails(
            &Unforwarded(MemoryBank::new().with(HELD, clip())),
            "snd 200 is missing",
        );
    }

    /// A bank that is silent at `UNHELD` the first time and missing after.
    struct Fickle(Cell<bool>);

    impl SoundBank for Fickle {
        fn snd(&self, _: SoundId) -> Option<Vec<u8>> {
            None
        }

        fn clip(&self, id: SoundId) -> Option<Clip> {
            if id == HELD {
                return Some(clip());
            }
            let first = !self.0.replace(true);
            first.then_some(Clip::Silent)
        }
    }

    #[test]
    fn a_bank_that_answers_differently_when_asked_again_fails() {
        fails(
            &Fickle(Cell::new(false)),
            "snd 153 answers differently when asked again",
        );
    }

    #[test]
    fn it_records_every_command_in_order_for_every_handle() {
        let mut audio = RecordingAudio::new();
        let before = audio.log();
        audio.run(AudioCommand::StopEffects);
        let after = audio.log();
        audio.run(AudioCommand::StopMusic);
        let expected = [AudioCommand::StopEffects, AudioCommand::StopMusic];
        assert_eq!(*before.borrow(), expected);
        assert_eq!(*after.borrow(), expected);
    }

    #[test]
    fn the_music_ends_when_told_to_until_it_starts_again() {
        let mut audio = RecordingAudio::new();
        let other = audio.clone();
        assert!(!audio.music_ended());
        other.end_music();
        assert!(audio.music_ended());
        audio.run(AudioCommand::StartMusic {
            volume: crate::port::Volume::FULL,
        });
        assert!(!audio.music_ended());
    }

    #[test]
    fn a_sound_plays_while_told_it_does_for_every_clone() {
        let audio = RecordingAudio::new();
        let other = audio.clone();
        assert!(!audio.playing(SoundId(600)));
        other.set_playing(SoundId(600), true);
        assert!(audio.playing(SoundId(600)));
        assert!(!audio.playing(SoundId(601)));
        other.set_playing(SoundId(600), false);
        assert!(!audio.playing(SoundId(600)));
    }

    #[test]
    fn the_memory_store_shares_its_text_across_clones() {
        let mut store = MemorySettings::new();
        let other = store.clone();
        assert_eq!(store.read().expect("reads"), None);
        store.write("saved").expect("writes");
        assert_eq!(other.text().as_deref(), Some("saved"));
        assert_eq!(other.writes(), 1);
        let mut seeded = MemorySettings::holding("seed");
        assert_eq!(seeded.read().expect("reads").as_deref(), Some("seed"));
        assert_eq!(seeded.writes(), 0);
    }

    #[test]
    fn the_memory_store_fails_when_told_to() {
        let mut store = MemorySettings::holding("kept");
        store.fail_reads(true);
        assert!(store.read().is_err());
        store.fail_reads(false);
        assert!(store.read().is_ok());
        store.fail_writes(true);
        assert!(store.write("lost").is_err());
        assert_eq!((store.text().as_deref(), store.writes()), (Some("kept"), 0));
        store.fail_writes(false);
        store.write("new").expect("writes");
        assert_eq!(store.text().as_deref(), Some("new"));
    }
}
