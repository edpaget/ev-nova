//! The layers ships are drawn in, as `_HandleShipDisplay` puts them.
//!
//! # The original
//!
//! The original draws through `SpriteWorld`'s sprite layers.
//! `_SetupSpriteWorld` creates 14 (@0x21c3d-0x21cf6) and adds them with
//! `_SWAddSpriteLayer` (@0x21d0c-0x21e24), which appends each at the
//! world's tail (@0xb71b0-0xb71df); `_SWAnimateSpriteWorld` walks them from
//! the head (@0xb96f4), so the first added is drawn at the bottom. Three of
//! them hold ships, under most shots' layer (`0x3b52f0`) for the lowest and
//! over it for the other two: `0x3b53a0`, `0x3b52a8` and `0x3b541c`.
//!
//! Each frame, for every ship and the player (slot 0) too,
//! `_HandleShipDisplay` (@0x2bd24-0x2bd47) picks its layer ([`ShipLayer::of`]):
//!
//! - a disabled ship (`_IsDisabled`, read into -0x152 @0x2b61f-0x2b624)
//!   goes in `0x3b53a0`, [`ShipLayer::Disabled`];
//! - otherwise a ship whose leader (+0x9a) is the player, 0, goes in
//!   `0x3b52a8`, [`ShipLayer::Led`];
//! - every other ship, and the player, goes in `0x3b541c`,
//!   [`ShipLayer::Ships`].
//!
//! `_IsDisabled` is true at armour 0 or less (@0x2dd2-0x2e02), so a ship
//! breaking up is in the bottom layer too ([`disabled`]). Its defence-ship
//! exception (+0x8c not -1) never applies, as the engine has no defence
//! ships. None of these branches plays a sound.
//!
//! **Within a layer** `_PutSpriteInLayer` (@0x766c) moves a sprite only
//! when its layer changes, appending it at the tail (`_SWAddSprite`
//! @0xac620-0xac63b), and `_InitSprites` fills `0x3b541c` from slot 63 down
//! to slot 0 (@0x1f0eb-0x1f88b), so the order within a layer is the
//! history of entries, the player starting on top. Here
//! [`in_draw_order`] keeps each tier in the order it is given: the
//! session's NPC order, the player last
//! ([`Session::draw_order`](crate::Session::draw_order)). The history is
//! left to rdm:phase/player-led-ships/phase-21-ships-overlap-in-layer-entry-order.

use crate::combat::ShipRef;
use crate::combat::hull::Condition;

/// The layer a ship is drawn in, bottom to top (see the module docs).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ShipLayer {
    /// A disabled or breaking-up ship: `0x3b53a0`, the bottom ship layer.
    Disabled,
    /// A ship the player leads: `0x3b52a8`, the escorts' layer.
    Led,
    /// Every other ship, and the player: `0x3b541c`, the top ship layer.
    Ships,
}

impl ShipLayer {
    /// The layer of a ship `disabled` or not, `led` by the player or not:
    /// being disabled wins, as @0x2bd24 asks it first.
    #[must_use]
    pub fn of(disabled: bool, led: bool) -> Self {
        if disabled {
            Self::Disabled
        } else if led {
            Self::Led
        } else {
            Self::Ships
        }
    }
}

/// `ships` in the order they are drawn, bottom first: by layer, each
/// layer keeping the order it is given in.
#[must_use]
pub fn in_draw_order(ships: impl IntoIterator<Item = (ShipRef, ShipLayer)>) -> Vec<ShipRef> {
    let mut ships: Vec<(ShipRef, ShipLayer)> = ships.into_iter().collect();
    ships.sort_by_key(|&(_, layer)| layer);
    ships.into_iter().map(|(ship, _)| ship).collect()
}

/// Whether a ship in `condition` is disabled for its layer, as
/// `_IsDisabled` is at armour 0 or less: anything but intact.
#[must_use]
pub fn disabled(condition: Condition) -> bool {
    !matches!(condition, Condition::Intact)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traffic::npc::NpcId;

    #[test]
    fn a_disabled_ship_is_in_the_bottom_layer_even_when_led() {
        assert_eq!(ShipLayer::of(true, true), ShipLayer::Disabled);
        assert_eq!(ShipLayer::of(true, false), ShipLayer::Disabled);
        assert_eq!(ShipLayer::of(false, true), ShipLayer::Led);
        assert_eq!(ShipLayer::of(false, false), ShipLayer::Ships);
        assert!(ShipLayer::Disabled < ShipLayer::Led);
        assert!(ShipLayer::Led < ShipLayer::Ships);
    }

    #[test]
    fn the_draw_order_keeps_each_layers_own_order() {
        let npc = |n| ShipRef::Npc(NpcId(n));
        let order = in_draw_order([
            (npc(0), ShipLayer::Ships),
            (npc(1), ShipLayer::Led),
            (npc(2), ShipLayer::Disabled),
            (npc(3), ShipLayer::Ships),
            (npc(4), ShipLayer::Led),
            (npc(5), ShipLayer::Disabled),
            (ShipRef::Player, ShipLayer::Ships),
        ]);
        assert_eq!(
            order,
            [
                npc(2),
                npc(5),
                npc(1),
                npc(4),
                npc(0),
                npc(3),
                ShipRef::Player
            ]
        );
    }

    #[test]
    fn only_an_intact_ship_is_not_disabled() {
        assert!(!disabled(Condition::Intact));
        assert!(disabled(Condition::Disabled));
        assert!(disabled(Condition::Dying { ticks_left: 1 }));
        assert!(disabled(Condition::Destroyed));
    }
}
