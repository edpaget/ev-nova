//! A check box: an on/off toggle with a label, clicked like a button or
//! flipped with Space while it has the focus.
//!
//! The original drew its check boxes with the system's control; there is
//! no stock graphic, so a toggle draws a small outlined box at the left of
//! its rectangle, crossed while it is on, and its label beside it in
//! Geneva. A greyed toggle is drawn dim and takes no input.

use crate::color::Color;
use crate::draw::{DrawList, crossed_box};
use crate::font::Font;
use crate::geometry::{Bounds, Point};
use crate::input::{Input, Key};
use crate::text::TextMetrics;

use super::button::ButtonTracker;
use super::dialog::outline;

/// The box's side.
pub const BOX_SIZE: f32 = 12.0;

/// The space between the box and the label.
pub const LABEL_GAP: f32 = 4.0;

/// The label's size, in Geneva.
pub const LABEL_SIZE: f32 = 12.0;

/// An enabled toggle's colour.
pub const ENABLED_COLOR: Color = Color::WHITE;

/// A greyed toggle's colour.
pub const GREYED_COLOR: Color = Color::DIM;

/// An on/off check box with a label.
#[derive(Clone, Debug, PartialEq)]
pub struct Toggle {
    rect: Bounds,
    label: String,
    on: bool,
    enabled: bool,
    tracker: ButtonTracker,
}

impl Toggle {
    /// An enabled toggle at `rect` (on the screen), labelled `label`, `on`
    /// or off.
    #[must_use]
    pub fn new(rect: Bounds, label: impl Into<String>, on: bool) -> Self {
        Self {
            rect,
            label: label.into(),
            on,
            enabled: true,
            tracker: ButtonTracker::default(),
        }
    }

    /// A greyed toggle: off, drawn dim, ignoring every input.
    #[must_use]
    pub fn greyed(rect: Bounds, label: impl Into<String>) -> Self {
        Self {
            enabled: false,
            ..Self::new(rect, label, false)
        }
    }

    /// Where it is on the screen.
    #[must_use]
    pub fn rect(&self) -> Bounds {
        self.rect
    }

