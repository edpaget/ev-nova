//! The offer dialog (`DLOG`/`DITL` 1016, `_OfferOneMission` @0xa21ac in
//! the `EV Nova` executable): a mission offered outside the mission
//! computer, such as at the bar, its text and its buttons.
//!
//! The stock dialog is 441 x 317, centred, over the frame of `PICT`s
//! 8521-8523: the offer text in item 3, and below it the accept (item 1)
//! and refuse (item 2) buttons, labelled as the session says
//! ([`OfferPrompt`]); a mission that cannot be refused shows the one
//! button of item 6 in their place, labelled to accept. The stock items
//! are all disabled, so the dialog enables its buttons itself. Items 4, 5
//! and 7 (the map, player info and mission info) lie outside it, and the
//! arrows of items 9 and 10 are not drawn: Up and Down scroll the text.
//!
//! The original shrinks `DLOG` 1016 to its text's height, at least 48,
//! moving the buttons up (@0xa2548-0xa2816), and widens the buttons to the
//! wider label + 40, at least 100 (@0xa2a29-0xa2b46); this dialog keeps
//! the template's height and widths. The picture variant (`DLOG` 1020) is
//! rdm:phase/missions/phase-16-offer-pictures-and-movies.
//!
//! Keys (`_SingleMissionFilter` @0x98d9a): Return and Enter accept; the
//! accept label's first letter accepts and, when the mission can be
//! refused, the refuse label's refuses, unless both start with the same
//! letter. Escape does nothing. The choice is taken once
//! ([`OfferDialog::take_choice`]), and the dialog is then answered.
//!
//! A venue that offers missions (the bar, the spaceport, the trade
//! center, the shipyard and the outfitter) holds its offers in a
//! `VenueOffers`: when it next asks for an offer, the offer dialog open
//! over it, and the choice made, until it is taken.

use std::rc::Rc;
use std::time::Duration;

use nova_sim::{MissionId, OfferPrompt};

use super::view::{PROBLEM_AT, PROBLEM_SIZE};
use crate::color::Color;
use crate::draw::DrawList;
use crate::font::Font;
use crate::image::ImageKey;
use crate::input::{Input, Key};
use crate::screen::{Screen, ScreenAction};
use crate::sound::Sound;
use crate::text::TextMetrics;
use crate::ui::button::{ButtonSkin, ButtonStyle};
use crate::ui::desc::{BODY_COLOR, BODY_SIZE};
use crate::ui::dialog::{Dialog, DialogEvent, DialogFrame, DialogTemplate, Role};

/// The offer dialog's `DLOG` (and `DITL`) ID.
pub const OFFER_DIALOG: i16 = 1016;

/// The accept button's item.
pub const ACCEPT_ITEM: usize = 1;
/// The refuse button's item.
pub const REFUSE_ITEM: usize = 2;
/// The text's item.
pub const TEXT_ITEM: usize = 3;
/// The one button's item, for a mission that cannot be refused.
pub const SINGLE_ITEM: usize = 6;

/// The game's mission offer frame (`PICT`s 8521 to 8523, 441 wide), which
/// the offer dialog is drawn over.
pub const FRAME: DialogFrame = DialogFrame {
    top: ImageKey::picture(8521),
    middle: ImageKey::picture(8522),
    bottom: ImageKey::picture(8523),
    top_height: 9.0,
    bottom_height: 40.0,
};

/// What the player chose.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OfferChoice {
    /// Accept the mission.
    Accept,
    /// Refuse it.
    Refuse,
}

/// The offer dialog over the screen that offers it.
#[derive(Clone, Debug)]
pub struct OfferDialog {
    /// The dialog, or why it cannot be shown.
    laid: Result<Dialog, String>,
    prompt: OfferPrompt,
    /// The choice made, until it is taken.
    choice: Option<OfferChoice>,
    answered: bool,
    sounds: Vec<Sound>,
}

