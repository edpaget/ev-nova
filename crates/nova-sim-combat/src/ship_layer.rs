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
//! breaking up is in the bottom layer too ([`disabled`]), but for a
//! stellar's defence ship, which it never calls disabled by its armour
//! (+0x8c not -1, @0x2d6f; [`disabled`] takes whether the ship defends
//! one). None of these
//! branches plays a sound.
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
//! history by [`Slot`], and
//! `nova_sim::Session::draw_order` draws each layer
//! in it. The history outlives a pilot, as the original's sprites do:
//! `_ResetPlayer` and `_LoadPilotData` call no sprite routine, and
//! `_EnterGameFromMainScreen`'s `_PrepareAllSprites` lays each slot out
//! as a tick does, so a session loaded later in the same run starts from
//! the history the last one left
//! (`nova_sim::Session::with_slot_sprites`),
//! which the `nova` app hands on.
//!
//! **Hidden ships.** For a ship in the player's system (@0x2b7d8-0x2b7ed),
//! the player's too, `_HandleShipDisplay` asks whether its type's `Cost`
//! (the ship-type record's +0x58, filled from the `shïp`'s `Cost`
//! @0x7a16a-0x7a173) is -1 (@0x2b7f3). If so it jumps to @0x2cc6b, which
//! hides the base sprite and each of the other five it had shown
//! (@0x2cc6b-0x2cd10), and joins the common tail (@0x2cd17), skipping
//! the showing, frame and paint work and the whole layer block
//! (@0x2b7fd-0x2cc69). So such a ship is not drawn and its slot keeps
//! its place in whatever layer it last stood in
//! ([`HullSpec::hidden`](crate::combat::hull::HullSpec::hidden),
//! `nova_sim::Session::is_hidden`;
//! [`RuleKey::HiddenShips`](crate::rulebook::RuleKey::HiddenShips)
//! chooses). By
//! [`RuleKey::ShipOverlap`](crate::rulebook::RuleKey::ShipOverlap)'s
//! other reading, [`in_draw_order`] keeps each layer in the order it is
//! given, the session's NPC order with the player on top. The step-1
//! findings of
//! rdm:phase/player-led-ships/phase-21-ships-overlap-in-layer-entry-order
//! list every site with its address.
//!
//! **Cloned frames.** `_HandleMouse` tests the pointer against a slot's
//! base sprite rect (+0x1a), which `_SWSetCurrentFrameIndex` (@0xa90b6)
//! sizes to the current frame (frame +0x22..+0x29) and
//! `_SWSetSpriteLocation` (@0xab640) only moves. The hidden branch's tail
//! (@0x2cd17-0x2cd9d) centres that sprite on the ship, by its own size.
//! Each frame `_HandleShipDisplay` clones a shown ship's type's master
//! (`0x3b50a4[type]`) into its slot's base sprite
//! (`_SWCloneFramesFromMaster` @0x2b824), which keeps the frame index
//! when still in range (@0xac504-0xac514); every frame of one sheet is
//! one size, so only the sheet matters. A hidden ship skips the clone, so
//! its slot keeps the frames it last cloned: the type last shown there,
//! or `shïp` 128's, which `_InitSprites` clones into every slot at launch
//! (@0x1f17f-0x1f193). Like the layers, these outlive a pilot.
//! [`SlotFrames`] keeps them by [`Slot`],
//! `nova_sim::Session::framed_by` answers which sheet
//! a ship's hover box takes, and
//! [`RuleKey::HiddenShipHoverBox`](crate::rulebook::RuleKey::HiddenShipHoverBox)
//! chooses. The slot's other five sprites (+0x4..+0x14, cloned
//! @0x2bc52 and @0x2ca74) are not boxed, so are not kept. Neither path
//! plays a sound.
//!
//! **Laid-out places.** Only a display places a slot's base sprite:
//! `SWSetSpriteLocation` (@0x2cd9d) at `trunc(ship − player)` per axis
//! plus the play area's centre less half the frame (@0x2cd38-0x2cd8e),
//! for a shown ship and a hidden one alike (@0x2cc6b → @0x2cd17). An
//! inactive slot's display only hides it (@0x2b555-0x2b60a). So the rect
//! `_HandleMouse` boxes (+0x1a-+0x21, @0x38177) for a ship its slot has
//! not displayed is where the slot's last occupant was last laid out, on
//! screen. [`SlotPlaces`] keeps each slot's last offset from the player;
//! like the frames, it outlives a pilot. A slot never laid out this run
//! keeps the rect `_InitSprites`' clone left (@0x1f135), which is not
//! modelled (`nova_sim::Session::hover_place`).

