//! The order the flight world's sprite layers, beams and particles are
//! drawn in, as `_SetupSpriteWorld` builds the sprite world:
//! [`nova_sim_state::shown::sprite_layer`]'s, re-exported here. The
//! session orders the ships ([`ship_layer`](crate::ship_layer)) and the
//! fight the shots
//! ([`Combat::shots_drawn`](crate::combat::Combat::shots_drawn)); the view
//! only follows the order.

pub use nova_sim_state::shown::sprite_layer::{
    SpriteLayer, WORLD_PASSES, WorldPass, layered_stack, stack,
};