impl OfferDialog {
    /// `prompt`, laid out by `layout`: the offer dialog's template and the
    /// metrics its text is measured by, or why there are none; its
    /// buttons drawn in `style`.
    #[must_use]
    pub fn new(
        layout: Result<(DialogTemplate, Rc<dyn TextMetrics>), String>,
        prompt: OfferPrompt,
        style: ButtonStyle,
    ) -> Self {
        let laid = layout.map(|(mut template, metrics)| {
            let single = prompt.labels.refuse.is_none();
            let buttons: &[usize] = if single {
                &[SINGLE_ITEM]
            } else {
                &[ACCEPT_ITEM, REFUSE_ITEM]
            };
            for &item in buttons {
                if let Some(found) = template.items.get_mut(item - 1) {
                    found.enabled = true;
                }
            }
            let accept = Role::Button(prompt.labels.accept.clone());
            let mut roles = vec![(
                TEXT_ITEM,
                Role::ScrollText {
                    text: prompt.text.clone(),
                    font: Font::Geneva,
                    size: BODY_SIZE,
                    color: BODY_COLOR,
                },
            )];
            match &prompt.labels.refuse {
                Some(refuse) => roles.extend([
                    (ACCEPT_ITEM, accept),
                    (REFUSE_ITEM, Role::Button(refuse.clone())),
                    (SINGLE_ITEM, Role::Hidden),
                ]),
                None => roles.extend([
                    (SINGLE_ITEM, accept),
                    (ACCEPT_ITEM, Role::Hidden),
                    (REFUSE_ITEM, Role::Hidden),
                ]),
            }
            Dialog::new(&template, &roles, metrics)
                .with_frame(FRAME)
                .with_buttons(ButtonSkin::NOVA, style)
                .with_default(None)
                .with_cancel(None)
        });
        Self {
            laid,
            prompt,
            choice: None,
            answered: false,
            sounds: Vec::new(),
        }
    }

    /// The dialog, if it could be laid out.
    #[must_use]
    pub fn dialog(&self) -> Option<&Dialog> {
        self.laid.as_ref().ok()
    }

    /// Why the offer cannot be shown, if it cannot.
    #[must_use]
    pub fn problem(&self) -> Option<&str> {
        self.laid.as_ref().err().map(String::as_str)
    }

    /// The offer shown.
    #[must_use]
    pub fn prompt(&self) -> &OfferPrompt {
        &self.prompt
    }

    /// Whether a choice has been made.
    #[must_use]
    pub fn answered(&self) -> bool {
        self.answered
    }

    /// The choice made since this was last taken, once.
    pub fn take_choice(&mut self) -> Option<OfferChoice> {
        self.choice.take()
    }

    /// Chooses `choice`, once.
    fn choose(&mut self, choice: OfferChoice) {
        if !self.answered {
            self.answered = true;
            self.choice = Some(choice);
        }
    }

    /// What key `key` chooses: Return accepts, and each label's first
    /// letter, either case, chooses it, unless both start with the same
    /// letter.
    fn key_choice(&self, key: Key) -> Option<OfferChoice> {
        let first = |label: &str| label.chars().next().map(|c| c.to_ascii_lowercase());
        let accept = first(&self.prompt.labels.accept);
        let refuse = self.prompt.labels.refuse.as_deref().and_then(first);
        let letter = match key {
            Key::Enter => return Some(OfferChoice::Accept),
            Key::Char(c) if accept != refuse => Some(c.to_ascii_lowercase()),
            _ => return None,
        };
        if letter == accept {
            Some(OfferChoice::Accept)
        } else if letter == refuse {
            Some(OfferChoice::Refuse)
        } else {
            None
        }
    }
}

impl Screen for OfferDialog {
    /// Return, Enter and the labels' letters choose; everything else goes
    /// to the dialog. Once answered it takes nothing. It never quits.
    fn input(&mut self, input: &Input) -> ScreenAction {
        if self.answered {
            return ScreenAction::None;
        }
        if let Input::Key {
            key,
            pressed: true,
            repeat: false,
        } = *input
            && let Some(choice) = self.key_choice(key)
        {
            self.choose(choice);
            return ScreenAction::None;
        }
        let Ok(dialog) = &mut self.laid else {
            return ScreenAction::None;
        };
        let event = dialog.input(input);
        self.sounds.extend(dialog.take_sound().map(Sound::Ui));
        match event {
            Some(DialogEvent::Item(ACCEPT_ITEM | SINGLE_ITEM)) => self.choose(OfferChoice::Accept),
            Some(DialogEvent::Item(REFUSE_ITEM)) => self.choose(OfferChoice::Refuse),
            _ => {}
        }
        ScreenAction::None
    }

