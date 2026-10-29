//! The spaceport screen: the stellar landed on, laid out by the interface
//! file's "Spaceport" dialog over its background picture.
//!
//! The screen reads everything once, when it is built: the stellar's
//! record and landscape through the [`SpaceportCatalog`] port, and its
//! description (`dësc` = `spöb` ID) and the button style through
//! [`DescriptionSource`]. It draws the background, then the landscape
//! stretched into its item, then the stellar's name centred in the name
//! bar, then the dialog: the description scrolling in its box and the
//! seven buttons of [`buttons`](super::buttons), as `_DrawPlanetButtons`
//! draws them. A service the stellar's flags don't offer is drawn
//! disabled, and a click on it does nothing.
//!
//! Leave, Return and Escape leave: the router takes off. Each of the
//! [`PORT_KEYS`](super::layout::PORT_KEYS) presses its button, as
//! `_PortFilter` @0x4f209-0x4f257 does: R or F Recharge, C or T the Trade
//! Center, O the Outfitter, S the Shipyard, N the Mission BBS and B the
//! Bar. A letter does exactly what a click on its button does, so one for
//! a service the stellar doesn't offer does nothing, as `_DoPortDialog`'s
//! answers gate only on the stellar's flags (@0x5fd6c-0x60115). The keys
//! are a press, never a repeat (@0x4f15b), make no sound, and work with
//! modifiers held, as the view never reads them; they reach the spaceport
//! only while nothing is open over it. A service's
//! button opens its [`ServiceScreen`], which takes every input until it
//! closes. The Trade Center's opens the stellar's exchange instead, a
//! [`TradeScreen`] over it, when the spaceport is given one
//! ([`SpaceportView::with_trade`]): its orders are taken through the
//! spaceport ([`SpaceportView::take_trade`]), and the exchange after each
//! trade given back ([`SpaceportView::set_market`]). The Outfitter's opens
//! the stellar's outfitter likewise, an [`OutfitterScreen`] over it, when
//! given one ([`SpaceportView::with_outfitter`],
//! [`SpaceportView::take_outfit`], [`SpaceportView::set_outfitter`],
//! [`SpaceportView::outfit_ordered`]), and
//! the Shipyard's the stellar's shipyard, a [`ShipyardScreen`]
//! ([`SpaceportView::with_shipyard`], [`SpaceportView::set_shipyard`]):
//! its Buy Ship asks for a ship ([`SpaceportView::take_ship_request`]),
//! whoever flies the ship answers with its name prompt
//! ([`SpaceportView::open_ship_naming`]), and the ship named at the
//! prompt is taken through the spaceport ([`SpaceportView::take_ship`]),
//! or the ship declined there ([`SpaceportView::take_declined_ship`]). The Bar's opens the stellar's bar
//! likewise, a [`BarScreen`] over it, when given one
//! ([`SpaceportView::with_bar`]): its Hire Escort asks for the ships for
//! hire ([`SpaceportView::take_hire_request`]), whoever flies the ship
//! answers with them ([`SpaceportView::open_hire`]), the ship the hire
//! screen asks for is taken through the spaceport
//! ([`SpaceportView::take_hire`]), and after the hire the hire screen
//! closes back to the bar ([`SpaceportView::set_hire`]). As the bar
//! opens it says so ([`SpaceportView::take_bar_opened`]), so whoever
//! flies the ship draws the visit's news; its Holovid (News) asks for it
//! ([`SpaceportView::take_news_request`]), and the answer opens the news
//! over the bar ([`SpaceportView::open_news`]).
//!
//! Given the mission screens ([`SpaceportView::with_missions`]), Mission
//! BBS asks for the mission computer
//! ([`SpaceportView::take_mission_computer_request`]); whoever flies the
//! ship answers ([`SpaceportView::open_mission_computer`]), opening a
//! [`MissionComputerScreen`] over it, or showing why not. Its accept is
//! taken through the spaceport ([`SpaceportView::take_mission_accept`])
//! and the computer handed back ([`SpaceportView::set_mission_computer`]).
//! Missions are offered at five venues, each asking for an offer through
//! the spaceport, named by its `AvailLoc`
//! ([`SpaceportView::take_offer_request`]); whoever flies the ship answers
//! with one, opening the offer dialog over the venue
//! ([`SpaceportView::open_offer`]), or rearms it
//! ([`SpaceportView::rearm_offers`]), and the choice is taken through the
//! spaceport ([`SpaceportView::take_offer_choice`]). The spaceport asks
//! once, as it is landed on, before anything can open over it
//! (`_DoPortDialog` @0x5fc37): its offer shows over it, under the mission
//! info screen and the mission texts, and it never asks again, as its
//! timer's call (@0x60176) never runs. The trade center, outfitter and
//! shipyard ask as they open and again once rearmed, the bar a quarter
//! second after it opens and again once rearmed; the spaceport ticks the
//! screen open for their timers. While a mission text, the mission info
//! screen or the player info screen covers it, only the venue's offer
//! timer ticks, as covered, which runs on or waits as
//! [`RuleKey::OfferClock`](nova_sim::RuleKey::OfferClock) reads it
//! ([`SpaceportView::with_offer_clock`]); the bar's hire screen restarts
//! the bar's by the engine, which asks for a new gap
//! ([`SpaceportView::take_offer_restart`]).
//! [`MISSION_INFO_KEY`](crate::ui::mission_info::MISSION_INFO_KEY), in the spaceport, the bar, the mission
//! computer, the trade center, the outfitter, the shipyard (and its Info
//! panel) or an offer over any of them, asks for the mission info screen
//! ([`SpaceportView::take_mission_info_request`]), which opens over
//! whatever is open ([`SpaceportView::open_mission_info`]); its abort is
//! taken through the spaceport ([`SpaceportView::take_mission_abort`]).
//! The mission texts show one after another in "Desc Dialog" over
//! everything ([`SpaceportView::show_mission_texts`]), each taking every
//! input until Done closes it. The texts and the mission info screen are
//! the [`MissionDialogs`] flight shows too; the spaceport lets go of the
//! keys of the screen beneath once the info screen closes.
//!
//! [`PLAYER_INFO_KEY`] (a press) asks for the player info screen
//! ([`SpaceportView::take_player_info_request`]) from the spaceport
//! (`_PortFilter` @0x4f18f), the bar and its hire screen, the trade
//! center, the shipyard and its Info panel (never its name prompt), the
//! mission computer, and every offer; the outfitter, the news, the
//! mission info screen and the mission texts ignore it. Whoever flies the
//! ship answers ([`SpaceportView::open_player_info`]), opening a
//! [`PlayerInfoScreen`] laid out by the dialogs the spaceport was given
//! ([`SpaceportView::with_player_info`]) over whatever is open, which
//! pauses beneath it. Its jettison is taken through the spaceport
//! ([`SpaceportView::take_player_info_jettison`]); once it closes, the
//! screen beneath lets go of its keys.
//!
//! Where the stellar sells fuel, Recharge (item 4) asks for a refill,
//! taken through the spaceport ([`SpaceportView::take_recharge`]). Its
//! button is enabled as whoever flies the ship says
//! ([`SpaceportView::set_recharge_enabled`]), while R and F ask whatever
//! the button shows, as `_PortFilter` skips `_TrackPlanetButtons`. Nothing
//! is said of the refill, as `_DoPortDialog` says nothing (@0x5fd5d-0x5fe25).
//!
//! The stellar's ambient sound (its `CustSndID`, 128 and up) plays while
//! nothing covers the spaceport, as `_PortFilter` @0x4f041 plays it, and
//! once more for a press on the landscape ([`ambient`](super::ambient)):
//! its waits are drawn on the chance given
//! ([`SpaceportView::with_chance`]). Nothing open over it, a mission text,
//! the mission info or player info screen, or the spaceport's own offer,
//! asked for and not yet served, covers it; whoever shows something else
//! over it says so ([`SpaceportView::cover`]). Whatever closes it takes
//! its last sounds with the ambient sound's halt
//! ([`SpaceportView::close_sounds`]).
//!
//! Without the dialog, or the stellar's record, the screen says why, and
//! Return or Escape still leaves.
//!
//! The port dialogs, the bar's hire screen among them, say as they open
//! and close ([`SpaceportView::take_port_dialog_opened`],
//! [`SpaceportView::take_port_dialog_closed`]), and the other closings
//! the original ends in `_RefreshSAT` say so too
//! ([`SpaceportView::take_status_refresh`]): whoever flies the ship
//! redraws the status bar at each. The map key asks for the course map
//! ([`SpaceportView::take_map_request`], a [`PortMap`]) from every port
//! screen: the spaceport, the exchange, the outfitter, the shipyard, the
//! bar and its hire screen, the ship description over either, an offer
//! over any venue, the mission computer, mission info (not when opened
//! over an offer or the computer) and the accept's briefing text. Its
//! close redraws the whole bar and puts back the selection the map
//! opened with, then the nav the screen or the mission dialog saved, as
//! each does (flight's `FlightView::close_map`). An
//! outfitter, shipyard or hire screen with
//! nothing listed never opens: it says so in "Desc Dialog" (`STR#` 2002
//! #206, #223, #224) and returns before its nav save or its close
//! (@0x5bbf7 → @0x5dbd6, @0x5e8df → @0x5f906), so nothing redraws or is
//! restored ([`SpaceportView::set_opened_outfitter`],
//! [`SpaceportView::open_hire`]). The closings the original redraws at
//! that ours can't reach:
//!
//! - The bar's Gamble text (`_ShowTextDialog` @0x4918c, then
//!   `_RefreshSAT` @0x49191): ours' Gamble isn't built and shows greyed.
//! - Mission info's Abort with nothing selected (@0x9e51d-0x9e557): ours
//!   greys Abort unless the selected mission can be aborted.
//! - A screen whose dialog is missing: the original never shows the bar,
//!   News, player info, mission info or the computer without its `DLOG`
//!   (@0x4900d, @0x47fcb, @0x5a0a5, @0x9e20b, @0xa39a9), so none redraws.
//!   Ours shows a screen saying why, whose close asks for no redraw; the
//!   computer's still closes as a port dialog.
//! - Preferences: the menu's `pref` command (@0xccc49-0xccc4e) calls
//!   `_DoPrefsDialog` alone (@0xcccf1), with no `_RefreshSAT`; only the
//!   `pref` Apple Event's handler redraws (@0x56195), and the menu never
//!   sends it.
//! - The shops' text dialogs: `_ShowTextDialog`'s `_RefreshSAT` tails run
//!   only on a faded screen (`_screenFaded` @0x5a7c7, @0x5af9e, @0x5b45c),
//!   around a `dësc` movie ours doesn't play.
//! - `_MyModalDialog`'s update event (@0x59b17-0x59be6), an OS repaint:
//!   ours draws every frame whole.

use std::rc::Rc;
use std::time::Duration;

use nova_sim::hire::NONE_FOR_HIRE;
use nova_sim::mission::screens::{BAR, OUTFITTER, SHIPYARD, SPACEPORT, TRADE_CENTER};
use nova_sim::{
    ComputerRefusal, ComputerRow, DescMovie, HireList, Market, MissionComputer, MissionId,
    MissionInfo, MissionShown, News, OfferPrompt, Order, OutfitOrder, Outfitter, PlayerInfo,
    Reselection, RuleSource, SaleAlert, Service, ShipId, ShipNaming, Shipyard, sells_fuel,
    services,
};

use super::ambient::Ambient;
use super::bar::{BarScreen, BarTemplates, Hiring};
use super::buttons;
use super::catalog::{SpaceportCatalog, StellarId};
use super::hire::HireScreen;
use super::layout::{
    BACKGROUND, LANDSCAPE_ITEM, LEAVE_ITEM, NAME_FONT, NAME_ITEM, NAME_SIZE, RECHARGE_ITEM,
    TEXT_ITEM, item_service, landscape_id, port_key_item,
};
use super::missions::{MissionComputerScreen, MissionInfoScreen, OfferNames};
use super::offer::{OfferChoice, OfferDialog, OfferTemplates, VenueOffers};
use super::outfitter::{NO_ITEMS, OutfitterCatalog, OutfitterScreen};
use super::port_map::PortMap;
use super::service::ServiceScreen;
use super::shipyard::{InfoTemplates, NO_SHIPS, ShipOrder, ShipyardCatalog, ShipyardScreen};
use super::trade::{ListColors, TradeScreen};
use crate::color::Color;
use crate::draw::DrawList;
use crate::flight::view::{SharedChance, is_map_press};
use crate::flight::{MapOpener, PortScreen};
use crate::font::Font;
use crate::geometry::Point;
use crate::image::ImageKey;
use crate::input::{Input, Key};
use crate::movie::Movies;
use crate::screen::{Screen, ScreenAction};
use crate::sound::Sound;
use crate::text::TextMetrics;
use crate::ui::button::{ButtonSkin, ButtonStyle};
use crate::ui::catalog::DescriptionSource;
use crate::ui::desc::{BODY_COLOR, BODY_SIZE, DescDialog, DescTemplates};
use crate::ui::dialog::{Dialog, DialogEvent, DialogTemplate, FocusRing, Role, TextBacking};
use crate::ui::mission_dialogs::{DescShown, MissionDialogs};
use crate::ui::mission_info::is_mission_info_press;
#[cfg(doc)]
use crate::ui::player_info::PLAYER_INFO_KEY;
use crate::ui::player_info::{BackgroundSizes, PlayerInfoScreen, is_player_info_press};

/// Where the reason the spaceport cannot be shown goes, its size and
/// colour.
pub const PROBLEM_AT: Point = Point::new(16.0, 32.0);
/// The reason's size.
pub const PROBLEM_SIZE: f32 = 20.0;

/// The spaceport, laid out.
#[derive(Clone, Debug)]
struct Port {
    dialog: Dialog,
    name: String,
    /// The landscape `PICT`, or the missing one's ID.
    landscape: Result<i16, i16>,
    offered: Vec<Service>,
    /// Whether the stellar sells fuel, so Recharge can be enabled.
    sells_fuel: bool,
    /// Why there is no description, if there is none.
    description_problem: Option<String>,
    style: ButtonStyle,
    metrics: MetricsHandle,
    /// Whether its dialogs' buttons take the keyboard focus, and each
    /// screen's opened over it.
    focus_ring: FocusRing,
}

/// The metrics, shared, with a `Debug` that shows nothing of them.
#[derive(Clone)]
struct MetricsHandle(Rc<dyn TextMetrics>);

impl std::fmt::Debug for MetricsHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("TextMetrics")
    }
}

/// One of the port dialogs that save the player's nav as they open and
/// restore it as they close (`_DoPortDialog`; see
/// [`SpaceportView::take_port_dialog_opened`] and
/// [`SpaceportView::take_port_dialog_closed`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PortDialog {
    /// The outfitter.
    Outfitter,
    /// The exchange.
    Exchange,
    /// The shipyard.
    Shipyard,
    /// The mission computer.
    MissionComputer,
    /// The bar's hire screen: `_DoShipyardDialog` in hire mode
    /// (`_shipBuyMode` = 1, @0x49226-0x49230), which saves the nav as it
    /// opens (@0x5e70f-0x5e71e), and as it closes restores it
    /// (@0x5f8d9-0x5f8f5) and marks it (`_statusRedraw[0]` @0x5f8fe), as
    /// the shipyard does. With nothing for hire it returns before the
    /// save (@0x5e8da → @0x5f906), so it is no port dialog then.
    Hire,
}

/// What is open over the spaceport whose close redraws the status bar
/// ([`SpaceportView::take_status_refresh`]) or is the hire screen's
/// ([`PortDialog::Hire`]), taken before an input and compared after
/// ([`SpaceportView::note_closings`]).
#[derive(Clone, Copy, Debug, Default)]
struct Redrawing {
    /// The bar's News, laid out.
    news: bool,
    /// A ship description panel: the shipyard's or the hire screen's.
    panel: bool,
    /// The bar's hire screen.
    hire: bool,
}

/// A screen open over the spaceport.
#[derive(Clone, Debug)]
enum Open {
    /// A service's placeholder.
    Service(ServiceScreen),
    /// The exchange.
    Trade(Box<TradeScreen>),
    /// The outfitter.
    Outfitter(Box<OutfitterScreen>),
    /// The shipyard.
    Shipyard(Box<ShipyardScreen>),
    /// The bar, and the hire screen over it.
    Bar(Box<BarScreen>),
    /// The mission computer.
    Computer(Box<MissionComputerScreen>),
}

impl Open {
    fn screen_mut(&mut self) -> &mut dyn Screen {
        match self {
            Self::Service(screen) => screen,
            Self::Trade(screen) => screen.as_mut(),
            Self::Outfitter(screen) => screen.as_mut(),
            Self::Shipyard(screen) => screen.as_mut(),
            Self::Bar(screen) => screen.as_mut(),
            Self::Computer(screen) => screen.as_mut(),
        }
    }

    /// Ticks only the open venue's offer timer, as covered.
    fn tick_covered(&mut self, dt: Duration) {
        match self {
            Self::Trade(screen) => screen.tick_covered(dt),
            Self::Outfitter(screen) => screen.tick_covered(dt),
            Self::Shipyard(screen) => screen.tick_covered(dt),
            Self::Bar(screen) => screen.tick_covered(dt),
            Self::Service(_) | Self::Computer(_) => {}
        }
    }

    /// The port dialog it is, if it is one: the bar and a placeholder are
    /// not.
    fn port_dialog(&self) -> Option<PortDialog> {
        match self {
            Self::Trade(_) => Some(PortDialog::Exchange),
            Self::Outfitter(_) => Some(PortDialog::Outfitter),
            Self::Shipyard(_) => Some(PortDialog::Shipyard),
            Self::Computer(_) => Some(PortDialog::MissionComputer),
            Self::Service(_) | Self::Bar(_) => None,
        }
    }

    fn closed(&self) -> bool {
        match self {
            Self::Service(screen) => screen.closed(),
            Self::Trade(screen) => screen.closed(),
            Self::Outfitter(screen) => screen.closed(),
            Self::Shipyard(screen) => screen.closed(),
            Self::Bar(screen) => screen.closed(),
            Self::Computer(screen) => screen.closed(),
        }
    }
}

/// The outfitter given: its dialog template, the quantity dialog's and
/// the alert's (or why there are none), the outfitter as it is, and where
/// its pictures and descriptions come from.
#[derive(Clone)]
struct Outfitting {
    template: Result<DialogTemplate, String>,
    quantity: Result<DialogTemplate, String>,
    alert: Result<DialogTemplate, String>,
    outfitter: Outfitter,
    catalog: Rc<dyn OutfitterCatalog>,
}

impl std::fmt::Debug for Outfitting {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Outfitting")
            .field("template", &self.template)
            .field("quantity", &self.quantity)
            .field("alert", &self.alert)
            .field("outfitter", &self.outfitter)
            .finish_non_exhaustive()
    }
}

/// The shipyard given: its dialog templates (or why there are none), the
/// shipyard as it is, and where its pictures and descriptions come from.
#[derive(Clone)]
struct Shipbuying {
    template: Result<DialogTemplate, String>,
    info: InfoTemplates,
    text_input: Result<DialogTemplate, String>,
    shipyard: Shipyard,
    catalog: Rc<dyn ShipyardCatalog>,
}

impl std::fmt::Debug for Shipbuying {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Shipbuying")
            .field("template", &self.template)
            .field("info", &self.info)
            .field("text_input", &self.text_input)
            .field("shipyard", &self.shipyard)
            .finish_non_exhaustive()
    }
}

/// The bar given: its dialog template and its news dialog's (or why
/// there are none), what its hire screen is built from, and whether the
/// fleet has room.
#[derive(Clone, Debug)]
struct Barkeeping {
    templates: BarTemplates,
    /// The news dialog's template, or why there is none.
    news: Result<DialogTemplate, String>,
    hiring: Hiring,
    room: bool,
    /// Whether the bar has opened since this was last taken.
    opened: bool,
}

/// The mission screens given: their dialog templates, or why there are
/// none.
#[derive(Clone, Debug)]
struct Missioning {
    /// The mission computer's.
    computer: Result<DialogTemplate, String>,
    /// The offer dialog's, both variants, which every venue's offers show
    /// in.
    offer: OfferTemplates,
    /// The spaceport's own offer, asked for once, as it is landed on.
    landing: VenueOffers,
    /// Whether Mission BBS has asked for the mission computer since this
    /// was last taken.
    requested: bool,
    /// Whether [`MISSION_INFO_KEY`](crate::ui::mission_info::MISSION_INFO_KEY) has asked for the mission info screen
    /// since this was last taken.
    info_requested: bool,
}

/// What the player info screen is laid out from
/// ([`SpaceportView::with_player_info`]).
#[derive(Clone, Debug)]
struct PlayerInfoDialogs {
    /// `DLOG` 1017, or why there is none.
    template: Result<DialogTemplate, String>,
    /// The Yes/No dialog, `DLOG` 3002, or why there is none.
    confirm: Result<DialogTemplate, String>,
    /// The background pictures' sizes.
    background: BackgroundSizes,
}

/// The spaceport of the stellar landed on.
#[derive(Clone, Debug)]
#[allow(clippy::struct_excessive_bools)] // each is one request, taken on its own
pub struct SpaceportView {
    stellar: StellarId,
    /// The spaceport, or why it cannot be shown.
    port: Result<Port, String>,
    /// The screen open over it, if any.
    open: Option<Open>,
    /// The exchange's dialog template and its quantity dialog's (or why
    /// there are none), the exchange as it is and its list's colours,
    /// once given.
    #[allow(clippy::type_complexity)] // Two templates, the exchange and its colours.
    trade: Option<(
        Result<DialogTemplate, String>,
        Result<DialogTemplate, String>,
        Market,
        ListColors,
    )>,
    /// The outfitter, once given.
    outfitting: Option<Outfitting>,
    /// The shipyard, once given.
    shipbuying: Option<Shipbuying>,
    /// The bar, once given.
    barkeeping: Option<Barkeeping>,
    /// The mission screens, once given.
    missioning: Option<Missioning>,
    /// The offer whose `dësc` the mission info screen opened over its
    /// dialog asks to reload as it closes, until taken
    /// ([`SpaceportView::take_offer_reload`]).
    offer_reload: Option<MissionId>,
    /// How the venues' offer timers run
    /// ([`RuleKey::OfferClock`](nova_sim::RuleKey::OfferClock)).
    offer_clock: RuleSource,
    /// The movies the mission texts and offers play, and whether they play.
    movies: Movies,
    /// The mission texts, the first shown over everything, and the
    /// mission info screen under them, over whatever else is open: given
    /// with the mission screens, when the spaceport has its dialog.
    dialogs: Option<MissionDialogs>,
    /// How many mission texts were dropped, with no dialogs to show them,
    /// since the texts closed were last taken
    /// ([`SpaceportView::take_texts_closed`]).
    texts_dropped: usize,
    /// The player info screen's dialog, its Yes/No dialog and its
    /// background pictures' sizes, once given.
    player_info_dialogs: Option<PlayerInfoDialogs>,
    /// Whether [`PLAYER_INFO_KEY`] asked for the player info screen in the
    /// spaceport itself.
    player_info_requested: bool,
    /// Whether the map key asked for the course map in the spaceport
    /// itself (`_PortFilter` item 3).
    port_map_requested: bool,
    /// The player info screen, over everything else.
    player_info: Option<PlayerInfoScreen>,
    /// Whether its Jettison was confirmed since this was last taken.
    player_info_jettison: bool,
    left: bool,
    /// Whether Recharge has been clicked since this was last taken.
    recharge: bool,
    /// Whether the outfitter has opened since this was last taken.
    outfitter_opened: bool,
    /// Whether the outfitter open has yet to be shown its listing
    /// ([`SpaceportView::set_opened_outfitter`]): until then it asks for
    /// no offer, as `_DoOutfitDialog` builds its list before it offers
    /// anything, and with nothing listed never offers.
    outfitter_unlisted: bool,
    /// The port dialog opened since this was last taken.
    port_dialog_opened: Option<PortDialog>,
    /// The port dialog closed since this was last taken.
    port_dialog_closed: Option<PortDialog>,
    /// Whether the outfitter has closed since this was last taken.
    outfitter_closed: bool,
    /// Whether the spaceport's own `dësc` waits to be loaded.
    description_request: bool,
    /// Whether the outfitter just opened listed nothing, and so never
    /// opened, since this was last taken.
    empty_outfitter: bool,
    /// Whether a closing that redraws the status bar has come since this
    /// was last taken ([`SpaceportView::take_status_refresh`]).
    status_refresh: bool,
    /// The sounds made since they were last taken, kept here so a service
    /// that closes keeps its sounds.
    sounds: Vec<Sound>,
    /// The stellar's ambient sound, when it has one and the spaceport can
    /// be shown.
    ambient: Option<Ambient>,
    /// What the ambient sound's waits are drawn on.
    chance: SharedChance,
}

impl SpaceportView {
    /// The spaceport of `stellar`, read from `catalog` once and laid out by
    /// `layout`: the "Spaceport" dialog's template and the metrics its text
    /// is measured by, or why there are none.
    pub fn new(
        catalog: &(impl SpaceportCatalog + DescriptionSource),
        stellar: StellarId,
        layout: Result<(DialogTemplate, Rc<dyn TextMetrics>), String>,
    ) -> Self {
        let mut ambient = None;
        let port = layout.and_then(|(template, metrics)| {
            let record = catalog.stellar_port(stellar)?;
            ambient = Ambient::of(record.cust_snd_id, record.flags2);
            let offered = services(record.flags);
            let sells_fuel = sells_fuel(record.flags);
            let (text, description_problem) = match catalog.description(stellar.0) {
                Ok(text) => (text, None),
                Err(reason) => (String::new(), Some(reason)),
            };
            let mut roles = buttons::roles();
            roles.push((
                TEXT_ITEM,
                // White on a black box: `_PortDialogUpdate` sets it with
                // `TETextBox` (@0x4ef64) and inverts the item
                // (`InvertRect` @0x4ef72) when it is visible
                // (`RectInRgn` @0x4ef08), which `Dialog` models as the
                // item being inside the dialog.
                Role::ScrollText {
                    text,
                    font: Font::Geneva,
                    size: BODY_SIZE,
                    color: BODY_COLOR,
                    backing: Some(TextBacking::BLACK_BOX),
                },
            ));
            let style = catalog.button_style();
            let mut dialog = Dialog::new(&template, &roles, Rc::clone(&metrics))
                .with_buttons(ButtonSkin::NOVA, style)
                .with_default(Some(LEAVE_ITEM))
                .with_cancel(Some(LEAVE_ITEM));
            for item in buttons::greyed(&offered, sells_fuel) {
                dialog.set_greyed(item, true);
            }
            let landscape = landscape_id(record.cust_pic_id, record.graphic_type);
            Ok(Port {
                dialog,
                name: record.name,
                landscape: if catalog.picture_exists(landscape) {
                    Ok(landscape)
                } else {
                    Err(landscape)
                },
                offered,
                sells_fuel,
                description_problem,
                style,
                metrics: MetricsHandle(metrics),
                focus_ring: FocusRing::default(),
            })
        });
        let description_request = port
            .as_ref()
            .is_ok_and(|port| port.description_problem.is_none());
        Self {
            stellar,
            port,
            open: None,
            trade: None,
            outfitting: None,
            shipbuying: None,
            barkeeping: None,
            missioning: None,
            offer_reload: None,
            offer_clock: RuleSource::Engine,
            movies: Movies::none(),
            dialogs: None,
            texts_dropped: 0,
            player_info_dialogs: None,
            player_info_requested: false,
            port_map_requested: false,
            player_info: None,
            player_info_jettison: false,
            left: false,
            recharge: false,
            outfitter_opened: false,
            outfitter_unlisted: false,
            empty_outfitter: false,
            port_dialog_opened: None,
            port_dialog_closed: None,
            outfitter_closed: false,
            description_request,
            status_refresh: false,
            sounds: Vec::new(),
            ambient,
            chance: SharedChance::default(),
        }
    }

    /// The spaceport with its ambient sound's waits drawn on `chance`.
    /// Without one, each draw is the last outcome.
    #[must_use]
    pub fn with_chance(self, chance: SharedChance) -> Self {
        Self { chance, ..self }
    }

    /// Something that doesn't tick the spaceport covered it, such as the
    /// Preferences dialog: its next frontmost frame counts as the port
    /// coming back to the front, which rearms the ambient sound's wait
    /// (see [`ambient`](super::ambient)).
    pub fn cover(&mut self) {
        if let Some(ambient) = &mut self.ambient {
            ambient.cover();
        }
    }

    /// The spaceport closes, whatever closes it: its sounds not yet taken,
    /// then its ambient sound's halt, as `_DoPortDialog` halts `_planetSnd`
    /// as the port closes (@0x601d2-0x601eb).
    pub fn close_sounds(&mut self) -> Vec<Sound> {
        let mut sounds = std::mem::take(&mut self.sounds);
        sounds.extend(
            self.ambient
                .as_ref()
                .map(|ambient| Sound::Ui(ambient.halted())),
        );
        sounds
    }

    /// Whether something open over the spaceport covers it, so its own
    /// filter doesn't run: a service or shop, a mission text, the mission
    /// info or player info screen, or its own offer, asked for and not yet
    /// served, or open.
    fn covered(&self) -> bool {
        self.open.is_some()
            || self.mission_dialogs_open()
            || self.player_info.is_some()
            || self
                .missioning
                .as_ref()
                .is_some_and(|missioning| missioning.landing.pending())
    }

    /// The port dialog that has opened since this was last asked, once
    /// each time one opens: the exchange, the outfitter or the shipyard
    /// as its button opens it, the bar's hire screen as it opens over ships
    /// for hire ([`SpaceportView::open_hire`]), and the mission computer
    /// as it is answered
    /// ([`SpaceportView::open_mission_computer`]), refused too, as the
    /// original saves before every `_DoMissionDialog` (@0x5ffc0). Whoever
    /// flies the ship saves the nav, as each of the original's dialogs
    /// saves +0x6a and +0x6c (the outfitter @0x5bafc-0x5bb12, the
    /// exchange @0x5dbed-0x5dc08, the shipyard and the hire screen
    /// @0x5e70f-0x5e71e, the computer @0x5ffc0-0x5ffd0).
    pub fn take_port_dialog_opened(&mut self) -> Option<PortDialog> {
        self.port_dialog_opened.take()
    }

    /// The port dialog that has closed since this was last asked, once
    /// each time one closes: whoever flies the ship redraws what its close
    /// draws and restores the nav saved as it opened. The exchange,
    /// outfitter, shipyard and hire screen end in `_RefreshSAT` (the
    /// outfitter @0x5dba4, the exchange @0x5e642, the shipyard and the
    /// hire screen @0x5f8d4), then restore (@0x5dbb8, @0x5e64d, @0x5f8e8);
    /// the hire screen closes by Cancel or after a hire
    /// ([`SpaceportView::set_hire`]); the computer ends in `_RefreshSAT`
    /// (@0xa40d7) and restores after its loop (@0x60084-0x60096). While mission texts are queued
    /// it says nothing and keeps the close: the original's dialogs close
    /// only after their modal texts, an accept's `BriefText` before the
    /// re-entry finds nothing (`_DoMissionAccept` inside
    /// `_DoMissionDialog`) and the refusal's text before the restore
    /// (`_ShowTextDialog` @0xa3956).
    pub fn take_port_dialog_closed(&mut self) -> Option<PortDialog> {
        if self.texts_queued() {
            None
        } else {
            self.port_dialog_closed.take()
        }
    }

    /// The `dësc` a screen of the spaceport has shown since this was last
    /// taken, once, read raw until whoever flies the ship loads it as
    /// `_LoadDescription` does and answers with
    /// [`SpaceportView::show_description`], in the same input: the
    /// spaceport's own as it opens (`_DoPortDialog` @0x5f96b), then the
    /// open outfitter's, shipyard's, bar's or hire screen's.
    pub fn take_description_request(&mut self) -> Option<i16> {
        if std::mem::take(&mut self.description_request) {
            return Some(self.stellar.0);
        }
        match &mut self.open {
            Some(Open::Outfitter(screen)) => screen.take_description_request(),
            Some(Open::Shipyard(screen)) => screen.take_description_request(),
            Some(Open::Bar(screen)) => screen.take_description_request(),
            _ => None,
        }
    }

    /// Shows `text`, `dësc` `id` loaded, where it is shown: as the
    /// spaceport's description when `id` is the stellar's, and otherwise
    /// in the screen open.
    pub fn show_description(&mut self, id: i16, text: &str) {
        if id == self.stellar.0 {
            if let Ok(port) = &mut self.port {
                port.dialog.set_text(TEXT_ITEM, text);
            }
            return;
        }
        match &mut self.open {
            Some(Open::Outfitter(screen)) => screen.show_description(id, text),
            Some(Open::Shipyard(screen)) => screen.show_description(id, text),
            Some(Open::Bar(screen)) => screen.show_description(id, text),
            _ => {}
        }
    }

    /// Whether a closing that redraws the whole status bar has come since
    /// this was last asked, once each time: whoever flies the ship
    /// redraws the bar ([`FlightView::refresh_status`](crate::flight::FlightView::refresh_status)), as each of the
    /// original's ends in `_RefreshSAT`:
    ///
    /// - the bar's close (@0x4945e);
    /// - News closing back to the bar (@0x48153);
    /// - the ship description panel closing, the shipyard's or the hire
    ///   screen's (@0x5bac2);
    /// - the player info screen closing (@0x5a725);
    /// - the mission info screen closing, by its input or by an abort
    ///   leaving nothing (@0x9e769).
    ///
    /// A screen shown without its dialog closes with none, as without its
    /// `DLOG` the original never shows the bar (@0x4900d), News
    /// (@0x47fcb), player info (@0x5a0a5) or mission info (@0x9e20b).
    /// The port dialogs' closes, the hire screen's included, redraw
    /// through [`SpaceportView::take_port_dialog_closed`] instead. While
    /// mission texts are queued it says nothing and keeps the closing, as
    /// [`SpaceportView::take_port_dialog_closed`] does.
    pub fn take_status_refresh(&mut self) -> bool {
        !self.texts_queued() && std::mem::take(&mut self.status_refresh)
    }

    /// Whether Recharge has been clicked, or R or F pressed, since this
    /// was last asked: whoever flies the ship refills it, then tells the
    /// spaceport whether Recharge is still enabled
    /// ([`SpaceportView::set_recharge_enabled`]).
    pub fn take_recharge(&mut self) -> bool {
        std::mem::take(&mut self.recharge)
    }

    /// Enables or disables the Recharge button, as whoever flies the ship
    /// decides from the cash and the tank (`_DrawPlanetButtons`
    /// @0x297a3-0x297f4; `nova_sim::Session::recharge_enabled`). Until
    /// told it is enabled where fuel is sold; where none is, it stays
    /// disabled whatever it is told. A disabled Recharge takes no click,
    /// as `_TrackPlanetButtons` takes none (@0x29ae2-0x29b28), but R and F
    /// still ask. Whoever flies the ship tells it as the spaceport is
    /// shown and after each input, before the next frame is drawn.
    pub fn set_recharge_enabled(&mut self, enabled: bool) {
        if let Ok(port) = &mut self.port {
            port.dialog
                .set_greyed(RECHARGE_ITEM, !(enabled && port.sells_fuel));
        }
    }

