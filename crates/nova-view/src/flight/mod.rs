//! The flight screen: the player's ship flying in its starting system.
//!
//! - [`catalog`]: the [`ShipSprites`] port, the player ship's sprite sheet.
//! - [`data`]: the port's adapter over `nova_data`'s `GameData`.
//! - [`sprite`]: [`rotation_frame`], which frame of the ship's sheet shows
//!   its heading.
//! - [`view`]: the [`FlightView`] screen, which runs the flight and draws
//!   it.

pub mod catalog;
pub mod data;
pub mod sprite;
pub mod view;

pub use catalog::{ShipSheet, ShipSprites};
pub use sprite::rotation_frame;
pub use view::FlightView;
