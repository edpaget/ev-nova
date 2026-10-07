//! The comm dialog (`DLOG` 1007, over `PICT` 8511), in which the player
//! talks to a ship it hailed, and the haggle dialog (`DLOG` 1008, over
//! `PICT` 8514), in which it accepts or haggles over a price
//! (`_DoCommDialog`, `_CommDialogUpdate` @0x903f5,
//! `_DrawCommDialogButtons` @0x28c43 and `_DoHaggleDialog` @0x91e67 in
//! the `EV Nova` executable).
//!
//! **The comm dialog** shows a [`HailView`]: the ship's reply in item 10,
//! its picture (`PICT` 5000 + its ID - 128) in item 11, and in item 12
//! "Class: <ship> (<government>)" (`STR#` 2002 #195), with "Status:
//! Hostile" (#196, #174) in red below for a hostile ship, and for the
//! player's own escort "Status: Hired Escort" (#166) and its daily pay,
//! "Pay: <wage> credits per day" (#297, #267), or "Status: Escort"
//! (#168), in bright green (`_CommDialogUpdate` @0x90688-0x90704; the
//! pay line is the original's escort dialog's, shown here, as the help
//! says the comm window shows a hired escort's cost per day). Its buttons
//! stand in a column, [`BUTTON_STEP`] apart, at item 3's left, width and
//! height: the options listed, in order, the first at item 3's top (so
//! the second stands at item 2's place), and "Close Channel" (`STR#` 150
//! #21) at item 1's top, or a step below the last option when that is
//! lower, the frame growing to keep it inside. So Nova's options stand
//! where the original's buttons do, an untalkative ship's lone Greetings
//! leaves item 2's place empty, and an option registered beyond them
//! pushes Close Channel down. A click gives [`CommPress::Option`] (the
//! option's place in the list) or [`CommPress::Close`]; so do the keys
//! (`_CommFilter` @0x908ed): Return, Enter, Escape and E close the
//! channel, and a letter that is an option's hotkey, either case, presses
//! the first such option. The router passes each press to the flight and
//! shows what came of it ([`CommDialog::set_hail`]).
//!
//! **The haggle dialog** asks "Pay me <price> credits." in item 3 (`STR#`
//! 2002 #187, or "Pay us", #188, for a planet), with "Accept Price"
//! (item 1, `STR#` 150 #30) and "Lower Price" (item 2, #31). Return,
//! Enter and A accept, and L lowers; Escape does nothing, as in the
//! original (`_HaggleFilter`).
//!
//! Without the interface file, [`CommDialog::fallback`] and
//! [`HaggleDialog::fallback`] lay out the same items themselves, over a
//! dark backdrop.

use std::rc::Rc;
use std::time::Duration;

use nova_sim::hire::{ESCORT, HIRED_ESCORT, PAY_LABEL, PER_DAY};
use nova_sim::{Haggle, HailView, ShipId};

use crate::color::Color;
use crate::draw::{DrawList, fill_rect};
use crate::font::Font;
use crate::geometry::{Bounds, Point};
use crate::image::ImageKey;
use crate::input::{Input, Key};
use crate::screen::{Screen, ScreenAction};
use crate::sound::Sound;
use crate::text::TextMetrics;

use super::button::{ButtonSkin, ButtonStyle};
use super::dialog::{
    Dialog, DialogEvent, DialogTemplate, ItemSpec, ItemTemplate, Placement, Role, outline,
};
use super::prefs::{BACKDROP, BORDER};

/// The comm dialog's `DLOG` (and `DITL`) ID.
pub const COMM_DIALOG: i16 = 1007;
/// The picture the comm dialog is drawn over.
pub const COMM_PICTURE: ImageKey = ImageKey::picture(8511);
/// "Close Channel".
pub const CLOSE_ITEM: usize = 1;
/// The middle button's place: the second option's.
pub const MIDDLE_ITEM: usize = 2;
/// Greetings' place: the first option's, and the column's left, width
/// and height.
pub const FIRST_ITEM: usize = 3;
/// The ship's reply.
pub const REPLY_ITEM: usize = 10;
/// The ship's picture.
pub const PICTURE_ITEM: usize = 11;
/// The ship's class and status.
pub const CLASS_ITEM: usize = 12;
/// How far apart, in pixels, the buttons in the column stand.
pub const BUTTON_STEP: f32 = 28.0;
/// The first ship picture, `PICT` 5000, for `shïp` 128.
pub const FIRST_SHIP_PICTURE: i16 = 5000;
/// The first ship's ID.
pub const FIRST_SHIP: i16 = 128;
/// `STR#` 150 #21.
pub const CLOSE_CHANNEL: &str = "Close Channel";
/// `STR#` 2002 #195.
pub const CLASS_LABEL: &str = "Class:";
/// `STR#` 2002 #196.
pub const STATUS_LABEL: &str = "Status:";
/// `STR#` 2002 #174.
pub const HOSTILE: &str = "Hostile";
/// The hostile status's colour.
pub const HOSTILE_COLOR: Color = Color::rgba(0xFF, 0x00, 0x00, 255);
/// The colour of the player's escort's status and pay: bright green.
pub const ESCORT_COLOR: Color = Color::rgba(0x00, 0xFF, 0x00, 255);
/// The comm dialog's key that closes the channel, besides Return, Enter
/// and Escape.
pub const CLOSE_KEY: char = 'e';
/// The text's size, in Geneva.
pub const TEXT_SIZE: f32 = 10.0;
/// The text's colour.
pub const TEXT_COLOR: Color = Color::WHITE;

/// The haggle dialog's `DLOG` (and `DITL`) ID.
pub const HAGGLE_DIALOG: i16 = 1008;
/// The picture the haggle dialog is drawn over.
pub const HAGGLE_PICTURE: ImageKey = ImageKey::picture(8514);
/// "Accept Price".
pub const ACCEPT_ITEM: usize = 1;
/// "Lower Price".
pub const LOWER_ITEM: usize = 2;
/// The price asked.
pub const PRICE_ITEM: usize = 3;
/// `STR#` 150 #30.
pub const ACCEPT_PRICE: &str = "Accept Price";
/// `STR#` 150 #31.
pub const LOWER_PRICE: &str = "Lower Price";
/// `STR#` 2002 #187.
pub const PAY_ME: &str = "Pay me";
/// `STR#` 2002 #188.
pub const PAY_US: &str = "Pay us";
/// The haggle dialog's key that accepts, besides Return and Enter.
pub const ACCEPT_KEY: char = 'a';
/// The haggle dialog's key that lowers the price.
pub const LOWER_KEY: char = 'l';

