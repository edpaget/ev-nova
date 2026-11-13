//! The names of `nova_sim::planet_comm` the view reaches, re-exported from the
//! crates below the layers that nova-sim's `planet_comm` comes from, where they now
//! live: the same items at the same path, so the view can name them through
//! this crate instead of nova-sim.

pub use nova_sim_state::shown::planet_comm::{
    PlanetCommButton, PlanetCommItem, PlanetCommView, PlanetStatus, STATUS_LABEL,
};
