//! Modal dialogs laid out from a `DLOG`/`DITL` template.
//!
//! A [`DialogTemplate`] is the dialog's window and items in the core's own
//! terms, as [`DialogResources`](super::DialogResources) reads them. A
//! [`Dialog`] places it on the screen and gives its items life: standard
//! buttons and static text work as they are, and the caller gives each
//! application-drawn (user) item its meaning with a [`Role`]. Items are
//! numbered from 1, as the Dialog Manager numbers them.

use std::rc::Rc;

use crate::color::Color;
use crate::draw::DrawList;
use crate::font::Font;
use crate::geometry::{Bounds, Point};
use crate::image::ImageKey;
use crate::input::{Input, Key};
use crate::text::TextMetrics;

use super::button::{Button, ButtonSkin, ButtonStyle, ButtonTracker};
use super::scroll_text::ScrollText;
use super::slice::{Axis, three_slice};

/// The logical screen dialogs are placed on: Nova's 1024 x 768.
pub const SCREEN: (f32, f32) = (1024.0, 768.0);

/// Static text items' size, in Geneva.
pub const STATIC_TEXT_SIZE: f32 = 12.0;

/// Static text items' colour.
pub const STATIC_TEXT_COLOR: Color = Color::WHITE;

/// Where a dialog goes on the screen, from its `DLOG`'s positioning word.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Placement {
    /// Centred on the screen.
    Center,
    /// Centred across, with a third of the spare height above it: the
    /// Window Manager's alert position.
    Alert,
    /// At its `DLOG` bounds' own top-left corner.
    Fixed,
}

impl Placement {
    /// The placement a `DLOG` positioning word asks for: `0x280A`,
    /// `0x680A` and `0xA80A` centre the dialog; `0x300A`, `0x700A` and
    /// `0xB00A` put it in the alert position; no word, or 0, keeps its
    /// bounds; and any other word centres it.
    #[must_use]
    pub fn from_word(word: Option<u16>) -> Self {
        match word {
            None | Some(0) => Self::Fixed,
            Some(0x300A | 0x700A | 0xB00A) => Self::Alert,
            Some(_) => Self::Center,
        }
    }

    /// Where the top-left corner of a dialog with these `bounds` goes on
    /// [`SCREEN`], floored to whole units.
    #[must_use]
    pub fn origin(self, bounds: Bounds) -> Point {
        let spare = (SCREEN.0 - bounds.width(), SCREEN.1 - bounds.height());
        match self {
            Self::Center => Point::new((spare.0 / 2.0).floor(), (spare.1 / 2.0).floor()),
            Self::Alert => Point::new((spare.0 / 2.0).floor(), (spare.1 / 3.0).floor()),
            Self::Fixed => bounds.min,
        }
    }
}

/// What a dialog item is, with its data.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ItemSpec {
    /// An application-drawn area; a [`Role`] gives it its meaning.
    User,
    /// A push button with this title.
    Button(String),
    /// A check box with this title.
    CheckBox(String),
    /// A radio button with this title.
    Radio(String),
    /// Static text.
    StaticText(String),
    /// An editable text field's initial text.
    EditText(String),
    /// A picture: this interface-file `PICT`.
    Picture(i16),
    /// An icon with this ID.
    Icon(i16),
    /// A control from this `CNTL`.
    Control(i16),
    /// Any other kind.
    Other,
}

/// One item of a dialog template.
#[derive(Clone, Debug, PartialEq)]
pub struct ItemTemplate {
    /// Where it is, relative to the dialog's top-left corner.
    pub bounds: Bounds,
    /// Whether clicking it reports it.
    pub enabled: bool,
    /// What it is.
    pub kind: ItemSpec,
}

/// A dialog's window and items, in the core's terms.
#[derive(Clone, Debug, PartialEq)]
pub struct DialogTemplate {
    /// The window's content area as the `DLOG` records it.
    pub bounds: Bounds,
    /// Where it goes on the screen.
    pub placement: Placement,
    /// Its items, item 1 first.
    pub items: Vec<ItemTemplate>,
}

/// What the caller makes of a user item.
#[derive(Clone, Debug, PartialEq)]
pub enum Role {
    /// A push button with this label.
    Button(String),
    /// Scrolling text.
    ScrollText {
        /// The text, with `\r` line breaks as stored.
        text: String,
        /// Its font.
        font: Font,
        /// Its size.
        size: f32,
        /// Its colour.
        color: Color,
    },
}

/// A dialog's frame: three pictures stacked top to bottom over its bounds,
/// the middle one stretched.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DialogFrame {
    /// The top cap.
    pub top: ImageKey,
    /// The middle, stretched to fill the height.
    pub middle: ImageKey,
    /// The bottom cap.
    pub bottom: ImageKey,
    /// The top cap's height.
    pub top_height: f32,
    /// The bottom cap's height.
    pub bottom_height: f32,
}

