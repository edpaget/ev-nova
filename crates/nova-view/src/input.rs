//! Input events, in the core's own terms and logical coordinates.

use crate::geometry::Point;

/// A physical key, by its position: `keyboard_types`' `Code`, which names
/// each key for what it types on a US keyboard. `Key::KeyW` is the key
/// where a US keyboard has W, whatever the player's layout types there;
/// [`key_letter`] and [`letter_key`] read that US letter.
///
/// A pair of keys the platform folds arrives as its left or main key: the
/// platform adapter's `map_key` says which pairs it folds. `MetaLeft` is
/// Command on a Mac and the Windows or Super key elsewhere, `AltLeft`
/// Option, and `Backspace` the Mac keyboard's Delete. Page Up, Page Down,
/// Home and End are fn with an arrow on a Mac laptop. A key the platform
/// can't identify is `Key::Unidentified`.
///
/// `Code` is `#[non_exhaustive]` and grows with the standard it follows,
/// so every `match` on a `Key` keeps a wildcard arm.
pub use keyboard_types::Code as Key;

/// The letter keys and the top row's digit keys, each with its lowercase
/// letter or digit on a US keyboard.
const LETTER_KEYS: [(Key, char); 36] = [
    (Key::KeyA, 'a'),
    (Key::KeyB, 'b'),
    (Key::KeyC, 'c'),
    (Key::KeyD, 'd'),
    (Key::KeyE, 'e'),
    (Key::KeyF, 'f'),
    (Key::KeyG, 'g'),
    (Key::KeyH, 'h'),
    (Key::KeyI, 'i'),
    (Key::KeyJ, 'j'),
    (Key::KeyK, 'k'),
    (Key::KeyL, 'l'),
    (Key::KeyM, 'm'),
    (Key::KeyN, 'n'),
    (Key::KeyO, 'o'),
    (Key::KeyP, 'p'),
    (Key::KeyQ, 'q'),
    (Key::KeyR, 'r'),
    (Key::KeyS, 's'),
    (Key::KeyT, 't'),
    (Key::KeyU, 'u'),
    (Key::KeyV, 'v'),
    (Key::KeyW, 'w'),
    (Key::KeyX, 'x'),
    (Key::KeyY, 'y'),
    (Key::KeyZ, 'z'),
    (Key::Digit0, '0'),
    (Key::Digit1, '1'),
    (Key::Digit2, '2'),
    (Key::Digit3, '3'),
    (Key::Digit4, '4'),
    (Key::Digit5, '5'),
    (Key::Digit6, '6'),
    (Key::Digit7, '7'),
    (Key::Digit8, '8'),
    (Key::Digit9, '9'),
];

/// The lowercase letter or digit a letter key or top-row digit key is
/// named for on a US keyboard, or `None` for any other key (the keypad's
/// digits included).
///
/// This is the key's US position, not what the player's layout types
/// there: a screen that reads a letter by key code, as the original's
/// filters read `EventRecord.message`'s key-code byte, reads it this way.
/// The layout's own names for keys come from the key-bindings roadmap's
/// `KeyNames` port (its phase 5).
#[must_use]
pub fn key_letter(key: Key) -> Option<char> {
    LETTER_KEYS
        .iter()
        .find_map(|&(k, letter)| (k == key).then_some(letter))
}

/// The letter key or top-row digit key a letter, either case, or a digit
/// is named for on a US keyboard, or `None` for any other character.
///
/// It is [`key_letter`] turned around, and reads a US position the same
/// way.
#[must_use]
pub fn letter_key(letter: char) -> Option<Key> {
    let letter = letter.to_ascii_lowercase();
    LETTER_KEYS
        .iter()
        .find_map(|&(key, l)| (l == letter).then_some(key))
}

/// Shift's two keys, left first.
pub const SHIFT_KEYS: [Key; 2] = [Key::ShiftLeft, Key::ShiftRight];
/// Option's (Alt's) two keys, left first.
pub const OPTION_KEYS: [Key; 2] = [Key::AltLeft, Key::AltRight];
/// Control's two keys, left first.
pub const CONTROL_KEYS: [Key; 2] = [Key::ControlLeft, Key::ControlRight];
/// Command's two keys (the Windows or Super keys off a Mac), left first.
pub const COMMAND_KEYS: [Key; 2] = [Key::MetaLeft, Key::MetaRight];

