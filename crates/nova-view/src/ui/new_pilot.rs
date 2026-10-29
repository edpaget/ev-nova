//! The New Pilot dialog (`DLOG` 3102, "Create a new pilot:"): the new
//! pilot's name, nickname, gender and Strict Play.
//!
//! The stock dialog asks for a full name, a nickname, Strict Play and a
//! gender. The Full Name field (item 8) and the Nickname field (item 9)
//! take the typed text. They open with the defaults the caller draws
//! ([`NewPilotDialog::with_defaults`]; `nova_sim::new_pilot`), the name
//! focused and selected whole (`SelectDialogItemText(8, 0, 0xfe)`
//! @0x16802), so typing replaces it. Typing goes to the focused field; Tab
//! moves the focus to the other field and selects all of it, as the Dialog
//! Manager does; a click in a field focuses it with the caret at its end
//! and nothing selected (the field places no caret by the pointer). Only
//! the focused field draws its caret or selection. The Strict Play check
//! box (item 4) starts clear and flips at each click, as
//! `_DoNewPilotNameDialog` flips it (@0x16a1c-0x16a8f), the gender control
//! (item 11) switches between Male and Female at each click, OK (item 1,
//! and Return) creates the pilot with the name and nickname as typed (the
//! caller keeps them as the original does), and Cancel (item 2, and
//! Escape) goes back. The pictures are inert. Without the interface file,
//! [`NewPilotDialog::fallback`] lays out the name, the nickname, the
//! gender, OK and Cancel itself; it has no Strict Play box, so its pilots
//! do not play Strict Play.
//!
//! Each field holds [`NAME_LENGTH`] characters, and OK does nothing while
//! the name is blank: the engine's own checks. `_DoNewPilotNameDialog`'s
//! (beeping at a name or nickname over 24 characters, @0x16a00) are not
//! ported yet.
//!
//! The stock gender control is `CNTL` 500, a pop-up menu titled "Gender:"
//! (a 95-unit title) over `MENU` 500, whose items are "Male" and
//! "Female", starting on Male. It is drawn here as its title, then the
//! choice in an outlined box; a click picks the other item rather than
//! opening a menu, which with two items comes to the same. Neither `CNTL`
//! nor `MENU` is read: the strings are the stock ones.
//!
//! The dialog has no stock frame: like the Preferences dialog, it is drawn
//! over a dark backdrop with a 1-unit outline.

use std::rc::Rc;
use std::time::Duration;

use nova_sim::Gender;

use crate::color::Color;
use crate::draw::{DrawList, fill_rect};
use crate::font::Font;
use crate::geometry::{Bounds, Point};
use crate::input::{Input, Key};
use crate::screen::{Screen, ScreenAction};
use crate::text::TextMetrics;

use super::button::{ButtonSkin, ButtonStyle};
use super::dialog::{
    Dialog, DialogEvent, DialogTemplate, ItemSpec, ItemTemplate, Placement, STOCK_WORD, outline,
};
use super::prefs::{BACKDROP, BORDER};
use super::text_field::TextField;
use super::toggle::Toggle;

/// The dialog's `DLOG` (and `DITL`) ID.
pub const NEW_PILOT_DIALOG: i16 = 3102;
/// The OK button's item: it creates the pilot.
pub const OK_ITEM: usize = 1;
/// The Cancel button's item.
pub const CANCEL_ITEM: usize = 2;
/// The Full Name field's item.
pub const NAME_ITEM: usize = 8;
/// The Nickname field's item.
pub const NICKNAME_ITEM: usize = 9;
/// The gender control's item.
pub const GENDER_ITEM: usize = 11;
/// The Strict Play check box's item.
pub const STRICT_PLAY_ITEM: usize = 4;
/// The gender control's title: stock `CNTL` 500's.
pub const GENDER_TITLE: &str = "Gender:";
/// The gender menu's first item, stock `MENU` 500 #1.
pub const MALE: &str = "Male";
/// The gender menu's second item, stock `MENU` 500 #2.
pub const FEMALE: &str = "Female";
/// How wide the gender control's title is: stock `CNTL` 500's title width.
const GENDER_TITLE_WIDTH: f32 = 95.0;
/// How far inside its box the choice is drawn.
const GENDER_INSET: f32 = 4.0;
/// The gender control's text size, in Geneva.
const GENDER_SIZE: f32 = 12.0;
/// The longest name or nickname, in characters: the original's `Str31`.
pub const NAME_LENGTH: usize = 31;
/// What the dialog says when a pilot by the name typed is already saved:
/// the first sentence of `STR#` 140 #9.
pub const NAME_TAKEN: &str = "A pilot file by that name already exists.";
/// How far below the name field a refusal is drawn.
const REFUSAL_GAP: f32 = 4.0;
/// A refusal's size, in Geneva.
const REFUSAL_SIZE: f32 = 10.0;

/// What the player chose.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NewPilotOutcome {
    /// Create a pilot.
    Create {
        /// The name, as typed and not blank.
        name: String,
        /// The nickname, as typed; empty without the field.
        nickname: String,
        /// The gender chosen.
        gender: Gender,
        /// Whether Strict Play was ticked.
        strict_play: bool,
    },
    /// Go back without one.
    Cancel,
}

/// The gender pop-up, drawn in its item's bounds: the title, then the
/// choice in an outlined box.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GenderControl {
    rect: Bounds,
}

impl GenderControl {
    /// The control over `rect`.
    #[must_use]
    pub fn new(rect: Bounds) -> Self {
        Self { rect }
    }

