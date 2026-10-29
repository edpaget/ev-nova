//! Modal dialogs laid out from a `DLOG`/`DITL` template.
//!
//! A [`DialogTemplate`] is the dialog's window and items in the core's own
//! terms, as [`DialogResources`](super::DialogResources) reads them. A
//! [`Dialog`] places it on the screen and gives its items life: standard
//! buttons and static text work as they are, picture items draw their
//! interface-file `PICT` stretched over their rectangle (and nothing when
//! the interface file lacks it, as stock 1431), and the caller gives each
//! application-drawn (user) item its meaning with a [`Role`]. Items are
//! numbered from 1, as the Dialog Manager numbers them.
//!
//! # Keyboard focus
//!
//! The original's in-game dialogs have no keyboard focus: `_MyModalDialog`
//! (@0x59954) draws none, and each dialog's filter maps only its own keys
//! (`_TextDialogFilter` @0x51f77 Return, Enter and Escape to item 1;
//! `_CommFilter` @0x908ed, `_PlanetCommFilter` @0x911a9, `_HaggleFilter`
//! @0x91d9d, `_PlunderFilter` @0x92f20, `_CapturedAssignmentFilter`
//! @0x51782 and `_EscortFilter` @0x950e9 their letters), none of them Tab
//! or Space. A [`Dialog`] keeps a focus ring anyway by default
//! ([`FocusRing::Tab`]): Tab moves an outlined focus through the enabled
//! buttons and Space presses the focused one, kept for accessibility by
//! the user's decision of 2026-10-28. The `dialog_focus_ring` rule's engine
//! reading ([`FocusRing::Off`]) gives the original's dialogs.
//!
//! Two screens' filters do map Tab: the exchange's (`_TradeFilter`
//! @0x4e184, Shift stepping back) and the mission computer's
//! (`_MissionFilter` @0x9f8cc) step their list's selection. There the
//! ring keeps Tab under [`FocusRing::Tab`], and the screen's row step
//! takes it under [`FocusRing::Off`] (see
//! [`TradeScreen`](crate::spaceport::trade::TradeScreen) and
//! [`MissionComputerScreen`](crate::spaceport::missions::MissionComputerScreen)).

use std::rc::Rc;

use nova_sim::{RuleKey, RuleSource, Rulebook};

use crate::color::Color;
use crate::draw::{DrawList, fill_rect};
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

/// Where a dialog goes on the screen: moved by `_CenterWindow` for the
/// `DLOG`s in [`CENTER_WINDOW_DLOGS`], else as its `DLOG`'s positioning
/// word asks the Window Manager to place it ([`Placement::for_dlog`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Placement {
    /// Centred on the screen by the Window Manager, from a `DLOG`
    /// positioning word (`0x280A`, `0x680A`, `0xA80A`) on a dialog no
    /// `_CenterWindow` call moves: the spare space halved and floored in
    /// one step. Over a widened view this is floored in the 1024 x 768
    /// frame, which is then centred; a single floor over the whole width
    /// would differ by a pixel when both widths are odd. That rule is the
    /// system's, not `EV Nova.app`'s, so it can't be read from the binary.
    Center,
    /// Centred on `_gWind` by `_CenterWindow`, as [`center_window`]
    /// places it: half the window less half the dialog, each truncated.
    /// For an odd size it sits a pixel right of or below
    /// [`Placement::Center`].
    CenterWindow,
    /// Centred across, with a third of the spare height above it: the
    /// Window Manager's alert position.
    Alert,
    /// At its `DLOG` bounds' own top-left corner.
    Fixed,
}

/// The `DLOG`s the original moves with `_CenterWindow` (@0x220e-0x2360)
/// onto `_gWind` after its `GetNewDialog` and before `ShowWindow`. Its
/// 24 callers, by the `_CenterWindow` call:
///
/// | call | function | `DLOG`(s) |
/// |---|---|---|
/// | 0x11cf1 | `_DoSystemSearch` | 3005 |
/// | 0x12534 | `_DoSystemMap` | 2000 |
/// | 0x166ed | `_DoNewPilotNameDialog` | 3101, 3102 |
/// | 0x47ff3 | `_DoNewsDialog` | 1014 |
/// | 0x48aaa | `_DoRaceDialog` | 1023 |
/// | 0x49064 | `_DoBarDialog` | 1021, 1013 |
/// | 0x55ba2 | `_PlayQTMovie` | none: its `NewCWindow` movie window |
/// | 0x56dbe | `_DoQuantityDialog` | 1003 |
/// | 0x5a50c | `_DoPlayerInfoDialog` | 1017, after growing it (`SizeWindow` @0x5a313) |
/// | 0x5ae16 | `_ShowTextDialog` | 3004, 3003 (after shrinking 3003, `SizeWindow` @0x5ab4c) |
/// | 0x5b5a1 | `_DoCapturedShipAssignmentDialog` | 1018 |
/// | 0x5b7ab | `_DoShipyardDescription` | 1019, 1005 |
/// | 0x5bcba | `_DoOutfitDialog` | 1002 |
/// | 0x5df10 | `_DoTradeDialog` | 1001 |
/// | 0x5e7d9 | `_DoShipyardDialog` | 1004 |
/// | 0x5faaa | `_DoPortDialog` | 1000 |
/// | 0x91f05 | `_DoHaggleDialog` | 1008 |
/// | 0x934a4 | `_DoPlunderDialog` | 1011 |
/// | 0x952df | `_DoEscortDialog` | 1022 |
/// | 0x95ee8 | `_DoCommDialog` | 1007 |
/// | 0x96eaf | `_DoPlanetCommDialog` | 1009 |
/// | 0x9e243 | `_DoMissionInfoDialog` | 1012 |
/// | 0xa23fd | `_OfferOneMission` | 1020, 1016 |
/// | 0xa39de | `_DoMissionDialog` | 1006 |
///
/// The other `GetNewDialog` callers (`_EVNoteAlert`'s 3000,
/// `_EVTextInputDialog`'s 3001, `_EVYesNoDialog`'s 3002) only
/// `ShowWindow` what they load, so the Window Manager places those from
/// their word. Our Preferences dialog (4003) is Cocoa in this build and
/// keeps its word's placement too.
pub const CENTER_WINDOW_DLOGS: [i16; 28] = [
    1000, 1001, 1002, 1003, 1004, 1005, 1006, 1007, 1008, 1009, 1011, 1012, 1013, 1014, 1016, 1017,
    1018, 1019, 1020, 1021, 1022, 1023, 2000, 3003, 3004, 3005, 3101, 3102,
];

