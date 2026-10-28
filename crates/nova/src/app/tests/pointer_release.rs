//! A primary-button release outside the game, in a letterbox bar or off
//! the window, never reaches the screen as a release, so the app abandons
//! the screen's press instead: `_TrackMainScreenButtons` ends its press on
//! `Button()` going false wherever the pointer is.

use std::cell::RefCell;
use std::rc::Rc;

use nova_sim::Chance;
use nova_sim::mission::screens::mac_ticks;
use nova_view::flight::SharedChance;
use nova_view::menu::fixture::main_screen_art;
use nova_view::menu::{MainMenu, MenuChoice};
use nova_view::text::fixture::MonoMetrics;
use nova_view::ui::ButtonStyle;

use super::*;

fn button(button: MouseButton, pressed: bool) -> WindowEvent {
    WindowEvent::PointerButton { button, pressed }
}

#[test]
fn a_primary_release_outside_the_game_abandons_the_screens_press() {
    let mut window = FakeWindow::new((2560, 1536), 2.0);
    let mut app = app(&window);
    // Pillarboxed: content x 256..2304 at scale 2.
    handle(&mut app, &mut window, moved(456.0, 100.0));
    handle(&mut app, &mut window, button(MouseButton::Left, true));
    handle(&mut app, &mut window, moved(10.0, 10.0));
    handle(&mut app, &mut window, button(MouseButton::Right, false));
    handle(&mut app, &mut window, button(MouseButton::Left, true));
    assert_eq!(
        app.screen().cancels,
        0,
        "a press, or another button's release"
    );
    handle(&mut app, &mut window, button(MouseButton::Left, false));
    assert_eq!(app.screen().cancels, 1, "in a bar");
    handle(&mut app, &mut window, WindowEvent::PointerLeft);
    handle(&mut app, &mut window, button(MouseButton::Left, false));
    assert_eq!(app.screen().cancels, 2, "off the window");
    assert_eq!(
        app.screen().inputs,
        [
            Input::PointerMoved(Point::new(100.0, 50.0)),
            Input::PointerButton {
                button: MouseButton::Left,
                pressed: true,
                at: Point::new(100.0, 50.0)
            },
            Input::PointerLeft,
            Input::PointerLeft,
        ],
        "no release reaches the screen"
    );
}

#[test]
fn a_primary_release_in_the_game_is_no_cancel() {
    let mut window = FakeWindow::new((1024, 768), 1.0);
    let mut app = app(&window);
    handle(&mut app, &mut window, moved(10.0, 10.0));
    handle(&mut app, &mut window, button(MouseButton::Left, true));
    handle(&mut app, &mut window, button(MouseButton::Left, false));
    assert_eq!(app.screen().cancels, 0);
}

/// Always rolls 3, so the logo's every pick is frame 4.
struct RollsThree;

impl Chance for RollsThree {
    fn fires(&mut self, _percent: u8) -> bool {
        false
    }

    fn below(&mut self, n: u32) -> u32 {
        n - 1
    }

    fn roll(&mut self, _sides: u16) -> u16 {
        3
    }
}

/// An app over the main screen's art, at one window pixel to a logical
/// point, with audio recorded.
fn main_screen(window: &FakeWindow) -> (App<NoImages, MainMenu>, AudioLog) {
    let chance: Rc<RefCell<dyn Chance>> = Rc::new(RefCell::new(RollsThree));
    let menu = MainMenu::new(
        Some(main_screen_art()),
        ButtonStyle::STOCK,
        Rc::new(MonoMetrics),
    )
    .with_chance(SharedChance::new(chance));
    let audio = RecordingAudio::new();
    let log = audio.log();
    let core = AudioCore::new(Box::new(audio) as Box<dyn Audio>);
    (App::new(window, NoImages, menu).with_audio(core), log)
}

#[test]
fn a_main_screen_press_let_go_of_off_the_window_holds_nothing() {
    let mut window = FakeWindow::new((1024, 768), 1.0);
    let (mut app, log) = main_screen(&window);
    let mut gpu = RecordingGpu::new();
    let new = app.screen().press_point(MenuChoice::NewPilot);
    let quit = app.screen().press_point(MenuChoice::Quit);
    let px = |at: Point| (f64::from(at.x), f64::from(at.y));
    for event in [
        WindowEvent::PointerMoved { px: px(new) },
        button(MouseButton::Left, true),
        WindowEvent::PointerLeft,
        button(MouseButton::Left, false),
    ] {
        app.handle(event, &mut window, &mut gpu);
    }
    assert_eq!(
        log.borrow()[log.borrow().len() - 2..],
        [cued(DOWN), cued(UP)],
        "pressed on New Pilot, then dragged off the window"
    );
    log.borrow_mut().clear();
    app.handle(
        WindowEvent::PointerMoved { px: px(quit) },
        &mut window,
        &mut gpu,
    );
    assert_eq!(*log.borrow(), [], "no press held to drag onto Quit");
    app.handle(
        WindowEvent::Redraw {
            elapsed: mac_ticks(5),
        },
        &mut window,
        &mut gpu,
    );
    assert_eq!(app.screen().logo_frame(), Some(4), "the logo runs on");
}
