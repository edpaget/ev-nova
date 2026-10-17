//! The galaxy map: every star system at its map position, its hyperlinks,
//! each system coloured by its government, and the nebulae behind them.
//!
//! - [`map`]: the [`GalaxyMap`] screen, which only reads the port.
//! - [`catalog`]: the [`GalaxyCatalog`] port, the whole galaxy already
//!   resolved and owned.
//! - [`data`]: the port's adapter over `nova_data`'s `GameData`.
//! - [`view`]: the [`MapView`], which part of the galaxy the map shows and
//!   at what scale.
//! - [`model`]: the [`GalaxyModel`], the galaxy laid out for drawing and
//!   clicking: colours, hyperlinks, nebula placement and hit-testing.
//! - [`panel`]: the in-flight maps' two info panels, laid out at DITL
//!   2000's offsets as `_MapDialogUpdate` draws them.
//! - [`buttons`]: the in-flight maps' Done and Find buttons, until the
//!   map is laid out as DITL 2000.
//! - [`find`]: the map's Find dialog, `DLOG` 3005, as `_DoSystemSearch`
//!   opens it.

pub mod buttons;
pub mod catalog;
pub mod data;
pub mod find;
pub mod map;
pub mod model;
pub mod panel;
pub mod view;

#[cfg(test)]
mod testkit;

pub use catalog::{
    Galaxy, GalaxyCatalog, GovtId, NebulaEntry, NebulaId, NebulaPicture, StellarEntry, StellarId,
    SystemEntry, SystemId,
};
pub use find::{FindDialog, FindOutcome};
pub use map::{GalaxyMap, MapCycle, MapMarks, MapMode};
pub use model::{GalaxyModel, MapSystem, NEUTRAL};
pub use view::{MapView, Transform};
