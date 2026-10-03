//! The galaxy map: every star system at its map position, its hyperlinks,
//! each system coloured by its government, and the nebulae behind them.
//!
//! - [`catalog`]: the [`GalaxyCatalog`] port, the whole galaxy already
//!   resolved and owned.
//! - [`data`]: the port's adapter over `nova_data`'s `GameData`.
//! - [`view`]: the [`MapView`], which part of the galaxy the map shows and
//!   at what scale.
//! - [`model`]: the [`GalaxyModel`], the galaxy laid out for drawing and
//!   clicking: colours, hyperlinks, nebula placement and hit-testing.

pub mod catalog;
pub mod data;
pub mod model;
pub mod view;

pub use catalog::{
    Galaxy, GalaxyCatalog, GovtId, NebulaEntry, NebulaId, NebulaPicture, StellarEntry, StellarId,
    SystemEntry, SystemId,
};
pub use model::{GalaxyModel, MapSystem, NEUTRAL};
pub use view::MapView;
