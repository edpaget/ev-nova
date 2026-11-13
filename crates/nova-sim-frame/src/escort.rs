//! The names of `nova_sim::escort` the view reaches, re-exported from the
//! crates below the layers that nova-sim's `escort` comes from, where they now
//! live: the same items at the same path, so the view can name them through
//! this crate instead of nova-sim.

pub use nova_sim_state::shown::escort::{
    ESCORT_COMMANDS, NEW_ORDERS, NO_ESCORTS, WILL_ATTACK, order_label,
};
