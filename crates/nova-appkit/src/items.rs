//! The menu bar's items as plain data: the three menus, each item's
//! title, key equivalent and action. [`MenuBar::install`] builds the
//! real menus from [`MENUS`], so a plain test can check what it builds.
//!
//! [`MenuBar::install`]: crate::MenuBar::install

use std::ffi::CStr;

use objc2_app_kit::NSEventModifierFlags;

/// A game-visible item: the crate's own vocabulary, which the game maps
/// to its menu commands.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Command {
    /// About the game.
    About,
    /// Preferences….
    Preferences,
    /// Quit: the game's own quit, never `AppKit`'s `terminate:`, which would
    /// end the process with no chance to save.
    Quit,
}

impl Command {
    /// Every command, in the app menu's order.
    pub const ALL: [Self; 3] = [Self::About, Self::Preferences, Self::Quit];

    /// The item's tag, which says which command a pick of it is. Never 0,
    /// every other item's tag.
    #[must_use]
    pub fn tag(self) -> isize {
        match self {
            Self::About => 1,
            Self::Preferences => 2,
            Self::Quit => 3,
        }
    }

    /// The command an item tagged `tag` sends, if any.
    #[must_use]
    pub fn from_tag(tag: isize) -> Option<Self> {
        Self::ALL.into_iter().find(|command| command.tag() == tag)
    }
}

/// The modifiers a key equivalent is typed with.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Modifiers {
    /// Command (⌘).
    pub command: bool,
    /// Option (⌥), the game's Alt.
    pub option: bool,
    /// Control (⌃).
    pub control: bool,
}

impl Modifiers {
    /// ⌘ alone.
    pub const COMMAND: Self = Self {
        command: true,
        option: false,
        control: false,
    };
    /// ⌥⌘.
    pub const COMMAND_OPTION: Self = Self {
        command: true,
        option: true,
        control: false,
    };
    /// ⌃⌘.
    pub const COMMAND_CONTROL: Self = Self {
        command: true,
        option: false,
        control: true,
    };

    /// `AppKit`'s mask for these modifiers.
    #[must_use]
    pub fn flags(self) -> NSEventModifierFlags {
        let mut flags = NSEventModifierFlags::empty();
        if self.command {
            flags |= NSEventModifierFlags::Command;
        }
        if self.option {
            flags |= NSEventModifierFlags::Option;
        }
        if self.control {
            flags |= NSEventModifierFlags::Control;
        }
        flags
    }
}

/// A key equivalent: the key (as `AppKit`'s `keyEquivalent` string holds
/// it, lower case) and its modifiers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Shortcut {
    /// The key.
    pub key: char,
    /// The modifiers held with it.
    pub modifiers: Modifiers,
}

impl Shortcut {
    const fn new(key: char, modifiers: Modifiers) -> Self {
        Self { key, modifiers }
    }

    /// The key as `AppKit`'s `keyEquivalent` string.
    #[must_use]
    pub fn key_string(self) -> String {
        self.key.to_string()
    }
}

/// An item's title.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Title {
    /// As written.
    Fixed(&'static str),
    /// The words, then a space and the app's name ("Quit EV Nova").
    Named(&'static str),
    /// The app's name alone.
    AppName,
}

impl Title {
    /// The title shown, for an app named `app_name`.
    #[must_use]
    pub fn text(self, app_name: &str) -> String {
        match self {
            Self::Fixed(text) => text.to_owned(),
            Self::Named(words) => format!("{words} {app_name}"),
            Self::AppName => app_name.to_owned(),
        }
    }
}

/// What picking an item does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    /// Sends the game a command, through the crate's target.
    Command(Command),
    /// Sends `AppKit`'s action, named so, up the responder chain from the
    /// key window (no target), where the window or the application takes
    /// it: `AppKit` validates it, greying it and, for
    /// `toggleFullScreen:`, titling it Enter or Exit Full Screen.
    System(&'static CStr),
    /// The Services submenu, which `AppKit` fills.
    Services,
    /// A separator line.
    Separator,
}

/// One item of a menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Item {
    /// Its title.
    pub title: Title,
    /// Its key equivalent, if any.
    pub shortcut: Option<Shortcut>,
    /// What picking it does.
    pub action: Action,
    /// Whether it replaces the item before it while Option is held.
    pub alternate: bool,
}

impl Item {
    const fn new(title: Title, shortcut: Option<Shortcut>, action: Action) -> Self {
        Self {
            title,
            shortcut,
            action,
            alternate: false,
        }
    }

    const SEPARATOR: Self = Self::new(Title::Fixed(""), None, Action::Separator);
}

/// Which menu of the bar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    /// The app menu, in bold, titled by the system with the app's name.
    App,
    /// View.
    View,
    /// Window, which `AppKit` also lists the windows in.
    Window,
}

