//! The "Desc Dialog" (`DLOG` 3003): a long description in a scrolling box
//! with a Done button, over the game's text-and-briefing frame.

use std::rc::Rc;
use std::time::Duration;

use crate::color::Color;
use crate::draw::DrawList;
use crate::font::Font;
use crate::image::ImageKey;
use crate::input::Input;
use crate::screen::{Screen, ScreenAction};
use crate::text::TextMetrics;

use super::button::{ButtonSkin, ButtonStyle};
use super::dialog::{Dialog, DialogEvent, DialogFrame, DialogTemplate, Role};
use super::scroll_text::ScrollText;

/// The dialog's `DLOG` (and `DITL`) ID.
pub const DESC_DIALOG: i16 = 3003;

/// The Done button's item.
pub const DONE_ITEM: usize = 1;

/// The text box's item.
pub const TEXT_ITEM: usize = 3;

/// The description's size, in Geneva: near the original's 9 to 10.
pub const BODY_SIZE: f32 = 10.0;

/// The description's colour.
pub const BODY_COLOR: Color = Color::WHITE;

/// The Done button's label.
pub const DONE_LABEL: &str = "Done";

/// The game's "Generic text/briefing" frame (`PICT`s 8524 to 8526, 441
/// wide), which "Desc Dialog" is drawn over.
pub const FRAME: DialogFrame = DialogFrame {
    top: ImageKey::picture(8524),
    middle: ImageKey::picture(8525),
    bottom: ImageKey::picture(8526),
    top_height: 9.0,
    bottom_height: 40.0,
};

/// "Desc Dialog": `text` scrolling in item 3, and item 1 a Done button
/// that Return, Escape or a click activates, which closes it.
#[derive(Clone, Debug)]
pub struct DescDialog {
    dialog: Dialog,
    closed: bool,
}

impl DescDialog {
    /// The dialog `template` (stock `DLOG` 3003) showing `text`, its button
    /// labelled in `style`, its text measured by `metrics`.
    #[must_use]
    pub fn new(
        template: &DialogTemplate,
        text: &str,
        style: ButtonStyle,
        metrics: Rc<dyn TextMetrics>,
    ) -> Self {
        let roles = [
            (DONE_ITEM, Role::Button(DONE_LABEL.to_owned())),
            (
                TEXT_ITEM,
                Role::ScrollText {
                    text: text.to_owned(),
                    font: Font::Geneva,
                    size: BODY_SIZE,
                    color: BODY_COLOR,
                },
            ),
        ];
        let dialog = Dialog::new(template, &roles, metrics)
            .with_frame(FRAME)
            .with_buttons(ButtonSkin::NOVA, style)
            .with_default(Some(DONE_ITEM))
            .with_cancel(Some(DONE_ITEM));
        Self {
            dialog,
            closed: false,
        }
    }

    /// Whether Done has been activated.
    #[must_use]
    pub fn closed(&self) -> bool {
        self.closed
    }

    /// The first line of the description shown.
    #[must_use]
    pub fn first_line(&self) -> usize {
        self.dialog.scroll_text().map_or(0, ScrollText::first)
    }

    /// Every wrapped line of the description.
    #[must_use]
    pub fn lines(&self) -> &[String] {
        self.dialog.scroll_text().map_or(&[], ScrollText::lines)
    }

    /// The dialog itself, for its layout.
    #[must_use]
    pub fn dialog(&self) -> &Dialog {
        &self.dialog
    }
}

impl Screen for DescDialog {
    /// Every event goes to the dialog; activating Done closes it. It never
    /// quits.
    fn input(&mut self, input: &Input) -> ScreenAction {
        if self.dialog.input(input) == Some(DialogEvent::Item(DONE_ITEM)) {
            self.closed = true;
        }
        ScreenAction::None
    }

    /// Nothing moves on its own.
    fn tick(&mut self, _dt: Duration) {}

    fn draw(&self, list: &mut DrawList) {
        self.dialog.draw(list);
    }

