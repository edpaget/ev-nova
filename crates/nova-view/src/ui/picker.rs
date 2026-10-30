//! A row picker: one row holding two arrows, stacked, and the chosen
//! entry's label, stepping through a list of entries without wrapping.
//! The display mode picker ([`ModePicker`](super::mode_picker::ModePicker))
//! and the Preferences dialog's Scaling list are pickers.
//!
//! Its arrows are drawn and tracked as the volume control's are
//! ([`draw_arrow`], [`ButtonTracker`]): the interface file's `PICT`s 134
//! (up) and 135 (down), 136 and 137 while held. Its label is set as the
//! volume's number is.

use crate::draw::DrawList;
use crate::font::Font;
use crate::geometry::{Bounds, Point};
use crate::input::{Input, Key};
use crate::scaling::Scaling;
use crate::video_mode::FullscreenKind;

use super::button::ButtonTracker;
use super::dialog::outline;
use super::volume::{Arrow, COLOR, TEXT_SIZE, draw_arrow};

/// Each arrow's width.
pub const ARROW_WIDTH: f32 = 11.0;

/// Each arrow's height: the two stack in an 18-unit row.
pub const ARROW_HEIGHT: f32 = 9.0;

/// How far right of the row's left edge the arrows start, as the volume's
/// do of its label.
pub const ARROW_INSET: f32 = 1.0;

/// How far right of the row's left edge the label starts, as the volume's
/// number does.
pub const LABEL_INSET: f32 = 18.0;

/// Where a picker's parts are on the screen.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PickerRects {
    /// The chosen entry's label.
    pub value: Bounds,
    /// The arrow that steps toward the list's start.
    pub up: Bounds,
    /// The arrow that steps toward the list's end.
    pub down: Bounds,
}

impl PickerRects {
    /// The parts of a picker in `row`: the arrows [`ARROW_INSET`] in from
    /// its left, the up one on top, and the label from [`LABEL_INSET`] to
    /// its right edge.
    #[must_use]
    pub fn in_row(row: Bounds) -> Self {
        let left = row.min.x + ARROW_INSET;
        let arrow = |top: f32| Bounds {
            min: Point::new(left, top),
            max: Point::new(left + ARROW_WIDTH, top + ARROW_HEIGHT),
        };
        Self {
            value: Bounds {
                min: Point::new(row.min.x + LABEL_INSET, row.min.y),
                max: row.max,
            },
            up: arrow(row.min.y),
            down: arrow(row.min.y + ARROW_HEIGHT),
        }
    }

    /// The smallest rectangle round every part.
    #[must_use]
    pub fn bounds(&self) -> Bounds {
        let parts = [self.value, self.up, self.down];
        Bounds::around(parts.iter().flat_map(|part| [part.min, part.max])).expect("three parts")
    }
}

/// An entry a [`Picker`] offers: a plain value with a label to show.
pub trait PickerEntry: Copy + PartialEq + std::fmt::Debug {
    /// What the picker shows for it.
    fn label(&self) -> String;
}

impl PickerEntry for FullscreenKind {
    /// "Borderless", or the mode's size and rate.
    fn label(&self) -> String {
        FullscreenKind::label(self)
    }
}

impl PickerEntry for Scaling {
    /// "Scaling: " and the scaling's name.
    fn label(&self) -> String {
        Scaling::label(*self).to_owned()
    }
}

/// A choice among `T`s, stepped by its arrows or the Up and Down keys
/// without wrapping. It always offers at least one entry.
#[derive(Clone, Debug, PartialEq)]
pub struct Picker<T> {
    rects: PickerRects,
    entries: Vec<T>,
    selected: usize,
    /// The arrow a press is held on, and its tracking.
    tracked: Option<Arrow>,
    tracker: ButtonTracker,
}

impl<T: PickerEntry> Picker<T> {
    /// A picker in `row` offering `entries`, on `current`: see
    /// [`Picker::set_entries`].
    #[must_use]
    pub fn new(row: Bounds, entries: Vec<T>, current: T) -> Self {
        let mut picker = Self {
            rects: PickerRects::in_row(row),
            entries: vec![current],
            selected: 0,
            tracked: None,
            tracker: ButtonTracker::default(),
        };
        picker.set_entries(entries, current);
        picker
    }

