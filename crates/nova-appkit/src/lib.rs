//! The macOS menu bar and Apple event handlers, built with `AppKit` behind
//! a safe API: the app menu, View and Window, as a Mac game's standard
//! menus, and the quit and Preferences events the system sends.
//!
//! The workspace forbids unsafe code; this crate is one of the places it
//! lives (its two FFI modules, `bar` and `apple_event_handler`, allow it,
//! every block with a `SAFETY` comment), so the game itself keeps the ban. On any target but macOS
//! the crate is empty, and none of its dependencies are built.
//!
//! - [`MENUS`]: the menus and their items as plain data, each item's
//!   title, key equivalent and action.
//! - [`MenuBar`]: the real menu bar, built from them
//!   ([`MenuBar::install`]), and the game's word on its commands
//!   ([`MenuBar::set_enabled`]).
//! - [`Command`]: the game-visible items (About, Preferences…, Quit), the
//!   crate's own vocabulary, so it depends on nothing of the game's.
//! - [`key_equivalents`]: every key the menu bar takes before the game's
//!   window, for the game to check against its own keys.
//! - [`APPLE_EVENTS`]: the Apple events the game handles as plain data,
//!   each event's class and ID and the [`Command`] it sends, and
//!   [`AppleEvents`], their real handlers ([`AppleEvents::install`]).
//! - [`abort_modal`]: ends the modal `AppKit` is running, such as the
//!   startup's open panel, for a quit event that arrives while it is up.
//!
//! The adapters only build items, register handlers and forward picks and
//! events. Quit, from the menu or as the quit Apple event (the Dock's
//! Quit, logout, `osascript`'s `quit app`), sends the game's quit, never
//! `AppKit`'s `terminate:`, which would end the process with no chance to
//! save. Enter Full Screen is `AppKit`'s
//! `toggleFullScreen:`, sent up the responder chain, so `AppKit` titles it
//! Enter or Exit Full Screen and Globe+F works too; the window reports
//! the change as any other.
//!
//! The original's own menus (its Carbon `main.nib`) have these items, and
//! more the crate leaves out: its Edit menu, whose Cut would take the
//! game's eject chord, Command-X, before the window; Register EV Nova and
//! Check for Updates, the Ambrosia shareware's; and Arrange in Front.

// Off macOS the crate is empty, so the links above have nothing to name.
#![cfg_attr(not(target_os = "macos"), allow(rustdoc::broken_intra_doc_links))]
#![cfg(target_os = "macos")]

mod apple_event_handler;
mod apple_events;
mod bar;
mod items;
mod modal;

pub use apple_event_handler::AppleEvents;
pub use apple_events::{APPLE_EVENTS, AppleEvent, command_for, four_char_code};
pub use bar::{InstallError, MenuBar};
pub use items::{
    Action, Command, Item, KeyEquivalent, MENUS, Menu, Modifiers, Role, Shortcut, Title,
    key_equivalents,
};
pub use modal::abort_modal;