    /// The spaceport with the stellar's exchange, `market`, which the
    /// Trade Center opens laid out by `template`, the "Trade" dialog (or
    /// saying why there is none), its quantity dialog by `quantity` (or,
    /// without it, the dialog's fallback), its list in `colors`. Without
    /// it, the Trade Center opens its placeholder.
    #[must_use]
    pub fn with_trade(
        self,
        template: Result<DialogTemplate, String>,
        quantity: Result<DialogTemplate, String>,
        market: Market,
        colors: ListColors,
    ) -> Self {
        Self {
            trade: Some((template, quantity, market, colors)),
            ..self
        }
    }

    /// The exchange open, if it is.
    #[must_use]
    pub fn open_trade(&self) -> Option<&TradeScreen> {
        match &self.open {
            Some(Open::Trade(screen)) => Some(screen),
            _ => None,
        }
    }

    /// The order the exchange open asked for since it was last taken,
    /// once.
    pub fn take_trade(&mut self) -> Option<Order> {
        match &mut self.open {
            Some(Open::Trade(screen)) => screen.take_order(),
            _ => None,
        }
    }

    /// Shows `market`, the exchange after a trade: the exchange open shows
    /// it, and so does the exchange opened next.
    pub fn set_market(&mut self, market: Market) {
        if let Some(Open::Trade(screen)) = &mut self.open {
            screen.set_market(market.clone());
        }
        if let Some((_, _, kept, _)) = &mut self.trade {
            *kept = market;
        }
    }

    /// The spaceport with the stellar's outfitter, `outfitter`, which the
    /// Outfitter opens laid out by `template`, the "Outfit" dialog (or
    /// saying why there is none), its quantity dialog laid out by
    /// `quantity` (or its fallback), a sale's alert by `alert` ("Desc
    /// Dialog", `DLOG` 3003, or none), each outfit's picture and
    /// description read from `catalog`. Without it, the Outfitter opens
    /// its placeholder.
    #[must_use]
    pub fn with_outfitter(
        self,
        template: Result<DialogTemplate, String>,
        quantity: Result<DialogTemplate, String>,
        alert: Result<DialogTemplate, String>,
        outfitter: Outfitter,
        catalog: Rc<dyn OutfitterCatalog>,
    ) -> Self {
        Self {
            outfitting: Some(Outfitting {
                template,
                quantity,
                alert,
                outfitter,
                catalog,
            }),
            ..self
        }
    }

    /// The outfitter open, if it is.
    #[must_use]
    pub fn open_outfitter(&self) -> Option<&OutfitterScreen> {
        match &self.open {
            Some(Open::Outfitter(screen)) => Some(screen),
            _ => None,
        }
    }

    /// The order the outfitter open asked for since it was last taken,
    /// and how many it moves, once.
    pub fn take_outfit(&mut self) -> Option<(OutfitOrder, u32)> {
        match &mut self.open {
            Some(Open::Outfitter(screen)) => screen.take_order(),
            _ => None,
        }
    }

    /// Opens `alert`'s words over the outfitter open
    /// ([`OutfitterScreen::alert`]); with none open, does nothing.
    pub fn alert_outfit(&mut self, alert: &SaleAlert) {
        if let Some(Open::Outfitter(screen)) = &mut self.open {
            screen.alert(alert);
        }
    }

    /// Follows what an order left the outfitter's selection at
    /// ([`OutfitterScreen::reselect`]), with the outfitter open; when
    /// that closes it (nothing listed, and no alert open), it closes at
    /// once, as the original leaves once the click is handled (@0x5da33).
    /// With an alert open, the input that closes the alert closes the
    /// outfitter.
    pub fn outfit_ordered(&mut self, after: Reselection) {
        if let Some(Open::Outfitter(screen)) = &mut self.open {
            screen.reselect(after);
            if screen.closed() {
                self.close_open();
            }
        }
    }

    /// Whether the outfitter open has closed since this was last asked,
    /// once each time it closes, however it closes: Done, an order that
    /// leaves nothing listed, or the input that closes that order's
    /// alert, every way out the original's one tail (@0x5da33). Whoever
    /// flies the ship tells the session, then shows the shops afresh.
    pub fn take_outfitter_closed(&mut self) -> bool {
        std::mem::take(&mut self.outfitter_closed)
    }

    /// Closes the screen open, noting a port dialog's close
    /// ([`SpaceportView::take_port_dialog_closed`]), the outfitter's
    /// ([`SpaceportView::take_outfitter_closed`]) and the bar's, laid
    /// out ([`SpaceportView::take_status_refresh`]).
    fn close_open(&mut self) {
        match &self.open {
            Some(Open::Outfitter(_)) => self.outfitter_closed = true,
            Some(Open::Bar(bar)) if bar.problem().is_none() => self.status_refresh = true,
            _ => {}
        }
        if let Some(dialog) = self.open.take().and_then(|open| open.port_dialog()) {
            self.port_dialog_closed = Some(dialog);
        }
    }

    /// Whether the outfitter given has opened since this was last asked,
    /// once each time it opens: whoever flies the ship clears what lasts
    /// only an opening, then shows the outfitter afresh with
    /// [`SpaceportView::set_outfitter`].
    pub fn take_outfitter_opened(&mut self) -> bool {
        std::mem::take(&mut self.outfitter_opened)
    }

    /// Shows `outfitter`, the outfitter as it opens
    /// ([`SpaceportView::take_outfitter_opened`]), as
    /// [`SpaceportView::set_outfitter`] does. With nothing listed, the
    /// outfitter never opens, as `_DoOutfitDialog` builds its list first
    /// and, with nothing in it, shows `STR#` 2002 #206 ([`NO_ITEMS`],
    /// @0x5bba0-0x5bbf2) in "Desc Dialog" and jumps to its very end
    /// (@0x5bbf7 → @0x5dbd6): the outfitter is dropped with no close, so
    /// no close tail runs (@0x5da33-0x5dbd1) and nothing redraws, the nav
    /// saved as it opened is never restored (nothing ran in between, so
    /// it changes nothing), and no offer is asked for: the outfitter asks
    /// for none until its listing is shown here, so its ask is dropped
    /// with it, whenever the offers are served. That is told once
    /// ([`SpaceportView::take_empty_outfitter`]).
    pub fn set_opened_outfitter(&mut self, outfitter: Outfitter) {
        self.outfitter_unlisted = false;
        let empty = outfitter.rows.is_empty();
        self.set_outfitter(outfitter);
        if empty && matches!(self.open, Some(Open::Outfitter(_))) {
            self.open = None;
            self.port_dialog_opened = None;
            self.show_text(NO_ITEMS);
            self.empty_outfitter = true;
        }
    }

    /// Whether the outfitter opened with nothing listed, and so never
    /// opened ([`SpaceportView::set_opened_outfitter`]), since this was
    /// last asked, once: the port's `_DrawStatus` after the outfitter
    /// returns still runs (`_DoPortDialog` @0x5ff09-0x5ff10).
    pub fn take_empty_outfitter(&mut self) -> bool {
        std::mem::take(&mut self.empty_outfitter)
    }

    /// Shows `outfitter`, the outfitter after an order: the outfitter open
    /// shows it, and so does the outfitter opened next.
    pub fn set_outfitter(&mut self, outfitter: Outfitter) {
        if let Some(Open::Outfitter(screen)) = &mut self.open {
            screen.set_outfitter(outfitter.clone());
        }
        if let Some(outfitting) = &mut self.outfitting {
            outfitting.outfitter = outfitter;
        }
    }

    /// The spaceport with the stellar's shipyard, `shipyard`, which the
    /// Shipyard opens laid out by `template`, the "Shipyard" dialog, with
    /// its info panel laid out by `info`, "Shipyard Info", and its name
    /// prompt by `text_input`, "Text Input" (or saying why there are
    /// none), each ship's picture and description read from `catalog`.
    /// Without it, the Shipyard opens its placeholder.
    #[must_use]
    pub fn with_shipyard(
        self,
        template: Result<DialogTemplate, String>,
        info: InfoTemplates,
        text_input: Result<DialogTemplate, String>,
        shipyard: Shipyard,
        catalog: Rc<dyn ShipyardCatalog>,
    ) -> Self {
        Self {
            shipbuying: Some(Shipbuying {
                template,
                info,
                text_input,
                shipyard,
                catalog,
            }),
            ..self
        }
    }

    /// The shipyard open, if it is.
    #[must_use]
    pub fn open_shipyard(&self) -> Option<&ShipyardScreen> {
        match &self.open {
            Some(Open::Shipyard(screen)) => Some(screen),
            _ => None,
        }
    }

    /// The ship the shipyard open requested with Buy Ship since it was
    /// last taken, once.
    pub fn take_ship_request(&mut self) -> Option<ShipId> {
        match &mut self.open {
            Some(Open::Shipyard(screen)) => screen.take_request(),
            _ => None,
        }
    }

    /// Opens `naming`'s name prompt over the shipyard open, if one is.
    pub fn open_ship_naming(&mut self, naming: &ShipNaming) {
        if let Some(Open::Shipyard(screen)) = &mut self.open {
            screen.open_naming(naming);
        }
    }

    /// The ship the shipyard open ordered at its name prompt, with its
    /// name, since it was last taken, once.
    pub fn take_ship(&mut self) -> Option<ShipOrder> {
        match &mut self.open {
            Some(Open::Shipyard(screen)) => screen.take_order(),
            _ => None,
        }
    }

    /// The ship declined at the shipyard open's name prompt since it was
    /// last taken, once.
    pub fn take_declined_ship(&mut self) -> Option<ShipId> {
        match &mut self.open {
            Some(Open::Shipyard(screen)) => screen.take_declined(),
            _ => None,
        }
    }

    /// Shows `shipyard`, the shipyard after a purchase: the shipyard open
    /// shows it, and so does the shipyard opened next.
    pub fn set_shipyard(&mut self, shipyard: Shipyard) {
        if let Some(Open::Shipyard(screen)) = &mut self.open {
            screen.set_shipyard(shipyard.clone());
        }
        if let Some(shipbuying) = &mut self.shipbuying {
            shipbuying.shipyard = shipyard;
        }
    }

    /// The spaceport with the stellar's bar, which the Bar opens laid out
    /// by `templates`, the bar dialog's variants (or saying why there is
    /// none), its
    /// text and the ships' pictures and descriptions read from `catalog`;
    /// its news is laid out by `news_template`, the news dialog, and its
    /// hire screen by `hire_template`, the "Shipyard" dialog, with its
    /// info panel by `info_template`, "Shipyard Info". Hire Escort is
    /// greyed unless the fleet has `room`. Without it, the Bar opens its
    /// placeholder.
    #[must_use]
    pub fn with_bar(
        self,
        templates: BarTemplates,
        news_template: Result<DialogTemplate, String>,
        hire_template: Result<DialogTemplate, String>,
        info_template: InfoTemplates,
        catalog: Rc<dyn ShipyardCatalog>,
        room: bool,
    ) -> Self {
        Self {
            barkeeping: Some(Barkeeping {
                templates,
                news: news_template,
                hiring: Hiring {
                    template: hire_template,
                    info: info_template,
                    catalog,
                },
                room,
                opened: false,
            }),
            ..self
        }
    }

    /// The bar open, if it is.
    #[must_use]
    pub fn open_bar(&self) -> Option<&BarScreen> {
        match &self.open {
            Some(Open::Bar(screen)) => Some(screen),
            _ => None,
        }
    }

    /// Whether the bar has opened since this was last asked: whoever flies
    /// the ship draws the visit's news then, before anything can show it.
    pub fn take_bar_opened(&mut self) -> bool {
        self.barkeeping
            .as_mut()
            .is_some_and(|barkeeping| std::mem::take(&mut barkeeping.opened))
    }

    /// Whether the bar's Holovid (News) has asked for the visit's news
    /// since this was last asked: whoever flies the ship answers with
    /// [`SpaceportView::open_news`].
    pub fn take_news_request(&mut self) -> bool {
        match &mut self.open {
            Some(Open::Bar(screen)) => screen.take_news_request(),
            _ => false,
        }
    }

    /// Opens the news dialog over the bar open on `news`, drawn over
    /// `picture` ([`BarScreen::open_news`]).
    pub fn open_news(&mut self, news: &News, picture: i16) {
        if let Some(Open::Bar(screen)) = &mut self.open {
            screen.open_news(news, picture);
            self.sounds.extend(screen.take_sounds());
        }
    }

    /// The course map asked for since this was last asked, once (see the
    /// [`port_map`](super::port_map) module): the spaceport's own, the
    /// exchange's, the outfitter's or the shipyard's (or its ship
    /// description's), the bar's or its hire screen's
    /// ([`BarScreen::take_map_request`]), or a mission dialog's, asked by
    /// the offer open over the venue open or the spaceport, the mission
    /// computer with its selected offer, or the mission texts and the
    /// mission info screen over them ([`MissionDialogs::take_map_request`]).
    /// Whoever flies the ship opens it over the spaceport.
    pub fn take_map_request(&mut self) -> Option<PortMap> {
        if std::mem::take(&mut self.port_map_requested) {
            return Some(PortMap::Screen(PortScreen::Spaceport));
        }
        let open = match &mut self.open {
            Some(Open::Bar(screen)) => {
                if let Some(map) = screen.take_map_request() {
                    return Some(map);
                }
                screen.take_offer_map_request()
            }
            Some(Open::Trade(screen)) => {
                if screen.take_map_request() {
                    return Some(PortMap::Screen(PortScreen::Exchange));
                }
                screen.take_offer_map_request()
            }
            Some(Open::Outfitter(screen)) => {
                if screen.take_map_request() {
                    return Some(PortMap::Screen(PortScreen::Outfitter));
                }
                screen.take_offer_map_request()
            }
            Some(Open::Shipyard(screen)) => {
                if let Some(asked) = screen.take_map_request() {
                    return Some(PortMap::Screen(asked));
                }
                screen.take_offer_map_request()
            }
            Some(Open::Computer(screen)) => {
                if let Some(opener) = screen.take_map_request() {
                    return Some(PortMap::Dialog(opener));
                }
                None
            }
            Some(Open::Service(_)) | None => None,
        };
        let landing = self
            .missioning
            .as_mut()
            .and_then(|missioning| missioning.landing.take_map_request());
        if let Some(mission) = open.or(landing) {
            return Some(PortMap::Dialog(MapOpener::Offer(mission)));
        }
        self.dialogs
            .as_mut()?
            .take_map_request()
            .map(PortMap::Dialog)
    }

    /// Whether the bar's Hire Escort has asked for the ships for hire
    /// since this was last asked: whoever flies the ship answers with
    /// [`SpaceportView::open_hire`].
    pub fn take_hire_request(&mut self) -> bool {
        match &mut self.open {
            Some(Open::Bar(screen)) => screen.take_hire_request(),
            _ => false,
        }
    }

    /// Opens the hire screen over the bar open on `list`. The hire screen
    /// opened is a port dialog ([`PortDialog::Hire`]). With nothing for
    /// hire nothing opens, and `STR#` 2002 #224 ([`NONE_FOR_HIRE`]) shows
    /// in "Desc Dialog", as the original's shipyard function in hire mode
    /// shows it by `_ShowTextDialog` and returns before its save
    /// (@0x5e8ab → @0x5e8da → @0x5f906).
    pub fn open_hire(&mut self, list: HireList) {
        let Some(Open::Bar(screen)) = &mut self.open else {
            return;
        };
        if list.rows.is_empty() {
            screen.open_hire(list);
            self.show_text(NONE_FOR_HIRE);
            return;
        }
        let before = screen.hire_screen().is_some();
        screen.open_hire(list);
        if !before && screen.hire_screen().is_some() {
            self.port_dialog_opened = Some(PortDialog::Hire);
        }
    }

    /// The ship the hire screen open asked for since it was last taken,
    /// once.
    pub fn take_hire(&mut self) -> Option<ShipId> {
        match &mut self.open {
            Some(Open::Bar(screen)) => screen.take_hire(),
            _ => None,
        }
    }

    /// Whether the bar's hire screen has closed since this was last
    /// asked ([`BarScreen::take_hire_closed`]), once: whoever flies the
    /// ship tells the session, after any hire it made.
    pub fn take_hire_closed(&mut self) -> bool {
        match &mut self.open {
            Some(Open::Bar(screen)) => screen.take_hire_closed(),
            _ => false,
        }
    }

    /// After a hire, with `list` the ships for hire now: the hire screen
    /// closes back to the bar, and the bar's Hire Escort, and the next
    /// bar's, is greyed unless the fleet still has room.
    pub fn set_hire(&mut self, list: &HireList) {
        let before = self.redrawing();
        if let Some(Open::Bar(screen)) = &mut self.open {
            screen.set_hire(list);
        }
        self.note_closings(before);
        if let Some(barkeeping) = &mut self.barkeeping {
            barkeeping.room = list.room;
        }
    }

    /// The spaceport with the mission screens: Mission BBS opens the
    /// mission computer laid out by `computer`, its `DLOG` (or saying why
    /// there is none); [`MISSION_INFO_KEY`](crate::ui::mission_info::MISSION_INFO_KEY) asks for the mission info
    /// screen, laid out by `info`; the spaceport, and the trade center,
    /// outfitter, shipyard and bar when given, offer missions in the offer
    /// dialog of each offer's variant laid out by `offer`, the spaceport
    /// asking for its offer at once; and the mission texts show in "Desc
    /// Dialog" of each text's variant, laid out by `text` (and do not show
    /// without it). Without them, Mission BBS
    /// opens its placeholder and nothing offers missions.
    #[must_use]
    pub fn with_missions(
        self,
        computer: Result<DialogTemplate, String>,
        info: Result<DialogTemplate, String>,
        offer: OfferTemplates,
        text: DescTemplates,
    ) -> Self {
        let dialogs = self.port.as_ref().ok().map(|port| {
            MissionDialogs::new(text, info, Rc::clone(&port.metrics.0), port.style)
                .with_movies(self.movies.clone())
                .with_focus_ring(port.focus_ring)
        });
        let ring = self
            .port
            .as_ref()
            .map_or(FocusRing::default(), |port| port.focus_ring);
        let mut landing = VenueOffers::asking(offer.clone()).with_focus_ring(ring);
        landing.set_movies(self.movies.clone());
        Self {
            missioning: Some(Missioning {
                computer,
                landing,
                offer,
                requested: false,
                info_requested: false,
            }),
            dialogs,
            ..self
        }
    }

    /// The spaceport with the mission texts' and offers' movies played as
    /// `movies` says, over the spaceport and in each venue opened after it.
    #[must_use]
    pub fn with_movies(self, movies: Movies) -> Self {
        let dialogs = self
            .dialogs
            .map(|dialogs| dialogs.with_movies(movies.clone()));
        let missioning = self.missioning.map(|mut missioning| {
            missioning.landing.set_movies(movies.clone());
            missioning
        });
        Self {
            missioning,
            movies,
            dialogs,
            ..self
        }
    }

    /// The spaceport with each venue's offer timer, the bar's, the trade
    /// center's, the outfitter's and the shipyard's, running as `clock`
    /// reads [`RuleKey::OfferClock`](nova_sim::RuleKey::OfferClock): for
    /// each venue opened after it.
    #[must_use]
    pub fn with_offer_clock(self, clock: RuleSource) -> Self {
        Self {
            offer_clock: clock,
            ..self
        }
    }

    /// The spaceport with its dialogs' buttons taking the keyboard focus
    /// as `ring` says (the `dialog_focus_ring` rule): its own, its
    /// mission texts and offers, and each screen opened after it.
    #[must_use]
    pub fn with_focus_ring(mut self, ring: FocusRing) -> Self {
        if let Ok(port) = &mut self.port {
            port.focus_ring = ring;
            port.dialog.set_focus_ring(ring);
        }
        self.dialogs = self.dialogs.map(|dialogs| dialogs.with_focus_ring(ring));
        if let Some(missioning) = &mut self.missioning {
            missioning.landing = missioning.landing.clone().with_focus_ring(ring);
        }
        self
    }

    /// Whether the venue open restarted its offer timer since this was
    /// last asked, as the bar's hire screen opening does by the engine:
    /// whoever flies the ship rearms it
    /// ([`SpaceportView::rearm_offers`]) with a new gap.
    pub fn take_offer_restart(&mut self) -> bool {
        match &mut self.open {
            Some(Open::Bar(screen)) => screen.take_offer_restart(),
            _ => false,
        }
    }

    /// The venue that has asked for an offer since this was last asked,
    /// by its `AvailLoc`: the screen open's ([`BAR`], [`TRADE_CENTER`],
    /// [`OUTFITTER`] or [`SHIPYARD`]), or, with nothing open, the
    /// spaceport's ([`SPACEPORT`]); never the mission computer's or a
    /// placeholder's; nor the outfitter's before its listing is shown
    /// ([`SpaceportView::set_opened_outfitter`]), its ask kept until then.
    /// Whoever flies the ship answers with
    /// [`SpaceportView::open_offer`], or rearms it
    /// ([`SpaceportView::rearm_offers`]).
    pub fn take_offer_request(&mut self) -> Option<i16> {
        let (asked, venue) = match &mut self.open {
            Some(Open::Outfitter(_)) if self.outfitter_unlisted => return None,
            Some(Open::Bar(screen)) => (screen.take_offer_request(), BAR),
            Some(Open::Trade(screen)) => (screen.take_offer_request(), TRADE_CENTER),
            Some(Open::Outfitter(screen)) => (screen.take_offer_request(), OUTFITTER),
            Some(Open::Shipyard(screen)) => (screen.take_offer_request(), SHIPYARD),
            Some(Open::Service(_) | Open::Computer(_)) => return None,
            None => (
                self.missioning
                    .as_mut()
                    .is_some_and(|missioning| missioning.landing.take_request()),
                SPACEPORT,
            ),
        };
        asked.then_some(venue)
    }

    /// Opens the offer dialog on `prompt` over the venue that asks: the
    /// screen open, or, with nothing open, the spaceport.
    pub fn open_offer(&mut self, prompt: OfferPrompt) {
        match &mut self.open {
            Some(Open::Bar(screen)) => screen.open_offer(prompt),
            Some(Open::Trade(screen)) => screen.open_offer(prompt),
            Some(Open::Outfitter(screen)) => screen.open_offer(prompt),
            Some(Open::Shipyard(screen)) => screen.open_offer(prompt),
            Some(Open::Service(_) | Open::Computer(_)) => {}
            None => {
                if let (Ok(port), Some(missioning)) = (&self.port, &mut self.missioning) {
                    missioning
                        .landing
                        .open(prompt, Rc::clone(&port.metrics.0), port.style);
                }
            }
        }
    }

    /// The offer dialog open: the spaceport's, or the one over the venue
    /// open.
    #[must_use]
    pub fn offer_dialog(&self) -> Option<&OfferDialog> {
        if let Some(dialog) = self
            .missioning
            .as_ref()
            .and_then(|missioning| missioning.landing.dialog())
        {
            return Some(dialog);
        }
        match &self.open {
            Some(Open::Bar(screen)) => screen.offer_dialog(),
            Some(Open::Trade(screen)) => screen.offer_dialog(),
            Some(Open::Outfitter(screen)) => screen.offer_dialog(),
            Some(Open::Shipyard(screen)) => screen.offer_dialog(),
            _ => None,
        }
    }

    /// The offer the venue answered since this was last taken, and the
    /// choice made, once: the screen open's, or, with nothing open, the
    /// spaceport's.
    pub fn take_offer_choice(&mut self) -> Option<(MissionId, OfferChoice)> {
        match &mut self.open {
            Some(Open::Bar(screen)) => screen.take_offer_choice(),
            Some(Open::Trade(screen)) => screen.take_offer_choice(),
            Some(Open::Outfitter(screen)) => screen.take_offer_choice(),
            Some(Open::Shipyard(screen)) => screen.take_offer_choice(),
            Some(Open::Service(_) | Open::Computer(_)) => None,
            None => self.missioning.as_mut()?.landing.take_choice(),
        }
    }

    /// The venue open asks for its next offer after `gap`. The spaceport
    /// never asks again.
    pub fn rearm_offers(&mut self, gap: Duration) {
        match &mut self.open {
            Some(Open::Bar(screen)) => screen.rearm(gap),
            Some(Open::Trade(screen)) => screen.rearm(gap),
            Some(Open::Outfitter(screen)) => screen.rearm(gap),
            Some(Open::Shipyard(screen)) => screen.rearm(gap),
            _ => {}
        }
    }

    /// Whether Mission BBS has asked for the mission computer since this
    /// was last asked: whoever flies the ship answers with
    /// [`SpaceportView::open_mission_computer`].
    pub fn take_mission_computer_request(&mut self) -> bool {
        self.missioning
            .as_mut()
            .is_some_and(|missioning| std::mem::take(&mut missioning.requested))
    }

    /// Opens the mission computer on `computer`; or, refused, shows why
    /// over the spaceport (`_DoMissionDialog` @0xa380a-0xa3956), with no
    /// movie ([`SpaceportView::open_mission_computer_after`]).
    pub fn open_mission_computer(&mut self, computer: Result<MissionComputer, ComputerRefusal>) {
        self.open_mission_computer_after(computer, None);
    }

    /// Opens the mission computer on `computer`; or, refused, shows why
    /// over the spaceport (`_DoMissionDialog` @0xa380a-0xa3956), and then
    /// `after`, the movie a `STR#` text plays once it closes: the last
    /// `dësc` loaded's after-movie, as `_ShowTextDialog`'s after-check
    /// reads it (@0xa3956, then @0x5b40b-0x5b44f; see
    /// [`SessionFlight::loaded_after_movie`](crate::flight::SessionFlight::loaded_after_movie)).
    pub fn open_mission_computer_after(
        &mut self,
        computer: Result<MissionComputer, ComputerRefusal>,
        after: Option<DescMovie>,
    ) {
        let computer = match computer {
            Ok(computer) => computer,
            Err(refusal) => {
                self.port_dialog_opened = Some(PortDialog::MissionComputer);
                if let Some(dialogs) = &mut self.dialogs {
                    dialogs.show_descs(vec![DescShown {
                        text: refusal.message(),
                        graphic: None,
                        movie: after,
                    }]);
                }
                self.port_dialog_closed = Some(PortDialog::MissionComputer);
                return;
            }
        };
        let (Ok(port), Some(missioning)) = (&self.port, &self.missioning) else {
            return;
        };
        let layout = missioning
            .computer
            .clone()
            .map(|template| (template, Rc::clone(&port.metrics.0)));
        self.open = Some(Open::Computer(Box::new(
            MissionComputerScreen::new(layout, computer, port.style)
                .with_focus_ring(port.focus_ring),
        )));
        self.port_dialog_opened = Some(PortDialog::MissionComputer);
    }

    /// The mission computer open, if it is.
    #[must_use]
    pub fn mission_computer(&self) -> Option<&MissionComputerScreen> {
        match &self.open {
            Some(Open::Computer(screen)) => Some(screen),
            _ => None,
        }
    }

    /// The offer the mission computer open has shown selected since this
    /// was last taken, once, whose text is to be loaded: whoever flies the
    /// ship answers with [`SpaceportView::show_computer_row`].
    pub fn take_computer_text_request(&mut self) -> Option<MissionId> {
        match &mut self.open {
            Some(Open::Computer(screen)) => screen.take_text_request(),
            _ => None,
        }
    }

    /// Shows `row`, loaded, in the mission computer open while its offer
    /// is selected, its cut name filled by `names`
    /// ([`MissionComputerScreen::show_row`]).
    pub fn show_computer_row(&mut self, row: &ComputerRow, names: &mut impl OfferNames) {
        if let Some(Open::Computer(screen)) = &mut self.open {
            screen.show_row(row, names);
        }
    }

    /// The slot the mission info screen open has shown selected since
    /// this was last taken, once, whose `QuickBrief` is to be loaded:
    /// whoever flies the ship answers with [`SpaceportView::show_info_text`].
    pub fn take_info_text_request(&mut self) -> Option<u8> {
        self.dialogs.as_mut()?.take_info_text_request()
    }

    /// Shows `text`, slot `slot`'s loaded, in the mission info screen
    /// open while that slot is selected.
    pub fn show_info_text(&mut self, slot: u8, text: &str) {
        if let Some(dialogs) = &mut self.dialogs {
            dialogs.show_info_text(slot, text);
        }
    }

    /// The mission the mission computer open asked to accept since this
    /// was last taken, once.
    pub fn take_mission_accept(&mut self) -> Option<MissionId> {
        match &mut self.open {
            Some(Open::Computer(screen)) => screen.take_accept(),
            _ => None,
        }
    }

    /// After an accept, the mission computer open shows `computer`; or,
    /// refused, it closes with nothing said, as the original's re-entry
    /// does (`_reenterMissionDialog` @0xa380a, @0xa3910).
    pub fn set_mission_computer(&mut self, computer: Result<MissionComputer, ComputerRefusal>) {
        let Some(Open::Computer(screen)) = &mut self.open else {
            return;
        };
        match computer {
            Ok(computer) => screen.set_computer(computer),
            Err(_) => self.close_open(),
        }
    }

    /// Shows each of `texts` in "Desc Dialog", one after another as Done
    /// closes each, over whatever is open, each taking every input until
    /// it closes (`_ShowTextDialog` @0x5a752), and each offer movie among
    /// them in its place ([`MissionDialogs::show_texts`]).
    pub fn show_mission_texts(&mut self, texts: impl IntoIterator<Item = impl Into<MissionShown>>) {
        match &mut self.dialogs {
            Some(dialogs) => dialogs.show_texts(texts),
            None => {
                self.texts_dropped += texts
                    .into_iter()
                    .map(Into::into)
                    .filter(|shown| matches!(shown, MissionShown::Text(_)))
                    .count();
            }
        }
        self.dispose_answered_offer();
    }

    /// Lets go of the offer dialog answered and left drawn under what its
    /// answer shows, at the spaceport or over the venue open: whoever
    /// flies the ship hides it once the refusal showed a `RefuseText`
    /// ([`RefuseShown`](nova_sim::RefuseShown), `_OfferOneMission`
    /// @0xa2e8e).
    pub fn release_answered_offer(&mut self) {
        if let Some(missioning) = &mut self.missioning {
            missioning.landing.release_answered();
        }
        match &mut self.open {
            Some(Open::Bar(screen)) => screen.release_answered_offer(),
            Some(Open::Trade(screen)) => screen.release_answered_offer(),
            Some(Open::Outfitter(screen)) => screen.release_answered_offer(),
            Some(Open::Shipyard(screen)) => screen.release_answered_offer(),
            Some(Open::Service(_) | Open::Computer(_)) | None => {}
        }
    }

    /// Disposes of the offer dialog answered and left drawn once no
    /// mission dialog covers it (`DisposeDialogNoFlash` @0xa361e, after
    /// the after-check @0xa3580-0xa35b8), in the input or tick that closed
    /// the last, so no frame draws it uncovered.
    fn dispose_answered_offer(&mut self) {
        if !self.mission_dialogs_open() {
            self.release_answered_offer();
        }
    }

    /// The offer whose dialog the mission info screen opened over asks to
    /// have its `dësc` loaded again since this was last taken, once: the
    /// screen has closed (`_OfferOneMission` @0xa335c). Whoever flies the
    /// ship reloads it
    /// ([`FlightView::reload_offer_desc`](crate::flight::FlightView::reload_offer_desc)).
    pub fn take_offer_reload(&mut self) -> Option<MissionId> {
        self.offer_reload.take()
    }

    /// How many of the mission texts shown have closed since this was
    /// last taken, each once ([`MissionDialogs::take_texts_closed`]);
    /// without the dialogs to show them, each closes as it is dropped.
    /// Whoever flies the ship redraws the bar for each
    /// ([`FlightView::close_mission_text`](crate::flight::FlightView::close_mission_text)).
    pub fn take_texts_closed(&mut self) -> usize {
        std::mem::take(&mut self.texts_dropped)
            + self
                .dialogs
                .as_mut()
                .map_or(0, MissionDialogs::take_texts_closed)
    }

    /// The spaceport with the player info screen's dialog, `template`
    /// (`DLOG` 1017), and its Yes/No dialog, `confirm` (`DLOG` 3002), or
    /// why there are none, and its background pictures' sizes,
    /// `background`.
    #[must_use]
    pub fn with_player_info(
        self,
        template: Result<DialogTemplate, String>,
        confirm: Result<DialogTemplate, String>,
        background: BackgroundSizes,
    ) -> Self {
        Self {
            player_info_dialogs: Some(PlayerInfoDialogs {
                template,
                confirm,
                background,
            }),
            ..self
        }
    }

    /// Whether [`PLAYER_INFO_KEY`] has asked for the player info screen
    /// since this was last asked, wherever it is read (see the module
    /// docs): whoever flies the ship answers with
    /// [`SpaceportView::open_player_info`].
    pub fn take_player_info_request(&mut self) -> bool {
        let own = std::mem::take(&mut self.player_info_requested);
        let landing = self
            .missioning
            .as_mut()
            .is_some_and(|missioning| missioning.landing.take_player_info_request());
        let open = match &mut self.open {
            Some(Open::Bar(screen)) => screen.take_player_info_request(),
            Some(Open::Trade(screen)) => screen.take_player_info_request(),
            Some(Open::Shipyard(screen)) => screen.take_player_info_request(),
            Some(Open::Outfitter(screen)) => screen.take_player_info_request(),
            Some(Open::Computer(screen)) => screen.take_player_info_request(),
            Some(Open::Service(_)) | None => false,
        };
        own || landing || open
    }

    /// Opens the player info screen on `info` over whatever is open, laid
    /// out by the dialogs given ([`SpaceportView::with_player_info`]); it
    /// says why not without them. Without the spaceport's own dialog,
    /// nothing opens.
    pub fn open_player_info(&mut self, info: PlayerInfo) {
        let Ok(port) = &self.port else {
            return;
        };
        let dialogs = self.player_info_dialogs.clone().unwrap_or_else(|| {
            let none = || Err("the player info dialog was not given".to_owned());
            PlayerInfoDialogs {
                template: none(),
                confirm: none(),
                background: BackgroundSizes {
                    upper: None,
                    middle: None,
                    lower: None,
                },
            }
        });
        let layout = dialogs
            .template
            .map(|template| (template, Rc::clone(&port.metrics.0)));
        self.player_info = Some(
            PlayerInfoScreen::new(
                layout,
                dialogs.confirm,
                info,
                port.style,
                dialogs.background,
            )
            .with_focus_ring(port.focus_ring),
        );
    }

    /// The player info screen open, if it is.
    #[must_use]
    pub fn player_info(&self) -> Option<&PlayerInfoScreen> {
        self.player_info.as_ref()
    }

    /// Whether the player info screen's Jettison was confirmed since this
    /// was last taken, once: whoever flies the ship jettisons everything.
    pub fn take_player_info_jettison(&mut self) -> bool {
        std::mem::take(&mut self.player_info_jettison)
    }

