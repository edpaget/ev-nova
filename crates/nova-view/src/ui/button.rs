//! Push buttons drawn with Nova's own button graphics, at any size, and
//! pointer tracking that follows the Mac's `TrackControl`.

use crate::color::Color;
use crate::draw::{DrawList, fill_rect};
use crate::font::Font;
use crate::geometry::{Bounds, Point};
use crate::image::ImageKey;
use crate::input::{Input, MouseButton};
use crate::sound::UiSound;
use crate::text::TextMetrics;

use super::button_glyph;
use super::slice::{Axis, Piece, three_slice};

/// One state's three pictures: left cap, middle and right cap.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ButtonImages {
    /// The left cap.
    pub left: ImageKey,
    /// The middle, stretched to fill the width.
    pub middle: ImageKey,
    /// The right cap.
    pub right: ImageKey,
}

impl ButtonImages {
    /// Stock Nova's pictures from `left`: the left cap `left` and the right
    /// cap `left + 2`, each masked by the picture 100 above it, and the
    /// middle `left + 1`, unmasked.
    const fn nova(left: i16) -> Self {
        Self {
            left: ImageKey::masked_picture(left, left + 100),
            middle: ImageKey::picture(left + 1),
            right: ImageKey::masked_picture(left + 2, left + 102),
        }
    }

    fn piece(&self, piece: Piece) -> ImageKey {
        match piece {
            Piece::Start => self.left,
            Piece::Middle => self.middle,
            Piece::End => self.right,
        }
    }
}

/// The pictures a button is drawn with in each state, and how wide its
/// caps are.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ButtonSkin {
    /// Enabled and not pressed.
    pub normal: ButtonImages,
    /// Pressed.
    pub pressed: ButtonImages,
    /// Disabled.
    pub disabled: ButtonImages,
    /// The caps' width, in logical units.
    pub cap: f32,
}

impl ButtonSkin {
    /// Stock Nova's button graphics, in the game data: `PICT`s 7500-7502
    /// ("Button ... Bright"), 7503-7505 (pressed, "click") and 7506-7508
    /// (disabled, "grey"), each a left cap, a middle and a right cap. The
    /// caps (13 x 25) are masked by the `PICT` 100 above them (7600, ...);
    /// the middles (2 x 25) have no mask.
    pub const NOVA: Self = Self {
        normal: ButtonImages::nova(7500),
        pressed: ButtonImages::nova(7503),
        disabled: ButtonImages::nova(7506),
        cap: 13.0,
    };
}

/// How button labels are set: their font, size and colour in each state,
/// from the game's `cölr`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ButtonStyle {
    /// The label's font (`cölr` `ButtonFont`).
    pub font: Font,
    /// The label's size (`cölr` `ButtonFontSz`).
    pub size: f32,
    /// The label's colour when enabled and not pressed (`ButtonUp`).
    pub up: Color,
    /// The label's colour when pressed (`ButtonDown`).
    pub down: Color,
    /// The label's colour when disabled (`ButtonGrey`).
    pub grey: Color,
}

impl ButtonStyle {
    /// Stock `cölr` 128: Charcoal 12, white, mid grey when pressed and
    /// near-black when disabled.
    pub const STOCK: Self = Self {
        font: Font::Charcoal,
        size: 12.0,
        up: Color::from_rgb24(0x00FF_FFFF),
        down: Color::from_rgb24(0x0080_8080),
        grey: Color::from_rgb24(0x0026_2626),
    };
}

/// A push button: where it is, what it says and whether it can be pressed.
#[derive(Clone, Debug, PartialEq)]
pub struct Button {
    /// Where it is, in logical units.
    pub rect: Bounds,
    /// Its label, centred on it.
    pub label: String,
    /// Whether it can be pressed.
    pub enabled: bool,
}

impl Button {
    /// An enabled button.
    #[must_use]
    pub fn new(rect: Bounds, label: impl Into<String>) -> Self {
        Self {
            rect,
            label: label.into(),
            enabled: true,
        }
    }

