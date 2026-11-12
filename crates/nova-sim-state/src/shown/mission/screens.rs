//! What the mission screens show: the offer dialog ([`OfferPrompt`],
//! [`OfferLabels`], [`VenueOffer`]), the original's words, the venues an
//! offer comes from and the Mac's ticks the offers are timed in
//! ([`mac_ticks`]), from `nova_sim_story::mission::screens`, which
//! re-exports them and keeps the offers' order, labels and timing.

use std::time::Duration;

use nova_sim_catalog::catalog::{DescMovie, MissionId};
use nova_sim_catalog::warp::MAC_TICKS_PER_SECOND;

use super::active::{AcceptError, AcceptRefusal, SLOTS};

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

/// `STR#` 150 #35: the mission info screen's abort button.
pub const ABORT_LABEL: &str = "Abort";

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

/// What the mission info screen puts before a failed mission's name:
/// 0xA5 0x20 in Mac Roman (`_SetupMissionInfoList`).
pub const FAILED_MARK: &str = "\u{2022} ";

/// The offer dialog's buttons' labels.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OfferLabels {
    /// The accept button's.
    pub accept: String,
    /// The refuse button's: none when the offer cannot be refused, and it
    /// has one button alone.
    pub refuse: Option<String>,
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
    /// The movie its offer `dësc` names: the dialog plays it before it
    /// opens (`_OfferOneMission` @0xa2273-0xa22ab); one played
    /// [`after`](DescMovie::after) the session queues after the texts the
    /// answer shows (@0xa3580-0xa35b8; see
    /// `nova_sim::Session::take_mission_texts`).
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
/// (`nova_sim::Session::next_offer`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VenueOffer {
    /// The offer dialog asks.
    Prompt(OfferPrompt),
    /// A mission that cannot be refused, with no offer text, was accepted
    /// at once, with no dialog (@0xa22ec-0xa231c): how the accept went.
    /// Its offer's before-movie is queued ahead of the accept's texts
    /// (`nova_sim::Session::take_mission_texts`).
    Accepted {
        /// The mission.
        mission: MissionId,
        /// The slot it went into, or why it was not accepted.
        accepted: Result<u8, AcceptError>,
    },
}

/// The ticks (60 a second) after the bar opens before it first offers a
/// mission (`_DoBarDialog` @0x48f51-0x48f5f).
pub const FIRST_OFFER_TICKS: u32 = 15;

/// `ticks` of the Mac's clock as a duration.
#[must_use]
pub const fn mac_ticks(ticks: u32) -> Duration {
    Duration::from_nanos(ticks as u64 * 1_000_000_000 / MAC_TICKS_PER_SECOND as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_macs_ticks_are_60_a_second() {
        assert_eq!(mac_ticks(FIRST_OFFER_TICKS), Duration::from_millis(250));
        assert_eq!(mac_ticks(60), Duration::from_secs(1));
        assert_eq!(mac_ticks(1), Duration::from_nanos(16_666_666));
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

    #[test]
    fn the_bar_first_offers_15_ticks_after_it_opens() {
        assert_eq!(FIRST_OFFER_TICKS, 15);
    }

    #[test]
    fn with_no_offer_the_computer_says_the_originals_words() {
        assert_eq!(
            NO_MISSIONS_HERE, "There are no missions available here.",
            "STR# 2002 #353"
        );
    }

    #[test]
    fn the_headings_and_the_accept_and_abort_labels_are_the_originals() {
        assert_eq!(
            [NO_ACTIVE_MISSIONS, AVAILABLE_HERE, ACTIVE_MISSIONS],
            [
                "You have no active missions.",
                "The following missions are available here:",
                "Currently active missions:",
            ],
            "STR# 2002 #354, #359 and #360"
        );
        assert_eq!(
            [ACCEPT_LABEL, ABORT_LABEL],
            ["Accept", "Abort"],
            "STR# 150 #26 and #35"
        );
    }
}