/// Something that happened in a dialog.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DialogEvent {
    /// Item `n` (from 1) was activated: clicked, or chosen with a key.
    Item(usize),
}

/// What an item does on screen.
#[derive(Clone, Debug, PartialEq)]
enum Widget {
    Button(Button),
    /// Wrapped text; it scrolls when `scrolls`.
    Text {
        text: ScrollText,
        scrolls: bool,
    },
    /// Laid out and hit-tested, but not drawn.
    Blank,
}

#[derive(Clone, Debug, PartialEq)]
struct Item {
    /// Where it is on the screen.
    bounds: Bounds,
    enabled: bool,
    /// Whether any of it is inside the dialog: items wholly outside are
    /// neither drawn nor hit-tested.
    shown: bool,
    widget: Widget,
}

impl Item {
    /// Whether clicking it or choosing it with a key can activate it.
    fn active(&self) -> bool {
        self.shown && self.enabled
    }

    fn is_button(&self) -> bool {
        matches!(self.widget, Widget::Button(_))
    }
}

/// A modal dialog: placed on the screen, drawn over whatever is below, and
/// taking every input event while it is open.
#[derive(Clone)]
pub struct Dialog {
    bounds: Bounds,
    items: Vec<Item>,
    frame: Option<DialogFrame>,
    skin: ButtonSkin,
    style: ButtonStyle,
    metrics: Rc<dyn TextMetrics>,
    default_item: Option<usize>,
    cancel_item: Option<usize>,
    /// The focused button's index in `items`.
    focus: Option<usize>,
    /// The item a press is held on, by index, and its tracking.
    tracked: Option<usize>,
    tracker: ButtonTracker,
}

impl std::fmt::Debug for Dialog {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Dialog")
            .field("bounds", &self.bounds)
            .field("items", &self.items)
            .field("focus", &self.focus)
            .finish_non_exhaustive()
    }
}

impl Dialog {
    /// The dialog `template` describes, placed on the screen, its user
    /// items given meaning by `roles` (item number, from 1, and role) and
    /// its text measured by `metrics`.
    ///
    /// Standard buttons become buttons with their titles and static text
    /// is wrapped in Geneva. Pictures, check boxes, radio buttons, edit
    /// text, controls, icons, other kinds and user items with no role are
    /// laid out and hit-tested but draw nothing. Items wholly outside the
    /// dialog's bounds, where stock dialogs park unused items, are hidden.
    ///
    /// Item 1 is the default item (Return), and there is no cancel item
    /// (Escape), until the caller says otherwise. Buttons are drawn with
    /// [`ButtonSkin::NOVA`] and [`ButtonStyle::STOCK`], and there is no
    /// frame.
    #[must_use]
    pub fn new(
        template: &DialogTemplate,
        roles: &[(usize, Role)],
        metrics: Rc<dyn TextMetrics>,
    ) -> Self {
        let origin = template.placement.origin(template.bounds);
        let (width, height) = (template.bounds.width(), template.bounds.height());
        let items = template
            .items
            .iter()
            .enumerate()
            .map(|(index, item)| {
                let role = roles
                    .iter()
                    .find(|(number, _)| *number == index + 1)
                    .map(|(_, role)| role);
                let local = item.bounds;
                let shown = local.max.x > 0.0
                    && local.min.x < width
                    && local.max.y > 0.0
                    && local.min.y < height;
                let bounds = local.offset(origin);
                Item {
                    bounds,
                    enabled: item.enabled,
                    shown,
                    widget: widget(&item.kind, role, bounds, item.enabled, &metrics),
                }
            })
            .collect();
        Self {
            bounds: Bounds::at(origin, width, height),
            items,
            frame: None,
            skin: ButtonSkin::NOVA,
            style: ButtonStyle::STOCK,
            metrics,
            default_item: Some(1),
            cancel_item: None,
            focus: None,
            tracked: None,
            tracker: ButtonTracker::default(),
        }
    }

    /// The dialog with `frame` drawn under its items.
    #[must_use]
    pub fn with_frame(self, frame: DialogFrame) -> Self {
        Self {
            frame: Some(frame),
            ..self
        }
    }

    /// The dialog with Return activating item `item` (from 1), or nothing.
    #[must_use]
    pub fn with_default(self, item: Option<usize>) -> Self {
        Self {
            default_item: item,
            ..self
        }
    }

    /// The dialog with Escape activating item `item` (from 1), or nothing.
    #[must_use]
    pub fn with_cancel(self, item: Option<usize>) -> Self {
        Self {
            cancel_item: item,
            ..self
        }
    }

    /// The dialog with buttons drawn in `skin` and labelled in `style`.
    #[must_use]
    pub fn with_buttons(self, skin: ButtonSkin, style: ButtonStyle) -> Self {
        Self {
            skin,
            style,
            ..self
        }
    }

    /// Where the dialog is on the screen.
    #[must_use]
    pub fn bounds(&self) -> Bounds {
        self.bounds
    }

