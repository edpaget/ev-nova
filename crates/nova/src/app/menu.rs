//! The platform's menu bar, in the core's terms.
//!
//! The menu bar's adapter (macOS's, in `platform`) only builds the items
//! and forwards a pick. A pick arrives as [`WindowEvent::Menu`], which the
//! app hands the screen as [`Screen::menu_command`] and then treats as it
//! treats an input's answer (the viewport refitted, the audio fed, the
//! warnings kept, and an exit when the screen quits). It skips the
//! developer overlay: a menu command is not a key, and the overlay never
//! sees the key equivalents the menu bar takes.
//!
//! Which items the menu bar offers is the screen's to say
//! ([`Screen::menu_enabled`]). The adapter's [`MenuPort`] is told each
//! change through a [`MenuSync`], which the runner asks after every event.
//!
//! [`WindowEvent::Menu`]: super::WindowEvent::Menu

use nova_render::ImageSource;
use nova_view::{MenuCommand, Screen};

use super::{App, Control};

/// What the core needs of a platform's menu bar.
pub trait MenuPort {
    /// Offers `command` (`enabled`), or greys it.
    fn set_enabled(&mut self, command: MenuCommand, enabled: bool);
}

/// Keeps a menu bar's items offered as the screen says: what it last told
/// the menu bar, so it tells only each change, and everything the first
/// time.
#[derive(Clone, Debug, Default)]
pub struct MenuSync {
    told: Option<[bool; MenuCommand::ALL.len()]>,
}

impl MenuSync {
    /// A sync that has told the menu bar nothing yet.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Tells `menu` each command whose offer `screen` has changed since
    /// the last sync, in [`MenuCommand::ALL`]'s order: every command on
    /// the first.
    pub fn sync(&mut self, screen: &impl Screen, menu: &mut impl MenuPort) {
        let now = MenuCommand::ALL.map(|command| screen.menu_enabled(command));
        for (index, command) in MenuCommand::ALL.into_iter().enumerate() {
            if self.told.is_none_or(|told| told[index] != now[index]) {
                menu.set_enabled(command, now[index]);
            }
        }
        self.told = Some(now);
    }
}

impl<S: ImageSource, C: Screen> App<S, C> {
    /// A menu bar pick: the screen's answer to `command`, then what follows
    /// an input's (see the module docs).
    pub(super) fn route_menu(&mut self, command: MenuCommand) -> Control {
        let action = self.screen.menu_command(command);
        self.after_input(action)
    }
}
