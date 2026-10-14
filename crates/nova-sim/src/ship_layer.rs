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
//! **Within a layer** the order is history, not a sort. `_InitSprites`,
//! run once at launch (`_InitEVNova` @0x47382), adds every slot's sprites
//! to `0x3b541c` from slot 63 down to slot 0 (@0x1f0eb-0x1f88b), so the
//! player (slot 0) starts on top. `_PutSpriteInLayer` (@0x766c-0x76b1)
//! moves a sprite only when its layer changes, and then appends it at the
//! layer's tail (`_SWAddSprite` @0xac620-0xac63b), drawn last. Nothing
//! else re-lays a ship slot's sprites, not a system entry, a new pilot or
//! a new ship in the slot, so a slot keeps its place until it changes
//! layer: an escort the player releases, or a ship repaired, is drawn
//! over the player until history moves it. [`LayerEntries`] keeps that
//! history by [`Slot`] for the session's life (the original keeps it
//! across pilots in one run:
//! rdm:phase/player-led-ships/phase-29-layer-history-across-pilots), and
//! [`Session::draw_order`](crate::Session::draw_order) draws each layer
//! in it. A ship whose type has no sprite, which the original hides and
//! never moves (@0x2b7f3), is laid out here as any other:
//! rdm:phase/player-led-ships/phase-30-unsprited-ship-hidden. By
//! [`RuleKey::ShipOverlap`](crate::rulebook::RuleKey::ShipOverlap)'s
//! other reading, [`in_draw_order`] keeps each layer in the order it is
//! given, the session's NPC order with the player on top. The step-1
//! findings of
//! rdm:phase/player-led-ships/phase-21-ships-overlap-in-layer-entry-order
//! list every site with its address.

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

/// A ship's sprite slot: the player's, slot 0, or an NPC's
/// [`Npc::berth`](crate::traffic::npc::Npc::berth), its slot in the ship
/// table. The slot, not the ship, keeps a place in a layer: a new ship at
/// a berth takes the place its last ship left.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Slot {
    /// The player's slot, 0.
    Player,
    /// An NPC's berth.
    Berth(u32),
}

/// Each slot's layer and when it entered it, as `SpriteWorld`'s layers
/// keep them (see the module docs): the history the original's order
/// within a layer comes from.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LayerEntries {
    /// The slots put in a layer other than their first, each with its
    /// layer and its entry, counted from 1.
    moved: std::collections::BTreeMap<Slot, (ShipLayer, i64)>,
    /// The last entry given.
    last: i64,
}

impl LayerEntries {
    /// The order `_InitSprites` leaves (@0x1f0eb-0x1f88b): every slot in
    /// [`ShipLayer::Ships`], added from slot 63 down to slot 0, so a lower
    /// berth over a higher and the player over every berth. Berths past
    /// 63 go on below.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// `slot`'s layer and entry: ranks compare as the slots are drawn,
    /// bottom first.
    #[must_use]
    pub fn rank(&self, slot: Slot) -> (ShipLayer, i64) {
        self.moved.get(&slot).copied().unwrap_or_else(|| {
            let first = match slot {
                Slot::Player => 0,
                Slot::Berth(berth) => -1 - i64::from(berth),
            };
            (ShipLayer::Ships, first)
        })
    }

    /// Puts `slot` in `layer`, as `_PutSpriteInLayer` (@0x766c-0x76b1)
    /// does: nothing when it is there already (@0x768c); otherwise it
    /// leaves its layer (`_SWRemoveSprite` @0x7693) and enters `layer` on
    /// top of every slot there (`_SWAddSprite` @0x76a5). Its null-sprite
    /// and null-layer guard (@0x767e-0x768a) has no counterpart: every
    /// slot and layer here exists.
    pub fn put(&mut self, slot: Slot, layer: ShipLayer) {
        if self.rank(slot).0 != layer {
            self.last += 1;
            self.moved.insert(slot, (layer, self.last));
        }
    }

    /// `ships`, each at its slot, in the order they are drawn, bottom
    /// first: by layer, and in each layer by entry.
    #[must_use]
    pub fn in_entry_order(&self, ships: impl IntoIterator<Item = (ShipRef, Slot)>) -> Vec<ShipRef> {
        let mut ships: Vec<(ShipRef, Slot)> = ships.into_iter().collect();
        ships.sort_by_key(|&(_, slot)| self.rank(slot));
        ships.into_iter().map(|(ship, _)| ship).collect()
    }
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

