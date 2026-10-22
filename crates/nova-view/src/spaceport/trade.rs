//! The Trade Center: the stellar's commodity exchange, laid out by the
//! interface file's "Trade" dialog (`DLOG`/`DITL` 1001) over its
//! background picture.
//!
//! The stock dialog is 426 x 252, centred, and drawn over `PICT` 8510.
//! `_TradeDialogUpdate` (0x4ceea-0x4e092 in the `EV Nova` executable)
//! draws it all in Geneva 9 (`_smallTextFont`, `_smallTextSize`):
//!
//! - The header, item 3: framed in dark grey, with "Commodity:" at
//!   left+7, "In Fleet:" (escorts add holds) or "In Hold:" at right-123,
//!   and "Price:" at right-70, in light grey on the baseline top+12
//!   (@0x4cf77-0x4d0b6; [`Market::held_in`]).
//! - The eight list slots, items 4 to 11 (@0x4d103-0x4d5c5). Slot n holds
//!   commodity n, slot 6 a `jünk` the stellar buys and slot 7 one it
//!   sells ([`MarketRow::slot`](nova_sim::MarketRow::slot)). Every slot,
//!   empty or not, is filled inset by 1 in `cölr`'s `ListHilite` when its
//!   row is selected, else `ListBkgnd`, with dark grey edges left and
//!   right, along the top of item 4 and the bottom of item 11. A slot's
//!   row is drawn in `ListText` on the baseline top+10: its name at
//!   left+7 (a `jünk`'s at left+6), the tons held ending at right-110
//!   when above 0, its marker at right-70 ("Low", "Med", "High",
//!   "Higher", "Lower"; [`MarketRow::marker`](nova_sim::MarketRow::marker))
//!   and its price, signed, ending at right-10.
//! - The cargo summary, item 12 (@0x4d5cb-0x4dc93): its text
//!   ([`CargoSummary`](nova_sim::CargoSummary)) wrapped white on black in
//!   the item inset by 5 and 3, as `TETextBox` and `InvertRect` draw it,
//!   and the item framed in dark grey.
//! - The event line, item 15 (@0x4df54-0x4dfb5): only when there is one
//!   ([`Market::events`]: by the engine the first local event's sentence,
//!   "… has lowered the price of food.", or every local event's name
//!   under `exchange_events` = bible; see `nova_sim::exchange_events`),
//!   wrapped white on black in the whole item. Without one, the picture
//!   shows.
//! - Buy (13), Sell (14) and Done (1) along the bottom.
//!
//! There is no status line: the original shows the free space in the
//! summary and the credits on the status bar beside the port.
//!
//! One row is selected, by a click on its slot or with Up and Down, which
//! step over empty slots and stop at the ends. The original has one row a
//! slot; the engine can list more than one `jünk` a side, whose extra rows
//! follow their slot, and the list then scrolls with the selection. Buy and Sell, clicked or with B and S (key repeats too, so
//! holding a key keeps trading), ask for a plain trade of the selected
//! good ([`Lot::Click`]), which moves up to 10 tons by the engine; Shift,
//! Command and Control change nothing, as the original tests Option alone
//! (`_DoTradeDialog` @0x5e211, @0x5e45c). With Alt held, by the engine's
//! reading of [`RuleKey::TradeCount`](nova_sim::RuleKey::TradeCount)
//! ([`Market::option_asks_count`]), they open the quantity dialog over the
//! exchange ([`QuantityDialog`], `DLOG` 1003, or its fallback) at the most
//! the row offers ([`Market::row_max`]), even when that is 0 or less; a
//! count above 0 confirmed there is traded ([`Lot::Count`]), and 0 or
//! Cancel trades nothing. While it is open it takes every input: B, S,
//! Return and Escape go to it. By the other reading, Alt asks for the
//! most at once ([`Lot::Max`]). Each is greyed as the
//! original's `_CanBuyGoods` and `_CanSellGoods` say
//! ([`Market::row_allows`]): so Buy is enabled on a row priced below
//! nothing while the cash is at least its price, though the buy then
//! moves nothing. The screen only records the order ([`TradeScreen::take_order`]):
//! whoever holds the session makes the trade and gives back the exchange
//! as it now is ([`TradeScreen::set_market`]).
//!
//! Given the offer dialog's template ([`TradeScreen::with_offers`]), the
//! trade center offers missions as `_DoTradeDialog` and `_TradeFilter`
//! do: it asks for an offer as it opens ([`TradeScreen::take_offer_request`],
//! `_DoTradeDialog` @0x5df7f), and whoever holds the session answers with
//! one, opening the [`OfferDialog`] over it ([`TradeScreen::open_offer`]),
//! or rearms it ([`TradeScreen::rearm`]) with the gap drawn, after which it
//! asks again (`_TradeFilter` @0x4e550-0x4e581). The offer takes every
//! input while it is open, and its choice is taken through the screen
//! ([`TradeScreen::take_offer_choice`]). The quantity dialog covers the
//! trade center, as the offer does: nothing is asked while either is
//! open, and the wait runs on or stops as
//! [`RuleKey::OfferClock`](nova_sim::RuleKey::OfferClock) reads it
//! ([`TradeScreen::with_offer_clock`]).
//!
//! Done, Return and Escape close it. Without the dialog, the screen says
//! why, and Return or Escape still closes it.

use std::rc::Rc;
use std::time::Duration;

use nova_sim::{Direction, Lot, Market, MissionId, OfferPrompt, Order, RuleSource};

use super::layout::DONE_LABEL;
use super::offer::{OfferChoice, OfferDialog, OfferTemplates, VenueOffers};
use super::quantity_ask::QuantityAsk;
use super::view::{PROBLEM_AT, PROBLEM_SIZE};
use crate::color::Color;
use crate::draw::{DrawList, fill_rect};
use crate::font::Font;
use crate::geometry::{Bounds, Point};
use crate::image::ImageKey;
use crate::input::{Input, Key};
use crate::movie::Movies;
use crate::screen::{Screen, ScreenAction};
use crate::sound::Sound;
use crate::text::TextMetrics;
use crate::ui::button::{ButtonSkin, ButtonStyle};
use crate::ui::dialog::{Dialog, DialogEvent, DialogTemplate, Role};
use crate::ui::mission_info::is_mission_info_press;
use crate::ui::player_info::is_player_info_press;
use crate::ui::quantity::QuantityDialog;
use crate::ui::text_box::TextBox;

/// The "Trade" dialog's `DLOG` (and `DITL`) ID.
pub const TRADE_DIALOG: i16 = 1001;

/// The picture the dialog is drawn over: "Trade", in Nova Graphics.
pub const BACKGROUND: ImageKey = ImageKey::picture(8510);

/// Done's item.
pub const DONE_ITEM: usize = 1;
/// The header row's item.
pub const HEADER_ITEM: usize = 3;
/// The first slot's item; the slots are this and the seven after it.
pub const FIRST_ROW_ITEM: usize = 4;
/// How many slots show at once.
pub const ROWS: usize = 8;
/// The cargo summary's item.
pub const SUMMARY_ITEM: usize = 12;
/// Buy's item.
pub const BUY_ITEM: usize = 13;
/// Sell's item.
pub const SELL_ITEM: usize = 14;
/// The item the event line is drawn in, wrapped and inverted
/// (@0x4df54-0x4dfb5), the entries of [`Market::events`] a line each.
pub const EVENT_ITEM: usize = 15;

/// `STR#` 150 #2.
pub const BUY_LABEL: &str = "Buy";
/// `STR#` 150 #3.
pub const SELL_LABEL: &str = "Sell";

/// Buys, as Buy does: the original's default.
pub const BUY_KEY: Key = Key::Char('b');
/// Sells, as Sell does: the original's default.
pub const SELL_KEY: Key = Key::Char('s');
/// Held, makes a Buy or Sell ask for a count in the quantity dialog, as
/// the original's Option does, or by the other reading of
/// [`RuleKey::TradeCount`](nova_sim::RuleKey::TradeCount) trade the most
/// at once.
pub const MAX_LOT_KEY: Key = Key::Alt;

/// The dialog's font: `_smallTextFont`, Geneva (@0x4cf4b-0x4cf72).
pub const TEXT_FONT: Font = Font::Geneva;
/// The dialog's size: `_smallTextSize`, 9.
pub const TEXT_SIZE: f32 = 9.0;
/// `_rgbDarkGrey` (0x3b50d8, 0x4000 grey): the frames and the slots'
/// edges.
pub const DARK_GREY: Color = Color::from_rgb24(0x0040_4040);
/// `_rgbLightGrey` (0x3b5170, 0xc000 grey): the header's labels.
pub const LIGHT_GREY: Color = Color::from_rgb24(0x00C0_C0C0);
/// The summary's and the event line's text, inverted from black.
pub const BOX_TEXT: Color = Color::WHITE;
/// The summary's and the event line's background, inverted from white.
pub const BOX_BACKGROUND: Color = Color::BLACK;
/// Where a commodity's name starts, from its slot's left (@0x4d2c9).
pub const NAME_X: f32 = 7.0;
/// Where a `jünk`'s name starts, from its slot's left (@0x4d31d).
pub const JUNK_NAME_X: f32 = 6.0;
/// How far left of its slot's right the tons held end (@0x4d382).
pub const HELD_END: f32 = 110.0;
/// How far left of its slot's right the marker starts (@0x4d3ce), and
/// the header's "Price:" (@0x4d07f).
pub const MARKER_FROM_RIGHT: f32 = 70.0;
/// How far left of its slot's right the price ends (@0x4d535).
pub const PRICE_END: f32 = 10.0;
/// A slot's baseline, below its top (@0x4d2b2).
pub const ROW_BASELINE: f32 = 10.0;
/// The header's baseline, below its top (@0x4d004).
pub const HEADER_BASELINE: f32 = 12.0;
/// Where the header's "Commodity:" starts, from its left (@0x4d004).
pub const HEADER_NAME_X: f32 = 7.0;
/// How far left of the header's right "In Fleet:" or "In Hold:" starts
/// (@0x4d034).
pub const HELD_HEADER_FROM_RIGHT: f32 = 123.0;
/// How far the summary's text box is inset in item 12, across and down
/// (@0x4d636-0x4d65b).
pub const SUMMARY_INSET: (f32, f32) = (5.0, 3.0);

/// The list's colours, `cölr`'s `ListText`, `ListBkgnd` and `ListHilite`
/// (0x3b5638, loaded by `_LoadObjectData` @0x7e0c5-0x7e130).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ListColors {
    /// A row's text.
    pub text: Color,
    /// A slot's fill.
    pub background: Color,
    /// The selected slot's fill.
    pub hilite: Color,
}

impl ListColors {
    /// Stock `cölr` 128's: white text on black, the selection dark red.
    pub const STOCK: Self = Self {
        text: Color::WHITE,
        background: Color::BLACK,
        hilite: Color::from_rgb24(0x0080_0000),
    };
}

/// The list's look.
pub trait ListLooks {
    /// The list's colours, from `cölr` 128, or [`ListColors::STOCK`] when
    /// it is missing or does not decode.
    fn list_colors(&self) -> ListColors;
}

/// The list positions of `market`'s rows, slot by slot from 0 to 7: each
/// row in that slot, in the rows' order, or `None` for an empty slot.
/// The original has one row a slot; extra `jünk` rows of a side follow
/// their slot.
fn positions(market: &Market) -> Vec<Option<usize>> {
    let slots = u8::try_from(ROWS).unwrap_or(u8::MAX);
    let mut positions = Vec::new();
    for slot in 0..slots {
        let before = positions.len();
        positions.extend(
            (0..market.rows.len())
                .filter(|&index| market.rows[index].slot == slot)
                .map(Some),
        );
        if positions.len() == before {
            positions.push(None);
        }
    }
    positions
}

