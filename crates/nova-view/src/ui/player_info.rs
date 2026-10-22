//! The player info screen ([`PlayerInfoScreen`], `DLOG`/`DITL` 1017
//! "Player Info", `_DoPlayerInfoDialog` @0x5a00a in the `EV Nova`
//! executable): the pilot, the ship and its cargo, extras, ranks and
//! honors, on four tabs. What it shows is the session's
//! [`PlayerInfo`], decided in `nova-sim`
//! ([`player_info`](nova_sim::player_info)).
//!
//! **Layout** (`_PlayerInfoDialogUpdate` 0x5244f). The stock dialog is
//! 413 x 227, centred: Done (item 1), the General, Cargo, Extras and
//! Honors tabs (2-5), the content box (6) and Jettison Cargo (7), each
//! labelled from `STR#` 150 (#5, #36-#39, #61). It is painted black, then
//! `PICT` 8519 between the upper cap 8518 and the lower cap 8520
//! ([`BACKGROUND`], [`draw_background`]), each at its own width and laid
//! out by its own height ([`BackgroundSizes`], read from the game data):
//! a span between the caps under the middle's own height crops it, drawn
//! at its own size and cut by the dialog under the lower cap, and a
//! taller one stretches it (@0x52581-0x525a8). A missing picture is left
//! out of the layout (@0x524a0, @0x524df, @0x5252e). Every text is Geneva 9
//! ([`TEXT_SIZE`]), the labels light grey ([`LABEL_COLOR`]) and the values
//! white. Every `STR#` 2002 string it draws, the labels, words and
//! messages, is the info's [`Words`], read with the texts, so a plug-in
//! or translation that changes an entry changes it everywhere at once;
//! the button labels are `STR#` 150's.
//!
//! - **General** ([`general_texts`]): each row a label at h +5 or +0xcd
//!   and a value at +0x50 or +0x118 from the content box's top-left, on
//!   baselines +0xc to +0x8c, 0x10 apart (@0x527b0-0x53731). Combat
//!   Rating shows [`PLACEHOLDER`] until rdm:task/combat-rating-readers
//!   lands ([`PLACEHOLDER_FIELDS`]). With the info's cheat mode on,
//!   System, Legal Status (in an inhabited system), Combat Rating and
//!   Ship Class are followed by their IDs, " (n)", in the value's white
//!   ([`cheat_suffix`], @0x52951, @0x52a16, @0x52ae3, @0x52c1d).
//! - **Cargo, Extras and Honors**: the text, wrapped in the content box
//!   inset by 5 (10 at the bottom), white on a black box (`TETextBox`
//!   then `InvertRect`, @0x53749-0x539c3), a line straddling the box's
//!   bottom clipped to it ([`TextBox`]); with nothing to show, a light
//!   grey message centred across the box on baseline +0x37
//!   (`_DrawCenteredText` @0x53a14): #268 or #269 for the cargo (by
//!   whether the fleet holds more than the ship), #270 and #271.
//! - **Growth** ([`growth`], @0x5a23d-0x5a4e7). Each text's lines are
//!   counted at the content box's full width; the tallest, `(lines + 2)
//!   x` the line height, grows the dialog by `tallest - 141 + 4` when it
//!   is over the box's 141, moving Done and Jettison down by as much and
//!   lengthening the box.
//!
//! **Buttons** (`_DrawPlayerInfoButtons` 0x27aae,
//! `_TrackPlayerInfoButtons` 0x27c4d). Done and the tabs are always
//! drawn, Jettison only on the Cargo tab with cargo that can be
//! jettisoned; the current tab and the button tracked draw pressed. A
//! click is tracked on all six rectangles, Jettison's even when hidden,
//! and chooses the button it is released on. Done closes; a tab shows its
//! tab; Jettison acts only on the Cargo tab with cargo that can be
//! jettisoned (@0x5a621-0x5a634): it asks `STR#` 2002 #291 in the Yes/No
//! dialog (`DLOG` 3002, `_EVYesNoDialog` 0x5558c), OK (item 1) yes and
//! Cancel (5) no, Return OK and Escape nothing; a yes jettisons
//! everything ([`PlayerInfoScreen::take_jettison`]) and closes the
//! screen; with no Yes/No dialog the answer is no. No button is inert
//! ([`INERT_BUTTONS`]).
//!
//! **Keys** (`_PlayerInfoFilter` 0x53ae9). Tab, a press or a repeat,
//! shows the next tab, or with Shift held the previous one, wrapping
//! (@0x53b14-0x53b80). Return, Enter, Escape and P (a press) close it
//! (@0x53b86-0x53bb6). The original closes on any key pressed while P
//! is held; P is the only one it can be held for here, as the screen
//! opens on P's press.
//!
//! **Sounds.** None: `_TrackPlayerInfoButtons` and `_DrawButton` play
//! nothing, and the Yes/No dialog is the system's, so the buttons'
//! tracking sounds are dropped. Without its dialog it says why, and
//! Return or Escape still closes it.

use std::rc::Rc;
use std::time::Duration;

use nova_sim::PlayerInfo;
use nova_sim::player_info::{EnergyShown, IncomeLine, Reach, Words, cheat_suffix};

use super::button::{Button, ButtonSkin, ButtonStyle, ButtonTracker};
use super::catalog::PictureSizes;
use super::desc::DONE_LABEL;
use super::dialog::{Dialog, DialogEvent, DialogTemplate};
use super::text_box::TextBox;
use crate::color::Color;
use crate::draw::{DrawCommand, DrawList, fill_rect};
use crate::font::Font;
use crate::geometry::{Bounds, Point};
use crate::image::ImageKey;
use crate::input::{Input, Key, MouseButton};
use crate::screen::{Screen, ScreenAction};
use crate::spaceport::credits::comma_number;
use crate::spaceport::view::{PROBLEM_AT, PROBLEM_SIZE};
use crate::text::{TextMetrics, wrap};

/// The player info screen's `DLOG` (and `DITL`) ID.
pub const PLAYER_INFO_DIALOG: i16 = 1017;
/// The pictures it is drawn over: the upper cap, the middle and the lower
/// cap (`PICT` 8518-8520).
pub const BACKGROUND: [ImageKey; 3] = [
    ImageKey::picture(8518),
    ImageKey::picture(8519),
    ImageKey::picture(8520),
];

/// A picture's own width and height.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PictureSize {
    pub width: f32,
    pub height: f32,
}

/// The [`BACKGROUND`] pictures' own sizes, each none when the picture is
/// missing or does not decode, as `_PlayerInfoDialogUpdate` finds them
/// before laying the background out (@0x5248a-0x524d0).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BackgroundSizes {
    /// The upper cap's, `PICT` 8518.
    pub upper: Option<PictureSize>,
    /// The middle's, `PICT` 8519.
    pub middle: Option<PictureSize>,
    /// The lower cap's, `PICT` 8520.
    pub lower: Option<PictureSize>,
}

impl BackgroundSizes {
    /// The three pictures' sizes, read from `pictures`.
    #[must_use]
    pub fn read(pictures: impl PictureSizes) -> Self {
        let read = |image: ImageKey| {
            pictures
                .picture_size(image.id)
                .map(|(width, height)| PictureSize {
                    width: width as f32,
                    height: height as f32,
                })
        };
        Self {
            upper: read(BACKGROUND[0]),
            middle: read(BACKGROUND[1]),
            lower: read(BACKGROUND[2]),
        }
    }
}
/// The Yes/No dialog Jettison asks in (`DLOG` 3002).
pub const YES_NO_DIALOG: i16 = 3002;
/// The Yes/No dialog's OK, its yes.
pub const YES_ITEM: usize = 1;
/// The Yes/No dialog's question item.
pub const QUESTION_ITEM: usize = 3;
/// The Yes/No dialog's Cancel, its no.
pub const NO_ITEM: usize = 5;

/// Done's item.
pub const DONE_ITEM: usize = 1;
/// The tabs' items, General first.
pub const TAB_ITEMS: [usize; 4] = [2, 3, 4, 5];
/// The content box's item.
pub const CONTENT_ITEM: usize = 6;
/// Jettison's item.
pub const JETTISON_ITEM: usize = 7;

/// The player info key: the original's default `playerInfoKey`
/// (`_keyPrefs` +0x32, key code 0x23, `_loadKeys` @0xce5a9-0xce5cc).
pub const PLAYER_INFO_KEY: Key = Key::Char('p');
/// The key that, held, makes Tab go back (`shiftKey`, 0x0200).
pub const BACKWARDS_KEY: Key = Key::Shift;

/// Whether `input` presses [`PLAYER_INFO_KEY`], not as a repeat: what
/// asks for the player info screen where the original's filters read the
/// key.
#[must_use]
pub fn is_player_info_press(input: &Input) -> bool {
    *input
        == Input::Key {
            key: PLAYER_INFO_KEY,
            pressed: true,
            repeat: false,
        }
}

/// The tabs' labels, `STR#` 150 #36-#39.
pub const TAB_LABELS: [&str; 4] = ["General", "Cargo", "Extras", "Honors"];
/// Jettison's label, `STR#` 150 #61.
pub const JETTISON_LABEL: &str = "Jettison Cargo";

/// The labels' colour, `_rgbLightGrey` (0xc000 a channel, @0x1bf41).
pub const LABEL_COLOR: Color = Color::rgba(192, 192, 192, 255);
/// The values' colour, `ForeColor(30)`.
pub const VALUE_COLOR: Color = Color::WHITE;
/// Every text's size, in Geneva: `_smallTextSize` (@0x1d3eb).
pub const TEXT_SIZE: f32 = 9.0;