    /// Where item `item` (from 1) is on the screen, shown or not.
    #[must_use]
    pub fn item_bounds(&self, item: usize) -> Option<Bounds> {
        self.item(item).map(|item| item.bounds)
    }

    /// Whether item `item` (from 1) is shown: inside the dialog.
    #[must_use]
    pub fn item_shown(&self, item: usize) -> bool {
        self.item(item).is_some_and(|item| item.shown)
    }

    /// The focused button's item number (from 1), if any.
    #[must_use]
    pub fn focus(&self) -> Option<usize> {
        self.focus.map(|index| index + 1)
    }

    /// The first scrolling text, if any.
    #[must_use]
    pub fn scroll_text(&self) -> Option<&ScrollText> {
        self.items.iter().find_map(|item| match &item.widget {
            Widget::Text {
                text,
                scrolls: true,
            } => Some(text),
            _ => None,
        })
    }

    fn item(&self, item: usize) -> Option<&Item> {
        self.items.get(item.checked_sub(1)?)
    }

    /// Item `item` (from 1), if it can be activated.
    fn activate(&self, item: Option<usize>) -> Option<DialogEvent> {
        let item = item?;
        self.item(item)
            .filter(|found| found.active())
            .map(|_| DialogEvent::Item(item))
    }

    /// Handles one input event. The dialog is modal: it takes every event,
    /// and pointer events outside it do nothing.
    ///
    /// - A click (press and release, tracked as a button tracks it) on an
    ///   enabled, shown item activates it.
    /// - Return activates the default item, and Escape the cancel item.
    /// - Tab moves the focus to the next enabled button, and Space
    ///   activates the focused one.
    /// - Up and Down, repeats included, scroll the scrolling text.
    ///
    /// Key repeats never activate anything.
    pub fn input(&mut self, input: &Input) -> Option<DialogEvent> {
        match *input {
            Input::Key {
                key,
                pressed: true,
                repeat,
            } => self.key(key, repeat),
            Input::Key { .. } => None,
            Input::PointerButton { .. } | Input::PointerMoved(_) => self.pointer(input),
        }
    }

    fn key(&mut self, key: Key, repeat: bool) -> Option<DialogEvent> {
        match key {
            Key::Up | Key::Down => {
                let scroll = self
                    .items
                    .iter_mut()
                    .find_map(|item| match &mut item.widget {
                        Widget::Text {
                            text,
                            scrolls: true,
                        } => Some(text),
                        _ => None,
                    });
                if let Some(text) = scroll {
                    text.input(&Input::Key {
                        key,
                        pressed: true,
                        repeat,
                    });
                }
                None
            }
            _ if repeat => None,
            Key::Enter => self.activate(self.default_item),
            Key::Escape => self.activate(self.cancel_item),
            Key::Tab => {
                self.focus_next();
                None
            }
            Key::Space => self.activate(self.focus.map(|index| index + 1)),
            _ => None,
        }
    }

    /// Moves the focus to the next enabled, shown button after the focused
    /// one, wrapping round; the first such button when none is focused.
    fn focus_next(&mut self) {
        let count = self.items.len();
        let start = self.focus.map_or(0, |index| index + 1);
        self.focus = (0..count)
            .map(|step| (start + step) % count)
            .find(|&index| self.items[index].active() && self.items[index].is_button());
    }

    fn pointer(&mut self, input: &Input) -> Option<DialogEvent> {
        if let Input::PointerButton {
            pressed: true, at, ..
        } = *input
        {
            self.cancel_pointer();
            self.tracked = self
                .items
                .iter()
                .position(|item| item.active() && item.bounds.contains(at));
        }
        let index = self.tracked?;
        let item = &self.items[index];
        let activated = self.tracker.track(item.bounds, item.active(), input);
        if !self.tracker.armed() {
            self.tracked = None;
        }
        activated.then_some(DialogEvent::Item(index + 1))
    }

    /// Abandons any click in progress without activating anything.
    pub fn cancel_pointer(&mut self) {
        self.tracker.cancel_pointer();
        self.tracked = None;
    }

    /// Draws the frame, if there is one, then each shown item in item
    /// order. The focused button is outlined.
    pub fn draw(&self, list: &mut DrawList) {
        if let Some(frame) = &self.frame {
            let pieces = three_slice(
                self.bounds,
                Axis::Vertical,
                frame.top_height,
                frame.bottom_height,
            );
            for (piece, at) in pieces {
                let image = match piece {
                    super::slice::Piece::Start => frame.top,
                    super::slice::Piece::Middle => frame.middle,
                    super::slice::Piece::End => frame.bottom,
                };
                list.stretched_picture(image, at.min, at.width(), at.height());
            }
        }
        for (index, item) in self.items.iter().enumerate() {
            if !item.shown {
                continue;
            }
            match &item.widget {
                Widget::Button(button) => {
                    let pressed = self.tracked == Some(index) && self.tracker.pressed();
                    button.draw(pressed, &self.skin, &self.style, &self.metrics, list);
                    if self.focus == Some(index) {
                        outline(list, item.bounds, self.style.up);
                    }
                }
                Widget::Text { text, .. } => text.draw(list),
                Widget::Blank => {}
            }
        }
    }
}