use crate::catalog::ShipId;
use crate::combat::ShipRef;
use crate::combat::hull::Condition;
use crate::combat::shot_slots::ShotSlots;
use crate::geometry::Vec2;

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
/// `nova_sim::traffic::npc::Npc::berth`, its slot in the ship
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

/// The ship type whose frames every slot's base sprite holds at launch:
/// `_InitSprites` clones ship type index 0's master (`0x3b50a4[0]`),
/// `shïp` 128, into each slot (@0x1f17f-0x1f193).
pub const INIT_FRAMES: ShipId = ShipId(128);

/// The ship type whose frames each slot's base sprite last cloned (see
/// the module docs' "Cloned frames"): the sheet a hidden ship's hover
/// box is sized by.
///
/// `_HandleShipDisplay` clones a slot's base sprite from its ship's type
/// each frame it shows the ship (`_SWCloneFramesFromMaster` @0x2b824),
/// and `nova_sim::Session::lay_out_ships` records that here.
/// `_HandlePlayer`'s eject clones two more ship slots' base sprites, the
/// hulk it leaves to the player's old type (@0x6f3f5) and the player's
/// slot to the type it becomes, the escape pod (`shïp` index 0x2ff) or
/// the escape ship (@0x6f768), and the session's eject records both here
/// (see its `eject` module).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SlotFrames {
    /// The slots cloned since launch, each with the type it last cloned.
    cloned: std::collections::BTreeMap<Slot, ShipId>,
}

impl SlotFrames {
    /// Every slot holding [`INIT_FRAMES`], as `_InitSprites` leaves them.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Clones `ship`'s frames into `slot`'s base sprite, as
    /// `_HandleShipDisplay` does for a shown ship (@0x2b824).
    pub fn clone_frames(&mut self, slot: Slot, ship: ShipId) {
        self.cloned.insert(slot, ship);
    }

    /// The ship type whose frames `slot` holds.
    #[must_use]
    pub fn of(&self, slot: Slot) -> ShipId {
        self.cloned.get(&slot).copied().unwrap_or(INIT_FRAMES)
    }
}

/// Where each slot's base sprite was last laid out (see the module docs'
/// "Laid-out places"): its offset from the player at its last display,
/// in whole pixels, as `_HandleShipDisplay` places it at
/// `trunc(ship − player)` per axis (`SWSetSpriteLocation` @0x2cd9d, the
/// offsets @0x2cd56 and @0x2cd87), shown or hidden alike.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SlotPlaces {
    /// The slots laid out since launch, each with its last offset.
    placed: std::collections::BTreeMap<Slot, (i32, i32)>,
}

impl SlotPlaces {
    /// No slot laid out, as `_InitSprites` leaves them.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Lays `slot` out at `offset` from the player, truncated to whole
    /// pixels per axis.
    // The offset is on screen, far inside 32 bits.
    #[allow(clippy::cast_possible_truncation)]
    pub fn place(&mut self, slot: Slot, offset: Vec2) {
        let whole = (offset.x.trunc() as i32, offset.y.trunc() as i32);
        self.placed.insert(slot, whole);
    }