    /// Nothing moves on its own.
    fn tick(&mut self, _dt: Duration) {}

    fn draw(&self, list: &mut DrawList) {
        match &self.laid {
            Ok(dialog) => dialog.draw(list),
            Err(reason) => {
                list.text(
                    format!("Cannot show the mission offer: {reason}. Press Return to accept it."),
                    PROBLEM_AT,
                    PROBLEM_SIZE,
                    None,
                    Color::ERROR,
                );
            }
        }
    }

    fn cancel_pointer(&mut self) {
        if let Ok(dialog) = &mut self.laid {
            dialog.cancel_pointer();
        }
    }

    /// The buttons' sounds, in order.
    fn take_sounds(&mut self) -> Vec<Sound> {
        std::mem::take(&mut self.sounds)
    }
}

/// A venue's mission offers: the offer dialog's template (or why there is
/// none), the wait before it next asks, and the offer open over it.
#[derive(Clone, Debug)]
pub(crate) struct VenueOffers {
    template: Result<DialogTemplate, String>,
    /// How long until the venue asks for an offer: none while it waits to
    /// be rearmed.
    wait: Option<Duration>,
    /// Whether the venue has asked since this was last taken.
    requested: bool,
    /// The offer dialog, while it is open.
    dialog: Option<OfferDialog>,
    /// The offer answered and its choice, until it is taken.
    choice: Option<(MissionId, OfferChoice)>,
}

impl VenueOffers {
    /// Offers that ask for the first after `first`, as the bar does
    /// (`_DoBarDialog` @0x48f51-0x48f5f).
    pub(crate) fn waiting(template: Result<DialogTemplate, String>, first: Duration) -> Self {
        Self {
            template,
            wait: Some(first),
            requested: false,
            dialog: None,
            choice: None,
        }
    }

    /// Offers already asking for the first, as the spaceport and the shops
    /// ask as they open (`_DoPortDialog` @0x5fc37, `_DoTradeDialog`
    /// @0x5df7f, `_DoShipyardDialog` @0x5e8eb, `_DoOutfitDialog`
    /// @0x5bd92).
    pub(crate) fn asking(template: Result<DialogTemplate, String>) -> Self {
        Self {
            requested: true,
            wait: None,
            ..Self::waiting(template, Duration::ZERO)
        }
    }

    /// Whether the venue has asked for an offer since this was last taken.
    pub(crate) fn take_request(&mut self) -> bool {
        std::mem::take(&mut self.requested)
    }

    /// Opens the offer dialog on `prompt`, its text measured by `metrics`
    /// and its buttons drawn in `style`.
    pub(crate) fn open(
        &mut self,
        prompt: OfferPrompt,
        metrics: Rc<dyn TextMetrics>,
        style: ButtonStyle,
    ) {
        let layout = self.template.clone().map(|template| (template, metrics));
        self.dialog = Some(OfferDialog::new(layout, prompt, style));
    }

    /// The offer dialog, while it is open.
    pub(crate) fn dialog(&self) -> Option<&OfferDialog> {
        self.dialog.as_ref()
    }

    /// The offer answered since this was last taken, and the choice made,
    /// once.
    pub(crate) fn take_choice(&mut self) -> Option<(MissionId, OfferChoice)> {
        self.choice.take()
    }

    /// Asks for the next offer after `gap`.
    pub(crate) fn rearm(&mut self, gap: Duration) {
        self.wait = Some(gap);
    }

    /// Gives `input` to the offer dialog, if it is open, adding its sounds
    /// to `sounds`; answered, it closes. Whether it took the input.
    pub(crate) fn input(&mut self, input: &Input, sounds: &mut Vec<Sound>) -> bool {
        let Some(dialog) = &mut self.dialog else {
            return false;
        };
        dialog.input(input);
        sounds.extend(dialog.take_sounds());
        if let Some(choice) = dialog.take_choice() {
            self.choice = Some((dialog.prompt().mission, choice));
            self.dialog = None;
        }
        true
    }

    /// The wait for the next offer runs down while no offer is open; once
    /// it is out the venue asks.
    pub(crate) fn tick(&mut self, dt: Duration) {
        if self.dialog.is_some() {
            return;
        }
        if let Some(wait) = &mut self.wait {
            *wait = wait.saturating_sub(dt);
            if wait.is_zero() {
                self.wait = None;
                self.requested = true;
            }
        }
    }