    /// Whether `point` is on the button, as `PtInRect` counts it: the
    /// left and top edges in, the right and bottom edges out.
    #[must_use]
    pub fn contains(&self, point: Point) -> bool {
        self.rect.contains_half_open(point)
    }

    /// Draws the button into `list` as `_DrawButton` @0x25ea8 does: its
    /// pictures for its state (disabled, else `pressed` or normal), then
    /// its label in the state's colour. A rectangle with no drawable size
    /// draws nothing.
    ///
    /// The caps keep their width and take the rectangle's height, with the
    /// middle stretched between them, so any width and height work. A
    /// rectangle narrower than two caps has no middle, and its right cap
    /// overlaps its left (@0x26042-0x26215).
    ///
    /// A label starting with `^`, `&`, `+` or `-` draws its pen glyph (see
    /// `button_glyph`). Any other is centred text.
    ///
    /// Two of `_DrawButton`'s states aren't here. State −2 draws no face
    /// and no label, so a caller that wants it draws no button. State −3,
    /// a picture in place of a button (@0x25ed7-0x25f61), has no constant
    /// caller, and nothing on the engine's paths reaches it.
    pub fn draw(
        &self,
        pressed: bool,
        skin: &ButtonSkin,
        style: &ButtonStyle,
        metrics: &impl TextMetrics,
        list: &mut DrawList,
    ) {
        let (width, height) = (self.rect.width(), self.rect.height());
        let drawable = |side: f32| side.is_finite() && side > 0.0;
        if !(drawable(width) && drawable(height)) {
            return;
        }
        let (images, color) = if !self.enabled {
            (&skin.disabled, style.grey)
        } else if pressed {
            (&skin.pressed, style.down)
        } else {
            (&skin.normal, style.up)
        };
        if width < 2.0 * skin.cap {
            let (min, max) = (self.rect.min, self.rect.max);
            list.stretched_picture(images.left, min, skin.cap, height);
            let right = Point::new(max.x - skin.cap, min.y);
            list.stretched_picture(images.right, right, skin.cap, height);
        } else {
            for (piece, at) in three_slice(self.rect, Axis::Horizontal, skin.cap, skin.cap) {
                list.stretched_picture(images.piece(piece), at.min, at.width(), at.height());
            }
        }
        if let Some(strokes) = button_glyph::strokes(&self.label, width, height) {
            for stroke in strokes {
                fill_rect(list, stroke.offset(self.rect.min), color);
            }
            return;
        }
        if self.label.is_empty() {
            return;
        }
        let width = metrics.width(style.font, style.size, &self.label);
        let height = metrics.line_height(style.font, style.size);
        let center = self.rect.center();
        let origin = Point::new(center.x - width / 2.0, center.y - height / 2.0);
        list.text_in(style.font, &self.label, origin, style.size, None, color);
    }
}

/// Tracks the pointer over one button, as the Mac's `TrackControl` does:
/// pressing the primary button on an enabled button arms it; it shows
/// pressed only while the pointer stays on it; releasing on it activates
/// it, and releasing anywhere else does not.
///
/// The press, the pointer while held and the release are hit-tested as
/// QuickDraw's `PtInRect` does, as every `_Track*Buttons` routine of the
/// original does: the left and top edges are on the button, the right and
/// bottom edges are off it ([`Bounds::contains_half_open`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ButtonTracker {
    armed: bool,
    inside: bool,
    /// The sound the last input made, until it is taken.
    sound: Option<UiSound>,
}

impl ButtonTracker {
    /// Handles `input` for `button`; `true` when it activates the button.
    pub fn input(&mut self, button: &Button, input: &Input) -> bool {
        self.track(button.rect, button.enabled, input)
    }

