//! The can-scoop flag (+0xc3) each ship-table slot keeps after its ship
//! is gone.
//!
//! The spawns that call `_ShipCanScoop` (@0x75c7-0x766b) write the word:
//! `_RandomShipSpawn` @0x3c608, `_LaunchFighter` @0x3d74e, `_SpawnEscort`
//! @0x3ff6b, `_SpawnPerson` @0x40d79, `_DoShipCapture` @0x41255 and
//! `_HyperSpawnFleet` @0x41e31, for a fleet's lead only.
//! `_GenericRandomShipSpawn` (@0x3c89f-0x3cd3a) writes the flag bytes
//! around it (+0xbd = 0 @0x3cb83, +0xbe and +0xbf @0x3cbbf-0x3cbc9) but
//! not this one, so every ship it brings in with no later ask holds what
//! the slot's last writer left: a mission ship, an auxiliary or defence
//! ship, a fleet's escort and the eject's hulk
//! ([`NewShip::asks_scoop`](super::spawn::NewShip::asks_scoop),
//! [`Traffic::add_npc_unscooped`]). The table is zero-filled once
//! (`_InitArrays` @0x1b7a1-0x1b7b5) and nothing clears the word, so a
//! never-used slot reads 0.
//!
//! Only `_MinerAI` reads an NPC's flag (@0x8b2de, behind its `SCOOPS`
//! test; see [`miner`](crate::ai::miner)), so the word matters only for a
//! miner class, and in stock only for a mission Asteroid Miner (see
//! [`scoop`](crate::scoop)).
//!
//! [`BerthScoops`], over a [`BerthKept`], keeps that word for each berth
//! and gives a ship whose spawn asks none its berth's by the reading of
//! [`RuleKey::ReusedSlotScoop`](crate::RuleKey::ReusedSlotScoop): the last
//! writer's, or a never-used berth's false, by the engine; its class's
//! answer by the other reading. A ship that leaves keeps its word in the
//! berth.
//!
//! It lasts as long as the app, as the original keeps the word through a
//! pilot load or a new game (see [`kept_berths`](super::kept_berths)).

use crate::rulebook::RuleSource;

use super::Traffic;
use super::berth_kept::BerthKept;
use super::npc::{Npc, NpcId};

/// The can-scoop flag each berth's last writer left, and the reading a
/// ship whose spawn asks none takes it by (see the module docs).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BerthScoops(BerthKept<bool>);

impl BerthScoops {
    /// The flag the last writer in `berth` left, whatever the reading:
    /// none for a berth no writer has used.
    #[must_use]
    pub fn left(&self, berth: u32) -> Option<bool> {
        self.0.left(berth)
    }

    /// The can-scoop flag of a new ship brought into `berth`: `written`,
    /// its spawn's `_ShipCanScoop` answer, which it leaves in the berth;
    /// with none, by the engine the berth's word, false for a never-used
    /// berth, and by the other reading `class`, its class's answer.
    pub fn bring_in(&mut self, berth: u32, written: Option<bool>, class: bool) -> bool {
        if let Some(flag) = written {
            self.0.write(berth, Some(flag));
            return flag;
        }
        match self.0.reading() {
            RuleSource::Engine => self.left(berth).unwrap_or(false),
            RuleSource::Bible => class,
        }
    }

    /// Takes new ships' flags by `reading`.
    pub fn set_reading(&mut self, reading: RuleSource) {
        self.0.set_reading(reading);
    }

    /// Takes `other`'s flag for every berth in place of its own, keeping
    /// its own reading (see [`kept_berths`](super::kept_berths)).
    pub fn take_scoops_of(&mut self, other: Self) {
        self.0.take_words_of(other.0);
    }

    /// The reading new ships' flags are taken by.
    #[must_use]
    pub fn reading(&self) -> RuleSource {
        self.0.reading()
    }
}

impl Traffic {
    /// Gives each ship whose spawn asks no `_ShipCanScoop` its berth's
    /// last can-scoop flag by `reading`
    /// ([`RuleKey::ReusedSlotScoop`](crate::RuleKey::ReusedSlotScoop)):
    /// by the engine, a never-used berth's false included; its class's
    /// answer by the other reading.
    pub fn set_reused_slot_scoop(&mut self, reading: RuleSource) {
        self.berth_scoops.set_reading(reading);
    }

