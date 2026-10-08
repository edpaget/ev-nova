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
//! [`SpaceportView::take_outfit`], [`SpaceportView::set_outfitter`]), and
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
//! closes back to the bar ([`SpaceportView::set_hire`]).
//!
//! Where the stellar sells fuel, Recharge (item 4) asks for a refill,
//! taken through the spaceport ([`SpaceportView::take_recharge`]). A
//! refusal ([`SpaceportView::refuse_recharge`]) shows in the description
//! box in place of the description until the next button is activated, or
//! the ship is recharged ([`SpaceportView::recharged`]).
//!
//! Without the dialog, or the stellar's record, the screen says why, and
//! Return or Escape still leaves.

use std::rc::Rc;
use std::time::Duration;

use nova_sim::{
    HireList, Market, Order, OutfitOrder, Outfitter, RechargeRefusal, Service, ShipId, ShipNaming,
    Shipyard, sells_fuel, services,
};

use super::bar::{BarScreen, Hiring};
use super::catalog::{SpaceportCatalog, StellarId};
use super::layout::{
    BACKGROUND, LANDSCAPE_ITEM, LEAVE_ITEM, LEAVE_LABEL, NAME_FONT, NAME_ITEM, NAME_SIZE,
    RECHARGE_ITEM, RECHARGE_LABEL, TEXT_ITEM, item_service, label, landscape_id, service_item,
};
use super::outfitter::{OutfitterCatalog, OutfitterScreen};
use super::service::ServiceScreen;
use super::shipyard::{ShipOrder, ShipyardCatalog, ShipyardScreen};
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
use crate::ui::desc::{BODY_COLOR, BODY_SIZE};
use crate::ui::dialog::{Dialog, DialogEvent, DialogTemplate, Role};

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
}

impl Open {
    fn screen_mut(&mut self) -> &mut dyn Screen {
        match self {
            Self::Service(screen) => screen,
            Self::Trade(screen) => screen.as_mut(),
            Self::Outfitter(screen) => screen.as_mut(),
            Self::Shipyard(screen) => screen.as_mut(),
            Self::Bar(screen) => screen.as_mut(),
        }
    }

    fn closed(&self) -> bool {
        match self {
            Self::Service(screen) => screen.closed(),
            Self::Trade(screen) => screen.closed(),
            Self::Outfitter(screen) => screen.closed(),
            Self::Shipyard(screen) => screen.closed(),
            Self::Bar(screen) => screen.closed(),
        }
    }
}

/// The outfitter given: its dialog template (or why there is none), the
/// outfitter as it is, and where its pictures and descriptions come from.
#[derive(Clone)]
struct Outfitting {
    template: Result<DialogTemplate, String>,
    outfitter: Outfitter,
    catalog: Rc<dyn OutfitterCatalog>,
}

impl std::fmt::Debug for Outfitting {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Outfitting")
            .field("template", &self.template)
            .field("outfitter", &self.outfitter)
            .finish_non_exhaustive()
    }
}

