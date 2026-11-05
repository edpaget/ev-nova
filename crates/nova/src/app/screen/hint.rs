//! The hint the router draws over the ships, the galaxy map and the main
//! menu, naming the keys the router reads through `nova_view`'s one
//! naming function (`nova_view::bound_names::name_read`).

use nova_view::bound_names::{Spelling, name_read};
use nova_view::keymap::{MainScreenCommand, UiCommand};
use nova_view::{Command, CommandReader};

use super::Side;

/// The router's hint on `side`, naming the keys `keys` reads for the
/// device it reads, or `None` where it draws none (flight and the
/// spaceport). Tab switches sides everywhere; Fly is offered over the
/// ships and the map, not the main menu; About and Preferences only with
/// `dialogs`, as only then do they open anything. Under the defaults
/// and a US layout: "Tab: ships / galaxy map   F: fly   I: about   P:
/// preferences". A command left unbound drops its entry.
pub(super) fn router_hint(keys: &CommandReader, side: Side, dialogs: bool) -> Option<String> {
    let fly = match side {
        Side::Ships | Side::Galaxy => true,
        Side::MainMenu => false,
        Side::Flight | Side::Spaceport => return None,
    };
    let entries: [(Command, &str, bool); 4] = [
        (Command::Ui(UiCommand::Tab), "ships / galaxy map", true),
        (Command::MainScreen(MainScreenCommand::Fly), "fly", fly),
        (
            Command::MainScreen(MainScreenCommand::About),
            "about",
            dialogs,
        ),
        (
            Command::MainScreen(MainScreenCommand::Preferences),
            "preferences",
            dialogs,
        ),
    ];
    let named: Vec<String> = entries
        .into_iter()
        .filter(|&(_, _, offered)| offered)
        .filter_map(|(command, does, _)| {
            name_read(keys, command, Spelling::Bound).map(|key| format!("{key}: {does}"))
        })
        .collect();
    Some(named.join("   "))
}