/// Whether a modifier is held, as a screen's original reads it.
///
/// The original reads a modifier two ways. A `_KeyCheck` or
/// `_QuickKeyCheck` poll tests one key code, so one key: build it with
/// [`ModifierHeld::only`] and the left key. An `EventRecord.modifiers`
/// bit (`shiftKey`, `optionKey`, `controlKey`, `cmdKey`) is set by either
/// key, as Mac OS X sets no right-hand bit: build it with
/// [`ModifierHeld::either`] and the pair ([`SHIFT_KEYS`] and the rest).
/// Each key is followed on its own, so letting go of one while the other
/// is down keeps the modifier held.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ModifierHeld {
    /// The keys read; both the same for one key.
    keys: [Key; 2],
    /// Which of them are down.
    down: [bool; 2],
}

impl ModifierHeld {
    /// A modifier read by its bit: held while either of `keys` is.
    #[must_use]
    pub const fn either(keys: [Key; 2]) -> Self {
        Self {
            keys,
            down: [false; 2],
        }
    }

    /// A modifier polled by `key`'s key code: held while `key` is.
    #[must_use]
    pub const fn only(key: Key) -> Self {
        Self::either([key, key])
    }

    /// Follows `input`'s presses, repeats and releases of the keys read,
    /// and says whether it was one of them.
    pub fn follow(&mut self, input: &Input) -> bool {
        let Input::Key { key, pressed, .. } = *input else {
            return false;
        };
        let mut read = false;
        for (each, down) in self.keys.iter().zip(&mut self.down) {
            if *each == key {
                *down = pressed;
                read = true;
            }
        }
        read
    }

    /// Whether the modifier is held.
    #[must_use]
    pub fn held(&self) -> bool {
        self.down.contains(&true)
    }

    /// Lets go of both keys, as when the keys are released for the
    /// screen.
    pub fn let_go(&mut self) {
        self.down = [false; 2];
    }
}

/// A mouse button.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MouseButton {
    /// The primary button.
    Left,
    /// The secondary button.
    Right,
    /// The middle button.
    Middle,
    /// Any other button.
    Other,
}

/// How far a wheel or trackpad scroll moves the text under the pointer,
/// toward the text's end for a positive amount.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ScrollDelta {
    /// Whole or fractional lines: wheel notches.
    Lines(f32),
    /// Logical units: a trackpad's movement.
    Units(f32),
}

/// One input event for a screen.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Input {
    /// A key went down or up.
    ///
    /// A key held down past the OS key-repeat delay sends further presses
    /// with `repeat` set. Screens that act on every press (panning,
    /// zooming, stepping through ships) take repeats as presses; a press
    /// that toggles something should ignore them.
    Key {
        /// The key.
        key: Key,
        /// Down (`true`) or up.
        pressed: bool,
        /// Whether this press is the OS repeating a held key; never set on
        /// a release.
        repeat: bool,
    },
    /// A printable character was typed, as the keyboard's layout and
    /// modifiers make it. It follows the [`Input::Key`] press that typed
    /// it; only text entry acts on it, and every other screen ignores it.
    Text(char),
    /// The pointer moved to a point in the logical space.
    PointerMoved(Point),
    /// The pointer left the window.
    PointerLeft,
    /// The pointer moved outside the game, in a letterbox bar or off the
    /// window, to `Point` in the logical space past the content's edges,
    /// while the primary button that went down over the game is still held.
    /// [`Input::PointerLeft`] came first, as the pointer left; only a drag
    /// that follows the pointer anywhere (the map's, `_DoSystemMap`'s
    /// `_GetMouse`/`_StillDown` loop @0x137e9-0x138d8) reads it, and every
    /// other screen ignores it.
    PointerMovedOutside(Point),
    /// A mouse button went down or up with the pointer at `at`.
    PointerButton {
        /// The button.
        button: MouseButton,
        /// Down (`true`) or up.
        pressed: bool,
        /// Where the pointer was.
        at: Point,
    },
    /// The mouse wheel turned, or a trackpad scrolled, with the pointer at
    /// `at`. Only the vertical axis is kept.
    ///
    /// The view scrolls whatever scroll reaches it; whether one reaches a
    /// screen at all is the router's call (the `dialog_text_wheel` rule
    /// key, read by `nova::app`'s screen router), since the original
    /// ignores the wheel.
    Scroll {
        /// How far, toward the text's end for positive.
        delta: ScrollDelta,
        /// Where the pointer was.
        at: Point,
    },
}

