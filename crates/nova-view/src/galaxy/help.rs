//! The galaxy viewer's key texts, the port's own: its help line and its
//! Enter button's label, each naming the keys the map reads through the
//! one naming function ([`name_read`]).

use crate::bound_names::{Spelling, group, name_read};
use crate::input::Key;
use crate::keymap::{CommandReader, MapCommand, UiCommand};

/// The viewer's help line, naming the keys `keys` reads for the device it
/// reads: under the defaults and a US layout, "Arrows or drag: pan
/// +/-: zoom   Click: select   Return: enter   S: find".
///
/// - The pan is the shared arrows, as a group ([`group`]).
/// - The zoom's `+` is the keypad's, a fixed twin of Zoom In
///   ([`Spelling::Fixed`]: the layout would spell it `Pad +`), and its `-`
///   Zoom Out's key.
/// - Find is its fixed twin S ([`Spelling::Twin`]), as the viewer's F is
///   the router's Fly.
/// - A command left unbound is left out, with its entry when it was the
///   entry's only key.
#[must_use]
pub fn help_line(keys: &CommandReader) -> String {
    let pan = group(
        [
            UiCommand::Up,
            UiCommand::Down,
            UiCommand::Left,
            UiCommand::Right,
        ]
        .map(|command| name_read(keys, command, Spelling::Bound)),
        "/",
    )
    .map_or_else(
        || "Drag: pan".to_owned(),
        |arrows| format!("{arrows} or drag: pan"),
    );
    let zoom = group(
        [
            name_read(keys, MapCommand::ZoomIn, Spelling::Fixed("+")),
            name_read(keys, MapCommand::ZoomOut, Spelling::Bound),
        ],
        "/",
    );
    let enter = name_read(keys, UiCommand::Confirm, Spelling::Bound);
    let find = name_read(keys, MapCommand::Find, Spelling::Twin(Key::KeyS));
    [
        Some(pan),
        zoom.map(|zoom| format!("{zoom}: zoom")),
        Some("Click: select".to_owned()),
        enter.map(|enter| format!("{enter}: enter")),
        find.map(|find| format!("{find}: find")),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join("   ")
}

/// The Enter button's label, naming Confirm's key for the device `keys`
/// reads: "Enter system (Return)" under the defaults, and "Enter system"
/// with Confirm unbound.
#[must_use]
pub fn enter_label(keys: &CommandReader) -> String {
    name_read(keys, UiCommand::Confirm, Spelling::Bound).map_or_else(
        || "Enter system".to_owned(),
        |key| format!("Enter system ({key})"),
    )
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::*;
    use crate::key_names::fixture::FakeKeyNames;
    use crate::keymap::{Bindings, KeyCombo, SharedBindings, contexts};

    /// The viewer's keys over `bindings`, named through `names`.
    fn keys(bindings: Bindings, names: FakeKeyNames) -> CommandReader {
        CommandReader::new(contexts::GALAXY_VIEWER)
            .with_bindings(SharedBindings::new(bindings).with_key_names(Rc::new(names)))
    }

    #[test]
    fn under_the_defaults_the_help_and_the_label_read_as_before() {
        let keys = keys(Bindings::default(), FakeKeyNames::new());
        assert_eq!(
            help_line(&keys),
            "Arrows or drag: pan   +/-: zoom   Click: select   Return: enter   S: find"
        );
        assert_eq!(enter_label(&keys), "Enter system (Return)");
    }

    #[test]
    fn a_rebound_confirm_is_named_by_the_help_and_the_label() {
        let keys = keys(
            Bindings::default().rebound(UiCommand::Confirm, KeyCombo::key(Key::KeyW)),
            FakeKeyNames::azerty(),
        );
        assert!(
            help_line(&keys).contains("   Z: enter   "),
            "{}",
            help_line(&keys)
        );
        assert_eq!(enter_label(&keys), "Enter system (Z)");
    }

    #[test]
    fn a_rebound_zoom_out_and_an_unbound_one_read_in_the_zoom_entry() {
        let rebound = keys(
            Bindings::default().rebound(MapCommand::ZoomOut, KeyCombo::key(Key::KeyO)),
            FakeKeyNames::new(),
        );
        assert!(
            help_line(&rebound).contains("   +/O: zoom   "),
            "{}",
            help_line(&rebound)
        );
        let unbound = keys(
            Bindings::default().unbound(MapCommand::ZoomOut),
            FakeKeyNames::new(),
        );
        assert!(
            help_line(&unbound).contains("   +: zoom   "),
            "{}",
            help_line(&unbound)
        );
    }

    #[test]
    fn unbound_arrows_and_confirm_leave_the_drag_and_drop_the_enter() {
        let keys = keys(
            Bindings::default()
                .unbound(UiCommand::Up)
                .unbound(UiCommand::Down)
                .unbound(UiCommand::Left)
                .unbound(UiCommand::Right)
                .unbound(UiCommand::Confirm),
            FakeKeyNames::new(),
        );
        assert_eq!(
            help_line(&keys),
            "Drag: pan   +/-: zoom   Click: select   S: find"
        );
        assert_eq!(enter_label(&keys), "Enter system");
    }

    #[test]
    fn find_names_its_twin_through_the_layout() {
        let keys = keys(
            Bindings::default(),
            FakeKeyNames::new().with(Key::KeyS, "o"),
        );
        assert!(
            help_line(&keys).ends_with("   O: find"),
            "{}",
            help_line(&keys)
        );
    }
}
