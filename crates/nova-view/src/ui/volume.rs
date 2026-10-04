//! A volume control: a label, the level as a number and two arrows that
//! step it, as "new prefs dialog" lays its Sound Volume out.
//!
//! The arrows are the interface file's `PICT`s 134 and 135 in the
//! original (136 and 137 pressed), which cannot be drawn yet: each is
//! drawn here as an 11 x 9 triangle, row by row, outlined at rest and
//! filled while pressed.

use crate::color::Color;
use crate::draw::DrawList;
use crate::font::Font;
use crate::geometry::{Bounds, Point};
use crate::input::{Input, Key};
use crate::sound::MAX_LEVEL;

use super::button::ButtonTracker;
use super::dialog::outline;

/// The label's and the number's size, in Geneva.
pub const TEXT_SIZE: f32 = 12.0;

/// The label's, the number's and the arrows' colour.
pub const COLOR: Color = Color::WHITE;

/// Where a volume control's parts are on the screen.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VolumeRects {
    /// The label.
    pub label: Bounds,
    /// The level, as a number.
    pub value: Bounds,
    /// The arrow that raises it.
    pub up: Bounds,
    /// The arrow that lowers it.
    pub down: Bounds,
}

impl VolumeRects {
    /// Every part moved by `by`.
    #[must_use]
    pub fn offset(self, by: Point) -> Self {
        Self {
            label: self.label.offset(by),
            value: self.value.offset(by),
            up: self.up.offset(by),
            down: self.down.offset(by),
        }
    }

    /// The smallest rectangle round every part.
    #[must_use]
    pub fn bounds(&self) -> Bounds {
        let parts = [self.label, self.value, self.up, self.down];
        Bounds::around(parts.iter().flat_map(|part| [part.min, part.max])).expect("four parts")
    }
}

/// One of the two arrows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Arrow {
    /// Raises the level.
    Up,
    /// Lowers the level.
    Down,
}

/// A volume level, 0 through [`MAX_LEVEL`], that its arrows and the arrow
/// keys step.
#[derive(Clone, Debug, PartialEq)]
pub struct VolumeControl {
    rects: VolumeRects,
    label: String,
    level: u8,
    /// The arrow a press is held on, and its tracking.
    tracked: Option<Arrow>,
    tracker: ButtonTracker,
}

impl VolumeControl {
    /// A control with its parts at `rects`, labelled `label`, at `level`
    /// (clamped to [`MAX_LEVEL`]).
    #[must_use]
    pub fn new(rects: VolumeRects, label: impl Into<String>, level: u8) -> Self {
        Self {
            rects,
            label: label.into(),
            level: level.min(MAX_LEVEL),
            tracked: None,
            tracker: ButtonTracker::default(),
        }
    }

    /// Where its parts are.
    #[must_use]
    pub fn rects(&self) -> VolumeRects {
        self.rects
    }

