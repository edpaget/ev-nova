//! What the mission screens decide (see the [module docs](super)'s
//! "Screens"): the order offers show in, and the original's words.

use std::cmp::Reverse;

use super::{AcceptRefusal, SLOTS};
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
