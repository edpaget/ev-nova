//! The macOS menu bar's adapter: `nova_appkit`'s commands as the game's
//! [`MenuPick`]s (its [`MenuCommand`]s, and Check for Updates…), and its
//! `MenuBar` as the core's [`MenuPort`]. The Apple events `nova_appkit`
//! handles send the same commands, mapped the same way: the quit event is
//! the menu's Quit.
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

use crate::app::{MenuPick, MenuPort};

/// The game's pick for the menu bar's `command`.
pub(super) fn pick(command: Command) -> MenuPick {
    match command {
        Command::About => MenuPick::Command(MenuCommand::About),
        Command::Preferences => MenuPick::Command(MenuCommand::Preferences),
        Command::Quit => MenuPick::Command(MenuCommand::Quit),
        Command::CheckForUpdates => MenuPick::CheckForUpdates,
    }
}

/// The menu bar's command for the game's `pick`.
fn appkit_command(pick: MenuPick) -> Command {
    match pick {
        MenuPick::Command(MenuCommand::About) => Command::About,
        MenuPick::Command(MenuCommand::Preferences) => Command::Preferences,
        MenuPick::Command(MenuCommand::Quit) => Command::Quit,
        MenuPick::CheckForUpdates => Command::CheckForUpdates,
    }
}

impl MenuPort for MenuBar {
    fn set_enabled(&mut self, pick: MenuPick, enabled: bool) {
        MenuBar::set_enabled(self, appkit_command(pick), enabled);
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
    fn each_game_command_is_its_pick_and_back() {
        for command in Command::ALL {
            assert_eq!(appkit_command(pick(command)), command);
        }
        for game in MenuPick::ALL {
            assert_eq!(pick(appkit_command(game)), game);
        }
        assert_eq!(pick(Command::Quit), MenuPick::Command(MenuCommand::Quit));
        assert_eq!(
            pick(Command::Preferences),
            MenuPick::Command(MenuCommand::Preferences)
        );
        assert_eq!(pick(Command::About), MenuPick::Command(MenuCommand::About));
    }

    #[test]
    fn check_for_updates_is_a_pick_of_its_own_and_no_game_command() {
        assert_eq!(pick(Command::CheckForUpdates), MenuPick::CheckForUpdates);
        assert_eq!(
            appkit_command(MenuPick::CheckForUpdates),
            Command::CheckForUpdates
        );
        for command in MenuCommand::ALL {
            assert_ne!(
                appkit_command(MenuPick::Command(command)),
                Command::CheckForUpdates
            );
        }
    }

    #[test]
    fn the_quit_apple_event_is_the_menus_quit_and_pref_its_preferences() {
        let sent: Vec<_> = nova_appkit::APPLE_EVENTS
            .iter()
            .map(|event| (event.id, pick(event.command)))
            .collect();
        assert_eq!(
            sent,
            [
                (*b"quit", MenuPick::Command(MenuCommand::Quit)),
                (*b"pref", MenuPick::Command(MenuCommand::Preferences))
            ]
        );
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
                    command.map(pick),
                    sends.map(MenuPick::Command),
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
