//! Draws `nova-view` draw lists: sprites and pictures from texture atlases,
//! text, lines and dots, in a fixed logical space letterboxed into the
//! window.
//!
//! # Core and adapters
//!
//! The core decides everything and never names a `wgpu` or `glyphon` type:
//!
//! - [`viewport`]: the logical-to-window transform and letterbox.

pub mod viewport;

pub use viewport::{LOGICAL, Letterbox, LogicalSize, PixelRect, Viewport};