    /// Whether [`MISSION_INFO_KEY`](crate::ui::mission_info::MISSION_INFO_KEY) has asked for the mission info screen
    /// since this was last asked, in the spaceport, the bar or the mission
    /// computer: whoever flies the ship answers with
    /// [`SpaceportView::open_mission_info`].
    pub fn take_mission_info_request(&mut self) -> bool {
        let own = self
            .missioning
            .as_mut()
            .is_some_and(|missioning| std::mem::take(&mut missioning.info_requested));
        let landing = self
            .missioning
            .as_mut()
            .is_some_and(|missioning| missioning.landing.take_info_request());
        let open = match &mut self.open {
            Some(Open::Bar(screen)) => screen.take_info_request(),
            Some(Open::Computer(screen)) => screen.take_info_request(),
            Some(Open::Trade(screen)) => screen.take_info_request(),
            Some(Open::Outfitter(screen)) => screen.take_info_request(),
            Some(Open::Shipyard(screen)) => screen.take_info_request(),
            Some(Open::Service(_)) | None => false,
        };
        own || landing || open
    }

    /// Opens the mission info screen on `info` over whatever is open; with
    /// nothing to show, it does not open, as the original's does not
    /// (@0xa3e37-0xa3ec1). Opened over an offer or the mission computer,
    /// its map key opens no map, as `_missionDialog` is set there
    /// (`_DoMissionInfoDialog` @0x9e3c6-0x9e3cd).
    pub fn open_mission_info(&mut self, info: MissionInfo) {
        let over_mission_dialog =
            self.offer_dialog().is_some() || matches!(self.open, Some(Open::Computer(_)));
        if let Some(dialogs) = &mut self.dialogs {
            dialogs.open_info_with_map(info, !over_mission_dialog);
            self.sounds.extend(dialogs.take_sounds());
        }
    }

    /// The mission info screen open, if it is.
    #[must_use]
    pub fn mission_info(&self) -> Option<&MissionInfoScreen> {
        self.dialogs.as_ref()?.info()
    }

    /// The slot the mission info screen asked to abort since this was
    /// last taken, once.
    pub fn take_mission_abort(&mut self) -> Option<u8> {
        self.dialogs.as_mut()?.take_abort()
    }

    /// After an abort, the mission info screen shows `info`, closing once
    /// nothing is left.
    pub fn set_mission_info(&mut self, info: MissionInfo) {
        if let Some(dialogs) = &mut self.dialogs {
            dialogs.set_info(info);
            self.sounds.extend(dialogs.take_sounds());
        }
        self.note_info_closed();
    }

    /// Notes the mission info screen's close since the mission dialogs
    /// were last asked, if it closed: the keys of the screen beneath are
    /// let go, and a close laid out redraws the status bar
    /// ([`SpaceportView::take_status_refresh`]).
    fn note_info_closed(&mut self) {
        let Some(dialogs) = &mut self.dialogs else {
            return;
        };
        let closed = dialogs.take_info_closed();
        if dialogs.take_info_closed_laid_out() {
            self.status_refresh = true;
        }
        if closed {
            self.release_keys();
            if let Some(dialog) = self.offer_dialog() {
                self.offer_reload = Some(dialog.prompt().mission);
            }
        }
    }

    /// What is open whose close redraws the status bar, or is the hire
    /// screen's, for [`SpaceportView::note_closings`].
    fn redrawing(&self) -> Redrawing {
        match &self.open {
            Some(Open::Bar(bar)) => Redrawing {
                news: bar
                    .news_screen()
                    .is_some_and(|news| news.problem().is_none()),
                panel: bar.hire_screen().and_then(HireScreen::info_panel).is_some(),
                hire: bar.hire_screen().is_some(),
            },
            Some(Open::Shipyard(shipyard)) => Redrawing {
                panel: shipyard.info_panel().is_some(),
                ..Redrawing::default()
            },
            _ => Redrawing::default(),
        }
    }

    /// Notes what has closed since `before` was taken: News or a ship
    /// description panel ([`SpaceportView::take_status_refresh`]), or the
    /// hire screen ([`SpaceportView::take_port_dialog_closed`]).
    fn note_closings(&mut self, before: Redrawing) {
        let after = self.redrawing();
        let closed = |was: bool, is: bool| was && !is;
        if closed(before.news, after.news) || closed(before.panel, after.panel) {
            self.status_refresh = true;
        }
        if closed(before.hire, after.hire) {
            self.port_dialog_closed = Some(PortDialog::Hire);
        }
    }

    /// The mission text shown, if any.
    #[must_use]
    pub fn mission_text(&self) -> Option<&DescDialog> {
        self.dialogs.as_ref()?.text()
    }

    /// Queues `text` to show in "Desc Dialog", with no picture or movie,
    /// over the spaceport, when it has the mission dialogs: a text no
    /// mission is behind, such as the report of the escorts sold and
    /// upgraded on leaving.
    pub fn show_text(&mut self, text: &str) {
        if let Some(dialogs) = &mut self.dialogs {
            dialogs.show_descs(vec![DescShown {
                text: text.to_owned(),
                graphic: None,
                movie: None,
            }]);
        }
    }

    /// Whether mission texts (or their movies) are queued over the
    /// spaceport.
    fn texts_queued(&self) -> bool {
        self.dialogs
            .as_ref()
            .is_some_and(MissionDialogs::texts_queued)
    }

    /// Whether a mission text's movie or an offer's is in front, playing
    /// or about to: `_PlayQTMovie`'s modal loop, which takes no menu
    /// events (`WaitNextEvent` with mask 0 @0x55e4e).
    #[must_use]
    pub fn playing(&self) -> bool {
        self.dialogs.as_ref().is_some_and(MissionDialogs::playing)
            || self.offer_dialog().is_some_and(OfferDialog::playing)
    }

    /// Whether a mission text or the mission info screen is open.
    fn mission_dialogs_open(&self) -> bool {
        self.dialogs.as_ref().is_some_and(MissionDialogs::is_open)
    }

    /// Whether a dialog open over the spaceport or one of its screens
    /// holds a `Q`'s leave: one whose filter never reads
    /// `_absquatulateStr`, so the original leaves only once it closes.
    /// Those are the mission texts (`_ShowTextDialog` @0x5a752) and the
    /// outfitter's alert, an offer (`_OfferOneMission`), the mission info
    /// and player info screens (`_DoMissionInfoDialog`,
    /// `_DoPlayerInfoDialog`), the exchange's and the outfitter's
    /// quantity (`_DoQuantityDialog` @0x5e25a, @0x5c1aa), the shipyard's
    /// naming prompt (`_EVTextInputDialog` @0x5eb6c), the shipyard's and
    /// the hire screen's Info panel (`_DoShipyardDescription`, whose
    /// `_ShipDescFilter` @0x510e5 never reads the string) and the bar's
    /// news (`_DoNewsDialog` @0x491d7). The spaceport itself and the screens
    /// over it close at once instead, their filters reading the string
    /// first: `_PortFilter` @0x4f047, `_BarFilter` @0x47a51,
    /// `_TradeFilter` @0x4e0b5, `_OutfitFilter` @0x58042, and
    /// `_ShipyardFilter` @0x60452 (the shipyard and the bar's hire
    /// screen) and `_MissionFilter` @0x9f7b2 (the mission computer).
    #[must_use]
    pub fn holds_leave(&self) -> bool {
        if self.mission_dialogs_open()
            || self.offer_dialog().is_some()
            || self.player_info.is_some()
        {
            return true;
        }
        match &self.open {
            Some(Open::Trade(screen)) => screen.quantity().is_some(),
            Some(Open::Outfitter(screen)) => {
                screen.quantity().is_some() || screen.alert_dialog().is_some()
            }
            Some(Open::Shipyard(screen)) => {
                screen.naming().is_some() || screen.info_panel().is_some()
            }
            Some(Open::Bar(screen)) => {
                screen.news_screen().is_some()
                    || screen
                        .hire_screen()
                        .is_some_and(|hire| hire.info_panel().is_some())
            }
            Some(Open::Service(_) | Open::Computer(_)) | None => false,
        }
    }

    /// The stellar landed on.
    #[must_use]
    pub fn stellar(&self) -> StellarId {
        self.stellar
    }

    /// Whether the player has left: the router takes off.
    #[must_use]
    pub fn left(&self) -> bool {
        self.left
    }

    /// The services offered, in [`Service`] order.
    #[must_use]
    pub fn offered(&self) -> &[Service] {
        self.port.as_ref().map_or(&[], |port| &port.offered)
    }

    /// The service placeholder open, if any.
    #[must_use]
    pub fn open_service(&self) -> Option<&ServiceScreen> {
        match &self.open {
            Some(Open::Service(screen)) => Some(screen),
            _ => None,
        }
    }

    /// The dialog, if the spaceport could be laid out.
    #[must_use]
    pub fn dialog(&self) -> Option<&Dialog> {
        self.port.as_ref().ok().map(|port| &port.dialog)
    }

    /// Why the spaceport cannot be shown, if it cannot.
    #[must_use]
    pub fn problem(&self) -> Option<&str> {
        self.port.as_ref().err().map(String::as_str)
    }

    /// The landscape `PICT` that is missing, if it is.
    #[must_use]
    pub fn missing_landscape(&self) -> Option<i16> {
        self.port.as_ref().ok()?.landscape.err()
    }

    /// Why the description box is empty, if it is for want of a `dësc`.
    #[must_use]
    pub fn description_problem(&self) -> Option<&str> {
        self.port.as_ref().ok()?.description_problem.as_deref()
    }

    /// Activates dialog item `item`: Leave leaves, Recharge (where fuel is
    /// sold) asks for a refill, and an offered service's button opens it.
    /// Anything else does nothing.
    /// Opens the bar, laid out with `metrics` and its buttons in `style`,
    /// when it is given, and true; false, opening nothing, without it.
    fn open_bar_screen(
        &mut self,
        metrics: Rc<dyn TextMetrics>,
        style: ButtonStyle,
        ring: FocusRing,
    ) -> bool {
        let Some(barkeeping) = &mut self.barkeeping else {
            return false;
        };
        let layout = Ok((barkeeping.templates.clone(), metrics));
        let mut bar = BarScreen::new(
            layout,
            self.stellar,
            barkeeping.hiring.clone(),
            style,
            barkeeping.room,
        );
        bar = bar.with_news(barkeeping.news.clone()).with_focus_ring(ring);
        if let Some(missioning) = &self.missioning {
            bar = bar
                .with_offers(missioning.offer.clone())
                .with_offer_clock(self.offer_clock);
            bar.set_offer_movies(self.movies.clone());
        }
        barkeeping.opened = true;
        self.open = Some(Open::Bar(Box::new(bar)));
        true
    }

    fn activate(&mut self, item: usize) {
        if item == LEAVE_ITEM {
            self.left = true;
            return;
        }
        let Ok(port) = &mut self.port else {
            return;
        };
        if item == RECHARGE_ITEM && port.sells_fuel {
            self.recharge = true;
            return;
        }
        let Some(service) = item_service(item).filter(|s| port.offered.contains(s)) else {
            return;
        };
        if let (Some(missioning), Service::MissionBbs) = (&mut self.missioning, service) {
            missioning.requested = true;
            return;
        }
        let style = port.style;
        let metrics = Rc::clone(&port.metrics.0);
        let ring = port.focus_ring;
        if service == Service::Bar && self.open_bar_screen(Rc::clone(&metrics), style, ring) {
            return;
        }
        if lists_no_ships(self.shipbuying.as_ref(), service) {
            self.show_text(NO_SHIPS);
            return;
        }
        let offer = self
            .missioning
            .as_ref()
            .map(|missioning| missioning.offer.clone());
        let clock = self.offer_clock;
        self.open = Some(
            match (&self.trade, &self.outfitting, &self.shipbuying, service) {
                (Some((template, quantity, market, colors)), _, _, Service::TradeCenter) => {
                    let layout = template.clone().map(|template| (template, metrics));
                    let screen = TradeScreen::new(layout, quantity.clone(), market.clone(), style)
                        .with_colors(*colors)
                        .with_focus_ring(ring);
                    Open::Trade(Box::new(match offer {
                        Some(offer) => screen.with_offers(offer).with_offer_clock(clock),
                        None => screen,
                    }))
                }
                (_, _, Some(shipbuying), Service::Shipyard) => {
                    let layout = shipbuying
                        .template
                        .clone()
                        .map(|template| (template, metrics));
                    let screen = ShipyardScreen::new(
                        layout,
                        shipbuying.info.clone(),
                        shipbuying.text_input.clone(),
                        shipbuying.shipyard.clone(),
                        Rc::clone(&shipbuying.catalog),
                        style,
                    )
                    .with_focus_ring(ring);
                    Open::Shipyard(Box::new(match offer {
                        Some(offer) => screen.with_offers(offer).with_offer_clock(clock),
                        None => screen,
                    }))
                }
                (_, Some(outfitting), _, Service::Outfitter) => {
                    self.outfitter_opened = true;
                    self.outfitter_unlisted = true;
                    let layout = outfitting
                        .template
                        .clone()
                        .map(|template| (template, metrics));
                    let screen = OutfitterScreen::new(
                        layout,
                        outfitting.quantity.clone(),
                        outfitting.alert.clone(),
                        outfitting.outfitter.clone(),
                        Rc::clone(&outfitting.catalog),
                        style,
                    )
                    .with_focus_ring(ring);
                    Open::Outfitter(Box::new(match offer {
                        Some(offer) => screen.with_offers(offer).with_offer_clock(clock),
                        None => screen,
                    }))
                }
                _ => Open::Service(ServiceScreen::new(service, style, metrics)),
            },
        );
        self.give_offer_movies();
        self.port_dialog_opened = self.open.as_ref().and_then(Open::port_dialog);
    }

    /// Gives the movies to the offers of the venue open.
    fn give_offer_movies(&mut self) {
        let movies = self.movies.clone();
        match &mut self.open {
            Some(Open::Trade(screen)) => screen.set_offer_movies(movies),
            Some(Open::Outfitter(screen)) => screen.set_offer_movies(movies),
            Some(Open::Shipyard(screen)) => screen.set_offer_movies(movies),
            _ => {}
        }
    }
}

/// Whether `service` is the shipyard of `shipbuying` listing nothing:
/// then `_DoShipyardDialog` shows `STR#` 2002 #223 ([`NO_SHIPS`]) in "Desc
/// Dialog" and returns before its nav save, its dialog and its offer
/// (@0x5e694-0x5e6be → @0x5e8da → @0x5f906), so the Shipyard opens
/// nothing.
fn lists_no_ships(shipbuying: Option<&Shipbuying>, service: Service) -> bool {
    service == Service::Shipyard
        && shipbuying.is_some_and(|shipbuying| shipbuying.shipyard.rows.is_empty())
}

impl Screen for SpaceportView {
    /// An open service takes every input until it closes. Otherwise the
    /// map key asks for the course map, as `_PortFilter`'s first key check
    /// does, the mission info and player info keys ask for their screens, a press of
    /// one of the [`PORT_KEYS`](super::layout::PORT_KEYS) presses its
    /// button, and every other input goes to the dialog. It never quits.
    fn input(&mut self, input: &Input) -> ScreenAction {
        if let Some(screen) = &mut self.player_info {
            screen.input(input);
            self.player_info_jettison |= screen.take_jettison();
            if screen.closed() {
                self.status_refresh |= screen.problem().is_none();
                self.player_info = None;
                self.release_keys();
            }
            return ScreenAction::None;
        }
        if let Some(dialogs) = &mut self.dialogs
            && dialogs.is_open()
        {
            dialogs.input(input);
            self.sounds.extend(dialogs.take_sounds());
            self.note_info_closed();
            self.pass_unclaimed();
            self.dispose_answered_offer();
            return ScreenAction::None;
        }
        if let Some(missioning) = &mut self.missioning
            && missioning.landing.input(input, &mut self.sounds)
        {
            return ScreenAction::None;
        }
        if self.open.is_some() {
            let before = self.redrawing();
            if let Some(open) = &mut self.open {
                let screen = open.screen_mut();
                screen.input(input);
                self.sounds.extend(screen.take_sounds());
                if open.closed() {
                    self.close_open();
                }
            }
            self.note_closings(before);
            return ScreenAction::None;
        }
        // `_PortFilter`'s first key check (@0x4f165-0x4f185).
        if is_map_press(input) && self.port.is_ok() {
            self.port_map_requested = true;
            return ScreenAction::None;
        }
        if is_mission_info_press(input)
            && let Some(missioning) = &mut self.missioning
        {
            missioning.info_requested = true;
            return ScreenAction::None;
        }
        if is_player_info_press(input) && self.port.is_ok() {
            self.player_info_requested = true;
            return ScreenAction::None;
        }
        if let (
            Input::PointerButton {
                pressed: true, at, ..
            },
            Ok(port),
            Some(ambient),
        ) = (*input, &self.port, &self.ambient)
            && port
                .dialog
                .item_bounds(LANDSCAPE_ITEM)
                .is_some_and(|landscape| landscape.contains(at))
        {
            self.sounds.extend(ambient.pressed().map(Sound::Ui));
        }
        if let Input::Key {
            key,
            pressed: true,
            repeat: false,
        } = *input
            && self.port.is_ok()
            && let Some(item) = port_key_item(key)
        {
            self.activate(item);
            return ScreenAction::None;
        }
        let event = match &mut self.port {
            Ok(port) => port.dialog.input(input),
            Err(_) => match *input {
                Input::Key {
                    key: Key::Enter | Key::Escape,
                    pressed: true,
                    repeat: false,
                } => Some(DialogEvent::Item(LEAVE_ITEM)),
                _ => None,
            },
        };
        if let Some(DialogEvent::Item(item)) = event {
            self.activate(item);
        }
        ScreenAction::None
    }

    /// The mission texts' movie moves on, and the spaceport's offer's.
    /// The screen open ticks; while a mission text, the mission info
    /// screen or the player info screen covers it, only its offer timer
    /// does, as covered. The ambient sound's frame passes, covered or not
    /// ([`Ambient::tick`]).
    fn tick(&mut self, dt: Duration) {
        let covered = self.covered();
        if let Some(sound) = self
            .ambient
            .as_mut()
            .and_then(|ambient| ambient.tick(dt, covered, &mut self.chance))
        {
            self.sounds.push(Sound::Ui(sound));
        }
        if let Some(dialogs) = &mut self.dialogs {
            dialogs.tick(dt);
            self.sounds.extend(dialogs.take_sounds());
        }
        self.pass_unclaimed();
        self.dispose_answered_offer();
        let open_covered = self.mission_dialogs_open() || self.player_info.is_some();
        if let Some(missioning) = &mut self.missioning
            && !open_covered
        {
            missioning.landing.tick_dialog(dt);
        }
        if let Some(open) = &mut self.open {
            if open_covered {
                open.tick_covered(dt);
            } else {
                open.screen_mut().tick(dt);
            }
        }
    }

    /// What is open, the spaceport's offer, the mission info screen, the
    /// mission text shown over them, then the player info screen over
    /// everything.
    fn draw(&self, list: &mut DrawList) {
        self.draw_below(list);
        if let Some(missioning) = &self.missioning {
            missioning.landing.draw(list);
        }
        if let Some(dialogs) = &self.dialogs {
            dialogs.draw(list);
        }
        if let Some(screen) = &self.player_info {
            screen.draw(list);
        }
    }

    fn cancel_pointer(&mut self) {
        if let Some(screen) = &mut self.player_info {
            screen.cancel_pointer();
            return;
        }
        if let Some(dialogs) = &mut self.dialogs
            && dialogs.is_open()
        {
            dialogs.cancel_pointer();
            return;
        }
        if self
            .missioning
            .as_mut()
            .is_some_and(|missioning| missioning.landing.cancel_pointer())
        {
            return;
        }
        match (&mut self.open, &mut self.port) {
            (Some(open), _) => open.screen_mut().cancel_pointer(),
            (None, Ok(port)) => port.dialog.cancel_pointer(),
            (None, Err(_)) => {}
        }
    }

    /// Lets go of the keys the player info screen holds, or else the
    /// screen open.
    fn release_keys(&mut self) {
        if let Some(screen) = &mut self.player_info {
            screen.release_keys();
        } else if let Some(open) = &mut self.open {
            open.screen_mut().release_keys();
        }
    }

    /// The open service's sounds and the ambient sound's, in order. The
    /// spaceport's own buttons are silent under the pointer, as
    /// `_TrackPlanetButtons` @0x2996f plays nothing.
    fn take_sounds(&mut self) -> Vec<Sound> {
        std::mem::take(&mut self.sounds)
    }

    /// While a cinematic movie hides the cursor: a mission text's, or an
    /// offer's.
    fn hides_pointer(&self) -> bool {
        self.dialogs.as_ref().is_some_and(Screen::hides_pointer)
            || self.offer_dialog().is_some_and(Screen::hides_pointer)
    }
}

impl SpaceportView {
    /// Gives what was pressed under the last mission movie to what is
    /// beneath it, as the original's queued events reach the next filter.
    fn pass_unclaimed(&mut self) {
        let unclaimed = self
            .dialogs
            .as_mut()
            .map(MissionDialogs::take_unclaimed)
            .unwrap_or_default();
        for input in &unclaimed {
            self.input(input);
        }
    }

    /// Draws the spaceport and the screen open over it.
    fn draw_below(&self, list: &mut DrawList) {
        // A service's placeholder takes the whole screen; the exchange and
        // the outfitter are dialogs over the spaceport.
        if let Some(Open::Service(screen)) = &self.open {
            screen.draw(list);
            return;
        }
        let port = match &self.port {
            Ok(port) => port,
            Err(reason) => {
                list.text(
                    format!(
                        "Cannot show the spaceport: {reason}. Press Return or Escape to take off."
                    ),
                    PROBLEM_AT,
                    PROBLEM_SIZE,
                    None,
                    Color::ERROR,
                );
                return;
            }
        };
        let dialog = &port.dialog;
        list.picture(BACKGROUND, dialog.bounds().min);
        if let (Ok(id), Some(area)) = (port.landscape, dialog.item_bounds(LANDSCAPE_ITEM)) {
            list.stretched_picture(ImageKey::picture(id), area.min, area.width(), area.height());
        }
        if let Some(bar) = dialog.item_bounds(NAME_ITEM) {
            let metrics = &port.metrics.0;
            let width = metrics.width(NAME_FONT, NAME_SIZE, &port.name);
            let height = metrics.line_height(NAME_FONT, NAME_SIZE);
            let centre = bar.center();
            let origin = Point::new(centre.x - width / 2.0, centre.y - height / 2.0);
            list.text_in(NAME_FONT, &port.name, origin, NAME_SIZE, None, Color::WHITE);
        }
        dialog.draw(list);
        match &self.open {
            Some(Open::Trade(screen)) => screen.draw(list),
            Some(Open::Outfitter(screen)) => screen.draw(list),
            Some(Open::Shipyard(screen)) => screen.draw(list),
            Some(Open::Bar(screen)) => screen.draw(list),
            Some(Open::Computer(screen)) => screen.draw(list),
            _ => {}
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
pub(in crate::spaceport) mod tests {
    use crate::ui::mission_info::MISSION_INFO_KEY;
    use std::cell::RefCell;

    use nova_sim::landing::StellarFlags;

    use super::*;
    use crate::draw::{DrawCommand, fill_command};
    use crate::geometry::Bounds;
    use crate::input::MouseButton;
    use crate::sound::{Sound, UiSound};
    use crate::spaceport::catalog::PortRecord;
    use crate::spaceport::layout::{SERVICE_ITEMS, service_item};
    use crate::text::fixture::MonoMetrics;
    use crate::ui::ScrollText;
    use crate::ui::dialog::{ItemSpec, ItemTemplate, Placement};

    /// Stellar 140, Earth: a planet with a trade center and a bar, of
    /// `Type` 3 with no custom picture, so its landscape is `PICT` 10003.
    pub(in crate::spaceport) struct FakePort {
        port: Result<PortRecord, String>,
        pictures: Vec<i16>,
        description: Result<String, String>,
        /// The stellars and descriptions asked for.
        asked: RefCell<Vec<String>>,
    }

    const FLAGS: u32 = StellarFlags::CAN_LAND | StellarFlags::TRADE_CENTER | StellarFlags::BAR;

    fn catalog() -> FakePort {
        FakePort {
            port: Ok(PortRecord {
                name: "Earth".to_owned(),
                flags: FLAGS,
                cust_pic_id: -1,
                graphic_type: 3,
                cust_snd_id: -1,
                flags2: 0,
            }),
            pictures: vec![10_003],
            description: Ok("Blue and green.\rHome.".to_owned()),
            asked: RefCell::default(),
        }
    }

    impl SpaceportCatalog for FakePort {
        fn stellar_port(&self, id: StellarId) -> Result<PortRecord, String> {
            self.asked.borrow_mut().push(format!("spöb {}", id.0));
            self.port.clone()
        }

        fn picture_exists(&self, id: i16) -> bool {
            self.pictures.contains(&id)
        }
    }

    impl DescriptionSource for FakePort {
        fn description(&self, id: i16) -> Result<String, String> {
            self.asked.borrow_mut().push(format!("dësc {id}"));
            self.description.clone()
        }

        fn button_style(&self) -> ButtonStyle {
            ButtonStyle {
                up: Color::BLACK,
                ..ButtonStyle::STOCK
            }
        }
    }

    fn rect(x: f32, y: f32, w: f32, h: f32) -> Bounds {
        Bounds::at(Point::new(x, y), w, h)
    }

    /// Stock "Spaceport": `DLOG` 1000, 618 x 517 and centred, and its
    /// fifteen user items, where items 2, 3, 6, 14 and 15 are disabled and
    /// items 1, 2, 14 and 15 parked outside it.
    fn template() -> DialogTemplate {
        let items = [
            (242.0, 551.0, 200.0, 25.0, true),
            (452.0, 549.0, 68.0, 30.0, false),
            (159.0, 297.0, 303.0, 18.0, false),
            (471.0, 416.0, 145.0, 25.0, true),
            (3.0, 3.0, 612.0, 285.0, true),
            (160.0, 327.0, 301.0, 185.0, false),
            (3.0, 414.0, 145.0, 25.0, true),
            (471.0, 375.0, 145.0, 25.0, true),
            (471.0, 333.0, 145.0, 25.0, true),
            (3.0, 374.0, 145.0, 25.0, true),
            (3.0, 333.0, 145.0, 25.0, true),
            (471.0, 456.0, 145.0, 25.0, true),
            (3.0, 456.0, 145.0, 25.0, true),
            (524.0, 550.0, 68.0, 30.0, false),
            (605.0, 548.0, 32.0, 32.0, false),
        ];
        DialogTemplate {
            bounds: Bounds {
                min: Point::new(60.0, -201.0),
                max: Point::new(678.0, 316.0),
            },
            placement: Placement::Center,
            items: items
                .into_iter()
                .map(|(x, y, w, h, enabled)| ItemTemplate {
                    bounds: rect(x, y, w, h),
                    enabled,
                    kind: ItemSpec::User,
                })
                .collect(),
        }
    }

    /// Where the dialog goes: (1024 - 618) / 2, (768 - 517) / 2 floored.
    const ORIGIN: Point = Point::new(203.0, 125.0);

    fn on_screen(x: f32, y: f32, w: f32, h: f32) -> Bounds {
        rect(ORIGIN.x + x, ORIGIN.y + y, w, h)
    }

    fn layout() -> (DialogTemplate, Rc<dyn TextMetrics>) {
        (template(), Rc::new(MonoMetrics))
    }

    pub(in crate::spaceport) fn view_of(catalog: &FakePort) -> SpaceportView {
        SpaceportView::new(catalog, StellarId(140), Ok(layout()))
    }

    /// Earth without the port dialog.
    pub(in crate::spaceport) fn without_its_dialog() -> SpaceportView {
        SpaceportView::new(&catalog(), StellarId(140), Err("no DLOG 1000".to_owned()))
    }

    fn earth() -> SpaceportView {
        view_of(&catalog())
    }

    pub(in crate::spaceport) fn drawn(view: &impl Screen) -> Vec<DrawCommand> {
        let mut list = DrawList::new();
        view.draw(&mut list);
        list.iter().cloned().collect()
    }

    fn texts(commands: &[DrawCommand]) -> Vec<String> {
        commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect()
    }

    pub(in crate::spaceport) fn key(key: Key) -> Input {
        Input::Key {
            key,
            pressed: true,
            repeat: false,
        }
    }

    pub(in crate::spaceport) fn click(view: &mut SpaceportView, at: Point) {
        for pressed in [true, false] {
            view.input(&Input::PointerButton {
                button: MouseButton::Left,
                pressed,
                at,
            });
        }
    }

    pub(in crate::spaceport) fn click_item(view: &mut SpaceportView, number: usize) {
        let at = item(view, number).center();
        click(view, at);
    }

    pub(in crate::spaceport) fn item(view: &SpaceportView, item: usize) -> Bounds {
        view.dialog()
            .expect("laid out")
            .item_bounds(item)
            .expect("an item")
    }

    #[test]
    fn it_reads_the_stellar_and_its_description_once() {
        let catalog = catalog();
        let view = view_of(&catalog);
        assert_eq!(*catalog.asked.borrow(), ["spöb 140", "dësc 140"]);
        assert_eq!(view.stellar(), StellarId(140));
        assert_eq!(
            view.offered(),
            [Service::TradeCenter, Service::Bar, Service::MissionBbs]
        );
        assert!(!view.left());
        assert_eq!(view.problem(), None);
        assert_eq!(view.missing_landscape(), None);
        assert_eq!(view.description_problem(), None);
        assert!(view.open_service().is_none());
        assert_eq!(view.dialog().expect("laid out").bounds().min, ORIGIN);
    }

    #[test]
    fn it_draws_the_background_the_landscape_the_name_then_the_dialog() {
        let view = earth();
        let commands = drawn(&view);
        let name_width = 5.0 * 7.2;
        assert_eq!(
            commands[..3],
            [
                DrawCommand::Picture {
                    image: ImageKey::picture(8500),
                    top_left: ORIGIN,
                },
                DrawCommand::StretchedPicture {
                    image: ImageKey::picture(10_003),
                    top_left: Point::new(206.0, 128.0),
                    width: 612.0,
                    height: 285.0,
                },
                DrawCommand::Text {
                    text: "Earth".to_owned(),
                    font: Font::Charcoal,
                    origin: Point::new(
                        203.0 + 159.0 + 151.5 - name_width / 2.0,
                        125.0 + 297.0 + 9.0 - 14.4 / 2.0
                    ),
                    size: 12.0,
                    wrap_width: None,
                    color: Color::WHITE,
                },
            ]
        );
        let mut dialog = DrawList::new();
        view.dialog().expect("laid out").draw(&mut dialog);
        assert_eq!(
            commands[3..],
            dialog.iter().cloned().collect::<Vec<_>>()[..]
        );
    }

    #[test]
    fn every_button_is_drawn_in_its_item_in_the_style_asked_for() {
        let view = earth();
        let commands = drawn(&view);
        assert!(!texts(&commands).contains(&"Done".to_owned()));
        // Each in its own item, with the button style asked for.
        let label_at = |text: &str| {
            commands.iter().find_map(|command| match command {
                DrawCommand::Text {
                    text: t,
                    origin,
                    color,
                    ..
                } if t == text => Some((*origin, *color)),
                _ => None,
            })
        };
        for (text, item_number) in [
            ("Trade Center", 7),
            ("Mission BBS", 10),
            ("Bar", 11),
            ("Recharge", 4),
            ("Leave", 12),
        ] {
            let (origin, color) = label_at(text).expect(text);
            assert!(item(&view, item_number).contains(origin), "{text}");
            assert_eq!(color, Color::BLACK, "{text}");
        }
        // Those Earth doesn't offer, greyed.
        for (text, item_number) in [("Outfitter", 8), ("Shipyard", 9)] {
            let (origin, color) = label_at(text).expect(text);
            assert!(item(&view, item_number).contains(origin), "{text}");
            assert_eq!(color, ButtonStyle::STOCK.grey, "{text}");
        }
        assert_eq!(item(&view, 12), on_screen(471.0, 456.0, 145.0, 25.0));
    }

    #[test]
    fn the_description_scrolls_in_its_box() {
        let view = earth();
        let text = view
            .dialog()
            .expect("laid out")
            .scroll_text()
            .expect("the description");
        assert_eq!(text.lines(), ["Blue and green.", "Home."]);
        let first = drawn(&view)
            .into_iter()
            .find(|c| matches!(c, DrawCommand::Text { text, .. } if text == "Blue and green."))
            .expect("drawn");
        let DrawCommand::Text {
            origin,
            font,
            size,
            color,
            ..
        } = first
        else {
            unreachable!()
        };
        assert_eq!(origin, on_screen(160.0, 327.0, 0.0, 0.0).min);
        assert_eq!((font, size, color), (Font::Geneva, 10.0, Color::WHITE));
    }

    #[test]
    fn the_description_draws_over_a_black_box_filling_its_item() {
        let view = earth();
        let commands = drawn(&view);
        let fill = commands
            .iter()
            .position(|command| *command == fill_command(item(&view, TEXT_ITEM), Color::BLACK))
            .expect("filled black");
        let text = commands
            .iter()
            .position(|c| matches!(c, DrawCommand::Text { text, .. } if text == "Blue and green."))
            .expect("the description");
        assert!(fill < text, "the box is under the text");
    }

    #[test]
    fn up_and_down_still_scroll_the_description() {
        let long: Vec<String> = (0..40).map(|n| format!("line {n}")).collect();
        let catalog = FakePort {
            description: Ok(long.join("\r")),
            ..catalog()
        };
        let mut view = view_of(&catalog);
        let first = |view: &SpaceportView| {
            view.dialog()
                .and_then(|dialog| dialog.text(TEXT_ITEM))
                .map(ScrollText::first)
        };
        view.input(&key(Key::Down));
        view.input(&key(Key::Down));
        assert_eq!(first(&view), Some(2));
        view.input(&key(Key::Up));
        assert_eq!(first(&view), Some(1));
    }

    #[test]
    fn leave_return_and_escape_leave() {
        let mut view = earth();
        click_item(&mut view, 12);
        assert!(view.left());
        for k in [Key::Enter, Key::Escape] {
            let mut view = earth();
            assert_eq!(view.input(&key(k)), ScreenAction::None);
            assert!(view.left(), "{k:?}");
        }
    }

    #[test]
    fn clicks_elsewhere_do_nothing() {
        let mut view = earth();
        // The shipyard's and outfitter's greyed buttons, the blank item,
        // the landscape and the description.
        for number in [9, 8, 13, 5, 6, 3] {
            click_item(&mut view, number);
        }
        click(&mut view, Point::new(5.0, 5.0));
        view.input(&Input::Key {
            key: Key::Escape,
            pressed: true,
            repeat: true,
        });
        assert!(!view.left());
        assert!(view.open_service().is_none());
        assert!(!view.take_recharge());
    }

    #[test]
    fn a_service_opens_its_placeholder_which_done_closes() {
        let mut view = earth();
        click_item(&mut view, 7);
        let open = view.open_service().expect("open");
        assert_eq!(open.service(), Service::TradeCenter);
        let mut expected = DrawList::new();
        open.draw(&mut expected);
        assert_eq!(drawn(&view), expected.iter().cloned().collect::<Vec<_>>());
        assert!(texts(&drawn(&view)).contains(&"Trade Center".to_owned()));
        assert!(!texts(&drawn(&view)).contains(&"Leave".to_owned()));
        // Escape closes the service, not the spaceport.
        view.input(&key(Key::Escape));
        assert!(view.open_service().is_none());
        assert!(!view.left());
        // Done, clicked.
        click_item(&mut view, 11);
        assert_eq!(
            view.open_service().map(ServiceScreen::service),
            Some(Service::Bar)
        );
        click(&mut view, super::super::service::DONE_BUTTON.center());
        assert!(view.open_service().is_none());
        assert!(!view.left());
        // Every service's button opens it.
        for (service, number) in SERVICE_ITEMS {
            let catalog = FakePort {
                port: Ok(PortRecord {
                    flags: 0xFF & !StellarFlags::UNINHABITED,
                    ..catalog().port.expect("a record")
                }),
                ..catalog()
            };
            let mut view = view_of(&catalog);
            click_item(&mut view, number);
            assert_eq!(
                view.open_service().map(ServiceScreen::service),
                Some(service)
            );
        }
    }

    #[test]
    fn an_open_service_takes_cancel_pointer() {
        let mut view = earth();
        click_item(&mut view, 7);
        let done = super::super::service::DONE_BUTTON.center();
        let button = |pressed| Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at: done,
        };
        view.input(&button(true));
        view.cancel_pointer();
        view.input(&button(false));
        assert!(view.open_service().is_some(), "the click was abandoned");
    }

    #[test]
    fn cancelling_the_pointer_abandons_a_click_on_leave() {
        let mut view = earth();
        let leave = item(&view, 12).center();
        let button = |pressed| Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at: leave,
        };
        view.input(&button(true));
        view.cancel_pointer();
        view.input(&button(false));
        assert!(!view.left());
    }

    #[test]
    fn a_missing_landscape_leaves_its_box_as_the_background_shows_it() {
        let catalog = FakePort {
            pictures: Vec::new(),
            ..catalog()
        };
        let view = view_of(&catalog);
        assert_eq!(view.missing_landscape(), Some(10_003));
        let commands = drawn(&view);
        assert!(matches!(commands[0], DrawCommand::Picture { .. }));
        assert!(
            !commands
                .iter()
                .any(|c| matches!(c, DrawCommand::StretchedPicture { image, .. } if *image == ImageKey::picture(10_003)))
        );
        assert!(matches!(&commands[1], DrawCommand::Text { text, .. } if text == "Earth"));
    }

    #[test]
    fn a_custom_landscape_is_drawn_when_named() {
        let catalog = FakePort {
            port: Ok(PortRecord {
                cust_pic_id: 9000,
                ..catalog().port.expect("a record")
            }),
            pictures: vec![9000],
            ..catalog()
        };
        let commands = drawn(&view_of(&catalog));
        assert!(
            matches!(commands[1], DrawCommand::StretchedPicture { image, .. } if image == ImageKey::picture(9000))
        );
    }

    #[test]
    fn a_missing_description_leaves_its_box_empty() {
        let catalog = FakePort {
            description: Err("no dësc 140".to_owned()),
            ..catalog()
        };
        let view = view_of(&catalog);
        assert_eq!(view.description_problem(), Some("no dësc 140"));
        let text = view
            .dialog()
            .expect("laid out")
            .scroll_text()
            .expect("a box");
        assert!(text.lines().is_empty());
        assert!(!texts(&drawn(&view)).iter().any(|t| t.contains("dësc")));
    }

    fn problem(reason: &str) -> DrawCommand {
        DrawCommand::Text {
            text: format!(
                "Cannot show the spaceport: {reason}. Press Return or Escape to take off."
            ),
            font: Font::Geneva,
            origin: PROBLEM_AT,
            size: PROBLEM_SIZE,
            wrap_width: None,
            color: Color::ERROR,
        }
    }

    #[test]
    fn without_the_dialog_it_says_why_and_return_or_escape_leaves() {
        for k in [Key::Enter, Key::Escape] {
            let mut view =
                SpaceportView::new(&catalog(), StellarId(140), Err("no DLOG 1000".to_owned()));
            assert_eq!(view.problem(), Some("no DLOG 1000"));
            assert_eq!(drawn(&view), [problem("no DLOG 1000")]);
            assert!(view.dialog().is_none());
            assert_eq!(view.offered(), []);
            assert_eq!(view.missing_landscape(), None);
            assert_eq!(view.description_problem(), None);
            for other in [
                Input::Key {
                    key: k,
                    pressed: true,
                    repeat: true,
                },
                Input::Key {
                    key: k,
                    pressed: false,
                    repeat: false,
                },
                key(Key::Space),
            ] {
                view.input(&other);
            }
            click(&mut view, Point::new(500.0, 500.0));
            view.cancel_pointer();
            assert!(!view.left());
            view.input(&key(k));
            assert!(view.left(), "{k:?}");
        }
        assert_eq!((PROBLEM_AT, PROBLEM_SIZE), (Point::new(16.0, 32.0), 20.0));
    }

    #[test]
    fn an_unreadable_stellar_says_why() {
        let catalog = FakePort {
            port: Err("no spöb 140".to_owned()),
            ..catalog()
        };
        let view = view_of(&catalog);
        assert_eq!(view.problem(), Some("no spöb 140"));
        assert_eq!(drawn(&view), [problem("no spöb 140")]);
    }

    #[test]
    fn nothing_moves_with_time() {
        let mut view = earth();
        let before = drawn(&view);
        view.tick(Duration::from_secs(5));
        assert_eq!(drawn(&view), before);
        assert!(format!("{view:?}").contains("TextMetrics"));
    }

    /// `_TrackPlanetButtons` @0x2996f plays nothing under the pointer.
    #[test]
    fn pointer_click_on_leave_is_silent_as_the_original() {
        let mut view = earth();
        let leave = item(&view, 12).center();
        let button = |pressed| Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at: leave,
        };
        view.input(&button(true));
        assert_eq!(view.take_sounds(), []);
        view.input(&button(false));
        assert!(view.left());
        assert_eq!(view.take_sounds(), []);
        assert_eq!(view.take_sounds(), []);
    }

