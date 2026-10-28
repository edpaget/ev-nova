//! The menu bar through the app: a picked command goes to the screen as
//! an input does, past the developer overlay, and [`MenuSync`] tells the
//! menu bar each change in what the screen offers.

use std::collections::HashMap;

use nova_view::MenuCommand;

use super::*;
use crate::app::{MenuPort, MenuSync};

/// Records the menu commands it takes, answering each with `answer`;
/// offers each command `enabled` says (all by default); counts quits.
#[derive(Default)]
struct MenuScreen {
    commands: Vec<MenuCommand>,
    inputs: usize,
    answer: Option<ScreenAction>,
    asks_quit: bool,
    disabled: Vec<MenuCommand>,
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
        !self.disabled.contains(&command)
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

/// Records each change it is told, and what each command now is.
#[derive(Default)]
struct RecordingMenu {
    calls: Vec<(MenuCommand, bool)>,
    enabled: HashMap<MenuCommand, bool>,
}

impl MenuPort for RecordingMenu {
    fn set_enabled(&mut self, command: MenuCommand, enabled: bool) {
        self.calls.push((command, enabled));
        self.enabled.insert(command, enabled);
    }
}

#[test]
fn the_first_sync_tells_the_menu_bar_every_command() {
    let screen = MenuScreen {
        disabled: vec![MenuCommand::About],
        ..MenuScreen::default()
    };
    let mut menu = RecordingMenu::default();
    MenuSync::new().sync(&screen, &mut menu);
    assert_eq!(
        menu.calls,
        [
            (MenuCommand::About, false),
            (MenuCommand::Preferences, true),
            (MenuCommand::Quit, true)
        ]
    );
}

#[test]
fn a_sync_tells_only_what_changed() {
    let mut screen = MenuScreen::default();
    let mut menu = RecordingMenu::default();
    let mut sync = MenuSync::new();
    sync.sync(&screen, &mut menu);
    menu.calls.clear();
    sync.sync(&screen, &mut menu);
    assert_eq!(menu.calls, [], "nothing changed");
    screen.disabled = vec![MenuCommand::Preferences];
    sync.sync(&screen, &mut menu);
    assert_eq!(menu.calls, [(MenuCommand::Preferences, false)]);
    screen.disabled.clear();
    sync.sync(&screen, &mut menu);
    assert_eq!(
        menu.calls,
        [
            (MenuCommand::Preferences, false),
            (MenuCommand::Preferences, true)
        ]
    );
    assert_eq!(menu.enabled.get(&MenuCommand::About), Some(&true));
}