/// One menu of the bar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Menu {
    /// Which menu.
    pub role: Role,
    /// Its title.
    pub title: Title,
    /// Whether `AppKit` enables its items by validating their actions. The
    /// app menu doesn't, so the game's word on its commands stands
    /// ([`MenuBar::set_enabled`](crate::MenuBar::set_enabled)).
    pub autoenables: bool,
    /// Its items, in order.
    pub items: &'static [Item],
}

const fn command(title: Title, shortcut: Option<Shortcut>, command: Command) -> Item {
    Item::new(title, shortcut, Action::Command(command))
}

const fn system(title: &'static str, shortcut: Option<Shortcut>, action: &'static CStr) -> Item {
    Item::new(Title::Fixed(title), shortcut, Action::System(action))
}

const APP_ITEMS: [Item; 11] = [
    command(Title::Named("About"), None, Command::About),
    Item::SEPARATOR,
    command(
        Title::Fixed("Preferences…"),
        Some(Shortcut::new(',', Modifiers::COMMAND)),
        Command::Preferences,
    ),
    Item::SEPARATOR,
    Item::new(Title::Fixed("Services"), None, Action::Services),
    Item::SEPARATOR,
    Item::new(
        Title::Named("Hide"),
        Some(Shortcut::new('h', Modifiers::COMMAND)),
        Action::System(c"hide:"),
    ),
    system(
        "Hide Others",
        Some(Shortcut::new('h', Modifiers::COMMAND_OPTION)),
        c"hideOtherApplications:",
    ),
    system("Show All", None, c"unhideAllApplications:"),
    Item::SEPARATOR,
    command(
        Title::Named("Quit"),
        Some(Shortcut::new('q', Modifiers::COMMAND)),
        Command::Quit,
    ),
];

const VIEW_ITEMS: [Item; 1] = [system(
    "Enter Full Screen",
    Some(Shortcut::new('f', Modifiers::COMMAND_CONTROL)),
    c"toggleFullScreen:",
)];

const WINDOW_ITEMS: [Item; 5] = [
    system(
        "Minimize",
        Some(Shortcut::new('m', Modifiers::COMMAND)),
        c"performMiniaturize:",
    ),
    Item {
        alternate: true,
        ..system(
            "Minimize All",
            Some(Shortcut::new('m', Modifiers::COMMAND_OPTION)),
            c"miniaturizeAll:",
        )
    },
    system("Zoom", None, c"performZoom:"),
    Item::SEPARATOR,
    system("Bring All to Front", None, c"arrangeInFront:"),
];

/// The menu bar: the app menu, View and Window, as the original's Carbon
/// nib (`main.nib`) has them, but its Edit menu and its shareware items
/// (see the crate docs).
pub const MENUS: [Menu; 3] = [
    Menu {
        role: Role::App,
        title: Title::AppName,
        autoenables: false,
        items: &APP_ITEMS,
    },
    Menu {
        role: Role::View,
        title: Title::Fixed("View"),
        autoenables: true,
        items: &VIEW_ITEMS,
    },
    Menu {
        role: Role::Window,
        title: Title::Fixed("Window"),
        autoenables: true,
        items: &WINDOW_ITEMS,
    },
];

/// A key equivalent on the menu bar, and the game command it sends, if it
/// sends one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyEquivalent {
    /// The key equivalent.
    pub shortcut: Shortcut,
    /// The command it sends; `None` for `AppKit`'s own actions.
    pub command: Option<Command>,
}

