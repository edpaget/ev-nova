//! What the game calls a key: the [`KeyNames`] port, which asks the
//! player's keyboard layout, and the one function every screen names a
//! key through ([`key_name`], [`combo_name`]).
//!
//! A binding is a physical position ([`Key`] is `keyboard_types`' `Code`),
//! so its name comes from the player's layout: the key at the US `W`
//! position reads Z on AZERTY. The original's Keys window names a key the
//! same way, through the current layout on every draw (`SRKeyCodeTransformer`
//! in `ShortcutRecorder.framework`; rdm:phase/key-bindings/phase-4-keys-screen's
//! findings), so the layout's name is the default and no rulebook reading
//! is needed.
//!
//! # How a key is named
//!
//! - **A key that prints nothing** takes a fixed name and never reaches
//!   the port: the arrows, Return, the keypad's Enter, Escape, Tab, Space,
//!   Backspace, Delete, Home, End, the page keys, Help, Clear, Caps Lock,
//!   the modifiers and the F keys ([`FIXED_NAMES`]), as the original's
//!   transformer names them from its table before it asks the layout
//!   (`+[SRKeyCodeTransformer initialize]` @0x9990-0xaade). Any other key
//!   that prints nothing (a media key, a browser key, a language key) is
//!   named by its W3C name, `Code`'s `Display`.
//! - **A key that prints** ([`PRINTING_KEYS`]: the letters, the digits,
//!   the punctuation and the keypad's digits and signs) asks the port, and
//!   shows the answer's first character, upper-cased, as the transformer
//!   keeps the first unit `UCKeyTranslate` gives
//!   (`stringWithCharacters:length:` with length 1, @0xae21) and
//!   upper-cases it (@0xad05, @0xae3f). The engine keeps a whole `char`,
//!   so a character beyond the BMP isn't cut in half as the original's
//!   UTF-16 unit would be. An answer that is `None` or empty, or whose
//!   first character is a control character or a dead key's combining
//!   mark, falls back to the key's US legend ([`us_legend`]); a key with
//!   no US legend (the international keys) to its W3C name. So every key
//!   has a name.
//! - **A keypad key** reads `Pad ` before its character, as the
//!   transformer's `Pad %@` does (@0xad72, @0xaeac).
//!
//! # Spelling
//!
//! The original draws the fixed names and the modifiers as glyphs (⌘, ↩,
//! ⇥, ←). The engine spells them in words, as flight's `HELP` line
//! does, so the keys screen and every other place that names a key share
//! one spelling: "Return", "Tab", "Up Arrow", and the modifiers as
//! `Ctrl-`, `Alt-`, `Shift-` and `Cmd-` before the key, in the original's
//! ⌃⌥⇧⌘ order (`SRStringForCocoaModifierFlags` @0xbe39). The glyphs are
//! left to rdm:phase/front-end/phase-15-keys-window-left-outs.

use std::fmt::Debug;

use crate::input::{Key, key_letter};
use crate::keymap::{KeyCombo, Modifiers};

/// What the player's keyboard layout prints on a key.
pub trait KeyNames: Debug {
    /// What the current layout prints on `key`, with no modifier held, or
    /// `None` when it can't say.
    fn name(&self, key: Key) -> Option<String>;
}

/// A [`KeyNames`] that can never say, so every key reads its US legend:
/// the game's where no platform adapter reads the layout (Linux), and
/// what the macOS adapter amounts to off the main thread.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NoKeyNames;

impl KeyNames for NoKeyNames {
    fn name(&self, _key: Key) -> Option<String> {
        None
    }
}