    /// Draws it showing `gender`.
    pub fn draw(&self, gender: Gender, metrics: &(impl TextMetrics + ?Sized), list: &mut DrawList) {
        let rect = self.rect;
        let y = rect.center().y - metrics.line_height(Font::Geneva, GENDER_SIZE) / 2.0;
        let menu_x = rect.min.x + GENDER_TITLE_WIDTH;
        list.text(
            GENDER_TITLE,
            Point::new(rect.min.x, y),
            GENDER_SIZE,
            None,
            Color::WHITE,
        );
        let choice = match gender {
            Gender::Male => MALE,
            Gender::Female => FEMALE,
        };
        list.text(
            choice,
            Point::new(menu_x + GENDER_INSET, y),
            GENDER_SIZE,
            None,
            Color::WHITE,
        );
        let menu = Bounds {
            min: Point::new(menu_x, rect.min.y),
            max: rect.max,
        };
        outline(list, menu, Color::WHITE);
    }
}

/// Which of the dialog's fields has the focus.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Focus {
    /// The Full Name field.
    Name,
    /// The Nickname field.
    Nickname,
}

/// "Create a new pilot:": the name typed, and OK or Cancel.
#[derive(Clone)]
pub struct NewPilotDialog {
    dialog: Dialog,
    field: TextField,
    /// The Nickname field, when the template shows it.
    nickname: Option<TextField>,
    /// The field typing goes to.
    focus: Focus,
    /// The Strict Play check box, when the template shows it.
    strict_play: Option<Toggle>,
    /// The gender control, when the template shows it.
    gender_control: Option<GenderControl>,
    /// The gender chosen.
    gender: Gender,
    metrics: Rc<dyn TextMetrics>,
    outcome: Option<NewPilotOutcome>,
    /// Why the last name was refused, until the name changes.
    refusal: Option<String>,
}

impl std::fmt::Debug for NewPilotDialog {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NewPilotDialog")
            .field("name", &self.name())
            .field("nickname", &self.nickname())
            .field("gender", &self.gender)
            .field("strict_play", &self.strict_play())
            .field("outcome", &self.outcome)
            .field("refusal", &self.refusal)
            .finish_non_exhaustive()
    }
}

impl NewPilotDialog {
    /// The dialog `template` (stock `DLOG` 3102), its buttons labelled in
    /// `style` and its text measured by `metrics`.
    ///
    /// # Errors
    ///
    /// When the template has no name field (item 8).
    pub fn new(
        template: &DialogTemplate,
        style: ButtonStyle,
        metrics: Rc<dyn TextMetrics>,
    ) -> Result<Self, String> {
        if template.items.len() < NAME_ITEM {
            return Err(format!(
                "DITL {NEW_PILOT_DIALOG} has no item {NAME_ITEM} for the pilot's name"
            ));
        }
        let dialog = Dialog::new(template, &[], Rc::clone(&metrics))
            .with_buttons(ButtonSkin::NOVA, style)
            .with_default(Some(OK_ITEM))
            .with_cancel(Some(CANCEL_ITEM));
        let field_rect = dialog.item_bounds(NAME_ITEM).expect("checked above");
        let strict_play = dialog
            .item_shown(STRICT_PLAY_ITEM)
            .then(|| match &template.items[STRICT_PLAY_ITEM - 1].kind {
                ItemSpec::CheckBox(title) => Some(Toggle::new(
                    dialog.item_bounds(STRICT_PLAY_ITEM).expect("an item"),
                    title.clone(),
                    false,
                )),
                _ => None,
            })
            .flatten();
        let nickname = dialog
            .item_shown(NICKNAME_ITEM)
            .then(|| match &template.items[NICKNAME_ITEM - 1].kind {
                ItemSpec::EditText(_) => dialog.item_bounds(NICKNAME_ITEM),
                _ => None,
            })
            .flatten()
            .map(|rect| TextField::new(rect, NAME_LENGTH));
        let gender_control = dialog
            .item_shown(GENDER_ITEM)
            .then(|| dialog.item_bounds(GENDER_ITEM))
            .flatten()
            .map(GenderControl::new);
        Ok(Self {
            dialog,
            field: TextField::new(field_rect, NAME_LENGTH),
            nickname,
            focus: Focus::Name,
            strict_play,
            gender_control,
            gender: Gender::default(),
            metrics,
            outcome: None,
            refusal: None,
        })
    }

    /// The dialog with its fields holding `name` and `nickname`: the
    /// name focused and selected whole, so typing replaces it, and the
    /// nickname unselected until it is focused (`_DoNewPilotNameDialog`
    /// @0x1679c-0x16802).
    #[must_use]
    pub fn with_defaults(mut self, name: &str, nickname: &str) -> Self {
        self.field.set_text(name);
        if let Some(field) = &mut self.nickname {
            field.set_text(nickname);
            field.deselect_to_end();
        }
        self.focus = Focus::Name;
        self
    }

    /// The dialog laid out without the interface file: the title, the
    /// Full Name and Nickname labels and fields, the gender control, and
    /// OK and Cancel, centred.
    #[must_use]
    pub fn fallback(style: ButtonStyle, metrics: Rc<dyn TextMetrics>) -> Self {
        Self::new(&fallback_template(), style, metrics).expect("the fallback has a name field")
    }

    /// The name typed so far.
    #[must_use]
    pub fn name(&self) -> &str {
        self.field.text()
    }

    /// The nickname typed so far; empty without the field.
    #[must_use]
    pub fn nickname(&self) -> &str {
        self.nickname.as_ref().map_or("", TextField::text)
    }

    /// The Nickname field, when the template shows it.
    #[must_use]
    pub fn nickname_field(&self) -> Option<&TextField> {
        self.nickname.as_ref()
    }

    /// Which field has the focus.
    #[must_use]
    pub fn focus(&self) -> Focus {
        self.focus
    }

