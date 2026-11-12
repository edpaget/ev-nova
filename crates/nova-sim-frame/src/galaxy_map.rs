//! What a galaxy map marks beside the mission systems ([`MapMarks`]),
//! from `nova_sim::galaxy_map`, which re-exports it and keeps the map's
//! drawing.

use nova_sim_catalog::catalog::SystemId;

/// What a map marks beside the mission systems: the green mission system
/// (`_greenMissionSyst`) and whether a system with the green arrow keeps
/// its red one (`_multiArrowsOnSingleSystem`). Only the mission dialogs'
/// maps set them, each for the map it opens, and put them back before
/// returning; `_InitMissions` (@0x97bc9-0x97bd8) and `_ResetPlayer`
/// (@0x1d8f5) leave [`MapMarks::NONE`]. So ours holds neither between
/// maps: each map opens with the marks its opener gives, and every other
/// map with none.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MapMarks {
    /// The system the green arrow marks, raw, if any.
    pub green: Option<SystemId>,
    /// Whether the green system keeps its red arrow.
    pub multi_arrows: bool,
}

impl MapMarks {
    /// What the in-flight maps mark, and what the resets leave: no green
    /// system (−1), multiple arrows allowed (1).
    pub const NONE: Self = Self {
        green: None,
        multi_arrows: true,
    };
}
