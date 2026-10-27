//! The startup dialogs: the open panel that asks for the `Nova Files`
//! ([`NovaFilesDialog`]), the alert for a fatal startup error
//! ([`FatalAlert`]) and the sheet listing the startup warnings over the
//! game window ([`show_warnings`]).
//!
//! On macOS they are rfd's: an `NSOpenPanel` that can pick a folder or a
//! package (so both `EV Nova.app` and its `Nova Files`), and `NSAlert`s.
//! On Windows the alerts are rfd's too, each a `MessageBoxW` (rfd's
//! `common-controls-v6` feature is off; the manifest's Common Controls 6
//! dependency themes the box all the same). The warnings box is owned by
//! the game window, so it stays over it, and Windows takes no input in
//! the window until it is dismissed. It runs on a thread of its own, so
//! the game goes on drawing beneath. No first-party `unsafe` is added for
//! them. The Windows open panel comes with its own phase: until then, and
//! on Linux, nothing is picked ([`Dialogs`]), and on Linux nothing is shown either (stderr
//! already has every line).
//!
//! The panel and the fatal alert are modal, so they run only outside the
//! event loop (winit panics when its handler is entered again while it
//! runs). The warnings are a sheet (on Windows a box on its own thread),
//! started once the game window is up and the application is running,
//! which needs no nested run loop.

use std::path::PathBuf;

use winit::window::Window;

use crate::startup::{FatalAlert, NovaFilesDialog};

/// The fatal alert's title.
pub const FATAL_TITLE: &str = "EV Nova could not start";

/// The warnings sheet's title.
pub const WARNINGS_TITLE: &str = "EV Nova started with warnings";

/// The platform's startup dialogs.
#[derive(Clone, Copy, Debug, Default)]
pub struct Dialogs;

#[cfg(target_os = "macos")]
impl NovaFilesDialog for Dialogs {
    /// An open panel showing `prompt`, starting in `/Applications`.
    fn pick(&mut self, prompt: &str) -> Option<PathBuf> {
        rfd::FileDialog::new()
            .set_title(prompt)
            .set_directory("/Applications")
            .pick_file_or_folder()
    }
}

#[cfg(any(target_os = "macos", windows))]
impl FatalAlert for Dialogs {
    fn show_fatal(&mut self, message: &str) {
        let _ = rfd::MessageDialog::new()
            .set_level(rfd::MessageLevel::Error)
            .set_title(FATAL_TITLE)
            .set_description(message)
            .show();
    }
}

/// Starts a sheet over `window` listing the startup warnings in `body`,
/// and returns at once: the sheet stays until the player dismisses it,
/// while the game goes on beneath it.
#[cfg(target_os = "macos")]
pub fn show_warnings(window: &Window, body: &str) {
    // rfd's macOS backend begins the sheet as the future is made, and its
    // completion keeps the alert alive: the future itself is not needed.
    drop(
        rfd::AsyncMessageDialog::new()
            .set_level(rfd::MessageLevel::Warning)
            .set_title(WARNINGS_TITLE)
            .set_description(body)
            .set_parent(window)
            .show(),
    );
}

/// Shows a message box owned by `window` listing the startup warnings in
/// `body`, and returns at once: the box runs on a thread of its own until
/// the player dismisses it, while the game goes on beneath it.
///
/// rfd's async dialog can't be used here as on macOS: its Windows backend
/// spawns the `MessageBoxW` thread only when the future is first polled,
/// so a dropped future shows nothing. The blocking dialog takes the
/// window's handle here, on the event loop's thread, and rfd marks it
/// `Send`, so the thread only runs it.
#[cfg(windows)]
pub fn show_warnings(window: &Window, body: &str) {
    let dialog = rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Warning)
        .set_title(WARNINGS_TITLE)
        .set_description(body)
        .set_parent(window);
    std::thread::spawn(move || dialog.show());
}

#[cfg(not(target_os = "macos"))]
impl NovaFilesDialog for Dialogs {
    /// No dialog yet: nothing is picked.
    fn pick(&mut self, _prompt: &str) -> Option<PathBuf> {
        None
    }
}

#[cfg(not(any(target_os = "macos", windows)))]
impl FatalAlert for Dialogs {
    /// No alert yet: stderr and the log have the message.
    fn show_fatal(&mut self, _message: &str) {}
}

/// No sheet yet: stderr and the log have the warnings.
#[cfg(not(any(target_os = "macos", windows)))]
pub fn show_warnings(_window: &Window, _body: &str) {}
