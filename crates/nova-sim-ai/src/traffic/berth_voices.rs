//! The voice variant (+0xc922) each ship-table slot keeps after its ship
//! is gone.
//!
//! Ten places write the word, each with 0 or 1: `_RandomShipSpawn`
//! @0x3c719, `_LaunchFighter` @0x3d9d8 and @0x3da3a, `_DoShipCapture`
//! @0x4129a and @0x412db, `_EscortAI` @0x839b9 and @0x839f6,
//! `_AIShipCapturedByPlayer` @0x8975c and @0x89799, and `_LoadPilotData`
//! @0x75b85. Nothing copies a whole slot over it. `_GenericRandomShipSpawn`
//! (@0x3c89f-0x3cd3a) writes +0xc920, +0xc924 and +0xc926 but not this word,
//! so everything spawned through it holds what the slot's last ship left: a
//! hired escort, a person, a mission or auxiliary ship, a fleet's ship, the
//! nag ship and the escape pod. Only the traffic of `_RandomShipSpawn`
//! draws its own, Rand(2) with no government
//! ([`NewShip::rolls_voice`](super::spawn::NewShip::rolls_voice)).
//!
//! The readers (`_HandleShipHit` @0x36d18, `_IssueNewEscortCommand`
//! @0x66313 and @0x663a1, `_EscortAI` @0x8399d and @0x83e0a, and the save,
//! `_WritePilotData` @0x72753) read only ships the player leads, so the word
//! matters only there; any other ship only leaves it in the slot.
//!
//! The table is a `NewPtr(0x325000)` (`_InitArrays` @0x1b7a1-0x1b7b5) that
//! only the system's zero fill sets, and `_InitObjects` (@0x1c2e5-0x1c58d)
//! doesn't touch the word, so a never-used slot reads 0, the even clips.
//! No value above 1 ever reaches `_EscortAI`'s roll (@0x83980-0x839f6): the
//! roll is dead on the zero fill, and with it the forced variant of a
//! government, for any ship that didn't go through `_LaunchFighter` or a
//! capture.
//!
//! [`BerthVoices`], over a [`BerthKept`], keeps that word for each berth
//! and gives a new ship its berth's by the reading of
//! [`RuleKey::ReusedSlotVoice`](crate::RuleKey::ReusedSlotVoice): the last
//! ship's, or a never-used berth's even, by the engine; none by the other
//! reading, which leaves `_EscortAI`'s roll to give the ship one at its
//! first think, a forced variant included.
//!
//! It lasts as long as the app, as the original keeps the word through a
//! pilot load or a new game (see [`kept_berths`](super::kept_berths)).

use crate::rulebook::RuleSource;
use crate::voice::VoiceVariant;

use super::Traffic;
use super::berth_kept::BerthKept;
use super::npc::NpcId;

/// The voice variant each berth's last ship left, and the reading a new
/// ship takes it by (see the module docs).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BerthVoices(BerthKept<VoiceVariant>);

impl BerthVoices {
    /// Writes `variant` as the word of the ship in `berth`.
    pub fn write(&mut self, berth: u32, variant: VoiceVariant) {
        self.0.write(berth, Some(variant));
    }

    /// The word a new ship brought into `berth` holds: by the engine, the
    /// one its last ship left, or a never-used berth's even; none by the
    /// other reading.
    #[must_use]
    pub fn seed(&self, berth: u32) -> Option<VoiceVariant> {
        match self.0.reading() {
            RuleSource::Engine => Some(self.0.left(berth).unwrap_or(VoiceVariant::Even)),
            RuleSource::Bible => None,
        }
    }

    /// The word of a new ship brought into `berth`: `written`, its spawn's
    /// own, which it leaves in the berth; else what it [seeds](Self::seed).
    pub fn bring_in(&mut self, berth: u32, written: Option<VoiceVariant>) -> Option<VoiceVariant> {
        if let Some(variant) = written {
            self.write(berth, variant);
        }
        written.or(self.seed(berth))
    }

    /// Takes new ships' words by `reading`.
    pub fn set_reading(&mut self, reading: RuleSource) {
        self.0.set_reading(reading);
    }

    /// Takes `other`'s word for every berth in place of its own, keeping
    /// its own reading (see [`kept_berths`](super::kept_berths)).
    pub fn take_variants_of(&mut self, other: Self) {
        self.0.take_words_of(other.0);
    }

    /// The reading new ships' words are taken by.
    #[must_use]
    pub fn reading(&self) -> RuleSource {
        self.0.reading()
    }
}

