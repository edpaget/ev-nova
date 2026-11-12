//! How a link the galaxy map draws looks by
//! [`RuleKey::MapLaneStyle`](nova_sim_base::rulebook::RuleKey::MapLaneStyle)'s
//! other reading, the new game's, as plain data: each way of the link in
//! its gate lane's stability's look, and a one-way link marked
//! ([`LaneLook`], [`Back`]). From `nova_sim::map_lanes`, which re-exports
//! them and keeps the builder that reads the overlaid galaxy's gate lanes
//! (`nova_sim::map_lanes::lane_look`), with the module docs on how each
//! way is read.

use nova_sim_catalog::catalog::{GateStability, SystemId};

/// The way back along a drawn link, `to` → `from`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Back {
    /// The far system doesn't list the link back: one-way, arrowed.
    OneWay,
    /// It does: the way back's stability, or `None` where no gate serves
    /// it, drawn grey.
    Way(Option<GateStability>),
}

/// How one drawn link looks (see `nova_sim::map_lanes`'s module docs).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LaneLook {
    /// The system the link is drawn from.
    pub from: SystemId,
    /// The system it is drawn to.
    pub to: SystemId,
    /// The way out, `from` → `to`: its lane's stability, or `None` where
    /// no gate serves it, drawn grey.
    pub out: Option<GateStability>,
    /// The way back.
    pub back: Back,
}
