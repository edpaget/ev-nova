//! The menu bar's commands ([`MenuCommand`]) through the router, and
//! which of them the menu bar offers.
//!
//! - **Quit and Preferences…** are their Command chords
//!   ([`menu_chord`]): the command feeds the chord through
//!   [`Screen::input`] exactly as its keys arrive, so every gate and hold
//!   the keys meet applies to it unchanged (Quit's delay, the entry's and
//!   the appear's held chords, a keyed click's hold, the overlays, the
//!   spaceport and a map's Find). With Command already held (the chord's
//!   key equivalent was typed: the window saw Command go down, and the menu
//!   bar took the chord's press) it feeds the chord's press and release;
//!   picked with the mouse, Command's press and release around them. The
//!   router's notion of Command stays the physical one either way.
//! - **About** opens the About text over the side shown, as I does, where
//!   nothing holds input and nothing is in front ([`AppScreen::front`]).
//!   The original's `abou` command (`_AppEventHandler` @0xccb83, the
//!   `abou` compare @0xccc1d, its branch @0xcccb0) is gated by nothing in
//!   the game; it shows Ambrosia's About box, which the engine doesn't
//!   have (rdm:phase/macos-platform-fidelity/phase-1-original-menu-items).
//!   It plays no click: the original's menus don't.
//!
//! The menu bar greys a command where it would do nothing: Quit never;
//! Preferences… while dialogs are missing, a quit is pending, or nothing
//! holds input and something in front takes it; About likewise, and
//! also while anything holds input.

use nova_view::{Input, Key, MenuCommand, Screen, ScreenAction};

use super::{AppScreen, menu_chord};

impl AppScreen {
    /// `command`, picked from the menu bar: see the module docs.
    pub(super) fn run_menu_command(&mut self, command: MenuCommand) -> ScreenAction {
        let Some(chord) = menu_chord(command) else {
            if self.menu_enabled_now(command) {
                self.open_about();
            }
            return ScreenAction::None;
        };
        let around = !self.command_held;
        let command_key = |pressed| Input::Key {
            key: Key::Command,
            pressed,
            repeat: false,
        };
        let chord_key = |pressed| Input::Key {
            key: chord,
            pressed,
            repeat: false,
        };
        let mut inputs = Vec::with_capacity(4);
        if around {
            inputs.push(command_key(true));
        }
        inputs.extend([chord_key(true), chord_key(false)]);
        if around {
            inputs.push(command_key(false));
        }
        // The app quits at the first input that asks, as it would for the
        // keys.
        for input in &inputs {
            if self.input(input) == ScreenAction::Quit {
                return ScreenAction::Quit;
            }
        }
        ScreenAction::None
    }

    /// Whether the menu bar offers `command` now: see the module docs.
    pub(super) fn menu_enabled_now(&self, command: MenuCommand) -> bool {
        if command == MenuCommand::Quit {
            return true;
        }
        if self.dialogs.is_none() || self.quit_in.is_some() || self.quit_due {
            return false;
        }
        let holds = self.holds_input();
        let taken = self.front().is_none();
        match command {
            MenuCommand::Preferences => holds || taken,
            MenuCommand::About => !holds && taken,
            MenuCommand::Quit => true,
        }
    }

    /// Whether the router holds a Command chord typed now, to act on it
    /// later: the entry holds it for the handover, the main menu's appear
    /// until it ends, and a keyed click until its sound ends.
    fn holds_input(&self) -> bool {
        self.entry_holds_commands() || self.appear_holds_menu() || self.menu_hold.is_some()
    }
}
