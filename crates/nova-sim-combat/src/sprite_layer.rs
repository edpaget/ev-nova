//! The order the flight world's sprite layers, beams and particles are
//! drawn in, as `_SetupSpriteWorld` builds the sprite world.
//!
//! `_SetupSpriteWorld` creates the 14 layers @0x21c3d-0x21cf6 (calls to
//! `_SWCreateSpriteLayer` @0xac560) and adds the layers @0x21d0c-0x21e24
//! (calls to `_SWAddSpriteLayer` @0xb71b0), each at the world's tail.
//! `_SWAnimateSpriteWorld` draws them from the head (@0xb995e-0xb99ea), so
//! the first added is at the bottom ([`SpriteLayer`], bottom to top):
//!
//! | layer | pointer | global | holds |
//! |---|---|---|---|
//! | [`Stars`](SpriteLayer::Stars) | 0x3b5388 | `_starLayer` | the 20 stars |
//! | [`Stellars`](SpriteLayer::Stellars) | 0x3b533c | `_stellarLayer` | the 16 stellars |
//! | [`Asteroids`](SpriteLayer::Asteroids) | 0x3b53b4 | `_asteroidLayer` | the 16 asteroids |
//! | [`Disabled`](SpriteLayer::Disabled) | 0x3b53a0 | `_disabledShipLayer` | disabled and breaking-up ships |
//! | [`Boxes`](SpriteLayer::Boxes) | 0x3b53ec | `_boxLayer` | the 64 cargo boxes, then the 32 escape pods |
//! | [`Smoke`](SpriteLayer::Smoke) | 0x3b52d0 | `_smokeLayer` | the 64 smoke puffs |
//! | [`PointDefence`](SpriteLayer::PointDefence) | 0x3b53bc | `_pointDefenseLayer` | point-defence shots |
//! | [`Homing`](SpriteLayer::Homing) | 0x3b531c | `_guidedShotLayer` | homing shots |
//! | [`Shots`](SpriteLayer::Shots) | 0x3b52f0 | `_shotLayer` | the other shots |
//! | [`Led`](SpriteLayer::Led) | 0x3b52a8 | `_escortLayer` | ships the player leads |
//! | [`Ships`](SpriteLayer::Ships) | 0x3b541c | `_shipLayer` | the other ships and the player |
//! | [`OverShips`](SpriteLayer::OverShips) | 0x3b5308 | `_turretedShotLayer` | `Flags3` 0x0040 turret shots |
//! | [`Explosions`](SpriteLayer::Explosions) | 0x3b52b8 | `_explodLayer` | the 32 explosions |
//! | [`Targets`](SpriteLayer::Targets) | 0x3b5318 | `_targetLayer` | the brackets, the cursor, the red-alert and double-time icons |
//!
//! After each layer's sprites, `_SWAnimateSpriteWorld` calls the layer's
//! callback if it has one (layer +0x10, @0xb99d7-0xb99e4), and after the
//! last layer the world's post-draw callback (world +0x30,
//! @0xb99f8-0xb9a05). `_SetupSpriteWorld` gives the stellar layer
//! `_BeamDrawOffscreenCallback` (the beams under the ships), the target
//! layer `_BeamDrawCallback` (the other beams), and the world
//! `_PostDrawCallback` (the particles). [`WORLD_PASSES`] is the whole
//! order, layers and callbacks.
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
//! The simulation decides the order; the view only follows it.

use crate::combat::shot_layer::ShotLayer;
use crate::ship_layer::ShipLayer;