/// `rect` inset by `dx` across and `dy` down, as `InsetRect` does.
fn inset(rect: Bounds, dx: f32, dy: f32) -> Bounds {
    Bounds {
        min: Point::new(rect.min.x + dx, rect.min.y + dy),
        max: Point::new(rect.max.x - dx, rect.max.y - dy),
    }
}

/// A one-pixel rule from `from` to `to` in `color`, as a pen-1 `LineTo`
/// covers it: both ends included.
fn rule(list: &mut DrawList, from: Point, to: Point, color: Color) {
    let area = Bounds {
        min: from,
        max: Point::new(to.x + 1.0, to.y + 1.0),
    };
    fill_rect(list, area, color);
}

/// `FrameRect(rect)` in `color`: a one-pixel border just inside it.
fn frame(list: &mut DrawList, rect: Bounds, color: Color) {
    let (left, top) = (rect.min.x, rect.min.y);
    let (right, bottom) = (rect.max.x - 1.0, rect.max.y - 1.0);
    rule(list, Point::new(left, top), Point::new(right, top), color);
    rule(
        list,
        Point::new(left, bottom),
        Point::new(right, bottom),
        color,
    );
    rule(list, Point::new(left, top), Point::new(left, bottom), color);
    rule(
        list,
        Point::new(right, top),
        Point::new(right, bottom),
        color,
    );
}

/// The exchange, laid out.
#[derive(Clone, Debug)]
struct Laid {
    dialog: Dialog,
    metrics: MetricsHandle,
}

/// The metrics, shared, with a `Debug` that shows nothing of them.
#[derive(Clone)]
struct MetricsHandle(Rc<dyn TextMetrics>);

impl std::fmt::Debug for MetricsHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("TextMetrics")
    }
}

/// The Trade Center of the stellar landed on.
#[derive(Clone, Debug)]
pub struct TradeScreen {
    /// The dialog, or why it cannot be shown.
    laid: Result<Laid, String>,
    market: Market,
    /// The list's colours.
    colors: ListColors,
    /// The selected row, by index into the market's rows.
    selected: usize,
    /// The list position shown first (see [`positions`]).
    top: usize,
    /// The quantity dialog, while it is open, the row and direction it
    /// counts, and whether Alt is held.
    quantity: QuantityAsk<(usize, Direction)>,
    /// The mission offers, once given.
    offering: Option<VenueOffers>,
    /// Whether [`PLAYER_INFO_KEY`](crate::ui::player_info::PLAYER_INFO_KEY) asked for the player info screen.
    player_info: bool,
    /// Whether [`MISSION_INFO_KEY`](crate::ui::mission_info::MISSION_INFO_KEY)
    /// asked for the mission info screen.
    info_requested: bool,
    /// The order asked for, until it is taken.
    order: Option<Order>,
    closed: bool,
    /// The buttons' style, for the offer dialog.
    style: ButtonStyle,
    sounds: Vec<Sound>,
}

impl TradeScreen {
    /// The exchange `market`, laid out by `layout`: the "Trade" dialog's
    /// template and the metrics its text is measured by, or why there are
    /// none. The quantity dialog is laid out by `quantity`, its template
    /// (or, without one, its fallback). Its buttons are labelled in
    /// `style`. The first row in the list is selected. Its list is in
    /// the stock colours until given others ([`TradeScreen::with_colors`]).
    #[must_use]
    pub fn new(
        layout: Result<(DialogTemplate, Rc<dyn TextMetrics>), String>,
        quantity: Result<DialogTemplate, String>,
        market: Market,
        style: ButtonStyle,
    ) -> Self {
        let laid = layout.map(|(template, metrics)| {
            let roles = [
                (DONE_ITEM, Role::Button(DONE_LABEL.to_owned())),
                (BUY_ITEM, Role::Button(BUY_LABEL.to_owned())),
                (SELL_ITEM, Role::Button(SELL_LABEL.to_owned())),
            ];
            let dialog = Dialog::new(&template, &roles, Rc::clone(&metrics))
                .with_buttons(ButtonSkin::NOVA, style)
                .with_default(Some(DONE_ITEM))
                .with_cancel(Some(DONE_ITEM));
            Laid {
                dialog,
                metrics: MetricsHandle(metrics),
            }
        });
        let mut screen = Self {
            laid,
            market,
            colors: ListColors::STOCK,
            selected: 0,
            top: 0,
            quantity: QuantityAsk::new(quantity.ok(), style),
            offering: None,
            player_info: false,
            info_requested: false,
            order: None,
            closed: false,
            style,
            sounds: Vec::new(),
        };
        let first = positions(&screen.market).into_iter().flatten().next();
        screen.select(first.unwrap_or(0));
        screen
    }

    /// Its list drawn in `colors`.
    #[must_use]
    pub fn with_colors(self, colors: ListColors) -> Self {
        Self { colors, ..self }
    }

    /// The trade center with mission offers, each shown in the offer dialog of
    /// its variant laid out by `templates` (or saying why there is none): it asks for the
    /// first as it opens, and again once rearmed
    /// ([`TradeScreen::rearm`]), as the original's does ([`TRADE_CENTER`](nova_sim::mission::screens::TRADE_CENTER)).
    #[must_use]
    pub fn with_offers(self, templates: OfferTemplates) -> Self {
        Self {
            offering: Some(VenueOffers::asking(templates)),
            ..self
        }
    }

    /// Its offers' timer running as `clock` reads
    /// [`RuleKey::OfferClock`](nova_sim::RuleKey::OfferClock), for offers
    /// given ([`TradeScreen::with_offers`]) before it.
    #[must_use]
    pub fn with_offer_clock(self, clock: RuleSource) -> Self {
        Self {
            offering: self.offering.map(|offering| offering.with_clock(clock)),
            ..self
        }
    }

    /// Whether [`PLAYER_INFO_KEY`](crate::ui::player_info::PLAYER_INFO_KEY) has asked for the player info screen
    /// since this was last asked, in the trade center or its offer
    /// (`_TradeFilter` @0x4e109), not in its quantity dialog.
    pub fn take_player_info_request(&mut self) -> bool {
        let offer = self
            .offering
            .as_mut()
            .is_some_and(VenueOffers::take_player_info_request);
        std::mem::take(&mut self.player_info) || offer
    }

    /// The mission of the offer open over it, when the offer's map key
    /// asked for the map since this was last asked, once.
    pub fn take_offer_map_request(&mut self) -> Option<MissionId> {
        self.offering.as_mut()?.take_map_request()
    }

    /// Whether [`MISSION_INFO_KEY`](crate::ui::mission_info::MISSION_INFO_KEY)
    /// has asked for the mission info screen since this was last asked,
    /// here or in its offer.
    pub fn take_info_request(&mut self) -> bool {
        let offer = self
            .offering
            .as_mut()
            .is_some_and(VenueOffers::take_info_request);
        std::mem::take(&mut self.info_requested) || offer
    }

    /// Whether the trade center has asked for an offer since this was last
    /// asked: whoever holds the session answers with
    /// [`TradeScreen::open_offer`], or, with none, rearms it
    /// ([`TradeScreen::rearm`]).
    pub fn take_offer_request(&mut self) -> bool {
        self.offering
            .as_mut()
            .is_some_and(VenueOffers::take_request)
    }

    /// Opens the offer dialog over the trade center on `prompt`.
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

    /// Lets go of its offer dialog answered and left drawn, if any
    /// ([`SpaceportView::release_answered_offer`](super::SpaceportView::release_answered_offer)).
    pub fn release_answered_offer(&mut self) {
        if let Some(offering) = &mut self.offering {
            offering.release_answered();
        }
    }

    /// Its offers play their movies as `movies` says.
    pub fn set_offer_movies(&mut self, movies: Movies) {
        if let Some(offering) = &mut self.offering {
            offering.set_movies(movies);
        }
    }

    /// Asks for the next offer after `gap`.
    pub fn rearm(&mut self, gap: Duration) {
        if let Some(offering) = &mut self.offering {
            offering.rearm(gap);
        }
    }

    /// Ticks only its offers' timer, as covered: something is open over
    /// the trade center.
    pub(crate) fn tick_covered(&mut self, dt: Duration) {
        if let Some(offering) = &mut self.offering {
            offering.tick(dt, true);
        }
    }

    /// The exchange shown.
    #[must_use]
    pub fn market(&self) -> &Market {
        &self.market
    }

    /// The list's colours.
    #[must_use]
    pub fn colors(&self) -> ListColors {
        self.colors
    }

    /// The selected row's index, if there are any rows.
    #[must_use]
    pub fn selected(&self) -> Option<usize> {
        (self.selected < self.market.rows.len()).then_some(self.selected)
    }

    /// The list position shown first: slot 0 and up, extra `jünk` rows
    /// counted (see the module docs).
    #[must_use]
    pub fn top(&self) -> usize {
        self.top
    }

    /// The dialog, if it could be laid out.
    #[must_use]
    pub fn dialog(&self) -> Option<&Dialog> {
        self.laid.as_ref().ok().map(|laid| &laid.dialog)
    }

    /// Why the exchange cannot be shown, if it cannot.
    #[must_use]
    pub fn problem(&self) -> Option<&str> {
        self.laid.as_ref().err().map(String::as_str)
    }

    /// Whether Done has been activated.
    #[must_use]
    pub fn closed(&self) -> bool {
        self.closed
    }

    /// The order asked for since it was last taken, once.
    pub fn take_order(&mut self) -> Option<Order> {
        self.order.take()
    }

    /// Shows `market`, the exchange after a trade, keeping the selection
    /// where it can.
    pub fn set_market(&mut self, market: Market) {
        self.market = market;
        self.select(self.selected);
    }

    /// Selects row `index`, or the last row when there is no such row, and
    /// scrolls the list to show its slot, never past the list's end.
    fn select(&mut self, index: usize) {
        let count = self.market.rows.len();
        self.selected = index.min(count.saturating_sub(1));
        let positions = positions(&self.market);
        let at = positions
            .iter()
            .position(|&row| row == Some(self.selected))
            .unwrap_or(0);
        self.top = self
            .top
            .min(positions.len().saturating_sub(ROWS))
            .min(at)
            .max((at + 1).saturating_sub(ROWS));
        self.regrey();
    }

    /// Selects the row of the next filled list position after the
    /// selected one (`forward`) or before it, if there is one.
    fn step(&mut self, forward: bool) {
        let positions = positions(&self.market);
        let Some(at) = positions.iter().position(|&row| row == Some(self.selected)) else {
            return;
        };
        let next = if forward {
            positions[at + 1..].iter().flatten().next()
        } else {
            positions[..at].iter().rev().flatten().next()
        };
        if let Some(&row) = next {
            self.select(row);
        }
    }

    /// Greys Buy and Sell when a ton could not be traded that way on the
    /// selected row.
    fn regrey(&mut self) {
        let buys = self.allows(Direction::Buy);
        let sells = self.allows(Direction::Sell);
        if let Ok(laid) = &mut self.laid {
            laid.dialog.set_greyed(BUY_ITEM, !buys);
            laid.dialog.set_greyed(SELL_ITEM, !sells);
        }
    }

    /// Whether Buy or Sell (`direction`) is enabled on the selected row
    /// ([`Market::row_allows`]).
    fn allows(&self, direction: Direction) -> bool {
        self.selected()
            .is_some_and(|index| self.market.row_allows(index, direction))
    }

    /// The quantity dialog, while it is open.
    #[must_use]
    pub fn quantity(&self) -> Option<&QuantityDialog> {
        self.quantity.dialog()
    }

