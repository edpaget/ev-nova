//! The Windows resources `nova.exe` embeds, built by nova's build script
//! on any platform: the icon (an `.ico` made from the macOS `.icns`,
//! [`ico_from_icns`]), the version numbers ([`version_quad`]) and the
//! resource script that names them and the manifest ([`resource_script`]).
//!
//! The crate is plain, safe Rust with no platform code, so every decision
//! the build script makes is tested everywhere. The script itself only
//! reads the files, writes what these functions return, and hands the
//! script to the resource compiler.
//!
//! The ids are the ones Windows looks for: the icon group Explorer shows
//! is the first, and the game's window loads it by [`ICON_ID`]; the
//! manifest is `CREATEPROCESS_MANIFEST_RESOURCE_ID`; the version
//! information is `VS_VERSION_INFO`.

mod ico;
mod script;
mod version;

pub use ico::{ICON_SIZES, IconError, ico_from_icns};
pub use script::{Resources, escape, resource_script};
pub use version::{VersionError, version_quad};

/// The icon group's resource id, shared by the resource script and the
/// window that loads it as its icon.
pub const ICON_ID: u16 = 1;

/// The manifest's resource id (`CREATEPROCESS_MANIFEST_RESOURCE_ID`): the
/// one the loader reads for an executable.
pub const MANIFEST_ID: u16 = 1;

/// The version information's resource id (`VS_VERSION_INFO`).
pub const VERSION_ID: u16 = 1;