/// The positioning word every stock `DLOG` carries (0xA80A, centred on
/// the main screen). A built-in template, used when the interface file
/// has no usable `DLOG`, is placed with [`Placement::for_dlog`] from its
/// `DLOG`'s id and this word, so it lands where the loaded one would.
pub const STOCK_WORD: u16 = 0xA80A;

impl Placement {
    /// Where `DLOG` `id` with positioning `word` goes:
    /// [`Placement::CenterWindow`] for a `DLOG` in
    /// [`CENTER_WINDOW_DLOGS`], whatever its word, since `_CenterWindow`
    /// moves the window after it is placed; else as
    /// [`Placement::from_word`] reads the word.
    #[must_use]
    pub fn for_dlog(id: i16, word: Option<u16>) -> Self {
        if CENTER_WINDOW_DLOGS.contains(&id) {
            Self::CenterWindow
        } else {
            Self::from_word(word)
        }
    }

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
    /// [`SCREEN`], in whole units.
    #[must_use]
    pub fn origin(self, bounds: Bounds) -> Point {
        let spare = (SCREEN.0 - bounds.width(), SCREEN.1 - bounds.height());
        match self {
            Self::Center => Point::new((spare.0 / 2.0).floor(), (spare.1 / 2.0).floor()),
            Self::CenterWindow => center_window(SCREEN, (bounds.width(), bounds.height())),
            Self::Alert => Point::new((spare.0 / 2.0).floor(), (spare.1 / 3.0).floor()),
            Self::Fixed => bounds.min,
        }
    }
}

/// Where `_CenterWindow` (@0x220e-0x2359 in `EV Nova.app`) puts the
/// top-left corner of a dialog `size` wide and high in a `window`: the
/// window's centre less half the dialog, each half truncated toward zero
/// as the original's integer halving is. For an odd size it sits a pixel
/// right of or below [`Placement::Center`], which floors the spare space
/// in one step. Every `DLOG` in [`CENTER_WINDOW_DLOGS`] is placed so,
/// through [`Placement::CenterWindow`]; the galaxy map's dialog (`DLOG`
/// 2000) calls it directly. Placing in the 1024 x 768 frame and then
/// centring that frame in a widened view is exact: trunc(W / 2) −
/// trunc(w / 2) = (512 − trunc(w / 2)) + floor((W − 1024) / 2) for every
/// W ≥ 1024.
#[must_use]
pub fn center_window(window: (f32, f32), size: (f32, f32)) -> Point {
    let half = |length: f32| (length / 2.0).trunc();
    Point::new(half(window.0) - half(size.0), half(window.1) - half(size.1))
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
    /// A picture item whose `PICT` the interface file does not hold, as
    /// stock "Desc Dialog"'s item 2 names `PICT` 1431: laid out and
    /// hit-tested, but drawing nothing, as `DrawPicture` of a nil handle
    /// draws nothing. The absence is the original's normal behaviour, not
    /// a fault, so it is never reported.
    MissingPicture(i16),
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
        /// A filled box drawn under it, with its text inset, or `None` for
        /// text straight on whatever is below.
        backing: Option<TextBacking>,
    },
    /// Not there at all: not drawn, not hit-tested and never focused, as
    /// if it were outside the dialog. Any kind of item can be hidden.
    Hidden,
    /// Shown greyed and never activated: a button draws disabled. Any
    /// kind of item can be greyed.
    Greyed,
}

/// A filled box under a scrolling text: the box the original paints or
/// inverts behind a dialog's text item, white text on black.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextBacking {
    /// The box's colour.
    pub fill: Color,
    /// How far inside the item the box is, on every side.
    pub fill_inset: f32,
    /// How far inside the item the text is set: across, then down, on
    /// both sides of each axis, as `InsetRect` takes them.
    pub text_inset: (f32, f32),
}

impl TextBacking {
    /// A black box filling the whole item, the text set over all of it:
    /// `TETextBox` black on white, then `InvertRect` over the item.
    pub const BLACK_BOX: Self = Self {
        fill: Color::BLACK,
        fill_inset: 0.0,
        text_inset: (0.0, 0.0),
    };
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
    /// Wrapped text; it scrolls when `scrolls`, over a box filled in a
    /// colour when `fill` holds one.
    Text {
        text: ScrollText,
        scrolls: bool,
        fill: Option<(Bounds, Color)>,
    },
    /// A picture stretched over the item.
    Picture(ImageKey),
    /// Laid out and hit-tested, but not drawn.
    Blank,
}

#[derive(Clone, Debug, PartialEq)]
struct Item {
    /// Where it is on the screen.
    bounds: Bounds,
    enabled: bool,
    /// Whether the template enables it: greying it can be undone only
    /// back to this.
    template_enabled: bool,
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

/// Whether a dialog's buttons take a keyboard focus: the `dialog_focus_ring`
/// rule's two readings ([`RuleKey::DialogFocusRing`]).
///
/// The original has none in any in-game dialog: `_MyModalDialog`
/// (@0x59954) draws no focus, and no filter (`_TextDialogFilter` @0x51f77,
/// `_CommFilter` @0x908ed, `_PlanetCommFilter` @0x911a9, `_HaggleFilter`
/// @0x91d9d, `_PlunderFilter` @0x92f20, `_CapturedAssignmentFilter`
/// @0x51782, `_EscortFilter` @0x950e9, nor the Toolbox's standard filter)
/// maps Tab or Space to a button. The focus ring is kept on by default
/// for accessibility, by the user's decision of 2026-10-28.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FocusRing {
    /// Tab moves an outlined focus through the enabled buttons, and Space
    /// activates the focused one: the default.
    #[default]
    Tab,
    /// No focus: Tab and Space do nothing unless the dialog's own filter
    /// maps them, as the original's dialogs have it.
    Off,
}

