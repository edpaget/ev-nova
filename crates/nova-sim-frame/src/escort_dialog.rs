//! The names of `nova_sim::escort_dialog` the view reaches, re-exported from the
//! crates below the layers that nova-sim's `escort_dialog` comes from, where they now
//! live: the same items at the same path, so the view can name them through
//! this crate instead of nova-sim.

pub use nova_sim_state::shown::escort_dialog::{
    CANCEL_SALE, CANCEL_UPGRADE, CANNOT_BE_UPGRADED, CAPTURED_ESCORT, CREDIT, CREDITS,
    EscortButton, EscortDialogView, RELEASE, SELL_ESCORT, SELL_PRICE, SaleLine, UPGRADE_COST,
    UPGRADE_ESCORT, UpgradeLine, WILL_BE_SOLD, WILL_BE_UPGRADED,
};
