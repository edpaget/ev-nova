//! Picture and sprite decoders: bytes in, RGBA8 images out.
//!
//! Four pure decoders turn one resource's bytes into [`Image`]s
//! (row-major, top-down, straight-alpha RGBA8). They do no resource lookup
//! and depend on nothing renderer- or file-format-specific:
//!
//! | Type | Decoder | Output |
//! |------|---------|--------|
//! | `PICT` ([`PICT`]) | [`decode_pict`] | one image the size of `picFrame` |
//! | `cicn` | [`decode_cicn`] | one image, the mask as alpha |
//! | `ppat` | [`decode_ppat`] | one opaque image |
//! | `rlëD` ([`RLED`]) | [`decode_rled`] | a [`SpriteSheet`] of equal-sized frames |
//!
//! [`apply_mask`] gives a decoded picture its alpha from a second picture,
//! its mask, as Nova masks its button caps.
//!
//! A sprite sheet's header gives frame size and count but not how the frames
//! are arranged. That comes from the `spïn` or `shän` record pointing at the
//! sheet ([`records::spin::Spin::sheet_layout`],
//! [`records::ship_anim::ShipAnim::sheet_layout`]), or from
//! [`SheetLayout::DEFAULT`]. The layout only affects
//! [`SpriteSheet::compose`]; the frames are the same either way.
//!
//! [`records::spin::Spin::sheet_layout`]: crate::records::spin::Spin::sheet_layout
//! [`records::ship_anim::ShipAnim::sheet_layout`]: crate::records::ship_anim::ShipAnim::sheet_layout
//!
//! # Sources
//!
//! - `PICT` version 2, `PixMap`, `ColorTable`: Apple, *Inside Macintosh:
//!   Imaging With QuickDraw* (1994), appendix A "Picture Opcodes" (opcode
//!   table, data lengths, the version 2 header, word alignment) and ch. 4
//!   "Color QuickDraw" (the `PixMap` record, `packType`, the `ColorTable`
//!   record).
//! - PackBits: Apple Technical Note TN1023 "Understanding PackBits"; its
//!   worked example is a test vector.
//! - `cicn`: *Imaging With QuickDraw* ch. 4, the `CIcon` record and "The
//!   Color Icon Resource".
//! - `ppat`: *Imaging With QuickDraw* ch. 4, the `PixPat` record and "The
//!   Pixel Pattern Resource".
//! - `rlëD`: the EV Nova Bible for meaning, and the open-source ResForge
//!   editor's (MIT) decoder as a second reading of the byte layout and
//!   token semantics, including the pixel-run token (see [`decode_rled`]).
//!   No code was copied.
//! - Layouts: the Bible's `spïn` (`xTiles`, "Horizontal grid dimension")
//!   and `shän` (`FramesPer`, "The number of frames for one rotation";
//!   `BaseSetCount`, "The number of sprite sets").
//!
//! # Stock data
//!
//! A survey of the 21 stock `.ndat` files found, and the stock tests check:
//!
//! - 671 `PICT`s, all version 2, each a single `DirectBitsRect` covering
//!   the whole frame: 598 16-bit (word PackBits) and 73 32-bit (byte
//!   PackBits of red, green and blue planes). Their opcodes are only
//!   HeaderOp, DefHilite, Clip, LongComment, DirectBitsRect and EndPic.
//! - 282 `rlëD`s, all 16-bit, using tokens 0 to 4.
//! - 29 `cicn`s (2, 4 and 8-bit) and 10 `ppat`s (8-bit, 64x64), all
//!   indexed through sparse colour tables looked up by `value`.
//!
//! # Supported `PICT` opcodes
//!
//! `0000` NOP, `0001` Clip (skipped, not applied), `001E` DefHilite, `00A0`
//! ShortComment, `00A1` LongComment, `0C00` HeaderOp, `00FF` EndPic, and the
//! pixel opcodes `009A` DirectBitsRect (16-bit `packType` 0 or 3; 32-bit
//! with three components, `packType` 0 or 4) and `0098` PackBitsRect (1, 2,
//! 4 or 8-bit indexed, which plug-ins commonly use). Pixel opcodes must copy
//! without scaling, in srcCopy or ditherCopy mode; several of them compose
//! into one picture, and pixels none of them cover stay transparent. Every
//! other opcode is [`GraphicsError::UnsupportedOpcode`], naming the opcode
//! and its offset, rather than an approximation.
//!
//! # Bounded work
//!
//! Before allocating its output, every decoder checks the pixels its header
//! claims against [`MAX_PIXELS`] and [`MAX_PIXELS_PER_INPUT_BYTE`] times the
//! input length, so a tiny hostile resource cannot ask for gigabytes. Every
//! loop is bounded by the input length or by already-checked dimensions,
//! and every read either advances or fails, so no input can hang a decoder.
//!
//! # Dependencies
//!
//! This module depends only on `std` and `thiserror`, never on the record
//! side of the crate (`records`, `wire`, `decode`, `error`, `file`); the
//! records depend on it for [`SheetLayout`]. Keeping the dependency one-way
//! keeps a later move into its own crate mechanical.
//!
//! The `fixture` feature exposes the `fixture` module, builders for synthetic
//! pictures and sprites, to other crates' tests.

mod budget;
mod color;
mod error;
mod icon;
mod image;
mod mask;
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
pub use mask::apply_mask;
pub use pattern::decode_ppat;
pub use pict::{PICT, decode_pict};
pub use rled::{RLED, decode_rled};
pub use sheet::{SheetLayout, SpriteSheet};
