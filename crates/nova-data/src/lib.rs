//! Typed EV Nova game records decoded from resource data.

pub mod decode;
pub mod error;
pub mod file;
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
pub use wire::id::*;
pub use wire::raw::{RawArray, RawBytes};
pub use wire::string::MacString;
