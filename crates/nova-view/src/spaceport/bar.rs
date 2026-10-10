//! The spaceport bar: the stellar's bar text and its buttons, laid out by
//! the interface file's bar dialog (`DLOG`/`DITL` 1013) over its
//! background picture (`_DoBarDialog` @0x48ef5, `_DrawBarButtons`
//! @0x26e1d and `_BarFilter` @0x47a4b in the `EV Nova` executable).
//!
//! The stock dialog is 263 x 185, centred, over `PICT` 8503: the bar text
//! (`dësc` 10000 plus the stellar's ID less 128) in item 7, and below it
//! Hire Escort (item 5, `STR#` 150 #13) and Gamble (item 2, #11) on one
//! row, Holovid (item 3, #12) and Leave (item 1, #1) on the next. Gamble
//! and Holovid are not built yet, so they show greyed. The screen enables
//! its buttons itself, as the stock items are all disabled.
//!
//! Hire Escort, clicked or with H or E (presses, never repeats), asks for
//! the ships for hire ([`BarScreen::take_hire_request`]); it is greyed
//! while the fleet has no room (`_CanHireEscorts` @0x5795). Whoever holds
//! the session answers with the list ([`BarScreen::open_hire`]): the hire
//! screen ([`HireScreen`]) opens over the bar, or, with nothing for hire,
//! the bar says so (`STR#` 2002 #224) in its text box in place of its
//! text, until the next button is activated, as the spaceport shows a
//! refusal. A ship the hire screen asks for is taken through the bar
//! ([`BarScreen::take_hire`]), and after the hire the screen closes back
//! to the bar ([`BarScreen::set_hire`]).
//!
//! Given the offer dialog's template ([`BarScreen::with_offers`]), the bar
//! offers missions as `_BarFilter` (@0x47be0-0x47c0f) and
//! `_OfferMissionFromPort` (@0xa369e) do: [`FIRST_OFFER_DELAY`] after it
//! opens it asks for an offer ([`BarScreen::take_offer_request`]), and
//! whoever holds the session answers with one, opening the
//! [`OfferDialog`] over the bar ([`BarScreen::open_offer`]), or rearms it
//! ([`BarScreen::rearm`]) with the gap drawn. The offer's choice is taken
//! through the bar ([`BarScreen::take_offer_choice`]), and the bar asks
//! again once rearmed. Nothing is asked while the hire screen or an offer
//! is open. [`MISSION_INFO_KEY`] asks for the mission info screen
//! ([`BarScreen::take_info_request`]).
//!
//! Leave, Return, Enter and Escape leave the bar. Without its dialog, the
//! bar says why, and Return or Escape still leaves.

use std::rc::Rc;
use std::time::Duration;

use nova_sim::hire::{HIRE_ESCORT, NONE_FOR_HIRE};
use nova_sim::mission::screens::{FIRST_OFFER_TICKS, mac_ticks};
use nova_sim::{HireList, MissionId, OfferPrompt, ShipId};

use super::catalog::StellarId;
use super::hire::HireScreen;
use super::layout::LEAVE_LABEL;
use super::offer::{OfferChoice, OfferDialog, VenueOffers};
use super::shipyard::ShipyardCatalog;
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
use crate::ui::dialog::{Dialog, DialogEvent, DialogTemplate, Role};

/// The bar dialog's `DLOG` (and `DITL`) ID.
pub const BAR_DIALOG: i16 = 1013;
/// The picture the bar is drawn over.
pub const BACKGROUND: ImageKey = ImageKey::picture(8503);

/// Leave's item.
pub const LEAVE_ITEM: usize = 1;
/// Gamble's item.
pub const GAMBLE_ITEM: usize = 2;
/// Holovid's item.
pub const HOLOVID_ITEM: usize = 3;
/// Hire Escort's item.
pub const HIRE_ITEM: usize = 5;
/// The bar text's item.
pub const TEXT_ITEM: usize = 7;

/// `STR#` 150 #11.
pub const GAMBLE_LABEL: &str = "Gamble";
/// `STR#` 150 #12.
pub const HOLOVID_LABEL: &str = "Holovid";

/// The keys that press Hire Escort (`_BarFilter`: Mac virtual keys 4 and
/// 14).
pub const HIRE_KEYS: [Key; 2] = [Key::Char('h'), Key::Char('e')];

/// The key that asks for the mission info screen from the bar, the
/// spaceport and the mission computer. The original reads it from the
/// player's key settings (`_keyPrefs` +0x50), whose default is not in
/// `Nova-DF.rsrc`; I is free on every spaceport screen.
pub const MISSION_INFO_KEY: Key = Key::Char('i');

/// How long after the bar opens it first asks for an offer: 15 of the
/// Mac's ticks (`_DoBarDialog` @0x48f51-0x48f5f).
pub const FIRST_OFFER_DELAY: Duration = mac_ticks(FIRST_OFFER_TICKS);

/// The first bar text's `dësc`, for `spöb` 128.
pub const FIRST_BAR_DESC: i16 = 10_000;
/// The first `spöb` ID.
const FIRST_STELLAR: i16 = 128;

