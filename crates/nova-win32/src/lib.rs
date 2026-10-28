//! The Win32 calls the game makes, behind a safe API.
//!
//! - The console: whether the process has a standard error handle
//!   ([`stderr_handle`]), and attaching to the console of the process that
//!   started it ([`attach_parent_console`]).
//! - WinSparkle, the Windows updater: [`WinSparkleDll`] loads
//!   `WinSparkle.dll` from a path at run time and offers each of its calls
//!   the game makes as a safe method, the shutdown request as a Rust
//!   closure.
//!
//! The workspace forbids unsafe code; this crate is one of the places it
//! lives (its FFI modules, `console` and `winsparkle`, allow it, every
//! block with a `SAFETY` comment), so the game itself keeps the ban. It
//! decides nothing: what to do with the answers, and when to call
//! WinSparkle, is the game's (`nova`'s `platform::console` and
//! `nova::winsparkle`). On any target but Windows the crate is empty, and
//! none of its dependencies are built.

// Off Windows the crate is empty, so the links above have nothing to name.
#![cfg_attr(not(windows), allow(rustdoc::broken_intra_doc_links))]
#![cfg(windows)]

mod console;
mod winsparkle;

pub use console::{StdHandle, attach_parent_console, stderr_handle};
pub use winsparkle::{
    FILE as WINSPARKLE_FILE, LoadError as WinSparkleLoadError,
    ShutdownRequest as WinSparkleShutdown, WinSparkleDll,
};
