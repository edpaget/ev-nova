//! The escorts' voices: the clips each voice set holds for each message,
//! the government's voice that picks the set, and the one voice channel
//! the session plays them through, as the original's `_SpeakVoiceMessage`
//! (@0x44558), `_HandleVoiceSound` (@0x31e20), `_KillVoiceSound`
//! (@0x31f95) and `_LoadSounds`' voice count (@0x1c03f) have them.
//!
//! - **The clips.** Voice set `s` (0-7) holds, for message `m`
//!   ([`VoiceMessage`]: 0 acknowledge, 1 target, 2 victory), the `snd `s
//!   found in a row from 1000 + 100s + 10m, stopping at the first gap and
//!   at most [`MOST_CLIPS`] (`_LoadSounds` @0x1c03f-0x1c0b3,
//!   [`VoiceTable::read`]). So base + 9 never plays, though the Bible
//!   lists it. A clip `_ResourceExists` finds counts whether or not it can
//!   be decoded; the engine gives one it cannot decode no length.
//! - **The set.** A `gövt`'s `VoiceType` ([`GovtVoice::decode`],
//!   `_LoadObjectData` @0x7baa0-0x7bb00) gives its set and the variant it
//!   forces, if any: 0-7 a set with a free variant, 1000-1007 one forced
//!   odd, 2000-2007 one forced even, anything else silent. A speaker's
//!   voice is its class's attributes government (class +0x9f6): a class
//!   with none speaks set 0, as the Bible says ([`Voices::set_of`]).
//! - **The variant.** Each ship's +0xc922, rolled as `_LaunchFighter`,
//!   `_DoShipCapture`, `_AIShipCapturedByPlayer` and `_EscortAI` roll it
//!   ([`roll_variant`]): Rand(2), or the variant the government forces.
//!   Even voice counts split by it, as only sets 4-6 do in stock: variant
//!   0 picks the even clips and 1 the odd ones ([`VoiceTable::pick`]).
//! - **The channel** ([`VoiceChannel`]): `_SpeakVoiceMessage` stores one
//!   message waiting, overwriting any ([`VoiceChannel::speak`]);
//!   `_HandleVoiceSound` plays it once a pass, cutting off a clip still
//!   playing a pass before ([`VoiceChannel::handle`]); and
//!   `_KillVoiceSound` halts it as the ship lands ([`VoiceChannel::kill`]).
//!
//! **What the engine can't ask.** The session can't call
//! `ST_IsSoundPlaying`. It models a clip's play by its length, counted
//! down a frame's worth on each advance pass ([`VoiceChannel::step`]), as
//! [`WarpOut`](crate::warp::WarpOut) does for `snd ` 130. As there, the
//! countdown stands still under a dialog, where the sound plays on, and a
//! clip the 8-voice library would refuse still counts as playing.
//!
//! A `gövt` that has no record here (one a plug-in names but does not
//! hold) speaks no voice. The original reads whatever its runtime slot
//! holds; stock names no such government.

use std::collections::BTreeMap;

use crate::catalog::{GovtId, SoundId};
use crate::chance::Chance;
use crate::sound::SimSound;
use crate::warp::{MAC_TICKS_PER_TICK, SoundLength, length_ticks};

/// The voice sets: 0-7 (`_LoadSounds`' outer loop, @0x1c03f).
pub const SETS: u8 = 8;

/// The most clips a voice set holds for one message: `_LoadSounds` stops
/// counting at 9 (@0x1c08c).
pub const MOST_CLIPS: usize = 9;

/// The first voice clip, `snd ` 1000: set 0's first acknowledgement.
const FIRST_CLIP: i16 = 1000;

/// What each voice set adds to the clip's `snd ` ID.
const PER_SET: i16 = 100;

/// What each message adds to the clip's `snd ` ID.
const PER_MESSAGE: i16 = 10;

/// The variant of a ship's voice, +0xc922: 0 for the even clips, 1 for
/// the odd ones. A ship whose variant is unset (above 1) holds none.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VoiceVariant {
    /// 0: the even clips (stock: the male voices of sets 4-6).
    Even,
    /// 1: the odd clips (stock: the female voices).
    Odd,
}

