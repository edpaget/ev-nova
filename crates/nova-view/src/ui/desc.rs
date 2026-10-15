//! The "Desc Dialog" (`DLOG` 3003): a long description in a scrolling box
//! with a Done button, over the game's text-and-briefing frame.
//!
//! A `dësc` whose `Graphic` is above [`LAST_UNSHOWN_GRAPHIC`] shows in
//! its picture variant, `DLOG` 3004 ([`DescTemplates::open`]), as
//! `_ShowTextDialog` picks it when its caller allows a picture
//! (@0x5a881-0x5a8df): the text in item 3 as before, over the single
//! frame `PICT` 8527, and the graphic stretched into item 2
//! (`_TextDialogUpdate` @0x51b7a, @0x51dc7-0x51e28). The choice reads the
//! ID alone; a graphic whose `PICT` is missing leaves item 2 empty, as
//! `GetPicture` finds nothing to draw.

use std::rc::Rc;
use std::time::Duration;

use crate::color::Color;
use crate::draw::DrawList;
use crate::font::Font;
use crate::image::ImageKey;
use crate::input::Input;
use crate::screen::{Screen, ScreenAction};
use crate::sound::Sound;
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

/// The picture variant's `DLOG` (and `DITL`) ID ("Desc + pict",
/// `GetNewDialog` 0xbbc @0x5a8bf).
pub const PICTURE_DESC_DIALOG: i16 = 3004;

/// The picture variant's picture item (`GetDialogItem` 2 @0x51e00).
pub const DESC_PICTURE_ITEM: usize = 2;

/// The picture variant's frame, one picture over the whole dialog: `PICT`
/// 8527, "Desc + pict" (0x214f @0x5a913).
pub const PICTURE_FRAME: ImageKey = ImageKey::picture(8527);

/// The highest `dësc` `Graphic` that shows no picture: the original shows
/// one only above it (`cmpw $0x7f` in `_ShowTextDialog` @0x5a89a,
/// `_OfferOneMission` @0xa2344, `_DoBarDialog` @0x48faa and
/// `_DoShipyardDescription` @0x5b6db-0x5b6e4).
pub const LAST_UNSHOWN_GRAPHIC: i16 = 127;

/// The picture `graphic` shows: itself when above
/// [`LAST_UNSHOWN_GRAPHIC`], else none.
#[must_use]
pub fn shown_graphic(graphic: Option<i16>) -> Option<i16> {
    graphic.filter(|&id| id > LAST_UNSHOWN_GRAPHIC)
}

/// Both "Desc Dialog" templates, or why each is missing: `DLOG` 3003's and
/// `DLOG` 3004's.
#[derive(Clone, Debug)]
pub struct DescTemplates {
    /// "Desc Dialog" ([`DESC_DIALOG`]).
    pub plain: Result<DialogTemplate, String>,
    /// "Desc Dialog" with a picture ([`PICTURE_DESC_DIALOG`]).
    pub picture: Result<DialogTemplate, String>,
}

impl DescTemplates {
    /// `text` in the variant `graphic` picks ([`shown_graphic`]), its
    /// button labelled in `style`, its text measured by `metrics`.
    ///
    /// # Errors
    ///
    /// Why that variant's template is missing: nothing shows then, as
    /// `_ShowTextDialog` shows nothing when `GetNewDialog` fails
    /// (@0x5a8fb).
    pub fn open(
        &self,
        text: &str,
        graphic: Option<i16>,
        style: ButtonStyle,
        metrics: Rc<dyn TextMetrics>,
    ) -> Result<DescDialog, String> {
        if let Some(id) = shown_graphic(graphic) {
            let template = self.picture.as_ref().map_err(Clone::clone)?;
            Ok(DescDialog::pictured(template, text, id, style, metrics))
        } else {
            let template = self.plain.as_ref().map_err(Clone::clone)?;
            Ok(DescDialog::new(template, text, style, metrics))
        }
    }
}

