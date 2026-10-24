//! The escort dialog (`DLOG` 1022, over `PICT` 8513), which a fleet
//! escort hailed opens in place of the comm dialog (`_DoEscortDialog`
//! @0x951ee, `_EscortDialogUpdate` @0x94a1b, `_EscortFilter` @0x950e9,
//! `_DrawEscortButtons` @0x277a4 and `_TrackEscortButtons` @0x27987 in
//! the `EV Nova` executable).
//!
//! It shows an [`EscortDialogView`] in Geneva 9 ([`TEXT_SIZE`]):
//!
//! - **Item 12, the title** (@0x94b40-0x94c65): on the baseline 12 below
//!   the item's top, at its left, "Hired Escort:" or "Captured Escort:"
//!   (`STR#` 2002 #166, #167) in medium grey ([`LABEL_COLOR`]); then, 3
//!   right of the left, in white, the class's `CommName` on the baseline
//!   27 down and its `Subtitle`, when it has one, on 41.
//! - **Item 10, the info** (@0x94cc1-0x9502d), its lines on the baselines
//!   12, 28 and 42 below the item's top: the upgrade, the sale (only for
//!   an escort that can be sold) and a hired escort's pay. A note ("Will
//!   be upgraded at next shipyard", "This ship class cannot be
//!   upgraded.", "Will be sold off at next shipyard") is light grey
//!   ([`NOTE_COLOR`]) at the item's left; a figure is its label in medium
//!   grey at the left, then the amount and "credit" or "credits" in white
//!   at [`VALUE_COLUMN`] (the cost and price) or [`PAY_COLUMN`] (the pay,
//!   followed by "per day", `STR#` 2002 #267). The amounts are
//!   `_DrawCommaNumber`'s ([`comma_number`]).
//! - **Item 11** shows the class's picture (`PICT` 5000 + its ID - 128).
//! - **Items 1 to 4** are its buttons: "Close Channel" (`STR#` 150 #21),
//!   "Release" (#32), "Upgrade Escort" (#52) or "Cancel Upgrade" (#53)
//!   while marked, and "Sell Escort" (#54) or "Cancel Sale" (#55) while
//!   marked. Upgrade and Sell are greyed when the escort can't be
//!   upgraded or sold. The original returns a greyed button's item to a
//!   click, which its gate then ignores and which only redraws; here a
//!   greyed button takes no click, which shows the same.
//!
//! **Keys** (`_EscortFilter`): a key press, never its repeat, whatever
//! modifiers are held: Return, Enter, Escape and C close the channel, R
//! releases, U upgrades and S sells, either case. A key is passed on even
//! for a greyed button, as the original's is, and the session ignores it.
//!
//! Without the interface file, [`EscortDialog::fallback`] lays out the
//! same items itself, over a dark backdrop.

use std::rc::Rc;
use std::time::Duration;

use nova_sim::escort_dialog::{
    CANCEL_SALE, CANCEL_UPGRADE, CANNOT_BE_UPGRADED, CAPTURED_ESCORT, CREDIT, CREDITS,
    EscortButton, EscortDialogView, RELEASE, SELL_ESCORT, SELL_PRICE, SaleLine, UPGRADE_COST,
    UPGRADE_ESCORT, UpgradeLine, WILL_BE_SOLD, WILL_BE_UPGRADED,
};
use nova_sim::fleet::EscortMark;
use nova_sim::hire::{HIRED_ESCORT, PAY_LABEL, PER_DAY};
use nova_sim::{CreditUnit, ShownWage};

use crate::color::Color;
use crate::draw::DrawList;
use crate::geometry::{Bounds, Point};
use crate::image::ImageKey;
use crate::input::{Input, Key};
use crate::screen::{Screen, ScreenAction};
use crate::spaceport::credits::comma_number;
use crate::text::TextMetrics;

use super::button::{ButtonSkin, ButtonStyle};
use super::comm::{CLOSE_CHANNEL, LABEL_COLOR, TEXT_COLOR, TEXT_SIZE, backdrop, ship_picture};
use super::dialog::{Dialog, DialogEvent, DialogTemplate, ItemSpec, ItemTemplate, Placement, Role};

