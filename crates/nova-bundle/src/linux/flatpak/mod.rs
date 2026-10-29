//! The Flatpak: a single-file bundle of the game on the freedesktop
//! runtime, built from source offline by `flatpak-builder`, as Flathub
//! builds.
//!
//! ```text
//! <target>/flatpak/
//!   staging/<identifier>.json   the manifest (manifest::manifest)
//!   staging/cargo-config.toml   the vendored-sources replacement
//!   staging/vendor/             `cargo vendor`'s tree
//!   staging/share/...           the share/ tree (crate::linux::share)
//!   build/                      flatpak-builder's build directory
//!   repo/                       the OSTree repository it exports to, on the
//!                               channel's branch ([`branch`])
//!   state/                      its cache (.flatpak-builder)
//! <target>/flatpak/out/
//!   ev-nova-x86_64-unknown-linux-gnu.flatpak   the plain bundle: no repo
//! ```
//!
//! That bundle names no repository, so it can't update. On `main`, the
//! [`repo`] publisher imports it into the GPG-signed repository the Pages
//! site serves, with its own work, pages and dist trees, and builds the
//! bundle the nightly ships, which follows the repo.
//!
//! The manifest is rendered here, not committed, so its app id and
//! command come from the spec and can't drift from the desktop file's
//! name. Its `nova` module builds from two `dir` sources (the workspace,
//! and the vendored crates) with no network, which Flathub requires;
//! moving to Flathub swaps those two for a git source and a generated
//! `cargo-sources.json`, and keeps the runtime, the permissions and the
//! rest.
//!
//! It ships no zenity: the runtime's `/usr/bin/zenity` (zenity 3.44,
//! GTK3) is on the sandbox's `PATH`, which is where rfd's message boxes
//! (the startup alerts) look for it.
//!
//! [`plan::plan`] is the sequence as data and [`plan::build_flatpak`]
//! runs it; [`host`] reads the edge's arguments and refuses any host but
//! x86-64 Linux. `NOVA_CHANNEL` names the branch it builds onto.

pub mod host;
pub mod layout;
pub mod manifest;
pub mod plan;
pub mod repo;

use crate::appcast::Channel;
use crate::bundle::BundleError;
use crate::linux::OutputOverlapsWork;

/// The runtime: the freedesktop one, which has every library the game
/// needs (GTK3 and zenity for the alerts, ALSA routed to `PulseAudio`,
/// libdbus for the portal), two years of support per branch, and asks
/// KDE players to download no GNOME.
pub const RUNTIME: &str = "org.freedesktop.Platform";
/// The SDK the module builds with.
pub const SDK: &str = "org.freedesktop.Sdk";
/// The runtime's branch: the newest stable one, whose Rust extension has
/// rustc 1.98, the workspace's `rust-version`.
pub const RUNTIME_VERSION: &str = "26.08";
/// The SDK extension giving `cargo` and `rustc`.
pub const RUST_EXTENSION: &str = "org.freedesktop.Sdk.Extension.rust-stable";
/// Where the extension puts `cargo`.
pub const RUST_BIN: &str = "/usr/lib/sdk/rust-stable/bin";
/// The module's name, and so its build directory's in the sandbox
/// (`/run/build/<name>`).
pub const MODULE: &str = "nova";

/// The sandbox's permissions, as `finish-args`: the fewest the game
/// needs.
///
/// - `--socket=wayland`, `--socket=fallback-x11`: the window, on Wayland
///   first and on X11 only when there is no Wayland.
/// - `--share=ipc`: X11's MIT-SHM, which X11 clients need to draw at a
///   usable speed.
/// - `--device=dri`: the GPU, for wgpu (Vulkan or GL).
/// - `--socket=pulseaudio`: sound. The runtime's ALSA sends the default
///   device to the pulse plugin, so kira/cpal's ALSA output reaches the
///   Pulse socket, which `PipeWire` serves on current desktops. No
///   `--device=snd` or `--device=all`.
/// - `--filesystem=~/.wine:ro`, `--filesystem=~/Games/ev-nova:ro`,
///   `--filesystem=~/Games/EV Nova:ro`: where the game looks for the
///   `Nova Files` (a Wine prefix, Lutris's prefix, a copied folder), read
///   only. Anywhere else the player picks through the file-chooser
///   portal, which grants that folder alone and keeps the grant.
/// - `--persist=.local/state`: the startup log. flatpak 1.13.1 and later
///   set `XDG_STATE_HOME` to `~/.var/app/<id>/.local/state`; before that
///   (Ubuntu 22.04's 1.12) the game falls back to `~/.local/state`, which
///   this maps onto the same folder.
///
/// Nothing else: no home or host filesystem, no network (the game makes
/// no network calls), no session bus (the portals are always reachable).
pub const FINISH_ARGS: [&str; 9] = [
    "--socket=wayland",
    "--socket=fallback-x11",
    "--share=ipc",
    "--device=dri",
    "--socket=pulseaudio",
    "--filesystem=~/.wine:ro",
    "--filesystem=~/Games/ev-nova:ro",
    "--filesystem=~/Games/EV Nova:ro",
    "--persist=.local/state",
];

/// The remote the runtime comes from, as `flatpak-builder
/// --install-deps-from` names it.
pub const REMOTE: &str = "flathub";
/// Flathub's repository file, which the bundle names so installing it
/// offers to pull the runtime from there.
pub const RUNTIME_REPO: &str = "https://dl.flathub.org/repo/flathub.flatpakrepo";
/// The bundle's file name: the nightly's per-target asset name.
pub const ASSET: &str = "ev-nova-x86_64-unknown-linux-gnu.flatpak";

/// The branch flatpak builds for: the channel's name, or flatpak's own
/// default, `master`, for a development build. `flatpak-builder
/// --default-branch` exports onto it, and `flatpak build-bundle` must be
/// told it, or it looks for `master`.
#[must_use]
pub fn branch(channel: Option<Channel>) -> &'static str {
    channel.map_or("master", Channel::name)
}

/// Why the Flatpak could not be built.
#[derive(Debug, thiserror::Error)]
pub enum FlatpakError {
    /// The output directory can't be used.
    #[error(transparent)]
    Layout(#[from] OutputOverlapsWork),
    /// `cargo metadata`, `cargo vendor`, a file step, `flatpak-builder`
    /// or `flatpak build-bundle` failed.
    #[error(transparent)]
    Step(#[from] BundleError),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_branch_is_the_channel_s_name_or_flatpak_s_default() {
        assert_eq!(branch(Some(Channel::Nightly)), "nightly");
        assert_eq!(branch(None), "master");
    }
}
