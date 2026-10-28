//! [`FlightView`]'s map opened from a port screen on the map key
//! (`_keyPrefs` +0x12, [`FlightView::open_port_screen_map`]): the
//! spaceport, the exchange, the outfitter, the shipyard or the bar's hire
//! screen, or the ship description over either. All addresses are in
//! `EV Nova.app`'s `EV Nova` (i386).
//!
//! Each screen's filter turns the key into a pseudo item, and its dialog
//! loop calls `_DoSystemMap` for it, clearing the hypergate flag first:
//!
//! | Screen | Filter, item | `_DoSystemMap` call |
//! |---|---|---|
//! | The spaceport | `_PortFilter` @0x4f165, 3 | `_DoPortDialog` @0x5fd08 |
//! | The exchange | `_TradeFilter` @0x4e0e7, 2 | `_DoTradeDialog` @0x5dfff |
//! | The outfitter | `_OutfitFilter` @0x580f3, 2 | `_DoOutfitDialog` @0x5be99 |
//! | The shipyard, the hire screen | `_ShipyardFilter` @0x608ed, 3 | `_DoShipyardDialog` @0x5f28c |
//! | The ship description | `_ShipDescFilter` @0x51108, 4 | `_DoShipyardDescription` @0x5b8b0 |
//!
//! It is flight's course map over the spaceport, in
//! [`MapMode::Course`](crate::galaxy::MapMode::Course), unmarked
//! ([`MapMarks::NONE`](nova_sim::MapMarks::NONE)), and its close is the
//! docked [`FlightView::close_map`]: the whole bar redrawn, no cue, and
//! the selection the map opened with put back. Neither the opening nor
//! the close beeps: no caller's map branch plays a sound
//! (@0x5fcea-0x5fd58, @0x5dff1-0x5e057, @0x5be87-0x5bf4f,
//! @0x5f25b-0x5f342, @0x5b88e-0x5b937).
//!
//! Then the caller's own restore of +0x6a and +0x6c, with no nav redraw
//! (`_UpdateAllPortDialogs` and `_PortDialogUpdate` draw no status), so
//! the nav area shows the map's selection until the port's next nav
//! redraw ([`PortScreen`] says which restore each runs).
//!
//! Left out, record only: each caller's invalidations and dialog updates
//! after the map, as ours draws every frame whole; and the filters'
//! `_mapDialog` guards (@0x4f17a, @0x58103, @0x608fd, @0x51118), which
//! nothing can trip while the map's own filter runs.

use nova_sim::flight_frame::FlightCommand;

use super::FlightView;
use super::dialog_map::{MapOpened, MapOpening};
use crate::flight::sim_port::FlightSim;

/// The port screen that asks for the map.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PortScreen {
    /// The spaceport itself (`_DoPortDialog` item 3, @0x5fcea): the nav
    /// saved just before the map (@0x5fcf0-0x5fcfc) and written back after
    /// it (@0x5fd0d-0x5fd1b).
    Spaceport,
    /// The commodity exchange (`_DoTradeDialog` item 2, @0x5dff1): the nav
    /// written back from the save the exchange took as it opened
    /// (@0x5dbed-0x5dc08 / @0x5e004-0x5e01a), which stays saved.
    Exchange,
    /// The outfitter (`_DoOutfitDialog` item 2, @0x5be87): the nav written
    /// back from the save the outfitter took as it opened
    /// (@0x5bafc-0x5bb12 / @0x5be9e-0x5beba), which stays saved.
    Outfitter,
    /// The shipyard, or the bar's hire screen, the same function in hire
    /// mode (`_DoShipyardDialog` item 3, @0x5f25b): the nav saved just
    /// before the map (@0x5f266-0x5f27d) and written back after it
    /// (@0x5f291-0x5f2b3).
    Shipyard,
    /// The ship description, the shipyard's Info panel or the hire
    /// screen's (`_DoShipyardDescription` item 4, @0x5b88e): the nav saved
    /// just before the map (@0x5b898-0x5b8a4) and written back after it
    /// (@0x5b8b5-0x5b8c3).
    ShipDescription,
}

/// The command that restores a port screen's map after its close, for the
/// view to send: the exchange's and the outfitter's put back their
/// open-time save ([`FlightCommand::PutBackPortNav`],
/// [`Session::put_back_port_nav`](nova_sim::Session::put_back_port_nav)),
/// and the others the nav as the map `opened` with
/// ([`FlightCommand::RestoreMapNav`],
/// [`Session::restore_map_nav`](nova_sim::Session::restore_map_nav)), none
/// when it saved none.
pub(super) fn restore_command(screen: PortScreen, opened: &MapOpened) -> Option<FlightCommand> {
    match screen {
        PortScreen::Exchange | PortScreen::Outfitter => Some(FlightCommand::PutBackPortNav),
        PortScreen::Spaceport | PortScreen::Shipyard | PortScreen::ShipDescription => {
            opened.nav.map(FlightCommand::RestoreMapNav)
        }
    }
}

/// The port screens' maps, over the [`FlightSim`] port.
impl<C, S: FlightSim> FlightView<C, S> {
    /// Opens the course map from `screen` over the spaceport, as its
    /// filter's map key does: in
    /// [`MapMode::Course`](crate::galaxy::MapMode::Course), as each caller
    /// clears the hypergate flag first (@0x5fd00, @0x5dff7, @0x5be91,
    /// @0x5f284, @0x5b8a8), unmarked and silently. Its close is
    /// [`FlightView::close_map`]'s in port, then the caller's restore (see
    /// the module docs).
    pub fn open_port_screen_map(&mut self, screen: PortScreen) {
        self.open_map_as(MapOpening::Port(screen));
    }
}