/// A press in the comm dialog.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommPress {
    /// The option at this place in the list, from 0.
    Option(usize),
    /// "Close Channel".
    Close,
}

/// `credits` with its thousands grouped by commas.
#[must_use]
pub fn grouped(credits: i64) -> String {
    let digits = credits.unsigned_abs().to_string();
    let mut out = String::new();
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            out.push(',');
        }
        out.push(digit);
    }
    if credits < 0 {
        out.insert(0, '-');
    }
    out
}

/// What the haggle dialog asks: "Pay me <price> credits." for a ship, or
/// "Pay us …" for a planet.
#[must_use]
pub fn price_text(price: i64, pay_me: bool) -> String {
    let who = if pay_me { PAY_ME } else { PAY_US };
    format!("{who} {} credits.", grouped(price))
}

/// Item 12's class line for `view`: "Class: <ship> (<government>)", or
/// the ship alone for an independent.
#[must_use]
pub fn class_text(view: &HailView) -> String {
    match &view.govt_name {
        Some(govt) => format!("{CLASS_LABEL} {} ({govt})", view.comm_name),
        None => format!("{CLASS_LABEL} {}", view.comm_name),
    }
}

/// A hired escort's pay line for `wage` a day: "Pay: <wage> credits per
/// day" (`STR#` 2002 #297 and #267), "credit" for a wage of 1.
#[must_use]
pub fn pay_text(wage: i64) -> String {
    let unit = if wage == 1 { "credit" } else { "credits" };
    format!("{PAY_LABEL} {} {unit} {PER_DAY}", grouped(wage))
}

/// The status lines item 12 shows below the class for `view`, each with
/// its colour: for the player's escort "Status: Hired Escort" (`STR#`
/// 2002 #166) and its pay, or "Status: Escort" (#168), in bright green;
/// for a hostile ship "Status: Hostile", in red; otherwise none.
#[must_use]
pub fn status_lines(view: &HailView) -> Vec<(String, Color)> {
    match view.escort {
        Some(status) => {
            let what = if status.wage.is_some() {
                HIRED_ESCORT
            } else {
                ESCORT
            };
            let mut lines = vec![(format!("{STATUS_LABEL} {what}"), ESCORT_COLOR)];
            lines.extend(status.wage.map(|wage| (pay_text(wage), ESCORT_COLOR)));
            lines
        }
        None if view.hostile => vec![(format!("{STATUS_LABEL} {HOSTILE}"), HOSTILE_COLOR)],
        None => Vec::new(),
    }
}

/// The picture of `ship`: `PICT` 5000 + its ID - 128.
#[must_use]
pub fn ship_picture(ship: ShipId) -> ImageKey {
    ImageKey::picture(FIRST_SHIP_PICTURE.saturating_add(ship.0.saturating_sub(FIRST_SHIP)))
}

/// Draws `picture` over `bounds`, or without one a dark backdrop with an
/// outline.
fn backdrop(list: &mut DrawList, bounds: Bounds, picture: Option<ImageKey>) {
    if let Some(image) = picture {
        list.stretched_picture(image, bounds.min, bounds.width(), bounds.height());
    } else {
        fill_rect(list, bounds, BACKDROP);
        outline(list, bounds, BORDER);
    }
}

/// The scrolling text role of `text`.
fn text_role(text: String) -> Role {
    Role::ScrollText {
        text,
        font: Font::Geneva,
        size: TEXT_SIZE,
        color: TEXT_COLOR,
    }
}

/// The comm dialog (see the module docs).
#[derive(Clone)]
pub struct CommDialog {
    /// The template the column is laid out on.
    template: DialogTemplate,
    style: ButtonStyle,
    metrics: Rc<dyn TextMetrics>,
    picture: Option<ImageKey>,
    view: HailView,
    dialog: Dialog,
    /// Each option's hotkey, in order.
    keys: Vec<Option<char>>,
    press: Option<CommPress>,
}

impl std::fmt::Debug for CommDialog {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CommDialog")
            .field("view", &self.view)
            .field("dialog", &self.dialog)
            .field("press", &self.press)
            .finish_non_exhaustive()
    }
}

impl CommDialog {
    /// The dialog `template` (stock `DLOG` 1007) showing `view`, its
    /// buttons labelled in `style` and its text measured by `metrics`,
    /// over [`COMM_PICTURE`].
    ///
    /// # Errors
    ///
    /// When the template has fewer than its twelve items.
    pub fn new(
        template: &DialogTemplate,
        view: &HailView,
        style: ButtonStyle,
        metrics: Rc<dyn TextMetrics>,
    ) -> Result<Self, String> {
        Self::built(template, view, style, metrics, Some(COMM_PICTURE))
    }

    /// The dialog laid out without the interface file, as `DLOG` 1007 is,
    /// over a dark backdrop.
    #[must_use]
    pub fn fallback(view: &HailView, style: ButtonStyle, metrics: Rc<dyn TextMetrics>) -> Self {
        Self::built(&comm_template(), view, style, metrics, None)
            .expect("the fallback has every item")
    }

