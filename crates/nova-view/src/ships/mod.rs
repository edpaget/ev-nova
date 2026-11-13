//! The ship browser: pages through every ship class, showing its sprite,
//! or its model, rotating with its glow and lights layers, its name, key
//! stats and description.
//!
//! - [`browser`]: the [`ShipBrowser`] screen, which only reads the port.
//! - [`catalog`]: the [`ShipCatalog`] port, everything the screen needs to
//!   know about one ship, already resolved and owned.
//! - [`data`]: the port's adapter over `nova_data`'s `GameData`.
//! - `model_turn`: how a class with no sprite sheet turns its model in
//!   the browser, in flight's pose.

pub mod browser;
pub mod catalog;
pub mod data;
mod model_turn;

pub use browser::ShipBrowser;
pub use catalog::{SheetInfo, ShipCatalog, ShipEntry, ShipId, ShipStats};
