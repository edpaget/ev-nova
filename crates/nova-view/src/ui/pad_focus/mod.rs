//! The pad's focus: one directional picker over a screen's focusable
//! controls, and the one focus each screen keeps, which the pad and the
//! keyboard share (rdm:phase/key-bindings/phase-8-gamepad-focus).
//!
//! The original has no focus in its in-game dialogs and no directional
//! navigation anywhere (`_MyModalDialog` @0x59954, `_TextDialogFilter`
//! @0x51f77; the phase's findings), so all of this is a new feature with
//! no `Rulebook` key, and a focus move makes no sound.
//!
//! - [`Target`]s are a screen's controls: an id of the screen's own (an
//!   item number, a Preferences part, a keys-screen control), its rect in
//!   logical units, and, where the geometry misleads, an explicit
//!   neighbour in a direction. The slice a screen lists them in is its Tab
//!   order.
//! - [`pick`] chooses the target a d-pad direction moves to, and [`step`]
//!   applies a [`FocusMove`] (a direction, or the shoulders' Next and
//!   Previous) to a screen's focus.
//! - [`FocusState`] is the one focus a screen keeps, remembering whether
//!   the pad or the keyboard placed it, so the keyboard's ring and the
//!   pad's highlight ([`draw_highlight`]) each show only their own.
//!
//! # Nothing hidden acts
//!
//! The pad's focus moves and South act only while the screen has been
//! told the pad was the device used last ([`FocusState::pad_shown`]), so
//! the highlight shows where the focus is. Until then a d-pad press, a
//! shoulder or South does nothing ([`FocusState::pad_holds_back`]): it
//! neither moves the focus nor presses a control, not even one the
//! keyboard's ring shows. East and every other command act as their keys
//! do. A release passes, as it only finishes a press the pad made while
//! its focus showed.
//!
//! A press that arrives before the surroundings doesn't make the focus
//! appear either. The app tells each screen the device used last, and a
//! screen that showed the highlight on its own would never be told when
//! the player went back to the keyboard, leaving it stuck. So a screen the
//! app doesn't tell yet (the spaceport's side, until
//! rdm:phase/key-bindings/phase-9-gamepad-spaceport) ignores the pad's
//! focus altogether, rather than act on a control the player can't see.
//!
//! # Where additions go
//!
//! - A new screen lists its targets in its own module (a `pad` child
//!   module where its file is large), with the initial target the pad's
//!   focus starts on, and keeps its focus in a [`FocusState`].
//! - A layout whose geometry leads the picker astray adds explicit
//!   neighbours ([`Target::with_neighbour`]) in that screen's module, and
//!   says so in its docs.
//! - Every screen with a pad focus has a row in the table-driven tests
//!   (`every_screen.rs`), which reach and press each control from pad
//!   inputs alone.

use crate::color::Color;
use crate::draw::DrawList;
use crate::geometry::Bounds;
use crate::input::{Device, Input};
use crate::keymap::{Command, CommandInput, UiCommand};

use super::dialog::outline;

#[cfg(test)]
pub(crate) mod every_screen;
#[cfg(test)]
mod tests;

/// The weight of the gap across a direction against the gap along it,
/// in a candidate's score: a target off to the side loses to one twice as
/// far away in line.
pub const OFF_AXIS_WEIGHT: f32 = 2.0;

/// A d-pad direction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    /// Toward smaller y.
    Up,
    /// Toward larger y.
    Down,
    /// Toward smaller x.
    Left,
    /// Toward larger x.
    Right,
}

impl Direction {
    /// Its slot in a target's neighbours.
    const fn slot(self) -> usize {
        match self {
            Self::Up => 0,
            Self::Down => 1,
            Self::Left => 2,
            Self::Right => 3,
        }
    }
}

/// A move of the focus: toward a direction (the d-pad), or to the next or
/// previous control in Tab order (the shoulders, or Tab).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FocusMove {
    /// The nearest control that way.
    Toward(Direction),
    /// The next control in Tab order, wrapping.
    Next,
    /// The previous control in Tab order, wrapping.
    Previous,
}

impl FocusMove {
    /// The move `command` asks for: the four focus directions, Next Field
    /// (`Tab`) and Previous Field. `None` for any other command.
    #[must_use]
    pub const fn of(command: Command) -> Option<Self> {
        match command {
            Command::Ui(UiCommand::FocusUp) => Some(Self::Toward(Direction::Up)),
            Command::Ui(UiCommand::FocusDown) => Some(Self::Toward(Direction::Down)),
            Command::Ui(UiCommand::FocusLeft) => Some(Self::Toward(Direction::Left)),
            Command::Ui(UiCommand::FocusRight) => Some(Self::Toward(Direction::Right)),
            Command::Ui(UiCommand::Tab) => Some(Self::Next),
            Command::Ui(UiCommand::PrevFocus) => Some(Self::Previous),
            _ => None,
        }
    }
}

