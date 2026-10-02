//! Reader for classic Mac OS resource forks, as stored in EV Nova's `.ndat` files.
//!
//! This crate knows the resource fork format, not EV Nova: it hands out raw
//! type codes, IDs, names and data, and leaves their meaning to callers.
//!
//! # Format
//!
//! A flattened fork is a 16-byte header (data offset, map offset, data
//! length, map length; all big-endian `u32`), a data section of
//! length-prefixed blobs, and a resource map. The map holds a type list
//! (each type with its resource count and the offset of its reference list),
//! the reference lists (ID, name offset, attributes, 24-bit data offset) and
//! a name list of length-prefixed Mac Roman strings. Counts are stored minus
//! one. Bytes outside the header's two sections are ignored.
//!
//! [`ResourceFile::from_bytes`] validates all of it up front: any
//! out-of-bounds offset, duplicate type or ID, or compressed resource is a
//! [`ParseError`], never a panic or a silently dropped resource.
//!
//! # Example
//!
//! ```
//! use nova_rsrc::{ResType, ResourceFile};
//!
//! // One 'TEXT' resource, ID 128, unnamed, holding "hi".
//! let fork: Vec<u8> = [
//!     &[0, 0, 0, 16, 0, 0, 0, 22, 0, 0, 0, 6, 0, 0, 0, 50][..], // header
//!     &[0, 0, 0, 2, b'h', b'i'],                                 // data
//!     &[0; 24],                                                  // map header
//!     &[0, 28, 0, 50],          // type list and name list offsets
//!     &[0, 0],                  // one type (stored minus one)
//!     b"TEXT",
//!     &[0, 0, 0, 10],           // one resource; references at +10
//!     &[0, 128, 0xFF, 0xFF, 0], // ID 128, no name, no attributes
//!     &[0, 0, 0, 0, 0, 0, 0],   // data offset 0, reserved handle
//! ]
//! .concat();
//!
//! let file = ResourceFile::from_bytes(fork)?;
//! let text = ResType::from_mac_roman("TEXT").unwrap();
//! let resource = file.get(text, 128).unwrap();
//! assert_eq!(resource.data(), b"hi");
//! assert_eq!(resource.name(), None);
//! assert_eq!(file.types().collect::<Vec<_>>(), vec![text]);
//! # Ok::<(), nova_rsrc::ParseError>(())
//! ```
//!
//! Files on disk are opened with [`ResourceFile::open`], which reads through
//! the [`ForkReader`] port's [`StdForkReader`] adapter and also accepts a
//! real macOS resource fork (`..namedfork/rsrc`).
//!
//! # Design decision
//!
//! This crate parses resource forks with a small in-house strict parser
//! rather than wrapping the [`macbinary`](https://crates.io/crates/macbinary)
//! crate. `macbinary` is kept as a dev-dependency only, as an independent
//! second reading of the stock data in the `stock` tests.
//!
//! The decision came from a spike that ran `macbinary` 0.2.1 over the 21
//! stock `.ndat` files and probed its public API with corrupted forks:
//!
//! - On stock data `macbinary` reads well: across 30 types and 8,362
//!   resources, every type's iterated count equals the count declared in the
//!   file's type list, and the known totals match (288 `shïp`, 791 `mïsn`,
//!   545 `sÿst`, 671 `PICT`).
//! - Its strictness gaps cannot be closed from outside the crate. An
//!   out-of-bounds data entry silently ends the `Resources` iterator early
//!   (only a short count is visible, not which resource failed). An
//!   out-of-bounds reference list shows up only as a `(0, None)` size hint.
//!   An out-of-bounds *name* offset becomes `name: None`, which is
//!   indistinguishable from an unnamed resource. The attribute byte (and so
//!   the compressed flag), all offsets and the declared counts are private.
//!
//! Nova plug-ins are hand-edited, so malformed input must be reported rather
//! than dropped, and the format is small. Sending the `.ok() // FIXME` fixes
//! upstream to `macbinary` remains possible future work; if that lands, this
//! crate could switch to it behind the same public API.

mod error;
mod file;
mod fs;
mod parse;
mod res_type;

pub use error::{LoadError, ParseError, Section};
pub use file::{Resource, ResourceFile};
pub use fs::{Fork, ForkReader, StdForkReader};
pub use res_type::ResType;

#[cfg(any(test, feature = "fixture"))]
pub mod fixture;
