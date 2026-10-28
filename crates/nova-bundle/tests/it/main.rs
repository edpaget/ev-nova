//! nova-bundle's integration tests, built as one test binary.
//!
//! Each file under `tests/it/` is a module of this binary. Add new
//! integration tests as a module here, not as a new `tests/*.rs` file:
//! every `tests/*.rs` file is a separate binary to link.

mod env;
mod files;
mod flatpak_metadata;
#[cfg(target_os = "linux")]
mod linux_appimage;
mod linux_desktop;
#[cfg(target_os = "linux")]
mod linux_flatpak;
#[cfg(target_os = "macos")]
mod macos_tools;
mod process;
mod site;
#[cfg(windows)]
mod windows_installer;