    fn built(
        template: &DialogTemplate,
        view: &HailView,
        style: ButtonStyle,
        metrics: Rc<dyn TextMetrics>,
        picture: Option<ImageKey>,
    ) -> Result<Self, String> {
        if template.items.len() < CLASS_ITEM {
            return Err(format!(
                "DITL {COMM_DIALOG} has {} items, not the comm dialog's {CLASS_ITEM}",
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
            keys: view.options.iter().map(|option| option.key).collect(),
            press: None,
        })
    }

    /// Shows `view`: the reply, the class and status, and the column of
    /// buttons rebuilt for its options.
    pub fn set_hail(&mut self, view: &HailView) {
        self.dialog = layout(&self.template, view, self.style, &self.metrics);
        self.keys = view.options.iter().map(|option| option.key).collect();
        self.view = view.clone();
    }

    /// The press the player made, once.
    pub fn take_press(&mut self) -> Option<CommPress> {
        self.press.take()
    }

    /// The dialog itself, for its layout.
    #[must_use]
    pub fn dialog(&self) -> &Dialog {
        &self.dialog
    }

    /// The item (from 1) the option at `place` (from 0) stands at.
    #[must_use]
    pub fn option_item(&self, place: usize) -> usize {
        option_item(place, self.template.items.len())
    }

    /// The press item `item` (from 1) is, if it is a button.
    fn press_of(&self, item: usize) -> Option<CommPress> {
        if item == CLOSE_ITEM {
            return Some(CommPress::Close);
        }
        (0..self.keys.len())
            .find(|&place| self.option_item(place) == item)
            .map(CommPress::Option)
    }

    /// The press a key is, if any (see the module docs).
    fn key_press(&self, key: Key) -> Option<CommPress> {
        match key {
            Key::Enter | Key::Escape => Some(CommPress::Close),
            Key::Char(c) if c.eq_ignore_ascii_case(&CLOSE_KEY) => Some(CommPress::Close),
            Key::Char(c) => self
                .keys
                .iter()
                .position(|key| key.is_some_and(|key| key.eq_ignore_ascii_case(&c)))
                .map(CommPress::Option),
            _ => None,
        }
    }
}

/// The item (from 1) the option at `place` (from 0) stands at, in a
/// template of `items` items: Greetings' place, the middle button's, then
/// items added after the template's.
fn option_item(place: usize, items: usize) -> usize {
    match place {
        0 => FIRST_ITEM,
        1 => MIDDLE_ITEM,
        later => items + later - 1,
    }
}

/// `template` with the column of buttons for `count` options (see the
/// module docs), and the frame grown to hold it.
#[must_use]
pub fn column(template: &DialogTemplate, count: usize) -> DialogTemplate {
    let mut laid = template.clone();
    let first = template.items[FIRST_ITEM - 1].bounds;
    let close = template.items[CLOSE_ITEM - 1].bounds;
    let place = |index: usize| {
        Bounds::at(
            Point::new(
                first.min.x,
                (index as f32).mul_add(BUTTON_STEP, first.min.y),
            ),
            first.width(),
            first.height(),
        )
    };
    let below = count
        .checked_sub(1)
        .map_or(close.min.y, |last| place(last).min.y + BUTTON_STEP);
    let close_top = close.min.y.max(below);
    laid.items[CLOSE_ITEM - 1].bounds = Bounds::at(
        Point::new(close.min.x, close_top),
        close.width(),
        close.height(),
    );
    let items = template.items.len();
    for index in 0..count {
        let bounds = place(index);
        match index {
            0 | 1 => laid.items[option_item(index, items) - 1].bounds = bounds,
            _ => laid.items.push(ItemTemplate {
                bounds,
                enabled: true,
                kind: ItemSpec::User,
            }),
        }
    }
    laid.bounds = Bounds::at(
        template.bounds.min,
        template.bounds.width(),
        template.bounds.height() + (close_top - close.min.y),
    );
    laid
}

/// The comm dialog for `view` on `template`.
fn layout(
    template: &DialogTemplate,
    view: &HailView,
    style: ButtonStyle,
    metrics: &Rc<dyn TextMetrics>,
) -> Dialog {
    let count = view.options.len();
    let laid = column(template, count);
    let items = template.items.len();
    let mut roles = vec![
        (CLOSE_ITEM, Role::Button(CLOSE_CHANNEL.to_owned())),
        (REPLY_ITEM, text_role(view.reply.clone())),
        (CLASS_ITEM, text_role(class_text(view))),
    ];
    let mut buttons = [CLOSE_ITEM].to_vec();
    for (place, option) in view.options.iter().enumerate() {
        let item = option_item(place, items);
        roles.push((item, Role::Button(option.label.clone())));
        buttons.push(item);
    }
    for unused in [FIRST_ITEM, MIDDLE_ITEM] {
        if !buttons.contains(&unused) {
            roles.push((unused, Role::Hidden));
        }
    }
    let mut enabled = laid;
    for item in buttons {
        if let Some(found) = enabled.items.get_mut(item - 1) {
            found.enabled = true;
        }
    }
    Dialog::new(&enabled, &roles, Rc::clone(metrics))
        .with_buttons(ButtonSkin::NOVA, style)
        .with_default(None)
        .with_cancel(None)
}

/// The built-in comm dialog: `DLOG` 1007's items, 423 x 215, centred;
/// items 4-9, the escort dialog's, parked below it as stock parks them.
fn comm_template() -> DialogTemplate {
    let at = |x, y, w, h| Bounds::at(Point::new(x, y), w, h);
    let user = |bounds, enabled| ItemTemplate {
        bounds,
        enabled,
        kind: ItemSpec::User,
    };
    DialogTemplate {
        bounds: at(0.0, 0.0, 423.0, 215.0),
        placement: Placement::Center,
        items: vec![
            user(at(21.0, 181.0, 166.0, 26.0), true),
            user(at(21.0, 153.0, 166.0, 26.0), true),
            user(at(21.0, 125.0, 166.0, 26.0), true),
            user(at(46.0, 241.0, 200.0, 25.0), true),
            user(at(7.0, 320.0, 200.0, 25.0), true),
            user(at(199.0, 335.0, 200.0, 25.0), true),
            user(at(178.0, 261.0, 200.0, 25.0), true),
            user(at(178.0, 289.0, 200.0, 25.0), true),
            ItemTemplate {
                bounds: at(34.0, 299.0, 112.0, 16.0),
                enabled: false,
                kind: ItemSpec::StaticText("Ship Identifier".to_owned()),
            },
            user(at(11.0, 8.0, 192.0, 58.0), false),
            user(at(216.0, 7.0, 200.0, 200.0), false),
            user(at(40.0, 73.0, 134.0, 46.0), false),
        ],
    }
}

impl Screen for CommDialog {
    /// A key press (not a repeat) that is a press is one; every other
    /// event goes to the dialog, and a button activated is its press. It
    /// never quits.
    fn input(&mut self, input: &Input) -> ScreenAction {
        if let Input::Key {
            key,
            pressed: true,
            repeat,
        } = *input
            && let Some(press) = self.key_press(key)
        {
            if !repeat {
                self.press = Some(press);
            }
            return ScreenAction::None;
        }
        if let Some(DialogEvent::Item(item)) = self.dialog.input(input) {
            self.press = self.press_of(item);
        }
        ScreenAction::None
    }