/// The escort dialog's `DLOG` (and `DITL`) ID.
pub const ESCORT_DIALOG: i16 = 1022;
/// The picture the escort dialog is drawn over (@0x952b3).
pub const ESCORT_PICTURE: ImageKey = ImageKey::picture(8513);
/// "Close Channel".
pub const CLOSE_ITEM: usize = 1;
/// "Release".
pub const RELEASE_ITEM: usize = 2;
/// "Upgrade Escort" or "Cancel Upgrade".
pub const UPGRADE_ITEM: usize = 3;
/// "Sell Escort" or "Cancel Sale".
pub const SELL_ITEM: usize = 4;
/// The info.
pub const INFO_ITEM: usize = 10;
/// The class's picture.
pub const PICTURE_ITEM: usize = 11;
/// The title.
pub const TITLE_ITEM: usize = 12;
/// A note's colour: `_rgbLightGrey` (0xC000 in each channel).
pub const NOTE_COLOR: Color = Color::rgba(0xC0, 0xC0, 0xC0, 255);
/// How far right of item 10's left the cost and the price stand: 0x46
/// (@0x94d9c).
pub const VALUE_COLUMN: f32 = 70.0;
/// How far right of item 10's left the pay stands (@0x94f7b).
pub const PAY_COLUMN: f32 = 30.0;
/// How far right of item 12's left the class's names stand (@0x94bd5).
pub const NAME_INDENT: f32 = 3.0;
/// The title's baselines below item 12's top: the title, the `CommName`
/// and the `Subtitle`.
pub const TITLE_BASELINES: [f32; 3] = [12.0, 27.0, 41.0];
/// The info's baselines below item 10's top: the upgrade, the sale and
/// the pay.
pub const INFO_BASELINES: [f32; 3] = [12.0, 28.0, 42.0];
/// What follows the title (the Pascal string at 0xd97b2).
pub const TITLE_COLON: &str = ":";

/// "credit" or "credits".
fn unit(unit: CreditUnit) -> &'static str {
    match unit {
        CreditUnit::Credit => CREDIT,
        CreditUnit::Credits => CREDITS,
    }
}

/// A figure's amount and unit, the amount as `_DrawCommaNumber` draws it:
/// "6,000 credits".
#[must_use]
pub fn amount_text(shown: ShownWage) -> String {
    format!("{} {}", comma_number(shown.credits), unit(shown.unit))
}

/// A hired escort's pay after its label, as `_EscortDialogUpdate` draws
/// it (@0x94f54-0x9502d): "<amount> <unit> per day" (`STR#` 2002 #267).
#[must_use]
pub fn pay_text(wage: ShownWage) -> String {
    format!("{} {PER_DAY}", amount_text(wage))
}

/// The title's first line: "Hired Escort:" or "Captured Escort:".
#[must_use]
pub fn title_text(view: &EscortDialogView) -> String {
    let title = if view.hired {
        HIRED_ESCORT
    } else {
        CAPTURED_ESCORT
    };
    format!("{title}{TITLE_COLON}")
}

/// One run of the dialog's text: what, where along its line (from the
/// item's left) and in what colour.
#[derive(Clone, Debug, PartialEq)]
pub struct Run {
    /// The text.
    pub text: String,
    /// How far right of the item's left it starts.
    pub x: f32,
    /// Its colour.
    pub color: Color,
}

impl Run {
    fn new(text: impl Into<String>, x: f32, color: Color) -> Self {
        Self {
            text: text.into(),
            x,
            color,
        }
    }
}

/// The info's lines for `view`, each its runs: the upgrade, the sale and
/// the pay, an empty line for one not drawn.
#[must_use]
pub fn info_lines(view: &EscortDialogView) -> [Vec<Run>; 3] {
    let figure = |label: &str, column: f32, value: String| {
        vec![
            Run::new(format!("{label} "), 0.0, LABEL_COLOR),
            Run::new(value, column, TEXT_COLOR),
        ]
    };
    let upgrade = match view.upgrade {
        UpgradeLine::Marked => vec![Run::new(WILL_BE_UPGRADED, 0.0, NOTE_COLOR)],
        UpgradeLine::Cost(cost) => figure(UPGRADE_COST, VALUE_COLUMN, amount_text(cost)),
        UpgradeLine::Cannot => vec![Run::new(CANNOT_BE_UPGRADED, 0.0, NOTE_COLOR)],
    };
    let sale = match view.sale {
        None => Vec::new(),
        Some(SaleLine::Marked) => vec![Run::new(WILL_BE_SOLD, 0.0, NOTE_COLOR)],
        Some(SaleLine::Price(price)) => figure(SELL_PRICE, VALUE_COLUMN, amount_text(price)),
    };
    let pay = view.pay.map_or_else(Vec::new, |wage| {
        vec![
            Run::new(PAY_LABEL, 0.0, LABEL_COLOR),
            Run::new(pay_text(wage), PAY_COLUMN, TEXT_COLOR),
        ]
    });
    [upgrade, sale, pay]
}

