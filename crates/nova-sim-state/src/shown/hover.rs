//! The hover test's view: the box a ship is drawn in ([`DrawnBox`]), the
//! port the view hands the boxes through ([`DrawnShips`]) and what the
//! pointer is over ([`Hover`]), from `nova_sim_ai::hover`, which
//! re-exports them and keeps the hover test.

use nova_sim_catalog::catalog::StellarId;

use super::combat::NpcId;
use super::navigation::SpriteRect;

/// A ship's sprite as it is drawn: the rect the view draws its hull in, in
/// system coordinates, placed at whole pixels from the player as the
/// original places the slot (`_HandleShipDisplay` @0x2cd17-0x2cd9d).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DrawnBox {
    /// The rect the hull is drawn in.
    pub rect: SpriteRect,
}

impl DrawnBox {
    /// The box of a sprite drawn in `rect`.
    #[must_use]
    pub fn at(rect: SpriteRect) -> Self {
        Self { rect }
    }
}

/// Where the view draws the ships: the port the hover test reads.
pub trait DrawnShips {
    /// The box NPC `id` is drawn in, or would be when it is hidden, or is
    /// boxed in where its slot has not displayed it yet (where its slot
    /// was last laid out, or at the window's top-left in a slot never laid
    /// out this run: `nova_sim::Session::hover_place`), or `None` when it
    /// has no box: no sprite slot (the original's null sprite, @0x38171),
    /// which the engine never has.
    fn npc(&self, id: NpcId) -> Option<DrawnBox>;
    /// The box the player's ship is drawn in, or `None` when it has none.
    fn player(&self) -> Option<DrawnBox>;
}

/// What the pointer is over.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hover {
    /// The player's own ship, which counts only while it has a target.
    Player,
    /// An NPC.
    Ship(NpcId),
    /// A stellar.
    Stellar(StellarId),
}
