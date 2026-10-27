//! The Win32 console calls the game makes, behind a safe API: whether the
//! process has a standard error handle ([`stderr_handle`]), and attaching
//! to the console of the process that started it
//! ([`attach_parent_console`]).
//!
//! The workspace forbids unsafe code; this crate is one of the places it
//! lives (its FFI module, `console`, allows it, every block with a
//! `SAFETY` comment), so the game itself keeps the ban. It decides
//! nothing: what to do with the answers is the game's (`nova`'s
//! `platform::console`). On any target but Windows the crate is empty,
//! and none of its dependencies are built.

// Off Windows the crate is empty, so the links above have nothing to name.
#![cfg_attr(not(windows), allow(rustdoc::broken_intra_doc_links))]
#![cfg(windows)]

mod console;

pub use console::{StdHandle, attach_parent_console, stderr_handle};
