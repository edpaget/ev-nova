//! The developer tools overlay's model, free of any GUI toolkit.
//!
//! The developer tools are a window drawn over the running game, shown and
//! hidden with [`TOGGLE_KEY`] (backquote). While it shows, it takes every
//! key and pointer event, so typing into it never moves the game; the
//! toggle key's own events reach neither. Everything it decides lives here
//! and is tested here:
//!
//! - [`overlay`]: the [`DevOverlay`], its visibility, where each input goes
//!   ([`Routing`]) and the frame times measured from injected elapsed time.
//! - [`frame_time`]: the [`FrameTimes`] readout.
//! - [`catalog`]: the [`ResourceCatalog`] port, every resource and one
//!   resource's bytes, files and record, owned.
//! - [`data`]: the port's adapter over `nova_data`'s `GameData`.
//!
//! egui draws the overlay only in the `nova` crate's `dev-tools` build,
//! and only renders this model's strings and images.

pub mod catalog;
pub mod data;
pub mod frame_time;
pub mod overlay;

pub use catalog::{
    Origin, RecordView, ResType, ResourceCatalog, ResourceDetail, ResourceSummary, SourceInfo,
    type_code,
};
pub use frame_time::{FRAME_WINDOW, FrameTimes};
pub use overlay::{DevOverlay, Routing, TOGGLE_KEY};