/// Every key equivalent on the menu bar, in menu order: the keys the
/// menu bar takes before the game's window sees their press.
#[must_use]
pub fn key_equivalents() -> Vec<KeyEquivalent> {
    MENUS
        .iter()
        .flat_map(|menu| menu.items)
        .filter_map(|item| {
            let command = match item.action {
                Action::Command(command) => Some(command),
                _ => None,
            };
            item.shortcut
                .map(|shortcut| KeyEquivalent { shortcut, command })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each item of `menu` as (title, key equivalent, action), for an app
    /// named "EV Nova".
    fn listed(menu: &Menu) -> Vec<(String, Option<Shortcut>, Action)> {
        menu.items
            .iter()
            .map(|item| (item.title.text("EV Nova"), item.shortcut, item.action))
            .collect()
    }

    #[allow(clippy::unnecessary_wraps)] // Each item's `shortcut`, as listed.
    fn cmd(key: char) -> Option<Shortcut> {
        Some(Shortcut::new(key, Modifiers::COMMAND))
    }

    #[test]
    fn the_bar_holds_the_app_menu_view_and_window_in_order() {
        let titles: Vec<(Role, String)> = MENUS
            .iter()
            .map(|menu| (menu.role, menu.title.text("EV Nova")))
            .collect();
        assert_eq!(
            titles,
            [
                (Role::App, "EV Nova".to_owned()),
                (Role::View, "View".to_owned()),
                (Role::Window, "Window".to_owned())
            ]
        );
    }

    #[test]
    fn the_app_menu_lists_about_preferences_services_hide_and_quit() {
        let separator = (String::new(), None, Action::Separator);
        assert_eq!(
            listed(&MENUS[0]),
            [
                (
                    "About EV Nova".into(),
                    None,
                    Action::Command(Command::About)
                ),
                separator.clone(),
                (
                    "Preferences…".into(),
                    cmd(','),
                    Action::Command(Command::Preferences)
                ),
                separator.clone(),
                ("Services".into(), None, Action::Services),
                separator.clone(),
                ("Hide EV Nova".into(), cmd('h'), Action::System(c"hide:")),
                (
                    "Hide Others".into(),
                    Some(Shortcut::new('h', Modifiers::COMMAND_OPTION)),
                    Action::System(c"hideOtherApplications:")
                ),
                (
                    "Show All".into(),
                    None,
                    Action::System(c"unhideAllApplications:")
                ),
                separator,
                (
                    "Quit EV Nova".into(),
                    cmd('q'),
                    Action::Command(Command::Quit)
                ),
            ]
        );
        assert!(!MENUS[0].autoenables, "the game greys its commands");
    }

    #[test]
    fn quit_sends_the_games_quit_never_terminate() {
        let quit = MENUS[0]
            .items
            .iter()
            .find(|item| item.title == Title::Named("Quit"))
            .expect("Quit");
        assert_eq!(quit.action, Action::Command(Command::Quit));
        assert!(
            MENUS
                .iter()
                .flat_map(|menu| menu.items)
                .all(|item| item.action != Action::System(c"terminate:")),
            "no item terminates"
        );
    }

    #[test]
    fn view_toggles_full_screen_up_the_responder_chain_with_control_command_f() {
        assert_eq!(
            listed(&MENUS[1]),
            [(
                "Enter Full Screen".into(),
                Some(Shortcut::new('f', Modifiers::COMMAND_CONTROL)),
                Action::System(c"toggleFullScreen:")
            )]
        );
        assert!(MENUS[1].autoenables, "AppKit titles it Enter or Exit");
    }

    #[test]
    fn window_minimizes_zooms_and_brings_all_to_front() {
        assert_eq!(
            listed(&MENUS[2]),
            [
                (
                    "Minimize".into(),
                    cmd('m'),
                    Action::System(c"performMiniaturize:")
                ),
                (
                    "Minimize All".into(),
                    Some(Shortcut::new('m', Modifiers::COMMAND_OPTION)),
                    Action::System(c"miniaturizeAll:")
                ),
                ("Zoom".into(), None, Action::System(c"performZoom:")),
                (String::new(), None, Action::Separator),
                (
                    "Bring All to Front".into(),
                    None,
                    Action::System(c"arrangeInFront:")
                ),
            ]
        );
        let alternates: Vec<bool> = MENUS[2].items.iter().map(|item| item.alternate).collect();
        assert_eq!(alternates, [false, true, false, false, false]);
        assert!(
            MENUS[..2]
                .iter()
                .flat_map(|menu| menu.items)
                .all(|item| !item.alternate)
        );
        assert!(MENUS[2].autoenables);
    }

    #[test]
    fn each_commands_tag_comes_back_and_no_other_tag_is_a_command() {
        for command in Command::ALL {
            assert_ne!(command.tag(), 0, "{command:?}");
            assert_eq!(Command::from_tag(command.tag()), Some(command));
        }
        assert_eq!(Command::from_tag(0), None);
        assert_eq!(Command::from_tag(4), None);
        assert_eq!(Command::from_tag(-1), None);
    }

    #[test]
    fn each_shortcut_is_its_key_string_and_appkits_modifier_mask() {
        assert_eq!(Shortcut::new('q', Modifiers::COMMAND).key_string(), "q");
        assert_eq!(Shortcut::new(',', Modifiers::COMMAND).key_string(), ",");
        assert_eq!(Modifiers::COMMAND.flags(), NSEventModifierFlags::Command);
        assert_eq!(
            Modifiers::COMMAND_OPTION.flags(),
            NSEventModifierFlags::Command | NSEventModifierFlags::Option
        );
        assert_eq!(
            Modifiers::COMMAND_CONTROL.flags(),
            NSEventModifierFlags::Command | NSEventModifierFlags::Control
        );
        assert_eq!(Modifiers::default().flags(), NSEventModifierFlags::empty());
    }

    #[test]
    fn the_key_equivalents_are_every_shortcut_with_the_command_it_sends() {
        let listed: Vec<(char, Modifiers, Option<Command>)> = key_equivalents()
            .into_iter()
            .map(|equivalent| {
                (
                    equivalent.shortcut.key,
                    equivalent.shortcut.modifiers,
                    equivalent.command,
                )
            })
            .collect();
        assert_eq!(
            listed,
            [
                (',', Modifiers::COMMAND, Some(Command::Preferences)),
                ('h', Modifiers::COMMAND, None),
                ('h', Modifiers::COMMAND_OPTION, None),
                ('q', Modifiers::COMMAND, Some(Command::Quit)),
                ('f', Modifiers::COMMAND_CONTROL, None),
                ('m', Modifiers::COMMAND, None),
                ('m', Modifiers::COMMAND_OPTION, None),
            ]
        );
    }
}
