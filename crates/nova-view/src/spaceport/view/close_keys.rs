//! The keys a close waits on at the spaceport: P for the player info
//! screen (`_DoPlayerInfoDialog` @0x5a6aa-0x5a6d4) and I for a mission
//! offer (`_OfferOneMission` @0xa35fc-0xa3614). All addresses are in `EV
//! Nova.app`'s `EV Nova` (i386).
//!
//! Each reads its key live, so whoever flies the ship says which are
//! physically down ([`SpaceportView::set_close_keys`]) before each input
//! and tick, as it says what it knows of the missions listed. A close made
//! with its key down waits for the release (the `ui::key_wait` module):
//! the player info screen stays open and inert, an answered offer stays
//! as it was and its venue takes nothing. What is typed during the player
//! info screen's wait goes back to whoever flies the ship once
//! ([`SpaceportView::take_wait_replay`]), since what reads it next may be
//! an offer over flight or the spaceport; an offer's goes to its venue.

use super::SpaceportView;
use crate::input::Input;
use crate::spaceport::offer::VenueOffers;

/// Which of the keys a close waits on are physically down.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CloseKeys {
    /// [`PLAYER_INFO_KEY`](crate::ui::player_info::PLAYER_INFO_KEY), P.
    pub player_info: bool,
    /// [`MISSION_INFO_KEY`](crate::ui::mission_info::MISSION_INFO_KEY), I.
    pub mission_info: bool,
}

impl SpaceportView {
    /// Says which of the keys a close waits on are down: the player info
    /// screen opens with P as it says, and an answered offer is disposed
    /// of with I as it says.
    pub fn set_close_keys(&mut self, keys: CloseKeys) {
        self.close_keys = keys;
    }

    /// Disposes of the offer dialog answered and left drawn under what its
    /// answer shows, at the spaceport or over the venue open: at once with
    /// I up, otherwise once I is let go (the module docs).
    pub(super) fn release_answered_offer(&mut self) {
        let held = self.close_keys.mission_info;
        self.each_offers(|offers| offers.release_answered(held));
    }

    /// Hides the offer dialog answered, at the spaceport or over the venue
    /// open: whoever flies the ship hides it once the refusal showed a
    /// `RefuseText` ([`RefuseShown`](nova_sim::RefuseShown),
    /// `_OfferOneMission` @0xa2e8e). It is disposed of as any answer is.
    pub fn hide_answered_offer(&mut self) {
        self.each_offers(VenueOffers::hide_answered);
    }

    /// What the player info screen's close wait left, once, for whoever
    /// flies the ship to replay to what reads input next.
    pub fn take_wait_replay(&mut self) -> Vec<Input> {
        std::mem::take(&mut self.wait_replay)
    }

    /// Drops the player info screen if it has closed, keeping what its
    /// wait left and asking for the status bar's refresh: whether it had.
    pub(super) fn drop_closed_player_info(&mut self) -> bool {
        let Some(screen) = &mut self.player_info else {
            return false;
        };
        if !screen.closed() {
            return false;
        }
        self.status_refresh |= screen.problem().is_none();
        self.wait_replay.extend(screen.take_replay());
        self.player_info = None;
        true
    }
}
