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
    /// A key that types a character.
    Char(char),
    /// Any other key.
    Other,
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
    /// A mouse button went down or up with the pointer at `at`.
    PointerButton {
        /// The button.
        button: MouseButton,
        /// Down (`true`) or up.
        pressed: bool,
        /// Where the pointer was.
        at: Point,
    },
}
