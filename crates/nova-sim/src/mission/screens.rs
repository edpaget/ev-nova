//! What the mission screens decide (see the [module docs](super)'s
//! "Screens"): the order offers show in, and the original's words.

use std::cmp::Reverse;

use super::{AcceptRefusal, SLOTS};
use crate::chance::Chance;
use crate::rulebook::RuleSource;

/// `STR#` 2002 #351: what the mission computer says before the slots'
/// count when every slot is taken.
pub const SLOTS_FULL_BEFORE: &str = "You're already on";
/// `STR#` 2002 #352: what it says after the count.
pub const SLOTS_FULL_AFTER: &str =
    "missions - you'll have to abort or finish one before you can accept another.";
/// `STR#` 2002 #353: what the mission computer says with no offer
/// available.
pub const NO_MISSIONS_HERE: &str = "There are no missions available here.";
/// `STR#` 2002 #354: what the in-flight mission info key says with no
/// mission to show (missions phase 17).
pub const NO_ACTIVE_MISSIONS: &str = "You have no active missions.";
/// `STR#` 2002 #359: the mission computer's heading.
pub const AVAILABLE_HERE: &str = "The following missions are available here:";
/// `STR#` 2002 #360: the mission info screen's heading.
pub const ACTIVE_MISSIONS: &str = "Currently active missions:";

/// `STR#` 150 #26: the mission computer's accept button.
pub const ACCEPT_LABEL: &str = "Accept";
/// `STR#` 150 #27: a blank accept label, on a mission that cannot be
/// refused.
pub const OKAY_LABEL: &str = "Okay";
/// `STR#` 150 #35: the mission info screen's abort button.
pub const ABORT_LABEL: &str = "Abort";
/// `STR#` 150 #50: a blank accept label, on a mission that can be
/// refused.
pub const YES_LABEL: &str = "Yes";
/// `STR#` 150 #51: a blank refuse label.
pub const NO_LABEL: &str = "No";

impl AcceptRefusal {
    /// What the original says of the refusal: with every slot taken,
    /// `STR#` 2002 #351, the slots' count and #352 (`_DoMissionDialog`
    /// @0xa380a-0xa38b8).
    #[must_use]
    pub fn message(self) -> String {
        match self {
            Self::SlotsFull => format!("{SLOTS_FULL_BEFORE} {SLOTS} {SLOTS_FULL_AFTER}"),
        }
    }
}

/// `Flags`: the offer cannot be refused (`_OfferOneMission`
/// @0xa21e2-0xa21f7).
pub const UNREFUSABLE: u16 = 0x0004;

/// The offer dialog's buttons' labels.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OfferLabels {
    /// The accept button's.
    pub accept: String,
    /// The refuse button's: none when the offer cannot be refused, and it
    /// has one button alone.
    pub refuse: Option<String>,
}

/// The offer dialog's labels from a mission's raw `AcceptButton` and
/// `RefuseButton` (`_OfferOneMission` @0xa289c-0xa2a14).
#[must_use]
pub fn offer_labels(accept: &str, refuse: &str, can_refuse: bool) -> OfferLabels {
    // Each is blanked unless its first character is a letter; a blank
    // accept reads "Yes" beside a refuse button, else "Okay"; a blank
    // refuse reads "No".
    let lettered = |label: &str| {
        label
            .chars()
            .next()
            .is_some_and(|first| first.is_ascii_alphabetic())
    };
    let accept = match accept {
        label if lettered(label) => label,
        _ if can_refuse => YES_LABEL,
        _ => OKAY_LABEL,
    };
    let refuse = can_refuse.then(|| if lettered(refuse) { refuse } else { NO_LABEL });
    OfferLabels {
        accept: accept.to_owned(),
        refuse: refuse.map(str::to_owned),
    }
}

/// The ticks (60 a second) after the bar opens before it first offers a
/// mission (`_DoBarDialog` @0x48f51-0x48f5f).
pub const FIRST_OFFER_TICKS: u32 = 15;
/// The fewest ticks between one offer and the next.
pub const OFFER_GAP_TICKS: u32 = 30;

/// The ticks before the next offer at a venue, drawing on `chance`:
/// [`OFFER_GAP_TICKS`] + `Rand(30)` (`_BarFilter` @0x47be0-0x47c0f,
/// `_OfferMissionFromPort` @0xa37a3-0xa37c0).
pub fn offer_gap(chance: &mut dyn Chance) -> u32 {
    OFFER_GAP_TICKS + chance.below(OFFER_GAP_TICKS)
}