    /// Draws the offer dialog, if it is open.
    pub(crate) fn draw(&self, list: &mut DrawList) {
        if let Some(dialog) = &self.dialog {
            dialog.draw(list);
        }
    }

    /// Cancels the offer dialog's pointer, if it is open: whether it was.
    pub(crate) fn cancel_pointer(&mut self) -> bool {
        self.dialog
            .as_mut()
            .map(OfferDialog::cancel_pointer)
            .is_some()
    }
}

/// An offer dialog's template and prompt, for the venues' tests.
#[cfg(test)]
pub(crate) mod fixture {
    use nova_sim::mission::OfferLabels;
    use nova_sim::{MissionId, OfferPrompt};

    use crate::geometry::{Bounds, Point};
    use crate::ui::dialog::{DialogTemplate, ItemSpec, ItemTemplate, Placement};

    /// The offer dialog's shape, smaller: 300 x 200, fixed at (0, 0), with
    /// accept (1), refuse (2), the text (3) and the one button (6).
    pub(crate) fn template() -> DialogTemplate {
        let rect = |x, y, w, h| Bounds::at(Point::new(x, y), w, h);
        let mut items: Vec<ItemTemplate> = (0..6)
            .map(|_| ItemTemplate {
                bounds: rect(0.0, 300.0, 1.0, 1.0),
                enabled: false,
                kind: ItemSpec::User,
            })
            .collect();
        items[0].bounds = rect(200.0, 170.0, 90.0, 25.0);
        items[1].bounds = rect(100.0, 170.0, 90.0, 25.0);
        items[2].bounds = rect(10.0, 10.0, 280.0, 150.0);
        items[5].bounds = rect(150.0, 170.0, 90.0, 25.0);
        DialogTemplate {
            bounds: rect(0.0, 0.0, 300.0, 200.0),
            placement: Placement::Fixed,
            items,
        }
    }

    /// Mission 141's offer, to Mars, "I'm in" or "No".
    pub(crate) fn prompt() -> OfferPrompt {
        OfferPrompt {
            mission: MissionId(141),
            text: "Ferry this to Mars.".to_owned(),
            labels: OfferLabels {
                accept: "I'm in".to_owned(),
                refuse: Some("No".to_owned()),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use nova_sim::mission::OfferLabels;

    use super::*;
    use crate::draw::DrawCommand;
    use crate::geometry::{Bounds, Point};
    use crate::input::MouseButton;
    use crate::text::fixture::MonoMetrics;
    use crate::ui::dialog::{ItemSpec, ItemTemplate, Placement};

    fn rect(x: f32, y: f32, w: f32, h: f32) -> Bounds {
        Bounds::at(Point::new(x, y), w, h)
    }

    /// Stock `DLOG` 1016, 441 x 317 and centred, with its ten disabled
    /// user items as `DITL` 1016 has them, 4, 5 and 7 parked outside it.
    fn template() -> DialogTemplate {
        let items = [
            (225.0, 285.0, 99.0, 25.0),
            (117.0, 285.0, 99.0, 25.0),
            (12.0, 9.0, 415.0, 267.0),
            (150.0, 323.0, 32.0, 32.0),
            (197.0, 326.0, 32.0, 32.0),
            (173.0, 285.0, 99.0, 25.0),
            (172.0, 337.0, 32.0, 32.0),
            (272.0, 345.0, 23.0, 23.0),
            (340.0, 287.0, 23.0, 23.0),
            (373.0, 287.0, 23.0, 23.0),
        ];
        DialogTemplate {
            bounds: rect(40.0, 40.0, 441.0, 317.0),
            placement: Placement::Center,
            items: items
                .into_iter()
                .map(|(x, y, w, h)| ItemTemplate {
                    bounds: rect(x, y, w, h),
                    enabled: false,
                    kind: ItemSpec::User,
                })
                .collect(),
        }
    }

    /// Where the dialog goes: (1024 - 441) / 2 and (768 - 317) / 2,
    /// floored.
    const ORIGIN: Point = Point::new(291.0, 225.0);

    fn prompt(accept: &str, refuse: Option<&str>) -> OfferPrompt {
        OfferPrompt {
            mission: MissionId(141),
            text: "Ferry this to Mars.".to_owned(),
            labels: OfferLabels {
                accept: accept.to_owned(),
                refuse: refuse.map(str::to_owned),
            },
        }
    }

    fn dialog_of(prompt: OfferPrompt) -> OfferDialog {
        OfferDialog::new(
            Ok((template(), Rc::new(MonoMetrics))),
            prompt,
            ButtonStyle::STOCK,
        )
    }

    fn offer() -> OfferDialog {
        dialog_of(prompt("I'm in", Some("No thanks")))
    }

    fn drawn(dialog: &OfferDialog) -> Vec<DrawCommand> {
        let mut list = DrawList::new();
        dialog.draw(&mut list);
        list.iter().cloned().collect()
    }

    fn texts(dialog: &OfferDialog) -> Vec<String> {
        drawn(dialog)
            .into_iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, .. } => Some(text),
                _ => None,
            })
            .collect()
    }

