//! The flight screen: the player's ship flying in its starting system.
//!
//! - [`catalog`]: the [`ShipSprites`] port, the player ship's sprite
//!   sheet, and the [`StatusBars`] port, the `ïntf` status bar layouts.
//! - [`data`]: the ports' adapters over `nova_data`'s `GameData`.
//! - [`jump`]: the hyperspace jump's [`JumpEffect`], the stars streaking
//!   and the screen fading out and in.
//! - [`hud`]: the HUD, the `ïntf` status bar with its radar, shield,
//!   armour and fuel bars and the system's name.
//! - [`sprite`]: [`rotation_frame`], which frame of the ship's sheet shows
//!   its heading.
//! - [`view`]: the [`FlightView`] screen, which runs the flight and draws
//!   it.

pub mod catalog;
pub mod data;
pub mod hud;
pub mod jump;
pub mod sprite;
pub mod view;

pub use catalog::{ShipSheet, ShipSprites, StatusBarLayout, StatusBars};
pub use jump::{JumpEffect, JumpPhase};
pub use sprite::rotation_frame;
pub use view::FlightView;
