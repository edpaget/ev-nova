//! The flight screen: the player's ship flying in its starting system.
//!
//! - [`catalog`]: the [`ShipSprites`] port, the player ship's sprite
//!   sheet, and the [`StatusBars`] port, the `ïntf` status bar layouts.
//! - [`data`]: the ports' adapters over `nova_data`'s `GameData`.
//! - [`effects`]: the fight's explosions, debris and sounds, from the
//!   session's combat events.
//! - [`jump`]: the hyperspace jump's [`JumpEffect`], the stars streaking
//!   and the screen fading out and in.
//! - [`hud`]: the HUD, the `ïntf` status bar with its radar, shield,
//!   armour and fuel bars and the system's name.
//! - [`sprite`]: [`rotation_frame`], which frame of the ship's sheet shows
//!   its heading.
//! - [`target`]: the target panel, the secondary weapon's line and the
//!   brackets round the target.
//! - [`view`]: the [`FlightView`] screen, which runs the flight and draws
//!   it.
//! - [`weapons`]: drawing shots and beams.

pub mod catalog;
pub mod data;
pub mod effects;
pub mod hud;
pub mod jump;
pub mod sprite;
pub mod target;
pub mod view;
pub mod weapons;

pub use catalog::{
    BoomLook, CombatLooks, EffectSheet, Looks, ShipSheet, ShipSprites, StatusBarLayout, StatusBars,
    TargetCard, WeaponLook,
};
pub use jump::{JumpEffect, JumpPhase};
pub use sprite::rotation_frame;
pub use view::{FlightView, SharedChance};