/// The `dësc` of `stellar`'s bar text: [`FIRST_BAR_DESC`] + its ID - 128.
#[must_use]
pub fn bar_desc_id(stellar: StellarId) -> i16 {
    FIRST_BAR_DESC.saturating_add(stellar.0.saturating_sub(FIRST_STELLAR))
}

/// What the hire screen is built from: the "Shipyard" dialog's template
/// and the "Shipyard Info" panel's (or why there are none), and where the
/// ships' pictures and descriptions, and the bar's text, come from.
#[derive(Clone)]
pub struct Hiring {
    /// The "Shipyard" dialog's template, or why there is none.
    pub template: Result<DialogTemplate, String>,
    /// The "Shipyard Info" panel's template, or why there is none.
    pub info: Result<DialogTemplate, String>,
    /// The pictures and descriptions.
    pub catalog: Rc<dyn ShipyardCatalog>,
}

impl std::fmt::Debug for Hiring {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Hiring")
            .field("template", &self.template)
            .field("info", &self.info)
            .finish_non_exhaustive()
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

/// The bar, laid out.
#[derive(Clone, Debug)]
struct Laid {
    dialog: Dialog,
    metrics: MetricsHandle,
    /// The bar's text, to show again after a message.
    text: String,
    /// The message shown in its place, if any.
    said: Option<&'static str>,
}

/// What the bar has asked whoever holds the session for.
#[derive(Clone, Copy, Debug, Default)]
struct Asks {
    /// The ships for hire: Hire Escort was pressed.
    hire: bool,
    /// The mission info screen: [`MISSION_INFO_KEY`] was pressed.
    info: bool,
}

/// The bar of the stellar landed on.
#[derive(Clone, Debug)]
pub struct BarScreen {
    /// The dialog, or why it cannot be shown.
    laid: Result<Laid, String>,
    hiring: Hiring,
    style: ButtonStyle,
    /// Whether the fleet has room for another escort.
    room: bool,
    /// The hire screen, while it is open.
    hire: Option<HireScreen>,
    /// The mission offers, once given.
    offering: Option<VenueOffers>,
    /// What the bar has asked for since each was last taken.
    asks: Asks,
    closed: bool,
    sounds: Vec<Sound>,
}

impl BarScreen {
    /// `stellar`'s bar, laid out by `layout`: the bar dialog's template and
    /// the metrics its text is measured by, or why there are none. Its text
    /// is read from `hiring`'s catalog now (none when it is missing), its
    /// buttons are labelled in `style`, and Hire Escort is greyed unless
    /// the fleet has `room`.
    #[must_use]
    pub fn new(
        layout: Result<(DialogTemplate, Rc<dyn TextMetrics>), String>,
        stellar: StellarId,
        hiring: Hiring,
        style: ButtonStyle,
        room: bool,
    ) -> Self {
        let laid = layout.map(|(mut template, metrics)| {
            for item in [LEAVE_ITEM, GAMBLE_ITEM, HOLOVID_ITEM, HIRE_ITEM] {
                if let Some(found) = template.items.get_mut(item - 1) {
                    found.enabled = true;
                }
            }
            let text = hiring
                .catalog
                .description(bar_desc_id(stellar))
                .unwrap_or_default();
            let roles = [
                (LEAVE_ITEM, Role::Button(LEAVE_LABEL.to_owned())),
                (GAMBLE_ITEM, Role::Button(GAMBLE_LABEL.to_owned())),
                (HOLOVID_ITEM, Role::Button(HOLOVID_LABEL.to_owned())),
                (HIRE_ITEM, Role::Button(HIRE_ESCORT.to_owned())),
                (
                    TEXT_ITEM,
                    Role::ScrollText {
                        text: text.clone(),
                        font: Font::Geneva,
                        size: BODY_SIZE,
                        color: BODY_COLOR,
                    },
                ),
            ];
            let mut dialog = Dialog::new(&template, &roles, Rc::clone(&metrics))
                .with_buttons(ButtonSkin::NOVA, style)
                .with_default(Some(LEAVE_ITEM))
                .with_cancel(Some(LEAVE_ITEM));
            dialog.set_greyed(GAMBLE_ITEM, true);
            dialog.set_greyed(HOLOVID_ITEM, true);
            dialog.set_greyed(HIRE_ITEM, !room);
            Laid {
                dialog,
                metrics: MetricsHandle(metrics),
                text,
                said: None,
            }
        });
        Self {
            laid,
            hiring,
            style,
            room,
            hire: None,
            offering: None,
            asks: Asks::default(),
            closed: false,
            sounds: Vec::new(),
        }
    }

    /// The bar with mission offers, each shown in the offer dialog laid
    /// out by `template` (or saying why there is none): it asks for the
    /// first [`FIRST_OFFER_DELAY`] after it opens.
    #[must_use]
    pub fn with_offers(self, template: Result<DialogTemplate, String>) -> Self {
        Self {
            offering: Some(VenueOffers::waiting(template, FIRST_OFFER_DELAY)),
            ..self
        }
    }