    /// The reading ships whose spawn asks no `_ShipCanScoop` take their
    /// berth's flag by (see [`Traffic::set_reused_slot_scoop`]).
    #[must_use]
    pub fn reused_slot_scoop(&self) -> RuleSource {
        self.berth_scoops.reading()
    }

    /// Adds `npc` to the system at the next berth, as
    /// [`Traffic::add_npc`] does, for a spawn that asks no
    /// `_ShipCanScoop`: the eject's hulk (`_HandlePlayer` @0x6f2f8, through
    /// `_GenericRandomShipSpawn`). It holds its berth's flag by the
    /// engine, its own `can_scoop` by the other reading, and writes none.
    /// It keeps its berth's escort-command mark too
    /// ([`berth_marks`](super::berth_marks)).
    pub fn add_npc_unscooped(&mut self, npc: Npc) -> NpcId {
        let berth = self.next_berth();
        self.add_npc_scooped(npc, berth, false, false)
    }

    /// Writes `flag` as the can-scoop word of the ship last in `berth`,
    /// for tests that set a berth's word up.
    ///
    /// Kept for nova-sim's slot-scoop tests (`session::slot_scoop`).
    #[cfg(any(test, feature = "fixture"))]
    pub fn write_berth_scoop(&mut self, berth: u32, flag: bool) {
        self.berth_scoops.0.write(berth, Some(flag));
    }

