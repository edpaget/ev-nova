//! The escort-command mark (+0xc4) each ship-table slot keeps after its
//! ship is gone.
//!
//! **Who writes it.** Two sites set the byte, three clear it, and nothing
//! else writes it; nothing copies a whole slot either.
//! - `_IssueNewEscortCommand` sets it (@0x66190) on a ship the player
//!   leads that takes a command differing from its order (`nova_sim`'s
//!   `class_orders` and `command_word` modules), and
//!   `_AIIssueEscortOrders` sets it (@0x89b71) on every follower an NPC
//!   lead orders, every eighth frame
//!   ([`escort_orders`](super::escort_orders)): so an NPC fleet's escorts
//!   are marked within a few frames of arriving.
//! - `_RandomShipSpawn` (@0x3c6ff), `_RespawnEscort` (@0x3d546) and
//!   `_LaunchFighter` (@0x3d9c2, after its own `_GenericRandomShipSpawn`
//!   @0x3d657) clear it: a traffic `düde` ship
//!   ([`NewShip::clears_mark`](super::spawn::NewShip::clears_mark)), the
//!   fleet's respawn and a launched fighter ([`Traffic::add_npc`],
//!   [`Traffic::add_npc_at`]).
//! - `_GenericRandomShipSpawn` (@0x3c89f-0x3cd3a) never writes it, so
//!   every other placement keeps the slot's byte: a mission ship
//!   (`_MissionDudeSpawn` @0x3cd67), a person (`_SpawnPerson` @0x40cc5),
//!   an auxiliary or defence ship (`_SpecificDudeSpawn` @0x3d10f), a
//!   fleet's lead and escorts (`_HyperSpawnFleet` @0x41cf4, @0x421b0), a
//!   hire (`_SpawnEscort` @0x3fe8f), the old ship of "Use As My Ship"
//!   (`_DoShipCapture` @0x4114d; [`Traffic::add_npc_keeping_mark`]) and
//!   the eject's hulk (`_HandlePlayer` @0x6f2f8,
//!   [`Traffic::add_npc_unscooped`]). A capture, a hail's release and a
//!   join by hail keep the slot and its byte.
//!
//! **Who reads it.** `_HandlePlayer`'s class-order bookkeeping: loop 1
//! (@0x6e368, AI type 5 only) and loop 2 (@0x6e3f1, any led ship); and
//! `_EscortAI` (@0x83b22, AI type 5 only). Every AI-type-5 ship comes from
//! `_LaunchFighter`, which clears the byte, so a kept mark reaches only
//! loop 2 (`nova_sim`'s `class_orders` module).
//!
//! The table is zero-filled once (`_InitArrays` @0x1b7a1-0x1b7b5), so a
//! never-used slot reads 0, and the word outlasts its ship, its system, its
//! pilot and its game (see [`kept_berths`](super::kept_berths)).
//!
//! **Here.** [`BerthMarks`], over a [`BerthKept`], keeps the word for each
//! berth. The writes go to the berth as the ship is marked
//! ([`Traffic::mark_berth_of`], and the escort-orders pass): a ship's mark
//! only ever goes from unmarked to marked, so the word stays equal to the
//! slot's byte. A placement that clears writes false; one that keeps
//! gives its ship the berth's word
//! ([`Npc::berth_mark`](super::npc::Npc::berth_mark)) by the reading of
//! [`RuleKey::ReusedSlotMark`](crate::RuleKey::ReusedSlotMark): the last
//! ship's, or a never-used berth's false, by the engine; false by the other
//! reading, where every ship placed starts unmarked.

use crate::rulebook::RuleSource;

use super::Traffic;
use super::berth_kept::BerthKept;
use super::npc::{Npc, NpcId};

/// The escort-command mark each berth's last ship left, and the reading a
/// ship placed by a path that keeps it takes it by (see the module docs).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BerthMarks(BerthKept<bool>);

impl BerthMarks {
    /// The mark the ship last in `berth` left, whatever the reading: none
    /// for a berth no ship has written.
    #[must_use]
    pub fn left(&self, berth: u32) -> Option<bool> {
        self.0.left(berth)
    }

