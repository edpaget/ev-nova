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
//!
//! The plain dialog is shrunk to its text before it is placed
//! ([`DescDialog::new`]), as `_ShowTextDialog` shrinks 3003
//! (@0x5aac9-0x5adee, the rule in [`text_shrink`](super::text_shrink)),
//! so `_CenterWindow` @0x5ae16 centres the shrunk window. It always takes
//! the shrink; the picture variant never does, since the plain path alone
//! leaves `*0x3b52a0` at −1 (@0x5a8eb, tested @0x5aad4). The shrink's own
//! `MoveWindow` down by `trunc(0.3 × d)` (@0x5ab68-0x5abbe) is left out:
//! `_CenterWindow` moves the window again before it shows, so nothing of it
//! is seen.
//!
//! Its keys are `_TextDialogFilter`'s (@0x51f77): Return, Enter and Escape
//! close it, and Home, End, Page Up and Page Down scroll the text by the
//! original's pixel amounts, each only while the text can scroll its way,
//! on the press alone (the user's standing press-only decision). Up and
//! Down scroll 10 px, repeats included, as the original's items 5 and 6
//! do. See [`DescDialog`]'s `Screen::input`.
//!
//! The text moves by pixels, as `_ScrollDialogText` @0x51839 moves it, a
//! partial line cut at the box ([`ScrollText::pixel_scrolled`]). Every text
//! it shows does, whichever host opens it.

use std::rc::Rc;
use std::time::Duration;

use nova_sim::MapMarks;

use crate::color::Color;
use crate::draw::DrawList;
use crate::flight::view::is_map_press;
use crate::font::Font;
use crate::image::ImageKey;
use crate::input::{Input, Key};
use crate::screen::{Screen, ScreenAction};
use crate::text::{TextMetrics, wrap};

use super::button::{ButtonSkin, ButtonStyle};
use super::dialog::{Dialog, DialogEvent, DialogFrame, DialogTemplate, FocusRing, Role};
use super::picture_variant::VariantTemplates;
pub use super::scroll_text::ARROW_UNITS;
use super::scroll_text::ScrollText;
use super::text_shrink::shrink_to_text;

/// The dialog's `DLOG` (and `DITL`) ID.
pub const DESC_DIALOG: i16 = 3003;

/// The Done button's item.
pub const DONE_ITEM: usize = 1;

/// The text box's item.
pub const TEXT_ITEM: usize = 3;

/// The scroll-up arrow's item, which the shrink moves with Done.
pub const SCROLL_UP_ITEM: usize = 5;

/// The scroll-down arrow's item, which the shrink moves with Done.
pub const SCROLL_DOWN_ITEM: usize = 6;

/// How far Home and End scroll the text, in the original's pixels (the
/// engine's logical units): `_TextDialogFilter` passes `_ScrollDialogText`
/// 9999 (@0x52026, @0x520ca), past either end of any stock text.
pub const HOME_END_UNITS: f32 = 9999.0;

/// How far Page Up and Page Down scroll the text, in the original's pixels
/// (the engine's logical units): 250 (@0x5206b, @0x52112).
pub const PAGE_UNITS: f32 = 250.0;

/// The description's font, which both lays it out and measures it for
/// the shrink.
pub const BODY_FONT: Font = Font::Geneva;

/// The description's size, in [`BODY_FONT`]: near the original's 9 to 10.
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

pub use super::picture_variant::{LAST_UNSHOWN_GRAPHIC, shown_graphic};

/// Both "Desc Dialog" templates, or why each is missing: `DLOG` 3003's and
/// `DLOG` 3004's.
#[derive(Clone, Debug)]
pub struct DescTemplates {
    /// "Desc Dialog" ([`DESC_DIALOG`]).
    pub plain: Result<DialogTemplate, String>,
    /// "Desc Dialog" with a picture ([`PICTURE_DESC_DIALOG`]).
    pub picture: Result<DialogTemplate, String>,
}