    #[test]
    fn a_service_and_its_done_are_silent_under_the_pointer() {
        let mut view = earth();
        click_item(&mut view, 7);
        assert_eq!(view.take_sounds(), []);
        click(&mut view, super::super::service::DONE_BUTTON.center());
        assert!(view.open_service().is_none(), "closed");
        assert_eq!(view.take_sounds(), []);
        click_item(&mut view, 11);
        view.input(&key(Key::Escape));
        assert!(view.open_service().is_none());
        assert_eq!(view.take_sounds(), [], "Escape is silent");
    }

    #[test]
    fn keys_greyed_buttons_and_a_spaceport_that_cannot_be_shown_are_silent() {
        let mut view = earth();
        click_item(&mut view, 9);
        view.input(&key(Key::Enter));
        assert!(view.left());
        assert_eq!(view.take_sounds(), []);
        let mut broken = SpaceportView::new(&catalog(), StellarId(140), Err("no".to_owned()));
        click(&mut broken, Point::new(5.0, 5.0));
        broken.input(&key(Key::Escape));
        assert!(broken.left());
        assert_eq!(broken.take_sounds(), []);
    }

    /// `_PortFilter` @0x4f041 maps Return, Enter and Escape to Leave
    /// (@0x4f1e3-0x4f204) and the letters to their services (@0x4f209-0x4f257),
    /// all with no click.
    #[test]
    fn keyboard_activation_is_silent_as_the_original() {
        for k in [Key::Enter, Key::Escape, MISSION_INFO_KEY, PLAYER_INFO_KEY]
            .into_iter()
            .chain(('a'..='z').map(Key::Char))
        {
            let mut view = earth();
            view.take_sounds();
            view.input(&key(k));
            view.input(&Input::Key {
                key: k,
                pressed: false,
                repeat: false,
            });
            assert_eq!(view.take_sounds(), [], "{k:?}");
        }
    }

    // The letter keys.

    /// A stellar offering every service: every flag but uninhabited's.
    fn everything() -> FakePort {
        flagged(0xFF & !StellarFlags::UNINHABITED)
    }

    /// A stellar with `flags`, otherwise Earth.
    pub(in crate::spaceport) fn flagged(flags: u32) -> FakePort {
        FakePort {
            port: Ok(PortRecord {
                flags,
                ..catalog().port.expect("a record")
            }),
            ..catalog()
        }
    }

    /// The eight letters `_PortFilter` reads (@0x4f209-0x4f257).
    const LETTERS: [char; 8] = ['r', 'f', 'c', 't', 'o', 's', 'n', 'b'];

    /// `_PortFilter` @0x4f209-0x4f257: C or T the Trade Center, O the
    /// Outfitter, S the Shipyard, N the Mission BBS and B the Bar, each
    /// opening what its button opens (`_DoPortDialog` @0x5fe63-0x6011b),
    /// with no sound.
    #[test]
    fn each_letter_opens_its_service_as_its_button_does() {
        use Service::{Bar, MissionBbs, Outfitter, Shipyard, TradeCenter};
        for (letter, service) in [
            ('c', TradeCenter),
            ('t', TradeCenter),
            ('o', Outfitter),
            ('s', Shipyard),
            ('n', MissionBbs),
            ('b', Bar),
        ] {
            let mut clicked = view_of(&everything());
            click_item(&mut clicked, service_item(service));
            let mut view = view_of(&everything());
            view.take_sounds();
            view.input(&key(Key::Char(letter)));
            assert_eq!(
                view.open_service().map(ServiceScreen::service),
                Some(service),
                "{letter}"
            );
            assert_eq!(drawn(&view), drawn(&clicked), "{letter}");
            assert_eq!(view.take_sounds(), [], "{letter}");
            assert!(!view.left() && !view.take_recharge(), "{letter}");
        }
    }

    #[test]
    fn n_asks_for_the_mission_computer_and_b_opens_the_bar() {
        let mut view = missioned();
        view.input(&key(Key::Char('n')));
        assert!(view.open_service().is_none(), "no placeholder");
        assert!(view.take_mission_computer_request());
        let mut view = drinking(true);
        view.take_sounds();
        view.input(&key(Key::Char('b')));
        assert!(view.open_bar().is_some());
        assert_eq!(view.take_sounds(), []);
    }

    /// R or F → item 4 (`_PortFilter` @0x4f209), whose answer fills the
    /// tank (`_DoPortDialog` @0x5fd5d).
    #[test]
    fn r_and_f_ask_for_a_recharge_as_its_button_does() {
        for letter in ['r', 'f'] {
            let mut view = earth();
            view.input(&key(Key::Char(letter)));
            assert!(view.take_recharge(), "{letter}");
            assert!(!view.take_recharge(), "once");
            view.input(&key(Key::Char(letter)));
            assert!(view.take_recharge(), "{letter}: again");
            assert_eq!(box_lines(&view), DESCRIPTION, "{letter}: nothing said");
            assert!(view.open_service().is_none());
            assert!(!view.left());
            assert_eq!(view.take_sounds(), [], "{letter}");
        }
    }

    /// `_DoPortDialog`'s answers gate each item on the stellar's flags
    /// (@0x5fd6c, @0x5fe71, @0x5feb5, @0x5ff2c, @0x5ffb7, @0x60115), and
    /// a key meets no other gate, so a letter for a service the stellar
    /// doesn't offer does nothing, as a click on its greyed button does.
    #[test]
    fn a_letter_for_a_service_not_offered_does_nothing() {
        let mut bare = view_of(&flagged(StellarFlags::CAN_LAND));
        for letter in ['o', 's', 'b', 'c', 't'] {
            bare.input(&key(Key::Char(letter)));
            assert!(bare.open_service().is_none(), "{letter}");
            assert_eq!(box_lines(&bare), DESCRIPTION, "{letter}");
        }
        assert_eq!(bare.take_sounds(), []);
        let uninhabited = || flagged(StellarFlags::CAN_LAND | StellarFlags::UNINHABITED);
        let mut view = view_of(&uninhabited());
        for letter in LETTERS {
            view.input(&key(Key::Char(letter)));
            assert!(view.open_service().is_none(), "{letter}");
        }
        assert!(!view.take_recharge(), "no fuel sold");
        assert!(!view.left());
        assert_eq!(view.take_sounds(), []);
        let mut view = view_of(&uninhabited()).with_missions(
            Ok(computer_template()),
            Ok(info_template()),
            plain_offers(),
            plain_texts(),
        );
        view.input(&key(Key::Char('n')));
        assert!(!view.take_mission_computer_request(), "no missions");
    }

    /// `_PortFilter` acts on a `keyDown` alone (@0x4f15b): an `autoKey`
    /// and a key-up do nothing.
    #[test]
    fn a_letters_repeat_and_release_do_nothing() {
        for letter in LETTERS {
            let mut view = view_of(&everything());
            for (pressed, repeat) in [(true, true), (false, false)] {
                view.input(&Input::Key {
                    key: Key::Char(letter),
                    pressed,
                    repeat,
                });
            }
            assert!(view.open_service().is_none(), "{letter}");
            assert!(!view.take_recharge(), "{letter}");
            assert!(!view.left(), "{letter}");
        }
    }

    /// The filter only runs while the spaceport is in front: a letter
    /// over an open service goes to it.
    #[test]
    fn a_letter_goes_to_whatever_is_open_over_the_spaceport() {
        let mut view = view_of(&everything());
        click_item(&mut view, service_item(Service::TradeCenter));
        for letter in LETTERS {
            view.input(&key(Key::Char(letter)));
        }
        assert_eq!(
            view.open_service().map(ServiceScreen::service),
            Some(Service::TradeCenter)
        );
        assert!(!view.take_recharge());
        let mut broken = SpaceportView::new(&everything(), StellarId(140), Err("no".to_owned()));
        for letter in LETTERS {
            broken.input(&key(Key::Char(letter)));
        }
        assert!(broken.open_service().is_none());
        assert!(!broken.take_recharge() && !broken.left());
    }

    /// The map, player info and mission info keys come first
    /// (@0x4f165-0x4f1de), and none of them is a letter.
    #[test]
    fn the_other_keys_still_win() {
        use crate::spaceport::PORT_KEYS;
        let others = [
            PLAYER_INFO_KEY,
            MISSION_INFO_KEY,
            crate::flight::view::MAP_KEY,
            Key::Enter,
            Key::Escape,
        ];
        for (letter, _) in PORT_KEYS {
            assert!(!others.contains(&letter), "{letter:?}");
        }
        let mut view = view_of(&everything()).with_missions(
            Ok(computer_template()),
            Ok(info_template()),
            plain_offers(),
            plain_texts(),
        );
        view.input(&key(PLAYER_INFO_KEY));
        assert!(view.take_player_info_request());
        view.input(&key(MISSION_INFO_KEY));
        assert!(view.take_mission_info_request());
        assert!(view.open_service().is_none());
    }

    // Recharging.

    use crate::spaceport::layout::RECHARGE_ITEM;

    /// The lines in the description box.
    fn box_lines(view: &SpaceportView) -> Vec<String> {
        view.dialog()
            .expect("laid out")
            .scroll_text()
            .expect("the box")
            .lines()
            .to_vec()
    }

    /// The first line the description box shows.
    fn view_first_line(view: &SpaceportView) -> usize {
        view.dialog()
            .expect("laid out")
            .scroll_text()
            .expect("the box")
            .first()
    }

    const DESCRIPTION: [&str; 2] = ["Blue and green.", "Home."];

    #[test]
    fn recharge_is_asked_for_once_per_click() {
        let mut view = earth();
        assert!(!view.take_recharge(), "nothing clicked");
        click_item(&mut view, RECHARGE_ITEM);
        assert!(view.take_recharge());
        assert!(!view.take_recharge(), "once");
        assert!(!view.left());
        assert!(view.open_service().is_none());
        assert_eq!(view.take_sounds(), []);
    }

    #[test]
    fn the_description_is_asked_to_load_and_shows_loaded() {
        let mut view = earth();
        assert_eq!(view.take_description_request(), Some(140));
        assert_eq!(view.take_description_request(), None, "once");
        view.show_description(140, "Loaded port.");
        assert_eq!(box_lines(&view), ["Loaded port."]);
        click_item(&mut view, RECHARGE_ITEM);
        view.show_description(140, "Loaded again.");
        assert_eq!(box_lines(&view), ["Loaded again."]);
        view.show_description(141, "Another.");
        assert_eq!(box_lines(&view), ["Loaded again."], "not the port's");
    }

    #[test]
    fn recharge_keeps_the_description_where_it_is() {
        let catalog = FakePort {
            description: Ok((0..40)
                .map(|n| format!("l{n}"))
                .collect::<Vec<_>>()
                .join("\r")),
            ..catalog()
        };
        let mut view = view_of(&catalog);
        view.input(&key(Key::Down));
        click_item(&mut view, RECHARGE_ITEM);
        view.set_recharge_enabled(false);
        assert!(view.take_recharge());
        assert_eq!(view_first_line(&view), 1, "not scrolled back");
    }

    #[test]
    fn a_spaceport_that_cannot_be_shown_takes_recharge_news_quietly() {
        let mut broken = SpaceportView::new(&catalog(), StellarId(140), Err("no".to_owned()));
        broken.set_recharge_enabled(true);
        broken.set_recharge_enabled(false);
        assert!(!broken.take_recharge());
        assert_eq!(broken.problem(), Some("no"));
    }

    // The Trade Center.

    use crate::spaceport::trade::{BUY_ITEM, TradeScreen};
    use crate::ui::dialog::DialogTemplate as Template;
    use nova_sim::{Direction, Good, Lot, Market, MarketRow, Order};

    /// "Trade": 426 x 252, centred, with Done (1), eight rows (4-11), Buy
    /// (13) and Sell (14).
    fn trade_template() -> Template {
        let mut items: Vec<ItemTemplate> = (0..15)
            .map(|_| ItemTemplate {
                bounds: rect(0.0, 0.0, 1.0, 1.0),
                enabled: false,
                kind: ItemSpec::User,
            })
            .collect();
        let mut place = |number: usize, x, y, w, h| {
            items[number - 1] = ItemTemplate {
                bounds: rect(x, y, w, h),
                enabled: true,
                kind: ItemSpec::User,
            };
        };
        place(1, 272.0, 221.0, 99.0, 25.0);
        for row in 0..8_u8 {
            place(
                4 + usize::from(row),
                38.0,
                25.0 + 12.0 * f32::from(row),
                352.0,
                12.0,
            );
        }
        place(13, 60.0, 221.0, 99.0, 25.0);
        place(14, 166.0, 221.0, 99.0, 25.0);
        Template {
            bounds: rect(32.0, 35.0, 426.0, 252.0),
            placement: Placement::Center,
            items,
        }
    }

    /// Food at 75, none held, with 1000 credits and 10 tons free.
    fn exchange(held: u32) -> Market {
        Market {
            rows: vec![MarketRow {
                good: Good::Commodity(0),
                name: "Food".to_owned(),
                price: 75,
                held,
                sold_here: true,
                bought_here: true,
                slot: 0,
                marker: None,
            }],
            events: Vec::new(),
            cash: 1000,
            capacity: 10,
            free: 10,
            ..Market::default()
        }
    }

    /// The quantity dialog, fixed at (5, 6): OK (1), the prompt (2), the
    /// field (3) and Cancel (4).
    fn quantity_template() -> Template {
        let item = |x, kind| ItemTemplate {
            bounds: rect(x, 8.0, 40.0, 16.0),
            enabled: true,
            kind,
        };
        Template {
            bounds: rect(5.0, 6.0, 172.0, 72.0),
            placement: Placement::Fixed,
            items: vec![
                item(0.0, ItemSpec::Button("OK".into())),
                item(40.0, ItemSpec::StaticText(String::new())),
                item(80.0, ItemSpec::EditText(String::new())),
                item(120.0, ItemSpec::Button("Cancel".into())),
            ],
        }
    }

    fn trading(template: Result<Template, String>) -> SpaceportView {
        earth().with_trade(template, Ok(quantity_template()), exchange(0), COLORS)
    }

    /// The list colours the spaceport's exchange is given.
    const COLORS: ListColors = ListColors {
        text: Color::rgba(1, 2, 3, 255),
        background: Color::rgba(4, 5, 6, 255),
        hilite: Color::rgba(7, 8, 9, 255),
    };

    fn trade_item(view: &SpaceportView, number: usize) -> Point {
        view.open_trade()
            .expect("trading")
            .dialog()
            .expect("laid out")
            .item_bounds(number)
            .expect("an item")
            .center()
    }

    #[test]
    fn with_an_exchange_the_trade_center_opens_it() {
        let mut view = trading(Ok(trade_template()));
        assert!(view.open_trade().is_none());
        click_item(&mut view, 7);
        let open = view.open_trade().expect("trading");
        assert_eq!(open.market(), &exchange(0));
        assert_eq!(open.colors(), COLORS, "in the colours given");
        assert!(view.open_service().is_none());
        // Drawn over the spaceport, as the original's dialog is.
        let mut expected: Vec<DrawCommand> = drawn(&earth());
        let mut trade = DrawList::new();
        open.draw(&mut trade);
        expected.extend(trade.iter().cloned());
        assert_eq!(drawn(&view), expected);
        assert!(texts(&drawn(&view)).contains(&"Food".to_owned()));
        // Escape closes the exchange, not the spaceport.
        view.input(&key(Key::Escape));
        assert!(view.open_trade().is_none());
        assert!(!view.left());
        // The other services still open their placeholders.
        click_item(&mut view, 11);
        assert_eq!(
            view.open_service().map(ServiceScreen::service),
            Some(Service::Bar)
        );
        assert!(view.open_trade().is_none());
    }

    #[test]
    fn the_focus_ring_reaches_the_spaceport_and_each_screen_opened_over_it() {
        let ringed = trading(Ok(trade_template()));
        assert_eq!(
            ringed.dialog().map(Dialog::focus_ring),
            Some(FocusRing::Tab),
            "by default"
        );
        let mut off = trading(Ok(trade_template())).with_focus_ring(FocusRing::Off);
        off.input(&key(Key::Tab));
        assert_eq!(off.dialog().and_then(Dialog::focus), None);
        click_item(&mut off, 7);
        assert_eq!(
            off.open_trade()
                .and_then(TradeScreen::dialog)
                .map(Dialog::focus_ring),
            Some(FocusRing::Off)
        );
    }

    #[test]
    fn the_exchange_is_laid_out_by_its_own_dialog_or_says_why_not() {
        let mut view = trading(Err("no DLOG 1001".to_owned()));
        click_item(&mut view, 7);
        assert_eq!(
            view.open_trade().and_then(TradeScreen::problem),
            Some("no DLOG 1001")
        );
        view.input(&key(Key::Enter));
        assert!(view.open_trade().is_none());
        assert!(!view.left());
    }

    #[test]
    fn the_exchanges_orders_are_taken_through_the_spaceport_and_its_market_set() {
        let mut view = trading(Ok(trade_template()));
        assert_eq!(view.take_trade(), None, "nothing open");
        view.set_market(exchange(4));
        click_item(&mut view, 7);
        assert_eq!(
            view.open_trade().map(|open| open.market().rows[0].held),
            Some(4),
            "it opens on the latest"
        );
        view.input(&key(Key::Char('b')));
        assert_eq!(
            view.take_trade(),
            Some(Order {
                row: 0,
                good: Good::Commodity(0),
                direction: Direction::Buy,
                lot: Lot::Click,
            })
        );
        assert_eq!(view.take_trade(), None, "once");
        view.set_market(exchange(5));
        assert_eq!(
            view.open_trade().map(|open| open.market().rows[0].held),
            Some(5)
        );
        view.input(&key(Key::Escape));
        click_item(&mut view, 7);
        assert_eq!(
            view.open_trade().map(|open| open.market().rows[0].held),
            Some(5),
            "and reopens on it"
        );
    }