    /// Asks to trade on the selected row `direction`, if that button is
    /// enabled: a plain trade, or with Alt held the quantity dialog (or by
    /// the other reading the most). `by_key` says B or S asked, whose
    /// character the dialog then drops. The order names the row, as a
    /// `jünk` can be listed on two at two prices.
    fn ask(&mut self, direction: Direction, by_key: bool) {
        let Some(index) = self.selected().filter(|_| self.allows(direction)) else {
            return;
        };
        let lot = match (self.quantity.alt_held(), self.market.option_asks_count()) {
            (false, _) => Lot::Click,
            (true, false) => Lot::Max,
            (true, true) => {
                if let (Some(max), Ok(laid)) = (self.market.row_max(index, direction), &self.laid) {
                    let metrics = Rc::clone(&laid.metrics.0);
                    self.quantity
                        .open((index, direction), direction, max, by_key, metrics);
                }
                return;
            }
        };
        self.order = Some(Order {
            row: index,
            good: self.market.rows[index].good,
            direction,
            lot,
        });
    }

    /// Activates dialog item `item`: Done closes, Buy and Sell ask, and a
    /// row with a good in it is selected. Any other item does nothing.
    fn activate(&mut self, item: usize) {
        match item {
            DONE_ITEM => self.closed = true,
            BUY_ITEM => self.ask(Direction::Buy, false),
            SELL_ITEM => self.ask(Direction::Sell, false),
            slot if (FIRST_ROW_ITEM..FIRST_ROW_ITEM + ROWS).contains(&slot) => {
                let at = self.top + (slot - FIRST_ROW_ITEM);
                if let Some(Some(row)) = positions(&self.market).get(at) {
                    self.select(*row);
                }
            }
            _ => {}
        }
    }

    /// The header, item 3 (see the module docs).
    fn draw_header(&self, laid: &Laid, list: &mut DrawList) {
        let Some(header) = laid.dialog.item_bounds(HEADER_ITEM) else {
            return;
        };
        frame(list, header, DARK_GREY);
        let words = &self.market.words;
        let baseline = header.min.y + HEADER_BASELINE;
        let labels = [
            (&*words.commodity, header.min.x + HEADER_NAME_X),
            (
                words.held_in(self.market.held_in()),
                header.max.x - HELD_HEADER_FROM_RIGHT,
            ),
            (&*words.price, header.max.x - MARKER_FROM_RIGHT),
        ];
        for (label, x) in labels {
            text_at(list, label, x, baseline, LIGHT_GREY);
        }
    }

    /// The eight slots shown, items 4 to 11 (see the module docs).
    fn draw_slots(&self, laid: &Laid, list: &mut DrawList) {
        let positions = positions(&self.market);
        let selected = self.selected();
        for shown in 0..ROWS {
            let Some(slot) = laid.dialog.item_bounds(FIRST_ROW_ITEM + shown) else {
                continue;
            };
            let row = positions.get(self.top + shown).copied().flatten();
            let fill = if row.is_some() && row == selected {
                self.colors.hilite
            } else {
                self.colors.background
            };
            fill_rect(list, inset(slot, 1.0, 1.0), fill);
            let (left, top) = (slot.min.x, slot.min.y);
            let (right, bottom) = (slot.max.x - 1.0, slot.max.y - 1.0);
            rule(
                list,
                Point::new(left, top),
                Point::new(left, bottom),
                DARK_GREY,
            );
            rule(
                list,
                Point::new(right, top),
                Point::new(right, bottom),
                DARK_GREY,
            );
            if shown == 0 {
                rule(
                    list,
                    Point::new(left, top),
                    Point::new(right, top),
                    DARK_GREY,
                );
            }
            if shown == ROWS - 1 {
                rule(
                    list,
                    Point::new(left, bottom),
                    Point::new(right, bottom),
                    DARK_GREY,
                );
            }
            if let Some(row) = row {
                self.draw_row(laid, list, slot, &self.market.rows[row]);
            }
        }
    }

    /// `row`'s text in its `slot`: name, tons held, marker and price.
    fn draw_row(&self, laid: &Laid, list: &mut DrawList, slot: Bounds, row: &nova_sim::MarketRow) {
        let color = self.colors.text;
        let baseline = slot.min.y + ROW_BASELINE;
        let name_x = match row.good {
            nova_sim::Good::Commodity(_) => NAME_X,
            nova_sim::Good::Junk(_) => JUNK_NAME_X,
        };
        text_at(list, &row.name, slot.min.x + name_x, baseline, color);
        let ending = |list: &mut DrawList, text: &str, end: f32| {
            let width = laid.metrics.0.width(TEXT_FONT, TEXT_SIZE, text);
            text_at(list, text, slot.max.x - end - width, baseline, color);
        };
        if row.held > 0 {
            ending(list, &row.held.to_string(), HELD_END);
        }
        if let Some(marker) = row.marker {
            let word = self.market.words.marker(marker);
            text_at(list, word, slot.max.x - MARKER_FROM_RIGHT, baseline, color);
        }
        ending(list, &row.price.to_string(), PRICE_END);
    }

    /// The cargo summary, item 12, and the event line, item 15, when
    /// there is one (see the module docs).
    fn draw_boxes(&self, laid: &Laid, list: &mut DrawList) {
        if let Some(summary) = laid.dialog.item_bounds(SUMMARY_ITEM) {
            let (dx, dy) = SUMMARY_INSET;
            let text = self.market.summary.text(&self.market.words);
            inverted_box(laid, list, &text, inset(summary, dx, dy));
            frame(list, summary, DARK_GREY);
        }
        if !self.market.events.is_empty()
            && let Some(event) = laid.dialog.item_bounds(EVENT_ITEM)
        {
            inverted_box(laid, list, &self.market.events.join("\r"), event);
        }
    }
}

/// `text` in the dialog's font, starting at `x` on `baseline`, its top a
/// size above it.
fn text_at(list: &mut DrawList, text: &str, x: f32, baseline: f32, color: Color) {
    let origin = Point::new(x, baseline - TEXT_SIZE);
    list.text_in(TEXT_FONT, text, origin, TEXT_SIZE, None, color);
}

/// `text` wrapped into `rect` by `TETextBox`, black on white, then
/// inverted by `InvertRect`: white on black.
fn inverted_box(laid: &Laid, list: &mut DrawList, text: &str, rect: Bounds) {
    fill_rect(list, rect, BOX_BACKGROUND);
    TextBox::new(text, TEXT_FONT, TEXT_SIZE, BOX_TEXT, rect, &laid.metrics.0).draw(list);
}

