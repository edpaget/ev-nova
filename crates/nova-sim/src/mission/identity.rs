//! Whether a docked stellar is a mission's stop
//! (`_StellarsAreIdentical` @0xab35-0xac90).

use std::collections::BTreeMap;

use crate::catalog::{StellarId, StellarMark};
use crate::rulebook::{RuleKey, RuleSource, Rulebook};

/// The characters of a name the original's stellar record keeps:
/// `_LoadObjectData`'s `strncpy` of 0x1f bytes (@0x77310-0x7732a), less
/// the length byte.
const NAME_FIELD_CHARS: usize = 30;

/// The IDs the original's stellar record array holds: its 0x800 slots
/// from 128 (`_StellarsAreIdentical`'s bound @0xab51-0xab6c).
const RECORD_IDS: std::ops::RangeInclusive<i16> = 128..=2175;

/// The two Rulebook readings [`StellarMarks::identical`] follows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IdentityRules {
    /// [`MissionStellarIdentity`](RuleKey::MissionStellarIdentity):
    /// whether a place and name make two IDs one stop.
    pub place_and_name: RuleSource,
    /// [`StellarIdentityRecord`](RuleKey::StellarIdentityRecord): whether
    /// the stellars read as the original's records hold them.
    pub record: RuleSource,
}

impl IdentityRules {
    /// The readings `rules` gives.
    #[must_use]
    pub fn of(rules: &Rulebook) -> Self {
        Self {
            place_and_name: rules.source_for(RuleKey::MissionStellarIdentity),
            record: rules.source_for(RuleKey::StellarIdentityRecord),
        }
    }
}

/// The place and name of the zeroed record a stellar that didn't load
/// reads as by the engine: 0,0 and no name.
const ZEROED: ((i16, i16), &str) = ((0, 0), "");

