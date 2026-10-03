//! The galaxy map: every star system at its map position, its hyperlinks,
//! each system coloured by its government, and the nebulae behind them.
//!
//! - [`catalog`]: the [`GalaxyCatalog`] port, the whole galaxy already
//!   resolved and owned.
//! - [`data`]: the port's adapter over `nova_data`'s `GameData`.

pub mod catalog;
pub mod data;
pub mod view;

pub use catalog::{
    Galaxy, GalaxyCatalog, GovtId, NebulaEntry, NebulaId, NebulaPicture, StellarEntry, StellarId,
    SystemEntry, SystemId,
};