impl VariantTemplates for DescTemplates {
    fn plain_template(&self) -> &Result<DialogTemplate, String> {
        &self.plain
    }

    fn picture_template(&self) -> &Result<DialogTemplate, String> {
        &self.picture
    }
}

impl DescTemplates {
    /// `text` in the variant `graphic` picks ([`VariantTemplates::pick`]), its
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
        let (shown, template) = self.pick(graphic);
        let template = template.as_ref().map_err(Clone::clone)?;
        Ok(match shown {
            Some(id) => DescDialog::pictured(template, text, id, style, metrics),
            None => DescDialog::new(template, text, style, metrics),
        })
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
    /// The marks of the map [`MAP_KEY`](crate::flight::view::MAP_KEY)
    /// opens from it, or none when it opens no map.
    map: Option<MapMarks>,
    /// Whether [`MAP_KEY`](crate::flight::view::MAP_KEY) asked for the
    /// map, until it is taken.
    map_requested: bool,
    closed: bool,
    reported: bool,
}

impl DescDialog {
    /// The dialog `template` (stock `DLOG` 3003) showing `text`, its button
    /// labelled in `style`, its text measured by `metrics`.
    ///
    /// Before it is placed, `template` is shrunk to `text` as
    /// `_ShowTextDialog` shrinks it (@0x5aac9-0x5adee): the text's height
    /// wrapped to item 3's width sets how far the bottom edge, Done, the
    /// scroll arrows (items 5 and 6) and the text box's bottom come up
    /// (see [`text_shrink`](super::text_shrink)). A text that fills the box
    /// keeps the template's size. A template with no item 3 is not shrunk.
    #[must_use]
    pub fn new(
        template: &DialogTemplate,
        text: &str,
        style: ButtonStyle,
        metrics: Rc<dyn TextMetrics>,
    ) -> Self {
        let text_height = template.items.get(TEXT_ITEM - 1).map_or(0.0, |item| {
            let lines = wrap(&metrics, BODY_FONT, BODY_SIZE, text, item.bounds.width());
            lines.len() as f32 * metrics.line_height(BODY_FONT, BODY_SIZE)
        });
        let template = shrink_to_text(
            template,
            TEXT_ITEM,
            &[DONE_ITEM, SCROLL_UP_ITEM, SCROLL_DOWN_ITEM],
            text_height,
        );
        let mut desc = Self::laid(&template, text, style, metrics);
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
                    font: BODY_FONT,
                    size: BODY_SIZE,
                    color: BODY_COLOR,
                    backing: None,
                },
            ),
        ];
        let dialog = Dialog::new(template, &roles, metrics)
            .with_buttons(ButtonSkin::NOVA, style)
            .with_default(Some(DONE_ITEM))
            .with_cancel(Some(DONE_ITEM))
            .with_pixel_scroll();
        Self {
            dialog,
            picture: None,
            map: None,
            map_requested: false,
            closed: false,
            reported: false,
        }
    }

    /// This dialog opening the map marked `map` on
    /// [`MAP_KEY`](crate::flight::view::MAP_KEY), or none: only the
    /// accept's briefing does, as only `_DoMissionAccept` passes
    /// `_ShowTextDialog` a first argument of 1, which allows item 4
    /// (@0xa1f0e, @0x5b135-0x5b146); `_TextDialogFilter` gives item 4 for
    /// the key in every text (@0x51f9c-0x51fbe), which opens nothing then.
    #[must_use]
    pub fn with_map(self, map: Option<MapMarks>) -> Self {
        Self { map, ..self }
    }

    /// The marks of the map [`MAP_KEY`](crate::flight::view::MAP_KEY)
    /// asked for since this was last taken, once: `_ShowTextDialog` opens
    /// the course map over the text (@0x5b14c-0x5b17d; see
    /// [`FlightView::open_dialog_map`](crate::flight::FlightView::open_dialog_map)),
    /// which stays open under it.
    pub fn take_map_request(&mut self) -> Option<MapMarks> {
        std::mem::take(&mut self.map_requested)
            .then_some(self.map)
            .flatten()
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

    /// The first line of the description shown, perhaps partly above the
    /// box.
    #[must_use]
    pub fn first_line(&self) -> usize {
        self.dialog.scroll_text().map_or(0, ScrollText::first)
    }

    /// How far the description is scrolled, in the original's pixels (the
    /// engine's logical units).
    #[must_use]
    pub fn scroll_offset(&self) -> f32 {
        self.dialog.scroll_text().map_or(0.0, ScrollText::offset)
    }

    /// Every wrapped line of the description.
    #[must_use]
    pub fn lines(&self) -> &[String] {
        self.dialog.scroll_text().map_or(&[], ScrollText::lines)
    }

    /// The description with its buttons taking the keyboard focus as `ring`
    /// says (the `dialog_focus_ring` rule).
    #[must_use]
    pub fn with_focus_ring(mut self, ring: FocusRing) -> Self {
        self.dialog.set_focus_ring(ring);
        self
    }

    /// The dialog itself, for its layout.
    #[must_use]
    pub fn dialog(&self) -> &Dialog {
        &self.dialog
    }
}

