//! Picture and sprite decoders: bytes in, RGBA8 images out.
//!
//! (Module documentation is completed once the decoders are in place.)

// The decoders that use these building blocks land in later commits.
#![allow(dead_code)]

mod budget;
mod color;
mod error;
mod icon;
mod image;
mod packbits;
mod pict;
mod pixmap;
mod reader;

#[cfg(any(test, feature = "fixture"))]
pub mod fixture;

pub use budget::{MAX_PIXELS, MAX_PIXELS_PER_INPUT_BYTE};
pub use error::GraphicsError;
pub use icon::decode_cicn;
pub use image::Image;
pub use pict::decode_pict;
