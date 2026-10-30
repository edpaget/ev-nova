//! The platform's menu bar, in the core's terms.
//!
//! The menu bar's adapter (macOS's, in `platform`) only builds the items
//! and forwards a pick, a [`MenuPick`]: one of the game's commands, or
//! Check for Updates…, which is the app's, not the screen's. A command
//! arrives as [`WindowEvent::Menu`], which the app hands the screen as
//! [`Screen::menu_command`] and then treats as it treats an input's answer
//! (the viewport refitted, the audio fed, the warnings kept, and an exit
//! when the screen quits). It skips the developer overlay: a menu command
//! is not a key, and the overlay never sees the key equivalents the menu
//! bar takes. With the app in the background it asks for one redraw too,
//! to show what the command opened. Check for Updates… arrives as
//! [`WindowEvent::CheckForUpdates`], which asks the updater to check now
//! and never reaches the screen.
//!
//! Which items the menu bar offers is the app's to say
//! ([`App::menu_enabled`]): each command as the screen says
//! ([`Screen::menu_enabled`]), and Check for Updates… when the updater
//! can check. The adapter's [`MenuPort`] is told each change through a
//! [`MenuSync`], which the runner asks after every event.
//!
//! A new item the game handles is a [`MenuPick`] variant (or a
//! [`MenuCommand`] one, when the screen answers it): the port gains no
//! method, its adapter stays one call, and the pick's routing and offer
//! are decided here and pinned by `app`'s menu tests.
//!
//! [`WindowEvent::Menu`]: super::WindowEvent::Menu
//! [`WindowEvent::CheckForUpdates`]: super::WindowEvent::CheckForUpdates

use nova_render::ImageSource;
use nova_view::{MenuCommand, Screen};

use super::{App, Control, WindowEvent, WindowPort};

/// One item of the menu bar the game handles, as the adapter forwards a
/// pick of it and is told whether to offer it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MenuPick {
    /// A command the screen answers.
    Command(MenuCommand),
    /// Check for Updates…: the updater checks now.
    CheckForUpdates,
}

impl MenuPick {
    /// Every pick: the commands in [`MenuCommand::ALL`]'s order, then
    /// Check for Updates….
    pub const ALL: [Self; MenuCommand::ALL.len() + 1] = {
        let mut all = [Self::CheckForUpdates; MenuCommand::ALL.len() + 1];
        let mut index = 0;
        while index < MenuCommand::ALL.len() {
            all[index] = Self::Command(MenuCommand::ALL[index]);
            index += 1;
        }
        all
    };
}

/// A pick is the window event the app takes it as.
impl From<MenuPick> for WindowEvent {
    fn from(pick: MenuPick) -> Self {
        match pick {
            MenuPick::Command(command) => Self::Menu(command),
            MenuPick::CheckForUpdates => Self::CheckForUpdates,
        }
    }
}

/// What the core needs of a platform's menu bar.
pub trait MenuPort {
    /// Offers `pick`'s item (`enabled`), or greys it.
    fn set_enabled(&mut self, pick: MenuPick, enabled: bool);
}

/// Keeps a menu bar's items offered as the app says: what it last told
/// the menu bar, so it tells only each change, and everything the first
/// time.
#[derive(Clone, Debug, Default)]
pub struct MenuSync {
    told: Option<[bool; MenuPick::ALL.len()]>,
}

impl MenuSync {
    /// A sync that has told the menu bar nothing yet.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Tells `menu` each pick whose offer `app` has changed since the last
    /// sync ([`App::menu_enabled`]), in [`MenuPick::ALL`]'s order: every
    /// pick on the first.
    pub fn sync<S: ImageSource, C: Screen>(&mut self, app: &App<S, C>, menu: &mut impl MenuPort) {
        let now = MenuPick::ALL.map(|pick| app.menu_enabled(pick));
        for (index, pick) in MenuPick::ALL.into_iter().enumerate() {
            if self.told.is_none_or(|told| told[index] != now[index]) {
                menu.set_enabled(pick, now[index]);
            }
        }
        self.told = Some(now);
    }
}

impl<S: ImageSource, C: Screen> App<S, C> {
    /// Whether the menu bar offers `pick`: a command when the screen does,
    /// and Check for Updates… when the updater can check.
    pub fn menu_enabled(&self, pick: MenuPick) -> bool {
        match pick {
            MenuPick::Command(command) => self.screen.menu_enabled(command),
            MenuPick::CheckForUpdates => self.updater.can_check(),
        }
    }

    /// Check for Updates…: the updater checks now; the screen never hears
    /// of it.
    pub(super) fn check_for_updates(&mut self) -> Control {
        self.updater.check_now();
        Control::Continue
    }

    /// A menu bar pick: the screen's answer to `command`, then what follows
    /// an input's, and a redraw in the background (see the module docs).
    pub(super) fn route_menu(
        &mut self,
        command: MenuCommand,
        window: &mut impl WindowPort,
    ) -> Control {
        let action = self.screen.menu_command(command);
        if self.background {
            window.request_redraw();
        }
        self.after_input(action)
    }
}
