//! The AppImage: one file holding `nova` and phase 1's desktop file,
//! icons and metainfo, which runs on any x86-64 Linux with glibc
//! [`GLIBC_FLOOR`] or newer and installs nothing.
//!
//! ```text
//! <target>/appimage/staging/AppDir/
//!   AppRun -> usr/bin/nova
//!   <identifier>.desktop
//!   <identifier>.png          the 256 px icon
//!   .DirIcon -> <identifier>.png
//!   usr/bin/nova
//!   usr/share/...             the share/ tree (crate::linux::share)
//! <target>/appimage/out/
//!   ev-nova-x86_64-unknown-linux-gnu.AppImage
//! ```
//!
//! The AppDir is laid out by hand and packed with `appimagetool`, not
//! `linuxdeploy`: it bundles no library. The one library `nova` links
//! besides glibc is `libasound.so.2`, which must be the host's so it loads
//! the host's ALSA plugins (pipewire-alsa, pulse), and AppImage's own
//! excludelist leaves it out for that reason. Everything else (Vulkan,
//! Wayland, X11, xkbcommon, libdbus) wgpu, winit and rfd `dlopen` from the
//! host. [`links::check_links`] refuses a build that needs more.
//!
//! The floor comes from where the binary is built: an `ubuntu:22.04`
//! container (glibc 2.35) in CI. `nova-dump` stays in the tarball only.
//!
//! [`plan::plan`] is the sequence as data and [`plan::build_appimage`]
//! runs it; [`host`] reads the edge's arguments and refuses any host but
//! x86-64 Linux.

pub mod host;
pub mod layout;
pub mod links;
pub mod plan;

use std::path::PathBuf;

use crate::bundle::BundleError;
use crate::linux::OutputOverlapsWork;

/// The one target built and packed.
pub const TARGET: &str = "x86_64-unknown-linux-gnu";
/// The AppImage's file name: the nightly's per-target asset name.
pub const ASSET: &str = "ev-nova-x86_64-unknown-linux-gnu.AppImage";

/// The newest glibc the binary may need: 2.35, Ubuntu 22.04's, which
/// covers Ubuntu 22.04 and 24.04, Debian 12 and 13, and current Fedora and
/// openSUSE. `packaging/README.txt` records it.
pub const GLIBC_FLOOR: [u32; 2] = [2, 35];

/// The libraries the binary may link: glibc's (`libc`, `libm`, the
/// loader, and `libdl`, `libpthread` and `librt`, which glibc 2.34 folded
/// into `libc` but a build may still name), GCC's unwinder, and ALSA,
/// which must be the host's. Any other is a portability regression.
pub const ALLOWED_LIBRARIES: [&str; 8] = [
    "libc.so.6",
    "libm.so.6",
    "ld-linux-x86-64.so.2",
    "libdl.so.2",
    "libpthread.so.0",
    "librt.so.1",
    "libgcc_s.so.1",
    "libasound.so.2",
];

/// The variable naming the `appimagetool` to run.
pub const TOOL_VAR: &str = "NOVA_APPIMAGETOOL";
/// The variable naming the type2 runtime to embed.
pub const RUNTIME_VAR: &str = "NOVA_APPIMAGE_RUNTIME";

/// Why the AppImage could not be built.
#[derive(Debug, thiserror::Error)]
pub enum AppImageError {
    /// No runtime file was given: `appimagetool` would download one.
    #[error(
        "{RUNTIME_VAR} is unset: set it to a downloaded AppImage type2 runtime \
         (runtime-x86_64), so appimagetool never fetches one"
    )]
    NoRuntime,
    /// The runtime file given is not there.
    #[error("the AppImage runtime {} ({RUNTIME_VAR}) is missing", .0.display())]
    MissingRuntime(PathBuf),
    /// The output directory can't be used.
    #[error(transparent)]
    Layout(#[from] OutputOverlapsWork),
    /// `cargo metadata`, the build, a file step, the check or
    /// `appimagetool` failed.
    #[error(transparent)]
    Step(#[from] BundleError),
}
