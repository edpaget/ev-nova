//! Input events, in the core's own terms and logical coordinates.

use crate::geometry::Point;

/// A physical key, by its position: `keyboard_types`' `Code`, which names
/// each key for what it types on a US keyboard. `Key::KeyW` is the key
/// where a US keyboard has W, whatever the player's layout types there;
/// [`key_letter`] and [`letter_key`] read that US letter.
///
/// A key the platform folds into another arrives as that other key: the
/// platform adapter's `map_key` says which pair it folds. `MetaLeft` is
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
/// The layout's own names for keys come from the
/// [`KeyNames`](crate::key_names::KeyNames) port.
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

/// The device the player used last, whose input a text names: what a
/// screen asks its [`CommandReader`](crate::CommandReader) for
/// ([`device`](crate::CommandReader::device)), which follows the
/// [`Surroundings`] the app sends, and what every key-naming text passes
/// to [`name_command`](crate::bound_names::name_command).
///
/// The app decides which it is (`nova`'s device-last-used module) and
/// tells the screens through [`Input::Surroundings`]. Phase 12
/// (rdm:phase/key-bindings/phase-12-gamepad-rebind-glyphs) adds pad families
/// and names pad buttons; phase 14 adds `Touch`. No call site changes
/// then, as each asks its reader.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Device {
    /// The keyboard: a command is named by its key.
    #[default]
    Keyboard,
    /// The mouse or trackpad: a pointer button, move or scroll.
    Mouse,
    /// A gamepad of a family, which names its buttons.
    Gamepad(ControllerFamily),
}

/// The family a gamepad belongs to, which phase 12 reads to name its
/// buttons with that family's labels and glyphs. Every pad is
/// [`ControllerFamily::Generic`] until then.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ControllerFamily {
    /// A pad named by position alone.
    #[default]
    Generic,
}

/// What the app tells every screen about where it runs, as data: sent as
/// [`Input::Surroundings`] whenever it changes, and again to a screen the
/// app switches to. Later phases add fields (the safe area, points per
/// logical pixel, whether a soft keyboard exists).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Surroundings {
    /// The device the player used last.
    pub device: Device,
}

/// A gamepad button, by its position on the pad (SDL's names: South is
/// the bottom face button, whatever its label), never by its printed
/// label: phase 12 names it by the pad's family. Each stick's four
/// directions are buttons too, pressed as the stick passes the press
/// threshold (`nova`'s pad module decides when), so a screen reads a
/// stick as it reads the d-pad.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PadButton {
    /// The bottom face button (A on an Xbox pad, Cross on a `PlayStation`
    /// pad).
    South,
    /// The right face button.
    East,
    /// The left face button.
    West,
    /// The top face button.
    North,
    /// The left shoulder button.
    LeftShoulder,
    /// The right shoulder button.
    RightShoulder,
    /// The left trigger, pressed past the press threshold.
    LeftTrigger,
    /// The right trigger, pressed past the press threshold.
    RightTrigger,
    /// The d-pad's up.
    DPadUp,
    /// The d-pad's down.
    DPadDown,
    /// The d-pad's left.
    DPadLeft,
    /// The d-pad's right.
    DPadRight,
    /// Start (Menu, Options, +).
    Start,
    /// Select (Back, View, Share, -).
    Select,
    /// The left stick, clicked.
    LeftStick,
    /// The right stick, clicked.
    RightStick,
    /// The left stick pushed up past the press threshold.
    LeftStickUp,
    /// The left stick pulled down past the press threshold.
    LeftStickDown,
    /// The left stick pushed left past the press threshold.
    LeftStickLeft,
    /// The left stick pushed right past the press threshold.
    LeftStickRight,
    /// The right stick pushed up past the press threshold.
    RightStickUp,
    /// The right stick pulled down past the press threshold.
    RightStickDown,
    /// The right stick pushed left past the press threshold.
    RightStickLeft,
    /// The right stick pushed right past the press threshold.
    RightStickRight,
}

impl PadButton {
    /// Every button, in declaration order.
    pub const ALL: [Self; 24] = [
        Self::South,
        Self::East,
        Self::West,
        Self::North,
        Self::LeftShoulder,
        Self::RightShoulder,
        Self::LeftTrigger,
        Self::RightTrigger,
        Self::DPadUp,
        Self::DPadDown,
        Self::DPadLeft,
        Self::DPadRight,
        Self::Start,
        Self::Select,
        Self::LeftStick,
        Self::RightStick,
        Self::LeftStickUp,
        Self::LeftStickDown,
        Self::LeftStickLeft,
        Self::LeftStickRight,
        Self::RightStickUp,
        Self::RightStickDown,
        Self::RightStickLeft,
        Self::RightStickRight,
    ];