    /// Offers `entries`, in order, selecting `current`, or the first entry
    /// when `current` is not among them. No entries at all offer `current`
    /// alone.
    pub fn set_entries(&mut self, entries: Vec<T>, current: T) {
        self.entries = if entries.is_empty() {
            vec![current]
        } else {
            entries
        };
        self.selected = self
            .entries
            .iter()
            .position(|&entry| entry == current)
            .unwrap_or(0);
    }

    /// Where its parts are.
    #[must_use]
    pub fn rects(&self) -> PickerRects {
        self.rects
    }

    /// What it offers, in order.
    #[must_use]
    pub fn entries(&self) -> &[T] {
        &self.entries
    }

    /// The entry chosen.
    #[must_use]
    pub fn selected(&self) -> T {
        self.entries[self.selected]
    }

    /// The arrow at `at`, if any: only the arrows take clicks. Each is
    /// hit-tested half-open, as the Dialog Manager's `PtInRect` does, so
    /// the edge the two share is the down arrow's.
    #[must_use]
    pub fn arrow_at(&self, at: Point) -> Option<Arrow> {
        [Arrow::Up, Arrow::Down]
            .into_iter()
            .find(|&arrow| self.arrow_rect(arrow).contains_half_open(at))
    }

    fn arrow_rect(&self, arrow: Arrow) -> Bounds {
        match arrow {
            Arrow::Up => self.rects.up,
            Arrow::Down => self.rects.down,
        }
    }

    /// Whether `arrow` shows pressed: a press held on it, with the pointer
    /// still on it.
    #[must_use]
    pub fn pressed(&self, arrow: Arrow) -> bool {
        self.tracked == Some(arrow) && self.tracker.pressed()
    }

    /// Handles one input event; `true` when the selection changes.
    ///
    /// - A click on an arrow (a press and a release on it, tracked as a
    ///   button tracks it) steps the selection: up toward the list's
    ///   start, down toward its end, stopping at either.
    /// - Up and Down step it the same way, key repeats included. The
    ///   owner sends keys only while the picker has the focus.
    pub fn input(&mut self, input: &Input) -> bool {
        let step = match *input {
            Input::Key {
                key: Key::Up,
                pressed: true,
                ..
            } => Some(Arrow::Up),
            Input::Key {
                key: Key::Down,
                pressed: true,
                ..
            } => Some(Arrow::Down),
            Input::Key { .. } | Input::Text(_) | Input::PointerLeft | Input::Scroll { .. } => None,
            Input::PointerButton { .. } | Input::PointerMoved(_) => self.pointer(input),
        };
        step.is_some_and(|arrow| self.step(arrow))
    }

    /// The arrow a click on it activates, if this input completes one.
    fn pointer(&mut self, input: &Input) -> Option<Arrow> {
        if let Input::PointerButton {
            pressed: true, at, ..
        } = *input
        {
            self.cancel_pointer();
            self.tracked = self.arrow_at(at);
        }
        let arrow = self.tracked?;
        let clicked = self.tracker.track(self.arrow_rect(arrow), true, input);
        if !self.tracker.armed() {
            self.tracked = None;
        }
        clicked.then_some(arrow)
    }

    /// Steps the selection one way; `true` when it changes.
    fn step(&mut self, arrow: Arrow) -> bool {
        let selected = match arrow {
            Arrow::Up => self.selected.saturating_sub(1),
            Arrow::Down => (self.selected + 1).min(self.entries.len() - 1),
        };
        let changed = selected != self.selected;
        self.selected = selected;
        changed
    }

    /// Abandons a click in progress without stepping.
    pub fn cancel_pointer(&mut self) {
        self.tracker.cancel_pointer();
        self.tracked = None;
    }