/// What a field the engine cannot show yet shows.
pub const PLACEHOLDER: &str = "\u{2014}";
/// The fields that show [`PLACEHOLDER`], by label: the combat rating,
/// until rdm:task/combat-rating-readers.
pub const PLACEHOLDER_FIELDS: [&str; 1] = ["Combat Rating"];
/// The buttons drawn that do nothing: none.
pub const INERT_BUTTONS: [&str; 0] = [];

/// The content box's height in the stock dialog, which the texts grow
/// past.
pub const CONTENT_HEIGHT: f32 = 141.0;
/// The left column's label and value, and the right's, from the content
/// box's left.
const COLUMNS: [f32; 4] = [5.0, 80.0, 205.0, 280.0];
/// The first row's baseline below the content box's top, and the step.
const FIRST_ROW: f32 = 12.0;
/// How far each row's baseline is below the one before.
const ROW_STEP: f32 = 16.0;
/// The message's baseline below the content box's top.
const MESSAGE_BASELINE: f32 = 55.0;
/// The text box's inset from the content box: top, left and right.
const TEXT_INSET: f32 = 5.0;
/// The text box's inset from the content box's bottom.
const TEXT_INSET_BOTTOM: f32 = 10.0;

/// The tab shown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tab {
    /// The pilot and the ship.
    General,
    /// The cargo.
    Cargo,
    /// The outfits.
    Extras,
    /// The ranks and honors.
    Honors,
}

impl Tab {
    /// Every tab, in order.
    pub const ALL: [Self; 4] = [Self::General, Self::Cargo, Self::Extras, Self::Honors];

    /// Its place among the tabs, from 0.
    fn index(self) -> usize {
        match self {
            Self::General => 0,
            Self::Cargo => 1,
            Self::Extras => 2,
            Self::Honors => 3,
        }
    }

    /// The next tab, Honors wrapping to General.
    #[must_use]
    pub fn next(self) -> Self {
        Self::ALL[(self.index() + 1) % 4]
    }

    /// The previous tab, General wrapping to Honors.
    #[must_use]
    pub fn previous(self) -> Self {
        Self::ALL[(self.index() + 3) % 4]
    }
}

/// A button of the screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlayerInfoButton {
    /// Done.
    Done,
    /// One of the tabs.
    Tab(Tab),
    /// Jettison Cargo.
    Jettison,
}

impl PlayerInfoButton {
    /// Every button, in item order.
    const ALL: [Self; 6] = [
        Self::Done,
        Self::Tab(Tab::General),
        Self::Tab(Tab::Cargo),
        Self::Tab(Tab::Extras),
        Self::Tab(Tab::Honors),
        Self::Jettison,
    ];

    /// Its item.
    fn item(self) -> usize {
        match self {
            Self::Done => DONE_ITEM,
            Self::Tab(tab) => TAB_ITEMS[tab.index()],
            Self::Jettison => JETTISON_ITEM,
        }
    }

    /// Its label.
    fn label(self) -> &'static str {
        match self {
            Self::Done => DONE_LABEL,
            Self::Tab(tab) => TAB_LABELS[tab.index()],
            Self::Jettison => JETTISON_LABEL,
        }
    }
}

/// A text set at a place: from `at.x`, on baseline `at.y`.
#[derive(Clone, Debug, PartialEq)]
pub struct PlacedText {
    /// The text.
    pub text: String,
    /// Where it starts and its baseline.
    pub at: Point,
    /// Its colour.
    pub color: Color,
}

impl PlacedText {
    fn new(text: impl Into<String>, x: f32, baseline: f32, color: Color) -> Self {
        Self {
            text: text.into(),
            at: Point::new(x, baseline),
            color,
        }
    }

    /// Draws it in Geneva at [`TEXT_SIZE`], its top a size above its
    /// baseline.
    fn draw(&self, list: &mut DrawList) {
        list.text_in(
            Font::Geneva,
            self.text.clone(),
            Point::new(self.at.x, self.at.y - TEXT_SIZE),
            TEXT_SIZE,
            None,
            self.color,
        );
    }
}

/// A percentage, or #396 "N/A".
fn percent(shown: Option<i64>, words: &Words) -> String {
    shown.map_or_else(
        || words.not_available.to_string(),
        |percent| format!("{percent}%"),
    )
}

/// The Energy row's value (@0x5327a-0x53451), in `words`.
#[must_use]
pub fn energy_text(energy: Option<EnergyShown>, words: &Words) -> String {
    let Some(energy) = energy else {
        return words.no_energy.to_string();
    };
    let reach = match energy.reach {
        Reach::Jumps { jumps, spare } => format!(
            "  ({jumps} {}{})",
            if jumps == 1 {
                &words.jump
            } else {
                &words.jumps
            },
            if spare {
                format!(" {} {}", words.plus, words.maneuvering)
            } else {
                String::new()
            }
        ),
        Reach::ManeuveringOnly => format!("  ({} {})", words.maneuvering, words.only),
        Reach::Nothing => String::new(),
    };
    format!("{}%{reach}", energy.percent)
}

/// The daily income line's pieces, in order, each with its colour
/// (@0x535cf-0x53731), in `words`: the label, then the amount and
/// "credits", then "per day".
#[must_use]
pub fn income_texts(line: IncomeLine, words: &Words) -> [(String, Color); 3] {
    let (label, amount) = match line {
        IncomeLine::Expenses(amount) => (words.expenses.to_string(), amount),
        IncomeLine::NetIncome(amount) => (format!("{} {}", words.net, words.income), amount),
        IncomeLine::Income(amount) => (words.income.to_string(), amount),
    };
    [
        (format!("{label}  "), LABEL_COLOR),
        (
            format!("{} {}", comma_number(amount), words.credits),
            VALUE_COLOR,
        ),
        (format!(" {}", words.per_day), LABEL_COLOR),
    ]
}

/// The General tab's texts for `info`, in its words, the content box's
/// top-left at `origin`, the income line's pieces set one after another
/// as `metrics` measures them.
#[must_use]
pub fn general_texts(
    info: &PlayerInfo,
    origin: Point,
    metrics: &dyn TextMetrics,
) -> Vec<PlacedText> {
    let words = &info.words;
    let ids = info.cheat_ids;
    let with_id = |value: &str, id: Option<i64>| match id.filter(|_| info.cheats) {
        Some(id) => format!("{value}{}", cheat_suffix(id)),
        None => value.to_owned(),
    };
    let rows: [[(&str, String); 2]; 6] = [
        [
            (&words.pilot_name_label, info.pilot_name.clone()),
            (&words.ship_name_label, info.ship_name.clone()),
        ],
        [
            (&words.date_label, info.date.clone()),
            (
                &words.ship_class_label,
                with_id(&info.ship_class, Some(i64::from(ids.class))),
            ),
        ],
        [
            (
                &words.system_label,
                with_id(&info.system, Some(i64::from(ids.system))),
            ),
            (
                &words.turn_rate_label,
                format!("{}{}", info.turn_rate, words.degrees_per_second),
            ),
        ],
        [
            (
                &words.legal_status_label,
                with_id(&info.legal_status, ids.record.map(i64::from)),
            ),
            (&words.accel_label, info.accel.to_string()),
        ],
        [
            (
                &words.combat_rating_label,
                with_id(PLACEHOLDER, Some(ids.kills)),
            ),
            (&words.max_speed_label, info.max_speed.to_string()),
        ],
        [
            (&words.shield_label, percent(info.shield, words)),
            (&words.credits_label(), comma_number(info.credits)),
        ],
    ];
    let baseline = |row: usize| origin.y + FIRST_ROW + ROW_STEP * row as f32;
    let mut texts = Vec::new();
    let mut place = |row: usize, column: usize, label: &str, value: String| {
        let x = origin.x + COLUMNS[2 * column];
        texts.push(PlacedText::new(label, x, baseline(row), LABEL_COLOR));
        let x = origin.x + COLUMNS[2 * column + 1];
        texts.push(PlacedText::new(value, x, baseline(row), VALUE_COLOR));
    };
    for (row, pair) in rows.into_iter().enumerate() {
        for (column, (label, value)) in pair.into_iter().enumerate() {
            place(row, column, label, value);
        }
    }
    place(6, 0, &words.armor_label, percent(info.armor, words));
    place(7, 0, &words.energy_label, energy_text(info.energy, words));
    if let Some(line) = info.income {
        let mut x = origin.x + COLUMNS[0];
        for (text, color) in income_texts(line, words) {
            let width = metrics.width(Font::Geneva, TEXT_SIZE, &text);
            texts.push(PlacedText::new(text, x, baseline(8), color));
            x += width;
        }
    }
    texts
}