    /// `ships` (NPC id = berth, `None` for the player) in `entries`'
    /// order, as their ids.
    fn ordered(entries: &LayerEntries, ships: &[Option<u32>]) -> Vec<Option<u32>> {
        let ships = ships.iter().map(|&ship| match ship {
            Some(berth) => (ShipRef::Npc(NpcId(berth)), Slot::Berth(berth)),
            None => (ShipRef::Player, Slot::Player),
        });
        entries
            .in_entry_order(ships)
            .into_iter()
            .map(|ship| match ship {
                ShipRef::Npc(NpcId(id)) => Some(id),
                ShipRef::Player => None,
            })
            .collect()
    }

    #[test]
    fn at_first_lower_berths_are_drawn_over_higher_and_the_player_over_all() {
        let entries = LayerEntries::new();
        assert_eq!(
            ordered(&entries, &[None, Some(1), Some(5), Some(70)]),
            [Some(70), Some(5), Some(1), None]
        );
        assert_eq!(entries.rank(Slot::Player).0, ShipLayer::Ships);
        assert_eq!(entries.rank(Slot::Berth(9)).0, ShipLayer::Ships);
        assert!(entries.rank(Slot::Berth(0)) < entries.rank(Slot::Player));
        assert_eq!(LayerEntries::default(), entries);
    }

    #[test]
    fn a_put_in_the_layer_a_slot_is_in_changes_nothing() {
        let mut entries = LayerEntries::new();
        entries.put(Slot::Berth(5), ShipLayer::Ships);
        entries.put(Slot::Player, ShipLayer::Ships);
        assert_eq!(entries, LayerEntries::new());
        entries.put(Slot::Berth(2), ShipLayer::Led);
        entries.put(Slot::Berth(3), ShipLayer::Led);
        let before = entries.clone();
        entries.put(Slot::Berth(2), ShipLayer::Led);
        assert_eq!(entries, before, "berth 2 keeps its place under berth 3");
        assert_eq!(ordered(&entries, &[Some(3), Some(2)]), [Some(2), Some(3)]);
    }

    #[test]
    fn a_slot_back_in_a_layer_enters_it_on_top() {
        let mut entries = LayerEntries::new();
        entries.put(Slot::Berth(4), ShipLayer::Led);
        assert_eq!(
            ordered(&entries, &[None, Some(1), Some(4)]),
            [Some(4), Some(1), None]
        );
        entries.put(Slot::Berth(4), ShipLayer::Ships);
        assert_eq!(
            ordered(&entries, &[None, Some(1), Some(4)]),
            [Some(1), None, Some(4)],
            "above the player"
        );
        entries.put(Slot::Player, ShipLayer::Disabled);
        entries.put(Slot::Player, ShipLayer::Ships);
        assert_eq!(
            ordered(&entries, &[None, Some(1), Some(4)]),
            [Some(1), Some(4), None]
        );
    }

    #[test]
    fn slots_put_in_a_layer_one_after_another_stack_in_that_order() {
        let mut entries = LayerEntries::new();
        entries.put(Slot::Berth(6), ShipLayer::Disabled);
        entries.put(Slot::Berth(2), ShipLayer::Disabled);
        entries.put(Slot::Berth(9), ShipLayer::Led);
        entries.put(Slot::Berth(3), ShipLayer::Led);
        assert_eq!(
            ordered(&entries, &[Some(2), Some(3), Some(6), Some(9), None]),
            [Some(6), Some(2), Some(9), Some(3), None]
        );
        assert_eq!(entries.rank(Slot::Berth(9)).0, ShipLayer::Led);
        assert!(entries.rank(Slot::Berth(9)) < entries.rank(Slot::Berth(3)));
    }

    #[test]
    fn only_an_intact_ship_is_not_disabled() {
        assert!(!disabled(Condition::Intact));
        assert!(disabled(Condition::Disabled));
        assert!(disabled(Condition::Dying {
            ticks_left: 1,
            starting: false
        }));
        assert!(disabled(Condition::Destroyed));
    }
}
