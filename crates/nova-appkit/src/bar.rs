//! The FFI: the real menu bar, built from [`MENUS`] with `AppKit`, and the
//! target its command items send their picks to.
//!
//! Every `unsafe` block here carries a `SAFETY` comment. Building the menus
//! needs the main thread and a running `NSApplication`, so the crate's
//! unit tests don't run it; `tests/it` does, on its binary's main thread.

#![allow(unsafe_code)]

use std::fmt;

use objc2::rc::Retained;
use objc2::runtime::{AnyObject, Sel};
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, sel};
use objc2_app_kit::{NSApplication, NSMenu, NSMenuItem, NSWindow};
use objc2_foundation::{NSObject, NSString};

use crate::items::{Action, Command, Item, MENUS, Role, Shortcut};

/// Why the menu bar or the Apple event handlers could not be installed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InstallError {
    /// They were asked for off the main thread, where `AppKit` can't be
    /// used.
    NotMainThread,
}

impl fmt::Display for InstallError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotMainThread => f.write_str("AppKit can only be used on the main thread"),
        }
    }
}

impl std::error::Error for InstallError {}

/// The target's state: what it forwards each pick to.
struct TargetIvars {
    on_command: Box<dyn Fn(Command)>,
}

define_class!(
    // SAFETY:
    // - NSObject has no subclassing requirements.
    // - `MenuTarget` does not implement `Drop`.
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "NovaMenuTarget"]
    #[ivars = TargetIvars]
    struct MenuTarget;

    impl MenuTarget {
        /// A command item was picked: its tag says which command, and an
        /// unknown tag is ignored. The closure only forwards, so `AppKit`'s
        /// callback never runs the game.
        // SAFETY: `menuCommand:` takes the sender, an `NSMenuItem` (only
        // command items, which this crate builds, send it), and returns
        // nothing, as the signature says.
        #[unsafe(method(menuCommand:))]
        fn menu_command(&self, sender: &NSMenuItem) {
            if let Some(command) = Command::from_tag(sender.tag()) {
                (self.ivars().on_command)(command);
            }
        }
    }
);

impl MenuTarget {
    fn new(mtm: MainThreadMarker, on_command: Box<dyn Fn(Command)>) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(TargetIvars { on_command });
        // SAFETY: NSObject's `init` on a freshly allocated instance whose
        // ivars are set.
        unsafe { msg_send![super(this), init] }
    }
}

/// The installed menu bar: the app menu, View and Window ([`MENUS`]).
///
/// It keeps the target the command items send their picks to, which
/// `AppKit` holds only weakly, so it must outlive the menus' use: keep it
/// for the whole run. Dropping it unhooks the command items, which then do
/// nothing.
pub struct MenuBar {
    _target: Retained<MenuTarget>,
    commands: Vec<(Command, Retained<NSMenuItem>)>,
}

impl fmt::Debug for MenuBar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MenuBar")
            .field(
                "commands",
                &self
                    .commands
                    .iter()
                    .map(|(command, _)| command)
                    .collect::<Vec<_>>(),
            )
            .finish_non_exhaustive()
    }
}