    /// Gives `focus` the focus, its text selected whole when `select_all`
    /// (Tab) or its caret at its end with nothing selected (a click). The
    /// Nickname without its field leaves the focus where it is.
    fn focus_on(&mut self, focus: Focus, select_all: bool) {
        let field = match focus {
            Focus::Name => &mut self.field,
            Focus::Nickname => match &mut self.nickname {
                Some(field) => field,
                None => return,
            },
        };
        if select_all {
            field.select_first(usize::MAX);
        } else {
            field.deselect_to_end();
        }
        self.focus = focus;
    }

    /// The gender chosen so far: Male until a click switches it.
    #[must_use]
    pub fn gender(&self) -> Gender {
        self.gender
    }

    /// Whether Strict Play is ticked so far: clear until a click ticks
    /// it, and always clear without the box.
    #[must_use]
    pub fn strict_play(&self) -> bool {
        self.strict_play.as_ref().is_some_and(Toggle::on)
    }

    /// The name field.
    #[must_use]
    pub fn field(&self) -> &TextField {
        &self.field
    }

    /// The dialog itself, for its layout.
    #[must_use]
    pub fn dialog(&self) -> &Dialog {
        &self.dialog
    }

    /// What the player chose, once.
    pub fn take_outcome(&mut self) -> Option<NewPilotOutcome> {
        self.outcome.take()
    }

    /// Refuses the name chosen, saying why under the field until the name
    /// changes. The dialog stays open.
    pub fn refuse(&mut self, why: &str) {
        self.refusal = Some(why.to_owned());
    }

    /// Why the name was refused, while it is shown.
    #[must_use]
    pub fn refusal(&self) -> Option<&str> {
        self.refusal.as_deref()
    }
}

/// The built-in "Create a new pilot:": 326 x 213, centred, with OK (1),
/// Cancel (2), the Full Name and Nickname labels (5, 6), their fields (8,
/// 9), the gender control (11) and the title (12), each where stock has
/// it; the other items are parked outside it.
fn fallback_template() -> DialogTemplate {
    let at = |x, y, w, h| Bounds::at(Point::new(x, y), w, h);
    let parked = || ItemTemplate {
        bounds: at(0.0, 400.0, 10.0, 10.0),
        enabled: false,
        kind: ItemSpec::User,
    };
    let mut items: Vec<ItemTemplate> = (0..12).map(|_| parked()).collect();
    items[OK_ITEM - 1] = ItemTemplate {
        bounds: at(238.0, 183.0, 70.0, 20.0),
        enabled: true,
        kind: ItemSpec::Button("OK".to_owned()),
    };
    items[CANCEL_ITEM - 1] = ItemTemplate {
        bounds: at(156.0, 183.0, 70.0, 20.0),
        enabled: true,
        kind: ItemSpec::Button("Cancel".to_owned()),
    };
    items[4] = ItemTemplate {
        bounds: at(54.0, 36.0, 84.0, 16.0),
        enabled: false,
        kind: ItemSpec::StaticText("Full Name:".to_owned()),
    };
    items[5] = ItemTemplate {
        bounds: at(54.0, 59.0, 79.0, 16.0),
        enabled: false,
        kind: ItemSpec::StaticText("Nickname:".to_owned()),
    };
    items[NAME_ITEM - 1] = ItemTemplate {
        bounds: at(149.0, 36.0, 170.0, 16.0),
        enabled: true,
        kind: ItemSpec::EditText(String::new()),
    };
    items[NICKNAME_ITEM - 1] = ItemTemplate {
        bounds: at(149.0, 59.0, 170.0, 16.0),
        enabled: true,
        kind: ItemSpec::EditText(String::new()),
    };
    items[GENDER_ITEM - 1] = ItemTemplate {
        bounds: at(51.0, 92.0, 200.0, 20.0),
        enabled: true,
        kind: ItemSpec::Control(500),
    };
    items[11] = ItemTemplate {
        bounds: at(53.0, 6.0, 160.0, 16.0),
        enabled: false,
        kind: ItemSpec::StaticText("Create a new pilot:".to_owned()),
    };
    DialogTemplate {
        bounds: at(0.0, 0.0, 326.0, 213.0),
        placement: Placement::for_dlog(NEW_PILOT_DIALOG, Some(STOCK_WORD)),
        items,
    }
}

impl Screen for NewPilotDialog {
    /// Typed characters and Backspace go to the focused field, and a
    /// change of the name clears a refusal. Space only types a space. Tab
    /// (its repeats too) focuses the other field, its text selected whole.
    /// Everything else goes to the dialog: a click in a field focuses it, a
    /// click on Strict Play ticks or clears it, a click on the gender
    /// control switches the gender, OK (or Return) creates the pilot when
    /// the name is not blank, and Cancel (or Escape) cancels. It never
    /// quits.
    fn input(&mut self, input: &Input) -> ScreenAction {
        if let Input::Key {
            key: Key::Tab,
            pressed: true,
            ..
        } = input
        {
            let next = match self.focus {
                Focus::Name if self.nickname.is_some() => Focus::Nickname,
                _ => Focus::Name,
            };
            self.focus_on(next, true);
            return ScreenAction::None;
        }
        let typing = matches!(
            input,
            Input::Text(_)
                | Input::Key {
                    key: Key::Backspace | Key::Space,
                    ..
                }
        );
        if typing {
            match (self.focus, &mut self.nickname) {
                (Focus::Nickname, Some(field)) => {
                    field.input(input);
                }
                _ => {
                    if self.field.input(input) {
                        self.refusal = None;
                    }
                }
            }
            return ScreenAction::None;
        }
        match self.dialog.input(input) {
            Some(DialogEvent::Item(OK_ITEM)) => {
                let name = self.field.text();
                if !name.trim().is_empty() {
                    self.outcome = Some(NewPilotOutcome::Create {
                        name: name.to_owned(),
                        nickname: self.nickname().to_owned(),
                        gender: self.gender,
                        strict_play: self.strict_play(),
                    });
                }
            }
            Some(DialogEvent::Item(NAME_ITEM)) => self.focus_on(Focus::Name, false),
            Some(DialogEvent::Item(NICKNAME_ITEM)) => self.focus_on(Focus::Nickname, false),
            Some(DialogEvent::Item(STRICT_PLAY_ITEM)) => {
                if let Some(toggle) = &mut self.strict_play {
                    *toggle = Toggle::new(toggle.rect(), toggle.label(), !toggle.on());
                }
            }
            Some(DialogEvent::Item(GENDER_ITEM)) => {
                self.gender = match self.gender {
                    Gender::Male => Gender::Female,
                    Gender::Female => Gender::Male,
                };
            }
            Some(DialogEvent::Item(CANCEL_ITEM)) => self.outcome = Some(NewPilotOutcome::Cancel),
            _ => {}
        }
        ScreenAction::None
    }

