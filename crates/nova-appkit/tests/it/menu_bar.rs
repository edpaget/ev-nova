//! The real menu bar, installed on the main thread and read back from
//! `NSApp`'s `mainMenu`.

use std::cell::RefCell;
use std::rc::Rc;

use nova_appkit::{Command, MenuBar};
use objc2::MainThreadMarker;
use objc2::sel;
use objc2_app_kit::{NSApplication, NSEventModifierFlags, NSMenu, NSMenuItem};

/// The item of `menu` titled `title`, and where it is.
fn item(menu: &NSMenu, title: &str) -> (isize, objc2::rc::Retained<NSMenuItem>) {
    (0..menu.numberOfItems())
        .find_map(|index| {
            let item = menu.itemAtIndex(index)?;
            (item.title().to_string() == title).then_some((index, item))
        })
        .unwrap_or_else(|| panic!("no item {title:?}"))
}

pub fn the_installed_bar_is_the_applications_and_sends_the_games_commands() {
    let mtm = MainThreadMarker::new().expect("the test binary's main thread");
    let picked = Rc::new(RefCell::new(Vec::new()));
    let bar = {
        let picked = Rc::clone(&picked);
        MenuBar::install("EV Nova", move |command| picked.borrow_mut().push(command))
            .expect("on the main thread")
    };
    let app = NSApplication::sharedApplication(mtm);
    let main = app.mainMenu().expect("a main menu");
    let menus: Vec<_> = (0..main.numberOfItems())
        .map(|index| {
            main.itemAtIndex(index)
                .and_then(|top| top.submenu())
                .expect("a submenu")
        })
        .collect();
    let titles: Vec<String> = menus.iter().map(|menu| menu.title().to_string()).collect();
    assert_eq!(titles, ["EV Nova", "View", "Window"]);
    let (app_menu, view, window) = (&menus[0], &menus[1], &menus[2]);

    // Quit is the game's, Command-Q, never `terminate:`.
    let (quit_at, quit) = item(app_menu, "Quit EV Nova");
    assert_eq!(quit.action(), Some(sel!(menuCommand:)));
    assert_ne!(quit.action(), Some(sel!(terminate:)));
    assert_eq!(quit.keyEquivalent().to_string(), "q");
    assert_eq!(
        quit.keyEquivalentModifierMask(),
        NSEventModifierFlags::Command
    );
    assert!(quit.target().is_some(), "the crate's target");
    let (prefs_at, prefs) = item(app_menu, "Preferences…");
    assert_eq!(prefs.keyEquivalent().to_string(), ",");
    let (about_at, _) = item(app_menu, "About EV Nova");
    assert!(!app_menu.autoenablesItems());

    // Each pick reaches the closure as its command.
    app_menu.performActionForItemAtIndex(quit_at);
    app_menu.performActionForItemAtIndex(prefs_at);
    app_menu.performActionForItemAtIndex(about_at);
    assert_eq!(
        *picked.borrow(),
        [Command::Quit, Command::Preferences, Command::About]
    );

    // The game greys and offers its commands.
    bar.set_enabled(Command::Preferences, false);
    assert!(!prefs.isEnabled());
    assert!(quit.isEnabled(), "the others left alone");
    bar.set_enabled(Command::Preferences, true);
    assert!(prefs.isEnabled());

    // View's item is AppKit's, up the responder chain.
    let (_, full_screen) = item(view, "Enter Full Screen");
    assert_eq!(full_screen.action(), Some(sel!(toggleFullScreen:)));
    assert!(full_screen.target().is_none(), "the first responder");
    assert_eq!(full_screen.keyEquivalent().to_string(), "f");
    assert_eq!(
        full_screen.keyEquivalentModifierMask(),
        NSEventModifierFlags::Command | NSEventModifierFlags::Control
    );
    assert!(view.autoenablesItems(), "AppKit titles it Enter or Exit");

    // Window is AppKit's Window menu.
    let (_, minimize) = item(window, "Minimize");
    assert_eq!(minimize.action(), Some(sel!(performMiniaturize:)));
    let windows_menu = app.windowsMenu().expect("a Windows menu");
    assert_eq!(windows_menu.title().to_string(), "Window");
    let services = app.servicesMenu().expect("a Services menu");
    assert_eq!(services.title().to_string(), "Services");

    // Dropped, the bar unhooks its items: a pick sends to no one.
    drop(bar);
    assert!(quit.target().is_none());
    app_menu.performActionForItemAtIndex(quit_at);
    assert_eq!(picked.borrow().len(), 3, "nothing more");
}
