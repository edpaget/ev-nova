//! The names of `nova_sim::combat` the view reaches, re-exported from the
//! crates below the layers that nova-sim's `combat` comes from, where they now
//! live: the same items at the same path, so the view can name them through
//! this crate instead of nova-sim.

pub use nova_sim_state::shown::combat::ParticleBurst;
pub use nova_sim_state::shown::combat::sprite_load::SpriteLoad;

/// The names of `nova_sim::combat::exits` the view reaches.
pub mod exits {
    pub use nova_sim_state::shown::combat::exits::drawn_frame;
}

/// The names of `nova_sim::combat::hull_turn` the view reaches.
pub mod hull_turn {
    pub use nova_sim_state::shown::combat::hull_turn::HullTurn;
}

/// The names of `nova_sim::combat::ion_tint` the view reaches.
pub mod ion_tint {
    pub use nova_sim_state::shown::combat::ion_tint::IonTint;
}

/// The names of `nova_sim::combat::projectile` the view reaches.
pub mod projectile {
    pub use nova_sim_state::shown::combat::projectile::ShotId;
}

/// The names of `nova_sim::combat::ship_frames` the view reaches.
pub mod ship_frames {
    pub use nova_sim_state::shown::combat::ship_frames::{Alt, Drawn, FrameSpec};
}

/// The names of `nova_sim::combat::shot_fade` the view reaches.
pub mod shot_fade {
    pub use nova_sim_state::shown::combat::shot_fade::{FADE_FULL, at_full_gain};
}

/// The names of `nova_sim::combat::shot_layer` the view reaches.
pub mod shot_layer {
    pub use nova_sim_state::shown::combat::shot_layer::ShotLayer;
}

/// The names of `nova_sim::combat::weapon` the view reaches.
pub mod weapon {
    pub use nova_sim_catalog::catalog::Explosion;
    pub use nova_sim_state::shown::combat::weapon::loaded_falloff;
}