impl Traffic {
    /// Writes `variant` as NPC `id`'s voice and its berth's word, as every
    /// write of +0xc922 is both.
    pub fn set_voice(&mut self, id: NpcId, variant: VoiceVariant) {
        if let Some(npc) = self.npcs.iter_mut().find(|npc| npc.id == id) {
            npc.voice = Some(variant);
            self.berth_voices.write(npc.berth, variant);
        }
    }

    /// Gives each new ship its berth's last voice variant by `reading`
    /// ([`RuleKey::ReusedSlotVoice`](crate::RuleKey::ReusedSlotVoice)): by
    /// the engine, as most spawns write no +0xc922, a never-used berth's
    /// even included; none by the other reading.
    pub fn set_reused_slot_voice(&mut self, reading: RuleSource) {
        self.berth_voices.set_reading(reading);
    }

    /// The reading new ships take their berth's last voice variant by (see
    /// [`Traffic::set_reused_slot_voice`]).
    #[must_use]
    pub fn reused_slot_voice(&self) -> RuleSource {
        self.berth_voices.reading()
    }

    /// Writes `variant` as the voice word of the ship last in `berth`, for
    /// tests that set a berth's word up.
    ///
    /// Kept for nova-sim's slot-voice tests (`session::slot_voice`).
    #[cfg(any(test, feature = "fixture"))]
    pub fn write_berth_voice(&mut self, berth: u32, variant: VoiceVariant) {
        self.berth_voices.write(berth, variant);
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::catalog::ShipId;
    use crate::flight::ShipState;
    use crate::govt::Governments;
    use crate::stats::ShipStats;
    use crate::traffic::SPAWN_RESERVE;
    use crate::traffic::fixture::npc;
    use crate::traffic::npc::AiType;
    use crate::traffic::spawn::{Entry, NewShip};
    use crate::traffic::table::{ShipKind, SpawnTable};
    use nova_sim_base::fixture::{Draws, FAST};

    #[test]
    fn a_never_used_berth_seeds_even_by_the_engine_and_none_by_the_other_reading() {
        let mut kept = BerthVoices::default();
        assert_eq!(kept.reading(), RuleSource::Engine);
        assert_eq!(kept.seed(3), Some(VoiceVariant::Even));
        kept.set_reading(RuleSource::Bible);
        assert_eq!(kept.reading(), RuleSource::Bible);
        assert_eq!(kept.seed(3), None);
    }

    #[test]
    fn a_berths_word_reads_back_and_is_overwritten() {
        let mut kept = BerthVoices::default();
        kept.write(1, VoiceVariant::Odd);
        assert_eq!(kept.seed(1), Some(VoiceVariant::Odd));
        assert_eq!(kept.seed(2), Some(VoiceVariant::Even), "another berth");
        kept.write(1, VoiceVariant::Even);
        assert_eq!(kept.seed(1), Some(VoiceVariant::Even));
    }

    #[test]
    fn taking_anothers_variants_keeps_this_ones_reading() {
        let mut given = BerthVoices::default();
        given.write(1, VoiceVariant::Odd);
        let mut taker = BerthVoices::default();
        taker.set_reading(RuleSource::Bible);
        taker.take_variants_of(given);
        assert_eq!(taker.reading(), RuleSource::Bible);
        taker.set_reading(RuleSource::Engine);
        assert_eq!(taker.seed(1), Some(VoiceVariant::Odd));
    }

    /// The voice of `id` in `traffic`.
    fn voice_of(traffic: &Traffic, id: NpcId) -> Option<VoiceVariant> {
        traffic
            .npcs()
            .iter()
            .find(|npc| npc.id == id)
            .expect("in the system")
            .voice
    }

    #[test]
    fn a_ship_added_takes_its_berths_word() {
        let mut traffic = Traffic::new();
        traffic.write_berth_voice(4, VoiceVariant::Odd);
        let id = traffic.add_npc_at(npc(0, ShipStats::new(FAST, &[])), 4);
        assert_eq!(voice_of(&traffic, id), Some(VoiceVariant::Odd));
        let fresh = traffic.add_npc_at(npc(0, ShipStats::new(FAST, &[])), 5);
        assert_eq!(voice_of(&traffic, fresh), Some(VoiceVariant::Even));
    }

    #[test]
    fn a_ship_added_by_the_other_reading_takes_no_word() {
        let mut traffic = Traffic::new();
        traffic.write_berth_voice(4, VoiceVariant::Odd);
        traffic.set_reused_slot_voice(RuleSource::Bible);
        assert_eq!(traffic.reused_slot_voice(), RuleSource::Bible);
        let id = traffic.add_npc_at(npc(0, ShipStats::new(FAST, &[])), 4);
        assert_eq!(voice_of(&traffic, id), None);
    }

    #[test]
    fn a_ship_added_with_a_variant_keeps_it_and_leaves_it_to_the_berths_next_ship() {
        let mut traffic = Traffic::new();
        let mut first = npc(0, ShipStats::new(FAST, &[]));
        first.voice = Some(VoiceVariant::Odd);
        let id = traffic.add_npc_at(first, 4);
        assert_eq!(voice_of(&traffic, id), Some(VoiceVariant::Odd));
        traffic.remove(id);
        let next = traffic.add_npc_at(npc(0, ShipStats::new(FAST, &[])), 4);
        assert_eq!(voice_of(&traffic, next), Some(VoiceVariant::Odd));
    }

    /// `ship` of ship type 200, brought in on `draws`: the voice of the
    /// ship, at berth 1, and `chance`.
    fn brought_in(mut traffic: Traffic, ship: NewShip, draws: &[u32]) -> (Traffic, Draws) {
        *traffic.table_mut() = SpawnTable {
            ships: BTreeMap::from([(ShipId(200), ShipKind::default())]),
            ..SpawnTable::default()
        };
        let mut chance = Draws::of(draws);
        let came = traffic.bring_in_within(
            vec![ship],
            SPAWN_RESERVE,
            &Governments::empty(),
            &mut chance,
        );
        assert_eq!(came, 1);
        (traffic, chance)
    }

    /// The ship of a spawn other than `_RandomShipSpawn`.
    fn new_ship() -> NewShip {
        NewShip {
            ship: ShipId(200),
            govt: None,
            ai_type: AiType::WimpyTrader,
            lead: None,
            state: ShipState::default(),
            entry: Entry::Placed,
            aggression: 0,
            booty: None,
            info_types: 0,
            person: None,
            mission: None,
            aux: None,
            derelict: false,
            held: false,
            boarded: false,
            cloaked: false,
            quote: None,
            cargo: None,
            rolls_voice: false,
        }
    }

    #[test]
    fn a_ship_that_rolls_draws_its_voice_and_leaves_it_in_its_berth() {
        let mut traffic = Traffic::new();
        traffic.write_berth_voice(0, VoiceVariant::Even);
        let ship = NewShip {
            rolls_voice: true,
            ..new_ship()
        };
        let (mut traffic, chance) = brought_in(traffic, ship, &[1]);
        let drawn = traffic.npcs()[0].clone();
        assert_eq!(drawn.voice, Some(VoiceVariant::Odd));
        assert_eq!(chance.times_asked(2), 1);
        traffic.remove(drawn.id);
        let next = traffic.add_npc_at(npc(0, ShipStats::new(FAST, &[])), drawn.berth);
        assert_eq!(voice_of(&traffic, next), Some(VoiceVariant::Odd));
    }

    #[test]
    fn a_ship_that_rolls_stands_on_its_draw_by_the_other_reading_too() {
        let mut traffic = Traffic::new();
        traffic.set_reused_slot_voice(RuleSource::Bible);
        let ship = NewShip {
            rolls_voice: true,
            ..new_ship()
        };
        let (traffic, _) = brought_in(traffic, ship, &[1]);
        assert_eq!(traffic.npcs()[0].voice, Some(VoiceVariant::Odd));
    }

    #[test]
    fn a_ship_that_doesnt_roll_takes_its_berths_word_and_draws_nothing_for_it() {
        let mut traffic = Traffic::new();
        for berth in 0..4 {
            traffic.write_berth_voice(berth, VoiceVariant::Odd);
        }
        let (traffic, chance) = brought_in(traffic, new_ship(), &[]);
        assert_eq!(traffic.npcs()[0].voice, Some(VoiceVariant::Odd));
        assert_eq!(chance.times_asked(2), 0);
    }

    #[test]
    fn set_voice_writes_the_ship_and_its_berth() {
        let mut traffic = Traffic::new();
        let id = traffic.add_npc_at(npc(0, ShipStats::new(FAST, &[])), 4);
        traffic.set_voice(id, VoiceVariant::Odd);
        assert_eq!(voice_of(&traffic, id), Some(VoiceVariant::Odd));
        assert_eq!(traffic.berth_voices.seed(4), Some(VoiceVariant::Odd));
        assert_eq!(
            traffic.berth_voices.seed(5),
            Some(VoiceVariant::Even),
            "no other berth"
        );
    }

    #[test]
    fn set_voice_on_a_ship_not_in_the_system_writes_nothing() {
        let mut traffic = Traffic::new();
        traffic.set_voice(NpcId(9), VoiceVariant::Odd);
        assert_eq!(traffic.berth_voices, BerthVoices::default());
    }
}