    /// The whole-pixel offset from the player `slot` was last laid out
    /// at, or none for a slot no display has laid out.
    // A whole-pixel offset on screen is exact in an `f32`.
    #[allow(clippy::cast_precision_loss)]
    #[must_use]
    pub fn of(&self, slot: Slot) -> Option<Vec2> {
        let &(x, y) = self.placed.get(&slot)?;
        Some(Vec2::new(x as f32, y as f32))
    }
}

/// What the ship and shot slots' sprites keep across pilots in one run:
/// each ship slot's layer history, its last cloned frames and its last
/// laid-out place (see the module docs), and each shot slot's place in
/// its layer and its leftover miss roll ([`ShotSlots`]).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SlotSprites {
    /// Each slot's layer and when it entered it.
    pub layers: LayerEntries,
    /// Each slot's last cloned frames.
    pub frames: SlotFrames,
    /// Each slot's last laid-out place.
    pub places: SlotPlaces,
    /// Each shot slot's layer history and leftover roll, none held: the
    /// 128 shot sprites `_InitSprites` lays out once (@0x1fa90-0x1fb83)
    /// keep their places across systems and pilots.
    pub shots: ShotSlots,
}

/// Whether a ship in `condition` that `defends` a stellar or not is
/// disabled for its layer, as `_IsDisabled` is at armour 0 or less
/// ([`Condition::stored_disabled`]): anything but intact, but for a
/// stellar's defence ship, which is disabled only while it is held.
#[must_use]
pub fn disabled(condition: Condition, defends: bool) -> bool {
    condition.stored_disabled(defends)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat::NpcId;

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
    fn a_fresh_slot_holds_shïp_128s_frames_and_keeps_the_last_cloned() {
        let mut frames = SlotFrames::new();
        assert_eq!(frames.of(Slot::Berth(5)), ShipId(128));
        assert_eq!(frames.of(Slot::Player), ShipId(128));
        assert_eq!(SlotFrames::default(), frames);
        frames.clone_frames(Slot::Berth(5), ShipId(200));
        assert_eq!(frames.of(Slot::Berth(5)), ShipId(200));
        assert_eq!(frames.of(Slot::Berth(6)), ShipId(128));
        assert_eq!(frames.of(Slot::Player), ShipId(128));
        frames.clone_frames(Slot::Player, ShipId(130));
        assert_eq!(frames.of(Slot::Player), ShipId(130));
    }

    #[test]
    fn a_slot_keeps_the_whole_pixel_offset_of_its_last_layout() {
        let mut places = SlotPlaces::new();
        assert_eq!(places.of(Slot::Berth(3)), None, "never laid out");
        places.place(Slot::Berth(3), Vec2::new(100.7, -20.3));
        assert_eq!(places.of(Slot::Berth(3)), Some(Vec2::new(100.0, -20.0)));
        assert_eq!(places.of(Slot::Berth(4)), None);
        places.place(Slot::Berth(3), Vec2::new(-5.9, 7.2));
        assert_eq!(places.of(Slot::Berth(3)), Some(Vec2::new(-5.0, 7.0)));
        assert_eq!(SlotPlaces::default(), SlotPlaces::new());
    }

    #[test]
    fn only_an_intact_ship_is_not_disabled() {
        assert!(!disabled(Condition::Intact, false));
        assert!(disabled(Condition::Disabled, false));
        assert!(disabled(BREAKING_UP, false));
        assert!(disabled(Condition::Destroyed, false));
    }

    const BREAKING_UP: Condition = Condition::Dying {
        ticks_left: 1,
        starting: false,
    };

    #[test]
    fn a_defence_ship_breaking_up_stays_out_of_the_disabled_layer_unless_held() {
        assert!(!disabled(BREAKING_UP, true), "@0x2d6f");
        assert!(!disabled(Condition::Intact, true));
        assert!(disabled(Condition::Disabled, true), "stored disabled: held");
    }
}
