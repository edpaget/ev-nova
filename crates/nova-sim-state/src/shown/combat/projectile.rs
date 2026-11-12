//! A shot's number, [`ShotId`], from `nova_sim_combat::combat::projectile`,
//! which re-exports it.

/// A shot's number in its fight, never reused.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ShotId(pub u32);
