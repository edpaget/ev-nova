//! The commands a platform's menu bar sends the game.
//!
//! On macOS the game's app menu offers About, Preferences… and Quit, as
//! the original's does (its Carbon nib's `abou`, `pref` and the quit
//! command). The platform adapter only builds the items and forwards a
//! pick as a [`MenuCommand`]; what each does, and whether it can be picked
//! now, is the screen's to say ([`Screen::menu_command`],
//! [`Screen::menu_enabled`]).
//!
//! [`Screen::menu_command`]: crate::Screen::menu_command
//! [`Screen::menu_enabled`]: crate::Screen::menu_enabled

/// A game-visible item of the platform's menu bar.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MenuCommand {
    /// About the game.
    About,
    /// Preferences…, the Preferences dialog.
    Preferences,
    /// Quit.
    Quit,
}

impl MenuCommand {
    /// Every command, in the order the app menu lists them.
    pub const ALL: [Self; 3] = [Self::About, Self::Preferences, Self::Quit];
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_command_is_listed_once_in_the_app_menus_order() {
        assert_eq!(
            MenuCommand::ALL,
            [
                MenuCommand::About,
                MenuCommand::Preferences,
                MenuCommand::Quit
            ]
        );
    }
}