/// What item `kind`, given `role`, does at `bounds` on the screen.
fn widget(
    kind: &ItemSpec,
    role: Option<&Role>,
    bounds: Bounds,
    enabled: bool,
    metrics: &impl TextMetrics,
) -> Widget {
    let button = |label: &str| {
        Widget::Button(Button {
            rect: bounds,
            label: label.to_owned(),
            enabled,
        })
    };
    match (kind, role) {
        (ItemSpec::Button(title), _) => button(title),
        (ItemSpec::StaticText(text), _) => Widget::Text {
            text: ScrollText::new(
                text,
                Font::Geneva,
                STATIC_TEXT_SIZE,
                STATIC_TEXT_COLOR,
                bounds,
                metrics,
            ),
            scrolls: false,
        },
        (ItemSpec::User, Some(Role::Button(label))) => button(label),
        (
            ItemSpec::User,
            Some(Role::ScrollText {
                text,
                font,
                size,
                color,
            }),
        ) => Widget::Text {
            text: ScrollText::new(text, *font, *size, *color, bounds, metrics),
            scrolls: true,
        },
        _ => Widget::Blank,
    }
}

/// A 1-unit outline just inside `rect`.
fn outline(list: &mut DrawList, rect: Bounds, color: Color) {
    let (left, top, right, bottom) = (rect.min.x, rect.min.y, rect.max.x, rect.max.y);
    let corners = [
        Point::new(left, top),
        Point::new(right, top),
        Point::new(right, bottom),
        Point::new(left, bottom),
    ];
    for (at, &from) in corners.iter().enumerate() {
        list.line(from, corners[(at + 1) % 4], 1.0, color);
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

    fn rect(x: f32, y: f32, w: f32, h: f32) -> Bounds {
        Bounds::at(at(x, y), w, h)
    }

    fn item(bounds: Bounds, enabled: bool, kind: ItemSpec) -> ItemTemplate {
        ItemTemplate {
            bounds,
            enabled,
            kind,
        }
    }

    /// A 441 x 313 dialog at (40, 40) placed by `word`, with no items.
    fn sized(word: Option<u16>) -> DialogTemplate {
        DialogTemplate {
            bounds: rect(40.0, 40.0, 441.0, 313.0),
            placement: Placement::from_word(word),
            items: Vec::new(),
        }
    }

    #[test]
    fn centre_words_and_unknown_words_centre_the_dialog() {
        for word in [0x280A, 0x680A, 0xA80A, 0x1234, 0x300B, 1] {
            let template = sized(Some(word));
            assert_eq!(template.placement, Placement::Center, "{word:#x}");
            assert_eq!(
                template.placement.origin(template.bounds),
                at(291.0, 227.0),
                "{word:#x}"
            );
        }
    }

    #[test]
    fn alert_words_put_a_third_of_the_spare_height_above() {
        for word in [0x300A, 0x700A, 0xB00A] {
            let template = sized(Some(word));
            assert_eq!(template.placement, Placement::Alert, "{word:#x}");
            assert_eq!(
                template.placement.origin(template.bounds),
                at(291.0, 151.0),
                "{word:#x}"
            );
        }
    }

    #[test]
    fn no_word_or_zero_keeps_the_bounds_corner() {
        for word in [None, Some(0)] {
            let template = sized(word);
            assert_eq!(template.placement, Placement::Fixed, "{word:?}");
            assert_eq!(template.placement.origin(template.bounds), at(40.0, 40.0));
        }
    }

    #[test]
    fn centring_floors_odd_spare_space() {
        let bounds = rect(0.0, 0.0, 1023.0, 767.0);
        assert_eq!(Placement::Center.origin(bounds), at(0.0, 0.0));
        let bounds = rect(0.0, 0.0, 1021.0, 764.0);
        assert_eq!(Placement::Center.origin(bounds), at(1.0, 2.0));
        assert_eq!(Placement::Alert.origin(bounds), at(1.0, 1.0));
    }

    /// Stock `YesNo` (`DLOG` 3002): 355 x 109, centred, with OK (1), a
    /// picture parked outside (2), static text (3), a picture (4) and
    /// Cancel (5).
    fn yes_no() -> DialogTemplate {
        DialogTemplate {
            bounds: rect(40.0, 40.0, 355.0, 109.0),
            placement: Placement::Center,
            items: vec![
                item(
                    rect(263.0, 75.0, 70.0, 20.0),
                    true,
                    ItemSpec::Button("OK".into()),
                ),
                item(
                    rect(277.0, 131.0, 68.0, 30.0),
                    false,
                    ItemSpec::Picture(129),
                ),
                item(
                    rect(52.0, 5.0, 295.0, 50.0),
                    false,
                    ItemSpec::StaticText("Text".into()),
                ),
                item(rect(7.0, 5.0, 32.0, 32.0), false, ItemSpec::Picture(130)),
                item(
                    rect(181.0, 75.0, 70.0, 20.0),
                    true,
                    ItemSpec::Button("Cancel".into()),
                ),
            ],
        }
    }

    /// Where `yes_no` goes: ((1024 - 355) / 2, (768 - 109) / 2), floored.
    const YES_NO: Point = Point::new(334.0, 329.0);

    fn dialog(template: &DialogTemplate, roles: &[(usize, Role)]) -> Dialog {
        Dialog::new(template, roles, Rc::new(MonoMetrics))
    }

    fn drawn(dialog: &Dialog) -> Vec<DrawCommand> {
        let mut list = DrawList::new();
        dialog.draw(&mut list);
        list.iter().cloned().collect()
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

    fn key(key: Key) -> Input {
        Input::Key {
            key,
            pressed: true,
            repeat: false,
        }
    }

    fn held(key: Key) -> Input {
        Input::Key {
            key,
            pressed: true,
            repeat: true,
        }
    }

    /// Clicks at `point`: press, then release.
    fn click(dialog: &mut Dialog, point: Point) -> Option<DialogEvent> {
        assert_eq!(dialog.input(&press(point)), None);
        dialog.input(&release(point))
    }

    #[test]
    fn items_are_offset_by_the_dialog_origin() {
        let dialog = dialog(&yes_no(), &[]);
        assert_eq!(dialog.bounds(), Bounds::at(YES_NO, 355.0, 109.0));
        assert_eq!(
            dialog.item_bounds(1),
            Some(rect(334.0 + 263.0, 329.0 + 75.0, 70.0, 20.0))
        );
        assert_eq!(
            dialog.item_bounds(5),
            Some(rect(334.0 + 181.0, 329.0 + 75.0, 70.0, 20.0))
        );
        assert_eq!(dialog.item_bounds(0), None);
        assert_eq!(dialog.item_bounds(6), None);
    }

    #[test]
    fn items_wholly_outside_the_bounds_are_hidden() {
        let dialog = dialog(&yes_no(), &[]);
        let shown: Vec<bool> = (1..=5).map(|n| dialog.item_shown(n)).collect();
        assert_eq!(shown, [true, false, true, true, true]);
        assert!(!dialog.item_shown(0) && !dialog.item_shown(6));
    }

    #[test]
    fn an_item_touching_an_edge_from_outside_is_hidden() {
        let edges = [
            rect(-10.0, 0.0, 10.0, 10.0),
            rect(100.0, 0.0, 10.0, 10.0),
            rect(0.0, -10.0, 10.0, 10.0),
            rect(0.0, 50.0, 10.0, 10.0),
        ];
        let overlapping = [
            rect(-9.0, 0.0, 10.0, 10.0),
            rect(99.0, 0.0, 10.0, 10.0),
            rect(0.0, -9.0, 10.0, 10.0),
            rect(0.0, 49.0, 10.0, 10.0),
        ];
        let items = edges
            .iter()
            .chain(&overlapping)
            .map(|&bounds| item(bounds, true, ItemSpec::User))
            .collect();
        let template = DialogTemplate {
            bounds: rect(0.0, 0.0, 100.0, 50.0),
            placement: Placement::Fixed,
            items,
        };
        let dialog = dialog(&template, &[]);
        let shown: Vec<bool> = (1..=8).map(|n| dialog.item_shown(n)).collect();
        assert_eq!(shown, [false, false, false, false, true, true, true, true]);
    }

    #[test]
    fn buttons_and_static_text_draw_at_their_recorded_positions() {
        let dialog = dialog(&yes_no(), &[]);
        let commands = drawn(&dialog);
        let normal = ButtonSkin::NOVA.normal;
        let ok = rect(597.0, 404.0, 70.0, 20.0);
        let cancel = rect(515.0, 404.0, 70.0, 20.0);
        let picture = |image, x, w| DrawCommand::StretchedPicture {
            image,
            top_left: at(x, 404.0),
            width: w,
            height: 20.0,
        };
        let label = |text: &str, button: Bounds| {
            let width = 0.6 * 12.0 * text.len() as f32;
            DrawCommand::Text {
                text: text.to_owned(),
                font: Font::Charcoal,
                origin: at(
                    button.center().x - width / 2.0,
                    button.center().y - 14.4 / 2.0,
                ),
                size: 12.0,
                wrap_width: None,
                color: Color::WHITE,
            }
        };
        assert_eq!(
            commands,
            [
                picture(normal.left, ok.min.x, 13.0),
                picture(normal.middle, ok.min.x + 13.0, 44.0),
                picture(normal.right, ok.max.x - 13.0, 13.0),
                label("OK", ok),
                DrawCommand::Text {
                    text: "Text".to_owned(),
                    font: Font::Geneva,
                    origin: at(334.0 + 52.0, 329.0 + 5.0),
                    size: STATIC_TEXT_SIZE,
                    wrap_width: None,
                    color: STATIC_TEXT_COLOR,
                },
                picture(normal.left, cancel.min.x, 13.0),
                picture(normal.middle, cancel.min.x + 13.0, 44.0),
                picture(normal.right, cancel.max.x - 13.0, 13.0),
                label("Cancel", cancel),
            ]
        );
    }

    #[test]
    fn a_click_on_an_enabled_item_reports_it() {
        let mut dialog = dialog(&yes_no(), &[]);
        let ok = dialog.item_bounds(1).expect("OK").center();
        let cancel = dialog.item_bounds(5).expect("Cancel").center();
        assert_eq!(click(&mut dialog, ok), Some(DialogEvent::Item(1)));
        assert_eq!(click(&mut dialog, cancel), Some(DialogEvent::Item(5)));
    }

    #[test]
    fn a_picture_item_is_hit_tested_when_enabled() {
        let mut template = yes_no();
        template.items[3].enabled = true;
        let mut dialog = dialog(&template, &[]);
        let picture = dialog.item_bounds(4).expect("picture").center();
        assert_eq!(click(&mut dialog, picture), Some(DialogEvent::Item(4)));
    }

    #[test]
    fn disabled_hidden_and_outside_clicks_report_nothing() {
        let mut template = yes_no();
        template.items[1].enabled = true;
        let mut dialog = dialog(&template, &[]);
        let text = dialog.item_bounds(3).expect("text").center();
        let picture = dialog.item_bounds(4).expect("picture").center();
        let hidden = dialog.item_bounds(2).expect("hidden").center();
        for point in [text, picture, hidden, at(10.0, 10.0), at(340.0, 335.0)] {
            assert_eq!(click(&mut dialog, point), None, "{point:?}");
        }
    }

    #[test]
    fn a_press_released_elsewhere_reports_nothing() {
        let mut dialog = dialog(&yes_no(), &[]);
        let ok = dialog.item_bounds(1).expect("OK").center();
        let cancel = dialog.item_bounds(5).expect("Cancel").center();
        dialog.input(&press(ok));
        assert_eq!(dialog.input(&release(cancel)), None);
        assert_eq!(dialog.input(&release(ok)), None, "the press is over");
    }

    #[test]
    fn a_held_button_draws_pressed_only_while_the_pointer_is_on_it() {
        let mut dialog = dialog(&yes_no(), &[]);
        let ok = dialog.item_bounds(1).expect("OK").center();
        let pressed_left = ButtonSkin::NOVA.pressed.left;
        let first_picture = |dialog: &Dialog| match drawn(dialog)[0] {
            DrawCommand::StretchedPicture { image, .. } => image,
            ref other => panic!("{other:?}"),
        };
        dialog.input(&press(ok));
        assert_eq!(first_picture(&dialog), pressed_left);
        dialog.input(&Input::PointerMoved(at(0.0, 0.0)));
        assert_eq!(first_picture(&dialog), ButtonSkin::NOVA.normal.left);
        dialog.input(&Input::PointerMoved(ok));
        assert_eq!(first_picture(&dialog), pressed_left);
        // Cancel draws normally while OK is held.
        let commands = drawn(&dialog);
        assert!(matches!(
            commands[5],
            DrawCommand::StretchedPicture { image, .. } if image == ButtonSkin::NOVA.normal.left
        ));
        dialog.cancel_pointer();
        assert_eq!(first_picture(&dialog), ButtonSkin::NOVA.normal.left);
        assert_eq!(dialog.input(&release(ok)), None);
    }

    #[test]
    fn moving_with_no_press_does_nothing() {
        let mut dialog = dialog(&yes_no(), &[]);
        let ok = dialog.item_bounds(1).expect("OK").center();
        assert_eq!(dialog.input(&Input::PointerMoved(ok)), None);
        assert_eq!(dialog.input(&release(ok)), None);
    }

    #[test]
    fn return_activates_the_default_item() {
        let mut dialog = dialog(&yes_no(), &[]);
        assert_eq!(dialog.input(&key(Key::Enter)), Some(DialogEvent::Item(1)));
        let mut dialog = dialog.with_default(Some(5));
        assert_eq!(dialog.input(&key(Key::Enter)), Some(DialogEvent::Item(5)));
        let mut dialog = dialog.with_default(None);
        assert_eq!(dialog.input(&key(Key::Enter)), None);
    }

    #[test]
    fn a_default_item_that_cannot_be_activated_does_nothing() {
        for default in [2, 3, 9] {
            let mut dialog = dialog(&yes_no(), &[]).with_default(Some(default));
            assert_eq!(dialog.input(&key(Key::Enter)), None, "{default}");
        }
    }

    #[test]
    fn escape_activates_the_cancel_item_when_there_is_one() {
        let mut dialog = dialog(&yes_no(), &[]);
        assert_eq!(dialog.input(&key(Key::Escape)), None);
        let mut dialog = dialog.with_cancel(Some(5));
        assert_eq!(dialog.input(&key(Key::Escape)), Some(DialogEvent::Item(5)));
    }

    #[test]
    fn repeats_and_releases_never_activate() {
        let mut dialog = dialog(&yes_no(), &[]).with_cancel(Some(5));
        dialog.input(&key(Key::Tab));
        for k in [Key::Enter, Key::Escape, Key::Space] {
            assert_eq!(dialog.input(&held(k)), None, "{k:?}");
            let release = Input::Key {
                key: k,
                pressed: false,
                repeat: false,
            };
            assert_eq!(dialog.input(&release), None, "{k:?}");
        }
        dialog.input(&held(Key::Tab));
        assert_eq!(dialog.focus(), Some(1), "a held Tab moves once");
    }

    #[test]
    fn tab_cycles_the_focus_through_enabled_buttons() {
        let mut template = yes_no();
        template.items.push(item(
            rect(10.0, 75.0, 70.0, 20.0),
            false,
            ItemSpec::Button("Off".into()),
        ));
        // An enabled picture is clickable, but not a button to focus.
        template.items[3].enabled = true;
        let mut dialog = dialog(&template, &[]);
        assert_eq!(dialog.focus(), None);
        assert_eq!(dialog.input(&key(Key::Space)), None, "nothing focused");
        let mut order = Vec::new();
        for _ in 0..4 {
            assert_eq!(dialog.input(&key(Key::Tab)), None);
            order.push(dialog.focus());
        }
        assert_eq!(order, [Some(1), Some(5), Some(1), Some(5)]);
        assert_eq!(dialog.input(&key(Key::Space)), Some(DialogEvent::Item(5)));
    }

    #[test]
    fn with_no_enabled_button_tab_focuses_nothing() {
        let mut template = yes_no();
        template.items[0].enabled = false;
        template.items[4].enabled = false;
        let mut dialog = dialog(&template, &[]);
        dialog.input(&key(Key::Tab));
        assert_eq!(dialog.focus(), None);
        let empty = DialogTemplate {
            items: Vec::new(),
            ..yes_no()
        };
        let mut dialog = super::tests::dialog(&empty, &[]);
        dialog.input(&key(Key::Tab));
        assert_eq!(dialog.focus(), None);
    }

    #[test]
    fn the_focused_button_is_outlined() {
        let mut dialog = dialog(&yes_no(), &[]);
        let before = drawn(&dialog).len();
        dialog.input(&key(Key::Tab));
        let commands = drawn(&dialog);
        assert_eq!(commands.len(), before + 4);
        let ok = dialog.item_bounds(1).expect("OK");
        assert_eq!(
            commands[4],
            DrawCommand::Line {
                from: ok.min,
                to: at(ok.max.x, ok.min.y),
                width: 1.0,
                color: Color::WHITE,
            }
        );
        assert_eq!(
            commands[6],
            DrawCommand::Line {
                from: ok.max,
                to: at(ok.min.x, ok.max.y),
                width: 1.0,
                color: Color::WHITE,
            }
        );
    }

    /// "Desc Dialog"-shaped: a user-item button (1), a parked picture (2),
    /// a 40 x 36 user item for text (3) and a role-less user item (4).
    fn desc_like() -> DialogTemplate {
        DialogTemplate {
            bounds: rect(0.0, 0.0, 200.0, 100.0),
            placement: Placement::Fixed,
            items: vec![
                item(rect(100.0, 70.0, 99.0, 25.0), true, ItemSpec::User),
                item(
                    rect(10.0, 150.0, 20.0, 20.0),
                    false,
                    ItemSpec::Picture(1431),
                ),
                item(rect(10.0, 10.0, 40.0, 36.0), false, ItemSpec::User),
                item(rect(150.0, 10.0, 23.0, 23.0), true, ItemSpec::User),
            ],
        }
    }

    fn desc_roles(text: &str) -> Vec<(usize, Role)> {
        vec![
            (1, Role::Button("Done".into())),
            (
                3,
                Role::ScrollText {
                    text: text.into(),
                    font: Font::Geneva,
                    size: 10.0,
                    color: Color::WHITE,
                },
            ),
        ]
    }

    #[test]
    fn roles_give_user_items_their_meaning() {
        let dialog = dialog(&desc_like(), &desc_roles("l0\rl1\rl2\rl3\rl4"));
        let commands = drawn(&dialog);
        // The button's three pictures and label, then three text lines.
        assert_eq!(commands.len(), 7, "{commands:#?}");
        assert!(matches!(
            &commands[3],
            DrawCommand::Text { text, .. } if text == "Done"
        ));
        let lines: Vec<&str> = commands[4..]
            .iter()
            .map(|command| match command {
                DrawCommand::Text { text, .. } => text.as_str(),
                other => panic!("{other:?}"),
            })
            .collect();
        assert_eq!(lines, ["l0", "l1", "l2"]);
        let text = dialog.scroll_text().expect("scrolling text");
        assert_eq!((text.visible(), text.lines().len()), (3, 5));
    }

    #[test]
    fn up_and_down_reach_the_scroll_text_with_repeats() {
        let mut dialog = dialog(&desc_like(), &desc_roles("l0\rl1\rl2\rl3\rl4"));
        assert_eq!(dialog.input(&key(Key::Down)), None);
        assert_eq!(dialog.input(&held(Key::Down)), None);
        assert_eq!(dialog.scroll_text().expect("text").first(), 2);
        dialog.input(&held(Key::Down));
        assert_eq!(dialog.scroll_text().expect("text").first(), 2, "clamped");
        dialog.input(&key(Key::Up));
        assert_eq!(dialog.scroll_text().expect("text").first(), 1);
        let release = Input::Key {
            key: Key::Up,
            pressed: false,
            repeat: false,
        };
        dialog.input(&release);
        assert_eq!(dialog.scroll_text().expect("text").first(), 1);
    }

    #[test]
    fn static_text_does_not_scroll() {
        let mut template = yes_no();
        template.items[2].kind = ItemSpec::StaticText("a b c d e f g h i j k l m".repeat(20));
        let mut dialog = dialog(&template, &[]);
        assert!(dialog.scroll_text().is_none());
        let before = drawn(&dialog);
        dialog.input(&key(Key::Down));
        assert_eq!(drawn(&dialog), before);
    }

    #[test]
    fn a_role_less_user_item_is_hit_tested_but_not_drawn() {
        let mut dialog = dialog(&desc_like(), &[]);
        assert!(drawn(&dialog).is_empty());
        let user = dialog.item_bounds(4).expect("item 4").center();
        assert_eq!(click(&mut dialog, user), Some(DialogEvent::Item(4)));
    }

    #[test]
    fn roles_apply_only_to_user_items() {
        let mut template = yes_no();
        template.items[3].kind = ItemSpec::Picture(130);
        let dialog = dialog(&template, &[(4, Role::Button("No".into()))]);
        let labels: Vec<String> = drawn(&dialog)
            .into_iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, .. } => Some(text),
                _ => None,
            })
            .collect();
        assert_eq!(labels, ["OK", "Text", "Cancel"]);
    }

    #[test]
    fn every_other_kind_draws_nothing() {
        let kinds = [
            ItemSpec::CheckBox("Check".into()),
            ItemSpec::Radio("Radio".into()),
            ItemSpec::EditText("Edit".into()),
            ItemSpec::Picture(5),
            ItemSpec::Icon(6),
            ItemSpec::Control(7),
            ItemSpec::Other,
            ItemSpec::User,
        ];
        let template = DialogTemplate {
            bounds: rect(0.0, 0.0, 100.0, 100.0),
            placement: Placement::Fixed,
            items: kinds
                .into_iter()
                .map(|kind| item(rect(10.0, 10.0, 20.0, 20.0), true, kind))
                .collect(),
        };
        assert!(drawn(&dialog(&template, &[])).is_empty());
    }

    #[test]
    fn the_frame_draws_before_the_items() {
        let frame = DialogFrame {
            top: ImageKey::picture(8524),
            middle: ImageKey::picture(8525),
            bottom: ImageKey::picture(8526),
            top_height: 9.0,
            bottom_height: 40.0,
        };
        let dialog = dialog(&yes_no(), &[]).with_frame(frame);
        let commands = drawn(&dialog);
        let piece = |image, y, height| DrawCommand::StretchedPicture {
            image,
            top_left: at(334.0, y),
            width: 355.0,
            height,
        };
        assert_eq!(
            commands[..3],
            [
                piece(frame.top, 329.0, 9.0),
                piece(frame.middle, 338.0, 60.0),
                piece(frame.bottom, 398.0, 40.0),
            ]
        );
        assert_eq!(
            commands[3..],
            drawn(&super::tests::dialog(&yes_no(), &[]))[..]
        );
    }

    #[test]
    fn buttons_take_the_given_skin_and_style() {
        let style = ButtonStyle {
            up: Color::rgba(1, 2, 3, 255),
            ..ButtonStyle::STOCK
        };
        let skin = ButtonSkin {
            cap: 5.0,
            ..ButtonSkin::NOVA
        };
        let dialog = dialog(&yes_no(), &[]).with_buttons(skin, style);
        let commands = drawn(&dialog);
        assert!(matches!(
            commands[0],
            DrawCommand::StretchedPicture { width, .. } if width == 5.0
        ));
        assert!(matches!(
            commands[3],
            DrawCommand::Text { color, .. } if color == Color::rgba(1, 2, 3, 255)
        ));
    }

    #[test]
    fn debug_shows_the_layout() {
        let debug = format!("{:?}", dialog(&yes_no(), &[]));
        assert!(debug.starts_with("Dialog { bounds: Bounds"), "{debug}");
        assert!(debug.contains("focus: None"), "{debug}");
    }
}
