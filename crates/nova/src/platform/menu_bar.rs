//! The macOS menu bar's adapter: `nova_appkit`'s commands as the game's
//! [`MenuCommand`]s, and its `MenuBar` as the core's [`MenuPort`].
//!
//! The menu bar takes its key equivalents before the window sees their
//! press, so each one must be the chord of the command it sends, or a
//! chord the game doesn't read. The tests check that against the game's
//! Command chords:
//!
//! - Command-Q and Command-comma are the game's own Quit and Preferences
//!   chords ([`menu_chord`](crate::app::screen::menu_chord)), and the
//!   menu sends the same commands.
//! - Command-K and Command-Option-K (jettison) and Command-X (eject) are
//!   no menu's key equivalents, so flight still reads them.
//! - Command-H, Command-Option-H and Command-M are no game chords. Before
//!   the menu bar they reached flight as a plain H (Hyperspace Mode) or M
//!   (the map) with Command held; now they hide and minimize the game, as
//!   the original's own menus did (its `main.nib`).
//! - Control-Command-F was the app's own fullscreen shortcut, which never
//!   reached the screen; the View menu's Enter Full Screen takes it now.

use nova_appkit::{Command, MenuBar};
use nova_view::MenuCommand;

use crate::app::MenuPort;

/// The game's command for the menu bar's `command`.
pub(super) fn menu_command(command: Command) -> MenuCommand {
    match command {
        Command::About => MenuCommand::About,
        Command::Preferences => MenuCommand::Preferences,
        Command::Quit => MenuCommand::Quit,
    }
}

/// The menu bar's command for the game's `command`.
fn appkit_command(command: MenuCommand) -> Command {
    match command {
        MenuCommand::About => Command::About,
        MenuCommand::Preferences => Command::Preferences,
        MenuCommand::Quit => Command::Quit,
    }
}

impl MenuPort for MenuBar {
    fn set_enabled(&mut self, command: MenuCommand, enabled: bool) {
        MenuBar::set_enabled(self, appkit_command(command), enabled);
    }
}

#[cfg(test)]
mod tests {
    use nova_appkit::{KeyEquivalent, Modifiers, key_equivalents};
    use nova_view::Key;
    use nova_view::flight::view::{EJECT_KEY, JETTISON_KEY};

    use super::*;
    use crate::app::screen::menu_chord;

    #[test]
    fn each_command_maps_to_the_games_and_back() {
        for command in Command::ALL {
            assert_eq!(appkit_command(menu_command(command)), command);
        }
        for command in MenuCommand::ALL {
            assert_eq!(menu_command(appkit_command(command)), command);
        }
        assert_eq!(menu_command(Command::Quit), MenuCommand::Quit);
        assert_eq!(menu_command(Command::Preferences), MenuCommand::Preferences);
        assert_eq!(menu_command(Command::About), MenuCommand::About);
    }

    /// The key a chord presses, as the menu bar's key equivalents name
    /// it.
    fn key_char(key: Key) -> char {
        match key {
            Key::Char(c) => c,
            other => panic!("{other:?} is no character key"),
        }
    }

    /// Every Command chord the game reads, and the menu command it is, if
    /// it is one. `_loadKeys` (@0xcd5f8-0xce6c8) reads each binding's
    /// `keyCode` alone, never its modifiers, so no stock key binding
    /// carries Command: these are the router's and flight's own chords.
    fn game_command_chords() -> Vec<(char, Modifiers, Option<MenuCommand>)> {
        let chord = |command| key_char(menu_chord(command).expect("a chord"));
        vec![
            (
                chord(MenuCommand::Quit),
                Modifiers::COMMAND,
                Some(MenuCommand::Quit),
            ),
            (
                chord(MenuCommand::Preferences),
                Modifiers::COMMAND,
                Some(MenuCommand::Preferences),
            ),
            (key_char(JETTISON_KEY), Modifiers::COMMAND, None),
            (key_char(JETTISON_KEY), Modifiers::COMMAND_OPTION, None),
            (key_char(EJECT_KEY), Modifiers::COMMAND, None),
        ]
    }

    #[test]
    fn every_menu_key_equivalent_is_its_commands_chord_or_no_game_chord() {
        let chords = game_command_chords();
        for KeyEquivalent { shortcut, command } in key_equivalents() {
            let game = chords.iter().find(|(key, modifiers, _)| {
                *key == shortcut.key && *modifiers == shortcut.modifiers
            });
            match game {
                Some((_, _, sends)) => assert_eq!(
                    command.map(menu_command),
                    *sends,
                    "{shortcut:?} takes a game chord for another command"
                ),
                None => assert_eq!(command, None, "{shortcut:?} sends a command with no chord"),
            }
        }
    }

    #[test]
    fn quit_and_preferences_are_the_menus_and_jettison_and_eject_stay_the_games() {
        let taken: Vec<(char, Modifiers)> = key_equivalents()
            .iter()
            .map(|equivalent| (equivalent.shortcut.key, equivalent.shortcut.modifiers))
            .collect();
        for (key, modifiers, command) in game_command_chords() {
            assert_eq!(
                taken.contains(&(key, modifiers)),
                command.is_some(),
                "{key:?} with {modifiers:?}"
            );
        }
    }
}