impl VoiceVariant {
    /// Its word: 0 even, 1 odd.
    #[must_use]
    pub const fn index(self) -> u32 {
        match self {
            Self::Even => 0,
            Self::Odd => 1,
        }
    }
}

/// What an escort says: the message word `_SpeakVoiceMessage` stores
/// (*0x3b5458).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum VoiceMessage {
    /// 0: a command acknowledged (`_IssueNewEscortCommand` @0x6667a).
    Acknowledge,
    /// 1: a target taken: a command to attack a ship the player doesn't
    /// lead (@0x6660a-0x6663c), or `_EscortAI`'s 1-in-3 call on a new
    /// threat (@0x83e34).
    Target,
    /// 2: a victory, as the ship an escort attacks starts to die
    /// (`_HandleShipHit` @0x36d35).
    Victory,
}

impl VoiceMessage {
    /// Every message, in word order.
    pub const ALL: [Self; 3] = [Self::Acknowledge, Self::Target, Self::Victory];

    /// Its word: 0, 1 or 2.
    #[must_use]
    pub const fn index(self) -> usize {
        match self {
            Self::Acknowledge => 0,
            Self::Target => 1,
            Self::Victory => 2,
        }
    }
}

/// A government's voice, from its `VoiceType`: the set its ships speak
/// (runtime +0x3e) and the variant it forces on them (+0x40), if any.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GovtVoice {
    /// Its voice set, 0-7; none for a silent government (-1).
    pub set: Option<u8>,
    /// The variant it forces; none for a free one (-1).
    pub forced: Option<VoiceVariant>,
}

impl GovtVoice {
    /// The voice of a `gövt` of `VoiceType` `voice_type`
    /// (`_LoadObjectData` @0x7baa0-0x7bb00): 0-7 a set with a free
    /// variant, 1000-1007 the set forced odd, 2000-2007 the set forced
    /// even, anything else silent.
    #[must_use]
    pub fn decode(voice_type: i16) -> Self {
        let set = |base: i16| u8::try_from(voice_type - base).ok();
        match voice_type {
            0..=7 => Self {
                set: set(0),
                forced: None,
            },
            1000..=1007 => Self {
                set: set(1000),
                forced: Some(VoiceVariant::Odd),
            },
            2000..=2007 => Self {
                set: set(2000),
                forced: Some(VoiceVariant::Even),
            },
            _ => Self::default(),
        }
    }
}

/// A voice clip: its `snd ` and how long it plays, in Mac ticks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Clip {
    /// The `snd `.
    pub sound: SoundId,
    /// How long it plays, in Mac ticks; 0 for one that can't be decoded.
    pub length: u32,
}

/// The clips of every voice set and message: `_LoadSounds`' count
/// (*0x3b52fc, a word for each set × 3 + message), with each clip's
/// length.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct VoiceTable {
    /// Each clip's length in Mac ticks, by set and message, in clip
    /// order.
    clips: [[Vec<u32>; 3]; SETS as usize],
}

impl VoiceTable {
    /// The clips among `sounds` (each `snd ` that exists, with its length
    /// when it can be decoded), counted as `_LoadSounds` counts them
    /// (@0x1c03f-0x1c0b3): for each set and message, the `snd `s in a row
    /// from 1000 + 100 × set + 10 × message, up to the first gap and at
    /// most [`MOST_CLIPS`]. A clip that can't be decoded counts, as
    /// `_ResourceExists` finds it, with length 0.
    #[must_use]
    pub fn read(sounds: &[(SoundId, Option<SoundLength>)]) -> Self {
        let lengths: BTreeMap<SoundId, u32> = sounds
            .iter()
            .map(|&(id, length)| (id, length.and_then(length_ticks).unwrap_or(0)))
            .collect();
        let mut table = Self::default();
        for (set, messages) in (0..).zip(&mut table.clips) {
            for (message, clips) in VoiceMessage::ALL.into_iter().zip(messages) {
                let base = base_of(set, message);
                *clips = (0..)
                    .map(|clip| lengths.get(&SoundId(base + clip)).copied())
                    .take(MOST_CLIPS)
                    .map_while(|length| length)
                    .collect();
            }
        }
        table
    }

