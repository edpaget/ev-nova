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
//! refreshes the dialog ([`PlunderDialog::set_plunder`]). A click that
//! reaches no enabled button (on a greyed button, on the text, outside
//! the dialog, or released off the button it began on) gives
//! [`Take::Miss`], as `_TrackPlunderButtons` (@0x2732a) hit-tests only the
//! enabled buttons (@0x273d3-0x27426) and `_PlunderFilter` gives anything
//! else as item -1, which still ends the modal wait (@0x92fdf). The clicks
//! themselves are silent; what a press beeps is the session's. The
//! dialog keeps how long it has been open ([`PlunderDialog::held`]), the
//! `TickCount` the flight's floating map is read against at each press.
//!
//! The assignment dialog asks `STR#` 2002 #118's question in item 3, with
//! "Use As My Ship" (item 1, `STR#` 150 #47) and "Use As Escort" (item 2,
//! #46); a click gives its [`Assignment`], and so does a press of M or E
//! (`_CapturedAssignmentFilter` @0x51782: M is item 1 @0x51799 and
//! @0x517e4, E item 2 @0x5179f-0x517ac). The filter maps no Return, Enter
//! or Escape, so they choose nothing.
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
use crate::input::{Input, Key, MouseButton};
use crate::screen::{Screen, ScreenAction};
use crate::text::TextMetrics;

use super::button::{ButtonSkin, ButtonStyle};
use super::dialog::{
    Dialog, DialogEvent, DialogTemplate, FocusRing, ItemSpec, ItemTemplate, Placement, Role,
    STOCK_WORD, TextBacking, outline,
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
        |ammo| {
            format!(
                "{} {}",
                ammo.rounds,
                shown.outfit.as_deref().unwrap_or("rounds")
            )
        },
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
    /// Whether a click is under way: a primary press that has not been
    /// released or abandoned.
    pressing: bool,
    /// How long it has been open.
    held: Duration,
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
                backing: None,
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
            pressing: false,
            held: Duration::ZERO,
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

    /// How long it has been open, in real time: the original's
    /// `TickCount` moves on under it.
    #[must_use]
    pub fn held(&self) -> Duration {
        self.held
    }

    /// The plunder dialog with its buttons taking the keyboard focus as `ring`
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
        placement: Placement::for_dlog(PLUNDER_DIALOG, Some(STOCK_WORD)),
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
    /// Every event goes to the dialog; a button activated is its take,
    /// and a primary click released without activating an enabled button
    /// is a [`Take::Miss`] (see the module docs). It never quits.
    fn input(&mut self, input: &Input) -> ScreenAction {
        let event = self.dialog.input(input);
        if let Some(DialogEvent::Item(item)) = event {
            self.take = TAKE_BUTTONS
                .iter()
                .find(|&&(button, ..)| button == item)
                .map(|&(.., take)| take);
        }
        if let Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            ..
        } = *input
        {
            if pressed {
                self.pressing = true;
            } else if std::mem::take(&mut self.pressing) && event.is_none() {
                self.take = Some(Take::Miss);
            }
        }
        ScreenAction::None
    }

    /// Keeps how long it has been open.
    fn tick(&mut self, dt: Duration) {
        self.held += dt;
    }

    /// The picture (or the backdrop), then the dialog.
    fn draw(&self, list: &mut DrawList) {
        backdrop(list, self.dialog.bounds(), self.picture);
        self.dialog.draw(list);
    }

    fn cancel_pointer(&mut self) {
        self.dialog.cancel_pointer();
        self.pressing = false;
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
                // White on black: `_CapturedAssignmentDialogUpdate` sets
                // `ForeColor` 30 on `BackColor` 33 (@0x51604-0x51620) and
                // erases the item black under its `TETextBox` (@0x51689).
                Role::ScrollText {
                    text: QUESTION.to_owned(),
                    font: Font::Geneva,
                    size: TEXT_SIZE,
                    color: TEXT_COLOR,
                    backing: Some(TextBacking::BLACK_BOX),
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

    /// The assignment dialog with its buttons taking the keyboard focus as `ring`
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
        placement: Placement::for_dlog(ASSIGNMENT_DIALOG, Some(STOCK_WORD)),
        items: vec![
            user(at(55.0, 51.0, 146.0, 26.0)),
            user(at(55.0, 83.0, 146.0, 26.0)),
            user(at(9.0, 6.0, 238.0, 40.0)),
        ],
    }
}