/// The shipyard given: its dialog templates (or why there are none), the
/// shipyard as it is, and where its pictures and descriptions come from.
#[derive(Clone)]
struct Shipbuying {
    template: Result<DialogTemplate, String>,
    info: Result<DialogTemplate, String>,
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

/// The bar given: its dialog template (or why there is none), what its
/// hire screen is built from, and whether the fleet has room.
#[derive(Clone, Debug)]
struct Barkeeping {
    template: Result<DialogTemplate, String>,
    hiring: Hiring,
    room: bool,
}

/// The spaceport of the stellar landed on.
#[derive(Clone, Debug)]
pub struct SpaceportView {
    stellar: StellarId,
    /// The spaceport, or why it cannot be shown.
    port: Result<Port, String>,
    /// The screen open over it, if any.
    open: Option<Open>,
    /// The exchange's dialog template (or why there is none) and the
    /// exchange as it is, once given.
    trade: Option<(Result<DialogTemplate, String>, Market)>,
    /// The outfitter, once given.
    outfitting: Option<Outfitting>,
    /// The shipyard, once given.
    shipbuying: Option<Shipbuying>,
    /// The bar, once given.
    barkeeping: Option<Barkeeping>,
    left: bool,
    /// Whether Recharge has been clicked since this was last taken.
    recharge: bool,
    /// Whether the outfitter has opened since this was last taken.
    outfitter_opened: bool,
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
            left: false,
            recharge: false,
            outfitter_opened: false,
            sounds: Vec::new(),
        }
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
    /// saying why there is none). Without it, the Trade Center opens its
    /// placeholder.
    #[must_use]
    pub fn with_trade(self, template: Result<DialogTemplate, String>, market: Market) -> Self {
        Self {
            trade: Some((template, market)),
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
        if let Some((_, kept)) = &mut self.trade {
            *kept = market;
        }
    }

    /// The spaceport with the stellar's outfitter, `outfitter`, which the
    /// Outfitter opens laid out by `template`, the "Outfit" dialog (or
    /// saying why there is none), each outfit's picture and description
    /// read from `catalog`. Without it, the Outfitter opens its
    /// placeholder.
    #[must_use]
    pub fn with_outfitter(
        self,
        template: Result<DialogTemplate, String>,
        outfitter: Outfitter,
        catalog: Rc<dyn OutfitterCatalog>,
    ) -> Self {
        Self {
            outfitting: Some(Outfitting {
                template,
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
    /// once.
    pub fn take_outfit(&mut self) -> Option<OutfitOrder> {
        match &mut self.open {
            Some(Open::Outfitter(screen)) => screen.take_order(),
            _ => None,
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
        info: Result<DialogTemplate, String>,
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
    /// its hire screen is laid out by `hire_template`, the "Shipyard"
    /// dialog, with its info panel by `info_template`, "Shipyard Info".
    /// Hire Escort is greyed unless the fleet has `room`. Without it, the
    /// Bar opens its placeholder.
    #[must_use]
    pub fn with_bar(
        self,
        template: Result<DialogTemplate, String>,
        hire_template: Result<DialogTemplate, String>,
        info_template: Result<DialogTemplate, String>,
        catalog: Rc<dyn ShipyardCatalog>,
        room: bool,
    ) -> Self {
        Self {
            barkeeping: Some(Barkeeping {
                template,
                hiring: Hiring {
                    template: hire_template,
                    info: info_template,
                    catalog,
                },
                room,
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
    /// for hire, has the bar say so.
    pub fn open_hire(&mut self, list: HireList) {
        if let Some(Open::Bar(screen)) = &mut self.open {
            screen.open_hire(list);
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

    /// After a hire, with `list` the ships for hire now: the hire screen
    /// closes back to the bar, and the bar's Hire Escort, and the next
    /// bar's, is greyed unless the fleet still has room.
    pub fn set_hire(&mut self, list: &HireList) {
        if let Some(Open::Bar(screen)) = &mut self.open {
            screen.set_hire(list);
        }
        if let Some(barkeeping) = &mut self.barkeeping {
            barkeeping.room = list.room;
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
        let port = &*port;
        let metrics = Rc::clone(&port.metrics.0);
        if let (Some(barkeeping), Service::Bar) = (&self.barkeeping, service) {
            let layout = barkeeping
                .template
                .clone()
                .map(|template| (template, metrics));
            self.open = Some(Open::Bar(Box::new(BarScreen::new(
                layout,
                self.stellar,
                barkeeping.hiring.clone(),
                port.style,
                barkeeping.room,
            ))));
            return;
        }
        self.open = Some(
            match (&self.trade, &self.outfitting, &self.shipbuying, service) {
                (Some((template, market)), _, _, Service::TradeCenter) => {
                    let layout = template.clone().map(|template| (template, metrics));
                    Open::Trade(Box::new(TradeScreen::new(
                        layout,
                        market.clone(),
                        port.style,
                    )))
                }
                (_, _, Some(shipbuying), Service::Shipyard) => {
                    let layout = shipbuying
                        .template
                        .clone()
                        .map(|template| (template, metrics));
                    Open::Shipyard(Box::new(ShipyardScreen::new(
                        layout,
                        shipbuying.info.clone(),
                        shipbuying.text_input.clone(),
                        shipbuying.shipyard.clone(),
                        Rc::clone(&shipbuying.catalog),
                        port.style,
                    )))
                }
                (_, Some(outfitting), _, Service::Outfitter) => {
                    self.outfitter_opened = true;
                    let layout = outfitting
                        .template
                        .clone()
                        .map(|template| (template, metrics));
                    Open::Outfitter(Box::new(OutfitterScreen::new(
                        layout,
                        outfitting.outfitter.clone(),
                        Rc::clone(&outfitting.catalog),
                        port.style,
                    )))
                }
                _ => Open::Service(ServiceScreen::new(service, port.style, metrics)),
            },
        );
    }
}

impl Screen for SpaceportView {
    /// An open service takes every input until it closes. Otherwise every
    /// input goes to the dialog. It never quits.
    fn input(&mut self, input: &Input) -> ScreenAction {
        if let Some(open) = &mut self.open {
            let screen = open.screen_mut();
            screen.input(input);
            self.sounds.extend(screen.take_sounds());
            if open.closed() {
                self.open = None;
            }
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

    /// Nothing moves on its own.
    fn tick(&mut self, _dt: Duration) {}

    fn draw(&self, list: &mut DrawList) {
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
            _ => {}
        }
    }

    fn cancel_pointer(&mut self) {
        match (&mut self.open, &mut self.port) {
            (Some(open), _) => open.screen_mut().cancel_pointer(),
            (None, Ok(port)) => port.dialog.cancel_pointer(),
            (None, Err(_)) => {}
        }
    }

    /// Lets go of the keys the screen open holds.
    fn release_keys(&mut self) {
        if let Some(open) = &mut self.open {
            open.screen_mut().release_keys();
        }
    }

    /// The buttons' sounds, the open service's included, in order.
    fn take_sounds(&mut self) -> Vec<Sound> {
        std::mem::take(&mut self.sounds)
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
        }
    }

    fn trading(template: Result<Template, String>) -> SpaceportView {
        earth().with_trade(template, exchange(0))
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
                lot: Lot::One,
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
        assert_eq!(view.take_trade().map(|order| order.lot), Some(Lot::One));
        view.input(&key(Key::Alt));
        view.input(&key(Key::Char('b')));
        assert_eq!(view.take_trade().map(|order| order.lot), Some(Lot::Max));
    }

    // The Outfitter.

    use crate::spaceport::outfitter::{OutfitterCatalog, OutfitterScreen};
    use nova_sim::{OutfitId, OutfitOrder, OutfitRow, Outfitter};

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
                mass: 1,
                owned,
                max: 10,
                buy: Ok(()),
                sell: Ok(()),
                words: None,
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
        view_of(&outfitting_port()).with_outfitter(template, tanks(0), art)
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
            Some(OutfitOrder {
                outfit: OutfitId(200),
                direction: Direction::Buy,
            })
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
    use nova_sim::{ShipId, ShipNaming, ShipRow, ShipSpecs, Shipyard};

    use crate::spaceport::shipyard::ShipOrder;

    impl ShipBaseImages for FakePort {
        fn ship_base_images(&self) -> Vec<(ShipId, i16)> {
            Vec::new()
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
                },
                buy: Ok(()),
            }],
            trade_in: 2500,
            cash: 25_000,
            current: ShipId(128),
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
            Err("no DLOG 1005".to_owned()),
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
            Ok(shipyard_template()),
            Err("no DLOG 1005".to_owned()),
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
    fn a_full_fleet_greys_hire_escort_when_the_bar_opens() {
        let mut view = drinking(false);
        click_item(&mut view, 10);
        view.input(&key(Key::Char('h')));
        assert!(!view.take_hire_request());
        let debug = format!("{view:?}");
        assert!(debug.contains("Barkeeping"), "{debug}");
    }
}