    /// How many clips set `set` holds for `message`: 0 for a set past 7.
    #[must_use]
    pub fn count(&self, set: u8, message: VoiceMessage) -> usize {
        self.clips
            .get(usize::from(set))
            .map_or(0, |messages| messages[message.index()].len())
    }

    /// The clip set `set` plays for `message` in `variant`, drawing on
    /// `chance` (`_HandleVoiceSound` @0x31f00-0x31f45); none when it holds
    /// none. Of n clips, an unset variant or an odd n draws Rand(n), n = 2
    /// takes the variant itself, and any other even n takes the variant
    /// plus Rand(n/2) × 2, so the even variant picks the even clips and the
    /// odd one the odd. The `snd ` is 1000 + 100 × set + 10 × message + clip.
    pub fn pick(
        &self,
        set: u8,
        message: VoiceMessage,
        variant: Option<VoiceVariant>,
        chance: &mut (impl Chance + ?Sized),
    ) -> Option<Clip> {
        let clips = self.clips.get(usize::from(set))?.get(message.index())?;
        let n = u32::try_from(clips.len()).ok().filter(|&n| n > 0)?;
        let clip = match variant {
            Some(variant) if n == 2 => variant.index(),
            Some(variant) if n % 2 == 0 => chance.below(n / 2) * 2 + variant.index(),
            _ => chance.below(n),
        };
        let length = *clips.get(usize::try_from(clip).ok()?)?;
        let clip = i16::try_from(clip).ok()?;
        Some(Clip {
            sound: SoundId(base_of(set, message) + clip),
            length,
        })
    }
}

/// The first `snd ` of set `set`'s clips for `message`.
fn base_of(set: u8, message: VoiceMessage) -> i16 {
    let message = i16::try_from(message.index()).unwrap_or(0);
    FIRST_CLIP + PER_SET * i16::from(set) + PER_MESSAGE * message
}

/// The voices the session reads as it starts: the clips, and each
/// government's voice.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Voices {
    /// The clips of every set and message.
    pub table: VoiceTable,
    /// Each government's voice, by ID.
    pub govts: BTreeMap<GovtId, GovtVoice>,
}

impl Voices {
    /// The voices of `sounds` (see [`VoiceTable::read`]) and of the
    /// governments of `govt_voice_types`, each a `gövt` and its
    /// `VoiceType` ([`GovtVoice::decode`]).
    #[must_use]
    pub fn read(
        sounds: &[(SoundId, Option<SoundLength>)],
        govt_voice_types: &[(GovtId, i16)],
    ) -> Self {
        Self {
            table: VoiceTable::read(sounds),
            govts: govt_voice_types
                .iter()
                .map(|&(id, voice_type)| (id, GovtVoice::decode(voice_type)))
                .collect(),
        }
    }

    /// The set a ship of a class whose attributes government is
    /// `attributes_govt` speaks (`_HandleVoiceSound` @0x31eb2-0x31ee2):
    /// set 0 with none, as a voice above 255 reads; its government's set
    /// otherwise, none for a silent one or one with no record.
    #[must_use]
    pub fn set_of(&self, attributes_govt: Option<GovtId>) -> Option<u8> {
        match attributes_govt {
            None => Some(0),
            Some(govt) => self.govts.get(&govt).and_then(|voice| voice.set),
        }
    }

    /// The variant a ship of a class whose attributes government is
    /// `attributes_govt` is forced to: none with no government (+0x9f6 =
    /// -1), else its government's +0x40.
    #[must_use]
    pub fn forced_of(&self, attributes_govt: Option<GovtId>) -> Option<VoiceVariant> {
        attributes_govt
            .and_then(|govt| self.govts.get(&govt))
            .and_then(|voice| voice.forced)
    }
}

/// A ship's voice variant as `_LaunchFighter` (@0x3d9cc-0x3da3a),
/// `_DoShipCapture` (@0x41288-0x412db), `_AIShipCapturedByPlayer`
/// (@0x8974a-0x89799) and `_EscortAI` (@0x83980-0x839f6) roll it: the
/// variant its government forces, if any, else Rand(2) on `chance`.
pub fn roll_variant(
    forced: Option<VoiceVariant>,
    chance: &mut (impl Chance + ?Sized),
) -> VoiceVariant {
    forced.unwrap_or_else(|| {
        if chance.below(2) == 0 {
            VoiceVariant::Even
        } else {
            VoiceVariant::Odd
        }
    })
}