impl Screen for DescDialog {
    /// A press of [`MAP_KEY`](crate::flight::view::MAP_KEY) asks for the
    /// map when it opens one ([`DescDialog::take_map_request`]).
    ///
    /// Home, End, Page Up and Page Down scroll the text as
    /// `_TextDialogFilter` @0x51f77 does, through `_ScrollDialogText`
    /// @0x51839: while the text can scroll up (@0x51ff4), Home jumps
    /// [`HOME_END_UNITS`] and Page Up [`PAGE_UNITS`] toward its start;
    /// while it can scroll down (@0x52095), End and Page Down the same
    /// toward its end. A key that can't scroll its way goes on to the
    /// dialog, which ignores it, as the filter falls through to "not
    /// handled" (@0x523fc). That leaves the text's state untouched, a
    /// wheel's sub-pixel leftover included, where scrolling toward the end
    /// the text sits at would drop it. End then clears "can scroll down"
    /// at once (@0x520f7). They play no sound, as the original's arms play
    /// none.
    ///
    /// They act on the press alone. The original's filter also takes
    /// autoKey (event 5, @0x51f83-0x51f96) and so repeats them while held;
    /// by the user's standing decision of 2026-10-23, dialog keys here
    /// stay press-only, so repeats and releases do nothing.
    ///
    /// Up and Down stay with the dialog, [`ARROW_UNITS`] each while the
    /// text can scroll their way, repeats included: the original turns
    /// them into items 5 and 6, which `_ShowTextDialog` scrolls 10 px a
    /// press or repeat (@0x5b0af-0x5b130).
    ///
    /// Every other event goes to the dialog, and activating Done closes
    /// it. It never quits.
    fn input(&mut self, input: &Input) -> ScreenAction {
        if is_map_press(input) {
            self.map_requested = self.map.is_some();
            return ScreenAction::None;
        }
        if let Input::Key {
            key,
            pressed: true,
            repeat: false,
        } = *input
            && let Some(text) = self.dialog.scroll_text_mut()
        {
            let units = match key {
                Key::Home if text.can_scroll_up() => Some(-HOME_END_UNITS),
                Key::PageUp if text.can_scroll_up() => Some(-PAGE_UNITS),
                Key::End if text.can_scroll_down() => Some(HOME_END_UNITS),
                Key::PageDown if text.can_scroll_down() => Some(PAGE_UNITS),
                _ => None,
            };
            if let Some(units) = units {
                text.scroll_units(units);
                if key == Key::End {
                    text.clear_can_scroll_down();
                }
                return ScreenAction::None;
            }
        }
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
        self.dialog.draw_stretched_over(list, PICTURE_FRAME);
        self.dialog.draw(list);
        self.dialog
            .draw_picture_in(list, DESC_PICTURE_ITEM, graphic);
    }

