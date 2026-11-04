//! Input events, in the core's own terms and logical coordinates.

use crate::geometry::Point;

/// A keyboard key the screens care about.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Key {
    /// Left arrow.
    Left,
    /// Right arrow.
    Right,
    /// Up arrow.
    Up,
    /// Down arrow.
    Down,
    /// Page Up (fn+Up on a Mac laptop), which the shop grids take as Up.
    PageUp,
    /// Page Down (fn+Down on a Mac laptop), which the shop grids take as
    /// Down.
    PageDown,
    /// Home (fn+Left on a Mac laptop, Mac key code 0x73), which takes a
    /// text dialog's box to the start of its text.
    Home,
    /// End (fn+Right on a Mac laptop, Mac key code 0x77), which takes a
    /// text dialog's box to the end of its text.
    End,
    /// Return or Enter.
    Enter,
    /// Escape.
    Escape,
    /// The space bar.
    Space,
    /// Tab.
    Tab,
    /// Backspace (Delete on a Mac keyboard), which deletes the character
    /// before the caret.
    Backspace,
    /// Alt (Option on a Mac keyboard), either one, held as a modifier.
    Alt,
    /// Control, either one.
    Control,
    /// Shift, either one, held as a modifier.
    Shift,
    /// Command (the Windows or Super key off a Mac), either one, held as a
    /// modifier.
    Command,
    /// Caps Lock, each press of which turns double time on or off. It is
    /// no flight key: flight never holds it.
    CapsLock,
    /// A key that types a character.
    Char(char),
    /// Function key F`n`: F1 is `Function(1)`.
    Function(u8),
    /// Any other key.
    Other,
}

/// The letter keys and the top row's digit keys, each with its lowercase
/// letter or digit on a US keyboard.
const LETTER_KEYS: [(keyboard_types::Code, char); 36] = {
    use keyboard_types::Code;
    [
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
    ]
};

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
pub fn key_letter(key: keyboard_types::Code) -> Option<char> {
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
pub fn letter_key(letter: char) -> Option<keyboard_types::Code> {
    let letter = letter.to_ascii_lowercase();
    LETTER_KEYS
        .iter()
        .find_map(|&(key, l)| (l == letter).then_some(key))
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
