//! The updater through the app: started once as the app takes it, asked
//! to check now by Check for Updates…, which never reaches the screen, and
//! the item offered only when it can check.

use std::cell::RefCell;
use std::rc::Rc;

use nova_view::MenuCommand;

use super::*;
use crate::app::{MenuPick, Updater};

/// What the recording updater was asked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Call {
    Start,
    CheckNow,
}

/// Records each call in a log the test keeps.
struct RecordingUpdater {
    calls: Rc<RefCell<Vec<Call>>>,
    can_check: bool,
}

impl Updater for RecordingUpdater {
    fn start(&mut self) {
        self.calls.borrow_mut().push(Call::Start);
    }

    fn check_now(&mut self) {
        self.calls.borrow_mut().push(Call::CheckNow);
    }

    fn can_check(&self) -> bool {
        self.can_check
    }
}

fn recording(can_check: bool) -> (Box<dyn Updater>, Rc<RefCell<Vec<Call>>>) {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let updater = RecordingUpdater {
        calls: Rc::clone(&calls),
        can_check,
    };
    (Box::new(updater), calls)
}

/// Records the menu commands and inputs it takes.
#[derive(Default)]
struct PickScreen {
    commands: Vec<MenuCommand>,
    inputs: usize,
}

impl Screen for PickScreen {
    fn input(&mut self, _input: &Input) -> ScreenAction {
        self.inputs += 1;
        ScreenAction::None
    }

    fn tick(&mut self, _dt: Duration) {}

    fn draw(&self, _list: &mut DrawList) {}

    fn menu_command(&mut self, command: MenuCommand) -> ScreenAction {
        self.commands.push(command);
        ScreenAction::None
    }
}

fn pick_app(window: &FakeWindow) -> App<NoImages, PickScreen> {
    App::new(window, NoImages, PickScreen::default())
}

#[test]
fn an_updater_given_at_launch_starts_its_background_checks_once() {
    let window = FakeWindow::new((1024, 768), 1.0);
    let (updater, calls) = recording(true);
    let _app = pick_app(&window).with_updater(updater);
    assert_eq!(*calls.borrow(), [Call::Start]);
}

#[test]
fn check_for_updates_asks_the_updater_to_check_now_and_not_the_screen() {
    let mut window = FakeWindow::new((1024, 768), 1.0);
    let (updater, calls) = recording(true);
    let mut app = pick_app(&window).with_updater(updater);
    let handled = app.handle_routed(
        WindowEvent::CheckForUpdates,
        &mut window,
        &mut RecordingGpu::new(),
    );
    assert_eq!(
        handled,
        Handled {
            control: Control::Continue,
            routing: Routing::Game,
        }
    );
    assert_eq!(*calls.borrow(), [Call::Start, Call::CheckNow]);
    assert_eq!(app.screen().commands, [], "no command reached the screen");
    assert_eq!(app.screen().inputs, 0, "and no input");
}

#[test]
fn without_an_updater_check_for_updates_does_nothing_and_cannot_check() {
    let mut window = FakeWindow::new((1024, 768), 1.0);
    let mut app = pick_app(&window);
    assert!(!app.menu_enabled(MenuPick::CheckForUpdates));
    assert_eq!(
        app.handle(
            WindowEvent::CheckForUpdates,
            &mut window,
            &mut RecordingGpu::new()
        ),
        Control::Continue
    );
    assert_eq!(app.screen().commands, []);
    assert_eq!(app.screen().inputs, 0);
}

#[test]
fn the_app_can_check_when_its_updater_can() {
    let window = FakeWindow::new((1024, 768), 1.0);
    for can in [true, false] {
        let (updater, _) = recording(can);
        let app = pick_app(&window).with_updater(updater);
        assert_eq!(app.menu_enabled(MenuPick::CheckForUpdates), can);
    }
}
