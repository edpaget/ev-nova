//! A volume control: a label, the level as a number and two arrows that
//! step it, as "new prefs dialog" lays its Sound Volume out.
//!
//! The arrows are the interface file's 11 x 9 `PICT`s, drawn stretched
//! over their items as the Dialog Manager draws a picture item: 134 ("Up
//! arrow", `DITL` 4003 item 7) and 135 ("Down Arrow", item 6) at rest.
//! While an arrow is held it draws 136 or 137 instead, the same box filled
//! with the arrow cut out of it. The Mac OS X build never draws any of
//! these: its Preferences is the Cocoa `EVPrefsDialogController`, whose
//! volume is an `NSStepper`, and none of its `GetPicture` calls names
//! 134-137. So the arrows come from the classic dialog's resources alone,
//! and 136 and 137 being the pressed looks is a reading of their pixels.
//! No sound comes with a press: the Dialog Manager plays none for a
//! picture item.

use crate::color::Color;
use crate::draw::DrawList;
use crate::font::Font;
use crate::geometry::{Bounds, Point};
use crate::image::ImageKey;
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

/// The up arrow at rest: `PICT` 134 "Up arrow", `DITL` 4003 item 7.
pub const UP_ARROW: i16 = 134;
/// The down arrow at rest: `PICT` 135 "Down Arrow", `DITL` 4003 item 6.
pub const DOWN_ARROW: i16 = 135;
/// The up arrow held: `PICT` 136, 134 pressed.
pub const UP_ARROW_PRESSED: i16 = 136;
/// The down arrow held: `PICT` 137, 135 pressed.
pub const DOWN_ARROW_PRESSED: i16 = 137;

/// One of the two arrows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Arrow {
    /// Raises the level.
    Up,
    /// Lowers the level.
    Down,
}

impl Arrow {
    /// The interface-file picture this arrow draws, at rest or `pressed`.
    #[must_use]
    pub const fn picture(self, pressed: bool) -> ImageKey {
        ImageKey::interface_picture(match (self, pressed) {
            (Self::Up, false) => UP_ARROW,
            (Self::Down, false) => DOWN_ARROW,
            (Self::Up, true) => UP_ARROW_PRESSED,
            (Self::Down, true) => DOWN_ARROW_PRESSED,
        })
    }
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
    /// corners, then the two arrows (each its pressed picture while
    /// pressed), and, when `focused`, an outline round every part.
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
            draw_arrow(list, self.arrow_rect(arrow), arrow, self.pressed(arrow));
        }
        if focused {
            outline(list, rects.bounds(), COLOR);
        }
    }
}

/// Draws `arrow`'s picture ([`Arrow::picture`]), its pressed one when
/// `pressed`, stretched over `rect` (11 x 9 for the stock arrows).
pub fn draw_arrow(list: &mut DrawList, rect: Bounds, arrow: Arrow, pressed: bool) {
    list.stretched_picture(
        arrow.picture(pressed),
        rect.min,
        rect.width(),
        rect.height(),
    );
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::draw::DrawCommand;
    use crate::image::ImageKey;
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

    fn arrow(rect: Bounds, arrow: Arrow, pressed: bool) -> Vec<DrawCommand> {
        let mut list = DrawList::new();
        draw_arrow(&mut list, rect, arrow, pressed);
        list.iter().cloned().collect()
    }

    /// The interface file's `PICT` `id` stretched over `rect`.
    fn picture(id: i16, rect: Bounds) -> DrawCommand {
        DrawCommand::StretchedPicture {
            image: ImageKey::interface_picture(id),
            top_left: rect.min,
            width: rect.width(),
            height: rect.height(),
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
        assert_eq!(control(9).level(), MAX_LEVEL);
        assert_eq!(control(u8::MAX).level(), MAX_LEVEL);
        assert_eq!(control(8).level(), 8);
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
        let mut control = control(7);
        assert!(click(&mut control, stock().up.center()));
        assert_eq!(control.level(), 8);
        assert!(!click(&mut control, stock().up.center()), "the loudest");
        assert_eq!(control.level(), 8);
        assert!(click(&mut control, stock().down.center()));
        assert_eq!(control.level(), 7);
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
        let mut control = super::tests::control(8);
        assert!(!control.input(&key(Key::Up, false)));
        let mut control = super::tests::control(0);
        assert!(!control.input(&key(Key::Left, false)));
        assert_eq!(control.level(), 0);
    }

    #[test]
    fn the_arrows_are_the_interface_files_pictures_134_to_137() {
        assert_eq!(Arrow::Up.picture(false), ImageKey::interface_picture(134));
        assert_eq!(Arrow::Down.picture(false), ImageKey::interface_picture(135));
        assert_eq!(Arrow::Up.picture(true), ImageKey::interface_picture(136));
        assert_eq!(Arrow::Down.picture(true), ImageKey::interface_picture(137));
    }

    #[test]
    fn an_arrow_is_its_picture_stretched_over_its_rect() {
        assert_eq!(
            arrow(stock().up, Arrow::Up, false),
            [picture(134, stock().up)]
        );
        assert_eq!(
            arrow(stock().down, Arrow::Down, true),
            [picture(137, stock().down)]
        );
        let wide = ltrb(10.0, 20.0, 40.0, 25.0);
        assert_eq!(arrow(wide, Arrow::Down, false), [picture(135, wide)]);
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
        expected.extend([picture(134, stock().up), picture(135, stock().down)]);
        assert_eq!(drawn(&control, false), expected);
        assert_eq!((TEXT_SIZE, COLOR), (12.0, Color::WHITE));
    }

    #[test]
    fn a_held_arrow_draws_its_pressed_picture() {
        let mut control = control(5);
        control.input(&button(true, stock().down.center()));
        let mut expected = vec![
            text("Sound Volume:", stock().label.min),
            text("5", stock().value.min),
        ];
        expected.extend([picture(134, stock().up), picture(137, stock().down)]);
        assert_eq!(drawn(&control, false), expected);
        control.input(&Input::PointerMoved(stock().up.center()));
        assert_eq!(
            drawn(&control, false)[3],
            picture(135, stock().down),
            "off the arrow, it is at rest"
        );
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
