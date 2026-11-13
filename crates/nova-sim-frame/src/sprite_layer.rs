//! The names of `nova_sim::sprite_layer` the view reaches, re-exported from the
//! crates below the layers that nova-sim's `sprite_layer` comes from, where they now
//! live: the same items at the same path, so the view can name them through
//! this crate instead of nova-sim.

pub use nova_sim_state::shown::sprite_layer::{
    SpriteLayer, WORLD_PASSES, WorldPass, layered_stack,
};
