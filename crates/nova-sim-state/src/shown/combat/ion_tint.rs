//! An ionized ship's tint, [`IonTint`], from
//! `nova_sim_combat::combat::ion_tint`, which re-exports it and keeps the
//! rule that works it out each frame.

/// The colorize an ionized ship's hull takes this frame: its level, out
/// of 32, and its ion colour, `00RRGGBB`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct IonTint {
    /// The level, 14 to 26 (the original's +0xb0).
    pub level: u8,
    /// The ship's ion colour (its +0xb0, the sprite's +0xb4).
    pub colour: u32,
}
