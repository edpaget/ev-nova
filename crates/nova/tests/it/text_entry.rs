//! Text entry through the app (rdm:phase/key-bindings/phase-17-text-entry-port):
//! a New Pilot dialog opened from the main menu starts the platform's
//! entry (the fake `TextEntry` port) at its name field's pixels, closing
//! the text dialogs stops it, the input method's commits name the pilot,
//! and a platform with a soft keyboard asks for it under touch with no
//! on-screen keyboard drawn. What each dialog asks, and when the app
//! starts and stops, is pinned in nova-view and in `app::text_entry`;
//! this pins that the pieces are joined.

use nova::app::text_entry::field_px;
use nova::app::{Control, Showing, WindowEvent};
use nova::text_entry::fixture::{EntryCall, FakeTextEntry};
use nova_sim::fixture::MemoryPilots;
use nova_view::FingerId;
use nova_view::menu::MenuChoice;
use nova_view::ui::entry_keyboard;
use winit::keyboard::KeyCode;

use crate::pilots::Harness;

/// The main menu, keeping pilots in `store`, with `entry` its port.
fn opening(store: &MemoryPilots, entry: &FakeTextEntry) -> Harness {
    Harness::opening(store).with_text_entry(Box::new(entry.clone()))
}

/// New Pilot's name field in the window's pixels.
fn name_px(game: &Harness) -> nova::app::PxRect {
    let field = game.app.screen().new_pilot().expect("open").field().rect();
    field_px(field, game.app.viewport()).expect("an area")
}

fn open_new_pilot(game: &mut Harness) {
    let at = game.menu_button(MenuChoice::NewPilot);
    game.click(at);
    assert_eq!(game.showing(), Showing::NewPilot);
}

/// Frames until `showing` shows, as a keyed OK's click plays out.
fn settle_on(game: &mut Harness, showing: Showing) {
    for _ in 0..200 {
        if game.showing() == showing {
            return;
        }
        game.frame();
    }
    panic!("never {showing:?}: {:?}", game.showing());
}

#[test]
fn opening_new_pilot_starts_entry_at_the_name_field() {
    let store = MemoryPilots::new();
    let entry = FakeTextEntry::new();
    let mut game = opening(&store, &entry);
    game.frame();
    assert_eq!(entry.calls(), [], "the main menu asks nothing");
    open_new_pilot(&mut game);
    game.frame();
    assert_eq!(entry.calls(), [EntryCall::Start(name_px(&game))]);
}

#[test]
fn closing_the_text_dialogs_stops_entry() {
    let store = MemoryPilots::new();
    let entry = FakeTextEntry::new();
    let mut game = opening(&store, &entry);
    open_new_pilot(&mut game);
    game.frame();
    // The harness's data draws no default name, and an empty one is
    // refused.
    game.handle(WindowEvent::Committed('A'));
    game.press(KeyCode::Enter);
    settle_on(&mut game, Showing::Christening);
    game.frame();
    game.press(KeyCode::Enter);
    settle_on(&mut game, Showing::MainMenu);
    game.frame();
    let calls = entry.calls();
    assert!(
        matches!(
            calls[..],
            [EntryCall::Start(_), EntryCall::Start(_), EntryCall::Stop]
        ),
        "New Pilot's, the christening's, then none: {calls:?}"
    );
}

#[test]
fn committed_ime_text_names_the_pilot() {
    let store = MemoryPilots::new();
    let entry = FakeTextEntry::new();
    let mut game = opening(&store, &entry);
    open_new_pilot(&mut game);
    for c in "Zoë".chars() {
        assert_eq!(game.handle(WindowEvent::Committed(c)), Control::Continue);
    }
    assert_eq!(game.app.screen().new_pilot().expect("open").name(), "Zoë");
    game.press(KeyCode::Enter);
    game.press(KeyCode::Enter);
    assert_eq!(game.handle(WindowEvent::CloseRequested), Control::Exit);
    assert_eq!(store.keys(), ["Zoë"]);
}

#[test]
fn with_a_soft_keyboard_a_touch_on_new_pilot_starts_entry_and_no_keyboard_is_drawn() {
    let store = MemoryPilots::new();
    let entry = FakeTextEntry::with_soft_keyboard();
    let mut game = opening(&store, &entry);
    let at = game.menu_button(MenuChoice::NewPilot);
    let px = (f64::from(at.x), f64::from(at.y));
    let finger = FingerId::new(1);
    game.handle(WindowEvent::TouchStarted { finger, px });
    game.handle(WindowEvent::TouchEnded { finger, px });
    assert_eq!(game.showing(), Showing::NewPilot);
    game.frame();
    let dialog = game.app.screen().new_pilot().expect("open");
    assert!(
        entry_keyboard::shown(dialog).is_none(),
        "the system's keyboard, not the game's"
    );
    assert_eq!(entry.calls(), [EntryCall::Start(name_px(&game))]);
}
