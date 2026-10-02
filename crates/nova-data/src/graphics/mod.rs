//! Picture and sprite decoders: bytes in, RGBA8 images out.
//!
//! (Module documentation is completed once the decoders are in place.)

// The decoders that use these building blocks land in later commits.
#![allow(dead_code)]

mod budget;
mod error;
mod image;
mod packbits;
mod reader;

#[cfg(any(test, feature = "fixture"))]
pub mod fixture;

pub use budget::{MAX_PIXELS, MAX_PIXELS_PER_INPUT_BYTE};
pub use error::GraphicsError;
pub use image::Image;