    /// Nothing moves on its own.
    fn tick(&mut self, _dt: Duration) {}

    /// The backdrop and its outline, the dialog, the Strict Play check
    /// box, the name and nickname fields (only the focused one with its
    /// caret or selection), the gender control, and the refusal under the
    /// name in the error colour.
    fn draw(&self, list: &mut DrawList) {
        let bounds = self.dialog.bounds();
        fill_rect(list, bounds, BACKDROP);
        outline(list, bounds, BORDER);
        self.dialog.draw(list);
        if let Some(toggle) = &self.strict_play {
            toggle.draw(false, &self.metrics, list);
        }
        let metrics = &*self.metrics;
        match self.focus {
            Focus::Name => self.field.draw(metrics, list),
            Focus::Nickname => self.field.draw_unfocused(metrics, list),
        }
        if let Some(field) = &self.nickname {
            match self.focus {
                Focus::Nickname => field.draw(metrics, list),
                Focus::Name => field.draw_unfocused(metrics, list),
            }
        }
        if let Some(control) = &self.gender_control {
            control.draw(self.gender, &*self.metrics, list);
        }
        if let Some(why) = &self.refusal {
            let field = self.field.rect();
            list.text(
                why.clone(),
                Point::new(field.min.x, field.max.y + REFUSAL_GAP),
                REFUSAL_SIZE,
                None,
                Color::ERROR,
            );
        }
    }

