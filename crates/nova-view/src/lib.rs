//! The renderer-free vocabulary that EV Nova's screens produce and consume.
//!
//! A screen turns [`Input`] into state changes and draws itself into a
//! [`DrawList`] of [`DrawCommand`]s in logical coordinates, naming images by
//! [`ImageKey`]. `nova-render` turns draw lists into pixels; this crate knows
//! nothing about windows or GPUs.
//!
//! # Screens
//!
//! - [`ships`]: the ship browser, which pages through every `shïp` and
//!   reads the game data through its [`ships::ShipCatalog`] port.
//! - [`galaxy`]: the galaxy map, which draws every system, hyperlink and
//!   nebula and reads the game data through its
//!   [`galaxy::GalaxyCatalog`] port.
//! - [`system`]: the system view, which shows one system's stellars at
//!   their positions over a parallax starfield, through a camera the
//!   keyboard moves, and reads the game data through its
//!   [`system::SystemCatalog`] port.
//! - [`navigator`]: the [`Navigator`], which shows the galaxy map and the
//!   system view opened from it, and goes back to the map on Escape.

pub mod color;
pub mod draw;
pub mod galaxy;
pub mod geometry;
pub mod image;
pub mod input;
pub mod navigator;
pub mod screen;
pub mod ships;
pub mod system;
mod time;

pub use color::Color;
pub use draw::{DrawCommand, DrawList};
pub use geometry::Point;
pub use image::{ImageKey, ImageKind};
pub use input::{Input, Key, MouseButton};
pub use navigator::Navigator;
pub use screen::{Screen, ScreenAction};