    /// [`Self::input`] for any enabled or disabled area.
    pub(crate) fn track(&mut self, area: Bounds, enabled: bool, input: &Input) -> bool {
        match *input {
            Input::PointerButton {
                button: MouseButton::Left,
                pressed: true,
                at,
            } => {
                self.armed = enabled && area.contains_half_open(at);
                self.inside = self.armed;
                if self.armed {
                    self.sound = Some(UiSound::ButtonDown);
                }
                false
            }
            // Only matters while armed: a press sets it afresh.
            Input::PointerMoved(at) => {
                self.inside = area.contains_half_open(at);
                false
            }
            Input::PointerButton {
                button: MouseButton::Left,
                pressed: false,
                at,
            } if self.armed => {
                self.cancel_pointer();
                self.sound = Some(UiSound::ButtonUp);
                area.contains_half_open(at)
            }
            _ => false,
        }
    }

    /// Whether a press is being held on the button.
    #[must_use]
    pub fn armed(&self) -> bool {
        self.armed
    }

    /// Whether the button shows pressed: armed with the pointer on it.
    #[must_use]
    pub fn pressed(&self) -> bool {
        self.armed && self.inside
    }

    /// Abandons the press without activating the button, silently: a
    /// sound already made is kept until it is taken.
    pub fn cancel_pointer(&mut self) {
        self.armed = false;
        self.inside = false;
    }