impl MenuBar {
    /// Builds the menu bar for an app named `app_name` and makes it the
    /// application's, calling `on_command` with each game command picked.
    ///
    /// It turns off automatic window tabbing first, so `AppKit` adds no tab
    /// items to View, and sets the Window and Services menus as `AppKit`'s.
    /// It needs the main thread, after `NSApplication` has finished
    /// launching (winit's `resumed`); off it, it builds nothing.
    ///
    /// `on_command` runs inside `AppKit`'s menu handling: it should only
    /// pass the command on (to an event loop proxy, say), never run the
    /// game.
    pub fn install(
        app_name: &str,
        on_command: impl Fn(Command) + 'static,
    ) -> Result<Self, InstallError> {
        let mtm = MainThreadMarker::new().ok_or(InstallError::NotMainThread)?;
        NSWindow::setAllowsAutomaticWindowTabbing(false, mtm);
        let app = NSApplication::sharedApplication(mtm);
        let target = MenuTarget::new(mtm, Box::new(on_command));
        let target_object: &AnyObject = &target;
        let mut commands = Vec::new();
        let bar = NSMenu::initWithTitle(NSMenu::alloc(mtm), &NSString::from_str(""));
        for menu in &MENUS {
            let title = NSString::from_str(&menu.title.text(app_name));
            let submenu = NSMenu::initWithTitle(NSMenu::alloc(mtm), &title);
            submenu.setAutoenablesItems(menu.autoenables);
            for item in menu.items {
                let built = build_item(mtm, item, app_name);
                match item.action {
                    Action::Command(command) => {
                        built.setTag(command.tag());
                        // SAFETY: the target responds to `menuCommand:`,
                        // the item's action, and `MenuBar` keeps it alive
                        // while the item can send to it (its drop unhooks
                        // the item first).
                        unsafe { built.setTarget(Some(target_object)) };
                        commands.push((command, built.clone()));
                    }
                    Action::Services => {
                        let services = NSMenu::initWithTitle(
                            NSMenu::alloc(mtm),
                            &NSString::from_str("Services"),
                        );
                        built.setSubmenu(Some(&services));
                        app.setServicesMenu(Some(&services));
                    }
                    Action::System(_) | Action::Separator => {}
                }
                submenu.addItem(&built);
            }
            let top = plain_item(mtm, &title, None, "");
            top.setSubmenu(Some(&submenu));
            bar.addItem(&top);
            if menu.role == Role::Window {
                app.setWindowsMenu(Some(&submenu));
            }
        }
        app.setMainMenu(Some(&bar));
        Ok(Self {
            _target: target,
            commands,
        })
    }

    /// Offers `command`'s item, or greys it.
    pub fn set_enabled(&self, command: Command, enabled: bool) {
        for (_, item) in self.commands.iter().filter(|(of, _)| *of == command) {
            item.setEnabled(enabled);
        }
    }
}

impl Drop for MenuBar {
    /// Unhooks the command items from the target about to go, so a later
    /// pick sends to no one.
    fn drop(&mut self) {
        for (_, item) in &self.commands {
            // SAFETY: no target is always a valid target.
            unsafe { item.setTarget(None) };
        }
    }
}

/// `item` built: its title, action, key equivalent and modifiers, and
/// whether it is an alternate.
fn build_item(mtm: MainThreadMarker, item: &Item, app_name: &str) -> Retained<NSMenuItem> {
    let action = match item.action {
        Action::Separator => return NSMenuItem::separatorItem(mtm),
        Action::Command(_) => Some(sel!(menuCommand:)),
        Action::System(name) => Some(Sel::register(name)),
        Action::Services => None,
    };
    let key = item.shortcut.map(Shortcut::key_string).unwrap_or_default();
    let built = plain_item(
        mtm,
        &NSString::from_str(&item.title.text(app_name)),
        action,
        &key,
    );
    if let Some(shortcut) = item.shortcut {
        built.setKeyEquivalentModifierMask(shortcut.modifiers.flags());
    }
    built.setAlternate(item.alternate);
    built
}

/// A menu item titled `title`, sending `action`, with the key equivalent
/// `key` (none when empty).
fn plain_item(
    mtm: MainThreadMarker,
    title: &NSString,
    action: Option<Sel>,
    key: &str,
) -> Retained<NSMenuItem> {
    // SAFETY: `action` is a registered selector, or none: an AppKit
    // action or the target's `menuCommand:`.
    unsafe {
        NSMenuItem::initWithTitle_action_keyEquivalent(
            NSMenuItem::alloc(mtm),
            title,
            action,
            &NSString::from_str(key),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn off_the_main_thread_nothing_is_installed() {
        let result = std::thread::spawn(|| MenuBar::install("EV Nova", |_| {}).map(|_| ()))
            .join()
            .expect("the thread ends");
        assert_eq!(result, Err(InstallError::NotMainThread));
        assert_eq!(
            InstallError::NotMainThread.to_string(),
            "AppKit can only be used on the main thread"
        );
    }
}
