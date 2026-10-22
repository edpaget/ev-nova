//! The flight view's sprite stack: the ships and the shots, bottom first,
//! each in the layer the simulation puts it in.
//!
//! The order is the simulation's ([`nova_sim::sprite_layer::layered_stack`]):
//! the disabled ships, the point-defence shots, the homing shots, the
//! other shots, the ships the player leads, the other ships and the
//! player, and a `Flags3` 0x0040 ship's turret shots, as
//! `_SetupSpriteWorld` adds the layers (@0x21d0c-0x21e24, calls to
//! `_SWAddSpriteLayer` @0xb71b0) and `_SpawnShot` fills them
//! (@0x3f0a9-0x3f143). Each layer keeps the order it is given in: the
//! ships the session's draw order, the shots the session's
//! ([`nova_sim::Session::shots_drawn`], each layer in the order its shot
//! slots entered it). Each sprite keeps its [`SpriteLayer`], so the world
//! draws each layer in its place among the others
//! ([`nova_sim::sprite_layer::WORLD_PASSES`]). The view only draws what the
//! stack gives.

use nova_sim::ShipRef;
use nova_sim::ship_layer::ShipLayer;
use nova_sim::sprite_layer::{SpriteLayer, layered_stack};

use crate::flight::weapons::ShotShown;

/// One sprite of the stack: a ship, or a shot.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Sprite {
    /// A ship, drawn as the view draws it.
    Ship(ShipRef),
    /// A shot, drawn as [`weapons::draw_shot`](crate::flight::weapons::draw_shot) draws it.
    Shot(ShotShown),
}

/// `ships`, each with its layer in draw order, and `shots`, as they are
/// drawn, bottom first, each with its layer (see the module docs).
pub(super) fn sprite_stack(
    ships: impl IntoIterator<Item = (ShipRef, ShipLayer)>,
    shots: impl IntoIterator<Item = ShotShown>,
) -> Vec<(SpriteLayer, Sprite)> {
    let ships = ships
        .into_iter()
        .map(|(ship, layer)| (SpriteLayer::from(layer), Sprite::Ship(ship)));
    let shots = shots
        .into_iter()
        .map(|shot| (shot.layer, Sprite::Shot(shot)));
    layered_stack(ships.chain(shots))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Point;
    use crate::flight::catalog::WeaponId;
    use nova_sim::NpcId;

    fn shot(weapon: i16, layer: SpriteLayer) -> ShotShown {
        ShotShown {
            id: nova_sim::combat::projectile::ShotId(0),
            spin_start: 0,
            spin_timer: 0,
            murk: 0,
            at: Point::new(0.0, 0.0),
            heading: 0.0,
            age: 0,
            weapon: WeaponId(weapon),
            layer,
        }
    }

    const INTACT: ShipRef = ShipRef::Npc(NpcId(1));
    const DISABLED: ShipRef = ShipRef::Npc(NpcId(2));
    const ESCORT: ShipRef = ShipRef::Npc(NpcId(3));

    /// The ships of each layer, in draw order.
    fn ships() -> [(ShipRef, ShipLayer); 4] {
        [
            (DISABLED, ShipLayer::Disabled),
            (ESCORT, ShipLayer::Led),
            (INTACT, ShipLayer::Ships),
            (ShipRef::Player, ShipLayer::Ships),
        ]
    }

    #[test]
    fn an_unguided_shot_is_drawn_under_an_intact_ship_and_over_a_disabled_one() {
        let unguided = shot(128, SpriteLayer::Shots);
        assert_eq!(
            sprite_stack(ships(), [unguided]),
            [
                (SpriteLayer::Disabled, Sprite::Ship(DISABLED)),
                (SpriteLayer::Shots, Sprite::Shot(unguided)),
                (SpriteLayer::Led, Sprite::Ship(ESCORT)),
                (SpriteLayer::Ships, Sprite::Ship(INTACT)),
                (SpriteLayer::Ships, Sprite::Ship(ShipRef::Player)),
            ]
        );
    }

    #[test]
    fn homing_and_point_defence_shots_are_drawn_under_unguided_ones() {
        let unguided = shot(128, SpriteLayer::Shots);
        let homing = shot(131, SpriteLayer::Homing);
        let defence = shot(133, SpriteLayer::PointDefence);
        assert_eq!(
            sprite_stack([], [unguided, homing, defence]),
            [
                (SpriteLayer::PointDefence, Sprite::Shot(defence)),
                (SpriteLayer::Homing, Sprite::Shot(homing)),
                (SpriteLayer::Shots, Sprite::Shot(unguided)),
            ]
        );
    }

    #[test]
    fn a_turret_shot_over_the_ships_is_drawn_over_every_ship() {
        let over = shot(130, SpriteLayer::OverShips);
        let stacked = sprite_stack(ships(), [over]);
        assert_eq!(
            stacked.last(),
            Some(&(SpriteLayer::OverShips, Sprite::Shot(over)))
        );
    }

    #[test]
    fn each_layer_keeps_the_order_it_is_given() {
        let (first, second) = (shot(128, SpriteLayer::Shots), shot(129, SpriteLayer::Shots));
        assert_eq!(
            sprite_stack([], [first, second]),
            [
                (SpriteLayer::Shots, Sprite::Shot(first)),
                (SpriteLayer::Shots, Sprite::Shot(second)),
            ]
        );
    }
}