    fn key(key: Key) -> Input {
        Input::Key {
            key,
            pressed: true,
            repeat: false,
        }
    }

    fn click_item(dialog: &mut OfferDialog, number: usize) {
        let at = dialog
            .dialog()
            .expect("laid out")
            .item_bounds(number)
            .expect("an item")
            .center();
        for pressed in [true, false] {
            dialog.input(&Input::PointerButton {
                button: MouseButton::Left,
                pressed,
                at,
            });
        }
    }

    #[test]
    fn the_named_values() {
        assert_eq!(OFFER_DIALOG, 1016);
        assert_eq!(
            [ACCEPT_ITEM, REFUSE_ITEM, TEXT_ITEM, SINGLE_ITEM],
            [1, 2, 3, 6]
        );
        assert_eq!(
            [FRAME.top, FRAME.middle, FRAME.bottom],
            [8521, 8522, 8523].map(ImageKey::picture)
        );
    }

    #[test]
    fn it_shows_the_text_and_both_labels_over_its_frame() {
        let dialog = offer();
        assert_eq!(dialog.problem(), None);
        let laid = dialog.dialog().expect("laid out");
        assert_eq!(laid.bounds().min, ORIGIN);
        assert_eq!(
            laid.text(TEXT_ITEM).map(|text| text.lines().join(" ")),
            Some("Ferry this to Mars.".to_owned())
        );
        let commands = drawn(&dialog);
        assert!(matches!(
            commands[0],
            DrawCommand::StretchedPicture { image, top_left, .. }
                if image == FRAME.top && top_left == ORIGIN
        ));
        let shown = texts(&dialog);
        for label in ["I'm in", "No thanks", "Ferry this to Mars."] {
            assert!(shown.contains(&label.to_owned()), "{label}: {shown:?}");
        }
        assert!(laid.item_shown(ACCEPT_ITEM) && laid.item_shown(REFUSE_ITEM));
        assert!(!laid.item_shown(SINGLE_ITEM));
        assert!(!dialog.answered());
    }

    #[test]
    fn an_offer_that_cannot_be_refused_shows_one_button() {
        let mut dialog = dialog_of(prompt("Okay", None));
        let laid = dialog.dialog().expect("laid out");
        assert!(laid.item_shown(SINGLE_ITEM));
        assert!(!laid.item_shown(ACCEPT_ITEM) && !laid.item_shown(REFUSE_ITEM));
        assert_eq!(
            texts(&dialog).iter().filter(|text| *text == "Okay").count(),
            1
        );
        click_item(&mut dialog, SINGLE_ITEM);
        assert_eq!(dialog.take_choice(), Some(OfferChoice::Accept));
    }

    #[test]
    fn the_buttons_choose_and_the_choice_is_taken_once() {
        let mut dialog = offer();
        assert_eq!(dialog.take_choice(), None);
        click_item(&mut dialog, REFUSE_ITEM);
        assert!(dialog.answered());
        assert_eq!(dialog.take_choice(), Some(OfferChoice::Refuse));
        assert_eq!(dialog.take_choice(), None, "once");
        click_item(&mut dialog, ACCEPT_ITEM);
        assert_eq!(dialog.take_choice(), None, "answered already");
        let mut dialog = offer();
        click_item(&mut dialog, ACCEPT_ITEM);
        assert_eq!(dialog.take_choice(), Some(OfferChoice::Accept));
    }

