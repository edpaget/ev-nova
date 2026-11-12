//! The sprite layer a shot is in, as `_SpawnShot` puts it
//! (@0x3f0a9-0x3f143 in the `EV Nova` executable):
//! [`ShotLayer`], [`nova_sim_state::shown::combat::shot_layer`]'s,
//! re-exported here. The fight's hit pass reads it
//! ([`Combat`](super::Combat),
//! [`Shot::touched`](super::projectile::Shot::touched)).

pub use nova_sim_state::shown::combat::shot_layer::ShotLayer;
