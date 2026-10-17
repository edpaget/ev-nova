//! The voice list: which sounds play when more are asked for than there
//! are voices, as the `AmbrosiaTools` sound library decides it.
//!
//! The original opens its sound library with 8 voices (`_InitEVNova`'s
//! `ST_Open(8, 0)` @0x3a127-0x3a136; `ST_Open` @0xe3bc0 caps the number at
//! 16 or 32, which 8 never reaches) and keeps them in one list, shared by
//! every sound it plays: weapons, explosions, beeps, the warp-ups and the
//! spaceport's ambient sound. `ST_PlaySoundParam` (@0xe3dc2) places each
//! new sound:
//!
//! - It walks the voices from the first while each is occupied and none
//!   has been chosen (@0xe3e5a-0xe3ea6). A voice is the place when the
//!   new sound's priority is at least the voice's **and** its loudness
//!   (left plus right) is at least the voice's. Past the last voice, the
//!   place is the first free one.
//! - With every voice occupied and none the place, the sound is not
//!   played (@0xe3ea8-0xe3ebb).
//! - With every voice occupied, the last is evicted (@0xe3ec0-0xe3f37).
//!   The voices from the place on move down one (`MemoryMove`
//!   @0xe3f3b-0xe3f61), and the new sound takes the place
//!   (@0xe3f66-0xe4004).
//!
//! A voice leaves the list the moment it ends (`__ST_MixerDispatch`
//! @0xe486e calls `__ST_HaltASound` @0xe4549) or is halted (`ST_HaltSound`
//! @0xe39ab), and `ST_IsSoundPlaying` (@0xe40c0) asks the whole list.
//!
//! [`VoiceList`] makes these decisions on plain data, in two steps so an
//! adapter never starts a sound it then has to stop: [`VoiceList::place`]
//! says where a sound would go, and [`VoiceList::insert`] puts it there,
//! handing back the voice it evicts. The adapter keeps the list in step
//! with the device: it drops the voices that have ended, and stops the
//! ones it is handed back.
//!
//! Loudness is compared as [`Volume`] amplitudes. Every voice the engine
//! plays is mono, so that orders voices as the original's left plus right
//! does, but for a tie that rounding could break differently.

use std::num::NonZeroUsize;

use nova_data::SoundId;

use crate::port::{Priority, Volume};

/// The original's number of voices: `ST_Open(8, 0)` (@0x3a136).
pub const ORIGINAL_VOICES: NonZeroUsize = NonZeroUsize::new(8).expect("8 is not 0");

/// A sound playing on one voice.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Voice<T> {
    /// The `snd `.
    pub sound: SoundId,
    /// Its priority.
    pub priority: Priority,
    /// How loud it was played.
    pub volume: Volume,
    /// The adapter's own hold on it.
    pub handle: T,
}

/// Where a new sound goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Placement {
    /// Nowhere: every voice is taken by a sound it doesn't match or beat.
    Refused,
    /// At this index in the list, the voices from it on moving down one.
    At(usize),
}

/// The voices playing, first the one that matters most: by priority, then
/// loudness, then age, newest first among equals.
#[derive(Clone, Debug, PartialEq)]
pub struct VoiceList<T> {
    /// How many voices there are; `None` is as many as are asked for.
    limit: Option<NonZeroUsize>,
    voices: Vec<Voice<T>>,
}

impl<T> VoiceList<T> {
    /// An empty list of `limit` voices, or of as many as are asked for.
    #[must_use]
    pub fn new(limit: Option<NonZeroUsize>) -> Self {
        Self {
            limit,
            voices: Vec::new(),
        }
    }

    /// How many voices there are, if they are limited.
    #[must_use]
    pub fn limit(&self) -> Option<NonZeroUsize> {
        self.limit
    }

    /// Whether every voice is taken.
    fn full(&self) -> bool {
        self.limit
            .is_some_and(|limit| self.voices.len() >= limit.get())
    }

    /// Where a sound of `priority` played at `volume` goes: before the
    /// first voice whose priority and loudness it is each at least, else
    /// after the last voice while one is free, else nowhere.
    #[must_use]
    pub fn place(&self, priority: Priority, volume: Volume) -> Placement {
        let first = self
            .voices
            .iter()
            .position(|voice| priority >= voice.priority && volume >= voice.volume);
        match first {
            Some(index) => Placement::At(index),
            None if self.full() => Placement::Refused,
            None => Placement::At(self.voices.len()),
        }
    }

    /// Puts `voice` at `index` (past the end goes last), the voices from it
    /// on moving down one. With every voice taken, the last is evicted
    /// first and handed back.
    pub fn insert(&mut self, index: usize, voice: Voice<T>) -> Option<Voice<T>> {
        let evicted = if self.full() { self.voices.pop() } else { None };
        let index = index.min(self.voices.len());
        self.voices.insert(index, voice);
        evicted
    }

