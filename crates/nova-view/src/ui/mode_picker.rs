//! A display mode picker: a row [`Picker`] stepping through "Borderless"
//! and then a monitor's exclusive modes.

use crate::draw::DrawList;
use crate::geometry::{Bounds, Point};
use crate::input::Input;
use crate::video_mode::{FullscreenKind, VideoMode};

use super::picker::Picker;
pub use super::picker::{ARROW_HEIGHT, ARROW_INSET, ARROW_WIDTH, LABEL_INSET, PickerRects};
use super::volume::Arrow;

/// A choice of how the game runs fullscreen: "Borderless" first, then
/// each exclusive mode offered, stepped by its arrows or the Up and Down
/// keys without wrapping.
#[derive(Clone, Debug, PartialEq)]
pub struct ModePicker(Picker<FullscreenKind>);

impl ModePicker {
    /// A picker in `row` offering only "Borderless", until its entries are
    /// set.
    #[must_use]
    pub fn new(row: Bounds) -> Self {
        let borderless = FullscreenKind::Borderless;
        Self(Picker::new(row, vec![borderless], borderless))
    }

    /// Offers "Borderless" and then `modes`, selecting `current`, or
    /// "Borderless" when `current` is a mode not among them.
    pub fn set_entries(&mut self, modes: &[VideoMode], current: FullscreenKind) {
        let entries = std::iter::once(FullscreenKind::Borderless)
            .chain(modes.iter().copied().map(FullscreenKind::Exclusive))
            .collect();
        self.0.set_entries(entries, current);
    }

    /// Where its parts are.
    #[must_use]
    pub fn rects(&self) -> PickerRects {
        self.0.rects()
    }

    /// What it offers, in order.
    #[must_use]
    pub fn entries(&self) -> &[FullscreenKind] {
        self.0.entries()
    }

    /// The entry chosen.
    #[must_use]
    pub fn selected(&self) -> FullscreenKind {
        self.0.selected()
    }

    /// The arrow at `at`, if any: only the arrows take clicks.
    #[must_use]
    pub fn arrow_at(&self, at: Point) -> Option<Arrow> {
        self.0.arrow_at(at)
    }

    /// Whether `arrow` shows pressed: a press held on it, with the pointer
    /// still on it.
    #[must_use]
    pub fn pressed(&self, arrow: Arrow) -> bool {
        self.0.pressed(arrow)
    }

    /// Handles one input event; `true` when the selection changes
    /// ([`Picker::input`]).
    pub fn input(&mut self, input: &Input) -> bool {
        self.0.input(input)
    }

    /// Abandons a click in progress without stepping.
    pub fn cancel_pointer(&mut self) {
        self.0.cancel_pointer();
    }

    /// Draws the chosen entry's label, the two arrows and, when
    /// `focused`, an outline round them ([`Picker::draw`]).
    pub fn draw(&self, focused: bool, list: &mut DrawList) {
        self.0.draw(focused, list);
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::draw::DrawCommand;
    use crate::font::Font;
    use crate::input::{Key, MouseButton};
    use crate::ui::dialog::outline;
    use crate::ui::volume::{COLOR, TEXT_SIZE, draw_arrow};

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

    /// "Run in a window"'s row, running on to the dialog's right margin.
    fn row() -> Bounds {
        ltrb(171.0, 77.0, 325.0, 95.0)
    }

    fn mode(width: u32, height: u32, hz: u32) -> VideoMode {
        VideoMode {
            width,
            height,
            refresh_millihertz: hz * 1000,
        }
    }

    fn modes() -> [VideoMode; 3] {
        [
            mode(2560, 1440, 60),
            mode(1920, 1080, 60),
            mode(1024, 768, 60),
        ]
    }

    fn exclusive(mode: VideoMode) -> FullscreenKind {
        FullscreenKind::Exclusive(mode)
    }

    /// A picker offering [`modes`], on `current`.
    fn picker(current: FullscreenKind) -> ModePicker {
        let mut picker = ModePicker::new(row());
        picker.set_entries(&modes(), current);
        picker
    }

    fn button(pressed: bool, at: Point) -> Input {
        Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at,
        }
    }

    fn click(picker: &mut ModePicker, at: Point) -> bool {
        assert!(!picker.input(&button(true, at)), "a press never steps");
        picker.input(&button(false, at))
    }

    fn key(key: Key, repeat: bool) -> Input {
        Input::Key {
            key,
            pressed: true,
            repeat,
        }
    }

    fn drawn(picker: &ModePicker, focused: bool) -> Vec<DrawCommand> {
        let mut list = DrawList::new();
        picker.draw(focused, &mut list);
        list.iter().cloned().collect()
    }