/// The title's lines for `view`, each its runs: the title, the
/// `CommName`, and the `Subtitle` when there is one.
#[must_use]
pub fn title_lines(view: &EscortDialogView) -> [Vec<Run>; 3] {
    [
        vec![Run::new(title_text(view), 0.0, LABEL_COLOR)],
        vec![Run::new(view.comm_name.clone(), NAME_INDENT, TEXT_COLOR)],
        view.subtitle.as_ref().map_or_else(Vec::new, |subtitle| {
            vec![Run::new(subtitle.clone(), NAME_INDENT, TEXT_COLOR)]
        }),
    ]
}

/// The Upgrade and Sell buttons' labels for `mark`.
#[must_use]
pub fn mark_labels(mark: Option<EscortMark>) -> (&'static str, &'static str) {
    (
        if mark == Some(EscortMark::Upgrade) {
            CANCEL_UPGRADE
        } else {
            UPGRADE_ESCORT
        },
        if mark == Some(EscortMark::Sell) {
            CANCEL_SALE
        } else {
            SELL_ESCORT
        },
    )
}

/// The press a key is, if any (see the module docs).
#[must_use]
pub fn key_press(key: Key) -> Option<EscortButton> {
    match key {
        Key::Enter | Key::Escape => Some(EscortButton::Close),
        Key::Char(c) => match c.to_ascii_lowercase() {
            'c' => Some(EscortButton::Close),
            'r' => Some(EscortButton::Release),
            'u' => Some(EscortButton::Upgrade),
            's' => Some(EscortButton::Sell),
            _ => None,
        },
        _ => None,
    }
}

/// The escort dialog (see the module docs).
#[derive(Clone)]
pub struct EscortDialog {
    template: DialogTemplate,
    style: ButtonStyle,
    metrics: Rc<dyn TextMetrics>,
    picture: Option<ImageKey>,
    view: EscortDialogView,
    dialog: Dialog,
    press: Option<EscortButton>,
}

impl std::fmt::Debug for EscortDialog {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EscortDialog")
            .field("view", &self.view)
            .field("dialog", &self.dialog)
            .field("press", &self.press)
            .finish_non_exhaustive()
    }
}

impl EscortDialog {
    /// The dialog `template` (stock `DLOG` 1022) showing `view`, its
    /// buttons labelled in `style` and its text measured by `metrics`,
    /// over [`ESCORT_PICTURE`].
    ///
    /// # Errors
    ///
    /// When the template has fewer than its twelve items.
    pub fn new(
        template: &DialogTemplate,
        view: &EscortDialogView,
        style: ButtonStyle,
        metrics: Rc<dyn TextMetrics>,
    ) -> Result<Self, String> {
        Self::built(template, view, style, metrics, Some(ESCORT_PICTURE))
    }

    /// The dialog laid out without the interface file, as `DLOG` 1022 is,
    /// over a dark backdrop.
    #[must_use]
    pub fn fallback(
        view: &EscortDialogView,
        style: ButtonStyle,
        metrics: Rc<dyn TextMetrics>,
    ) -> Self {
        Self::built(&escort_template(), view, style, metrics, None)
            .expect("the fallback has every item")
    }

    fn built(
        template: &DialogTemplate,
        view: &EscortDialogView,
        style: ButtonStyle,
        metrics: Rc<dyn TextMetrics>,
        picture: Option<ImageKey>,
    ) -> Result<Self, String> {
        if template.items.len() < TITLE_ITEM {
            return Err(format!(
                "DITL {ESCORT_DIALOG} has {} items, not the escort dialog's {TITLE_ITEM}",
                template.items.len()
            ));
        }
        let dialog = layout(template, view, style, &metrics);
        Ok(Self {
            template: template.clone(),
            style,
            metrics,
            picture,
            view: view.clone(),
            dialog,
            press: None,
        })
    }

    /// Shows `view`: its lines and its buttons' labels and greying.
    pub fn set_view(&mut self, view: &EscortDialogView) {
        self.dialog = layout(&self.template, view, self.style, &self.metrics);
        self.view = view.clone();
    }