    /// The button's SDL-style position name (`south`, `leftshoulder`,
    /// `dpup`, `+lefty`), as phase 12 saves it. A stick direction is its
    /// axis's name with the direction's sign, as SDL's mappings write a
    /// half axis.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::South => "south",
            Self::East => "east",
            Self::West => "west",
            Self::North => "north",
            Self::LeftShoulder => "leftshoulder",
            Self::RightShoulder => "rightshoulder",
            Self::LeftTrigger => "lefttrigger",
            Self::RightTrigger => "righttrigger",
            Self::DPadUp => "dpup",
            Self::DPadDown => "dpdown",
            Self::DPadLeft => "dpleft",
            Self::DPadRight => "dpright",
            Self::Start => "start",
            Self::Select => "back",
            Self::LeftStick => "leftstick",
            Self::RightStick => "rightstick",
            Self::LeftStickUp => "+lefty",
            Self::LeftStickDown => "-lefty",
            Self::LeftStickLeft => "-leftx",
            Self::LeftStickRight => "+leftx",
            Self::RightStickUp => "+righty",
            Self::RightStickDown => "-righty",
            Self::RightStickLeft => "-rightx",
            Self::RightStickRight => "+rightx",
        }
    }

    /// The stick axis and its direction (`1` or `-1`) a stick-direction
    /// button is, `None` for any other button.
    #[must_use]
    pub const fn stick_direction(self) -> Option<(PadAxis, i8)> {
        Some(match self {
            Self::LeftStickUp => (PadAxis::LeftY, 1),
            Self::LeftStickDown => (PadAxis::LeftY, -1),
            Self::LeftStickLeft => (PadAxis::LeftX, -1),
            Self::LeftStickRight => (PadAxis::LeftX, 1),
            Self::RightStickUp => (PadAxis::RightY, 1),
            Self::RightStickDown => (PadAxis::RightY, -1),
            Self::RightStickLeft => (PadAxis::RightX, -1),
            Self::RightStickRight => (PadAxis::RightX, 1),
            _ => return None,
        })
    }
}

/// A stick axis. X is positive to the right and Y positive up (the stick
/// pushed forward), each from -1 to 1; `nova`'s gamepad adapter gives
/// them so.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PadAxis {
    /// The left stick's X.
    LeftX,
    /// The left stick's Y.
    LeftY,
    /// The right stick's X.
    RightX,
    /// The right stick's Y.
    RightY,
}

impl PadAxis {
    /// Every axis, in declaration order.
    pub const ALL: [Self; 4] = [Self::LeftX, Self::LeftY, Self::RightX, Self::RightY];

