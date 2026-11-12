//! A shot's fade, the original's +0x24, as `_HandleShot` steps it each
//! live visit at the frame's gain (@0x359ac-0x35c07): its rules and
//! builder are [`nova_sim_state::shown::combat::shot_fade`]'s, re-exported
//! here, and a fight steps a shot's fade by them
//! ([`Shot::fade_shown`](super::projectile::Shot::fade_shown)). None of
//! this plays a sound.

pub use nova_sim_state::shown::combat::shot_fade::{FADE_FULL, at_full_gain, step};
