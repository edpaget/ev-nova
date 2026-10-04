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
//! - [`flight`]: the flight screen, which flies the player's ship
//!   (`nova_sim`'s session) in its starting system with the original's
//!   default keys, the camera following the ship, and reads the ship's
//!   sprite through its [`flight::ShipSprites`] port.
//!
//! # Text
//!
//! Each [`DrawCommand::Text`] names its [`Font`], one of the game's two
//! interface fonts (Geneva or Charcoal); the renderer picks the face that
//! draws it. [`text`] holds the [`text::TextMetrics`] port that measures
//! text as it will be drawn, the line-height rule the renderer shares, and
//! word wrap.
//!
//! # Widgets
//!
//! - [`ui`]: immediate-mode widgets (buttons drawn with Nova's button
//!   graphics, scrolling text) and modal dialogs laid out from the
//!   interface file's `DLOG`/`DITL`s, such as the "Desc Dialog"
//!   ([`ui::DescDialog`]).
//!
//! # Developer tools
//!
//! - [`devtools`]: the developer tools overlay's model, which `nova`
//!   draws with egui only in its `dev-tools` build.

pub mod color;
pub mod devtools;
pub mod draw;
pub mod flight;
pub mod font;
pub mod galaxy;
pub mod geometry;
pub mod image;
pub mod input;
pub mod navigator;
pub mod screen;
pub mod ships;
pub mod system;
pub mod text;
mod time;
pub mod ui;

pub use color::Color;
pub use draw::{DrawCommand, DrawList};
pub use font::Font;
pub use geometry::Point;
pub use image::{ImageKey, ImageKind};
pub use input::{Input, Key, MouseButton};
pub use navigator::Navigator;
pub use screen::{Screen, ScreenAction};