impl Screen for AssignmentDialog {
    /// A press (not a repeat) of M, either case, is Use As My Ship, and of
    /// E Use As Escort, as `_CapturedAssignmentFilter` has them; every
    /// other event goes to the dialog, and a button activated is its
    /// choice. It never quits.
    fn input(&mut self, input: &Input) -> ScreenAction {
        if let Input::Key {
            key: Key::Char(letter),
            pressed: true,
            repeat,
        } = *input
        {
            let choice = match letter.to_ascii_lowercase() {
                'm' => Some(Assignment::MyShip),
                'e' => Some(Assignment::Escort),
                _ => None,
            };
            if let Some(choice) = choice {
                if !repeat {
                    self.choice = Some(choice);
                }
                return ScreenAction::None;
            }
        }
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
}

#[cfg(test)]
mod tests {
    use nova_sim::{Good, NpcId, OutfitId, ShipId};

    use super::*;
    use crate::draw::{DrawCommand, fill_command};
    use crate::input::{Key, MouseButton};
    use crate::sound::Sound;
    use crate::text::fixture::MonoMetrics;
    use crate::ui::ScrollText;

    /// A trader with 13 tons of food, 2 rounds of rockets, credits and
    /// energy, at odds of 43.
    fn loaded() -> PlunderView {
        PlunderView {
            npc: NpcId(4),
            ship: ShipId(129),
            credits: 5750,
            cargo: Some((Good::Commodity(0), 13)),
            ammo: Some(nova_sim::AmmoOffer {
                slot: nova_sim::WeaponId(140),
                rounds: 2,
                named: Some(OutfitId(310)),
            }),
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

    /// A pointer click plays nothing: `_TrackPlunderButtons` @0x2732a
    /// calls no sound routine.
    #[test]
    fn a_click_gives_its_take_once_silently() {
        for (item, _, take) in TAKE_BUTTONS {
            let mut dialog = dialog(loaded());
            let at = center(dialog.dialog(), item);
            let sounds = click(&mut dialog, at);
            assert_eq!(dialog.take_take(), Some(take), "{item}");
            assert_eq!(dialog.take_take(), None, "once");
            assert_eq!(sounds, []);
        }
    }

    /// `_TrackPlunderButtons` hit-tests only the enabled buttons
    /// (@0x273d3-0x27426), and `_PlunderFilter` gives anything else as
    /// item -1, which still ends the modal wait (@0x92fdf).
    #[test]
    fn a_click_that_reaches_no_enabled_button_is_a_miss_once_silently() {
        let mut dialog = dialog(loaded());
        let text = center(dialog.dialog(), TEXT_ITEM);
        let sounds = click(&mut dialog, text);
        assert_eq!(
            dialog.take_take(),
            Some(Take::Miss),
            "the text is no button"
        );
        assert_eq!(dialog.take_take(), None, "once");
        assert_eq!(sounds, []);
        let outside = Point::new(-50.0, -50.0);
        click(&mut dialog, outside);
        assert_eq!(dialog.take_take(), Some(Take::Miss), "outside the dialog");
        let credits = center(dialog.dialog(), 3);
        dialog.input(&Input::PointerButton {
            button: MouseButton::Left,
            pressed: true,
            at: credits,
        });
        dialog.input(&Input::PointerButton {
            button: MouseButton::Left,
            pressed: false,
            at: text,
        });
        assert_eq!(
            dialog.take_take(),
            Some(Take::Miss),
            "released off the button it began on"
        );
    }

    #[test]
    fn return_enter_and_escape_abort() {
        for pressed in [Key::Enter, Key::Escape] {
            let mut dialog = dialog(loaded());
            dialog.input(&key(pressed));
            assert_eq!(dialog.take_take(), Some(Take::Abort), "{pressed:?}");
        }
    }

    /// A key that presses a button plays nothing in the original:
    /// `_PlunderFilter` @0x92f20 calls no sound routine, and neither does
    /// `_MyModalDialog` @0x59954.
    #[test]
    fn keyboard_activation_is_silent_as_the_original() {
        for pressed in [Key::Enter, Key::Escape] {
            let mut dialog = dialog(loaded());
            assert_eq!(dialog.take_sounds(), []);
            dialog.input(&key(pressed));
            assert_eq!(dialog.take_take(), Some(Take::Abort), "{pressed:?}");
            assert_eq!(dialog.take_sounds(), [], "{pressed:?}");
        }
        let mut dialog = dialog(loaded());
        dialog.input(&key(Key::Tab));
        dialog.input(&key(Key::Space));
        assert!(dialog.take_take().is_some(), "Space on the focused button");
        assert_eq!(dialog.take_sounds(), [], "Space");
    }

    #[test]
    fn a_greyed_button_pressed_is_a_miss() {
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
            let expected = if take == Take::Abort {
                take
            } else {
                Take::Miss
            };
            assert_eq!(dialog.take_take(), Some(expected), "{item}");
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
        assert_eq!(dialog.take_take(), Some(Take::Miss), "greyed now");
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
        assert_eq!(
            bounds.min,
            Point::new(358.0, 285.0),
            "as _CenterWindow centres it"
        );
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
        assert_eq!(dialog.take_take(), None, "abandoned, not missed");
        dialog.tick(Duration::from_secs(1));
        assert_eq!(dialog.input(&key(Key::Tab)), ScreenAction::None);
    }

    /// The time it has been open, `TickCount`'s, which the flight's
    /// floating map reads at each press (`_HandleComm` @0x30262-0x30285).
    #[test]
    fn it_keeps_how_long_it_has_been_open() {
        let mut dialog = dialog(loaded());
        assert_eq!(dialog.held(), Duration::ZERO);
        dialog.tick(Duration::from_millis(250));
        dialog.tick(Duration::from_millis(500));
        assert_eq!(dialog.held(), Duration::from_millis(750));
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
    fn the_assignment_question_draws_over_a_black_box_filling_its_item() {
        let dialog = assignment();
        let area = dialog
            .dialog()
            .item_bounds(QUESTION_ITEM)
            .expect("the question");
        let commands = drawn(&dialog);
        let fill = commands
            .iter()
            .position(|command| *command == fill_command(area, Color::BLACK))
            .expect("filled black");
        let text = commands
            .iter()
            .position(|command| {
                matches!(command, DrawCommand::Text { origin, .. } if area.contains(*origin))
            })
            .expect("the question's first line");
        assert!(fill < text, "the box is under the text");
    }

    #[test]
    fn up_and_down_still_scroll_the_assignment_question() {
        // The question's item cut to one line, so the question overflows.
        let mut template = stock_assignment();
        template.items[QUESTION_ITEM - 1].bounds.max.y = 6.0 + 12.0;
        let mut dialog = AssignmentDialog::new(&template, ButtonStyle::STOCK, Rc::new(MonoMetrics))
            .expect("builds");
        let first =
            |dialog: &AssignmentDialog| dialog.dialog().text(QUESTION_ITEM).map(ScrollText::first);
        dialog.input(&key(Key::Down));
        assert_eq!(first(&dialog), Some(1));
        dialog.input(&key(Key::Up));
        assert_eq!(first(&dialog), Some(0));
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
            assert_eq!(sounds, [], "silent, as `_TrackCapAssignButtons` @0x291c7");
        }
        let mut dialog = assignment();
        for pressed in [Key::Enter, Key::Escape] {
            dialog.input(&key(pressed));
        }
        let at = center(dialog.dialog(), QUESTION_ITEM);
        click(&mut dialog, at);
        assert_eq!(dialog.take_choice(), None, "only a button chooses");
    }

    /// Keys play nothing in the original: `_DoCapturedShipAssignmentDialog`
    /// runs `_MyModalDialog` @0x59954, which calls no sound routine. Its
    /// `_beepSnd` plays (@0x5b56b, @0x5b66b) follow the choice, not the
    /// input that made it, and are the caller's.
    #[test]
    fn assignment_keyboard_activation_is_silent_as_the_original() {
        let mut dialog = assignment();
        assert_eq!(dialog.take_sounds(), []);
        for pressed in [Key::Enter, Key::Escape] {
            dialog.input(&key(pressed));
            assert_eq!(dialog.take_sounds(), [], "{pressed:?}");
        }
        dialog.input(&key(Key::Tab));
        dialog.input(&key(Key::Space));
        assert!(
            dialog.take_choice().is_some(),
            "Space on the focused button"
        );
        assert_eq!(dialog.take_sounds(), [], "Space");
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

    #[test]
    fn the_built_in_plunder_dialog_is_placed_as_center_window_places_dlog_1011() {
        let template = plunder_template();
        assert_eq!(
            template.placement,
            crate::ui::dialog::Placement::CenterWindow
        );
        assert_eq!(
            template.placement,
            crate::ui::dialog::Placement::for_dlog(
                PLUNDER_DIALOG,
                Some(crate::ui::dialog::STOCK_WORD)
            ),
            "as the loaded DLOG is placed"
        );
        assert_eq!(
            template.placement.origin(template.bounds),
            crate::geometry::Point::new(358.0, 285.0)
        );
    }

    #[test]
    fn the_built_in_assignment_dialog_is_placed_as_center_window_places_dlog_1018() {
        let template = assignment_template();
        assert_eq!(
            template.placement,
            crate::ui::dialog::Placement::CenterWindow
        );
        assert_eq!(
            template.placement,
            crate::ui::dialog::Placement::for_dlog(
                ASSIGNMENT_DIALOG,
                Some(crate::ui::dialog::STOCK_WORD)
            ),
            "as the loaded DLOG is placed"
        );
        assert_eq!(
            template.placement.origin(template.bounds),
            crate::geometry::Point::new(384.0, 327.0)
        );
    }

    /// `_PlunderFilter` @0x92f20 and `_CapturedAssignmentFilter` @0x51782
    /// map no Tab or Space.
    #[test]
    fn tab_then_space_presses_the_focused_button_only_with_the_focus_ring() {
        let mut ringed = dialog(loaded());
        ringed.input(&key(Key::Tab));
        ringed.input(&key(Key::Space));
        assert!(ringed.take_take().is_some(), "plunder by default");
        let mut off = dialog(loaded()).with_focus_ring(FocusRing::Off);
        off.input(&key(Key::Tab));
        off.input(&key(Key::Space));
        assert_eq!(off.take_take(), None);
        assert_eq!(off.dialog().focus(), None);
        let mut ringed = assignment();
        ringed.input(&key(Key::Tab));
        ringed.input(&key(Key::Space));
        assert!(ringed.take_choice().is_some(), "assignment by default");
        let mut off = assignment().with_focus_ring(FocusRing::Off);
        off.input(&key(Key::Tab));
        off.input(&key(Key::Space));
        assert_eq!(off.take_choice(), None);
        assert_eq!(off.dialog().focus(), None);
    }

    /// `_CapturedAssignmentFilter` @0x51782: M is Use As My Ship
    /// (@0x51799, @0x517e4), E Use As Escort (@0x5179f-0x517ac), on a
    /// keyDown only, and Return, Enter and Escape are nothing.
    #[test]
    fn m_and_e_choose_and_return_and_escape_do_not() {
        for (pressed, choice) in [
            (Key::Char('m'), Assignment::MyShip),
            (Key::Char('M'), Assignment::MyShip),
            (Key::Char('e'), Assignment::Escort),
            (Key::Char('E'), Assignment::Escort),
        ] {
            let mut dialog = assignment();
            dialog.input(&Input::Key {
                key: pressed,
                pressed: true,
                repeat: true,
            });
            assert_eq!(dialog.take_choice(), None, "{pressed:?} repeating");
            dialog.input(&key(pressed));
            assert_eq!(dialog.take_choice(), Some(choice), "{pressed:?}");
            assert_eq!(dialog.take_sounds(), [], "{pressed:?}");
        }
        let mut dialog = assignment();
        for pressed in [Key::Enter, Key::Escape, Key::Char('x')] {
            dialog.input(&key(pressed));
            assert_eq!(dialog.take_choice(), None, "{pressed:?}");
        }
    }
}