    /// Whether the bar has asked for an offer since this was last asked
    /// (`_BarFilter` @0x47be0-0x47c0f): whoever holds the session answers
    /// with [`BarScreen::open_offer`], or, with none, rearms it
    /// ([`BarScreen::rearm`]).
    pub fn take_offer_request(&mut self) -> bool {
        self.offering
            .as_mut()
            .is_some_and(VenueOffers::take_request)
    }

    /// Opens the offer dialog over the bar on `prompt`.
    pub fn open_offer(&mut self, prompt: OfferPrompt) {
        let (Ok(laid), Some(offering)) = (&self.laid, &mut self.offering) else {
            return;
        };
        offering.open(prompt, Rc::clone(&laid.metrics.0), self.style);
    }

    /// The offer dialog, while it is open.
    #[must_use]
    pub fn offer_dialog(&self) -> Option<&OfferDialog> {
        self.offering.as_ref()?.dialog()
    }

    /// The offer answered since this was last taken, and the choice made,
    /// once.
    pub fn take_offer_choice(&mut self) -> Option<(MissionId, OfferChoice)> {
        self.offering.as_mut()?.take_choice()
    }

    /// Asks for the next offer after `gap`.
    pub fn rearm(&mut self, gap: Duration) {
        if let Some(offering) = &mut self.offering {
            offering.rearm(gap);
        }
    }

    /// The dialog, if it could be laid out.
    #[must_use]
    pub fn dialog(&self) -> Option<&Dialog> {
        self.laid.as_ref().ok().map(|laid| &laid.dialog)
    }

    /// Why the bar cannot be shown, if it cannot.
    #[must_use]
    pub fn problem(&self) -> Option<&str> {
        self.laid.as_ref().err().map(String::as_str)
    }

    /// The message shown in place of the bar's text, if any.
    #[must_use]
    pub fn message(&self) -> Option<&str> {
        self.laid.as_ref().ok()?.said
    }

    /// Whether the player has left the bar.
    #[must_use]
    pub fn closed(&self) -> bool {
        self.closed
    }

    /// The hire screen, while it is open.
    #[must_use]
    pub fn hire_screen(&self) -> Option<&HireScreen> {
        self.hire.as_ref()
    }

    /// Whether Hire Escort has asked for the ships for hire since this
    /// was last asked: whoever holds the session answers with
    /// [`BarScreen::open_hire`].
    pub fn take_hire_request(&mut self) -> bool {
        std::mem::take(&mut self.asks.hire)
    }

    /// Whether [`MISSION_INFO_KEY`] has asked for the mission info screen
    /// since this was last asked.
    pub fn take_info_request(&mut self) -> bool {
        std::mem::take(&mut self.asks.info)
    }

    /// Opens the hire screen over the bar on `list`; with nothing for
    /// hire, the bar says so instead.
    pub fn open_hire(&mut self, list: HireList) {
        let Ok(laid) = &mut self.laid else {
            return;
        };
        if list.rows.is_empty() {
            laid.dialog.set_text(TEXT_ITEM, NONE_FOR_HIRE);
            laid.said = Some(NONE_FOR_HIRE);
            return;
        }
        let layout = self
            .hiring
            .template
            .clone()
            .map(|template| (template, Rc::clone(&laid.metrics.0)));
        self.hire = Some(HireScreen::new(
            layout,
            self.hiring.info.clone(),
            list,
            Rc::clone(&self.hiring.catalog),
            self.style,
        ));
    }

    /// The ship the hire screen asked for since it was last taken, once.
    pub fn take_hire(&mut self) -> Option<ShipId> {
        self.hire.as_mut()?.take_order()
    }

    /// After a hire: the hire screen closes back to the bar, whose Hire
    /// Escort is greyed unless `list`'s fleet still has room.
    pub fn set_hire(&mut self, list: &HireList) {
        self.hire = None;
        self.room = list.room;
        if let Ok(laid) = &mut self.laid {
            laid.dialog.set_greyed(HIRE_ITEM, !self.room);
        }
    }

    /// Asks for the ships for hire, if the fleet has room, showing the
    /// bar's text again in place of a message.
    fn ask(&mut self) {
        if !self.room {
            return;
        }
        if let Ok(laid) = &mut self.laid
            && laid.said.take().is_some()
        {
            laid.dialog.set_text(TEXT_ITEM, &laid.text);
        }
        self.asks.hire = true;
    }

    /// Activates dialog item `item`: Leave leaves and Hire Escort asks.
    /// Anything else does nothing.
    fn activate(&mut self, item: usize) {
        match item {
            LEAVE_ITEM => self.closed = true,
            HIRE_ITEM => self.ask(),
            _ => {}
        }
    }
}

impl Screen for BarScreen {
    /// The hire screen takes every input while it is open; otherwise
    /// every input goes to the bar. It never quits.
    fn input(&mut self, input: &Input) -> ScreenAction {
        if let Some(offering) = &mut self.offering
            && offering.input(input, &mut self.sounds)
        {
            return ScreenAction::None;
        }
        if let Some(hire) = &mut self.hire {
            hire.input(input);
            self.sounds.extend(hire.take_sounds());
            if hire.closed() {
                self.hire = None;
            }
            return ScreenAction::None;
        }
        let Ok(laid) = &mut self.laid else {
            if let Input::Key {
                key: Key::Enter | Key::Escape,
                pressed: true,
                repeat: false,
            } = *input
            {
                self.closed = true;
            }
            return ScreenAction::None;
        };
        if let Input::Key {
            key,
            pressed: true,
            repeat: false,
        } = *input
            && HIRE_KEYS.contains(&key)
        {
            self.ask();
            return ScreenAction::None;
        }
        if *input
            == (Input::Key {
                key: MISSION_INFO_KEY,
                pressed: true,
                repeat: false,
            })
        {
            self.asks.info = true;
            return ScreenAction::None;
        }
        let event = laid.dialog.input(input);
        self.sounds.extend(laid.dialog.take_sound().map(Sound::Ui));
        if let Some(DialogEvent::Item(item)) = event {
            self.activate(item);
        }
        ScreenAction::None
    }

