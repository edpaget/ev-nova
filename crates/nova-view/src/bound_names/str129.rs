//! `STR#` 129, the original's key-name table, and the Mac virtual key
//! code each [`Key`] sits at, which indexes it.
//!
//! `_HandlePlayer` names the tutorial hints' keys with
//! `GetIndString(buf, 129, keyPrefs[x] + 1)` (@0x70b01-0x70b1b,
//! @0x70c3d-0x70cb2): entry `code + 1` of a 128-entry table in
//! `Nova-DF.rsrc`, one per Mac key code 0x00-0x7f, so the name ignores the
//! keyboard layout. [`STR_129`] holds the table as the stock file has it,
//! pinned by `nova-view`'s stock test (`stock_flight_hud.rs`), and
//! [`str129_name`] reads it for a key.
//!
//! A plug-in's `STR#` 129 would shadow this one in the original, as
//! `GetIndString` searches the resource chain; the engine hard-codes it, as
//! it does `STR#` 2002's hint strings (rdm:task/plugin-interface-string-overrides).

use crate::input::Key;

/// `STR#` 129, indexed by Mac virtual key code: what the original calls
/// each key in the tutorial hints. An empty entry is a code the table
/// names nothing at (the ISO `§` key, the right modifiers), which the
/// original prints as nothing.
pub const STR_129: [&str; 128] = [
    // 0x00
    "A",
    "S",
    "D",
    "F",
    "H",
    "G",
    "Z",
    "X",
    "C",
    "V",
    "",
    "B",
    "Q",
    "W",
    "E",
    "R",
    // 0x10
    "Y",
    "T",
    "1",
    "2",
    "3",
    "4",
    "6",
    "5",
    "=",
    "9",
    "7",
    "-",
    "8",
    "0",
    "]",
    "O",
    // 0x20
    "U",
    "[",
    "I",
    "P",
    "Return",
    "L",
    "J",
    "\u{2019}",
    "K",
    ";",
    "\\",
    ",",
    "/",
    "N",
    "M",
    ".",
    // 0x30
    "Tab",
    "Space",
    "~",
    "Delete",
    "",
    "Esc",
    "",
    "Command",
    "Shift",
    "Caps Lock",
    "Option",
    "Control",
    "",
    "",
    "",
    "Fn",
    // 0x40
    "F17",
    "Kpd .",
    "",
    "Kpd *",
    "",
    "Kpd +",
    "",
    "Clear",
    "",
    "",
    "",
    "Kpd /",
    "Enter",
    "",
    "Kpd -",
    "F18",
    // 0x50
    "F19",
    "Kpd =",
    "Kpd 0",
    "Kpd 1",
    "Kpd 2",
    "Kpd 3",
    "Kpd 4",
    "Kpd 5",
    "Kpd 6",
    "Kpd 7",
    "",
    "Kpd 8",
    "Kpd 9",
    "",
    "",
    "",
    // 0x60
    "F5",
    "F6",
    "F7",
    "F3",
    "F8",
    "F9",
    "",
    "F11",
    "",
    "F13",
    "F16",
    "F14",
    "",
    "F10",
    "Menu",
    "F12",
    // 0x70
    "",
    "F15",
    "Insert",
    "Home",
    "Page Up",
    "Forward Delete",
    "F4",
    "End",
    "F2",
    "Page Down",
    "F1",
    "Left",
    "Right",
    "Down",
    "Up",
    "",
];

