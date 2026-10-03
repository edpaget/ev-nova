//! The system view: one star system's stellars at their positions, over a
//! parallax starfield, seen through a camera the keyboard moves.
//!
//! - [`catalog`]: the [`SystemCatalog`] port, one system's contents
//!   already resolved and owned.
//! - [`data`]: the port's adapter over `nova_data`'s `GameData`.
//! - [`camera`]: the [`Camera`], which part of the system the screen shows,
//!   at Nova's own scale.

pub mod camera;
pub mod catalog;
pub mod data;

pub use camera::Camera;
pub use catalog::{
    AnimationData, StellarContents, StellarId, StellarSheet, SystemCatalog, SystemContents,
    SystemId,
};
