//! A shot's number, [`ShotId`], and the life rules a shot's fade reads
//! ([`SPENT`], [`above`], [`whole_frames`]), from
//! `nova_sim_combat::combat::projectile`, which re-exports them.

/// A shot's number in its fight, never reused.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ShotId(pub u32);

/// The life a shot is put at on the visit that takes its life to 0 or
/// below, NaN included (`_HandleShot` @0x356bb-0x356c1).
pub const SPENT: f32 = -1.0;

/// Whether `value` is above `line`: false for a NaN, as the original's
/// `ucomiss`/`ucomisd` and `jbe` read it, so its negation holds for one.
/// `pub` for `nova_sim_combat::combat::projectile`, which reads it.
#[must_use]
pub fn above<T: PartialOrd + Copy>(value: T, line: T) -> bool {
    value > line
}

/// `frames` as a float, as `_SpawnShot` converts a `Count` (@0x3e7fd).
/// `pub` for `nova_sim_combat::combat::projectile`, which reads it.
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn whole_frames(frames: u32) -> f32 {
    frames as f32
}
