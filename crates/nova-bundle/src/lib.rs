//! `nova-bundle`: packs the nightly's installers. Its `nova-bundle`
//! binary builds `Nova.app`, the macOS bundle of the game, and packs it
//! into a `.dmg`; its `nova-installer` binary packs the built Windows
//! executables into a per-user MSI ([`installer`]); its `nova-linux`
//! binary writes the Linux desktop file, AppStream metainfo and icons
//! ([`linux`]); its `nova-flatpak` binary packs the game into the Linux
//! Flatpak ([`linux::flatpak`]); its `nova-build-id` binary prints the build ID
//! ([`build_id`]); and its `nova-site` binary renders the download
//! page that links them all ([`site`]).
//!
//! # Usage
//!
//! ```text
//! mise run bundle -- [OUT_DIR]
//! ```
//!
//! On macOS only: it needs `lipo`, `codesign` and `hdiutil`. It builds
//! `nova` and `nova-dump` for Apple silicon (`aarch64-apple-darwin`; Intel
//! Macs are not supported) in release, stages `Nova.app` (its `Info.plist`
//! from the `nova` package's version and `[package.metadata.bundle]`
//! table, and its icon) with `nova-dump`, `README.txt`, `LICENSE` and an
//! `Applications` link beside it, checks with `lipo -archs` that both
//! binaries are arm64 alone, signs and verifies both, and writes
//! `ev-nova-aarch64-apple-darwin.dmg` to `OUT_DIR` (by default
//! `target/bundle/out`). The layout is [`layout`]'s, and the sequence
//! [`bundle::plan`].
//!
//! # Signing
//!
//! Six environment variables, named as the CI secrets that fill them
//! ([`signing::VARS`]), choose how:
//!
//! - `MACOS_CERT_P12_BASE64` and `MACOS_CERT_PASSWORD`: the Developer ID
//!   Application certificate and its key, as a base64 `.p12`, and its
//!   password.
//! - `MACOS_SIGNING_IDENTITY`: the `codesign` identity,
//!   `Developer ID Application: <name> (<team>)`.
//! - `APPLE_API_KEY_P8_BASE64`, `APPLE_API_KEY_ID` and
//!   `APPLE_API_ISSUER_ID`: the App Store Connect API key the notary
//!   service is reached with.
//!
//! With all six set, the bundler signs with Developer ID and the hardened
//! runtime from a temporary keychain, notarizes and staples `Nova.app`,
//! then makes the `.dmg` from it and signs, notarizes and staples that
//! ([`signing::developer_id`], [`notary`]). With none set (a local build,
//! a fork's pull request) it signs ad-hoc and notarizes nothing. Any other
//! mix is an error naming the missing variables.
//!
//! The core builds every argument list and runs them through the
//! [`ports::Process`] port, writes the staging tree through the
//! [`ports::Files`] port, and reads the signing variables through the
//! [`ports::Env`] port; [`system`] holds the real adapters, which `main`
//! wires together.
//!
//! # The Windows installer
//!
//! ```text
//! mise run build-windows
//! mise run pack-installer -- --exes <DIR> [OUT_DIR]
//! ```
//!
//! Or `mise run installer`, which runs both. On Windows only, with WiX v5
//! installed (`dotnet tool install --global wix --version 5.0.2`). The
//! first builds `nova.exe` and `nova-dump.exe` in release for
//! `x86_64-pc-windows-msvc`. The second, `nova-installer`, never builds:
//! it packs the executables in `<DIR>` (by default
//! `target/x86_64-pc-windows-msvc/release`) as they are, with `README.txt`
//! and `LICENSE`, into `ev-nova-x86_64-pc-windows-msvc.msi` in `OUT_DIR`
//! (by default `target/installer/out`), so the executables can be signed
//! between the two steps and the `.msi` after them. See [`installer`].

//!
//! # The Linux desktop files
//!
//! ```text
//! mise run linux-desktop -- [OUT_DIR]
//! ```
//!
//! On any host: `nova-linux` writes `<identifier>.desktop`,
//! `<identifier>.metainfo.xml` and the hicolor icons as a `share/` tree
//! in `OUT_DIR` (by default `target/linux`), which the player copies into
//! `~/.local/share`. The nightly's Linux tarball carries it. See
//! [`linux`]. The opt-in tarball test checks with `objdump -p` that the
//! tarball's `nova` needs no glibc above [`linux::links::GLIBC_FLOOR`]
//! and links only [`linux::links::ALLOWED_LIBRARIES`].
//!
//! # The Flatpak
//!
//! ```text
//! mise run flatpak -- [OUT_DIR]
//! ```
//!
//! On x86-64 Linux only. `nova-flatpak` vendors the crates, renders the
//! `flatpak-builder` manifest, builds it offline on the freedesktop
//! runtime and packs `ev-nova-x86_64-unknown-linux-gnu.flatpak` in
//! `OUT_DIR` (by default `target/flatpak/out`); it needs `flatpak`,
//! `flatpak-builder` and the Flathub remote added with `--user`.
//!
//! # The build ID
//!
//! ```text
//! mise run build-id
//! ```
//!
//! Every package names its build by HEAD's committer time in UTC,
//! `YYYYMMDDHHMMSS` ([`build_id::BuildId`]), or by `NOVA_BUILD_ID` when it
//! is set, which is how CI gives every leg of a run the same one
//! ([`build_id::resolve`]). It is `Info.plist`'s `CFBundleVersion` and the
//! AppStream metainfo's `<release>`, and each bundler compiles `nova` with
//! it in `NOVA_BUILD_ID`, so the game reports the build it is.
//! `nova-build-id` prints it, and exits 1 when it has none (no variable
//! and no git checkout).
//!
//! # The download page
//!
//! ```text
//! mise run site -- [OUT_DIR]
//! ```
//!
//! On any host: `nova-site` writes the download page, `index.html`, into
//! `OUT_DIR` (by default `target/site`), linking each nightly asset by the
//! name its packer gives it. CI's `pages` job deploys it at the GitHub
//! Pages site's root. `nova-site --archive <TARGET>` and
//! `--archive-dir <TARGET>` print the Windows zip's or Linux tarball's
//! file name or top-level directory ([`archive`]), which CI's packaging
//! uses instead of spelling them. See [`site`].

pub mod archive;
pub mod build_id;
pub mod bundle;
pub mod host;
pub mod icon;
pub mod info_plist;
pub mod installer;
pub mod layout;
pub mod linux;
pub mod notary;
pub mod ports;
pub mod signing;
pub mod site;
pub mod spec;
pub mod system;
#[cfg(test)]
mod test_support;
pub mod xml;
