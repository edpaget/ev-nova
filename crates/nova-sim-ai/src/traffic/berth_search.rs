//! The search timer (+0xc90c) each ship-table slot keeps after its ship
//! is gone.
//!
//! Only `_HighLevelAIHandler`'s lost-sight block touches +0xc90c (@0x8d81a,
//! @0x8d852, @0x8d876, @0x8d89c, @0x8d8ae; see
//! [`lost_sight`](crate::ai::lost_sight)): no spawn (`_RandomShipSpawn`
//! @0x3c0f3, `_RespawnEscort`, `_LaunchFighter`, …), capture or system
//! change writes it, and `_InitObjects` doesn't clear it. The table is a
//! `NewPtr(0x325000)` (`_InitArrays` @0x1b7a1-0x1b7b5) that only the
//! system's zero fill sets, so a never-used slot's timer reads 0.0. A new
//! ship brought into a slot (a
//! [`Npc::berth`](crate::traffic::npc::Npc::berth)) takes the timer the
//! slot's last ship left: -1 ([`SearchTimer::Unset`]) when that ship last
//! saw its target or gave up, a count when it broke off a search, which
//! the new ship counts down from with no roll; and a never-used slot's 0,
//! which drops a lost target on the first frame out of sight (@0x8d830,
//! @0x8d85f-0x8d8a6).
//!
//! [`BerthSearch`], over a [`BerthKept`], keeps that timer for each berth,
//! written as each ship's changes ([`Traffic`]'s goals), and gives a new
//! ship its berth's by the reading of
//! [`RuleKey::ReusedSlotSearch`](crate::RuleKey::ReusedSlotSearch): the
//! last ship's, or a never-used berth's 0, by the engine; none by the
//! other reading. A search under way reaches the new ship as a count
//! carried ([`SearchTimer::carried`]): the new ship isn't searching.
//!
//! It lasts as long as the app, as the original keeps a slot's timer
//! through a pilot load or a new game until the app quits: across
//! systems, and handed on from each [`Traffic`] to the next, across a
//! traffic Reset and a new session, each new traffic keeping its own
//! reading (see [`kept_berths`](super::kept_berths)).

use crate::ai::lost_sight::SearchTimer;
use crate::rulebook::RuleSource;

use super::Traffic;
use super::berth_kept::BerthKept;

/// The search timer each berth's last ship left, and the reading a new
/// ship takes it by (see the module docs).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BerthSearch(BerthKept<SearchTimer>);

impl BerthSearch {
    /// Writes `timer` as the timer of the ship in `berth`.
    pub fn write(&mut self, berth: u32, timer: SearchTimer) {
        self.0.write(berth, Some(timer));
    }

    /// The timer a new ship brought into `berth` holds: by the engine, the
    /// one its last ship left, carried, or a never-used berth's 0; none by
    /// the other reading.
    #[must_use]
    pub fn seed(&self, berth: u32) -> SearchTimer {
        match self.0.reading() {
            RuleSource::Engine => self
                .0
                .left(berth)
                .map_or(SearchTimer::Carried(0), SearchTimer::carried),
            RuleSource::Bible => SearchTimer::Unset,
        }
    }

    /// Takes new ships' timers by `reading`.
    pub fn set_reading(&mut self, reading: RuleSource) {
        self.0.set_reading(reading);
    }

    /// Takes `other`'s timer for every berth in place of its own, keeping
    /// its own reading (see [`kept_berths`](super::kept_berths)).
    pub fn take_timers_of(&mut self, other: Self) {
        self.0.take_words_of(other.0);
    }

    /// The reading new ships' timers are taken by.
    #[must_use]
    pub fn reading(&self) -> RuleSource {
        self.0.reading()
    }
}

impl Traffic {
    /// Gives each new ship its berth's last search timer by `reading`
    /// ([`RuleKey::ReusedSlotSearch`](crate::RuleKey::ReusedSlotSearch)):
    /// by the engine, as no spawn writes +0xc90c, a never-used berth's 0
    /// included; none by the other reading.
    pub fn set_reused_slot_search(&mut self, reading: RuleSource) {
        self.berth_search.set_reading(reading);
    }

    /// The reading new ships take their berth's last search timer by (see
    /// [`Traffic::set_reused_slot_search`]).
    #[must_use]
    pub fn reused_slot_search(&self) -> RuleSource {
        self.berth_search.reading()
    }

    /// Writes `timer` as the search timer of the ship last in `berth`, for
    /// tests that set a berth's timer up.
    ///
    /// Kept for nova-sim's kept-berths test (`session::kept_berths`).
    #[cfg(any(test, feature = "fixture"))]
    pub fn write_berth_search(&mut self, berth: u32, timer: SearchTimer) {
        self.berth_search.write(berth, timer);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_never_used_berth_seeds_a_zero_timer_by_the_engine_and_none_by_the_other_reading() {
        let mut kept = BerthSearch::default();
        assert_eq!(kept.reading(), RuleSource::Engine);
        assert_eq!(kept.seed(3), SearchTimer::Carried(0));
        kept.set_reading(RuleSource::Bible);
        assert_eq!(kept.reading(), RuleSource::Bible);
        assert_eq!(kept.seed(3), SearchTimer::Unset);
    }

    #[test]
    fn a_berth_seeds_its_last_ships_timer_as_carried() {
        let mut kept = BerthSearch::default();
        kept.write(1, SearchTimer::Searching(40));
        assert_eq!(kept.seed(1), SearchTimer::Carried(40));
        kept.write(1, SearchTimer::Unset);
        assert_eq!(kept.seed(1), SearchTimer::Unset, "a written -1");
        kept.write(2, SearchTimer::Carried(7));
        assert_eq!(kept.seed(2), SearchTimer::Carried(7));
        kept.set_reading(RuleSource::Bible);
        assert_eq!(kept.seed(2), SearchTimer::Unset);
    }
}
