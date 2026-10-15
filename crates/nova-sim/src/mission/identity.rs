//! Whether a docked stellar is a mission's stop
//! (`_StellarsAreIdentical` @0xab35-0xac90).

use std::collections::BTreeMap;

use crate::catalog::{StellarId, StellarMark};
use crate::rulebook::RuleSource;

/// Every stellar's [`StellarMark`], by ID: what the original compares to
/// count two stellars as one mission stop.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StellarMarks {
    marks: BTreeMap<StellarId, StellarMark>,
}

impl StellarMarks {
    /// The marks of `marks`, by ID.
    #[must_use]
    pub fn new(marks: Vec<StellarMark>) -> Self {
        Self {
            marks: marks.into_iter().map(|mark| (mark.id, mark)).collect(),
        }
    }

    /// Whether the stellar docked at, `docked`, is a mission's `stop`
    /// (the [`MissionStellarIdentity`](crate::RuleKey::MissionStellarIdentity)
    /// rule). Equal IDs always are (@0xab72-0xab75). By the engine, two
    /// IDs also are when both are marked with the same place, the whole
    /// `xPos`/`yPos` word (@0xab98-0xaba0), and exactly the same name
    /// (@0xaba6-0xac74), whatever their systems; by the Bible, only equal
    /// IDs are. An unmarked stellar is identical only to itself.
    #[must_use]
    pub fn identical(&self, stop: StellarId, docked: StellarId, source: RuleSource) -> bool {
        if stop == docked {
            return true;
        }
        if source == RuleSource::Bible {
            return false;
        }
        match (self.marks.get(&stop), self.marks.get(&docked)) {
            (Some(stop), Some(docked)) => stop.place == docked.place && stop.name == docked.name,
            _ => false,
        }
    }

    /// Every marked stellar other than `stellar` that is
    /// [`identical`](Self::identical) to it by `source`, by ascending ID:
    /// the variants a stop at `stellar` is also reached at. None by the
    /// Bible, and none for an unmarked stellar.
    pub fn variants(
        &self,
        stellar: StellarId,
        source: RuleSource,
    ) -> impl Iterator<Item = StellarId> + '_ {
        self.marks
            .keys()
            .copied()
            .filter(move |&other| other != stellar && self.identical(stellar, other, source))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mark(id: i16, place: (i16, i16), name: &str) -> StellarMark {
        StellarMark {
            id: StellarId(id),
            place,
            name: name.to_owned(),
        }
    }

    fn marks(list: &[StellarMark]) -> StellarMarks {
        StellarMarks::new(list.to_vec())
    }

    fn engine(marks: &StellarMarks, a: i16, b: i16) -> bool {
        marks.identical(StellarId(a), StellarId(b), RuleSource::Engine)
    }

    #[test]
    fn two_ids_at_one_place_with_one_name_are_identical_by_the_engine() {
        let marks = marks(&[
            mark(227, (0, 0), "Tre'ar Zalom"),
            mark(448, (0, 0), "Tre'ar Zalom"),
        ]);
        assert!(engine(&marks, 227, 448));
        assert!(engine(&marks, 448, 227));
    }

    #[test]
    fn a_different_x_or_y_is_not_identical() {
        let marks = marks(&[
            mark(128, (100, 200), "Earth"),
            mark(129, (101, 200), "Earth"),
            mark(130, (100, 201), "Earth"),
        ]);
        assert!(!engine(&marks, 128, 129));
        assert!(!engine(&marks, 128, 130));
    }

    #[test]
    fn a_different_name_is_not_identical() {
        let marks = marks(&[
            mark(128, (0, 0), "Earth"),
            mark(129, (0, 0), "earth"),
            mark(130, (0, 0), "Earth II"),
        ]);
        assert!(!engine(&marks, 128, 129));
        assert!(!engine(&marks, 128, 130));
        assert!(!engine(&marks, 130, 128));
    }

    /// A mark carries no system: the original compares only the place in
    /// the system and the name, so two unrelated wormholes match (465 in
    /// Sol and 481 in system 414, both at 10000,0).
    #[test]
    fn stellars_of_different_systems_match_since_marks_carry_no_system() {
        let marks = marks(&[
            mark(465, (10000, 0), "Wormhole"),
            mark(481, (10000, 0), "Wormhole"),
        ]);
        assert!(engine(&marks, 465, 481));
    }

    #[test]
    fn one_id_is_identical_to_itself_marked_or_not() {
        let marks = marks(&[mark(128, (0, 0), "Earth")]);
        for source in [RuleSource::Engine, RuleSource::Bible] {
            assert!(marks.identical(StellarId(128), StellarId(128), source));
            assert!(marks.identical(StellarId(200), StellarId(200), source));
        }
    }

    #[test]
    fn an_unmarked_stellar_matches_no_other_id() {
        let marks = marks(&[mark(128, (0, 0), "")]);
        assert!(!engine(&marks, 128, 129));
        assert!(!engine(&marks, 129, 128));
        assert!(!engine(&marks, 129, 130));
    }

    fn variants(marks: &StellarMarks, stellar: i16, source: RuleSource) -> Vec<i16> {
        marks
            .variants(StellarId(stellar), source)
            .map(|id| id.0)
            .collect()
    }

    #[test]
    fn the_variants_are_the_other_stellars_at_one_place_with_one_name_ascending() {
        let marks = marks(&[
            mark(448, (0, 0), "Tre'ar Zalom"),
            mark(130, (0, 0), "Other"),
            mark(227, (0, 0), "Tre'ar Zalom"),
            mark(129, (0, 0), "Tre'ar Zalom"),
            mark(131, (1, 0), "Tre'ar Zalom"),
        ]);
        assert_eq!(variants(&marks, 227, RuleSource::Engine), [129, 448]);
        assert_eq!(variants(&marks, 130, RuleSource::Engine), [0_i16; 0]);
        assert_eq!(variants(&marks, 227, RuleSource::Bible), [0_i16; 0]);
        assert_eq!(
            variants(&marks, 999, RuleSource::Engine),
            [0_i16; 0],
            "unmarked"
        );
    }

    #[test]
    fn by_the_bible_only_the_id_counts() {
        let marks = marks(&[mark(128, (0, 0), "Earth"), mark(426, (0, 0), "Earth")]);
        assert!(!marks.identical(StellarId(128), StellarId(426), RuleSource::Bible));
    }
}