impl Screen for TradeScreen {
    /// Every input goes to the exchange, or to the quantity dialog while
    /// it is open; it never quits.
    fn input(&mut self, input: &Input) -> ScreenAction {
        if let Some(offering) = &mut self.offering
            && offering.input(input, &mut self.sounds)
        {
            return ScreenAction::None;
        }
        if self.quantity.is_open() {
            if let Some(((row, direction), count)) = self.quantity.input(input, &mut self.sounds) {
                self.order = Some(Order {
                    row,
                    good: self.market.rows[row].good,
                    direction,
                    lot: Lot::Count(count),
                });
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
        if is_player_info_press(input) {
            self.player_info = true;
            return ScreenAction::None;
        }
        if is_mission_info_press(input) {
            self.info_requested = true;
            return ScreenAction::None;
        }
        match *input {
            Input::Key {
                key: MAX_LOT_KEY, ..
            } => self.quantity.follow_alt(input),
            Input::Key {
                key: Key::Up,
                pressed: true,
                ..
            } => self.step(false),
            Input::Key {
                key: Key::Down,
                pressed: true,
                ..
            } => self.step(true),
            Input::Key {
                key: BUY_KEY,
                pressed: true,
                ..
            } => self.ask(Direction::Buy, true),
            Input::Key {
                key: SELL_KEY,
                pressed: true,
                ..
            } => self.ask(Direction::Sell, true),
            _ => {
                let event = laid.dialog.input(input);
                if let Some(DialogEvent::Item(item)) = event {
                    self.activate(item);
                }
            }
        }
        ScreenAction::None
    }

    /// The wait for the next offer runs down, the quantity dialog
    /// covering the trade center while it is open.
    fn tick(&mut self, dt: Duration) {
        let covered = self.quantity.is_open();
        if let Some(offering) = &mut self.offering {
            offering.tick(dt, covered);
        }
    }

    fn draw(&self, list: &mut DrawList) {
        let laid = match &self.laid {
            Ok(laid) => laid,
            Err(reason) => {
                list.text(
                    format!(
                        "Cannot show the trade center: {reason}. Press Return or Escape to close it."
                    ),
                    PROBLEM_AT,
                    PROBLEM_SIZE,
                    None,
                    Color::ERROR,
                );
                return;
            }
        };
        let dialog = &laid.dialog;
        list.picture(BACKGROUND, dialog.bounds().min);
        self.draw_header(laid, list);
        self.draw_slots(laid, list);
        self.draw_boxes(laid, list);
        dialog.draw(list);
        self.quantity.draw(list);
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
        if !self.quantity.cancel_pointer()
            && let Ok(laid) = &mut self.laid
        {
            laid.dialog.cancel_pointer();
        }
    }

    /// Lets go of Alt, and of the keys held in the quantity dialog.
    fn release_keys(&mut self) {
        self.quantity.release_keys();
    }

    /// The quantity dialog's and the offer's sounds, in order. The
    /// buttons are silent under the pointer, as `_TrackTradeButtons`
    /// @0x294cd plays nothing.
    fn take_sounds(&mut self) -> Vec<Sound> {
        std::mem::take(&mut self.sounds)
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use nova_sim::trade_dialog::cargo_summary;
    use nova_sim::{
        Good, JunkId, MarketRow, PriceMarker, RuleKey, RuleSource, Rulebook, TradeWords,
    };

    use super::*;
    use crate::draw::DrawCommand;
    use crate::input::MouseButton;
    use crate::sound::UiSound;
    use crate::text::fixture::MonoMetrics;
    use crate::ui::dialog::{ItemSpec, ItemTemplate, Placement};

    fn rect(x: f32, y: f32, w: f32, h: f32) -> Bounds {
        Bounds::at(Point::new(x, y), w, h)
    }

    /// Stock "Trade": `DLOG` 1001, 426 x 252 and centred, and its
    /// seventeen user items, where 2, 3, 12, 15, 16 and 17 are disabled
    /// and 2, 16 and 17 parked outside it.
    fn template() -> DialogTemplate {
        let items = [
            (272.0, 221.0, 99.0, 25.0, true),
            (129.0, 299.0, 68.0, 30.0, false),
            (38.0, 9.0, 352.0, 17.0, false),
            (38.0, 25.0, 352.0, 13.0, true),
            (38.0, 37.0, 352.0, 13.0, true),
            (38.0, 49.0, 352.0, 13.0, true),
            (38.0, 61.0, 352.0, 13.0, true),
            (38.0, 73.0, 352.0, 13.0, true),
            (38.0, 85.0, 352.0, 14.0, true),
            (38.0, 98.0, 352.0, 14.0, true),
            (38.0, 111.0, 352.0, 14.0, true),
            (38.0, 124.0, 352.0, 60.0, false),
            (60.0, 221.0, 99.0, 25.0, true),
            (166.0, 221.0, 99.0, 25.0, true),
            (41.0, 190.0, 346.0, 24.0, false),
            (212.0, 304.0, 68.0, 30.0, false),
            (178.0, 339.0, 68.0, 30.0, false),
        ];
        DialogTemplate {
            bounds: rect(32.0, 35.0, 426.0, 252.0),
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

    /// Where the dialog goes: (1024 - 426) / 2, (768 - 252) / 2.
    const ORIGIN: Point = Point::new(299.0, 258.0);

    const FOOD: Good = Good::Commodity(0);
    const METAL: Good = Good::Commodity(4);
    const OPALS: Good = Good::Junk(JunkId(146));

    /// A row of `good`, traded both ways, unmarked: a commodity in its
    /// own slot, a `jünk` in slot 6 (bought here).
    fn row(good: Good, name: &str, price: i64, held: u32) -> MarketRow {
        let slot = match good {
            Good::Commodity(n) => n,
            Good::Junk(_) => 6,
        };
        MarketRow {
            good,
            name: name.to_owned(),
            price,
            held,
            sold_here: true,
            bought_here: true,
            slot,
            marker: None,
        }
    }

    /// Food at 93 (2 held, lowered), metal at 200 (medium), and opals
    /// bought here only at 1500 (1 held, high), for a player with 1000
    /// credits and 7 of 10 tons free, during a food surplus.
    fn market() -> Market {
        Market {
            rows: vec![
                MarketRow {
                    marker: Some(PriceMarker::Lower),
                    ..row(FOOD, "Food", 93, 2)
                },
                MarketRow {
                    marker: Some(PriceMarker::Medium),
                    ..row(METAL, "Metal", 200, 0)
                },
                MarketRow {
                    sold_here: false,
                    marker: Some(PriceMarker::High),
                    ..row(OPALS, "Opals", 1500, 1)
                },
            ],
            events: vec!["An enormous food surplus".to_owned()],
            cash: 1000,
            capacity: 10,
            ship_capacity: 10,
            free: 7,
            summary: cargo_summary(&[], [], 10, 10, 3),
            ..Market::default()
        }
    }

    fn layout() -> (DialogTemplate, Rc<dyn TextMetrics>) {
        (template(), Rc::new(MonoMetrics))
    }

    fn screen_of(market: Market) -> TradeScreen {
        TradeScreen::new(Ok(layout()), Ok(quantity()), market, ButtonStyle::STOCK)
    }

    /// Stock `DITL` 1003, fixed at (10, 20): OK (1), the prompt (2), the
    /// field (3) and Cancel (4).
    fn quantity() -> DialogTemplate {
        let item = |x, y, w, h, kind| ItemTemplate {
            bounds: rect(x, y, w, h),
            enabled: true,
            kind,
        };
        DialogTemplate {
            bounds: rect(10.0, 20.0, 172.0, 72.0),
            placement: Placement::Fixed,
            items: vec![
                item(92.0, 42.0, 70.0, 20.0, ItemSpec::Button("OK".into())),
                item(6.0, 8.0, 102.0, 16.0, ItemSpec::StaticText("^0".into())),
                item(112.0, 8.0, 51.0, 16.0, ItemSpec::EditText("12345".into())),
                item(10.0, 42.0, 70.0, 20.0, ItemSpec::Button("Cancel".into())),
            ],
        }
    }

    fn screen() -> TradeScreen {
        screen_of(market())
    }

    fn drawn(screen: &TradeScreen) -> Vec<DrawCommand> {
        let mut list = DrawList::new();
        screen.draw(&mut list);
        list.iter().cloned().collect()
    }

    fn texts(commands: &[DrawCommand]) -> Vec<(String, Point)> {
        commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, origin, .. } => Some((text.clone(), *origin)),
                _ => None,
            })
            .collect()
    }

    fn item(screen: &TradeScreen, number: usize) -> Bounds {
        screen
            .dialog()
            .expect("laid out")
            .item_bounds(number)
            .expect("an item")
    }

    fn key(key: Key, pressed: bool, repeat: bool) -> Input {
        Input::Key {
            key,
            pressed,
            repeat,
        }
    }

    fn press(screen: &mut TradeScreen, k: Key) {
        screen.input(&key(k, true, false));
    }

    fn click(screen: &mut TradeScreen, at: Point) {
        for pressed in [true, false] {
            screen.input(&Input::PointerButton {
                button: MouseButton::Left,
                pressed,
                at,
            });
        }
    }

    fn click_item(screen: &mut TradeScreen, number: usize) {
        let at = item(screen, number).center();
        click(screen, at);
    }

    fn order(row: usize, good: Good, direction: Direction, lot: Lot) -> Order {
        Order {
            row,
            good,
            direction,
            lot,
        }
    }

    #[test]
    fn the_named_values() {
        assert_eq!(TRADE_DIALOG, 1001);
        assert_eq!(BACKGROUND, ImageKey::picture(8510));
        assert_eq!(
            [
                DONE_ITEM,
                HEADER_ITEM,
                FIRST_ROW_ITEM,
                ROWS,
                SUMMARY_ITEM,
                BUY_ITEM,
                SELL_ITEM,
                EVENT_ITEM
            ],
            [1, 3, 4, 8, 12, 13, 14, 15]
        );
        assert_eq!((BUY_LABEL, SELL_LABEL), ("Buy", "Sell"));
        assert_eq!(
            (BUY_KEY, SELL_KEY, MAX_LOT_KEY),
            (Key::Char('b'), Key::Char('s'), Key::Alt)
        );
        assert_eq!((TEXT_FONT, TEXT_SIZE), (Font::Geneva, 9.0));
        assert_eq!(DARK_GREY, Color::rgba(0x40, 0x40, 0x40, 255));
        assert_eq!(LIGHT_GREY, Color::rgba(0xC0, 0xC0, 0xC0, 255));
        assert_eq!(
            [
                NAME_X,
                JUNK_NAME_X,
                HELD_END,
                MARKER_FROM_RIGHT,
                PRICE_END,
                ROW_BASELINE,
                HEADER_BASELINE,
                HEADER_NAME_X,
                HELD_HEADER_FROM_RIGHT
            ],
            [7.0, 6.0, 110.0, 70.0, 10.0, 10.0, 12.0, 7.0, 123.0]
        );
        assert_eq!(SUMMARY_INSET, (5.0, 3.0));
        assert_eq!(
            ListColors::STOCK,
            ListColors {
                text: Color::WHITE,
                background: Color::BLACK,
                hilite: Color::rgba(0x80, 0, 0, 255),
            }
        );
    }

    #[test]
    fn it_is_centred_with_the_first_row_selected() {
        let screen = screen();
        assert_eq!(screen.problem(), None);
        assert_eq!(screen.dialog().expect("laid out").bounds().min, ORIGIN);
        assert_eq!(screen.selected(), Some(0));
        assert_eq!(screen.top(), 0);
        assert!(!screen.closed());
        assert_eq!(screen.market(), &market());
        assert_eq!(screen.colors(), ListColors::STOCK);
        assert_eq!(item(&screen, 4), rect(337.0, 283.0, 352.0, 13.0));
    }

    /// Each text drawn, with where it starts, its size and colour.
    fn styled(commands: &[DrawCommand]) -> Vec<(String, Point, Font, f32, Color)> {
        commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text {
                    text,
                    origin,
                    font,
                    size,
                    color,
                    ..
                } => Some((text.clone(), *origin, *font, *size, *color)),
                _ => None,
            })
            .collect()
    }

    /// Where and in what colour `wanted` is drawn, if it is, in Geneva 9.
    fn drawn_at(commands: &[DrawCommand], wanted: &str) -> Option<(Point, Color)> {
        styled(commands)
            .into_iter()
            .find(|(text, ..)| text == wanted)
            .map(|(_, at, font, size, color)| {
                assert_eq!((font, size), (Font::Geneva, 9.0), "{wanted}");
                (at, color)
            })
    }

    /// Text starting at `x` on `baseline`: its top 9 above.
    fn on(x: f32, baseline: f32) -> Point {
        Point::new(x, baseline - 9.0)
    }

    /// `rect` with `dx` off each side and `dy` off its top and bottom,
    /// worked out here rather than by the screen's own `inset`.
    fn shrunk(rect: Bounds, dx: f32, dy: f32) -> Bounds {
        Bounds::at(
            Point::new(rect.min.x + dx, rect.min.y + dy),
            rect.width() - 2.0 * dx,
            rect.height() - 2.0 * dy,
        )
    }

    /// How many times `part` is drawn in `commands`.
    fn count(commands: &[DrawCommand], part: &[DrawCommand]) -> usize {
        commands
            .windows(part.len())
            .filter(|window| *window == part)
            .count()
    }

    /// `rect` filled in `color`.
    fn filled(rect: Bounds, color: Color) -> Vec<DrawCommand> {
        let mut list = DrawList::new();
        fill_rect(&mut list, rect, color);
        list.iter().cloned().collect()
    }

    /// A one-pixel dark grey rule from (x0, y0) to (x1, y1), ends included.
    fn ruled(x0: f32, y0: f32, x1: f32, y1: f32) -> Vec<DrawCommand> {
        filled(
            Bounds {
                min: Point::new(x0, y0),
                max: Point::new(x1 + 1.0, y1 + 1.0),
            },
            DARK_GREY,
        )
    }

    /// `rect` framed in dark grey.
    fn framed(rect: Bounds) -> Vec<DrawCommand> {
        let (l, t, r, b) = (rect.min.x, rect.min.y, rect.max.x - 1.0, rect.max.y - 1.0);
        [
            ruled(l, t, r, t),
            ruled(l, b, r, b),
            ruled(l, t, l, b),
            ruled(r, t, r, b),
        ]
        .concat()
    }

    fn contains(commands: &[DrawCommand], part: &[DrawCommand]) -> bool {
        commands.windows(part.len()).any(|window| window == part)
    }

    /// Where `part` starts in `commands`.
    fn index_of(commands: &[DrawCommand], part: &[DrawCommand]) -> usize {
        commands
            .windows(part.len())
            .position(|window| window == part)
            .expect("drawn")
    }

    #[test]
    fn it_draws_the_background_the_header_the_slots_the_boxes_then_the_dialog() {
        let screen = screen();
        let commands = drawn(&screen);
        assert_eq!(
            commands[0],
            DrawCommand::Picture {
                image: BACKGROUND,
                top_left: ORIGIN,
            }
        );
        let header = index_of(&commands, &framed(item(&screen, 3)));
        let first_slot = index_of(
            &commands,
            &filled(
                shrunk(item(&screen, 4), 1.0, 1.0),
                Color::rgba(0x80, 0, 0, 255),
            ),
        );
        let summary = index_of(&commands, &framed(item(&screen, 12)));
        let event = index_of(&commands, &filled(item(&screen, 15), Color::BLACK));
        assert!(header < first_slot && first_slot < summary && summary < event);
        let mut dialog = DrawList::new();
        screen.dialog().expect("laid out").draw(&mut dialog);
        let buttons: Vec<DrawCommand> = dialog.iter().cloned().collect();
        assert_eq!(commands[commands.len() - buttons.len()..], buttons[..]);
        let labels: Vec<_> = texts(&buttons).into_iter().map(|(t, _)| t).collect();
        assert_eq!(labels, ["Done", "Buy", "Sell"]);
    }

    #[test]
    fn the_header_is_framed_dark_grey_with_light_grey_labels_in_geneva_9() {
        let screen = screen();
        let header = item(&screen, 3);
        let commands = drawn(&screen);
        assert!(contains(&commands, &framed(header)));
        let baseline = header.min.y + 12.0;
        assert_eq!(
            drawn_at(&commands, "Commodity:"),
            Some((on(header.min.x + 7.0, baseline), LIGHT_GREY))
        );
        assert_eq!(
            drawn_at(&commands, "In Hold:"),
            Some((on(header.max.x - 123.0, baseline), LIGHT_GREY))
        );
        assert_eq!(
            drawn_at(&commands, "Price:"),
            Some((on(header.max.x - 70.0, baseline), LIGHT_GREY))
        );
        assert_eq!(drawn_at(&commands, "In Fleet:"), None);
        let fleet = screen_of(Market {
            capacity: 15,
            ..market()
        });
        let commands = drawn(&fleet);
        assert_eq!(
            drawn_at(&commands, "In Fleet:"),
            Some((on(header.max.x - 123.0, baseline), LIGHT_GREY))
        );
        assert_eq!(drawn_at(&commands, "In Hold:"), None);
    }

