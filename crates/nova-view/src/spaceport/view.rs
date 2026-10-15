//! The spaceport screen: the stellar landed on, laid out by the interface
//! file's "Spaceport" dialog over its background picture.
//!
//! The screen reads everything once, when it is built: the stellar's
//! record and landscape through the [`SpaceportCatalog`] port, and its
//! description (`dësc` = `spöb` ID) and the button style through
//! [`DescriptionSource`]. It draws the background, then the landscape
//! stretched into its item, then the stellar's name centred in the name
//! bar, then the dialog: the description scrolling in its box, Leave, and
//! a button for each service the stellar's flags offer. The other service
//! items stay blank, and clicks on them do nothing.
//!
//! Leave, Return and Escape leave: the router takes off. A service's
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
//! screen open for their timers.
//! [`MISSION_INFO_KEY`], in the spaceport, the bar or the mission
//! computer, asks for the mission info screen
//! ([`SpaceportView::take_mission_info_request`]), which opens over
//! whatever is open ([`SpaceportView::open_mission_info`]); its abort is
//! taken through the spaceport ([`SpaceportView::take_mission_abort`]).
//! The mission texts show one after another in "Desc Dialog" over
//! everything ([`SpaceportView::show_mission_texts`]), each taking every
//! input until Done closes it.
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
//! taken through the spaceport ([`SpaceportView::take_recharge`]). A
//! refusal ([`SpaceportView::refuse_recharge`]) shows in the description
//! box in place of the description until the next button is activated, or
//! the ship is recharged ([`SpaceportView::recharged`]).
//!
//! Without the dialog, or the stellar's record, the screen says why, and
//! Return or Escape still leaves.
//!
//! The port dialogs, the bar's hire screen among them, say as they open
//! and close ([`SpaceportView::take_port_dialog_opened`],
//! [`SpaceportView::take_port_dialog_closed`]), and the other closings
//! the original ends in `_RefreshSAT` say so too
//! ([`SpaceportView::take_status_refresh`]): whoever flies the ship
//! redraws the status bar at each. The closings the original redraws at
//! that ours can't reach:
//!
//! - The bar's Gamble text (`_ShowTextDialog` @0x4918c, then
//!   `_RefreshSAT` @0x49191): ours' Gamble isn't built and shows greyed.
//! - The bar's map (the map key in `_BarFilter` @0x47a8a-0x47a9b;
//!   `_DoSystemMap` @0x492b2 ends in `_RefreshSAT` @0x13bc9, then the bar
//!   restores the nav it saved @0x492b7-0x492d3): ours' bar opens no map.
//! - Mission info opened from the outfitter (@0x5bf8d), the exchange
//!   (@0x5e0d4), the shipyard (@0x5f48d), the ship description (@0x5b9f3)
//!   or an offer (@0xa3344): ours opens it only from the spaceport, the
//!   bar and the computer. Its close redraws the same wherever it opens.
//! - Mission info's Abort with nothing selected (@0x9e51d-0x9e557): ours
//!   greys Abort unless the selected mission can be aborted.
//! - A screen whose dialog is missing: the original never shows the bar,
//!   News, player info, mission info or the computer without its `DLOG`
//!   (@0x4900d, @0x47fcb, @0x5a0a5, @0x9e20b, @0xa39a9), so none redraws.
//!   Ours shows a screen saying why, whose close asks for no redraw; the
//!   computer's still closes as a port dialog.
//! - An outfitter or shipyard with nothing listed: the original says so
//!   and returns before the dialog, skipping the close's `_RefreshSAT` and
//!   restore (@0x5bbf7 → @0x5dbd6, @0x5e8df → @0x5f906). Ours opens the
//!   screen with an empty grid, whose close redraws and restores as a
//!   shop's.
//! - Preferences: the menu's `pref` command (@0xccc49-0xccc4e) calls
//!   `_DoPrefsDialog` alone (@0xcccf1), with no `_RefreshSAT`; only the
//!   `pref` Apple Event's handler redraws (@0x56195), and the menu never
//!   sends it.
//! - The shops' text dialogs: `_ShowTextDialog`'s `_RefreshSAT` tails run
//!   only on a faded screen (`_screenFaded` @0x5a7c7, @0x5af9e, @0x5b45c),
//!   around a `dësc` movie ours doesn't play.
//! - `_MyModalDialog`'s update event (@0x59b17-0x59be6), an OS repaint:
//!   ours draws every frame whole.

use std::collections::VecDeque;
use std::rc::Rc;
use std::time::Duration;

use nova_sim::mission::screens::{BAR, OUTFITTER, SHIPYARD, SPACEPORT, TRADE_CENTER};
use nova_sim::{
    ComputerRefusal, HireList, Market, MissionComputer, MissionId, MissionInfo, MissionText, News,
    OfferPrompt, Order, OutfitOrder, Outfitter, PlayerInfo, RechargeRefusal, Reselection,
    SaleAlert, Service, ShipId, ShipNaming, Shipyard, sells_fuel, services,
};

use super::bar::MISSION_INFO_KEY;
use super::bar::{BarScreen, Hiring};
use super::catalog::{SpaceportCatalog, StellarId};
use super::hire::HireScreen;
use super::layout::{
    BACKGROUND, LANDSCAPE_ITEM, LEAVE_ITEM, LEAVE_LABEL, NAME_FONT, NAME_ITEM, NAME_SIZE,
    RECHARGE_ITEM, RECHARGE_LABEL, TEXT_ITEM, item_service, label, landscape_id, service_item,
};
use super::missions::{MissionComputerScreen, MissionInfoScreen};
use super::offer::{OfferChoice, OfferDialog, VenueOffers};
use super::outfitter::{OutfitterCatalog, OutfitterScreen};
use super::service::ServiceScreen;
use super::shipyard::{InfoTemplates, ShipOrder, ShipyardCatalog, ShipyardScreen};
use super::trade::TradeScreen;
use crate::color::Color;
use crate::draw::DrawList;
use crate::font::Font;
use crate::geometry::Point;
use crate::image::ImageKey;
use crate::input::{Input, Key};
use crate::screen::{Screen, ScreenAction};
use crate::sound::Sound;
use crate::text::TextMetrics;
use crate::ui::button::{ButtonSkin, ButtonStyle};
use crate::ui::catalog::DescriptionSource;
use crate::ui::desc::{BODY_COLOR, BODY_SIZE, DescDialog};
use crate::ui::dialog::{Dialog, DialogEvent, DialogTemplate, Role};
#[cfg(doc)]
use crate::ui::player_info::PLAYER_INFO_KEY;
use crate::ui::player_info::{PlayerInfoScreen, is_player_info_press};