    #[test]
    fn the_exchange_takes_cancel_pointer() {
        let mut view = trading(Ok(trade_template()));
        click_item(&mut view, 7);
        assert_eq!(view.take_sounds(), []);
        let buy = trade_item(&view, BUY_ITEM);
        let button = |pressed| Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at: buy,
        };
        view.input(&button(true));
        view.cancel_pointer();
        view.input(&button(false));
        assert_eq!(view.take_trade(), None, "the click was abandoned");
        assert_eq!(view.take_sounds(), []);
        let done = trade_item(&view, 1);
        click(&mut view, done);
        assert!(view.open_trade().is_none(), "closed");
        assert_eq!(view.take_sounds(), []);
    }

    #[test]
    fn letting_go_of_the_keys_reaches_the_exchange() {
        let mut view = trading(Ok(trade_template()));
        click_item(&mut view, 7);
        view.input(&key(Key::Alt));
        view.release_keys();
        view.input(&key(Key::Char('b')));
        assert_eq!(view.take_trade().map(|order| order.lot), Some(Lot::Click));
        view.input(&key(Key::Alt));
        view.input(&key(Key::Char('b')));
        assert_eq!(view.take_trade(), None, "the quantity dialog asks");
    }

    #[test]
    fn the_exchanges_quantity_dialog_is_laid_out_by_its_template() {
        let mut view = trading(Ok(trade_template()));
        click_item(&mut view, 7);
        view.input(&key(Key::Alt));
        view.input(&key(Key::Char('b')));
        let asking = view
            .open_trade()
            .and_then(TradeScreen::quantity)
            .expect("asking");
        assert_eq!(asking.dialog().bounds().min, Point::new(5.0, 6.0));
        assert_eq!(asking.field().text(), "10", "min(1000 / 75, 10 free)");
        view.input(&key(Key::Enter));
        assert_eq!(
            view.take_trade().map(|order| order.lot),
            Some(Lot::Count(10))
        );
    }

    // The Outfitter.

    use std::collections::BTreeMap;

    use crate::spaceport::outfitter::{NEGATIVE_FREE_MASS, OutfitterCatalog, OutfitterScreen};
    use crate::ui::desc::DescDialog;
    use nova_sim::{
        OutfitFacts, OutfitId, OutfitOrder, OutfitRefusal, OutfitRow, Outfitter, SaleAlert,
    };

    /// "Outfit": 765 x 321, centred, with Done (1), Sell (4), the grid
    /// (5), the description (6), Buy (7), the picture (8) and the info box
    /// (9).
    fn outfit_template() -> Template {
        let mut items: Vec<ItemTemplate> = (0..11)
            .map(|_| ItemTemplate {
                bounds: rect(0.0, 0.0, 1.0, 1.0),
                enabled: false,
                kind: ItemSpec::User,
            })
            .collect();
        let mut place = |number: usize, x, y, w, h| {
            items[number - 1] = ItemTemplate {
                bounds: rect(x, y, w, h),
                enabled: true,
                kind: ItemSpec::User,
            };
        };
        place(1, 500.0, 289.0, 99.0, 25.0);
        place(4, 394.0, 289.0, 99.0, 25.0);
        place(5, 9.0, 8.0, 333.0, 271.0);
        place(6, 354.0, 10.0, 192.0, 267.0);
        place(7, 288.0, 289.0, 99.0, 25.0);
        place(8, 557.0, 8.0, 200.0, 200.0);
        place(9, 618.0, 214.0, 135.0, 100.0);
        Template {
            bounds: rect(100.0, 100.0, 765.0, 321.0),
            placement: Placement::Center,
            items,
        }
    }

    /// A fuel tank, `owned` owned, with 5000 credits and 8 tons free.
    pub(in crate::spaceport) fn tanks(owned: u16) -> Outfitter {
        Outfitter {
            rows: vec![OutfitRow {
                id: OutfitId(200),
                name: "Fuel Tank".to_owned(),
                short_name: "Fuel Tank".to_owned(),
                price: 1000,
                buy_price: 1000,
                mass: 1,
                owned,
                max: 10,
                facts: OutfitFacts {
                    listed: true,
                    buy_tech: true,
                    required: true,
                    available: true,
                    sold_here: true,
                    ..OutfitFacts::default()
                },
            }],
            cash: 5000,
            free_mass: 8,
            ..Outfitter::default()
        }
    }

    /// Earth with an outfitter too.
    fn outfitting_port() -> FakePort {
        FakePort {
            port: Ok(PortRecord {
                flags: FLAGS | StellarFlags::OUTFITTER,
                ..catalog().port.expect("a record")
            }),
            ..catalog()
        }
    }

    fn outfitting(template: Result<Template, String>) -> SpaceportView {
        let art: Rc<dyn OutfitterCatalog> = Rc::new(outfitting_port());
        view_of(&outfitting_port()).with_outfitter(
            template,
            Ok(quantity_template()),
            Ok(desc_template()),
            tanks(0),
            art,
        )
    }

    fn outfit_item(view: &SpaceportView, number: usize) -> Point {
        view.open_outfitter()
            .expect("outfitting")
            .dialog()
            .expect("laid out")
            .item_bounds(number)
            .expect("an item")
            .center()
    }

    /// Clicks the open outfitter's cell `index`, as a player picks an
    /// outfit: it opens with none selected.
    fn click_outfit_cell(view: &mut SpaceportView, index: usize) {
        let at = view
            .open_outfitter()
            .expect("outfitting")
            .cell_bounds(index)
            .expect("a cell")
            .center();
        click(view, at);
    }

    #[test]
    fn with_an_outfitter_its_button_opens_it_over_the_spaceport() {
        let mut view = outfitting(Ok(outfit_template()));
        assert!(view.open_outfitter().is_none());
        click_item(&mut view, 8);
        let open = view.open_outfitter().expect("outfitting");
        assert_eq!(open.outfitter(), &tanks(0));
        assert!(view.open_service().is_none());
        assert!(view.open_trade().is_none());
        let mut expected: Vec<DrawCommand> = drawn(&view_of(&outfitting_port()));
        let mut outfitter = DrawList::new();
        open.draw(&mut outfitter);
        expected.extend(outfitter.iter().cloned());
        assert_eq!(drawn(&view), expected);
        assert!(texts(&drawn(&view)).contains(&"Fuel Tank".to_owned()));
        // Escape closes the outfitter, not the spaceport.
        view.input(&key(Key::Escape));
        assert!(view.open_outfitter().is_none());
        assert!(!view.left());
        // The other services keep their placeholders.
        click_item(&mut view, 11);
        assert_eq!(
            view.open_service().map(ServiceScreen::service),
            Some(Service::Bar)
        );
        assert!(view.open_outfitter().is_none());
    }

    #[test]
    fn opening_the_outfitter_is_reported_once_each_time() {
        let mut view = outfitting(Ok(outfit_template()));
        assert!(!view.take_outfitter_opened(), "nothing open");
        click_item(&mut view, 8);
        assert!(view.take_outfitter_opened());
        assert!(!view.take_outfitter_opened(), "once");
        view.input(&key(Key::Escape));
        assert!(!view.take_outfitter_opened(), "closing is not opening");
        click_item(&mut view, 11);
        assert!(!view.take_outfitter_opened(), "the bar is not it");
        view.input(&key(Key::Escape));
        click_item(&mut view, 8);
        assert!(view.take_outfitter_opened(), "opened again");
        // The placeholder outfitter has no flags to clear.
        let mut view = view_of(&outfitting_port());
        click_item(&mut view, 8);
        assert!(!view.take_outfitter_opened());
    }

    #[test]
    fn closing_the_outfitter_is_reported_once_each_time() {
        let mut view = outfitting(Ok(outfit_template()));
        assert!(!view.take_outfitter_closed(), "nothing open");
        click_item(&mut view, 8);
        assert!(!view.take_outfitter_closed(), "opening is not closing");
        view.input(&key(Key::Escape));
        assert!(view.take_outfitter_closed());
        assert!(!view.take_outfitter_closed(), "once");
        click_item(&mut view, 11);
        view.input(&key(Key::Escape));
        assert!(!view.take_outfitter_closed(), "the bar is not it");
        click_item(&mut view, 8);
        let done = outfit_item(&view, 1);
        click(&mut view, done);
        assert!(view.take_outfitter_closed(), "Done");
    }

    #[test]
    fn without_an_outfitter_given_its_button_opens_the_placeholder() {
        let mut view = view_of(&outfitting_port());
        click_item(&mut view, 8);
        assert_eq!(
            view.open_service().map(ServiceScreen::service),
            Some(Service::Outfitter)
        );
        assert!(view.open_outfitter().is_none());
        assert_eq!(view.take_outfit(), None);
    }

    #[test]
    fn the_outfitter_is_laid_out_by_its_own_dialog_or_says_why_not() {
        let mut view = outfitting(Err("no DLOG 1002".to_owned()));
        click_item(&mut view, 8);
        assert_eq!(
            view.open_outfitter().and_then(OutfitterScreen::problem),
            Some("no DLOG 1002")
        );
        view.input(&key(Key::Enter));
        assert!(view.open_outfitter().is_none());
        assert!(!view.left());
    }

    #[test]
    fn the_outfitters_orders_are_taken_through_the_spaceport_and_its_state_set() {
        let mut view = outfitting(Ok(outfit_template()));
        assert_eq!(view.take_outfit(), None, "nothing open");
        view.set_outfitter(tanks(4));
        click_item(&mut view, 8);
        assert_eq!(
            view.open_outfitter()
                .map(|open| open.outfitter().rows[0].owned),
            Some(4),
            "it opens on the latest"
        );
        click_outfit_cell(&mut view, 0);
        view.input(&key(Key::Char('b')));
        assert_eq!(
            view.take_outfit(),
            Some((
                OutfitOrder {
                    outfit: OutfitId(200),
                    direction: Direction::Buy,
                },
                1
            ))
        );
        assert_eq!(view.take_outfit(), None, "once");
        assert_eq!(view.take_trade(), None, "not a trade");
        view.set_outfitter(tanks(5));
        assert_eq!(
            view.open_outfitter()
                .map(|open| open.outfitter().rows[0].owned),
            Some(5)
        );
        view.input(&key(Key::Escape));
        click_item(&mut view, 8);
        assert_eq!(
            view.open_outfitter()
                .map(|open| open.outfitter().rows[0].owned),
            Some(5),
            "and reopens on it"
        );
    }

    #[test]
    fn the_outfitters_quantity_dialog_is_laid_out_by_its_template() {
        let mut view = outfitting(Ok(outfit_template()));
        click_item(&mut view, 8);
        click_outfit_cell(&mut view, 0);
        view.input(&key(Key::Alt));
        let buy = outfit_item(&view, 7);
        click(&mut view, buy);
        let open = view.open_outfitter().expect("outfitting");
        let asking = open.quantity().expect("asking");
        assert_eq!(asking.field().text(), "5", "5000 / 1000");
        assert_eq!(
            asking.dialog().bounds(),
            quantity_template().bounds,
            "laid out by its template"
        );
        view.input(&key(Key::Enter));
        assert_eq!(
            view.take_outfit(),
            Some((
                OutfitOrder {
                    outfit: OutfitId(200),
                    direction: Direction::Buy,
                },
                5
            ))
        );
    }

    /// "Desc Dialog", fixed at (0, 0): Done (1), a parked picture (2) and
    /// a text box (3).
    fn desc_template() -> Template {
        let item = |bounds, kind| ItemTemplate {
            bounds,
            enabled: true,
            kind,
        };
        Template {
            bounds: rect(0.0, 0.0, 200.0, 120.0),
            placement: Placement::Fixed,
            items: vec![
                item(rect(100.0, 90.0, 99.0, 25.0), ItemSpec::User),
                item(rect(10.0, 150.0, 20.0, 20.0), ItemSpec::Picture(1431)),
                item(rect(10.0, 10.0, 2000.0, 60.0), ItemSpec::User),
            ],
        }
    }

    fn alert() -> SaleAlert {
        SaleAlert {
            outfit: OutfitId(200),
            refusal: OutfitRefusal::NegativeFreeMass,
            names: BTreeMap::new(),
            plural: false,
        }
    }

    #[test]
    fn an_alert_opens_over_the_open_outfitter_only() {
        let mut view = outfitting(Ok(outfit_template()));
        view.alert_outfit(&alert());
        assert!(view.open_outfitter().is_none(), "nothing open");
        click_item(&mut view, 8);
        view.alert_outfit(&alert());
        let open = view.open_outfitter().expect("outfitting");
        assert_eq!(
            open.alert_dialog().map(DescDialog::lines),
            Some([NEGATIVE_FREE_MASS.to_owned()].as_slice())
        );
        view.input(&key(Key::Escape));
        let open = view.open_outfitter().expect("still outfitting");
        assert!(open.alert_dialog().is_none(), "Escape closed the alert");
    }

    #[test]
    fn an_order_that_leaves_nothing_listed_closes_the_outfitter() {
        let mut view = outfitting(Ok(outfit_template()));
        view.outfit_ordered(Reselection::Leave);
        assert!(view.open_outfitter().is_none(), "nothing open");
        click_item(&mut view, 8);
        view.outfit_ordered(Reselection::Reset(OutfitId(128)));
        let open = view.open_outfitter().expect("still outfitting");
        assert_eq!(open.selected(), None);
        assert_eq!(open.selected_outfit(), Some(OutfitId(128)));
        assert!(!view.take_outfitter_closed(), "a reselection keeps it open");
        view.outfit_ordered(Reselection::Leave);
        assert!(view.open_outfitter().is_none(), "closed at once");
        assert!(view.take_outfitter_closed(), "reported closed (@0x5da33)");
        assert!(!view.take_outfitter_closed(), "once");
        assert!(!view.left());
        click_item(&mut view, 8);
        view.alert_outfit(&alert());
        view.outfit_ordered(Reselection::Leave);
        assert!(view.open_outfitter().is_some(), "its alert first");
        assert!(!view.take_outfitter_closed());
        view.input(&key(Key::Escape));
        assert!(view.open_outfitter().is_none(), "closed with its alert");
        assert!(
            view.take_outfitter_closed(),
            "reported closed with its alert"
        );
        assert!(!view.left());
    }

    #[test]
    fn a_port_dialog_closing_is_told_once() {
        // The outfitter, by Done, then by an order leaving nothing listed.
        let mut view = outfitting(Ok(outfit_template()));
        assert_eq!(view.take_port_dialog_closed(), None, "nothing open");
        click_item(&mut view, 8);
        assert_eq!(view.take_port_dialog_closed(), None, "opening");
        view.outfit_ordered(Reselection::Kept);
        assert_eq!(view.take_port_dialog_closed(), None, "still open");
        let done = outfit_item(&view, 1);
        click(&mut view, done);
        assert!(view.open_outfitter().is_none(), "Done");
        assert_eq!(
            view.take_port_dialog_closed(),
            Some(PortDialog::Outfitter),
            "the outfitter closed"
        );
        assert_eq!(view.take_port_dialog_closed(), None, "once");
        click_item(&mut view, 8);
        view.outfit_ordered(Reselection::Leave);
        assert!(view.open_outfitter().is_none());
        assert_eq!(
            view.take_port_dialog_closed(),
            Some(PortDialog::Outfitter),
            "closed by its order"
        );
        assert_eq!(view.take_port_dialog_closed(), None, "once");
        // The shipyard and the exchange.
        let mut view = shipbuying(Ok(shipyard_template()));
        click_item(&mut view, 9);
        view.input(&key(Key::Down));
        assert_eq!(view.take_port_dialog_closed(), None, "still open");
        view.input(&key(Key::Escape));
        assert!(view.open_shipyard().is_none());
        assert_eq!(
            view.take_port_dialog_closed(),
            Some(PortDialog::Shipyard),
            "the shipyard closed"
        );
        let mut view = trading(Ok(trade_template()));
        click_item(&mut view, 7);
        view.input(&key(Key::Escape));
        assert!(view.open_trade().is_none());
        assert_eq!(
            view.take_port_dialog_closed(),
            Some(PortDialog::Exchange),
            "the exchange closed"
        );
        // The mission computer is one; the bar and a placeholder are not.
        let mut view = drinking(true);
        click_item(&mut view, 11);
        view.input(&key(Key::Escape));
        assert!(view.open_bar().is_none());
        assert_eq!(view.take_port_dialog_closed(), None, "the bar");
        let mut view = missioned();
        click_item(&mut view, 10);
        view.open_mission_computer(Ok(offers()));
        view.input(&key(Key::Escape));
        assert!(view.mission_computer().is_none());
        assert_eq!(
            view.take_port_dialog_closed(),
            Some(PortDialog::MissionComputer),
            "the mission computer"
        );
        let mut view = earth();
        click_item(&mut view, 7);
        view.input(&key(Key::Escape));
        assert!(view.open_service().is_none());
        assert_eq!(view.take_port_dialog_closed(), None, "a placeholder");
    }

    #[test]
    fn each_port_dialog_tells_its_opening_once() {
        type Shop = fn() -> SpaceportView;
        let shops: [(Shop, usize, PortDialog); 3] = [
            (
                || outfitting(Ok(outfit_template())),
                8,
                PortDialog::Outfitter,
            ),
            (
                || shipbuying(Ok(shipyard_template())),
                9,
                PortDialog::Shipyard,
            ),
            (|| trading(Ok(trade_template())), 7, PortDialog::Exchange),
        ];
        for (shop, item, dialog) in shops {
            let mut view = shop();
            assert_eq!(view.take_port_dialog_opened(), None, "nothing open");
            click_item(&mut view, item);
            assert_eq!(view.take_port_dialog_opened(), Some(dialog));
            assert_eq!(view.take_port_dialog_opened(), None, "once");
            view.input(&key(Key::Escape));
            assert_eq!(view.take_port_dialog_opened(), None, "closing");
        }
        let mut view = missioned();
        click_item(&mut view, 10);
        assert_eq!(view.take_port_dialog_opened(), None, "only asked for");
        view.open_mission_computer(Ok(offers()));
        assert_eq!(
            view.take_port_dialog_opened(),
            Some(PortDialog::MissionComputer)
        );
        assert_eq!(view.take_port_dialog_opened(), None, "once");
        let mut view = drinking(true);
        click_item(&mut view, 11);
        assert_eq!(view.take_port_dialog_opened(), None, "the bar");
        let mut view = earth();
        click_item(&mut view, 7);
        assert_eq!(view.take_port_dialog_opened(), None, "a placeholder");
    }

    #[test]
    fn the_outfitter_takes_cancel_pointer() {
        let mut view = outfitting(Ok(outfit_template()));
        click_item(&mut view, 8);
        assert_eq!(view.take_sounds(), []);
        let buy = outfit_item(&view, 7);
        let button = |pressed| Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at: buy,
        };
        view.input(&button(true));
        view.cancel_pointer();
        view.input(&button(false));
        assert_eq!(view.take_outfit(), None, "the click was abandoned");
        assert_eq!(view.take_sounds(), []);
        let done = outfit_item(&view, 1);
        click(&mut view, done);
        assert!(view.open_outfitter().is_none(), "closed");
        assert_eq!(view.take_sounds(), []);
        let debug = format!("{view:?}");
        assert!(debug.contains("Outfitting"), "{debug}");
        assert!(debug.contains("Fuel Tank"), "{debug}");
    }

    // The Shipyard.

    use crate::spaceport::shipyard::{ShipBaseImages, ShipyardCatalog, ShipyardScreen};
    use nova_sim::{Rulebook, ShipFacts, ShipId, ShipNaming, ShipRow, ShipSpecs, Shipyard};

    use crate::spaceport::shipyard::ShipOrder;

    impl crate::spaceport::grid::GridLooks for FakePort {
        fn grid_colors(&self) -> crate::spaceport::grid::GridColors {
            crate::spaceport::grid::GridColors::STOCK
        }
    }

    impl ShipBaseImages for FakePort {
        fn ship_base_images(&self) -> Vec<(ShipId, i16)> {
            Vec::new()
        }
    }

    impl crate::ui::catalog::DescriptionGraphics for FakePort {
        fn description_graphic(&self, _id: i16) -> Option<i16> {
            None
        }

        /// Names no movie: these screens never play one.
        fn description_movie(&self, _id: i16) -> Option<nova_sim::DescMovie> {
            None
        }
    }

    /// Neither info template.
    fn no_info() -> InfoTemplates {
        InfoTemplates {
            plain: Err("no DLOG 1005".to_owned()),
            picture: Err("no DLOG 1019".to_owned()),
        }
    }

    /// "Shipyard": 765 x 323, centred, with Done (1), the grid (5), the
    /// description (6), Buy Ship (7), the picture (8), the info box (9)
    /// and Info (10).
    fn shipyard_template() -> Template {
        let mut items: Vec<ItemTemplate> = (0..13)
            .map(|_| ItemTemplate {
                bounds: rect(0.0, 0.0, 1.0, 1.0),
                enabled: false,
                kind: ItemSpec::User,
            })
            .collect();
        let mut place = |number: usize, x, y, w, h| {
            items[number - 1] = ItemTemplate {
                bounds: rect(x, y, w, h),
                enabled: true,
                kind: ItemSpec::User,
            };
        };
        place(1, 365.0, 289.0, 109.0, 25.0);
        place(5, 9.0, 8.0, 333.0, 271.0);
        place(6, 354.0, 10.0, 192.0, 267.0);
        place(7, 480.0, 289.0, 109.0, 25.0);
        place(8, 557.0, 8.0, 200.0, 200.0);
        place(9, 614.0, 214.0, 143.0, 100.0);
        place(10, 253.0, 289.0, 89.0, 25.0);
        Template {
            bounds: rect(100.0, 100.0, 765.0, 323.0),
            placement: Placement::Center,
            items,
        }
    }

    /// A shipyard selling ship 129 for `price`, the player flying 128.
    fn yard(price: i64) -> Shipyard {
        Shipyard {
            rows: vec![ShipRow {
                id: ShipId(129),
                name: "Heavy Shuttle".to_owned(),
                short_name: "Heavy Shuttle".to_owned(),
                price,
                specs: ShipSpecs {
                    fields: nova_sim::ShipFields::default(),
                    max_gun: 0,
                    max_tur: 0,
                    length: 0,
                    crew: 0,
                    space: 0,
                    rounding: nova_sim::RuleSource::Engine,
                    weapons: Vec::new(),
                    long_name: String::new(),
                },
                facts: ShipFacts { buyable: true },
            }],
            trade_in: 2500,
            cash: 25_000,
            current: ShipId(128),
            rules: Rulebook::default(),
        }
    }

    /// Earth with a shipyard too.
    fn shipyard_port() -> FakePort {
        FakePort {
            port: Ok(PortRecord {
                flags: FLAGS | StellarFlags::SHIPYARD,
                ..catalog().port.expect("a record")
            }),
            ..catalog()
        }
    }

    fn shipbuying(template: Result<Template, String>) -> SpaceportView {
        let art: Rc<dyn ShipyardCatalog> = Rc::new(shipyard_port());
        view_of(&shipyard_port()).with_shipyard(
            template,
            no_info(),
            Err("no DLOG 3001".to_owned()),
            yard(17_500),
            art,
        )
    }

    fn shipyard_item(view: &SpaceportView, number: usize) -> Point {
        view.open_shipyard()
            .expect("shipbuying")
            .dialog()
            .expect("laid out")
            .item_bounds(number)
            .expect("an item")
            .center()
    }

    #[test]
    fn with_a_shipyard_its_button_opens_it_over_the_spaceport() {
        let mut view = shipbuying(Ok(shipyard_template()));
        assert!(view.open_shipyard().is_none());
        click_item(&mut view, 9);
        let open = view.open_shipyard().expect("shipbuying");
        assert_eq!(open.shipyard(), &yard(17_500));
        assert!(view.open_service().is_none());
        assert!(view.open_outfitter().is_none());
        let mut expected: Vec<DrawCommand> = drawn(&view_of(&shipyard_port()));
        let mut shipyard = DrawList::new();
        open.draw(&mut shipyard);
        expected.extend(shipyard.iter().cloned());
        assert_eq!(drawn(&view), expected);
        assert!(texts(&drawn(&view)).contains(&"Heavy Shuttle".to_owned()));
        // Escape closes the shipyard, not the spaceport.
        view.input(&key(Key::Escape));
        assert!(view.open_shipyard().is_none());
        assert!(!view.left());
        click_item(&mut view, 11);
        assert_eq!(
            view.open_service().map(ServiceScreen::service),
            Some(Service::Bar)
        );
        assert!(view.open_shipyard().is_none());
    }

    #[test]
    fn without_a_shipyard_given_its_button_opens_the_placeholder() {
        let mut view = view_of(&shipyard_port());
        click_item(&mut view, 9);
        assert_eq!(
            view.open_service().map(ServiceScreen::service),
            Some(Service::Shipyard)
        );
        assert!(view.open_shipyard().is_none());
        assert_eq!(view.take_ship_request(), None);
        view.open_ship_naming(&naming());
        assert_eq!(view.take_ship(), None);
        assert_eq!(view.take_declined_ship(), None);
    }

    #[test]
    fn the_shipyard_is_laid_out_by_its_own_dialog_or_says_why_not() {
        let mut view = shipbuying(Err("no DLOG 1004".to_owned()));
        click_item(&mut view, 9);
        assert_eq!(
            view.open_shipyard().and_then(ShipyardScreen::problem),
            Some("no DLOG 1004")
        );
        view.input(&key(Key::Enter));
        assert!(view.open_shipyard().is_none());
        assert!(!view.left());
    }

    #[test]
    fn the_shipyards_orders_are_taken_through_the_spaceport_and_its_state_set() {
        let mut view = shipbuying(Ok(shipyard_template()));
        assert_eq!(view.take_ship(), None, "nothing open");
        view.set_shipyard(yard(16_000));
        click_item(&mut view, 9);
        view.input(&key(Key::Right));
        assert_eq!(
            view.open_shipyard()
                .map(|open| open.shipyard().rows[0].price),
            Some(16_000),
            "it opens on the latest"
        );
        view.input(&key(Key::Char('b')));
        assert_eq!(view.take_ship_request(), Some(ShipId(129)));
        assert_eq!(view.take_ship_request(), None, "once");
        assert_eq!(view.take_ship(), None, "not named yet");
        view.open_ship_naming(&naming());
        let prompt = view
            .open_shipyard()
            .and_then(ShipyardScreen::naming)
            .expect("the prompt");
        assert_eq!(prompt.field().text(), "Ship 129 491");
        view.input(&Input::Text('b'));
        view.input(&Input::Text('K'));
        view.input(&key(Key::Enter));
        assert_eq!(
            view.take_ship(),
            Some(ShipOrder {
                ship: ShipId(129),
                name: "K".to_owned(),
            }),
            "the B that asked typed nothing"
        );
        assert_eq!(view.take_ship(), None, "once");
        assert_eq!(view.take_outfit(), None, "not an outfit");
        assert_eq!(view.take_trade(), None, "nor a trade");
        view.set_shipyard(yard(15_000));
        assert_eq!(
            view.open_shipyard()
                .map(|open| open.shipyard().rows[0].price),
            Some(15_000)
        );
        view.input(&key(Key::Escape));
        click_item(&mut view, 9);
        assert_eq!(
            view.open_shipyard()
                .map(|open| open.shipyard().rows[0].price),
            Some(15_000),
            "and reopens on it"
        );
    }

    fn naming() -> ShipNaming {
        ShipNaming {
            ship: ShipId(129),
            prompt: "Please name your new Heavy Shuttle: ".to_owned(),
            default: "Ship 129 491".to_owned(),
        }
    }

    #[test]
    fn a_ship_declined_at_its_prompt_is_taken_through_the_spaceport() {
        let mut view = shipbuying(Ok(shipyard_template()));
        click_item(&mut view, 9);
        view.input(&key(Key::Right));
        let buy = shipyard_item(&view, 1);
        click(&mut view, buy);
        assert_eq!(view.take_ship_request(), Some(ShipId(129)));
        view.open_ship_naming(&naming());
        let cancel = view
            .open_shipyard()
            .and_then(ShipyardScreen::naming)
            .and_then(|prompt| prompt.dialog().item_bounds(6))
            .expect("Cancel")
            .center();
        click(&mut view, cancel);
        assert_eq!(view.take_declined_ship(), Some(ShipId(129)));
        assert_eq!(view.take_declined_ship(), None, "once");
        assert_eq!(view.take_ship(), None);
        assert!(view.open_shipyard().is_some(), "back in the shipyard");
    }

    #[test]
    fn the_shipyard_takes_cancel_pointer() {
        let mut view = shipbuying(Ok(shipyard_template()));
        click_item(&mut view, 9);
        assert_eq!(view.take_sounds(), []);
        let buy = shipyard_item(&view, 1);
        let button = |pressed| Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at: buy,
        };
        view.input(&button(true));
        view.cancel_pointer();
        view.input(&button(false));
        assert_eq!(view.take_ship_request(), None, "the click was abandoned");
        assert_eq!(view.take_sounds(), []);
        let done = shipyard_item(&view, 7);
        click(&mut view, done);
        assert!(view.open_shipyard().is_none(), "closed");
        assert_eq!(view.take_sounds(), []);
        let debug = format!("{view:?}");
        assert!(debug.contains("Shipbuying"), "{debug}");
        assert!(debug.contains("Heavy Shuttle"), "{debug}");
    }

    // The Bar.

    use crate::spaceport::bar::BarScreen;
    use nova_sim::hire::NONE_FOR_HIRE;
    use nova_sim::{HireList, HireRow};

    /// "Bar": 263 x 185, centred, with Leave (1), Gamble (2), Holovid (3),
    /// Hire Escort (5) and the text (7), all disabled as stock has them.
    fn bar_template() -> Template {
        let mut items: Vec<ItemTemplate> = (0..10)
            .map(|_| ItemTemplate {
                bounds: rect(0.0, 300.0, 1.0, 1.0),
                enabled: false,
                kind: ItemSpec::User,
            })
            .collect();
        let mut place = |number: usize, x, y, w, h| {
            items[number - 1].bounds = rect(x, y, w, h);
        };
        place(1, 156.0, 154.0, 99.0, 26.0);
        place(2, 156.0, 125.0, 99.0, 26.0);
        place(3, 6.0, 154.0, 146.0, 26.0);
        place(5, 6.0, 125.0, 146.0, 26.0);
        place(7, 16.0, 10.0, 230.0, 106.0);
        Template {
            bounds: rect(40.0, 40.0, 263.0, 185.0),
            placement: Placement::Center,
            items,
        }
    }

    /// "News": 300 x 230, centred, with no items.
    fn news_template() -> Template {
        Template {
            bounds: rect(40.0, 40.0, 300.0, 230.0),
            placement: Placement::Center,
            items: Vec::new(),
        }
    }

    fn the_news() -> nova_sim::News {
        nova_sim::News {
            commercial: "Buy Bibble".to_owned(),
            story: "<PN> wins".to_owned(),
        }
    }

    #[test]
    fn opening_the_bar_is_told_once() {
        let mut view = drinking(true);
        assert!(!view.take_bar_opened(), "nothing open");
        click_item(&mut view, 11);
        assert!(view.take_bar_opened());
        assert!(!view.take_bar_opened(), "once");
        view.input(&key(Key::Escape));
        assert!(!view.take_bar_opened(), "leaving is not opening");
        click_item(&mut view, 11);
        assert!(view.take_bar_opened(), "each time it opens");
        let mut view = earth();
        click_item(&mut view, 11);
        assert!(!view.take_bar_opened(), "the placeholder is not the bar");
    }

    #[test]
    fn the_bars_news_asks_through_the_spaceport_and_opens_over_the_bar() {
        let mut view = drinking(true);
        assert!(!view.take_news_request(), "nothing open");
        view.open_news(&the_news(), 9000);
        click_item(&mut view, 11);
        assert!(
            view.open_bar().and_then(BarScreen::news_screen).is_none(),
            "only an answer opens it"
        );
        let at = bar_item(&view, 3);
        click(&mut view, at);
        assert!(view.take_news_request());
        assert!(!view.take_news_request(), "once");
        view.take_sounds();
        view.open_news(&the_news(), 9000);
        let news = view
            .open_bar()
            .and_then(BarScreen::news_screen)
            .expect("open over the bar");
        assert_eq!(news.problem(), None, "laid out by the template given");
        assert_eq!(
            news.story().map(|story| story.lines().join(" ")),
            Some("<PN> wins".to_owned())
        );
        assert_eq!(view.take_sounds(), [Sound::Ui(UiSound::NewsOpened)]);
        view.input(&key(Key::Escape));
        assert!(view.open_bar().is_some(), "closed back to the bar");
        assert!(view.open_bar().and_then(BarScreen::news_screen).is_none());
        assert_eq!(view.take_sounds(), [Sound::Ui(UiSound::NewsClosed)]);
    }

    /// `rows` ships for hire, the fleet with `room` or not.
    fn hirelings(rows: usize, room: bool) -> HireList {
        HireList {
            rules: nova_sim::Rulebook::default(),
            rows: (0..rows)
                .map(|n| HireRow {
                    id: ShipId(128 + n as i16),
                    name: format!("Ship {n}"),
                    short_name: format!("Ship {n}"),
                    fee: 970,
                    wage: 100,
                    specs: ShipSpecs {
                        fields: nova_sim::ShipFields::default(),
                        max_gun: 0,
                        max_tur: 0,
                        length: 0,
                        crew: 0,
                        space: 0,
                        rounding: nova_sim::RuleSource::Engine,
                        weapons: Vec::new(),
                        long_name: String::new(),
                    },
                    hire: Ok(()),
                })
                .collect(),
            cash: 25_000,
            room,
        }
    }

    /// Earth with its bar given, the fleet with `room` or not.
    fn drinking(room: bool) -> SpaceportView {
        let art: Rc<dyn ShipyardCatalog> = Rc::new(catalog());
        earth().with_bar(
            plain_bar(Ok(bar_template())),
            Ok(news_template()),
            Ok(shipyard_template()),
            no_info(),
            art,
            room,
        )
    }

    fn bar_item(view: &SpaceportView, number: usize) -> Point {
        view.open_bar()
            .expect("in the bar")
            .dialog()
            .expect("laid out")
            .item_bounds(number)
            .expect("an item")
            .center()
    }

    #[test]
    fn with_a_bar_its_button_opens_it_over_the_spaceport() {
        let mut view = drinking(true);
        assert!(view.open_bar().is_none());
        click_item(&mut view, 11);
        let bar = view.open_bar().expect("in the bar");
        assert!(view.open_service().is_none());
        assert_eq!(
            bar.dialog()
                .and_then(|dialog| dialog.scroll_text())
                .map(|text| text.lines().join(" ")),
            Some("Blue and green. Home.".to_owned()),
            "its text, dësc 10012, read through the catalog given"
        );
        let mut expected: Vec<DrawCommand> = drawn(&earth());
        let mut over = DrawList::new();
        bar.draw(&mut over);
        expected.extend(over.iter().cloned());
        assert_eq!(drawn(&view), expected);
        view.input(&key(Key::Escape));
        assert!(view.open_bar().is_none(), "Escape leaves the bar");
        assert!(!view.left());
    }

    #[test]
    fn hire_escort_asks_through_the_spaceport_and_the_list_opens_the_hire_screen() {
        let mut view = drinking(true);
        assert!(!view.take_hire_request(), "nothing open");
        view.open_hire(hirelings(1, true));
        assert_eq!(view.take_hire(), None);
        click_item(&mut view, 11);
        view.input(&key(Key::Char('h')));
        assert!(view.take_hire_request());
        assert!(!view.take_hire_request(), "once");
        view.open_hire(hirelings(2, true));
        view.input(&key(Key::Right));
        let hire = view
            .open_bar()
            .and_then(BarScreen::hire_screen)
            .expect("the hire screen");
        assert_eq!(hire.list().rows.len(), 2);
        assert_eq!(hire.problem(), None, "laid out by DLOG 1004");
        view.input(&key(Key::Char('h')));
        assert_eq!(view.take_hire(), Some(ShipId(128)));
        assert_eq!(view.take_hire(), None, "once");
        assert_eq!(view.take_ship_request(), None, "not a ship bought");
        view.set_hire(&hirelings(2, false));
        let bar = view.open_bar().expect("back in the bar");
        assert!(bar.hire_screen().is_none());
        let hire_label = bar_item(&view, 5);
        click(&mut view, hire_label);
        assert!(!view.take_hire_request(), "the fleet is full");
        view.input(&key(Key::Escape));
        click_item(&mut view, 11);
        view.input(&key(Key::Char('e')));
        assert!(
            !view.take_hire_request(),
            "the bar opened next knows it too"
        );
    }

    #[test]
    fn the_hire_screen_closing_is_reported_once_after_a_hire_a_cancel_or_nothing_for_hire() {
        let mut view = drinking(true);
        assert!(!view.take_hire_closed(), "nothing open");
        click_item(&mut view, 11);
        view.input(&key(Key::Char('h')));
        assert!(view.take_hire_request());
        view.open_hire(hirelings(2, true));
        view.input(&key(Key::Right));
        assert!(!view.take_hire_closed(), "opening is not closing");
        view.input(&key(Key::Char('h')));
        assert_eq!(view.take_hire(), Some(ShipId(128)));
        assert!(!view.take_hire_closed(), "not until the hire is made");
        view.set_hire(&hirelings(2, true));
        assert!(view.take_hire_closed(), "after a hire");
        assert!(!view.take_hire_closed(), "once");
        view.input(&key(Key::Char('h')));
        assert!(view.take_hire_request());
        view.open_hire(hirelings(2, true));
        view.input(&key(Key::Escape));
        assert!(
            view.open_bar()
                .is_some_and(|bar| bar.hire_screen().is_none())
        );
        assert!(view.take_hire_closed(), "cancelled");
        view.input(&key(Key::Char('h')));
        assert!(view.take_hire_request());
        view.open_hire(hirelings(0, true));
        assert!(
            view.take_hire_closed(),
            "nothing for hire: the hire dialog returns at once (@0x5e8df)"
        );
        view.input(&key(Key::Escape));
        assert!(!view.take_hire_closed(), "leaving the bar is not it");
    }

    #[test]
    fn with_nothing_for_hire_a_text_says_so() {
        let mut view = drinking_with_info();
        click_item(&mut view, 11);
        let _ = view.take_offer_request();
        let _ = view.take_status_refresh();
        view.input(&key(Key::Char('h')));
        assert!(view.take_hire_request());
        view.open_hire(hirelings(0, true));
        let bar = view.open_bar().expect("in the bar");
        assert!(bar.hire_screen().is_none());
        assert_eq!(shown_text(&view), Some(NONE_FOR_HIRE.to_owned()));
        assert_eq!(view.take_port_dialog_opened(), None);
        assert!(!view.take_status_refresh(), "nothing redrawn");
        assert_eq!(view.take_offer_request(), None, "no offer asked");
    }

    #[test]
    fn the_bars_map_key_asks_through_the_spaceport() {
        use crate::flight::view::MAP_KEY;
        let mut view = drinking(true);
        view.input(&key(MAP_KEY));
        assert_eq!(
            view.take_map_request(),
            Some(PortMap::Screen(PortScreen::Spaceport)),
            "no bar open: the spaceport's own"
        );
        click_item(&mut view, 11);
        view.input(&key(MAP_KEY));
        assert_eq!(view.take_map_request(), Some(PortMap::Bar));
        assert_eq!(view.take_map_request(), None, "once");
        assert!(view.open_bar().is_some(), "still in the bar");
    }

    #[test]
    fn the_map_key_asks_from_an_offer_over_each_venue() {
        use crate::flight::view::MAP_KEY;
        for venue in [SPACEPORT, BAR, TRADE_CENTER, OUTFITTER, SHIPYARD] {
            let mut view = venues();
            assert_eq!(view.take_offer_request(), Some(SPACEPORT));
            if venue != SPACEPORT {
                let item = match venue {
                    BAR => 11,
                    TRADE_CENTER => 7,
                    OUTFITTER => 8,
                    _ => 9,
                };
                click_item(&mut view, item);
                if view.take_outfitter_opened() {
                    view.set_opened_outfitter(tanks(1));
                }
                view.tick(FIRST_OFFER_DELAY);
                assert_eq!(view.take_offer_request(), Some(venue));
            }
            view.open_offer(offered());
            view.take_sounds();
            view.input(&key(MAP_KEY));
            assert_eq!(
                view.take_map_request(),
                Some(PortMap::Dialog(MapOpener::Offer(MissionId(141)))),
                "{venue}"
            );
            assert_eq!(view.take_map_request(), None, "{venue}: once");
            assert!(view.offer_dialog().is_some(), "{venue}: still open");
            assert_eq!(view.take_sounds(), [], "{venue}: silent");
        }
    }

    #[test]
    fn the_map_key_asks_from_the_mission_computer_with_its_selected_offer() {
        use crate::flight::view::MAP_KEY;
        let mut view = missioned();
        view.open_mission_computer(Ok(offers()));
        view.input(&key(MAP_KEY));
        assert_eq!(
            view.take_map_request(),
            Some(PortMap::Dialog(MapOpener::Computer(Some(MissionId(130)))))
        );
        assert!(view.mission_computer().is_some(), "still open");
    }

    #[test]
    fn mission_info_asks_for_the_map_unless_opened_over_an_offer_or_the_computer() {
        use crate::flight::view::MAP_KEY;
        let mut view = missioned();
        view.open_mission_info(actives(2));
        view.input(&key(MAP_KEY));
        assert_eq!(
            view.take_map_request(),
            Some(PortMap::Dialog(MapOpener::MissionInfo(Some(0)))),
            "over the spaceport"
        );
        let mut view = missioned();
        view.open_mission_computer(Ok(offers()));
        view.open_mission_info(actives(2));
        view.input(&key(MAP_KEY));
        assert_eq!(view.take_map_request(), None, "over the computer");
        assert!(view.mission_info().is_some());
        let mut view = missioned();
        view.open_offer(offered());
        view.open_mission_info(actives(2));
        view.input(&key(MAP_KEY));
        assert_eq!(view.take_map_request(), None, "over an offer");
    }

    #[test]
    fn the_briefing_asks_for_its_map_and_another_text_for_none() {
        use crate::flight::view::MAP_KEY;
        let marks = nova_sim::MapMarks {
            green: Some(nova_sim::SystemId(131)),
            multi_arrows: true,
        };
        let text = |map| MissionText {
            kind: MissionTextKind::Brief,
            mission: MissionId(130),
            text: "Go.".to_owned(),
            graphic: None,
            movie: None,
            map,
        };
        let mut view = missioned();
        view.open_mission_computer(Ok(offers()));
        view.show_mission_texts(vec![text(Some(marks)), text(None)]);
        view.input(&key(MAP_KEY));
        assert_eq!(
            view.take_map_request(),
            Some(PortMap::Dialog(MapOpener::Briefing(marks)))
        );
        view.input(&key(Key::Enter));
        view.input(&key(MAP_KEY));
        assert_eq!(view.take_map_request(), None, "a text with no map");
        assert!(shown_text(&view).is_some(), "still open");
    }

    #[test]
    fn a_full_fleet_greys_hire_escort_when_the_bar_opens() {
        let mut view = drinking(false);
        click_item(&mut view, 11);
        view.input(&key(Key::Char('h')));
        assert!(!view.take_hire_request());
        let debug = format!("{view:?}");
        assert!(debug.contains("Barkeeping"), "{debug}");
    }

    // The mission computer and the mission texts.

    use crate::spaceport::missions::{ACCEPT_ITEM as COMPUTER_ACCEPT, MissionComputerScreen};
    use nova_sim::{ComputerRow, MissionText, MissionTextKind};

    /// The mission computer's shape, smaller: 300 x 200, fixed at (0, 0),
    /// with Accept (1), the list (2), the text (4), the name (5), Leave
    /// (7), the heading (8) and the date (11).
    fn computer_template() -> Template {
        let mut items: Vec<ItemTemplate> = (0..11)
            .map(|_| ItemTemplate {
                bounds: rect(0.0, 300.0, 1.0, 1.0),
                enabled: false,
                kind: ItemSpec::User,
            })
            .collect();
        let mut place = |number: usize, x, y, w, h| {
            items[number - 1].bounds = rect(x, y, w, h);
        };
        place(1, 10.0, 170.0, 99.0, 25.0);
        place(2, 10.0, 30.0, 100.0, 120.0);
        place(4, 120.0, 60.0, 170.0, 90.0);
        place(5, 120.0, 34.0, 170.0, 20.0);
        place(7, 120.0, 170.0, 99.0, 25.0);
        place(8, 10.0, 3.0, 200.0, 15.0);
        place(11, 210.0, 3.0, 90.0, 15.0);
        Template {
            bounds: rect(0.0, 0.0, 300.0, 200.0),
            placement: Placement::Fixed,
            items,
        }
    }

    /// The picture variants' shape, wider than [`text_template`]: 300 x
    /// 120 at (0, 0), fixed, with Done (1), the picture (2) and the text
    /// box (3).
    fn picture_template() -> Template {
        Template {
            bounds: rect(0.0, 0.0, 300.0, 120.0),
            ..text_template()
        }
    }

    /// The offer dialog's templates: [`text_template`], and no picture
    /// variant.
    fn plain_offers() -> OfferTemplates {
        OfferTemplates {
            plain: Ok(text_template()),
            picture: Err("no DLOG 1020".to_owned()),
        }
    }

    /// "Desc Dialog"'s templates: [`text_template`], and no picture
    /// variant.
    fn plain_texts() -> DescTemplates {
        DescTemplates {
            plain: Ok(text_template()),
            picture: Err("no DLOG 3004".to_owned()),
        }
    }

    /// The bar's templates: `plain`, and no picture variant.
    fn plain_bar(plain: Result<Template, String>) -> BarTemplates {
        BarTemplates {
            plain,
            picture: Err("no DLOG 1021".to_owned()),
        }
    }

    /// "Desc Dialog"'s shape, smaller: 200 x 120 at (0, 0), fixed, with
    /// Done (1) and a 60 x 36 text box (3).
    fn text_template() -> Template {
        let item = |bounds, enabled| ItemTemplate {
            bounds,
            enabled,
            kind: ItemSpec::User,
        };
        Template {
            bounds: rect(0.0, 0.0, 200.0, 120.0),
            placement: Placement::Fixed,
            items: vec![
                item(rect(100.0, 90.0, 99.0, 25.0), true),
                item(rect(10.0, 150.0, 20.0, 20.0), false),
                item(rect(10.0, 10.0, 180.0, 36.0), false),
            ],
        }
    }

    /// Earth with its mission screens given, its offers laid out by the
    /// text's shape, and both picture variants by [`picture_template`].
    pub(in crate::spaceport) fn missioned() -> SpaceportView {
        earth().with_missions(
            Ok(computer_template()),
            Ok(info_template()),
            pictured_offers(),
            pictured_texts(),
        )
    }

    fn pictured_offers() -> OfferTemplates {
        OfferTemplates {
            plain: Ok(text_template()),
            picture: Ok(picture_template()),
        }
    }

    fn pictured_texts() -> DescTemplates {
        DescTemplates {
            plain: Ok(text_template()),
            picture: Ok(picture_template()),
        }
    }

    fn offers() -> MissionComputer {
        MissionComputer {
            rows: vec![ComputerRow {
                mission: MissionId(130),
                name: "Run".to_owned(),
                raw_name: "Run".to_owned(),
                text: "Go.".to_owned(),
            }],
            today: "June 23rd, 1177 NC".to_owned(),
        }
    }

    fn shown_text(view: &SpaceportView) -> Option<String> {
        view.mission_text().map(|dialog| dialog.lines().join(" "))
    }

    fn done(view: &mut SpaceportView) {
        view.input(&key(Key::Enter));
    }

    #[test]
    fn mission_bbs_asks_for_the_mission_computer() {
        let mut view = missioned();
        assert!(!view.take_mission_computer_request());
        click_item(&mut view, 10);
        assert!(view.open_service().is_none(), "no placeholder");
        assert!(view.take_mission_computer_request());
        assert!(!view.take_mission_computer_request(), "once");
        view.open_mission_computer(Ok(offers()));
        let computer = view.mission_computer().expect("open");
        assert_eq!(computer.computer(), &offers());
        assert_eq!(computer.problem(), None, "laid out by the template given");
        let mut expected: Vec<DrawCommand> = drawn(&earth());
        let mut over = DrawList::new();
        computer.draw(&mut over);
        expected.extend(over.iter().cloned());
        assert_eq!(drawn(&view), expected);
        view.input(&key(Key::Escape));
        assert!(view.mission_computer().is_none(), "left");
        assert!(!view.left());
    }

    #[test]
    fn without_mission_screens_mission_bbs_opens_its_placeholder() {
        let mut view = earth();
        click_item(&mut view, 10);
        assert_eq!(
            view.open_service().map(ServiceScreen::service),
            Some(Service::MissionBbs)
        );
        assert!(!view.take_mission_computer_request());
    }

    #[test]
    fn a_refused_computer_says_why_over_the_spaceport() {
        let mut view = missioned();
        click_item(&mut view, 10);
        view.open_mission_computer(Err(ComputerRefusal::NoneAvailable));
        assert!(view.mission_computer().is_none());
        assert_eq!(
            shown_text(&view).as_deref(),
            Some("There are no missions available here.")
        );
        let dialog = view.mission_text().expect("shown");
        let mut expected: Vec<DrawCommand> = drawn(&earth());
        let mut over = DrawList::new();
        dialog.draw(&mut over);
        expected.extend(over.iter().cloned());
        assert_eq!(drawn(&view), expected);
        click_item(&mut view, 11);
        assert!(
            view.open_bar().is_none() && view.open_service().is_none(),
            "it takes every input"
        );
        done(&mut view);
        assert_eq!(shown_text(&view), None);
        assert!(!view.left());
    }

    #[test]
    fn an_accept_is_taken_through_the_spaceport_and_the_computer_handed_back() {
        let mut view = missioned();
        assert_eq!(view.take_mission_accept(), None, "nothing open");
        click_item(&mut view, 10);
        view.open_mission_computer(Ok(offers()));
        let at = view
            .mission_computer()
            .and_then(MissionComputerScreen::dialog)
            .and_then(|dialog| dialog.item_bounds(COMPUTER_ACCEPT))
            .expect("Accept")
            .center();
        click(&mut view, at);
        assert_eq!(view.take_mission_accept(), Some(MissionId(130)));
        assert_eq!(view.take_mission_accept(), None, "once");
        let fewer = MissionComputer {
            rows: Vec::new(),
            ..offers()
        };
        view.set_mission_computer(Ok(fewer.clone()));
        assert_eq!(
            view.mission_computer().map(MissionComputerScreen::computer),
            Some(&fewer)
        );
        view.set_mission_computer(Err(ComputerRefusal::NoneAvailable));
        assert!(view.mission_computer().is_none(), "closed");
        assert_eq!(shown_text(&view), None, "silently");
    }

    #[test]
    fn the_mission_computer_closing_waits_for_the_texts_queued() {
        let mut view = missioned();
        click_item(&mut view, 10);
        view.open_mission_computer(Ok(offers()));
        view.show_mission_texts(vec![MissionText {
            kind: MissionTextKind::Brief,
            mission: MissionId(130),
            text: "Go.".to_owned(),
            graphic: None,
            movie: None,
            map: None,
        }]);
        view.set_mission_computer(Err(ComputerRefusal::NoneAvailable));
        assert!(view.mission_computer().is_none(), "closed");
        assert_eq!(
            view.take_port_dialog_closed(),
            None,
            "told once its text closes"
        );
        done(&mut view);
        assert_eq!(shown_text(&view), None);
        assert_eq!(
            view.take_port_dialog_closed(),
            Some(PortDialog::MissionComputer)
        );
        assert_eq!(view.take_port_dialog_closed(), None, "once");
    }

    #[test]
    fn each_mission_text_closing_is_told_once_and_with_no_dialogs_at_once() {
        let text = |text: &str| MissionText {
            kind: MissionTextKind::Brief,
            mission: MissionId(130),
            text: text.to_owned(),
            graphic: None,
            movie: None,
            map: None,
        };
        let mut view = missioned();
        view.show_mission_texts(vec![text("First."), text("Second.")]);
        assert_eq!(view.take_texts_closed(), 0, "shown");
        done(&mut view);
        assert_eq!(view.take_texts_closed(), 1, "the first");
        assert_eq!(view.take_texts_closed(), 0, "once");
        done(&mut view);
        assert_eq!(view.take_texts_closed(), 1, "the second");
        let mut bare = earth();
        bare.show_mission_texts(vec![text("Lost."), text("Gone.")]);
        assert_eq!(bare.take_texts_closed(), 2, "dropped, so closed at once");
        assert_eq!(bare.take_texts_closed(), 0, "once");
    }

    #[test]
    fn a_refused_mission_computer_opens_and_closes_once_its_refusal_closes() {
        let mut view = missioned();
        click_item(&mut view, 10);
        view.open_mission_computer(Err(ComputerRefusal::NoneAvailable));
        assert_eq!(
            view.take_port_dialog_opened(),
            Some(PortDialog::MissionComputer),
            "opened at once"
        );
        assert_eq!(view.take_port_dialog_closed(), None, "its text shows");
        done(&mut view);
        assert_eq!(
            view.take_port_dialog_closed(),
            Some(PortDialog::MissionComputer)
        );
        assert_eq!(view.take_port_dialog_closed(), None, "once");
    }

    #[test]
    fn mission_texts_show_one_after_another_over_whatever_is_open() {
        let mut view = missioned();
        click_item(&mut view, 10);
        view.open_mission_computer(Ok(offers()));
        let text = |mission, text: &str| MissionText {
            graphic: None,
            kind: MissionTextKind::Brief,
            mission: MissionId(mission),
            text: text.to_owned(),
            movie: None,
            map: None,
        };
        view.show_mission_texts(vec![text(130, "First."), text(131, "Second.")]);
        assert_eq!(shown_text(&view).as_deref(), Some("First."));
        let mut expected = Vec::new();
        let mut below = DrawList::new();
        view.mission_computer().expect("open").draw(&mut below);
        let dialog = view.mission_text().expect("shown");
        let mut over = DrawList::new();
        dialog.draw(&mut over);
        expected.extend(over.iter().cloned());
        let commands = drawn(&view);
        assert_eq!(commands[commands.len() - expected.len()..], expected[..]);
        view.input(&key(Key::Escape));
        assert_eq!(shown_text(&view).as_deref(), Some("Second."));
        assert!(view.mission_computer().is_some(), "Escape went to the text");
        done(&mut view);
        assert_eq!(shown_text(&view), None);
        view.input(&key(Key::Escape));
        assert!(view.mission_computer().is_none());
        view.show_mission_texts(Vec::<MissionText>::new());
        assert_eq!(shown_text(&view), None);
    }

    #[test]
    fn mission_texts_click_their_done_and_cancel_their_pointer() {
        let mut view = missioned();
        view.show_mission_texts(vec![MissionText {
            kind: MissionTextKind::Comp,
            mission: MissionId(130),
            text: "Done.".to_owned(),
            graphic: None,
            movie: None,
            map: None,
        }]);
        let at = view
            .mission_text()
            .and_then(|dialog| dialog.dialog().item_bounds(1))
            .expect("Done")
            .center();
        view.input(&Input::PointerButton {
            button: MouseButton::Left,
            pressed: true,
            at,
        });
        view.cancel_pointer();
        view.input(&Input::PointerButton {
            button: MouseButton::Left,
            pressed: false,
            at,
        });
        assert!(view.mission_text().is_some(), "abandoned");
        assert_eq!(view.take_sounds(), []);
        click(&mut view, at);
        assert!(view.mission_text().is_none());
        assert_eq!(view.take_sounds(), []);
    }

    #[test]
    fn without_a_text_dialog_mission_texts_are_not_shown() {
        let mut view = earth().with_missions(
            Ok(computer_template()),
            Err("no DLOG 1012".to_owned()),
            OfferTemplates {
                plain: Err("no DLOG 1016".to_owned()),
                picture: Err("no DLOG 1020".to_owned()),
            },
            DescTemplates {
                plain: Err("no DLOG 3003".to_owned()),
                picture: Err("no DLOG 3004".to_owned()),
            },
        );
        view.open_mission_computer(Err(ComputerRefusal::SlotsFull));
        assert!(view.mission_text().is_none());
        let debug = format!("{view:?}");
        assert!(debug.contains("Missioning"), "{debug}");
    }

    // The bar's offers.

    use crate::spaceport::bar::FIRST_OFFER_DELAY;
    use crate::spaceport::offer::{OfferChoice, OfferDialog};
    use nova_sim::OfferPrompt;
    use nova_sim::mission::OfferLabels;

    /// Earth with its bar and its mission screens given.
    fn drinking_with_offers() -> SpaceportView {
        let art: Rc<dyn ShipyardCatalog> = Rc::new(catalog());
        missioned().with_bar(
            plain_bar(Ok(bar_template())),
            Ok(news_template()),
            Ok(shipyard_template()),
            no_info(),
            art,
            true,
        )
    }

    pub(in crate::spaceport) fn offered() -> OfferPrompt {
        OfferPrompt {
            mission: MissionId(141),
            text: "Ferry this.".to_owned(),
            graphic: None,
            labels: OfferLabels {
                accept: "Yes".to_owned(),
                refuse: Some("No".to_owned()),
            },
            movie: None,
        }
    }

    // The offers at each venue.

    use nova_sim::mission::screens::{BAR, OUTFITTER, SHIPYARD, SPACEPORT, TRADE_CENTER};

    /// Earth with every venue: a trade center, an outfitter, a shipyard
    /// and a bar, and its mission screens.
    pub(in crate::spaceport) fn venues() -> SpaceportView {
        let port = FakePort {
            port: Ok(PortRecord {
                flags: FLAGS | StellarFlags::OUTFITTER | StellarFlags::SHIPYARD,
                ..catalog().port.expect("a record")
            }),
            ..catalog()
        };
        let outfits: Rc<dyn OutfitterCatalog> = Rc::new(outfitting_port());
        let ships: Rc<dyn ShipyardCatalog> = Rc::new(shipyard_port());
        view_of(&port)
            .with_trade(
                Ok(trade_template()),
                Ok(quantity_template()),
                exchange(0),
                ListColors::STOCK,
            )
            .with_outfitter(
                Ok(outfit_template()),
                Ok(quantity_template()),
                Ok(desc_template()),
                tanks(0),
                outfits,
            )
            .with_shipyard(
                Ok(shipyard_template()),
                no_info(),
                Err("no DLOG 3001".to_owned()),
                yard(17_500),
                Rc::clone(&ships),
            )
            .with_bar(
                plain_bar(Ok(bar_template())),
                Ok(news_template()),
                Ok(shipyard_template()),
                no_info(),
                ships,
                true,
            )
            .with_missions(
                Ok(computer_template()),
                Ok(info_template()),
                pictured_offers(),
                pictured_texts(),
            )
    }

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    #[test]
    fn landing_asks_for_the_spaceports_offer_once() {
        let mut view = missioned();
        assert_eq!(view.take_offer_request(), Some(SPACEPORT));
        assert_eq!(view.take_offer_request(), None, "once");
        view.tick(Duration::from_secs(10));
        assert_eq!(view.take_offer_request(), None);
        let mut view = earth();
        assert_eq!(view.take_offer_request(), None, "no mission screens");
        assert!(view.offer_dialog().is_none());
    }

    #[test]
    fn the_spaceports_offer_opens_over_it_under_the_mission_texts() {
        let mut view = missioned();
        assert_eq!(view.take_offer_request(), Some(SPACEPORT));
        view.show_mission_texts(vec![MissionText {
            kind: MissionTextKind::Comp,
            mission: MissionId(130),
            text: "Well done.".to_owned(),
            graphic: None,
            movie: None,
            map: None,
        }]);
        view.open_offer(offered());
        let offer = view.offer_dialog().expect("open");
        assert_eq!(offer.prompt(), &offered());
        assert_eq!(offer.problem(), None, "laid out by the template given");
        let mut expected = drawn(&earth());
        let mut over = DrawList::new();
        offer.draw(&mut over);
        view.mission_text().expect("shown").draw(&mut over);
        expected.extend(over.iter().cloned());
        assert_eq!(drawn(&view), expected, "the spaceport, the offer, the text");
        done(&mut view);
        assert_eq!(shown_text(&view), None, "the text took the Return");
        assert!(view.offer_dialog().is_some());
        assert_eq!(view.take_offer_choice(), None);
        view.input(&key(Key::Escape));
        assert!(!view.left(), "the offer took the Escape");
        click_item(&mut view, 11);
        assert!(view.open_bar().is_none() && view.open_service().is_none());
        view.input(&key(Key::Enter));
        assert!(view.offer_dialog().is_none(), "answered, it closes");
        assert_eq!(
            view.take_offer_choice(),
            Some((MissionId(141), OfferChoice::Accept))
        );
        assert_eq!(view.take_offer_choice(), None, "once");
        view.input(&key(Key::Escape));
        assert!(view.left(), "then the spaceport takes the input");
    }

    #[test]
    fn the_mission_info_screen_shows_over_the_spaceports_offer() {
        let mut view = missioned();
        view.open_offer(offered());
        view.open_mission_info(actives(1));
        let info = view.mission_info().expect("open");
        let commands = drawn(&view);
        let mut over = DrawList::new();
        info.draw(&mut over);
        let over: Vec<_> = over.iter().cloned().collect();
        assert_eq!(commands[commands.len() - over.len()..], over[..]);
        view.input(&key(Key::Escape));
        assert!(view.mission_info().is_none(), "Escape closed it");
        assert!(view.offer_dialog().is_some(), "and not the offer");
    }

    #[test]
    fn the_spaceports_offer_pointer_goes_through_it_silently() {
        let mut view = missioned();
        view.open_offer(offered());
        let at = view
            .offer_dialog()
            .and_then(OfferDialog::dialog)
            .and_then(|dialog| dialog.item_bounds(crate::spaceport::offer::ACCEPT_ITEM))
            .expect("accept")
            .center();
        let button = |pressed| Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at,
        };
        view.input(&button(true));
        view.cancel_pointer();
        view.input(&button(false));
        assert_eq!(view.take_offer_choice(), None, "abandoned");
        assert_eq!(view.take_sounds(), []);
        view.input(&button(true));
        view.input(&button(false));
        assert_eq!(
            view.take_offer_choice(),
            Some((MissionId(141), OfferChoice::Accept))
        );
    }

    #[test]
    fn the_spaceport_never_asks_again() {
        let mut view = venues();
        assert_eq!(view.take_offer_request(), Some(SPACEPORT));
        view.open_offer(offered());
        view.input(&key(Key::Enter));
        assert!(view.take_offer_choice().is_some());
        view.rearm_offers(ms(500));
        view.tick(Duration::from_secs(10));
        assert_eq!(view.take_offer_request(), None, "never rearmed");
        click_item(&mut view, 7);
        assert_eq!(view.take_offer_request(), Some(TRADE_CENTER));
        view.input(&key(Key::Escape));
        assert!(view.open_trade().is_none());
        view.tick(Duration::from_secs(10));
        assert_eq!(view.take_offer_request(), None, "back at the spaceport");
    }

    /// [`venues`] with its spaceport's offer taken.
    fn shopping() -> SpaceportView {
        let mut view = venues();
        assert_eq!(view.take_offer_request(), Some(SPACEPORT));
        view
    }

    /// Asserts nothing of `view` opened, closed, redrew or asked for an
    /// offer, as an empty shop returns before any of it.
    fn nothing_opened(view: &mut SpaceportView, what: &str) {
        assert_eq!(view.take_port_dialog_opened(), None, "{what}: opened");
        assert_eq!(view.take_port_dialog_closed(), None, "{what}: closed");
        assert!(!view.take_outfitter_closed(), "{what}: outfitter closed");
        assert!(!view.take_status_refresh(), "{what}: redrawn");
        assert_eq!(view.take_offer_request(), None, "{what}: an offer");
    }

    #[test]
    fn an_outfitter_with_nothing_listed_says_so_in_a_text_and_never_opens() {
        let mut view = shopping();
        click_item(&mut view, 8);
        assert!(view.take_outfitter_opened());
        assert!(!view.take_empty_outfitter(), "not until it is shown");
        assert_eq!(
            view.take_offer_request(),
            None,
            "no offer until its listing is shown"
        );
        view.set_opened_outfitter(Outfitter {
            rows: Vec::new(),
            ..tanks(0)
        });
        assert!(view.open_outfitter().is_none());
        assert_eq!(
            shown_text(&view),
            Some(crate::spaceport::outfitter::NO_ITEMS.to_owned())
        );
        nothing_opened(&mut view, "the outfitter");
        assert!(view.take_empty_outfitter());
        assert!(!view.take_empty_outfitter(), "once");

        let mut view = shopping();
        click_item(&mut view, 8);
        assert!(view.take_outfitter_opened());
        assert_eq!(view.take_offer_request(), None, "not yet listed");
        view.set_opened_outfitter(tanks(1));
        assert_eq!(
            view.take_offer_request(),
            Some(OUTFITTER),
            "a listing shown asks"
        );
        assert_eq!(
            view.open_outfitter()
                .map(|open| open.outfitter().rows[0].owned),
            Some(1),
            "a listing shown"
        );
        assert_eq!(view.take_port_dialog_opened(), Some(PortDialog::Outfitter));
        assert!(!view.take_empty_outfitter());
        assert_eq!(shown_text(&view), None);
    }

    #[test]
    fn a_shipyard_with_nothing_listed_says_so_in_a_text_and_never_opens() {
        let ships: Rc<dyn ShipyardCatalog> = Rc::new(shipyard_port());
        let mut view = shopping().with_shipyard(
            Ok(shipyard_template()),
            no_info(),
            Err("no DLOG 3001".to_owned()),
            Shipyard {
                rows: Vec::new(),
                ..yard(17_500)
            },
            ships,
        );
        click_item(&mut view, 9);
        assert!(view.open_shipyard().is_none());
        assert_eq!(
            shown_text(&view),
            Some(crate::spaceport::shipyard::NO_SHIPS.to_owned())
        );
        nothing_opened(&mut view, "the shipyard");
        assert!(!view.take_empty_outfitter());
    }

    #[test]
    fn each_open_screens_description_is_asked_through_the_spaceport() {
        for item in [8, 9, 11] {
            let mut view = venues();
            assert_eq!(view.take_description_request(), Some(140), "the port's");
            click_item(&mut view, item);
            if item == 8 || item == 9 {
                // The outfitter and the shipyard open with nothing
                // selected by the engine.
                view.input(&key(Key::Right));
            }
            let id = view.take_description_request().expect("asked");
            assert_ne!(id, 140, "{item}");
            assert_eq!(view.take_description_request(), None, "{item}: once");
            view.show_description(id, "Loaded venue.");
            assert!(
                texts(&drawn(&view)).contains(&"Loaded venue.".to_owned()),
                "{item}"
            );
        }
    }

    #[test]
    fn each_shop_asks_as_its_venue() {
        for (item, venue) in [(7, TRADE_CENTER), (8, OUTFITTER), (9, SHIPYARD)] {
            let mut view = venues();
            assert_eq!(view.take_offer_request(), Some(SPACEPORT));
            click_item(&mut view, item);
            if view.take_outfitter_opened() {
                view.set_opened_outfitter(tanks(1));
            }
            assert_eq!(view.take_offer_request(), Some(venue), "{venue}");
            assert_eq!(view.take_offer_request(), None, "{venue}: once");
            view.open_offer(offered());
            let opened = match venue {
                TRADE_CENTER => view.open_trade().and_then(TradeScreen::offer_dialog),
                OUTFITTER => view
                    .open_outfitter()
                    .and_then(OutfitterScreen::offer_dialog),
                _ => view.open_shipyard().and_then(ShipyardScreen::offer_dialog),
            };
            assert_eq!(opened.map(OfferDialog::prompt), Some(&offered()), "{venue}");
            assert_eq!(
                view.offer_dialog().map(OfferDialog::prompt),
                Some(&offered()),
                "{venue}"
            );
            view.input(&key(Key::Char('n')));
            assert_eq!(
                view.take_offer_choice(),
                Some((MissionId(141), OfferChoice::Refuse)),
                "{venue}"
            );
            view.rearm_offers(ms(500));
            view.tick(ms(499));
            assert_eq!(view.take_offer_request(), None, "{venue}");
            view.tick(ms(1));
            assert_eq!(view.take_offer_request(), Some(venue), "{venue}: again");
        }
    }

    #[test]
    fn the_bar_asks_as_its_venue_a_quarter_second_after_it_opens() {
        let mut view = drinking_with_offers();
        assert_eq!(view.take_offer_request(), Some(SPACEPORT));
        click_item(&mut view, 11);
        assert_eq!(view.take_offer_request(), None);
        view.tick(ms(249));
        assert_eq!(view.take_offer_request(), None);
        view.tick(ms(1));
        assert_eq!(view.take_offer_request(), Some(BAR));
        view.open_offer(offered());
        let offer = view
            .open_bar()
            .and_then(BarScreen::offer_dialog)
            .expect("open");
        assert_eq!(offer.problem(), None, "laid out by the template given");
        assert!(view.offer_dialog().is_some());
        view.input(&key(Key::Char('n')));
        assert_eq!(
            view.take_offer_choice(),
            Some((MissionId(141), OfferChoice::Refuse))
        );
        view.rearm_offers(ms(500));
        view.tick(ms(500));
        assert_eq!(view.take_offer_request(), Some(BAR));
    }

    #[test]
    fn a_pictured_offer_opens_in_the_picture_variant_at_the_bar_and_the_spaceport() {
        let pictured = || OfferPrompt {
            graphic: Some(5008),
            ..offered()
        };
        let width = |view: &SpaceportView| {
            view.offer_dialog()
                .and_then(OfferDialog::dialog)
                .map(|dialog| dialog.bounds().width())
        };
        let mut view = drinking_with_offers();
        assert_eq!(view.take_offer_request(), Some(SPACEPORT));
        view.open_offer(pictured());
        assert_eq!(
            width(&view),
            Some(300.0),
            "the spaceport's, in 1020's shape"
        );
        view.input(&key(Key::Char('n')));
        let _ = view.take_offer_choice();
        click_item(&mut view, 11);
        view.tick(ms(250));
        assert_eq!(view.take_offer_request(), Some(BAR));
        view.open_offer(pictured());
        assert_eq!(width(&view), Some(300.0), "the bar's, in 1020's shape");
        view.input(&key(Key::Char('n')));
        let _ = view.take_offer_choice();
        view.rearm_offers(ms(0));
        view.tick(ms(1));
        assert_eq!(view.take_offer_request(), Some(BAR));
        view.open_offer(offered());
        assert_eq!(width(&view), Some(200.0), "no graphic, 1016's shape");
    }

    #[test]
    fn a_mission_text_with_a_graphic_shows_in_the_picture_variant() {
        let mut view = missioned();
        let text = |graphic| MissionText {
            kind: MissionTextKind::Brief,
            mission: MissionId(130),
            text: "Off you go.".to_owned(),
            graphic,
            movie: None,
            map: None,
        };
        view.show_mission_texts(vec![text(Some(5008)), text(Some(127)), text(None)]);
        let mut shown = Vec::new();
        while let Some(dialog) = view.mission_text() {
            shown.push((dialog.dialog().bounds().width(), dialog.picture()));
            done(&mut view);
        }
        assert_eq!(shown, [(300.0, Some(5008)), (200.0, None), (200.0, None)]);
        let mut view = missioned();
        view.open_mission_computer(Err(ComputerRefusal::SlotsFull));
        assert_eq!(
            view.mission_text().map(DescDialog::picture),
            Some(None),
            "a refusal is no dësc"
        );
    }

    #[test]
    fn the_mission_computer_and_placeholder_services_ask_nothing() {
        let mut view = missioned();
        assert_eq!(view.take_offer_request(), Some(SPACEPORT));
        click_item(&mut view, 10);
        view.open_mission_computer(Ok(offers()));
        view.tick(Duration::from_secs(10));
        assert_eq!(view.take_offer_request(), None, "the computer");
        view.open_offer(offered());
        view.rearm_offers(Duration::ZERO);
        view.tick(Duration::from_secs(10));
        assert!(view.offer_dialog().is_none());
        assert_eq!(view.take_offer_request(), None);
        assert_eq!(view.take_offer_choice(), None);
        let mut view = missioned();
        assert_eq!(view.take_offer_request(), Some(SPACEPORT));
        click_item(&mut view, 7);
        assert!(
            view.open_service().is_some(),
            "the trade center's placeholder"
        );
        view.tick(Duration::from_secs(10));
        assert_eq!(view.take_offer_request(), None, "a placeholder");
        view.open_offer(offered());
        assert!(view.offer_dialog().is_none());
        assert_eq!(view.take_offer_choice(), None);
    }

    #[test]
    fn a_mission_text_shown_holds_the_bars_wait() {
        let mut view = drinking_with_offers();
        click_item(&mut view, 11);
        view.show_mission_texts(vec![MissionText {
            kind: MissionTextKind::Refuse,
            mission: MissionId(141),
            text: "Pity.".to_owned(),
            graphic: None,
            movie: None,
            map: None,
        }]);
        view.tick(FIRST_OFFER_DELAY);
        assert_eq!(view.take_offer_request(), None);
        done(&mut view);
        view.tick(FIRST_OFFER_DELAY);
        assert_eq!(view.take_offer_request(), Some(BAR));
    }

    #[test]
    fn without_mission_screens_the_bar_asks_for_no_offer() {
        let mut view = drinking(true);
        click_item(&mut view, 11);
        view.tick(Duration::from_secs(10));
        assert_eq!(view.take_offer_request(), None);
        view.open_offer(offered());
        view.rearm_offers(Duration::ZERO);
        assert_eq!(view.take_offer_choice(), None);
    }

    // Mission info.

    use crate::spaceport::missions::ABORT_ITEM;
    use nova_sim::InfoRow;

    /// The mission info screen's shape, smaller: 300 x 150, fixed at
    /// (0, 0), with Done (1), the list (2), the heading (3), the text (4),
    /// Abort (5) and the date (7).
    fn info_template() -> Template {
        let mut items: Vec<ItemTemplate> = (0..7)
            .map(|_| ItemTemplate {
                bounds: rect(0.0, 300.0, 1.0, 1.0),
                enabled: true,
                kind: ItemSpec::User,
            })
            .collect();
        let mut place = |number: usize, x, y, w, h| {
            items[number - 1].bounds = rect(x, y, w, h);
        };
        place(1, 190.0, 120.0, 99.0, 25.0);
        place(2, 10.0, 24.0, 100.0, 84.0);
        place(3, 10.0, 1.0, 190.0, 12.0);
        place(4, 120.0, 26.0, 170.0, 80.0);
        place(5, 60.0, 120.0, 99.0, 25.0);
        place(7, 200.0, 4.0, 90.0, 12.0);
        Template {
            bounds: rect(0.0, 0.0, 300.0, 150.0),
            placement: Placement::Fixed,
            items,
        }
    }

    pub(in crate::spaceport) fn actives(rows: usize) -> MissionInfo {
        MissionInfo {
            rows: (0..rows)
                .map(|n| InfoRow {
                    slot: n as u8,
                    name: format!("Mission {n}"),
                    failed: false,
                    text: format!("Text {n}."),
                    abortable: true,
                })
                .collect(),
            today: "June 23rd, 1177 NC".to_owned(),
        }
    }

    #[test]
    fn the_mission_info_key_asks_from_the_spaceport_the_bar_and_the_computer() {
        let mut view = missioned();
        view.input(&key(MISSION_INFO_KEY));
        assert!(view.take_mission_info_request(), "the spaceport");
        assert!(!view.take_mission_info_request(), "once");
        let mut view = drinking_with_offers();
        click_item(&mut view, 11);
        view.input(&key(MISSION_INFO_KEY));
        assert!(view.take_mission_info_request(), "the bar");
        let mut view = missioned();
        click_item(&mut view, 10);
        view.open_mission_computer(Ok(offers()));
        view.input(&key(MISSION_INFO_KEY));
        assert!(view.take_mission_info_request(), "the computer");
        let mut view = earth();
        view.input(&key(MISSION_INFO_KEY));
        assert!(!view.take_mission_info_request(), "no mission screens");
    }

    /// [`venues`], landed on and the venue at `item` opened, the
    /// spaceport's offer taken.
    fn shopping_at(item: usize) -> SpaceportView {
        let mut view = venues();
        assert_eq!(view.take_offer_request(), Some(SPACEPORT));
        click_item(&mut view, item);
        view
    }

    #[test]
    fn the_mission_info_key_asks_from_each_shop_and_opens_over_it() {
        for (shop, item) in [(TRADE_CENTER, 7), (OUTFITTER, 8), (SHIPYARD, 9)] {
            let open = |view: &SpaceportView| match shop {
                TRADE_CENTER => view.open_trade().is_some(),
                OUTFITTER => view.open_outfitter().is_some(),
                _ => view.open_shipyard().is_some(),
            };
            let mut view = shopping_at(item);
            assert!(open(&view), "{shop}");
            view.input(&key(MISSION_INFO_KEY));
            assert!(view.take_mission_info_request(), "{shop}");
            assert!(!view.take_mission_info_request(), "{shop}: once");
            view.open_mission_info(actives(0));
            assert!(view.mission_info().is_none(), "{shop}: nothing to show");
            view.take_sounds();
            view.open_mission_info(actives(2));
            assert!(view.mission_info().is_some(), "{shop}");
            assert_eq!(
                view.take_sounds(),
                [Sound::Ui(UiSound::MissionInfoOpened)],
                "{shop}"
            );
            assert!(open(&view), "{shop}: still open under it");
            view.input(&key(Key::Escape));
            assert!(view.mission_info().is_none(), "{shop}: Escape closed it");
            assert_eq!(
                view.take_sounds(),
                [Sound::Ui(UiSound::MissionInfoClosed)],
                "{shop}"
            );
            assert!(open(&view), "{shop}: and the shop is back");
            view.input(&key(MISSION_INFO_KEY));
            assert!(view.take_mission_info_request(), "{shop}: takes keys again");
        }
    }

    #[test]
    fn closing_the_mission_info_over_a_shop_lets_go_of_held_keys() {
        for close_by_abort in [false, true] {
            let mut view = shopping_at(7);
            view.input(&key(Key::Alt));
            view.input(&key(MISSION_INFO_KEY));
            assert!(view.take_mission_info_request());
            view.open_mission_info(actives(1));
            assert!(view.mission_info().is_some());
            if close_by_abort {
                view.set_mission_info(actives(0));
            } else {
                view.input(&key(Key::Escape));
            }
            assert!(view.mission_info().is_none(), "closed");
            view.input(&key(Key::Char('b')));
            assert_eq!(
                view.take_trade().map(|order| order.lot),
                Some(Lot::Click),
                "Alt let go (abort: {close_by_abort}), no quantity dialog"
            );
        }
    }

    #[test]
    fn the_mission_info_key_asks_from_an_offer_over_each_venue() {
        for venue in [SPACEPORT, BAR, TRADE_CENTER, OUTFITTER, SHIPYARD] {
            let mut view = venues();
            assert_eq!(view.take_offer_request(), Some(SPACEPORT));
            if venue != SPACEPORT {
                let item = match venue {
                    BAR => 11,
                    TRADE_CENTER => 7,
                    OUTFITTER => 8,
                    _ => 9,
                };
                click_item(&mut view, item);
                if view.take_outfitter_opened() {
                    view.set_opened_outfitter(tanks(1));
                }
                view.tick(FIRST_OFFER_DELAY);
                assert_eq!(view.take_offer_request(), Some(venue));
            }
            view.open_offer(offered());
            view.input(&key(MISSION_INFO_KEY));
            assert!(view.take_mission_info_request(), "{venue}");
            assert!(!view.take_mission_info_request(), "{venue}: once");
            assert!(view.offer_dialog().is_some(), "{venue}: still open");
        }
    }

    #[test]
    fn closing_the_mission_info_by_its_last_abort_beeps() {
        let mut view = missioned();
        view.open_mission_info(actives(1));
        view.take_sounds();
        view.set_mission_info(actives(1));
        assert_eq!(view.take_sounds(), [], "still listing");
        view.set_mission_info(actives(0));
        assert_eq!(view.take_sounds(), [Sound::Ui(UiSound::MissionInfoClosed)]);
    }

    /// Fills nothing: the stored name as it is.
    struct AsStored;

    impl OfferNames for AsStored {
        fn munged(&mut self, _: MissionId, raw: &str) -> Option<String> {
            Some(raw.to_owned())
        }
    }

    #[test]
    fn the_mission_screens_ask_for_their_selected_texts_and_show_them_loaded() {
        let mut view = missioned();
        assert_eq!(view.take_computer_text_request(), None, "nothing open");
        assert_eq!(view.take_info_text_request(), None);
        click_item(&mut view, 10);
        view.open_mission_computer(Ok(offers()));
        assert_eq!(view.take_computer_text_request(), Some(MissionId(130)));
        assert_eq!(view.take_computer_text_request(), None, "once");
        view.show_computer_row(
            &ComputerRow {
                mission: MissionId(130),
                name: "Run, loaded".to_owned(),
                raw_name: "Run <DST>".to_owned(),
                text: "Go, loaded.".to_owned(),
            },
            &mut AsStored,
        );
        let computer = view.mission_computer().expect("open");
        let item = |item| {
            computer
                .dialog()
                .and_then(|dialog| dialog.text(item))
                .expect("a text item")
                .lines()
                .join(" ")
        };
        assert_eq!(item(crate::spaceport::missions::TEXT_ITEM), "Go, loaded.");
        assert_eq!(item(crate::spaceport::missions::NAME_ITEM), "Run <DST>");
        view.open_mission_info(actives(2));
        assert_eq!(view.take_info_text_request(), Some(0));
        view.show_info_text(0, "Text 0, loaded.");
        let text = view
            .mission_info()
            .and_then(MissionInfoScreen::dialog)
            .and_then(|dialog| dialog.text(crate::spaceport::missions::INFO_TEXT_ITEM))
            .expect("the text")
            .lines()
            .join(" ");
        assert_eq!(text, "Text 0, loaded.");
    }

    #[test]
    fn mission_info_opens_over_whatever_is_open_only_with_missions_to_show() {
        let mut view = missioned();
        click_item(&mut view, 10);
        view.open_mission_computer(Ok(offers()));
        view.open_mission_info(actives(0));
        assert!(view.mission_info().is_none(), "nothing to show");
        view.open_mission_info(actives(2));
        let info = view.mission_info().expect("open");
        assert_eq!(info.problem(), None, "laid out by the template given");
        assert_eq!(info.info(), &actives(2));
        let commands = drawn(&view);
        let mut over = DrawList::new();
        info.draw(&mut over);
        let over: Vec<_> = over.iter().cloned().collect();
        assert_eq!(commands[commands.len() - over.len()..], over[..]);
        view.input(&key(Key::Escape));
        assert!(view.mission_info().is_none(), "Escape closed it");
        assert!(view.mission_computer().is_some(), "and nothing below");
    }

    // Player info.

    use crate::ui::player_info::{
        BACKGROUND, BackgroundSizes, PLAYER_INFO_KEY, PictureSize, PlayerInfoButton,
        PlayerInfoScreen as InfoScreen, Tab,
    };
    use nova_sim::PlayerInfo;

    /// The player info screen's shape: stock `DLOG` 1017's items, fixed
    /// at (0, 0).
    fn player_info_template() -> Template {
        let item = |x, y, w, h| ItemTemplate {
            bounds: rect(x, y, w, h),
            enabled: true,
            kind: ItemSpec::User,
        };
        Template {
            bounds: rect(0.0, 0.0, 413.0, 227.0),
            placement: Placement::Fixed,
            items: vec![
                item(293.0, 195.0, 99.0, 25.0),
                item(7.0, 8.0, 99.0, 25.0),
                item(107.0, 8.0, 99.0, 25.0),
                item(207.0, 8.0, 99.0, 25.0),
                item(307.0, 8.0, 99.0, 25.0),
                item(4.0, 40.0, 405.0, 141.0),
                item(60.0, 195.0, 150.0, 25.0),
            ],
        }
    }

    pub(in crate::spaceport) fn pilot_info() -> PlayerInfo {
        PlayerInfo {
            pilot_name: "Ada".to_owned(),
            date: String::new(),
            system: String::new(),
            legal_status: String::new(),
            combat_rating: String::new(),
            ship_name: String::new(),
            ship_class: String::new(),
            turn_rate: 0,
            accel: 0,
            max_speed: 0,
            credits: 0,
            shield: None,
            armor: None,
            energy: None,
            income: None,
            fleet: false,
            cargo: Some("Five tons of food.".to_owned()),
            extras: None,
            honors: None,
            jettisonable: true,
            words: nova_sim::player_info::Words::STOCK,
            cheat_ids: nova_sim::player_info::CheatIds::default(),
            cheats: false,
        }
    }

    /// The stock background pictures' sizes.
    fn stock_background() -> BackgroundSizes {
        let size = |width, height| Some(PictureSize { width, height });
        BackgroundSizes {
            upper: size(413.0, 40.0),
            middle: size(413.0, 215.0),
            lower: size(413.0, 40.0),
        }
    }

    /// `view` with the player info screen's dialogs given, its Yes/No
    /// dialog the text's shape, and its background `sizes`.
    fn informed_with(view: SpaceportView, sizes: BackgroundSizes) -> SpaceportView {
        view.with_player_info(Ok(player_info_template()), Ok(text_template()), sizes)
    }

    /// `view` with the player info screen's dialogs given, its Yes/No
    /// dialog the text's shape, over the stock pictures.
    pub(in crate::spaceport) fn informed(view: SpaceportView) -> SpaceportView {
        informed_with(view, stock_background())
    }

    /// Presses P on `view`, which asked nothing before: whether it asks.
    fn asks_player_info(view: &mut SpaceportView) -> bool {
        assert!(!view.take_player_info_request(), "nothing asked before");
        view.input(&key(PLAYER_INFO_KEY));
        view.take_player_info_request()
    }

    #[test]
    fn p_asks_for_the_player_info_from_the_port_and_every_dialog_that_reads_it() {
        let mut view = earth();
        view.input(&Input::Key {
            key: PLAYER_INFO_KEY,
            pressed: true,
            repeat: true,
        });
        assert!(!view.take_player_info_request(), "not a repeat");
        assert!(asks_player_info(&mut view), "the spaceport");
        assert!(!view.take_player_info_request(), "once");
        let mut view = drinking_with_offers();
        click_item(&mut view, 11);
        assert!(asks_player_info(&mut view), "the bar");
        view.open_hire(hirelings(1, true));
        assert!(view.open_bar().and_then(BarScreen::hire_screen).is_some());
        assert!(asks_player_info(&mut view), "the hire screen");
        let mut view = trading(Ok(trade_template()));
        click_item(&mut view, 7);
        assert!(asks_player_info(&mut view), "the trade center");
        let mut view = shipbuying(Ok(shipyard_template()));
        click_item(&mut view, 9);
        assert!(asks_player_info(&mut view), "the shipyard");
        view.open_ship_naming(&naming());
        assert!(!asks_player_info(&mut view), "its name prompt");
        let mut view = missioned();
        click_item(&mut view, 10);
        view.open_mission_computer(Ok(offers()));
        assert!(asks_player_info(&mut view), "the mission computer");
        let mut view = missioned();
        view.open_offer(offered());
        assert!(asks_player_info(&mut view), "the spaceport's offer");
        assert!(view.offer_dialog().is_some(), "which stays open");
        let mut view = outfitting(Ok(outfit_template()));
        click_item(&mut view, 8);
        assert!(view.open_outfitter().is_some());
        assert!(!asks_player_info(&mut view), "not the outfitter");
    }

    #[test]
    fn p_asks_from_an_offer_over_each_venue() {
        for venue in [BAR, TRADE_CENTER, OUTFITTER, SHIPYARD] {
            let mut view = venues();
            assert_eq!(view.take_offer_request(), Some(SPACEPORT));
            let item = match venue {
                BAR => 11,
                TRADE_CENTER => 7,
                OUTFITTER => 8,
                _ => 9,
            };
            click_item(&mut view, item);
            if view.take_outfitter_opened() {
                view.set_opened_outfitter(tanks(1));
            }
            view.tick(FIRST_OFFER_DELAY);
            assert_eq!(view.take_offer_request(), Some(venue));
            view.open_offer(offered());
            assert!(view.offer_dialog().is_some(), "{venue}");
            assert!(asks_player_info(&mut view), "{venue}");
            assert!(view.offer_dialog().is_some(), "{venue}: still open");
        }
    }

    // The offers' and the mission texts' movies.

    use crate::movie::fixture::{Call, MockMoviePlayer};
    use crate::movie::{MOVIE_FADE_FOR, MoviePlayer};
    use nova_sim::{DescMovie, MissionShown};

    fn movies_of(player: &Rc<MockMoviePlayer>) -> Movies {
        Movies::new(Rc::clone(player) as Rc<dyn MoviePlayer>, true)
    }

    /// An offer movie the session queued, `file` with `flags`.
    fn queued_movie(file: &str, flags: u16) -> MissionShown {
        MissionShown::Movie(DescMovie::from_desc(file, flags).expect("named"))
    }

    fn filmed_offer(flags: u16) -> OfferPrompt {
        OfferPrompt {
            movie: DescMovie::from_desc("Offer.mov", flags),
            ..offered()
        }
    }

    fn offer_shown(view: &SpaceportView) -> bool {
        view.offer_dialog().is_some_and(|offer| {
            let mut list = DrawList::new();
            offer.draw(&mut list);
            list.iter().any(
                |command| matches!(command, DrawCommand::Text { text, .. } if text == "Ferry this."),
            )
        })
    }

    #[test]
    fn the_spaceports_offer_plays_its_before_movie_and_ticks_with_the_spaceport() {
        let player = MockMoviePlayer::playing((160, 120), 2);
        let mut view = missioned().with_movies(movies_of(&player));
        view.open_offer(filmed_offer(0));
        assert!(
            player.calls().is_empty(),
            "staged until the spaceport ticks it"
        );
        assert!(!offer_shown(&view), "not until the movie ends");
        view.tick(ms(100));
        assert_eq!(player.calls()[0], Call::Start("Offer.mov".to_owned()));
        view.tick(ms(100));
        view.tick(ms(100));
        assert!(offer_shown(&view));
    }

    /// The movies `player` was asked to start, in order.
    fn started(player: &MockMoviePlayer) -> Vec<String> {
        player
            .calls()
            .into_iter()
            .filter_map(|call| match call {
                Call::Start(file) => Some(file),
                _ => None,
            })
            .collect()
    }

    /// The Comp text, its `dësc` naming Comp.mov before it.
    fn filmed_comp() -> MissionText {
        MissionText {
            kind: MissionTextKind::Comp,
            mission: MissionId(130),
            text: "Well done.".to_owned(),
            graphic: None,
            movie: DescMovie::from_desc("Comp.mov", 0),
            map: None,
        }
    }

    #[test]
    fn an_offer_opened_over_a_mission_text_waits_for_its_movie() {
        let player = MockMoviePlayer::playing((160, 120), 2);
        let mut view = earth().with_movies(movies_of(&player)).with_missions(
            Ok(computer_template()),
            Ok(info_template()),
            pictured_offers(),
            pictured_texts(),
        );
        view.open_offer(filmed_offer(0));
        view.show_mission_texts(vec![filmed_comp()]);
        for _ in 0..4 {
            view.tick(ms(100));
        }
        assert_eq!(
            started(&player),
            ["Comp.mov"],
            "`_PlayQTMovie` is modal: the offer's movie waits for the text and its movie"
        );
        assert_eq!(shown_text(&view).as_deref(), Some("Well done."));
        done(&mut view);
        view.tick(ms(100));
        assert_eq!(started(&player), ["Comp.mov", "Offer.mov"]);
    }

    #[test]
    fn a_bar_offer_opened_over_a_mission_text_waits_for_its_movie() {
        let player = MockMoviePlayer::playing((160, 120), 2);
        let mut view = venues().with_movies(movies_of(&player));
        assert_eq!(view.take_offer_request(), Some(SPACEPORT));
        click_item(&mut view, 11);
        view.tick(FIRST_OFFER_DELAY);
        assert_eq!(view.take_offer_request(), Some(BAR));
        view.open_offer(filmed_offer(0));
        view.show_mission_texts(vec![filmed_comp()]);
        for _ in 0..4 {
            view.tick(ms(100));
        }
        assert_eq!(started(&player), ["Comp.mov"]);
        done(&mut view);
        view.tick(ms(100));
        assert_eq!(started(&player), ["Comp.mov", "Offer.mov"]);
    }

    fn offer_drawn(view: &SpaceportView) -> bool {
        drawn(view).iter().any(
            |command| matches!(command, DrawCommand::Text { text, .. } if text == "Ferry this."),
        )
    }

    #[test]
    fn a_refused_offer_without_a_refuse_text_stays_drawn_under_its_after_movie() {
        let player = MockMoviePlayer::playing((160, 120), 2);
        let mut view = missioned().with_movies(movies_of(&player));
        view.open_offer(offered());
        view.input(&key(Key::Char('n')));
        assert_eq!(
            view.take_offer_choice(),
            Some((MissionId(141), OfferChoice::Refuse))
        );
        view.show_mission_texts([queued_movie("Offer.mov", 0x0001)]);
        view.tick(ms(100));
        assert!(
            offer_drawn(&view),
            "not hidden (@0xa2e70-0xa2e78), until `DisposeDialogNoFlash` @0xa361e"
        );
        assert!(view.offer_dialog().is_none(), "answered, it takes nothing");
        for _ in 0..3 {
            view.tick(ms(100));
        }
        assert!(!offer_drawn(&view), "gone once the movie has closed");
    }

    #[test]
    fn a_refused_offer_with_nothing_to_show_closes_at_once() {
        let mut view = missioned();
        view.open_offer(offered());
        view.input(&key(Key::Char('n')));
        assert!(view.take_offer_choice().is_some());
        view.show_mission_texts(Vec::<MissionShown>::new());
        assert!(!offer_drawn(&view));
    }

    #[test]
    fn a_refused_offer_that_showed_a_refuse_text_is_released_at_once() {
        let mut view = missioned();
        view.open_offer(offered());
        view.input(&key(Key::Char('n')));
        assert!(view.take_offer_choice().is_some());
        view.release_answered_offer();
        view.show_mission_texts(vec![MissionText {
            movie: None,
            ..filmed_comp()
        }]);
        assert!(!offer_drawn(&view), "hidden for its RefuseText (@0xa2e8e)");
    }

    /// [`venues`] playing movies on `player`, landed on and `venue`
    /// opened, offering [`offered`], refused.
    fn refused_over(venue: i16, player: &Rc<MockMoviePlayer>) -> SpaceportView {
        let mut view = venues().with_movies(movies_of(player));
        assert_eq!(view.take_offer_request(), Some(SPACEPORT));
        click_item(&mut view, venue_item(venue));
        if view.take_outfitter_opened() {
            view.set_opened_outfitter(tanks(1));
        }
        view.tick(FIRST_OFFER_DELAY);
        assert_eq!(view.take_offer_request(), Some(venue));
        view.open_offer(offered());
        assert!(offer_drawn(&view), "{venue}: offered");
        view.input(&key(Key::Char('n')));
        assert_eq!(
            view.take_offer_choice(),
            Some((MissionId(141), OfferChoice::Refuse)),
            "{venue}"
        );
        view
    }

    #[test]
    fn each_venues_refused_offer_stays_drawn_under_its_after_movie_or_hides_for_a_refuse_text() {
        for venue in OFFER_VENUES {
            let player = MockMoviePlayer::playing((160, 120), 2);
            let mut view = refused_over(venue, &player);
            view.show_mission_texts([queued_movie("Offer.mov", 0x0001)]);
            view.tick(ms(100));
            assert_eq!(started(&player), ["Offer.mov"], "{venue}");
            assert!(
                offer_drawn(&view),
                "{venue}: over the venue, not hidden (@0xa2e70-0xa2e78)"
            );
            for _ in 0..3 {
                view.tick(ms(100));
            }
            assert!(!view.mission_dialogs_open(), "{venue}: the movie closed");
            assert!(
                !offer_drawn(&view),
                "{venue}: disposed once the movie has closed (@0xa361e)"
            );

            let player = MockMoviePlayer::playing((160, 120), 2);
            let mut view = refused_over(venue, &player);
            assert!(offer_drawn(&view), "{venue}: drawn until released");
            view.release_answered_offer();
            assert!(
                !offer_drawn(&view),
                "{venue}: hidden for its RefuseText (@0xa2e8e)"
            );
            view.show_mission_texts(vec![MissionText {
                movie: None,
                ..filmed_comp()
            }]);
            assert!(!offer_drawn(&view), "{venue}: not under the text");
        }
    }

    #[test]
    fn an_accepted_offer_closes_at_once() {
        let mut view = missioned();
        view.open_offer(offered());
        view.input(&key(Key::Enter));
        assert!(view.take_offer_choice().is_some());
        view.show_mission_texts(vec![MissionText {
            movie: None,
            ..filmed_comp()
        }]);
        assert!(!offer_drawn(&view), "hidden before the accept (@0xa2df3)");
    }

    #[test]
    fn the_computers_refusal_plays_the_after_movie_it_is_given() {
        let player = MockMoviePlayer::playing((160, 120), 1);
        let mut view = missioned().with_movies(movies_of(&player));
        view.open_mission_computer_after(
            Err(ComputerRefusal::NoneAvailable),
            DescMovie::from_desc("Port.mov", 0x0001),
        );
        assert!(shown_text(&view).is_some(), "the refusal first");
        assert!(started(&player).is_empty());
        done(&mut view);
        assert_eq!(
            started(&player),
            ["Port.mov"],
            "the STR# text's after-check reads the last dësc loaded (@0x5b40b)"
        );
    }

    #[test]
    fn closing_the_mission_info_over_an_offer_asks_to_reload_it() {
        let mut view = missioned();
        view.open_offer(offered());
        view.input(&key(MISSION_INFO_KEY));
        assert!(view.take_mission_info_request());
        view.open_mission_info(actives(1));
        assert_eq!(view.take_offer_reload(), None, "not while it is open");
        view.input(&key(Key::Escape));
        assert_eq!(
            view.take_offer_reload(),
            Some(MissionId(141)),
            "`_OfferOneMission` reloads the offer dësc (@0xa335c)"
        );
        assert_eq!(view.take_offer_reload(), None, "once");
        let mut view = missioned();
        view.open_mission_info(actives(1));
        view.input(&key(Key::Escape));
        assert_eq!(view.take_offer_reload(), None, "no offer under it");
    }

    #[test]
    fn each_venues_offer_plays_its_movie() {
        for venue in [BAR, TRADE_CENTER, OUTFITTER, SHIPYARD] {
            let player = MockMoviePlayer::playing((160, 120), 1);
            let mut view = venues().with_movies(movies_of(&player));
            assert_eq!(view.take_offer_request(), Some(SPACEPORT));
            let item = match venue {
                BAR => 11,
                TRADE_CENTER => 7,
                OUTFITTER => 8,
                _ => 9,
            };
            click_item(&mut view, item);
            if view.take_outfitter_opened() {
                view.set_opened_outfitter(tanks(1));
            }
            view.tick(FIRST_OFFER_DELAY);
            assert_eq!(view.take_offer_request(), Some(venue));
            view.open_offer(filmed_offer(0));
            assert!(player.calls().is_empty(), "{venue}: staged");
            assert!(!offer_shown(&view), "{venue}");
            view.tick(ms(100));
            assert!(offer_shown(&view), "{venue}: the venue ticks it");
            view.input(&key(Key::Enter));
            view.open_offer(filmed_offer(0x0001));
            view.input(&key(Key::Enter));
            assert!(view.take_offer_choice().is_some(), "{venue}");
            view.show_mission_texts([queued_movie("Offer.mov", 0x0001)]);
            assert_eq!(player.calls().len(), 4, "{venue}: {:?}", player.calls());
        }
    }

    #[test]
    fn an_offers_after_movie_plays_after_the_texts_its_answer_shows() {
        let player = MockMoviePlayer::playing((160, 120), 1);
        let mut view = missioned().with_movies(movies_of(&player));
        view.open_offer(filmed_offer(0x0001));
        assert!(offer_shown(&view), "the dialog first");
        view.input(&key(Key::Enter));
        assert_eq!(
            view.take_offer_choice(),
            Some((MissionId(141), OfferChoice::Accept))
        );
        view.show_mission_texts([
            MissionShown::Text(MissionText {
                kind: MissionTextKind::Brief,
                mission: MissionId(141),
                text: "Go.".to_owned(),
                graphic: None,
                movie: None,
                map: None,
            }),
            queued_movie("Offer.mov", 0x0001),
        ]);
        assert!(player.calls().is_empty(), "after the brief");
        done(&mut view);
        assert_eq!(player.calls(), [Call::Start("Offer.mov".to_owned())]);
        assert!(view.mission_dialogs_open(), "while it plays");
        view.tick(ms(100));
        assert!(!view.mission_dialogs_open());
    }

    #[test]
    fn a_movie_played_over_the_spaceport_holds_its_input_and_a_leave() {
        let player = MockMoviePlayer::playing((160, 120), 1);
        let mut view = missioned().with_movies(movies_of(&player));
        view.show_mission_texts([queued_movie("Accepted.mov", 0)]);
        assert!(view.holds_leave());
        view.input(&key(Key::Escape));
        assert!(!view.left(), "the movie took the Escape");
        view.tick(ms(100));
        assert!(view.left(), "which reached the spaceport as it ended");
    }

    #[test]
    fn the_spaceport_plays_while_a_mission_texts_movie_or_an_offers_is_in_front() {
        let player = MockMoviePlayer::playing((160, 120), 1);
        let mut view = missioned().with_movies(movies_of(&player));
        assert!(!view.playing(), "nothing open");
        view.show_mission_texts([queued_movie("Accepted.mov", 0)]);
        assert!(view.playing(), "a mission text's");
        view.tick(ms(100));
        assert!(!view.playing(), "ended");
        view.open_offer(filmed_offer(0));
        assert!(view.playing(), "an offer's");
        view.tick(ms(100));
        assert!(offer_shown(&view));
        assert!(!view.playing(), "the offer's dialog shows");
    }

    #[test]
    fn a_cinematic_movie_over_the_spaceport_hides_the_pointer() {
        let player = MockMoviePlayer::playing((160, 120), 1);
        let mut view = missioned().with_movies(movies_of(&player));
        assert!(!view.hides_pointer());
        view.show_mission_texts([queued_movie("Accepted.mov", 0x0004)]);
        view.tick(MOVIE_FADE_FOR);
        assert!(view.hides_pointer(), "a mission text's");
        let mut view = missioned().with_movies(movies_of(&player));
        view.open_offer(filmed_offer(0x0004));
        view.tick(MOVIE_FADE_FOR);
        assert!(view.hides_pointer(), "an offer's");
    }

    #[test]
    fn the_player_info_opens_over_whatever_is_open_and_closes_back_to_it() {
        let mut view = informed(missioned());
        click_item(&mut view, 10);
        view.open_mission_computer(Ok(offers()));
        view.open_player_info(pilot_info());
        let screen = view.player_info().expect("open");
        assert_eq!(screen.problem(), None, "laid out by the template given");
        assert_eq!(screen.info(), &pilot_info());
        let commands = drawn(&view);
        let mut over = DrawList::new();
        screen.draw(&mut over);
        let over: Vec<_> = over.iter().cloned().collect();
        assert_eq!(
            commands[commands.len() - over.len()..],
            over[..],
            "drawn last"
        );
        view.input(&key(Key::Tab));
        assert_eq!(view.player_info().map(InfoScreen::tab), Some(Tab::Cargo));
        assert!(!view.take_player_info_request(), "it takes the input first");
        view.input(&key(Key::Escape));
        assert!(view.player_info().is_none(), "Escape closed it");
        assert!(view.mission_computer().is_some(), "and nothing below");
        view.input(&key(PLAYER_INFO_KEY));
        assert!(view.take_player_info_request(), "asked again");
    }

    #[test]
    fn the_player_info_draws_its_background_from_the_sizes_given() {
        let sizes = BackgroundSizes {
            middle: None,
            ..stock_background()
        };
        let mut view = informed_with(missioned(), sizes);
        view.open_player_info(pilot_info());
        let screen = view.player_info().expect("open");
        let mut list = DrawList::new();
        screen.draw(&mut list);
        let pictures: Vec<i16> = list
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Clipped { command, .. } => match **command {
                    DrawCommand::StretchedPicture { image, .. }
                    | DrawCommand::Picture { image, .. } => Some(image),
                    _ => None,
                },
                _ => None,
            })
            .filter(|image| BACKGROUND.contains(image))
            .map(|image| image.id)
            .collect();
        assert_eq!(pictures, [8518, 8520], "no middle given, none drawn");
    }

    #[test]
    fn without_its_dialogs_the_player_info_says_why() {
        let mut view = missioned();
        view.open_player_info(pilot_info());
        assert_eq!(
            view.player_info().and_then(InfoScreen::problem),
            Some("the player info dialog was not given")
        );
        let mut broken = informed(view_of(&FakePort {
            port: Err("no stellar".to_owned()),
            ..catalog()
        }));
        broken.open_player_info(pilot_info());
        assert!(broken.player_info().is_none(), "no spaceport, no screen");
    }

    /// The ring given after the missions, as the app gives it, reaches the
    /// spaceport's mission texts, its mission info screen and its landing
    /// offer.
    #[test]
    fn the_missions_given_before_the_focus_ring_take_it() {
        for ring in [FocusRing::Tab, FocusRing::Off] {
            let mut view = missioned().with_focus_ring(ring);
            view.show_text("Nothing here.");
            assert_eq!(
                view.mission_text().map(|text| text.dialog().focus_ring()),
                Some(ring),
                "the mission text"
            );
            view.input(&key(Key::Enter));
            assert!(view.mission_text().is_none());
            view.open_mission_info(actives(1));
            assert_eq!(
                view.mission_info()
                    .and_then(MissionInfoScreen::dialog)
                    .map(Dialog::focus_ring),
                Some(ring),
                "the mission info screen"
            );
            let mut view = missioned().with_focus_ring(ring);
            view.open_offer(crate::spaceport::offer::fixture::prompt());
            assert_eq!(
                view.offer_dialog()
                    .and_then(OfferDialog::dialog)
                    .map(Dialog::focus_ring),
                Some(ring),
                "the landing offer"
            );
            view.input(&key(Key::Tab));
            view.input(&key(Key::Space));
            assert_eq!(
                view.take_offer_choice().is_some(),
                ring == FocusRing::Tab,
                "Tab then Space on the landing offer, {ring:?}"
            );
        }
    }

    /// The spaceport passes its focus ring to the mission computer and
    /// the player info screen it opens, down to its Jettison question
    /// (whose keys `ui::player_info`'s own tests drive).
    #[test]
    fn the_computer_and_the_player_info_opened_take_the_spaceports_focus_ring() {
        for ring in [FocusRing::Tab, FocusRing::Off] {
            let mut view = informed(missioned()).with_focus_ring(ring);
            click_item(&mut view, 10);
            view.open_mission_computer(Ok(offers()));
            assert_eq!(
                view.mission_computer()
                    .and_then(MissionComputerScreen::dialog)
                    .map(Dialog::focus_ring),
                Some(ring),
                "the computer"
            );
            view.open_player_info(pilot_info());
            let at = |view: &SpaceportView, button| {
                view.player_info()
                    .and_then(|screen| screen.button_bounds(button))
                    .expect("laid out")
                    .center()
            };
            let cargo = at(&view, PlayerInfoButton::Tab(Tab::Cargo));
            click(&mut view, cargo);
            let jettison = at(&view, PlayerInfoButton::Jettison);
            click(&mut view, jettison);
            assert_eq!(
                view.player_info()
                    .and_then(InfoScreen::confirm)
                    .map(Dialog::focus_ring),
                Some(ring),
                "the Jettison question"
            );
        }
    }

    #[test]
    fn its_jettison_is_taken_through_the_spaceport_once() {
        let mut view = informed(missioned());
        view.open_player_info(pilot_info());
        let at = |view: &SpaceportView, button| {
            view.player_info()
                .and_then(|screen| screen.button_bounds(button))
                .expect("laid out")
                .center()
        };
        let cargo = at(&view, PlayerInfoButton::Tab(Tab::Cargo));
        click(&mut view, cargo);
        let jettison = at(&view, PlayerInfoButton::Jettison);
        click(&mut view, jettison);
        assert!(
            view.player_info().is_some_and(InfoScreen::confirm_open),
            "asked"
        );
        assert!(!view.take_player_info_jettison());
        view.input(&key(Key::Enter));
        assert!(view.player_info().is_none(), "closed");
        assert!(view.take_player_info_jettison());
        assert!(!view.take_player_info_jettison(), "once");
    }

    #[test]
    fn the_screen_below_waits_while_it_is_open() {
        let mut view = informed(drinking_with_offers());
        assert_eq!(view.take_offer_request(), Some(SPACEPORT));
        click_item(&mut view, 11);
        view.open_player_info(pilot_info());
        view.tick(Duration::from_secs(10));
        assert_eq!(view.take_offer_request(), None, "the bar's wait held");
        view.input(&key(Key::Escape));
        view.tick(FIRST_OFFER_DELAY);
        assert_eq!(view.take_offer_request(), Some(BAR));
    }

    #[test]
    fn an_abort_is_taken_through_the_spaceport_and_the_info_handed_back() {
        let mut view = missioned();
        assert_eq!(view.take_mission_abort(), None);
        view.open_mission_info(actives(2));
        let at = view
            .mission_info()
            .and_then(MissionInfoScreen::dialog)
            .and_then(|dialog| dialog.item_bounds(ABORT_ITEM))
            .expect("Abort")
            .center();
        click(&mut view, at);
        assert_eq!(view.take_mission_abort(), Some(0));
        assert_eq!(view.take_mission_abort(), None, "once");
        view.set_mission_info(actives(1));
        assert_eq!(
            view.mission_info().map(|info| info.info().rows.len()),
            Some(1)
        );
        view.set_mission_info(actives(0));
        assert!(view.mission_info().is_none(), "closed with nothing left");
    }

    // The offer timer under what opens over a venue (`RuleKey::OfferClock`).

    use nova_sim::RuleSource;

    const OFFER_VENUES: [i16; 4] = [BAR, TRADE_CENTER, OUTFITTER, SHIPYARD];

    /// The spaceport button that opens `venue`.
    fn venue_item(venue: i16) -> usize {
        match venue {
            BAR => 11,
            TRADE_CENTER => 7,
            OUTFITTER => 8,
            _ => 9,
        }
    }

    /// [`venues`], with the player info screen's dialogs, its offers'
    /// timer on `clock`, landed on and `venue` opened, its first ask taken.
    fn asked_at(venue: i16, clock: RuleSource) -> SpaceportView {
        let mut view = informed(venues()).with_offer_clock(clock);
        assert_eq!(view.take_offer_request(), Some(SPACEPORT));
        click_item(&mut view, venue_item(venue));
        if view.take_outfitter_opened() {
            view.set_opened_outfitter(tanks(1));
        }
        view.tick(FIRST_OFFER_DELAY);
        assert_eq!(view.take_offer_request(), Some(venue));
        view
    }

    /// [`asked_at`], the offer refused and the venue rearmed for 500 ms.
    fn refused_at(venue: i16, clock: RuleSource) -> SpaceportView {
        let mut view = asked_at(venue, clock);
        view.open_offer(offered());
        view.input(&key(Key::Char('n')));
        assert!(view.take_offer_choice().is_some());
        view.rearm_offers(ms(500));
        view
    }

    /// What the spaceport opens over a venue.
    #[derive(Clone, Copy, Debug)]
    enum Overlay {
        MissionInfo,
        PlayerInfo,
        MissionText,
    }

    const OVERLAYS: [Overlay; 3] = [
        Overlay::MissionInfo,
        Overlay::PlayerInfo,
        Overlay::MissionText,
    ];

    fn open_overlay(view: &mut SpaceportView, overlay: Overlay) {
        match overlay {
            Overlay::MissionInfo => view.open_mission_info(actives(2)),
            Overlay::PlayerInfo => view.open_player_info(pilot_info()),
            Overlay::MissionText => view.show_mission_texts(vec![MissionText {
                kind: MissionTextKind::Refuse,
                mission: MissionId(141),
                text: "Pity.".to_owned(),
                graphic: None,
                movie: None,
                map: None,
            }]),
        }
        let open = view.mission_info().is_some()
            || view.player_info().is_some()
            || view.mission_text().is_some();
        assert!(open, "{overlay:?}");
    }

    fn close_overlay(view: &mut SpaceportView, overlay: Overlay) {
        match overlay {
            Overlay::MissionInfo | Overlay::PlayerInfo => {
                view.input(&key(Key::Escape));
            }
            Overlay::MissionText => done(view),
        }
        let open = view.mission_info().is_some()
            || view.player_info().is_some()
            || view.mission_text().is_some();
        assert!(!open, "{overlay:?}");
    }

    #[test]
    fn by_the_engine_an_offer_due_under_an_overlay_is_asked_for_as_it_closes() {
        for venue in OFFER_VENUES {
            for overlay in OVERLAYS {
                let mut view = refused_at(venue, RuleSource::Engine);
                open_overlay(&mut view, overlay);
                view.tick(Duration::from_secs(10));
                assert_eq!(view.take_offer_request(), None, "{venue} {overlay:?}");
                close_overlay(&mut view, overlay);
                view.tick(ms(0));
                assert_eq!(
                    view.take_offer_request(),
                    Some(venue),
                    "{venue} {overlay:?}"
                );
            }
        }
    }

    #[test]
    fn by_the_other_reading_the_wait_stops_under_an_overlay() {
        for venue in OFFER_VENUES {
            for overlay in OVERLAYS {
                let mut view = refused_at(venue, RuleSource::Bible);
                open_overlay(&mut view, overlay);
                view.tick(Duration::from_secs(10));
                assert_eq!(view.take_offer_request(), None, "{venue} {overlay:?}");
                close_overlay(&mut view, overlay);
                view.tick(ms(499));
                assert_eq!(view.take_offer_request(), None, "{venue} {overlay:?}");
                view.tick(ms(1));
                assert_eq!(
                    view.take_offer_request(),
                    Some(venue),
                    "{venue} {overlay:?}"
                );
            }
        }
    }

    #[test]
    fn mission_info_over_an_offer_runs_the_gap_from_the_ask_by_the_engine_alone() {
        for (clock, wait) in [(RuleSource::Engine, 0), (RuleSource::Bible, 500)] {
            for venue in OFFER_VENUES {
                let mut view = asked_at(venue, clock);
                view.open_offer(offered());
                view.input(&key(MISSION_INFO_KEY));
                assert!(view.take_mission_info_request(), "{venue}");
                view.open_mission_info(actives(2));
                view.tick(Duration::from_secs(10));
                view.input(&key(Key::Escape));
                assert!(view.mission_info().is_none(), "{venue}");
                view.input(&key(Key::Char('n')));
                assert!(view.take_offer_choice().is_some(), "{venue}");
                view.rearm_offers(ms(500));
                if wait > 0 {
                    view.tick(ms(wait - 1));
                    assert_eq!(view.take_offer_request(), None, "{clock:?} {venue}");
                    view.tick(ms(1));
                } else {
                    view.tick(ms(0));
                }
                assert_eq!(view.take_offer_request(), Some(venue), "{clock:?} {venue}");
            }
        }
    }

    #[test]
    fn the_offer_clock_reaches_each_venue_whenever_it_is_given() {
        for venue in OFFER_VENUES {
            let mut view = informed(venues());
            assert_eq!(view.take_offer_request(), Some(SPACEPORT));
            view = view.with_offer_clock(RuleSource::Bible);
            click_item(&mut view, venue_item(venue));
            if view.take_outfitter_opened() {
                view.set_opened_outfitter(tanks(1));
            }
            view.tick(FIRST_OFFER_DELAY);
            assert_eq!(view.take_offer_request(), Some(venue));
            view.rearm_offers(ms(500));
            open_overlay(&mut view, Overlay::PlayerInfo);
            view.tick(Duration::from_secs(10));
            close_overlay(&mut view, Overlay::PlayerInfo);
            view.tick(ms(0));
            assert_eq!(view.take_offer_request(), None, "{venue}: the Bible's");
        }
    }

    #[test]
    fn by_the_engine_the_bars_hire_screen_restarts_its_wait_through_the_spaceport() {
        let mut view = refused_at(BAR, RuleSource::Engine);
        assert!(!view.take_offer_restart());
        view.open_hire(hirelings(1, true));
        assert!(view.take_offer_restart());
        assert!(!view.take_offer_restart(), "once");
        let mut view = refused_at(BAR, RuleSource::Bible);
        view.open_hire(hirelings(1, true));
        assert!(!view.take_offer_restart(), "the other reading");
        for venue in [TRADE_CENTER, OUTFITTER, SHIPYARD] {
            let mut view = refused_at(venue, RuleSource::Engine);
            assert!(!view.take_offer_restart(), "{venue}");
        }
    }

    // A `Q`'s leave held (`_absquatulateStr`; see `holds_leave`).

    #[test]
    fn the_spaceport_and_the_screens_whose_filters_leave_hold_no_leave() {
        assert!(!earth().holds_leave());
        let mut view = missioned();
        click_item(&mut view, 10);
        view.open_mission_computer(Ok(offers()));
        assert!(view.mission_computer().is_some());
        assert!(!view.holds_leave(), "the computer (`_MissionFilter`)");
        let mut view = trading(Ok(trade_template()));
        click_item(&mut view, 7);
        assert!(view.open_trade().is_some());
        assert!(!view.holds_leave(), "the exchange (`_TradeFilter`)");
        let mut view = outfitting(Ok(outfit_template()));
        click_item(&mut view, 8);
        assert!(view.open_outfitter().is_some());
        assert!(!view.holds_leave(), "the outfitter (`_OutfitFilter`)");
        let mut view = shipbuying(Ok(shipyard_template()));
        click_item(&mut view, 9);
        assert!(view.open_shipyard().is_some());
        assert!(!view.holds_leave(), "the shipyard (`_ShipyardFilter`)");
        let mut view = drinking(true);
        click_item(&mut view, 11);
        assert!(!view.holds_leave(), "the bar (`_BarFilter`)");
        view.input(&key(Key::Char('h')));
        view.open_hire(hirelings(1, true));
        assert!(view.open_bar().and_then(BarScreen::hire_screen).is_some());
        assert!(!view.holds_leave(), "the hire screen (`_ShipyardFilter`)");
    }

    #[test]
    fn the_shipyards_info_panel_holds_a_leave() {
        let mut view = in_the_shipyards_panel();
        assert!(view.holds_leave(), "`_ShipDescFilter` never reads it");
        view.input(&key(Key::Enter));
        let shipyard = view.open_shipyard().expect("still open");
        assert!(shipyard.info_panel().is_none());
        assert!(!view.holds_leave(), "`_ShipyardFilter` answers it");
    }

    #[test]
    fn the_hire_screens_info_panel_holds_a_leave() {
        let mut view = in_the_hire_panel();
        assert!(view.holds_leave(), "`_ShipDescFilter` never reads it");
        view.input(&key(Key::Enter));
        let hire = view.open_bar().and_then(BarScreen::hire_screen);
        assert!(hire.is_some_and(|hire| hire.info_panel().is_none()));
        assert!(!view.holds_leave(), "`_ShipyardFilter` answers it");
    }

    #[test]
    fn a_mission_text_holds_a_leave_until_its_done() {
        let mut view = missioned();
        let text = MissionText {
            graphic: None,
            kind: MissionTextKind::Comp,
            mission: MissionId(130),
            text: "Well done.".to_owned(),
            movie: None,
            map: None,
        };
        view.show_mission_texts(vec![text.clone(), text]);
        assert!(view.holds_leave());
        done(&mut view);
        assert!(view.holds_leave(), "the second");
        done(&mut view);
        assert!(!view.holds_leave());
    }

    #[test]
    fn an_offer_and_the_mission_info_over_it_hold_a_leave() {
        let mut view = missioned();
        view.open_offer(offered());
        assert!(view.holds_leave(), "the offer");
        view.open_mission_info(actives(1));
        assert!(view.holds_leave(), "the info over it");
        view.input(&key(Key::Escape));
        assert!(view.mission_info().is_none());
        assert!(view.holds_leave(), "the offer still");
        let mut view = missioned();
        view.open_mission_info(actives(1));
        assert!(view.holds_leave(), "the info alone");
        view.input(&key(Key::Escape));
        assert!(!view.holds_leave());
    }

    #[test]
    fn the_player_info_holds_a_leave() {
        let mut view = informed(missioned());
        view.open_player_info(pilot_info());
        assert!(view.holds_leave());
        view.input(&key(Key::Escape));
        assert!(!view.holds_leave());
    }

    #[test]
    fn the_outfitters_alert_and_quantity_hold_a_leave() {
        let mut view = outfitting(Ok(outfit_template()));
        click_item(&mut view, 8);
        view.alert_outfit(&alert());
        assert!(view.holds_leave(), "the alert");
        view.input(&key(Key::Escape));
        assert!(!view.holds_leave());
        click_outfit_cell(&mut view, 0);
        view.input(&key(Key::Alt));
        let buy = outfit_item(&view, 7);
        click(&mut view, buy);
        assert!(
            view.open_outfitter()
                .and_then(OutfitterScreen::quantity)
                .is_some()
        );
        assert!(view.holds_leave(), "the quantity");
        view.input(&key(Key::Enter));
        assert!(!view.holds_leave());
    }

    #[test]
    fn the_exchanges_quantity_holds_a_leave() {
        let mut view = trading(Ok(trade_template()));
        click_item(&mut view, 7);
        view.input(&key(Key::Alt));
        view.input(&key(Key::Char('b')));
        assert!(view.open_trade().and_then(TradeScreen::quantity).is_some());
        assert!(view.holds_leave());
        view.input(&key(Key::Enter));
        assert!(!view.holds_leave());
    }

    #[test]
    fn the_shipyards_naming_prompt_holds_a_leave() {
        let mut view = shipbuying(Ok(shipyard_template()));
        click_item(&mut view, 9);
        view.input(&key(Key::Right));
        let buy = shipyard_item(&view, 1);
        click(&mut view, buy);
        view.open_ship_naming(&naming());
        assert!(view.holds_leave());
        let cancel = view
            .open_shipyard()
            .and_then(ShipyardScreen::naming)
            .and_then(|prompt| prompt.dialog().item_bounds(6))
            .expect("Cancel")
            .center();
        click(&mut view, cancel);
        assert!(!view.holds_leave());
    }

    #[test]
    fn the_bars_news_holds_a_leave() {
        let mut view = drinking(true);
        click_item(&mut view, 11);
        let at = bar_item(&view, 3);
        click(&mut view, at);
        view.open_news(&the_news(), 9000);
        assert!(view.holds_leave());
        view.input(&key(Key::Escape));
        assert!(!view.holds_leave());
    }

    // The ambient sound.

    use crate::flight::view::SharedChance;
    use crate::sound::SoundId;
    use crate::spaceport::ambient::LOOPS;
    use nova_sim::mission::screens::mac_ticks;

    /// Port Kane's ambient sound.
    const KANE: i16 = 10_032;

    /// Earth sounding Port Kane's ambient sound, with this `Flags2`.
    fn sounding(flags2: u16) -> FakePort {
        FakePort {
            port: Ok(PortRecord {
                cust_snd_id: KANE,
                flags2,
                ..catalog().port.expect("a record")
            }),
            ..catalog()
        }
    }

    /// Earth sounding Port Kane's, its waits drawn on a source whose every
    /// draw is the last outcome, so each wait is 959 ticks.
    fn kane() -> SpaceportView {
        view_of(&sounding(0))
    }

    fn ambient(until_stopped: bool) -> Vec<Sound> {
        vec![Sound::Ui(UiSound::PortAmbient {
            sound: SoundId(KANE),
            until_stopped,
        })]
    }

    /// One 50 ms frame's sounds.
    fn frame(view: &mut SpaceportView) -> Vec<Sound> {
        view.tick(Duration::from_millis(50));
        view.take_sounds()
    }

    /// Runs out a fresh 959-tick wait: nothing up to its last tick, which
    /// plays.
    fn plays_after_a_fresh_wait(view: &mut SpaceportView) {
        view.tick(mac_ticks(958));
        assert_eq!(view.take_sounds(), [], "958 ticks");
        view.tick(mac_ticks(959).saturating_sub(mac_ticks(958)));
        assert_eq!(view.take_sounds(), ambient(false), "959 ticks");
    }

    #[test]
    fn the_ambient_sound_plays_on_the_first_frame_then_after_each_wait() {
        let mut view = kane();
        assert_eq!(view.take_sounds(), [], "not as it is built");
        assert_eq!(frame(&mut view), ambient(false));
        assert_eq!(frame(&mut view), []);
        view.tick(mac_ticks(959).saturating_sub(Duration::from_millis(50)));
        assert_eq!(view.take_sounds(), ambient(false));
    }

    #[test]
    fn its_wait_is_drawn_on_the_chance_given() {
        let chance: Rc<RefCell<dyn nova_sim::Chance>> = Rc::new(RefCell::new(First));
        let mut view = kane().with_chance(SharedChance::new(chance));
        assert_eq!(frame(&mut view), ambient(false));
        view.tick(mac_ticks(479));
        assert_eq!(view.take_sounds(), [], "479 ticks");
        view.tick(mac_ticks(480).saturating_sub(mac_ticks(479)));
        assert_eq!(view.take_sounds(), ambient(false), "480 ticks");
    }

    /// A source whose every draw is the first outcome.
    struct First;

    impl nova_sim::Chance for First {
        fn fires(&mut self, _percent: u8) -> bool {
            true
        }

        fn below(&mut self, _n: u32) -> u32 {
            0
        }
    }

    #[test]
    fn a_stellar_without_an_ambient_sound_stays_silent() {
        for id in [-1, 0, 127] {
            let catalog = FakePort {
                port: Ok(PortRecord {
                    cust_snd_id: id,
                    ..catalog().port.expect("a record")
                }),
                ..catalog()
            };
            let mut view = view_of(&catalog);
            view.tick(Duration::from_secs(60));
            assert_eq!(view.take_sounds(), [], "{id}");
            click_item(&mut view, LANDSCAPE_ITEM);
            assert_eq!(view.take_sounds(), [], "{id}");
            assert_eq!(view.close_sounds(), [], "{id}");
        }
    }

    #[test]
    fn a_spaceport_that_cannot_be_shown_has_no_ambient_sound() {
        let mut view = SpaceportView::new(&sounding(0), StellarId(140), Err("no DLOG".into()));
        view.tick(Duration::from_secs(60));
        assert_eq!(view.take_sounds(), []);
    }

    #[test]
    fn a_service_open_over_the_port_holds_the_sound_and_its_close_rearms_it() {
        let mut view = kane();
        assert_eq!(frame(&mut view), ambient(false));
        view.tick(mac_ticks(900));
        click_item(&mut view, 11);
        assert!(view.open_service().is_some());
        view.tick(Duration::from_secs(60));
        assert_eq!(view.take_sounds(), [], "under the bar");
        view.input(&key(Key::Escape));
        assert!(view.open_service().is_none());
        assert_eq!(frame(&mut view), [], "the activate rearms, no play");
        plays_after_a_fresh_wait(&mut view);
    }

    fn text(text: &str) -> MissionText {
        MissionText {
            kind: MissionTextKind::Comp,
            mission: MissionId(130),
            text: text.to_owned(),
            graphic: None,
            movie: None,
            map: None,
        }
    }

    /// Port Kane's spaceport with its mission screens, its own offer
    /// already asked for and answered with none.
    fn kane_missioned() -> SpaceportView {
        let mut view = kane().with_missions(
            Ok(computer_template()),
            Ok(info_template()),
            plain_offers(),
            plain_texts(),
        );
        assert_eq!(view.take_offer_request(), Some(SPACEPORT));
        view
    }

    // The closings that redraw the status bar.

    use crate::spaceport::news::NewsScreen;
    use crate::spaceport::shipyard::INFO_KEY;

    /// "Shipyard Info", plain and with a picture both of `DLOG` 1005's
    /// shape: 250 x 285, centred, with Done (1), the title (3) and the
    /// text (5).
    fn ship_info() -> InfoTemplates {
        let item = |x, y, w, h| ItemTemplate {
            bounds: rect(x, y, w, h),
            enabled: true,
            kind: ItemSpec::User,
        };
        let template = Template {
            bounds: rect(0.0, 0.0, 250.0, 285.0),
            placement: Placement::Center,
            items: vec![
                item(86.0, 253.0, 74.0, 25.0),
                item(300.0, 0.0, 1.0, 1.0),
                item(3.0, 3.0, 240.0, 24.0),
                item(300.0, 0.0, 1.0, 1.0),
                item(9.0, 32.0, 234.0, 214.0),
            ],
        };
        InfoTemplates {
            plain: Ok(template.clone()),
            picture: Ok(template),
        }
    }

    /// [`shipbuying`] with the shipyard's info panel given.
    fn shipbuying_with_info() -> SpaceportView {
        let art: Rc<dyn ShipyardCatalog> = Rc::new(shipyard_port());
        view_of(&shipyard_port()).with_shipyard(
            Ok(shipyard_template()),
            ship_info(),
            Err("no DLOG 3001".to_owned()),
            yard(17_500),
            art,
        )
    }

    /// Earth with its mission screens, the player info screen and its bar
    /// given, the hire screen's info panel too.
    pub(in crate::spaceport) fn drinking_with_info() -> SpaceportView {
        let art: Rc<dyn ShipyardCatalog> = Rc::new(catalog());
        informed(missioned()).with_bar(
            plain_bar(Ok(bar_template())),
            Ok(news_template()),
            Ok(shipyard_template()),
            ship_info(),
            art,
            true,
        )
    }

    /// `view` in its bar, with the hire screen open over two ships.
    pub(in crate::spaceport) fn hiring(mut view: SpaceportView) -> SpaceportView {
        click_item(&mut view, 11);
        view.input(&key(Key::Char('h')));
        assert!(view.take_hire_request());
        view.open_hire(hirelings(2, true));
        assert!(view.open_bar().and_then(BarScreen::hire_screen).is_some());
        // By the engine nothing is selected as it opens: Right picks the
        // first cell.
        view.input(&key(Key::Right));
        view
    }

    #[test]
    fn the_first_play_waits_for_the_spaceports_own_offer_to_be_asked_for() {
        let mut view = kane().with_missions(
            Ok(computer_template()),
            Ok(info_template()),
            plain_offers(),
            plain_texts(),
        );
        assert_eq!(frame(&mut view), [], "the offer is still to be asked for");
        assert_eq!(view.take_offer_request(), Some(SPACEPORT));
        assert_eq!(frame(&mut view), ambient(false));
    }

    #[test]
    fn the_first_play_waits_for_the_landings_texts_and_offer_to_close() {
        let mut view = kane_missioned();
        view.show_mission_texts(vec![text("Well done.")]);
        view.open_offer(offered());
        assert_eq!(frame(&mut view), []);
        done(&mut view);
        assert!(view.mission_text().is_none());
        assert_eq!(frame(&mut view), [], "under the offer");
        view.input(&key(Key::Enter));
        assert!(view.offer_dialog().is_none());
        assert_eq!(frame(&mut view), ambient(false), "once both are gone");
        plays_after_a_fresh_wait(&mut view);
    }

    #[test]
    fn the_mission_info_and_the_player_info_over_the_port_hold_the_sound() {
        let mut view = informed(kane_missioned());
        view.open_mission_info(actives(1));
        assert_eq!(view.take_sounds(), [Sound::Ui(UiSound::MissionInfoOpened)]);
        assert_eq!(frame(&mut view), [], "under the mission info");
        view.input(&key(Key::Escape));
        assert!(view.mission_info().is_none());
        assert_eq!(view.take_sounds(), [Sound::Ui(UiSound::MissionInfoClosed)]);
        view.open_player_info(pilot_info());
        assert_eq!(frame(&mut view), [], "under the player info");
        view.input(&key(Key::Escape));
        assert!(view.player_info().is_none());
        assert_eq!(frame(&mut view), ambient(false));
    }

    #[test]
    fn a_looped_ambient_sound_asks_every_frontmost_frame() {
        let mut view = view_of(&sounding(LOOPS));
        assert_eq!(frame(&mut view), ambient(true));
        assert_eq!(frame(&mut view), ambient(true));
        click_item(&mut view, 11);
        assert_eq!(frame(&mut view), [], "covered");
        view.input(&key(Key::Escape));
        assert_eq!(frame(&mut view), ambient(true));
    }

    #[test]
    fn a_press_on_the_landscape_plays_it_once_unless_it_loops() {
        let mut view = kane();
        assert_eq!(frame(&mut view), ambient(false));
        let landscape = item(&view, LANDSCAPE_ITEM).center();
        view.input(&Input::PointerButton {
            button: MouseButton::Left,
            pressed: true,
            at: landscape,
        });
        assert_eq!(view.take_sounds(), ambient(false), "on the press");
        view.input(&Input::PointerButton {
            button: MouseButton::Left,
            pressed: false,
            at: landscape,
        });
        assert_eq!(view.take_sounds(), [], "not the release");
        click_item(&mut view, TEXT_ITEM);
        assert_eq!(view.take_sounds(), [], "elsewhere");
        view.input(&Input::PointerButton {
            button: MouseButton::Right,
            pressed: true,
            at: landscape,
        });
        assert_eq!(view.take_sounds(), ambient(false), "any button");
        plays_after_a_fresh_wait(&mut view);
        let mut looped = view_of(&sounding(LOOPS));
        click(&mut looped, landscape);
        assert_eq!(looped.take_sounds(), []);
    }

    #[test]
    fn a_press_on_the_landscape_under_something_open_is_not_the_ports() {
        let mut view = kane();
        frame(&mut view);
        click_item(&mut view, 11);
        let landscape = item(&view, LANDSCAPE_ITEM).center();
        click(&mut view, landscape);
        assert_eq!(view.take_sounds(), []);
        let mut view = kane_missioned();
        view.show_mission_texts(vec![text("Hello.")]);
        click(&mut view, landscape);
        assert_eq!(view.take_sounds(), []);
    }

    #[test]
    fn a_cover_from_outside_rearms_the_wait() {
        let mut view = kane();
        assert_eq!(frame(&mut view), ambient(false));
        view.tick(mac_ticks(900));
        view.cover();
        assert_eq!(frame(&mut view), [], "the activate rearms, no play");
        plays_after_a_fresh_wait(&mut view);
        let mut fresh = kane();
        fresh.cover();
        assert_eq!(frame(&mut fresh), ambient(false), "never played");
    }

    #[test]
    fn closing_halts_the_sound_after_whatever_was_made() {
        let mut view = kane();
        view.tick(Duration::from_millis(50));
        assert_eq!(
            view.close_sounds(),
            [
                Sound::Ui(UiSound::PortAmbient {
                    sound: SoundId(KANE),
                    until_stopped: false,
                }),
                Sound::Ui(UiSound::PortAmbientHalted(SoundId(KANE))),
            ]
        );
        let mut looped = view_of(&sounding(LOOPS));
        assert_eq!(
            looped.close_sounds(),
            [Sound::Ui(UiSound::PortAmbientHalted(SoundId(KANE)))],
            "halted even before it played"
        );
    }

    #[test]
    fn the_hire_screen_is_a_port_dialog() {
        let mut view = drinking(true);
        click_item(&mut view, 11);
        assert_eq!(view.take_port_dialog_opened(), None, "the bar");
        view.input(&key(Key::Char('h')));
        assert!(view.take_hire_request());
        assert_eq!(view.take_port_dialog_opened(), None, "only asked for");
        view.open_hire(hirelings(2, true));
        view.input(&key(Key::Right));
        assert_eq!(view.take_port_dialog_opened(), Some(PortDialog::Hire));
        assert_eq!(view.take_port_dialog_opened(), None, "once");
        view.open_hire(hirelings(2, true));
        assert_eq!(view.take_port_dialog_opened(), None, "already open");
        view.input(&key(Key::Down));
        assert_eq!(view.take_port_dialog_closed(), None, "still open");
        view.input(&key(Key::Escape));
        assert!(view.open_bar().is_some(), "back in the bar");
        assert_eq!(
            view.take_port_dialog_closed(),
            Some(PortDialog::Hire),
            "Cancel"
        );
        assert_eq!(view.take_port_dialog_closed(), None, "once");
        view.input(&key(Key::Char('h')));
        assert!(view.take_hire_request());
        view.open_hire(hirelings(2, true));
        view.input(&key(Key::Right));
        assert_eq!(view.take_port_dialog_opened(), Some(PortDialog::Hire));
        view.input(&key(Key::Char('h')));
        assert_eq!(view.take_hire(), Some(ShipId(128)));
        assert_eq!(view.take_port_dialog_closed(), None, "not until hired");
        view.set_hire(&hirelings(2, true));
        assert_eq!(
            view.take_port_dialog_closed(),
            Some(PortDialog::Hire),
            "a hire"
        );
        view.set_hire(&hirelings(2, true));
        assert_eq!(view.take_port_dialog_closed(), None, "nothing open");
        view.input(&key(Key::Char('h')));
        assert!(view.take_hire_request());
        view.open_hire(hirelings(0, true));
        assert_eq!(
            view.take_port_dialog_opened(),
            None,
            "nothing for hire returns before the save (@0x5e8da)"
        );
        assert_eq!(view.take_port_dialog_closed(), None, "nothing for hire");
        view.input(&key(Key::Escape));
        view.input(&key(Key::Escape));
        assert!(view.open_bar().is_none(), "left");
        assert_eq!(view.take_port_dialog_closed(), None, "the bar");
    }

    /// [`drinking_with_info`] in its bar.
    fn in_the_bar() -> SpaceportView {
        let mut view = drinking_with_info();
        click_item(&mut view, 11);
        view
    }

    /// [`in_the_bar`] with News open over it.
    fn in_the_news() -> SpaceportView {
        let mut view = in_the_bar();
        let at = bar_item(&view, 3);
        click(&mut view, at);
        assert!(view.take_news_request());
        view.open_news(&the_news(), 9000);
        view
    }

    /// [`shipbuying_with_info`] in its shipyard, its info panel open.
    pub(in crate::spaceport) fn in_the_shipyards_panel() -> SpaceportView {
        let mut view = shipbuying_with_info();
        click_item(&mut view, 9);
        view.input(&key(Key::Right));
        view.input(&key(INFO_KEY));
        let shipyard = view.open_shipyard().expect("shipbuying");
        assert!(shipyard.info_panel().is_some());
        view
    }

    /// The hire screen open, its info panel open over it.
    fn in_the_hire_panel() -> SpaceportView {
        let mut view = hiring(drinking_with_info());
        view.input(&key(INFO_KEY));
        let hire = view.open_bar().and_then(BarScreen::hire_screen);
        assert!(hire.and_then(HireScreen::info_panel).is_some());
        view
    }

    /// Mission info open over the spaceport on `rows` missions.
    fn in_mission_info(rows: usize) -> SpaceportView {
        let mut view = missioned();
        view.open_mission_info(actives(rows));
        view
    }

    #[test]
    fn each_redrawing_closing_asks_for_a_status_refresh() {
        type Open = fn() -> SpaceportView;
        type Close = fn(&mut SpaceportView);
        let rows: [(&str, Open, Close); 7] = [
            ("the bar's Leave", in_the_bar, |view| {
                view.input(&key(Key::Escape));
                assert!(view.open_bar().is_none());
            }),
            ("News closing", in_the_news, |view| {
                view.input(&key(Key::Enter));
                let bar = view.open_bar().expect("back in the bar");
                assert!(bar.news_screen().is_none());
            }),
            (
                "the shipyard's Info panel's Done",
                in_the_shipyards_panel,
                |view| {
                    view.input(&key(Key::Enter));
                    let shipyard = view.open_shipyard().expect("still open");
                    assert!(shipyard.info_panel().is_none());
                },
            ),
            (
                "the hire screen's Info panel's Done",
                in_the_hire_panel,
                |view| {
                    view.input(&key(Key::Enter));
                    let hire = view.open_bar().and_then(BarScreen::hire_screen);
                    assert!(hire.expect("still open").info_panel().is_none());
                },
            ),
            (
                "player info's close",
                || {
                    let mut view = informed(missioned());
                    view.open_player_info(pilot_info());
                    view
                },
                |view| {
                    view.input(&key(Key::Escape));
                    assert!(view.player_info().is_none());
                },
            ),
            (
                "mission info's Done",
                || in_mission_info(2),
                |view| {
                    view.input(&key(Key::Enter));
                    assert!(view.mission_info().is_none());
                },
            ),
            (
                "mission info closed by an abort leaving nothing",
                || in_mission_info(1),
                |view| {
                    view.set_mission_info(actives(0));
                    assert!(view.mission_info().is_none());
                },
            ),
        ];
        for (what, open, close) in rows {
            let mut view = open();
            assert!(!view.take_status_refresh(), "{what}: opening");
            close(&mut view);
            assert!(view.take_status_refresh(), "{what}");
            assert!(!view.take_status_refresh(), "{what}: once");
        }
    }

    /// `view` with mission screens laid out by `info`: the shops' and
    /// panels' fixtures come without them.
    fn with_info(view: SpaceportView, info: Result<DialogTemplate, String>) -> SpaceportView {
        view.with_missions(Ok(computer_template()), info, plain_offers(), plain_texts())
    }

    /// Each screen that opens mission info over itself: a view with it
    /// open, given mission info laid out by `info`, and whether it is
    /// still open.
    type Opener = (
        &'static str,
        fn(Result<DialogTemplate, String>) -> SpaceportView,
        fn(&SpaceportView) -> bool,
    );

    /// The outfitter (@0x5bf8d), the exchange (@0x5e0d4), the shipyard
    /// and the hire screen (@0x5f48d), the ship description of each
    /// (@0x5b9f3) and an offer over a venue (@0xa3344).
    const INFO_OPENERS: [Opener; 7] = [
        (
            "the exchange",
            |info| {
                let mut view = with_info(venues(), info);
                assert_eq!(view.take_offer_request(), Some(SPACEPORT));
                click_item(&mut view, 7);
                view
            },
            |view| view.open_trade().is_some(),
        ),
        (
            "the outfitter",
            |info| {
                let mut view = with_info(venues(), info);
                assert_eq!(view.take_offer_request(), Some(SPACEPORT));
                click_item(&mut view, 8);
                view
            },
            |view| view.open_outfitter().is_some(),
        ),
        (
            "the shipyard",
            |info| {
                let mut view = with_info(venues(), info);
                assert_eq!(view.take_offer_request(), Some(SPACEPORT));
                click_item(&mut view, 9);
                view
            },
            |view| view.open_shipyard().is_some(),
        ),
        (
            "the hire screen",
            |info| hiring(with_info(drinking_with_info(), info)),
            |view| view.open_bar().and_then(BarScreen::hire_screen).is_some(),
        ),
        (
            "the shipyard's description",
            |info| {
                let mut view = with_info(shipbuying_with_info(), info);
                click_item(&mut view, 9);
                view.input(&key(Key::Right));
                view.input(&key(INFO_KEY));
                view
            },
            |view| {
                view.open_shipyard()
                    .is_some_and(|shipyard| shipyard.info_panel().is_some())
            },
        ),
        (
            "the hire screen's description",
            |info| {
                let mut view = hiring(with_info(drinking_with_info(), info));
                view.input(&key(INFO_KEY));
                view
            },
            |view| {
                view.open_bar()
                    .and_then(BarScreen::hire_screen)
                    .and_then(HireScreen::info_panel)
                    .is_some()
            },
        ),
        (
            "an offer over the bar",
            |info| {
                let mut view = with_info(venues(), info);
                assert_eq!(view.take_offer_request(), Some(SPACEPORT));
                click_item(&mut view, 11);
                view.tick(FIRST_OFFER_DELAY);
                assert_eq!(view.take_offer_request(), Some(BAR));
                view.open_offer(offered());
                view
            },
            |view| view.offer_dialog().is_some(),
        ),
    ];

    #[test]
    fn mission_info_closing_over_each_opener_asks_for_a_status_refresh() {
        for (what, open, still_open) in INFO_OPENERS {
            for by_abort in [false, true] {
                let what = format!("{what}, abort: {by_abort}");
                let mut view = open(Ok(info_template()));
                assert!(still_open(&view), "{what}: open");
                view.input(&key(MISSION_INFO_KEY));
                assert!(view.take_mission_info_request(), "{what}: asked");
                view.open_mission_info(actives(if by_abort { 1 } else { 2 }));
                assert!(view.mission_info().is_some(), "{what}");
                assert!(!view.take_status_refresh(), "{what}: opening");
                if by_abort {
                    view.set_mission_info(actives(0));
                } else {
                    view.input(&key(Key::Escape));
                }
                assert!(view.mission_info().is_none(), "{what}: closed");
                assert!(view.take_status_refresh(), "{what}");
                assert!(!view.take_status_refresh(), "{what}: once");
                assert!(still_open(&view), "{what}: the opener is back");
                assert!(!view.take_status_refresh(), "{what}: not while it stays");
            }
        }
    }

    #[test]
    fn mission_info_without_its_dialog_over_each_opener_asks_for_no_refresh() {
        for (what, open, still_open) in INFO_OPENERS {
            for by_abort in [false, true] {
                let what = format!("{what}, abort: {by_abort}");
                let mut view = open(Err("no DLOG 1012".to_owned()));
                view.input(&key(MISSION_INFO_KEY));
                assert!(view.take_mission_info_request(), "{what}: asked");
                view.open_mission_info(actives(if by_abort { 1 } else { 2 }));
                assert!(
                    view.mission_info()
                        .and_then(MissionInfoScreen::problem)
                        .is_some(),
                    "{what}"
                );
                if by_abort {
                    view.set_mission_info(actives(0));
                } else {
                    view.input(&key(Key::Escape));
                }
                assert!(view.mission_info().is_none(), "{what}: closed");
                assert!(!view.take_status_refresh(), "{what} (@0x9e20b)");
                assert!(still_open(&view), "{what}: the opener is back");
            }
        }
    }

    #[test]
    fn mission_infos_beeps_come_out_as_it_opens_and_as_an_abort_closes_it() {
        let mut view = in_mission_info(1);
        assert_eq!(view.take_sounds(), [Sound::Ui(UiSound::MissionInfoOpened)]);
        view.set_mission_info(actives(0));
        assert_eq!(view.take_sounds(), [Sound::Ui(UiSound::MissionInfoClosed)]);
    }

    #[test]
    fn a_status_refresh_waits_for_the_mission_texts() {
        let mut view = missioned();
        view.open_mission_info(actives(1));
        view.show_mission_texts(vec![MissionText {
            kind: MissionTextKind::Brief,
            mission: MissionId(130),
            text: "Go.".to_owned(),
            graphic: None,
            movie: None,
            map: None,
        }]);
        view.set_mission_info(actives(0));
        assert!(view.mission_info().is_none(), "closed");
        assert!(!view.take_status_refresh(), "held while the text shows");
        done(&mut view);
        assert_eq!(shown_text(&view), None);
        assert!(view.take_status_refresh(), "its text closed");
        assert!(!view.take_status_refresh(), "once");
    }

    #[test]
    fn a_screen_without_its_dialog_asks_for_no_refresh() {
        let art: Rc<dyn ShipyardCatalog> = Rc::new(catalog());
        let mut view = earth().with_bar(
            plain_bar(Err("no DLOG 1013".to_owned())),
            Ok(news_template()),
            Ok(shipyard_template()),
            no_info(),
            Rc::clone(&art),
            true,
        );
        click_item(&mut view, 11);
        assert!(view.open_bar().and_then(BarScreen::problem).is_some());
        view.input(&key(Key::Escape));
        assert!(view.open_bar().is_none());
        assert!(!view.take_status_refresh(), "the bar (@0x4900d)");
        let mut view = earth().with_bar(
            plain_bar(Ok(bar_template())),
            Err("no DLOG 1014".to_owned()),
            Ok(shipyard_template()),
            no_info(),
            art,
            true,
        );
        click_item(&mut view, 11);
        let at = bar_item(&view, 3);
        click(&mut view, at);
        assert!(view.take_news_request());
        view.open_news(&the_news(), 9000);
        let news = view.open_bar().and_then(BarScreen::news_screen);
        assert!(news.and_then(NewsScreen::problem).is_some());
        view.input(&key(Key::Escape));
        assert!(
            view.open_bar()
                .is_some_and(|bar| bar.news_screen().is_none())
        );
        assert!(!view.take_status_refresh(), "News (@0x47fcb)");
        let mut view = missioned();
        view.open_player_info(pilot_info());
        assert!(view.player_info().and_then(InfoScreen::problem).is_some());
        view.input(&key(Key::Escape));
        assert!(view.player_info().is_none());
        assert!(!view.take_status_refresh(), "player info (@0x5a0a5)");
        let bare = || {
            earth().with_missions(
                Ok(computer_template()),
                Err("no DLOG 1012".to_owned()),
                plain_offers(),
                plain_texts(),
            )
        };
        let mut view = bare();
        view.open_mission_info(actives(2));
        assert!(
            view.mission_info()
                .and_then(MissionInfoScreen::problem)
                .is_some()
        );
        view.input(&key(Key::Escape));
        assert!(view.mission_info().is_none());
        assert!(!view.take_status_refresh(), "mission info (@0x9e20b)");
        let mut view = bare();
        view.open_mission_info(actives(1));
        view.set_mission_info(actives(0));
        assert!(view.mission_info().is_none());
        assert!(!view.take_status_refresh(), "mission info, by an abort");
    }

    #[test]
    fn the_port_dialogs_and_a_placeholder_ask_for_no_status_refresh() {
        let mut view = earth();
        click_item(&mut view, 7);
        view.input(&key(Key::Escape));
        assert!(view.open_service().is_none());
        assert!(!view.take_status_refresh(), "a placeholder");
        let mut view = shipbuying_with_info();
        click_item(&mut view, 9);
        view.input(&key(Key::Escape));
        assert!(view.open_shipyard().is_none());
        assert!(!view.take_status_refresh(), "the shipyard: a port dialog");
        let mut view = hiring(drinking_with_info());
        view.input(&key(Key::Escape));
        assert!(
            view.open_bar()
                .is_some_and(|bar| bar.hire_screen().is_none())
        );
        assert!(
            !view.take_status_refresh(),
            "the hire screen: a port dialog"
        );
    }
}