    /// The sound the last input made, once: [`UiSound::ButtonDown`] for a
    /// press that armed the button, [`UiSound::ButtonUp`] for the release
    /// of an armed press, on the button or off it. Only the main menu
    /// reports them (`_TrackMainScreenButtons` @0x195b9); every other
    /// reader drops them, as the original's other trackers play nothing.
    pub fn take_sound(&mut self) -> Option<UiSound> {
        self.sound.take()
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::draw::DrawCommand;
    use crate::text::fixture::MonoMetrics;

    fn at(x: f32, y: f32) -> Point {
        Point::new(x, y)
    }

    /// A 99 x 25 button at (173, 281), like stock "Desc Dialog"'s.
    fn done() -> Button {
        Button::new(Bounds::at(at(173.0, 281.0), 99.0, 25.0), "Done")
    }

    fn drawn(button: &Button, pressed: bool) -> Vec<DrawCommand> {
        let mut list = DrawList::new();
        button.draw(
            pressed,
            &ButtonSkin::NOVA,
            &ButtonStyle::STOCK,
            &MonoMetrics,
            &mut list,
        );
        list.iter().cloned().collect()
    }

    fn picture(image: ImageKey, x: f32, y: f32, width: f32, height: f32) -> DrawCommand {
        DrawCommand::StretchedPicture {
            image,
            top_left: at(x, y),
            width,
            height,
        }
    }

    #[test]
    fn the_stock_skin_names_nova_s_button_pictures() {
        let skin = ButtonSkin::NOVA;
        assert_eq!(
            skin.normal,
            ButtonImages {
                left: ImageKey::masked_picture(7500, 7600),
                middle: ImageKey::picture(7501),
                right: ImageKey::masked_picture(7502, 7602),
            }
        );
        assert_eq!(
            skin.pressed,
            ButtonImages {
                left: ImageKey::masked_picture(7503, 7603),
                middle: ImageKey::picture(7504),
                right: ImageKey::masked_picture(7505, 7605),
            }
        );
        assert_eq!(
            skin.disabled,
            ButtonImages {
                left: ImageKey::masked_picture(7506, 7606),
                middle: ImageKey::picture(7507),
                right: ImageKey::masked_picture(7508, 7608),
            }
        );
        assert_eq!(skin.cap, 13.0);
    }

    #[test]
    fn the_stock_style_is_stock_colr_128() {
        assert_eq!(
            ButtonStyle::STOCK,
            ButtonStyle {
                font: Font::Charcoal,
                size: 12.0,
                up: Color::rgba(255, 255, 255, 255),
                down: Color::rgba(128, 128, 128, 255),
                grey: Color::rgba(38, 38, 38, 255),
            }
        );
    }

    #[test]
    fn a_button_is_its_three_pictures_then_its_centred_label() {
        // "Done" is 4 x 7.2 = 28.8 wide in Charcoal 12 and 14.4 high.
        let label = DrawCommand::Text {
            text: "Done".to_owned(),
            font: Font::Charcoal,
            origin: at(222.5 - 14.4, 293.5 - 7.2),
            size: 12.0,
            wrap_width: None,
            color: Color::WHITE,
        };
        let normal = ButtonSkin::NOVA.normal;
        assert_eq!(
            drawn(&done(), false),
            [
                picture(normal.left, 173.0, 281.0, 13.0, 25.0),
                picture(normal.middle, 186.0, 281.0, 73.0, 25.0),
                picture(normal.right, 259.0, 281.0, 13.0, 25.0),
                label,
            ]
        );
    }

    #[test]
    fn a_pressed_button_uses_the_pressed_pictures_and_colour() {
        let pressed = ButtonSkin::NOVA.pressed;
        let commands = drawn(&done(), true);
        assert_eq!(commands[0], picture(pressed.left, 173.0, 281.0, 13.0, 25.0));
        assert_eq!(
            commands[1],
            picture(pressed.middle, 186.0, 281.0, 73.0, 25.0)
        );
        assert_eq!(
            commands[2],
            picture(pressed.right, 259.0, 281.0, 13.0, 25.0)
        );
        assert!(
            matches!(commands[3], DrawCommand::Text { color, .. } if color == ButtonStyle::STOCK.down)
        );
    }

    #[test]
    fn a_disabled_button_is_grey_even_when_pressed() {
        let disabled = ButtonSkin::NOVA.disabled;
        let button = Button {
            enabled: false,
            ..done()
        };
        for pressed in [false, true] {
            let commands = drawn(&button, pressed);
            assert_eq!(
                commands[0],
                picture(disabled.left, 173.0, 281.0, 13.0, 25.0)
            );
            assert_eq!(
                commands[1],
                picture(disabled.middle, 186.0, 281.0, 73.0, 25.0)
            );
            assert_eq!(
                commands[2],
                picture(disabled.right, 259.0, 281.0, 13.0, 25.0)
            );
            assert!(
                matches!(commands[3], DrawCommand::Text { color, .. } if color == ButtonStyle::STOCK.grey)
            );
        }
    }

    /// `_DrawButton` keeps each cap's width and draws no middle
    /// (@0x26042-0x26215), so narrow caps overlap.
    #[test]
    fn a_narrow_buttons_caps_keep_their_width_and_overlap_and_any_height_works() {
        let button = Button::new(Bounds::at(at(0.0, 0.0), 20.0, 40.0), "");
        let normal = ButtonSkin::NOVA.normal;
        assert_eq!(
            drawn(&button, false),
            [
                picture(normal.left, 0.0, 0.0, 13.0, 40.0),
                picture(normal.right, 7.0, 0.0, 13.0, 40.0),
            ]
        );
        let arrow = Button::new(Bounds::at(at(330.0, 282.0), 23.0, 23.0), "");
        assert_eq!(
            drawn(&arrow, false),
            [
                picture(normal.left, 330.0, 282.0, 13.0, 23.0),
                picture(normal.right, 340.0, 282.0, 13.0, 23.0),
            ]
        );
        let exact = Button::new(Bounds::at(at(0.0, 0.0), 26.0, 25.0), "");
        assert_eq!(
            drawn(&exact, false),
            [
                picture(normal.left, 0.0, 0.0, 13.0, 25.0),
                picture(normal.right, 13.0, 0.0, 13.0, 25.0),
            ]
        );
    }

    #[test]
    fn a_button_with_no_drawable_size_draws_nothing() {
        for (width, height) in [
            (0.0, 25.0),
            (23.0, 0.0),
            (-5.0, 25.0),
            (23.0, f32::NAN),
            (f32::INFINITY, 25.0),
        ] {
            for label in ["", "^", "Okay"] {
                let rect = Bounds::at(at(10.0, 10.0), width, height);
                let button = Button::new(rect, label);
                assert_eq!(drawn(&button, false), [], "{width} x {height} {label:?}");
            }
        }
    }

    /// One of the 2 x 2 squares a pen glyph's diagonal paints, as
    /// [`fill_rect`] draws it.
    fn square(x: f32, y: f32, color: Color) -> DrawCommand {
        let mut list = DrawList::new();
        fill_rect(&mut list, Bounds::at(at(x, y), 2.0, 2.0), color);
        list.iter().next().expect("a fill").clone()
    }

    fn arrow(label: &str) -> Button {
        Button::new(Bounds::at(at(330.0, 282.0), 23.0, 23.0), label)
    }

    /// The caps, then the squares of `_DrawButton`'s `^` arm
    /// (@0x26239-0x262f4), in each state's colour and with no text.
    #[test]
    fn an_up_arrow_label_is_its_pen_glyph_in_the_states_colour() {
        let style = ButtonStyle::STOCK;
        let disabled = Button {
            enabled: false,
            ..arrow("^")
        };
        for (commands, images, color) in [
            (drawn(&arrow("^"), false), ButtonSkin::NOVA.normal, style.up),
            (
                drawn(&arrow("^"), true),
                ButtonSkin::NOVA.pressed,
                style.down,
            ),
            (
                drawn(&disabled, true),
                ButtonSkin::NOVA.disabled,
                style.grey,
            ),
        ] {
            let mut expected = vec![
                picture(images.left, 330.0, 282.0, 13.0, 23.0),
                picture(images.right, 340.0, 282.0, 13.0, 23.0),
                square(341.0, 291.0, color),
            ];
            for (x, y) in [(1.0, 1.0), (2.0, 2.0), (3.0, 3.0), (4.0, 4.0)] {
                expected.push(square(341.0 - x, 291.0 + y, color));
                expected.push(square(341.0 + x, 291.0 + y, color));
            }
            assert_eq!(commands, expected);
        }
    }

    /// `_DrawButton` picks its glyph on the label's first character.
    #[test]
    fn only_a_labels_first_character_picks_a_glyph() {
        let is_text = |label: &str| {
            drawn(&arrow(label), false)
                .iter()
                .any(|command| matches!(command, DrawCommand::Text { .. }))
        };
        for glyph in ["^", "&", "+", "-", "^ up", "-5"] {
            assert!(!is_text(glyph), "{glyph:?}");
        }
        for text in ["A^", "Okay", "a-b", " +"] {
            assert!(is_text(text), "{text:?}");
        }
    }

    #[test]
    fn the_label_follows_the_style() {
        let style = ButtonStyle {
            font: Font::Geneva,
            size: 20.0,
            ..ButtonStyle::STOCK
        };
        let mut list = DrawList::new();
        let button = Button::new(Bounds::at(at(0.0, 0.0), 100.0, 40.0), "OK");
        button.draw(false, &ButtonSkin::NOVA, &style, &MonoMetrics, &mut list);
        // "OK" is 2 x 10 = 20 wide in Geneva 20, and 24 high.
        assert_eq!(
            list.iter().last(),
            Some(&DrawCommand::Text {
                text: "OK".to_owned(),
                font: Font::Geneva,
                origin: at(40.0, 8.0),
                size: 20.0,
                wrap_width: None,
                color: Color::WHITE,
            })
        );
    }

    /// As `PtInRect` counts it (`_TrackPlayerInfoButtons` @0x27cea).
    #[test]
    fn contains_takes_the_left_and_top_edges_but_not_the_right_and_bottom() {
        let button = done();
        for inside in [at(173.0, 281.0), at(271.9, 305.9), at(200.0, 290.0)] {
            assert!(button.contains(inside), "{inside:?}");
        }
        for outside in [
            at(172.9, 290.0),
            at(272.0, 290.0),
            at(200.0, 306.0),
            at(173.0, 306.0),
            at(272.1, 290.0),
            at(200.0, 306.1),
        ] {
            assert!(!button.contains(outside), "{outside:?}");
        }
    }

    #[test]
    fn a_press_on_the_right_or_bottom_edge_does_not_arm() {
        let button = done();
        for edge in [at(272.0, 290.0), at(200.0, 306.0)] {
            let mut tracker = ButtonTracker::default();
            tracker.input(&button, &press(edge));
            assert!(!tracker.armed(), "{edge:?}");
            assert!(!tracker.input(&button, &release(edge)), "{edge:?}");
        }
        for near in [at(271.0, 290.0), at(200.0, 305.0)] {
            let mut tracker = ButtonTracker::default();
            tracker.input(&button, &press(near));
            assert!(tracker.armed(), "{near:?}");
            assert!(tracker.input(&button, &release(near)), "{near:?}");
        }
    }

    #[test]
    fn a_press_on_the_left_or_top_edge_arms() {
        let button = done();
        for edge in [at(173.0, 290.0), at(200.0, 281.0)] {
            let mut tracker = ButtonTracker::default();
            tracker.input(&button, &press(edge));
            assert!(tracker.armed(), "{edge:?}");
            assert!(tracker.input(&button, &release(edge)), "{edge:?}");
        }
    }

    #[test]
    fn a_move_onto_the_right_or_bottom_edge_is_off_the_button() {
        let (button, mut tracker) = (done(), ButtonTracker::default());
        tracker.input(&button, &press(INSIDE));
        tracker.input(&button, &Input::PointerMoved(at(272.0, 290.0)));
        assert!(!tracker.pressed(), "on the right edge");
        tracker.input(&button, &Input::PointerMoved(at(271.0, 290.0)));
        assert!(tracker.pressed(), "a pixel inside");
        tracker.input(&button, &Input::PointerMoved(at(200.0, 306.0)));
        assert!(!tracker.pressed(), "on the bottom edge");
        assert!(!tracker.input(&button, &release(at(200.0, 306.0))));
    }

    fn press(at: Point) -> Input {
        Input::PointerButton {
            button: MouseButton::Left,
            pressed: true,
            at,
        }
    }

    fn release(at: Point) -> Input {
        Input::PointerButton {
            button: MouseButton::Left,
            pressed: false,
            at,
        }
    }

    const INSIDE: Point = Point::new(200.0, 290.0);
    const OUTSIDE: Point = Point::new(100.0, 100.0);

    #[test]
    fn press_and_release_inside_activates() {
        let (button, mut tracker) = (done(), ButtonTracker::default());
        assert!(!tracker.input(&button, &press(INSIDE)));
        assert!(tracker.armed() && tracker.pressed());
        assert!(tracker.input(&button, &release(INSIDE)));
        assert!(!tracker.armed() && !tracker.pressed());
    }

    #[test]
    fn releasing_outside_does_not_activate() {
        let (button, mut tracker) = (done(), ButtonTracker::default());
        tracker.input(&button, &press(INSIDE));
        assert!(!tracker.input(&button, &release(OUTSIDE)));
        assert!(!tracker.armed());
        assert!(!tracker.input(&button, &release(INSIDE)), "disarmed");
    }

    #[test]
    fn dragging_out_and_back_in_re_highlights() {
        let (button, mut tracker) = (done(), ButtonTracker::default());
        tracker.input(&button, &press(INSIDE));
        assert!(!tracker.input(&button, &Input::PointerMoved(OUTSIDE)));
        assert!(tracker.armed() && !tracker.pressed());
        tracker.input(&button, &Input::PointerMoved(INSIDE));
        assert!(tracker.pressed());
        assert!(tracker.input(&button, &release(INSIDE)));
    }

    #[test]
    fn moving_without_a_press_does_nothing() {
        let (button, mut tracker) = (done(), ButtonTracker::default());
        tracker.input(&button, &Input::PointerMoved(INSIDE));
        assert!(!tracker.pressed() && !tracker.armed());
        assert!(!tracker.input(&button, &release(INSIDE)));
    }

    #[test]
    fn pressing_outside_does_not_arm() {
        let (button, mut tracker) = (done(), ButtonTracker::default());
        tracker.input(&button, &press(OUTSIDE));
        tracker.input(&button, &Input::PointerMoved(INSIDE));
        assert!(!tracker.armed() && !tracker.pressed());
        assert!(!tracker.input(&button, &release(INSIDE)));
    }

    #[test]
    fn a_disabled_button_ignores_the_pointer() {
        let button = Button {
            enabled: false,
            ..done()
        };
        let mut tracker = ButtonTracker::default();
        tracker.input(&button, &press(INSIDE));
        assert!(!tracker.armed() && !tracker.pressed());
        assert!(!tracker.input(&button, &release(INSIDE)));
    }

    #[test]
    fn other_mouse_buttons_and_keys_are_ignored() {
        let (button, mut tracker) = (done(), ButtonTracker::default());
        let right = |pressed| Input::PointerButton {
            button: MouseButton::Right,
            pressed,
            at: INSIDE,
        };
        tracker.input(&button, &right(true));
        assert!(!tracker.armed());
        tracker.input(&button, &press(INSIDE));
        assert!(!tracker.input(&button, &right(false)));
        let key = Input::Key {
            key: crate::input::Key::Enter,
            pressed: true,
            repeat: false,
        };
        assert!(!tracker.input(&button, &key));
        assert!(tracker.armed(), "still held");
        assert!(tracker.input(&button, &release(INSIDE)));
    }

    #[test]
    fn cancelling_disarms_without_activating() {
        let (button, mut tracker) = (done(), ButtonTracker::default());
        tracker.input(&button, &press(INSIDE));
        tracker.cancel_pointer();
        assert!(!tracker.armed() && !tracker.pressed());
        assert!(!tracker.input(&button, &release(INSIDE)));
    }

    #[test]
    fn an_armed_press_sounds_the_button_going_down_once() {
        let (button, mut tracker) = (done(), ButtonTracker::default());
        assert_eq!(tracker.take_sound(), None);
        tracker.input(&button, &press(INSIDE));
        assert_eq!(tracker.take_sound(), Some(UiSound::ButtonDown));
        assert_eq!(tracker.take_sound(), None, "once");
        tracker.input(&button, &Input::PointerMoved(OUTSIDE));
        tracker.input(&button, &Input::PointerMoved(INSIDE));
        assert_eq!(tracker.take_sound(), None, "moving makes no sound");
    }

    #[test]
    fn releasing_an_armed_press_sounds_the_button_coming_up_on_it_or_off_it() {
        for (to, activates) in [(INSIDE, true), (OUTSIDE, false)] {
            let (button, mut tracker) = (done(), ButtonTracker::default());
            tracker.input(&button, &press(INSIDE));
            tracker.take_sound();
            assert_eq!(tracker.input(&button, &release(to)), activates);
            assert_eq!(tracker.take_sound(), Some(UiSound::ButtonUp), "{to:?}");
            assert_eq!(tracker.take_sound(), None, "once");
        }
    }

    #[test]
    fn a_press_that_does_not_arm_and_a_stray_release_are_silent() {
        let disabled = Button {
            enabled: false,
            ..done()
        };
        let mut tracker = ButtonTracker::default();
        tracker.input(&disabled, &press(INSIDE));
        assert_eq!(tracker.take_sound(), None, "disabled");
        tracker.input(&disabled, &release(INSIDE));
        assert_eq!(tracker.take_sound(), None, "never armed");
        let (button, mut tracker) = (done(), ButtonTracker::default());
        tracker.input(&button, &press(OUTSIDE));
        assert_eq!(tracker.take_sound(), None, "empty space");
        tracker.input(&button, &release(INSIDE));
        assert_eq!(tracker.take_sound(), None, "never armed");
    }

    #[test]
    fn cancelling_makes_no_sound_and_keeps_one_already_made() {
        let (button, mut tracker) = (done(), ButtonTracker::default());
        tracker.input(&button, &press(INSIDE));
        tracker.cancel_pointer();
        assert_eq!(tracker.take_sound(), Some(UiSound::ButtonDown), "kept");
        tracker.input(&button, &press(INSIDE));
        tracker.take_sound();
        tracker.cancel_pointer();
        assert_eq!(tracker.take_sound(), None, "cancelling is silent");
        tracker.input(&button, &release(INSIDE));
        assert_eq!(tracker.take_sound(), None, "nor is the release after");
    }
}
