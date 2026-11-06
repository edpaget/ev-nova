//! What the router shows over a text dialog covers it
//! (rdm:phase/key-bindings/phase-17-text-entry-port): an overlay in front
//! of New Pilot, of the spaceport's quantity dialog or of the galaxy
//! side's Find stops the frame's ask for text entry, which comes back as
//! it closes. What a covered dialog does is `nova_view`'s
//! (`ui::entry_keyboard`); this pins the wiring alone, through the frame.

use super::menu::menu_with_dialogs;
use super::*;

/// What the frame `screen` draws asks of text entry.
fn asked(screen: &AppScreen) -> Option<nova_view::EntryAsk> {
    drawn(screen).text_entry()
}

/// Opens the Preferences over whatever shows, as a tick then settles
/// what covers what.
fn preferences_over(screen: &mut AppScreen) {
    screen.open_preferences();
    screen.tick(Duration::from_millis(1));
    assert!(screen.preferences().is_some(), "the Preferences open");
}

/// Closes the Preferences, as Escape does.
fn close_preferences(screen: &mut AppScreen) {
    screen.input(&key(Key::Escape, true));
    screen.input(&key(Key::Escape, false));
    assert!(screen.preferences().is_none(), "closed");
}

#[test]
fn an_overlay_in_front_covers_new_pilot_whose_ask_stops() {
    let store = MemoryPilots::new();
    let mut screen = menu_with_dialogs(&store);
    choose(&mut screen, MenuChoice::NewPilot);
    assert_eq!(screen.showing(), Showing::NewPilot);
    let field = screen.new_pilot().expect("open").field().rect();
    assert_eq!(
        asked(&screen).map(|ask| ask.field),
        Some(field),
        "asks at the name"
    );
    preferences_over(&mut screen);
    assert_eq!(asked(&screen), None, "covered");
    close_preferences(&mut screen);
    assert_eq!(asked(&screen).map(|ask| ask.field), Some(field), "back");
}

#[test]
fn an_overlay_in_front_covers_the_spaceport_s_quantity_dialog() {
    let store = MemoryPilots::new();
    let mut screen = landed_trader(&store);
    click_port_item(&mut screen, 7);
    screen.input(&key(Key::AltLeft, true));
    screen.input(&key(Key::KeyB, true));
    screen.input(&Input::Text('b'));
    screen.input(&key(Key::AltLeft, false));
    assert!(
        spaceport(&screen)
            .open_trade()
            .and_then(nova_view::spaceport::trade::TradeScreen::quantity)
            .is_some(),
        "asking"
    );
    assert!(asked(&screen).is_some(), "asks at the count");
    preferences_over(&mut screen);
    assert_eq!(asked(&screen), None, "covered");
    close_preferences(&mut screen);
    assert!(asked(&screen).is_some(), "back");
}

#[test]
fn the_galaxy_side_s_find_is_covered_by_an_overlay() {
    let mut screen = with_dialogs(data());
    screen.input(&key(Key::Tab, true));
    screen.input(&key(Key::Tab, false));
    assert_eq!(screen.showing(), Showing::GalaxyMap);
    screen.input(&key(Key::KeyS, true));
    screen.input(&Input::Text('s'));
    screen.input(&key(Key::KeyS, false));
    assert!(screen.galaxy_map().finding());
    assert!(asked(&screen).is_some(), "asks at Find's field");
    preferences_over(&mut screen);
    assert_eq!(asked(&screen), None, "covered");
}