    /// The wait for the next offer runs down, while neither the hire
    /// screen nor an offer is open; once it is out the bar asks.
    fn tick(&mut self, dt: Duration) {
        if let Some(offering) = &mut self.offering
            && self.hire.is_none()
        {
            offering.tick(dt);
        }
    }

    fn draw(&self, list: &mut DrawList) {
        match &self.laid {
            Ok(laid) => {
                list.picture(BACKGROUND, laid.dialog.bounds().min);
                laid.dialog.draw(list);
            }
            Err(reason) => {
                list.text(
                    format!("Cannot show the bar: {reason}. Press Return or Escape to leave it."),
                    PROBLEM_AT,
                    PROBLEM_SIZE,
                    None,
                    Color::ERROR,
                );
            }
        }
        if let Some(hire) = &self.hire {
            hire.draw(list);
        }
        if let Some(offering) = &self.offering {
            offering.draw(list);
        }
    }

    fn cancel_pointer(&mut self) {
        if self
            .offering
            .as_mut()
            .is_some_and(VenueOffers::cancel_pointer)
        {
            return;
        }
        if let Some(hire) = &mut self.hire {
            hire.cancel_pointer();
        } else if let Ok(laid) = &mut self.laid {
            laid.dialog.cancel_pointer();
        }
    }

