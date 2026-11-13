//! The system view: one star system's stellars at their positions, over a
//! parallax starfield, seen through a camera the keyboard moves.
//!
//! - [`catalog`]: the [`SystemCatalog`] port, one system's contents
//!   already resolved and owned.
//! - [`data`]: the port's adapter over `nova_data`'s `GameData`.
//! - [`camera`]: the [`Camera`], which part of the system the screen shows,
//!   at Nova's own scale.
//! - [`starfield`]: the parallax starfield behind the stellars.
//! - [`scene`]: the [`SystemScene`], the system read once and laid out:
//!   each stellar's world position and sheet, and the frame it shows: the
//!   one its flight's session reports, or, in the viewer, its own
//!   animation by the sim's rule (`nova_sim::stellar_frames`).
//! - [`view`]: the [`SystemView`] screen, which only reads the port, and
//!   draws a stellar the world models' table gives a model or a planet as
//!   flight does ([`SystemView::with_world_models`]).

pub mod camera;
pub mod catalog;
#[cfg(test)]
mod clipped_sheet_tests;
pub mod data;
#[cfg(test)]
mod eight_bit_sheet_tests;
#[cfg(test)]
mod pict_pair_tests;
pub mod scene;
pub mod starfield;
pub mod view;
#[cfg(test)]
mod wide_sheet_tests;

pub use camera::Camera;
pub use catalog::{
    StellarAnimation, StellarContents, StellarId, StellarSheet, SystemCatalog, SystemContents,
    SystemId,
};
pub use scene::{SceneStellar, ShownFrame, StellarLook, SystemScene};
pub use view::SystemView;
