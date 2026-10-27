//! The Linux desktop integration: the desktop file, the AppStream
//! metainfo and the hicolor icons, written as a `share/` tree that
//! installs into `~/.local/share` (or `/usr/share`).
//!
//! ```text
//! <OUT_DIR>/share/
//!   applications/<identifier>.desktop
//!   metainfo/<identifier>.metainfo.xml
//!   icons/hicolor/<n>x<n>/apps/<identifier>.png
//! ```
//!
//! Each file is named for `[package.metadata.bundle] identifier`, the
//! same key the game's window takes its Wayland app id and X11 `WM_CLASS`
//! from (`nova::platform::APP_ID`), so docks and task switchers match the
//! running window to the desktop file and show its icon and name.
//!
//! [`desktop::desktop_entry`] and [`metainfo::metainfo`] render the files
//! from the [`BundleSpec`](crate::spec::BundleSpec), as
//! [`info_plist`](crate::info_plist) renders `Info.plist`; [`share`] lays
//! the tree out and writes it through the [`Files`](crate::ports::Files)
//! port. Nothing here needs a Linux host: the `nova-linux` binary runs
//! anywhere.

pub mod desktop;
pub mod host;
pub mod icons;
pub mod metainfo;
pub mod share;

use crate::spec::GAME_PACKAGE;

/// The command the desktop file starts and the metainfo says the game
/// provides: the game package's binary, found on `PATH`.
pub const EXECUTABLE: &str = GAME_PACKAGE;

/// The desktop file's menu categories: a game (the main category), and a
/// role-playing one. Not the Apple category in the spec, which is a
/// different vocabulary.
pub const CATEGORIES: &str = "Game;RolePlaying;";
