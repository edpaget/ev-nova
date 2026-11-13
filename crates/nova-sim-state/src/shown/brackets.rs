//! The corner brackets round the nav target ([`NavBrackets`]) and the
//! ship target ([`TargetBrackets`]), and how a ship target stands towards
//! the player ([`Standing`]), from `nova_sim_ai::brackets`, which
//! re-exports them and keeps the fly-in and the standing rule that fill
//! them.

use nova_sim_base::geometry::Vec2;
use nova_sim_catalog::catalog::StellarId;

use super::combat::NpcId;

/// How a ship target stands towards the player, for its brackets' frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Standing {
    /// A threat to the player: red.
    Hostile,
    /// Any other ship: yellow.
    Neutral,
    /// The player's escort, or a ship one leads: green.
    Escort,
    /// Disabled or breaking up: grey.
    Disabled,
}

/// The brackets round the nav target.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NavBrackets {
    /// The stellar bracketed.
    pub stellar: StellarId,
    /// The stellar's spöb position (+0x4/+0x6), where the brackets are
    /// drawn whether its system shows it or not (`_HandleNavTargetSprites`
    /// @0x2fd9f-0x2fdda).
    pub position: Vec2,
    /// How far out they are (`nova_sim_ai::brackets::FlyIn::expand`).
    pub expand: f32,
    /// Whether the stellar is dominated: the green frames.
    pub dominated: bool,
}

/// The brackets round the ship target.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TargetBrackets {
    /// The ship bracketed.
    pub npc: NpcId,
    /// How far out they are (`nova_sim_ai::brackets::FlyIn::expand`).
    pub expand: f32,
    /// How it stands towards the player: the frames.
    pub standing: Standing,
}