/// The keys that print nothing, each with its fixed name.
pub const FIXED_NAMES: &[(Key, &str)] = &[
    (Key::ArrowUp, "Up Arrow"),
    (Key::ArrowDown, "Down Arrow"),
    (Key::ArrowLeft, "Left Arrow"),
    (Key::ArrowRight, "Right Arrow"),
    (Key::Enter, "Return"),
    (Key::NumpadEnter, "Enter"),
    (Key::Escape, "Esc"),
    (Key::Tab, "Tab"),
    (Key::Space, "Space"),
    (Key::Backspace, "Backspace"),
    (Key::Delete, "Delete"),
    (Key::Home, "Home"),
    (Key::End, "End"),
    (Key::PageUp, "Page Up"),
    (Key::PageDown, "Page Down"),
    (Key::Help, "Help"),
    (Key::Insert, "Insert"),
    (Key::NumpadClear, "Clear"),
    (Key::CapsLock, "Caps Lock"),
    (Key::ShiftLeft, "Shift"),
    (Key::ShiftRight, "Right Shift"),
    (Key::ControlLeft, "Ctrl"),
    (Key::ControlRight, "Right Ctrl"),
    (Key::AltLeft, "Alt"),
    (Key::AltRight, "Right Alt"),
    (Key::MetaLeft, "Cmd"),
    (Key::MetaRight, "Right Cmd"),
    (Key::Fn, "Fn"),
    (Key::F1, "F1"),
    (Key::F2, "F2"),
    (Key::F3, "F3"),
    (Key::F4, "F4"),
    (Key::F5, "F5"),
    (Key::F6, "F6"),
    (Key::F7, "F7"),
    (Key::F8, "F8"),
    (Key::F9, "F9"),
    (Key::F10, "F10"),
    (Key::F11, "F11"),
    (Key::F12, "F12"),
    (Key::F13, "F13"),
    (Key::F14, "F14"),
    (Key::F15, "F15"),
    (Key::F16, "F16"),
    (Key::F17, "F17"),
    (Key::F18, "F18"),
    (Key::F19, "F19"),
    (Key::Unidentified, "Unknown Key"),
];

/// The keys that print a character, which the port is asked about, each
/// with its US legend where it has one, and whether it is on the keypad.
const PRINTING: &[(Key, Option<&str>, bool)] = &[
    (Key::Backquote, Some("`"), false),
    (Key::Backslash, Some("\\"), false),
    (Key::BracketLeft, Some("["), false),
    (Key::BracketRight, Some("]"), false),
    (Key::Comma, Some(","), false),
    (Key::Equal, Some("="), false),
    (Key::Minus, Some("-"), false),
    (Key::Period, Some("."), false),
    (Key::Quote, Some("'"), false),
    (Key::Semicolon, Some(";"), false),
    (Key::Slash, Some("/"), false),
    (Key::IntlBackslash, None, false),
    (Key::IntlRo, None, false),
    (Key::IntlYen, None, false),
    (Key::Numpad0, Some("0"), true),
    (Key::Numpad1, Some("1"), true),
    (Key::Numpad2, Some("2"), true),
    (Key::Numpad3, Some("3"), true),
    (Key::Numpad4, Some("4"), true),
    (Key::Numpad5, Some("5"), true),
    (Key::Numpad6, Some("6"), true),
    (Key::Numpad7, Some("7"), true),
    (Key::Numpad8, Some("8"), true),
    (Key::Numpad9, Some("9"), true),
    (Key::NumpadAdd, Some("+"), true),
    (Key::NumpadSubtract, Some("-"), true),
    (Key::NumpadMultiply, Some("*"), true),
    (Key::NumpadStar, Some("*"), true),
    (Key::NumpadDivide, Some("/"), true),
    (Key::NumpadDecimal, Some("."), true),
    (Key::NumpadComma, Some(","), true),
    (Key::NumpadEqual, Some("="), true),
    (Key::NumpadHash, Some("#"), true),
    (Key::NumpadParenLeft, Some("("), true),
    (Key::NumpadParenRight, Some(")"), true),
];

/// Every key that prints a character, in order: the letters and the top
/// row's digits, then the punctuation, the international keys and the
/// keypad's digits and signs. Only these reach the port.
pub const PRINTING_KEYS: [Key; 71] = printing_keys();

/// [`PRINTING_KEYS`], built from the letter keys and [`PRINTING`].
const fn printing_keys() -> [Key; 71] {
    const LETTERS: [Key; 36] = [
        Key::KeyA,
        Key::KeyB,
        Key::KeyC,
        Key::KeyD,
        Key::KeyE,
        Key::KeyF,
        Key::KeyG,
        Key::KeyH,
        Key::KeyI,
        Key::KeyJ,
        Key::KeyK,
        Key::KeyL,
        Key::KeyM,
        Key::KeyN,
        Key::KeyO,
        Key::KeyP,
        Key::KeyQ,
        Key::KeyR,
        Key::KeyS,
        Key::KeyT,
        Key::KeyU,
        Key::KeyV,
        Key::KeyW,
        Key::KeyX,
        Key::KeyY,
        Key::KeyZ,
        Key::Digit0,
        Key::Digit1,
        Key::Digit2,
        Key::Digit3,
        Key::Digit4,
        Key::Digit5,
        Key::Digit6,
        Key::Digit7,
        Key::Digit8,
        Key::Digit9,
    ];
    let mut keys = [Key::Unidentified; 71];
    let mut at = 0;
    while at < LETTERS.len() {
        keys[at] = LETTERS[at];
        at += 1;
    }
    let mut row = 0;
    while row < PRINTING.len() {
        keys[LETTERS.len() + row] = PRINTING[row].0;
        row += 1;
    }
    keys
}