impl FocusRing {
    /// The reading `rules` gives `dialog_focus_ring`: the other reading's
    /// [`FocusRing::Tab`] (the default), or the engine's [`FocusRing::Off`].
    #[must_use]
    pub fn from_rulebook(rules: &Rulebook) -> Self {
        match rules.source_for(RuleKey::DialogFocusRing) {
            RuleSource::Bible => Self::Tab,
            RuleSource::Engine => Self::Off,
        }
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
    /// Whether Tab and Space move and press a focus.
    focus_ring: FocusRing,
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
    /// Standard buttons become buttons with their titles, static text is
    /// wrapped in Geneva, and a picture draws its interface-file `PICT`
    /// ([`ImageKey::interface_picture`]) stretched over its rectangle, as
    /// `DrawDialog` `DrawPicture`s each picture item into its rect; a
    /// greyed picture draws the same, as the Dialog Manager doesn't dim
    /// pictures. [`Role::Hidden`] hides an item of any kind, and
    /// [`Role::Greyed`] greys one. Pictures the interface file lacks
    /// ([`ItemSpec::MissingPicture`]), check boxes, radio buttons, edit
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
                let shown = role != Some(&Role::Hidden)
                    && local.max.x > 0.0
                    && local.min.x < width
                    && local.max.y > 0.0
                    && local.min.y < height;
                let enabled = item.enabled && role != Some(&Role::Greyed);
                let bounds = local.offset(origin);
                Item {
                    bounds,
                    enabled,
                    template_enabled: item.enabled,
                    shown,
                    widget: widget(&item.kind, role, bounds, enabled, &metrics),
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
            focus_ring: FocusRing::default(),
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

    /// The dialog with its buttons taking the keyboard focus as `ring`
    /// says.
    #[must_use]
    pub fn with_focus_ring(mut self, ring: FocusRing) -> Self {
        self.set_focus_ring(ring);
        self
    }

    /// Makes the buttons take the keyboard focus as `ring` says. Turning
    /// the ring off drops any focus.
    pub fn set_focus_ring(&mut self, ring: FocusRing) {
        self.focus_ring = ring;
        if ring == FocusRing::Off {
            self.focus = None;
        }
    }

    /// Whether the buttons take the keyboard focus.
    #[must_use]
    pub fn focus_ring(&self) -> FocusRing {
        self.focus_ring
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

    /// Greys item `item` (from 1), as [`Role::Greyed`] does, or ungreys it
    /// back to what its template says. No such item, nothing changes.
    pub fn set_greyed(&mut self, item: usize, greyed: bool) {
        let Some(found) = item
            .checked_sub(1)
            .and_then(|index| self.items.get_mut(index))
        else {
            return;
        };
        found.enabled = found.template_enabled && !greyed;
        if let Widget::Button(button) = &mut found.widget {
            button.enabled = found.enabled;
        }
    }

    /// Replaces the text of item `item` (from 1) with `text`, wrapped in
    /// its font, size, colour and box and shown from the first line. An
    /// item that is not text, or no such item, changes nothing.
    pub fn set_text(&mut self, item: usize, text: &str) {
        let Some(found) = item
            .checked_sub(1)
            .and_then(|index| self.items.get_mut(index))
        else {
            return;
        };
        if let Widget::Text { text: shown, .. } = &mut found.widget {
            *shown = shown.rewrapped(text, &self.metrics);
        }
    }

    /// The text of item `item` (from 1), if it is a text item.
    #[must_use]
    pub fn text(&self, item: usize) -> Option<&ScrollText> {
        match &self.item(item)?.widget {
            Widget::Text { text, .. } => Some(text),
            _ => None,
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

    /// Draws `image` stretched over the whole dialog, as `DrawPicHandle`
    /// fills a picture dialog's port with its frame.
    pub fn draw_stretched_over(&self, list: &mut DrawList, image: ImageKey) {
        let bounds = self.bounds;
        list.stretched_picture(image, bounds.min, bounds.width(), bounds.height());
    }

    /// Draws `PICT` `graphic` stretched into item `item` (from 1); nothing
    /// when the dialog has no such item.
    pub fn draw_picture_in(&self, list: &mut DrawList, item: usize, graphic: i16) {
        if let Some(at) = self.item_bounds(item) {
            list.stretched_picture(ImageKey::picture(graphic), at.min, at.width(), at.height());
        }
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
                ..
            } => Some(text),
            _ => None,
        })
    }

    /// The first scrolling text, if any, to scroll: the one
    /// [`Self::scroll_text`] shows and Up and Down move.
    pub(crate) fn scroll_text_mut(&mut self) -> Option<&mut ScrollText> {
        self.items
            .iter_mut()
            .find_map(|item| match &mut item.widget {
                Widget::Text {
                    text,
                    scrolls: true,
                    ..
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
    /// - Under [`FocusRing::Tab`], the default, Tab moves the focus to the
    ///   next enabled button, and Space activates the focused one. Under
    ///   [`FocusRing::Off`], the original's reading, they do nothing.
    /// - Up and Down, repeats included, scroll the first scrolling text.
    /// - A wheel or trackpad scroll inside the dialog scrolls the scrolling
    ///   text under the pointer, if any; static text ignores it, and a
    ///   click in progress stays armed.
    ///
    /// Key repeats never activate anything.
    pub fn input(&mut self, input: &Input) -> Option<DialogEvent> {
        match *input {
            Input::Key {
                key,
                pressed: true,
                repeat,
            } => self.key(key, repeat),
            Input::Scroll { at, .. } => {
                if self.bounds.contains(at) {
                    self.items.iter_mut().any(|item| match &mut item.widget {
                        Widget::Text {
                            text,
                            scrolls: true,
                            ..
                        } => text.input(input),
                        _ => false,
                    });
                }
                None
            }
            Input::Key { .. } | Input::Text(_) | Input::PointerLeft => None,
            Input::PointerButton { .. } | Input::PointerMoved(_) => self.pointer(input),
        }
    }

    fn key(&mut self, key: Key, repeat: bool) -> Option<DialogEvent> {
        match key {
            Key::Up | Key::Down => {
                if let Some(text) = self.scroll_text_mut() {
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
            Key::Tab if self.focus_ring == FocusRing::Tab => {
                self.focus_next();
                None
            }
            // Under `FocusRing::Off` nothing is ever focused, so Space
            // activates nothing.
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
                .position(|item| item.active() && self.hit_area(item).contains(at));
        }
        let index = self.tracked?;
        let item = &self.items[index];
        let activated = self
            .tracker
            .track(self.hit_area(item), item.active(), input);
        // Silent under the pointer, as every one of the original's
        // in-game and spaceport button trackers (`_TrackTextButtons`
        // @0x26bb0 and its kin) plays nothing: only the main screen's
        // `_TrackMainScreenButtons` @0x195b9 clicks.
        let _ = self.tracker.take_sound();
        if !self.tracker.armed() {
            self.tracked = None;
        }
        activated.then_some(DialogEvent::Item(index + 1))
    }

    /// Where `item` takes clicks: the part of it inside the dialog, since
    /// pointer events outside the dialog do nothing.
    fn hit_area(&self, item: &Item) -> Bounds {
        let (outer, inner) = (self.bounds, item.bounds);
        Bounds {
            min: Point::new(inner.min.x.max(outer.min.x), inner.min.y.max(outer.min.y)),
            max: Point::new(inner.max.x.min(outer.max.x), inner.max.y.min(outer.max.y)),
        }
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
                Widget::Text { text, fill, .. } => {
                    if let Some((area, color)) = fill {
                        fill_rect(list, *area, *color);
                    }
                    text.draw(list);
                }
                Widget::Picture(image) => {
                    let at = item.bounds;
                    list.stretched_picture(*image, at.min, at.width(), at.height());
                }
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
        (ItemSpec::Picture(id), _) => Widget::Picture(ImageKey::interface_picture(*id)),
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
            fill: None,
        },
        (ItemSpec::User, Some(Role::Button(label))) => button(label),
        (
            ItemSpec::User,
            Some(Role::ScrollText {
                text,
                font,
                size,
                color,
                backing,
            }),
        ) => {
            let (text_rect, fill) = match backing {
                None => (bounds, None),
                Some(backing) => (
                    bounds.inset(backing.text_inset.0, backing.text_inset.1),
                    Some((bounds.grown(-backing.fill_inset), backing.fill)),
                ),
            };
            Widget::Text {
                text: ScrollText::new(text, *font, *size, *color, text_rect, metrics),
                scrolls: true,
                fill,
            }
        }
        _ => Widget::Blank,
    }
}

/// A 1-unit outline just inside `rect`.
pub(crate) fn outline(list: &mut DrawList, rect: Bounds, color: Color) {
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
    use crate::draw::{DrawCommand, fill_command};
    use crate::input::{MouseButton, ScrollDelta};
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

    #[test]
    fn center_window_takes_half_the_window_less_half_the_dialog_each_truncated() {
        // The map's DLOG 2000, 601 x 513: (512 − 300, 384 − 256).
        assert_eq!(center_window(SCREEN, (601.0, 513.0)), at(212.0, 128.0));
        let bounds = rect(0.0, 0.0, 601.0, 513.0);
        assert_eq!(
            Placement::Center.origin(bounds),
            at(211.0, 127.0),
            "a pixel up and left"
        );
        // Even sizes agree with the Window Manager's centring.
        assert_eq!(center_window(SCREEN, (600.0, 512.0)), at(212.0, 128.0));
        // An odd window too: trunc(1023 / 2) − trunc(601 / 2).
        assert_eq!(
            center_window((1023.0, 767.0), (601.0, 513.0)),
            at(211.0, 127.0)
        );
        // A dialog wider than the window sits left of it.
        assert_eq!(
            center_window((100.0, 100.0), (201.0, 101.0)),
            at(-50.0, 0.0)
        );
    }

    #[test]
    fn center_window_dlogs_place_as_center_window_does() {
        // The port dialog, DLOG 1000, 618 x 517: an odd height.
        let port = Placement::for_dlog(1000, Some(0xA80A));
        assert_eq!(port, Placement::CenterWindow);
        let bounds = rect(0.0, 0.0, 618.0, 517.0);
        assert_eq!(port.origin(bounds), at(203.0, 126.0));
        assert_eq!(Placement::Center.origin(bounds), at(203.0, 125.0));
        // The mission offer, DLOG 1020, 651 x 244: an odd width.
        let offer = Placement::for_dlog(1020, Some(0xA80A));
        assert_eq!(offer, Placement::CenterWindow);
        let bounds = rect(0.0, 0.0, 651.0, 244.0);
        assert_eq!(offer.origin(bounds), at(187.0, 262.0));
        assert_eq!(Placement::Center.origin(bounds), at(186.0, 262.0));
    }

    #[test]
    fn center_window_moves_a_listed_dlog_whatever_its_word() {
        for word in [None, Some(0), Some(0x300A), Some(0xA80A)] {
            assert_eq!(
                Placement::for_dlog(1000, word),
                Placement::CenterWindow,
                "{word:?}"
            );
        }
    }

    #[test]
    fn a_dlog_no_caller_moves_keeps_the_window_managers_centring() {
        let yes_no = Placement::for_dlog(3002, Some(0xA80A));
        assert_eq!(yes_no, Placement::Center);
        assert_eq!(
            yes_no.origin(rect(0.0, 0.0, 355.0, 109.0)),
            at(334.0, 329.0)
        );
        assert_eq!(
            center_window(SCREEN, (355.0, 109.0)),
            at(335.0, 330.0),
            "_CenterWindow would put it a pixel lower and right"
        );
        let note = Placement::for_dlog(3000, Some(0xA80A));
        assert_eq!(note.origin(rect(0.0, 0.0, 357.0, 150.0)), at(333.0, 309.0));
        assert_eq!(Placement::for_dlog(128, None), Placement::Fixed);
        assert_eq!(Placement::for_dlog(128, Some(0x300A)), Placement::Alert);
    }

    #[test]
    fn every_listed_dlog_and_none_other_is_moved_by_center_window() {
        for id in CENTER_WINDOW_DLOGS {
            assert_eq!(
                Placement::for_dlog(id, Some(0xA80A)),
                Placement::CenterWindow,
                "DLOG {id}"
            );
        }
        for id in [3000, 3001, 3002, 4003, 1010, 1015, 1024, 3006, 3100] {
            assert_eq!(
                Placement::for_dlog(id, Some(0xA80A)),
                Placement::Center,
                "DLOG {id}"
            );
        }
    }

    #[test]
    fn a_center_window_dialog_places_its_items_from_center_windows_corner() {
        let template = DialogTemplate {
            bounds: rect(30.0, 40.0, 618.0, 517.0),
            placement: Placement::CenterWindow,
            items: vec![item(
                rect(10.0, 20.0, 70.0, 20.0),
                true,
                ItemSpec::Button("OK".into()),
            )],
        };
        let dialog = dialog(&template, &[]);
        assert_eq!(dialog.bounds(), rect(203.0, 126.0, 618.0, 517.0));
        assert_eq!(dialog.item_bounds(1), Some(rect(213.0, 146.0, 70.0, 20.0)));
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

    #[test]
    fn draw_stretched_over_fills_the_dialog_bounds_with_the_image() {
        let dialog = dialog(&yes_no(), &[]);
        let mut list = DrawList::new();
        dialog.draw_stretched_over(&mut list, ImageKey::picture(8527));
        let drawn: Vec<DrawCommand> = list.iter().cloned().collect();
        assert_eq!(
            drawn,
            vec![DrawCommand::StretchedPicture {
                image: ImageKey::picture(8527),
                top_left: YES_NO,
                width: 355.0,
                height: 109.0,
            }]
        );
    }

    #[test]
    fn draw_picture_in_stretches_the_graphic_into_the_item() {
        let dialog = dialog(&yes_no(), &[]);
        let mut list = DrawList::new();
        dialog.draw_picture_in(&mut list, 4, 5008);
        let drawn: Vec<DrawCommand> = list.iter().cloned().collect();
        assert_eq!(
            drawn,
            vec![DrawCommand::StretchedPicture {
                image: ImageKey::picture(5008),
                top_left: Point::new(YES_NO.x + 7.0, YES_NO.y + 5.0),
                width: 32.0,
                height: 32.0,
            }]
        );
    }

    #[test]
    fn draw_picture_in_an_absent_item_draws_nothing() {
        let dialog = dialog(&yes_no(), &[]);
        let mut list = DrawList::new();
        dialog.draw_picture_in(&mut list, 6, 5008);
        assert_eq!(list.iter().count(), 0);
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
                interface_picture(130, rect(334.0 + 7.0, 329.0 + 5.0, 32.0, 32.0)),
                picture(normal.left, cancel.min.x, 13.0),
                picture(normal.middle, cancel.min.x + 13.0, 44.0),
                picture(normal.right, cancel.max.x - 13.0, 13.0),
                label("Cancel", cancel),
            ]
        );
    }

    /// A 100 x 50 dialog at the origin holding `items`.
    fn fixed(items: Vec<ItemTemplate>) -> DialogTemplate {
        DialogTemplate {
            bounds: rect(0.0, 0.0, 100.0, 50.0),
            placement: Placement::Fixed,
            items,
        }
    }

    /// The interface file's `PICT` `id` stretched over `bounds`.
    fn interface_picture(id: i16, bounds: Bounds) -> DrawCommand {
        DrawCommand::StretchedPicture {
            image: ImageKey::interface_picture(id),
            top_left: bounds.min,
            width: bounds.width(),
            height: bounds.height(),
        }
    }

    #[test]
    fn a_picture_item_draws_its_interface_pict_stretched_over_it() {
        let at = rect(7.0, 5.0, 40.0, 20.0);
        let dialog = dialog(&fixed(vec![item(at, false, ItemSpec::Picture(130))]), &[]);
        assert_eq!(drawn(&dialog), [interface_picture(130, at)]);
    }

    #[test]
    fn pictures_draw_in_item_order_among_the_other_items() {
        let (first, text, last) = (
            rect(0.0, 0.0, 10.0, 10.0),
            rect(20.0, 0.0, 30.0, 20.0),
            rect(60.0, 0.0, 10.0, 10.0),
        );
        let dialog = dialog(
            &fixed(vec![
                item(first, false, ItemSpec::Picture(129)),
                item(text, false, ItemSpec::StaticText("Hi".into())),
                item(last, false, ItemSpec::Picture(130)),
            ]),
            &[],
        );
        let commands = drawn(&dialog);
        assert_eq!(commands.len(), 3);
        assert_eq!(commands[0], interface_picture(129, first));
        assert!(matches!(commands[1], DrawCommand::Text { .. }));
        assert_eq!(commands[2], interface_picture(130, last));
    }

    #[test]
    fn hidden_and_parked_pictures_draw_nothing() {
        let parked = dialog(&yes_no(), &[(4, Role::Hidden)]);
        assert!(
            !drawn(&parked).iter().any(
                |command| matches!(command, DrawCommand::StretchedPicture { image, .. }
                    if image.kind == crate::ImageKind::InterfacePict)
            ),
            "item 2 is parked outside, item 4 hidden"
        );
    }

    #[test]
    fn a_greyed_picture_still_draws() {
        let at = rect(7.0, 5.0, 40.0, 20.0);
        let dialog = dialog(
            &fixed(vec![item(at, true, ItemSpec::Picture(130))]),
            &[(1, Role::Greyed)],
        );
        assert_eq!(drawn(&dialog), [interface_picture(130, at)]);
    }

    #[test]
    fn a_picture_the_interface_file_lacks_is_laid_out_but_draws_nothing() {
        let at = rect(7.0, 5.0, 40.0, 20.0);
        let mut dialog = dialog(
            &fixed(vec![item(at, true, ItemSpec::MissingPicture(1431))]),
            &[],
        );
        assert_eq!(drawn(&dialog), []);
        assert!(dialog.item_shown(1));
        assert_eq!(dialog.item_bounds(1), Some(at));
        assert_eq!(click(&mut dialog, at.center()), Some(DialogEvent::Item(1)));
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
    fn only_the_part_of_an_item_inside_the_dialog_takes_clicks() {
        let items = [
            rect(-9.0, 0.0, 10.0, 10.0),
            rect(99.0, 0.0, 10.0, 10.0),
            rect(0.0, -9.0, 10.0, 10.0),
            rect(0.0, 49.0, 10.0, 10.0),
        ]
        .iter()
        .map(|&bounds| item(bounds, true, ItemSpec::User))
        .collect();
        let template = DialogTemplate {
            bounds: rect(0.0, 0.0, 100.0, 50.0),
            placement: Placement::Fixed,
            items,
        };
        let mut dialog = dialog(&template, &[]);
        // For each item: a point on it outside the dialog, then one inside.
        let points = [
            (at(-5.0, 5.0), at(0.5, 5.0)),
            (at(105.0, 5.0), at(99.5, 5.0)),
            (at(5.0, -5.0), at(5.0, 0.5)),
            (at(5.0, 55.0), at(5.0, 49.5)),
        ];
        for (n, (outside, inside)) in (1..).zip(points) {
            assert_eq!(click(&mut dialog, outside), None, "item {n} at {outside:?}");
            assert_eq!(
                click(&mut dialog, inside),
                Some(DialogEvent::Item(n)),
                "item {n} at {inside:?}"
            );
            dialog.input(&press(inside));
            assert_eq!(
                dialog.input(&release(outside)),
                None,
                "item {n} released at {outside:?}"
            );
        }
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
        // Cancel draws normally while OK is held (after OK's four
        // commands, the text and the picture).
        let commands = drawn(&dialog);
        assert!(matches!(
            commands[6],
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
    fn typed_text_activates_nothing() {
        let mut dialog = dialog(&yes_no(), &[]).with_cancel(Some(5));
        for c in ['\r', ' ', 'o', '\u{1b}'] {
            assert_eq!(dialog.input(&Input::Text(c)), None, "{c:?}");
        }
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
                    backing: None,
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
    fn a_text_items_text_is_read_by_its_number_and_any_other_has_none() {
        let mut dialog = dialog(&desc_like(), &desc_roles("l0"));
        dialog.set_text(3, "new");
        assert_eq!(
            dialog.text(3).map(ScrollText::lines),
            Some(&["new".to_owned()][..])
        );
        for other in [0, 1, 2, 4, 99] {
            assert_eq!(dialog.text(other), None, "item {other}");
        }
    }

    #[test]
    fn set_text_rewraps_a_text_item_and_nothing_else() {
        let mut dialog = dialog(&desc_like(), &desc_roles("l0\rl1\rl2\rl3\rl4"));
        dialog.input(&key(Key::Down));
        dialog.set_text(3, "new");
        let text = dialog.scroll_text().expect("scrolling text");
        assert_eq!((text.lines(), text.first()), (&["new".to_owned()][..], 0));
        assert_eq!(text.rect(), rect(10.0, 10.0, 40.0, 36.0));
        let before = drawn(&dialog);
        for other in [0, 1, 2, 4, 99] {
            dialog.set_text(other, "other");
        }
        assert_eq!(drawn(&dialog), before, "only text items change");
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

    fn wheel(at: Point) -> Input {
        Input::Scroll {
            delta: ScrollDelta::Lines(1.0),
            at,
        }
    }

    /// `desc_like` with item 4 a second scrolling text, at `x`.
    fn two_texts(x: f32) -> Dialog {
        let mut template = desc_like();
        template.items[3].bounds = rect(x, 10.0, 23.0, 23.0);
        let mut roles = desc_roles("l0\rl1\rl2\rl3\rl4");
        roles.push((
            4,
            Role::ScrollText {
                text: "m0\rm1\rm2".into(),
                font: Font::Geneva,
                size: 10.0,
                color: Color::WHITE,
                backing: None,
            },
        ));
        dialog(&template, &roles)
    }

    fn first(dialog: &Dialog, item: usize) -> usize {
        dialog.text(item).expect("text").first()
    }

    #[test]
    fn a_wheel_over_the_scrolling_text_scrolls_it() {
        let mut dialog = dialog(&desc_like(), &desc_roles("l0\rl1\rl2\rl3\rl4"));
        assert_eq!(dialog.input(&wheel(at(20.0, 20.0))), None);
        assert_eq!(first(&dialog, 3), 1);
    }

    #[test]
    fn a_wheel_over_a_button_or_outside_the_dialog_does_nothing() {
        let mut dialog = dialog(&desc_like(), &desc_roles("l0\rl1\rl2\rl3\rl4"));
        for point in [at(150.0, 80.0), at(300.0, 300.0)] {
            assert_eq!(dialog.input(&wheel(point)), None, "{point:?}");
        }
        assert_eq!(first(&dialog, 3), 0);
    }

    #[test]
    fn a_wheel_over_static_text_does_nothing() {
        let mut template = yes_no();
        template.items[2].kind = ItemSpec::StaticText("a b c d e f g h i j k l m".repeat(20));
        let mut dialog = dialog(&template, &[]);
        let before = drawn(&dialog);
        let over = at(YES_NO.x + 60.0, YES_NO.y + 10.0);
        assert_eq!(dialog.input(&wheel(over)), None);
        assert_eq!(drawn(&dialog), before);
    }

    #[test]
    fn a_wheel_scrolls_only_the_text_under_the_pointer() {
        let mut dialog = two_texts(150.0);
        dialog.input(&wheel(at(160.0, 20.0)));
        assert_eq!((first(&dialog, 3), first(&dialog, 4)), (0, 1));
        dialog.input(&wheel(at(20.0, 20.0)));
        assert_eq!((first(&dialog, 3), first(&dialog, 4)), (1, 1));
        dialog.input(&key(Key::Down));
        assert_eq!(
            (first(&dialog, 3), first(&dialog, 4)),
            (2, 1),
            "the arrow keys drive the first"
        );
    }

    #[test]
    fn a_wheel_over_a_text_but_outside_the_dialog_does_nothing() {
        // Item 4 straddles the dialog's right edge, at 200.
        let mut dialog = two_texts(190.0);
        assert!(dialog.item_shown(4));
        dialog.input(&wheel(at(205.0, 20.0)));
        assert_eq!(first(&dialog, 4), 0);
        dialog.input(&wheel(at(195.0, 20.0)));
        assert_eq!(first(&dialog, 4), 1);
    }

    #[test]
    fn a_wheel_during_a_click_leaves_it_armed() {
        let mut dialog = dialog(&desc_like(), &desc_roles("l0\rl1\rl2\rl3\rl4"));
        let button = at(150.0, 80.0);
        assert_eq!(dialog.input(&press(button)), None);
        dialog.input(&wheel(at(20.0, 20.0)));
        assert_eq!(first(&dialog, 3), 1);
        assert_eq!(dialog.input(&release(button)), Some(DialogEvent::Item(1)));
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

    /// `desc_like`'s roles, its text item (3) backed by `backing`.
    fn backed_roles(text: &str, backing: TextBacking) -> Vec<(usize, Role)> {
        let mut roles = desc_roles(text);
        if let Role::ScrollText { backing: slot, .. } = &mut roles[1].1 {
            *slot = Some(backing);
        }
        roles
    }

    /// A box filled red, inset 1, its text inset (5, 3).
    const INSET: TextBacking = TextBacking {
        fill: Color::rgba(255, 0, 0, 255),
        fill_inset: 1.0,
        text_inset: (5.0, 3.0),
    };

    #[test]
    fn a_backed_text_fills_its_box_before_its_text_runs() {
        let dialog = dialog(
            &desc_like(),
            &backed_roles("l0\rl1", TextBacking::BLACK_BOX),
        );
        let commands = drawn(&dialog);
        // The button's three pictures and label, the fill, then the lines.
        assert_eq!(
            commands[4],
            fill_command(rect(10.0, 10.0, 40.0, 36.0), Color::BLACK)
        );
        assert!(
            matches!(&commands[5], DrawCommand::Text { text, .. } if text == "l0"),
            "{commands:#?}"
        );
        assert_eq!(commands.len(), 7);
    }

    #[test]
    fn a_backed_text_is_wrapped_and_placed_in_its_inset_box() {
        // Item 3 is 40 x 36 at (10, 10): inset (5, 3) it is 30 x 30 at
        // (15, 13), 6 Geneva-10 characters a line and 2 lines of 12.
        let mut dialog = dialog(&desc_like(), &backed_roles("abcdef gh\rl1\rl2", INSET));
        let commands = drawn(&dialog);
        assert_eq!(
            commands[4],
            fill_command(rect(11.0, 11.0, 38.0, 34.0), INSET.fill)
        );
        assert!(
            matches!(
                &commands[5],
                DrawCommand::Text { text, origin, .. }
                    if text == "abcdef" && *origin == at(15.0, 13.0)
            ),
            "{commands:#?}"
        );
        let text = dialog.text(3).expect("text");
        assert_eq!((text.visible(), text.lines().len()), (2, 4));
        dialog.set_text(3, "abcdefg");
        let text = dialog.text(3).expect("text");
        assert_eq!(
            text.rect(),
            rect(15.0, 13.0, 30.0, 30.0),
            "kept on set_text"
        );
        assert_eq!(text.lines(), ["abcdef", "g"]);
    }

    #[test]
    fn an_unbacked_text_draws_no_fill() {
        let dialog = dialog(&desc_like(), &desc_roles("l0"));
        assert!(
            !drawn(&dialog)
                .iter()
                .any(|command| matches!(command, DrawCommand::Line { .. })),
        );
    }

    #[test]
    fn a_backed_text_outside_the_dialog_draws_nothing() {
        let mut template = desc_like();
        template.items[2].bounds = rect(10.0, 150.0, 40.0, 36.0);
        let dialog = dialog(&template, &backed_roles("l0", TextBacking::BLACK_BOX));
        // Only the button's pictures and label.
        assert_eq!(drawn(&dialog).len(), 4);
    }

    #[test]
    fn up_and_down_still_scroll_a_backed_text() {
        let mut dialog = dialog(&desc_like(), &backed_roles("l0\rl1\rl2\rl3\rl4", INSET));
        dialog.input(&key(Key::Down));
        assert_eq!(dialog.text(3).expect("text").first(), 1);
        dialog.input(&key(Key::Up));
        assert_eq!(dialog.text(3).expect("text").first(), 0);
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
            ItemSpec::MissingPicture(5),
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
    fn a_hidden_item_of_any_kind_is_not_drawn_hit_or_focused() {
        let mut template = yes_no();
        template.items[3].enabled = true;
        let roles = [(1, Role::Hidden), (3, Role::Hidden), (4, Role::Hidden)];
        let mut dialog = dialog(&template, &roles);
        let shown: Vec<bool> = (1..=5).map(|n| dialog.item_shown(n)).collect();
        assert_eq!(shown, [false, false, false, false, true]);
        let labels: Vec<String> = drawn(&dialog)
            .into_iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, .. } => Some(text),
                _ => None,
            })
            .collect();
        assert_eq!(labels, ["Cancel"], "neither OK nor the text");
        let ok = dialog.item_bounds(1).expect("OK").center();
        let picture = dialog.item_bounds(4).expect("picture").center();
        assert_eq!(click(&mut dialog, ok), None);
        assert_eq!(click(&mut dialog, picture), None);
        assert_eq!(dialog.input(&key(Key::Enter)), None, "OK is the default");
        dialog.input(&key(Key::Tab));
        dialog.input(&key(Key::Tab));
        assert_eq!(dialog.focus(), Some(5), "only Cancel takes the focus");
    }

    #[test]
    fn a_greyed_button_draws_disabled_and_is_never_activated() {
        let mut dialog = dialog(&yes_no(), &[(5, Role::Greyed)]).with_cancel(Some(5));
        assert!(dialog.item_shown(5));
        let commands = drawn(&dialog);
        let disabled = ButtonSkin::NOVA.disabled;
        assert!(matches!(
            commands[6],
            DrawCommand::StretchedPicture { image, .. } if image == disabled.left
        ));
        assert!(matches!(
            &commands[9],
            DrawCommand::Text { text, color, .. }
                if text == "Cancel" && *color == ButtonStyle::STOCK.grey
        ));
        let cancel = dialog.item_bounds(5).expect("Cancel").center();
        assert_eq!(click(&mut dialog, cancel), None);
        assert_eq!(dialog.input(&key(Key::Escape)), None);
        dialog.input(&key(Key::Tab));
        dialog.input(&key(Key::Tab));
        assert_eq!(dialog.focus(), Some(1), "only OK takes the focus");
    }

    #[test]
    fn a_button_greyed_after_it_is_built_is_greyed_until_it_is_ungreyed() {
        let built = dialog(&yes_no(), &[]).with_cancel(Some(5));
        let mut greyed = built.clone();
        greyed.set_greyed(5, true);
        let from_role = dialog(&yes_no(), &[(5, Role::Greyed)]).with_cancel(Some(5));
        assert_eq!(drawn(&greyed), drawn(&from_role));
        let cancel = greyed.item_bounds(5).expect("Cancel").center();
        assert_eq!(click(&mut greyed, cancel), None);
        assert_eq!(greyed.input(&key(Key::Escape)), None);
        greyed.set_greyed(5, false);
        assert_eq!(drawn(&greyed), drawn(&built));
        assert_eq!(click(&mut greyed, cancel), Some(DialogEvent::Item(5)));
        assert_eq!(greyed.input(&key(Key::Escape)), Some(DialogEvent::Item(5)));
    }

    #[test]
    fn ungreying_never_enables_an_item_the_template_disables_and_no_item_is_nothing() {
        let mut dialog = dialog(&yes_no(), &[(1, Role::Greyed)]);
        dialog.set_greyed(1, false);
        let ok = dialog.item_bounds(1).expect("OK").center();
        assert_eq!(
            click(&mut dialog, ok),
            Some(DialogEvent::Item(1)),
            "a role is undone"
        );
        dialog.set_greyed(3, false);
        let text = dialog.item_bounds(3).expect("text").center();
        assert_eq!(click(&mut dialog, text), None, "disabled in the template");
        let before = drawn(&dialog);
        dialog.set_greyed(0, true);
        dialog.set_greyed(99, true);
        assert_eq!(drawn(&dialog), before);
    }

    #[test]
    fn a_greyed_user_item_with_no_other_role_draws_nothing_and_is_not_hit() {
        let mut dialog = dialog(&desc_like(), &[(4, Role::Greyed)]);
        assert!(drawn(&dialog).is_empty());
        let user = dialog.item_bounds(4).expect("item 4").center();
        assert_eq!(click(&mut dialog, user), None);
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

    #[test]
    fn clicking_anything_but_a_button_activates_only_user_items() {
        let mut dialog = dialog(&desc_like(), &desc_roles("text"));
        let user = dialog.item_bounds(4).expect("item 4").center();
        assert_eq!(click(&mut dialog, user), Some(DialogEvent::Item(4)));
        let text = dialog.item_bounds(3).expect("item 3").center();
        assert_eq!(click(&mut dialog, text), None, "disabled text");
        assert_eq!(click(&mut dialog, at(199.0, 1.0)), None, "empty space");
    }

    /// `_TextDialogFilter` maps Return, Enter and Escape straight to an
    /// item (@0x51fca-0x51fef).
    #[test]
    fn keys_activate_the_default_and_cancel_buttons() {
        let mut dialog = dialog(&yes_no(), &[]).with_cancel(Some(5));
        assert_eq!(dialog.input(&key(Key::Enter)), Some(DialogEvent::Item(1)));
        assert_eq!(dialog.input(&key(Key::Escape)), Some(DialogEvent::Item(5)));
        assert_eq!(dialog.input(&key(Key::Tab)), None);
        assert_eq!(dialog.input(&key(Key::Space)), Some(DialogEvent::Item(1)));
    }

    #[test]
    fn the_focus_ring_is_tab_by_default_and_follows_the_rulebook() {
        assert_eq!(FocusRing::default(), FocusRing::Tab);
        assert_eq!(
            FocusRing::from_rulebook(&Rulebook::default()),
            FocusRing::Tab
        );
        assert_eq!(
            FocusRing::from_rulebook(&Rulebook::new(RuleSource::Bible)),
            FocusRing::Tab
        );
        assert_eq!(
            FocusRing::from_rulebook(&Rulebook::new(RuleSource::Engine)),
            FocusRing::Off
        );
        assert_eq!(
            FocusRing::from_rulebook(
                &Rulebook::default().with_override(RuleKey::DialogFocusRing, RuleSource::Engine)
            ),
            FocusRing::Off
        );
        assert_eq!(dialog(&yes_no(), &[]).focus_ring(), FocusRing::Tab);
    }

    /// The original's reading: `_MyModalDialog` @0x59954 draws no focus,
    /// and no in-game filter maps Tab or Space.
    #[test]
    fn with_the_focus_ring_off_tab_and_space_do_nothing() {
        let mut dialog = dialog(&yes_no(), &[])
            .with_cancel(Some(5))
            .with_focus_ring(FocusRing::Off);
        assert_eq!(dialog.focus_ring(), FocusRing::Off);
        let before = drawn(&dialog);
        for _ in 0..3 {
            assert_eq!(dialog.input(&key(Key::Tab)), None);
            assert_eq!(dialog.focus(), None);
            assert_eq!(dialog.input(&key(Key::Space)), None);
        }
        assert_eq!(drawn(&dialog), before, "nothing is outlined");
        assert_eq!(dialog.input(&key(Key::Enter)), Some(DialogEvent::Item(1)));
        assert_eq!(dialog.input(&key(Key::Escape)), Some(DialogEvent::Item(5)));
    }

    #[test]
    fn turning_the_focus_ring_off_drops_the_focus_and_back_on_tabs_again() {
        let mut dialog = dialog(&yes_no(), &[]);
        dialog.input(&key(Key::Tab));
        assert_eq!(dialog.focus(), Some(1));
        dialog.set_focus_ring(FocusRing::Off);
        assert_eq!(dialog.focus(), None);
        assert_eq!(dialog.input(&key(Key::Space)), None);
        dialog.set_focus_ring(FocusRing::Tab);
        dialog.input(&key(Key::Tab));
        assert_eq!(dialog.input(&key(Key::Space)), Some(DialogEvent::Item(1)));
    }
}
