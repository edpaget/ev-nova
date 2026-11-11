//! The new game's dialogs over flight, backed: the fixture
//! (`nova_data::new_game::fixture::folder`) has no interface file, so its
//! dialogs come from `nova_view`'s `DefaultDialogs`, which back each one
//! with the new game's panel style. These tests pin that the backing
//! reaches the app's frame between the flight and the dialog's text; the
//! backing's style and shape are pinned in `nova_view`.
//!
//! The app is wired as `main` wires the new game with no interface file
//! (`new_game_port::app_over`), so its dialogs are the placeholders.
//! Two dialogs open over flight, in a new pilot's start system, Hearth:
//!
//! - Preferences, a placeholder `DLOG` of the new game's sixteen, opened
//!   by its menu command. It stands in for a note: the note alert opens
//!   only over the main menu (a pilot that fails to load). The About
//!   text, the other text the menu opens over flight, is the fixture's
//!   `game.md` (`new_game_descriptions` shows it). Preferences draws a
//!   backdrop of its own under an unbacked dialog, so this also pins that
//!   a backed one draws the backing in its place;
//! - the comm dialog, opened by hailing a ship: Tab targets the nearest
//!   ship of Hearth's traffic and Y hails it. It is built over the comm
//!   picture the new game lacks.
//!
//! "The backing" is the open dialog's own: its `Dialog::backing`, which
//! must be there, drawn over its bounds, both read from the dialog.

use nova::app::{Showing, WindowEvent};
use nova_sim::fixture::MemoryPilots;
use nova_view::draw::{DrawCommand, DrawList};
use nova_view::ui::Dialog;
use nova_view::{MenuCommand, Screen};
use winit::keyboard::KeyCode;

use crate::new_game_conversation::{new_pilot_flying, opened_from};
use crate::new_game_port::app_over;
use crate::pilots::Harness;

/// A new pilot, Ada, flying in the fixture's start system, the app wired
/// as `main` wires the new game with no interface file: its dialogs are
/// the placeholders.
fn flying(store: &MemoryPilots) -> Harness {
    let data = opened_from(&nova_data::new_game::fixture::folder());
    new_pilot_flying(app_over(data, store))
}

/// What the app draws next, as commands.
fn commands(game: &Harness) -> Vec<DrawCommand> {
    let mut list = DrawList::new();
    game.app.screen().draw(&mut list);
    list.iter().cloned().collect()
}

/// The first text `dialog_screen` draws on its own.
fn first_text(dialog_screen: &dyn Screen) -> DrawCommand {
    let mut list = DrawList::new();
    dialog_screen.draw(&mut list);
    list.iter()
        .find(|command| matches!(command, DrawCommand::Text { .. }))
        .cloned()
        .expect("the dialog draws a text")
}

/// Asserts that `drawn` holds the flight, then `dialog`'s backing over
/// its bounds, then `text`, in that order.
#[track_caller]
fn assert_backed_between(drawn: &[DrawCommand], dialog: &Dialog, text: &DrawCommand) {
    let backing = dialog.backing().expect("the dialog is backed");
    let mut fill = DrawList::new();
    backing.draw(&mut fill, dialog.bounds());
    let fill = fill.iter().next().cloned().expect("a fill");
    let backed = drawn
        .iter()
        .position(|command| *command == fill)
        .expect("the backing's fill over the dialog is drawn");
    let flight = drawn[..backed]
        .iter()
        .rposition(|command| {
            matches!(
                command,
                DrawCommand::Model { .. }
                    | DrawCommand::Planet { .. }
                    | DrawCommand::Sky { .. }
                    | DrawCommand::Sprite { .. }
                    | DrawCommand::Dot { .. }
            )
        })
        .expect("the flight is drawn under the dialog");
    let text = drawn[backed..]
        .iter()
        .position(|command| command == text)
        .map(|at| backed + at)
        .expect("the dialog's text is drawn over its backing");
    assert!(
        flight < backed && backed < text,
        "{flight} < {backed} < {text}"
    );
}

#[test]
fn a_placeholder_dialog_over_flight_is_backed_between_the_flight_and_its_text() {
    let store = MemoryPilots::new();
    let mut game = flying(&store);
    game.handle(WindowEvent::Menu(MenuCommand::Preferences));
    assert_eq!(game.showing(), Showing::Preferences);
    let prefs = game.app.screen().preferences().expect("Preferences shows");
    let text = first_text(prefs);
    assert_backed_between(&commands(&game), prefs.dialog(), &text);
}

#[test]
fn a_ship_hail_over_flight_is_backed_between_the_flight_and_its_text() {
    let store = MemoryPilots::new();
    let mut game = flying(&store);
    game.press(KeyCode::Tab);
    game.press(KeyCode::KeyY);
    assert_eq!(game.showing(), Showing::Comm);
    let comm = game.app.screen().comm().expect("the comm dialog shows");
    let text = first_text(comm);
    assert_backed_between(&commands(&game), comm.dialog(), &text);
}
