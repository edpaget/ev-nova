//! The app in the background: it asks for no redraws, the engine's form
//! of the original's 30-tick `WaitNextEvent` sleep there (@0x56885,
//! @0x1b1dc, @0x59965), and the screen still counts the time.

use super::*;

fn redraw_at(app: &mut TestApp, window: &mut FakeWindow, ms: u64) {
    let elapsed = Duration::from_millis(ms);
    handle(app, window, WindowEvent::Redraw { elapsed });
}

#[test]
fn going_to_the_background_asks_for_one_redraw_and_that_one_for_none() {
    let mut window = FakeWindow::new((1024, 768), 1.0);
    let mut app = app(&window);
    redraw_at(&mut app, &mut window, 16);
    assert_eq!(window.redraws, 1);
    handle(&mut app, &mut window, WindowEvent::FocusLost);
    assert_eq!(window.redraws, 2, "the switched-out picture, drawn once");
    redraw_at(&mut app, &mut window, 32);
    redraw_at(&mut app, &mut window, 48);
    assert_eq!(window.redraws, 2, "no next one");
}

/// The press ends here, not with its release, which is dropped in the
/// background.
#[test]
fn going_to_the_background_abandons_the_screens_press() {
    let mut window = FakeWindow::new((1024, 768), 1.0);
    let mut app = app(&window);
    handle(&mut app, &mut window, WindowEvent::FocusLost);
    assert_eq!(app.screen().cancels, 1);
}

#[test]
fn coming_back_asks_for_a_redraw_and_the_redraws_after_ask_again() {
    let mut window = FakeWindow::new((1024, 768), 1.0);
    let mut app = app(&window);
    handle(&mut app, &mut window, WindowEvent::FocusLost);
    redraw_at(&mut app, &mut window, 16);
    let asked = window.redraws;
    handle(&mut app, &mut window, WindowEvent::FocusGained);
    assert_eq!(window.redraws, asked + 1, "the loop restarts");
    redraw_at(&mut app, &mut window, 32);
    assert_eq!(window.redraws, asked + 2, "and runs on");
}

#[test]
fn coming_to_the_front_without_going_away_asks_for_a_redraw() {
    let mut window = FakeWindow::new((1024, 768), 1.0);
    let mut app = app(&window);
    handle(&mut app, &mut window, WindowEvent::FocusGained);
    assert_eq!(window.redraws, 1);
    redraw_at(&mut app, &mut window, 16);
    assert_eq!(window.redraws, 2);
}

#[test]
fn while_away_the_screen_still_ticks_by_the_time_since_the_last_redraw() {
    let mut window = FakeWindow::new((1024, 768), 1.0);
    let mut app = app(&window);
    redraw_at(&mut app, &mut window, 100);
    handle(&mut app, &mut window, WindowEvent::FocusLost);
    redraw_at(&mut app, &mut window, 150);
    handle(&mut app, &mut window, WindowEvent::FocusGained);
    redraw_at(&mut app, &mut window, 5150);
    assert_eq!(
        app.screen().ticks,
        [100, 50, 5000].map(Duration::from_millis),
        "the router counts the time away"
    );
}

/// `_DoPrefsDialog` reads no `_inBackground`: what a command opens while
/// the app asks for no redraws still shows.
#[test]
fn a_menu_command_in_the_background_asks_for_a_redraw() {
    let mut window = FakeWindow::new((1024, 768), 1.0);
    let mut app = app(&window);
    handle(&mut app, &mut window, WindowEvent::FocusLost);
    redraw_at(&mut app, &mut window, 16);
    let asked = window.redraws;
    handle(
        &mut app,
        &mut window,
        WindowEvent::Menu(MenuCommand::Preferences),
    );
    assert_eq!(window.redraws, asked + 1);
    redraw_at(&mut app, &mut window, 32);
    assert_eq!(window.redraws, asked + 1, "and that one for none");
}

#[test]
fn a_menu_command_in_the_front_asks_for_no_redraw_of_its_own() {
    let mut window = FakeWindow::new((1024, 768), 1.0);
    let mut app = app(&window);
    handle(
        &mut app,
        &mut window,
        WindowEvent::Menu(MenuCommand::Preferences),
    );
    assert_eq!(window.redraws, 0);
}