    /// The mark of a new ship brought into `berth`: a placement that
    /// `clears` writes false and holds it; one that keeps writes nothing
    /// and holds, by the engine, the berth's word (false for a never-used
    /// berth), and false by the other reading.
    pub fn bring_in(&mut self, berth: u32, clears: bool) -> bool {
        if clears {
            self.0.write(berth, Some(false));
            return false;
        }
        match self.0.reading() {
            RuleSource::Engine => self.left(berth).unwrap_or(false),
            RuleSource::Bible => false,
        }
    }

    /// Marks the ship in `berth`, writing true to its word.
    pub fn mark(&mut self, berth: u32) {
        self.0.write(berth, Some(true));
    }

    /// Takes new ships' marks by `reading`.
    pub fn set_reading(&mut self, reading: RuleSource) {
        self.0.set_reading(reading);
    }

    /// Takes `other`'s mark for every berth in place of its own, keeping
    /// its own reading (see [`kept_berths`](super::kept_berths)).
    pub fn take_marks_of(&mut self, other: Self) {
        self.0.take_words_of(other.0);
    }

    /// The reading new ships' marks are taken by.
    #[must_use]
    pub fn reading(&self) -> RuleSource {
        self.0.reading()
    }
}

impl Traffic {
    /// Gives each ship whose placement writes no escort-command mark its
    /// berth's last by `reading`
    /// ([`RuleKey::ReusedSlotMark`](crate::RuleKey::ReusedSlotMark)): by
    /// the engine, a never-used berth's false included; unmarked by the
    /// other reading.
    pub fn set_reused_slot_mark(&mut self, reading: RuleSource) {
        self.berth_marks.set_reading(reading);
    }

    /// The reading ships whose placement writes no mark take their berth's
    /// by (see [`Traffic::set_reused_slot_mark`]).
    #[must_use]
    pub fn reused_slot_mark(&self) -> RuleSource {
        self.berth_marks.reading()
    }

    /// Adds `npc` to the system at `berth`, or else the next berth, as
    /// [`Traffic::add_npc_at`] does but writing no escort-command mark
    /// (see the module docs): `_SpawnEscort` (@0x3fe8f) and
    /// `_DoShipCapture`'s spawn of the old ship (@0x4114d), both through
    /// `_GenericRandomShipSpawn`. Its spawn asks `_ShipCanScoop` (@0x3ff6b,
    /// @0x41255), and it holds its berth's mark by the reading.
    pub fn add_npc_keeping_mark(&mut self, npc: Npc, berth: Option<u32>) -> NpcId {
        let berth = berth.unwrap_or_else(|| self.next_berth());
        self.add_npc_scooped(npc, berth, true, false)
    }

    /// Marks the berth of NPC `id` as its ship takes a command
    /// (`_IssueNewEscortCommand` @0x66190): the word its berth's next ship
    /// may keep. Nothing for an NPC not in the system.
    pub fn mark_berth_of(&mut self, id: NpcId) {
        if let Some(berth) = self
            .npcs
            .iter()
            .find(|npc| npc.id == id)
            .map(|npc| npc.berth)
        {
            self.berth_marks.mark(berth);
        }
    }

    /// Writes `mark` as the escort-command word of the ship last in
    /// `berth`, for tests that set a berth's word up.
    ///
    /// Kept for nova-sim's berth-mark tests (`session::berth_marks`).
    #[cfg(any(test, feature = "fixture"))]
    pub fn write_berth_mark(&mut self, berth: u32, mark: bool) {
        self.berth_marks.0.write(berth, Some(mark));
    }

