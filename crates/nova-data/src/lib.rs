//! Typed EV Nova game records decoded from resource data.

pub mod decode;
pub mod error;
pub mod wire;

pub use decode::{Decoded, Entry, Record, TypedReport, decode, decode_all, decode_bytes};
pub use error::{Cause, DecodeError, DecodeWarning, FieldError, FieldPath};
pub use wire::flags::{Flags16, Flags32, Flags64};
pub use wire::id::*;
pub use wire::raw::{RawArray, RawBytes};
pub use wire::string::MacString;
