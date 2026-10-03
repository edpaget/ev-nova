//! The ship browser: pages through every ship class, showing its sprite
//! rotating with its glow and lights layers, its name, key stats and
//! description.
//!
//! - [`catalog`]: the [`ShipCatalog`] port, everything the screen needs to
//!   know about one ship, already resolved and owned.
//! - [`data`]: the port's adapter over `nova_data`'s `GameData`.

pub mod catalog;
pub mod data;

pub use catalog::{SheetInfo, ShipCatalog, ShipEntry, ShipStats};
