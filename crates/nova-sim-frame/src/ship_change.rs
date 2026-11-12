//! A ship change's interface load, as the status bar sees it
//! ([`InterfaceLoad`]), from `nova_sim::session::ship_change`, which
//! re-exports it and keeps the ship-change set operators.

use nova_sim_base::reserves::Reserves;
use nova_sim_catalog::catalog::GovtId;
use nova_sim_state::shown::navigation::NavMode;

/// A `C`, `E` or `H` run while docked, as the status bar sees it: the
/// original's `_LoadIntfResource` call at the operator (@0x156c7, after
/// the armour bump @0x15677-0x156c5). It asks for the `ïntf` of
/// `government` (as `nova_sim::Session::interface_government` reads it), and
/// when that is not the one loaded (`_currentIntf`, @0x73663-0x73672) it
/// loads it and redraws the whole bar (`_RedrawStatus(1)`
/// @0x739b6-0x739bd), drawing `nav` and `reserves` as they stood at the
/// operator. Noted by `Session::change_ship`, taken by
/// `nova_sim::Session::take_interface_loads`, with how many mission texts had been
/// queued by then, so that a view can order the redraws against the texts
/// a script queues around them ([`TextBar`](crate::mission_screens::TextBar)).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InterfaceLoad {
    /// The government whose `Interface` the operator's load asks for.
    pub government: Option<GovtId>,
    /// The nav mode and selection at the operator.
    pub nav: NavMode,
    /// The shield, armour and fuel at the operator, after its armour
    /// bump.
    pub reserves: Reserves,
    /// How many mission texts had been queued at the operator
    /// (`nova_sim::Session::texts_queued`): it ran after each text whose
    /// [`TextBar::texts`](crate::mission_screens::TextBar::texts) is at most this, and
    /// before the others.
    pub texts: u64,
}
