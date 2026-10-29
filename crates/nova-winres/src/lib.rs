//! What nova's build script makes, built on any platform: the Windows
//! resources `nova.exe` embeds, and the app id every target's window
//! takes from nova's `Cargo.toml` ([`bundle_identifier`]).
//!
//! It also works out the update feed a Windows build compiles in, from
//! `NOVA_CHANNEL` and the committed public key
//! ([`windows_update_feed`]), and the version WinSparkle compares the
//! feed's with ([`windows_build_version`]).
//!
//! The Windows resources are the icon (an `.ico` made from the macOS
//! `.icns`, [`ico_from_icns`]), the version numbers ([`version_quad`]),
//! the resource script that names them and the manifest
//! ([`resource_script`]), and the linker arguments that go with them
//! ([`link_args`]).
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

mod app_id;
mod build_version;
mod ico;
mod link;
mod script;
mod update_feed;
mod version;

pub use app_id::{AppIdError, bundle_identifier};
pub use build_version::windows_build_version;
pub use ico::{ICON_SIZES, IconError, ico_from_icns};
pub use link::{MAIN_STACK_BYTES, link_args};
pub use script::{Resources, escape, resource_script};
pub use update_feed::{CHANNEL_VAR, FeedError, UpdateFeed, windows_update_feed};
pub use version::{VersionError, version_quad};

/// The icon group's resource id, shared by the resource script and the
/// window that loads it as its icon.
pub const ICON_ID: u16 = 1;

/// The manifest's resource id (`CREATEPROCESS_MANIFEST_RESOURCE_ID`): the
/// one the loader reads for an executable.
pub const MANIFEST_ID: u16 = 1;

/// The version information's resource id (`VS_VERSION_INFO`).
pub const VERSION_ID: u16 = 1;