/// Where the reason the spaceport cannot be shown goes, its size and
/// colour.
pub const PROBLEM_AT: Point = Point::new(16.0, 32.0);
/// The reason's size.
pub const PROBLEM_SIZE: f32 = 20.0;

/// Why recharging was refused for want of credits. No stock `STR#` has
/// one, so it is built the way the original builds its fee refusals from
/// `STR#` 2002: #61 "You don't have enough" + #33 "credits" + a tail like
/// #63's "to pay the docking fee.".
pub const CANNOT_AFFORD_RECHARGE: &str = "You don't have enough credits to recharge.";

/// Why recharging was refused with the tank full. No stock `STR#` has
/// one, so these are our own words.
pub const ALREADY_RECHARGED: &str = "Your ship is already fully recharged.";

/// The spaceport, laid out.
#[derive(Clone, Debug)]
struct Port {
    dialog: Dialog,
    name: String,
    /// The landscape `PICT`, or the missing one's ID.
    landscape: Result<i16, i16>,
    offered: Vec<Service>,
    /// Whether the stellar sells fuel, so Recharge is a button.
    sells_fuel: bool,
    /// The stellar's description, to show again after a refusal.
    description: String,
    /// Whether a refusal shows in the description box.
    refused: bool,
    /// Why there is no description, if there is none.
    description_problem: Option<String>,
    style: ButtonStyle,
    metrics: MetricsHandle,
}

impl Port {
    /// Shows `message` in the description box in its place.
    fn refuse(&mut self, message: &str) {
        self.dialog.set_text(TEXT_ITEM, message);
        self.refused = true;
    }

    /// Shows the description again, if a refusal shows in its place.
    fn show_description(&mut self) {
        if std::mem::take(&mut self.refused) {
            self.dialog.set_text(TEXT_ITEM, &self.description);
        }
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
    template: Result<DialogTemplate, String>,
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
    /// The mission info screen's.
    info: Result<DialogTemplate, String>,
    /// The offer dialog's, which every venue's offers show in.
    offer: Result<DialogTemplate, String>,
    /// The spaceport's own offer, asked for once, as it is landed on.
    landing: VenueOffers,
    /// The "Desc Dialog"'s, which the mission texts show in.
    text: Result<DialogTemplate, String>,
    /// Whether Mission BBS has asked for the mission computer since this
    /// was last taken.
    requested: bool,
    /// Whether [`MISSION_INFO_KEY`] has asked for the mission info screen
    /// since this was last taken.
    info_requested: bool,
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
    /// there are none) and the exchange as it is, once given.
    #[allow(clippy::type_complexity)] // Two templates and the exchange.
    trade: Option<(
        Result<DialogTemplate, String>,
        Result<DialogTemplate, String>,
        Market,
    )>,
    /// The outfitter, once given.
    outfitting: Option<Outfitting>,
    /// The shipyard, once given.
    shipbuying: Option<Shipbuying>,
    /// The bar, once given.
    barkeeping: Option<Barkeeping>,
    /// The mission screens, once given.
    missioning: Option<Missioning>,
    /// The mission info screen, over whatever else is open.
    info: Option<MissionInfoScreen>,
    /// The player info screen's dialog and its Yes/No dialog, once given.
    #[allow(clippy::type_complexity)] // Two templates.
    player_info_dialogs: Option<(
        Result<DialogTemplate, String>,
        Result<DialogTemplate, String>,
    )>,
    /// Whether [`PLAYER_INFO_KEY`] asked for the player info screen in the
    /// spaceport itself.
    player_info_requested: bool,
    /// The player info screen, over everything else.
    player_info: Option<PlayerInfoScreen>,
    /// Whether its Jettison was confirmed since this was last taken.
    player_info_jettison: bool,
    /// The mission texts to show, the first shown over everything.
    texts: VecDeque<DescDialog>,
    left: bool,
    /// Whether Recharge has been clicked since this was last taken.
    recharge: bool,
    /// Whether the outfitter has opened since this was last taken.
    outfitter_opened: bool,
    /// The port dialog opened since this was last taken.
    port_dialog_opened: Option<PortDialog>,
    /// The port dialog closed since this was last taken.
    port_dialog_closed: Option<PortDialog>,
    /// Whether the outfitter has closed since this was last taken.
    outfitter_closed: bool,
    /// Whether a closing that redraws the status bar has come since this
    /// was last taken ([`SpaceportView::take_status_refresh`]).
    status_refresh: bool,
    /// The sounds made since they were last taken, kept here so a service
    /// that closes keeps its sounds.
    sounds: Vec<Sound>,
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
        let port = layout.and_then(|(template, metrics)| {
            let record = catalog.stellar_port(stellar)?;
            let offered = services(record.flags);
            let sells_fuel = sells_fuel(record.flags);
            let (text, description_problem) = match catalog.description(stellar.0) {
                Ok(text) => (text, None),
                Err(reason) => (String::new(), Some(reason)),
            };
            let mut roles = vec![
                (LEAVE_ITEM, Role::Button(LEAVE_LABEL.to_owned())),
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
            roles.extend(offered.iter().map(|&service| {
                (
                    service_item(service),
                    Role::Button(label(service).to_owned()),
                )
            }));
            if sells_fuel {
                roles.push((RECHARGE_ITEM, Role::Button(RECHARGE_LABEL.to_owned())));
            }
            let style = catalog.button_style();
            let dialog = Dialog::new(&template, &roles, Rc::clone(&metrics))
                .with_buttons(ButtonSkin::NOVA, style)
                .with_default(Some(LEAVE_ITEM))
                .with_cancel(Some(LEAVE_ITEM));
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
                description: text,
                refused: false,
                description_problem,
                style,
                metrics: MetricsHandle(metrics),
            })
        });
        Self {
            stellar,
            port,
            open: None,
            trade: None,
            outfitting: None,
            shipbuying: None,
            barkeeping: None,
            missioning: None,
            info: None,
            player_info_dialogs: None,
            player_info_requested: false,
            player_info: None,
            player_info_jettison: false,
            texts: VecDeque::new(),
            left: false,
            recharge: false,
            outfitter_opened: false,
            port_dialog_opened: None,
            port_dialog_closed: None,
            outfitter_closed: false,
            status_refresh: false,
            sounds: Vec::new(),
        }
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
        if self.texts.is_empty() {
            self.port_dialog_closed.take()
        } else {
            None
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
        self.texts.is_empty() && std::mem::take(&mut self.status_refresh)
    }

