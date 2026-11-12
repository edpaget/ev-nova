//! The names of `nova_sim::traffic` the view reaches, re-exported from the
//! crates below the layers that nova-sim's `traffic` comes from, where they now
//! live: the same items at the same path, so the view can name them through
//! this crate instead of nova-sim.

/// The names of `nova_sim::traffic::emerge` the view reaches.
pub mod emerge {
    pub use nova_sim_base::shown::traffic::emerge::{FULL_LEVEL, emerge_level};
}

/// The names of `nova_sim::traffic::gate_entry` the view reaches.
pub mod gate_entry {
    pub use nova_sim_base::shown::traffic::gate_entry::enter_level;
}

/// The names of `nova_sim::traffic::npc` the view reaches.
pub mod npc {
    pub use nova_sim_state::shown::combat::NpcId;
}
