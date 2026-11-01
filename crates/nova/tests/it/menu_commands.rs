//! The menu bar's Quit through the whole app: in flight it exits at once
//! and the pilot is saved; during the start-up appear it waits for the
//! appear to end, as Command-Q does; and with the app in the background
//! it exits all the same, as the Dock's Quit and the updater's do.

use std::time::Duration;

use nova::app::{Control, Showing, WindowEvent};
use nova_sim::fixture::MemoryPilots;
use nova_view::MenuCommand;

use crate::common::EntryHarness;
use crate::pilots::{Harness, saved};

/// How many 16 ms frames it takes to reach `delay`.
fn frames_to(delay: Duration) -> u64 {
    delay.as_millis().div_ceil(16) as u64
}

/// A new pilot called Ada, flying, ten frames in.
fn flying(store: &MemoryPilots) -> Harness {
    let mut game = Harness::opening(store);
    game.click(game.menu_button(nova_view::menu::MenuChoice::NewPilot));
    game.type_name("Ada");
    game.press(winit::keyboard::KeyCode::Enter);
    // The ship's christening, confirmed with its default.
    game.press(winit::keyboard::KeyCode::Enter);
    game.enter_ship();
    assert_eq!(game.showing(), Showing::Flight);
    for _ in 0..10 {
        assert_eq!(game.redraw(), Control::Continue);
    }
    game
}

/// The pilot `game` flies.
fn pilot(game: &Harness) -> nova_sim::Pilot {
    game.app
        .screen()
        .flight_view()
        .and_then(|flight| flight.sim().pilot())
        .expect("a pilot")
        .clone()
}

#[test]
fn quit_from_the_menu_bar_in_flight_exits_and_saves_the_pilot() {
    let store = MemoryPilots::new();
    let mut game = flying(&store);
    let flying = pilot(&game);
    let writes = store.writes();
    assert_eq!(
        game.handle(WindowEvent::Menu(MenuCommand::Quit)),
        Control::Exit
    );
    assert_eq!(store.writes(), writes + 1, "saved as it quits");
    assert_eq!(saved(&store, "Ada"), flying);
}

#[test]
fn quit_from_the_menu_bar_during_the_start_up_appear_exits_once_it_ends() {
    let store = MemoryPilots::new();
    let mut game = Harness::starting_up(&store);
    assert_eq!(
        game.handle(WindowEvent::Menu(MenuCommand::Quit)),
        Control::Continue,
        "held for the appear's end"
    );
    let mut frames = 0;
    while game.redraw() == Control::Continue {
        frames += 1;
        assert!(frames < 200, "the app never exits");
    }
    assert!(
        frames + 1 >= frames_to(nova_view::fade::DEATH_FADE_FOR),
        "not before the appear ends: {frames}"
    );
    assert_eq!(store.writes(), 0, "nothing to save on the main menu");
}

/// `_QuitAEVTHandler` (@0x5619e) reads no `_inBackground`: the Dock's
/// Quit, a logout and the updater's quit, all this command, work there.
#[test]
fn quit_from_the_menu_bar_on_the_main_menu_in_the_background_exits() {
    let store = MemoryPilots::new();
    let mut game = Harness::opening(&store);
    game.redraw();
    game.handle(WindowEvent::FocusLost);
    assert_eq!(
        game.handle(WindowEvent::Menu(MenuCommand::Quit)),
        Control::Exit
    );
    assert_eq!(store.writes(), 0, "nothing to save on the main menu");
}

#[test]
fn quit_from_the_menu_bar_in_flight_in_the_background_exits_and_saves_the_pilot() {
    let store = MemoryPilots::new();
    let mut game = flying(&store);
    game.handle(WindowEvent::FocusLost);
    let flying = pilot(&game);
    let writes = store.writes();
    assert_eq!(
        game.handle(WindowEvent::Menu(MenuCommand::Quit)),
        Control::Exit
    );
    assert_eq!(store.writes(), writes + 1, "saved as it quits");
    assert_eq!(saved(&store, "Ada"), flying);
}

/// What holds a quit in the front waits on a tick the background doesn't
/// give, so it doesn't hold it here.
#[test]
fn quit_from_the_menu_bar_during_the_start_up_appear_in_the_background_exits() {
    let store = MemoryPilots::new();
    let mut game = Harness::starting_up(&store);
    game.handle(WindowEvent::FocusLost);
    assert_eq!(
        game.handle(WindowEvent::Menu(MenuCommand::Quit)),
        Control::Exit
    );
}
