//! The course map the spaceport asks for
//! ([`SpaceportView::take_map_request`](super::SpaceportView::take_map_request)):
//! the bar's, on its map key (`_BarFilter` item 8, @0x47a6e-0x47a9b), or a
//! mission dialog's, on its own (an offer over any venue, the mission
//! computer, mission info, or the accept's briefing text). Whoever flies
//! the ship opens it over the spaceport, which lets go of its pointer and
//! keys, and the screen that asked stays open under it.

use crate::flight::MapOpener;

/// Which course map the spaceport asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PortMap {
    /// The bar's
    /// ([`FlightView::open_bar_map`](crate::flight::FlightView::open_bar_map)),
    /// whose close puts back the nav the bar opened with.
    Bar,
    /// A mission dialog's
    /// ([`FlightView::open_dialog_map`](crate::flight::FlightView::open_dialog_map)),
    /// marked and restored as that dialog does.
    Dialog(MapOpener),
}