    fn cancel_pointer(&mut self) {
        self.dialog.cancel_pointer();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::draw::DrawCommand;
    use crate::geometry::Point;
    use crate::input::MouseButton;
    use crate::sound::Sound;
    use crate::text::fixture::MonoMetrics;
    use crate::ui::dialog::{ItemSpec, ItemTemplate, Placement};

    fn rect(x: f32, y: f32, w: f32, h: f32) -> Bounds {
        Bounds::at(Point::new(x, y), w, h)
    }

    fn item(bounds: Bounds, enabled: bool, kind: ItemSpec) -> ItemTemplate {
        ItemTemplate {
            bounds,
            enabled,
            kind,
        }
    }

    /// Stock "Create a new pilot:"'s items, the dialog fixed at (0, 0):
    /// OK (1), Cancel (2), a picture (3), Strict Play (4), the Full Name
    /// and Nickname labels (5, 6), a parked label (7), the Full Name and
    /// Nickname fields (8, 9), a parked field (10), a control (11), the
    /// title (12), a parked control (13) and an icon (14).
    fn stock() -> DialogTemplate {
        let text = |s: &str| ItemSpec::StaticText(s.to_owned());
        let field = || ItemSpec::EditText("Edit Text".to_owned());
        DialogTemplate {
            bounds: rect(0.0, 0.0, 326.0, 213.0),
            placement: Placement::Fixed,
            items: vec![
                item(
                    rect(238.0, 183.0, 70.0, 20.0),
                    true,
                    ItemSpec::Button("OK".into()),
                ),
                item(
                    rect(156.0, 183.0, 70.0, 20.0),
                    true,
                    ItemSpec::Button("Cancel".into()),
                ),
                item(
                    rect(79.0, 141.0, 182.0, 22.0),
                    false,
                    ItemSpec::Picture(129),
                ),
                item(
                    rect(59.0, 122.0, 95.0, 17.0),
                    true,
                    ItemSpec::CheckBox("Strict Play".into()),
                ),
                item(rect(54.0, 36.0, 84.0, 16.0), false, text("Full Name:")),
                item(rect(54.0, 59.0, 79.0, 16.0), false, text("Nickname:")),
                item(rect(49.0, 357.0, 78.0, 16.0), false, text("name3:")),
                item(rect(149.0, 36.0, 170.0, 16.0), true, field()),
                item(rect(149.0, 59.0, 170.0, 16.0), true, field()),
                item(rect(144.0, 357.0, 170.0, 16.0), true, field()),
                item(rect(51.0, 92.0, 200.0, 20.0), true, ItemSpec::Control(500)),
                item(
                    rect(53.0, 6.0, 128.0, 16.0),
                    false,
                    text("Create a new pilot:"),
                ),
                item(rect(85.0, 277.0, 271.0, 20.0), true, ItemSpec::Control(501)),
                item(rect(7.0, 5.0, 32.0, 32.0), false, ItemSpec::Picture(130)),
            ],
        }
    }

    fn dialog() -> NewPilotDialog {
        NewPilotDialog::new(&stock(), ButtonStyle::STOCK, Rc::new(MonoMetrics)).expect("builds")
    }

    fn created(name: &str, gender: Gender) -> NewPilotOutcome {
        NewPilotOutcome::Create {
            name: name.to_owned(),
            nickname: String::new(),
            gender,
            strict_play: false,
        }
    }

    fn key(key: Key) -> Input {
        Input::Key {
            key,
            pressed: true,
            repeat: false,
        }
    }

    fn typed(dialog: &mut NewPilotDialog, text: &str) {
        for c in text.chars() {
            // As the platform sends them: the key, then its character.
            dialog.input(&key(Key::Char(c.to_ascii_lowercase())));
            dialog.input(&Input::Text(c));
        }
    }

    fn click(dialog: &mut NewPilotDialog, at: Point) -> Vec<Sound> {
        let mut sounds = Vec::new();
        for pressed in [true, false] {
            dialog.input(&Input::PointerButton {
                button: MouseButton::Left,
                pressed,
                at,
            });
            sounds.extend(dialog.take_sounds());
        }
        sounds
    }

    fn center(dialog: &NewPilotDialog, item: usize) -> Point {
        dialog.dialog().item_bounds(item).expect("an item").center()
    }

    fn drawn(dialog: &NewPilotDialog) -> Vec<DrawCommand> {
        let mut list = DrawList::new();
        dialog.draw(&mut list);
        list.iter().cloned().collect()
    }

    fn texts(dialog: &NewPilotDialog) -> Vec<String> {
        drawn(dialog)
            .into_iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, .. } => Some(text),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn the_name_field_is_the_full_name_item_and_starts_empty() {
        let dialog = dialog();
        assert_eq!(dialog.name(), "");
        assert_eq!(dialog.field().rect(), rect(149.0, 36.0, 170.0, 16.0));
        assert_eq!(NEW_PILOT_DIALOG, 3102);
        assert_eq!((OK_ITEM, CANCEL_ITEM, NAME_ITEM), (1, 2, 8));
        assert_eq!(NAME_LENGTH, 31);
    }

    #[test]
    fn typing_fills_the_name_and_backspace_deletes() {
        let mut dialog = dialog();
        typed(&mut dialog, "Adap");
        assert_eq!(dialog.name(), "Adap");
        dialog.input(&key(Key::Backspace));
        assert_eq!(dialog.name(), "Ada");
        assert_eq!(dialog.take_outcome(), None);
    }

    #[test]
    fn a_name_holds_31_characters() {
        let mut dialog = dialog();
        typed(&mut dialog, &"x".repeat(40));
        assert_eq!(dialog.name().chars().count(), 31);
    }

    #[test]
    fn return_or_ok_creates_the_pilot_with_the_name_as_typed() {
        let mut dialog = dialog();
        typed(&mut dialog, " Ada ");
        dialog.input(&key(Key::Enter));
        assert_eq!(dialog.take_outcome(), Some(created(" Ada ", Gender::Male)));
        assert_eq!(dialog.take_outcome(), None, "taken once");

        let mut dialog = self::dialog();
        typed(&mut dialog, "Bob");
        let ok = center(&dialog, OK_ITEM);
        assert_eq!(click(&mut dialog, ok), []);
        assert_eq!(dialog.take_outcome(), Some(created("Bob", Gender::Male)));
    }

    #[test]
    fn ok_with_a_blank_name_does_nothing() {
        let mut dialog = dialog();
        dialog.input(&key(Key::Enter));
        assert_eq!(dialog.take_outcome(), None);
        typed(&mut dialog, "   ");
        let ok = center(&dialog, OK_ITEM);
        click(&mut dialog, ok);
        assert_eq!(dialog.take_outcome(), None);
    }

    #[test]
    fn escape_or_cancel_cancels() {
        let mut dialog = dialog();
        typed(&mut dialog, "Ada");
        dialog.input(&key(Key::Escape));
        assert_eq!(dialog.take_outcome(), Some(NewPilotOutcome::Cancel));
        let mut dialog = self::dialog();
        let cancel = center(&dialog, CANCEL_ITEM);
        click(&mut dialog, cancel);
        assert_eq!(dialog.take_outcome(), Some(NewPilotOutcome::Cancel));
    }

    /// Return and Escape play nothing in the original:
    /// `_DoNewPilotNameDialog` runs `ModalDialog` (@0x16956) with the
    /// standard filter, which only flashes the button.
    #[test]
    fn keyboard_activation_is_silent_as_the_original() {
        let mut dialog = dialog();
        typed(&mut dialog, "Ada");
        assert_eq!(dialog.take_sounds(), []);
        dialog.input(&key(Key::Enter));
        assert_eq!(dialog.take_outcome(), Some(created("Ada", Gender::Male)));
        assert_eq!(dialog.take_sounds(), [], "Return");
        dialog.input(&key(Key::Escape));
        assert_eq!(dialog.take_outcome(), Some(NewPilotOutcome::Cancel));
        assert_eq!(dialog.take_sounds(), [], "Escape");
    }

    #[test]
    fn space_types_a_space_and_activates_nothing() {
        let mut dialog = dialog();
        dialog.input(&key(Key::Tab));
        dialog.input(&key(Key::Tab));
        typed(&mut dialog, "A b");
        assert_eq!(dialog.name(), "A b");
        assert_eq!(dialog.take_outcome(), None);
    }

    #[test]
    fn a_refusal_shows_under_the_field_until_the_name_changes() {
        let mut dialog = dialog();
        typed(&mut dialog, "Ada");
        dialog.input(&key(Key::Enter));
        dialog.take_outcome();
        dialog.refuse(NAME_TAKEN);
        assert_eq!(dialog.refusal(), Some(NAME_TAKEN));
        let refusal = drawn(&dialog)
            .into_iter()
            .find_map(|command| match command {
                DrawCommand::Text {
                    text,
                    origin,
                    color,
                    ..
                } if text == NAME_TAKEN => Some((origin, color)),
                _ => None,
            })
            .expect("drawn");
        assert_eq!(refusal, (Point::new(149.0, 56.0), Color::ERROR));
        dialog.input(&key(Key::Enter));
        assert_eq!(
            dialog.take_outcome(),
            Some(created("Ada", Gender::Male)),
            "OK still works"
        );
        assert_eq!(dialog.refusal(), Some(NAME_TAKEN));
        typed(&mut dialog, "2");
        assert_eq!(dialog.refusal(), None);
        assert!(!texts(&dialog).contains(&NAME_TAKEN.to_owned()));
        assert_eq!(NAME_TAKEN, "A pilot file by that name already exists.");
    }

    #[test]
    fn it_draws_a_backdrop_the_dialog_strict_play_and_the_field() {
        let mut dialog = dialog();
        typed(&mut dialog, "Ada");
        let commands = drawn(&dialog);
        let bounds = dialog.dialog().bounds();
        let mut expected = DrawList::new();
        fill_rect(&mut expected, bounds, BACKDROP);
        outline(&mut expected, bounds, BORDER);
        dialog.dialog().draw(&mut expected);
        Toggle::new(rect(59.0, 122.0, 95.0, 17.0), "Strict Play", false).draw(
            false,
            &MonoMetrics,
            &mut expected,
        );
        dialog.field().draw(&MonoMetrics, &mut expected);
        dialog
            .nickname_field()
            .expect("stock has one")
            .draw_unfocused(&MonoMetrics, &mut expected);
        GenderControl::new(rect(51.0, 92.0, 200.0, 20.0)).draw(
            Gender::Male,
            &MonoMetrics,
            &mut expected,
        );
        assert_eq!(commands, expected.iter().cloned().collect::<Vec<_>>());
        let texts = texts(&dialog);
        for shown in [
            "OK",
            "Cancel",
            "Full Name:",
            "Nickname:",
            "Create a new pilot:",
            "Ada",
            "Gender:",
            "Male",
        ] {
            assert!(texts.contains(&shown.to_owned()), "{shown}: {texts:?}");
        }
        assert!(!texts.contains(&"name3:".to_owned()), "parked");
    }

    #[test]
    fn the_nickname_field_is_item_9_and_starts_empty() {
        let dialog = dialog();
        assert_eq!(NICKNAME_ITEM, 9);
        assert_eq!(dialog.nickname(), "");
        assert_eq!(
            dialog.nickname_field().map(TextField::rect),
            Some(rect(149.0, 59.0, 170.0, 16.0))
        );
        assert_eq!(dialog.focus(), Focus::Name);
    }

    #[test]
    fn with_defaults_both_fields_show_them_and_typing_replaces_the_name() {
        let mut dialog = dialog().with_defaults("Shane Merrol", "Hunter");
        assert_eq!(dialog.name(), "Shane Merrol");
        assert_eq!(dialog.nickname(), "Hunter");
        assert_eq!(dialog.focus(), Focus::Name);
        assert!(dialog.field().selected(), "the name is selected whole");
        assert_eq!(dialog.field().selection(), "Shane Merrol".len());
        let nickname = dialog.nickname_field().expect("a field");
        assert!(!nickname.selected(), "the nickname is not");
        typed(&mut dialog, "A");
        assert_eq!(dialog.name(), "A");
        assert_eq!(dialog.nickname(), "Hunter");
    }

    #[test]
    fn tab_focuses_the_other_field_and_selects_all_of_it() {
        let mut dialog = dialog().with_defaults("Shane Merrol", "Hunter");
        dialog.input(&key(Key::Tab));
        assert_eq!(dialog.focus(), Focus::Nickname);
        let nickname = dialog.nickname_field().expect("a field");
        assert_eq!(nickname.selection(), "Hunter".len());
        typed(&mut dialog, "Ace");
        assert_eq!(dialog.nickname(), "Ace");
        assert_eq!(dialog.name(), "Shane Merrol");
        dialog.input(&key(Key::Backspace));
        assert_eq!(dialog.nickname(), "Ac", "Backspace goes to the focus");
        dialog.input(&Input::Key {
            key: Key::Tab,
            pressed: true,
            repeat: true,
        });
        assert_eq!(dialog.focus(), Focus::Name, "a repeat moves it too");
        assert_eq!(dialog.field().selection(), "Shane Merrol".len());
        typed(&mut dialog, "Bo");
        assert_eq!(dialog.name(), "Bo");
        dialog.input(&Input::Key {
            key: Key::Tab,
            pressed: false,
            repeat: false,
        });
        assert_eq!(dialog.focus(), Focus::Name, "a release does nothing");
        assert_eq!(dialog.take_outcome(), None);
    }

    #[test]
    fn a_click_in_a_field_focuses_it_with_the_caret_at_its_end() {
        let mut dialog = dialog().with_defaults("Shane", "Hunter");
        let at = center(&dialog, NICKNAME_ITEM);
        assert_eq!(click(&mut dialog, at), [], "silently");
        assert_eq!(dialog.focus(), Focus::Nickname);
        assert!(!dialog.nickname_field().expect("a field").selected());
        typed(&mut dialog, "s");
        assert_eq!(dialog.nickname(), "Hunters");
        let at = center(&dialog, NAME_ITEM);
        click(&mut dialog, at);
        assert_eq!(dialog.focus(), Focus::Name);
        assert!(!dialog.field().selected());
        typed(&mut dialog, "y");
        assert_eq!(dialog.name(), "Shaney");
        assert_eq!(dialog.take_outcome(), None);
    }

    #[test]
    fn only_the_focused_field_draws_its_caret_or_selection() {
        let mut dialog = dialog().with_defaults("Shane", "Hunter");
        let field_draw = |dialog: &NewPilotDialog, focused: Focus| {
            let mut expected = DrawList::new();
            let nickname = dialog.nickname_field().expect("a field");
            match focused {
                Focus::Name => {
                    dialog.field().draw(&MonoMetrics, &mut expected);
                    nickname.draw_unfocused(&MonoMetrics, &mut expected);
                }
                Focus::Nickname => {
                    dialog.field().draw_unfocused(&MonoMetrics, &mut expected);
                    nickname.draw(&MonoMetrics, &mut expected);
                }
            }
            expected.iter().cloned().collect::<Vec<_>>()
        };
        for focus in [Focus::Name, Focus::Nickname] {
            if dialog.focus() != focus {
                dialog.input(&key(Key::Tab));
            }
            let fields = field_draw(&dialog, focus);
            assert!(
                drawn(&dialog)
                    .windows(fields.len())
                    .any(|window| window == fields),
                "{focus:?}"
            );
        }
    }

    #[test]
    fn ok_creates_the_pilot_with_the_name_and_nickname_as_typed() {
        let mut dialog = dialog().with_defaults("The Ace;x", " Hunter ");
        dialog.input(&key(Key::Enter));
        assert_eq!(
            dialog.take_outcome(),
            Some(NewPilotOutcome::Create {
                name: "The Ace;x".to_owned(),
                nickname: " Hunter ".to_owned(),
                gender: Gender::Male,
                strict_play: false,
            })
        );
    }

    #[test]
    fn a_nickname_holds_31_characters() {
        let mut dialog = dialog();
        dialog.input(&key(Key::Tab));
        typed(&mut dialog, &"x".repeat(40));
        assert_eq!(dialog.nickname().chars().count(), 31);
    }

    #[test]
    fn without_a_nickname_field_tab_keeps_the_name_and_selects_it() {
        let mut template = stock();
        template.items[NICKNAME_ITEM - 1].kind = ItemSpec::User;
        let mut dialog = NewPilotDialog::new(&template, ButtonStyle::STOCK, Rc::new(MonoMetrics))
            .expect("builds")
            .with_defaults("Ada", "Hunter");
        assert_eq!(dialog.nickname_field(), None);
        assert_eq!(dialog.nickname(), "");
        let name = center(&dialog, NAME_ITEM);
        click(&mut dialog, name);
        assert!(!dialog.field().selected(), "a click leaves the caret");
        dialog.input(&key(Key::Tab));
        assert_eq!(dialog.focus(), Focus::Name);
        assert!(dialog.field().selected(), "Tab selects the name again");
        let at = center(&dialog, NICKNAME_ITEM);
        click(&mut dialog, at);
        assert_eq!(dialog.focus(), Focus::Name);
        dialog.input(&key(Key::Enter));
        assert_eq!(dialog.take_outcome(), Some(created("Ada", Gender::Male)));
    }

    #[test]
    fn strict_play_starts_clear_and_a_click_ticks_it_and_another_clears_it() {
        let mut dialog = dialog();
        assert!(!dialog.strict_play());
        assert_eq!(STRICT_PLAY_ITEM, 4);
        let at = center(&dialog, STRICT_PLAY_ITEM);
        assert!(
            click(&mut dialog, at).is_empty(),
            "a check box clicks silently"
        );
        assert!(dialog.strict_play());
        assert_eq!(dialog.take_outcome(), None);
        let mut expected = DrawList::new();
        Toggle::new(rect(59.0, 122.0, 95.0, 17.0), "Strict Play", true).draw(
            false,
            &MonoMetrics,
            &mut expected,
        );
        let ticked: Vec<DrawCommand> = expected.iter().cloned().collect();
        assert!(
            drawn(&dialog)
                .windows(ticked.len())
                .any(|window| window == ticked),
            "drawn ticked"
        );
        click(&mut dialog, at);
        assert!(!dialog.strict_play());
    }

    #[test]
    fn ok_creates_the_pilot_with_strict_play_as_ticked() {
        let mut dialog = dialog();
        let at = center(&dialog, STRICT_PLAY_ITEM);
        click(&mut dialog, at);
        typed(&mut dialog, "Ada");
        dialog.input(&key(Key::Enter));
        assert_eq!(
            dialog.take_outcome(),
            Some(NewPilotOutcome::Create {
                name: "Ada".to_owned(),
                nickname: String::new(),
                gender: Gender::Male,
                strict_play: true,
            })
        );
    }

    #[test]
    fn the_fallback_creates_a_pilot_without_strict_play() {
        let mut dialog = NewPilotDialog::fallback(ButtonStyle::STOCK, Rc::new(MonoMetrics));
        assert!(!dialog.dialog().item_shown(STRICT_PLAY_ITEM), "parked");
        assert!(!dialog.strict_play());
        typed(&mut dialog, "Ada");
        dialog.input(&key(Key::Enter));
        assert_eq!(dialog.take_outcome(), Some(created("Ada", Gender::Male)));
    }

    #[test]
    fn the_gender_starts_on_male_and_a_click_switches_it() {
        let mut dialog = dialog();
        assert_eq!(dialog.gender(), Gender::Male);
        assert_eq!(GENDER_ITEM, 11);
        let at = center(&dialog, GENDER_ITEM);
        assert!(click(&mut dialog, at).is_empty(), "no button sound");
        assert_eq!(dialog.gender(), Gender::Female);
        assert_eq!(dialog.take_outcome(), None);
        click(&mut dialog, at);
        assert_eq!(dialog.gender(), Gender::Male);
    }

    #[test]
    fn ok_creates_the_pilot_with_the_gender_chosen() {
        let mut dialog = dialog();
        let at = center(&dialog, GENDER_ITEM);
        click(&mut dialog, at);
        typed(&mut dialog, "Ada");
        dialog.input(&key(Key::Enter));
        assert_eq!(dialog.take_outcome(), Some(created("Ada", Gender::Female)));
    }

    #[test]
    fn the_gender_control_shows_its_title_then_the_choice() {
        let mut dialog = dialog();
        let shown = |dialog: &NewPilotDialog| {
            drawn(dialog)
                .into_iter()
                .filter_map(|command| match command {
                    DrawCommand::Text {
                        text,
                        origin,
                        size,
                        color,
                        ..
                    } if text == GENDER_TITLE || text == MALE || text == FEMALE => {
                        Some((text, origin, size, color))
                    }
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        // Centred on the control's 20-unit height.
        let y = 92.0 + 10.0 - MonoMetrics.line_height(crate::font::Font::Geneva, 12.0) / 2.0;
        assert_eq!(
            shown(&dialog),
            [
                (
                    "Gender:".to_owned(),
                    Point::new(51.0, y),
                    12.0,
                    Color::WHITE
                ),
                (
                    "Male".to_owned(),
                    Point::new(51.0 + 95.0 + 4.0, y),
                    12.0,
                    Color::WHITE
                ),
            ]
        );
        let at = center(&dialog, GENDER_ITEM);
        click(&mut dialog, at);
        assert_eq!(shown(&dialog)[1].0, "Female");
        assert_eq!((GENDER_TITLE, MALE, FEMALE), ("Gender:", "Male", "Female"));
    }

    #[test]
    fn the_gender_controls_menu_is_outlined_after_its_title() {
        let control = GenderControl::new(rect(51.0, 92.0, 200.0, 20.0));
        let mut list = DrawList::new();
        control.draw(Gender::Female, &MonoMetrics, &mut list);
        let mut expected = DrawList::new();
        outline(
            &mut expected,
            rect(51.0 + 95.0, 92.0, 105.0, 20.0),
            Color::WHITE,
        );
        let lines: Vec<DrawCommand> = list
            .iter()
            .filter(|command| matches!(command, DrawCommand::Line { .. }))
            .cloned()
            .collect();
        assert_eq!(lines, expected.iter().cloned().collect::<Vec<_>>());
    }

    #[test]
    fn a_template_without_a_name_field_is_an_error() {
        let mut short = stock();
        short.items.truncate(7);
        assert_eq!(
            NewPilotDialog::new(&short, ButtonStyle::STOCK, Rc::new(MonoMetrics)).err(),
            Some("DITL 3102 has no item 8 for the pilot's name".to_owned())
        );
    }

    #[test]
    fn the_fallback_has_a_name_field_ok_and_cancel() {
        let mut dialog = NewPilotDialog::fallback(ButtonStyle::STOCK, Rc::new(MonoMetrics));
        let texts = texts(&dialog);
        for shown in [
            "OK",
            "Cancel",
            "Full Name:",
            "Nickname:",
            "Create a new pilot:",
        ] {
            assert!(texts.contains(&shown.to_owned()), "{shown}: {texts:?}");
        }
        let origin = dialog.dialog().bounds().min;
        assert_eq!(
            dialog.nickname_field().map(TextField::rect),
            Some(rect(origin.x + 149.0, origin.y + 59.0, 170.0, 16.0)),
            "the fallback has a nickname where stock has it"
        );
        dialog.input(&key(Key::Tab));
        typed(&mut dialog, "Ace");
        assert_eq!(dialog.nickname(), "Ace");
        dialog.input(&key(Key::Tab));
        dialog.input(&key(Key::Backspace));
        dialog.input(&key(Key::Backspace));
        dialog.input(&key(Key::Backspace));
        typed(&mut dialog, "Ada");
        let ok = center(&dialog, OK_ITEM);
        click(&mut dialog, ok);
        assert_eq!(
            dialog.take_outcome(),
            Some(NewPilotOutcome::Create {
                name: "Ada".to_owned(),
                nickname: "Ace".to_owned(),
                gender: Gender::Male,
                strict_play: false,
            })
        );
        let gender = center(&dialog, GENDER_ITEM);
        click(&mut dialog, gender);
        assert_eq!(
            dialog.gender(),
            Gender::Female,
            "the fallback has the control"
        );
        assert_eq!(
            dialog.dialog().item_bounds(GENDER_ITEM),
            Some(rect(origin.x + 51.0, origin.y + 92.0, 200.0, 20.0)),
            "where stock has it"
        );
        dialog.input(&key(Key::Escape));
        assert_eq!(dialog.take_outcome(), Some(NewPilotOutcome::Cancel));
        let field = dialog.field().rect();
        let bounds = dialog.dialog().bounds();
        assert!(bounds.contains(field.min) && bounds.contains(field.max));
    }

    #[test]
    fn cancelling_the_pointer_abandons_a_click() {
        let mut dialog = dialog();
        typed(&mut dialog, "Ada");
        let ok = center(&dialog, OK_ITEM);
        let button = |pressed| Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at: ok,
        };
        dialog.input(&button(true));
        dialog.cancel_pointer();
        dialog.input(&button(false));
        assert_eq!(dialog.take_outcome(), None);
    }

    #[test]
    fn debug_shows_the_name_the_outcome_and_the_refusal() {
        let mut dialog = dialog();
        typed(&mut dialog, "Ada");
        dialog.refuse(NAME_TAKEN);
        assert_eq!(
            format!("{dialog:?}"),
            format!(
                "NewPilotDialog {{ name: \"Ada\", nickname: \"\", gender: Male, strict_play: false, outcome: None, refusal: Some({NAME_TAKEN:?}), .. }}"
            )
        );
    }

    #[test]
    fn the_built_in_new_pilot_dialog_is_placed_as_center_window_places_dlog_3102() {
        let template = fallback_template();
        assert_eq!(
            template.placement,
            crate::ui::dialog::Placement::CenterWindow
        );
        assert_eq!(
            template.placement,
            crate::ui::dialog::Placement::for_dlog(
                NEW_PILOT_DIALOG,
                Some(crate::ui::dialog::STOCK_WORD)
            ),
            "as the loaded DLOG is placed"
        );
        assert_eq!(
            template.placement.origin(template.bounds),
            crate::geometry::Point::new(349.0, 278.0)
        );
    }
}
