//! What the platform sends the app through its event loop from outside
//! the window: a [`LoopEvent`].
//!
//! The menu bar's picks ([`MenuPick`]) arrive this way on macOS, and the
//! updater's request to quit ([`LoopEvent::UpdaterQuit`]) on Windows,
//! where WinSparkle asks from a thread of its own before its installer
//! replaces `nova.exe`. The platform sends each through the loop's proxy,
//! so the app takes it on the main thread as the window event it converts
//! to.
//!
//! The updater's quit is the Quit command ([`MenuCommand::Quit`]), not a
//! window close: Quit quits at once and saves the pilot flying on every
//! screen, where a close on the main menu asks the original's quit
//! question, and a player who declined would leave the game running while
//! the installer replaces it.
//!
//! A new thing sent through the loop is a variant here, converted to the
//! window event the app already handles; the app gains no method. The
//! conversion is pinned by this module's tests and `app`'s updates tests.

use nova_view::MenuCommand;

use super::{MenuPick, WindowEvent};

/// An event the platform sends through the event loop.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoopEvent {
    /// A pick of the menu bar, or an Apple event taken as one.
    Menu(MenuPick),
    /// The updater asks the game to quit so its installer can replace it.
    UpdaterQuit,
}

/// A loop event is the window event the app takes it as.
impl From<LoopEvent> for WindowEvent {
    fn from(event: LoopEvent) -> Self {
        match event {
            LoopEvent::Menu(pick) => pick.into(),
            LoopEvent::UpdaterQuit => Self::Menu(MenuCommand::Quit),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_menu_event_is_its_pick_s_window_event() {
        for pick in MenuPick::ALL {
            assert_eq!(
                WindowEvent::from(LoopEvent::Menu(pick)),
                WindowEvent::from(pick),
                "{pick:?}"
            );
        }
    }

    #[test]
    fn the_updater_s_quit_is_the_quit_command() {
        assert_eq!(
            WindowEvent::from(LoopEvent::UpdaterQuit),
            WindowEvent::Menu(MenuCommand::Quit)
        );
    }
}