/// A message waiting to be said: `_SpeakVoiceMessage`'s three globals,
/// the message, the voice (as the set it reads) and the variant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Speech {
    /// What is said.
    pub message: VoiceMessage,
    /// The voice set it is said in; none for a silent government.
    pub set: Option<u8>,
    /// The speaker's variant; none while unset.
    pub variant: Option<VoiceVariant>,
}

/// The one voice channel (see the module docs): the message waiting
/// (*0x3b5458, *0x3b5470, *0x3b52e0) and the clip loaded (*0x3b5424),
/// with the Mac ticks it still plays.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VoiceChannel {
    /// The message waiting, if any.
    pending: Option<Speech>,
    /// The clip loaded and the Mac ticks it still plays, if any.
    loaded: Option<(SoundId, u32)>,
}

impl VoiceChannel {
    /// Queues `speech`, overwriting any message waiting
    /// (`_SpeakVoiceMessage` @0x44558-0x4457d, which tests nothing).
    pub fn speak(&mut self, speech: Speech) {
        self.pending = Some(speech);
    }

    /// The message waiting, if any.
    #[must_use]
    pub fn pending(&self) -> Option<Speech> {
        self.pending
    }

    /// Whether the channel is idle, `_EscortAI`'s gate on its target call
    /// (@0x83df9-0x83e34): no message waiting and no clip loaded, over or
    /// not, as the handle stays set until the next
    /// [`handle`](Self::handle) disposes it.
    #[must_use]
    pub fn idle(&self) -> bool {
        self.pending.is_none() && self.loaded.is_none()
    }

    /// Plays one engine tick of the clip loaded: [`MAC_TICKS_PER_TICK`]
    /// off.
    pub fn step(&mut self) {
        if let Some((_, left)) = &mut self.loaded {
            *left = left.saturating_sub(MAC_TICKS_PER_TICK);
        }
    }

    /// `_HandleVoiceSound` (@0x31e20-0x31f94), once an advance pass,
    /// picking the clip from `table` on `chance`:
    ///
    /// 1. A clip loaded that is over is disposed (@0x31e29-0x31e52).
    /// 2. With no message waiting, nothing more (@0x31e58-0x31e65).
    /// 3. A clip still playing is halted ([`SimSound::HaltVoice`]) and
    ///    marked over, the message kept, so the next pass disposes it and
    ///    plays the message (@0x31e6b-0x31e8e).
    /// 4. Otherwise the message is taken: a silent set
    ///    (@0x31eb2-0x31ee2) or a count of 0 (@0x31ee5-0x31efa) drops it;
    ///    else its clip ([`VoiceTable::pick`]) is loaded and played at
    ///    priority 15 ([`SimSound::Voice`], @0x31f51-0x31f7a).
    pub fn handle(
        &mut self,
        table: &VoiceTable,
        chance: &mut (impl Chance + ?Sized),
    ) -> Option<SimSound> {
        if matches!(self.loaded, Some((_, 0))) {
            self.loaded = None;
        }
        let speech = self.pending?;
        if let Some((sound, left)) = &mut self.loaded {
            *left = 0;
            return Some(SimSound::HaltVoice { sound: *sound });
        }
        self.pending = None;
        let clip = table.pick(speech.set?, speech.message, speech.variant, chance)?;
        self.loaded = Some((clip.sound, clip.length));
        Some(SimSound::Voice { sound: clip.sound })
    }