/// A sprite layer of the flight world, in the order `_SetupSpriteWorld`
/// adds them, bottom first (see the module docs). `_InitSprites` puts each
/// layer's sprites in it once, in slot order, except the ships' and the
/// shots', which move between layers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SpriteLayer {
    /// 0x3b5388 (`_starLayer`): the 20 star sprites of `_starData`
    /// (`_InitSprites` @0x1fd5d-0x1fe69).
    Stars,
    /// 0x3b533c (`_stellarLayer`): the 16 stellar sprites of
    /// `_stellarSprite` (@0x1f8b5-0x1f95c). Its callback draws the beams
    /// under the ships ([`WorldPass::BeamsUnderShips`]).
    Stellars,
    /// 0x3b53b4 (`_asteroidLayer`): the 16 asteroid sprites of
    /// `_asteroidData` (@0x1ff87-0x2008d).
    Asteroids,
    /// 0x3b53a0 (`_disabledShipLayer`): disabled and breaking-up ships
    /// ([`ShipLayer::Disabled`]).
    Disabled,
    /// 0x3b53ec (`_boxLayer`): the 64 cargo boxes of `_boxData` (the loop
    /// @0x1fb89-0x1fc6b), then the 32 escape pods of `_podData`
    /// (@0x1fc71-0x1fd57), so the pods draw over the boxes.
    Boxes,
    /// 0x3b52d0 (`_smokeLayer`): the 64 smoke puffs of `_smokeData`
    /// (@0x1fe6f-0x1ff63), cloned from `_masterSmokeSprite`; `_SpawnSmoke`
    /// (@0x3f957-0x3f9d4) takes a slot of them.
    Smoke,
    /// 0x3b53bc (`_pointDefenseLayer`): point-defence shots
    /// ([`ShotLayer::PointDefence`]).
    PointDefence,
    /// 0x3b531c (`_guidedShotLayer`): homing shots ([`ShotLayer::Homing`]).
    Homing,
    /// 0x3b52f0 (`_shotLayer`): most shots ([`ShotLayer::Shots`]). The 128
    /// shot sprites of `_shotData` (@0x1fa90-0x1fb83) start here, and
    /// `_SpawnShot` moves them.
    Shots,
    /// 0x3b52a8 (`_escortLayer`): ships the player leads ([`ShipLayer::Led`]).
    Led,
    /// 0x3b541c (`_shipLayer`): every other ship, and the player
    /// ([`ShipLayer::Ships`]).
    Ships,
    /// 0x3b5308 (`_turretedShotLayer`): a `Flags3` 0x0040 ship's turret shots
    /// ([`ShotLayer::OverShips`]).
    OverShips,
    /// 0x3b52b8 (`_explodLayer`): the 32 explosion sprites of `_explodData`
    /// (@0x1f962-0x1fa8a); `_SpawnExplod` (@0x3f5ef) takes a slot of them.
    Explosions,
    /// 0x3b5318 (`_targetLayer`): the nav and target brackets
    /// (@0x20093-0x20277), the cursor (@0x2027d-0x202f9), the red-alert icon
    /// (@0x202fe-0x2038b) and the double-time icon (@0x20390-0x203fd). Its
    /// callback draws the beams over the ships
    /// ([`WorldPass::BeamsOverShips`]).
    Targets,
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

/// One pass of drawing the flight world: a sprite layer's sprites, or one
/// of the callbacks `_SetupSpriteWorld` sets.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WorldPass {
    /// The layer's sprites, in the order they entered it.
    Layer(SpriteLayer),
    /// The beams whose weapon has `Flags2` 0x2000:
    /// `_BeamDrawOffscreenCallback` (0x37e90), the stellar layer's callback
    /// (set @0x21e29-0x21e37), draws only those (@0x37ed1-0x37ed5), each
    /// with `_DrawLightningBeam` (@0x37f56) or `_DrawBeam` (@0x37f9c). So
    /// they go over the stellars and under the asteroids and every layer
    /// above.
    BeamsUnderShips,
    /// The other beams: `_BeamDrawCallback` (0x37952), the target layer's
    /// callback (set @0x21e3c-0x21e50), draws only the beams without
    /// `Flags2` 0x2000 (@0x37af9-0x37afd, then @0x37c25-0x37e72). So they
    /// go over every layer, the explosions and the target layer too.
    BeamsOverShips,
    /// The particles (hit bursts, shot trails, debris; `_SpawnParticles`
    /// @0x44b70), which are no layer's: the world's post-draw callback
    /// `_PostDrawCallback` (0x38c80, set @0x22057-0x22065) draws them last,
    /// `jmp _DrawParticlesOffscreen` @0x38c9a, after `_FloaterDrawCallback`
    /// (@0x38c8d).
    Particles,
}