/// Whether two names match by `record`: by the engine, the same full
/// length (the record's length byte, one byte a character in `MacRoman`)
/// and the same first [`NAME_FIELD_CHARS`] characters, as
/// `_StellarsAreIdentical` compares the 30 characters the record keeps
/// (@0xaba6-0xac74); by the other reading, the whole names.
fn names_match(a: &str, b: &str, record: RuleSource) -> bool {
    match record {
        RuleSource::Engine => {
            a.chars().count() == b.chars().count()
                && a.chars()
                    .take(NAME_FIELD_CHARS)
                    .eq(b.chars().take(NAME_FIELD_CHARS))
        }
        RuleSource::Bible => a == b,
    }
}

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

    /// Whether the stellar docked at, `docked`, is a mission's `stop`.
    ///
    /// By [`StellarIdentityRecord`](RuleKey::StellarIdentityRecord)'s
    /// engine reading, either ID outside 128..=2175 is identical to
    /// nothing, not even itself, as `_StellarsAreIdentical` checks its
    /// indices (@0xab51-0xab6c) before it compares them. Otherwise equal
    /// IDs always are (@0xab72-0xab75). By
    /// [`MissionStellarIdentity`](RuleKey::MissionStellarIdentity)'s
    /// engine reading, two IDs also are when both have the same place,
    /// the whole `xPos`/`yPos` word (@0xab98-0xaba0), and the same name
    /// (@0xaba6-0xac74), whatever their systems; by its other reading,
    /// only equal IDs are.
    ///
    /// How the names and an unmarked stellar compare is the record
    /// rule's. By its engine reading, a name counts as its full length
    /// and its first 30 characters, all the record keeps: the original
    /// reads a longer name on past the field, which isn't modelled, so
    /// two names of one length that share their first 30 characters are
    /// identical. And an unmarked stellar reads as a zeroed record, place
    /// 0,0 and no name, so it matches any unnamed stellar at 0,0, and two
    /// distinct unmarked IDs in the bound (129 and 130, say) are
    /// identical. By its other reading, names compare whole and an
    /// unmarked stellar is identical only to itself.
    #[must_use]
    pub fn identical(&self, stop: StellarId, docked: StellarId, rules: IdentityRules) -> bool {
        let by_record = rules.record == RuleSource::Engine;
        if by_record && !(RECORD_IDS.contains(&stop.0) && RECORD_IDS.contains(&docked.0)) {
            return false;
        }
        if stop == docked {
            return true;
        }
        if rules.place_and_name == RuleSource::Bible {
            return false;
        }
        let mark = |id: StellarId| match self.marks.get(&id) {
            Some(mark) => Some((mark.place, mark.name.as_str())),
            None if by_record => Some(ZEROED),
            None => None,
        };
        match (mark(stop), mark(docked)) {
            (Some((place, name)), Some((other_place, other_name))) => {
                place == other_place && names_match(name, other_name, rules.record)
            }
            _ => false,
        }
    }

    /// Every marked stellar other than `stellar` that is
    /// [`identical`](Self::identical) to it by `rules`, by ascending ID:
    /// the variants a stop at `stellar` is also reached at. None when only
    /// IDs count; an unmarked stellar's are those its zeroed record
    /// matches by the record's engine reading, none by its other.
    pub fn variants(
        &self,
        stellar: StellarId,
        rules: IdentityRules,
    ) -> impl Iterator<Item = StellarId> + '_ {
        self.marks
            .keys()
            .copied()
            .filter(move |&other| other != stellar && self.identical(stellar, other, rules))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rulebook::{RuleKey, Rulebook};

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

    /// Both readings the engine's.
    const ENGINE: IdentityRules = IdentityRules {
        place_and_name: RuleSource::Engine,
        record: RuleSource::Engine,
    };

    /// Places and names count, but the `spöb`s read as written.
    const RECORD_BIBLE: IdentityRules = IdentityRules {
        place_and_name: RuleSource::Engine,
        record: RuleSource::Bible,
    };

    /// Only IDs count, the record read as the engine's.
    const ID_ONLY: IdentityRules = IdentityRules {
        place_and_name: RuleSource::Bible,
        record: RuleSource::Engine,
    };

    fn identical(marks: &StellarMarks, a: i16, b: i16, rules: IdentityRules) -> bool {
        marks.identical(StellarId(a), StellarId(b), rules)
    }

    fn engine(marks: &StellarMarks, a: i16, b: i16) -> bool {
        identical(marks, a, b, ENGINE)
    }

    #[test]
    fn the_rules_are_read_from_their_two_keys() {
        assert_eq!(IdentityRules::of(&Rulebook::default()), ENGINE);
        let rules =
            Rulebook::default().with_override(RuleKey::StellarIdentityRecord, RuleSource::Bible);
        assert_eq!(IdentityRules::of(&rules), RECORD_BIBLE);
        let rules =
            Rulebook::default().with_override(RuleKey::MissionStellarIdentity, RuleSource::Bible);
        assert_eq!(IdentityRules::of(&rules), ID_ONLY);
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
        for rules in [ENGINE, RECORD_BIBLE] {
            assert!(!identical(&marks, 128, 129, rules));
            assert!(!identical(&marks, 128, 130, rules));
            assert!(!identical(&marks, 130, 128, rules));
        }
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
        assert!(identical(&marks, 465, 481, RECORD_BIBLE));
    }

    #[test]
    fn one_id_is_identical_to_itself_marked_or_not() {
        let marks = marks(&[mark(128, (0, 0), "Earth")]);
        for rules in [ENGINE, RECORD_BIBLE, ID_ONLY] {
            assert!(identical(&marks, 128, 128, rules));
            assert!(identical(&marks, 200, 200, rules));
        }
    }

    #[test]
    fn an_unmarked_stellar_matches_no_other_id_when_the_spobs_read_as_written() {
        let marks = marks(&[mark(128, (0, 0), "")]);
        assert!(!identical(&marks, 128, 129, RECORD_BIBLE));
        assert!(!identical(&marks, 129, 128, RECORD_BIBLE));
        assert!(!identical(&marks, 129, 130, RECORD_BIBLE));
    }

    /// Two unmarked IDs in the bound are both zeroed records by the engine,
    /// so they read as one place with one name.
    #[test]
    fn two_unmarked_stellars_are_identical_as_zeroed_records_by_the_engine() {
        let marks = marks(&[mark(128, (0, 0), "Earth")]);
        assert!(engine(&marks, 129, 130));
        assert!(!engine(&marks, 128, 129), "named");
        assert!(!engine(&marks, 129, 128), "named");
    }

    #[test]
    fn equal_ids_outside_the_index_bound_are_not_identical_by_the_engine() {
        let marks = marks(&[
            mark(127, (0, 0), "Below"),
            mark(2176, (0, 0), "Above"),
            mark(-1, (0, 0), "Negative"),
        ]);
        for id in [127, 2176, -1, i16::MIN, i16::MAX, 0] {
            assert!(!engine(&marks, id, id), "{id}");
        }
        for id in [128, 2175] {
            assert!(engine(&marks, id, id), "{id}");
        }
    }

    #[test]
    fn equal_ids_outside_the_bound_are_identical_by_the_records_other_reading() {
        let marks = marks(&[mark(2176, (0, 0), "Above")]);
        for id in [127, 2176, -1, i16::MIN, i16::MAX] {
            assert!(identical(&marks, id, id, RECORD_BIBLE), "{id}");
        }
    }

    #[test]
    fn the_bound_holds_when_only_ids_count() {
        let marks = marks(&[]);
        assert!(!identical(&marks, 2176, 2176, ID_ONLY));
        assert!(!identical(&marks, 127, 127, ID_ONLY));
        assert!(identical(&marks, 200, 200, ID_ONLY));
    }

    #[test]
    fn a_marked_stellar_out_of_bound_matches_no_variant_by_the_engine() {
        let marks = marks(&[
            mark(128, (5, 5), "Twin"),
            mark(2176, (5, 5), "Twin"),
            mark(127, (5, 5), "Twin"),
        ]);
        for (a, b) in [(128, 2176), (2176, 128), (128, 127), (127, 128)] {
            assert!(!engine(&marks, a, b), "{a} {b}");
            assert!(identical(&marks, a, b, RECORD_BIBLE), "{a} {b}");
        }
        assert_eq!(variants(&marks, 128, ENGINE), [0_i16; 0]);
        assert_eq!(variants(&marks, 128, RECORD_BIBLE), [127, 2176]);
    }

    #[test]
    fn names_over_30_characters_match_on_their_length_and_first_30_by_the_engine() {
        let first_30 = "abcdefghijklmnopqrstuvwxyz0123";
        let marks = marks(&[
            mark(128, (1, 2), &format!("{first_30}AAAAA")),
            mark(129, (1, 2), &format!("{first_30}BBBBB")),
        ]);
        assert!(engine(&marks, 128, 129));
        assert!(!identical(&marks, 128, 129, RECORD_BIBLE));
    }

    #[test]
    fn names_differing_within_their_first_30_characters_are_not_identical() {
        let marks = marks(&[
            mark(128, (1, 2), "abcdefghijklmnopqrstuvwxyz012Xtail"),
            mark(129, (1, 2), "abcdefghijklmnopqrstuvwxyz012Ytail"),
            mark(130, (1, 2), "Xbcdefghijklmnopqrstuvwxyz0123tail"),
        ]);
        assert!(!engine(&marks, 128, 129));
        assert!(!engine(&marks, 128, 130));
    }

    #[test]
    fn names_of_different_lengths_sharing_30_characters_are_not_identical() {
        let first_30 = "abcdefghijklmnopqrstuvwxyz0123";
        let marks = marks(&[
            mark(128, (1, 2), &format!("{first_30}A")),
            mark(129, (1, 2), &format!("{first_30}AB")),
            mark(130, (1, 2), first_30),
        ]);
        for rules in [ENGINE, RECORD_BIBLE] {
            assert!(!identical(&marks, 128, 129, rules));
            assert!(!identical(&marks, 128, 130, rules));
            assert!(!identical(&marks, 130, 128, rules));
        }
    }

    /// A name's length is its characters, one byte each in `MacRoman`, not
    /// its UTF-8 bytes.
    #[test]
    fn a_names_length_counts_its_characters() {
        let marks = marks(&[
            mark(128, (1, 2), "ééééééééééééééééééééééééééééééA"),
            mark(129, (1, 2), "ééééééééééééééééééééééééééééééB"),
        ]);
        assert!(engine(&marks, 128, 129));
    }

    #[test]
    fn a_missing_stop_matches_an_unnamed_stellar_at_0_0_by_the_engine() {
        let marks = marks(&[
            mark(129, (0, 0), ""),
            mark(130, (0, 0), "Earth"),
            mark(131, (1, 0), ""),
            mark(132, (0, 1), ""),
        ]);
        assert!(engine(&marks, 300, 129));
        assert!(engine(&marks, 129, 300));
        assert!(!engine(&marks, 300, 130));
        assert!(!engine(&marks, 300, 131));
        assert!(!engine(&marks, 300, 132));
        for other in [129, 130, 131, 132] {
            assert!(!identical(&marks, 300, other, RECORD_BIBLE), "{other}");
        }
        assert_eq!(variants(&marks, 300, ENGINE), [129]);
        assert_eq!(variants(&marks, 300, RECORD_BIBLE), [0_i16; 0]);
    }

    #[test]
    fn a_missing_stop_matches_nothing_when_only_ids_count() {
        let marks = marks(&[mark(129, (0, 0), "")]);
        assert!(!identical(&marks, 300, 129, ID_ONLY));
        assert_eq!(variants(&marks, 300, ID_ONLY), [0_i16; 0]);
    }

    #[test]
    fn a_missing_stop_out_of_bound_matches_nothing() {
        let marks = marks(&[mark(129, (0, 0), "")]);
        assert!(!engine(&marks, 3000, 129));
        assert!(!engine(&marks, 129, 3000));
        assert!(!engine(&marks, 127, 129));
    }

    fn variants(marks: &StellarMarks, stellar: i16, rules: IdentityRules) -> Vec<i16> {
        marks
            .variants(StellarId(stellar), rules)
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
        assert_eq!(variants(&marks, 227, ENGINE), [129, 448]);
        assert_eq!(variants(&marks, 130, ENGINE), [0_i16; 0]);
        assert_eq!(variants(&marks, 227, ID_ONLY), [0_i16; 0]);
        assert_eq!(
            variants(&marks, 999, ENGINE),
            [0_i16; 0],
            "unmarked, so a zeroed record no mark shares"
        );
    }

    #[test]
    fn by_the_bible_only_the_id_counts() {
        let marks = marks(&[mark(128, (0, 0), "Earth"), mark(426, (0, 0), "Earth")]);
        assert!(!identical(&marks, 128, 426, ID_ONLY));
        assert!(!identical(
            &marks,
            128,
            426,
            IdentityRules {
                place_and_name: RuleSource::Bible,
                record: RuleSource::Bible,
            }
        ));
    }
}