    /// The escort-command word the ship last in `berth` left, none for a
    /// berth no ship has written.
    ///
    /// Kept for nova-sim's berth-mark tests (`session::berth_marks`).
    #[cfg(any(test, feature = "fixture"))]
    #[must_use]
    pub fn berth_mark(&self, berth: u32) -> Option<bool> {
        self.berth_marks.left(berth)
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
    use nova_sim_base::fixture::{FAST, Ranged};

    #[test]
    fn a_never_used_berth_gives_a_keeping_placement_false_under_both_readings() {
        for reading in [RuleSource::Engine, RuleSource::Bible] {
            let mut kept = BerthMarks::default();
            kept.set_reading(reading);
            assert_eq!(kept.reading(), reading);
            assert!(!kept.bring_in(3, false), "{reading:?}");
            assert_eq!(kept.left(3), None, "{reading:?}: nothing written");
        }
    }

    #[test]
    fn a_keeping_placement_holds_the_berths_mark_by_the_engine_and_none_by_the_other_reading() {
        let mut kept = BerthMarks::default();
        assert_eq!(kept.reading(), RuleSource::Engine);
        kept.mark(1);
        assert!(kept.bring_in(1, false), "the berth's");
        kept.set_reading(RuleSource::Bible);
        assert!(!kept.bring_in(1, false), "unmarked");
    }

    #[test]
    fn a_keeping_placement_leaves_the_berths_word_as_it_was() {
        for reading in [RuleSource::Engine, RuleSource::Bible] {
            let mut kept = BerthMarks::default();
            kept.set_reading(reading);
            kept.mark(1);
            kept.bring_in(1, false);
            assert_eq!(kept.left(1), Some(true), "{reading:?}");
        }
    }

    #[test]
    fn a_clearing_placement_writes_false_and_holds_it_under_both_readings() {
        for reading in [RuleSource::Engine, RuleSource::Bible] {
            let mut kept = BerthMarks::default();
            kept.set_reading(reading);
            kept.mark(2);
            assert!(!kept.bring_in(2, true), "{reading:?}");
            assert_eq!(kept.left(2), Some(false), "{reading:?}");
            assert!(!kept.bring_in(2, false), "{reading:?}: cleared");
        }
    }

    #[test]
    fn taking_anothers_marks_keeps_this_ones_reading() {
        let mut given = BerthMarks::default();
        given.mark(1);
        let mut taker = BerthMarks::default();
        taker.mark(2);
        taker.set_reading(RuleSource::Bible);
        taker.take_marks_of(given);
        assert_eq!(taker.reading(), RuleSource::Bible);
        assert_eq!((taker.left(1), taker.left(2)), (Some(true), None));
        taker.set_reading(RuleSource::Engine);
        assert!(taker.bring_in(1, false));
    }

    /// The berth mark NPC `id` of `traffic` came in with.
    fn mark_of(traffic: &Traffic, id: NpcId) -> bool {
        traffic
            .npcs()
            .iter()
            .find(|npc| npc.id == id)
            .expect("in the system")
            .berth_mark
    }

    /// A default NPC, its own mark set to show the placement overwrites it.
    fn ship() -> Npc {
        Npc {
            berth_mark: true,
            ..npc(0, ShipStats::new(FAST, &[]))
        }
    }

    #[test]
    fn the_reading_is_set_on_the_traffic() {
        let mut traffic = Traffic::new();
        assert_eq!(traffic.reused_slot_mark(), RuleSource::Engine);
        traffic.set_reused_slot_mark(RuleSource::Bible);
        assert_eq!(traffic.reused_slot_mark(), RuleSource::Bible);
    }

    #[test]
    fn add_npc_and_add_npc_at_clear_the_berth_under_both_readings() {
        for reading in [RuleSource::Engine, RuleSource::Bible] {
            let mut traffic = Traffic::new();
            traffic.set_reused_slot_mark(reading);
            traffic.write_berth_mark(4, true);
            let id = traffic.add_npc_at(ship(), 4);
            assert!(!mark_of(&traffic, id), "{reading:?}");
            assert_eq!(traffic.berth_mark(4), Some(false), "{reading:?}");
            traffic.write_berth_mark(1, true);
            let id = traffic.add_npc(ship());
            assert_eq!(traffic.npcs().last().map(|npc| npc.berth), Some(1));
            assert!(!mark_of(&traffic, id), "{reading:?}");
            assert_eq!(traffic.berth_mark(1), Some(false), "{reading:?}");
        }
    }

    #[test]
    fn add_npc_keeping_mark_holds_its_berths_mark_by_the_engine_and_writes_none() {
        let mut traffic = Traffic::new();
        traffic.write_berth_mark(4, true);
        let id = traffic.add_npc_keeping_mark(ship(), Some(4));
        assert!(mark_of(&traffic, id), "the berth's");
        assert_eq!(traffic.berth_mark(4), Some(true));
        traffic.write_berth_mark(1, true);
        let id = traffic.add_npc_keeping_mark(ship(), None);
        assert_eq!(traffic.npcs().last().map(|npc| npc.berth), Some(1));
        assert!(mark_of(&traffic, id), "the next berth's");

        let mut fresh = Traffic::new();
        let id = fresh.add_npc_keeping_mark(ship(), Some(4));
        assert!(!mark_of(&fresh, id), "the zero fill");
        assert_eq!(fresh.berth_mark(4), None, "nothing written");
    }

    #[test]
    fn add_npc_keeping_mark_starts_unmarked_by_the_other_reading() {
        let mut traffic = Traffic::new();
        traffic.set_reused_slot_mark(RuleSource::Bible);
        traffic.write_berth_mark(4, true);
        let id = traffic.add_npc_keeping_mark(ship(), Some(4));
        assert!(!mark_of(&traffic, id));
        assert_eq!(traffic.berth_mark(4), Some(true), "left as it was");
    }

    #[test]
    fn add_npc_keeping_mark_asks_the_scoop_as_add_npc_at_does() {
        let mut traffic = Traffic::new();
        traffic.write_berth_scoop(4, false);
        let mut scooper = ship();
        scooper.can_scoop = true;
        traffic.add_npc_keeping_mark(scooper, Some(4));
        assert_eq!(traffic.berth_scoop(4), Some(true));
    }

    #[test]
    fn add_npc_unscooped_keeps_its_berths_mark() {
        let mut traffic = Traffic::new();
        traffic.write_berth_mark(1, true);
        let id = traffic.add_npc_unscooped(ship());
        assert!(mark_of(&traffic, id));
        assert_eq!(traffic.berth_mark(1), Some(true));
    }

    /// A ship of type 200 that `clears` the mark or not.
    fn new_ship(clears: bool) -> NewShip {
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
            rolls_voice: clears,
            asks_scoop: clears,
            clears_mark: clears,
        }
    }