    /// The press the player made, once.
    pub fn take_press(&mut self) -> Option<EscortButton> {
        self.press.take()
    }

    /// The dialog itself, for its layout.
    #[must_use]
    pub fn dialog(&self) -> &Dialog {
        &self.dialog
    }

    /// Draws `lines` in item `item` on `baselines` below its top.
    fn draw_lines(
        &self,
        list: &mut DrawList,
        item: usize,
        lines: &[Vec<Run>; 3],
        baselines: [f32; 3],
    ) {
        let Some(at) = self.dialog.item_bounds(item) else {
            return;
        };
        for (runs, baseline) in lines.iter().zip(baselines) {
            let top = at.min.y + baseline - TEXT_SIZE;
            for run in runs {
                list.text(
                    run.text.clone(),
                    Point::new(at.min.x + run.x, top),
                    TEXT_SIZE,
                    None,
                    run.color,
                );
            }
        }
    }
}

/// The escort dialog for `view` on `template`: the four buttons labelled,
/// Upgrade and Sell greyed when not allowed, items 5 to 9 hidden.
fn layout(
    template: &DialogTemplate,
    view: &EscortDialogView,
    style: ButtonStyle,
    metrics: &Rc<dyn TextMetrics>,
) -> Dialog {
    let (upgrade, sell) = mark_labels(view.mark);
    let mut roles = vec![
        (CLOSE_ITEM, Role::Button(CLOSE_CHANNEL.to_owned())),
        (RELEASE_ITEM, Role::Button(RELEASE.to_owned())),
        (UPGRADE_ITEM, Role::Button(upgrade.to_owned())),
        (SELL_ITEM, Role::Button(sell.to_owned())),
    ];
    roles.extend((5..INFO_ITEM).map(|item| (item, Role::Hidden)));
    let mut enabled = template.clone();
    for item in [CLOSE_ITEM, RELEASE_ITEM, UPGRADE_ITEM, SELL_ITEM] {
        enabled.items[item - 1].enabled = true;
    }
    let mut dialog = Dialog::new(&enabled, &roles, Rc::clone(metrics))
        .with_buttons(ButtonSkin::NOVA, style)
        .with_default(None)
        .with_cancel(None);
    dialog.set_greyed(UPGRADE_ITEM, !view.can_upgrade);
    dialog.set_greyed(SELL_ITEM, !view.can_sell);
    dialog
}

/// The built-in escort dialog: `DLOG` 1022's items, 424 x 259, centred;
/// items 5 to 9 parked below it as stock parks them.
fn escort_template() -> DialogTemplate {
    let at = |x, y, w, h| Bounds::at(Point::new(x, y), w, h);
    let user = |bounds, enabled| ItemTemplate {
        bounds,
        enabled,
        kind: ItemSpec::User,
    };
    DialogTemplate {
        bounds: at(0.0, 0.0, 424.0, 259.0),
        placement: Placement::Center,
        items: vec![
            user(at(29.0, 225.0, 146.0, 26.0), true),
            user(at(29.0, 197.0, 146.0, 26.0), true),
            user(at(29.0, 141.0, 146.0, 26.0), true),
            user(at(29.0, 169.0, 146.0, 26.0), true),
            user(at(5.0, 304.0, 200.0, 25.0), true),
            user(at(213.0, 316.0, 200.0, 25.0), true),
            user(at(226.0, 309.0, 200.0, 25.0), true),
            user(at(226.0, 337.0, 200.0, 25.0), true),
            ItemTemplate {
                bounds: at(38.0, 315.0, 112.0, 16.0),
                enabled: false,
                kind: ItemSpec::StaticText("Ship Identifier".to_owned()),
            },
            user(at(14.0, 9.0, 192.0, 58.0), false),
            user(at(217.0, 30.0, 200.0, 200.0), false),
            user(at(14.0, 79.0, 192.0, 52.0), false),
        ],
    }
}

impl Screen for EscortDialog {
    /// A key press (not a repeat) that is a press is one; every other
    /// event goes to the dialog, and a button activated is its press. It
    /// never quits.
    fn input(&mut self, input: &Input) -> ScreenAction {
        if let Input::Key {
            key,
            pressed: true,
            repeat,
        } = *input
            && let Some(press) = key_press(key)
        {
            if !repeat {
                self.press = Some(press);
            }
            return ScreenAction::None;
        }
        self.press = match self.dialog.input(input) {
            Some(DialogEvent::Item(CLOSE_ITEM)) => Some(EscortButton::Close),
            Some(DialogEvent::Item(RELEASE_ITEM)) => Some(EscortButton::Release),
            Some(DialogEvent::Item(UPGRADE_ITEM)) => Some(EscortButton::Upgrade),
            Some(DialogEvent::Item(SELL_ITEM)) => Some(EscortButton::Sell),
            _ => self.press,
        };
        ScreenAction::None
    }

