//! Typed EV Nova game records decoded from resource data.
//!
//! Each resource type is one plain struct, decoded declaratively with
//! `binrw` (`#[derive(BinRead)]`, big-endian) and serializable with `serde`.
//! Byte and offset handling lives only in the shared building blocks in
//! [`wire`]: Mac Roman strings, resource ID newtypes (`-1` becomes `None`),
//! flag words, points and rectangles, and raw bytes.
//!
//! # Example
//!
//! ```
//! use nova_data::records::spin::Spin;
//! use nova_data::{PictId, decode_bytes};
//!
//! // A spïn: sprites 1000, masks 1001, 48x48 frames in a 6x6 grid.
//! let bytes = [0x03, 0xE8, 0x03, 0xE9, 0, 48, 0, 48, 0, 6, 0, 6];
//! let decoded = decode_bytes::<Spin>(&bytes)?;
//! assert_eq!(decoded.consumed, 12);
//! assert_eq!(decoded.record.masks_id, Some(PictId(1001)));
//! assert_eq!((decoded.record.x_tiles, decoded.record.y_tiles), (6, 6));
//!
//! // One byte short: the error names the field and where it starts.
//! let err = decode_bytes::<Spin>(&bytes[..11]).unwrap_err();
//! assert_eq!(err.to_string(), "field Spin → y_tiles at byte 0xa: unexpected end of data");
//! # Ok::<(), nova_data::FieldError>(())
//! ```
//!
//! Whole files go through [`decode_file`], which decodes every registered
//! resource, keeps going past failures and reports each one with the
//! resource's type, ID, name, field path and byte offset ([`DecodeError`]).
//! Records longer than their layout decode with a [`DecodeWarning`]; shorter
//! ones are always errors.
//!
//! # Layout sources
//!
//! Field names and meanings come from the EV Nova Bible (Matt Burch's
//! resource reference). The Bible names fields but gives no byte widths or
//! offsets, and describes some fields in a different order from the bytes,
//! so byte order and widths come from the Nova templates shipped with the
//! open-source ResForge editor (`TMPB` resources 500-522). Every template
//! sums to exactly the size of every stock record of its type, and the
//! stock tests check cross-references between types. Each field's doc
//! comment gives its Bible name, offset and width.
//!
//! Bytes the Bible does not describe are kept as raw fields named by offset
//! (`unknown_0x18`), never given a meaning. Encoded fields (ranges such as
//! "128-2175 system ID, 10000+ government") stay raw integers with the
//! encoding documented; decoding them is left to later work. A field becomes
//! an ID newtype only when it is a plain reference to another resource.
//! Control-bit expressions are kept as text.
//!
//! # Record types
//!
//! | Code | Struct | Bible | Size | Stock |
//! |------|--------|-------|-----:|------:|
//! | `bööm` | [`records::boom::Boom`] | explosion type | 6 | 15 |
//! | `chär` | [`records::character::Character`] | character template | 362 | 1 |
//! | `cölr` | [`records::colors::Colors`] | interface colours | 244 | 1 |
//! | `crön` | [`records::cron::Cron`] | timed event | 822 | 125 |
//! | `csüm` | [`records::checksum::Checksum`] | (not described) | raw | 1 |
//! | `dësc` | [`records::desc::Desc`] | description | text + 36 | 3032 |
//! | `düde` | [`records::dude::Dude`] | ship group | 88 | 147 |
//! | `flët` | [`records::fleet::Fleet`] | fleet | 306 | 128 |
//! | `gövt` | [`records::govt::Govt`] | government | 192 | 68 |
//! | `ïntf` | [`records::interface::Interface`] | status bar | 166 | 7 |
//! | `jünk` | [`records::junk::Junk`] | special commodity | 676 | 23 |
//! | `mïsn` | [`records::mission::Mission`] | mission | 1970 | 791 |
//! | `nëbu` | [`records::nebula::Nebula`] | nebula | 534 | 4 |
//! | `öops` | [`records::disaster::Disaster`] | planetary disaster | 282 | 19 |
//! | `oütf` | [`records::outfit::Outfit`] | outfit item | 1028 | 242 |
//! | `përs` | [`records::person::Person`] | AI personality | 400 | 516 |
//! | `ränk` | [`records::rank::Rank`] | rank | 152 | 31 |
//! | `röid` | [`records::roid::Roid`] | asteroid type | 40 | 16 |
//! | `shän` | [`records::ship_anim::ShipAnim`] | ship animation | 192 | 288 |
//! | `shïp` | [`records::ship::Ship`] | ship class | 1860 | 288 |
//! | `spïn` | [`records::spin::Spin`] | sprite info | 12 | 136 |
//! | `spöb` | [`records::stellar::Stellar`] | stellar object | 1118 | 411 |
//! | `STR ` | [`records::string::StrResource`] | string (patch) | Pascal string | 0 |
//! | `STR#` | [`records::string_list::StrList`] | string list | counted list | 227 |
//! | `sÿst` | [`records::system::System`] | star system | 428 | 545 |
//! | `vers` | [`records::version::Version`] | (not described) | raw | 0 |
//! | `wëap` | [`records::weapon::Weapon`] | weapon | 134 | 81 |
//!
//! # Raw types
//!
//! These carry no documented gameplay content and serialize as lowercase
//! hex:
//!
//! - `csüm`: neither the Bible nor the templates describe it; the stock data
//!   holds one 4-byte resource that looks like a checksum.
//! - `vers`: Mac OS version information, absent from the stock data.
//!
//! # Out of scope
//!
//! [`OUT_OF_SCOPE`] lists the media types other phases own; [`decode_file`]
//! skips them (and unknown types) without error:
//!
//! - `PICT`, `rlëD`, `cicn`, `ppat`: pictures and sprites (phase 3).
//! - `snd `: sounds (phase 4).

pub mod decode;
pub mod error;
pub mod file;
pub mod graphics;
pub mod records;
pub mod registry;
pub mod wire;

#[cfg(test)]
mod testutil;

pub use decode::{Decoded, Entry, Record, TypedReport, decode, decode_all, decode_bytes};
pub use error::{Cause, DecodeError, DecodeWarning, FieldError, FieldPath};
pub use file::{FileReport, OUT_OF_SCOPE, decode_file};
pub use registry::{AnyDecoded, AnyRecord, TYPES, decode_any};
pub use wire::flags::{Flags16, Flags32, Flags64};
pub use wire::geometry::{Point, Rect};
pub use wire::id::*;
pub use wire::raw::{RawArray, RawBytes};
pub use wire::string::MacString;