    fn cancel_pointer(&mut self) {
        self.dialog.cancel_pointer();
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::draw::DrawCommand;
    use crate::geometry::{Bounds, Point};
    use crate::input::MouseButton;
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
    fn with_marks_the_map_key_asks_for_the_map_and_without_none() {
        let m = crate::flight::view::MAP_KEY;
        let marks = MapMarks {
            green: Some(nova_sim::SystemId(131)),
            multi_arrows: true,
        };
        let mut briefing = desc().with_map(Some(marks));
        for input in [
            Input::Key {
                key: m,
                pressed: true,
                repeat: true,
            },
            Input::Key {
                key: m,
                pressed: false,
                repeat: false,
            },
        ] {
            briefing.input(&input);
            assert_eq!(briefing.take_map_request(), None, "{input:?}");
        }
        briefing.input(&key(m));
        assert_eq!(briefing.take_map_request(), Some(marks));
        assert_eq!(briefing.take_map_request(), None, "once");
        assert!(!briefing.closed(), "it stays open under the map");
        assert_eq!(briefing.take_sounds(), [], "silent");
        let mut plain = desc();
        plain.input(&key(m));
        assert_eq!(plain.take_map_request(), None, "no map for this text");
        assert!(!plain.closed());
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
        assert_eq!(desc.scroll_offset(), ARROW_UNITS);
        assert_eq!(desc.first_line(), 0);
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
        let text = desc.dialog().item_bounds(TEXT_ITEM).expect("text").center();
        for at in [text, Point::new(1.0, 1.0)] {
            for pressed in [true, false] {
                desc.input(&Input::PointerButton {
                    button: MouseButton::Left,
                    pressed,
                    at,
                });
            }
            assert!(!desc.closed(), "a click at {at:?}");
        }
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
        assert_eq!(desc.scroll_offset(), 0.0);
    }

    /// `_TrackTextButtons` @0x26bb0 plays nothing under the pointer.
    #[test]
    fn pointer_click_is_silent_as_the_original() {
        let mut desc = desc();
        let done = desc.dialog().item_bounds(DONE_ITEM).expect("Done").center();
        let button = |pressed| Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at: done,
        };
        desc.input(&button(true));
        assert_eq!(desc.take_sounds(), []);
        desc.input(&button(false));
        assert_eq!(desc.take_sounds(), []);
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

    /// A key that activates Done plays nothing in the original:
    /// `_ShipDescFilter` @0x510e5 and `_TextDialogFilter` @0x51f77 map
    /// Return, Enter and Escape to item 1 (@0x51fca-0x51fef) with no
    /// sound, and `_MyModalDialog` @0x59954 calls no sound routine.
    #[test]
    fn keyboard_activation_is_silent_as_the_original() {
        for input in [key(Key::Enter), key(Key::Escape)] {
            let mut desc = desc();
            assert_eq!(desc.take_sounds(), []);
            desc.input(&input);
            assert!(desc.closed(), "{input:?}");
            assert_eq!(desc.take_sounds(), [], "{input:?}");
        }
        let mut desc = desc();
        desc.input(&key(Key::Tab));
        desc.input(&key(Key::Space));
        assert!(desc.closed(), "Space on Done");
        assert_eq!(desc.take_sounds(), [], "Space");
    }

    // The shrink to the text, `_ShowTextDialog` @0x5aabe-0x5adee.

    /// Stock `DLOG` 3003 and its `DITL`, placed by `_CenterWindow`: 441 x
    /// 313, Done (1), the parked picture (2), the 417 x 262 text box (3),
    /// the parked map item (4) and the scroll arrows (5, 6).
    fn stock_template() -> DialogTemplate {
        let item = |bounds| ItemTemplate {
            bounds,
            enabled: true,
            kind: ItemSpec::User,
        };
        DialogTemplate {
            bounds: rect(0.0, 0.0, 441.0, 313.0),
            placement: Placement::CenterWindow,
            items: vec![
                item(rect(173.0, 281.0, 99.0, 25.0)),
                item(rect(221.0, 329.0, 68.0, 30.0)),
                item(rect(11.0, 10.0, 417.0, 262.0)),
                item(rect(70.0, 345.0, 78.0, 25.0)),
                item(rect(330.0, SCROLL_ARROW_Y, 23.0, 23.0)),
                item(rect(363.0, SCROLL_ARROW_Y, 23.0, 23.0)),
            ],
        }
    }

    const SCROLL_ARROW_Y: f32 = 282.0;

    fn stock_desc(text: &str) -> DescDialog {
        DescDialog::new(
            &stock_template(),
            text,
            ButtonStyle::STOCK,
            Rc::new(MonoMetrics),
        )
    }

    fn centred(width: f32, height: f32) -> Bounds {
        Bounds::at(
            crate::ui::dialog::center_window(crate::ui::dialog::SCREEN, (width, height)),
            width,
            height,
        )
    }

    #[test]
    fn a_one_line_text_shrinks_3003_to_441_by_115_centred_on_the_window() {
        let desc = stock_desc("A short text.");
        let bounds = desc.dialog().bounds();
        assert_eq!(bounds, centred(441.0, 115.0));
        assert_eq!(bounds.min, Point::new(292.0, 327.0));
        let local = |item| {
            let at = desc.dialog().item_bounds(item).expect("item");
            at.offset(Point::new(-bounds.min.x, -bounds.min.y))
        };
        assert_eq!(local(DONE_ITEM), rect(173.0, 83.0, 99.0, 25.0));
        assert_eq!(local(SCROLL_UP_ITEM).min.y, SCROLL_ARROW_Y - 198.0);
        assert_eq!(local(SCROLL_DOWN_ITEM).min.y, SCROLL_ARROW_Y - 198.0);
        assert_eq!(local(TEXT_ITEM), rect(11.0, 10.0, 417.0, 64.0));
        let text = desc.dialog().scroll_text().expect("text");
        assert_eq!(text.rect().height(), 64.0);
        assert_eq!(local(DESC_PICTURE_ITEM).min.y, 329.0, "parked");
    }

    #[test]
    fn a_text_that_fills_the_box_keeps_3003_at_441_by_313() {
        let long = "line\r".repeat(21) + "line";
        let desc = stock_desc(&long);
        assert_eq!(desc.lines().len(), 22, "264 high, over the 262 box");
        let bounds = desc.dialog().bounds();
        assert_eq!(bounds, centred(441.0, 313.0));
        assert_eq!(bounds.min, Point::new(292.0, 228.0));
        let done = desc.dialog().item_bounds(DONE_ITEM).expect("Done");
        assert_eq!(done.min.y - bounds.min.y, 281.0);
    }

    #[test]
    fn the_text_is_measured_wrapped_to_the_text_boxs_width() {
        // 85 characters wrap to two lines in the 417 wide box (83 a line
        // at 5 each): 24 high, still under the 48 floor.
        let wrapped = stock_desc(&("x ".repeat(42) + "x"));
        assert_eq!(wrapped.lines().len(), 2);
        assert_eq!(wrapped.dialog().bounds().height(), 115.0);
        // Ten lines, 120 high: the dialog is 120 + 16 + 51 tall.
        let ten = stock_desc(&("word\r".repeat(9) + "word"));
        assert_eq!(ten.dialog().bounds().height(), 187.0);
    }

    #[test]
    fn the_frame_spans_the_shrunk_dialog() {
        let desc = stock_desc("A short text.");
        let bounds = desc.dialog().bounds();
        let commands = drawn(&desc);
        assert!(
            commands.contains(&DrawCommand::StretchedPicture {
                image: FRAME.bottom,
                top_left: Point::new(bounds.min.x, bounds.max.y - FRAME.bottom_height),
                width: 441.0,
                height: FRAME.bottom_height,
            }),
            "the bottom cap at the shrunk bottom: {commands:?}"
        );
    }

    #[test]
    fn the_picture_variant_never_shrinks() {
        let one_line = templates()
            .open("one", Some(5008), ButtonStyle::STOCK, Rc::new(MonoMetrics))
            .expect("shown");
        assert_eq!(one_line.dialog().bounds(), rect(0.0, 0.0, 300.0, 120.0));
        let pictured = DescDialog::pictured(
            &stock_template(),
            "one",
            5008,
            ButtonStyle::STOCK,
            Rc::new(MonoMetrics),
        );
        assert_eq!(pictured.dialog().bounds(), centred(441.0, 313.0));
    }

    #[test]
    fn the_scroll_arrows_are_items_5_and_6() {
        assert_eq!((SCROLL_UP_ITEM, SCROLL_DOWN_ITEM), (5, 6));
    }

    // Home, End, Page Up and Page Down, `_TextDialogFilter`
    // @0x51ff4-0x52137.

    /// `lines` lines, "l0" on, in [`template`]'s 36 high box, 12 units a
    /// line.
    fn long_desc_of(lines: usize) -> DescDialog {
        DescDialog::new(
            &template(),
            &numbered(lines),
            ButtonStyle::STOCK,
            Rc::new(MonoMetrics),
        )
    }

    fn numbered(lines: usize) -> String {
        (0..lines)
            .map(|n| format!("l{n}"))
            .collect::<Vec<_>>()
            .join("\r")
    }

    /// Sixty lines, 720 high: [`END`] is as far as they scroll.
    fn long_desc() -> DescDialog {
        long_desc_of(60)
    }

    const END: f32 = 684.0;

    #[test]
    fn page_down_scrolls_exactly_250_units() {
        let mut desc = long_desc();
        assert_eq!(desc.lines().len(), 60);
        assert_eq!(desc.input(&key(Key::PageDown)), ScreenAction::None);
        assert_eq!(desc.scroll_offset(), 250.0);
        assert_eq!(desc.first_line(), 20, "250 / 12, rounded down");
        desc.input(&key(Key::PageDown));
        assert_eq!(desc.scroll_offset(), 500.0);
    }

    #[test]
    fn page_up_scrolls_back_exactly_250_units() {
        let mut desc = long_desc();
        desc.input(&key(Key::End));
        desc.input(&key(Key::PageUp));
        assert_eq!(desc.scroll_offset(), END - 250.0);
    }

    #[test]
    fn end_reaches_the_end_and_home_the_start() {
        let mut desc = long_desc();
        desc.input(&key(Key::End));
        assert_eq!(desc.scroll_offset(), END);
        desc.input(&key(Key::Home));
        assert_eq!(desc.scroll_offset(), 0.0);
    }

    #[test]
    fn at_the_top_home_and_page_up_do_nothing() {
        let mut desc = long_desc();
        desc.input(&key(Key::Home));
        desc.input(&key(Key::PageUp));
        assert_eq!(desc.scroll_offset(), 0.0);
        desc.input(&key(Key::PageDown));
        assert_eq!(desc.scroll_offset(), 250.0, "nothing past the top");
    }

    #[test]
    fn at_the_end_end_and_page_down_do_nothing() {
        let mut desc = long_desc();
        desc.input(&key(Key::End));
        desc.input(&key(Key::End));
        desc.input(&key(Key::PageDown));
        assert_eq!(desc.scroll_offset(), END);
        desc.input(&key(Key::PageUp));
        assert_eq!(desc.scroll_offset(), END - 250.0, "nothing past the end");
    }

    /// `_TextDialogFilter`'s End arm clears the flag itself (@0x520f7).
    #[test]
    fn end_clears_down_at_once() {
        let mut desc = long_desc();
        desc.input(&key(Key::End));
        let text = desc.dialog().scroll_text().expect("text");
        assert!(!text.can_scroll_down());
    }

    /// `_ScrollDialogText` clears it only when a try fails (@0x51958).
    #[test]
    fn a_page_down_landing_exactly_on_the_end_leaves_down_set_until_the_next() {
        // 128 lines are 1536 high, and 1500 past the 36 box: six pages.
        let mut desc = long_desc_of(128);
        for _ in 0..6 {
            desc.input(&key(Key::PageDown));
        }
        assert_eq!(desc.scroll_offset(), 1500.0);
        let can_down = |desc: &DescDialog| {
            let text = desc.dialog().scroll_text().expect("text");
            text.can_scroll_down()
        };
        assert!(can_down(&desc));
        desc.input(&key(Key::PageDown));
        assert_eq!(desc.scroll_offset(), 1500.0);
        assert!(!can_down(&desc));
    }

    /// A wheel scroll of `units` over [`template`]'s text box.
    fn wheel_units(units: f32) -> Input {
        Input::Scroll {
            delta: crate::input::ScrollDelta::Units(units),
            at: Point::new(30.0, 20.0),
        }
    }

    /// The filter's not-handled fall-through (@0x523fc) leaves the text as
    /// it was, a wheel's leftover included: scrolling up from the top
    /// would drop it.
    #[test]
    fn at_the_top_home_and_page_up_keep_the_wheels_leftover() {
        let mut desc = long_desc();
        desc.input(&wheel_units(0.6));
        assert_eq!(desc.scroll_offset(), 0.0);
        desc.input(&key(Key::Home));
        desc.input(&key(Key::PageUp));
        desc.input(&wheel_units(0.6));
        assert_eq!(desc.scroll_offset(), 1.0, "the 0.6 left over and 0.6");
    }

    /// The mirror of
    /// [`at_the_top_home_and_page_up_keep_the_wheels_leftover`].
    #[test]
    fn at_the_end_end_and_page_down_keep_the_wheels_leftover() {
        let mut desc = long_desc();
        desc.input(&key(Key::End));
        desc.input(&wheel_units(-0.6));
        assert_eq!(desc.scroll_offset(), END);
        desc.input(&key(Key::End));
        desc.input(&key(Key::PageDown));
        desc.input(&wheel_units(-0.6));
        assert_eq!(desc.scroll_offset(), END - 1.0, "the 0.6 left over and 0.6");
    }

    #[test]
    fn up_and_down_scroll_on_a_press_or_a_repeat() {
        let mut desc = long_desc();
        for (key, repeat, offset) in [
            (Key::Down, false, 10.0),
            (Key::Down, true, 20.0),
            (Key::Up, true, 10.0),
            (Key::Up, false, 0.0),
        ] {
            desc.input(&Input::Key {
                key,
                pressed: true,
                repeat,
            });
            assert_eq!(desc.scroll_offset(), offset, "{key:?} {repeat}");
        }
    }

    #[test]
    fn scroll_keys_are_press_only() {
        for scroll in [Key::Home, Key::End, Key::PageUp, Key::PageDown] {
            let mut desc = long_desc();
            desc.input(&key(Key::PageDown));
            for input in [
                Input::Key {
                    key: scroll,
                    pressed: true,
                    repeat: true,
                },
                Input::Key {
                    key: scroll,
                    pressed: false,
                    repeat: false,
                },
            ] {
                desc.input(&input);
                assert_eq!(desc.scroll_offset(), 250.0, "{input:?}");
                assert!(!desc.closed(), "{input:?}");
            }
        }
    }

    #[test]
    fn scroll_keys_never_close_it_or_ask_for_the_map() {
        let marks = MapMarks {
            green: Some(nova_sim::SystemId(131)),
            multi_arrows: false,
        };
        let mut desc = long_desc().with_map(Some(marks));
        for scroll in [Key::End, Key::PageUp, Key::Home, Key::PageDown] {
            assert_eq!(desc.input(&key(scroll)), ScreenAction::None);
            assert!(!desc.closed(), "{scroll:?}");
            assert_eq!(desc.take_map_request(), None, "{scroll:?}");
            assert_eq!(desc.take_sounds(), [], "{scroll:?}");
        }
    }

    #[test]
    fn a_short_text_ends_where_its_last_line_fills_the_box_and_one_line_never_moves() {
        let mut five = desc();
        five.input(&key(Key::End));
        assert_eq!(five.scroll_offset(), 24.0, "60 high in a 36 box");
        five.input(&key(Key::Home));
        assert_eq!(five.scroll_offset(), 0.0);
        five.input(&key(Key::PageDown));
        assert_eq!(five.scroll_offset(), 24.0);
        let mut one = DescDialog::new(&template(), "one", ButtonStyle::STOCK, Rc::new(MonoMetrics));
        for scroll in [Key::End, Key::PageDown] {
            one.input(&key(scroll));
            assert_eq!(one.scroll_offset(), 0.0, "{scroll:?}");
            assert!(!one.closed());
        }
    }

    #[test]
    fn the_picture_variant_takes_the_scroll_keys_too() {
        let mut desc = pictured_long();
        assert_eq!(desc.picture(), Some(5008));
        desc.input(&key(Key::End));
        assert_eq!(desc.scroll_offset(), END);
        desc.input(&key(Key::PageUp));
        assert_eq!(desc.scroll_offset(), END - 250.0);
        desc.input(&key(Key::Home));
        assert_eq!(desc.scroll_offset(), 0.0);
        desc.input(&key(Key::Down));
        assert_eq!(desc.scroll_offset(), ARROW_UNITS);
    }

    fn pictured_long() -> DescDialog {
        templates()
            .open(
                &numbered(60),
                Some(5008),
                ButtonStyle::STOCK,
                Rc::new(MonoMetrics),
            )
            .expect("shown")
    }

    /// Each body run drawn: its text, top and clip, if any.
    fn body_runs(desc: &DescDialog) -> Vec<(String, f32, Option<Bounds>)> {
        drawn(desc)
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text {
                    text,
                    origin,
                    font: Font::Geneva,
                    ..
                } => Some((text.clone(), origin.y, None)),
                DrawCommand::Clipped { clip, command } => match &**command {
                    DrawCommand::Text { text, origin, .. } => {
                        Some((text.clone(), origin.y, Some(*clip)))
                    }
                    _ => None,
                },
                _ => None,
            })
            .collect()
    }

    /// `CopyBits` copies item 3's rect to the window (@0x51a1f), so a
    /// partial line is cut at both its edges.
    #[test]
    fn a_partial_line_is_clipped_to_item_3_at_the_top_and_the_bottom() {
        for mut desc in [long_desc(), pictured_long()] {
            desc.input(&key(Key::Down));
            let clip = desc.dialog().item_bounds(TEXT_ITEM);
            assert_eq!(
                body_runs(&desc),
                [
                    ("l0".to_owned(), 0.0, clip),
                    ("l1".to_owned(), 12.0, None),
                    ("l2".to_owned(), 24.0, None),
                    ("l3".to_owned(), 36.0, clip),
                ]
            );
        }
    }

    /// `_TextDialogFilter` @0x51f77 maps no Tab or Space.
    #[test]
    fn tab_then_space_closes_it_only_with_the_focus_ring() {
        let mut ringed = desc();
        ringed.input(&key(Key::Tab));
        ringed.input(&key(Key::Space));
        assert!(ringed.closed(), "by default");
        let mut off = desc().with_focus_ring(FocusRing::Off);
        off.input(&key(Key::Tab));
        off.input(&key(Key::Space));
        assert!(!off.closed());
        assert_eq!(off.dialog().focus(), None);
    }
}
