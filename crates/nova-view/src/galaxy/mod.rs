//! The galaxy map: every star system at its map position, its hyperlinks,
//! each system coloured by its government, and the nebulae behind them.
//!
//! - [`map`]: the [`GalaxyMap`] screen, which only reads the port.
//! - [`catalog`]: the [`GalaxyCatalog`] port, the whole galaxy already
//!   resolved and owned.
//! - [`data`]: the port's adapter over `nova_data`'s `GameData`.
//! - [`view`]: the viewer's [`MapView`], which part of the galaxy it shows
//!   and at what scale, and the [`Transform`] every map draws through.
//! - [`model`]: the [`GalaxyModel`], the galaxy laid out for drawing and
//!   clicking: colours, hyperlinks and hit-testing.
//! - [`nebulae`]: the maps' nebulae, as `_MapDialogUpdate`'s nebula pass
//!   draws them: the truncated rect, the picture, the clip and `addMax`.
//! - [`borders`]: the maps' government borders, as `_DrawMapInfluence`
//!   paints the cells `nova_sim::map_borders` works out.
//! - [`update`]: which update item 3 shows, the full one or a drag's
//!   partial one.
//! - [`panel`]: the in-flight maps' two info panels, laid out at DITL
//!   2000's offsets as `_MapDialogUpdate` draws them.
//! - [`dialog`]: the in-flight maps' dialog, DITL 2000: where it sits,
//!   its map item and its frame.
//! - [`place`]: where the in-flight maps look, the original's `_mapZoom`
//!   and `_mapScroll`.
//! - [`clip`]: the in-flight maps' clip of what they draw to the map item.
//! - [`click`]: the in-flight maps' click: the hit box, and the press
//!   held for the map's owner to answer.
//! - [`buttons`]: the in-flight maps' six buttons, as `_DrawMapButtons`
//!   draws them and `_TrackMapButtons` tracks them.
//! - [`find`]: the map's Find dialog, `DLOG` 3005, as `_DoSystemSearch`
//!   opens it.

pub mod borders;
pub mod buttons;
pub mod catalog;
pub mod click;
pub mod clip;
pub mod data;
pub mod dialog;
pub mod find;
pub mod map;
pub mod model;
pub mod nebulae;
pub mod panel;
pub mod place;
pub mod update;
pub mod view;

#[cfg(test)]
mod testkit;

pub use catalog::{
    Galaxy, GalaxyCatalog, GovtId, NebulaEntry, NebulaId, NebulaPicture, StellarEntry, StellarId,
    SystemEntry, SystemId,
};
pub use click::{PendingPress, PressAnswer};
pub use find::{FindDialog, FindOutcome};
pub use map::{GalaxyMap, MapCycle, MapMarks, MapMode};
pub use model::{GalaxyModel, MapSystem, NEUTRAL};
pub use view::{MapView, Transform};