impl Input {
    /// The input with its pointer point moved by `by`, and any other input
    /// as it is: how a frame drawn moved by `−by` takes the pointer.
    #[must_use]
    pub fn translated(self, by: Point) -> Self {
        let moved = |p: Point| Point::new(p.x + by.x, p.y + by.y);
        match self {
            Self::PointerMoved(at) => Self::PointerMoved(moved(at)),
            Self::PointerMovedOutside(at) => Self::PointerMovedOutside(moved(at)),
            Self::PointerButton {
                button,
                pressed,
                at,
            } => Self::PointerButton {
                button,
                pressed,
                at: moved(at),
            },
            Self::Scroll { delta, at } => Self::Scroll {
                delta,
                at: moved(at),
            },
            Self::Key { .. } | Self::Text(_) | Self::PointerLeft => self,
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn a_translated_input_moves_its_pointer_point_and_nothing_else() {
        let by = Point::new(-170.0, 2.0);
        assert_eq!(
            Input::PointerMoved(Point::new(300.0, 40.0)).translated(by),
            Input::PointerMoved(Point::new(130.0, 42.0))
        );
        assert_eq!(
            Input::PointerMovedOutside(Point::new(-40.0, 900.0)).translated(by),
            Input::PointerMovedOutside(Point::new(-210.0, 902.0))
        );
        assert_eq!(
            Input::PointerButton {
                button: MouseButton::Right,
                pressed: false,
                at: Point::new(170.0, 0.0),
            }
            .translated(by),
            Input::PointerButton {
                button: MouseButton::Right,
                pressed: false,
                at: Point::new(0.0, 2.0),
            }
        );
        assert_eq!(
            Input::Scroll {
                delta: ScrollDelta::Lines(1.0),
                at: Point::new(200.0, 10.0),
            }
            .translated(by),
            Input::Scroll {
                delta: ScrollDelta::Lines(1.0),
                at: Point::new(30.0, 12.0),
            }
        );
        let key = Input::Key {
            key: Key::Enter,
            pressed: true,
            repeat: false,
        };
        for unmoved in [key, Input::Text('a'), Input::PointerLeft] {
            assert_eq!(unmoved.translated(by), unmoved);
        }
    }

    fn press(key: Key) -> Input {
        Input::Key {
            key,
            pressed: true,
            repeat: false,
        }
    }

    fn release(key: Key) -> Input {
        Input::Key {
            key,
            pressed: false,
            repeat: false,
        }
    }

    #[test]
    fn a_modifier_read_by_its_bit_is_held_while_either_key_is() {
        for (left, right) in [
            (Key::ShiftLeft, Key::ShiftRight),
            (Key::AltLeft, Key::AltRight),
            (Key::ControlLeft, Key::ControlRight),
            (Key::MetaLeft, Key::MetaRight),
        ] {
            let mut held = ModifierHeld::either([left, right]);
            assert!(!held.held());
            assert!(held.follow(&press(right)), "{right}");
            assert!(held.held(), "{right}");
            assert!(held.follow(&press(left)));
            assert!(held.follow(&release(right)));
            assert!(held.held(), "{left} still down");
            assert!(held.follow(&release(left)));
            assert!(!held.held(), "both up");
        }
    }

    #[test]
    fn a_modifier_polled_by_its_key_code_is_held_while_that_key_is() {
        let mut held = ModifierHeld::only(Key::ShiftLeft);
        assert!(!held.follow(&press(Key::ShiftRight)));
        assert!(!held.held(), "the other key isn't polled");
        assert!(held.follow(&press(Key::ShiftLeft)));
        assert!(held.held());
        assert!(held.follow(&release(Key::ShiftLeft)));
        assert!(!held.held());
    }

    #[test]
    fn a_held_modifier_follows_repeats_ignores_other_input_and_can_be_let_go() {
        let mut held = ModifierHeld::either([Key::AltLeft, Key::AltRight]);
        assert!(held.follow(&Input::Key {
            key: Key::AltRight,
            pressed: true,
            repeat: true,
        }));
        assert!(held.held(), "a repeat holds it");
        assert!(!held.follow(&press(Key::KeyB)));
        assert!(!held.follow(&Input::Text('b')));
        assert!(held.held(), "other input changes nothing");
        held.let_go();
        assert!(!held.held());
    }

    #[test]
    fn the_modifier_pairs_are_their_left_and_right_keys() {
        assert_eq!(SHIFT_KEYS, [Key::ShiftLeft, Key::ShiftRight]);
        assert_eq!(OPTION_KEYS, [Key::AltLeft, Key::AltRight]);
        assert_eq!(CONTROL_KEYS, [Key::ControlLeft, Key::ControlRight]);
        assert_eq!(COMMAND_KEYS, [Key::MetaLeft, Key::MetaRight]);
    }

    use keyboard_types::Code;

    const LETTERS: [(Code, char); 26] = [
        (Code::KeyA, 'a'),
        (Code::KeyB, 'b'),
        (Code::KeyC, 'c'),
        (Code::KeyD, 'd'),
        (Code::KeyE, 'e'),
        (Code::KeyF, 'f'),
        (Code::KeyG, 'g'),
        (Code::KeyH, 'h'),
        (Code::KeyI, 'i'),
        (Code::KeyJ, 'j'),
        (Code::KeyK, 'k'),
        (Code::KeyL, 'l'),
        (Code::KeyM, 'm'),
        (Code::KeyN, 'n'),
        (Code::KeyO, 'o'),
        (Code::KeyP, 'p'),
        (Code::KeyQ, 'q'),
        (Code::KeyR, 'r'),
        (Code::KeyS, 's'),
        (Code::KeyT, 't'),
        (Code::KeyU, 'u'),
        (Code::KeyV, 'v'),
        (Code::KeyW, 'w'),
        (Code::KeyX, 'x'),
        (Code::KeyY, 'y'),
        (Code::KeyZ, 'z'),
    ];

    const DIGITS: [(Code, char); 10] = [
        (Code::Digit0, '0'),
        (Code::Digit1, '1'),
        (Code::Digit2, '2'),
        (Code::Digit3, '3'),
        (Code::Digit4, '4'),
        (Code::Digit5, '5'),
        (Code::Digit6, '6'),
        (Code::Digit7, '7'),
        (Code::Digit8, '8'),
        (Code::Digit9, '9'),
    ];

    #[test]
    fn each_letter_and_top_row_digit_key_is_its_lowercase_letter_or_digit() {
        for (key, letter) in LETTERS.into_iter().chain(DIGITS) {
            assert_eq!(key_letter(key), Some(letter), "{key}");
        }
    }

    #[test]
    fn each_letter_either_case_and_each_digit_is_its_key() {
        for (key, letter) in LETTERS.into_iter().chain(DIGITS) {
            assert_eq!(letter_key(letter), Some(key), "{letter}");
            assert_eq!(
                letter_key(letter.to_ascii_uppercase()),
                Some(key),
                "{letter}"
            );
        }
    }

    #[test]
    fn a_letter_and_its_key_turn_into_each_other_and_back() {
        for (key, letter) in LETTERS.into_iter().chain(DIGITS) {
            assert_eq!(letter_key(key_letter(key).expect("a letter")), Some(key));
            for c in [letter, letter.to_ascii_uppercase()] {
                assert_eq!(
                    key_letter(letter_key(c).expect("a key")),
                    Some(c.to_ascii_lowercase())
                );
            }
        }
    }

    #[test]
    fn a_key_with_no_letter_and_a_character_with_no_key_are_none() {
        for key in [
            Code::Space,
            Code::Enter,
            Code::F1,
            Code::Numpad1,
            Code::Minus,
            Code::Comma,
            Code::ShiftLeft,
            Code::Unidentified,
        ] {
            assert_eq!(key_letter(key), None, "{key}");
        }
        for c in [' ', '-', ',', 'é', '!'] {
            assert_eq!(letter_key(c), None, "{c}");
        }
    }
}
