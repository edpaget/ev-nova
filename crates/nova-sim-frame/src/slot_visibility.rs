//! Where a ship's hover and click box is ([`HoverPlace`]), from
//! `nova_sim::session::slot_visibility`, which re-exports it and keeps
//! which ship slots are shown.

use nova_sim_base::geometry::Vec2;

/// Where a ship's hover and click box is (`nova_sim::Session::hover_place`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum HoverPlace {
    /// Where the ship is drawn, or would be when hidden.
    Own,
    /// At this whole-pixel offset from where the player is drawn: where
    /// its slot was last laid out, by another ship.
    Stale(Vec2),
    /// At the window's top-left, the sprite world's (0, 0): its slot has
    /// not been laid out this run, so its rect is where `_InitSprites`'
    /// clone left it (`_SWCloneSprite` @0x1f135 copies the unlocated
    /// master's rect, @0xa9462). Sized by a frame of the frames its slot
    /// holds (`nova_sim::Session::framed_by`).
    Cloned,
}