    /// Brings `ship` into `traffic`, every berth below 8 marked, read by
    /// `reading`: its mark and its berth's word after.
    fn bring(reading: RuleSource, ship: NewShip) -> (bool, Option<bool>) {
        let mut traffic = Traffic::new();
        traffic.set_reused_slot_mark(reading);
        for berth in 0..8 {
            traffic.write_berth_mark(berth, true);
        }
        *traffic.table_mut() = SpawnTable {
            ships: BTreeMap::from([(ShipId(200), ShipKind::default())]),
            ..SpawnTable::default()
        };
        let came = traffic.bring_in_within(
            vec![ship],
            SPAWN_RESERVE,
            &Governments::empty(),
            &mut Ranged::of(&[]),
        );
        assert_eq!(came, 1);
        let npc = traffic.npcs().last().expect("brought in");
        (npc.berth_mark, traffic.berth_mark(npc.berth))
    }

    #[test]
    fn a_traffic_dude_ship_clears_its_berths_mark_under_both_readings() {
        for reading in [RuleSource::Engine, RuleSource::Bible] {
            assert_eq!(
                bring(reading, new_ship(true)),
                (false, Some(false)),
                "{reading:?}"
            );
        }
    }

    #[test]
    fn a_generic_ship_holds_its_berths_mark_by_the_engine_and_none_by_the_other_reading() {
        assert_eq!(
            bring(RuleSource::Engine, new_ship(false)),
            (true, Some(true))
        );
        assert_eq!(
            bring(RuleSource::Bible, new_ship(false)),
            (false, Some(true))
        );
    }

    #[test]
    fn marking_an_npcs_berth_writes_true_to_its_berth_alone() {
        let mut traffic = Traffic::new();
        let id = traffic.add_npc_at(ship(), 6);
        traffic.mark_berth_of(id);
        assert_eq!(traffic.berth_mark(6), Some(true));
        traffic.remove(id);
        assert_eq!(traffic.berth_mark(6), Some(true), "it outlives its ship");
        traffic.mark_berth_of(id);
        assert_eq!(traffic.berth_mark(6), Some(true), "a gone NPC: nothing");
        assert_eq!(traffic.berth_mark(1), None);
    }
}
