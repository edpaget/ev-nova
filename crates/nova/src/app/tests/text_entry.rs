//! Text entry (rdm:phase/key-bindings/phase-17-text-entry-port): each
//! frame's ask starts, restarts or stops the port's entry at the field's
//! pixels; the port's soft keyboard reaches the screen in its
//! surroundings; and the input method's commits reach the screen as typed
//! text, or the developer overlay while it shows.

use nova_view::geometry::Bounds;
use nova_view::{Device, EntryAsk, Point};

use super::*;
use crate::app::text_entry::field_px;
use crate::text_entry::fixture::{EntryCall, FakeTextEntry};

fn ask(x: f32, focused: u32) -> EntryAsk {
    EntryAsk {
        field: Bounds::at(Point::new(x, 300.0), 200.0, 16.0),
        focused,
    }
}

/// An app over a 2x window, with `entry` as its port.
fn entry_app(window: &FakeWindow, entry: &FakeTextEntry) -> TestApp {
    App::new(window, NoImages, RecordingScreen::default())
        .with_dev_overlay()
        .with_text_entry(Box::new(entry.clone()))
}

fn frame(app: &mut TestApp, window: &mut FakeWindow) {
    handle(
        app,
        window,
        WindowEvent::Redraw {
            elapsed: Duration::from_millis(16),
        },
    );
}

/// `ask`'s field in `app`'s window's pixels.
fn px(app: &TestApp, ask: EntryAsk) -> PxRect {
    field_px(ask.field, app.viewport()).expect("an area")
}

#[test]
fn a_frame_that_asks_starts_entry_at_the_field_s_pixels() {
    let mut window = FakeWindow::new((2048, 1536), 2.0);
    let entry = FakeTextEntry::new();
    let mut app = entry_app(&window, &entry);
    frame(&mut app, &mut window);
    assert_eq!(entry.calls(), [], "nothing asked, nothing started");
    app.screen.entry_ask = Some(ask(100.0, 1));
    frame(&mut app, &mut window);
    let at = px(&app, ask(100.0, 1));
    assert_eq!(at.x, 200, "two pixels a unit");
    assert_eq!(entry.calls(), [EntryCall::Start(at)]);
}

#[test]
fn an_unchanged_ask_starts_nothing_again() {
    let mut window = FakeWindow::new((1024, 768), 1.0);
    let entry = FakeTextEntry::new();
    let mut app = entry_app(&window, &entry);
    app.screen.entry_ask = Some(ask(100.0, 1));
    for _ in 0..3 {
        frame(&mut app, &mut window);
    }
    assert_eq!(entry.calls().len(), 1);
}

#[test]
fn a_moved_field_or_a_new_focus_starts_entry_again() {
    let mut window = FakeWindow::new((1024, 768), 1.0);
    let entry = FakeTextEntry::new();
    let mut app = entry_app(&window, &entry);
    app.screen.entry_ask = Some(ask(100.0, 1));
    frame(&mut app, &mut window);
    app.screen.entry_ask = Some(ask(140.0, 1));
    frame(&mut app, &mut window);
    app.screen.entry_ask = Some(ask(140.0, 2));
    frame(&mut app, &mut window);
    let moved = px(&app, ask(140.0, 1));
    assert_eq!(
        entry.take_calls()[1..],
        [EntryCall::Start(moved), EntryCall::Start(moved)],
        "moved, then focused again in place"
    );
    handle(
        &mut app,
        &mut window,
        WindowEvent::Resized {
            size_px: (2048, 1536),
            scale_factor: 2.0,
        },
    );
    frame(&mut app, &mut window);
    assert_eq!(
        entry.calls(),
        [EntryCall::Start(px(&app, ask(140.0, 2)))],
        "a resize moves the field's pixels"
    );
}

#[test]
fn a_frame_with_no_ask_stops_entry_once() {
    let mut window = FakeWindow::new((1024, 768), 1.0);
    let entry = FakeTextEntry::new();
    let mut app = entry_app(&window, &entry);
    app.screen.entry_ask = Some(ask(100.0, 1));
    frame(&mut app, &mut window);
    app.screen.entry_ask = None;
    frame(&mut app, &mut window);
    frame(&mut app, &mut window);
    assert_eq!(entry.take_calls()[1..], [EntryCall::Stop]);
    app.screen.entry_ask = Some(ask(100.0, 1));
    frame(&mut app, &mut window);
    assert_eq!(
        entry.calls(),
        [EntryCall::Start(px(&app, ask(100.0, 1)))],
        "the same ask again starts again"
    );
}