    /// `_KillVoiceSound` (@0x31f95-0x31ff2): the clip loaded is halted
    /// ([`SimSound::HaltVoice`]) and disposed, and the message waiting
    /// dropped.
    pub fn kill(&mut self) -> Option<SimSound> {
        self.pending = None;
        self.loaded
            .take()
            .map(|(sound, _)| SimSound::HaltVoice { sound })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nova_sim_base::fixture::Draws;

    /// A clip a second long: 60 Mac ticks.
    const SECOND: SoundLength = SoundLength {
        frames: 22_050,
        rate: 22_050,
    };

    /// `count` decodable clips a second long from `base`.
    fn run(base: i16, count: i16) -> Vec<(SoundId, Option<SoundLength>)> {
        (0..count)
            .map(|clip| (SoundId(base + clip), Some(SECOND)))
            .collect()
    }

    fn speech(message: VoiceMessage, set: Option<u8>, variant: Option<VoiceVariant>) -> Speech {
        Speech {
            message,
            set,
            variant,
        }
    }

    #[test]
    fn a_govts_voice_type_gives_its_set_and_forced_variant() {
        let free = |set| GovtVoice {
            set: Some(set),
            forced: None,
        };
        assert_eq!(GovtVoice::decode(0), free(0));
        assert_eq!(GovtVoice::decode(7), free(7));
        assert_eq!(
            GovtVoice::decode(1003),
            GovtVoice {
                set: Some(3),
                forced: Some(VoiceVariant::Odd)
            }
        );
        assert_eq!(
            GovtVoice::decode(2004),
            GovtVoice {
                set: Some(4),
                forced: Some(VoiceVariant::Even)
            }
        );
        for silent in [-1, 8, 999, 1008, 2008] {
            assert_eq!(GovtVoice::decode(silent), GovtVoice::default(), "{silent}");
        }
    }

    #[test]
    fn a_run_of_clips_stops_at_the_first_gap_and_at_nine() {
        let mut sounds = run(1000, 10);
        sounds.extend(run(1010, 3));
        sounds.push((SoundId(1014), Some(SECOND)));
        let table = VoiceTable::read(&sounds);
        assert_eq!(table.count(0, VoiceMessage::Acknowledge), 9, "base + 9 not");
        assert_eq!(table.count(0, VoiceMessage::Target), 3, "1013 missing");
        assert_eq!(table.count(0, VoiceMessage::Victory), 0);
        assert_eq!(table.count(7, VoiceMessage::Acknowledge), 0, "set 7 empty");
        assert_eq!(table.count(8, VoiceMessage::Acknowledge), 0, "no set 8");
    }

    #[test]
    fn a_clip_that_cannot_be_decoded_counts_with_no_length() {
        let table = VoiceTable::read(&[(SoundId(1520), None), (SoundId(1521), Some(SECOND))]);
        assert_eq!(table.count(5, VoiceMessage::Victory), 2);
        let mut first = Draws::of(&[0]);
        assert_eq!(
            table.pick(5, VoiceMessage::Victory, None, &mut first),
            Some(Clip {
                sound: SoundId(1520),
                length: 0
            })
        );
        let mut second = Draws::of(&[1]);
        assert_eq!(
            table.pick(5, VoiceMessage::Victory, None, &mut second),
            Some(Clip {
                sound: SoundId(1521),
                length: 60
            })
        );
    }

    #[test]
    fn a_set_with_no_clips_picks_none() {
        let table = VoiceTable::read(&run(1000, 3));
        let mut draws = Draws::default();
        assert_eq!(table.pick(0, VoiceMessage::Target, None, &mut draws), None);
        assert!(draws.asked.is_empty(), "no Rand(0): {:?}", draws.asked);
        assert_eq!(
            table.pick(9, VoiceMessage::Acknowledge, None, &mut Draws::default()),
            None
        );
    }

    #[test]
    fn an_unset_variant_draws_over_every_clip() {
        let table = VoiceTable::read(&run(1400, 8));
        let mut draws = Draws::of(&[4]);
        let clip = table.pick(4, VoiceMessage::Acknowledge, None, &mut draws);
        assert_eq!(clip.map(|clip| clip.sound), Some(SoundId(1404)));
        assert_eq!(draws.asked, [8]);
    }

    #[test]
    fn an_odd_count_ignores_the_variant() {
        let table = VoiceTable::read(&run(1000, 7));
        let mut draws = Draws::of(&[6]);
        let clip = table.pick(
            0,
            VoiceMessage::Acknowledge,
            Some(VoiceVariant::Odd),
            &mut draws,
        );
        assert_eq!(clip.map(|clip| clip.sound), Some(SoundId(1006)));
        assert_eq!(draws.asked, [7]);
    }

    #[test]
    fn two_clips_give_the_variants_own() {
        let table = VoiceTable::read(&run(1010, 2));
        for (variant, sound) in [(VoiceVariant::Even, 1010), (VoiceVariant::Odd, 1011)] {
            let clip = table.pick(
                0,
                VoiceMessage::Target,
                Some(variant),
                &mut Draws::of(&[1, 1]),
            );
            assert_eq!(clip.map(|clip| clip.sound), Some(SoundId(sound)));
        }
    }

    #[test]
    fn an_even_count_of_four_or_more_picks_the_variants_clips() {
        let table = VoiceTable::read(&run(1400, 8));
        let mut draws = Draws::of(&[1]);
        let clip = table.pick(
            4,
            VoiceMessage::Acknowledge,
            Some(VoiceVariant::Odd),
            &mut draws,
        );
        assert_eq!(clip.map(|clip| clip.sound), Some(SoundId(1403)));
        assert_eq!(draws.asked, [4]);
        let even = table.pick(
            4,
            VoiceMessage::Acknowledge,
            Some(VoiceVariant::Even),
            &mut Draws::of(&[3]),
        );
        assert_eq!(even.map(|clip| clip.sound), Some(SoundId(1406)));
    }

    #[test]
    fn a_clip_is_1000_and_100_a_set_and_10_a_message_and_its_number() {
        let table = VoiceTable::read(&run(1520, 5));
        let clip = table.pick(5, VoiceMessage::Victory, None, &mut Draws::of(&[3]));
        assert_eq!(
            clip,
            Some(Clip {
                sound: SoundId(1523),
                length: 60
            })
        );
    }

    #[test]
    fn a_variant_is_the_forced_one_or_else_drawn() {
        for draw in [0, 1] {
            assert_eq!(
                roll_variant(Some(VoiceVariant::Odd), &mut Draws::of(&[draw])),
                VoiceVariant::Odd
            );
            assert_eq!(
                roll_variant(Some(VoiceVariant::Even), &mut Draws::of(&[draw])),
                VoiceVariant::Even
            );
        }
        assert_eq!(roll_variant(None, &mut Draws::of(&[0])), VoiceVariant::Even);
        assert_eq!(roll_variant(None, &mut Draws::of(&[1])), VoiceVariant::Odd);
    }

    #[test]
    fn a_set_is_its_govts_or_zero_with_none() {
        let voices = Voices::read(
            &[],
            &[(GovtId(128), 1), (GovtId(182), 2004), (GovtId(140), -1)],
        );
        assert_eq!(voices.set_of(None), Some(0), "no attributes government");
        assert_eq!(voices.set_of(Some(GovtId(128))), Some(1));
        assert_eq!(voices.set_of(Some(GovtId(182))), Some(4));
        assert_eq!(voices.set_of(Some(GovtId(140))), None, "silent");
        assert_eq!(voices.set_of(Some(GovtId(141))), None, "no record");
        assert_eq!(voices.forced_of(None), None);
        assert_eq!(voices.forced_of(Some(GovtId(128))), None);
        assert_eq!(
            voices.forced_of(Some(GovtId(182))),
            Some(VoiceVariant::Even)
        );
        assert_eq!(voices.forced_of(Some(GovtId(141))), None);
    }

    #[test]
    fn speaking_overwrites_the_message_waiting() {
        let mut channel = VoiceChannel::default();
        channel.speak(speech(VoiceMessage::Acknowledge, Some(0), None));
        channel.speak(speech(VoiceMessage::Victory, Some(1), None));
        assert_eq!(
            channel.pending(),
            Some(speech(VoiceMessage::Victory, Some(1), None))
        );
    }

    #[test]
    fn the_channel_is_idle_with_nothing_waiting_or_loaded() {
        let table = VoiceTable::read(&run(1000, 1));
        let mut channel = VoiceChannel::default();
        assert!(channel.idle());
        channel.speak(speech(VoiceMessage::Acknowledge, Some(0), None));
        assert!(!channel.idle(), "waiting");
        assert!(channel.handle(&table, &mut Draws::default()).is_some());
        assert!(!channel.idle(), "loaded");
        for _ in 0..30 {
            channel.step();
        }
        assert!(!channel.idle(), "over, but loaded until handled");
        assert_eq!(channel.handle(&table, &mut Draws::default()), None);
        assert!(channel.idle(), "disposed");
    }

    #[test]
    fn handling_with_nothing_waiting_raises_nothing() {
        let table = VoiceTable::read(&run(1000, 1));
        assert_eq!(
            VoiceChannel::default().handle(&table, &mut Draws::default()),
            None
        );
    }

    #[test]
    fn handling_plays_the_message_waiting_and_clears_it() {
        let table = VoiceTable::read(&run(1520, 5));
        let mut channel = VoiceChannel::default();
        channel.speak(speech(VoiceMessage::Victory, Some(5), None));
        assert_eq!(
            channel.handle(&table, &mut Draws::of(&[3])),
            Some(SimSound::Voice {
                sound: SoundId(1523)
            })
        );
        assert_eq!(channel.pending(), None);
    }

    #[test]
    fn a_clip_plays_out_its_length_and_is_disposed_at_the_handle_after() {
        let table = VoiceTable::read(&run(1000, 1));
        let mut channel = VoiceChannel::default();
        channel.speak(speech(VoiceMessage::Acknowledge, Some(0), None));
        channel.handle(&table, &mut Draws::default());
        for _ in 0..29 {
            channel.step();
        }
        // Two Mac ticks left: a message now cuts it off.
        channel.speak(speech(VoiceMessage::Acknowledge, Some(0), None));
        let mut now = channel;
        assert_eq!(
            now.handle(&table, &mut Draws::default()),
            Some(SimSound::HaltVoice {
                sound: SoundId(1000)
            })
        );
        channel.step();
        assert_eq!(
            channel.handle(&table, &mut Draws::default()),
            Some(SimSound::Voice {
                sound: SoundId(1000)
            }),
            "over: disposed, and the message plays at once"
        );
    }

    #[test]
    fn a_message_waiting_halts_the_clip_playing_and_plays_at_the_next_handle() {
        let table = VoiceTable::read(&run(1000, 1));
        let mut channel = VoiceChannel::default();
        channel.speak(speech(VoiceMessage::Acknowledge, Some(0), None));
        channel.handle(&table, &mut Draws::default());
        channel.step();
        let waiting = speech(VoiceMessage::Acknowledge, Some(0), None);
        channel.speak(waiting);
        assert_eq!(
            channel.handle(&table, &mut Draws::default()),
            Some(SimSound::HaltVoice {
                sound: SoundId(1000)
            })
        );
        assert_eq!(channel.pending(), Some(waiting), "kept");
        assert_eq!(
            channel.handle(&table, &mut Draws::default()),
            Some(SimSound::Voice {
                sound: SoundId(1000)
            })
        );
        assert_eq!(channel.pending(), None);
    }

    #[test]
    fn a_silent_set_or_a_count_of_0_drops_the_message_with_no_sound() {
        let table = VoiceTable::read(&run(1000, 1));
        for dropped in [
            speech(VoiceMessage::Acknowledge, None, None),
            speech(VoiceMessage::Target, Some(0), None),
            speech(VoiceMessage::Acknowledge, Some(7), None),
        ] {
            let mut channel = VoiceChannel::default();
            channel.speak(dropped);
            assert_eq!(
                channel.handle(&table, &mut Draws::default()),
                None,
                "{dropped:?}"
            );
            assert!(channel.idle(), "{dropped:?}");
        }
    }

    #[test]
    fn killing_halts_the_clip_loaded_and_drops_the_message() {
        let table = VoiceTable::read(&run(1000, 1));
        let mut channel = VoiceChannel::default();
        channel.speak(speech(VoiceMessage::Acknowledge, Some(0), None));
        channel.handle(&table, &mut Draws::default());
        channel.speak(speech(VoiceMessage::Victory, Some(0), None));
        assert_eq!(
            channel.kill(),
            Some(SimSound::HaltVoice {
                sound: SoundId(1000)
            })
        );
        assert!(channel.idle());
        assert_eq!(VoiceChannel::default().kill(), None);
    }
}
