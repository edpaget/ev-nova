//! The collision passes, as `_DoPlayGameWork` walks them.

use crate::ship_layer::ShipLayer;

/// Where a ship's slot sprite stands as the collision passes find it: its
/// layer, and its place in that layer's order, which is the order the
/// layer is drawn in, bottom first (`_SWAddSprite` @0xac620). The session
/// gives each ship its rank as the last frame drew it
/// (`nova_sim::Session::draw_layers`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SpriteRank {
    /// The layer it stands in.
    pub layer: ShipLayer,
    /// Its place in the layer: a ship of a lower entry is walked before
    /// one of a higher. Only the order counts, not the number itself.
    pub entry: i64,
}
