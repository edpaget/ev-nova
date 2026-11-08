//! The original's modifier clicks: the commands a screen off flight reads
//! held while a click is made, each bound by default to a bare modifier
//! key. A touch player gets each from an on-screen toggle
//! (`ui::touch_aids::toggle`), which reaches the screen as the touch
//! device's press of the same command, so the screen's reader holds it
//! across the tap as it holds the key across a click
//! (rdm:phase/key-bindings/phase-16-touch-pointer-gaps).
//!
//! The binary has no other: no right-button read anywhere (no
//! `IsShowContextualMenuClick`, `GetCurrentEventButtonState` or
//! `GetCurrentKeyModifiers` import, and `Button()` @0x3803b reads the
//! primary button), and `_MyModalDialog` hands the event's modifiers back
//! (@0x59ff1-0x59fff) only to `_DoOutfitDialog` (@0x5be50) and
//! `_DoTradeDialog` (@0x5dfd3), which test only `optionKey`.

use super::command::{Command, MapCommand, OutfitterCommand, RaceCommand, TradeCommand};

/// Every modifier click, with the toggle that gives it on touch.
pub const CLICK_MODIFIERS: [Command; 4] = [
    // Trade Most: `_DoTradeDialog`'s `optionKey` test on Buy and Sell
    // (@0x5e211, @0x5e45c).
    Command::Trade(TradeCommand::MaxLot),
    // Ask Count: `_DoOutfitDialog`'s `optionKey` test on Buy and Sell
    // (@0x5c0c0, @0x5c5e3).
    Command::Outfitter(OutfitterCommand::Count),
    // Ask Bet: `_DoRaceDialog` polls the left Option, `_KeyCheck(0x3a)`
    // (@0x48be9), as a racer is clicked.
    Command::Race(RaceCommand::Ask),
    // Plot Course: `_DoSystemMap` polls the left Shift, `_KeyCheck(0x38)`
    // (@0x13071, @0x1325e), as a system is clicked.
    Command::Map(MapCommand::PlotCourse),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Key;
    use crate::keymap::contexts::FLIGHT;
    use crate::keymap::{KeyMap, Modifiers};

    /// The bare modifier keys: Shift, Option, Control and Command, each
    /// side.
    const MODIFIERS: [Key; 8] = [
        Key::ShiftLeft,
        Key::ShiftRight,
        Key::AltLeft,
        Key::AltRight,
        Key::ControlLeft,
        Key::ControlRight,
        Key::MetaLeft,
        Key::MetaRight,
    ];

    #[test]
    fn the_click_modifiers_are_the_held_commands_off_flight_bound_to_a_bare_modifier_key() {
        let keys = KeyMap::default();
        let flight: Vec<_> = FLIGHT.entries.iter().map(|entry| entry.context).collect();
        let found: Vec<Command> = Command::all()
            .filter(|command| !flight.contains(&command.context()))
            .filter(|&command| {
                keys.combo(command).is_some_and(|combo| {
                    MODIFIERS.contains(&combo.key) && combo.modifiers == Modifiers::default()
                })
            })
            .collect();
        assert_eq!(found, CLICK_MODIFIERS);
    }
}