    fn cancel_pointer(&mut self) {
        self.dialog.cancel_pointer();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::draw::DrawCommand;
    use crate::geometry::{Bounds, Point};
    use crate::input::{Key, MouseButton};
    use crate::text::fixture::MonoMetrics;
    use crate::ui::dialog::{ItemSpec, ItemTemplate, Placement};

    fn rect(x: f32, y: f32, w: f32, h: f32) -> Bounds {
        Bounds::at(Point::new(x, y), w, h)
    }

    /// "Desc Dialog"'s shape, smaller: 200 x 120 at (0, 0), fixed, with
    /// Done (1) at (100, 90), a parked picture (2) and a 60 x 36 text box
    /// (3) at (10, 10): 12 characters a line, 3 lines.
    fn template() -> DialogTemplate {
        let item = |bounds, enabled, kind| ItemTemplate {
            bounds,
            enabled,
            kind,
        };
        DialogTemplate {
            bounds: rect(0.0, 0.0, 200.0, 120.0),
            placement: Placement::Fixed,
            items: vec![
                item(rect(100.0, 90.0, 99.0, 25.0), true, ItemSpec::User),
                item(
                    rect(10.0, 150.0, 20.0, 20.0),
                    false,
                    ItemSpec::Picture(1431),
                ),
                item(rect(10.0, 10.0, 60.0, 36.0), false, ItemSpec::User),
            ],
        }
    }

    const TEXT: &str = "one\rtwo\rthree\rfour\rfive";

    fn desc() -> DescDialog {
        DescDialog::new(&template(), TEXT, ButtonStyle::STOCK, Rc::new(MonoMetrics))
    }

    fn drawn(desc: &DescDialog) -> Vec<DrawCommand> {
        let mut list = DrawList::new();
        desc.draw(&mut list);
        list.iter().cloned().collect()
    }

    fn key(key: Key) -> Input {
        Input::Key {
            key,
            pressed: true,
            repeat: false,
        }
    }

    #[test]
    fn it_draws_the_frame_then_the_button_then_the_text() {
        let commands = drawn(&desc());
        let picture = |image, x, y, w, h| DrawCommand::StretchedPicture {
            image,
            top_left: Point::new(x, y),
            width: w,
            height: h,
        };
        let normal = ButtonSkin::NOVA.normal;
        let text = |text: &str, y| DrawCommand::Text {
            text: text.to_owned(),
            font: Font::Geneva,
            origin: Point::new(10.0, y),
            size: BODY_SIZE,
            wrap_width: None,
            color: BODY_COLOR,
        };
        let done_width = 4.0 * 7.2_f32;
        assert_eq!(
            commands,
            [
                picture(FRAME.top, 0.0, 0.0, 200.0, 9.0),
                picture(FRAME.middle, 0.0, 9.0, 200.0, 71.0),
                picture(FRAME.bottom, 0.0, 80.0, 200.0, 40.0),
                picture(normal.left, 100.0, 90.0, 13.0, 25.0),
                picture(normal.middle, 113.0, 90.0, 73.0, 25.0),
                picture(normal.right, 186.0, 90.0, 13.0, 25.0),
                DrawCommand::Text {
                    text: DONE_LABEL.to_owned(),
                    font: Font::Charcoal,
                    origin: Point::new(149.5 - done_width / 2.0, 102.5 - 14.4 / 2.0),
                    size: 12.0,
                    wrap_width: None,
                    color: Color::WHITE,
                },
                text("one", 10.0),
                text("two", 22.0),
                text("three", 34.0),
            ]
        );
    }

    #[test]
    fn the_frame_is_the_generic_text_frame() {
        assert_eq!(
            FRAME,
            DialogFrame {
                top: ImageKey::picture(8524),
                middle: ImageKey::picture(8525),
                bottom: ImageKey::picture(8526),
                top_height: 9.0,
                bottom_height: 40.0,
            }
        );
        assert_eq!(DESC_DIALOG, 3003);
    }

    #[test]
    fn down_scrolls_the_text() {
        let mut desc = desc();
        assert_eq!(desc.lines(), ["one", "two", "three", "four", "five"]);
        assert_eq!(desc.first_line(), 0);
        assert_eq!(desc.input(&key(Key::Down)), ScreenAction::None);
        assert_eq!(desc.first_line(), 1);
        assert!(!desc.closed());
    }

    #[test]
    fn return_escape_and_a_click_on_done_close_it() {
        for input in [key(Key::Enter), key(Key::Escape)] {
            let mut desc = desc();
            assert_eq!(desc.input(&input), ScreenAction::None);
            assert!(desc.closed(), "{input:?}");
        }
        let mut desc = desc();
        let done = desc.dialog().item_bounds(DONE_ITEM).expect("Done").center();
        for pressed in [true, false] {
            desc.input(&Input::PointerButton {
                button: MouseButton::Left,
                pressed,
                at: done,
            });
        }
        assert!(desc.closed());
    }

    #[test]
    fn other_input_leaves_it_open() {
        let mut desc = desc();
        for input in [key(Key::Space), key(Key::Tab), key(Key::Char('i'))] {
            desc.input(&input);
        }
        assert!(!desc.closed());
        desc.tick(Duration::from_secs(1));
        assert!(!desc.closed());
    }

    #[test]
    fn cancelling_the_pointer_abandons_a_click_on_done() {
        let mut desc = desc();
        let done = desc.dialog().item_bounds(DONE_ITEM).expect("Done").center();
        let button = |pressed| Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at: done,
        };
        desc.input(&button(true));
        desc.cancel_pointer();
        desc.release_keys();
        desc.input(&button(false));
        assert!(!desc.closed());
    }

    #[test]
    fn with_no_text_box_there_are_no_lines() {
        let mut template = template();
        template.items.truncate(1);
        let desc = DescDialog::new(&template, TEXT, ButtonStyle::STOCK, Rc::new(MonoMetrics));
        assert!(desc.lines().is_empty());
        assert_eq!(desc.first_line(), 0);
    }
}