    #[test]
    fn the_header_and_markers_are_told_in_the_markets_words() {
        let mut words = TradeWords::STOCK;
        words.commodity = "Ware:".into();
        words.lower = "Down".into();
        let commands = drawn(&screen_of(Market { words, ..market() }));
        assert!(drawn_at(&commands, "Ware:").is_some());
        assert!(drawn_at(&commands, "Down").is_some());
        assert_eq!(drawn_at(&commands, "Commodity:"), None);
    }

    #[test]
    fn rows_are_drawn_in_their_slots_with_empty_slots_framed() {
        let screen = screen();
        let commands = drawn(&screen);
        let at = |number: usize, dx: f32| {
            let slot = item(&screen, number);
            on(slot.min.x + dx, slot.min.y + 10.0)
        };
        assert_eq!(
            drawn_at(&commands, "Food"),
            Some((at(4, 7.0), Color::WHITE))
        );
        assert_eq!(
            drawn_at(&commands, "Metal"),
            Some((at(8, 7.0), Color::WHITE))
        );
        assert_eq!(
            drawn_at(&commands, "Opals"),
            Some((at(10, 6.0), Color::WHITE))
        );
        for number in [5, 6, 7, 9, 11] {
            let slot = item(&screen, number);
            assert!(
                contains(&commands, &filled(shrunk(slot, 1.0, 1.0), Color::BLACK)),
                "{number} filled"
            );
            assert!(
                contains(
                    &commands,
                    &ruled(slot.min.x, slot.min.y, slot.min.x, slot.max.y - 1.0)
                ),
                "{number} edged"
            );
            let inside = styled(&commands)
                .into_iter()
                .filter(|(_, origin, ..)| slot.contains(Point::new(origin.x, origin.y + 9.0)))
                .count();
            assert_eq!(inside, 0, "{number} has no text");
        }
    }

    #[test]
    fn the_selected_slot_is_filled_in_the_list_hilite() {
        let colors = ListColors {
            text: Color::rgba(1, 2, 3, 255),
            background: Color::rgba(4, 5, 6, 255),
            hilite: Color::rgba(7, 8, 9, 255),
        };
        let mut screen = screen().with_colors(colors);
        let slot = |screen: &TradeScreen, number| shrunk(item(screen, number), 1.0, 1.0);
        let commands = drawn(&screen);
        assert!(contains(
            &commands,
            &filled(slot(&screen, 4), colors.hilite)
        ));
        for number in 5..=11 {
            assert!(contains(
                &commands,
                &filled(slot(&screen, number), colors.background)
            ));
        }
        let fill = index_of(&commands, &filled(slot(&screen, 4), colors.hilite));
        let food = commands
            .iter()
            .position(|c| matches!(c, DrawCommand::Text { text, .. } if text == "Food"))
            .expect("drawn");
        assert!(fill < food, "under the text");
        assert_eq!(
            drawn_at(&commands, "Food").map(|(_, c)| c),
            Some(colors.text)
        );
        press(&mut screen, Key::Down);
        let commands = drawn(&screen);
        assert!(contains(
            &commands,
            &filled(slot(&screen, 4), colors.background)
        ));
        assert!(contains(
            &commands,
            &filled(slot(&screen, 8), colors.hilite)
        ));
    }

    #[test]
    fn edges_left_and_right_on_every_slot_top_on_item_4_bottom_on_item_11() {
        let screen = screen();
        let commands = drawn(&screen);
        for number in 4..=11 {
            let slot = item(&screen, number);
            let (l, t) = (slot.min.x, slot.min.y);
            let (r, b) = (slot.max.x - 1.0, slot.max.y - 1.0);
            assert!(contains(&commands, &ruled(l, t, l, b)), "{number} left");
            assert!(contains(&commands, &ruled(r, t, r, b)), "{number} right");
            assert_eq!(
                contains(&commands, &ruled(l, t, r, t)),
                number == 4,
                "{number} top"
            );
            // Item 11's bottom is also item 12's frame's top: drawn twice.
            let bottoms = count(&commands, &ruled(l, b, r, b));
            assert_eq!(bottoms, if number == 11 { 2 } else { 0 }, "{number} bottom");
        }
    }

    #[test]
    fn a_junk_name_starts_one_pixel_left_of_a_commoditys() {
        let screen = screen();
        let commands = drawn(&screen);
        let food = drawn_at(&commands, "Food").expect("drawn").0;
        let opals = drawn_at(&commands, "Opals").expect("drawn").0;
        assert_eq!(food.x - item(&screen, 4).min.x, 7.0);
        assert_eq!(opals.x - item(&screen, 10).min.x, 6.0);
    }

    #[test]
    fn the_held_count_is_drawn_right_aligned_at_right_110_only_above_0() {
        let screen = screen_of(Market {
            rows: vec![row(FOOD, "Food", 93, 12), row(METAL, "Metal", 200, 0)],
            ..market()
        });
        let commands = drawn(&screen);
        let food = item(&screen, 4);
        // "12" is 9 wide in Geneva 9 by the fixture's metrics.
        assert_eq!(
            drawn_at(&commands, "12").map(|(at, _)| at),
            Some(on(food.max.x - 110.0 - 9.0, food.min.y + 10.0))
        );
        assert_eq!(drawn_at(&commands, "0"), None, "none held draws nothing");
    }

    #[test]
    fn the_price_is_drawn_signed_right_aligned_at_right_10() {
        let screen = screen_of(Market {
            rows: vec![row(FOOD, "Food", 93, 0), row(OPALS, "Opals", -1200, 0)],
            ..market()
        });
        let commands = drawn(&screen);
        let food = item(&screen, 4);
        let opals = item(&screen, 10);
        assert_eq!(
            drawn_at(&commands, "93").map(|(at, _)| at),
            Some(on(food.max.x - 10.0 - 9.0, food.min.y + 10.0))
        );
        assert_eq!(
            drawn_at(&commands, "-1200").map(|(at, _)| at),
            Some(on(opals.max.x - 10.0 - 22.5, opals.min.y + 10.0))
        );
    }

    #[test]
    fn a_row_with_no_listing_draws_no_text() {
        let screen = screen_of(Market {
            rows: Vec::new(),
            ..market()
        });
        let commands = drawn(&screen);
        for number in 4..=11 {
            let slot = item(&screen, number);
            let inside = styled(&commands)
                .into_iter()
                .filter(|(_, origin, ..)| slot.contains(Point::new(origin.x, origin.y + 9.0)))
                .count();
            assert_eq!(inside, 0, "{number}");
            assert!(contains(
                &commands,
                &filled(shrunk(slot, 1.0, 1.0), Color::BLACK)
            ));
        }
        assert_eq!(screen.selected(), None);
    }

    #[test]
    fn each_marker_is_drawn_at_right_70() {
        let markers = [
            (PriceMarker::Higher, "Higher"),
            (PriceMarker::Lower, "Lower"),
            (PriceMarker::Low, "Low"),
            (PriceMarker::Medium, "Med"),
            (PriceMarker::High, "High"),
        ];
        let rows = (0..5)
            .zip(markers)
            .map(|(n, (marker, _))| MarketRow {
                marker: Some(marker),
                ..row(Good::Commodity(n), &format!("Good {n}"), 10, 0)
            })
            .collect();
        let screen = screen_of(Market { rows, ..market() });
        let commands = drawn(&screen);
        for (n, (_, word)) in markers.into_iter().enumerate() {
            let slot = item(&screen, 4 + n);
            assert_eq!(
                drawn_at(&commands, word).map(|(at, _)| at),
                Some(on(slot.max.x - 70.0, slot.min.y + 10.0)),
                "{word}"
            );
        }
        let unmarked = screen_of(Market {
            rows: vec![row(FOOD, "Food", 93, 0)],
            ..market()
        });
        let words: Vec<_> = texts(&drawn(&unmarked))
            .into_iter()
            .map(|(t, _)| t)
            .collect();
        for (_, word) in markers {
            assert!(!words.contains(&word.to_owned()), "{word}");
        }
    }

    #[test]
    fn the_cargo_summary_is_white_on_black_in_item_12_inset_5_3_and_framed() {
        let screen = screen_of(Market {
            summary: cargo_summary(&[], [4], 10, 30, 6),
            ..market()
        });
        let summary = item(&screen, 12);
        let boxed = shrunk(summary, 5.0, 3.0);
        let commands = drawn(&screen);
        let fill = index_of(&commands, &filled(boxed, Color::BLACK));
        let frame = index_of(&commands, &framed(summary));
        assert!(fill < frame);
        let mut expected = DrawList::new();
        TextBox::new(
            "Other cargo: 4 tons of miscellaneous goods\r\r\
             Free cargo space in your ship: 10 tons\r\
             Free cargo space in your fleet: 24 tons",
            Font::Geneva,
            9.0,
            Color::WHITE,
            boxed,
            &MonoMetrics,
        )
        .draw(&mut expected);
        let expected: Vec<DrawCommand> = expected.iter().cloned().collect();
        assert_eq!(expected.len(), 3, "a blank line between");
        assert!(contains(&commands, &expected));
        assert!(index_of(&commands, &expected) > fill);
    }

    #[test]
    fn the_event_line_is_wrapped_white_on_black_in_item_15() {
        let sentence = "An enormous food surplus has lowered the price of food at the \
                        market here, and the traders are worried.";
        let screen = screen_of(Market {
            events: vec![sentence.to_owned()],
            ..market()
        });
        let event = item(&screen, EVENT_ITEM);
        let commands = drawn(&screen);
        let boxed = TextBox::new(
            sentence,
            Font::Geneva,
            9.0,
            Color::WHITE,
            event,
            &MonoMetrics,
        );
        assert_eq!(boxed.lines().len(), 2, "wrapped to item 15's width");
        let mut expected = DrawList::new();
        boxed.draw(&mut expected);
        let expected: Vec<DrawCommand> = expected.iter().cloned().collect();
        let fill = index_of(&commands, &filled(event, Color::BLACK));
        assert!(index_of(&commands, &expected) > fill);
        let names = screen_of(Market {
            events: vec!["A glut".to_owned(), "A drought".to_owned()],
            ..market()
        });
        assert_eq!(
            drawn_at(&drawn(&names), "A drought").map(|(at, _)| at),
            Some(Point::new(event.min.x, event.min.y + 9.0 * 1.2)),
            "a line each"
        );
    }

    #[test]
    fn no_event_draws_nothing_in_item_15() {
        let screen = screen_of(Market {
            events: Vec::new(),
            ..market()
        });
        let event = item(&screen, EVENT_ITEM);
        let commands = drawn(&screen);
        assert!(!contains(&commands, &filled(event, Color::BLACK)));
        let inside = styled(&commands)
            .into_iter()
            .filter(|(_, origin, ..)| event.contains(*origin))
            .count();
        assert_eq!(inside, 0);
    }

    #[test]
    fn there_is_no_status_line() {
        let words: Vec<_> = texts(&drawn(&screen()))
            .into_iter()
            .map(|(t, _)| t)
            .collect();
        assert!(
            !words.iter().any(|text| text.contains("Credits")),
            "{words:?}"
        );
        assert!(
            words.contains(&"Free cargo space: 7 tons".to_owned()),
            "{words:?}"
        );
    }

    #[test]
    fn up_and_down_move_the_selection_and_stop_at_the_ends() {
        let mut screen = screen();
        press(&mut screen, Key::Up);
        assert_eq!(screen.selected(), Some(0));
        press(&mut screen, Key::Down);
        assert_eq!(screen.selected(), Some(1));
        screen.input(&key(Key::Down, true, true));
        assert_eq!(screen.selected(), Some(2), "repeats too");
        press(&mut screen, Key::Down);
        assert_eq!(screen.selected(), Some(2));
        screen.input(&key(Key::Up, false, false));
        assert_eq!(screen.selected(), Some(2), "not on release");
        press(&mut screen, Key::Up);
        assert_eq!(screen.selected(), Some(1));
    }