/// The flight world's draw passes, bottom first: each layer, with the
/// stellar layer's and the target layer's beam callbacks after their
/// layers, then the particles (see the module docs).
pub const WORLD_PASSES: [WorldPass; 17] = [
    WorldPass::Layer(SpriteLayer::Stars),
    WorldPass::Layer(SpriteLayer::Stellars),
    WorldPass::BeamsUnderShips,
    WorldPass::Layer(SpriteLayer::Asteroids),
    WorldPass::Layer(SpriteLayer::Disabled),
    WorldPass::Layer(SpriteLayer::Boxes),
    WorldPass::Layer(SpriteLayer::Smoke),
    WorldPass::Layer(SpriteLayer::PointDefence),
    WorldPass::Layer(SpriteLayer::Homing),
    WorldPass::Layer(SpriteLayer::Shots),
    WorldPass::Layer(SpriteLayer::Led),
    WorldPass::Layer(SpriteLayer::Ships),
    WorldPass::Layer(SpriteLayer::OverShips),
    WorldPass::Layer(SpriteLayer::Explosions),
    WorldPass::Layer(SpriteLayer::Targets),
    WorldPass::BeamsOverShips,
    WorldPass::Particles,
];

/// `items` in the order they are drawn, bottom first, each with its
/// layer: by layer, each layer keeping the order it is given in.
#[must_use]
pub fn layered_stack<T>(
    items: impl IntoIterator<Item = (SpriteLayer, T)>,
) -> Vec<(SpriteLayer, T)> {
    let mut items: Vec<(SpriteLayer, T)> = items.into_iter().collect();
    items.sort_by_key(|&(layer, _)| layer);
    items
}

/// `items` in the order they are drawn, bottom first: by layer, each
/// layer keeping the order it is given in.
#[must_use]
pub fn stack<T>(items: impl IntoIterator<Item = (SpriteLayer, T)>) -> Vec<T> {
    layered_stack(items)
        .into_iter()
        .map(|(_, item)| item)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_layers_go_as_setup_sprite_world_adds_them() {
        let mut layers = [
            SpriteLayer::Targets,
            SpriteLayer::Explosions,
            SpriteLayer::OverShips,
            SpriteLayer::Ships,
            SpriteLayer::Led,
            SpriteLayer::Shots,
            SpriteLayer::Homing,
            SpriteLayer::PointDefence,
            SpriteLayer::Smoke,
            SpriteLayer::Boxes,
            SpriteLayer::Disabled,
            SpriteLayer::Asteroids,
            SpriteLayer::Stellars,
            SpriteLayer::Stars,
        ];
        layers.sort();
        assert_eq!(
            layers,
            [
                SpriteLayer::Stars,
                SpriteLayer::Stellars,
                SpriteLayer::Asteroids,
                SpriteLayer::Disabled,
                SpriteLayer::Boxes,
                SpriteLayer::Smoke,
                SpriteLayer::PointDefence,
                SpriteLayer::Homing,
                SpriteLayer::Shots,
                SpriteLayer::Led,
                SpriteLayer::Ships,
                SpriteLayer::OverShips,
                SpriteLayer::Explosions,
                SpriteLayer::Targets,
            ]
        );
    }

    #[test]
    fn the_world_draws_its_layers_with_the_beams_after_the_stellars_and_the_targets_then_the_particles()
     {
        use SpriteLayer as L;
        use WorldPass::{BeamsOverShips, BeamsUnderShips, Layer, Particles};
        assert_eq!(
            WORLD_PASSES,
            [
                Layer(L::Stars),
                Layer(L::Stellars),
                BeamsUnderShips,
                Layer(L::Asteroids),
                Layer(L::Disabled),
                Layer(L::Boxes),
                Layer(L::Smoke),
                Layer(L::PointDefence),
                Layer(L::Homing),
                Layer(L::Shots),
                Layer(L::Led),
                Layer(L::Ships),
                Layer(L::OverShips),
                Layer(L::Explosions),
                Layer(L::Targets),
                BeamsOverShips,
                Particles,
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
    fn a_layered_stack_keeps_each_items_layer() {
        let stacked = layered_stack([
            (SpriteLayer::Ships, 'a'),
            (SpriteLayer::Boxes, 'b'),
            (SpriteLayer::Ships, 'c'),
            (SpriteLayer::Explosions, 'd'),
        ]);
        assert_eq!(
            stacked,
            [
                (SpriteLayer::Boxes, 'b'),
                (SpriteLayer::Ships, 'a'),
                (SpriteLayer::Ships, 'c'),
                (SpriteLayer::Explosions, 'd'),
            ]
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