/// `items`, each with its `DispWeight`, in list order, put in the order
/// they show, as [`RuleKey::MissionOrder`](crate::RuleKey::MissionOrder)
/// reads it. By the engine (`_RandomizeMissionData` @0xa1842-0xa197d),
/// those weighing 0 or more keep their order and come first, then the
/// rest by descending weight, ties in list order; by the Bible, all by
/// descending weight, ties in list order.
#[must_use]
pub fn display_order<T>(items: Vec<(T, i16)>, reading: RuleSource) -> Vec<T> {
    // The engine's first threshold is index -1's weight, the zero padding
    // before `_missionAvailData` (@0xa18bb): 0.
    let (mut shown, mut rest): (Vec<_>, Vec<_>) = match reading {
        RuleSource::Engine => items.into_iter().partition(|&(_, weight)| weight >= 0),
        RuleSource::Bible => (Vec::new(), items),
    };
    rest.sort_by_key(|&(_, weight)| Reverse(weight));
    shown.extend(rest);
    shown.into_iter().map(|(item, _)| item).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn labels(accept: &str, refuse: Option<&str>) -> OfferLabels {
        OfferLabels {
            accept: accept.to_owned(),
            refuse: refuse.map(str::to_owned),
        }
    }

    #[test]
    fn an_offers_labels_are_kept_when_each_starts_with_a_letter() {
        assert_eq!(
            offer_labels("I accept", "No thanks", true),
            labels("I accept", Some("No thanks"))
        );
        assert_eq!(
            offer_labels("z", "Q", true),
            labels("z", Some("Q")),
            "lower case too"
        );
    }

    #[test]
    fn a_blank_or_non_letter_label_reads_the_originals_words() {
        for blank in ["", " ok", "1st", "-"] {
            assert_eq!(
                offer_labels(blank, blank, true),
                labels(YES_LABEL, Some(NO_LABEL)),
                "{blank:?}"
            );
            assert_eq!(
                offer_labels(blank, "Never", false),
                labels(OKAY_LABEL, None),
                "{blank:?}"
            );
        }
    }

    #[test]
    fn an_offer_that_cannot_be_refused_has_no_refuse_button() {
        assert_eq!(offer_labels("Sure", "No way", false), labels("Sure", None));
        assert_eq!(UNREFUSABLE, 0x0004);
    }

    #[test]
    fn the_gap_between_offers_is_30_ticks_and_a_draw_below_30() {
        let mut chance = crate::testkit::Draws::of(&[7, 29]);
        assert_eq!(offer_gap(&mut chance), 37);
        assert_eq!(offer_gap(&mut chance), 59);
        assert_eq!(chance.asked, [30, 30]);
        assert_eq!(FIRST_OFFER_TICKS, 15);
    }

    #[test]
    fn the_originals_strings() {
        assert_eq!(
            [
                NO_MISSIONS_HERE,
                NO_ACTIVE_MISSIONS,
                AVAILABLE_HERE,
                ACTIVE_MISSIONS,
            ],
            [
                "There are no missions available here.",
                "You have no active missions.",
                "The following missions are available here:",
                "Currently active missions:",
            ],
            "STR# 2002 #353, #354, #359 and #360"
        );
        assert_eq!(
            [ACCEPT_LABEL, OKAY_LABEL, ABORT_LABEL, YES_LABEL, NO_LABEL],
            ["Accept", "Okay", "Abort", "Yes", "No"],
            "STR# 150 #26, #27, #35, #50 and #51"
        );
    }

    #[test]
    fn every_slot_taken_says_so_with_the_slots_counted() {
        assert_eq!(SLOTS, 16);
        assert_eq!(
            AcceptRefusal::SlotsFull.message(),
            "You're already on 16 missions - you'll have to abort or finish one before you can accept another."
        );
    }

    fn ordered(weights: &[i16], reading: RuleSource) -> Vec<usize> {
        let items = weights.iter().copied().enumerate().collect();
        display_order(items, reading)
    }

    #[test]
    fn by_the_engine_weights_of_0_or_more_keep_their_order_then_the_rest_descend() {
        assert_eq!(
            ordered(&[3, -1, 0, -5, -1, 7], RuleSource::Engine),
            [0, 2, 5, 1, 4, 3]
        );
    }

    #[test]
    fn by_the_bible_every_weight_descends_ties_in_list_order() {
        assert_eq!(
            ordered(&[3, -1, 0, -5, -1, 7], RuleSource::Bible),
            [5, 0, 2, 1, 4, 3]
        );
    }

    #[test]
    fn all_zero_keeps_list_order_under_both_readings() {
        for reading in [RuleSource::Engine, RuleSource::Bible] {
            assert_eq!(ordered(&[0, 0, 0], reading), [0, 1, 2], "{reading:?}");
        }
        assert_eq!(ordered(&[], RuleSource::Engine), Vec::<usize>::new());
    }
}