    /// Its label.
    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }

    /// The level, 0 through [`MAX_LEVEL`].
    #[must_use]
    pub fn level(&self) -> u8 {
        self.level
    }

    /// The arrow at `at`, if any: only the arrows take clicks.
    #[must_use]
    pub fn arrow_at(&self, at: Point) -> Option<Arrow> {
        [Arrow::Up, Arrow::Down]
            .into_iter()
            .find(|&arrow| self.arrow_rect(arrow).contains(at))
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

    /// Handles one input event; `true` when the level changes.
    ///
    /// - A click on an arrow (a press and a release on it, tracked as a
    ///   button tracks it) steps the level one up or down, within 0
    ///   through [`MAX_LEVEL`].
    /// - Up and Right raise it one, and Down and Left lower it one, key
    ///   repeats included. The owner sends keys only while the control
    ///   has the focus.
    pub fn input(&mut self, input: &Input) -> bool {
        let step = match *input {
            Input::Key {
                key: Key::Up | Key::Right,
                pressed: true,
                ..
            } => Some(Arrow::Up),
            Input::Key {
                key: Key::Down | Key::Left,
                pressed: true,
                ..
            } => Some(Arrow::Down),
            Input::Key { .. } | Input::Text(_) => None,
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

    /// Steps the level one way; `true` when it changes.
    fn step(&mut self, arrow: Arrow) -> bool {
        let level = match arrow {
            Arrow::Up => (self.level + 1).min(MAX_LEVEL),
            Arrow::Down => self.level.saturating_sub(1),
        };
        let changed = level != self.level;
        self.level = level;
        changed
    }

    /// Abandons a click in progress without stepping.
    pub fn cancel_pointer(&mut self) {
        self.tracker.cancel_pointer();
        self.tracked = None;
    }

    /// Draws the label and the level in Geneva at their rectangles'
    /// corners, then the two arrows (each filled while pressed), and, when
    /// `focused`, an outline round every part.
    pub fn draw(&self, focused: bool, list: &mut DrawList) {
        let rects = self.rects;
        list.text_in(
            Font::Geneva,
            self.label.as_str(),
            rects.label.min,
            TEXT_SIZE,
            None,
            COLOR,
        );
        list.text_in(
            Font::Geneva,
            self.level.to_string(),
            rects.value.min,
            TEXT_SIZE,
            None,
            COLOR,
        );
        for arrow in [Arrow::Up, Arrow::Down] {
            let filled = self.pressed(arrow);
            draw_arrow(list, self.arrow_rect(arrow), arrow, filled, COLOR);
        }
        if focused {
            outline(list, rects.bounds(), COLOR);
        }
    }
}

/// Draws `arrow` as a triangle filling `rect` (11 x 9 for the stock
/// arrows): its point at the top for [`Arrow::Up`], at the bottom for
/// [`Arrow::Down`], each row a 1-unit line, its edges only unless
/// `filled`.
pub fn draw_arrow(list: &mut DrawList, rect: Bounds, arrow: Arrow, filled: bool, color: Color) {
    // Whole rows only: the stock arrows are 9 rows high.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let height = rect.height().round().max(1.0) as u16;
    let center = rect.center().x;
    let widest = (rect.width() - 1.0) / 2.0;
    let base = height - 1;
    for from_point in 0..height {
        let half = if base == 0 {
            widest
        } else {
            (f32::from(from_point) * widest / f32::from(base)).round()
        };
        let y = match arrow {
            Arrow::Up => rect.min.y + f32::from(from_point) + 0.5,
            Arrow::Down => rect.max.y - f32::from(from_point) - 0.5,
        };
        let (left, right) = (center - half - 0.5, center + half + 0.5);
        let mut span = |from: f32, to: f32| {
            list.line(Point::new(from, y), Point::new(to, y), 1.0, color);
        };
        if filled || from_point == base || half == 0.0 {
            span(left, right);
        } else {
            span(left, left + 1.0);
            span(right - 1.0, right);
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::draw::DrawCommand;
    use crate::input::MouseButton;

    fn at(x: f32, y: f32) -> Point {
        Point::new(x, y)
    }

    /// Bounds from (left, top) to (right, bottom).
    fn ltrb(left: f32, top: f32, right: f32, bottom: f32) -> Bounds {
        Bounds {
            min: at(left, top),
            max: at(right, bottom),
        }
    }

    /// Stock "new prefs dialog"'s Sound Volume (items 4, 5, 7 and 6),
    /// dialog-local.
    fn stock() -> VolumeRects {
        VolumeRects {
            label: ltrb(171.0, 167.0, 277.0, 183.0),
            value: ltrb(189.0, 186.0, 311.0, 202.0),
            up: ltrb(172.0, 185.0, 183.0, 194.0),
            down: ltrb(172.0, 194.0, 183.0, 203.0),
        }
    }

    fn control(level: u8) -> VolumeControl {
        VolumeControl::new(stock(), "Sound Volume:", level)
    }

    fn button(pressed: bool, at: Point) -> Input {
        Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at,
        }
    }

    fn click(control: &mut VolumeControl, at: Point) -> bool {
        assert!(!control.input(&button(true, at)), "a press never steps");
        control.input(&button(false, at))
    }

    fn key(key: Key, repeat: bool) -> Input {
        Input::Key {
            key,
            pressed: true,
            repeat,
        }
    }

    fn drawn(control: &VolumeControl, focused: bool) -> Vec<DrawCommand> {
        let mut list = DrawList::new();
        control.draw(focused, &mut list);
        list.iter().cloned().collect()
    }

    fn arrow(rect: Bounds, arrow: Arrow, filled: bool) -> Vec<DrawCommand> {
        let mut list = DrawList::new();
        draw_arrow(&mut list, rect, arrow, filled, COLOR);
        list.iter().cloned().collect()
    }

    fn row(left: f32, right: f32, y: f32) -> DrawCommand {
        DrawCommand::Line {
            from: at(left, y),
            to: at(right, y),
            width: 1.0,
            color: COLOR,
        }
    }

    #[test]
    fn its_parts_are_the_items_moved_by_an_offset() {
        let moved = stock().offset(at(10.0, 40.0));
        assert_eq!(moved.label, ltrb(181.0, 207.0, 287.0, 223.0));
        assert_eq!(moved.value, ltrb(199.0, 226.0, 321.0, 242.0));
        assert_eq!(moved.up, ltrb(182.0, 225.0, 193.0, 234.0));
        assert_eq!(moved.down, ltrb(182.0, 234.0, 193.0, 243.0));
        assert_eq!(stock().bounds(), ltrb(171.0, 167.0, 311.0, 203.0));
        assert_eq!(moved.bounds(), ltrb(181.0, 207.0, 321.0, 243.0));
        let control = control(3);
        assert_eq!(
            (control.rects(), control.label(), control.level()),
            (stock(), "Sound Volume:", 3)
        );
    }

    #[test]
    fn a_level_above_the_loudest_is_clamped() {
        assert_eq!(control(8).level(), MAX_LEVEL);
        assert_eq!(control(u8::MAX).level(), MAX_LEVEL);
        assert_eq!(control(7).level(), 7);
    }

    #[test]
    fn only_the_arrows_take_clicks() {
        let control = control(3);
        assert_eq!(control.arrow_at(stock().up.center()), Some(Arrow::Up));
        assert_eq!(control.arrow_at(stock().down.center()), Some(Arrow::Down));
        assert_eq!(control.arrow_at(at(172.0, 185.0)), Some(Arrow::Up));
        assert_eq!(control.arrow_at(at(183.0, 203.0)), Some(Arrow::Down));
        for elsewhere in [stock().label.center(), stock().value.center(), at(0.0, 0.0)] {
            assert_eq!(control.arrow_at(elsewhere), None, "{elsewhere:?}");
        }
    }

    #[test]
    fn a_click_on_an_arrow_steps_the_level_within_its_range() {
        let mut control = control(6);
        assert!(click(&mut control, stock().up.center()));
        assert_eq!(control.level(), 7);
        assert!(!click(&mut control, stock().up.center()), "the loudest");
        assert_eq!(control.level(), 7);
        assert!(click(&mut control, stock().down.center()));
        assert_eq!(control.level(), 6);
        let mut control = super::tests::control(0);
        assert!(!click(&mut control, stock().down.center()), "silent");
        assert_eq!(control.level(), 0);
    }

    #[test]
    fn a_click_on_the_label_or_the_number_does_nothing() {
        let mut control = control(3);
        assert!(!click(&mut control, stock().label.center()));
        assert!(!click(&mut control, stock().value.center()));
        assert_eq!(control.level(), 3);
    }

    #[test]
    fn a_press_let_go_of_off_its_arrow_does_not_step() {
        let mut control = control(3);
        control.input(&button(true, stock().up.center()));
        assert!(control.pressed(Arrow::Up));
        assert!(!control.pressed(Arrow::Down));
        control.input(&Input::PointerMoved(stock().down.center()));
        assert!(!control.pressed(Arrow::Up), "off it");
        assert!(!control.input(&button(false, stock().down.center())));
        assert_eq!(control.level(), 3);
        assert!(!control.pressed(Arrow::Up), "let go");
        // A cancelled press does not step either.
        control.input(&button(true, stock().down.center()));
        assert!(control.pressed(Arrow::Down));
        control.cancel_pointer();
        assert!(!control.pressed(Arrow::Down));
        assert!(!control.input(&button(false, stock().down.center())));
        assert_eq!(control.level(), 3);
    }

    #[test]
    fn the_arrow_keys_step_the_level_repeats_included() {
        let mut control = control(3);
        assert!(control.input(&key(Key::Up, false)));
        assert!(control.input(&key(Key::Right, true)));
        assert_eq!(control.level(), 5);
        assert!(control.input(&key(Key::Down, false)));
        assert!(control.input(&key(Key::Left, true)));
        assert!(control.input(&key(Key::Left, true)));
        assert_eq!(control.level(), 2);
        let release = Input::Key {
            key: Key::Up,
            pressed: false,
            repeat: false,
        };
        let typed = Input::Text('u');
        for other in [
            release,
            key(Key::Space, false),
            key(Key::Enter, false),
            typed,
        ] {
            assert!(!control.input(&other), "{other:?}");
        }
        assert_eq!(control.level(), 2);
        let mut control = super::tests::control(7);
        assert!(!control.input(&key(Key::Up, false)));
        let mut control = super::tests::control(0);
        assert!(!control.input(&key(Key::Left, false)));
        assert_eq!(control.level(), 0);
    }

    #[test]
    fn a_filled_arrow_is_one_line_a_row_widening_to_the_base() {
        let up = stock().up;
        let rows = arrow(up, Arrow::Up, true);
        assert_eq!(rows.len(), 9);
        // Centred on x = 177.5; the half-width grows 5/8 a row, rounded.
        assert_eq!(rows[0], row(177.0, 178.0, 185.5));
        assert_eq!(rows[1], row(176.0, 179.0, 186.5));
        assert_eq!(rows[4], row(174.0, 181.0, 189.5));
        assert_eq!(rows[8], row(172.0, 183.0, 193.5));
        let down = arrow(stock().down, Arrow::Down, true);
        assert_eq!(down[0], row(177.0, 178.0, 202.5), "pointing down");
        assert_eq!(down[8], row(172.0, 183.0, 194.5));
    }

    #[test]
    fn an_outlined_arrow_draws_each_rows_ends_and_the_whole_base() {
        let up = stock().up;
        let lines = arrow(up, Arrow::Up, false);
        // The point, two ends on each of the seven rows between, the base.
        assert_eq!(lines.len(), 1 + 7 * 2 + 1);
        assert_eq!(lines[0], row(177.0, 178.0, 185.5));
        assert_eq!(lines[1], row(176.0, 177.0, 186.5));
        assert_eq!(lines[2], row(178.0, 179.0, 186.5));
        assert_eq!(lines[15], row(172.0, 183.0, 193.5));
    }

    #[test]
    fn a_one_row_arrow_is_one_line_across() {
        let flat = ltrb(10.0, 20.0, 13.0, 21.0);
        for filled in [false, true] {
            assert_eq!(
                arrow(flat, Arrow::Up, filled),
                [row(10.0, 13.0, 20.5)],
                "{filled}"
            );
        }
    }

    fn text(text: &str, origin: Point) -> DrawCommand {
        DrawCommand::Text {
            text: text.to_owned(),
            font: Font::Geneva,
            origin,
            size: TEXT_SIZE,
            wrap_width: None,
            color: COLOR,
        }
    }

    #[test]
    fn it_draws_its_label_its_level_and_its_arrows() {
        let control = control(5);
        let mut expected = vec![
            text("Sound Volume:", stock().label.min),
            text("5", stock().value.min),
        ];
        expected.extend(arrow(stock().up, Arrow::Up, false));
        expected.extend(arrow(stock().down, Arrow::Down, false));
        assert_eq!(drawn(&control, false), expected);
        assert_eq!((TEXT_SIZE, COLOR), (12.0, Color::WHITE));
    }

    #[test]
    fn a_pressed_arrow_is_filled() {
        let mut control = control(5);
        control.input(&button(true, stock().down.center()));
        let mut expected = vec![
            text("Sound Volume:", stock().label.min),
            text("5", stock().value.min),
        ];
        expected.extend(arrow(stock().up, Arrow::Up, false));
        expected.extend(arrow(stock().down, Arrow::Down, true));
        assert_eq!(drawn(&control, false), expected);
    }

    #[test]
    fn focused_it_is_outlined() {
        let control = control(5);
        let mut expected = drawn(&control, false);
        let mut ring = DrawList::new();
        outline(&mut ring, stock().bounds(), COLOR);
        expected.extend(ring.iter().cloned());
        assert_eq!(drawn(&control, true), expected);
    }
}