    /// Its label.
    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }

    /// Whether it is on.
    #[must_use]
    pub fn on(&self) -> bool {
        self.on
    }

    /// Whether it takes input (and so the focus): not greyed.
    #[must_use]
    pub fn enabled(&self) -> bool {
        self.enabled
    }

    /// Where the box is: at the left of the rectangle, centred down it.
    #[must_use]
    pub fn box_bounds(&self) -> Bounds {
        let top = self.rect.center().y - BOX_SIZE / 2.0;
        Bounds::at(Point::new(self.rect.min.x, top), BOX_SIZE, BOX_SIZE)
    }

    /// Where the label starts: after the box and the gap, centred down
    /// the rectangle.
    #[must_use]
    pub fn label_origin(&self, metrics: &impl TextMetrics) -> Point {
        let height = metrics.line_height(Font::Geneva, LABEL_SIZE);
        Point::new(
            self.box_bounds().max.x + LABEL_GAP,
            self.rect.center().y - height / 2.0,
        )
    }

    /// Whether a click at `at` would reach it: on its rectangle, edges
    /// included, and enabled.
    #[must_use]
    pub fn contains(&self, at: Point) -> bool {
        self.enabled && self.rect.contains(at)
    }

    /// Whether a press is held on it with the pointer still on it.
    #[must_use]
    pub fn pressed(&self) -> bool {
        self.tracker.pressed()
    }

    /// Handles one input event; `true` when it flips.
    ///
    /// - A click (a press and a release on it, tracked as a button tracks
    ///   it) flips it; a press let go of elsewhere does not.
    /// - A Space press flips it; its repeats and release do not. The owner
    ///   sends keys only while the toggle has the focus.
    ///
    /// A greyed toggle ignores everything.
    pub fn input(&mut self, input: &Input) -> bool {
        let flips = match *input {
            Input::Key {
                key: Key::Space,
                pressed: true,
                repeat: false,
            } => self.enabled,
            Input::Key { .. } => false,
            // A check box clicks silently: the tracker's sounds are not
            // taken.
            Input::PointerButton { .. } | Input::PointerMoved(_) => {
                self.tracker.track(self.rect, self.enabled, input)
            }
        };
        if flips {
            self.on = !self.on;
        }
        flips
    }

    /// Abandons a click in progress without flipping.
    pub fn cancel_pointer(&mut self) {
        self.tracker.cancel_pointer();
    }

    /// Draws the box (crossed while on), the label in Geneva and, when
    /// `focused`, an outline round the rectangle; dim when greyed.
    pub fn draw(&self, focused: bool, metrics: &impl TextMetrics, list: &mut DrawList) {
        let color = if self.enabled {
            ENABLED_COLOR
        } else {
            GREYED_COLOR
        };
        let checkbox = self.box_bounds();
        outline(list, checkbox, color);
        if self.on {
            crossed_box(list, checkbox.center(), BOX_SIZE - 4.0, color);
        }
        list.text_in(
            Font::Geneva,
            self.label.as_str(),
            self.label_origin(metrics),
            LABEL_SIZE,
            None,
            color,
        );
        if focused {
            outline(list, self.rect, color);
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::draw::DrawCommand;
    use crate::input::MouseButton;
    use crate::text::fixture::MonoMetrics;

    fn at(x: f32, y: f32) -> Point {
        Point::new(x, y)
    }

    /// Stock "new prefs dialog"'s Intro Music check box, 99 x 18, placed at
    /// (500, 300).
    fn rect() -> Bounds {
        Bounds::at(at(500.0, 300.0), 99.0, 18.0)
    }

    fn music(on: bool) -> Toggle {
        Toggle::new(rect(), "Music", on)
    }

    fn button(pressed: bool, at: Point) -> Input {
        Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at,
        }
    }

    fn key(key: Key, pressed: bool, repeat: bool) -> Input {
        Input::Key {
            key,
            pressed,
            repeat,
        }
    }

    fn click(toggle: &mut Toggle, at: Point) -> bool {
        assert!(!toggle.input(&button(true, at)), "a press never flips");
        toggle.input(&button(false, at))
    }

    fn drawn(toggle: &Toggle, focused: bool) -> Vec<DrawCommand> {
        let mut list = DrawList::new();
        toggle.draw(focused, &MonoMetrics, &mut list);
        list.iter().cloned().collect()
    }

    fn line(from: Point, to: Point, width: f32, color: Color) -> DrawCommand {
        DrawCommand::Line {
            from,
            to,
            width,
            color,
        }
    }

    /// The four 1-unit edges of `rect`, clockwise from the top-left.
    fn edges(rect: Bounds, color: Color) -> Vec<DrawCommand> {
        let corners = [
            rect.min,
            at(rect.max.x, rect.min.y),
            rect.max,
            at(rect.min.x, rect.max.y),
        ];
        (0..4)
            .map(|n| line(corners[n], corners[(n + 1) % 4], 1.0, color))
            .collect()
    }

    #[test]
    fn the_box_sits_at_the_left_centred_down_and_the_label_follows_it() {
        let toggle = music(false);
        assert_eq!(
            toggle.box_bounds(),
            Bounds::at(at(500.0, 303.0), BOX_SIZE, BOX_SIZE)
        );
        // Geneva 12 is 14.4 high in the mock: centred down the 18 rows.
        assert_eq!(
            toggle.label_origin(&MonoMetrics),
            at(500.0 + BOX_SIZE + LABEL_GAP, 309.0 - 7.2)
        );
        assert_eq!((BOX_SIZE, LABEL_GAP, LABEL_SIZE), (12.0, 4.0, 12.0));
        assert_eq!((toggle.rect(), toggle.label()), (rect(), "Music"));
    }

    #[test]
    fn its_whole_rectangle_takes_clicks_unless_it_is_greyed() {
        let toggle = music(false);
        for inside in [rect().min, rect().max, rect().center(), at(598.0, 301.0)] {
            assert!(toggle.contains(inside), "{inside:?}");
        }
        for outside in [
            at(499.0, 305.0),
            at(600.0, 305.0),
            at(550.0, 299.0),
            at(550.0, 319.0),
        ] {
            assert!(!toggle.contains(outside), "{outside:?}");
        }
        let greyed = Toggle::greyed(rect(), "Smoke Trails");
        assert!(!greyed.contains(rect().center()));
        assert!(!greyed.enabled() && !greyed.on());
        assert!(toggle.enabled());
    }

    #[test]
    fn a_click_flips_it() {
        let mut toggle = music(false);
        assert!(click(&mut toggle, rect().center()));
        assert!(toggle.on());
        assert!(click(&mut toggle, rect().min));
        assert!(!toggle.on());
    }

    #[test]
    fn a_press_let_go_of_elsewhere_does_not_flip_it() {
        let mut toggle = music(true);
        toggle.input(&button(true, rect().center()));
        assert!(toggle.pressed());
        toggle.input(&Input::PointerMoved(at(10.0, 10.0)));
        assert!(!toggle.pressed(), "off it");
        assert!(!toggle.input(&button(false, at(10.0, 10.0))));
        assert!(toggle.on());
        // A press elsewhere let go of on it does not either.
        toggle.input(&button(true, at(10.0, 10.0)));
        assert!(!toggle.input(&button(false, rect().center())));
        assert!(toggle.on());
    }

    #[test]
    fn a_cancelled_press_does_not_flip_it() {
        let mut toggle = music(true);
        toggle.input(&button(true, rect().center()));
        toggle.cancel_pointer();
        assert!(!toggle.pressed());
        assert!(!toggle.input(&button(false, rect().center())));
        assert!(toggle.on());
    }

    #[test]
    fn space_flips_it_but_its_repeats_and_release_do_not() {
        let mut toggle = music(false);
        assert!(toggle.input(&key(Key::Space, true, false)));
        assert!(toggle.on());
        assert!(!toggle.input(&key(Key::Space, true, true)));
        assert!(!toggle.input(&key(Key::Space, false, false)));
        assert!(toggle.on());
        assert!(toggle.input(&key(Key::Space, true, false)));
        assert!(!toggle.on());
        for other in [Key::Enter, Key::Up, Key::Char(' ')] {
            assert!(!toggle.input(&key(other, true, false)), "{other:?}");
        }
        assert!(!toggle.on());
    }

    #[test]
    fn a_greyed_toggle_ignores_clicks_and_keys() {
        let mut toggle = Toggle::greyed(rect(), "Smoke Trails");
        assert!(!click(&mut toggle, rect().center()));
        assert!(!toggle.input(&key(Key::Space, true, false)));
        assert!(!toggle.on());
    }

    /// The label, at its origin in `color`.
    fn label(toggle: &Toggle, color: Color) -> DrawCommand {
        DrawCommand::Text {
            text: toggle.label().to_owned(),
            font: Font::Geneva,
            origin: toggle.label_origin(&MonoMetrics),
            size: LABEL_SIZE,
            wrap_width: None,
            color,
        }
    }

    #[test]
    fn off_it_is_an_empty_box_and_its_label() {
        let toggle = music(false);
        let mut expected = edges(toggle.box_bounds(), ENABLED_COLOR);
        expected.push(label(&toggle, ENABLED_COLOR));
        assert_eq!(drawn(&toggle, false), expected);
    }

    #[test]
    fn on_the_box_is_crossed() {
        let toggle = music(true);
        let mut expected = edges(toggle.box_bounds(), ENABLED_COLOR);
        let mut cross = DrawList::new();
        crossed_box(
            &mut cross,
            toggle.box_bounds().center(),
            BOX_SIZE - 4.0,
            ENABLED_COLOR,
        );
        expected.extend(cross.iter().cloned());
        expected.push(label(&toggle, ENABLED_COLOR));
        assert_eq!(drawn(&toggle, false), expected);
    }

    #[test]
    fn greyed_it_is_drawn_dim() {
        let toggle = Toggle::greyed(rect(), "Smoke Trails");
        let mut expected = edges(toggle.box_bounds(), GREYED_COLOR);
        expected.push(label(&toggle, GREYED_COLOR));
        assert_eq!(drawn(&toggle, false), expected);
        assert_eq!((ENABLED_COLOR, GREYED_COLOR), (Color::WHITE, Color::DIM));
    }

    #[test]
    fn focused_its_rectangle_is_outlined() {
        let toggle = music(false);
        let mut expected = drawn(&toggle, false);
        expected.extend(edges(rect(), ENABLED_COLOR));
        assert_eq!(drawn(&toggle, true), expected);
    }
}