#[test]
fn while_the_overlay_shows_entry_stops() {
    let mut window = FakeWindow::new((1024, 768), 1.0);
    let entry = FakeTextEntry::new();
    let mut app = entry_app(&window, &entry);
    app.screen.entry_ask = Some(ask(100.0, 1));
    frame(&mut app, &mut window);
    handle(&mut app, &mut window, key(Key::Backquote, true));
    frame(&mut app, &mut window);
    assert_eq!(entry.take_calls()[1..], [EntryCall::Stop]);
    handle(&mut app, &mut window, key(Key::Backquote, true));
    frame(&mut app, &mut window);
    assert_eq!(
        entry.calls(),
        [EntryCall::Start(px(&app, ask(100.0, 1)))],
        "back once it hides"
    );
}

#[test]
fn the_port_s_soft_keyboard_reaches_the_screen_in_its_surroundings() {
    let mut window = FakeWindow::new((1024, 768), 1.0);
    let entry = FakeTextEntry::with_soft_keyboard();
    let mut app = entry_app(&window, &entry);
    frame(&mut app, &mut window);
    assert!(
        app.screen()
            .surroundings
            .last()
            .expect("told")
            .soft_keyboard
    );
    let mut plain = self::app(&window);
    frame(&mut plain, &mut window);
    assert!(
        plain
            .screen()
            .surroundings
            .iter()
            .all(|told| !told.soft_keyboard),
        "no port, no soft keyboard"
    );
}

#[test]
fn committed_text_reaches_the_screen_as_typed_text() {
    let mut window = FakeWindow::new((1024, 768), 1.0);
    let mut app = app(&window);
    for c in ['日', '本'] {
        assert_eq!(
            routed(&mut app, &mut window, WindowEvent::Committed(c)),
            handled(Control::Continue, Routing::Game)
        );
    }
    assert_eq!(app.screen().inputs, [Input::Text('日'), Input::Text('本')]);
}

#[test]
fn committed_text_reaches_the_screen_after_the_overlay_s_toggle_closed_it() {
    let mut window = FakeWindow::new((1024, 768), 1.0);
    let mut app = overlay_app(&window);
    for _ in 0..2 {
        routed(&mut app, &mut window, key(Key::Backquote, true));
        routed(&mut app, &mut window, key(Key::Backquote, false));
    }
    assert!(!visible(&app));
    assert_eq!(
        routed(&mut app, &mut window, WindowEvent::Committed('é')),
        handled(Control::Continue, Routing::Game),
        "not the toggle's, whose key went last"
    );
    assert_eq!(app.screen().inputs, [Input::Text('é')]);
}

#[test]
fn committed_text_goes_to_the_overlay_while_it_shows() {
    let mut window = FakeWindow::new((1024, 768), 1.0);
    let mut app = overlay_app(&window);
    routed(&mut app, &mut window, key(Key::Backquote, true));
    let committed = routed(&mut app, &mut window, WindowEvent::Committed('é'));
    assert_eq!(committed, handled(Control::Continue, Routing::Overlay));
    assert!(app.overlay_wants(Some(&committed)));
    assert_eq!(app.screen().inputs, []);
}

#[test]
fn a_commit_makes_the_keyboard_the_device_used_last() {
    let mut window = FakeWindow::new((1024, 768), 1.0);
    let mut app = app(&window);
    handle(
        &mut app,
        &mut window,
        WindowEvent::PointerMoved { px: (5.0, 5.0) },
    );
    assert_eq!(
        app.screen().surroundings.last().map(|told| told.device),
        Some(Device::Mouse)
    );
    handle(&mut app, &mut window, WindowEvent::Committed('x'));
    assert_eq!(
        app.screen().surroundings.last().map(|told| told.device),
        Some(Device::Keyboard)
    );
}
