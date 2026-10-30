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
}
