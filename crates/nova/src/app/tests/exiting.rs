//! The event loop's end, [`App::exiting`]: the backstop for a quit that
//! bypasses the game (macOS's `terminate:`), which tells the screen once,
//! and after the app's own quit does nothing.

use super::*;

#[test]
fn exiting_a_running_app_tells_the_screen_once_and_keeps_what_it_reported() {
    let mut window = FakeWindow::new((1024, 768), 1.0);
    let mut app = app(&window);
    app.exiting(&mut window);
    assert_eq!(app.screen().quits, 1);
    assert_eq!(app.take_warnings(), ["quit 1"], "what quitting reported");
    app.exiting(&mut window);
    assert_eq!(app.screen().quits, 1, "told once");
    assert!(app.take_warnings().is_empty());
}

#[test]
fn after_the_apps_own_quit_exiting_does_nothing() {
    type Quit = fn(&mut TestApp, &mut FakeWindow) -> Control;
    let close: Quit = |app, window| handle(app, window, WindowEvent::CloseRequested);
    let screen_quit: Quit = |app, window| {
        app.screen.quit_on = Some(Key::Char('q'));
        handle(app, window, key(Key::Char('q'), true))
    };
    let quit_on_tick: Quit = |app, window| {
        app.screen.asks_quit = true;
        let elapsed = Duration::from_millis(16);
        handle(app, window, WindowEvent::Redraw { elapsed })
    };
    for (name, quit) in [
        ("close", close),
        ("screen", screen_quit),
        ("tick", quit_on_tick),
    ] {
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let mut app = app(&window);
        assert_eq!(quit(&mut app, &mut window), Control::Exit, "{name}");
        assert_eq!(app.screen().quits, 1, "{name}");
        app.take_warnings();
        app.exiting(&mut window);
        assert_eq!(app.screen().quits, 1, "{name}: not told again");
        assert!(app.take_warnings().is_empty(), "{name}");
    }
}

#[test]
fn exiting_from_an_exclusive_mode_leaves_it_and_saves_nothing() {
    let mut window = on_monitor();
    let store = saved_fhd(true, DELL);
    let mut app = app(&window).with_settings(keeper(&store));
    app.restore_window(&mut window);
    app.exiting(&mut window);
    assert_eq!(window.fullscreen, [exclusive(fhd()), None]);
    assert_eq!(app.screen().quits, 1);
    assert_eq!(store.writes(), 0);
    assert!(stored(&store).fullscreen, "reopens exclusive");
}

#[test]
fn exiting_after_an_exclusive_quit_leaves_nothing_twice() {
    let mut window = on_monitor();
    let store = saved_fhd(true, DELL);
    let mut app = app(&window).with_settings(keeper(&store));
    app.restore_window(&mut window);
    handle(&mut app, &mut window, WindowEvent::CloseRequested);
    app.exiting(&mut window);
    assert_eq!(window.fullscreen, [exclusive(fhd()), None]);
}