    /// Keeps only the voices `keep` says to, in order: the adapter drops
    /// the ones that have ended.
    pub fn retain(&mut self, keep: impl FnMut(&Voice<T>) -> bool) {
        self.voices.retain(keep);
    }

    /// Takes every voice playing `sound` out of the list, handing them
    /// back: a halt.
    pub fn remove_sound(&mut self, sound: SoundId) -> Vec<Voice<T>> {
        let (removed, kept) = std::mem::take(&mut self.voices)
            .into_iter()
            .partition(|voice| voice.sound == sound);
        self.voices = kept;
        removed
    }

    /// Takes every voice out of the list, handing them back.
    pub fn clear(&mut self) -> Vec<Voice<T>> {
        std::mem::take(&mut self.voices)
    }

    /// How many voices play `sound`: `ST_IsSoundPlaying`.
    #[must_use]
    pub fn count(&self, sound: SoundId) -> usize {
        self.voices
            .iter()
            .filter(|voice| voice.sound == sound)
            .count()
    }

    /// Whether any voice is one `test` picks.
    pub fn any(&self, test: impl FnMut(&Voice<T>) -> bool) -> bool {
        self.voices.iter().any(test)
    }

    /// The voices, first the one that matters most.
    pub fn iter(&self) -> impl Iterator<Item = &Voice<T>> {
        self.voices.iter()
    }

    /// How many voices are taken.
    #[must_use]
    pub fn len(&self) -> usize {
        self.voices.len()
    }

