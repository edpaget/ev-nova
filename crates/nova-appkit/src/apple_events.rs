//! The Apple events the game handles, as plain data: each event's class
//! and ID, and the [`Command`] it sends. [`AppleEvents::install`]
//! registers a handler for each of [`APPLE_EVENTS`], so a plain test can
//! check what it registers.
//!
//! [`AppleEvents::install`]: crate::AppleEvents::install

use crate::items::Command;

/// A four-character code (`FourCharCode`, the type of an Apple event's
/// class and ID) as the `u32` the Apple Event Manager takes: its four
/// bytes, big-endian, so `*b"quit"` is `0x7175_6974`.
#[must_use]
pub const fn four_char_code(code: [u8; 4]) -> u32 {
    u32::from_be_bytes(code)
}

/// An Apple event the game handles: its class and ID, and the command it
/// sends the game.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AppleEvent {
    /// The event class, such as `aevt` (`kCoreEventClass`).
    pub class: [u8; 4],
    /// The event ID, such as `quit` (`kAEQuitApplication`).
    pub id: [u8; 4],
    /// What the event asks of the game.
    pub command: Command,
}

/// Every Apple event the game handles, the original's
/// `_InitAEVTHandlers` (@0x55fa3-0x5611c) ported:
///
/// - `aevt`/`quit` (@0x560a0, to `_QuitAEVTHandler` @0x5619e), the Dock's
///   Quit, logout, restart, shutdown and `osascript`'s `quit app`: the
///   game's quit, [`Command::Quit`], as Command-Q and the menu's Quit
///   send. The original asks first in play (`STR#` 140 #10,
///   @0x5620d-0x56215); ours never asks, by the user's decision
///   (`play-feel` phase 11, won't-fixed 2026-10-23).
/// - `aevt`/`pref` (@0x560fa, to `_PrefAEVTHandler` @0x5617b), the
///   Preferences the system asks for: [`Command::Preferences`], as the
///   menu's Preferences… sends. The original registers it only on the
///   Aqua menu layout (`Gestalt('menu')` bit 1, @0x560c0-0x560d0), which
///   every macOS has.
///
/// Left out: `oapp`, `odoc` and `pdoc` (@0x55fd1, @0x56016, @0x5605b), which
/// the original hands to `_DummyAEVTHandler` (@0x56174), doing nothing.
/// The bundle declares no document types, so nothing sends `odoc` or
/// `pdoc`, and `AppKit`'s own `oapp` does nothing visible under winit.
pub const APPLE_EVENTS: [AppleEvent; 2] = [
    AppleEvent {
        class: *b"aevt",
        id: *b"quit",
        command: Command::Quit,
    },
    AppleEvent {
        class: *b"aevt",
        id: *b"pref",
        command: Command::Preferences,
    },
];

/// The command the Apple event of class `class` and ID `id` sends, if the
/// game handles it ([`APPLE_EVENTS`]).
#[must_use]
pub fn command_for(class: u32, id: u32) -> Option<Command> {
    APPLE_EVENTS
        .iter()
        .find(|event| four_char_code(event.class) == class && four_char_code(event.id) == id)
        .map(|event| event.command)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn four_char_codes_are_the_originals_immediates() {
        assert_eq!(four_char_code(*b"aevt"), 0x6165_7674);
        assert_eq!(four_char_code(*b"quit"), 0x7175_6974);
        assert_eq!(four_char_code(*b"pref"), 0x7072_6566);
    }

    #[test]
    fn the_quit_event_quits_and_the_pref_event_opens_preferences_and_nothing_else_is_handled() {
        assert_eq!(
            APPLE_EVENTS,
            [
                AppleEvent {
                    class: *b"aevt",
                    id: *b"quit",
                    command: Command::Quit,
                },
                AppleEvent {
                    class: *b"aevt",
                    id: *b"pref",
                    command: Command::Preferences,
                },
            ]
        );
    }

    #[test]
    fn command_for_answers_the_tables_events_and_nothing_else() {
        let aevt = four_char_code(*b"aevt");
        assert_eq!(
            command_for(aevt, four_char_code(*b"quit")),
            Some(Command::Quit)
        );
        assert_eq!(
            command_for(aevt, four_char_code(*b"pref")),
            Some(Command::Preferences)
        );
        assert_eq!(command_for(aevt, four_char_code(*b"odoc")), None);
        assert_eq!(command_for(aevt, four_char_code(*b"oapp")), None);
        // The right ID in another class is another event.
        assert_eq!(
            command_for(four_char_code(*b"misc"), four_char_code(*b"quit")),
            None
        );
    }
}