    #[test]
    fn up_and_down_skip_empty_slots() {
        // Food (slot 0) and metal (slot 4), with slots 1 to 3 empty, and
        // a sold jünk (slot 7) listed before a bought one (slot 6).
        let mut screen = screen_of(Market {
            rows: vec![
                row(FOOD, "Food", 93, 0),
                row(METAL, "Metal", 200, 0),
                MarketRow {
                    slot: 7,
                    ..row(Good::Junk(JunkId(134)), "Water", 240, 0)
                },
                row(OPALS, "Opals", 1500, 0),
            ],
            ..market()
        });
        let mut visited = vec![screen.selected()];
        for _ in 0..4 {
            press(&mut screen, Key::Down);
            visited.push(screen.selected());
        }
        assert_eq!(visited, [Some(0), Some(1), Some(3), Some(2), Some(2)]);
        press(&mut screen, Key::Up);
        assert_eq!(screen.selected(), Some(3), "back to slot 6");
    }

    #[test]
    fn the_first_row_in_the_list_is_selected() {
        let screen = screen_of(Market {
            rows: vec![
                MarketRow {
                    slot: 7,
                    ..row(Good::Junk(JunkId(134)), "Water", 240, 0)
                },
                row(OPALS, "Opals", 1500, 0),
            ],
            ..market()
        });
        assert_eq!(screen.selected(), Some(1), "slot 6 before slot 7");
    }

    #[test]
    fn a_click_on_a_slot_selects_its_row_and_on_an_empty_slot_nothing() {
        let mut screen = screen();
        click_item(&mut screen, 10);
        assert_eq!(screen.selected(), Some(2));
        click_item(&mut screen, 4);
        assert_eq!(screen.selected(), Some(0));
        click_item(&mut screen, 8);
        assert_eq!(screen.selected(), Some(1));
        for empty in [5, 6, 7, 9, 11] {
            click_item(&mut screen, empty);
            assert_eq!(screen.selected(), Some(1), "{empty} is empty");
        }
        assert_eq!(screen.take_sounds(), [], "rows are silent");
        assert_eq!(screen.take_order(), None);
    }

    /// `count` `jünk` bought here, numbered from 0, each 10 a ton: rows
    /// all in slot 6, after six empty commodity slots and before an empty
    /// slot 7.
    fn long(count: usize) -> Market {
        Market {
            rows: (0..count)
                .map(|n| {
                    let id = i16::try_from(n).expect("few");
                    row(Good::Junk(JunkId(id)), &format!("Good {n}"), 10, 0)
                })
                .collect(),
            ..market()
        }
    }

    fn names(screen: &TradeScreen) -> Vec<String> {
        texts(&drawn(screen))
            .into_iter()
            .map(|(text, _)| text)
            .filter(|text| text.starts_with("Good "))
            .collect()
    }

    #[test]
    fn extra_junk_rows_follow_their_slot_and_scroll() {
        let mut screen = screen_of(long(4));
        // Slots 0-5 empty, then goods 0-3, then slot 7: 11 positions, and
        // the first row shown scrolled into view.
        assert_eq!((screen.selected(), screen.top()), (Some(0), 0));
        assert_eq!(names(&screen), ["Good 0", "Good 1"]);
        press(&mut screen, Key::Down);
        press(&mut screen, Key::Down);
        assert_eq!((screen.selected(), screen.top()), (Some(2), 1));
        assert_eq!(names(&screen), ["Good 0", "Good 1", "Good 2"]);
        let good_2 = item(&screen, 11);
        assert!(contains(
            &drawn(&screen),
            &filled(shrunk(good_2, 1.0, 1.0), Color::rgba(0x80, 0, 0, 255))
        ));
        press(&mut screen, Key::Down);
        assert_eq!((screen.selected(), screen.top()), (Some(3), 2));
        press(&mut screen, Key::Down);
        assert_eq!((screen.selected(), screen.top()), (Some(3), 2), "the end");
        click_item(&mut screen, 4);
        assert_eq!(screen.selected(), Some(3), "slot 2, scrolled, is empty");
        click_item(&mut screen, 8);
        assert_eq!(screen.selected(), Some(0));
        press(&mut screen, Key::Up);
        assert_eq!(
            (screen.selected(), screen.top()),
            (Some(0), 2),
            "no row above"
        );
    }

    #[test]
    fn eight_positions_or_fewer_never_scroll() {
        let mut screen = screen_of(long(1));
        assert_eq!(screen.top(), 0);
        press(&mut screen, Key::Down);
        assert_eq!(screen.top(), 0);
        assert_eq!(names(&screen), ["Good 0"]);
    }

    #[test]
    fn a_click_on_an_enabled_item_that_is_no_row_or_button_does_nothing() {
        let mut template = template();
        for number in [2, 3, 12, 15, 16] {
            template.items[number - 1].enabled = true;
            template.items[number - 1].bounds = rect(10.0 * number as f32, 190.0, 8.0, 8.0);
        }
        let mut screen = TradeScreen::new(
            Ok((template, Rc::new(MonoMetrics))),
            Ok(quantity()),
            long(20),
            ButtonStyle::STOCK,
        );
        for number in [2, 3, 12, 15, 16] {
            click_item(&mut screen, number);
            assert_eq!(screen.selected(), Some(0), "{number}");
        }
        assert!(!screen.closed());
        assert_eq!(screen.take_order(), None);
    }

    #[test]
    fn buy_and_sell_ask_for_a_ton_of_the_selected_good() {
        let mut screen = screen();
        click_item(&mut screen, BUY_ITEM);
        assert_eq!(
            screen.take_order(),
            Some(order(0, FOOD, Direction::Buy, Lot::Click))
        );
        assert_eq!(screen.take_order(), None, "once");
        click_item(&mut screen, SELL_ITEM);
        assert_eq!(
            screen.take_order(),
            Some(order(0, FOOD, Direction::Sell, Lot::Click))
        );
        press(&mut screen, Key::Down);
        press(&mut screen, BUY_KEY);
        assert_eq!(
            screen.take_order(),
            Some(order(1, METAL, Direction::Buy, Lot::Click))
        );
        press(&mut screen, Key::Down);
        press(&mut screen, SELL_KEY);
        assert_eq!(
            screen.take_order(),
            Some(order(2, OPALS, Direction::Sell, Lot::Click))
        );
    }

    #[test]
    fn holding_b_or_s_keeps_trading() {
        let mut screen = screen();
        screen.input(&key(BUY_KEY, true, true));
        assert_eq!(
            screen.take_order(),
            Some(order(0, FOOD, Direction::Buy, Lot::Click))
        );
        screen.input(&key(SELL_KEY, true, true));
        assert_eq!(
            screen.take_order(),
            Some(order(0, FOOD, Direction::Sell, Lot::Click))
        );
        screen.input(&key(BUY_KEY, false, false));
        assert_eq!(screen.take_order(), None, "not on release");
    }

    #[test]
    fn by_the_other_count_reading_with_alt_held_buy_and_sell_ask_for_the_most() {
        let mut screen = screen_of(Market {
            rules: Rulebook::default().with_override(RuleKey::TradeCount, RuleSource::Bible),
            ..market()
        });
        press(&mut screen, MAX_LOT_KEY);
        click_item(&mut screen, BUY_ITEM);
        assert_eq!(
            screen.take_order(),
            Some(order(0, FOOD, Direction::Buy, Lot::Max))
        );
        press(&mut screen, SELL_KEY);
        assert_eq!(
            screen.take_order(),
            Some(order(0, FOOD, Direction::Sell, Lot::Max))
        );
        screen.input(&key(MAX_LOT_KEY, true, true));
        press(&mut screen, BUY_KEY);
        assert_eq!(
            screen.take_order(),
            Some(order(0, FOOD, Direction::Buy, Lot::Max))
        );
        screen.input(&key(MAX_LOT_KEY, false, false));
        press(&mut screen, BUY_KEY);
        assert_eq!(
            screen.take_order(),
            Some(order(0, FOOD, Direction::Buy, Lot::Click))
        );
        press(&mut screen, MAX_LOT_KEY);
        screen.release_keys();
        click_item(&mut screen, SELL_ITEM);
        assert_eq!(
            screen.take_order(),
            Some(order(0, FOOD, Direction::Sell, Lot::Click))
        );
    }

    // The quantity dialog, by the engine's count reading.

    fn quantity_item(screen: &TradeScreen, number: usize) -> Point {
        screen
            .quantity()
            .expect("asking")
            .dialog()
            .item_bounds(number)
            .expect("an item")
            .center()
    }

    fn quantity_texts(screen: &TradeScreen) -> Vec<String> {
        let mut list = DrawList::new();
        screen.quantity().expect("asking").draw(&mut list);
        let commands: Vec<DrawCommand> = list.iter().cloned().collect();
        texts(&commands).into_iter().map(|(text, _)| text).collect()
    }

    fn typed(screen: &mut TradeScreen, text: &str) {
        for c in text.chars() {
            screen.input(&key(Key::Char(c), true, false));
            screen.input(&Input::Text(c));
        }
    }

    /// Opens the quantity dialog on the selected row with Alt and a click
    /// on `button`, Alt then let go.
    fn alt_click(screen: &mut TradeScreen, button: usize) {
        press(screen, MAX_LOT_KEY);
        click_item(screen, button);
        screen.input(&key(MAX_LOT_KEY, false, false));
    }

    #[test]
    fn with_alt_held_buy_opens_the_quantity_dialog_at_the_most_and_ok_trades_the_count() {
        let mut screen = screen();
        alt_click(&mut screen, BUY_ITEM);
        assert_eq!(screen.take_order(), None, "nothing yet");
        let asking = screen.quantity().expect("asking");
        assert_eq!(asking.field().text(), "7", "min(1000 / 93, 7 free)");
        assert_eq!(asking.field().selection(), 1);
        assert_eq!(
            asking.dialog().bounds().min,
            Point::new(10.0, 20.0),
            "laid out by its template"
        );
        assert_eq!(
            quantity_texts(&screen),
            ["Buy", "Enter quantity:", "Cancel", "7"]
        );
        typed(&mut screen, "5");
        let ok = quantity_item(&screen, 1);
        click(&mut screen, ok);
        assert_eq!(
            screen.take_order(),
            Some(order(0, FOOD, Direction::Buy, Lot::Count(5)))
        );
        assert!(screen.quantity().is_none(), "closed");
    }

    #[test]
    fn alt_s_opens_a_sale_dialog_its_s_typing_nothing_and_return_trades_the_most() {
        let mut screen = screen();
        press(&mut screen, MAX_LOT_KEY);
        // As a Mac sends Option-S: the key, then the character it types.
        press(&mut screen, SELL_KEY);
        screen.input(&Input::Text('ß'));
        assert_eq!(
            quantity_texts(&screen),
            ["Sell", "Enter quantity:", "Cancel", "2"]
        );
        screen.input(&Input::Text('ß'));
        assert_eq!(
            screen.quantity().map(|asking| asking.field().text()),
            Some("ß"),
            "only the first typed character is dropped"
        );
        let mut screen = self::screen();
        press(&mut screen, MAX_LOT_KEY);
        typed(&mut screen, "s");
        press(&mut screen, Key::Enter);
        assert_eq!(
            screen.take_order(),
            Some(order(0, FOOD, Direction::Sell, Lot::Count(2)))
        );
        assert!(!screen.closed(), "Return went to the dialog");
    }

    #[test]
    fn a_click_on_buy_keeps_the_first_typed_character() {
        let mut screen = screen();
        alt_click(&mut screen, BUY_ITEM);
        screen.input(&Input::Text('3'));
        assert_eq!(
            screen.quantity().map(|asking| asking.field().text()),
            Some("3")
        );
    }