    /// Whether no voice is taken.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.voices.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn voice(sound: i16, priority: u16, volume: f32, handle: u32) -> Voice<u32> {
        Voice {
            sound: SoundId(sound),
            priority: Priority::new(priority),
            volume: Volume::new(volume),
            handle,
        }
    }

    /// A list of `limit` voices holding `voices`, in order.
    fn list(limit: Option<usize>, voices: &[Voice<u32>]) -> VoiceList<u32> {
        let mut list = VoiceList::new(limit.and_then(NonZeroUsize::new));
        for &voice in voices {
            let at = list.len();
            assert_eq!(list.insert(at, voice), None, "room for {voice:?}");
        }
        list
    }

    fn handles(list: &VoiceList<u32>) -> Vec<u32> {
        list.iter().map(|voice| voice.handle).collect()
    }

    fn at(priority: u16, volume: f32) -> (Priority, Volume) {
        (Priority::new(priority), Volume::new(volume))
    }

    #[test]
    fn the_original_has_eight_voices() {
        assert_eq!(ORIGINAL_VOICES.get(), 8);
    }

    #[test]
    fn an_empty_list_places_a_sound_first() {
        let list = list(Some(8), &[]);
        let (priority, volume) = at(1, 0.0);
        assert_eq!(list.place(priority, volume), Placement::At(0));
        assert!(list.is_empty());
        assert_eq!(list.limit(), NonZeroUsize::new(8));
    }

    #[test]
    fn a_sound_goes_before_the_first_voice_it_matches_or_beats_on_both() {
        let list = list(
            Some(8),
            &[
                voice(1, 9, 1.0, 1),
                voice(2, 5, 0.5, 2),
                voice(3, 2, 0.25, 3),
            ],
        );
        let (priority, volume) = at(5, 0.75);
        assert_eq!(list.place(priority, volume), Placement::At(1));
        let (priority, volume) = at(3, 0.25);
        assert_eq!(list.place(priority, volume), Placement::At(2));
    }

    #[test]
    fn a_sound_that_beats_none_goes_after_the_last_while_a_voice_is_free() {
        let list = list(Some(8), &[voice(1, 9, 1.0, 1), voice(2, 5, 0.5, 2)]);
        let (priority, volume) = at(1, 0.1);
        assert_eq!(list.place(priority, volume), Placement::At(2));
    }

    #[test]
    fn a_higher_priority_but_quieter_sound_does_not_go_before_a_voice() {
        let list = list(Some(8), &[voice(1, 4, 0.5, 1)]);
        let (priority, volume) = at(9, 0.25);
        assert_eq!(list.place(priority, volume), Placement::At(1));
    }

    #[test]
    fn a_louder_but_lower_priority_sound_does_not_go_before_a_voice() {
        let list = list(Some(8), &[voice(1, 4, 0.5, 1)]);
        let (priority, volume) = at(3, 1.0);
        assert_eq!(list.place(priority, volume), Placement::At(1));
    }

    #[test]
    fn a_sound_of_equal_priority_and_loudness_goes_before_the_voice() {
        let list = list(Some(8), &[voice(1, 4, 0.5, 1)]);
        let (priority, volume) = at(4, 0.5);
        assert_eq!(list.place(priority, volume), Placement::At(0), "at least");
    }

    #[test]
    fn inserting_moves_the_voices_from_the_place_down_one() {
        let mut list = list(Some(8), &[voice(1, 9, 1.0, 1), voice(2, 2, 0.5, 2)]);
        assert_eq!(list.insert(1, voice(3, 5, 0.5, 3)), None);
        assert_eq!(handles(&list), [1, 3, 2]);
        assert_eq!(list.insert(9, voice(4, 1, 0.1, 4)), None, "past the end");
        assert_eq!(handles(&list), [1, 3, 2, 4]);
    }

    /// Eight voices of priority 5, the last at `last`'s priority and
    /// loudness.
    fn full(last: (u16, f32)) -> VoiceList<u32> {
        let mut voices: Vec<_> = (1..8).map(|n| voice(1, 5, 0.5, n)).collect();
        voices.push(voice(2, last.0, last.1, 8));
        list(Some(8), &voices)
    }

    #[test]
    fn a_full_list_of_higher_priority_voices_refuses_a_priority_4_sound() {
        let list = full((5, 0.5));
        let (priority, volume) = at(4, 1.0);
        assert_eq!(list.place(priority, volume), Placement::Refused);
    }

    #[test]
    fn a_priority_4_sound_does_not_displace_an_equal_priority_louder_voice() {
        let list = full((4, 0.75));
        let (priority, volume) = at(4, 0.5);
        assert_eq!(list.place(priority, volume), Placement::Refused);
    }

    #[test]
    fn a_priority_4_sound_displaces_a_lower_quieter_last_voice() {
        let mut list = full((3, 0.25));
        let (priority, volume) = at(4, 0.5);
        assert_eq!(list.place(priority, volume), Placement::At(7));
        let evicted = list.insert(7, voice(3, 4, 0.5, 9));
        assert_eq!(evicted, Some(voice(2, 3, 0.25, 8)), "the last evicted");
        assert_eq!(handles(&list), [1, 2, 3, 4, 5, 6, 7, 9]);
    }

    #[test]
    fn a_full_list_evicts_its_last_voice_whatever_the_place() {
        let mut list = full((5, 0.5));
        let (priority, volume) = at(9, 1.0);
        assert_eq!(list.place(priority, volume), Placement::At(0));
        let evicted = list.insert(0, voice(3, 9, 1.0, 9));
        assert_eq!(evicted.map(|voice| voice.handle), Some(8));
        assert_eq!(handles(&list), [9, 1, 2, 3, 4, 5, 6, 7]);
        assert_eq!(list.len(), 8);
    }

    #[test]
    fn an_unlimited_list_never_refuses_or_evicts() {
        let voices: Vec<_> = (1..=20).map(|n| voice(1, 50, 1.0, n)).collect();
        let mut list = list(None, &voices);
        let (priority, volume) = at(1, 0.0);
        assert_eq!(list.place(priority, volume), Placement::At(20));
        assert_eq!(list.insert(20, voice(2, 1, 0.0, 21)), None);
        assert_eq!(list.len(), 21);
        assert_eq!(list.limit(), None);
    }

    #[test]
    fn count_counts_every_voice_of_a_sound() {
        let list = list(
            Some(8),
            &[
                voice(1, 5, 0.5, 1),
                voice(2, 5, 0.5, 2),
                voice(1, 1, 0.1, 3),
            ],
        );
        assert_eq!(list.count(SoundId(1)), 2);
        assert_eq!(list.count(SoundId(2)), 1);
        assert_eq!(list.count(SoundId(3)), 0);
        assert!(list.any(|voice| voice.handle == 3));
        assert!(!list.any(|voice| voice.handle == 4));
    }

    #[test]
    fn a_halt_takes_out_every_voice_of_its_sound_and_frees_them() {
        let mut list = full((5, 0.5));
        let removed = list.remove_sound(SoundId(2));
        assert_eq!(removed, [voice(2, 5, 0.5, 8)]);
        assert_eq!(list.len(), 7);
        let (priority, volume) = at(1, 0.0);
        assert_eq!(
            list.place(priority, volume),
            Placement::At(7),
            "a voice free"
        );
        assert_eq!(list.remove_sound(SoundId(9)), []);
    }

    #[test]
    fn retain_and_clear_take_voices_out_in_order() {
        let mut list = list(
            Some(8),
            &[
                voice(1, 5, 0.5, 1),
                voice(2, 5, 0.5, 2),
                voice(3, 5, 0.5, 3),
            ],
        );
        list.retain(|voice| voice.handle != 2);
        assert_eq!(handles(&list), [1, 3]);
        let cleared = list.clear();
        assert_eq!(
            cleared.iter().map(|voice| voice.handle).collect::<Vec<_>>(),
            [1, 3]
        );
        assert!(list.is_empty());
    }
}
