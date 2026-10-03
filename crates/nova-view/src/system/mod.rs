//! The system view: one star system's stellars at their positions, over a
//! parallax starfield, seen through a camera the keyboard moves.
//!
//! - [`catalog`]: the [`SystemCatalog`] port, one system's contents
//!   already resolved and owned.
//! - [`data`]: the port's adapter over `nova_data`'s `GameData`.
//! - [`camera`]: the [`Camera`], which part of the system the screen shows,
//!   at Nova's own scale.
//! - [`starfield`]: the parallax starfield behind the stellars.
//! - [`animation`]: the [`Animation`], which frame of a stellar's sheet
//!   shows when, from its `spöb`'s animation fields.
//! - [`scene`]: the [`SystemScene`], the system read once and laid out:
//!   each stellar's world position, sheet and animation.
//! - [`view`]: the [`SystemView`] screen, which only reads the port.

pub mod animation;
pub mod camera;
pub mod catalog;
pub mod data;
pub mod scene;
pub mod starfield;
pub mod view;

pub use animation::Animation;
pub use camera::Camera;
pub use catalog::{
    AnimationData, StellarContents, StellarId, StellarSheet, SystemCatalog, SystemContents,
    SystemId,
};
pub use scene::{SceneStellar, SystemScene};
pub use view::SystemView;