/// Paints the dialog's background over `bounds` from the pictures'
/// `sizes` (@0x524a6-0x5271c): black, then the middle, then the upper cap
/// at the top-left and the lower cap at the bottom-left, every picture
/// at its own width and cut by `bounds`, as the window cuts it.
///
/// - The middle (@0x524af-0x525f9) starts below the upper cap, or at the
///   top without one (@0x524df-0x5251e). It ends above the lower cap
///   (@0x52530-0x52568), or at its own height without one. A span under
///   its own height draws it at its own size, its bottom running under
///   the lower cap rather than being squashed (@0x52581-0x525a8);
///   otherwise it is stretched down the span. Without a middle, none is
///   drawn (@0x524a0).
/// - The upper cap (@0x5260a-0x52680) and the lower cap (@0x52696-0x5271c)
///   are drawn at their own sizes, when they are there.
pub fn draw_background(list: &mut DrawList, bounds: Bounds, sizes: &BackgroundSizes) {
    fill_rect(list, bounds, Color::BLACK);
    let picture = |list: &mut DrawList, image: ImageKey, top_left: Point, width, height| {
        list.clipped(
            bounds,
            DrawCommand::StretchedPicture {
                image,
                top_left,
                width,
                height,
            },
        );
    };
    if let Some(middle) = sizes.middle {
        let top = bounds.min.y + sizes.upper.map_or(0.0, |upper| upper.height);
        let bottom = sizes
            .lower
            .map_or(top + middle.height, |lower| bounds.max.y - lower.height);
        let middle_at = Point::new(bounds.min.x, top);
        let span = bottom - top;
        if span < middle.height {
            list.clipped(
                bounds,
                DrawCommand::Picture {
                    image: BACKGROUND[1],
                    top_left: middle_at,
                },
            );
        } else {
            picture(list, BACKGROUND[1], middle_at, middle.width, span);
        }
    }
    if let Some(upper) = sizes.upper {
        picture(list, BACKGROUND[0], bounds.min, upper.width, upper.height);
    }
    if let Some(lower) = sizes.lower {
        let at = Point::new(bounds.min.x, bounds.max.y - lower.height);
        picture(list, BACKGROUND[2], at, lower.width, lower.height);
    }
}

/// How much the dialog grows for texts `heights` high in a content box
/// `item_height` high (@0x5a2a6-0x5a4e7): `tallest - item_height + 4`
/// when the tallest is over it, else nothing.
#[must_use]
pub fn growth(heights: impl IntoIterator<Item = f32>, item_height: f32) -> f32 {
    let tallest = heights.into_iter().fold(0.0, f32::max);
    if tallest > item_height {
        tallest - item_height + 4.0
    } else {
        0.0
    }
}

/// The metrics, shared, with a `Debug` that shows nothing of them.
#[derive(Clone)]
struct MetricsHandle(Rc<dyn TextMetrics>);

impl std::fmt::Debug for MetricsHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("TextMetrics")
    }
}

/// The screen, laid out.
#[derive(Clone, Debug)]
struct Laid {
    bounds: Bounds,
    content: Bounds,
    buttons: [(PlayerInfoButton, Button); 6],
    /// The Cargo, Extras and Honors texts, wrapped in their box, where
    /// there is one, each drawn as `TETextBox` draws it: a line
    /// straddling the box's bottom shows its top, clipped to the box.
    texts: [Option<TextBox>; 3],
    metrics: MetricsHandle,
}

/// The player info screen.
#[derive(Clone, Debug)]
pub struct PlayerInfoScreen {
    /// The screen, or why it cannot be shown.
    laid: Result<Laid, String>,
    /// The Yes/No dialog's template, or why there is none.
    confirm_template: Result<DialogTemplate, String>,
    /// The Yes/No dialog, while it asks.
    confirm: Option<Dialog>,
    info: PlayerInfo,
    style: ButtonStyle,
    /// The background pictures' own sizes.
    background: BackgroundSizes,
    tab: Tab,
    /// The button a click is held on, and its tracking.
    tracked: Option<PlayerInfoButton>,
    tracker: ButtonTracker,
    /// Whether Shift is held.
    backwards: bool,
    jettison: bool,
    closed: bool,
}

impl PlayerInfoScreen {
    /// `info`, laid out by `layout`: the screen's template and the
    /// metrics its text is measured by, or why there are none; Jettison
    /// asking in the `confirm` Yes/No dialog, or answering no without
    /// one; its buttons labelled in `style`; its background laid out from
    /// the pictures' `background` sizes. It opens on General.
    #[must_use]
    pub fn new(
        layout: Result<(DialogTemplate, Rc<dyn TextMetrics>), String>,
        confirm: Result<DialogTemplate, String>,
        info: PlayerInfo,
        style: ButtonStyle,
        background: BackgroundSizes,
    ) -> Self {
        let laid = layout.and_then(|(template, metrics)| lay_out(&template, &info, metrics));
        Self {
            laid,
            confirm_template: confirm,
            confirm: None,
            info,
            style,
            background,
            tab: Tab::General,
            tracked: None,
            tracker: ButtonTracker::default(),
            backwards: false,
            jettison: false,
            closed: false,
        }
    }

    /// The tab shown.
    #[must_use]
    pub fn tab(&self) -> Tab {
        self.tab
    }

    /// Whether it has closed.
    #[must_use]
    pub fn closed(&self) -> bool {
        self.closed
    }

    /// The dialog's frame on the screen, if it could be laid out.
    #[must_use]
    pub fn bounds(&self) -> Option<Bounds> {
        self.laid.as_ref().ok().map(|laid| laid.bounds)
    }

    /// The content box on the screen, if it could be laid out.
    #[must_use]
    pub fn content_bounds(&self) -> Option<Bounds> {
        self.laid.as_ref().ok().map(|laid| laid.content)
    }

    /// Where `button` is on the screen, drawn or not.
    #[must_use]
    pub fn button_bounds(&self, button: PlayerInfoButton) -> Option<Bounds> {
        let laid = self.laid.as_ref().ok()?;
        laid.buttons
            .iter()
            .find(|(which, _)| *which == button)
            .map(|(_, drawn)| drawn.rect)
    }

    /// Why the screen cannot be shown, if it cannot.
    #[must_use]
    pub fn problem(&self) -> Option<&str> {
        self.laid.as_ref().err().map(String::as_str)
    }

    /// The Yes/No dialog, while Jettison asks.
    #[must_use]
    pub fn confirm(&self) -> Option<&Dialog> {
        self.confirm.as_ref()
    }

    /// Whether the Yes/No dialog is asking.
    #[must_use]
    pub fn confirm_open(&self) -> bool {
        self.confirm.is_some()
    }

    /// The player info shown.
    #[must_use]
    pub fn info(&self) -> &PlayerInfo {
        &self.info
    }

    /// Whether Jettison was confirmed since this was last taken, once:
    /// whoever holds the session jettisons everything.
    pub fn take_jettison(&mut self) -> bool {
        std::mem::take(&mut self.jettison)
    }

    /// Whether Jettison is drawn and acts: on the Cargo tab, with cargo
    /// that can be jettisoned.
    fn jettison_shown(&self) -> bool {
        self.tab == Tab::Cargo && self.info.jettisonable
    }

    /// Chooses `button` (@0x5a5ea-0x5a679).
    fn choose(&mut self, button: PlayerInfoButton) {
        match button {
            PlayerInfoButton::Done => self.closed = true,
            PlayerInfoButton::Tab(tab) => self.tab = tab,
            PlayerInfoButton::Jettison if self.jettison_shown() => self.ask(),
            PlayerInfoButton::Jettison => {}
        }
    }

    /// Opens the Yes/No dialog on #291; without it the answer is no.
    fn ask(&mut self) {
        let (Ok(template), Ok(laid)) = (&self.confirm_template, &self.laid) else {
            return;
        };
        let mut dialog = Dialog::new(template, &[], Rc::clone(&laid.metrics.0))
            .with_buttons(ButtonSkin::NOVA, self.style)
            .with_default(Some(YES_ITEM))
            .with_cancel(None);
        dialog.set_text(QUESTION_ITEM, &self.info.words.jettison_question);
        self.confirm = Some(dialog);
    }

    /// The Yes/No dialog's input: yes jettisons and closes; no goes back.
    fn confirm_input(&mut self, input: &Input) {
        let Some(dialog) = &mut self.confirm else {
            return;
        };
        let event = dialog.input(input);
        match event {
            Some(DialogEvent::Item(YES_ITEM)) => {
                self.confirm = None;
                self.jettison = true;
                self.closed = true;
            }
            Some(DialogEvent::Item(NO_ITEM)) => self.confirm = None,
            _ => {}
        }
    }

    /// A click on the buttons, tracked as `_TrackPlayerInfoButtons`
    /// tracks it, silently.
    fn pointer(&mut self, input: &Input) {
        let Ok(laid) = &self.laid else {
            return;
        };
        if let Input::PointerButton {
            button: MouseButton::Left,
            pressed: true,
            at,
        } = *input
        {
            self.tracker.cancel_pointer();
            self.tracked = laid
                .buttons
                .iter()
                .find(|(_, button)| button.contains(at))
                .map(|(which, _)| *which);
        }
        let Some(which) = self.tracked else {
            return;
        };
        let Some(rect) = self.button_bounds(which) else {
            return;
        };
        let chosen = self.tracker.track(rect, true, input);
        let _ = self.tracker.take_sound();
        if !self.tracker.armed() {
            self.tracked = None;
        }
        if chosen {
            self.choose(which);
        }
    }
}