/// Each key the Mac has a virtual key code for, with that code.
///
/// The codes are winit's (`physicalkey_to_scancode` in its macOS backend),
/// so a key reaches the game and is named at the same code; `nova`'s
/// platform tests check the two agree. Where winit names no code, the
/// code is Apple's (`HIToolbox/Events.h`): the ISO `§` key
/// (`kVK_ISO_Section`), Caps Lock, Fn, the JIS keys and the context menu
/// key. Three keys are left out, each a code another key holds: Help
/// (0x72, winit's Insert), the keypad's Clear (0x47, which winit reports as
/// Num Lock) and Mute (0x4a, winit's Volume Down).
const MAC_CODES: &[(Key, u8)] = &[
    (Key::KeyA, 0x00),
    (Key::KeyS, 0x01),
    (Key::KeyD, 0x02),
    (Key::KeyF, 0x03),
    (Key::KeyH, 0x04),
    (Key::KeyG, 0x05),
    (Key::KeyZ, 0x06),
    (Key::KeyX, 0x07),
    (Key::KeyC, 0x08),
    (Key::KeyV, 0x09),
    (Key::IntlBackslash, 0x0a),
    (Key::KeyB, 0x0b),
    (Key::KeyQ, 0x0c),
    (Key::KeyW, 0x0d),
    (Key::KeyE, 0x0e),
    (Key::KeyR, 0x0f),
    (Key::KeyY, 0x10),
    (Key::KeyT, 0x11),
    (Key::Digit1, 0x12),
    (Key::Digit2, 0x13),
    (Key::Digit3, 0x14),
    (Key::Digit4, 0x15),
    (Key::Digit6, 0x16),
    (Key::Digit5, 0x17),
    (Key::Equal, 0x18),
    (Key::Digit9, 0x19),
    (Key::Digit7, 0x1a),
    (Key::Minus, 0x1b),
    (Key::Digit8, 0x1c),
    (Key::Digit0, 0x1d),
    (Key::BracketRight, 0x1e),
    (Key::KeyO, 0x1f),
    (Key::KeyU, 0x20),
    (Key::BracketLeft, 0x21),
    (Key::KeyI, 0x22),
    (Key::KeyP, 0x23),
    (Key::Enter, 0x24),
    (Key::KeyL, 0x25),
    (Key::KeyJ, 0x26),
    (Key::Quote, 0x27),
    (Key::KeyK, 0x28),
    (Key::Semicolon, 0x29),
    (Key::Backslash, 0x2a),
    (Key::Comma, 0x2b),
    (Key::Slash, 0x2c),
    (Key::KeyN, 0x2d),
    (Key::KeyM, 0x2e),
    (Key::Period, 0x2f),
    (Key::Tab, 0x30),
    (Key::Space, 0x31),
    (Key::Backquote, 0x32),
    (Key::Backspace, 0x33),
    (Key::Escape, 0x35),
    (Key::MetaRight, 0x36),
    (Key::MetaLeft, 0x37),
    (Key::ShiftLeft, 0x38),
    (Key::CapsLock, 0x39),
    (Key::AltLeft, 0x3a),
    (Key::ControlLeft, 0x3b),
    (Key::ShiftRight, 0x3c),
    (Key::AltRight, 0x3d),
    (Key::ControlRight, 0x3e),
    (Key::Fn, 0x3f),
    (Key::F17, 0x40),
    (Key::NumpadDecimal, 0x41),
    (Key::NumpadMultiply, 0x43),
    (Key::NumpadAdd, 0x45),
    (Key::NumLock, 0x47),
    (Key::AudioVolumeUp, 0x49),
    (Key::AudioVolumeDown, 0x4a),
    (Key::NumpadDivide, 0x4b),
    (Key::NumpadEnter, 0x4c),
    (Key::NumpadSubtract, 0x4e),
    (Key::F18, 0x4f),
    (Key::F19, 0x50),
    (Key::NumpadEqual, 0x51),
    (Key::Numpad0, 0x52),
    (Key::Numpad1, 0x53),
    (Key::Numpad2, 0x54),
    (Key::Numpad3, 0x55),
    (Key::Numpad4, 0x56),
    (Key::Numpad5, 0x57),
    (Key::Numpad6, 0x58),
    (Key::Numpad7, 0x59),
    (Key::F20, 0x5a),
    (Key::Numpad8, 0x5b),
    (Key::Numpad9, 0x5c),
    (Key::IntlYen, 0x5d),
    (Key::IntlRo, 0x5e),
    (Key::NumpadComma, 0x5f),
    (Key::F5, 0x60),
    (Key::F6, 0x61),
    (Key::F7, 0x62),
    (Key::F3, 0x63),
    (Key::F8, 0x64),
    (Key::F9, 0x65),
    (Key::Lang2, 0x66),
    (Key::F11, 0x67),
    (Key::Lang1, 0x68),
    (Key::F13, 0x69),
    (Key::F16, 0x6a),
    (Key::F14, 0x6b),
    (Key::F10, 0x6d),
    (Key::ContextMenu, 0x6e),
    (Key::F12, 0x6f),
    (Key::F15, 0x71),
    (Key::Insert, 0x72),
    (Key::Home, 0x73),
    (Key::PageUp, 0x74),
    (Key::Delete, 0x75),
    (Key::F4, 0x76),
    (Key::End, 0x77),
    (Key::F2, 0x78),
    (Key::PageDown, 0x79),
    (Key::F1, 0x7a),
    (Key::ArrowLeft, 0x7b),
    (Key::ArrowRight, 0x7c),
    (Key::ArrowDown, 0x7d),
    (Key::ArrowUp, 0x7e),
];