    /// Nothing moves on its own.
    fn tick(&mut self, _dt: Duration) {}

    /// The picture (or the backdrop), the ship's picture (a person's
    /// `HailPict` in its place), then the dialog
    /// and the ship's status lines ([`status_lines`]), halfway down item
    /// 12, a line apart.
    fn draw(&self, list: &mut DrawList) {
        backdrop(list, self.dialog.bounds(), self.picture);
        if let Some(at) = self.dialog.item_bounds(PICTURE_ITEM) {
            let picture = self
                .view
                .portrait
                .map_or_else(|| ship_picture(self.view.ship), ImageKey::picture);
            list.stretched_picture(picture, at.min, at.width(), at.height());
        }
        self.dialog.draw(list);
        if let Some(at) = self.dialog.item_bounds(CLASS_ITEM) {
            let line_height = self.metrics.line_height(Font::Geneva, TEXT_SIZE);
            for (n, (text, color)) in status_lines(&self.view).into_iter().enumerate() {
                let y = at.min.y + at.height() / 2.0 + line_height * n as f32;
                list.text(
                    text,
                    Point::new(at.min.x, y),
                    TEXT_SIZE,
                    Some(at.width()),
                    color,
                );
            }
        }
    }

    fn cancel_pointer(&mut self) {
        self.dialog.cancel_pointer();
    }

    /// The buttons' sounds as they are clicked.
    fn take_sounds(&mut self) -> Vec<Sound> {
        self.dialog
            .take_sound()
            .map(Sound::Ui)
            .into_iter()
            .collect()
    }
}

/// The haggle dialog (see the module docs).
#[derive(Clone, Debug)]
pub struct HaggleDialog {
    dialog: Dialog,
    picture: Option<ImageKey>,
    choice: Option<Haggle>,
}

impl HaggleDialog {
    /// The dialog `template` (stock `DLOG` 1008) asking `price`, of a ship
    /// when `pay_me`, its buttons labelled in `style` and its text
    /// measured by `metrics`, over [`HAGGLE_PICTURE`].
    ///
    /// # Errors
    ///
    /// When the template has fewer than its three items.
    pub fn new(
        template: &DialogTemplate,
        price: i64,
        pay_me: bool,
        style: ButtonStyle,
        metrics: Rc<dyn TextMetrics>,
    ) -> Result<Self, String> {
        Self::built(
            template,
            price,
            pay_me,
            style,
            metrics,
            Some(HAGGLE_PICTURE),
        )
    }

    /// The dialog laid out without the interface file, as `DLOG` 1008 is,
    /// over a dark backdrop.
    #[must_use]
    pub fn fallback(
        price: i64,
        pay_me: bool,
        style: ButtonStyle,
        metrics: Rc<dyn TextMetrics>,
    ) -> Self {
        Self::built(&haggle_template(), price, pay_me, style, metrics, None)
            .expect("the fallback has every item")
    }

    fn built(
        template: &DialogTemplate,
        price: i64,
        pay_me: bool,
        style: ButtonStyle,
        metrics: Rc<dyn TextMetrics>,
        picture: Option<ImageKey>,
    ) -> Result<Self, String> {
        if template.items.len() < PRICE_ITEM {
            return Err(format!(
                "DITL {HAGGLE_DIALOG} has {} items, not the haggle dialog's {PRICE_ITEM}",
                template.items.len()
            ));
        }
        let roles = [
            (ACCEPT_ITEM, Role::Button(ACCEPT_PRICE.to_owned())),
            (LOWER_ITEM, Role::Button(LOWER_PRICE.to_owned())),
            (PRICE_ITEM, text_role(price_text(price, pay_me))),
        ];
        let mut enabled = template.clone();
        for item in [ACCEPT_ITEM, LOWER_ITEM] {
            enabled.items[item - 1].enabled = true;
        }
        let dialog = Dialog::new(&enabled, &roles, metrics)
            .with_buttons(ButtonSkin::NOVA, style)
            .with_default(Some(ACCEPT_ITEM))
            .with_cancel(None);
        Ok(Self {
            dialog,
            picture,
            choice: None,
        })
    }

    /// Shows `price` asked, of a ship when `pay_me`.
    pub fn set_asking(&mut self, price: i64, pay_me: bool) {
        self.dialog.set_text(PRICE_ITEM, &price_text(price, pay_me));
    }

    /// What the player chose, once.
    pub fn take_choice(&mut self) -> Option<Haggle> {
        self.choice.take()
    }

    /// The dialog itself, for its layout.
    #[must_use]
    pub fn dialog(&self) -> &Dialog {
        &self.dialog
    }