    #[test]
    fn cancel_or_a_count_of_0_trades_nothing_and_closes_the_dialog() {
        let mut screen = screen();
        alt_click(&mut screen, BUY_ITEM);
        let cancel = quantity_item(&screen, 4);
        click(&mut screen, cancel);
        assert!(screen.quantity().is_none());
        assert_eq!(screen.take_order(), None);
        alt_click(&mut screen, SELL_ITEM);
        typed(&mut screen, "0");
        press(&mut screen, Key::Enter);
        assert!(screen.quantity().is_none());
        assert_eq!(screen.take_order(), None);
        assert!(!screen.closed());
    }

    #[test]
    fn while_the_dialog_is_open_b_and_escape_do_not_reach_the_exchange() {
        let mut screen = screen();
        alt_click(&mut screen, BUY_ITEM);
        press(&mut screen, BUY_KEY);
        press(&mut screen, Key::Down);
        click_item(&mut screen, SELL_ITEM);
        press(&mut screen, Key::Escape);
        assert_eq!(screen.take_order(), None);
        assert!(!screen.closed());
        assert_eq!(screen.selected(), Some(0));
        assert!(screen.quantity().is_some(), "still open");
    }

    #[test]
    fn alt_let_go_in_the_dialog_is_let_go_after_it() {
        let mut screen = screen();
        press(&mut screen, MAX_LOT_KEY);
        click_item(&mut screen, BUY_ITEM);
        screen.input(&key(MAX_LOT_KEY, false, false));
        press(&mut screen, Key::Enter);
        assert_eq!(
            screen.take_order(),
            Some(order(0, FOOD, Direction::Buy, Lot::Count(7)))
        );
        press(&mut screen, BUY_KEY);
        assert_eq!(
            screen.take_order(),
            Some(order(0, FOOD, Direction::Buy, Lot::Click))
        );
        // And Alt pressed in the dialog stays held after it.
        click_item(&mut screen, BUY_ITEM);
        screen.take_order();
        alt_click(&mut screen, BUY_ITEM);
        press(&mut screen, MAX_LOT_KEY);
        press(&mut screen, Key::Enter);
        screen.take_order();
        press(&mut screen, BUY_KEY);
        assert_eq!(screen.take_order(), None);
        assert!(screen.quantity().is_some(), "Alt still held");
    }

    #[test]
    fn letting_go_of_the_keys_in_the_dialog_lets_go_of_alt() {
        let mut screen = screen();
        press(&mut screen, MAX_LOT_KEY);
        click_item(&mut screen, BUY_ITEM);
        screen.release_keys();
        press(&mut screen, Key::Enter);
        screen.take_order();
        press(&mut screen, BUY_KEY);
        assert_eq!(
            screen.take_order(),
            Some(order(0, FOOD, Direction::Buy, Lot::Click))
        );
    }

    #[test]
    fn the_dialogs_beep_comes_out_of_the_exchange_and_its_buttons_are_silent() {
        let mut screen = screen();
        alt_click(&mut screen, BUY_ITEM);
        screen.take_sounds();
        typed(&mut screen, "9");
        press(&mut screen, Key::Enter);
        assert_eq!(screen.take_sounds(), [Sound::Ui(UiSound::Alert)]);
        assert_eq!(
            screen.quantity().map(|asking| asking.field().text()),
            Some("7"),
            "reset to the most"
        );
        let cancel = quantity_item(&screen, 4);
        click(&mut screen, cancel);
        assert_eq!(screen.take_sounds(), []);
    }

    /// Waste at -200 a ton, none held, with `cash` and a ton free.
    fn waste(cash: i64) -> TradeScreen {
        screen_of(Market {
            rows: vec![row(OPALS, "Waste", -200, 0)],
            cash,
            free: 1,
            ..market()
        })
    }

    #[test]
    fn below_a_most_under_nothing_the_dialog_opens_and_only_cancel_leaves() {
        let mut screen = waste(1000);
        press(&mut screen, MAX_LOT_KEY);
        press(&mut screen, BUY_KEY);
        assert_eq!(
            screen.quantity().map(|asking| asking.field().text()),
            Some("-5")
        );
        for _ in 0..3 {
            press(&mut screen, Key::Enter);
        }
        assert!(screen.quantity().is_some());
        let cancel = quantity_item(&screen, 4);
        click(&mut screen, cancel);
        assert!(screen.quantity().is_none());
        assert_eq!(screen.take_order(), None);
        let mut screen = waste(0);
        press(&mut screen, MAX_LOT_KEY);
        press(&mut screen, BUY_KEY);
        press(&mut screen, Key::Enter);
        assert!(screen.quantity().is_none(), "0 of 0 confirmed");
        assert_eq!(screen.take_order(), None);
    }

    #[test]
    fn the_dialog_is_drawn_over_the_exchange() {
        let mut screen = screen();
        let exchange = drawn(&screen);
        alt_click(&mut screen, BUY_ITEM);
        let mut expected = exchange;
        let mut list = DrawList::new();
        screen.quantity().expect("asking").draw(&mut list);
        expected.extend(list.iter().cloned());
        assert_eq!(drawn(&screen), expected);
    }

    #[test]
    fn without_its_template_the_dialog_opens_with_the_fallback() {
        for quantity in [Err("no DLOG 1003".to_owned()), Ok(template())] {
            let mut screen = TradeScreen::new(Ok(layout()), quantity, market(), ButtonStyle::STOCK);
            alt_click(&mut screen, BUY_ITEM);
            let asking = screen.quantity().expect("asking");
            assert_eq!(
                asking.dialog().bounds(),
                rect(426.0, 348.0, 172.0, 72.0),
                "centred"
            );
            assert_eq!(asking.field().text(), "7");
        }
    }

