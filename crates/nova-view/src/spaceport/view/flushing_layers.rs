//! How many layers are open over the port that flushed the event queue
//! as they opened, and how many texts are queued: what tells whoever
//! replays queued keys to the spaceport that a key opened something
//! (the router's replay of a map's wait, `FlightView::take_map_replay`).
//! All addresses are in `EV Nova.app`'s `EV Nova` (i386).
//!
//! `FlushEvents(0xff7f, 0)` drops every queued key, so a key that opens one
//! of these drops those behind it. It is called by the bar
//! (`_DoBarDialog` @0x4908a), News (`_DoNewsDialog` @0x4802b), the race
//! (`_DoRaceDialog` @0x48ad0), the outfitter (`_DoOutfitDialog` @0x5bd51),
//! the shipyard and the hire screen (`_DoShipyardDialog` @0x5e85a), the
//! ship description (`_DoShipyardDescription` @0x5b7e3), the name prompt
//! (`_EVTextInputDialog` @0x55429), the Yes/No dialog (`_EVYesNoDialog`
//! @0x55689), the player info (`_DoPlayerInfoDialog` @0x5a544), mission
//! info (`_DoMissionInfoDialog` @0x9e287), the mission computer
//! (`_DoMissionDialog` @0xa3a1c), an offer (`_OfferOneMission` @0xa2caa)
//! and every text (`_ShowTextDialog` @0x5ae67), the outfitter's alert
//! among them.
//!
//! The exchange and the quantity dialog don't: `_DoTradeDialog`
//! (@0x5dbe1) and `_DoQuantityDialog` (@0x56d5b) never call
//! `FlushEvents`, so a key queued behind the one that opens either reaches
//! it. They count for nothing here.
//!
//! The texts are counted apart ([`SpaceportView::text_stages`]): closing
//! one shows the next, which flushes, with fewer left queued.

use super::{Open, SpaceportView};
use crate::spaceport::bar::BarScreen;
use crate::spaceport::hire::HireScreen;
use crate::ui::mission_dialogs::MissionDialogs;

impl Open {
    /// How many of its layers flushed as they opened (see the module
    /// docs), its offer's left to [`SpaceportView::offer_dialog`].
    fn flushing_layers(&self) -> usize {
        match self {
            Self::Service(_) | Self::Computer(_) => 1,
            Self::Trade(_) => 0,
            Self::Outfitter(screen) => 1 + usize::from(screen.alert_dialog().is_some()),
            Self::Shipyard(screen) => {
                1 + usize::from(screen.info_panel().is_some())
                    + usize::from(screen.naming().is_some())
            }
            Self::Bar(screen) => 1 + bar_layers(screen),
        }
    }
}

/// What is open over `bar` that flushed as it opened.
fn bar_layers(bar: &BarScreen) -> usize {
    usize::from(bar.news_screen().is_some())
        + usize::from(bar.race_screen().is_some())
        + bar.hire_screen().map_or(0, |hire| {
            1 + usize::from(HireScreen::info_panel(hire).is_some())
        })
}

impl SpaceportView {
    /// How many layers are open over the port that flushed the event
    /// queue as they opened (see the module docs), the texts left to
    /// [`SpaceportView::text_stages`]. A rise says something opened
    /// through a flush; a fall that something closed, which flushes
    /// nothing.
    #[must_use]
    pub fn flushing_layers(&self) -> usize {
        let open = self.open.as_ref().map_or(0, Open::flushing_layers);
        let info = self.mission_info().is_some();
        let offer = self.offer_dialog().is_some();
        let player_info = self
            .player_info
            .as_ref()
            .map_or(0, |screen| 1 + usize::from(screen.confirm_open()));
        open + usize::from(info) + usize::from(offer) + player_info
    }

    /// How many texts are shown or queued
    /// ([`MissionDialogs::text_stages`](MissionDialogs::text_stages)):
    /// each flushes as it comes to the front.
    #[must_use]
    pub fn text_stages(&self) -> usize {
        self.dialogs.as_ref().map_or(0, MissionDialogs::text_stages)
    }
}