    /// The text shown, a line each.
    #[must_use]
    pub fn lines(&self) -> &[String] {
        self.dialog
            .scroll_text()
            .map_or(&[], super::scroll_text::ScrollText::lines)
    }
}

/// The built-in haggle dialog: `DLOG` 1008's items, 262 x 107, centred.
fn haggle_template() -> DialogTemplate {
    let at = |x, y, w, h| Bounds::at(Point::new(x, y), w, h);
    let user = |bounds, enabled| ItemTemplate {
        bounds,
        enabled,
        kind: ItemSpec::User,
    };
    DialogTemplate {
        bounds: at(0.0, 0.0, 262.0, 107.0),
        placement: Placement::Center,
        items: vec![
            user(at(58.0, 74.0, 146.0, 26.0), true),
            user(at(58.0, 39.0, 146.0, 26.0), true),
            user(at(7.0, 6.0, 248.0, 25.0), false),
        ],
    }
}

impl Screen for HaggleDialog {
    /// A and L (not their repeats) choose; every other event goes to the
    /// dialog, Return and Enter accepting, and a button activated is its
    /// choice. Escape does nothing. It never quits.
    fn input(&mut self, input: &Input) -> ScreenAction {
        if let Input::Key {
            key: Key::Char(c),
            pressed: true,
            repeat,
        } = *input
        {
            let choice = if c.eq_ignore_ascii_case(&ACCEPT_KEY) {
                Some(Haggle::Accept)
            } else if c.eq_ignore_ascii_case(&LOWER_KEY) {
                Some(Haggle::LowerPrice)
            } else {
                None
            };
            if !repeat && choice.is_some() {
                self.choice = choice;
            }
            return ScreenAction::None;
        }
        match self.dialog.input(input) {
            Some(DialogEvent::Item(ACCEPT_ITEM)) => self.choice = Some(Haggle::Accept),
            Some(DialogEvent::Item(LOWER_ITEM)) => self.choice = Some(Haggle::LowerPrice),
            _ => {}
        }
        ScreenAction::None
    }

    /// Nothing moves on its own.
    fn tick(&mut self, _dt: Duration) {}

    /// The picture (or the backdrop), then the dialog.
    fn draw(&self, list: &mut DrawList) {
        backdrop(list, self.dialog.bounds(), self.picture);
        self.dialog.draw(list);
    }

    fn cancel_pointer(&mut self) {
        self.dialog.cancel_pointer();
    }