    /// The can-scoop word the last writer in `berth` left, none for a
    /// berth no writer has used.
    ///
    /// Kept for nova-sim's slot-scoop tests (`session::slot_scoop`).
    #[cfg(any(test, feature = "fixture"))]
    #[must_use]
    pub fn berth_scoop(&self, berth: u32) -> Option<bool> {
        self.berth_scoops.left(berth)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::ai::miner::MINER_BITS;
    use crate::catalog::ShipId;
    use crate::flight::ShipState;
    use crate::govt::Governments;
    use crate::stats::ShipStats;
    use crate::traffic::SPAWN_RESERVE;
    use crate::traffic::fixture::npc;
    use crate::traffic::npc::AiType;
    use crate::traffic::spawn::{Entry, NewShip};
    use crate::traffic::table::{ShipKind, SpawnTable};
    use nova_sim_base::fixture::{FAST, Ranged};

    #[test]
    fn a_never_used_berth_reads_false_by_the_engine_and_the_class_by_the_other_reading() {
        let mut kept = BerthScoops::default();
        assert_eq!(kept.reading(), RuleSource::Engine);
        assert!(!kept.bring_in(3, None, true));
        kept.set_reading(RuleSource::Bible);
        assert_eq!(kept.reading(), RuleSource::Bible);
        assert!(kept.bring_in(3, None, true));
        assert!(!kept.bring_in(3, None, false));
    }

    #[test]
    fn a_spawn_that_asks_writes_its_answer_and_holds_it_under_both_readings() {
        for reading in [RuleSource::Engine, RuleSource::Bible] {
            let mut kept = BerthScoops::default();
            kept.set_reading(reading);
            assert!(kept.bring_in(1, Some(true), false), "{reading:?}");
            assert_eq!(kept.left(1), Some(true), "{reading:?}");
            assert!(!kept.bring_in(1, Some(false), true), "{reading:?}");
            assert_eq!(kept.left(1), Some(false), "{reading:?}");
        }
    }

    #[test]
    fn a_spawn_that_asks_none_leaves_the_berths_word_as_it_was() {
        for reading in [RuleSource::Engine, RuleSource::Bible] {
            let mut kept = BerthScoops::default();
            kept.set_reading(reading);
            kept.bring_in(1, Some(true), true);
            kept.bring_in(1, None, false);
            assert_eq!(kept.left(1), Some(true), "{reading:?}");
            kept.bring_in(2, None, true);
            assert_eq!(kept.left(2), None, "{reading:?}: never written");
        }
    }

    #[test]
    fn a_spawn_that_asks_none_holds_the_berths_word_by_the_engine() {
        let mut kept = BerthScoops::default();
        kept.bring_in(1, Some(true), true);
        assert!(kept.bring_in(1, None, false));
        kept.bring_in(2, Some(false), false);
        assert!(!kept.bring_in(2, None, true));
    }

    #[test]
    fn taking_anothers_scoops_keeps_this_ones_reading() {
        let mut given = BerthScoops::default();
        given.bring_in(1, Some(true), true);
        let mut taker = BerthScoops::default();
        taker.bring_in(2, Some(true), true);
        taker.set_reading(RuleSource::Bible);
        taker.take_scoops_of(given);
        assert_eq!(taker.reading(), RuleSource::Bible);
        assert_eq!((taker.left(1), taker.left(2)), (Some(true), None));
        taker.set_reading(RuleSource::Engine);
        assert!(taker.bring_in(1, None, false));
    }

    /// A ship of a miner class (`SCOOPS` and `MINES`) of ship type 200,
    /// whose class answers `can_scoop`, that `asks` or not.
    fn miner(asks: bool) -> NewShip {
        NewShip {
            ship: ShipId(200),
            govt: None,
            ai_type: AiType::WimpyTrader.into(),
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
            asks_scoop: asks,
            clears_mark: false,
        }
    }

    /// `traffic`, read by `reading`, with ship type 200 a miner class
    /// whose class answers `can_scoop`.
    fn of_class(mut traffic: Traffic, can_scoop: bool, reading: RuleSource) -> Traffic {
        traffic.set_reused_slot_scoop(reading);
        *traffic.table_mut() = SpawnTable {
            ships: BTreeMap::from([(
                ShipId(200),
                ShipKind {
                    can_scoop,
                    miner: MINER_BITS,
                    ..ShipKind::default()
                },
            )]),
            ..SpawnTable::default()
        };
        traffic
    }

    /// Brings `ship` into `traffic` and gives its can-scoop flag and
    /// berth.
    fn bring(traffic: &mut Traffic, ship: NewShip) -> (bool, u32) {
        let came = traffic.bring_in_within(
            vec![ship],
            SPAWN_RESERVE,
            &Governments::empty(),
            &mut Ranged::of(&[]),
        );
        assert_eq!(came, 1);
        let npc = traffic.npcs().last().expect("brought in");
        (npc.can_scoop, npc.berth)
    }

    /// The can-scoop flag of `id` in `traffic`.
    fn flag_of(traffic: &Traffic, id: NpcId) -> bool {
        traffic
            .npcs()
            .iter()
            .find(|npc| npc.id == id)
            .expect("in the system")
            .can_scoop
    }

    /// A traffic whose every berth below 8 holds `flag`.
    fn every_berth(flag: bool) -> Traffic {
        let mut traffic = Traffic::new();
        for berth in 0..8 {
            traffic.write_berth_scoop(berth, flag);
        }
        traffic
    }

    #[test]
    fn the_reading_is_set_on_the_traffic() {
        let mut traffic = Traffic::new();
        assert_eq!(traffic.reused_slot_scoop(), RuleSource::Engine);
        traffic.set_reused_slot_scoop(RuleSource::Bible);
        assert_eq!(traffic.reused_slot_scoop(), RuleSource::Bible);
    }

    #[test]
    fn a_generic_ship_of_a_scooping_class_in_a_never_used_berth_holds_false_by_the_engine() {
        let mut engine = of_class(Traffic::new(), true, RuleSource::Engine);
        let (flag, berth) = bring(&mut engine, miner(false));
        assert!(!flag, "the zero fill");
        assert_eq!(engine.berth_scoop(berth), None, "and writes nothing");
        let mut bible = of_class(Traffic::new(), true, RuleSource::Bible);
        assert!(bring(&mut bible, miner(false)).0, "its class's answer");
    }

    #[test]
    fn a_generic_ship_of_a_scooping_class_holds_true_where_its_berths_writer_left_it() {
        for reading in [RuleSource::Engine, RuleSource::Bible] {
            let mut traffic = of_class(every_berth(true), true, reading);
            assert!(bring(&mut traffic, miner(false)).0, "{reading:?}");
        }
    }

    #[test]
    fn a_generic_ship_of_a_no_scoop_class_holds_the_true_its_berths_writer_left_by_the_engine() {
        let mut engine = of_class(every_berth(true), false, RuleSource::Engine);
        let (flag, berth) = bring(&mut engine, miner(false));
        assert!(flag, "the slot's");
        assert_eq!(engine.berth_scoop(berth), Some(true));
        let mut bible = of_class(every_berth(true), false, RuleSource::Bible);
        assert!(!bring(&mut bible, miner(false)).0, "its class's answer");
    }

    #[test]
    fn a_ship_that_asks_holds_its_classs_answer_and_leaves_it_to_its_berths_next_ship() {
        for (reading, class) in [
            (RuleSource::Engine, true),
            (RuleSource::Engine, false),
            (RuleSource::Bible, true),
            (RuleSource::Bible, false),
        ] {
            let mut traffic = of_class(every_berth(!class), class, reading);
            let (flag, berth) = bring(&mut traffic, miner(true));
            assert_eq!(flag, class, "{reading:?} {class}");
            assert_eq!(
                traffic.berth_scoop(berth),
                Some(class),
                "{reading:?} {class}"
            );
            let id = traffic.npcs()[0].id;
            traffic.remove(id);
            // The next generic ship there, of the other class.
            let mut next = of_class(traffic, !class, reading);
            let (flag, again) = bring(&mut next, miner(false));
            assert_eq!(again, berth, "the same berth");
            let expected = match reading {
                RuleSource::Engine => class,
                RuleSource::Bible => !class,
            };
            assert_eq!(flag, expected, "{reading:?} {class}");
        }
    }

    #[test]
    fn add_npc_at_writes_the_npcs_flag_to_its_berth() {
        for reading in [RuleSource::Engine, RuleSource::Bible] {
            let mut traffic = Traffic::new();
            traffic.set_reused_slot_scoop(reading);
            traffic.write_berth_scoop(4, false);
            let mut scooper = npc(0, ShipStats::new(FAST, &[]));
            scooper.can_scoop = true;
            let id = traffic.add_npc_at(scooper, 4);
            assert!(flag_of(&traffic, id), "{reading:?}");
            assert_eq!(traffic.berth_scoop(4), Some(true), "{reading:?}");
        }
    }

    #[test]
    fn add_npc_unscooped_takes_its_berths_word_by_the_engine_and_never_writes() {
        let mut traffic = every_berth(true);
        let id = traffic.add_npc_unscooped(npc(0, ShipStats::new(FAST, &[])));
        assert!(flag_of(&traffic, id), "the berth's");
        let berth = traffic.npcs()[0].berth;
        assert_eq!(traffic.berth_scoop(berth), Some(true));

        let mut fresh = Traffic::new();
        let mut scooper = npc(0, ShipStats::new(FAST, &[]));
        scooper.can_scoop = true;
        let id = fresh.add_npc_unscooped(scooper);
        assert!(!flag_of(&fresh, id), "the zero fill");
        let berth = fresh.npcs()[0].berth;
        assert_eq!(fresh.berth_scoop(berth), None, "nothing written");
    }

    #[test]
    fn add_npc_unscooped_keeps_the_npcs_own_by_the_other_reading() {
        let mut traffic = every_berth(true);
        traffic.set_reused_slot_scoop(RuleSource::Bible);
        let id = traffic.add_npc_unscooped(npc(0, ShipStats::new(FAST, &[])));
        assert!(!flag_of(&traffic, id), "its own");
        let berth = traffic.npcs()[0].berth;
        assert_eq!(traffic.berth_scoop(berth), Some(true), "left as it was");

        let mut traffic = every_berth(false);
        traffic.set_reused_slot_scoop(RuleSource::Bible);
        let mut scooper = npc(0, ShipStats::new(FAST, &[]));
        scooper.can_scoop = true;
        let id = traffic.add_npc_unscooped(scooper);
        assert!(flag_of(&traffic, id), "its own over the berth's false");
        let berth = traffic.npcs()[0].berth;
        assert_eq!(traffic.berth_scoop(berth), Some(false), "left as it was");

        let mut fresh = Traffic::new();
        fresh.set_reused_slot_scoop(RuleSource::Bible);
        let mut scooper = npc(0, ShipStats::new(FAST, &[]));
        scooper.can_scoop = true;
        let id = fresh.add_npc_unscooped(scooper);
        assert!(flag_of(&fresh, id), "its own over the zero fill");
        let berth = fresh.npcs()[0].berth;
        assert_eq!(fresh.berth_scoop(berth), None, "nothing written");
    }
}
