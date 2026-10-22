//! What a ship-table slot keeps after its ship is gone.
//!
//! The original never clears some of a slot's words when a new ship comes
//! into it, so the new ship starts with what the slot's last ship left: its
//! pick (+0x72, [`berth_picks`](super::berth_picks)) and its exit cursors
//! (+0xc8fe-+0xc905, [`berth_exits`](super::berth_exits)). [`BerthKept`]
//! keeps one such word for each berth (a
//! [`Npc::berth`](crate::traffic::npc::Npc::berth)), written as each ship's
//! changes, and gives a new ship its berth's by a rulebook reading: the
//! last ship's by the engine, none by the other reading.
//!
//! It lives as long as the [`Traffic`](crate::traffic::Traffic) holding
//! it: across systems, but not across a new traffic, where the
//! original's table keeps the picks and exit cursors until the app quits,
//! as it does the odds, search timers and persons the traffic hands on
//! ([`kept_berths`](super::kept_berths)). Whether to hand these on too is
//! the user's to decide: rdm task carry-slot-picks-and-exits-across-app-life.

use std::collections::BTreeMap;

use crate::rulebook::RuleSource;

/// The word each berth's last ship left, and the reading a new ship takes
/// it by (see the module docs).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BerthKept<T> {
    /// The word of the ship last in each berth, none for a berth whose
    /// ship left none.
    kept: BTreeMap<u32, T>,
    /// Whether a new ship takes its berth's word.
    reading: RuleSource,
}

impl<T> Default for BerthKept<T> {
    fn default() -> Self {
        Self {
            kept: BTreeMap::new(),
            reading: RuleSource::default(),
        }
    }
}

impl<T: Copy> BerthKept<T> {
    /// Writes `word` as the word of the ship in `berth`.
    pub fn write(&mut self, berth: u32, word: Option<T>) {
        match word {
            Some(word) => self.kept.insert(berth, word),
            None => self.kept.remove(&berth),
        };
    }

    /// The word the ship last in `berth` left, whatever the reading.
    #[must_use]
    pub fn left(&self, berth: u32) -> Option<T> {
        self.kept.get(&berth).copied()
    }

    /// The word a new ship brought into `berth` holds: the one its last
    /// ship left by the engine, none by the other reading.
    #[must_use]
    pub fn seed(&self, berth: u32) -> Option<T> {
        match self.reading {
            RuleSource::Engine => self.left(berth),
            RuleSource::Bible => None,
        }
    }

    /// Takes new ships' words by `reading`.
    pub fn set_reading(&mut self, reading: RuleSource) {
        self.reading = reading;
    }

    /// Takes `other`'s word for every berth in place of its own, keeping
    /// its own reading: the table handed on from another traffic.
    pub fn take_words_of(&mut self, other: Self) {
        self.kept = other.kept;
    }

    /// The reading new ships' words are taken by.
    #[must_use]
    pub fn reading(&self) -> RuleSource {
        self.reading
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_berths_word_reads_back_until_overwritten_with_none() {
        let mut kept = BerthKept::<u16>::default();
        assert_eq!(kept.left(1), None);
        kept.write(1, Some(131));
        assert_eq!(kept.left(1), Some(131));
        assert_eq!(kept.left(2), None);
        kept.write(1, Some(128));
        assert_eq!(kept.left(1), Some(128));
        kept.write(1, None);
        assert_eq!(kept.left(1), None);
    }

    #[test]
    fn a_new_ship_takes_its_berths_word_by_the_engine_and_none_by_the_other_reading() {
        let mut kept = BerthKept::<u16>::default();
        assert_eq!(kept.reading(), RuleSource::Engine);
        kept.write(1, Some(131));
        assert_eq!(kept.seed(1), Some(131));
        assert_eq!(kept.seed(2), None);
        kept.set_reading(RuleSource::Bible);
        assert_eq!(kept.reading(), RuleSource::Bible);
        assert_eq!(kept.seed(1), None);
        assert_eq!(kept.left(1), Some(131), "still kept");
    }

    #[test]
    fn taking_anothers_words_keeps_this_ones_reading() {
        let mut given = BerthKept::<u16>::default();
        given.write(1, Some(131));
        let mut taker = BerthKept::<u16>::default();
        taker.write(2, Some(128));
        taker.set_reading(RuleSource::Bible);
        taker.take_words_of(given);
        assert_eq!((taker.left(1), taker.left(2)), (Some(131), None));
        assert_eq!(taker.reading(), RuleSource::Bible);
    }
}