/// One control the focus can rest on.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Target<T> {
    /// The screen's own id for it.
    pub id: T,
    /// Where it is, in logical units.
    pub bounds: Bounds,
    /// The explicit neighbour each way, by [`Direction::slot`].
    neighbours: [Option<T>; 4],
}

impl<T: Copy + Eq> Target<T> {
    /// The control `id` at `bounds`, with no explicit neighbour.
    #[must_use]
    pub const fn at(id: T, bounds: Bounds) -> Self {
        Self {
            id,
            bounds,
            neighbours: [None; 4],
        }
    }

    /// The control with `id` as its neighbour toward `direction`, whatever
    /// the geometry says, while `id` is a target.
    #[must_use]
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "no screen's layout misleads the picker yet")
    )]
    pub const fn with_neighbour(mut self, direction: Direction, id: T) -> Self {
        self.neighbours[direction.slot()] = Some(id);
        self
    }

    /// The explicit neighbour toward `direction`, if any.
    #[must_use]
    pub const fn neighbour(&self, direction: Direction) -> Option<T> {
        self.neighbours[direction.slot()]
    }
}

/// The gap between `a` and `b` along `direction` (0 when they overlap or
/// `b` starts behind `a`'s far edge), the gap across it (0 when they
/// overlap there), and the offset of their centres across it.
fn gaps(a: Bounds, b: Bounds, direction: Direction) -> (f32, f32, f32) {
    let gap = |lo_a: f32, hi_a: f32, lo_b: f32, hi_b: f32| (lo_b - hi_a).max(lo_a - hi_b).max(0.0);
    let (ca, cb) = (a.center(), b.center());
    match direction {
        Direction::Right => (
            (b.min.x - a.max.x).max(0.0),
            gap(a.min.y, a.max.y, b.min.y, b.max.y),
            (cb.y - ca.y).abs(),
        ),
        Direction::Left => (
            (a.min.x - b.max.x).max(0.0),
            gap(a.min.y, a.max.y, b.min.y, b.max.y),
            (cb.y - ca.y).abs(),
        ),
        Direction::Down => (
            (b.min.y - a.max.y).max(0.0),
            gap(a.min.x, a.max.x, b.min.x, b.max.x),
            (cb.x - ca.x).abs(),
        ),
        Direction::Up => (
            (a.min.y - b.max.y).max(0.0),
            gap(a.min.x, a.max.x, b.min.x, b.max.x),
            (cb.x - ca.x).abs(),
        ),
    }
}

/// Whether `b`'s centre lies strictly beyond `a`'s toward `direction`.
fn beyond(a: Bounds, b: Bounds, direction: Direction) -> bool {
    let (ca, cb) = (a.center(), b.center());
    match direction {
        Direction::Right => cb.x > ca.x,
        Direction::Left => cb.x < ca.x,
        Direction::Down => cb.y > ca.y,
        Direction::Up => cb.y < ca.y,
    }
}

/// The target the focus on `from` moves to toward `direction`, or `None`
/// when nothing lies that way (the focus stays; directions never wrap).
///
/// `from`'s explicit neighbour that way wins while it is still a target.
/// Otherwise a candidate's centre must lie strictly beyond `from`'s along
/// the direction, and the lowest score wins: the edge gap along the
/// direction plus [`OFF_AXIS_WEIGHT`] times the edge gap across it. A tie
/// goes to the candidate whose centre is nearer across, then to the one
/// earlier in Tab order (the slice's).
#[must_use]
pub fn pick<T: Copy + Eq>(targets: &[Target<T>], from: T, direction: Direction) -> Option<T> {
    let origin = targets.iter().find(|target| target.id == from)?;
    if let Some(neighbour) = origin.neighbour(direction)
        && targets.iter().any(|target| target.id == neighbour)
    {
        return Some(neighbour);
    }
    targets
        .iter()
        .enumerate()
        .filter(|(_, target)| target.id != from && beyond(origin.bounds, target.bounds, direction))
        .map(|(order, target)| {
            let (along, across, offset) = gaps(origin.bounds, target.bounds, direction);
            (along + OFF_AXIS_WEIGHT * across, offset, order, target.id)
        })
        .min_by(|a, b| {
            a.0.total_cmp(&b.0)
                .then(a.1.total_cmp(&b.1))
                .then(a.2.cmp(&b.2))
        })
        .map(|(.., id)| id)
}

/// Where `mv` takes the focus `from` among `targets`.
///
/// With nothing focused, or a focus that is no longer a target, any move
/// lands on `initial` and goes no further. Next and Previous follow the
/// slice (Tab order) and wrap; a direction is [`pick`], the focus staying
/// where nothing lies that way. `None` only when there is no target.
#[must_use]
pub fn step<T: Copy + Eq>(
    targets: &[Target<T>],
    from: Option<T>,
    initial: T,
    mv: FocusMove,
) -> Option<T> {
    if targets.is_empty() {
        return None;
    }
    let Some((at, from)) = from.and_then(|from| {
        targets
            .iter()
            .position(|t| t.id == from)
            .map(|at| (at, from))
    }) else {
        return Some(initial);
    };
    let count = targets.len();
    Some(match mv {
        FocusMove::Next => targets[(at + 1) % count].id,
        FocusMove::Previous => targets[(at + count - 1) % count].id,
        FocusMove::Toward(direction) => pick(targets, from, direction).unwrap_or(from),
    })
}