    #[test]
    fn return_and_enter_accept_and_escape_does_nothing() {
        let mut dialog = offer();
        dialog.input(&key(Key::Escape));
        assert!(!dialog.answered());
        assert_eq!(dialog.take_choice(), None);
        dialog.input(&Input::Key {
            key: Key::Enter,
            pressed: true,
            repeat: true,
        });
        assert_eq!(dialog.take_choice(), None, "a repeat does nothing");
        dialog.input(&key(Key::Enter));
        assert_eq!(dialog.take_choice(), Some(OfferChoice::Accept));
    }

    #[test]
    fn each_labels_first_letter_chooses_it() {
        let mut dialog = offer();
        dialog.input(&key(Key::Char('n')));
        assert_eq!(dialog.take_choice(), Some(OfferChoice::Refuse));
        let mut dialog = offer();
        dialog.input(&key(Key::Char('I')));
        assert_eq!(
            dialog.take_choice(),
            Some(OfferChoice::Accept),
            "either case"
        );
        let mut dialog = offer();
        dialog.input(&key(Key::Char('x')));
        assert_eq!(dialog.take_choice(), None);
        let mut cannot = dialog_of(prompt("Okay", None));
        cannot.input(&key(Key::Char('n')));
        assert_eq!(cannot.take_choice(), None, "no refuse label");
        cannot.input(&key(Key::Char('o')));
        assert_eq!(cannot.take_choice(), Some(OfferChoice::Accept));
    }

    #[test]
    fn labels_starting_with_the_same_letter_take_no_letter() {
        let mut dialog = dialog_of(prompt("Sure", Some("Scram")));
        dialog.input(&key(Key::Char('s')));
        assert_eq!(dialog.take_choice(), None);
        dialog.input(&key(Key::Enter));
        assert_eq!(dialog.take_choice(), Some(OfferChoice::Accept));
    }

    #[test]
    fn up_and_down_scroll_the_text() {
        let mut dialog = dialog_of(OfferPrompt {
            text: "line\r".repeat(40),
            ..prompt("Yes", Some("No"))
        });
        dialog.input(&key(Key::Down));
        assert_eq!(
            dialog
                .dialog()
                .and_then(|laid| laid.text(TEXT_ITEM))
                .map(crate::ui::ScrollText::first),
            Some(1)
        );
        assert!(!dialog.answered());
    }

    #[test]
    fn without_its_dialog_it_says_why_and_return_accepts() {
        let mut dialog = OfferDialog::new(
            Err("no DLOG 1016".to_owned()),
            prompt("Yes", Some("No")),
            ButtonStyle::STOCK,
        );
        assert_eq!(dialog.problem(), Some("no DLOG 1016"));
        assert!(drawn(&dialog).iter().any(|command| matches!(
            command,
            DrawCommand::Text { text, color, .. } if text.contains("no DLOG 1016") && *color == Color::ERROR
        )));
        dialog.input(&key(Key::Escape));
        assert_eq!(dialog.take_choice(), None);
        dialog.input(&key(Key::Enter));
        assert_eq!(dialog.take_choice(), Some(OfferChoice::Accept));
        assert_eq!(dialog.prompt().mission, MissionId(141));
    }

    const DOWN: Sound = Sound::Ui(crate::sound::UiSound::ButtonDown);
    const UP: Sound = Sound::Ui(crate::sound::UiSound::ButtonUp);

