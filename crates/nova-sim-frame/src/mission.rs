//! The names of `nova_sim::mission` the view reaches, re-exported from the
//! crates below the layers that nova-sim's `mission` comes from, where they now
//! live: the same items at the same path, so the view can name them through
//! this crate instead of nova-sim.

pub use nova_sim_state::shown::mission::screens::OfferLabels;

/// The names of `nova_sim::mission::screens` the view reaches.
pub mod screens {
    pub use nova_sim_state::shown::mission::screens::{
        ABORT_LABEL, ACCEPT_LABEL, ACTIVE_MISSIONS, AVAILABLE_HERE, BAR, FAILED_MARK,
        FIRST_OFFER_TICKS, NO_ACTIVE_MISSIONS, OUTFITTER, SHIPYARD, SPACEPORT, TRADE_CENTER,
        mac_ticks,
    };
}
