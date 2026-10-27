//! nova-bundle's integration tests, built as one test binary.
//!
//! Each file under `tests/it/` is a module of this binary. Add new
//! integration tests as a module here, not as a new `tests/*.rs` file:
//! every `tests/*.rs` file is a separate binary to link.

mod env;
mod files;
#[cfg(target_os = "macos")]
mod macos_tools;
mod process;
#[cfg(windows)]
mod windows_installer;