    /// Draws the chosen entry's label in Geneva at its rectangle's corner,
    /// then the two arrows (the interface file's `PICT`s 134 and 135, or
    /// 136 and 137 while pressed), and, when `focused`, an outline round
    /// every part.
    pub fn draw(&self, focused: bool, list: &mut DrawList) {
        list.text_in(
            Font::Geneva,
            self.selected().label(),
            self.rects.value.min,
            TEXT_SIZE,
            None,
            COLOR,
        );
        for arrow in [Arrow::Up, Arrow::Down] {
            draw_arrow(list, self.arrow_rect(arrow), arrow, self.pressed(arrow));
        }
        if focused {
            outline(list, self.rects.bounds(), COLOR);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::draw::DrawCommand;

    fn row() -> Bounds {
        Bounds {
            min: Point::new(171.0, 55.0),
            max: Point::new(325.0, 73.0),
        }
    }

    const NATIVE: Scaling = Scaling::Native;
    const SHARP: Scaling = Scaling::SharpBilinear;

    fn key(key: Key) -> Input {
        Input::Key {
            key,
            pressed: true,
            repeat: false,
        }
    }

    #[test]
    fn it_offers_exactly_the_entries_it_is_given_in_order() {
        let alone = Picker::new(row(), vec![NATIVE], NATIVE);
        assert_eq!(alone.entries(), [NATIVE]);
        let both = Picker::new(row(), vec![NATIVE, SHARP], SHARP);
        assert_eq!(both.entries(), [NATIVE, SHARP]);
        assert_eq!(both.selected(), SHARP);
        assert_eq!(both.rects(), PickerRects::in_row(row()));
    }

    #[test]
    fn a_current_entry_not_offered_selects_the_first_and_none_offers_the_current() {
        assert_eq!(Picker::new(row(), vec![NATIVE], SHARP).selected(), NATIVE);
        let mut picker = Picker::new(row(), Vec::new(), SHARP);
        assert_eq!(picker.entries(), [SHARP]);
        assert_eq!(picker.selected(), SHARP);
        picker.set_entries(vec![SHARP, NATIVE], NATIVE);
        assert_eq!(picker.selected(), NATIVE);
    }

    #[test]
    fn it_steps_without_wrapping_and_draws_the_chosen_label() {
        let mut picker = Picker::new(row(), vec![NATIVE, SHARP], NATIVE);
        assert!(!picker.input(&key(Key::Up)));
        assert!(picker.input(&key(Key::Down)));
        assert!(!picker.input(&key(Key::Down)));
        assert_eq!(picker.selected(), SHARP);
        let mut list = DrawList::new();
        picker.draw(false, &mut list);
        assert_eq!(
            list.iter().next(),
            Some(&DrawCommand::Text {
                text: "Scaling: Sharp-bilinear".to_owned(),
                font: Font::Geneva,
                origin: picker.rects().value.min,
                size: TEXT_SIZE,
                wrap_width: None,
                color: COLOR,
            })
        );
    }

    /// The arrows are hit-tested half-open, as the Dialog Manager's
    /// `PtInRect` tests the volume's: the edge they share is the down
    /// arrow's top, and the down arrow's right and bottom edges are off it.
    #[test]
    #[allow(clippy::float_cmp)] // Whole pixels.
    fn a_press_on_the_edge_the_arrows_share_steps_down() {
        let mut picker = Picker::new(row(), vec![NATIVE, SHARP], NATIVE);
        let PickerRects { up, down, .. } = picker.rects();
        assert_eq!(up.max.y, down.min.y, "stacked");
        let shared = Point::new(up.center().x, up.max.y);
        assert_eq!(picker.arrow_at(shared), Some(Arrow::Down));
        assert_eq!(picker.arrow_at(up.min), Some(Arrow::Up));
        assert_eq!(picker.arrow_at(down.max), None);
        assert_eq!(
            picker.arrow_at(Point::new(down.max.x - 1.0, down.max.y - 1.0)),
            Some(Arrow::Down)
        );
        let press = |pressed| Input::PointerButton {
            button: crate::input::MouseButton::Left,
            pressed,
            at: shared,
        };
        picker.input(&press(true));
        assert!(picker.input(&press(false)), "stepped");
        assert_eq!(picker.selected(), SHARP);
    }

    #[test]
    fn each_entry_is_labelled_by_its_own_label() {
        assert_eq!(PickerEntry::label(&SHARP), "Scaling: Sharp-bilinear");
        for scaling in Scaling::ALL {
            assert_eq!(PickerEntry::label(&scaling), scaling.label());
        }
        assert_eq!(PickerEntry::label(&Scaling::SuperXbr), "Scaling: Super-xBR");
        assert_eq!(
            PickerEntry::label(&FullscreenKind::Borderless),
            "Borderless"
        );
    }
}
