//! What the mission screens decide (see the [module docs](super)'s
//! "Screens"): the order offers show in, and the original's words.

use std::cmp::Reverse;

use crate::chance::Chance;
use crate::rulebook::RuleSource;
pub use crate::warp::MAC_TICKS_PER_SECOND;
pub use nova_sim_state::shown::mission::screens::{
    ABORT_LABEL, ACCEPT_LABEL, ACTIVE_MISSIONS, AVAILABLE_HERE, BAR, FAILED_MARK,
    FIRST_OFFER_TICKS, NO_ACTIVE_MISSIONS, NO_HOLDS_TO_ACCEPT, NO_MISSIONS_HERE,
    NO_SPACE_TO_ACCEPT, OUTFITTER, OfferLabels, OfferPrompt, SHIPYARD, SLOTS_FULL_AFTER,
    SLOTS_FULL_BEFORE, SPACEPORT, TRADE_CENTER, VenueOffer, mac_ticks,
};

/// `STR#` 150 #27: a blank accept label, on a mission that cannot be
/// refused.
pub const OKAY_LABEL: &str = "Okay";
/// `STR#` 150 #50: a blank accept label, on a mission that can be
/// refused.
pub const YES_LABEL: &str = "Yes";
/// `STR#` 150 #51: a blank refuse label.
pub const NO_LABEL: &str = "No";

/// `Flags`: the mission is not shown in the mission info screen, which
/// does not open for it (`_SetupMissionInfoList` @0x9e04e-0x9e14e,
/// @0xa3e37-0xa3ec1).
pub const HIDDEN: u16 = 0x0400;

/// `Flags`: the offer cannot be refused (`_OfferOneMission`
/// @0xa21e2-0xa21f7).
pub const UNREFUSABLE: u16 = 0x0004;

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
        let mut chance = nova_sim_base::fixture::Draws::of(&[7, 29]);
        assert_eq!(offer_gap(&mut chance), 37);
        assert_eq!(offer_gap(&mut chance), 59);
        assert_eq!(chance.asked, [30, 30]);
    }

    #[test]
    fn the_originals_blank_labels() {
        assert_eq!(
            [OKAY_LABEL, YES_LABEL, NO_LABEL],
            ["Okay", "Yes", "No"],
            "STR# 150 #27, #50 and #51"
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

    #[test]
    fn an_offer_can_be_refused_only_with_a_refuse_button() {
        let prompt = |labels| OfferPrompt {
            mission: crate::catalog::MissionId(128),
            text: String::new(),
            labels,
            graphic: None,
            movie: None,
        };
        assert!(prompt(offer_labels("", "", true)).can_refuse());
        assert!(!prompt(offer_labels("", "", false)).can_refuse());
    }
}