    #[test]
    fn its_buttons_sound_and_a_click_abandoned_does_nothing() {
        let mut dialog = offer();
        let at = dialog
            .dialog()
            .and_then(|laid| laid.item_bounds(ACCEPT_ITEM))
            .expect("accept")
            .center();
        let button = |pressed| Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at,
        };
        dialog.input(&button(true));
        dialog.cancel_pointer();
        dialog.input(&button(false));
        assert_eq!(dialog.take_choice(), None, "abandoned");
        assert_eq!(dialog.take_sounds(), [DOWN]);
        click_item(&mut dialog, ACCEPT_ITEM);
        assert_eq!(dialog.take_sounds(), [DOWN, UP]);
        assert_eq!(dialog.take_choice(), Some(OfferChoice::Accept));
    }

    // A venue's offers.

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    fn opened(offers: &mut VenueOffers) {
        offers.open(
            prompt("I'm in", Some("No thanks")),
            Rc::new(MonoMetrics),
            ButtonStyle::STOCK,
        );
    }

    #[test]
    fn waiting_offers_ask_once_after_their_first_wait() {
        let mut offers = VenueOffers::waiting(Ok(template()), ms(250));
        assert!(!offers.take_request(), "not before any tick");
        offers.tick(ms(249));
        assert!(!offers.take_request());
        offers.tick(ms(1));
        assert!(offers.take_request());
        assert!(!offers.take_request(), "once");
        offers.tick(ms(10_000));
        assert!(!offers.take_request(), "not again until rearmed");
    }

    #[test]
    fn asking_offers_ask_before_any_tick_and_only_once() {
        let mut offers = VenueOffers::asking(Ok(template()));
        assert!(offers.take_request());
        assert!(!offers.take_request());
        offers.tick(ms(10_000));
        assert!(!offers.take_request());
    }

    #[test]
    fn the_wait_does_not_run_down_while_an_offer_is_open() {
        let mut offers = VenueOffers::waiting(Ok(template()), ms(100));
        opened(&mut offers);
        assert!(offers.dialog().is_some());
        offers.tick(ms(500));
        assert!(!offers.take_request(), "paused under the offer");
        offers.input(&key(Key::Enter), &mut Vec::new());
        offers.tick(ms(99));
        assert!(!offers.take_request());
        offers.tick(ms(1));
        assert!(offers.take_request());
    }

    #[test]
    fn after_a_rearm_they_ask_again_after_the_gap() {
        let mut offers = VenueOffers::asking(Ok(template()));
        assert!(offers.take_request());
        offers.rearm(ms(600));
        offers.tick(ms(599));
        assert!(!offers.take_request());
        offers.tick(ms(1));
        assert!(offers.take_request());
    }

    #[test]
    fn an_open_offer_takes_the_input_and_its_choice_is_taken_once() {
        let mut offers = VenueOffers::asking(Ok(template()));
        let mut sounds = Vec::new();
        assert!(
            !offers.input(&key(Key::Enter), &mut sounds),
            "nothing open takes nothing"
        );
        opened(&mut offers);
        assert!(offers.input(&key(Key::Char('x')), &mut sounds));
        assert!(offers.dialog().is_some(), "x chooses nothing");
        assert!(offers.input(&key(Key::Char('n')), &mut sounds));
        assert!(offers.dialog().is_none(), "answered, it closes");
        assert_eq!(
            offers.take_choice(),
            Some((MissionId(141), OfferChoice::Refuse))
        );
        assert_eq!(offers.take_choice(), None, "once");
        assert!(!offers.input(&key(Key::Enter), &mut sounds));
    }

    #[test]
    fn an_open_offers_sounds_pointer_and_drawing_go_through_them() {
        let mut offers = VenueOffers::asking(Ok(template()));
        let mut list = DrawList::new();
        offers.draw(&mut list);
        assert!(list.iter().next().is_none(), "nothing open draws nothing");
        assert!(!offers.cancel_pointer());
        opened(&mut offers);
        let mut list = DrawList::new();
        offers.draw(&mut list);
        let commands: Vec<DrawCommand> = list.iter().cloned().collect();
        assert_eq!(commands, drawn(offers.dialog().expect("open")));
        let at = offers
            .dialog()
            .and_then(OfferDialog::dialog)
            .and_then(|laid| laid.item_bounds(ACCEPT_ITEM))
            .expect("accept")
            .center();
        let button = |pressed| Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at,
        };
        let mut sounds = Vec::new();
        offers.input(&button(true), &mut sounds);
        assert!(offers.cancel_pointer());
        offers.input(&button(false), &mut sounds);
        assert_eq!(offers.take_choice(), None, "abandoned");
        assert_eq!(sounds, [DOWN]);
    }

    #[test]
    fn without_a_template_the_offer_says_why() {
        let mut offers = VenueOffers::asking(Err("no DLOG 1016".to_owned()));
        opened(&mut offers);
        assert_eq!(
            offers.dialog().and_then(OfferDialog::problem),
            Some("no DLOG 1016")
        );
    }
}
