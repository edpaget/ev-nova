//! The plunder dialog (`DLOG` 1011, over `PICT` 8515), in which the player
//! takes what is on board a ship boarded, and the captured-ship
//! assignment dialog (`DLOG` 1018, over `PICT` 8516), in which the player
//! makes a ship captured an escort or its own (`_DoPlunderDialog` and
//! `_DoCapturedShipAssignmentDialog` in the `EV Nova` executable).
//!
//! The plunder dialog's buttons are the original's (`STR#` 150 #35 and
//! #40-44): Abort (item 1, and Return, Enter and Escape), Cargo (2),
//! Credits (3), Ammo (4), Energy (6) and Capture Ship (7), each greyed
//! while what it takes is not on board. Item 5 says what is (`STR#` 2002
//! #109-112): the cargo's tons and good, the ammunition's rounds and
//! outfit, and the capture odds. A click gives its [`Take`]
//! ([`PlunderDialog::take_take`]); the router passes it to the flight and
//! refreshes the dialog ([`PlunderDialog::set_plunder`]).
//!
//! The assignment dialog asks `STR#` 2002 #118's question in item 3, with
//! "Use As My Ship" (item 1, `STR#` 150 #47) and "Use As Escort" (item 2,
//! #46); a click gives its [`Assignment`].
//!
//! Without the interface file, [`PlunderDialog::fallback`] and
//! [`AssignmentDialog::fallback`] lay out the same items themselves, over
//! a dark backdrop.

use std::rc::Rc;
use std::time::Duration;

use nova_sim::{Assignment, PlunderView, Take};

use crate::color::Color;
use crate::draw::{DrawList, fill_rect};
use crate::font::Font;
use crate::geometry::{Bounds, Point};
use crate::image::ImageKey;
use crate::input::Input;
use crate::screen::{Screen, ScreenAction};
use crate::sound::Sound;
use crate::text::TextMetrics;

use super::button::{ButtonSkin, ButtonStyle};
use super::dialog::{
    Dialog, DialogEvent, DialogTemplate, ItemSpec, ItemTemplate, Placement, Role, outline,
};
use super::prefs::{BACKDROP, BORDER};

/// The plunder dialog's `DLOG` (and `DITL`) ID.
pub const PLUNDER_DIALOG: i16 = 1011;
/// The picture the plunder dialog is drawn over.
pub const PLUNDER_PICTURE: ImageKey = ImageKey::picture(8515);
/// The plunder dialog's text: what is on board.
pub const TEXT_ITEM: usize = 5;
/// The plunder dialog's buttons: each item, its label and its take.
pub const TAKE_BUTTONS: [(usize, &str, Take); 6] = [
    (1, "Abort", Take::Abort),
    (2, "Cargo", Take::Cargo),
    (3, "Credits", Take::Credits),
    (4, "Ammo", Take::Ammo),
    (6, "Energy", Take::Energy),
    (7, "Capture Ship", Take::Capture),
];
/// `STR#` 2002 #109.
pub const PROMPT: &str = "Select what to plunder from this ship:";
/// `STR#` 2002 #110.
pub const CARGO_LABEL: &str = "Cargo:";
/// `STR#` 2002 #111.
pub const AMMO_LABEL: &str = "Ammo:";
/// `STR#` 2002 #112.
pub const ODDS_LABEL: &str = "Capture Odds:";
/// `STR#` 2002 #396: nothing of a kind on board.
pub const NONE: &str = "N/A";
/// The text's size, in Geneva.
pub const TEXT_SIZE: f32 = 10.0;
/// The text's colour.
pub const TEXT_COLOR: Color = Color::WHITE;

