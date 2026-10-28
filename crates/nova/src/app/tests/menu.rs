//! The menu bar through the app: a picked command goes to the screen as
//! an input does, past the developer overlay, and [`MenuSync`] tells the
//! menu bar each change in what the app offers: the screen's commands,
//! and Check for Updates… when the updater can check.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use nova_view::MenuCommand;

use super::*;
use crate::app::{MenuPick, MenuPort, MenuSync, Updater};

/// Records the menu commands it takes, answering each with `answer`;
/// offers each command `enabled` says (all by default); counts quits.
#[derive(Default)]
struct MenuScreen {
    commands: Vec<MenuCommand>,
    inputs: usize,
    answer: Option<ScreenAction>,
    asks_quit: bool,
    disabled: Rc<RefCell<Vec<MenuCommand>>>,
    quits: usize,
}

impl Screen for MenuScreen {
    fn input(&mut self, _input: &Input) -> ScreenAction {
        self.inputs += 1;
        ScreenAction::None
    }

    fn tick(&mut self, _dt: Duration) {}

    fn draw(&self, _list: &mut DrawList) {}

    fn menu_command(&mut self, command: MenuCommand) -> ScreenAction {
        self.commands.push(command);
        self.answer.unwrap_or(ScreenAction::None)
    }

    fn menu_enabled(&self, command: MenuCommand) -> bool {
        !self.disabled.borrow().contains(&command)
    }

    fn take_quit(&mut self) -> bool {
        std::mem::take(&mut self.asks_quit)
    }

    fn quit(&mut self) {
        self.quits += 1;
    }
}

fn menu_app(window: &FakeWindow, screen: MenuScreen) -> App<NoImages, MenuScreen> {
    App::new(window, NoImages, screen)
}

fn pick(
    app: &mut App<NoImages, MenuScreen>,
    window: &mut FakeWindow,
    command: MenuCommand,
) -> Control {
    app.handle(WindowEvent::Menu(command), window, &mut RecordingGpu::new())
}

#[test]
fn a_command_the_screen_answers_with_quit_tells_it_and_exits() {
    let mut window = FakeWindow::new((1024, 768), 1.0);
    let screen = MenuScreen {
        answer: Some(ScreenAction::Quit),
        ..MenuScreen::default()
    };
    let mut app = menu_app(&window, screen);
    assert_eq!(
        pick(&mut app, &mut window, MenuCommand::Quit),
        Control::Exit
    );
    assert_eq!(app.screen().commands, [MenuCommand::Quit]);
    assert_eq!(app.screen().quits, 1, "told once");
}

#[test]
fn after_a_menu_quit_the_event_loops_end_tells_the_screen_nothing_more() {
    let mut window = FakeWindow::new((1024, 768), 1.0);
    let screen = MenuScreen {
        answer: Some(ScreenAction::Quit),
        ..MenuScreen::default()
    };
    let mut app = menu_app(&window, screen);
    pick(&mut app, &mut window, MenuCommand::Quit);
    app.exiting(&mut window);
    assert_eq!(app.screen().quits, 1, "told once");
}

#[test]
fn a_command_the_screen_takes_carries_on() {
    let mut window = FakeWindow::new((1024, 768), 1.0);
    let mut app = menu_app(&window, MenuScreen::default());
    for command in MenuCommand::ALL {
        assert_eq!(pick(&mut app, &mut window, command), Control::Continue);
    }
    assert_eq!(app.screen().commands, MenuCommand::ALL);
    assert_eq!(app.screen().inputs, 0, "no input of its own");
    assert_eq!(app.screen().quits, 0);
}

#[test]
fn a_quit_the_screen_asks_after_a_command_exits() {
    let mut window = FakeWindow::new((1024, 768), 1.0);
    let screen = MenuScreen {
        asks_quit: true,
        ..MenuScreen::default()
    };
    let mut app = menu_app(&window, screen);
    assert_eq!(
        pick(&mut app, &mut window, MenuCommand::Preferences),
        Control::Exit
    );
    assert_eq!(app.screen().quits, 1);
}