    /// Whether Recharge has been clicked since this was last asked:
    /// whoever flies the ship refills it, then says how it went with
    /// [`SpaceportView::recharged`] or [`SpaceportView::refuse_recharge`].
    pub fn take_recharge(&mut self) -> bool {
        std::mem::take(&mut self.recharge)
    }

    /// Says why recharging was refused, in the description box in place of
    /// the description, until the next button is activated. With no fuel
    /// sold, which Recharge is never offered for, the description stays.
    pub fn refuse_recharge(&mut self, refusal: RechargeRefusal) {
        let Ok(port) = &mut self.port else {
            return;
        };
        match refusal {
            RechargeRefusal::CannotAfford => port.refuse(CANNOT_AFFORD_RECHARGE),
            RechargeRefusal::Full => port.refuse(ALREADY_RECHARGED),
            RechargeRefusal::NoFuel => port.show_description(),
        }
    }

    /// Says the ship has been recharged: the description shows again.
    pub fn recharged(&mut self) {
        if let Ok(port) = &mut self.port {
            port.show_description();
        }
    }

    /// The spaceport with the stellar's exchange, `market`, which the
    /// Trade Center opens laid out by `template`, the "Trade" dialog (or
    /// saying why there is none), its quantity dialog by `quantity` (or,
    /// without it, the dialog's fallback). Without it, the Trade Center
    /// opens its placeholder.
    #[must_use]
    pub fn with_trade(
        self,
        template: Result<DialogTemplate, String>,
        quantity: Result<DialogTemplate, String>,
        market: Market,
    ) -> Self {
        Self {
            trade: Some((template, quantity, market)),
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
        if let Some((_, _, kept)) = &mut self.trade {
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
    /// by `template`, the bar dialog (or saying why there is none), its
    /// text and the ships' pictures and descriptions read from `catalog`;
    /// its news is laid out by `news_template`, the news dialog, and its
    /// hire screen by `hire_template`, the "Shipyard" dialog, with its
    /// info panel by `info_template`, "Shipyard Info". Hire Escort is
    /// greyed unless the fleet has `room`. Without it, the Bar opens its
    /// placeholder.
    #[must_use]
    pub fn with_bar(
        self,
        template: Result<DialogTemplate, String>,
        news_template: Result<DialogTemplate, String>,
        hire_template: Result<DialogTemplate, String>,
        info_template: InfoTemplates,
        catalog: Rc<dyn ShipyardCatalog>,
        room: bool,
    ) -> Self {
        Self {
            barkeeping: Some(Barkeeping {
                template,
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

    /// Whether the bar's map key has asked for the course map since this
    /// was last asked ([`BarScreen::take_map_request`]): whoever flies the
    /// ship opens it over the spaceport.
    pub fn take_map_request(&mut self) -> bool {
        match &mut self.open {
            Some(Open::Bar(screen)) => screen.take_map_request(),
            _ => false,
        }
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

    /// Opens the hire screen over the bar open on `list`, or, with nothing
    /// for hire, has the bar say so. The hire screen opened is a port
    /// dialog ([`PortDialog::Hire`]); with nothing for hire nothing opens,
    /// as the original returns before its save (@0x5e8da → @0x5f906).
    pub fn open_hire(&mut self, list: HireList) {
        if let Some(Open::Bar(screen)) = &mut self.open {
            let before = screen.hire_screen().is_some();
            screen.open_hire(list);
            if !before && screen.hire_screen().is_some() {
                self.port_dialog_opened = Some(PortDialog::Hire);
            }
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
    /// there is none); [`MISSION_INFO_KEY`] asks for the mission info
    /// screen, laid out by `info`; the spaceport, and the trade center,
    /// outfitter, shipyard and bar when given, offer missions in the offer
    /// dialog laid out by `offer`, the spaceport asking for its offer at
    /// once; and the mission texts show in "Desc Dialog", laid out by
    /// `text` (and do not show without it). Without them, Mission BBS
    /// opens its placeholder and nothing offers missions.
    #[must_use]
    pub fn with_missions(
        self,
        computer: Result<DialogTemplate, String>,
        info: Result<DialogTemplate, String>,
        offer: Result<DialogTemplate, String>,
        text: Result<DialogTemplate, String>,
    ) -> Self {
        Self {
            missioning: Some(Missioning {
                computer,
                info,
                landing: VenueOffers::asking(offer.clone()),
                offer,
                text,
                requested: false,
                info_requested: false,
            }),
            ..self
        }
    }

    /// The venue that has asked for an offer since this was last asked,
    /// by its `AvailLoc`: the screen open's ([`BAR`], [`TRADE_CENTER`],
    /// [`OUTFITTER`] or [`SHIPYARD`]), or, with nothing open, the
    /// spaceport's ([`SPACEPORT`]); never the mission computer's or a
    /// placeholder's. Whoever flies the ship answers with
    /// [`SpaceportView::open_offer`], or rearms it
    /// ([`SpaceportView::rearm_offers`]).
    pub fn take_offer_request(&mut self) -> Option<i16> {
        let (asked, venue) = match &mut self.open {
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
    /// over the spaceport (`_DoMissionDialog` @0xa380a-0xa3956).
    pub fn open_mission_computer(&mut self, computer: Result<MissionComputer, ComputerRefusal>) {
        let computer = match computer {
            Ok(computer) => computer,
            Err(refusal) => {
                self.port_dialog_opened = Some(PortDialog::MissionComputer);
                self.show_text(&refusal.message());
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
        self.open = Some(Open::Computer(Box::new(MissionComputerScreen::new(
            layout, computer, port.style,
        ))));
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
    /// it closes (`_ShowTextDialog` @0x5a752).
    pub fn show_mission_texts(&mut self, texts: Vec<MissionText>) {
        for text in texts {
            self.show_text(&text.text);
        }
    }

    /// The spaceport with the player info screen's dialog, `template`
    /// (`DLOG` 1017), and its Yes/No dialog, `confirm` (`DLOG` 3002), or
    /// why there are none.
    #[must_use]
    pub fn with_player_info(
        self,
        template: Result<DialogTemplate, String>,
        confirm: Result<DialogTemplate, String>,
    ) -> Self {
        Self {
            player_info_dialogs: Some((template, confirm)),
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
        let (template, confirm) = self.player_info_dialogs.clone().unwrap_or_else(|| {
            let none = || Err("the player info dialog was not given".to_owned());
            (none(), none())
        });
        let layout = template.map(|template| (template, Rc::clone(&port.metrics.0)));
        self.player_info = Some(PlayerInfoScreen::new(layout, confirm, info, port.style));
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

    /// Whether [`MISSION_INFO_KEY`] has asked for the mission info screen
    /// since this was last asked, in the spaceport, the bar or the mission
    /// computer: whoever flies the ship answers with
    /// [`SpaceportView::open_mission_info`].
    pub fn take_mission_info_request(&mut self) -> bool {
        let own = self
            .missioning
            .as_mut()
            .is_some_and(|missioning| std::mem::take(&mut missioning.info_requested));
        let open = match &mut self.open {
            Some(Open::Bar(screen)) => screen.take_info_request(),
            Some(Open::Computer(screen)) => screen.take_info_request(),
            _ => false,
        };
        own || open
    }

    /// Opens the mission info screen on `info` over whatever is open; with
    /// nothing to show, it does not open, as the original's does not
    /// (@0xa3e37-0xa3ec1).
    pub fn open_mission_info(&mut self, info: MissionInfo) {
        let (Ok(port), Some(missioning)) = (&self.port, &self.missioning) else {
            return;
        };
        if info.rows.is_empty() {
            return;
        }
        let layout = missioning
            .info
            .clone()
            .map(|template| (template, Rc::clone(&port.metrics.0)));
        let mut screen = MissionInfoScreen::new(layout, info, port.style);
        self.sounds.extend(screen.take_sounds());
        self.info = Some(screen);
    }

    /// The mission info screen open, if it is.
    #[must_use]
    pub fn mission_info(&self) -> Option<&MissionInfoScreen> {
        self.info.as_ref()
    }

    /// The slot the mission info screen asked to abort since this was
    /// last taken, once.
    pub fn take_mission_abort(&mut self) -> Option<u8> {
        self.info.as_mut()?.take_abort()
    }

    /// After an abort, the mission info screen shows `info`, closing once
    /// nothing is left.
    pub fn set_mission_info(&mut self, info: MissionInfo) {
        if let Some(screen) = &mut self.info {
            screen.set_info(info);
            self.sounds.extend(screen.take_sounds());
            if screen.closed() {
                self.close_mission_info();
            }
        }
    }

    /// Closes the mission info screen, noting its close when it was laid
    /// out ([`SpaceportView::take_status_refresh`]).
    fn close_mission_info(&mut self) {
        if self
            .info
            .take()
            .is_some_and(|info| info.problem().is_none())
        {
            self.status_refresh = true;
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
        self.texts.front()
    }

    /// Queues `text` to show in "Desc Dialog", when there is one.
    fn show_text(&mut self, text: &str) {
        let (
            Ok(port),
            Some(Missioning {
                text: Ok(template), ..
            }),
        ) = (&self.port, &self.missioning)
        else {
            return;
        };
        self.texts.push_back(DescDialog::new(
            template,
            text,
            port.style,
            Rc::clone(&port.metrics.0),
        ));
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
    /// sold) asks for a refill, and an offered service's button opens it,
    /// each showing the description again in place of a refusal. Anything
    /// else does nothing.
    fn activate(&mut self, item: usize) {
        if item == LEAVE_ITEM {
            self.left = true;
            return;
        }
        let Ok(port) = &mut self.port else {
            return;
        };
        if item == RECHARGE_ITEM && port.sells_fuel {
            port.show_description();
            self.recharge = true;
            return;
        }
        let Some(service) = item_service(item).filter(|s| port.offered.contains(s)) else {
            return;
        };
        port.show_description();
        if let (Some(missioning), Service::MissionBbs) = (&mut self.missioning, service) {
            missioning.requested = true;
            return;
        }
        let port = &*port;
        let metrics = Rc::clone(&port.metrics.0);
        if let (Some(barkeeping), Service::Bar) = (&mut self.barkeeping, service) {
            let layout = barkeeping
                .template
                .clone()
                .map(|template| (template, metrics));
            let mut bar = BarScreen::new(
                layout,
                self.stellar,
                barkeeping.hiring.clone(),
                port.style,
                barkeeping.room,
            );
            bar = bar.with_news(barkeeping.news.clone());
            if let Some(missioning) = &self.missioning {
                bar = bar.with_offers(missioning.offer.clone());
            }
            barkeeping.opened = true;
            self.open = Some(Open::Bar(Box::new(bar)));
            return;
        }
        let offer = self
            .missioning
            .as_ref()
            .map(|missioning| missioning.offer.clone());
        self.open = Some(
            match (&self.trade, &self.outfitting, &self.shipbuying, service) {
                (Some((template, quantity, market)), _, _, Service::TradeCenter) => {
                    let layout = template.clone().map(|template| (template, metrics));
                    let screen =
                        TradeScreen::new(layout, quantity.clone(), market.clone(), port.style);
                    Open::Trade(Box::new(match offer {
                        Some(offer) => screen.with_offers(offer),
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
                        port.style,
                    );
                    Open::Shipyard(Box::new(match offer {
                        Some(offer) => screen.with_offers(offer),
                        None => screen,
                    }))
                }
                (_, Some(outfitting), _, Service::Outfitter) => {
                    self.outfitter_opened = true;
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
                        port.style,
                    );
                    Open::Outfitter(Box::new(match offer {
                        Some(offer) => screen.with_offers(offer),
                        None => screen,
                    }))
                }
                _ => Open::Service(ServiceScreen::new(service, port.style, metrics)),
            },
        );
        self.port_dialog_opened = self.open.as_ref().and_then(Open::port_dialog);
    }
}

impl Screen for SpaceportView {
    /// An open service takes every input until it closes. Otherwise every
    /// input goes to the dialog. It never quits.
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
        if let Some(text) = self.texts.front_mut() {
            text.input(input);
            self.sounds.extend(text.take_sounds());
            if text.closed() {
                self.texts.pop_front();
            }
            return ScreenAction::None;
        }
        if let Some(info) = &mut self.info {
            info.input(input);
            self.sounds.extend(info.take_sounds());
            if info.closed() {
                self.close_mission_info();
            }
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
        if let (
            Input::Key {
                key: MISSION_INFO_KEY,
                pressed: true,
                repeat: false,
            },
            Some(missioning),
        ) = (*input, &mut self.missioning)
        {
            missioning.info_requested = true;
            return ScreenAction::None;
        }
        if is_player_info_press(input) && self.port.is_ok() {
            self.player_info_requested = true;
            return ScreenAction::None;
        }
        let event = match &mut self.port {
            Ok(port) => {
                let event = port.dialog.input(input);
                self.sounds.extend(port.dialog.take_sound().map(Sound::Ui));
                event
            }
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

    /// The screen open ticks, unless a mission text is shown over it.
    fn tick(&mut self, dt: Duration) {
        if self.texts.is_empty()
            && self.info.is_none()
            && self.player_info.is_none()
            && let Some(open) = &mut self.open
        {
            open.screen_mut().tick(dt);
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
        if let Some(info) = &self.info {
            info.draw(list);
        }
        if let Some(text) = self.texts.front() {
            text.draw(list);
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
        if let Some(text) = self.texts.front_mut() {
            text.cancel_pointer();
            return;
        }
        if let Some(info) = &mut self.info {
            info.cancel_pointer();
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

    /// The buttons' sounds, the open service's included, in order.
    fn take_sounds(&mut self) -> Vec<Sound> {
        std::mem::take(&mut self.sounds)
    }
}

impl SpaceportView {
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
mod tests {
    use std::cell::RefCell;

    use nova_sim::landing::StellarFlags;

    use super::*;
    use crate::draw::DrawCommand;
    use crate::geometry::Bounds;
    use crate::input::MouseButton;
    use crate::sound::{Sound, UiSound};
    use crate::spaceport::catalog::PortRecord;
    use crate::spaceport::layout::SERVICE_ITEMS;
    use crate::text::fixture::MonoMetrics;
    use crate::ui::dialog::{ItemSpec, ItemTemplate, Placement};

    /// Stellar 140, Earth: a planet with a trade center and a bar, of
    /// `Type` 3 with no custom picture, so its landscape is `PICT` 10003.
    struct FakePort {
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

    fn view_of(catalog: &FakePort) -> SpaceportView {
        SpaceportView::new(catalog, StellarId(140), Ok(layout()))
    }

    fn earth() -> SpaceportView {
        view_of(&catalog())
    }

    fn drawn(view: &impl Screen) -> Vec<DrawCommand> {
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

    fn key(key: Key) -> Input {
        Input::Key {
            key,
            pressed: true,
            repeat: false,
        }
    }

    fn click(view: &mut SpaceportView, at: Point) {
        for pressed in [true, false] {
            view.input(&Input::PointerButton {
                button: MouseButton::Left,
                pressed,
                at,
            });
        }
    }

    fn click_item(view: &mut SpaceportView, number: usize) {
        let at = item(view, number).center();
        click(view, at);
    }

    fn item(view: &SpaceportView, item: usize) -> Bounds {
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
    fn only_the_offered_services_recharge_and_leave_are_buttons() {
        let view = earth();
        let commands = drawn(&view);
        let labels = texts(&commands);
        for shown in ["Trade Center", "Bar", "Mission BBS", "Recharge", "Leave"] {
            assert!(labels.contains(&shown.to_owned()), "{shown}: {labels:?}");
        }
        for hidden in ["Shipyard", "Outfitter", "Done"] {
            assert!(!labels.contains(&hidden.to_owned()), "{hidden}: {labels:?}");
        }
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
            ("Bar", 10),
            ("Mission BBS", 11),
            ("Recharge", 4),
            ("Leave", 12),
        ] {
            let (origin, color) = label_at(text).expect(text);
            assert!(item(&view, item_number).contains(origin), "{text}");
            assert_eq!(color, Color::BLACK, "{text}");
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
        // The shipyard's and outfitter's places, the blank button, the
        // landscape and the description.
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
        click_item(&mut view, 10);
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

    #[test]
    fn an_uninhabited_stellar_offers_only_leave() {
        let catalog = FakePort {
            port: Ok(PortRecord {
                flags: StellarFlags::CAN_LAND | StellarFlags::UNINHABITED,
                ..catalog().port.expect("a record")
            }),
            ..catalog()
        };
        let view = view_of(&catalog);
        assert_eq!(view.offered(), []);
        let labels = texts(&drawn(&view));
        assert!(labels.contains(&"Leave".to_owned()));
        assert!(!labels.contains(&"Mission BBS".to_owned()));
        assert!(!labels.contains(&"Recharge".to_owned()), "no fuel sold");
        let mut view = view;
        click_item(&mut view, RECHARGE_ITEM);
        assert!(!view.take_recharge());
        assert_eq!(view.take_sounds(), [], "a blank item is silent");
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

    const DOWN: Sound = Sound::Ui(UiSound::ButtonDown);
    const UP: Sound = Sound::Ui(UiSound::ButtonUp);

    #[test]
    fn a_click_on_leave_sounds_it_going_down_then_up() {
        let mut view = earth();
        let leave = item(&view, 12).center();
        let button = |pressed| Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at: leave,
        };
        view.input(&button(true));
        assert_eq!(view.take_sounds(), [DOWN]);
        view.input(&button(false));
        assert!(view.left());
        assert_eq!(view.take_sounds(), [UP]);
        assert_eq!(view.take_sounds(), []);
    }

    #[test]
    fn a_service_sounds_its_button_and_its_done_even_as_it_closes() {
        let mut view = earth();
        click_item(&mut view, 7);
        assert_eq!(view.take_sounds(), [DOWN, UP]);
        click(&mut view, super::super::service::DONE_BUTTON.center());
        assert!(view.open_service().is_none(), "closed");
        assert_eq!(view.take_sounds(), [DOWN, UP]);
        click_item(&mut view, 10);
        view.input(&key(Key::Escape));
        assert!(view.open_service().is_none());
        assert_eq!(view.take_sounds(), [DOWN, UP], "Escape is silent");
    }

    #[test]
    fn keys_blank_items_and_a_spaceport_that_cannot_be_shown_are_silent() {
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

    // Recharging.

    use nova_sim::RechargeRefusal;

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
    fn the_refusals_say_why_in_the_originals_words_or_ours() {
        assert_eq!(
            CANNOT_AFFORD_RECHARGE,
            "You don't have enough credits to recharge."
        );
        assert_eq!(ALREADY_RECHARGED, "Your ship is already fully recharged.");
    }

    #[test]
    fn recharge_is_asked_for_once_per_click() {
        let mut view = earth();
        assert!(!view.take_recharge(), "nothing clicked");
        click_item(&mut view, RECHARGE_ITEM);
        assert!(view.take_recharge());
        assert!(!view.take_recharge(), "once");
        assert!(!view.left());
        assert!(view.open_service().is_none());
        assert_eq!(view.take_sounds(), [DOWN, UP]);
    }

    #[test]
    fn a_refusal_shows_in_the_description_box_until_recharged() {
        for (refusal, message) in [
            (RechargeRefusal::CannotAfford, CANNOT_AFFORD_RECHARGE),
            (RechargeRefusal::Full, ALREADY_RECHARGED),
        ] {
            let mut view = earth();
            click_item(&mut view, RECHARGE_ITEM);
            view.refuse_recharge(refusal);
            assert_eq!(box_lines(&view), [message], "{refusal:?}");
            let labels = texts(&drawn(&view));
            assert!(labels.contains(&message.to_owned()), "{labels:?}");
            assert!(!labels.contains(&DESCRIPTION[0].to_owned()), "{labels:?}");
            view.recharged();
            assert_eq!(box_lines(&view), DESCRIPTION);
        }
    }

    #[test]
    fn with_no_fuel_sold_the_description_stays() {
        let mut view = earth();
        view.refuse_recharge(RechargeRefusal::NoFuel);
        assert_eq!(box_lines(&view), DESCRIPTION);
    }

    #[test]
    fn a_refusal_shows_until_the_next_button_is_activated() {
        let mut view = earth();
        view.refuse_recharge(RechargeRefusal::Full);
        click_item(&mut view, 13);
        assert_eq!(box_lines(&view), [ALREADY_RECHARGED], "a blank item");
        click_item(&mut view, RECHARGE_ITEM);
        assert_eq!(box_lines(&view), DESCRIPTION, "recharge again");
        assert!(view.take_recharge());
        view.refuse_recharge(RechargeRefusal::Full);
        click_item(&mut view, 7);
        view.input(&key(Key::Escape));
        assert!(view.open_service().is_none());
        assert_eq!(box_lines(&view), DESCRIPTION, "a service");
    }

    #[test]
    fn recharged_without_a_refusal_keeps_the_description_where_it_is() {
        let catalog = FakePort {
            description: Ok((0..40)
                .map(|n| format!("l{n}"))
                .collect::<Vec<_>>()
                .join("\r")),
            ..catalog()
        };
        let mut view = view_of(&catalog);
        view.input(&key(Key::Down));
        view.recharged();
        assert_eq!(view_first_line(&view), 1, "not scrolled back");
    }

    #[test]
    fn a_spaceport_that_cannot_be_shown_takes_recharge_news_quietly() {
        let mut broken = SpaceportView::new(&catalog(), StellarId(140), Err("no".to_owned()));
        broken.refuse_recharge(RechargeRefusal::Full);
        broken.recharged();
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
        earth().with_trade(template, Ok(quantity_template()), exchange(0))
    }

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
        click_item(&mut view, 10);
        assert_eq!(
            view.open_service().map(ServiceScreen::service),
            Some(Service::Bar)
        );
        assert!(view.open_trade().is_none());
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
    fn the_exchange_takes_cancel_pointer_and_its_sounds_are_kept() {
        let mut view = trading(Ok(trade_template()));
        click_item(&mut view, 7);
        assert_eq!(view.take_sounds(), [DOWN, UP]);
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
        assert_eq!(view.take_sounds(), [DOWN]);
        let done = trade_item(&view, 1);
        click(&mut view, done);
        assert!(view.open_trade().is_none(), "closed");
        assert_eq!(view.take_sounds(), [DOWN, UP]);
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
    fn tanks(owned: u16) -> Outfitter {
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
        click_item(&mut view, 10);
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
        click_item(&mut view, 10);
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
        click_item(&mut view, 10);
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
        click_item(&mut view, 10);
        view.input(&key(Key::Escape));
        assert!(view.open_bar().is_none());
        assert_eq!(view.take_port_dialog_closed(), None, "the bar");
        let mut view = missioned();
        click_item(&mut view, 11);
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
        click_item(&mut view, 11);
        assert_eq!(view.take_port_dialog_opened(), None, "only asked for");
        view.open_mission_computer(Ok(offers()));
        assert_eq!(
            view.take_port_dialog_opened(),
            Some(PortDialog::MissionComputer)
        );
        assert_eq!(view.take_port_dialog_opened(), None, "once");
        let mut view = drinking(true);
        click_item(&mut view, 10);
        assert_eq!(view.take_port_dialog_opened(), None, "the bar");
        let mut view = earth();
        click_item(&mut view, 7);
        assert_eq!(view.take_port_dialog_opened(), None, "a placeholder");
    }

    #[test]
    fn the_outfitter_takes_cancel_pointer_and_its_sounds_are_kept() {
        let mut view = outfitting(Ok(outfit_template()));
        click_item(&mut view, 8);
        assert_eq!(view.take_sounds(), [DOWN, UP]);
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
        assert_eq!(view.take_sounds(), [DOWN]);
        let done = outfit_item(&view, 1);
        click(&mut view, done);
        assert!(view.open_outfitter().is_none(), "closed");
        assert_eq!(view.take_sounds(), [DOWN, UP]);
        let debug = format!("{view:?}");
        assert!(debug.contains("Outfitting"), "{debug}");
        assert!(debug.contains("Fuel Tank"), "{debug}");
    }

    // The Shipyard.

    use crate::spaceport::shipyard::{ShipBaseImages, ShipyardCatalog, ShipyardScreen};
    use nova_sim::{Rulebook, ShipFacts, ShipId, ShipNaming, ShipRow, ShipSpecs, Shipyard};

    use crate::spaceport::shipyard::ShipOrder;

    impl ShipBaseImages for FakePort {
        fn ship_base_images(&self) -> Vec<(ShipId, i16)> {
            Vec::new()
        }
    }

    impl crate::ui::catalog::DescriptionGraphics for FakePort {
        fn description_graphic(&self, _id: i16) -> Option<i16> {
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
        click_item(&mut view, 10);
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
        let buy = shipyard_item(&view, 7);
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
    fn the_shipyard_takes_cancel_pointer_and_its_sounds_are_kept() {
        let mut view = shipbuying(Ok(shipyard_template()));
        click_item(&mut view, 9);
        assert_eq!(view.take_sounds(), [DOWN, UP]);
        let buy = shipyard_item(&view, 7);
        let button = |pressed| Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at: buy,
        };
        view.input(&button(true));
        view.cancel_pointer();
        view.input(&button(false));
        assert_eq!(view.take_ship_request(), None, "the click was abandoned");
        assert_eq!(view.take_sounds(), [DOWN]);
        let done = shipyard_item(&view, 1);
        click(&mut view, done);
        assert!(view.open_shipyard().is_none(), "closed");
        assert_eq!(view.take_sounds(), [DOWN, UP]);
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
        click_item(&mut view, 10);
        assert!(view.take_bar_opened());
        assert!(!view.take_bar_opened(), "once");
        view.input(&key(Key::Escape));
        assert!(!view.take_bar_opened(), "leaving is not opening");
        click_item(&mut view, 10);
        assert!(view.take_bar_opened(), "each time it opens");
        let mut view = earth();
        click_item(&mut view, 10);
        assert!(!view.take_bar_opened(), "the placeholder is not the bar");
    }

    #[test]
    fn the_bars_news_asks_through_the_spaceport_and_opens_over_the_bar() {
        let mut view = drinking(true);
        assert!(!view.take_news_request(), "nothing open");
        view.open_news(&the_news(), 9000);
        click_item(&mut view, 10);
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
            Ok(bar_template()),
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
        click_item(&mut view, 10);
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
        click_item(&mut view, 10);
        view.input(&key(Key::Char('h')));
        assert!(view.take_hire_request());
        assert!(!view.take_hire_request(), "once");
        view.open_hire(hirelings(2, true));
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
        click_item(&mut view, 10);
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
        click_item(&mut view, 10);
        view.input(&key(Key::Char('h')));
        assert!(view.take_hire_request());
        view.open_hire(hirelings(2, true));
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
    fn with_nothing_for_hire_the_bar_says_so() {
        let mut view = drinking(true);
        click_item(&mut view, 10);
        view.input(&key(Key::Char('h')));
        assert!(view.take_hire_request());
        view.open_hire(hirelings(0, true));
        let bar = view.open_bar().expect("in the bar");
        assert!(bar.hire_screen().is_none());
        assert_eq!(bar.message(), Some(NONE_FOR_HIRE));
    }

    #[test]
    fn the_bars_map_key_asks_through_the_spaceport() {
        use crate::flight::view::MAP_KEY;
        let mut view = drinking(true);
        view.input(&key(MAP_KEY));
        assert!(!view.take_map_request(), "no bar open");
        click_item(&mut view, 10);
        view.input(&key(MAP_KEY));
        assert!(view.take_map_request());
        assert!(!view.take_map_request(), "once");
        assert!(view.open_bar().is_some(), "still in the bar");
    }

    #[test]
    fn a_full_fleet_greys_hire_escort_when_the_bar_opens() {
        let mut view = drinking(false);
        click_item(&mut view, 10);
        view.input(&key(Key::Char('h')));
        assert!(!view.take_hire_request());
        let debug = format!("{view:?}");
        assert!(debug.contains("Barkeeping"), "{debug}");
    }

    // The mission computer and the mission texts.

    use crate::spaceport::missions::{ACCEPT_ITEM as COMPUTER_ACCEPT, MissionComputerScreen};
    use nova_sim::{ComputerRow, MissionTextKind};

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
    /// text's shape.
    fn missioned() -> SpaceportView {
        earth().with_missions(
            Ok(computer_template()),
            Ok(info_template()),
            Ok(text_template()),
            Ok(text_template()),
        )
    }

    fn offers() -> MissionComputer {
        MissionComputer {
            rows: vec![ComputerRow {
                mission: MissionId(130),
                name: "Run".to_owned(),
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
        click_item(&mut view, 11);
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
        click_item(&mut view, 11);
        assert_eq!(
            view.open_service().map(ServiceScreen::service),
            Some(Service::MissionBbs)
        );
        assert!(!view.take_mission_computer_request());
    }

    #[test]
    fn a_refused_computer_says_why_over_the_spaceport() {
        let mut view = missioned();
        click_item(&mut view, 11);
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
        click_item(&mut view, 10);
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
        click_item(&mut view, 11);
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
        click_item(&mut view, 11);
        view.open_mission_computer(Ok(offers()));
        view.show_mission_texts(vec![MissionText {
            kind: MissionTextKind::Brief,
            mission: MissionId(130),
            text: "Go.".to_owned(),
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
    fn a_refused_mission_computer_opens_and_closes_once_its_refusal_closes() {
        let mut view = missioned();
        click_item(&mut view, 11);
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
        click_item(&mut view, 11);
        view.open_mission_computer(Ok(offers()));
        let text = |mission, text: &str| MissionText {
            kind: MissionTextKind::Brief,
            mission: MissionId(mission),
            text: text.to_owned(),
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
        view.show_mission_texts(Vec::new());
        assert_eq!(shown_text(&view), None);
    }

    #[test]
    fn mission_texts_click_their_done_and_cancel_their_pointer() {
        let mut view = missioned();
        view.show_mission_texts(vec![MissionText {
            kind: MissionTextKind::Comp,
            mission: MissionId(130),
            text: "Done.".to_owned(),
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
        assert_eq!(view.take_sounds(), [Sound::Ui(UiSound::ButtonDown)]);
        click(&mut view, at);
        assert!(view.mission_text().is_none());
        assert_eq!(
            view.take_sounds(),
            [Sound::Ui(UiSound::ButtonDown), Sound::Ui(UiSound::ButtonUp)]
        );
    }

    #[test]
    fn without_a_text_dialog_mission_texts_are_not_shown() {
        let mut view = earth().with_missions(
            Ok(computer_template()),
            Err("no DLOG 1012".to_owned()),
            Err("no DLOG 1016".to_owned()),
            Err("no DLOG 3003".to_owned()),
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
            Ok(bar_template()),
            Ok(news_template()),
            Ok(shipyard_template()),
            no_info(),
            art,
            true,
        )
    }

    fn offered() -> OfferPrompt {
        OfferPrompt {
            mission: MissionId(141),
            text: "Ferry this.".to_owned(),
            labels: OfferLabels {
                accept: "Yes".to_owned(),
                refuse: Some("No".to_owned()),
            },
        }
    }

    // The offers at each venue.

    use nova_sim::mission::screens::{BAR, OUTFITTER, SHIPYARD, SPACEPORT, TRADE_CENTER};

    /// Earth with every venue: a trade center, an outfitter, a shipyard
    /// and a bar, and its mission screens.
    fn venues() -> SpaceportView {
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
            .with_trade(Ok(trade_template()), Ok(quantity_template()), exchange(0))
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
                Ok(bar_template()),
                Ok(news_template()),
                Ok(shipyard_template()),
                no_info(),
                ships,
                true,
            )
            .with_missions(
                Ok(computer_template()),
                Ok(info_template()),
                Ok(text_template()),
                Ok(text_template()),
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
        click_item(&mut view, 10);
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
    fn the_spaceports_offer_sounds_and_its_pointer_goes_through_it() {
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
        assert_eq!(view.take_sounds(), [Sound::Ui(UiSound::ButtonDown)]);
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

    #[test]
    fn each_shop_asks_as_its_venue() {
        for (item, venue) in [(7, TRADE_CENTER), (8, OUTFITTER), (9, SHIPYARD)] {
            let mut view = venues();
            assert_eq!(view.take_offer_request(), Some(SPACEPORT));
            click_item(&mut view, item);
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
        click_item(&mut view, 10);
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
    fn the_mission_computer_and_placeholder_services_ask_nothing() {
        let mut view = missioned();
        assert_eq!(view.take_offer_request(), Some(SPACEPORT));
        click_item(&mut view, 11);
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
        click_item(&mut view, 10);
        view.show_mission_texts(vec![MissionText {
            kind: MissionTextKind::Refuse,
            mission: MissionId(141),
            text: "Pity.".to_owned(),
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
        click_item(&mut view, 10);
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

    fn actives(rows: usize) -> MissionInfo {
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
        click_item(&mut view, 10);
        view.input(&key(MISSION_INFO_KEY));
        assert!(view.take_mission_info_request(), "the bar");
        let mut view = missioned();
        click_item(&mut view, 11);
        view.open_mission_computer(Ok(offers()));
        view.input(&key(MISSION_INFO_KEY));
        assert!(view.take_mission_info_request(), "the computer");
        let mut view = earth();
        view.input(&key(MISSION_INFO_KEY));
        assert!(!view.take_mission_info_request(), "no mission screens");
    }

    #[test]
    fn mission_info_opens_over_whatever_is_open_only_with_missions_to_show() {
        let mut view = missioned();
        click_item(&mut view, 11);
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
        PLAYER_INFO_KEY, PlayerInfoButton, PlayerInfoScreen as InfoScreen, Tab,
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

    fn pilot_info() -> PlayerInfo {
        PlayerInfo {
            pilot_name: "Ada".to_owned(),
            date: String::new(),
            system: String::new(),
            legal_status: String::new(),
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
        }
    }

    /// `view` with the player info screen's dialogs given, its Yes/No
    /// dialog the text's shape.
    fn informed(view: SpaceportView) -> SpaceportView {
        view.with_player_info(Ok(player_info_template()), Ok(text_template()))
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
        click_item(&mut view, 10);
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
        click_item(&mut view, 11);
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
                BAR => 10,
                TRADE_CENTER => 7,
                OUTFITTER => 8,
                _ => 9,
            };
            click_item(&mut view, item);
            view.tick(FIRST_OFFER_DELAY);
            assert_eq!(view.take_offer_request(), Some(venue));
            view.open_offer(offered());
            assert!(view.offer_dialog().is_some(), "{venue}");
            assert!(asks_player_info(&mut view), "{venue}");
            assert!(view.offer_dialog().is_some(), "{venue}: still open");
        }
    }

    #[test]
    fn the_player_info_opens_over_whatever_is_open_and_closes_back_to_it() {
        let mut view = informed(missioned());
        click_item(&mut view, 11);
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
        click_item(&mut view, 10);
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
    fn drinking_with_info() -> SpaceportView {
        let art: Rc<dyn ShipyardCatalog> = Rc::new(catalog());
        informed(missioned()).with_bar(
            Ok(bar_template()),
            Ok(news_template()),
            Ok(shipyard_template()),
            ship_info(),
            art,
            true,
        )
    }

    /// `view` in its bar, with the hire screen open over two ships.
    fn hiring(mut view: SpaceportView) -> SpaceportView {
        click_item(&mut view, 10);
        view.input(&key(Key::Char('h')));
        assert!(view.take_hire_request());
        view.open_hire(hirelings(2, true));
        assert!(view.open_bar().and_then(BarScreen::hire_screen).is_some());
        view
    }

    #[test]
    fn the_hire_screen_is_a_port_dialog() {
        let mut view = drinking(true);
        click_item(&mut view, 10);
        assert_eq!(view.take_port_dialog_opened(), None, "the bar");
        view.input(&key(Key::Char('h')));
        assert!(view.take_hire_request());
        assert_eq!(view.take_port_dialog_opened(), None, "only asked for");
        view.open_hire(hirelings(2, true));
        assert_eq!(view.take_port_dialog_opened(), Some(PortDialog::Hire));
        assert_eq!(view.take_port_dialog_opened(), None, "once");
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
        click_item(&mut view, 10);
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
    fn in_the_shipyards_panel() -> SpaceportView {
        let mut view = shipbuying_with_info();
        click_item(&mut view, 9);
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
            Err("no DLOG 1013".to_owned()),
            Ok(news_template()),
            Ok(shipyard_template()),
            no_info(),
            Rc::clone(&art),
            true,
        );
        click_item(&mut view, 10);
        assert!(view.open_bar().and_then(BarScreen::problem).is_some());
        view.input(&key(Key::Escape));
        assert!(view.open_bar().is_none());
        assert!(!view.take_status_refresh(), "the bar (@0x4900d)");
        let mut view = earth().with_bar(
            Ok(bar_template()),
            Err("no DLOG 1014".to_owned()),
            Ok(shipyard_template()),
            no_info(),
            art,
            true,
        );
        click_item(&mut view, 10);
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
                Ok(text_template()),
                Ok(text_template()),
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