    /// The buttons' sounds, the hire screen's included, in order.
    fn take_sounds(&mut self) -> Vec<Sound> {
        std::mem::take(&mut self.sounds)
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::cell::RefCell;

    use nova_sim::{HireRefusal, HireRow, ShipFields, ShipSpecs};

    use super::*;
    use crate::draw::DrawCommand;
    use crate::geometry::{Bounds, Point};
    use crate::input::MouseButton;
    use crate::spaceport::catalog::{PortRecord, SpaceportCatalog};
    use crate::spaceport::shipyard::ShipBaseImages;
    use crate::text::fixture::MonoMetrics;
    use crate::ui::catalog::DescriptionSource;
    use crate::ui::dialog::{ItemSpec, ItemTemplate, Placement};

    fn rect(x: f32, y: f32, w: f32, h: f32) -> Bounds {
        Bounds::at(Point::new(x, y), w, h)
    }

    /// `items`, each (x, y, w, h), as disabled user items.
    fn items(items: &[(f32, f32, f32, f32)]) -> Vec<ItemTemplate> {
        items
            .iter()
            .map(|&(x, y, w, h)| ItemTemplate {
                bounds: rect(x, y, w, h),
                enabled: false,
                kind: ItemSpec::User,
            })
            .collect()
    }

    /// Stock "Bar": `DLOG` 1013, 263 x 185 and centred, and its ten
    /// disabled user items as `DITL` 1013 has them, 4, 6 and 8-10 parked
    /// outside it.
    fn template() -> DialogTemplate {
        DialogTemplate {
            bounds: rect(40.0, 40.0, 263.0, 185.0),
            placement: Placement::Center,
            items: items(&[
                (156.0, 154.0, 99.0, 26.0),
                (156.0, 125.0, 99.0, 26.0),
                (6.0, 154.0, 146.0, 26.0),
                (54.0, 214.0, 121.0, 25.0),
                (6.0, 125.0, 146.0, 26.0),
                (105.0, 227.0, 121.0, 25.0),
                (16.0, 10.0, 230.0, 106.0),
                (61.0, 303.0, 32.0, 32.0),
                (103.0, 303.0, 32.0, 32.0),
                (146.0, 303.0, 32.0, 32.0),
            ]),
        }
    }

    /// Where the dialog goes: (1024 - 263) / 2 and (768 - 185) / 2,
    /// floored.
    const ORIGIN: Point = Point::new(380.0, 291.0);

    /// Every description is "dësc n."; records what it is asked.
    struct FakeArt {
        asked: RefCell<Vec<String>>,
    }

    impl SpaceportCatalog for FakeArt {
        fn stellar_port(&self, id: StellarId) -> Result<PortRecord, String> {
            panic!("asked for spöb {}", id.0)
        }

        fn picture_exists(&self, _id: i16) -> bool {
            false
        }
    }

    impl DescriptionSource for FakeArt {
        fn description(&self, id: i16) -> Result<String, String> {
            self.asked.borrow_mut().push(format!("dësc {id}"));
            Ok(format!("dësc {id}."))
        }

        fn button_style(&self) -> ButtonStyle {
            panic!("the style is given")
        }
    }

    impl ShipBaseImages for FakeArt {
        fn ship_base_images(&self) -> Vec<(ShipId, i16)> {
            Vec::new()
        }
    }

    fn hiring(art: &Rc<FakeArt>) -> Hiring {
        Hiring {
            template: Err("no DLOG 1004".to_owned()),
            info: Err("no DLOG 1005".to_owned()),
            catalog: Rc::clone(art) as Rc<dyn ShipyardCatalog>,
        }
    }

    /// Viking's bar (`spöb` 157), its fleet with `room` or not.
    fn bar_of(art: &Rc<FakeArt>, room: bool) -> BarScreen {
        BarScreen::new(
            Ok((template(), Rc::new(MonoMetrics))),
            StellarId(157),
            hiring(art),
            ButtonStyle::STOCK,
            room,
        )
    }

    fn bar() -> BarScreen {
        bar_of(
            &Rc::new(FakeArt {
                asked: RefCell::default(),
            }),
            true,
        )
    }

    fn drawn(screen: &BarScreen) -> Vec<DrawCommand> {
        let mut list = DrawList::new();
        screen.draw(&mut list);
        list.iter().cloned().collect()
    }

    fn texts(screen: &BarScreen) -> Vec<(String, Color)> {
        drawn(screen)
            .into_iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, color, .. } => Some((text, color)),
                _ => None,
            })
            .collect()
    }

    fn key(key: Key, repeat: bool) -> Input {
        Input::Key {
            key,
            pressed: true,
            repeat,
        }
    }

    fn click_item(screen: &mut BarScreen, number: usize) {
        let at = screen
            .dialog()
            .expect("laid out")
            .item_bounds(number)
            .expect("an item")
            .center();
        for pressed in [true, false] {
            screen.input(&Input::PointerButton {
                button: MouseButton::Left,
                pressed,
                at,
            });
        }
    }

    /// Whether `label` draws enabled.
    fn enabled(screen: &BarScreen, label: &str) -> bool {
        texts(screen)
            .into_iter()
            .find_map(|(text, color)| (text == label).then_some(color))
            .is_some_and(|color| color != ButtonStyle::STOCK.grey)
    }

    /// A list of `rows` ships, all hireable, the fleet with room.
    fn list(rows: usize) -> HireList {
        HireList {
            rows: (0..rows)
                .map(|n| HireRow {
                    id: ShipId(128 + n as i16),
                    name: format!("Ship {n}"),
                    short_name: format!("Ship {n}"),
                    fee: 970,
                    wage: 100,
                    specs: ShipSpecs {
                        fields: ShipFields::default(),
                        max_gun: 0,
                        max_tur: 0,
                        length: 0,
                        crew: 0,
                    },
                    hire: Ok(()),
                })
                .collect(),
            cash: 25_000,
            room: true,
        }
    }

    #[test]
    fn the_named_values() {
        assert_eq!(BAR_DIALOG, 1013);
        assert_eq!(BACKGROUND, ImageKey::picture(8503));
        assert_eq!(
            [LEAVE_ITEM, GAMBLE_ITEM, HOLOVID_ITEM, HIRE_ITEM, TEXT_ITEM],
            [1, 2, 3, 5, 7]
        );
        assert_eq!(bar_desc_id(StellarId(128)), 10_000);
        assert_eq!(bar_desc_id(StellarId(157)), 10_029, "Viking");
    }

    #[test]
    fn it_draws_its_picture_then_its_dialog_with_the_bar_text() {
        let art = Rc::new(FakeArt {
            asked: RefCell::default(),
        });
        let screen = bar_of(&art, true);
        assert_eq!(*art.asked.borrow(), ["dësc 10029"]);
        let dialog = screen.dialog().expect("laid out");
        assert_eq!(dialog.bounds().min, ORIGIN);
        let commands = drawn(&screen);
        assert_eq!(
            commands[0],
            DrawCommand::Picture {
                image: BACKGROUND,
                top_left: ORIGIN,
            }
        );
        let mut alone = DrawList::new();
        dialog.draw(&mut alone);
        assert_eq!(commands[1..], alone.iter().cloned().collect::<Vec<_>>()[..]);
        let text = dialog.scroll_text().expect("the bar text");
        assert_eq!(text.lines(), ["dësc 10029."]);
        assert_eq!(
            dialog.item_bounds(TEXT_ITEM),
            Some(rect(ORIGIN.x + 16.0, ORIGIN.y + 10.0, 230.0, 106.0))
        );
        let words = texts(&screen);
        for label in [LEAVE_LABEL, HIRE_ESCORT] {
            assert!(enabled(&screen, label), "{label}: {words:?}");
        }
        for label in [GAMBLE_LABEL, HOLOVID_LABEL] {
            assert!(!enabled(&screen, label), "{label} greyed");
            assert!(words.iter().any(|(text, _)| text == label), "{label}");
        }
        assert_eq!(screen.problem(), None);
        assert_eq!(screen.message(), None);
    }

    #[test]
    fn leave_return_enter_and_escape_leave_the_bar() {
        for leave in [Key::Enter, Key::Escape] {
            let mut screen = bar();
            screen.input(&key(leave, false));
            assert!(screen.closed(), "{leave:?}");
        }
        let mut screen = bar();
        click_item(&mut screen, LEAVE_ITEM);
        assert!(screen.closed());
        let mut screen = bar();
        click_item(&mut screen, GAMBLE_ITEM);
        click_item(&mut screen, HOLOVID_ITEM);
        assert!(!screen.closed());
        assert!(!screen.take_hire_request(), "greyed, they do nothing");
    }

    #[test]
    fn hire_escort_h_and_e_ask_for_the_ships_for_hire_on_a_press_only() {
        let mut screen = bar();
        assert!(!screen.take_hire_request());
        click_item(&mut screen, HIRE_ITEM);
        assert!(screen.take_hire_request());
        assert!(!screen.take_hire_request(), "once");
        for hire in HIRE_KEYS {
            screen.input(&key(hire, true));
            assert!(!screen.take_hire_request(), "{hire:?} repeating");
            screen.input(&key(hire, false));
            assert!(screen.take_hire_request(), "{hire:?}");
        }
        assert_eq!(HIRE_KEYS, [Key::Char('h'), Key::Char('e')]);
        assert!(!screen.closed());
    }

    #[test]
    fn with_a_full_fleet_hire_escort_is_greyed_and_asks_nothing() {
        let art = Rc::new(FakeArt {
            asked: RefCell::default(),
        });
        let mut screen = bar_of(&art, false);
        assert!(!enabled(&screen, HIRE_ESCORT));
        click_item(&mut screen, HIRE_ITEM);
        screen.input(&key(Key::Char('h'), false));
        assert!(!screen.take_hire_request());
    }

    #[test]
    fn with_nothing_for_hire_the_bar_says_so_until_the_next_button() {
        let mut screen = bar();
        click_item(&mut screen, HIRE_ITEM);
        screen.open_hire(list(0));
        assert!(screen.hire_screen().is_none());
        assert_eq!(screen.message(), Some(NONE_FOR_HIRE));
        let lines = |screen: &BarScreen| {
            screen
                .dialog()
                .expect("laid out")
                .scroll_text()
                .expect("text")
                .lines()
                .join(" ")
        };
        assert_eq!(lines(&screen), NONE_FOR_HIRE);
        click_item(&mut screen, HIRE_ITEM);
        assert_eq!(screen.message(), None);
        assert_eq!(lines(&screen), "dësc 10029.");
    }

    #[test]
    fn the_hire_screen_opens_over_the_bar_and_takes_every_input_until_it_closes() {
        let mut screen = bar();
        screen.open_hire(list(2));
        let hire = screen.hire_screen().expect("open");
        assert_eq!(hire.problem(), Some("no DLOG 1004"), "its own template");
        assert_eq!(hire.list().rows.len(), 2);
        assert!(
            texts(&screen)
                .iter()
                .any(|(text, _)| text.contains("no DLOG 1004")),
            "drawn over the bar"
        );
        screen.input(&key(Key::Enter, false));
        assert!(screen.hire_screen().is_none(), "closed back to the bar");
        assert!(!screen.closed());
    }

    #[test]
    fn a_hire_asked_for_is_taken_through_the_bar_and_closes_the_hire_screen() {
        let art = Rc::new(FakeArt {
            asked: RefCell::default(),
        });
        let mut screen = BarScreen::new(
            Ok((template(), Rc::new(MonoMetrics))),
            StellarId(157),
            Hiring {
                template: Ok(DialogTemplate {
                    bounds: rect(100.0, 100.0, 765.0, 323.0),
                    placement: Placement::Center,
                    items: items(&[
                        (365.0, 289.0, 109.0, 25.0),
                        (248.0, 440.0, 68.0, 30.0),
                        (201.0, 380.0, 102.0, 18.0),
                        (144.0, 438.0, 69.0, 22.0),
                        (9.0, 8.0, 333.0, 271.0),
                        (354.0, 10.0, 192.0, 267.0),
                        (480.0, 289.0, 109.0, 25.0),
                        (557.0, 8.0, 200.0, 200.0),
                        (614.0, 214.0, 143.0, 100.0),
                        (253.0, 289.0, 89.0, 25.0),
                        (365.0, 431.0, 69.0, 22.0),
                        (141.0, 288.0, 25.0, 25.0),
                        (171.0, 288.0, 25.0, 25.0),
                    ]),
                }),
                ..hiring(&art)
            },
            ButtonStyle::STOCK,
            true,
        );
        assert_eq!(screen.take_hire(), None, "no hire screen");
        screen.open_hire(list(1));
        screen.input(&key(Key::Char('h'), false));
        assert_eq!(screen.take_hire(), Some(ShipId(128)));
        assert_eq!(screen.take_hire(), None, "once");
        let full = HireList {
            room: false,
            rows: vec![HireRow {
                hire: Err(HireRefusal::FleetFull),
                ..list(1).rows[0].clone()
            }],
            ..list(1)
        };
        screen.set_hire(&full);
        assert!(screen.hire_screen().is_none());
        assert!(!enabled(&screen, HIRE_ESCORT), "the fleet is full");
        screen.set_hire(&list(1));
        assert!(enabled(&screen, HIRE_ESCORT));
    }

    #[test]
    fn without_its_dialog_the_bar_says_why_and_return_leaves() {
        let art = Rc::new(FakeArt {
            asked: RefCell::default(),
        });
        let mut screen = BarScreen::new(
            Err("no DLOG 1013".to_owned()),
            StellarId(157),
            hiring(&art),
            ButtonStyle::STOCK,
            true,
        );
        assert_eq!(screen.problem(), Some("no DLOG 1013"));
        assert!(
            texts(&screen)
                .iter()
                .any(|(text, color)| text.contains("no DLOG 1013") && *color == Color::ERROR)
        );
        screen.open_hire(list(1));
        assert!(screen.hire_screen().is_none());
        screen.input(&key(Key::Char('h'), false));
        assert!(!screen.take_hire_request());
        screen.input(&key(Key::Escape, false));
        assert!(screen.closed());
    }

    const DOWN: Sound = Sound::Ui(crate::sound::UiSound::ButtonDown);
    const UP: Sound = Sound::Ui(crate::sound::UiSound::ButtonUp);

    #[test]
    fn its_buttons_sound_and_a_click_abandoned_does_nothing() {
        let mut screen = bar();
        click_item(&mut screen, HIRE_ITEM);
        assert_eq!(screen.take_sounds(), [DOWN, UP]);
        assert_eq!(screen.take_sounds(), [], "taken");
        assert!(screen.take_hire_request());
        let at = screen
            .dialog()
            .expect("laid out")
            .item_bounds(HIRE_ITEM)
            .expect("an item")
            .center();
        let button = |pressed| Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at,
        };
        screen.input(&button(true));
        screen.cancel_pointer();
        screen.input(&button(false));
        assert!(!screen.take_hire_request(), "the click was abandoned");
        assert_eq!(screen.take_sounds(), [DOWN]);
    }

    #[test]
    fn the_hire_screens_sounds_and_pointer_go_through_the_bar() {
        let art = Rc::new(FakeArt {
            asked: RefCell::default(),
        });
        let mut screen = BarScreen::new(
            Ok((template(), Rc::new(MonoMetrics))),
            StellarId(157),
            Hiring {
                template: Ok(DialogTemplate {
                    bounds: rect(100.0, 100.0, 765.0, 323.0),
                    placement: Placement::Center,
                    items: items(&[
                        (365.0, 289.0, 109.0, 25.0),
                        (248.0, 440.0, 68.0, 30.0),
                        (201.0, 380.0, 102.0, 18.0),
                        (144.0, 438.0, 69.0, 22.0),
                        (9.0, 8.0, 333.0, 271.0),
                        (354.0, 10.0, 192.0, 267.0),
                        (480.0, 289.0, 109.0, 25.0),
                        (557.0, 8.0, 200.0, 200.0),
                        (614.0, 214.0, 143.0, 100.0),
                        (253.0, 289.0, 89.0, 25.0),
                        (365.0, 431.0, 69.0, 22.0),
                        (141.0, 288.0, 25.0, 25.0),
                        (171.0, 288.0, 25.0, 25.0),
                    ]),
                }),
                ..hiring(&art)
            },
            ButtonStyle::STOCK,
            true,
        );
        screen.open_hire(list(1));
        let hire = screen
            .hire_screen()
            .and_then(HireScreen::dialog)
            .and_then(|dialog| dialog.item_bounds(7))
            .expect("Hire Escort")
            .center();
        let button = |pressed| Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at: hire,
        };
        screen.input(&button(true));
        screen.cancel_pointer();
        screen.input(&button(false));
        assert_eq!(screen.take_hire(), None, "the click was abandoned");
        assert_eq!(screen.take_sounds(), [DOWN]);
        screen.input(&button(true));
        screen.input(&button(false));
        assert_eq!(screen.take_hire(), Some(ShipId(128)));
        assert_eq!(screen.take_sounds(), [DOWN, UP]);
    }

    #[test]
    fn its_debug_names_what_it_holds() {
        let debug = format!("{:?}", bar());
        for part in ["BarScreen", "Hiring", "no DLOG 1004", "TextMetrics"] {
            assert!(debug.contains(part), "{part}: {debug}");
        }
    }

    // The bar's mission offers.

    use crate::spaceport::offer::{OfferChoice, REFUSE_ITEM};
    use nova_sim::mission::OfferLabels;
    use nova_sim::{MissionId, OfferPrompt};

    /// The offer dialog's shape, smaller: 300 x 200, fixed at (0, 0),
    /// with accept (1), refuse (2), the text (3) and the one button (6).
    fn offer_template() -> DialogTemplate {
        let mut slots = items(&[(0.0, 300.0, 1.0, 1.0); 6]);
        slots[0].bounds = rect(200.0, 170.0, 90.0, 25.0);
        slots[1].bounds = rect(100.0, 170.0, 90.0, 25.0);
        slots[2].bounds = rect(10.0, 10.0, 280.0, 150.0);
        slots[5].bounds = rect(150.0, 170.0, 90.0, 25.0);
        DialogTemplate {
            bounds: rect(0.0, 0.0, 300.0, 200.0),
            placement: Placement::Fixed,
            items: slots,
        }
    }

    fn prompt() -> OfferPrompt {
        OfferPrompt {
            mission: MissionId(141),
            text: "Ferry this to Mars.".to_owned(),
            labels: OfferLabels {
                accept: "I'm in".to_owned(),
                refuse: Some("No".to_owned()),
            },
        }
    }

    /// Viking's bar, its offers given.
    fn offering() -> BarScreen {
        bar().with_offers(Ok(offer_template()))
    }

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    #[test]
    fn a_quarter_second_after_the_bar_opens_it_asks_for_an_offer_once() {
        let mut screen = offering();
        screen.tick(ms(249));
        assert!(!screen.take_offer_request());
        screen.tick(ms(1));
        assert!(screen.take_offer_request());
        assert!(!screen.take_offer_request(), "once");
        screen.tick(ms(10_000));
        assert!(!screen.take_offer_request(), "not again until rearmed");
        assert_eq!(FIRST_OFFER_DELAY, ms(250));
    }

    #[test]
    fn without_offers_given_the_bar_never_asks() {
        let mut screen = bar();
        screen.tick(ms(10_000));
        assert!(!screen.take_offer_request());
        screen.open_offer(prompt());
        assert!(screen.offer_dialog().is_none());
    }

    #[test]
    fn an_offer_opens_over_the_bar_and_takes_every_input_until_it_is_answered() {
        let mut screen = offering();
        screen.open_offer(prompt());
        let offer = screen.offer_dialog().expect("open");
        assert_eq!(offer.prompt(), &prompt());
        assert_eq!(offer.problem(), None, "laid out by the template given");
        let mut expected = drawn(&bar());
        let mut over = DrawList::new();
        offer.draw(&mut over);
        expected.extend(over.iter().cloned());
        assert_eq!(drawn(&screen), expected);
        screen.input(&key(Key::Escape, false));
        assert!(!screen.closed(), "Escape does nothing in the offer");
        assert!(screen.offer_dialog().is_some());
        assert_eq!(screen.take_offer_choice(), None);
        screen.input(&key(Key::Char('n'), false));
        assert!(screen.offer_dialog().is_none(), "answered, it closes");
        assert_eq!(
            screen.take_offer_choice(),
            Some((MissionId(141), OfferChoice::Refuse))
        );
        assert_eq!(screen.take_offer_choice(), None, "once");
    }

    #[test]
    fn after_an_offer_the_next_ask_comes_after_the_gap_given() {
        let mut screen = offering();
        screen.tick(ms(250));
        assert!(screen.take_offer_request());
        screen.open_offer(prompt());
        screen.tick(ms(10_000));
        assert!(!screen.take_offer_request(), "not while the offer is open");
        screen.input(&key(Key::Enter, false));
        assert_eq!(
            screen.take_offer_choice(),
            Some((MissionId(141), OfferChoice::Accept))
        );
        screen.rearm(ms(600));
        screen.tick(ms(599));
        assert!(!screen.take_offer_request());
        screen.tick(ms(1));
        assert!(screen.take_offer_request());
    }

    #[test]
    fn no_offer_is_asked_for_while_the_hire_screen_is_open() {
        let mut screen = offering();
        screen.open_hire(list(1));
        screen.tick(ms(10_000));
        assert!(!screen.take_offer_request());
        screen.input(&key(Key::Escape, false));
        assert!(screen.hire_screen().is_none());
        screen.tick(ms(250));
        assert!(
            screen.take_offer_request(),
            "the wait goes on once it closes"
        );
    }

    #[test]
    fn the_offers_buttons_sound_and_its_pointer_goes_through_the_bar() {
        let mut screen = offering();
        screen.open_offer(prompt());
        let at = screen
            .offer_dialog()
            .and_then(crate::spaceport::OfferDialog::dialog)
            .and_then(|dialog| dialog.item_bounds(REFUSE_ITEM))
            .expect("refuse")
            .center();
        let button = |pressed| Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at,
        };
        screen.input(&button(true));
        screen.cancel_pointer();
        screen.input(&button(false));
        assert_eq!(screen.take_offer_choice(), None, "abandoned");
        assert_eq!(screen.take_sounds(), [DOWN]);
        screen.input(&button(true));
        screen.input(&button(false));
        assert_eq!(
            screen.take_offer_choice(),
            Some((MissionId(141), OfferChoice::Refuse))
        );
        assert_eq!(screen.take_sounds(), [DOWN, UP]);
    }

    #[test]
    fn the_mission_info_key_asks_for_mission_info_on_a_press_only() {
        let mut screen = bar();
        assert!(!screen.take_info_request());
        screen.input(&key(MISSION_INFO_KEY, true));
        assert!(!screen.take_info_request(), "a repeat");
        screen.input(&key(MISSION_INFO_KEY, false));
        assert!(screen.take_info_request());
        assert!(!screen.take_info_request(), "once");
        assert!(!screen.take_hire_request());
        assert_eq!(MISSION_INFO_KEY, Key::Char('i'));
        assert!(!HIRE_KEYS.contains(&MISSION_INFO_KEY));
    }
}