/// `key`'s fixed name, if it prints nothing and has one.
fn fixed_name(key: Key) -> Option<&'static str> {
    FIXED_NAMES
        .iter()
        .find_map(|&(fixed, name)| (fixed == key).then_some(name))
}

/// Whether `key` prints a character, and so asks the port.
#[must_use]
pub fn prints(key: Key) -> bool {
    PRINTING_KEYS.contains(&key)
}

/// Whether `key` is on the keypad and prints there.
fn on_keypad(key: Key) -> bool {
    PRINTING
        .iter()
        .any(|&(printing, _, keypad)| printing == key && keypad)
}

/// What a US keyboard prints on `key`, upper-cased, if it prints and is
/// on a US keyboard: `W` for `Key::KeyW`, `\` for `Key::Backslash`.
#[must_use]
pub fn us_legend(key: Key) -> Option<String> {
    key_letter(key)
        .map(|letter| letter.to_ascii_uppercase().to_string())
        .or_else(|| {
            PRINTING
                .iter()
                .find_map(|&(printing, legend, _)| (printing == key).then_some(legend)?)
                .map(str::to_owned)
        })
}

/// Whether `c` is a combining mark, as a dead key gives: in one of
/// Unicode's combining blocks.
fn combining(c: char) -> bool {
    matches!(
        c,
        '\u{0300}'..='\u{036f}'
            | '\u{1ab0}'..='\u{1aff}'
            | '\u{1dc0}'..='\u{1dff}'
            | '\u{20d0}'..='\u{20ff}'
            | '\u{fe20}'..='\u{fe2f}'
    )
}

/// The character of the port's answer to show, if there is one: its
/// first, as the original keeps the layout's first unit alone
/// (`stringWithCharacters:length:` with length 1, @0xae21), unless that
/// is a control character or a combining mark.
fn shown(answer: Option<String>) -> Option<char> {
    answer?
        .chars()
        .next()
        .filter(|&c| !c.is_control() && !combining(c))
}

/// The name of `key`, as the player's layout gives it through `names`
/// (see the module docs): never empty.
#[must_use]
pub fn key_name(names: &dyn KeyNames, key: Key) -> String {
    if let Some(name) = fixed_name(key) {
        return name.to_owned();
    }
    if !prints(key) {
        return key.to_string();
    }
    let character = shown(names.name(key))
        .map(|c| c.to_uppercase().collect::<String>())
        .or_else(|| us_legend(key))
        .unwrap_or_else(|| key.to_string());
    if on_keypad(key) {
        format!("Pad {character}")
    } else {
        character
    }
}

/// Each modifier with the word it is spelled with, in the original's
/// ⌃⌥⇧⌘ order.
const MODIFIER_WORDS: [(Modifiers, &str); 4] = [
    (Modifiers::CONTROL, "Ctrl-"),
    (Modifiers::OPTION, "Alt-"),
    (Modifiers::SHIFT, "Shift-"),
    (Modifiers::COMMAND, "Cmd-"),
];

/// The name of `combo`: its modifiers' words, then its key's name
/// ([`key_name`]): `Cmd-K`, `Ctrl-Alt-Shift-Cmd-Q`, `Ctrl`.
#[must_use]
pub fn combo_name(names: &dyn KeyNames, combo: KeyCombo) -> String {
    let mut name: String = MODIFIER_WORDS
        .iter()
        .filter(|&&(flag, _)| combo.modifiers.contains(flag))
        .map(|&(_, word)| word)
        .collect();
    name.push_str(&key_name(names, combo.key));
    name
}

/// The port's fake and the contract every adapter meets, for this crate's
/// tests and other crates' (`nova`'s adapters run [`check_contract`]).
#[cfg(any(test, feature = "fixture"))]
pub mod fixture {
    use std::cell::RefCell;
    use std::collections::BTreeMap;

    use super::{KeyNames, PRINTING_KEYS};
    use crate::input::Key;

    /// Names from a map, recording each key it is asked about.
    #[derive(Debug, Default)]
    pub struct FakeKeyNames {
        names: BTreeMap<Key, String>,
        asked: RefCell<Vec<Key>>,
    }

    impl FakeKeyNames {
        /// A layout that says nothing about any key.
        #[must_use]
        pub fn new() -> Self {
            Self::default()
        }

