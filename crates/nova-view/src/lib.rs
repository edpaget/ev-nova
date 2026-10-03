//! The renderer-free vocabulary that EV Nova's screens produce and consume.
//!
//! A screen turns [`Input`] into state changes and draws itself into a
//! [`DrawList`] of [`DrawCommand`]s in logical coordinates, naming images by
//! [`ImageKey`]. `nova-render` turns draw lists into pixels; this crate knows
//! nothing about windows or GPUs.

pub mod color;
pub mod draw;
pub mod geometry;
pub mod image;
pub mod input;
pub mod screen;

pub use color::Color;
pub use draw::{DrawCommand, DrawList};
pub use geometry::Point;
pub use image::{ImageKey, ImageKind};
pub use input::{Input, Key, MouseButton};
pub use screen::{Screen, ScreenAction};