    /// The axis's SDL name (`leftx`, `lefty`, `rightx`, `righty`), as
    /// phase 12 saves a binding to it.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::LeftX => "leftx",
            Self::LeftY => "lefty",
            Self::RightX => "rightx",
            Self::RightY => "righty",
        }
    }

    /// The other axis of the same stick: Y for X, X for Y.
    #[must_use]
    pub const fn across(self) -> Self {
        match self {
            Self::LeftX => Self::LeftY,
            Self::LeftY => Self::LeftX,
            Self::RightX => Self::RightY,
            Self::RightY => Self::RightX,
        }
    }

    /// The stick-direction button the axis presses pushed the
    /// `positive` way (right or up) or the other.
    #[must_use]
    pub const fn direction(self, positive: bool) -> PadButton {
        match (self, positive) {
            (Self::LeftX, true) => PadButton::LeftStickRight,
            (Self::LeftX, false) => PadButton::LeftStickLeft,
            (Self::LeftY, true) => PadButton::LeftStickUp,
            (Self::LeftY, false) => PadButton::LeftStickDown,
            (Self::RightX, true) => PadButton::RightStickRight,
            (Self::RightX, false) => PadButton::RightStickLeft,
            (Self::RightY, true) => PadButton::RightStickUp,
            (Self::RightY, false) => PadButton::RightStickDown,
        }
    }
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
    /// it. Text entry acts on it, and so does the mission offer
    /// ([`crate::spaceport::offer`]), which picks a button from the
    /// character as `_SingleMissionFilter` does, but only from the
    /// character a fresh press typed, never a repeat's. Every other screen
    /// ignores it, so an adapter delivers it straight after its press,
    /// outside text fields too, never dropping or delaying it.
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
    /// A gamepad button went down or up: a button by position, a trigger
    /// or a stick direction past the press threshold. Pads merge: a button
    /// is down while any pad holds it.
    PadButton {
        /// The button.
        button: PadButton,
        /// Down (`true`) or up.
        pressed: bool,
    },
    /// A stick axis moved to `value`, -1 to 1 (see [`PadAxis`] for the
    /// signs), as the pad reports it, before any dead zone: flight reads
    /// its analog amounts from it.
    PadAxis {
        /// The axis.
        axis: PadAxis,
        /// Where it is.
        value: f32,
    },
    /// What the app tells the screens about where they run changed, or a
    /// screen was switched to.
    Surroundings(Surroundings),
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
            Self::Key { .. }
            | Self::Text(_)
            | Self::PointerLeft
            | Self::PadButton { .. }
            | Self::PadAxis { .. }
            | Self::Surroundings(_) => self,
        }
    }

    /// Whether `self` and `other` are both a key's press, repeat or
    /// release, of one key, or both a pad button's press or release, of
    /// one button.
    #[must_use]
    pub fn same_key(&self, other: &Self) -> bool {
        match (*self, *other) {
            (Self::Key { key: a, .. }, Self::Key { key: b, .. }) => a == b,
            (Self::PadButton { button: a, .. }, Self::PadButton { button: b, .. }) => a == b,
            _ => false,
        }
    }

    /// The release of the key or pad button `self` presses, repeats or
    /// releases; any other input as it is.
    #[must_use]
    pub const fn released(self) -> Self {
        match self {
            Self::Key { key, .. } => Self::Key {
                key,
                pressed: false,
                repeat: false,
            },
            Self::PadButton { button, .. } => Self::PadButton {
                button,
                pressed: false,
            },
            other => other,
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    fn key_input(key: Key, pressed: bool, repeat: bool) -> Input {
        Input::Key {
            key,
            pressed,
            repeat,
        }
    }

    #[test]
    fn same_key_matches_any_input_of_one_key() {
        let press = key_input(Key::KeyJ, true, false);
        assert!(press.same_key(&key_input(Key::KeyJ, false, false)));
        assert!(press.same_key(&key_input(Key::KeyJ, true, true)));
        assert!(!press.same_key(&key_input(Key::KeyK, true, false)));
        assert!(!press.same_key(&Input::Text('j')));
        assert!(!Input::PointerLeft.same_key(&Input::PointerLeft));
    }

    fn pad(button: PadButton, pressed: bool) -> Input {
        Input::PadButton { button, pressed }
    }

    #[test]
    fn same_key_matches_any_input_of_one_pad_button() {
        let press = pad(PadButton::South, true);
        assert!(press.same_key(&pad(PadButton::South, false)));
        assert!(!press.same_key(&pad(PadButton::East, true)));
        assert!(!press.same_key(&key_input(Key::KeyJ, true, false)));
        assert!(!key_input(Key::KeyJ, true, false).same_key(&press));
        let axis = Input::PadAxis {
            axis: PadAxis::LeftX,
            value: 0.5,
        };
        assert!(!axis.same_key(&axis));
    }

    #[test]
    fn released_is_the_release_of_its_pad_button() {
        assert_eq!(
            pad(PadButton::DPadUp, true).released(),
            pad(PadButton::DPadUp, false)
        );
        assert_eq!(
            pad(PadButton::DPadUp, false).released(),
            pad(PadButton::DPadUp, false)
        );
        let axis = Input::PadAxis {
            axis: PadAxis::RightY,
            value: -1.0,
        };
        assert_eq!(axis.released(), axis);
        let told = Input::Surroundings(Surroundings::default());
        assert_eq!(told.released(), told);
    }

    #[test]
    fn pad_inputs_and_surroundings_are_not_moved_by_a_translation() {
        let by = Point::new(-170.0, 2.0);
        for unmoved in [
            pad(PadButton::Start, true),
            Input::PadAxis {
                axis: PadAxis::LeftY,
                value: 0.25,
            },
            Input::Surroundings(Surroundings {
                device: Device::Gamepad(ControllerFamily::Generic),
            }),
        ] {
            assert_eq!(unmoved.translated(by), unmoved);
        }
    }

    #[test]
    fn every_pad_button_has_its_own_position_name() {
        let names: std::collections::BTreeSet<_> =
            PadButton::ALL.iter().map(|button| button.name()).collect();
        assert_eq!(names.len(), PadButton::ALL.len());
        assert_eq!(PadButton::South.name(), "south");
        assert_eq!(PadButton::Select.name(), "back");
        assert_eq!(PadButton::LeftStickDown.name(), "-lefty");
        assert_eq!(PadButton::RightStickRight.name(), "+rightx");
        let axes: std::collections::BTreeSet<_> =
            PadAxis::ALL.iter().map(|axis| axis.name()).collect();
        assert_eq!(axes.len(), 4);
        assert_eq!(PadAxis::RightX.name(), "rightx");
    }

    #[test]
    fn each_stick_direction_is_its_axis_pushed_its_way_and_back() {
        let mut directions = 0;
        for button in PadButton::ALL {
            if let Some((axis, sign)) = button.stick_direction() {
                assert_eq!(axis.direction(sign > 0), button, "{button:?}");
                directions += 1;
            }
        }
        assert_eq!(directions, 8);
        for axis in PadAxis::ALL {
            for positive in [true, false] {
                let button = axis.direction(positive);
                assert_eq!(
                    button.stick_direction(),
                    Some((axis, if positive { 1 } else { -1 }))
                );
            }
            assert_eq!(axis.across().across(), axis);
            assert_ne!(axis.across(), axis);
        }
        assert_eq!(PadAxis::LeftY.direction(false), PadButton::LeftStickDown);
        assert_eq!(PadAxis::RightX.across(), PadAxis::RightY);
        assert_eq!(PadButton::South.stick_direction(), None);
    }

    #[test]
    fn released_is_the_release_of_its_key() {
        assert_eq!(
            key_input(Key::KeyJ, true, true).released(),
            key_input(Key::KeyJ, false, false)
        );
        assert_eq!(Input::Text('j').released(), Input::Text('j'));
    }

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
