//! What of each ship's sprites loads, [`SpriteLoad`], from
//! `nova_sim_combat::combat::sprite_load`, which re-exports it and keeps
//! what the load does to a class's display.

/// What of each ship's sprites loads (see the module docs). The default
/// loads everything, as the original's preferences are registered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpriteLoad {
    /// The `ShipAnimations` preference: a class's sets all load.
    pub ship_animations: bool,
    /// The `RunningLights` preference: a class's lights sprite loads.
    pub running_lights: bool,
}

impl Default for SpriteLoad {
    fn default() -> Self {
        Self {
            ship_animations: true,
            running_lights: true,
        }
    }
}
