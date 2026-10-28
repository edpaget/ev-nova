//! nova-bundle's integration tests, built as one test binary.
//!
//! Each file under `tests/it/` is a module of this binary. Add new
//! integration tests as a module here, not as a new `tests/*.rs` file:
//! every `tests/*.rs` file is a separate binary to link.

mod appcast;
mod build_id;
mod env;
mod files;
mod flatpak_metadata;
mod linux_desktop;
#[cfg(target_os = "linux")]
mod linux_flatpak;
#[cfg(target_os = "linux")]
mod linux_tarball;
#[cfg(target_os = "macos")]
mod macos_tools;
mod process;
mod signer;
mod site;
#[cfg(windows)]
mod windows_installer;
