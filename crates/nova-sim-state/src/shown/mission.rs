//! The mission screens' vocabulary, from `nova_sim_story::mission`, which
//! re-exports each item at its old path and keeps the missions
//! themselves.
//!
//! - [`active`]: why a mission is not accepted or aborted, and how many
//!   can be active, from `nova_sim_story::mission::active`.
//! - [`screens`]: the offer dialog, the mission screens' words and the
//!   venues offers come from, from `nova_sim_story::mission::screens`.

pub mod active;
pub mod screens;
