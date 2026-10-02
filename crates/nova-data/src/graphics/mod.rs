//! Picture and sprite decoders: bytes in, RGBA8 images out.
//!
//! (Module documentation is completed once the decoders are in place.)

mod budget;
mod color;
mod error;
mod icon;
mod image;
mod packbits;
mod pattern;
mod pict;
mod pixmap;
mod reader;
mod rled;
mod sheet;

#[cfg(any(test, feature = "fixture"))]
pub mod fixture;

pub use budget::{MAX_PIXELS, MAX_PIXELS_PER_INPUT_BYTE};
pub use error::GraphicsError;
pub use icon::decode_cicn;
pub use image::Image;
pub use pattern::decode_ppat;
pub use pict::decode_pict;
pub use rled::decode_rled;
pub use sheet::{SheetLayout, SpriteSheet};
