//! The order the ship and shot layers are drawn in, as
//! `_SetupSpriteWorld` adds them.
//!
//! `_SetupSpriteWorld` appends each sprite layer at the world's tail
//! (`_SWAddSpriteLayer` @0x21d0c-0x21e24, @0xb71b0-0xb71df), and
//! `_SWAnimateSpriteWorld` draws them from the head (@0xb96f4), so the
//! first added is at the bottom. Among the ships' layers
//! ([`ShipLayer`]) and the shots' ([`ShotLayer`]) the order, bottom to
//! top, is: 0x3b53a0 (disabled ships), 0x3b53bc (point defence), 0x3b531c
//! (homing), 0x3b52f0 (shots), 0x3b52a8 (ships the player leads),
//! 0x3b541c (other ships and the player), 0x3b5308 (turret shots over the
//! ships) ([`SpriteLayer`]). The other layers (stellars, effects, beams)
//! are not ordered here.
//!
//! A layer draws its sprites in the order they entered it
//! (`_SWAddSprite` @0xac620): [`stack`] keeps each layer in the order it
//! is given, the session's for ships ([`ship_layer`](crate::ship_layer))
//! and the fight's for shots
//! ([`Combat::shots_drawn`](crate::combat::Combat::shots_drawn)): the
//! order the original's shot slots entered each layer, a reused slot whose
//! sprite was already in the layer keeping its place (`_PutSpriteInLayer`
//! @0x768c; see [`shot_slots`](crate::combat::shot_slots)).
//!
//! The simulation decides the order; the view only draws the stack.

use crate::combat::shot_layer::ShotLayer;
use crate::ship_layer::ShipLayer;

/// A ship or shot layer, in the order `_SetupSpriteWorld` draws them,
/// bottom first (see the module docs).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SpriteLayer {
    /// 0x3b53a0: disabled and breaking-up ships ([`ShipLayer::Disabled`]).
    Disabled,
    /// 0x3b53bc: point-defence shots ([`ShotLayer::PointDefence`]).
    PointDefence,
    /// 0x3b531c: homing shots ([`ShotLayer::Homing`]).
    Homing,
    /// 0x3b52f0: most shots ([`ShotLayer::Shots`]).
    Shots,
    /// 0x3b52a8: ships the player leads ([`ShipLayer::Led`]).
    Led,
    /// 0x3b541c: every other ship, and the player ([`ShipLayer::Ships`]).
    Ships,
    /// 0x3b5308: a `Flags3` 0x0040 ship's turret shots
    /// ([`ShotLayer::OverShips`]).
    OverShips,
}

impl From<ShipLayer> for SpriteLayer {
    fn from(layer: ShipLayer) -> Self {
        match layer {
            ShipLayer::Disabled => Self::Disabled,
            ShipLayer::Led => Self::Led,
            ShipLayer::Ships => Self::Ships,
        }
    }
}

impl From<ShotLayer> for SpriteLayer {
    fn from(layer: ShotLayer) -> Self {
        match layer {
            ShotLayer::PointDefence => Self::PointDefence,
            ShotLayer::Homing => Self::Homing,
            ShotLayer::Shots => Self::Shots,
            ShotLayer::OverShips => Self::OverShips,
        }
    }
}

/// `items` in the order they are drawn, bottom first: by layer, each
/// layer keeping the order it is given in.
#[must_use]
pub fn stack<T>(items: impl IntoIterator<Item = (SpriteLayer, T)>) -> Vec<T> {
    let mut items: Vec<(SpriteLayer, T)> = items.into_iter().collect();
    items.sort_by_key(|&(layer, _)| layer);
    items.into_iter().map(|(_, item)| item).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_layers_go_as_setup_sprite_world_adds_them() {
        let mut layers = [
            SpriteLayer::OverShips,
            SpriteLayer::Ships,
            SpriteLayer::Led,
            SpriteLayer::Shots,
            SpriteLayer::Homing,
            SpriteLayer::PointDefence,
            SpriteLayer::Disabled,
        ];
        layers.sort();
        assert_eq!(
            layers,
            [
                SpriteLayer::Disabled,
                SpriteLayer::PointDefence,
                SpriteLayer::Homing,
                SpriteLayer::Shots,
                SpriteLayer::Led,
                SpriteLayer::Ships,
                SpriteLayer::OverShips,
            ]
        );
    }

    #[test]
    fn each_ship_and_shot_layer_is_its_sprite_layer() {
        assert_eq!(
            SpriteLayer::from(ShipLayer::Disabled),
            SpriteLayer::Disabled
        );
        assert_eq!(SpriteLayer::from(ShipLayer::Led), SpriteLayer::Led);
        assert_eq!(SpriteLayer::from(ShipLayer::Ships), SpriteLayer::Ships);
        assert_eq!(
            SpriteLayer::from(ShotLayer::PointDefence),
            SpriteLayer::PointDefence
        );
        assert_eq!(SpriteLayer::from(ShotLayer::Homing), SpriteLayer::Homing);
        assert_eq!(SpriteLayer::from(ShotLayer::Shots), SpriteLayer::Shots);
        assert_eq!(
            SpriteLayer::from(ShotLayer::OverShips),
            SpriteLayer::OverShips
        );
    }

    #[test]
    fn a_stack_goes_by_layer_keeping_each_layers_order() {
        let stacked = stack([
            (SpriteLayer::Ships, 'a'),
            (SpriteLayer::Shots, 'b'),
            (SpriteLayer::Ships, 'c'),
            (SpriteLayer::Disabled, 'd'),
            (SpriteLayer::Shots, 'e'),
            (SpriteLayer::OverShips, 'f'),
        ]);
        assert_eq!(stacked, ['d', 'b', 'e', 'a', 'c', 'f']);
    }
}
