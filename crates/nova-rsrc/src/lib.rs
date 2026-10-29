//! Reader for classic Mac OS resource forks, as stored in EV Nova's `.ndat`
//! files, for the Windows `.rez` files that hold the same resources, and
//! for MacBinary (`.bin`) files that wrap a fork.
//!
//! This crate knows these container formats, not EV Nova: it hands out raw
//! type codes, IDs, names and data, and leaves their meaning to callers.
//!
//! # Format
//!
//! ## Resource forks
//!
//! A flattened fork is a 16-byte header (data offset, map offset, data
//! length, map length; all big-endian `u32`), a data section of
//! length-prefixed blobs, and a resource map. The map holds a type list
//! (each type with its resource count and the offset of its reference list),
//! the reference lists (ID, name offset, attributes, 24-bit data offset) and
//! a name list of length-prefixed Mac Roman strings. Counts are stored minus
//! one. Bytes outside the header's two sections are ignored.
//!
//! ## `.rez` files
//!
//! A `.rez` file starts with a 24-byte header of little-endian `u32`s: the
//! magic `BRGR`, a group count (1), the header length (counted from the end
//! of these first 12 bytes), a group type (1), a base index (the index of
//! the first entry, 1 in practice) and an entry count. An entry table
//! follows: one 12-byte row per entry, `(offset, size, name offset)` as
//! little-endian `u32`s, with absolute offsets. The rest of the header is a
//! name table of NUL-terminated strings; name offsets count from byte 12.
//!
//! The last entry is the resource map, named `resource.map`. The map is
//! big-endian: a `u32` type-list offset and a `u32` type count; 12-byte type
//! entries (type code, `u32` resource-list offset, `u32` count), with both
//! offsets measured from the map's start; and 266-byte resource entries (a
//! `u32` entry index counted from the base index, the type code again, an
//! `i16` ID and a 256-byte NUL-terminated Mac Roman name, empty when the
//! resource is unnamed). Data blobs have no length prefix: each resource's
//! data is its entry's `size` bytes at `offset`, and identical data may be
//! shared by several resources. The format has no attributes, so
//! [`Resource::attributes`] is always 0 for a `.rez` resource.
//!
//! ## MacBinary
//!
//! Classic plug-ins were often shipped as MacBinary (`.bin`) files, which
//! the original's launcher decodes with Apple's `macbinary` tool before the
//! game starts. A MacBinary file is a 128-byte header, an optional
//! secondary header (its length, a big-endian `u16` at byte 120, rounded
//! up to 128 bytes), the data fork padded to a multiple of 128 bytes, then
//! the resource fork, whose trailing padding is optional. The fork lengths
//! are big-endian `u32`s at bytes 83 and 87.
//!
//! A header is accepted as Apple's tool accepts it:
//!
//! - It is at least 128 bytes long and bytes 0, 74 and 82 are zero.
//! - If the CRC-16/XMODEM of bytes 0 to 123 equals the `u16` at byte 124,
//!   it is MacBinary II, or III when bytes 102 to 105 are `mBIN`. The
//!   version bytes 122 and 123 are ignored.
//! - Otherwise it is MacBinary I when bytes 101 to 125 are all zero, the
//!   name length (byte 1) is 63 or less, and the two forks are not both
//!   empty. A name length of 0 is accepted, and fork lengths have no cap.
//!
//! Only the fork with the resources is kept; the Finder metadata is not
//! read. As the original's `_EVOpenResourceFile` (0xa5287) opens the
//! decoded file's data fork first and falls back to its resource fork on
//! any failure, a data fork that parses as a flattened fork wins, and the
//! resource fork is read otherwise. Either is read only as a flattened
//! fork, never as `.rez` or nested MacBinary. [`ResourceFile::format`]
//! says which was read.
//!
//! ## Format detection
//!
//! [`ResourceFile::from_bytes`] picks the parser from the content, never
//! from a file extension:
//!
//! 1. Bytes starting with the `BRGR` magic are read as `.rez`. The magic
//!    cannot misfire on a fork: there it would be a data offset of over
//!    1 GB.
//! 2. Bytes that parse as a flattened fork are a fork.
//! 3. Only if that parse fails are the bytes checked for a MacBinary
//!    header. Bytes that are not MacBinary keep the fork's error.
//!
//! The order means no fork can be misread as MacBinary, whatever its byte
//! 102 holds. Nor can a MacBinary file parse as a fork: its name length
//! (byte 1) puts the fork's data offset at 64 KiB or more, and the
//! zero-padded name makes the map length 0. The stock `.ndat` files all
//! start `00 00 01 00`, which fails the MacBinary header checks anyway.
//!
//! A buffer shorter than 128 bytes is recognised as a truncated MacBinary
//! header only by the III signature: 106 to 127 bytes with byte 0 zero
//! and `mBIN` at byte 102. A shorter or unsigned one keeps its fork error.
//!
//! [`ResourceFile::load`] applies the same rule to a file on disk: its
//! data fork wins if it parses, and otherwise its real resource fork is
//! read.
//!
//! ## Strictness
//!
//! [`ResourceFile::from_bytes`] validates all of it up front: any
//! out-of-bounds offset, duplicate type or ID, overlapping reference
//! lists, compressed fork resource, or `.rez` resource that names a missing
//! entry, the map entry, the wrong type or an unterminated name is a
//! [`ParseError`], never a panic or a silently dropped resource. So is a
//! MacBinary fork that runs past the end of the file, a truncated III
//! header, a III header with a bad CRC, or a MacBinary file with no
//! resource fork. A malformed fork inside MacBinary is
//! [`ParseError::MacBinaryResourceFork`], whose
//! [`source`](std::error::Error::source) is the fork's own error.
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
//! The same resource as a `.rez` file reads the same way:
//!
//! ```
//! use nova_rsrc::{ResType, ResourceFile};
//!
//! let le = u32::to_le_bytes;
//! let rez: Vec<u8> = [
//!     &b"BRGR"[..],
//!     &le(1), &le(49), &le(1), &le(1), &le(2), // 1 group of type 1, base 1, 2 entries
//!     &le(61), &le(2), &le(0),                 // entry 1: the data
//!     &le(63), &le(286), &le(36),              // entry 2: the map, named at 12 + 36
//!     b"resource.map\0",                       // name table
//!     b"hi",                                   // data, with no length prefix
//!     &[0, 0, 0, 8, 0, 0, 0, 1],               // map: type list at +8, one type
//!     b"TEXT", &[0, 0, 0, 20, 0, 0, 0, 1],     // one resource, listed at +20
//!     &[0, 0, 0, 1], b"TEXT", &[0, 128],       // entry 1, ID 128
//!     &[0; 256],                               // no name
//! ]
//! .concat();
//!
//! let file = ResourceFile::from_bytes(rez)?;
//! let text = ResType::from_mac_roman("TEXT").unwrap();
//! assert_eq!(file.get(text, 128).unwrap().data(), b"hi");
//! # Ok::<(), nova_rsrc::ParseError>(())
//! ```
//!
//! Files on disk are opened with [`ResourceFile::open`], which reads through
//! the [`ForkReader`] port's [`StdForkReader`] adapter and also accepts a
//! real macOS resource fork (`..namedfork/rsrc`).
//!
//! # Design decisions
//!
//! No crate on crates.io reads `.rez` files, so that parser is in-house
//! too. Its reference is `RezFormat.swift` in
//! [ResForge](https://github.com/andrews05/ResForge), itself based on
//! Burgerlib's `brrezfile.cpp`; this parser adds the strictness checks
//! above.
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
//!
//! The MacBinary *container* is read in-house too, not with `macbinary`'s
//! `detect` and `parse`. A spike ran them over synthetic headers (valid I,
//! II and III, and corrupted variants) and the 21 stock `.ndat` files. They
//! never panic or truncate silently, but:
//!
//! - A truncated header can't be told apart from "not MacBinary" (both are
//!   `BadVersion`).
//! - Every overrun (data fork, resource fork, padding or secondary header)
//!   is the same untyped `BadEof`, and the header's fork lengths are
//!   private, so an error can't say which fork or how long.
//! - `detect` returns III for any buffer of 128 bytes or more with `mBIN`
//!   at byte 102 and a zero first byte, checking neither the CRC nor bytes
//!   74 and 82, so a fork holding `mBIN` there is misdetected.
//! - Its acceptance differs from Apple's tool, which the original's
//!   launcher runs: it rejects name lengths of 0 and of 32 to 63.
//!
//! The container is 128 bytes, a CRC and two padded lengths, so this crate
//! reads it with errors that name the defect and hands the fork to its own
//! strict parser. `macbinary` checks the MacBinary fixture builder (CRC,
//! padding, fork placement) in this crate's tests.

mod error;
mod file;
mod fs;
mod macbin;
mod parse;
mod res_type;
mod rez;

pub use error::{LoadError, ParseError, Section};
pub use file::{Format, Resource, ResourceFile};
pub use fs::{Fork, ForkReader, StdForkReader};
pub use res_type::ResType;

#[cfg(any(test, feature = "fixture"))]
pub mod fixture;