    #[test]
    fn cancelling_the_pointer_abandons_a_click_in_the_dialog() {
        let mut screen = screen();
        alt_click(&mut screen, BUY_ITEM);
        let ok = quantity_item(&screen, 1);
        let button = |pressed| Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at: ok,
        };
        screen.input(&button(true));
        screen.cancel_pointer();
        screen.input(&button(false));
        assert!(screen.quantity().is_some());
        assert_eq!(screen.take_order(), None);
    }

    /// The original tests Option alone (`_DoTradeDialog` @0x5e211,
    /// @0x5e45c): any other modifier is a plain trade.
    #[test]
    fn with_shift_control_or_command_held_buy_and_sell_ask_for_a_plain_trade() {
        for modifier in [Key::Shift, Key::Control, Key::Command] {
            let mut screen = screen();
            screen.input(&key(modifier, true, false));
            press(&mut screen, BUY_KEY);
            assert_eq!(
                screen.take_order(),
                Some(order(0, FOOD, Direction::Buy, Lot::Click)),
                "{modifier:?}"
            );
            click_item(&mut screen, SELL_ITEM);
            assert_eq!(
                screen.take_order(),
                Some(order(0, FOOD, Direction::Sell, Lot::Click)),
                "{modifier:?}"
            );
        }
    }

    /// Whether Buy and Sell draw enabled.
    fn enabled(screen: &TradeScreen) -> (bool, bool) {
        let label_color = |label: &str| {
            drawn(screen).into_iter().find_map(|command| match command {
                DrawCommand::Text { text, color, .. } if text == label => Some(color),
                _ => None,
            })
        };
        let grey = ButtonStyle::STOCK.grey;
        (
            label_color(BUY_LABEL) != Some(grey),
            label_color(SELL_LABEL) != Some(grey),
        )
    }

    #[test]
    fn buy_and_sell_are_greyed_as_the_market_allows() {
        let mut screen = screen();
        assert_eq!(enabled(&screen), (true, true), "food");
        press(&mut screen, Key::Down);
        assert_eq!(enabled(&screen), (true, false), "metal, none held");
        press(&mut screen, Key::Down);
        assert_eq!(enabled(&screen), (false, true), "opals, not sold here");
        click_item(&mut screen, BUY_ITEM);
        press(&mut screen, BUY_KEY);
        assert_eq!(screen.take_order(), None, "greyed asks for nothing");
        press(&mut screen, Key::Up);
        click_item(&mut screen, SELL_ITEM);
        press(&mut screen, SELL_KEY);
        assert_eq!(screen.take_order(), None);
        let broke = Market {
            cash: 10,
            ..market()
        };
        assert_eq!(enabled(&screen_of(broke)), (false, true));
        let empty = screen_of(Market {
            rows: Vec::new(),
            ..market()
        });
        assert_eq!(empty.selected(), None);
        assert_eq!(enabled(&empty), (false, false));
    }

    /// As the original's `_CanBuyGoods` (@0xccec-0xcd31): cash at least
    /// the price, compared signed, and space free, though the buy then
    /// moves nothing.
    #[test]
    fn buy_is_enabled_on_a_negative_price_with_the_cash_for_it_and_space_free() {
        let waste = |cash, free| {
            screen_of(Market {
                rows: vec![row(OPALS, "Waste", -200, 0)],
                cash,
                free,
                ..market()
            })
        };
        for cash in [0, -200, 1000] {
            let mut screen = waste(cash, 1);
            assert_eq!(enabled(&screen), (true, false), "{cash}");
            press(&mut screen, BUY_KEY);
            assert_eq!(
                screen.take_order(),
                Some(order(0, OPALS, Direction::Buy, Lot::Click)),
                "{cash}"
            );
            click_item(&mut screen, BUY_ITEM);
            assert_eq!(
                screen.take_order(),
                Some(order(0, OPALS, Direction::Buy, Lot::Click)),
                "{cash}"
            );
        }
        assert_eq!(enabled(&waste(-500, 1)), (false, false), "cash below it");
        assert_eq!(enabled(&waste(0, 0)), (false, false), "no space");
    }

    #[test]
    fn a_junk_listed_one_way_on_each_row_is_sold_on_its_high_row_and_bought_on_its_low_row() {
        let mut screen = screen_of(Market {
            rows: vec![
                MarketRow {
                    sold_here: false,
                    ..row(OPALS, "Opals", 1500, 1)
                },
                MarketRow {
                    bought_here: false,
                    ..row(OPALS, "Opals", 960, 1)
                },
            ],
            ..market()
        });
        assert_eq!(enabled(&screen), (false, true), "the high row");
        press(&mut screen, BUY_KEY);
        assert_eq!(screen.take_order(), None, "greyed asks for nothing");
        press(&mut screen, SELL_KEY);
        assert_eq!(
            screen.take_order(),
            Some(order(0, OPALS, Direction::Sell, Lot::Click))
        );
        press(&mut screen, Key::Down);
        assert_eq!(enabled(&screen), (true, false), "the low row");
        press(&mut screen, SELL_KEY);
        assert_eq!(screen.take_order(), None);
        press(&mut screen, BUY_KEY);
        assert_eq!(
            screen.take_order(),
            Some(order(1, OPALS, Direction::Buy, Lot::Click))
        );
    }

    #[test]
    fn each_row_of_a_junk_traded_both_ways_names_itself_in_the_order() {
        let mut screen = screen_of(Market {
            rows: vec![row(OPALS, "Opals", 990, 1), row(OPALS, "Opals", 633, 1)],
            ..market()
        });
        for row in [0, 1] {
            assert_eq!(enabled(&screen), (true, true), "{row}");
            press(&mut screen, BUY_KEY);
            assert_eq!(
                screen.take_order(),
                Some(order(row, OPALS, Direction::Buy, Lot::Click))
            );
            press(&mut screen, SELL_KEY);
            assert_eq!(
                screen.take_order(),
                Some(order(row, OPALS, Direction::Sell, Lot::Click))
            );
            press(&mut screen, Key::Down);
        }
    }

    #[test]
    fn a_new_market_shows_and_regreys_keeping_the_selection() {
        let mut screen = screen();
        press(&mut screen, Key::Down);
        let mut bought = market();
        bought.rows[1].held = 7;
        bought.free = 0;
        bought.cash = 0;
        screen.set_market(bought.clone());
        assert_eq!(screen.market(), &bought);
        assert_eq!(screen.selected(), Some(1));
        assert_eq!(enabled(&screen), (false, true));
        let shown: Vec<_> = texts(&drawn(&screen)).into_iter().map(|(t, _)| t).collect();
        assert!(shown.contains(&"7".to_owned()), "{shown:?}");
        let mut scrolled = screen_of(long(11));
        for _ in 0..10 {
            press(&mut scrolled, Key::Down);
        }
        assert_eq!((scrolled.selected(), scrolled.top()), (Some(10), 9));
        scrolled.set_market(long(3));
        assert_eq!(
            (scrolled.selected(), scrolled.top()),
            (Some(2), 2),
            "clamped"
        );
        scrolled.set_market(long(0));
        assert_eq!(scrolled.selected(), None);
        scrolled.set_market(long(5));
        assert_eq!(scrolled.selected(), Some(0));
    }

    #[test]
    fn done_return_and_escape_close_it() {
        let mut screen = screen();
        click_item(&mut screen, DONE_ITEM);
        assert!(screen.closed());
        for k in [Key::Enter, Key::Escape] {
            let mut screen = screen_of(market());
            assert_eq!(screen.input(&key(k, true, false)), ScreenAction::None);
            assert!(screen.closed(), "{k:?}");
            assert_eq!(screen.take_order(), None);
        }
        let mut held = screen_of(market());
        held.input(&key(Key::Escape, true, true));
        held.input(&key(Key::Space, true, false));
        held.tick(Duration::from_secs(1));
        assert!(!held.closed());
    }

    /// `_TrackTradeButtons` @0x294cd plays nothing under the pointer.
    #[test]
    fn pointer_click_and_keys_are_silent_as_the_original() {
        let mut screen = screen();
        let buy = item(&screen, BUY_ITEM).center();
        screen.input(&Input::PointerButton {
            button: MouseButton::Left,
            pressed: true,
            at: buy,
        });
        assert_eq!(screen.take_sounds(), []);
        screen.input(&Input::PointerButton {
            button: MouseButton::Left,
            pressed: false,
            at: buy,
        });
        assert_eq!(screen.take_sounds(), []);
        press(&mut screen, BUY_KEY);
        press(&mut screen, Key::Enter);
        assert_eq!(screen.take_sounds(), []);
    }

    #[test]
    fn cancelling_the_pointer_abandons_a_click() {
        let mut screen = screen();
        let buy = item(&screen, BUY_ITEM).center();
        let button = |pressed| Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at: buy,
        };
        screen.input(&button(true));
        screen.cancel_pointer();
        screen.input(&button(false));
        assert_eq!(screen.take_order(), None);
        assert!(format!("{screen:?}").contains("TextMetrics"));
    }

    fn problem(reason: &str) -> DrawCommand {
        DrawCommand::Text {
            text: format!(
                "Cannot show the trade center: {reason}. Press Return or Escape to close it."
            ),
            font: Font::Geneva,
            origin: PROBLEM_AT,
            size: PROBLEM_SIZE,
            wrap_width: None,
            color: Color::ERROR,
        }
    }

    #[test]
    fn without_the_dialog_it_says_why_and_return_or_escape_closes_it() {
        for k in [Key::Enter, Key::Escape] {
            let mut screen = TradeScreen::new(
                Err("no DLOG 1001".to_owned()),
                Ok(quantity()),
                market(),
                ButtonStyle::STOCK,
            );
            assert_eq!(screen.problem(), Some("no DLOG 1001"));
            assert!(screen.dialog().is_none());
            assert_eq!(drawn(&screen), [problem("no DLOG 1001")]);
            for other in [
                key(k, true, true),
                key(k, false, false),
                key(BUY_KEY, true, false),
                key(MAX_LOT_KEY, true, false),
                key(Key::Down, true, false),
            ] {
                screen.input(&other);
            }
            click(&mut screen, Point::new(500.0, 500.0));
            screen.cancel_pointer();
            assert!(!screen.closed());
            assert_eq!(screen.take_order(), None);
            screen.input(&key(k, true, false));
            assert!(screen.closed(), "{k:?}");
        }
    }

    // The trade center's mission offers.

    use crate::spaceport::offer::OfferChoice;
    use crate::spaceport::offer::fixture::{prompt, templates as offer_templates};

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    /// The trade center, its offers given.
    fn offering() -> TradeScreen {
        screen().with_offers(offer_templates())
    }

    #[test]
    fn the_trade_center_with_offers_asks_for_an_offer_as_it_opens() {
        let mut screen = offering();
        assert!(screen.take_offer_request(), "before any tick");
        assert!(!screen.take_offer_request(), "once");
        screen.tick(ms(10_000));
        assert!(!screen.take_offer_request(), "not again until rearmed");
    }

    #[test]
    fn without_offers_the_trade_center_never_asks() {
        let mut screen = screen();
        assert!(!screen.take_offer_request());
        screen.tick(ms(10_000));
        assert!(!screen.take_offer_request());
        screen.open_offer(prompt());
        assert!(screen.offer_dialog().is_none());
    }

    #[test]
    fn an_offer_opens_over_the_trade_center_and_takes_its_input() {
        let mut screen = offering();
        screen.open_offer(prompt());
        let offer = screen.offer_dialog().expect("open");
        assert_eq!(offer.prompt(), &prompt());
        assert_eq!(offer.problem(), None, "laid out by the template given");
        let mut expected = drawn(&self::screen());
        let mut over = DrawList::new();
        offer.draw(&mut over);
        expected.extend(over.iter().cloned());
        assert_eq!(drawn(&screen), expected, "drawn over the trade center");
        press(&mut screen, BUY_KEY);
        assert_eq!(screen.take_order(), None, "B went to the offer");
        assert_eq!(screen.take_offer_choice(), None);
        press(&mut screen, Key::Enter);
        assert!(screen.offer_dialog().is_none(), "answered, it closes");
        assert!(!screen.closed(), "Return went to the offer");
        assert_eq!(
            screen.take_offer_choice(),
            Some((MissionId(141), OfferChoice::Accept))
        );
        assert_eq!(screen.take_offer_choice(), None, "once");
    }

    #[test]
    fn the_trade_centers_offer_pointer_goes_through_it() {
        let mut screen = offering();
        screen.open_offer(prompt());
        let at = screen
            .offer_dialog()
            .and_then(OfferDialog::dialog)
            .and_then(|dialog| dialog.item_bounds(crate::spaceport::offer::REFUSE_ITEM))
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
        assert_eq!(screen.take_sounds(), [], "silent under the pointer");
        screen.input(&button(true));
        screen.input(&button(false));
        assert_eq!(
            screen.take_offer_choice(),
            Some((MissionId(141), OfferChoice::Refuse))
        );
    }

    #[test]
    fn after_a_rearm_the_trade_center_asks_again_after_the_gap() {
        let mut screen = offering();
        assert!(screen.take_offer_request());
        screen.open_offer(prompt());
        press(&mut screen, Key::Enter);
        assert!(screen.take_offer_choice().is_some());
        screen.rearm(ms(600));
        screen.tick(ms(599));
        assert!(!screen.take_offer_request());
        screen.tick(ms(1));
        assert!(screen.take_offer_request());
    }

    #[test]
    fn by_the_other_reading_the_trade_center_waits_out_its_quantity_dialog() {
        let mut screen = offering().with_offer_clock(RuleSource::Bible);
        assert!(screen.take_offer_request());
        screen.rearm(ms(600));
        alt_click(&mut screen, BUY_ITEM);
        assert!(screen.quantity().is_some());
        screen.tick(ms(10_000));
        assert!(
            !screen.take_offer_request(),
            "paused under the quantity dialog"
        );
        let cancel = quantity_item(&screen, 4);
        click(&mut screen, cancel);
        assert!(screen.quantity().is_none());
        screen.tick(ms(599));
        assert!(!screen.take_offer_request());
        screen.tick(ms(1));
        assert!(
            screen.take_offer_request(),
            "the wait goes on once it closes"
        );
    }

    #[test]
    fn by_the_engine_an_offer_due_under_the_quantity_dialog_is_asked_for_as_it_closes() {
        let mut screen = offering();
        assert!(screen.take_offer_request());
        screen.rearm(ms(600));
        alt_click(&mut screen, BUY_ITEM);
        screen.tick(ms(10_000));
        assert!(!screen.take_offer_request(), "not while it is open");
        let cancel = quantity_item(&screen, 4);
        click(&mut screen, cancel);
        assert!(screen.quantity().is_none());
        screen.tick(ms(0));
        assert!(screen.take_offer_request(), "at once once it closes");
    }

    #[test]
    fn the_trade_centers_gap_runs_from_the_ask_by_the_engine_and_the_answer_by_the_other() {
        for (clock, wait) in [(RuleSource::Engine, 200), (RuleSource::Bible, 600)] {
            let mut screen = offering().with_offer_clock(clock);
            assert!(screen.take_offer_request());
            screen.open_offer(prompt());
            screen.tick(ms(400));
            press(&mut screen, Key::Enter);
            assert!(screen.take_offer_choice().is_some());
            screen.rearm(ms(600));
            screen.tick(ms(wait - 1));
            assert!(!screen.take_offer_request(), "{clock:?}");
            screen.tick(ms(1));
            assert!(screen.take_offer_request(), "{clock:?}");
        }
    }

    #[test]
    fn covered_the_trade_center_runs_only_its_offer_clock() {
        for (clock, asked) in [(RuleSource::Engine, true), (RuleSource::Bible, false)] {
            let mut screen = offering().with_offer_clock(clock);
            assert!(screen.take_offer_request());
            screen.rearm(ms(600));
            screen.tick_covered(ms(10_000));
            assert!(!screen.take_offer_request(), "{clock:?}: covered");
            screen.tick(ms(0));
            assert_eq!(screen.take_offer_request(), asked, "{clock:?}");
        }
    }

    #[test]
    fn the_mission_info_key_asks_for_the_mission_info_screen() {
        let i = crate::ui::mission_info::MISSION_INFO_KEY;
        let mut screen = screen();
        assert!(!screen.take_info_request());
        screen.input(&key(i, true, true));
        assert!(!screen.take_info_request(), "not a repeat");
        press(&mut screen, i);
        assert!(screen.take_info_request());
        assert!(!screen.take_info_request(), "once");
        assert!(!screen.closed());
        alt_click(&mut screen, BUY_ITEM);
        press(&mut screen, i);
        assert!(!screen.take_info_request(), "the quantity dialog takes it");
        assert!(screen.quantity().is_some());
        let mut broken = TradeScreen::new(
            Err("no DLOG 1001".to_owned()),
            Ok(quantity()),
            market(),
            ButtonStyle::STOCK,
        );
        press(&mut broken, i);
        assert!(!broken.take_info_request(), "no dialog");
    }

    #[test]
    fn the_mission_info_key_asks_from_its_offer_which_stays_open() {
        let mut screen = offering();
        screen.open_offer(prompt());
        press(&mut screen, crate::ui::mission_info::MISSION_INFO_KEY);
        assert!(screen.take_info_request());
        assert!(screen.offer_dialog().is_some());
    }

    /// The original's `_TradeFilter` @0x4e092 maps Tab to `_TradeDialogTabKey`, Up/Down to the selection and Return/Escape to item 1 (@0x4e0dc-0x4e1cf), none with a click sound.
    #[test]
    fn keyboard_activation_is_silent_as_the_original() {
        for k in [
            Key::Tab,
            Key::Down,
            Key::Up,
            BUY_KEY,
            SELL_KEY,
            crate::ui::player_info::PLAYER_INFO_KEY,
            Key::Enter,
            Key::Escape,
        ] {
            let mut screen = screen();
            for pressed in [true, false] {
                screen.input(&Input::Key {
                    key: k,
                    pressed,
                    repeat: false,
                });
            }
            assert_eq!(screen.take_sounds(), [], "{k:?} makes no click");
        }
    }
}
