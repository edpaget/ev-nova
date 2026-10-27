//! `nova-bundle`: builds `Nova.app`, the macOS bundle of the game, and
//! packs it into a `.dmg` for the nightly.
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
//! binaries are arm64 alone, ad-hoc signs and verifies both, and writes
//! `ev-nova-aarch64-apple-darwin.dmg` to `OUT_DIR` (by default
//! `target/bundle/out`). The layout is [`layout`]'s, and the sequence
//! [`bundle::steps`].
//!
//! The core builds every argument list and runs them through the
//! [`ports::Process`] port, and writes the staging tree through the
//! [`ports::Files`] port; [`system`] holds the real adapters, which `main`
//! wires together.

pub mod bundle;
pub mod host;
pub mod icon;
pub mod info_plist;
pub mod layout;
pub mod ports;
pub mod signing;
pub mod spec;
pub mod system;