        /// This layout, printing `name` on `key`.
        #[must_use]
        pub fn with(mut self, key: Key, name: &str) -> Self {
            self.names.insert(key, name.to_owned());
            self
        }

        /// French AZERTY's letters where they differ from US QWERTY: `a`
        /// at the US Q, `z` at W, `q` at A, `w` at Z and `m` at `;`.
        #[must_use]
        pub fn azerty() -> Self {
            Self::new()
                .with(Key::KeyQ, "a")
                .with(Key::KeyW, "z")
                .with(Key::KeyA, "q")
                .with(Key::KeyZ, "w")
                .with(Key::Semicolon, "m")
                .with(Key::KeyM, ",")
        }

        /// Every key asked about, in order.
        #[must_use]
        pub fn asked(&self) -> Vec<Key> {
            self.asked.borrow().clone()
        }
    }

    impl KeyNames for FakeKeyNames {
        fn name(&self, key: Key) -> Option<String> {
            self.asked.borrow_mut().push(key);
            self.names.get(&key).cloned()
        }
    }

    /// Checks the port's contract against `names`, which [`FakeKeyNames`]
    /// meets: every key that prints ([`PRINTING_KEYS`]) gets an answer
    /// without a panic, a `Some` is never empty, and asking twice in a row
    /// gives the same answer. A layout that knows no key meets it. It takes
    /// an adapter behind the port's trait object too, as the game wires it.
    ///
    /// # Errors
    ///
    /// The first key whose answer breaks the contract, and how.
    pub fn check_contract<N: KeyNames + ?Sized>(names: &N) -> Result<(), String> {
        for key in PRINTING_KEYS {
            let first = names.name(key);
            if first.as_deref() == Some("") {
                return Err(format!("{key} is named with an empty string"));
            }
            let again = names.name(key);
            if again != first {
                return Err(format!("{key} is named {first:?}, then {again:?}"));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::fixture::{FakeKeyNames, check_contract};
    use super::*;

    #[test]
    fn azerty_names_the_us_w_position_z() {
        let azerty = FakeKeyNames::azerty();
        assert_eq!(key_name(&azerty, Key::KeyW), "Z");
        assert_eq!(key_name(&azerty, Key::KeyQ), "A");
        assert_eq!(key_name(&azerty, Key::Semicolon), "M");
        assert_eq!(key_name(&azerty, Key::KeyE), "E", "the US legend");
    }

    #[test]
    fn a_key_that_prints_nothing_never_reaches_the_port() {
        let names = FakeKeyNames::new()
            .with(Key::ArrowUp, "nope")
            .with(Key::F12, "nope");
        for &(key, name) in FIXED_NAMES {
            assert_eq!(key_name(&names, key), name);
        }
        assert_eq!(key_name(&names, Key::MediaPlayPause), "MediaPlayPause");
        assert_eq!(names.asked(), vec![]);
    }

    #[test]
    fn a_key_that_prints_asks_the_port_once() {
        let names = FakeKeyNames::new().with(Key::KeyW, "z");
        assert_eq!(key_name(&names, Key::KeyW), "Z");
        assert_eq!(names.asked(), vec![Key::KeyW]);
    }

    #[test]
    fn no_answer_an_empty_one_a_control_character_or_a_dead_key_falls_back_to_the_us_legend() {
        for answer in [
            None,
            Some(""),
            Some("\u{1b}"),
            Some("\u{0302}"),
            Some("\u{7f}a"),
            Some("\u{0302}e"),
        ] {
            let mut names = FakeKeyNames::new();
            if let Some(answer) = answer {
                names = names.with(Key::KeyW, answer).with(Key::Backslash, answer);
            }
            assert_eq!(key_name(&names, Key::KeyW), "W", "{answer:?}");
            assert_eq!(key_name(&names, Key::Backslash), "\\", "{answer:?}");
        }
    }

    #[test]
    fn a_layout_answer_longer_than_one_character_shows_its_first() {
        let names = FakeKeyNames::new()
            .with(Key::KeyW, "zx")
            .with(Key::Quote, "´e")
            .with(Key::Minus, "ß")
            .with(Key::KeyE, "a\u{7f}");
        assert_eq!(key_name(&names, Key::KeyW), "Z");
        assert_eq!(key_name(&names, Key::Quote), "´");
        assert_eq!(
            key_name(&names, Key::Minus),
            "SS",
            "the first character, then upper-cased"
        );
        assert_eq!(key_name(&names, Key::KeyE), "A", "what follows is dropped");
    }

    #[test]
    fn a_spacing_accent_is_shown_and_upper_casing_is_the_whole_answer_s() {
        let names = FakeKeyNames::new()
            .with(Key::BracketLeft, "^")
            .with(Key::Minus, "ß");
        assert_eq!(key_name(&names, Key::BracketLeft), "^");
        assert_eq!(key_name(&names, Key::Minus), "SS");
    }

    #[test]
    fn a_keypad_key_reads_pad_before_its_character() {
        let names = FakeKeyNames::new().with(Key::NumpadDecimal, ",");
        assert_eq!(key_name(&names, Key::NumpadDecimal), "Pad ,");
        assert_eq!(key_name(&names, Key::Numpad7), "Pad 7");
        assert_eq!(key_name(&names, Key::NumpadAdd), "Pad +");
        assert_eq!(key_name(&names, Key::NumpadEnter), "Enter", "fixed");
    }

    #[test]
    fn a_key_with_no_us_legend_and_no_answer_reads_its_w3c_name() {
        assert_eq!(key_name(&NoKeyNames, Key::IntlBackslash), "IntlBackslash");
        let names = FakeKeyNames::new().with(Key::IntlBackslash, "<");
        assert_eq!(key_name(&names, Key::IntlBackslash), "<");
    }

    #[test]
    fn every_key_that_prints_has_a_us_name_or_its_w3c_one() {
        for key in PRINTING_KEYS {
            assert!(prints(key), "{key}");
            assert!(!key_name(&NoKeyNames, key).is_empty(), "{key}");
        }
        assert_eq!(key_name(&NoKeyNames, Key::KeyA), "A");
        assert_eq!(key_name(&NoKeyNames, Key::Digit5), "5");
        assert_eq!(key_name(&NoKeyNames, Key::Quote), "'");
    }

    #[test]
    fn no_key_both_prints_and_has_a_fixed_name() {
        for &(key, name) in FIXED_NAMES {
            assert!(!prints(key), "{key}");
            assert!(!name.is_empty());
        }
        let distinct: std::collections::BTreeSet<Key> = PRINTING_KEYS.into_iter().collect();
        assert_eq!(distinct.len(), PRINTING_KEYS.len());
    }

    #[test]
    fn a_combo_spells_its_modifiers_in_the_original_s_order_before_its_key() {
        let all = Modifiers::CONTROL
            .union(Modifiers::OPTION)
            .union(Modifiers::SHIFT)
            .union(Modifiers::COMMAND);
        assert_eq!(
            combo_name(&NoKeyNames, KeyCombo::with(Key::KeyQ, all)),
            "Ctrl-Alt-Shift-Cmd-Q"
        );
        assert_eq!(
            combo_name(&NoKeyNames, KeyCombo::with(Key::KeyK, Modifiers::COMMAND)),
            "Cmd-K"
        );
        assert_eq!(
            combo_name(&NoKeyNames, KeyCombo::with(Key::Tab, Modifiers::SHIFT)),
            "Shift-Tab"
        );
        assert_eq!(
            combo_name(&NoKeyNames, KeyCombo::key(Key::ControlLeft)),
            "Ctrl"
        );
        assert_eq!(
            combo_name(
                &FakeKeyNames::azerty(),
                KeyCombo::with(Key::KeyW, Modifiers::OPTION)
            ),
            "Alt-Z"
        );
    }

    #[test]
    fn the_fake_and_the_no_names_layout_meet_the_contract() {
        assert_eq!(check_contract(&FakeKeyNames::azerty()), Ok(()));
        assert_eq!(check_contract(&NoKeyNames), Ok(()));
    }

    #[test]
    fn the_contract_checks_an_adapter_behind_the_port_s_trait_object() {
        let names: std::rc::Rc<dyn KeyNames> =
            std::rc::Rc::new(FakeKeyNames::new().with(Key::KeyB, ""));
        assert!(check_contract(&*names).unwrap_err().contains("KeyB"));
    }

    #[test]
    fn the_contract_refuses_an_empty_name_and_an_answer_that_changes() {
        #[derive(Debug, Default)]
        struct Fickle(std::cell::Cell<u32>);
        impl KeyNames for Fickle {
            fn name(&self, _key: Key) -> Option<String> {
                self.0.set(self.0.get() + 1);
                Some(self.0.get().to_string())
            }
        }
        let empty = FakeKeyNames::new().with(Key::KeyB, "");
        assert!(check_contract(&empty).unwrap_err().contains("KeyB"));
        assert!(
            check_contract(&Fickle::default())
                .unwrap_err()
                .contains("then")
        );
    }
}