/// The Mac virtual key code `key` sits at, or `None` for a key the Mac
/// has no code for (a media or browser key, F21 and up).
#[must_use]
pub fn mac_code(key: Key) -> Option<u8> {
    MAC_CODES
        .iter()
        .find_map(|&(each, code)| (each == key).then_some(code))
}

/// What `STR#` 129 calls `key`: its entry at `key`'s Mac key code, which
/// may be empty, as the original's table is at a code it names nothing
/// at. `None` for a key the Mac has no code for.
#[must_use]
pub fn str129_name(key: Key) -> Option<&'static str> {
    mac_code(key).map(|code| STR_129[usize::from(code)])
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn the_hints_default_keys_read_their_letters() {
        assert_eq!(str129_name(Key::KeyL), Some("L"));
        assert_eq!(str129_name(Key::KeyM), Some("M"));
        assert_eq!(str129_name(Key::KeyJ), Some("J"));
    }

    #[test]
    fn a_key_that_prints_nothing_reads_the_table_s_own_word() {
        assert_eq!(str129_name(Key::ArrowUp), Some("Up"));
        assert_eq!(str129_name(Key::ArrowLeft), Some("Left"));
        assert_eq!(str129_name(Key::NumpadAdd), Some("Kpd +"));
        assert_eq!(str129_name(Key::Escape), Some("Esc"));
        assert_eq!(str129_name(Key::Backspace), Some("Delete"));
        assert_eq!(str129_name(Key::Delete), Some("Forward Delete"));
        assert_eq!(str129_name(Key::MetaLeft), Some("Command"));
        assert_eq!(str129_name(Key::F1), Some("F1"));
    }

    #[test]
    fn the_quote_is_curly_and_the_backquote_a_tilde() {
        assert_eq!(str129_name(Key::Quote), Some("\u{2019}"));
        assert_eq!(str129_name(Key::Backquote), Some("~"));
    }

    #[test]
    fn a_code_the_table_names_nothing_at_reads_empty() {
        assert_eq!(str129_name(Key::IntlBackslash), Some(""));
        assert_eq!(str129_name(Key::MetaRight), Some(""));
        assert_eq!(str129_name(Key::ShiftRight), Some(""));
    }

    #[test]
    fn a_key_the_mac_has_no_code_for_has_no_entry() {
        assert_eq!(str129_name(Key::MediaPlayPause), None);
        assert_eq!(str129_name(Key::F21), None);
        assert_eq!(mac_code(Key::Unidentified), None);
    }

    #[test]
    fn mac_codes_are_distinct_and_inside_the_table() {
        let codes: BTreeSet<u8> = MAC_CODES.iter().map(|&(_, code)| code).collect();
        assert_eq!(codes.len(), MAC_CODES.len(), "each code once");
        let keys: BTreeSet<Key> = MAC_CODES.iter().map(|&(key, _)| key).collect();
        assert_eq!(keys.len(), MAC_CODES.len(), "each key once");
        assert!(codes.iter().all(|&code| usize::from(code) < STR_129.len()));
    }
}