    /// The buttons' sounds as they are clicked.
    fn take_sounds(&mut self) -> Vec<Sound> {
        self.dialog
            .take_sound()
            .map(Sound::Ui)
            .into_iter()
            .collect()
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use nova_sim::{EscortStatus, HailButton, NpcId};

    use super::*;
    use crate::draw::DrawCommand;
    use crate::input::MouseButton;
    use crate::sound::UiSound;
    use crate::text::fixture::MonoMetrics;

    fn button(label: &str, key: Option<char>) -> HailButton {
        HailButton {
            label: label.to_owned(),
            key,
        }
    }

    /// A friendly Federation cruiser (ship 129) offering Greetings and
    /// Request Assistance.
    fn view() -> HailView {
        HailView {
            npc: NpcId(3),
            ship: ShipId(129),
            reply: "Channel open.".to_owned(),
            comm_name: "Cruiser".to_owned(),
            govt_name: Some("Federation".to_owned()),
            hostile: false,
            options: vec![
                button("Greetings", Some('G')),
                button("Request Assistance", Some('R')),
            ],
            asking: None,
            pay_me: true,
            escort: None,
            portrait: None,
        }
    }

    /// Stock `DLOG` 1007's items, fixed at (0, 0).
    fn stock() -> DialogTemplate {
        DialogTemplate {
            placement: Placement::Fixed,
            ..comm_template()
        }
    }

    fn dialog(view: &HailView) -> CommDialog {
        CommDialog::new(&stock(), view, ButtonStyle::STOCK, Rc::new(MonoMetrics)).expect("builds")
    }

    fn key(key: Key) -> Input {
        Input::Key {
            key,
            pressed: true,
            repeat: false,
        }
    }

    fn click(screen: &mut impl Screen, at: Point) -> Vec<Sound> {
        let mut sounds = Vec::new();
        for pressed in [true, false] {
            screen.input(&Input::PointerButton {
                button: MouseButton::Left,
                pressed,
                at,
            });
            sounds.extend(screen.take_sounds());
        }
        sounds
    }

    fn drawn(screen: &impl Screen) -> Vec<DrawCommand> {
        let mut list = DrawList::new();
        screen.draw(&mut list);
        list.iter().cloned().collect()
    }

    fn texts(screen: &impl Screen) -> Vec<String> {
        drawn(screen)
            .into_iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, .. } => Some(text),
                _ => None,
            })
            .collect()
    }

    fn at(x: f32, y: f32) -> Bounds {
        Bounds::at(Point::new(x, y), 166.0, 26.0)
    }

    #[test]
    fn two_options_stand_at_items_3_and_2_and_close_at_item_1() {
        let dialog = dialog(&view());
        let laid = dialog.dialog();
        assert_eq!(laid.item_bounds(FIRST_ITEM), Some(at(21.0, 125.0)));
        assert_eq!(laid.item_bounds(MIDDLE_ITEM), Some(at(21.0, 153.0)));
        assert_eq!(laid.item_bounds(CLOSE_ITEM), Some(at(21.0, 181.0)));
        assert!(laid.item_shown(MIDDLE_ITEM));
        assert_eq!(laid.bounds().height(), 215.0);
        let labels = texts(&dialog);
        for label in ["Greetings", "Request Assistance", "Close Channel"] {
            assert!(labels.contains(&label.to_owned()), "{label}: {labels:?}");
        }
        assert_eq!(
            (COMM_DIALOG, COMM_PICTURE, BUTTON_STEP),
            (1007, ImageKey::picture(8511), 28.0)
        );
    }

    #[test]
    fn greetings_alone_leaves_the_middle_place_empty() {
        let alone = HailView {
            options: vec![button("Greetings", Some('G'))],
            ..view()
        };
        let dialog = dialog(&alone);
        let laid = dialog.dialog();
        assert_eq!(laid.item_bounds(FIRST_ITEM), Some(at(21.0, 125.0)));
        assert_eq!(laid.item_bounds(CLOSE_ITEM), Some(at(21.0, 181.0)));
        assert!(!laid.item_shown(MIDDLE_ITEM), "item 2's place is empty");
        assert_eq!(laid.bounds().height(), 215.0);
        let none = HailView {
            options: Vec::new(),
            ..view()
        };
        let dialog = self::dialog(&none);
        assert!(!dialog.dialog().item_shown(FIRST_ITEM));
        assert_eq!(
            dialog.dialog().item_bounds(CLOSE_ITEM),
            Some(at(21.0, 181.0))
        );
    }

    #[test]
    fn the_column_follows_item_3_and_its_buttons_work_however_the_template_has_them() {
        let mut moved = stock();
        moved.items[MIDDLE_ITEM - 1].bounds = Bounds::at(Point::new(300.0, 10.0), 50.0, 20.0);
        for item in [CLOSE_ITEM, MIDDLE_ITEM, FIRST_ITEM] {
            moved.items[item - 1].enabled = false;
        }
        let mut dialog = CommDialog::new(&moved, &view(), ButtonStyle::STOCK, Rc::new(MonoMetrics))
            .expect("builds");
        assert_eq!(
            dialog.dialog().item_bounds(MIDDLE_ITEM),
            Some(at(21.0, 153.0))
        );
        for (item, press) in [
            (FIRST_ITEM, CommPress::Option(0)),
            (MIDDLE_ITEM, CommPress::Option(1)),
            (CLOSE_ITEM, CommPress::Close),
        ] {
            let at = dialog.dialog().item_bounds(item).expect("an item").center();
            click(&mut dialog, at);
            assert_eq!(dialog.take_press(), Some(press), "{item}");
        }
    }

    #[test]
    fn a_third_option_stands_at_item_1s_place_and_pushes_close_down() {
        let three = HailView {
            options: vec![
                button("Greetings", Some('G')),
                button("Request Assistance", Some('R')),
                button("Taunt", Some('T')),
            ],
            ..view()
        };
        let dialog = dialog(&three);
        let laid = dialog.dialog();
        assert_eq!(dialog.option_item(2), 13);
        assert_eq!(laid.item_bounds(13), Some(at(21.0, 181.0)));
        assert!(laid.item_shown(13));
        assert_eq!(laid.item_bounds(CLOSE_ITEM), Some(at(21.0, 209.0)));
        assert_eq!(laid.bounds().height(), 243.0, "the frame grows by a step");
        assert!(texts(&dialog).contains(&"Taunt".to_owned()));
    }

    #[test]
    fn a_click_gives_its_press_once() {
        let mut three = dialog(&HailView {
            options: vec![
                button("Greetings", Some('G')),
                button("Request Assistance", Some('R')),
                button("Taunt", None),
            ],
            ..view()
        });
        for (item, press) in [
            (FIRST_ITEM, CommPress::Option(0)),
            (MIDDLE_ITEM, CommPress::Option(1)),
            (13, CommPress::Option(2)),
            (CLOSE_ITEM, CommPress::Close),
        ] {
            let at = three.dialog().item_bounds(item).expect("an item").center();
            let sounds = click(&mut three, at);
            assert_eq!(three.take_press(), Some(press), "{item}");
            assert_eq!(three.take_press(), None, "once");
            assert_eq!(
                sounds,
                [Sound::Ui(UiSound::ButtonDown), Sound::Ui(UiSound::ButtonUp)]
            );
        }
        let at = three
            .dialog()
            .item_bounds(REPLY_ITEM)
            .expect("an item")
            .center();
        click(&mut three, at);
        assert_eq!(three.take_press(), None, "the reply is no button");
    }

    #[test]
    fn return_enter_escape_and_e_close_the_channel() {
        for pressed in [Key::Enter, Key::Escape, Key::Char('e'), Key::Char('E')] {
            let mut dialog = dialog(&view());
            dialog.input(&key(pressed));
            assert_eq!(dialog.take_press(), Some(CommPress::Close), "{pressed:?}");
        }
    }

    #[test]
    fn an_options_hotkey_either_case_presses_the_first_with_it() {
        let both = HailView {
            options: vec![
                button("Greetings", Some('G')),
                button("Request Assistance", Some('R')),
                button("Rant", Some('r')),
            ],
            ..view()
        };
        for (pressed, place) in [('g', 0), ('G', 0), ('r', 1), ('R', 1)] {
            let mut dialog = dialog(&both);
            dialog.input(&key(Key::Char(pressed)));
            assert_eq!(
                dialog.take_press(),
                Some(CommPress::Option(place)),
                "{pressed}"
            );
        }
        let mut dialog = dialog(&both);
        dialog.input(&key(Key::Char('x')));
        assert_eq!(dialog.take_press(), None, "an unkeyed letter");
        dialog.input(&Input::Key {
            key: Key::Char('g'),
            pressed: true,
            repeat: true,
        });
        assert_eq!(dialog.take_press(), None, "a repeat");
        dialog.input(&Input::Key {
            key: Key::Char('g'),
            pressed: false,
            repeat: false,
        });
        assert_eq!(dialog.take_press(), None, "a release");
    }

    #[test]
    fn a_registered_options_label_shows_and_presses_with_no_change_here() {
        let fake = HailView {
            options: vec![button("Greetings", Some('G')), button("Sing", Some('S'))],
            ..view()
        };
        let mut dialog = dialog(&fake);
        assert!(texts(&dialog).contains(&"Sing".to_owned()));
        dialog.input(&key(Key::Char('s')));
        assert_eq!(dialog.take_press(), Some(CommPress::Option(1)));
    }

    #[test]
    fn it_shows_the_reply_class_and_picture() {
        let dialog = dialog(&view());
        let shown = texts(&dialog);
        assert!(shown.contains(&"Channel open.".to_owned()), "{shown:?}");
        assert_eq!(class_text(&view()), "Class: Cruiser (Federation)");
        assert!(shown.contains(&"Class: Cruiser".to_owned()), "{shown:?}");
        assert!(
            shown.contains(&"(Federation)".to_owned()),
            "wrapped: {shown:?}"
        );
        assert!(
            !shown.iter().any(|text| text.contains(HOSTILE)),
            "{shown:?}"
        );
        let pictures: Vec<(ImageKey, Point, f32, f32)> = drawn(&dialog)
            .into_iter()
            .filter_map(|command| match command {
                DrawCommand::StretchedPicture {
                    image,
                    top_left,
                    width,
                    height,
                } => Some((image, top_left, width, height)),
                _ => None,
            })
            .collect();
        assert_eq!(
            pictures[..2],
            [
                (COMM_PICTURE, Point::new(0.0, 0.0), 423.0, 215.0),
                (
                    ImageKey::picture(5001),
                    Point::new(216.0, 7.0),
                    200.0,
                    200.0
                ),
            ]
        );
        let independent = HailView {
            govt_name: None,
            ..view()
        };
        assert_eq!(class_text(&independent), "Class: Cruiser");
    }

    #[test]
    fn a_person_with_a_hail_picture_shows_it_in_place_of_its_ships() {
        let pictures = |view: &HailView| -> Vec<(ImageKey, Point, f32, f32)> {
            drawn(&dialog(view))
                .into_iter()
                .filter_map(|command| match command {
                    DrawCommand::StretchedPicture {
                        image,
                        top_left,
                        width,
                        height,
                    } => Some((image, top_left, width, height)),
                    _ => None,
                })
                .collect()
        };
        let person = HailView {
            portrait: Some(7800),
            ..view()
        };
        assert_eq!(
            pictures(&person)[..2],
            [
                (COMM_PICTURE, Point::new(0.0, 0.0), 423.0, 215.0),
                (
                    ImageKey::picture(7800),
                    Point::new(216.0, 7.0),
                    200.0,
                    200.0
                ),
            ]
        );
        assert_eq!(
            pictures(&view())[1].0,
            ImageKey::picture(5001),
            "without one, its ship's"
        );
    }

    #[test]
    fn a_hostile_ship_shows_its_status_in_red() {
        let hostile = HailView {
            hostile: true,
            ..view()
        };
        let dialog = dialog(&hostile);
        let status: Vec<(String, Point, Color)> = drawn(&dialog)
            .into_iter()
            .filter_map(|command| match command {
                DrawCommand::Text {
                    text,
                    origin,
                    color,
                    ..
                } if text.contains(HOSTILE) => Some((text, origin, color)),
                _ => None,
            })
            .collect();
        assert_eq!(
            status,
            [(
                "Status: Hostile".to_owned(),
                Point::new(40.0, 96.0),
                HOSTILE_COLOR
            )],
            "halfway down item 12"
        );
    }

    /// The status lines `view`'s dialog draws in item 12: each text, its
    /// origin and colour.
    fn status_lines_drawn(view: &HailView) -> Vec<(String, Point, Color)> {
        drawn(&dialog(view))
            .into_iter()
            .filter_map(|command| match command {
                DrawCommand::Text {
                    text,
                    origin,
                    color,
                    ..
                } if text.starts_with(STATUS_LABEL) || text.starts_with(PAY_LABEL) => {
                    Some((text, origin, color))
                }
                _ => None,
            })
            .collect()
    }

    #[test]
    fn a_hired_escort_shows_its_status_and_daily_pay_in_bright_green() {
        let hired = HailView {
            escort: Some(EscortStatus { wage: Some(1000) }),
            ..view()
        };
        let below = 96.0 + MonoMetrics.line_height(Font::Geneva, TEXT_SIZE);
        assert_eq!(
            status_lines_drawn(&hired),
            [
                (
                    "Status: Hired Escort".to_owned(),
                    Point::new(40.0, 96.0),
                    ESCORT_COLOR
                ),
                (
                    "Pay: 1,000 credits per day".to_owned(),
                    Point::new(40.0, below),
                    ESCORT_COLOR
                ),
            ]
        );
        assert_eq!(ESCORT_COLOR, Color::rgba(0x00, 0xFF, 0x00, 255));
    }

    #[test]
    fn an_escort_not_hired_shows_its_status_alone() {
        let captured = HailView {
            escort: Some(EscortStatus { wage: None }),
            hostile: true,
            ..view()
        };
        assert_eq!(
            status_lines_drawn(&captured),
            [(
                "Status: Escort".to_owned(),
                Point::new(40.0, 96.0),
                ESCORT_COLOR
            )],
            "an escort, never hostile"
        );
        assert_eq!(status_lines_drawn(&view()), [], "a ship that is neither");
    }

    #[test]
    fn a_days_pay_of_one_credit_is_singular() {
        assert_eq!(pay_text(1), "Pay: 1 credit per day");
        assert_eq!(pay_text(0), "Pay: 0 credits per day");
        assert_eq!(pay_text(2), "Pay: 2 credits per day");
        assert_eq!(pay_text(175), "Pay: 175 credits per day");
        assert_eq!(pay_text(12_000), "Pay: 12,000 credits per day");
    }

    #[test]
    fn set_hail_rebuilds_the_column_and_the_reply() {
        let mut dialog = dialog(&view());
        let later = HailView {
            reply: "I'm busy.".to_owned(),
            options: vec![button("Greetings", Some('G'))],
            ..view()
        };
        dialog.set_hail(&later);
        assert!(!dialog.dialog().item_shown(MIDDLE_ITEM));
        assert!(texts(&dialog).contains(&"I'm busy.".to_owned()));
        dialog.input(&key(Key::Char('r')));
        assert_eq!(dialog.take_press(), None, "its key is gone");
    }

    #[test]
    fn a_template_short_of_items_is_refused_and_the_fallback_is_centred() {
        let short = DialogTemplate {
            items: comm_template().items[..11].to_vec(),
            ..stock()
        };
        let err = CommDialog::new(&short, &view(), ButtonStyle::STOCK, Rc::new(MonoMetrics))
            .expect_err("short");
        assert!(err.contains("11 items"), "{err}");
        let fallback = CommDialog::fallback(&view(), ButtonStyle::STOCK, Rc::new(MonoMetrics));
        let bounds = fallback.dialog().bounds();
        assert_eq!(bounds.min, Point::new(300.0, 276.0));
        assert!(drawn(&fallback)
            .iter()
            .all(|command| !matches!(command, DrawCommand::StretchedPicture { image, .. } if *image == COMM_PICTURE)));
    }

    #[test]
    fn its_debug_names_it_and_its_hail() {
        let shown = format!("{:?}", dialog(&view()));
        assert!(shown.starts_with("CommDialog"), "{shown}");
        assert!(shown.contains("Channel open."), "{shown}");
    }

    #[test]
    fn credits_are_grouped_by_thousands() {
        assert_eq!(grouped(0), "0");
        assert_eq!(grouped(999), "999");
        assert_eq!(grouped(3000), "3,000");
        assert_eq!(grouped(20_000), "20,000");
        assert_eq!(grouped(1_234_567), "1,234,567");
        assert_eq!(grouped(-1500), "-1,500");
        assert_eq!(price_text(3000, true), "Pay me 3,000 credits.");
        assert_eq!(price_text(2200, false), "Pay us 2,200 credits.");
    }

    #[test]
    fn a_ships_picture_is_5000_on_from_ship_128() {
        assert_eq!(ship_picture(ShipId(128)), ImageKey::picture(5000));
        assert_eq!(ship_picture(ShipId(141)), ImageKey::picture(5013));
    }

    // The haggle dialog.

    fn haggle(price: i64) -> HaggleDialog {
        let template = DialogTemplate {
            placement: Placement::Fixed,
            ..haggle_template()
        };
        HaggleDialog::new(
            &template,
            price,
            true,
            ButtonStyle::STOCK,
            Rc::new(MonoMetrics),
        )
        .expect("builds")
    }

    #[test]
    fn the_haggle_dialog_asks_the_price_with_its_two_buttons() {
        let dialog = haggle(3000);
        assert_eq!(dialog.lines(), ["Pay me 3,000 credits."]);
        let labels = texts(&dialog);
        assert!(labels.contains(&"Accept Price".to_owned()), "{labels:?}");
        assert!(labels.contains(&"Lower Price".to_owned()), "{labels:?}");
        assert_eq!(
            dialog.dialog().item_bounds(ACCEPT_ITEM),
            Some(Bounds::at(Point::new(58.0, 74.0), 146.0, 26.0))
        );
        assert_eq!(
            (HAGGLE_DIALOG, HAGGLE_PICTURE),
            (1008, ImageKey::picture(8514))
        );
        let mut lowered = haggle(3000);
        lowered.set_asking(2200, true);
        assert_eq!(lowered.lines(), ["Pay me 2,200 credits."]);
    }

    #[test]
    fn its_buttons_and_keys_choose_and_escape_does_nothing() {
        for (input, choice) in [
            (key(Key::Enter), Some(Haggle::Accept)),
            (key(Key::Char('a')), Some(Haggle::Accept)),
            (key(Key::Char('A')), Some(Haggle::Accept)),
            (key(Key::Char('l')), Some(Haggle::LowerPrice)),
            (key(Key::Char('L')), Some(Haggle::LowerPrice)),
            (key(Key::Escape), None),
            (key(Key::Char('x')), None),
        ] {
            let mut dialog = haggle(3000);
            dialog.input(&input);
            assert_eq!(dialog.take_choice(), choice, "{input:?}");
        }
        let mut dialog = haggle(3000);
        dialog.input(&Input::Key {
            key: Key::Char('a'),
            pressed: true,
            repeat: true,
        });
        assert_eq!(dialog.take_choice(), None, "a repeat");
        for (item, choice) in [
            (ACCEPT_ITEM, Haggle::Accept),
            (LOWER_ITEM, Haggle::LowerPrice),
        ] {
            let mut dialog = haggle(3000);
            let at = dialog.dialog().item_bounds(item).expect("an item").center();
            click(&mut dialog, at);
            assert_eq!(dialog.take_choice(), Some(choice), "{item}");
            assert_eq!(dialog.take_choice(), None, "once");
        }
    }

    #[test]
    fn a_click_abandoned_presses_nothing() {
        let mut dialog = dialog(&view());
        let at = dialog
            .dialog()
            .item_bounds(CLOSE_ITEM)
            .expect("an item")
            .center();
        for pressed in [true, false] {
            if !pressed {
                dialog.cancel_pointer();
            }
            dialog.input(&Input::PointerButton {
                button: MouseButton::Left,
                pressed,
                at,
            });
        }
        assert_eq!(dialog.take_press(), None);
    }

    #[test]
    fn the_haggle_buttons_work_however_the_template_has_them() {
        let mut template = DialogTemplate {
            placement: Placement::Fixed,
            ..haggle_template()
        };
        for item in &mut template.items {
            item.enabled = false;
        }
        for (item, choice) in [
            (ACCEPT_ITEM, Haggle::Accept),
            (LOWER_ITEM, Haggle::LowerPrice),
        ] {
            let mut dialog = HaggleDialog::new(
                &template,
                3000,
                true,
                ButtonStyle::STOCK,
                Rc::new(MonoMetrics),
            )
            .expect("builds");
            let at = dialog.dialog().item_bounds(item).expect("an item").center();
            click(&mut dialog, at);
            assert_eq!(dialog.take_choice(), Some(choice), "{item}");
        }
    }

    #[test]
    fn a_haggle_button_sounds_as_it_is_clicked_and_an_abandoned_click_chooses_nothing() {
        let mut dialog = haggle(3000);
        let at = dialog
            .dialog()
            .item_bounds(ACCEPT_ITEM)
            .expect("an item")
            .center();
        assert_eq!(
            click(&mut dialog, at),
            [Sound::Ui(UiSound::ButtonDown), Sound::Ui(UiSound::ButtonUp)]
        );
        dialog.take_choice();
        for pressed in [true, false] {
            if !pressed {
                dialog.cancel_pointer();
            }
            dialog.input(&Input::PointerButton {
                button: MouseButton::Left,
                pressed,
                at,
            });
        }
        assert_eq!(dialog.take_choice(), None);
    }

    #[test]
    fn the_haggle_fallback_is_built_in() {
        let fallback =
            HaggleDialog::fallback(3000, false, ButtonStyle::STOCK, Rc::new(MonoMetrics));
        assert_eq!(fallback.lines(), ["Pay us 3,000 credits."]);
        let short = DialogTemplate {
            items: haggle_template().items[..2].to_vec(),
            ..haggle_template()
        };
        assert!(
            HaggleDialog::new(&short, 1, true, ButtonStyle::STOCK, Rc::new(MonoMetrics)).is_err()
        );
    }
}