/// The screen for `info` laid out from `template`, measured by
/// `metrics`: why not, when the template lacks an item.
fn lay_out(
    template: &DialogTemplate,
    info: &PlayerInfo,
    metrics: Rc<dyn TextMetrics>,
) -> Result<Laid, String> {
    let local = |item: usize| {
        template
            .items
            .get(item - 1)
            .map(|found| found.bounds)
            .ok_or_else(|| format!("its dialog has no item {item}"))
    };
    let content_local = local(CONTENT_ITEM)?;
    let line_height = metrics.line_height(Font::Geneva, TEXT_SIZE);
    let shown = [&info.cargo, &info.extras, &info.honors];
    let heights = shown.iter().filter_map(|text| text.as_deref()).map(|text| {
        let lines = wrap(
            &metrics,
            Font::Geneva,
            TEXT_SIZE,
            text,
            content_local.width(),
        )
        .len();
        (lines + 2) as f32 * line_height
    });
    let grown = growth(heights, content_local.height());
    let size = (template.bounds.width(), template.bounds.height() + grown);
    let origin = template
        .placement
        .origin(Bounds::at(template.bounds.min, size.0, size.1));
    let placed = content_local.offset(origin);
    let content = Bounds {
        min: placed.min,
        max: Point::new(placed.max.x, placed.max.y + grown),
    };
    let mut buttons = Vec::new();
    for which in PlayerInfoButton::ALL {
        let mut rect = local(which.item())?.offset(origin);
        if matches!(which, PlayerInfoButton::Done | PlayerInfoButton::Jettison) {
            rect = rect.offset(Point::new(0.0, grown));
        }
        buttons.push((which, Button::new(rect, which.label())));
    }
    let boxed = Bounds {
        min: Point::new(content.min.x + TEXT_INSET, content.min.y + TEXT_INSET),
        max: Point::new(
            content.max.x - TEXT_INSET,
            content.max.y - TEXT_INSET_BOTTOM,
        ),
    };
    let texts = shown.map(|text| {
        text.as_deref()
            .map(|text| TextBox::new(text, Font::Geneva, TEXT_SIZE, VALUE_COLOR, boxed, &metrics))
    });
    Ok(Laid {
        bounds: Bounds::at(origin, size.0, size.1),
        content,
        buttons: buttons
            .try_into()
            .map_err(|_| "its buttons could not be laid out".to_owned())?,
        texts,
        metrics: MetricsHandle(metrics),
    })
}

impl Screen for PlayerInfoScreen {
    /// The Yes/No dialog takes everything while it asks. Otherwise Tab
    /// changes the tab, Return, Enter, Escape and P close, and a click
    /// goes to the buttons. It never quits.
    fn input(&mut self, input: &Input) -> ScreenAction {
        if let Input::Key {
            key: BACKWARDS_KEY,
            pressed,
            ..
        } = *input
        {
            self.backwards = pressed;
        }
        if self.confirm.is_some() {
            self.confirm_input(input);
            return ScreenAction::None;
        }
        match *input {
            Input::Key {
                key: Key::Tab,
                pressed: true,
                ..
            } if self.laid.is_ok() => {
                self.tab = if self.backwards {
                    self.tab.previous()
                } else {
                    self.tab.next()
                };
            }
            Input::Key {
                key: Key::Enter | Key::Escape | PLAYER_INFO_KEY,
                pressed: true,
                repeat: false,
            } => self.closed = true,
            Input::PointerButton { .. } | Input::PointerMoved(_) => self.pointer(input),
            _ => {}
        }
        ScreenAction::None
    }

    /// Nothing moves on its own.
    fn tick(&mut self, _dt: Duration) {}

    fn draw(&self, list: &mut DrawList) {
        let laid = match &self.laid {
            Ok(laid) => laid,
            Err(reason) => {
                list.text(
                    format!(
                        "Cannot show the player info: {reason}. Press Return or Escape to close it."
                    ),
                    PROBLEM_AT,
                    PROBLEM_SIZE,
                    None,
                    Color::ERROR,
                );
                return;
            }
        };
        draw_background(list, laid.bounds, &self.background);
        for (which, button) in &laid.buttons {
            if *which == PlayerInfoButton::Jettison && !self.jettison_shown() {
                continue;
            }
            let pressed = *which == PlayerInfoButton::Tab(self.tab)
                || (self.tracked == Some(*which) && self.tracker.pressed());
            button.draw(
                pressed,
                &ButtonSkin::NOVA,
                &self.style,
                &laid.metrics.0,
                list,
            );
        }
        self.draw_content(laid, list);
        if let Some(dialog) = &self.confirm {
            fill_rect(list, dialog.bounds(), Color::BLACK);
            dialog.draw(list);
        }
    }

    fn cancel_pointer(&mut self) {
        self.tracker.cancel_pointer();
        self.tracked = None;
        if let Some(dialog) = &mut self.confirm {
            dialog.cancel_pointer();
        }
    }

    fn release_keys(&mut self) {
        self.backwards = false;
    }
}

