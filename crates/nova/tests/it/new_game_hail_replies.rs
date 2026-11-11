//! The new game's hails answered on screen: the fixture
//! (`nova_data::new_game::fixture::folder`) authors no `STR#` for the
//! engine's replies, so they come from the engine's own strings that the
//! new game's mapping supplies (`nova_data::new_game`'s `strings`
//! module). These tests pin that those strings reach the comm dialogs and
//! the drawn frame; which list each reply reads is pinned in `nova_sim`,
//! and the strings themselves in `nova_data`.
//!
//! The app is wired as `main` wires the new game with no interface file
//! (`new_game_port::app_over`), a new pilot, Ada, flying in Hearth:
//!
//! - a ship hail: Tab targets the nearest ship of Hearth's traffic and Y
//!   hails it, which opens the comm dialog on its opening line;
//! - a stellar hail: Digit1 selects Hearth's first stellar and Y hails
//!   it, which opens the planet comm dialog on its greeting, the
//!   stellar's name following, and its type label.

use nova::app::Showing;
use nova_sim::fixture::MemoryPilots;
use nova_view::ui::comm::REPLY_ITEM;
use winit::keyboard::KeyCode;

use crate::new_game_conversation::{new_pilot_flying, opened_from};
use crate::new_game_port::app_over;
use crate::pilots::{Harness, texts};

/// A new pilot, Ada, flying in the fixture's start system, the app wired
/// as `main` wires the new game with no interface file.
fn flying(store: &MemoryPilots) -> Harness {
    let data = opened_from(&nova_data::new_game::fixture::folder());
    new_pilot_flying(app_over(data, store))
}

#[test]
fn a_hailed_ship_s_opening_line_is_written_and_drawn() {
    let store = MemoryPilots::new();
    let mut game = flying(&store);
    game.press(KeyCode::Tab);
    game.press(KeyCode::KeyY);
    assert_eq!(game.showing(), Showing::Comm);
    let comm = game.app.screen().comm().expect("the comm dialog shows");
    let reply = comm
        .dialog()
        .text(REPLY_ITEM)
        .expect("the reply item")
        .lines()
        .join(" ");
    assert!(!reply.trim().is_empty(), "the ship says something");
    let drawn = texts(&game.frame()).join(" ");
    assert!(drawn.contains(reply.trim()), "{reply:?} in {drawn:?}");
}

#[test]
fn a_hailed_stellar_greets_before_its_name_and_shows_its_type() {
    let store = MemoryPilots::new();
    let mut game = flying(&store);
    game.press(KeyCode::Digit1);
    game.frame();
    game.press(KeyCode::KeyY);
    assert_eq!(game.showing(), Showing::Comm);
    let dialog = game
        .app
        .screen()
        .planet_comm()
        .expect("the planet comm dialog shows");
    let view = dialog.view().clone();
    let named = format!("{}.", view.name);
    assert!(
        view.reply.len() > named.len() && view.reply.ends_with(&named),
        "a greeting, then the name: {:?}",
        view.reply
    );
    assert!(!view.type_label.is_empty(), "a type label");
    let drawn = texts(&game.frame()).join(" ");
    for line in [&view.reply, &view.type_label] {
        assert!(drawn.contains(line.as_str()), "{line:?} in {drawn:?}");
    }
}
