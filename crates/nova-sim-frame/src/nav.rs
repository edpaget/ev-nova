//! The nav a mission dialog saves before the map it opens and writes
//! back after it ([`SavedNav`]), with the Con slot noted for a mode-3
//! selection ([`NavSlot`]), from `nova_sim::session::nav`, which
//! re-exports them and keeps the nav's setters and readers.

use nova_sim_catalog::catalog::SystemId;
use nova_sim_state::shown::navigation::NavMode;

/// The Con slot a mode-3 selection was written from, as the original
/// keeps it at +0x6c where ours keeps the system. A setter that knows the
/// slot notes it (`nova_sim::Session::select_next_system`,
/// `nova_sim::Session::cycle_map_selection`, `nova_sim::Session::cue_course`); it stands while the player is still in `system` and the raw selection
/// is still `selected`, and a later cycle steps on from it. Every other
/// write of the nav forgets it (`nova_sim::session::Chart::set_nav`).
///
/// Its fields are `pub` for `nova_sim::session`'s `Chart`, which notes
/// the slot and steps on from it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NavSlot {
    /// The system the slot is one of.
    pub system: SystemId,
    /// The Con slot, from 0.
    pub slot: usize,
    /// The raw selection the slot was written with.
    pub selected: SystemId,
}

/// The nav mode and its selection a mission dialog saves before the map
/// it opens and writes back after it (+0x6a and +0x6c: the offer
/// @0xa30eb / @0xa3108, the mission computer @0xa3d54 / @0xa3d86, the
/// text dialog @0x5b14c / @0x5b169; mission info saves them @0x9e442 and
/// never writes them back). Ours keeps the raw selection and the Con slot
/// noted for it ([`NavSlot`]), which together are the original's +0x6c.
/// Taken with `nova_sim::Session::map_nav`, put back with
/// `nova_sim::Session::restore_map_nav`.
///
/// Its fields are `pub` for `nova_sim::session`'s `Chart`, which takes
/// and puts back the nav.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SavedNav {
    /// The nav mode and its raw selection.
    pub nav: NavMode,
    /// The Con slot noted for the selection, if any.
    pub slot: Option<NavSlot>,
}

impl SavedNav {
    /// `nav` saved with no Con slot noted: for another crate's test that
    /// needs a saved nav without a session to take it from.
    #[cfg(any(test, feature = "fixture"))]
    #[must_use]
    pub fn of(nav: NavMode) -> Self {
        Self { nav, slot: None }
    }
}
