//! The names of `nova_sim::storylines` the view reaches, re-exported from the
//! crates below the layers that nova-sim's `storylines` comes from, where they now
//! live: the same items at the same path, so the view can name them through
//! this crate instead of nova-sim.

/// The names of `nova_sim::storylines::news` the view reaches.
pub mod news {
    pub use nova_sim_state::shown::storylines::news::HeardNews;
}