/// "Desc Dialog": `text` scrolling in item 3, and item 1 a Done button
/// that Return, Escape or a click activates, which closes it.
#[derive(Clone, Debug)]
pub struct DescDialog {
    dialog: Dialog,
    /// The picture variant's graphic, drawn in item 2 over
    /// [`PICTURE_FRAME`]; none for the plain dialog over [`FRAME`].
    picture: Option<i16>,
    closed: bool,
    reported: bool,
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
        let mut desc = Self::laid(template, text, style, metrics);
        desc.dialog = desc.dialog.with_frame(FRAME);
        desc
    }

    /// The picture variant `template` (stock `DLOG` 3004) showing `text`,
    /// over [`PICTURE_FRAME`], with `graphic` in item 2; its button
    /// labelled in `style`, its text measured by `metrics`.
    #[must_use]
    pub fn pictured(
        template: &DialogTemplate,
        text: &str,
        graphic: i16,
        style: ButtonStyle,
        metrics: Rc<dyn TextMetrics>,
    ) -> Self {
        Self {
            picture: Some(graphic),
            ..Self::laid(template, text, style, metrics)
        }
    }

    /// `text` laid out by `template`, with no frame and no picture.
    fn laid(
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
            .with_buttons(ButtonSkin::NOVA, style)
            .with_default(Some(DONE_ITEM))
            .with_cancel(Some(DONE_ITEM));
        Self {
            dialog,
            picture: None,
            closed: false,
            reported: false,
        }
    }

    /// The picture variant's graphic, if this is the picture variant.
    #[must_use]
    pub fn picture(&self) -> Option<i16> {
        self.picture
    }

    /// Whether Done has been activated.
    #[must_use]
    pub fn closed(&self) -> bool {
        self.closed
    }

    /// `Some(())` the first time it is asked after Done has been
    /// activated, and `None` before and after: how a host hears, once,
    /// that the dialog closed ([`Modal`](crate::spaceport::modal::Modal)).
    pub(crate) fn take_done(&mut self) -> Option<()> {
        if !self.closed || self.reported {
            return None;
        }
        self.reported = true;
        Some(())
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

    /// The plain dialog draws its frame under its items. The picture
    /// variant draws [`PICTURE_FRAME`] stretched over the dialog, as
    /// `DrawPicHandle` fills the port with it, then the items, then the
    /// graphic stretched into item 2.
    fn draw(&self, list: &mut DrawList) {
        let Some(graphic) = self.picture else {
            self.dialog.draw(list);
            return;
        };
        let bounds = self.dialog.bounds();
        list.stretched_picture(PICTURE_FRAME, bounds.min, bounds.width(), bounds.height());
        self.dialog.draw(list);
        if let Some(item) = self.dialog.item_bounds(DESC_PICTURE_ITEM) {
            list.stretched_picture(
                ImageKey::picture(graphic),
                item.min,
                item.width(),
                item.height(),
            );
        }
    }

    fn cancel_pointer(&mut self) {
        self.dialog.cancel_pointer();
    }

    /// Done's sounds as it is clicked.
    fn take_sounds(&mut self) -> Vec<Sound> {
        self.dialog
            .take_sound()
            .map(Sound::Ui)
            .into_iter()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::draw::DrawCommand;
    use crate::geometry::{Bounds, Point};
    use crate::input::{Key, MouseButton};
    use crate::sound::{Sound, UiSound};
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

    #[test]
    fn a_click_on_done_sounds_it_going_down_then_up_and_keys_are_silent() {
        let mut desc = desc();
        let done = desc.dialog().item_bounds(DONE_ITEM).expect("Done").center();
        let button = |pressed| Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at: done,
        };
        desc.input(&button(true));
        assert_eq!(desc.take_sounds(), [Sound::Ui(UiSound::ButtonDown)]);
        desc.input(&button(false));
        assert_eq!(desc.take_sounds(), [Sound::Ui(UiSound::ButtonUp)]);
        assert_eq!(desc.take_sounds(), []);
        let mut desc = self::desc();
        desc.input(&key(Key::Enter));
        assert_eq!(desc.take_sounds(), []);
    }

    // The picture variant, `DLOG` 3004.

    /// `DLOG` 3004's shape, smaller: 300 x 120 at (0, 0), fixed, with
    /// Done (1) at (100, 90), the picture (2) at (200, 10) 90 x 90 and a
    /// 60 x 36 text box (3) at (10, 10).
    fn picture_template() -> DialogTemplate {
        let item = |bounds, enabled| ItemTemplate {
            bounds,
            enabled,
            kind: ItemSpec::User,
        };
        DialogTemplate {
            bounds: rect(0.0, 0.0, 300.0, 120.0),
            placement: Placement::Fixed,
            items: vec![
                item(rect(100.0, 90.0, 99.0, 25.0), true),
                item(rect(200.0, 10.0, 90.0, 90.0), false),
                item(rect(10.0, 10.0, 60.0, 36.0), false),
            ],
        }
    }

    fn templates() -> DescTemplates {
        DescTemplates {
            plain: Ok(template()),
            picture: Ok(picture_template()),
        }
    }

    fn shown(templates: &DescTemplates, graphic: Option<i16>) -> Result<DescDialog, String> {
        templates.open(TEXT, graphic, ButtonStyle::STOCK, Rc::new(MonoMetrics))
    }

    #[test]
    fn the_picture_variants_named_values() {
        assert_eq!(
            (PICTURE_DESC_DIALOG, DESC_PICTURE_ITEM, PICTURE_FRAME),
            (3004, 2, ImageKey::picture(8527))
        );
        assert_eq!(LAST_UNSHOWN_GRAPHIC, 127);
    }

    #[test]
    fn a_graphic_is_shown_only_above_127() {
        for graphic in [None, Some(-5), Some(0), Some(127)] {
            assert_eq!(shown_graphic(graphic), None, "{graphic:?}");
        }
        for id in [128, 5008] {
            assert_eq!(shown_graphic(Some(id)), Some(id));
        }
    }

    #[test]
    fn a_graphic_above_127_shows_3004_over_8527_its_picture_in_item_2() {
        let desc = shown(&templates(), Some(5008)).expect("shown");
        assert_eq!(desc.dialog().bounds(), rect(0.0, 0.0, 300.0, 120.0));
        assert_eq!(desc.picture(), Some(5008));
        assert_eq!(desc.lines(), ["one", "two", "three", "four", "five"]);
        let commands = drawn(&desc);
        assert_eq!(
            commands[0],
            DrawCommand::StretchedPicture {
                image: PICTURE_FRAME,
                top_left: Point::new(0.0, 0.0),
                width: 300.0,
                height: 120.0,
            },
            "_TextDialogUpdate draws 8527 over the port (@0x51b7a)"
        );
        let mut alone = DrawList::new();
        desc.dialog().draw(&mut alone);
        let alone: Vec<DrawCommand> = alone.iter().cloned().collect();
        assert_eq!(commands[1..=alone.len()], alone[..]);
        assert_eq!(
            commands[alone.len() + 1..],
            [DrawCommand::StretchedPicture {
                image: ImageKey::picture(5008),
                top_left: Point::new(200.0, 10.0),
                width: 90.0,
                height: 90.0,
            }],
            "the graphic stretched into item 2 (@0x51dc7-0x51e28)"
        );
        let plain = [FRAME.top, FRAME.middle, FRAME.bottom];
        assert!(
            !commands.iter().any(|command| matches!(
                command,
                DrawCommand::Picture { image, .. }
                    | DrawCommand::StretchedPicture { image, .. }
                    | DrawCommand::TiledPicture { image, .. }
                    if plain.contains(image)
            )),
            "no plain three-slice frame under the picture variant: {commands:?}"
        );
    }

    #[test]
    fn the_picture_variant_closes_as_the_plain_one_does() {
        for input in [key(Key::Enter), key(Key::Escape)] {
            let mut desc = shown(&templates(), Some(200)).expect("shown");
            desc.input(&input);
            assert!(desc.closed(), "{input:?}");
        }
        let mut desc = shown(&templates(), Some(200)).expect("shown");
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
    fn a_graphic_of_127_or_none_shows_the_plain_dialog() {
        for graphic in [None, Some(127)] {
            let desc = shown(&templates(), graphic).expect("shown");
            assert_eq!(desc.picture(), None);
            assert_eq!(drawn(&desc), drawn(&self::desc()), "{graphic:?}");
        }
    }

    #[test]
    fn the_variants_missing_template_is_the_reason_none_shows() {
        let missing = DescTemplates {
            plain: Ok(template()),
            picture: Err("no DLOG 3004".to_owned()),
        };
        assert_eq!(
            shown(&missing, Some(200)).err().as_deref(),
            Some("no DLOG 3004")
        );
        assert!(shown(&missing, None).is_ok());
        let missing = DescTemplates {
            plain: Err("no DLOG 3003".to_owned()),
            picture: Ok(picture_template()),
        };
        assert_eq!(shown(&missing, None).err().as_deref(), Some("no DLOG 3003"));
        assert!(shown(&missing, Some(200)).is_ok());
    }
}