impl PlayerInfoScreen {
    /// Draws the tab shown in the content box: the General rows, a text
    /// in its box, or the message saying there is nothing to show.
    fn draw_content(&self, laid: &Laid, list: &mut DrawList) {
        let content = laid.content;
        let words = &self.info.words;
        let (text, message) = match self.tab {
            Tab::General => {
                for text in general_texts(&self.info, content.min, &*laid.metrics.0) {
                    text.draw(list);
                }
                return;
            }
            Tab::Cargo if self.info.fleet => (&laid.texts[0], &words.no_fleet_cargo),
            Tab::Cargo => (&laid.texts[0], &words.no_cargo),
            Tab::Extras => (&laid.texts[1], &words.no_extras),
            Tab::Honors => (&laid.texts[2], &words.no_honors),
        };
        if let Some(text) = text {
            fill_rect(list, text.rect(), Color::BLACK);
            text.draw(list);
        } else {
            let width = laid.metrics.0.width(Font::Geneva, TEXT_SIZE, message);
            let x = f32::midpoint(content.min.x, content.max.x).trunc() - (width / 2.0).trunc();
            PlacedText::new(&**message, x, content.min.y + MESSAGE_BASELINE, LABEL_COLOR)
                .draw(list);
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use nova_sim::player_info::CheatIds;

    use crate::sound::Sound;
    use crate::text::fixture::MonoMetrics;
    use crate::ui::dialog::{ItemSpec, ItemTemplate, Placement};

    fn rect(x: f32, y: f32, w: f32, h: f32) -> Bounds {
        Bounds::at(Point::new(x, y), w, h)
    }

    fn user(x: f32, y: f32, w: f32, h: f32) -> ItemTemplate {
        ItemTemplate {
            bounds: rect(x, y, w, h),
            enabled: true,
            kind: ItemSpec::User,
        }
    }

    /// Stock `DLOG` 1017, 413 x 227 and centred, with its seven items as
    /// `DITL` 1017 has them.
    fn template() -> DialogTemplate {
        DialogTemplate {
            bounds: rect(40.0, 40.0, 413.0, 227.0),
            placement: Placement::Center,
            items: vec![
                user(293.0, 195.0, 99.0, 25.0),
                user(7.0, 8.0, 99.0, 25.0),
                user(107.0, 8.0, 99.0, 25.0),
                user(207.0, 8.0, 99.0, 25.0),
                user(307.0, 8.0, 99.0, 25.0),
                ItemTemplate {
                    enabled: false,
                    ..user(4.0, 40.0, 405.0, 141.0)
                },
                user(60.0, 195.0, 150.0, 25.0),
            ],
        }
    }

    /// Where the dialog goes: (1024 - 413) / 2 and (768 - 227) / 2,
    /// floored.
    const ORIGIN: Point = Point::new(305.0, 270.0);
    /// The content box's top-left.
    const CONTENT: Point = Point::new(309.0, 310.0);

    /// Stock `YesNo` (`DLOG` 3002): 355 x 109, centred, with OK (1), a
    /// picture parked outside (2), static text (3), a picture (4) and
    /// Cancel (5).
    fn yes_no() -> DialogTemplate {
        DialogTemplate {
            bounds: rect(40.0, 40.0, 355.0, 109.0),
            placement: Placement::Center,
            items: vec![
                ItemTemplate {
                    bounds: rect(263.0, 75.0, 70.0, 20.0),
                    enabled: true,
                    kind: ItemSpec::Button("OK".into()),
                },
                ItemTemplate {
                    bounds: rect(277.0, 131.0, 68.0, 30.0),
                    enabled: false,
                    kind: ItemSpec::Picture(129),
                },
                ItemTemplate {
                    bounds: rect(52.0, 5.0, 295.0, 50.0),
                    enabled: false,
                    kind: ItemSpec::StaticText("Text".into()),
                },
                ItemTemplate {
                    bounds: rect(7.0, 5.0, 32.0, 32.0),
                    enabled: false,
                    kind: ItemSpec::Picture(130),
                },
                ItemTemplate {
                    bounds: rect(181.0, 75.0, 70.0, 20.0),
                    enabled: true,
                    kind: ItemSpec::Button("Cancel".into()),
                },
            ],
        }
    }

    fn info() -> PlayerInfo {
        PlayerInfo {
            pilot_name: "Ada".to_owned(),
            date: "June 23rd, 1177 NC".to_owned(),
            system: "Sol".to_owned(),
            legal_status: "Citizen".to_owned(),
            ship_name: "Bitsy".to_owned(),
            ship_class: "Shuttle".to_owned(),
            turn_rate: 90,
            accel: 450,
            max_speed: 600,
            credits: 12_345,
            shield: Some(100),
            armor: None,
            energy: Some(EnergyShown {
                percent: 83,
                reach: Reach::Jumps {
                    jumps: 2,
                    spare: true,
                },
            }),
            income: None,
            fleet: false,
            cargo: Some("Current cargo in your ship:\r\rFive tons of food.\r\r".to_owned()),
            extras: None,
            honors: None,
            jettisonable: true,
            words: Words::STOCK,
            cheat_ids: CheatIds {
                system: 130,
                record: Some(5),
                kills: 12,
                class: 128,
            },
            cheats: false,
        }
    }

    fn sized(width: f32, height: f32) -> PictureSize {
        PictureSize { width, height }
    }

    /// The stock pictures' sizes: 413 x 40, 413 x 215 and 413 x 40.
    fn stock() -> BackgroundSizes {
        BackgroundSizes {
            upper: Some(sized(413.0, 40.0)),
            middle: Some(sized(413.0, 215.0)),
            lower: Some(sized(413.0, 40.0)),
        }
    }

    fn screen_with(info: PlayerInfo, background: BackgroundSizes) -> PlayerInfoScreen {
        PlayerInfoScreen::new(
            Ok((template(), Rc::new(MonoMetrics))),
            Ok(yes_no()),
            info,
            ButtonStyle::STOCK,
            background,
        )
    }

    fn screen_of(info: PlayerInfo) -> PlayerInfoScreen {
        screen_with(info, stock())
    }

    fn screen() -> PlayerInfoScreen {
        screen_of(info())
    }

    fn key(key: Key, pressed: bool, repeat: bool) -> Input {
        Input::Key {
            key,
            pressed,
            repeat,
        }
    }

    fn press(key_pressed: Key) -> Input {
        key(key_pressed, true, false)
    }

    fn mouse(pressed: bool, at: Point) -> Input {
        Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at,
        }
    }

    /// A click (press and release) at `at`.
    fn click(screen: &mut PlayerInfoScreen, at: Point) {
        screen.input(&mouse(true, at));
        screen.input(&mouse(false, at));
    }

    fn click_button(screen: &mut PlayerInfoScreen, button: PlayerInfoButton) {
        let at = screen.button_bounds(button).expect("laid out").center();
        click(screen, at);
    }

    fn drawn(screen: &PlayerInfoScreen) -> Vec<DrawCommand> {
        let mut list = DrawList::new();
        screen.draw(&mut list);
        list.iter().cloned().collect()
    }

    fn texts(screen: &PlayerInfoScreen) -> Vec<String> {
        drawn(screen)
            .into_iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, .. } => Some(text),
                _ => None,
            })
            .collect()
    }

    fn general(info: &PlayerInfo) -> Vec<(String, f32, f32, Color)> {
        general_texts(info, Point::new(0.0, 0.0), &MonoMetrics)
            .into_iter()
            .map(|placed| (placed.text, placed.at.x, placed.at.y, placed.color))
            .collect()
    }

    fn row(label: &str, value: &str, x: f32, baseline: f32) -> [(String, f32, f32, Color); 2] {
        [
            (label.to_owned(), x, baseline, LABEL_COLOR),
            (value.to_owned(), x + 75.0, baseline, VALUE_COLOR),
        ]
    }

    #[test]
    fn the_named_values() {
        assert_eq!(PLAYER_INFO_DIALOG, 1017);
        assert_eq!(YES_NO_DIALOG, 3002);
        assert_eq!(BACKGROUND.map(|image| image.id), [8518, 8519, 8520]);
        assert_eq!(
            (DONE_ITEM, TAB_ITEMS, CONTENT_ITEM, JETTISON_ITEM),
            (1, [2, 3, 4, 5], 6, 7)
        );
        assert_eq!(LABEL_COLOR, Color::rgba(192, 192, 192, 255));
        assert_eq!(TEXT_SIZE, 9.0);
        assert_eq!(PLAYER_INFO_KEY, Key::Char('p'));
    }

    #[test]
    fn the_placeholder_fields_and_inert_buttons_are_listed() {
        assert_eq!(PLACEHOLDER_FIELDS, ["Combat Rating"]);
        assert!(INERT_BUTTONS.is_empty());
        assert!(
            Words::STOCK
                .combat_rating_label
                .starts_with(PLACEHOLDER_FIELDS[0])
        );
    }

    #[test]
    fn the_tabs_wrap_both_ways() {
        assert_eq!(Tab::General.next(), Tab::Cargo);
        assert_eq!(Tab::Cargo.next(), Tab::Extras);
        assert_eq!(Tab::Extras.next(), Tab::Honors);
        assert_eq!(Tab::Honors.next(), Tab::General);
        assert_eq!(Tab::General.previous(), Tab::Honors);
        assert_eq!(Tab::Honors.previous(), Tab::Extras);
        assert_eq!(Tab::Extras.previous(), Tab::Cargo);
        assert_eq!(Tab::Cargo.previous(), Tab::General);
    }

    /// Stock words but for the few a translation changes.
    fn translated() -> Words {
        Words {
            not_available: "n/a".into(),
            shield_label: "Schild:".into(),
            credits: "crédits".into(),
            no_energy: "Keine".into(),
            maneuvering: "Manöver".into(),
            per_day: "pro Tag".into(),
            no_cargo: "Keine Fracht".into(),
            jettison_question: "Abwerfen?".into(),
            ..Words::STOCK
        }
    }

    #[test]
    fn every_str_2002_string_drawn_is_the_sessions() {
        let shown = PlayerInfo {
            shield: None,
            energy: None,
            income: Some(IncomeLine::Income(300)),
            cargo: None,
            words: translated(),
            ..info()
        };
        let general = general(&shown);
        let has = |text: &str, x: f32| {
            general
                .iter()
                .any(|placed| placed.0 == text && placed.1 == x)
        };
        assert!(has("Schild:", 5.0) && has("n/a", 80.0), "{general:?}");
        assert!(has("Crédits:", 205.0), "{general:?}");
        assert!(has("Keine", 80.0), "{general:?}");
        assert!(has("300 crédits", 5.0 + 9.0 * 4.5), "{general:?}");
        assert!(has(" pro Tag", 5.0 + 20.0 * 4.5), "{general:?}");
        assert_eq!(
            energy_text(
                Some(EnergyShown {
                    percent: 10,
                    reach: Reach::ManeuveringOnly
                }),
                &shown.words
            ),
            "10%  (Manöver only)"
        );
        let mut screen = screen_of(shown);
        click_button(&mut screen, PlayerInfoButton::Tab(Tab::Cargo));
        assert!(texts(&screen).contains(&"Keine Fracht".to_owned()));
        let mut screen = screen_of(PlayerInfo {
            words: translated(),
            ..info()
        });
        screen.input(&press(Key::Tab));
        click_button(&mut screen, PlayerInfoButton::Jettison);
        assert_eq!(
            screen
                .confirm()
                .and_then(|dialog| dialog.text(QUESTION_ITEM))
                .map(|text| text.lines().join(" ")),
            Some("Abwerfen?".to_owned())
        );
    }

    #[test]
    fn every_general_row_is_at_its_place_in_its_colour() {
        let mut expected = Vec::new();
        for pair in [
            [
                row("Pilot Name:", "Ada", 5.0, 12.0),
                row("Ship Name:", "Bitsy", 205.0, 12.0),
            ],
            [
                row("Current Date:", "June 23rd, 1177 NC", 5.0, 28.0),
                row("Ship Class:", "Shuttle", 205.0, 28.0),
            ],
            [
                row("System:", "Sol", 5.0, 44.0),
                row("Turn Rate:", "90°/sec", 205.0, 44.0),
            ],
            [
                row("Legal Status:", "Citizen", 5.0, 60.0),
                row("Accel Rate:", "450", 205.0, 60.0),
            ],
            [
                row("Combat Rating:", PLACEHOLDER, 5.0, 76.0),
                row("Max Speed:", "600", 205.0, 76.0),
            ],
            [
                row("Shield Status:", "100%", 5.0, 92.0),
                row("Credits:", "12,345", 205.0, 92.0),
            ],
        ] {
            for placed in pair {
                expected.extend(placed);
            }
        }
        expected.extend(row("Armor Status:", "N/A", 5.0, 108.0));
        expected.extend(row(
            "Energy Status:",
            "83%  (2 jumps plus maneuvering energy)",
            5.0,
            124.0,
        ));
        assert_eq!(general(&info()), expected);
        let moved = general_texts(&info(), CONTENT, &MonoMetrics);
        assert_eq!(
            moved[0].at,
            Point::new(314.0, 322.0),
            "from the box's corner"
        );
    }

    #[test]
    fn the_energy_says_how_far_it_reaches() {
        let energy =
            |percent, reach| energy_text(Some(EnergyShown { percent, reach }), &Words::STOCK);
        assert_eq!(energy_text(None, &Words::STOCK), "None");
        assert_eq!(
            energy(
                33,
                Reach::Jumps {
                    jumps: 1,
                    spare: false
                }
            ),
            "33%  (1 jump)"
        );
        assert_eq!(
            energy(
                90,
                Reach::Jumps {
                    jumps: 1,
                    spare: true
                }
            ),
            "90%  (1 jump plus maneuvering energy)"
        );
        assert_eq!(
            energy(13, Reach::ManeuveringOnly),
            "13%  (maneuvering energy only)"
        );
        assert_eq!(energy(0, Reach::Nothing), "0%");
    }

    #[test]
    fn the_income_line_is_a_grey_label_a_white_amount_and_a_grey_per_day() {
        assert_eq!(
            income_texts(IncomeLine::Expenses(1500), &Words::STOCK).map(|(text, _)| text),
            ["Expenses:  ", "1,500 credits", " per day"]
        );
        assert_eq!(
            income_texts(IncomeLine::NetIncome(200), &Words::STOCK).map(|(text, _)| text),
            ["Net Income:  ", "200 credits", " per day"]
        );
        assert_eq!(
            income_texts(IncomeLine::Income(1), &Words::STOCK).map(|(_, color)| color),
            [LABEL_COLOR, VALUE_COLOR, LABEL_COLOR]
        );
        assert_eq!(
            income_texts(IncomeLine::Income(1), &Words::STOCK)[0].0,
            "Income:  ",
            "credits always plural"
        );
        let with = PlayerInfo {
            income: Some(IncomeLine::Income(300)),
            ..info()
        };
        let placed = general(&with);
        assert_eq!(
            placed[placed.len() - 3..],
            [
                ("Income:  ".to_owned(), 5.0, 140.0, LABEL_COLOR),
                (
                    "300 credits".to_owned(),
                    5.0 + 9.0 * 4.5,
                    140.0,
                    VALUE_COLOR
                ),
                (" per day".to_owned(), 5.0 + 20.0 * 4.5, 140.0, LABEL_COLOR),
            ],
            "each piece after the one before"
        );
        assert_eq!(general(&info()).len(), 28, "no income line");
    }

    #[test]
    fn the_dialog_grows_past_the_box_by_the_tallest_text_and_four() {
        assert_eq!(growth([], 141.0), 0.0);
        assert_eq!(growth([100.0, 141.0], 141.0), 0.0);
        assert_eq!(growth([200.0, 150.0], 141.0), 63.0);
    }

    #[test]
    fn it_opens_on_general_centred() {
        let screen = screen();
        assert!(format!("{screen:?}").contains("TextMetrics"), "its Debug");
        assert_eq!(screen.tab(), Tab::General);
        assert!(!screen.closed());
        assert_eq!(screen.bounds(), Some(Bounds::at(ORIGIN, 413.0, 227.0)));
        assert_eq!(
            screen.content_bounds(),
            Some(Bounds::at(CONTENT, 405.0, 141.0))
        );
        assert_eq!(
            screen.button_bounds(PlayerInfoButton::Done),
            Some(rect(598.0, 465.0, 99.0, 25.0))
        );
        assert_eq!(screen.problem(), None);
    }

    /// The background pictures among `commands`, in order, each with the
    /// clip it is drawn in.
    fn background(commands: Vec<DrawCommand>) -> Vec<DrawCommand> {
        let ours = |image: &ImageKey| BACKGROUND.contains(image);
        commands
            .into_iter()
            .filter(|command| match command {
                DrawCommand::StretchedPicture { image, .. }
                | DrawCommand::Picture { image, .. } => ours(image),
                DrawCommand::Clipped { command, .. } => matches!(
                    **command,
                    DrawCommand::Picture { ref image, .. }
                    | DrawCommand::StretchedPicture { ref image, .. } if ours(image)
                ),
                _ => false,
            })
            .collect()
    }

    /// Picture `id` stretched to `width` x `height` at `top_left`, cut by
    /// the window `clip`.
    fn stretched(clip: Bounds, id: i16, top_left: Point, width: f32, height: f32) -> DrawCommand {
        DrawCommand::Clipped {
            clip,
            command: Box::new(DrawCommand::StretchedPicture {
                image: ImageKey::picture(id),
                top_left,
                width,
                height,
            }),
        }
    }

    fn cropped_middle(clip: Bounds, top_left: Point) -> DrawCommand {
        DrawCommand::Clipped {
            clip,
            command: Box::new(DrawCommand::Picture {
                image: BACKGROUND[1],
                top_left,
            }),
        }
    }

    fn painted(bounds: Bounds, sizes: &BackgroundSizes) -> Vec<DrawCommand> {
        let mut list = DrawList::new();
        draw_background(&mut list, bounds, sizes);
        list.iter().cloned().collect()
    }

    fn background_in(bounds: Bounds, sizes: &BackgroundSizes) -> Vec<DrawCommand> {
        background(painted(bounds, sizes))
    }

    fn black_first(commands: &[DrawCommand]) -> bool {
        matches!(
            commands[0],
            DrawCommand::Line {
                color: Color::BLACK,
                ..
            }
        )
    }

    #[test]
    fn it_paints_black_then_the_middle_then_the_caps() {
        let window = Bounds::at(ORIGIN, 413.0, 227.0);
        assert_eq!(
            background(drawn(&screen())),
            [
                cropped_middle(window, Point::new(305.0, 310.0)),
                stretched(window, 8518, ORIGIN, 413.0, 40.0),
                stretched(window, 8520, Point::new(305.0, 457.0), 413.0, 40.0),
            ],
            "the stock dialog's span of 147 is under the middle's 215: drawn \
             at its own size from below the upper cap, cut by the window"
        );
        assert!(black_first(&drawn(&screen())), "black first");
    }

    #[test]
    fn a_dialog_shorter_than_the_middle_crops_it_rather_than_squashing_it() {
        let short = Bounds::at(Point::new(10.0, 20.0), 413.0, 200.0);
        assert_eq!(
            background_in(short, &stock()),
            [
                cropped_middle(short, Point::new(10.0, 60.0)),
                stretched(short, 8518, Point::new(10.0, 20.0), 413.0, 40.0),
                stretched(short, 8520, Point::new(10.0, 180.0), 413.0, 40.0),
            ],
            "200 rows: the 120-row span still draws the middle 215 high, clipped"
        );
    }

    #[test]
    fn a_dialog_grown_to_the_middles_height_stretches_it() {
        let tall = PlayerInfo {
            cargo: Some(vec!["line"; 20].join("\r")),
            ..info()
        };
        let screen = screen_of(tall);
        let bounds = screen.bounds().expect("laid out");
        let span = bounds.height() - 80.0;
        assert!(span > 215.0, "{span}");
        assert_eq!(
            background(drawn(&screen))[0],
            stretched(
                bounds,
                8519,
                Point::new(bounds.min.x, bounds.min.y + 40.0),
                413.0,
                span
            )
        );
        let at = Point::new(0.0, 0.0);
        let window = Bounds::at(at, 413.0, 295.0);
        assert_eq!(
            background_in(window, &stock())[0],
            stretched(window, 8519, Point::new(0.0, 40.0), 413.0, 215.0),
            "a span of exactly 215 stretches 1:1"
        );
        let window = Bounds::at(at, 413.0, 294.0);
        assert_eq!(
            background_in(window, &stock())[0],
            cropped_middle(window, Point::new(0.0, 40.0)),
            "a span of 214 crops"
        );
    }

    #[test]
    fn without_the_middle_only_the_caps_are_drawn_over_the_black() {
        let window = Bounds::at(Point::new(0.0, 0.0), 413.0, 227.0);
        let sizes = BackgroundSizes {
            middle: None,
            ..stock()
        };
        let commands = painted(window, &sizes);
        assert!(black_first(&commands), "black first");
        assert_eq!(
            background(commands),
            [
                stretched(window, 8518, Point::new(0.0, 0.0), 413.0, 40.0),
                stretched(window, 8520, Point::new(0.0, 187.0), 413.0, 40.0),
            ],
            "no PICT 8519 (@0x524a0)"
        );
    }

    #[test]
    fn without_the_upper_cap_the_middle_starts_at_the_top() {
        let window = Bounds::at(Point::new(0.0, 0.0), 413.0, 227.0);
        let sizes = BackgroundSizes {
            upper: None,
            ..stock()
        };
        assert_eq!(
            background_in(window, &sizes),
            [
                cropped_middle(window, Point::new(0.0, 0.0)),
                stretched(window, 8520, Point::new(0.0, 187.0), 413.0, 40.0),
            ],
            "a span of 187 from row 0 to the lower cap, under 215: cropped"
        );
        let tall = Bounds::at(Point::new(0.0, 0.0), 413.0, 300.0);
        assert_eq!(
            background_in(tall, &sizes)[0],
            stretched(tall, 8519, Point::new(0.0, 0.0), 413.0, 260.0),
            "a span of 260 from row 0 stretches"
        );
    }

    #[test]
    fn without_the_lower_cap_the_middle_keeps_its_own_height() {
        let window = Bounds::at(Point::new(0.0, 0.0), 413.0, 400.0);
        let sizes = BackgroundSizes {
            lower: None,
            ..stock()
        };
        assert_eq!(
            background_in(window, &sizes),
            [
                stretched(window, 8519, Point::new(0.0, 40.0), 413.0, 215.0),
                stretched(window, 8518, Point::new(0.0, 0.0), 413.0, 40.0),
            ],
            "drawn 215 high below the upper cap, not stretched to the bottom"
        );
    }

    #[test]
    fn a_plug_ins_pictures_are_laid_out_at_their_own_sizes() {
        let sizes = BackgroundSizes {
            upper: Some(sized(300.0, 30.0)),
            middle: Some(sized(380.0, 100.0)),
            lower: Some(sized(320.0, 50.0)),
        };
        let window = Bounds::at(Point::new(10.0, 20.0), 413.0, 227.0);
        assert_eq!(
            background_in(window, &sizes),
            [
                stretched(window, 8519, Point::new(10.0, 50.0), 380.0, 147.0),
                stretched(window, 8518, Point::new(10.0, 20.0), 300.0, 30.0),
                stretched(window, 8520, Point::new(10.0, 197.0), 320.0, 50.0),
            ],
            "each at its own width; the middle stretched over 227 - 30 - 50"
        );
        let short = Bounds::at(Point::new(10.0, 20.0), 413.0, 150.0);
        assert_eq!(
            background_in(short, &sizes)[0],
            cropped_middle(short, Point::new(10.0, 50.0)),
            "a span of 70, under the middle's 100: drawn at its own 380 x 100, cut"
        );
    }

    #[test]
    fn the_screen_draws_the_background_from_the_sizes_it_is_given() {
        let sizes = BackgroundSizes {
            upper: None,
            middle: None,
            lower: Some(sized(320.0, 50.0)),
        };
        let screen = screen_with(info(), sizes);
        let window = Bounds::at(ORIGIN, 413.0, 227.0);
        assert_eq!(
            background(drawn(&screen)),
            [stretched(
                window,
                8520,
                Point::new(305.0, 447.0),
                320.0,
                50.0
            )]
        );
    }

    /// Each picture is its ID less 8000 wide and 30 high; 8519 is
    /// missing.
    struct Pictures;

    impl PictureSizes for Pictures {
        fn picture_size(&self, id: i16) -> Option<(u32, u32)> {
            (id != 8519).then(|| ((id - 8000).unsigned_abs().into(), 30))
        }
    }

    #[test]
    fn the_sizes_are_read_from_the_three_pictures() {
        assert_eq!(
            BackgroundSizes::read(&Pictures),
            BackgroundSizes {
                upper: Some(sized(518.0, 30.0)),
                middle: None,
                lower: Some(sized(520.0, 30.0)),
            }
        );
    }

    #[test]
    fn with_cheats_on_system_legal_status_combat_rating_and_ship_class_show_their_ids() {
        let off = general(&info());
        let on = general(&PlayerInfo {
            cheats: true,
            ..info()
        });
        let mut expected = off.clone();
        for (value, with_id) in [
            ("Sol", "Sol (130)"),
            ("Citizen", "Citizen (5)"),
            (PLACEHOLDER, &format!("{PLACEHOLDER} (12)")),
            ("Shuttle", "Shuttle (128)"),
        ] {
            let placed = expected
                .iter_mut()
                .find(|placed| placed.0 == value)
                .expect("the value is drawn");
            assert_eq!(placed.3, VALUE_COLOR);
            with_id.clone_into(&mut placed.0);
        }
        assert_eq!(on, expected, "only those four change, in place and white");
    }

    #[test]
    fn with_cheats_off_no_value_shows_an_id() {
        let off = general(&info());
        assert!(!info().cheats);
        assert!(
            off.iter()
                .all(|placed| !placed.0.contains(" (130)") && !placed.0.contains(" (128)")),
            "{off:?}"
        );
        assert_eq!(
            off,
            general(&PlayerInfo {
                cheat_ids: CheatIds::default(),
                ..info()
            }),
            "the ids are not read"
        );
    }

    #[test]
    fn an_uninhabited_systems_legal_status_gets_no_id() {
        let uninhabited = PlayerInfo {
            legal_status: "N/A".to_owned(),
            cheat_ids: CheatIds {
                record: None,
                ..info().cheat_ids
            },
            cheats: true,
            ..info()
        };
        let placed = general(&uninhabited);
        assert_eq!(placed[13].0, "N/A", "{placed:?}");
        assert_eq!(placed[9].0, "Sol (130)", "the others still do");
    }

    #[test]
    fn a_tab_click_shows_its_tab_and_draws_it_pressed() {
        let mut screen = screen();
        click_button(&mut screen, PlayerInfoButton::Tab(Tab::Extras));
        assert_eq!(screen.tab(), Tab::Extras);
        assert!(!screen.closed());
        let shown = texts(&screen);
        assert!(
            shown.contains(&"You don't have any extras on your ship".to_owned()),
            "{shown:?}"
        );
        click_button(&mut screen, PlayerInfoButton::Tab(Tab::General));
        assert_eq!(screen.tab(), Tab::General);
    }

    #[test]
    fn tab_and_shift_tab_cycle_and_wrap() {
        let mut screen = screen();
        screen.input(&press(Key::Tab));
        assert_eq!(screen.tab(), Tab::Cargo);
        screen.input(&key(Key::Tab, true, true));
        assert_eq!(screen.tab(), Tab::Extras, "a repeat too");
        screen.input(&key(Key::Tab, false, false));
        assert_eq!(screen.tab(), Tab::Extras, "not a release");
        screen.input(&press(Key::Tab));
        screen.input(&press(Key::Tab));
        assert_eq!(screen.tab(), Tab::General, "wrapped");
        screen.input(&press(Key::Shift));
        screen.input(&press(Key::Tab));
        assert_eq!(screen.tab(), Tab::Honors, "back, wrapped");
        screen.input(&press(Key::Tab));
        assert_eq!(screen.tab(), Tab::Extras);
        screen.input(&key(Key::Shift, false, false));
        screen.input(&press(Key::Tab));
        assert_eq!(screen.tab(), Tab::Honors, "forward again");
        screen.input(&press(Key::Shift));
        screen.release_keys();
        screen.input(&press(Key::Tab));
        assert_eq!(screen.tab(), Tab::General, "let go of Shift");
    }

    #[test]
    fn return_enter_escape_and_p_close_it_but_not_a_repeat_or_release() {
        for closing in [Key::Enter, Key::Escape, PLAYER_INFO_KEY] {
            let mut screen = screen();
            screen.input(&key(closing, true, true));
            screen.input(&key(closing, false, false));
            assert!(!screen.closed(), "{closing:?}: a repeat or release");
            screen.input(&press(closing));
            assert!(screen.closed(), "{closing:?}");
        }
        let mut screen = screen();
        screen.input(&press(Key::Char('x')));
        assert!(!screen.closed(), "another key");
    }

    #[test]
    fn done_closes_it_and_a_release_off_a_button_does_nothing() {
        let mut screen = screen();
        let done = screen
            .button_bounds(PlayerInfoButton::Done)
            .expect("laid out");
        screen.input(&mouse(true, done.center()));
        screen.input(&Input::PointerMoved(Point::new(0.0, 0.0)));
        screen.input(&mouse(false, Point::new(0.0, 0.0)));
        assert!(!screen.closed(), "released off it");
        let extras = screen
            .button_bounds(PlayerInfoButton::Tab(Tab::Extras))
            .expect("laid out");
        screen.input(&mouse(true, done.center()));
        screen.input(&mouse(false, extras.center()));
        assert_eq!(screen.tab(), Tab::General, "released on another");
        assert!(!screen.closed());
        click(&mut screen, Point::new(1.0, 1.0));
        assert!(!screen.closed(), "a click off every button");
        click_button(&mut screen, PlayerInfoButton::Done);
        assert!(screen.closed());
    }

    #[test]
    fn a_held_button_draws_pressed_only_while_the_pointer_is_on_it() {
        let mut screen = screen();
        let done = screen
            .button_bounds(PlayerInfoButton::Done)
            .expect("laid out");
        let pressed = |screen: &PlayerInfoScreen| {
            drawn(screen).into_iter().any(|command| {
                matches!(command, DrawCommand::StretchedPicture { image, top_left, .. }
                    if image == ButtonSkin::NOVA.pressed.left && top_left == done.min)
            })
        };
        assert!(!pressed(&screen));
        screen.input(&mouse(true, done.center()));
        assert!(pressed(&screen));
        screen.input(&Input::PointerMoved(Point::new(0.0, 0.0)));
        assert!(!pressed(&screen));
        screen.input(&Input::PointerMoved(done.center()));
        assert!(pressed(&screen));
        screen.cancel_pointer();
        assert!(!pressed(&screen));
        screen.input(&mouse(false, done.center()));
        assert!(!screen.closed(), "cancelled");
    }

    /// Whether `screen` draws Jettison: its label among the texts.
    fn jettison_drawn(screen: &PlayerInfoScreen) -> bool {
        texts(screen).contains(&JETTISON_LABEL.to_owned())
    }

    #[test]
    fn jettison_is_drawn_and_acts_only_on_cargo_with_cargo_to_jettison() {
        let mut screen = screen();
        assert!(!jettison_drawn(&screen), "on General");
        click_button(&mut screen, PlayerInfoButton::Jettison);
        assert!(!screen.confirm_open(), "its hidden rectangle does nothing");
        screen.input(&press(Key::Tab));
        assert!(jettison_drawn(&screen));
        let mut none = screen_of(PlayerInfo {
            jettisonable: false,
            ..info()
        });
        none.input(&press(Key::Tab));
        assert!(!jettison_drawn(&none));
        click_button(&mut none, PlayerInfoButton::Jettison);
        assert!(!none.confirm_open());
    }

    /// [`screen`] on Cargo with Jettison clicked.
    fn asking() -> PlayerInfoScreen {
        let mut screen = screen();
        screen.input(&press(Key::Tab));
        click_button(&mut screen, PlayerInfoButton::Jettison);
        screen
    }

    #[test]
    fn jettison_asks_291_and_cancel_goes_back() {
        let mut screen = asking();
        assert!(screen.confirm_open());
        let question = screen
            .confirm()
            .and_then(|dialog| dialog.text(QUESTION_ITEM))
            .expect("the question")
            .lines()
            .join(" ");
        assert_eq!(question, "Are you sure you want to jettison your cargo?");
        assert!(texts(&screen).contains(&"OK".to_owned()), "drawn over it");
        let cancel = screen
            .confirm()
            .and_then(|dialog| dialog.item_bounds(NO_ITEM))
            .expect("Cancel");
        click(&mut screen, cancel.center());
        assert!(!screen.confirm_open());
        assert!(!screen.closed());
        assert!(!screen.take_jettison());
    }

    #[test]
    fn ok_jettisons_once_and_closes() {
        let mut screen = asking();
        let ok = screen
            .confirm()
            .and_then(|dialog| dialog.item_bounds(YES_ITEM))
            .expect("OK");
        click(&mut screen, ok.center());
        assert!(screen.closed());
        assert!(screen.take_jettison());
        assert!(!screen.take_jettison(), "once");
    }

    #[test]
    fn in_the_question_return_is_ok_and_escape_does_nothing() {
        let mut screen = asking();
        screen.input(&press(Key::Escape));
        assert!(screen.confirm_open());
        assert!(!screen.closed());
        screen.input(&press(PLAYER_INFO_KEY));
        assert!(!screen.closed(), "the question takes P too");
        screen.input(&press(Key::Tab));
        assert_eq!(screen.tab(), Tab::Cargo, "and Tab");
        screen.input(&press(Key::Enter));
        assert!(screen.closed());
        assert!(screen.take_jettison());
    }

    #[test]
    fn without_the_yes_no_dialog_jettison_answers_no() {
        let mut screen = PlayerInfoScreen::new(
            Ok((template(), Rc::new(MonoMetrics))),
            Err("no DLOG 3002".to_owned()),
            info(),
            ButtonStyle::STOCK,
            stock(),
        );
        screen.input(&press(Key::Tab));
        click_button(&mut screen, PlayerInfoButton::Jettison);
        assert!(!screen.confirm_open());
        assert!(!screen.closed());
        assert!(!screen.take_jettison());
    }

    #[test]
    fn a_tall_cargo_text_grows_the_dialog_and_moves_done_and_jettison() {
        // 20 lines of Geneva 9 (10.8 high): (20 + 2) x 10.8 = 237.6, so
        // the dialog grows by 237.6 - 141 + 4 = 100.6.
        let tall = PlayerInfo {
            cargo: Some(vec!["line"; 20].join("\r")),
            ..info()
        };
        let screen = screen_of(tall);
        let grown = 22.0 * 10.8 - 141.0 + 4.0;
        let bounds = screen.bounds().expect("laid out");
        assert!((bounds.height() - (227.0 + grown)).abs() < 1e-3);
        assert_eq!(bounds.min.y, ((768.0 - bounds.height()) / 2.0).floor());
        let done = screen
            .button_bounds(PlayerInfoButton::Done)
            .expect("laid out");
        assert!((done.min.y - (bounds.min.y + 195.0 + grown)).abs() < 1e-3);
        let jettison = screen
            .button_bounds(PlayerInfoButton::Jettison)
            .expect("laid out");
        assert!((jettison.min.y - (bounds.min.y + 195.0 + grown)).abs() < 1e-3);
        let tab = screen
            .button_bounds(PlayerInfoButton::Tab(Tab::General))
            .expect("laid out");
        assert_eq!(tab.min.y, bounds.min.y + 8.0, "the tabs stay");
        let content = screen.content_bounds().expect("laid out");
        assert!((content.height() - (141.0 + grown)).abs() < 1e-3);
        let wide = PlayerInfo {
            honors: Some("word ".repeat(400)),
            ..info()
        };
        assert!(
            screen_of(wide).bounds().expect("laid out").height() > 227.0,
            "wrapped at the box's width"
        );
    }

    #[test]
    fn a_text_tab_draws_its_text_white_in_a_black_box() {
        let mut screen = screen();
        screen.input(&press(Key::Tab));
        let commands = drawn(&screen);
        // The box: the content box inset 5 at the top, left and right and
        // 10 at the bottom, (314, 315) to (709, 441), filled black.
        assert!(
            commands.contains(&DrawCommand::Line {
                from: Point::new(314.0, 378.0),
                to: Point::new(709.0, 378.0),
                width: 126.0,
                color: Color::BLACK,
            }),
            "{commands:?}"
        );
        let lines: Vec<(String, Point, Color)> = commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text {
                    text,
                    origin,
                    color,
                    size,
                    ..
                } if *size == TEXT_SIZE => Some((text.clone(), *origin, *color)),
                _ => None,
            })
            .collect();
        assert_eq!(
            lines,
            [
                (
                    "Current cargo in your ship:".to_owned(),
                    Point::new(314.0, 315.0),
                    VALUE_COLOR
                ),
                (
                    "Five tons of food.".to_owned(),
                    Point::new(314.0, 315.0 + 2.0 * 10.8),
                    VALUE_COLOR
                ),
            ]
        );
    }

    #[test]
    fn a_cargo_text_wrapping_past_its_box_clips_the_straddling_line() {
        // Geneva 9 is 4.5 a character: the last hard line, 89 characters,
        // fits the content box's 405 but not the text box's 395. Counted
        // at the full width the text is 11 lines, (11 + 2) x 10.8 = 140.4
        // within 141, so the dialog doesn't grow; wrapped in the box it is
        // 12, and the 12th's top, 118.8, is inside the box's 126 and its
        // bottom, 129.6, past it.
        let long = vec!["abcd"; 18].join(" ");
        let mut hard = vec!["line"; 10];
        hard.push(&long);
        let cargo = hard.join("\r");
        let mut screen = screen_of(PlayerInfo {
            cargo: Some(cargo),
            ..info()
        });
        assert_eq!(screen.bounds().expect("laid out").height(), 227.0);
        screen.input(&press(Key::Tab));
        let boxed = Bounds {
            min: Point::new(314.0, 315.0),
            max: Point::new(709.0, 441.0),
        };
        let row = |text: &str, n: f32| DrawCommand::Text {
            text: text.to_owned(),
            font: Font::Geneva,
            origin: Point::new(314.0, 315.0 + n * 10.8),
            size: TEXT_SIZE,
            wrap_width: None,
            color: VALUE_COLOR,
        };
        let commands = drawn(&screen);
        let last_whole = row(&vec!["abcd"; 17].join(" "), 10.0);
        assert!(commands.contains(&last_whole), "{commands:?}");
        let straddling = row("abcd", 11.0);
        assert!(
            commands.contains(&DrawCommand::Clipped {
                clip: boxed,
                command: Box::new(straddling.clone()),
            }),
            "{commands:?}"
        );
        assert!(!commands.contains(&straddling), "not drawn whole");
    }

    #[test]
    fn a_text_tab_with_nothing_draws_its_message_centred_in_grey() {
        let message_at = |info: PlayerInfo, tab: Tab| {
            let mut screen = screen_of(info);
            click_button(&mut screen, PlayerInfoButton::Tab(tab));
            drawn(&screen)
                .into_iter()
                .find_map(|command| match command {
                    DrawCommand::Text {
                        text,
                        origin,
                        color,
                        ..
                    } if color == LABEL_COLOR => Some((text, origin)),
                    _ => None,
                })
        };
        let empty = PlayerInfo {
            cargo: None,
            ..info()
        };
        let centred = |message: &str| {
            let width = message.chars().count() as f32 * 4.5;
            (
                511.5_f32.trunc() - (width / 2.0).trunc(),
                310.0 + 55.0 - 9.0,
            )
        };
        let no_cargo = "You don't have any cargo aboard your ship";
        let (x, top) = centred(no_cargo);
        assert_eq!(
            message_at(empty.clone(), Tab::Cargo),
            Some((no_cargo.to_owned(), Point::new(x, top)))
        );
        let fleet = PlayerInfo {
            fleet: true,
            ..empty
        };
        assert_eq!(
            message_at(fleet, Tab::Cargo).map(|(text, _)| text),
            Some("You don't have any cargo aboard your fleet's ships".to_owned())
        );
        assert_eq!(
            message_at(info(), Tab::Honors).map(|(text, _)| text),
            Some("You don't have any ranks or honors".to_owned())
        );
        let honored = PlayerInfo {
            honors: Some(String::new()),
            ..info()
        };
        assert_eq!(
            message_at(honored, Tab::Honors),
            None,
            "an empty text shows its box"
        );
    }

    #[test]
    fn no_input_makes_a_sound() {
        let mut screen = screen();
        click_button(&mut screen, PlayerInfoButton::Tab(Tab::Cargo));
        click_button(&mut screen, PlayerInfoButton::Jettison);
        let cancel = screen
            .confirm()
            .and_then(|dialog| dialog.item_bounds(NO_ITEM))
            .expect("Cancel");
        click(&mut screen, cancel.center());
        screen.input(&press(Key::Tab));
        click_button(&mut screen, PlayerInfoButton::Done);
        assert_eq!(screen.take_sounds(), Vec::<Sound>::new());
    }

    /// A key that closes the screen or answers its question plays nothing
    /// in the original: `_PlayerInfoFilter` @0x53ae9 calls no sound
    /// routine, and neither does `_MyModalDialog` @0x59954.
    #[test]
    fn keyboard_activation_is_silent_as_the_original() {
        for closing in [Key::Enter, Key::Escape, PLAYER_INFO_KEY] {
            let mut screen = screen();
            assert_eq!(screen.take_sounds(), Vec::<Sound>::new());
            screen.input(&press(closing));
            assert!(screen.closed(), "{closing:?}");
            assert_eq!(screen.take_sounds(), Vec::<Sound>::new(), "{closing:?}");
        }
        let mut screen = asking();
        assert_eq!(screen.take_sounds(), Vec::<Sound>::new());
        screen.input(&press(Key::Enter));
        assert!(screen.take_jettison(), "Return answers the question");
        assert_eq!(screen.take_sounds(), Vec::<Sound>::new(), "Return");
        let mut screen = asking();
        screen.input(&press(Key::Tab));
        screen.input(&press(Key::Space));
        assert!(!screen.confirm_open(), "Space on the focused button");
        assert_eq!(screen.take_sounds(), Vec::<Sound>::new(), "Space");
    }

    #[test]
    fn without_its_template_it_says_why_and_escape_closes_it() {
        let mut screen = PlayerInfoScreen::new(
            Err("no DLOG 1017".to_owned()),
            Ok(yes_no()),
            info(),
            ButtonStyle::STOCK,
            stock(),
        );
        assert_eq!(screen.problem(), Some("no DLOG 1017"));
        assert_eq!(screen.bounds(), None);
        assert!(texts(&screen)[0].contains("no DLOG 1017"));
        screen.input(&press(Key::Tab));
        assert_eq!(screen.tab(), Tab::General);
        click(&mut screen, Point::new(600.0, 470.0));
        assert!(!screen.closed());
        screen.input(&press(Key::Escape));
        assert!(screen.closed());
        let short = DialogTemplate {
            items: template().items[..6].to_vec(),
            ..template()
        };
        let screen = PlayerInfoScreen::new(
            Ok((short, Rc::new(MonoMetrics))),
            Ok(yes_no()),
            info(),
            ButtonStyle::STOCK,
            stock(),
        );
        assert_eq!(screen.problem(), Some("its dialog has no item 7"));
    }
}
