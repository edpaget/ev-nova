//! Typed EV Nova game records decoded from resource data.

pub mod wire;

pub use wire::flags::{Flags16, Flags32, Flags64};
pub use wire::id::*;
pub use wire::raw::{RawArray, RawBytes};
pub use wire::string::MacString;