/// The one focus a screen keeps, shared by the pad and the keyboard, and
/// the device the player used last.
///
/// It remembers who placed the focus: the keyboard's ring shows only a
/// focus the keyboard placed ([`FocusState::keyboard_ring`]), so a
/// keyboard-only session draws exactly what it always did; the pad's
/// highlight only one the pad placed, while the pad was the device used
/// last ([`FocusState::pad_highlight`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FocusState<T> {
    /// The focused control, if any.
    focus: Option<T>,
    /// Whether the pad placed it.
    pad_moved: bool,
    /// The device the player used last, as the last surroundings said.
    device: Device,
}

impl<T> Default for FocusState<T> {
    fn default() -> Self {
        Self {
            focus: None,
            pad_moved: false,
            device: Device::Keyboard,
        }
    }
}

impl<T: Copy + Eq> FocusState<T> {
    /// The focus, whoever placed it.
    #[must_use]
    pub const fn get(&self) -> Option<T> {
        self.focus
    }

    /// The device the player used last, as the last surroundings said;
    /// the keyboard before any.
    #[must_use]
    pub const fn device(&self) -> Device {
        self.device
    }

    /// Whether the pad was the device used last.
    #[must_use]
    pub const fn pad_shown(&self) -> bool {
        matches!(self.device, Device::Gamepad(_))
    }

    /// Whether the pad placed the focus.
    #[must_use]
    pub const fn pad_moved(&self) -> bool {
        self.pad_moved
    }

    /// Whether a screen drops the pad's command `input`: a focus move's or
    /// South's press while the pad's focus can't show, because the screen
    /// hasn't been told the pad was the device used last
    /// ([`FocusState::pad_shown`]). See the module docs' "Nothing hidden
    /// acts". Releases, and every other command, pass.
    #[must_use]
    pub fn pad_holds_back(&self, input: CommandInput) -> bool {
        input.pressed
            && !self.pad_shown()
            && (FocusMove::of(input.command).is_some()
                || input.command == Command::Ui(UiCommand::Activate))
    }

    /// The keyboard (or the screen itself) sets the focus.
    pub const fn set_by_keys(&mut self, focus: Option<T>) {
        self.focus = focus;
        self.pad_moved = false;
    }

    /// The pad sets the focus.
    pub const fn set_by_pad(&mut self, focus: T) {
        self.focus = Some(focus);
        self.pad_moved = true;
    }

    /// The device used last changed to `device`.
    ///
    /// To the pad, the highlight shows and, with nothing focused, the
    /// focus lands on `initial` as the pad's. To any other device, the
    /// highlight hides and a focus the pad placed is dropped, unless the
    /// screen `keeps` its focus always (the keys screen), when it becomes
    /// the keyboard's.
    pub const fn surroundings(&mut self, device: Device, initial: Option<T>, keeps: bool) {
        self.device = device;
        if let Device::Gamepad(_) = device {
            if self.focus.is_none()
                && let Some(initial) = initial
            {
                self.set_by_pad(initial);
            }
        } else if self.pad_moved {
            if keeps {
                self.pad_moved = false;
            } else {
                self.set_by_keys(None);
            }
        }
    }

    /// The focus the keyboard's ring shows: one the keyboard placed.
    #[must_use]
    pub const fn keyboard_ring(&self) -> Option<T> {
        if self.pad_moved { None } else { self.focus }
    }

    /// The focus the pad's highlight shows: one the pad placed, while the
    /// pad was the device used last.
    #[must_use]
    pub const fn pad_highlight(&self) -> Option<T> {
        if self.pad_moved && self.pad_shown() {
            self.focus
        } else {
            None
        }
    }

    /// Takes `other`'s device, and its focus if the pad placed it: how a
    /// screen laid out again keeps the pad's place.
    pub const fn carry_pad_from(&mut self, other: &Self) {
        self.device = other.device;
        if other.pad_moved
            && let Some(focus) = other.focus
        {
            self.set_by_pad(focus);
        }
    }
}

/// Draws the pad's highlight around `rect`: a two-unit ring just outside
/// the control, in `color`, distinct from the keyboard's one-unit outline
/// on the control's edge.
pub fn draw_highlight(list: &mut DrawList, rect: Bounds, color: Color) {
    outline(list, rect.grown(2.0), color);
    outline(list, rect.grown(3.0), color);
}

/// Whether `input` came from a pad's button: how a screen tells a pad
/// command from a key's, as both come out of one reader.
#[must_use]
pub const fn from_pad(input: &Input) -> bool {
    matches!(input, Input::PadButton { .. })
}
