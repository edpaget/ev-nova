//! The startup dialogs: the open panel that asks for the `Nova Files`
//! ([`NovaFilesDialog`]), the alert for a fatal startup error
//! ([`FatalAlert`]) and the sheet listing the startup warnings over the
//! game window ([`show_warnings`]).
//!
//! On macOS they are rfd's: an `NSOpenPanel` that can pick a folder or a
//! package (so both `EV Nova.app` and its `Nova Files`), and `NSAlert`s.
//! On Windows they are rfd's too: the panel is a folder picker
//! (`IFileOpenDialog` with `FOS_PICKFOLDERS`), and either the `EV Nova`
//! folder or its `Nova Files` can be picked, since the lookup tries both
//! (`crate::startup::locate`). Each alert is a `MessageBoxW` (rfd's
//! `common-controls-v6` feature is off; the manifest's Common Controls 6
//! dependency themes the box all the same). The warnings box is owned by
//! the game window, so it stays over it, and Windows takes no input in
//! the window until it is dismissed. It runs on a thread of its own, so
//! the game goes on drawing beneath.
//!
//! On Linux they are rfd's `xdg-portal` backend. The panel is the XDG
//! desktop portal's file chooser picking a folder (its `FileChooser`
//! `OpenFile`, reached over a `dlopen`ed libdbus), so, as on Windows, the
//! `EV Nova` folder or its `Nova Files` is picked; without libdbus or a
//! portal service, rfd runs `zenity` for it instead, whose folder picker
//! shows no title, so a re-ask first shows its prompt in a warning box.
//! The alerts (and that box) have no portal: rfd always runs
//! `zenity --error` or `--warning`, so where zenity isn't installed (many
//! KDE desktops, the freedesktop Flatpak runtime) nothing is shown and rfd
//! only logs it. stderr and the startup log still have every line. The warnings box runs on a thread of its
//! own, as on Windows.
//!
//! No first-party `unsafe` is added for them. On any other system nothing
//! is picked or shown ([`Dialogs`]): stderr already has every line.
//!
//! The panel and the fatal alert are modal, so they run only outside the
//! event loop (winit panics when its handler is entered again while it
//! runs). The warnings are a sheet (on Windows a box on its own thread),
//! started once the game window is up and the application is running,
//! which needs no nested run loop.

use std::path::PathBuf;

use winit::window::Window;

use crate::startup::locate::PROMPT;
use crate::startup::{FatalAlert, NovaFilesDialog};

/// The fatal alert's title.
pub const FATAL_TITLE: &str = "EV Nova could not start";

/// The title of the box that, on Linux, says why the folder picker is
/// asking again.
pub const PICK_TITLE: &str = "No EV Nova data there";

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

#[cfg(windows)]
impl NovaFilesDialog for Dialogs {
    /// A folder picker titled `prompt`, opening where Windows last left
    /// it.
    ///
    /// It runs on the calling thread, the main one: the lookup asks
    /// before the event loop runs and before any window or audio is up,
    /// so a blocking modal dialog is safe there, as the fatal box is. The
    /// warnings box needs a thread of its own only because the event
    /// loop is running by then. rfd sets COM up for the call
    /// (`CoInitializeEx`, apartment-threaded); should that fail, as on a
    /// thread already in the multithreaded apartment, no picker shows
    /// and the call reads as a cancel.
    fn pick(&mut self, prompt: &str) -> Option<PathBuf> {
        rfd::FileDialog::new().set_title(prompt).pick_folder()
    }
}

#[cfg(target_os = "linux")]
impl NovaFilesDialog for Dialogs {
    /// The file-chooser portal picking a folder, titled `prompt`; zenity's
    /// when there is no portal, which shows no title (rfd 0.17's zenity
    /// `pick_folder` passes none).
    ///
    /// So that a re-ask after a pick that held no data says why the picker
    /// is back, its prompt ([`retry_notice`]) is shown first in a warning
    /// box. rfd can't say which path it will take, so the portal's picker
    /// gets the box too, ahead of its own title.
    ///
    /// It runs on the calling thread before the event loop and any window,
    /// as on the other platforms, so it has no parent window.
    fn pick(&mut self, prompt: &str) -> Option<PathBuf> {
        if let Some(notice) = retry_notice(prompt) {
            let _ = rfd::MessageDialog::new()
                .set_level(rfd::MessageLevel::Warning)
                .set_title(PICK_TITLE)
                .set_description(notice)
                .show();
        }
        rfd::FileDialog::new().set_title(prompt).pick_folder()
    }
}

#[cfg(any(target_os = "macos", windows, target_os = "linux"))]
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

/// Shows zenity's warning box listing the startup warnings in `body`, and
/// returns at once: the box runs on a thread of its own until the player
/// dismisses it, while the game goes on beneath it.
///
/// As on Windows, rfd's async dialog is a lazy future here, so a dropped
/// one shows nothing; and zenity takes no parent window, so none is set.
#[cfg(target_os = "linux")]
pub fn show_warnings(_window: &Window, body: &str) {
    let dialog = rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Warning)
        .set_title(WARNINGS_TITLE)
        .set_description(body);
    std::thread::spawn(move || dialog.show());
}

#[cfg(not(any(target_os = "macos", windows, target_os = "linux")))]
impl NovaFilesDialog for Dialogs {
    /// No dialog: nothing is picked.
    fn pick(&mut self, _prompt: &str) -> Option<PathBuf> {
        None
    }
}

#[cfg(not(any(target_os = "macos", windows, target_os = "linux")))]
impl FatalAlert for Dialogs {
    /// No alert: stderr and the log have the message.
    fn show_fatal(&mut self, _message: &str) {}
}

/// No sheet: stderr and the log have the warnings.
#[cfg(not(any(target_os = "macos", windows, target_os = "linux")))]
pub fn show_warnings(_window: &Window, _body: &str) {}

/// The notice to show before the folder picker asks with `prompt`: the
/// prompt itself when it re-asks after a pick that held no data, `None`
/// when it is the first ask ([`PROMPT`]).
///
/// Linux shows it in a warning box because zenity's folder picker, rfd's
/// fallback where there is no portal, shows no title, so the re-ask would
/// otherwise reopen the picker without saying why.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn retry_notice(prompt: &str) -> Option<&str> {
    (prompt != PROMPT).then_some(prompt)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_ask_has_no_notice() {
        assert_eq!(retry_notice(PROMPT), None);
    }

    #[test]
    fn a_re_ask_shows_its_prompt_as_the_notice() {
        let prompt = format!("/home/p/Games holds no EV Nova data. {PROMPT}");
        assert_eq!(retry_notice(&prompt), Some(prompt.as_str()));
    }
}