    fn arrow(rect: Bounds, arrow: Arrow, filled: bool) -> Vec<DrawCommand> {
        let mut list = DrawList::new();
        draw_arrow(&mut list, rect, arrow, filled, COLOR);
        list.iter().cloned().collect()
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
    fn its_arrows_stack_at_the_rows_left_and_the_label_runs_to_its_right() {
        let rects = PickerRects::in_row(row());
        assert_eq!(rects.up, ltrb(172.0, 77.0, 183.0, 86.0));
        assert_eq!(rects.down, ltrb(172.0, 86.0, 183.0, 95.0));
        assert_eq!(rects.value, ltrb(189.0, 77.0, 325.0, 95.0));
        assert_eq!(rects.bounds(), ltrb(172.0, 77.0, 325.0, 95.0));
        assert_eq!(ModePicker::new(row()).rects(), rects);
    }

    #[test]
    fn a_new_picker_offers_only_borderless() {
        let picker = ModePicker::new(row());
        assert_eq!(picker.entries(), [FullscreenKind::Borderless]);
        assert_eq!(picker.selected(), FullscreenKind::Borderless);
    }

    #[test]
    fn setting_the_entries_offers_borderless_then_the_modes_and_selects_the_current() {
        let [large, medium, small] = modes();
        let picker = picker(exclusive(medium));
        assert_eq!(
            picker.entries(),
            [
                FullscreenKind::Borderless,
                exclusive(large),
                exclusive(medium),
                exclusive(small),
            ]
        );
        assert_eq!(picker.selected(), exclusive(medium));
        assert_eq!(
            super::tests::picker(FullscreenKind::Borderless).selected(),
            FullscreenKind::Borderless
        );
    }

    #[test]
    fn a_current_mode_not_offered_selects_borderless() {
        let gone = exclusive(mode(3840, 2160, 60));
        let mut picker = picker(exclusive(modes()[2]));
        picker.set_entries(&modes(), gone);
        assert_eq!(picker.selected(), FullscreenKind::Borderless);
        picker.set_entries(&[], exclusive(modes()[0]));
        assert_eq!(picker.entries(), [FullscreenKind::Borderless]);
        assert_eq!(picker.selected(), FullscreenKind::Borderless);
    }

    #[test]
    fn the_arrows_step_through_the_list_without_wrapping() {
        let [large, medium, small] = modes();
        let mut picker = picker(FullscreenKind::Borderless);
        let rects = picker.rects();
        assert!(!click(&mut picker, rects.up.center()), "at the start");
        assert_eq!(picker.selected(), FullscreenKind::Borderless);
        assert!(click(&mut picker, rects.down.center()));
        assert_eq!(picker.selected(), exclusive(large));
        assert!(click(&mut picker, rects.down.center()));
        assert!(click(&mut picker, rects.down.center()));
        assert_eq!(picker.selected(), exclusive(small));
        assert!(!click(&mut picker, rects.down.center()), "at the end");
        assert_eq!(picker.selected(), exclusive(small));
        assert!(click(&mut picker, rects.up.center()));
        assert_eq!(picker.selected(), exclusive(medium));
    }

    #[test]
    fn up_and_down_step_it_repeats_included() {
        let [large, medium, _] = modes();
        let mut picker = picker(FullscreenKind::Borderless);
        assert!(!picker.input(&key(Key::Up, false)));
        assert!(picker.input(&key(Key::Down, false)));
        assert!(picker.input(&key(Key::Down, true)));
        assert_eq!(picker.selected(), exclusive(medium));
        assert!(picker.input(&key(Key::Up, true)));
        assert_eq!(picker.selected(), exclusive(large));
        let release = Input::Key {
            key: Key::Down,
            pressed: false,
            repeat: false,
        };
        for other in [
            release,
            key(Key::Space, false),
            key(Key::Left, false),
            key(Key::Right, false),
            key(Key::Enter, false),
            Input::Text('d'),
            Input::PointerLeft,
        ] {
            assert!(!picker.input(&other), "{other:?}");
        }
        assert_eq!(picker.selected(), exclusive(large));
    }

    #[test]
    fn only_the_arrows_take_clicks() {
        let mut picker = picker(FullscreenKind::Borderless);
        let rects = picker.rects();
        assert_eq!(picker.arrow_at(rects.up.center()), Some(Arrow::Up));
        assert_eq!(picker.arrow_at(rects.down.center()), Some(Arrow::Down));
        assert_eq!(picker.arrow_at(rects.value.center()), None);
        assert!(!click(&mut picker, rects.value.center()));
        assert_eq!(picker.selected(), FullscreenKind::Borderless);
    }

    #[test]
    fn a_press_let_go_of_off_its_arrow_does_not_step() {
        let mut picker = picker(FullscreenKind::Borderless);
        let rects = picker.rects();
        picker.input(&button(true, rects.down.center()));
        assert!(picker.pressed(Arrow::Down));
        assert!(!picker.pressed(Arrow::Up));
        picker.input(&Input::PointerMoved(rects.value.center()));
        assert!(!picker.pressed(Arrow::Down), "off it");
        assert!(!picker.input(&button(false, rects.value.center())));
        assert_eq!(picker.selected(), FullscreenKind::Borderless);
        picker.input(&button(true, rects.down.center()));
        picker.cancel_pointer();
        assert!(!picker.pressed(Arrow::Down));
        assert!(!picker.input(&button(false, rects.down.center())));
        assert_eq!(picker.selected(), FullscreenKind::Borderless);
    }

    #[test]
    fn it_draws_the_chosen_label_and_both_arrows() {
        let mut picker = picker(exclusive(modes()[1]));
        let rects = picker.rects();
        let mut expected = vec![text("1920 x 1080, 60 Hz", rects.value.min)];
        expected.extend(arrow(rects.up, Arrow::Up, false));
        expected.extend(arrow(rects.down, Arrow::Down, false));
        assert_eq!(drawn(&picker, false), expected);
        picker.input(&button(true, rects.up.center()));
        let mut pressed = vec![text("1920 x 1080, 60 Hz", rects.value.min)];
        pressed.extend(arrow(rects.up, Arrow::Up, true));
        pressed.extend(arrow(rects.down, Arrow::Down, false));
        assert_eq!(drawn(&picker, false), pressed);
        let borderless = ModePicker::new(row());
        assert_eq!(
            drawn(&borderless, false)[0],
            text("Borderless", rects.value.min)
        );
    }

    #[test]
    fn focused_it_is_outlined() {
        let picker = picker(FullscreenKind::Borderless);
        let mut expected = drawn(&picker, false);
        let mut ring = DrawList::new();
        outline(&mut ring, picker.rects().bounds(), COLOR);
        expected.extend(ring.iter().cloned());
        assert_eq!(drawn(&picker, true), expected);
    }
}