    /// Nothing moves on its own.
    fn tick(&mut self, _dt: Duration) {}

    /// The picture (or the backdrop), the class's picture, the buttons,
    /// then the title and the info.
    fn draw(&self, list: &mut DrawList) {
        backdrop(list, self.dialog.bounds(), self.picture);
        if let Some(at) = self.dialog.item_bounds(PICTURE_ITEM) {
            list.stretched_picture(
                ship_picture(self.view.ship),
                at.min,
                at.width(),
                at.height(),
            );
        }
        self.dialog.draw(list);
        self.draw_lines(list, TITLE_ITEM, &title_lines(&self.view), TITLE_BASELINES);
        self.draw_lines(list, INFO_ITEM, &info_lines(&self.view), INFO_BASELINES);
    }

    fn cancel_pointer(&mut self) {
        self.dialog.cancel_pointer();
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use nova_sim::ShipId;

    use super::*;
    use crate::draw::DrawCommand;
    use crate::input::MouseButton;
    use crate::text::fixture::MonoMetrics;

    /// A hired Cruiser (ship 129), upgradable for 6,000, paid 1,000 a day.
    fn hired() -> EscortDialogView {
        EscortDialogView {
            ship: ShipId(129),
            hired: true,
            comm_name: "Cruiser".to_owned(),
            subtitle: Some("Heavy Warship".to_owned()),
            upgrade: UpgradeLine::Cost(ShownWage::kept(6_000)),
            sale: None,
            pay: Some(ShownWage::kept(1_000)),
            can_upgrade: true,
            can_sell: false,
            mark: None,
        }
    }

    /// A captured Shuttle (ship 128) that can't be upgraded, selling for
    /// 499.
    fn captured() -> EscortDialogView {
        EscortDialogView {
            ship: ShipId(128),
            hired: false,
            comm_name: "Shuttle".to_owned(),
            subtitle: None,
            upgrade: UpgradeLine::Cannot,
            sale: Some(SaleLine::Price(ShownWage::kept(499))),
            pay: None,
            can_upgrade: false,
            can_sell: true,
            mark: None,
        }
    }

    /// Stock `DLOG` 1022's items, fixed at (0, 0).
    fn stock() -> DialogTemplate {
        DialogTemplate {
            placement: Placement::Fixed,
            ..escort_template()
        }
    }

    fn dialog(view: &EscortDialogView) -> EscortDialog {
        EscortDialog::new(&stock(), view, ButtonStyle::STOCK, Rc::new(MonoMetrics)).expect("builds")
    }

    fn drawn(screen: &impl Screen) -> Vec<DrawCommand> {
        let mut list = DrawList::new();
        screen.draw(&mut list);
        list.iter().cloned().collect()
    }

    /// Each text drawn, with its origin and colour.
    fn texts(screen: &impl Screen) -> Vec<(String, Point, Color)> {
        drawn(screen)
            .into_iter()
            .filter_map(|command| match command {
                DrawCommand::Text {
                    text,
                    origin,
                    color,
                    size,
                    ..
                } if size == TEXT_SIZE => Some((text, origin, color)),
                _ => None,
            })
            .collect()
    }

    /// Every text drawn, the buttons' labels included.
    fn labels(screen: &impl Screen) -> Vec<String> {
        drawn(screen)
            .into_iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, .. } => Some(text),
                _ => None,
            })
            .collect()
    }

    fn text(text: &str, x: f32, y: f32, color: Color) -> (String, Point, Color) {
        (text.to_owned(), Point::new(x, y), color)
    }

    fn key(key: Key, repeat: bool) -> Input {
        Input::Key {
            key,
            pressed: true,
            repeat,
        }
    }

    fn click(screen: &mut EscortDialog, item: usize) {
        let at = screen.dialog().item_bounds(item).expect("an item").center();
        for pressed in [true, false] {
            screen.input(&Input::PointerButton {
                button: MouseButton::Left,
                pressed,
                at,
            });
        }
    }

    #[test]
    fn its_items_stand_where_ditl_1022_puts_them() {
        let dialog = dialog(&hired());
        let laid = dialog.dialog();
        let button = |x, y| Bounds::at(Point::new(x, y), 146.0, 26.0);
        assert_eq!(laid.item_bounds(CLOSE_ITEM), Some(button(29.0, 225.0)));
        assert_eq!(laid.item_bounds(RELEASE_ITEM), Some(button(29.0, 197.0)));
        assert_eq!(laid.item_bounds(UPGRADE_ITEM), Some(button(29.0, 141.0)));
        assert_eq!(laid.item_bounds(SELL_ITEM), Some(button(29.0, 169.0)));
        for item in 5..INFO_ITEM {
            assert!(!laid.item_shown(item), "item {item}");
        }
        assert_eq!(
            (laid.bounds().width(), laid.bounds().height()),
            (424.0, 259.0)
        );
        assert_eq!(
            (ESCORT_DIALOG, ESCORT_PICTURE),
            (1022, ImageKey::picture(8513))
        );
    }

    #[test]
    fn a_hired_escort_shows_its_title_upgrade_cost_and_pay() {
        // Item 12's top is 79, item 10's 9; a baseline less 9 is a top.
        assert_eq!(
            texts(&dialog(&hired()))
                .into_iter()
                .filter(|(_, origin, _)| origin.x < 200.0 && origin.y < 140.0)
                .collect::<Vec<_>>(),
            [
                text("Hired Escort:", 14.0, 82.0, LABEL_COLOR),
                text("Cruiser", 17.0, 97.0, TEXT_COLOR),
                text("Heavy Warship", 17.0, 111.0, TEXT_COLOR),
                text("Upgrade Cost: ", 14.0, 12.0, LABEL_COLOR),
                text("6,000 credits", 84.0, 12.0, TEXT_COLOR),
                text("Pay:", 14.0, 42.0, LABEL_COLOR),
                text("1,000 credits per day", 44.0, 42.0, TEXT_COLOR),
            ]
        );
        assert_eq!(LABEL_COLOR, Color::rgba(0x80, 0x80, 0x80, 255));
    }

    #[test]
    fn a_captured_escort_shows_its_price_and_that_it_cannot_be_upgraded() {
        assert_eq!(
            texts(&dialog(&captured()))
                .into_iter()
                .filter(|(_, origin, _)| origin.x < 200.0 && origin.y < 140.0)
                .collect::<Vec<_>>(),
            [
                text("Captured Escort:", 14.0, 82.0, LABEL_COLOR),
                text("Shuttle", 17.0, 97.0, TEXT_COLOR),
                text(CANNOT_BE_UPGRADED, 14.0, 12.0, NOTE_COLOR),
                text("Sell Price: ", 14.0, 28.0, LABEL_COLOR),
                text("499 credits", 84.0, 28.0, TEXT_COLOR),
            ]
        );
        assert_eq!(NOTE_COLOR, Color::rgba(0xC0, 0xC0, 0xC0, 255));
    }

    #[test]
    fn a_mark_shows_its_note_and_names_its_button_cancel() {
        let upgrading = EscortDialogView {
            upgrade: UpgradeLine::Marked,
            mark: Some(EscortMark::Upgrade),
            ..hired()
        };
        let lines = info_lines(&upgrading);
        assert_eq!(lines[0], [Run::new(WILL_BE_UPGRADED, 0.0, NOTE_COLOR)]);
        let selling = EscortDialogView {
            sale: Some(SaleLine::Marked),
            mark: Some(EscortMark::Sell),
            ..captured()
        };
        assert_eq!(
            info_lines(&selling)[1],
            [Run::new(WILL_BE_SOLD, 0.0, NOTE_COLOR)]
        );
        assert_eq!(mark_labels(None), ("Upgrade Escort", "Sell Escort"));
        assert_eq!(
            mark_labels(Some(EscortMark::Upgrade)),
            ("Cancel Upgrade", "Sell Escort")
        );
        assert_eq!(
            mark_labels(Some(EscortMark::Sell)),
            ("Upgrade Escort", "Cancel Sale")
        );
        let labels = labels(&dialog(&upgrading));
        for label in ["Close Channel", "Release", "Cancel Upgrade", "Sell Escort"] {
            assert!(labels.contains(&label.to_owned()), "{label}: {labels:?}");
        }
    }

    #[test]
    fn each_button_gives_its_press_and_a_greyed_one_takes_no_click() {
        let mut open = dialog(&hired());
        for (item, press) in [
            (CLOSE_ITEM, EscortButton::Close),
            (RELEASE_ITEM, EscortButton::Release),
            (UPGRADE_ITEM, EscortButton::Upgrade),
        ] {
            click(&mut open, item);
            assert_eq!(open.take_press(), Some(press), "item {item}");
        }
        click(&mut open, SELL_ITEM);
        assert_eq!(open.take_press(), None, "Sell is greyed for a hired escort");
        let mut open = dialog(&captured());
        click(&mut open, UPGRADE_ITEM);
        assert_eq!(open.take_press(), None, "Upgrade is greyed");
        click(&mut open, SELL_ITEM);
        assert_eq!(open.take_press(), Some(EscortButton::Sell));
    }

    #[test]
    fn its_keys_press_once_whatever_the_case_and_never_on_a_repeat() {
        let mut open = dialog(&captured());
        for (pressed, press) in [
            (Key::Enter, EscortButton::Close),
            (Key::Escape, EscortButton::Close),
            (Key::Char('c'), EscortButton::Close),
            (Key::Char('C'), EscortButton::Close),
            (Key::Char('r'), EscortButton::Release),
            (Key::Char('R'), EscortButton::Release),
            (Key::Char('u'), EscortButton::Upgrade),
            (Key::Char('U'), EscortButton::Upgrade),
            (Key::Char('s'), EscortButton::Sell),
            (Key::Char('S'), EscortButton::Sell),
        ] {
            open.input(&key(pressed, false));
            assert_eq!(open.take_press(), Some(press), "{pressed:?}");
            open.input(&key(pressed, true));
            assert_eq!(open.take_press(), None, "{pressed:?} repeating");
        }
        open.input(&key(Key::Char('x'), false));
        assert_eq!(open.take_press(), None);
    }

    #[test]
    fn set_view_relabels_and_regreys_the_buttons() {
        let mut open = dialog(&captured());
        open.set_view(&EscortDialogView {
            sale: Some(SaleLine::Marked),
            mark: Some(EscortMark::Sell),
            ..captured()
        });
        let labels = labels(&open);
        assert!(labels.contains(&"Cancel Sale".to_owned()), "{labels:?}");
        assert!(labels.contains(&WILL_BE_SOLD.to_owned()), "{labels:?}");
        open.set_view(&hired());
        click(&mut open, SELL_ITEM);
        assert_eq!(open.take_press(), None, "now greyed");
    }

    #[test]
    fn the_amounts_read_as_draw_comma_number_draws_them() {
        assert_eq!(amount_text(ShownWage::kept(1)), "1 credit");
        assert_eq!(amount_text(ShownWage::kept(0)), "0 credits");
        assert_eq!(amount_text(ShownWage::kept(1_234_567)), "1.23M credits");
        for (credits, unit, text) in [
            (1, CreditUnit::Credit, "1 credit per day"),
            (1, CreditUnit::Credits, "1 credits per day"),
            (-1, CreditUnit::Credits, "-1 credits per day"),
            (12_000, CreditUnit::Credits, "12,000 credits per day"),
            (1_000_000, CreditUnit::Credits, "1.00M credits per day"),
        ] {
            assert_eq!(pay_text(ShownWage { credits, unit }), text);
        }
    }

    #[test]
    fn the_fallback_lays_out_the_same_items_over_a_backdrop() {
        let fallback = EscortDialog::fallback(&hired(), ButtonStyle::STOCK, Rc::new(MonoMetrics));
        assert_eq!(fallback.dialog().bounds().width(), 424.0);
        assert!(
            !drawn(&fallback)
                .iter()
                .any(|command| matches!(command, DrawCommand::StretchedPicture { image, .. } if *image == ESCORT_PICTURE)),
            "no stock picture"
        );
        assert!(
            drawn(&dialog(&hired()))
                .iter()
                .any(|command| matches!(command, DrawCommand::StretchedPicture { image, .. } if *image == ESCORT_PICTURE)),
            "the stock picture"
        );
        assert!(
            EscortDialog::new(
                &DialogTemplate {
                    items: Vec::new(),
                    ..stock()
                },
                &hired(),
                ButtonStyle::STOCK,
                Rc::new(MonoMetrics)
            )
            .is_err()
        );
    }
}