/// The assignment dialog's `DLOG` (and `DITL`) ID.
pub const ASSIGNMENT_DIALOG: i16 = 1018;
/// The picture the assignment dialog is drawn over.
pub const ASSIGNMENT_PICTURE: ImageKey = ImageKey::picture(8516);
/// "Use As My Ship".
pub const MY_SHIP_ITEM: usize = 1;
/// "Use As Escort".
pub const ESCORT_ITEM: usize = 2;
/// The question.
pub const QUESTION_ITEM: usize = 3;
/// `STR#` 150 #47.
pub const MY_SHIP_LABEL: &str = "Use As My Ship";
/// `STR#` 150 #46.
pub const ESCORT_LABEL: &str = "Use As Escort";
/// `STR#` 2002 #118.
pub const QUESTION: &str = "Do you want to use this ship as an escort, or would you rather trade \
                            places with its captain and use it as your own ship?";

/// What the plunder dialog shows: what is on board, with the cargo's
/// good and the ammunition's outfit named.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlunderShown {
    /// What is on board.
    pub view: PlunderView,
    /// The cargo's good's name, if it has one.
    pub good: Option<String>,
    /// The ammunition outfit's name, if it has one.
    pub outfit: Option<String>,
}

/// Item 5's text for `shown`: the prompt, then the cargo, the ammunition
/// and the capture odds, a line each.
#[must_use]
pub fn plunder_text(shown: &PlunderShown) -> String {
    let view = &shown.view;
    let cargo = view.cargo.map_or_else(
        || NONE.to_owned(),
        |(_, tons)| {
            let unit = if tons == 1 { "ton" } else { "tons" };
            format!(
                "{tons} {unit} of {}",
                shown.good.as_deref().unwrap_or("cargo")
            )
        },
    );
    let ammo = view.ammo.map_or_else(
        || NONE.to_owned(),
        |(_, rounds)| format!("{rounds} {}", shown.outfit.as_deref().unwrap_or("rounds")),
    );
    format!(
        "{PROMPT}\r\r{CARGO_LABEL} {cargo}\r{AMMO_LABEL} {ammo}\r{ODDS_LABEL} {}%",
        view.odds
    )
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

/// `template` with each of `items` enabled, so a click activates it.
fn enabling(template: &DialogTemplate, items: impl IntoIterator<Item = usize>) -> DialogTemplate {
    let mut template = template.clone();
    for item in items {
        if let Some(found) = template.items.get_mut(item - 1) {
            found.enabled = true;
        }
    }
    template
}

/// The plunder dialog (see the module docs).
#[derive(Clone, Debug)]
pub struct PlunderDialog {
    dialog: Dialog,
    picture: Option<ImageKey>,
    take: Option<Take>,
}

impl PlunderDialog {
    /// The dialog `template` (stock `DLOG` 1011) showing `shown`, its
    /// buttons labelled in `style` and its text measured by `metrics`,
    /// over [`PLUNDER_PICTURE`].
    ///
    /// # Errors
    ///
    /// When the template has fewer than its seven items.
    pub fn new(
        template: &DialogTemplate,
        shown: &PlunderShown,
        style: ButtonStyle,
        metrics: Rc<dyn TextMetrics>,
    ) -> Result<Self, String> {
        Self::built(template, shown, style, metrics, Some(PLUNDER_PICTURE))
    }

    /// The dialog laid out without the interface file, as `DLOG` 1011 is,
    /// over a dark backdrop.
    #[must_use]
    pub fn fallback(
        shown: &PlunderShown,
        style: ButtonStyle,
        metrics: Rc<dyn TextMetrics>,
    ) -> Self {
        Self::built(&plunder_template(), shown, style, metrics, None)
            .expect("the fallback has every item")
    }

    fn built(
        template: &DialogTemplate,
        shown: &PlunderShown,
        style: ButtonStyle,
        metrics: Rc<dyn TextMetrics>,
        picture: Option<ImageKey>,
    ) -> Result<Self, String> {
        let last = TAKE_BUTTONS
            .iter()
            .map(|&(item, ..)| item)
            .max()
            .unwrap_or(0);
        if template.items.len() < last {
            return Err(format!(
                "DITL {PLUNDER_DIALOG} has {} items, not the plunder dialog's {last}",
                template.items.len()
            ));
        }
        let mut roles: Vec<(usize, Role)> = TAKE_BUTTONS
            .iter()
            .map(|&(item, label, _)| (item, Role::Button(label.to_owned())))
            .collect();
        roles.push((
            TEXT_ITEM,
            Role::ScrollText {
                text: plunder_text(shown),
                font: Font::Geneva,
                size: TEXT_SIZE,
                color: TEXT_COLOR,
            },
        ));
        let template = enabling(template, TAKE_BUTTONS.iter().map(|&(item, ..)| item));
        let first = TAKE_BUTTONS[0].0;
        let dialog = Dialog::new(&template, &roles, metrics)
            .with_buttons(ButtonSkin::NOVA, style)
            .with_default(Some(first))
            .with_cancel(Some(first));
        let mut plunder = Self {
            dialog,
            picture,
            take: None,
        };
        plunder.set_plunder(shown);
        Ok(plunder)
    }

    /// Shows `shown`: item 5's text, and each button greyed while what
    /// it takes is not on board.
    pub fn set_plunder(&mut self, shown: &PlunderShown) {
        self.dialog.set_text(TEXT_ITEM, &plunder_text(shown));
        for &(item, _, take) in &TAKE_BUTTONS {
            self.dialog.set_greyed(item, !shown.view.offers(take));
        }
    }

    /// The take the player pressed, once.
    pub fn take_take(&mut self) -> Option<Take> {
        self.take.take()
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

/// The built-in plunder dialog: `DLOG` 1011's items, 309 x 198, centred.
fn plunder_template() -> DialogTemplate {
    let at = |x, y, w, h| Bounds::at(Point::new(x, y), w, h);
    let user = |bounds, enabled| ItemTemplate {
        bounds,
        enabled,
        kind: ItemSpec::User,
    };
    DialogTemplate {
        bounds: at(0.0, 0.0, 309.0, 198.0),
        placement: Placement::Center,
        items: vec![
            user(at(91.0, 166.0, 126.0, 25.0), true),
            user(at(110.0, 110.0, 89.0, 25.0), true),
            user(at(35.0, 138.0, 89.0, 25.0), true),
            user(at(204.0, 110.0, 89.0, 25.0), true),
            user(at(11.0, 7.0, 287.0, 96.0), false),
            user(at(16.0, 110.0, 89.0, 25.0), true),
            user(at(129.0, 138.0, 146.0, 25.0), true),
        ],
    }
}

impl Screen for PlunderDialog {
    /// Every event goes to the dialog; a button activated is its take.
    /// It never quits.
    fn input(&mut self, input: &Input) -> ScreenAction {
        if let Some(DialogEvent::Item(item)) = self.dialog.input(input) {
            self.take = TAKE_BUTTONS
                .iter()
                .find(|&&(button, ..)| button == item)
                .map(|&(.., take)| take);
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

/// The captured-ship assignment dialog (see the module docs).
#[derive(Clone, Debug)]
pub struct AssignmentDialog {
    dialog: Dialog,
    picture: Option<ImageKey>,
    choice: Option<Assignment>,
}

impl AssignmentDialog {
    /// The dialog `template` (stock `DLOG` 1018), its buttons labelled in
    /// `style` and its question measured by `metrics`, over
    /// [`ASSIGNMENT_PICTURE`].
    ///
    /// # Errors
    ///
    /// When the template has fewer than its three items.
    pub fn new(
        template: &DialogTemplate,
        style: ButtonStyle,
        metrics: Rc<dyn TextMetrics>,
    ) -> Result<Self, String> {
        Self::built(template, style, metrics, Some(ASSIGNMENT_PICTURE))
    }

    /// The dialog laid out without the interface file, as `DLOG` 1018 is,
    /// over a dark backdrop.
    #[must_use]
    pub fn fallback(style: ButtonStyle, metrics: Rc<dyn TextMetrics>) -> Self {
        Self::built(&assignment_template(), style, metrics, None)
            .expect("the fallback has every item")
    }

    fn built(
        template: &DialogTemplate,
        style: ButtonStyle,
        metrics: Rc<dyn TextMetrics>,
        picture: Option<ImageKey>,
    ) -> Result<Self, String> {
        if template.items.len() < QUESTION_ITEM {
            return Err(format!(
                "DITL {ASSIGNMENT_DIALOG} has {} items, not the assignment dialog's \
                 {QUESTION_ITEM}",
                template.items.len()
            ));
        }
        let roles = [
            (MY_SHIP_ITEM, Role::Button(MY_SHIP_LABEL.to_owned())),
            (ESCORT_ITEM, Role::Button(ESCORT_LABEL.to_owned())),
            (
                QUESTION_ITEM,
                Role::ScrollText {
                    text: QUESTION.to_owned(),
                    font: Font::Geneva,
                    size: TEXT_SIZE,
                    color: TEXT_COLOR,
                },
            ),
        ];
        let template = enabling(template, [MY_SHIP_ITEM, ESCORT_ITEM]);
        let dialog = Dialog::new(&template, &roles, metrics)
            .with_buttons(ButtonSkin::NOVA, style)
            .with_default(None)
            .with_cancel(None);
        Ok(Self {
            dialog,
            picture,
            choice: None,
        })
    }

    /// What the player chose, once.
    pub fn take_choice(&mut self) -> Option<Assignment> {
        self.choice.take()
    }

    /// The dialog itself, for its layout.
    #[must_use]
    pub fn dialog(&self) -> &Dialog {
        &self.dialog
    }
}

/// The built-in assignment dialog: `DLOG` 1018's items, 257 x 114,
/// centred.
fn assignment_template() -> DialogTemplate {
    let at = |x, y, w, h| Bounds::at(Point::new(x, y), w, h);
    let user = |bounds| ItemTemplate {
        bounds,
        enabled: true,
        kind: ItemSpec::User,
    };
    DialogTemplate {
        bounds: at(0.0, 0.0, 257.0, 114.0),
        placement: Placement::Center,
        items: vec![
            user(at(55.0, 51.0, 146.0, 26.0)),
            user(at(55.0, 83.0, 146.0, 26.0)),
            user(at(9.0, 6.0, 238.0, 40.0)),
        ],
    }
}

impl Screen for AssignmentDialog {
    /// Every event goes to the dialog; a button activated is its choice.
    /// It never quits.
    fn input(&mut self, input: &Input) -> ScreenAction {
        match self.dialog.input(input) {
            Some(DialogEvent::Item(MY_SHIP_ITEM)) => self.choice = Some(Assignment::MyShip),
            Some(DialogEvent::Item(ESCORT_ITEM)) => self.choice = Some(Assignment::Escort),
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
mod tests {
    use nova_sim::{Good, NpcId, OutfitId, ShipId};

    use super::*;
    use crate::draw::DrawCommand;
    use crate::input::{Key, MouseButton};
    use crate::sound::UiSound;
    use crate::text::fixture::MonoMetrics;

    /// A trader with 13 tons of food, 2 rounds of rockets, credits and
    /// energy, at odds of 43.
    fn loaded() -> PlunderView {
        PlunderView {
            npc: NpcId(4),
            ship: ShipId(129),
            credits: 5750,
            cargo: Some((Good::Commodity(0), 13)),
            ammo: Some((OutfitId(310), 2)),
            fuel: 170,
            odds: 43,
        }
    }

    fn shown(view: PlunderView) -> PlunderShown {
        PlunderShown {
            view,
            good: Some("Food".to_owned()),
            outfit: Some("Rockets".to_owned()),
        }
    }

    /// Stock `DLOG` 1011's items, fixed at (0, 0).
    fn stock() -> DialogTemplate {
        DialogTemplate {
            placement: Placement::Fixed,
            ..plunder_template()
        }
    }

    fn dialog(view: PlunderView) -> PlunderDialog {
        PlunderDialog::new(
            &stock(),
            &shown(view),
            ButtonStyle::STOCK,
            Rc::new(MonoMetrics),
        )
        .expect("builds")
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

    fn center(dialog: &Dialog, item: usize) -> Point {
        dialog.item_bounds(item).expect("an item").center()
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

    #[test]
    fn its_buttons_are_the_originals_on_its_items() {
        let dialog = dialog(loaded());
        let labels = texts(&dialog);
        for (item, label, _) in TAKE_BUTTONS {
            assert!(labels.contains(&label.to_owned()), "{label}: {labels:?}");
            assert!(dialog.dialog().item_shown(item), "{item}");
        }
        assert_eq!(
            dialog.dialog().item_bounds(7),
            Some(Bounds::at(Point::new(129.0, 138.0), 146.0, 25.0))
        );
        assert_eq!(PLUNDER_DIALOG, 1011);
        assert_eq!(PLUNDER_PICTURE, ImageKey::picture(8515));
        assert_eq!(
            TAKE_BUTTONS.map(|(item, label, _)| (item, label)),
            [
                (1, "Abort"),
                (2, "Cargo"),
                (3, "Credits"),
                (4, "Ammo"),
                (6, "Energy"),
                (7, "Capture Ship"),
            ]
        );
    }

    #[test]
    fn it_says_what_is_on_board() {
        let dialog = dialog(loaded());
        assert_eq!(
            dialog.lines(),
            [
                "Select what to plunder from this ship:",
                "",
                "Cargo: 13 tons of Food",
                "Ammo: 2 Rockets",
                "Capture Odds: 43%",
            ]
        );
        let single = PlunderView {
            cargo: Some((Good::Commodity(0), 1)),
            ammo: None,
            ..loaded()
        };
        let text = plunder_text(&PlunderShown {
            view: single,
            good: None,
            outfit: None,
        });
        assert_eq!(
            text,
            "Select what to plunder from this ship:\r\rCargo: 1 ton of cargo\rAmmo: N/A\rCapture Odds: 43%"
        );
        let unnamed = plunder_text(&PlunderShown {
            view: loaded(),
            good: None,
            outfit: None,
        });
        assert!(unnamed.contains("Ammo: 2 rounds"), "{unnamed}");
        assert_eq!(
            (PROMPT, CARGO_LABEL, AMMO_LABEL, ODDS_LABEL, NONE),
            (
                "Select what to plunder from this ship:",
                "Cargo:",
                "Ammo:",
                "Capture Odds:",
                "N/A"
            )
        );
    }

    #[test]
    fn a_click_gives_its_take_once() {
        for (item, _, take) in TAKE_BUTTONS {
            let mut dialog = dialog(loaded());
            let at = center(dialog.dialog(), item);
            let sounds = click(&mut dialog, at);
            assert_eq!(dialog.take_take(), Some(take), "{item}");
            assert_eq!(dialog.take_take(), None, "once");
            assert_eq!(
                sounds,
                [Sound::Ui(UiSound::ButtonDown), Sound::Ui(UiSound::ButtonUp)]
            );
        }
        let mut dialog = dialog(loaded());
        let at = center(dialog.dialog(), TEXT_ITEM);
        click(&mut dialog, at);
        assert_eq!(dialog.take_take(), None, "the text is no button");
    }

    #[test]
    fn return_enter_and_escape_abort() {
        for pressed in [Key::Enter, Key::Escape] {
            let mut dialog = dialog(loaded());
            dialog.input(&key(pressed));
            assert_eq!(dialog.take_take(), Some(Take::Abort), "{pressed:?}");
        }
    }

    #[test]
    fn a_button_is_greyed_while_its_value_is_not_on_board() {
        let empty = PlunderView {
            credits: 0,
            cargo: None,
            ammo: None,
            fuel: 0,
            odds: 0,
            ..loaded()
        };
        let mut dialog = dialog(empty);
        for (item, _, take) in TAKE_BUTTONS {
            let at = center(dialog.dialog(), item);
            click(&mut dialog, at);
            let expected = (take == Take::Abort).then_some(take);
            assert_eq!(dialog.take_take(), expected, "{item}");
        }
    }

    #[test]
    fn set_plunder_refreshes_the_text_and_the_buttons() {
        let mut dialog = dialog(loaded());
        let taken = PlunderView {
            cargo: None,
            ..loaded()
        };
        dialog.set_plunder(&shown(taken));
        assert_eq!(dialog.lines()[2], "Cargo: N/A");
        let cargo = center(dialog.dialog(), 2);
        click(&mut dialog, cargo);
        assert_eq!(dialog.take_take(), None, "greyed now");
        dialog.set_plunder(&shown(loaded()));
        click(&mut dialog, cargo);
        assert_eq!(dialog.take_take(), Some(Take::Cargo), "back on board");
    }

    #[test]
    fn it_is_drawn_over_its_picture_or_without_one_a_backdrop() {
        let dialog = dialog(loaded());
        assert_eq!(
            drawn(&dialog)[0],
            DrawCommand::StretchedPicture {
                image: PLUNDER_PICTURE,
                top_left: Point::new(0.0, 0.0),
                width: 309.0,
                height: 198.0,
            }
        );
        let fallback =
            PlunderDialog::fallback(&shown(loaded()), ButtonStyle::STOCK, Rc::new(MonoMetrics));
        let bounds = fallback.dialog().bounds();
        assert_eq!((bounds.width(), bounds.height()), (309.0, 198.0));
        assert_eq!(bounds.min, Point::new(357.0, 285.0), "centred");
        let commands = drawn(&fallback);
        assert!(
            matches!(commands[0], DrawCommand::Line { color, .. } if color == BACKDROP),
            "{:?}",
            commands[0]
        );
        assert!(!commands.iter().any(|command| matches!(
            command,
            DrawCommand::StretchedPicture { image, .. } if *image == PLUNDER_PICTURE
        )));
        assert_eq!(fallback.lines(), dialog.lines());
    }

    #[test]
    fn a_template_without_every_item_is_refused() {
        let mut short = stock();
        short.items.truncate(6);
        let refused = PlunderDialog::new(
            &short,
            &shown(loaded()),
            ButtonStyle::STOCK,
            Rc::new(MonoMetrics),
        );
        assert_eq!(
            refused.map(|_| ()),
            Err("DITL 1011 has 6 items, not the plunder dialog's 7".to_owned())
        );
    }

    #[test]
    fn disabled_button_items_in_the_template_still_work() {
        for (item, _, take) in TAKE_BUTTONS {
            let mut template = stock();
            for found in &mut template.items {
                found.enabled = false;
            }
            let mut dialog = PlunderDialog::new(
                &template,
                &shown(loaded()),
                ButtonStyle::STOCK,
                Rc::new(MonoMetrics),
            )
            .expect("builds");
            let at = center(dialog.dialog(), item);
            click(&mut dialog, at);
            assert_eq!(dialog.take_take(), Some(take), "{item}");
        }
        let mut template = stock_assignment();
        for found in &mut template.items {
            found.enabled = false;
        }
        let mut dialog = AssignmentDialog::new(&template, ButtonStyle::STOCK, Rc::new(MonoMetrics))
            .expect("builds");
        let at = center(dialog.dialog(), MY_SHIP_ITEM);
        click(&mut dialog, at);
        assert_eq!(dialog.take_choice(), Some(Assignment::MyShip));
    }

    #[test]
    fn its_clicks_can_be_abandoned() {
        let mut dialog = dialog(loaded());
        let at = center(dialog.dialog(), 3);
        dialog.input(&Input::PointerButton {
            button: MouseButton::Left,
            pressed: true,
            at,
        });
        dialog.cancel_pointer();
        dialog.input(&Input::PointerButton {
            button: MouseButton::Left,
            pressed: false,
            at,
        });
        assert_eq!(dialog.take_take(), None);
        dialog.tick(Duration::from_secs(1));
        assert_eq!(dialog.input(&key(Key::Tab)), ScreenAction::None);
    }

    // The assignment dialog.

    fn stock_assignment() -> DialogTemplate {
        DialogTemplate {
            placement: Placement::Fixed,
            ..assignment_template()
        }
    }

    fn assignment() -> AssignmentDialog {
        AssignmentDialog::new(
            &stock_assignment(),
            ButtonStyle::STOCK,
            Rc::new(MonoMetrics),
        )
        .expect("builds")
    }

    #[test]
    fn the_assignment_dialog_asks_the_originals_question() {
        let dialog = assignment();
        let shown = texts(&dialog);
        assert!(shown.contains(&MY_SHIP_LABEL.to_owned()), "{shown:?}");
        assert!(shown.contains(&ESCORT_LABEL.to_owned()), "{shown:?}");
        let question = dialog
            .dialog()
            .scroll_text()
            .map(|text| text.lines().join(" "))
            .expect("the question");
        assert_eq!(question, QUESTION);
        assert_eq!(
            (MY_SHIP_LABEL, ESCORT_LABEL),
            ("Use As My Ship", "Use As Escort")
        );
        assert_eq!(ASSIGNMENT_DIALOG, 1018);
        assert_eq!(ASSIGNMENT_PICTURE, ImageKey::picture(8516));
        assert_eq!(
            drawn(&dialog)[0],
            DrawCommand::StretchedPicture {
                image: ASSIGNMENT_PICTURE,
                top_left: Point::new(0.0, 0.0),
                width: 257.0,
                height: 114.0,
            }
        );
    }

    #[test]
    fn each_assignment_button_gives_its_choice_once() {
        for (item, choice) in [
            (MY_SHIP_ITEM, Assignment::MyShip),
            (ESCORT_ITEM, Assignment::Escort),
        ] {
            let mut dialog = assignment();
            let at = center(dialog.dialog(), item);
            let sounds = click(&mut dialog, at);
            assert_eq!(dialog.take_choice(), Some(choice));
            assert_eq!(dialog.take_choice(), None);
            assert_eq!(sounds.len(), 2);
        }
        let mut dialog = assignment();
        for pressed in [Key::Enter, Key::Escape] {
            dialog.input(&key(pressed));
        }
        let at = center(dialog.dialog(), QUESTION_ITEM);
        click(&mut dialog, at);
        assert_eq!(dialog.take_choice(), None, "only a button chooses");
    }

    #[test]
    fn the_assignment_fallback_is_its_items_over_a_backdrop() {
        let mut dialog = AssignmentDialog::fallback(ButtonStyle::STOCK, Rc::new(MonoMetrics));
        let bounds = dialog.dialog().bounds();
        assert_eq!((bounds.width(), bounds.height()), (257.0, 114.0));
        assert!(matches!(drawn(&dialog)[0], DrawCommand::Line { color, .. } if color == BACKDROP));
        let at = center(dialog.dialog(), ESCORT_ITEM);
        click(&mut dialog, at);
        assert_eq!(dialog.take_choice(), Some(Assignment::Escort));
        let mut short = stock_assignment();
        short.items.truncate(2);
        assert!(AssignmentDialog::new(&short, ButtonStyle::STOCK, Rc::new(MonoMetrics)).is_err());
        let at = center(dialog.dialog(), MY_SHIP_ITEM);
        dialog.input(&Input::PointerButton {
            button: MouseButton::Left,
            pressed: true,
            at,
        });
        dialog.cancel_pointer();
        dialog.input(&Input::PointerButton {
            button: MouseButton::Left,
            pressed: false,
            at,
        });
        dialog.tick(Duration::from_secs(1));
        assert_eq!(dialog.take_choice(), None, "the click was abandoned");
    }
}
