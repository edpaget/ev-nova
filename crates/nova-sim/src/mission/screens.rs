//! What the mission screens decide (see the [module docs](super)'s
//! "Screens"): the order offers show in, and the original's words.

use std::cmp::Reverse;
use std::time::Duration;

use super::{AcceptError, AcceptRefusal, SLOTS};
use crate::catalog::{DescMovie, MissionId};
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
/// `STR#` 2002 #354: what the in-flight mission info key says on the
/// flight message line with no mission to show (`_HandlePlayer`
/// @0x7055c-0x705a8).
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

/// `STR#` 2002 #355: an accept refused for want of cargo holds.
pub const NO_HOLDS_TO_ACCEPT: &str =
    "Your ship doesn't have enough cargo space to accept this mission.";
/// `STR#` 2002 #356: an accept refused for want of free cargo space.
pub const NO_SPACE_TO_ACCEPT: &str = "Your ship doesn't have enough free cargo space to accept this mission. Sell or jettison some and try again.";

impl AcceptRefusal {
    /// What the original says of the refusal: with every slot taken,
    /// `STR#` 2002 #351, the slots' count and #352 (`_DoMissionDialog`
    /// @0xa380a-0xa38b8); without the cargo holds #355, and without the
    /// free cargo space #356 (`_DoMissionAccept` @0xa1b61-0xa1be5).
    #[must_use]
    pub fn message(self) -> String {
        match self {
            Self::SlotsFull => format!("{SLOTS_FULL_BEFORE} {SLOTS} {SLOTS_FULL_AFTER}"),
            Self::NoHolds => NO_HOLDS_TO_ACCEPT.to_owned(),
            Self::NoSpace => NO_SPACE_TO_ACCEPT.to_owned(),
        }
    }
}

/// `AvailLoc`: the bar.
pub const BAR: i16 = 1;
/// `AvailLoc`: the spaceport, where entering keeps the offers suppressed
/// (`_OfferMissionFromPort` @0xa36b8). It offers once per landing, as
/// `_DoPortDialog` opens (@0x5fc30-0x5fc37); its timer item's call
/// (@0x6016f-0x60176) never runs.
pub const SPACEPORT: i16 = 3;
/// `AvailLoc`: the trade center, which offers as it opens and on its
/// timer (`_DoTradeDialog` @0x5df78-0x5df7f, @0x5e5de-0x5e5e5).
pub const TRADE_CENTER: i16 = 4;
/// `AvailLoc`: the shipyard, which offers as it opens and on its timer
/// while buying ships, not hiring escorts (`_DoShipyardDialog`
/// @0x5e8e4-0x5e8eb, @0x5f3f5-0x5f3fc).
pub const SHIPYARD: i16 = 5;
/// `AvailLoc`: the outfitter, which offers as it opens and on its timer
/// (`_DoOutfitDialog` @0x5bd8b-0x5bd92, @0x5c01e-0x5c025).
pub const OUTFITTER: i16 = 6;

/// `Flags`: the mission is not shown in the mission info screen, which
/// does not open for it (`_SetupMissionInfoList` @0x9e04e-0x9e14e,
/// @0xa3e37-0xa3ec1).
pub const HIDDEN: u16 = 0x0400;

/// What the mission info screen puts before a failed mission's name:
/// 0xA5 0x20 in Mac Roman (`_SetupMissionInfoList`).
pub const FAILED_MARK: &str = "\u{2022} ";

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

/// A mission offered outside the mission computer, as the offer dialog
/// shows it (`_OfferOneMission` @0xa21ac).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OfferPrompt {
    /// The mission.
    pub mission: MissionId,
    /// Its offer text, its tags filled for the offer.
    pub text: String,
    /// Its buttons' labels: no refuse button when it cannot be refused.
    pub labels: OfferLabels,
    /// Its offer `dësc`'s `Graphic`, raw: none for -1 or a missing
    /// `dësc`. The view shows the picture variant of the dialog (`DLOG`
    /// 1020) for one above 127 (@0xa2344).
    pub graphic: Option<i16>,
    /// The movie its offer `dësc` names: played before the dialog, or,
    /// [`after`](DescMovie::after), after the texts its answer shows
    /// (`_OfferOneMission` @0xa2273-0xa22ab, @0xa3580-0xa35b8).
    pub movie: Option<DescMovie>,
}

impl OfferPrompt {
    /// Whether the offer can be refused: `Flags` 0x0004 is clear.
    #[must_use]
    pub fn can_refuse(&self) -> bool {
        self.labels.refuse.is_some()
    }
}

/// What a venue's next offer comes to
/// ([`Session::next_offer`](crate::Session::next_offer)).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VenueOffer {
    /// The offer dialog asks.
    Prompt(OfferPrompt),
    /// A mission that cannot be refused, with no offer text, was accepted
    /// at once, with no dialog (@0xa22ec-0xa231c): how the accept went.
    Accepted {
        /// The mission.
        mission: MissionId,
        /// The slot it went into, or why it was not accepted.
        accepted: Result<u8, AcceptError>,
        /// The movie its offer `dësc` names, when it plays before the
        /// dialog: it plays before the accept's texts, as the before-movie
        /// comes before the empty-text check (@0xa2273-0xa22ec). The accept
        /// at once returns before the after-movie (@0xa230a-0xa231c).
        movie: Option<DescMovie>,
    },
}

/// The ticks (60 a second) after the bar opens before it first offers a
/// mission (`_DoBarDialog` @0x48f51-0x48f5f).
pub const FIRST_OFFER_TICKS: u32 = 15;
/// The fewest ticks between one offer and the next.
pub const OFFER_GAP_TICKS: u32 = 30;

/// The Mac's ticks a second (`TickCount`), which the offers' times are
/// counted in.
pub const MAC_TICKS_PER_SECOND: u32 = 60;

/// `ticks` of the Mac's clock as a duration.
#[must_use]
pub const fn mac_ticks(ticks: u32) -> Duration {
    Duration::from_nanos(ticks as u64 * 1_000_000_000 / MAC_TICKS_PER_SECOND as u64)
}

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
    fn the_macs_ticks_are_60_a_second() {
        assert_eq!(mac_ticks(FIRST_OFFER_TICKS), Duration::from_millis(250));
        assert_eq!(mac_ticks(60), Duration::from_secs(1));
        assert_eq!(mac_ticks(1), Duration::from_nanos(16_666_666));
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

    #[test]
    fn a_cargo_refusal_says_whether_the_holds_or_the_free_space_are_short() {
        assert_eq!(
            AcceptRefusal::NoHolds.message(),
            "Your ship doesn't have enough cargo space to accept this mission."
        );
        assert_eq!(
            AcceptRefusal::NoSpace.message(),
            "Your ship doesn't have enough free cargo space to accept this mission. Sell or jettison some and try again."
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