#[test]
fn a_command_goes_to_the_screen_past_a_showing_developer_overlay() {
    let mut window = FakeWindow::new((1024, 768), 1.0);
    let mut app = menu_app(&window, MenuScreen::default()).with_dev_overlay();
    let mut gpu = RecordingGpu::new();
    for pressed in [true, false] {
        app.handle(
            key(nova_view::devtools::TOGGLE_KEY, pressed),
            &mut window,
            &mut gpu,
        );
    }
    assert!(app.dev_overlay().is_some_and(DevOverlay::visible));
    let handled = app.handle_routed(WindowEvent::Menu(MenuCommand::About), &mut window, &mut gpu);
    assert_eq!(handled.routing, Routing::Game);
    assert_eq!(app.screen().commands, [MenuCommand::About]);
}

/// Records each change it is told, and what each pick now is.
#[derive(Default)]
struct RecordingMenu {
    calls: Vec<(MenuPick, bool)>,
    enabled: HashMap<MenuPick, bool>,
}

impl MenuPort for RecordingMenu {
    fn set_enabled(&mut self, pick: MenuPick, enabled: bool) {
        self.calls.push((pick, enabled));
        self.enabled.insert(pick, enabled);
    }
}

/// An updater that can check, or can't, and does nothing.
struct Checks(bool);

impl Updater for Checks {
    fn start(&mut self) {}

    fn check_now(&mut self) {}

    fn can_check(&self) -> bool {
        self.0
    }
}

fn command(command: MenuCommand) -> MenuPick {
    MenuPick::Command(command)
}

#[test]
fn the_first_sync_tells_the_menu_bar_every_pick() {
    let window = FakeWindow::new((1024, 768), 1.0);
    let screen = MenuScreen {
        disabled: Rc::new(RefCell::new(vec![MenuCommand::About])),
        ..MenuScreen::default()
    };
    let app = menu_app(&window, screen);
    let mut menu = RecordingMenu::default();
    MenuSync::new().sync(&app, &mut menu);
    assert_eq!(
        menu.calls,
        [
            (command(MenuCommand::About), false),
            (command(MenuCommand::Preferences), true),
            (command(MenuCommand::Quit), true),
            (MenuPick::CheckForUpdates, false),
        ],
        "no updater: Check for Updates… greyed"
    );
}

#[test]
fn a_sync_tells_only_what_changed() {
    let window = FakeWindow::new((1024, 768), 1.0);
    let screen = MenuScreen::default();
    let disabled = Rc::clone(&screen.disabled);
    let app = menu_app(&window, screen);
    let mut menu = RecordingMenu::default();
    let mut sync = MenuSync::new();
    sync.sync(&app, &mut menu);
    menu.calls.clear();
    sync.sync(&app, &mut menu);
    assert_eq!(menu.calls, [], "nothing changed");
    *disabled.borrow_mut() = vec![MenuCommand::Preferences];
    sync.sync(&app, &mut menu);
    assert_eq!(menu.calls, [(command(MenuCommand::Preferences), false)]);
    disabled.borrow_mut().clear();
    sync.sync(&app, &mut menu);
    assert_eq!(
        menu.calls,
        [
            (command(MenuCommand::Preferences), false),
            (command(MenuCommand::Preferences), true)
        ]
    );
    assert_eq!(menu.enabled.get(&command(MenuCommand::About)), Some(&true));
}

#[test]
fn check_for_updates_is_offered_when_the_updater_can_check() {
    let window = FakeWindow::new((1024, 768), 1.0);
    for can in [true, false] {
        let app = menu_app(&window, MenuScreen::default()).with_updater(Box::new(Checks(can)));
        assert_eq!(app.menu_enabled(MenuPick::CheckForUpdates), can);
        let mut menu = RecordingMenu::default();
        MenuSync::new().sync(&app, &mut menu);
        assert_eq!(menu.enabled.get(&MenuPick::CheckForUpdates), Some(&can));
        assert_eq!(
            menu.enabled.get(&command(MenuCommand::Quit)),
            Some(&true),
            "the screen's commands as it says"
        );
    }
}

#[test]
fn a_menu_pick_becomes_its_window_event() {
    for game in MenuCommand::ALL {
        assert_eq!(
            WindowEvent::from(MenuPick::Command(game)),
            WindowEvent::Menu(game)
        );
    }
    assert_eq!(
        WindowEvent::from(MenuPick::CheckForUpdates),
        WindowEvent::CheckForUpdates
    );
    assert_eq!(
        MenuPick::ALL,
        [
            command(MenuCommand::About),
            command(MenuCommand::Preferences),
            command(MenuCommand::Quit),
            MenuPick::CheckForUpdates,
        ]
    );
}
