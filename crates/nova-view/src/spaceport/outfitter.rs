//! The Outfitter: the stellar's outfits for sale, laid out by the
//! interface file's "Outfit" dialog (`DLOG`/`DITL` 1002) over its
//! background picture.
//!
//! The stock dialog is 765 x 321, centred, and drawn over `PICT` 8502,
//! whose painted frames match its items: the grid of outfits (item 5) on
//! the left, the selected outfit's description (item 6) beside it, its
//! picture (item 8) and the info box (item 9) on the right, and along the
//! bottom the grid's scroll arrows (items 10 and 11), Buy (7), Sell (4) and
//! Done (1). The stock template disables Sell, as the original enables it
//! as it greys it, so the screen enables Done, Buy, Sell and the arrows
//! itself.
//!
//! The grid shows each outfit listed ([`Outfitter`]) in a cell,
//! [`COLUMNS`] across and [`GRID_ROWS`] down: its picture (the same `PICT`
//! as the detail pane's, shrunk to 32 x 32, or a black square without
//! one) near the cell's top; its `ShortName` centred at the bottom, split
//! into lines on a literal `\n`, each line white when it starts with a
//! letter or digit and grey otherwise (the Bible); and how many the player
//! has, when any, in the top-right corner ([`outfit_picture`],
//! [`grid`](super::grid)).
//! Every listed cell is framed in the `cölr` `GridDim` colour. One cell is
//! selected, by a click on it or with the arrow keys, and highlighted:
//! framed again, last, in `GridBright`, not filled
//! ([`GridColors`](super::grid::GridColors)); the grid scrolls a row at a
//! time with the arrows below it,
//! or with the selection. The arrow keys, and Page Up and Page Down as Up
//! and Down, move the selection by
//! [`RuleKey::GridArrows`](nova_sim::RuleKey::GridArrows), as the
//! shipyard's do: by the engine, Up on the first row and Down onto an
//! empty cell stay (`_OutfitFilter` @0x58203-0x583b8). With nothing listed, the grid says so (`STR#`
//! 2002 #206).
//!
//! The selected outfit's picture is `PICT` 6000 plus its ID less 128, or
//! "No Picture" and "Available" (#213, #214) centred on the picture item in
//! white when the data has none, and its description is
//! `dësc` 3000 plus its ID less 128, both read through the
//! [`OutfitterCatalog`] as the selection changes. The info box shows, in
//! white at the original's baselines (`_OutfitDialogUpdate`
//! @0x57b36-0x57e7b), "Item Price:" with its price and "You Have:" with
//! the player's cash, each followed by " cr" (#215, #217, #34), then, for
//! an outfit of `Mass` above 0, "Item Mass:" and "Available:" (#216,
//! #218). Each label is at the box's left and each amount
//! [`AMOUNT_COLUMN`](super::credits::AMOUNT_COLUMN) right of it, drawn as
//! `_DrawCommaNumber` draws it ([`credits`](super::credits)). Below them,
//! at a fixed row, is why it cannot be bought (#219-222), when the
//! original has words for it. The count owned is only in the grid's
//! corner, as in the original. The buy words are
//! the outfit's own ([`Outfitter::words`]), not its buy refusal: the
//! original works them out without asking whether the outfit can be
//! bought, so an outfit not for sale, or a fighter refused for full bays,
//! still gets them. Every refusal `_HasMaxOfItem` makes (its `Max`, none
//! allowed, a gun or turret limit) reads by the count owned, #219 when the
//! player owns one or more of the outfit and #220 otherwise, as
//! `_OutfitDialogUpdate` words it (@0x57ddf-0x57df8); otherwise an outfit
//! that lacks the mass gets #221 or #222 (@0x57e33-0x57e4f).
//!
//! A sale refused for the free mass (#207), for its ammunition (a
//! launcher, the #208-212 sentence, [`ammunition_first`]) or for the `Max`
//! it raises (`ModType` 27, the same sentence naming the outfit to sell
//! first, [`raised_first`]) is shown as
//! [`RuleKey::SaleRefusal`](nova_sim::RuleKey::SaleRefusal) says. By the
//! engine, as the original does, Sell stays enabled (the Sell flag alone
//! enables it) and the info box never words it; a click refused in the
//! sell loop shows the words in "Desc Dialog" (`DLOG` 3003,
//! `_ShowTextDialog` @0x5a8cf) over the outfitter ([`OutfitterScreen::alert`],
//! [`sale_alert_text`]), naming the sold outfit in the plural while passes
//! of a counted sale remain. It takes every input while it is open, and
//! Done, Return or Escape closes it, the outfitter staying open. By the
//! other reading, the port's earlier behaviour, Sell is greyed and the
//! words are an info-box line ([`Outfitter::sale_words`]).
//!
//! Buy and Sell, clicked or with B and S (key repeats too, so holding a
//! key keeps going, but only on the outfit the key went down on: when an
//! order removes its cell, a held key stops until it is pressed again),
//! ask for one of the selected outfit, and each is
//! greyed when its button is not enabled ([`Outfitter::can`]). With Option held (Alt
//! here, [`COUNT_KEY`]), Buy or Sell asks for a count instead, as
//! `_DoOutfitDialog` does (@0x5c0c0-0x5c1aa for a buy, @0x5c5e3-0x5c628
//! for a sale): when the outfitter says it asks
//! ([`Outfitter::count_asked`], by the engine's reading of
//! [`RuleKey::OutfitCount`](nova_sim::RuleKey::OutfitCount) the maximum
//! `count_max` works out, none at 1 or less), the quantity dialog
//! ([`QuantityDialog`], `DLOG` 1003, or its fallback; `_DoQuantityDialog`
//! @0x56d5b) opens over the outfitter at that count, OK titled "Buy" or
//! "Sell" and asking "Enter quantity:". It takes every input while it is
//! open; the count confirmed there is the order, 0 or Cancel an order of
//! none, which moves nothing but still runs what the original runs after
//! its loop once the button's check passed (@0x5c2fc-0x5c2ff to the buy's
//! reset and `_DrawStatus`, the sell's likewise). B or S opening it types
//! nothing into it. Otherwise, by
//! the other reading too, a click moves one. Shift, Command and Control
//! change nothing: the original tests Option alone (`testb $0x8` on the
//! event's modifiers). The screen only records the order and its count
//! ([`OutfitterScreen::take_order`]): whoever holds the session makes it
//! and gives back the outfitter as it now is
//! ([`OutfitterScreen::set_outfitter`]).
//!
//! After an order that changes the listing, the screen follows what the
//! order left the selection at ([`OutfitterScreen::reselect`]), as
//! [`RuleKey::RelistedSelection`](nova_sim::RuleKey::RelistedSelection)
//! says. By the engine, the selection is reset as `_DoOutfitDialog` resets
//! it (@0x5c32c-0x5c35b, @0x5d021-0x5d04b): no cell is highlighted, the
//! grid scrolls to its top, and Buy and Sell answer for `oütf` 128,
//! listed or not ([`OutfitterScreen::selected_outfit`]), while the
//! picture, the description and the info box are blank
//! (`_OutfitDialogUpdate` @0x5789b-0x578a5). A click on a listed cell, or
//! an arrow, ends the reset: Right and Down pick the first cell shown, and
//! Left and Up the last cell shown, or the last listed when fewer are
//! (`_OutfitFilter` @0x583bd-0x58440). When the order leaves nothing
//! listed, the outfitter closes as Done closes it, once any alert the
//! order showed is closed (-0x63d, @0x5da33).
//!
//! The grid's presses and arrows follow
//! [`RuleKey::GridSelection`](nova_sim::RuleKey::GridSelection). By the
//! engine, the selection is the selected outfit, as `_OutfitFilter` keeps
//! it (`*0xded58`). A press on an empty cell position within the grid's
//! cells selects nothing (@0x58632-0x58648): no cell is highlighted, Buy
//! and Sell are greyed, the picture, the info box and the description are
//! blank, and the press reaches no button; after a reset, that lets the
//! reset's outfit go. An arrow then picks a cell as after a reset. An
//! arrow that leaves the selected outfit as it was reads no description
//! (@0x58442-0x5844f), while the picture and the info box follow the
//! highlighted cell (@0x57949-0x579d6): an arrow onto `oütf` 128's cell
//! after a reset shows them over a blank description. By the other
//! reading, the port's earlier behaviour, a press on an empty cell is
//! left to the dialog, keeping the selection or the reset, and an arrow
//! reads the description of an outfit not already shown. Either way, a
//! press on the highlighted cell changes nothing.
//!
//! The opening and the scroll arrows follow the same key. By the engine,
//! the outfitter opens with nothing selected, as `_DoOutfitDialog` does
//! (@0x5bb19-0x5bb54, the description emptied @0x5bd0f-0x5bd1c): no cell
//! is highlighted, Buy and Sell are greyed, and the picture, the info box
//! and the description are blank until a click or an arrow picks a cell.
//! A scroll arrow that scrolls keeps the highlighted cell while it stays
//! in view, reading its description again (@0x5d384-0x5d3b4,
//! @0x5d70c-0x5d737), and otherwise selects nothing, ending a reset
//! (@0x5d3bb-0x5d3d6, @0x5d73e-0x5d759); one that cannot scroll changes
//! nothing. By the other reading, the port's earlier behaviour, it opens
//! on the first cell, and a scroll keeps the selection, off screen too.
//!
//! Given the offer dialog's template ([`OutfitterScreen::with_offers`]),
//! the outfitter offers missions as `_DoOutfitDialog` and `_OutfitFilter`
//! do: it asks for an offer as it opens
//! ([`OutfitterScreen::take_offer_request`], `_DoOutfitDialog`
//! @0x5bd92), and whoever holds the session answers with one, opening the
//! [`OfferDialog`] over it ([`OutfitterScreen::open_offer`]), or rearms it
//! ([`OutfitterScreen::rearm`]) with the gap drawn, after which it asks
//! again (`_OutfitFilter` @0x589ce-0x589ff). The offer takes every input
//! while it is open, and its choice is taken through the screen
//! ([`OutfitterScreen::take_offer_choice`]). The quantity dialog and a
//! sale's alert cover the outfitter, as the offer does: nothing is asked
//! while any is open, and the wait runs on or stops as
//! [`RuleKey::OfferClock`](nova_sim::RuleKey::OfferClock) reads it
//! ([`OutfitterScreen::with_offer_clock`]).
//!
//! Done, Return and Escape close it. Without the dialog, the screen says
//! why, and Return or Escape still closes it.

use std::collections::BTreeMap;
use std::rc::Rc;
use std::time::Duration;

use nova_sim::{
    Direction, LcNames, MissionId, OfferPrompt, OutfitId, OutfitOrder, OutfitRefusal, OutfitRow,
    Outfitter, Reselection, RuleSource, SaleAlert,
};

use super::catalog::SpaceportCatalog;
use super::credits::{self, comma_tons_text, credits_text, draw_amount_rows};
use super::grid::{self, CellGrid, GridColors, GridLooks, Scrolled, Shown, text};
use super::grid_arrows::ArrowMoves;
use super::layout::DONE_LABEL;
use super::modal::ModalHost;
use super::offer::{OfferChoice, OfferDialog, OfferTemplates, VenueOffers};
use super::quantity_ask::QuantityAsk;
use super::trade::{BUY_LABEL, MAX_LOT_KEY, SELL_LABEL};
use super::view::{PROBLEM_AT, PROBLEM_SIZE};
use crate::color::Color;
use crate::draw::DrawList;
use crate::flight::escorts::NUMBER_WORDS;
use crate::flight::view::is_map_press;
use crate::geometry::{Bounds, Point};
use crate::image::ImageKey;
use crate::input::{Input, Key};
use crate::movie::Movies;
use crate::screen::{Screen, ScreenAction};
use crate::sound::Sound;
use crate::text::TextMetrics;
use crate::ui::button::{ButtonSkin, ButtonStyle};
use crate::ui::catalog::DescriptionSource;
use crate::ui::comm::grouped;
use crate::ui::desc::DescDialog;
use crate::ui::dialog::{Dialog, DialogEvent, DialogTemplate, FocusRing, Role};
use crate::ui::mission_info::is_mission_info_press;
use crate::ui::quantity::QuantityDialog;

/// The "Outfit" dialog's `DLOG` (and `DITL`) ID.
pub const OUTFIT_DIALOG: i16 = 1002;

/// The picture the dialog is drawn over: "Outfit", in Nova Graphics.
pub const BACKGROUND: ImageKey = ImageKey::picture(8502);

/// Done's item.
pub const DONE_ITEM: usize = 1;
/// Sell's item.
pub const SELL_ITEM: usize = 4;
/// The grid's item.
pub const GRID_ITEM: usize = 5;
/// The description's item.
pub const DESCRIPTION_ITEM: usize = 6;
/// Buy's item.
pub const BUY_ITEM: usize = 7;
/// The picture's item.
pub const PICTURE_ITEM: usize = 8;
/// The info box's item.
pub const INFO_ITEM: usize = 9;
/// The grid's scroll-up arrow's item.
pub const SCROLL_UP_ITEM: usize = 10;
/// The grid's scroll-down arrow's item.
pub const SCROLL_DOWN_ITEM: usize = 11;

pub use super::grid::{
    CELL_HEIGHT, CELL_WIDTH, COLUMNS, GREY, GRID_ROWS, INSET, NO_PICTURE, TEXT_COLOR, TEXT_FONT,
    TEXT_SIZE, name_lines,
};

/// Buys one, as Buy does: the original's default.
pub const BUY_KEY: Key = Key::Char('b');
/// Sells one, as Sell does: the original's default.
pub const SELL_KEY: Key = Key::Char('s');
/// Held, makes a Buy or Sell ask for a count in the quantity dialog, as
/// the original's Option does (`_DoOutfitDialog` tests `optionKey` alone,
/// `testb $0x8`), unless the other reading of
/// [`RuleKey::OutfitCount`](nova_sim::RuleKey::OutfitCount) has it change
/// nothing. The exchange's key ([`MAX_LOT_KEY`]).
pub const COUNT_KEY: Key = MAX_LOT_KEY;

/// The first outfit's `PICT`; outfit n's is n - 128 after it.
pub const FIRST_PICTURE: i16 = 6000;
/// The first outfit's `dësc`; outfit n's is n - 128 after it.
pub const FIRST_DESCRIPTION: i16 = 3000;
/// The first `oütf` ID.
const FIRST_OUTFIT: i16 = 128;

/// `STR#` 2002 #206.
pub const NO_ITEMS: &str = "There are no items available for purchase here.";
/// `STR#` 2002 #207.
pub const NEGATIVE_FREE_MASS: &str =
    "Can't sell that item, because your ship would have negative free mass afterwards.";
/// `STR#` 2002 #215.
pub const PRICE_LABEL: &str = "Item Price:";
/// `STR#` 2002 #216.
pub const MASS_LABEL: &str = "Item Mass:";
/// `STR#` 2002 #218.
pub const AVAILABLE_LABEL: &str = "Available:";
/// The info box's rows' baselines below its top: Item Price, You Have,
/// Item Mass and Available (`_OutfitDialogUpdate` @0x57b42, @0x57bef,
/// @0x57c7e, @0x57d1e).
pub const INFO_ROWS: [f32; 4] = [12.0, 24.0, 48.0, 60.0];
/// The refusal's baseline below the info box's top, mass lines or not
/// (@0x57da2-0x57db5).
pub const REFUSAL_ROW: f32 = 93.0;

/// The info box's amounts for `row`, in the order `_OutfitDialogUpdate`
/// draws them, each as its baseline below the box's top, its label and
/// its amount: "Item Price:" with what buying one costs, its
/// [`buy_price`](OutfitRow::buy_price) (@0x57b42-0x57bea), and "You
/// Have:" with the player's cash (@0x57bef-0x57c62), each in credits,
/// then, for an outfit of `Mass` above 0 (@0x57c72-0x57c78), "Item
/// Mass:" (@0x57c7e-0x57d19) and "Available:" (@0x57d1e-0x57d9d).
#[must_use]
pub fn amount_lines(outfitter: &Outfitter, row: &OutfitRow) -> Vec<(f32, &'static str, String)> {
    let mut lines = vec![
        (INFO_ROWS[0], PRICE_LABEL, credits_text(row.buy_price)),
        (
            INFO_ROWS[1],
            credits::YOU_HAVE,
            credits_text(outfitter.cash),
        ),
    ];
    if let Some(mass) = outfitter.item_mass_shown(row.id) {
        lines.push((INFO_ROWS[2], MASS_LABEL, comma_tons_text(mass)));
        lines.push((
            INFO_ROWS[3],
            AVAILABLE_LABEL,
            comma_tons_text(outfitter.free_mass_shown()),
        ));
    }
    lines
}
/// `STR#` 2002 #219.
pub const MAX_OWNED: &str = "Can't have any more!";
/// `STR#` 2002 #220.
pub const NONE_ALLOWED: &str = "Can't have any of this item!";
/// `STR#` 2002 #221.
pub const NO_SPACE: &str = "Can't hold any more!";
/// `STR#` 2002 #222.
pub const NO_SPACE_FOR_ANY: &str = "Can't hold any of this item!";

/// `STR#` 2002 #208.
pub const SELL_FIRST: &str = "You need to sell";
/// `STR#` 2002 #209.
pub const UNIT: &str = "unit";
/// `STR#` 2002 #210.
pub const UNITS: &str = "units";
/// `STR#` 2002 #211.
pub const OF_AMMUNITION: &str = "of ammunition";
/// `STR#` 2002 #212.
pub const BEFORE_SELLING: &str = "before you can sell your";

/// The original's words for selling `launcher` (its `LCName`) before
/// `rounds` of its ammunition, named by `ammo`'s `LCName` or `LCPlural`
/// (`_DoOutfitDialog` @0x5cbf3-0x5ce2c): #208, the rounds in words for 1-3
/// (`STR#` 137) and in comma-grouped digits above, the ammunition, or
/// #209/#210 and #211 without an outfit naming it, #212, the launcher and
/// a full stop.
#[must_use]
pub fn ammunition_first(rounds: u32, ammo: Option<&LcNames>, launcher: &str) -> String {
    let number = match rounds {
        1..=3 => NUMBER_WORDS[rounds as usize - 1].to_owned(),
        _ => grouped(i64::from(rounds)),
    };
    let plural = rounds > 1;
    let ammunition = match ammo {
        Some(names) if plural => names.plural.clone(),
        Some(names) => names.singular.clone(),
        None => format!("{} {OF_AMMUNITION}", if plural { UNITS } else { UNIT }),
    };
    format!("{SELL_FIRST} {number} {ammunition} {BEFORE_SELLING} {launcher}.")
}

/// The original's words for selling `outfit` (its `LCName`) before `count`
/// of the outfit whose `Max` it raises, named by `target`'s `LCName` or
/// `LCPlural` (`_DoOutfitDialog` @0x5c874-0x5ca11): the launcher's
/// sentence ([`ammunition_first`]), #208, the count, the target, #212,
/// the outfit and a full stop.
#[must_use]
pub fn raised_first(count: u32, target: &LcNames, outfit: &str) -> String {
    ammunition_first(count, Some(target), outfit)
}

/// The original's words for why one of `row` cannot be bought or sold, if
/// it has any, `names` giving the outfits' lower-case names. Every refusal
/// the original's `_HasMaxOfItem` makes (its `Max`, none allowed, a gun or
/// turret limit) is worded by the count owned: #219 when the player owns
/// one or more of the outfit, #220 otherwise (`_OutfitDialogUpdate`
/// @0x57ddf-0x57df8). Full fighter bays get no words of their own.
#[must_use]
pub fn refusal_text(
    refusal: OutfitRefusal,
    row: &OutfitRow,
    names: &BTreeMap<OutfitId, LcNames>,
) -> Option<String> {
    let text = match refusal {
        OutfitRefusal::MaxOwned
        | OutfitRefusal::NoneAllowed
        | OutfitRefusal::GunLimit
        | OutfitRefusal::TurretLimit
            if row.owned > 0 =>
        {
            MAX_OWNED
        }
        OutfitRefusal::MaxOwned
        | OutfitRefusal::NoneAllowed
        | OutfitRefusal::GunLimit
        | OutfitRefusal::TurretLimit => NONE_ALLOWED,
        OutfitRefusal::NoSpace => NO_SPACE,
        OutfitRefusal::NoSpaceForAny => NO_SPACE_FOR_ANY,
        OutfitRefusal::NegativeFreeMass
        | OutfitRefusal::AmmunitionFirst { .. }
        | OutfitRefusal::RaisedFirst { .. } => {
            let sold = names.get(&row.id).map_or("", |names| &names.singular);
            return sale_sentence(refusal, sold, names);
        }
        _ => return None,
    };
    Some(text.to_owned())
}

/// The original's words for a sale refused inside the sell loop, as
/// `DLOG` 3003 shows them (`_DoOutfitDialog` @0x5c735-0x5ce6c): #207 alone
/// for the free mass; the #208-#212 sentence for a raised `Max`
/// ([`raised_first`]) or a launcher's ammunition ([`ammunition_first`]),
/// the sold outfit named by its `LCPlural` when passes remain
/// ([`SaleAlert::plural`]) and its `LCName` otherwise. No alert carries
/// any other refusal, which gives no words.
#[must_use]
pub fn sale_alert_text(alert: &SaleAlert) -> String {
    let sold = alert.names.get(&alert.outfit).map_or("", |names| {
        if alert.plural {
            &names.plural
        } else {
            &names.singular
        }
    });
    sale_sentence(alert.refusal, sold, &alert.names).unwrap_or_default()
}

/// The words for a sale refusal of the sell loop's, naming the sold
/// outfit `sold` and the others by `names`: #207, or the #208-#212
/// sentence. None for any other refusal.
fn sale_sentence(
    refusal: OutfitRefusal,
    sold: &str,
    names: &BTreeMap<OutfitId, LcNames>,
) -> Option<String> {
    match refusal {
        OutfitRefusal::NegativeFreeMass => Some(NEGATIVE_FREE_MASS.to_owned()),
        OutfitRefusal::AmmunitionFirst { rounds, ammo } => {
            let ammo = ammo.and_then(|ammo| names.get(&ammo));
            Some(ammunition_first(rounds, ammo, sold))
        }
        OutfitRefusal::RaisedFirst { count, target } => {
            let target = names.get(&target).cloned().unwrap_or_default();
            Some(raised_first(count, &target, sold))
        }
        _ => None,
    }
}

/// Outfit `outfit`'s `PICT`, 6000 plus its ID less 128, if `exists` says
/// it is there: the picture both its grid cell and the detail pane show
/// (`_OutfitDialogUpdate` @0x57441-0x57462 and @0x57949-0x579a5).
#[must_use]
pub fn outfit_picture(outfit: OutfitId, exists: impl Fn(i16) -> bool) -> Option<i16> {
    let picture = FIRST_PICTURE.saturating_add(outfit.0.saturating_sub(FIRST_OUTFIT));
    exists(picture).then_some(picture)
}

/// Outfit `outfit`'s `dësc`, 3000 plus its ID less 128
/// (`_OutfitFilter` @0x58465, @0x588cc).
#[must_use]
pub fn outfit_desc_id(outfit: OutfitId) -> i16 {
    FIRST_DESCRIPTION.saturating_add(outfit.0.saturating_sub(FIRST_OUTFIT))
}

/// What the outfitter reads about an outfit: its description, through
/// [`DescriptionSource`], and whether its picture exists, through
/// [`SpaceportCatalog::picture_exists`], and the grid's colours, through
/// [`GridLooks`]. Anything that is all three is one.
pub trait OutfitterCatalog: SpaceportCatalog + DescriptionSource + GridLooks {}

impl<T: SpaceportCatalog + DescriptionSource + GridLooks + ?Sized> OutfitterCatalog for T {}

/// The catalog, shared, with a `Debug` that shows nothing of it.
#[derive(Clone)]
struct CatalogHandle(Rc<dyn OutfitterCatalog>);

impl std::fmt::Debug for CatalogHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("OutfitterCatalog")
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

/// The outfitter, laid out.
#[derive(Clone, Debug)]
struct Laid {
    dialog: Dialog,
    metrics: MetricsHandle,
    /// The outfit whose picture and description are shown, and them.
    shown: Option<(OutfitId, Shown)>,
}

/// What the screen's own keys asked for, until each is taken.
#[derive(Clone, Copy, Debug, Default)]
struct KeyAsks {
    /// Whether [`MISSION_INFO_KEY`](crate::ui::mission_info::MISSION_INFO_KEY)
    /// asked for the mission info screen.
    mission_info: bool,
    /// Whether the map key asked for the course map.
    map: bool,
}

/// The Outfitter of the stellar landed on.
#[derive(Clone, Debug)]
pub struct OutfitterScreen {
    /// The dialog, or why it cannot be shown.
    laid: Result<Laid, String>,
    outfitter: Outfitter,
    catalog: CatalogHandle,
    /// The grid's frame colours, read from the catalog once.
    colors: GridColors,
    /// Each listed outfit's picture, by row, read once a list (none
    /// without the dialog).
    pictures: Vec<Option<i16>>,
    /// The selected cell, whether one is highlighted (`*0xded5c`), and
    /// the grid's scrolling.
    grid: CellGrid,
    /// The reading of [`RuleKey::GridArrows`](nova_sim::RuleKey::GridArrows)
    /// the grid's arrows follow, read from the outfitter's rules once.
    arrow_moves: ArrowMoves,
    /// The order asked for, and how many it moves, until it is taken.
    order: Option<(OutfitOrder, u32)>,
    /// The key B or S last went down on (a press, not a repeat), and the
    /// outfit selected then, if any: its repeats act on that outfit
    /// alone.
    held: Option<(Key, Option<OutfitId>)>,
    /// The quantity dialog, while it is open, the order it counts, and
    /// whether Alt is held.
    quantity: QuantityAsk<OutfitOrder>,
    /// The alert a sale refused in the sell loop shows, while it is open.
    alert: ModalHost<(), DescDialog>,
    /// The alert's template (`DLOG` 3003), if the interface file has one.
    alert_template: Option<DialogTemplate>,
    /// The mission offers, once given.
    offering: Option<VenueOffers>,
    style: ButtonStyle,
    /// Whether its dialogs' buttons take the keyboard focus.
    focus_ring: FocusRing,
    closed: bool,
    /// What its keys asked for, until each is taken.
    asks: KeyAsks,
    /// The outfit Buy and Sell answer for while no cell is highlighted
    /// (`*0xded58`): `Some` is the outfit an order that changed the
    /// listing reset the selection to, listed or not
    /// ([`OutfitterScreen::reselect`]); `None` is nothing selected, after
    /// an empty cell clicked by the engine's reading of
    /// [`RuleKey::GridSelection`](nova_sim::RuleKey::GridSelection).
    /// Unread while a cell is highlighted.
    unhighlighted_outfit: Option<OutfitId>,
    /// Whether an order left nothing listed, so the outfitter closes
    /// once its alert is closed.
    leaving: bool,
    /// The `dësc` shown since this was last taken, to be loaded.
    description_request: Option<i16>,
    sounds: Vec<Sound>,
}

impl OutfitterScreen {
    /// The outfitter `outfitter`, laid out by `layout`: the "Outfit"
    /// dialog's template and the metrics its text is measured by, or why
    /// there are none. The quantity dialog is laid out by `quantity`, its
    /// template (or, without one, its fallback). A sale refused in the
    /// sell loop shows its words in "Desc Dialog", laid out by `alert`,
    /// its template (`DLOG` 3003), and none without one. Its buttons are
    /// labelled in `style`, and each outfit's picture and description are
    /// read from `catalog`. It opens as
    /// [`RuleKey::GridSelection`](nova_sim::RuleKey::GridSelection) reads
    /// it: by the engine with nothing selected, Buy and Sell greyed and
    /// no picture, info box or description, as `_DoOutfitDialog` sets
    /// `*0xded58` and `*0xded5c` to -1, the scroll to 0 and clears Sell
    /// and Buy (@0x5bb19-0x5bb54) and empties the description
    /// (@0x5bd0f-0x5bd1c); by the other, with the first cell selected.
    #[must_use]
    pub fn new(
        layout: Result<(DialogTemplate, Rc<dyn TextMetrics>), String>,
        quantity: Result<DialogTemplate, String>,
        alert: Result<DialogTemplate, String>,
        outfitter: Outfitter,
        catalog: Rc<dyn OutfitterCatalog>,
        style: ButtonStyle,
    ) -> Self {
        let laid = layout.map(|(mut template, metrics)| {
            for item in [
                DONE_ITEM,
                SELL_ITEM,
                BUY_ITEM,
                SCROLL_UP_ITEM,
                SCROLL_DOWN_ITEM,
            ] {
                if let Some(found) = template.items.get_mut(item - 1) {
                    found.enabled = true;
                }
            }
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
                shown: None,
            }
        });
        let arrow_moves = ArrowMoves::from_rules(&outfitter.rules);
        let mut screen = Self {
            laid,
            outfitter,
            colors: catalog.grid_colors(),
            catalog: CatalogHandle(catalog),
            pictures: Vec::new(),
            grid: CellGrid::default(),
            arrow_moves,
            order: None,
            held: None,
            quantity: QuantityAsk::new(quantity.ok(), style),
            alert: ModalHost::default(),
            alert_template: alert.ok(),
            offering: None,
            style,
            focus_ring: FocusRing::default(),
            closed: false,
            asks: KeyAsks::default(),
            unhighlighted_outfit: None,
            leaving: false,
            description_request: None,
            sounds: Vec::new(),
        };
        screen.read_pictures();
        if screen.follows_outfit_filter() {
            screen.grid.clear();
            screen.selected_changed();
        } else {
            screen.select(0);
        }
        screen
    }

    /// The outfitter with its dialogs' buttons taking the keyboard focus
    /// as `ring` says (the `dialog_focus_ring` rule), its own and each it
    /// opens after it.
    #[must_use]
    pub fn with_focus_ring(mut self, ring: FocusRing) -> Self {
        self.focus_ring = ring;
        if let Ok(laid) = &mut self.laid {
            laid.dialog.set_focus_ring(ring);
        }
        self.offering = self.offering.map(|offering| offering.with_focus_ring(ring));
        self
    }

    /// The outfitter with mission offers, each shown in the offer dialog of
    /// its variant laid out by `templates` (or saying why there is none): it asks for the
    /// first as it opens, and again once rearmed ([`OutfitterScreen::rearm`]), as
    /// the original's does ([`OUTFITTER`](nova_sim::mission::screens::OUTFITTER)).
    #[must_use]
    pub fn with_offers(self, templates: OfferTemplates) -> Self {
        Self {
            offering: Some(VenueOffers::asking(templates).with_focus_ring(self.focus_ring)),
            ..self
        }
    }

    /// Its offers' timer running as `clock` reads
    /// [`RuleKey::OfferClock`](nova_sim::RuleKey::OfferClock), for offers
    /// given ([`OutfitterScreen::with_offers`]) before it.
    #[must_use]
    pub fn with_offer_clock(self, clock: RuleSource) -> Self {
        Self {
            offering: self.offering.map(|offering| offering.with_clock(clock)),
            ..self
        }
    }

    /// Whether its offer has asked for the player info screen since this
    /// was last asked: the outfitter itself ignores
    /// [`PLAYER_INFO_KEY`](crate::ui::player_info::PLAYER_INFO_KEY), as `_OutfitFilter` never reads it.
    pub fn take_player_info_request(&mut self) -> bool {
        self.offering
            .as_mut()
            .is_some_and(VenueOffers::take_player_info_request)
    }

    /// The `dësc` of the outfit the outfitter has shown since this was
    /// last taken, once, read raw until whoever flies the ship loads it
    /// as `_OutfitFilter` does (@0x58476, @0x588dd; `_DoOutfitDialog`
    /// @0x5d3b4, @0x5d737) and answers with
    /// [`OutfitterScreen::show_description`], in the same input.
    pub fn take_description_request(&mut self) -> Option<i16> {
        self.description_request.take()
    }

    /// Shows `text`, `dësc` `id` loaded, as the description of the outfit
    /// shown, while it is the outfit whose `dësc` that is.
    pub fn show_description(&mut self, id: i16, text: &str) {
        let Ok(Laid {
            dialog,
            metrics,
            shown: Some((outfit, shown)),
        }) = &mut self.laid
        else {
            return;
        };
        if outfit_desc_id(*outfit) == id {
            let area = dialog.item_bounds(DESCRIPTION_ITEM).unwrap_or(Bounds::at(
                Point::new(0.0, 0.0),
                0.0,
                0.0,
            ));
            shown.retext(text, area, &metrics.0);
        }
    }

    /// Whether the map key (`_keyPrefs` +0x12, a press) has asked for the
    /// course map since this was last asked, once (`_OutfitFilter` item 2, @0x580f3-0x58111): never in the
    /// dialogs open over the outfitter or its offer, and never without its
    /// dialog. Whoever holds the session opens it over the spaceport.
    pub fn take_map_request(&mut self) -> bool {
        std::mem::take(&mut self.asks.map)
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
        std::mem::take(&mut self.asks.mission_info) || offer
    }

    /// Whether the outfitter has asked for an offer since this was last
    /// asked: whoever holds the session answers with
    /// [`OutfitterScreen::open_offer`], or, with none, rearms it
    /// ([`OutfitterScreen::rearm`]).
    pub fn take_offer_request(&mut self) -> bool {
        self.offering
            .as_mut()
            .is_some_and(VenueOffers::take_request)
    }

    /// Opens the offer dialog over the outfitter on `prompt`.
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
    /// the outfitter.
    pub(crate) fn tick_covered(&mut self, dt: Duration) {
        if let Some(offering) = &mut self.offering {
            offering.tick(dt, true);
        }
    }

    /// The outfitter shown.
    #[must_use]
    pub fn outfitter(&self) -> &Outfitter {
        &self.outfitter
    }

    /// The selected cell's index, if anything is listed and a cell is
    /// highlighted: none after an order that reset the selection, or by
    /// the engine after a click on an empty cell, until a cell is chosen
    /// ([`OutfitterScreen::reselect`]).
    #[must_use]
    pub fn selected(&self) -> Option<usize> {
        self.grid.selected(self.outfitter.rows.len())
    }

    /// The outfit Buy and Sell answer for (`*0xded58`, which
    /// `_OutfitFilter` reads, @0x5806f-0x580ca): the highlighted cell's;
    /// with none highlighted after a reset, the outfit the reset
    /// selected, listed or not; and none after a click on an empty cell
    /// by the engine's reading of
    /// [`RuleKey::GridSelection`](nova_sim::RuleKey::GridSelection)
    /// (@0x58632-0x58648).
    #[must_use]
    pub fn selected_outfit(&self) -> Option<OutfitId> {
        if self.grid.highlighted() {
            self.selected().map(|index| self.outfitter.rows[index].id)
        } else {
            self.unhighlighted_outfit
        }
    }

    /// The grid's first row shown.
    #[must_use]
    pub fn top_row(&self) -> usize {
        self.grid.top_row()
    }

    /// The dialog, if it could be laid out.
    #[must_use]
    pub fn dialog(&self) -> Option<&Dialog> {
        self.laid.as_ref().ok().map(|laid| &laid.dialog)
    }

    /// Why the outfitter cannot be shown, if it cannot.
    #[must_use]
    pub fn problem(&self) -> Option<&str> {
        self.laid.as_ref().err().map(String::as_str)
    }

    /// Whether Done has been activated, or an order left nothing listed
    /// and no alert is open: the original shows a sale's alert inside its
    /// loop, then leaves (-0x63d, @0x5da33).
    #[must_use]
    pub fn closed(&self) -> bool {
        self.closed || (self.leaving && !self.alert.is_open())
    }

    /// Follows what an order left the selection at, as
    /// [`RuleKey::RelistedSelection`](nova_sim::RuleKey::RelistedSelection)
    /// reads it: [`Reselection::Kept`] changes nothing;
    /// [`Reselection::Reset`] highlights no cell, scrolls the grid to its
    /// top and has Buy and Sell answer for its outfit, listed or not, with
    /// no picture, description or info box (`*0xded5c` = -1, `*0xdeda8`
    /// = 0, @0x5c32c-0x5c35b, @0x5d021-0x5d04b; `_OutfitDialogUpdate`
    /// @0x5789b-0x578a5); and [`Reselection::Leave`] closes the outfitter
    /// once its alert is closed ([`closed`](Self::closed)).
    pub fn reselect(&mut self, after: Reselection) {
        match after {
            Reselection::Kept => {}
            Reselection::Reset(outfit) => {
                self.unhighlighted_outfit = Some(outfit);
                self.grid = CellGrid::default();
                self.grid.clear();
                self.selected_changed();
            }
            Reselection::Leave => self.leaving = true,
        }
    }

    /// The order asked for since it was last taken, and how many it
    /// moves (1 for a plain click, 0 for the quantity dialog's Cancel or
    /// a count of 0), once.
    pub fn take_order(&mut self) -> Option<(OutfitOrder, u32)> {
        self.order.take()
    }

    /// The quantity dialog, while it is open.
    #[must_use]
    pub fn quantity(&self) -> Option<&QuantityDialog> {
        self.quantity.dialog()
    }

    /// Opens `alert`'s words ([`sale_alert_text`]) in "Desc Dialog" over
    /// the outfitter, as `_DoOutfitDialog` shows a sale refused in the
    /// sell loop (`_ShowTextDialog` @0x5a8cf). Without the outfitter's
    /// dialog, or without the alert's template, it opens nothing:
    /// "Desc Dialog" has no fallback.
    pub fn alert(&mut self, alert: &SaleAlert) {
        let (Ok(laid), Some(template)) = (&self.laid, &self.alert_template) else {
            return;
        };
        let dialog = DescDialog::new(
            template,
            &sale_alert_text(alert),
            self.style,
            Rc::clone(&laid.metrics.0),
        )
        .with_focus_ring(self.focus_ring);
        self.alert.open((), dialog);
    }

    /// The alert, while it is open.
    #[must_use]
    pub fn alert_dialog(&self) -> Option<&DescDialog> {
        self.alert.dialog()
    }

    /// Where cell `index` (counted from the grid's first row shown) is,
    /// if the grid could be laid out.
    #[must_use]
    pub fn cell_bounds(&self, index: usize) -> Option<Bounds> {
        let grid = self.dialog()?.item_bounds(GRID_ITEM)?;
        Some(grid::cell_bounds(grid, index))
    }

    /// Shows `outfitter`, the outfitter after an order, keeping the same
    /// outfit selected when it is still listed, or the cell where it was.
    /// With no cell highlighted, after a reset or an empty cell clicked,
    /// it keeps that and the grid's scroll.
    pub fn set_outfitter(&mut self, outfitter: Outfitter) {
        if !self.grid.highlighted() {
            self.outfitter = outfitter;
            self.read_pictures();
            self.selected_changed();
            return;
        }
        let kept = self
            .selected()
            .map(|index| self.outfitter.rows[index].id)
            .and_then(|id| outfitter.rows.iter().position(|row| row.id == id));
        self.outfitter = outfitter;
        self.read_pictures();
        self.select(kept.unwrap_or(self.grid.selected_raw()));
    }

    /// Reads each listed outfit's picture, when there is a dialog to show
    /// them in.
    fn read_pictures(&mut self) {
        if self.laid.is_err() {
            return;
        }
        let catalog = &self.catalog.0;
        self.pictures = self
            .outfitter
            .rows
            .iter()
            .map(|row| outfit_picture(row.id, |id| catalog.picture_exists(id)))
            .collect();
    }

    /// Selects cell `index`, or the last when there is no such cell,
    /// scrolls the grid to show it, and reads its picture and description
    /// if they are not shown already.
    fn select(&mut self, index: usize) {
        self.grid.select(index, self.outfitter.rows.len());
        self.selected_changed();
    }

    /// Regreys and shows the selected outfit.
    fn selected_changed(&mut self) {
        self.regrey();
        self.show(true);
    }

    /// Whether the grid's presses and arrows follow `_OutfitFilter`, by
    /// [`RuleKey::GridSelection`](nova_sim::RuleKey::GridSelection): an empty cell clicked selects nothing,
    /// and an arrow that leaves the selected outfit as it was reads no
    /// description.
    fn follows_outfit_filter(&self) -> bool {
        self.outfitter
            .rules
            .source_for(nova_sim::RuleKey::GridSelection)
            == nova_sim::RuleSource::Engine
    }

    /// Shows the highlighted cell's picture and description, unless they
    /// are shown already; with no cell highlighted, shows none. The
    /// description is read when `read` says so, and otherwise left blank,
    /// as the original's emptied buffer (`_OutfitFilter` skipping
    /// `_LoadDescription`, @0x58442-0x5844f).
    fn show(&mut self, read: bool) {
        let selected = self.selected().map(|index| {
            let picture = self.pictures.get(index).copied().flatten();
            (self.outfitter.rows[index].id, picture)
        });
        let catalog = Rc::clone(&self.catalog.0);
        let Ok(laid) = &mut self.laid else {
            return;
        };
        let Some((outfit, picture)) = selected else {
            laid.shown = None;
            return;
        };
        if laid
            .shown
            .as_ref()
            .is_some_and(|(shown, _)| *shown == outfit)
        {
            return;
        }
        let description = outfit_desc_id(outfit);
        let area = laid
            .dialog
            .item_bounds(DESCRIPTION_ITEM)
            .unwrap_or(Bounds::at(Point::new(0.0, 0.0), 0.0, 0.0));
        let description = if read {
            self.description_request = Some(description);
            catalog.description(description).unwrap_or_default()
        } else {
            String::new()
        };
        laid.shown = Some((
            outfit,
            Shown::read_text(&description, picture, area, &laid.metrics.0),
        ));
    }

    /// Greys Buy and Sell when one of the selected outfit could not be
    /// bought or sold.
    fn regrey(&mut self) {
        let buys = self.allows(Direction::Buy);
        let sells = self.allows(Direction::Sell);
        if let Ok(laid) = &mut self.laid {
            laid.dialog.set_greyed(BUY_ITEM, !buys);
            laid.dialog.set_greyed(SELL_ITEM, !sells);
        }
    }

    /// Whether one of the selected outfit ([`selected_outfit`](Self::selected_outfit))
    /// can go `direction` ([`Outfitter::can`]).
    fn allows(&self, direction: Direction) -> bool {
        self.selected_outfit().is_some_and(|outfit| {
            self.outfitter
                .can(OutfitOrder { outfit, direction })
                .is_ok()
        })
    }

    /// Asks to buy or sell the selected outfit, if it can go that way:
    /// one, or with Alt held, when the outfitter asks for a count
    /// ([`Outfitter::count_asked`]), the quantity dialog at that count.
    /// `by_key` says B or S asked, whose character the dialog then drops.
    fn ask(&mut self, direction: Direction, by_key: bool) {
        let Some(outfit) = self.selected_outfit().filter(|_| self.allows(direction)) else {
            return;
        };
        let order = OutfitOrder { outfit, direction };
        let asked = self
            .outfitter
            .count_asked(order.outfit, direction)
            .filter(|_| self.quantity.alt_held());
        match asked {
            Some(max) => {
                if let Ok(laid) = &self.laid {
                    let metrics = Rc::clone(&laid.metrics.0);
                    self.quantity
                        .open(order, order.direction, i64::from(max), by_key, metrics);
                }
            }
            None => self.order = Some((order, 1)),
        }
    }

    /// Asks to go `direction` for `key` going down: a press asks for the
    /// selected outfit and remembers it; a repeat asks only while that
    /// same outfit is still selected and the press was this key's. So a
    /// key held while an order removes the outfit's cell never acts on
    /// the outfit that takes the cell.
    fn ask_by_key(&mut self, key: Key, input: &Input, direction: Direction) {
        let selected = self.selected_outfit();
        let repeat = matches!(*input, Input::Key { repeat: true, .. });
        if !repeat {
            self.held = Some((key, selected));
        } else if self.held != Some((key, selected)) {
            return;
        }
        self.ask(direction, true);
    }

    /// Moves the selection for an arrow `key` going down, and says
    /// whether it was an arrow, as [`CellGrid::arrow`] moves it by the reading
    /// of [`RuleKey::GridArrows`](nova_sim::RuleKey::GridArrows): by the
    /// engine, Up on the first row and Down onto an empty cell stay, and
    /// Page Up and Page Down move as Up and Down. With no cell highlighted,
    /// after a reset or an empty cell clicked, an arrow highlights one:
    /// Right and Down pick the first cell shown, and Left and Up the last
    /// cell shown, slot 19 of the 20 (`andl $0x13` @0x583e5), or the last
    /// listed when fewer are (`_OutfitFilter` @0x583bd-0x58440).
    fn arrow(&mut self, key: Key) -> bool {
        self.grid
            .arrow(key, self.outfitter.rows.len(), self.arrow_moves)
    }

    /// Scrolls the grid a row down, or up, never past either end, with
    /// the selection as
    /// [`RuleKey::GridSelection`](nova_sim::RuleKey::GridSelection) reads
    /// it. By the engine, as `_DoOutfitDialog`'s scroll arrows do (item 10
    /// @0x5d351-0x5d3d6, item 11 @0x5d6b0-0x5d759), a scroll that moves
    /// keeps the highlighted cell while it stays in view and reads its
    /// description again (@0x5d384-0x5d3b4, @0x5d70c-0x5d737), and
    /// otherwise selects nothing, letting go of a reset's outfit
    /// (@0x5d3bb-0x5d3d6, @0x5d73e-0x5d759); one that cannot move changes
    /// nothing. By the other reading, the selection is kept, off screen
    /// too, and nothing is read.
    ///
    /// The arrows are reached only by a click released over them
    /// (`_TrackOutfitButtons` @0x2a1b9-0x2a2ce): one scroll a click, none
    /// while held, and no key. The original leaves Buy and Sell to the
    /// next event's `_OutfitFilter` prelude (@0x58061-0x580ca), which comes
    /// before the update event draws them; the screen regreys them here
    /// at once, which no frame can tell apart.
    fn scroll(&mut self, down: bool) {
        let count = self.outfitter.rows.len();
        if !self.follows_outfit_filter() {
            self.grid.scroll(down, count);
            return;
        }
        match self.grid.scroll_dropping_hidden(down, count) {
            Scrolled::Stayed => {}
            Scrolled::Kept => self.reread(),
            Scrolled::Dropped => {
                self.unhighlighted_outfit = None;
                self.selected_changed();
            }
        }
    }

    /// Regreys and shows the selected outfit, reading its description
    /// again even when it is already shown.
    fn reread(&mut self) {
        if let Ok(laid) = &mut self.laid {
            laid.shown = None;
        }
        self.selected_changed();
    }

    /// Activates dialog item `item`: Done closes, Buy and Sell ask, and the
    /// arrows scroll. Any other item does nothing.
    fn activate(&mut self, item: usize) {
        match item {
            DONE_ITEM => self.closed = true,
            BUY_ITEM => self.ask(Direction::Buy, false),
            SELL_ITEM => self.ask(Direction::Sell, false),
            SCROLL_UP_ITEM => self.scroll(false),
            SCROLL_DOWN_ITEM => self.scroll(true),
            _ => {}
        }
    }

    /// The cell position under `at` on the grid, listed or not
    /// ([`CellGrid::cell_under`]).
    fn cell_under(&self, at: Point) -> Option<usize> {
        let grid = self.dialog()?.item_bounds(GRID_ITEM)?;
        self.grid.cell_under(grid, at)
    }

    /// Follows `input` when it is a press on one of the grid's cell
    /// positions, and says whether it took it.
    fn press_grid(&mut self, input: &Input) -> bool {
        let Input::PointerButton {
            pressed: true, at, ..
        } = *input
        else {
            return false;
        };
        self.cell_under(at)
            .is_some_and(|index| self.press_cell(index))
    }

    /// Follows a press on the cell position `index` and says whether it
    /// took the press. A listed cell is highlighted and selected. An
    /// empty one, by the engine's reading of [`RuleKey::GridSelection`](nova_sim::RuleKey::GridSelection),
    /// highlights and selects nothing (`_OutfitFilter` @0x58632-0x58648),
    /// and by the other is left to the dialog.
    fn press_cell(&mut self, index: usize) -> bool {
        if index < self.outfitter.rows.len() {
            self.select(index);
        } else if self.follows_outfit_filter() {
            self.unhighlighted_outfit = None;
            self.grid.clear();
            self.selected_changed();
        } else {
            return false;
        }
        true
    }

    /// The shown cells, each its picture, owned count and name, then its
    /// frame in `GridDim`; then, last, the selected cell framed again in
    /// `GridBright` over its neighbours' shared lines. No cell is filled
    /// (`_OutfitDialogUpdate` @0x573ef-0x57895, @0x578a1-0x578c7).
    fn draw_grid(&self, laid: &Laid, list: &mut DrawList) {
        let metrics = &*laid.metrics.0;
        let Some(grid) = laid.dialog.item_bounds(GRID_ITEM) else {
            return;
        };
        if self.outfitter.rows.is_empty() {
            let origin = Point::new(grid.min.x + INSET, grid.min.y + INSET);
            text(list, NO_ITEMS, origin, TEXT_COLOR);
            return;
        }
        let count = self.outfitter.rows.len();
        let selected = self.selected();
        self.grid.draw_cells(
            list,
            grid,
            count,
            selected,
            self.colors,
            |list, index, cell| {
                let row = &self.outfitter.rows[index];
                grid::draw_cell_picture(list, cell, self.pictures[index]);
                if row.owned > 0 {
                    grid::draw_corner_text(list, metrics, cell, &row.owned.to_string(), true);
                }
                grid::draw_cell_name(list, metrics, cell, &row.short_name);
            },
        );
    }

    fn draw_selected(&self, laid: &Laid, list: &mut DrawList) {
        let Some(index) = self.selected() else {
            return;
        };
        let row = &self.outfitter.rows[index];
        if let (Some((_, shown)), Some(area)) = (&laid.shown, laid.dialog.item_bounds(PICTURE_ITEM))
        {
            shown.draw(list, area, &*laid.metrics.0);
        }
        let Some(info) = laid.dialog.item_bounds(INFO_ITEM) else {
            return;
        };
        draw_amount_rows(list, info, amount_lines(&self.outfitter, row));
        let line_height = laid.metrics.0.line_height(TEXT_FONT, TEXT_SIZE);
        let sale = OutfitOrder {
            outfit: row.id,
            direction: Direction::Sell,
        };
        let refusals = [
            self.outfitter.words(row.id),
            self.outfitter.sale_words(row.id),
        ];
        let names = self.outfitter.lc_names(sale);
        let words = refusals
            .into_iter()
            .flatten()
            .filter_map(|refusal| refusal_text(refusal, row, &names));
        let top = grid::baseline_top(info.min.y + REFUSAL_ROW);
        for (n, line) in words.enumerate() {
            let origin = Point::new(info.min.x, top + line_height * n as f32);
            text(list, &line, origin, TEXT_COLOR);
        }
    }

    /// The scroll arrows: a triangle in each item, white when the grid can
    /// scroll that way, grey otherwise.
    fn draw_arrows(&self, laid: &Laid, list: &mut DrawList) {
        grid::draw_arrows(
            list,
            laid.dialog.item_bounds(SCROLL_UP_ITEM),
            laid.dialog.item_bounds(SCROLL_DOWN_ITEM),
            self.grid.can_scroll(self.outfitter.rows.len()),
        );
    }
}

impl Screen for OutfitterScreen {
    /// Every input goes to the outfitter, or to the alert while it is
    /// open (Alt still followed), or else to the quantity dialog while it
    /// is open; it never quits.
    fn input(&mut self, input: &Input) -> ScreenAction {
        if let Some(offering) = &mut self.offering
            && offering.input(input, &mut self.sounds)
        {
            return ScreenAction::None;
        }
        if self.alert.is_open() {
            self.quantity.follow_alt(input);
            self.alert.input(input, &mut self.sounds);
            return ScreenAction::None;
        }
        if self.quantity.is_open() {
            if let Some(counted) = self.quantity.answer(input, &mut self.sounds) {
                self.order = Some(counted);
            }
            return ScreenAction::None;
        }
        self.quantity.follow_alt(input);
        if self.laid.is_err() {
            if let Input::Key {
                key: Key::Enter | Key::Escape,
                pressed: true,
                repeat: false,
            } = *input
            {
                self.closed = true;
            }
            return ScreenAction::None;
        }
        if is_map_press(input) {
            self.asks.map = true;
            return ScreenAction::None;
        }
        if is_mission_info_press(input) {
            self.asks.mission_info = true;
            return ScreenAction::None;
        }
        let pressed =
            |wanted: Key| matches!(*input, Input::Key { key, pressed: true, .. } if key == wanted);
        let before = self.selected_outfit();
        let arrow = match *input {
            Input::Key {
                key, pressed: true, ..
            } => self.arrow(key),
            _ => false,
        };
        if arrow {
            if self.follows_outfit_filter() && self.selected_outfit() == before {
                self.regrey();
                self.show(false);
            } else {
                self.selected_changed();
            }
        } else if pressed(BUY_KEY) {
            self.ask_by_key(BUY_KEY, input, Direction::Buy);
        } else if pressed(SELL_KEY) {
            self.ask_by_key(SELL_KEY, input, Direction::Sell);
        } else if !self.press_grid(input)
            && let Ok(laid) = &mut self.laid
        {
            let event = laid.dialog.input(input);
            if let Some(DialogEvent::Item(item)) = event {
                self.activate(item);
            }
        }
        ScreenAction::None
    }

    /// The wait for the next offer runs down, the quantity dialog or the
    /// alert covering the outfitter while either is open.
    fn tick(&mut self, dt: Duration) {
        let covered = self.quantity.is_open() || self.alert.is_open();
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
                        "Cannot show the outfitter: {reason}. Press Return or Escape to close it."
                    ),
                    PROBLEM_AT,
                    PROBLEM_SIZE,
                    None,
                    Color::ERROR,
                );
                return;
            }
        };
        list.picture(BACKGROUND, laid.dialog.bounds().min);
        self.draw_grid(laid, list);
        self.draw_selected(laid, list);
        self.draw_arrows(laid, list);
        laid.dialog.draw(list);
        self.quantity.draw(list);
        self.alert.draw(list);
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
        if !self.alert.cancel_pointer()
            && !self.quantity.cancel_pointer()
            && let Ok(laid) = &mut self.laid
        {
            laid.dialog.cancel_pointer();
        }
    }

    /// Lets go of Alt, and of the keys held in the alert and the quantity
    /// dialog.
    fn release_keys(&mut self) {
        self.alert.release_keys();
        self.quantity.release_keys();
    }

    /// The quantity dialog's and the alert's sounds, in order. The
    /// buttons are silent under the pointer, as `_TrackOutfitButtons`
    /// @0x2a029 plays nothing.
    fn take_sounds(&mut self) -> Vec<Sound> {
        std::mem::take(&mut self.sounds)
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::cell::RefCell;

    use super::*;
    use nova_sim::{
        Launcher, OutfitFacts, Raise, Reselection, RuleKey, RuleSource, Rulebook, SaleAlert,
    };

    use crate::draw::{DrawCommand, fill_rect, frame_rect};
    use crate::font::Font;
    use crate::input::MouseButton;
    use crate::sound::UiSound;
    use crate::spaceport::catalog::{PortRecord, StellarId};
    use crate::text::fixture::MonoMetrics;
    use crate::ui::dialog::{ItemSpec, ItemTemplate, Placement};

    fn rect(x: f32, y: f32, w: f32, h: f32) -> Bounds {
        Bounds::at(Point::new(x, y), w, h)
    }

    /// Stock "Outfit": `DLOG` 1002, 765 x 321 and centred, and its thirteen
    /// user items, where 2, 3, 4, 6, 8, 9, 12 and 13 are disabled and 2,
    /// 12 and 13 parked outside it.
    fn template() -> DialogTemplate {
        let items = [
            (500.0, 289.0, 99.0, 25.0, true),
            (248.0, 440.0, 68.0, 30.0, false),
            (251.0, 291.0, 29.0, 21.0, false),
            (394.0, 289.0, 99.0, 25.0, false),
            (9.0, 8.0, 333.0, 271.0, true),
            (354.0, 10.0, 192.0, 267.0, false),
            (288.0, 289.0, 99.0, 25.0, true),
            (557.0, 8.0, 200.0, 200.0, false),
            (618.0, 214.0, 135.0, 100.0, false),
            (148.0, 288.0, 25.0, 25.0, true),
            (178.0, 288.0, 25.0, 25.0, true),
            (170.0, 449.0, 32.0, 32.0, false),
            (356.0, 422.0, 68.0, 30.0, false),
        ];
        DialogTemplate {
            bounds: rect(100.0, 100.0, 765.0, 321.0),
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

    /// Where the dialog goes: (1024 - 765) / 2 and (768 - 321) / 2,
    /// floored.
    const ORIGIN: Point = Point::new(129.0, 223.0);

    /// Outfit 128's picture exists; every description is "dësc n.", and
    /// 3002's is missing. Records what it is asked.
    struct FakeArt {
        pictures: Vec<i16>,
        asked: RefCell<Vec<String>>,
    }

    impl SpaceportCatalog for FakeArt {
        fn stellar_port(&self, id: StellarId) -> Result<PortRecord, String> {
            panic!("asked for spöb {}", id.0)
        }

        fn picture_exists(&self, id: i16) -> bool {
            self.asked.borrow_mut().push(format!("PICT {id}"));
            self.pictures.contains(&id)
        }
    }

    impl GridLooks for FakeArt {
        fn grid_colors(&self) -> GridColors {
            grid::TEST_GRID_COLORS
        }
    }

    impl DescriptionSource for FakeArt {
        fn description(&self, id: i16) -> Result<String, String> {
            self.asked.borrow_mut().push(format!("dësc {id}"));
            if id == 3002 {
                return Err("no dësc 3002".to_owned());
            }
            Ok(format!("dësc {id}."))
        }

        fn button_style(&self) -> ButtonStyle {
            panic!("the style is given")
        }
    }

    fn art() -> Rc<FakeArt> {
        Rc::new(FakeArt {
            pictures: vec![6000],
            asked: RefCell::default(),
        })
    }

    fn row(id: i16, short_name: &str, price: i64, mass: i64, owned: u16) -> OutfitRow {
        OutfitRow {
            id: OutfitId(id),
            name: format!("Outfit {id}"),
            short_name: short_name.to_owned(),
            price,
            buy_price: price,
            mass,
            owned,
            max: 10,
            facts: OutfitFacts {
                listed: true,
                buy_tech: true,
                required: true,
                available: true,
                sold_here: true,
                mass_field: i16::try_from(mass).expect("a 16-bit mass"),
                ..OutfitFacts::default()
            },
        }
    }

    /// A map (1 owned) that can only be sold here.
    fn map() -> OutfitRow {
        let mut map = row(131, "Map", 0, 0, 1);
        map.facts.buy_tech = false;
        map
    }

    /// A light blaster (2 owned), ammunition, a gun too heavy to hold, and
    /// a map that can only be sold here (1 owned), for a player with 5000
    /// credits and 5 tons free.
    fn outfitter() -> Outfitter {
        Outfitter {
            rows: vec![
                row(128, "Light\\nBlaster", 1000, 1, 2),
                row(129, "*Ammo\\nPack", 50, 0, 0),
                row(130, "Big Gun", 9000, 10, 0),
                map(),
            ],
            cash: 5000,
            free_mass: 5,
            rules: Rulebook::default().with_override(RuleKey::OutfitCount, RuleSource::Engine),
            ..Outfitter::default()
        }
    }

    fn layout() -> (DialogTemplate, Rc<dyn TextMetrics>) {
        (template(), Rc::new(MonoMetrics))
    }

    /// The outfitter `outfitter` as it opens, reading from `art`.
    fn opened_with(outfitter: Outfitter, art: &Rc<FakeArt>) -> OutfitterScreen {
        let catalog: Rc<dyn OutfitterCatalog> = Rc::clone(art) as Rc<dyn OutfitterCatalog>;
        OutfitterScreen::new(
            Ok(layout()),
            Ok(quantity()),
            Ok(desc_template()),
            outfitter,
            catalog,
            ButtonStyle::STOCK,
        )
    }

    /// The outfitter `outfitter` with the first cell picked, as a click on
    /// it would, reading from `art`.
    fn screen_with(outfitter: Outfitter, art: &Rc<FakeArt>) -> OutfitterScreen {
        let mut screen = opened_with(outfitter, art);
        screen.select(0);
        screen
    }

    /// The outfitter `outfitter` as it opens.
    fn opened(outfitter: Outfitter) -> OutfitterScreen {
        opened_with(outfitter, &art())
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

    fn screen_of(outfitter: Outfitter) -> OutfitterScreen {
        screen_with(outfitter, &art())
    }

    fn screen() -> OutfitterScreen {
        screen_of(outfitter())
    }

    fn drawn(screen: &OutfitterScreen) -> Vec<DrawCommand> {
        let mut list = DrawList::new();
        screen.draw(&mut list);
        list.iter().cloned().collect()
    }

    fn texts(commands: &[DrawCommand]) -> Vec<(String, Point, Color)> {
        commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text {
                    text,
                    origin,
                    color,
                    ..
                } => Some((text.clone(), *origin, *color)),
                _ => None,
            })
            .collect()
    }

    fn words(screen: &OutfitterScreen) -> Vec<String> {
        texts(&drawn(screen))
            .into_iter()
            .map(|(text, _, _)| text)
            .collect()
    }

    fn item(screen: &OutfitterScreen, number: usize) -> Bounds {
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

    fn press(screen: &mut OutfitterScreen, k: Key) {
        screen.input(&key(k, true, false));
    }

    fn click(screen: &mut OutfitterScreen, at: Point) {
        for pressed in [true, false] {
            screen.input(&Input::PointerButton {
                button: MouseButton::Left,
                pressed,
                at,
            });
        }
    }

    fn click_item(screen: &mut OutfitterScreen, number: usize) {
        let at = item(screen, number).center();
        click(screen, at);
    }

    fn click_cell(screen: &mut OutfitterScreen, index: usize) {
        let at = screen.cell_bounds(index).expect("a cell").center();
        click(screen, at);
    }

    /// A plain click's order: one of outfit `id`, `direction`.
    fn order(id: i16, direction: Direction) -> (OutfitOrder, u32) {
        counted(id, direction, 1)
    }

    /// An order for `count` of outfit `id`, `direction`.
    fn counted(id: i16, direction: Direction, count: u32) -> (OutfitOrder, u32) {
        (
            OutfitOrder {
                outfit: OutfitId(id),
                direction,
            },
            count,
        )
    }

    /// Whether Buy and Sell draw enabled.
    fn enabled(screen: &OutfitterScreen) -> (bool, bool) {
        let label_color = |label: &str| {
            texts(&drawn(screen))
                .into_iter()
                .find_map(|(text, _, color)| (text == label).then_some(color))
        };
        let grey = ButtonStyle::STOCK.grey;
        (
            label_color(BUY_LABEL) != Some(grey),
            label_color(SELL_LABEL) != Some(grey),
        )
    }

    #[test]
    fn the_named_values() {
        assert_eq!(OUTFIT_DIALOG, 1002);
        assert_eq!(BACKGROUND, ImageKey::picture(8502));
        assert_eq!(
            [
                DONE_ITEM,
                SELL_ITEM,
                GRID_ITEM,
                DESCRIPTION_ITEM,
                BUY_ITEM,
                PICTURE_ITEM,
                INFO_ITEM,
                SCROLL_UP_ITEM,
                SCROLL_DOWN_ITEM
            ],
            [1, 4, 5, 6, 7, 8, 9, 10, 11]
        );
        assert_eq!((COLUMNS, GRID_ROWS), (4, 5));
        assert_eq!((CELL_WIDTH, CELL_HEIGHT), (84.0, 55.0));
        assert_eq!(grid::CELL_PITCH_X * COLUMNS as f32 + 1.0, 333.0);
        assert_eq!(grid::CELL_PITCH_Y * GRID_ROWS as f32 + 1.0, 271.0);
        assert_eq!((BUY_KEY, SELL_KEY), (Key::Char('b'), Key::Char('s')));
        assert_eq!((FIRST_PICTURE, FIRST_DESCRIPTION), (6000, 3000));
        assert_eq!(
            [PRICE_LABEL, MASS_LABEL, AVAILABLE_LABEL, NO_PICTURE],
            ["Item Price:", "Item Mass:", "Available:", "No Picture"]
        );
        assert_eq!((INFO_ROWS, REFUSAL_ROW), ([12.0, 24.0, 48.0, 60.0], 93.0));
    }

    /// The words for `refusal` of a row with `owned` owned, no names
    /// known.
    fn worded(refusal: OutfitRefusal, owned: u16) -> Option<String> {
        refusal_text(refusal, &row(128, "Gun", 1, 1, owned), &BTreeMap::new())
    }

    #[test]
    fn each_refusal_the_original_words_has_its_words() {
        let said = |text: &str| Some(text.to_owned());
        assert_eq!(
            worded(OutfitRefusal::MaxOwned, 1),
            said("Can't have any more!")
        );
        assert_eq!(
            worded(OutfitRefusal::NoneAllowed, 0),
            said("Can't have any of this item!")
        );
        assert_eq!(
            worded(OutfitRefusal::NoSpace, 0),
            said("Can't hold any more!")
        );
        assert_eq!(
            worded(OutfitRefusal::NoSpaceForAny, 0),
            said("Can't hold any of this item!")
        );
        assert_eq!(
            worded(OutfitRefusal::NegativeFreeMass, 1),
            said(
                "Can't sell that item, because your ship would have negative free mass afterwards."
            )
        );
        for limit in [
            OutfitRefusal::MaxOwned,
            OutfitRefusal::NoneAllowed,
            OutfitRefusal::GunLimit,
            OutfitRefusal::TurretLimit,
        ] {
            assert_eq!(worded(limit, 1), said(MAX_OWNED), "{limit:?}");
            assert_eq!(worded(limit, 3), said(MAX_OWNED), "{limit:?}");
            assert_eq!(worded(limit, 0), said(NONE_ALLOWED), "{limit:?}");
        }
        assert_eq!(
            worded(
                OutfitRefusal::AmmunitionFirst {
                    rounds: 2,
                    ammo: None
                },
                1
            ),
            said("You need to sell two units of ammunition before you can sell your .")
        );
        let raised = OutfitRefusal::RaisedFirst {
            count: 2,
            target: OutfitId(200),
        };
        assert_eq!(
            worded(raised, 1),
            said("You need to sell two  before you can sell your ."),
            "no names known: empty words, no panic"
        );
        let names = BTreeMap::from([
            (
                OutfitId(128),
                LcNames {
                    singular: "widget rack".to_owned(),
                    plural: "widget racks".to_owned(),
                },
            ),
            (OutfitId(200), widgets()),
        ]);
        assert_eq!(
            refusal_text(raised, &row(128, "Rack", 1, 1, 1), &names),
            said("You need to sell two widgets before you can sell your widget rack.")
        );
        for silent in [
            OutfitRefusal::NoOutfitter,
            OutfitRefusal::NotListed,
            OutfitRefusal::NotForSale,
            OutfitRefusal::NoExpansion,
            OutfitRefusal::NoCargoRoom,
            OutfitRefusal::BoughtThisOpening,
            OutfitRefusal::CannotAfford,
            OutfitRefusal::NoneOwned,
            OutfitRefusal::CannotSell,
            OutfitRefusal::NotBoughtHere,
            OutfitRefusal::BaysFull,
        ] {
            assert_eq!(worded(silent, 1), None, "{silent:?}");
            assert_eq!(worded(silent, 0), None, "{silent:?}");
        }
    }

    fn vipers() -> LcNames {
        LcNames {
            singular: "Viper".to_owned(),
            plural: "Vipers".to_owned(),
        }
    }

    fn widgets() -> LcNames {
        LcNames {
            singular: "widget".to_owned(),
            plural: "widgets".to_owned(),
        }
    }

    #[test]
    fn the_words_for_selling_a_raiser_before_its_targets() {
        let widgets = widgets();
        let words = |count| raised_first(count, &widgets, "widget rack");
        assert_eq!(
            words(2),
            "You need to sell two widgets before you can sell your widget rack."
        );
        assert_eq!(
            words(1),
            "You need to sell one widget before you can sell your widget rack."
        );
        assert_eq!(
            words(1234),
            "You need to sell 1,234 widgets before you can sell your widget rack."
        );
    }

    #[test]
    fn the_words_for_selling_a_launcher_before_its_ammunition() {
        let vipers = vipers();
        let words = |rounds| ammunition_first(rounds, Some(&vipers), "Viper bay");
        assert_eq!(
            words(2),
            "You need to sell two Vipers before you can sell your Viper bay."
        );
        assert_eq!(
            words(1),
            "You need to sell one Viper before you can sell your Viper bay."
        );
        assert_eq!(
            words(3),
            "You need to sell three Vipers before you can sell your Viper bay."
        );
        assert_eq!(
            words(4),
            "You need to sell 4 Vipers before you can sell your Viper bay."
        );
        assert_eq!(
            words(1234),
            "You need to sell 1,234 Vipers before you can sell your Viper bay."
        );
        assert_eq!(
            ammunition_first(2, None, "missile rack"),
            "You need to sell two units of ammunition before you can sell your missile rack."
        );
        assert_eq!(
            ammunition_first(1, None, "missile rack"),
            "You need to sell one unit of ammunition before you can sell your missile rack."
        );
        assert_eq!(
            ammunition_first(u32::MAX, None, "rack"),
            "You need to sell 4,294,967,295 units of ammunition before you can sell your rack."
        );
    }

    #[test]
    fn the_info_box_says_why_a_launcher_cannot_be_sold() {
        // One bay holds one Viper, and two are aboard.
        let mut bay = row(157, "Viper\\nBay", 100, 5, 1);
        bay.facts.names = LcNames {
            singular: "Viper bay".to_owned(),
            plural: "Viper bays".to_owned(),
        };
        bay.facts.launcher = Some(Launcher {
            max_ammo: 1,
            rounds: 2,
            ammo: Some(OutfitId(158)),
        });
        bay.facts.ammo_names = Some(vipers());
        let screen = screen_of(Outfitter {
            rows: vec![bay],
            rules: outfitter()
                .rules
                .with_override(RuleKey::SaleRefusal, RuleSource::Bible),
            ..outfitter()
        });
        let info = item(&screen, INFO_ITEM);
        let lines: Vec<_> = texts(&drawn(&screen))
            .into_iter()
            .filter(|(_, at, _)| info.contains(*at))
            .map(|(text, _, _)| text)
            .collect();
        assert_eq!(
            lines.last().map(String::as_str),
            Some("You need to sell two Vipers before you can sell your Viper bay.")
        );
        assert_eq!(enabled(&screen), (true, false), "Sell is greyed");
    }

    #[test]
    fn a_short_name_splits_on_backslash_n_and_greys_lines_not_starting_with_a_letter_or_digit() {
        assert_eq!(
            name_lines("Real Big\\nScary Gun"),
            [("Real Big", TEXT_COLOR), ("Scary Gun", TEXT_COLOR)]
        );
        assert_eq!(
            name_lines("*Ammo\\n9mm\\n(x10)"),
            [("*Ammo", GREY), ("9mm", TEXT_COLOR), ("(x10)", GREY)]
        );
        assert_eq!(name_lines(""), [("", GREY)]);
        assert_eq!(name_lines("Élan"), [("Élan", TEXT_COLOR)]);
    }

    #[test]
    fn by_the_engine_it_opens_with_nothing_selected() {
        let mut screen = opened(outfitter());
        assert_eq!((screen.selected(), screen.selected_outfit()), (None, None));
        assert_eq!(screen.top_row(), 0);
        assert_eq!(enabled(&screen), (false, false), "Buy and Sell grey");
        let commands = drawn(&screen);
        for index in 0..4 {
            assert!(
                !contains(&commands, &bright_frame(&screen, index)),
                "{index}"
            );
        }
        let words = words(&screen);
        assert!(!words.contains(&"dësc 3000.".to_owned()), "{words:?}");
        assert!(!words.contains(&PRICE_LABEL.to_owned()), "{words:?}");
        let picture = item(&screen, PICTURE_ITEM);
        assert!(
            !commands.iter().any(|command| matches!(
                command,
                DrawCommand::StretchedPicture { top_left, .. } if *top_left == picture.min
            )),
            "no picture"
        );
        assert_eq!(screen.take_description_request(), None);
    }

    #[test]
    fn by_the_engine_the_first_arrow_after_opening_picks_a_cell_and_reads_it() {
        let mut right = opened(long(30));
        press(&mut right, Key::Right);
        assert_eq!(right.selected(), Some(0));
        assert_eq!(right.take_description_request(), Some(3000 + 200 - 128));
        let mut up = opened(long(30));
        press(&mut up, Key::Up);
        assert_eq!(up.selected(), Some(19), "the last shown");
        assert_eq!(up.take_description_request(), Some(3000 + 219 - 128));
    }

    #[test]
    fn by_the_other_reading_it_opens_on_the_first_cell() {
        let mut screen = opened(by_the_other_grid_reading(outfitter()));
        assert_eq!(screen.selected(), Some(0));
        assert_eq!(screen.selected_outfit(), Some(OutfitId(128)));
        assert_eq!(enabled(&screen), (true, true));
        assert!(contains(&drawn(&screen), &bright_frame(&screen, 0)));
        assert!(words(&screen).contains(&"dësc 3000.".to_owned()));
        assert_eq!(screen.take_description_request(), Some(3000));
    }

    #[test]
    fn it_is_centred() {
        let screen = screen();
        assert_eq!(screen.problem(), None);
        assert_eq!(screen.dialog().expect("laid out").bounds().min, ORIGIN);
        assert!(!screen.closed());
        assert_eq!(screen.outfitter(), &outfitter());
        assert_eq!(item(&screen, GRID_ITEM), rect(138.0, 231.0, 333.0, 271.0));
        assert_eq!(
            screen.cell_bounds(5),
            Some(rect(138.0 + 83.0, 231.0 + 54.0, 84.0, 55.0))
        );
    }

    #[test]
    fn the_stock_templates_disabled_sell_and_arrows_work() {
        let mut template = template();
        for number in [SCROLL_UP_ITEM, SCROLL_DOWN_ITEM, DONE_ITEM, BUY_ITEM] {
            template.items[number - 1].enabled = false;
        }
        let mut screen = OutfitterScreen::new(
            Ok((template, Rc::new(MonoMetrics))),
            Ok(quantity()),
            Ok(desc_template()),
            outfitter(),
            art(),
            ButtonStyle::STOCK,
        );
        click_cell(&mut screen, 0);
        click_item(&mut screen, SELL_ITEM);
        assert_eq!(screen.take_order(), Some(order(128, Direction::Sell)));
        click_item(&mut screen, BUY_ITEM);
        assert_eq!(screen.take_order(), Some(order(128, Direction::Buy)));
        click_item(&mut screen, DONE_ITEM);
        assert!(screen.closed());
        let short = DialogTemplate {
            items: template_items(3),
            ..super::tests::template()
        };
        let screen = OutfitterScreen::new(
            Ok((short, Rc::new(MonoMetrics))),
            Ok(quantity()),
            Ok(desc_template()),
            outfitter(),
            art(),
            ButtonStyle::STOCK,
        );
        assert_eq!(screen.cell_bounds(0), None, "no grid");
        assert!(drawn(&screen).len() > 1);
    }

    /// The stock template's first `count` items.
    fn template_items(count: usize) -> Vec<ItemTemplate> {
        template().items.into_iter().take(count).collect()
    }

    #[test]
    fn it_draws_the_background_the_grid_the_selection_the_arrows_then_the_dialog() {
        let screen = screen();
        let commands = drawn(&screen);
        assert_eq!(
            commands[0],
            DrawCommand::Picture {
                image: BACKGROUND,
                top_left: ORIGIN,
            }
        );
        // A name line centred across the cell, `rise` (its baseline) above
        // the bottom; MonoMetrics sets Geneva 10 at 5 a character.
        let name = |index: usize, words: &str, rise: f32| {
            let bounds = screen.cell_bounds(index).expect("a cell");
            let width = 5.0 * words.chars().count() as f32;
            Point::new(bounds.center().x - width / 2.0, bounds.max.y - rise - 10.0)
        };
        // The count, ending 3 inside the right, its baseline 12 below the top.
        let count = |index: usize, words: &str| {
            let bounds = screen.cell_bounds(index).expect("a cell");
            let width = 5.0 * words.chars().count() as f32;
            Point::new(bounds.max.x - 3.0 - width, bounds.min.y + 12.0 - 10.0)
        };
        let at = info_at(&screen);
        let description = item(&screen, DESCRIPTION_ITEM).min;
        let mut expected = vec![
            ("2".to_owned(), count(0, "2"), TEXT_COLOR),
            ("Light".to_owned(), name(0, "Light", 14.0), TEXT_COLOR),
            ("Blaster".to_owned(), name(0, "Blaster", 3.0), TEXT_COLOR),
            ("*Ammo".to_owned(), name(1, "*Ammo", 14.0), GREY),
            ("Pack".to_owned(), name(1, "Pack", 3.0), TEXT_COLOR),
            ("Big Gun".to_owned(), name(2, "Big Gun", 6.0), TEXT_COLOR),
            ("1".to_owned(), count(3, "1"), TEXT_COLOR),
            ("Map".to_owned(), name(3, "Map", 6.0), TEXT_COLOR),
            ("dësc 3000.".to_owned(), description, Color::WHITE),
            ("Item Price:".to_owned(), at(0.0, 2.0), TEXT_COLOR),
            ("1,000 cr".to_owned(), at(70.0, 2.0), TEXT_COLOR),
            ("You Have:".to_owned(), at(0.0, 14.0), TEXT_COLOR),
            ("5,000 cr".to_owned(), at(70.0, 14.0), TEXT_COLOR),
            ("Item Mass:".to_owned(), at(0.0, 38.0), TEXT_COLOR),
            ("1 ton".to_owned(), at(70.0, 38.0), TEXT_COLOR),
            ("Available:".to_owned(), at(0.0, 50.0), TEXT_COLOR),
            ("5 tons".to_owned(), at(70.0, 50.0), TEXT_COLOR),
        ];
        let mut dialog = DrawList::new();
        screen.dialog().expect("laid out").draw(&mut dialog);
        let buttons: Vec<DrawCommand> = dialog.iter().cloned().collect();
        expected.extend(texts(&buttons));
        assert_eq!(texts(&commands), expected);
        assert_eq!(commands[commands.len() - buttons.len()..], buttons[..]);
        let labels: Vec<_> = texts(&buttons).into_iter().map(|(t, _, _)| t).collect();
        assert_eq!(labels, ["Done", "Sell", "Buy"]);
        let picture = item(&screen, PICTURE_ITEM);
        assert!(commands.contains(&DrawCommand::StretchedPicture {
            image: ImageKey::picture(6000),
            top_left: picture.min,
            width: 200.0,
            height: 200.0,
        }));
    }

    #[test]
    fn the_text_is_geneva_10() {
        let commands = drawn(&screen());
        for command in &commands {
            if let DrawCommand::Text {
                text,
                font,
                size,
                wrap_width,
                ..
            } = command
                && !["Done", "Sell", "Buy"].contains(&text.as_str())
            {
                assert_eq!(
                    (*font, *size, *wrap_width),
                    (Font::Geneva, 10.0, None),
                    "{text}"
                );
            }
        }
    }

    /// Shown cell `index`'s frame in `color`.
    fn framed(screen: &OutfitterScreen, index: usize, color: Color) -> Vec<DrawCommand> {
        let mut list = DrawList::new();
        frame_rect(&mut list, screen.cell_bounds(index).expect("a cell"), color);
        list.iter().cloned().collect()
    }

    /// Shown cell `index`'s frame in the fake's `GridDim`.
    fn frame(screen: &OutfitterScreen, index: usize) -> Vec<DrawCommand> {
        framed(screen, index, grid::TEST_GRID_COLORS.dim)
    }

    /// Shown cell `index`'s selection: framed in the fake's `GridBright`.
    fn bright_frame(screen: &OutfitterScreen, index: usize) -> Vec<DrawCommand> {
        framed(screen, index, grid::TEST_GRID_COLORS.bright)
    }

    fn contains(commands: &[DrawCommand], part: &[DrawCommand]) -> bool {
        commands.windows(part.len()).any(|window| window == part)
    }

    /// The frame rule itself is `grid::CellGrid::draw_cells`'s, tested
    /// there; this checks the outfitter hands it its grid item, rows,
    /// selection and `cölr` colours.
    #[test]
    fn the_grid_is_drawn_by_the_shared_cell_rule_on_its_item_in_the_catalogs_colours() {
        let mut screen = screen_of(long(22));
        let grid_item = item(&screen, GRID_ITEM);
        let commands = drawn(&screen);
        for index in 0..20 {
            assert_eq!(
                screen.cell_bounds(index),
                Some(grid::cell_bounds(grid_item, index))
            );
            assert!(contains(&commands, &frame(&screen, index)), "{index}");
        }
        assert!(contains(&commands, &bright_frame(&screen, 0)), "selected");
        press(&mut screen, Key::Right);
        let commands = drawn(&screen);
        assert!(!contains(&commands, &bright_frame(&screen, 0)));
        assert!(contains(&commands, &bright_frame(&screen, 1)));
        let few = drawn(&screen_of(long(3)));
        let three = screen_of(long(3));
        assert!(contains(&few, &frame(&three, 2)));
        assert!(
            !contains(&few, &frame(&three, 3)),
            "no frame on an empty cell"
        );
    }

    #[test]
    fn the_arrow_keys_move_the_selection_and_stop_at_the_ends() {
        let mut screen = screen_of(long(10));
        press(&mut screen, Key::Left);
        assert_eq!(screen.selected(), Some(0));
        press(&mut screen, Key::Up);
        assert_eq!(screen.selected(), Some(0));
        press(&mut screen, Key::Right);
        assert_eq!(screen.selected(), Some(1));
        screen.input(&key(Key::Down, true, true));
        assert_eq!(screen.selected(), Some(5), "a row down, repeats too");
        press(&mut screen, Key::Down);
        assert_eq!(screen.selected(), Some(9), "the last");
        press(&mut screen, Key::Right);
        assert_eq!(screen.selected(), Some(9));
        screen.input(&key(Key::Up, false, false));
        assert_eq!(screen.selected(), Some(9), "not on release");
        press(&mut screen, Key::Up);
        assert_eq!(screen.selected(), Some(5));
        press(&mut screen, Key::Left);
        assert_eq!(screen.selected(), Some(4));
        assert_eq!(screen.take_sounds(), [], "keys are silent");
    }

    #[test]
    fn by_the_engine_down_onto_an_empty_cell_and_up_on_the_first_row_stay() {
        let at = |outfitter: Outfitter, from: usize, k: Key| {
            let mut screen = screen_of(outfitter);
            click_cell(&mut screen, from);
            press(&mut screen, k);
            (screen.selected(), screen.selected_outfit())
        };
        let outfit = |index: usize| Some(OutfitId(200 + i16::try_from(index).expect("few")));
        assert_eq!(at(long(10), 6, Key::Down), (Some(6), outfit(6)));
        assert_eq!(at(long(10), 2, Key::Up), (Some(2), outfit(2)));
        assert_eq!(at(long(10), 1, Key::PageDown), (Some(5), outfit(5)));
        assert_eq!(at(long(10), 5, Key::PageUp), (Some(1), outfit(1)));
        let clamped = |outfitter: Outfitter| Outfitter {
            rules: outfitter
                .rules
                .with_override(RuleKey::GridArrows, RuleSource::Bible),
            ..outfitter
        };
        assert_eq!(at(clamped(long(10)), 6, Key::Down), (Some(9), outfit(9)));
        assert_eq!(at(clamped(long(10)), 2, Key::Up), (Some(0), outfit(0)));
        assert_eq!(
            at(clamped(long(10)), 6, Key::PageDown),
            (Some(9), outfit(9))
        );
    }

    #[test]
    fn a_click_on_a_cell_selects_it_and_by_the_other_reading_on_an_empty_one_nothing() {
        let mut screen = screen_of(by_the_other_grid_reading(outfitter()));
        click_cell(&mut screen, 2);
        assert_eq!(screen.selected(), Some(2));
        click_cell(&mut screen, 4);
        assert_eq!(screen.selected(), Some(2), "the first empty cell");
        let grid = item(&screen, GRID_ITEM);
        let mut full = screen_of(long(20));
        click(&mut full, Point::new(grid.max.x, grid.min.y + 1.0));
        assert_eq!(full.selected(), Some(0), "the right edge is out");
        click(&mut full, Point::new(grid.min.x + 1.0, grid.max.y));
        assert_eq!(full.selected(), Some(0), "the bottom edge is out");
        click(&mut full, Point::new(grid.min.x + 83.0, grid.min.y + 1.0));
        assert_eq!(full.selected(), Some(0), "83 right is still column 0");
        click(&mut full, Point::new(grid.min.x + 84.0, grid.min.y + 54.0));
        assert_eq!(
            full.selected(),
            Some(1),
            "84 right is column 1, 54 down row 0"
        );
        click(&mut full, Point::new(grid.max.x - 0.5, grid.max.y - 0.5));
        assert_eq!(
            full.selected(),
            Some(19),
            "the last cell reaches the corner"
        );
        click(&mut screen, Point::new(grid.min.x + 1.0, grid.min.y + 1.0));
        assert_eq!(screen.selected(), Some(0));
        click(&mut screen, Point::new(grid.min.x - 1.0, grid.min.y + 1.0));
        assert_eq!(screen.selected(), Some(0), "outside the grid");
        assert_eq!(screen.take_sounds(), [], "cells are silent");
        assert_eq!(screen.take_order(), None);
    }

    /// `count` outfits, numbered from 200, each 10 credits and a ton.
    fn long(count: usize) -> Outfitter {
        Outfitter {
            rows: (0..count)
                .map(|n| {
                    let id = 200 + i16::try_from(n).expect("few");
                    row(id, &format!("Good {n}"), 10, 1, 0)
                })
                .collect(),
            ..outfitter()
        }
    }

    fn names(screen: &OutfitterScreen) -> Vec<String> {
        words(screen)
            .into_iter()
            .filter(|text| text.starts_with("Good "))
            .collect()
    }

    /// `outfitter`'s screen with index 20 of 22 selected on the last row,
    /// then scrolled up past it to the first.
    fn scrolled_past_twenty(outfitter: Outfitter) -> OutfitterScreen {
        let mut screen = opened(outfitter);
        click_item(&mut screen, SCROLL_DOWN_ITEM);
        click_cell(&mut screen, 16);
        assert_eq!((screen.selected(), screen.top_row()), (Some(20), 1));
        click_item(&mut screen, SCROLL_UP_ITEM);
        assert_eq!(screen.top_row(), 0);
        screen
    }

    #[test]
    fn by_the_other_reading_a_scroll_keeps_the_selection_off_screen() {
        let screen = scrolled_past_twenty(by_the_other_grid_reading(long(22)));
        assert_eq!(screen.selected(), Some(20));
        assert_eq!(screen.selected_outfit(), Some(OutfitId(220)));
        assert_eq!(enabled(&screen), (true, false), "Buy still answers");
        assert!(!any_highlight(&screen), "off screen");
        let mut kept = opened(by_the_other_grid_reading(long(30)));
        let _ = kept.take_description_request();
        click_item(&mut kept, SCROLL_DOWN_ITEM);
        assert_eq!(kept.selected(), Some(0), "the first cell, off screen");
        assert_eq!(kept.take_description_request(), None, "nothing re-read");
    }

    #[test]
    fn by_the_engine_a_scroll_that_takes_the_cell_out_of_view_leaves_nothing_selected() {
        let up = scrolled_past_twenty(long(22));
        let mut down = opened(long(30));
        click_cell(&mut down, 3);
        assert!(any_info(&down) && described(&down));
        click_item(&mut down, SCROLL_DOWN_ITEM);
        assert_eq!(down.top_row(), 1);
        for screen in [up, down] {
            assert_eq!((screen.selected(), screen.selected_outfit()), (None, None));
            assert_eq!(enabled(&screen), (false, false));
            assert!(!any_highlight(&screen));
            assert!(!any_info(&screen), "{:?}", words(&screen));
            assert!(!described(&screen));
            assert_eq!(picture_drawn(&screen), None);
        }
    }

    #[test]
    fn by_the_engine_a_scroll_that_keeps_the_cell_in_view_reads_its_description_again() {
        let art = art();
        let mut screen = opened_with(long(30), &art);
        click_cell(&mut screen, 9);
        assert_eq!(screen.take_description_request(), Some(3000 + 209 - 128));
        art.asked.borrow_mut().clear();
        click_item(&mut screen, SCROLL_DOWN_ITEM);
        assert_eq!((screen.selected(), screen.top_row()), (Some(9), 1));
        assert_eq!(screen.take_description_request(), Some(3000 + 209 - 128));
        assert_eq!(*art.asked.borrow(), ["dësc 3081"], "read again");
        assert!(contains(&drawn(&screen), &bright_frame(&screen, 5)));
        assert_eq!(enabled(&screen), (true, false));
        click_item(&mut screen, SCROLL_UP_ITEM);
        assert_eq!((screen.selected(), screen.top_row()), (Some(9), 0));
        assert_eq!(screen.take_description_request(), Some(3081), "Up too");
    }

    /// 30 outfits numbered from 200, with the blaster in place of index
    /// `index`.
    fn long_with_blaster_at(index: usize) -> Outfitter {
        let mut outfitter = long(30);
        outfitter.rows[index] = row(128, "Light\\nBlaster", 1000, 1, 2);
        outfitter
    }

    #[test]
    fn by_the_engine_a_scroll_ends_a_reset() {
        let mut screen = reset(opened(long_with_blaster_at(0)));
        assert_eq!(screen.selected_outfit(), Some(BLASTER));
        assert_eq!(enabled(&screen), (true, true));
        click_item(&mut screen, SCROLL_UP_ITEM);
        assert_eq!(screen.top_row(), 0, "cannot scroll");
        assert_eq!(screen.selected_outfit(), Some(BLASTER), "unchanged");
        assert_eq!(enabled(&screen), (true, true));
        click_item(&mut screen, SCROLL_DOWN_ITEM);
        assert_eq!(screen.top_row(), 1);
        assert_eq!(screen.selected_outfit(), None, "the reset ended");
        assert_eq!(enabled(&screen), (false, false));
        press(&mut screen, BUY_KEY);
        assert_eq!(screen.take_order(), None);
    }

    #[test]
    fn by_the_engine_a_scroll_fills_a_blank_description() {
        let mut screen = reset(opened(long_with_blaster_at(19)));
        let _ = screen.take_description_request();
        press(&mut screen, Key::Up);
        assert_eq!(screen.selected(), Some(19));
        assert_eq!(screen.selected_outfit(), Some(BLASTER));
        assert!(!described(&screen), "blank, as the reset left it");
        assert_eq!(screen.take_description_request(), None);
        click_item(&mut screen, SCROLL_DOWN_ITEM);
        assert_eq!((screen.selected(), screen.top_row()), (Some(19), 1));
        assert!(words(&screen).contains(&"dësc 3000.".to_owned()));
        assert_eq!(screen.take_description_request(), Some(3000));
    }

    #[test]
    fn a_held_b_stops_after_a_scroll_drops_its_outfit() {
        let repeat_b = |screen: &mut OutfitterScreen| {
            screen.input(&key(BUY_KEY, true, true));
            screen.take_order()
        };
        let mut dropped = opened(long(30));
        click_cell(&mut dropped, 3);
        press(&mut dropped, BUY_KEY);
        assert_eq!(dropped.take_order(), Some(order(203, Direction::Buy)));
        click_item(&mut dropped, SCROLL_DOWN_ITEM);
        assert_eq!(repeat_b(&mut dropped), None, "the held B stops");
        let mut kept = opened(long(30));
        click_cell(&mut kept, 9);
        press(&mut kept, BUY_KEY);
        assert_eq!(kept.take_order(), Some(order(209, Direction::Buy)));
        click_item(&mut kept, SCROLL_DOWN_ITEM);
        assert_eq!(repeat_b(&mut kept), Some(order(209, Direction::Buy)));
    }

    #[test]
    fn by_the_engine_a_scroll_that_cannot_move_changes_nothing() {
        for (outfitter, item) in [(long(30), SCROLL_UP_ITEM), (long(20), SCROLL_DOWN_ITEM)] {
            let mut screen = opened(outfitter);
            click_cell(&mut screen, 2);
            let _ = screen.take_description_request();
            let before = drawn(&screen);
            click_item(&mut screen, item);
            assert_eq!(screen.selected(), Some(2), "{item}");
            assert_eq!(screen.take_description_request(), None, "{item}");
            assert_eq!(drawn(&screen), before, "{item}");
        }
        let mut unselected = opened(long(20));
        click_item(&mut unselected, SCROLL_DOWN_ITEM);
        assert_eq!(unselected.selected(), None);
        assert_eq!(unselected.take_description_request(), None);
    }

    #[test]
    fn the_grid_scrolls_a_row_at_a_time_with_the_selection_or_its_arrows() {
        let mut screen = screen_of(long(22));
        let first: Vec<_> = (0..20).map(|n| format!("Good {n}")).collect();
        assert_eq!(names(&screen), first);
        for _ in 0..4 {
            press(&mut screen, Key::Down);
        }
        assert_eq!((screen.selected(), screen.top_row()), (Some(16), 0));
        press(&mut screen, Key::Down);
        assert_eq!((screen.selected(), screen.top_row()), (Some(20), 1));
        assert_eq!(names(&screen)[0], "Good 4");
        assert_eq!(names(&screen).last().map(String::as_str), Some("Good 21"));
        assert!(contains(&drawn(&screen), &bright_frame(&screen, 16)));
        click_item(&mut screen, SCROLL_DOWN_ITEM);
        assert_eq!(screen.top_row(), 1, "the last row shows");
        click_item(&mut screen, SCROLL_UP_ITEM);
        assert_eq!(screen.top_row(), 0);
        click_item(&mut screen, SCROLL_UP_ITEM);
        assert_eq!(screen.top_row(), 0, "the first row shows");
        assert_eq!(
            screen.selected(),
            None,
            "scrolled out of view by the engine"
        );
        click_item(&mut screen, SCROLL_DOWN_ITEM);
        assert_eq!(screen.top_row(), 1);
        click_cell(&mut screen, 0);
        assert_eq!((screen.selected(), screen.top_row()), (Some(4), 1));
        press(&mut screen, Key::Up);
        assert_eq!((screen.selected(), screen.top_row()), (Some(0), 0));
    }

    /// The lines drawn from inside the scroll arrows' items.
    fn arrow_lines(screen: &OutfitterScreen) -> Vec<(Point, Point, Color)> {
        let up = item(screen, SCROLL_UP_ITEM);
        let down = item(screen, SCROLL_DOWN_ITEM);
        drawn(screen)
            .into_iter()
            .filter_map(|command| match command {
                DrawCommand::Line {
                    from, to, color, ..
                } => Some((from, to, color)),
                _ => None,
            })
            .filter(|(from, _, _)| up.contains(*from) || down.contains(*from))
            .collect()
    }

    /// The arrows' colours, up then down.
    fn arrows(screen: &OutfitterScreen) -> Vec<Color> {
        arrow_lines(screen)
            .into_iter()
            .step_by(3)
            .map(|(_, _, color)| color)
            .collect()
    }

    #[test]
    fn the_arrows_are_white_only_when_the_grid_can_scroll_that_way() {
        let mut screen = screen_of(long(22));
        let up = item(&screen, SCROLL_UP_ITEM);
        let down = item(&screen, SCROLL_DOWN_ITEM);
        let commands = drawn(&screen);
        let lines: Vec<_> = commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Line {
                    from, to, color, ..
                } => Some((*from, *to, *color)),
                _ => None,
            })
            .filter(|(from, _, _)| up.contains(*from) || down.contains(*from))
            .collect();
        let apex_up = Point::new(up.center().x, up.min.y + INSET);
        let apex_down = Point::new(down.center().x, down.max.y - INSET);
        assert_eq!(
            lines,
            [
                (
                    Point::new(up.min.x + INSET, up.max.y - INSET),
                    Point::new(up.max.x - INSET, up.max.y - INSET),
                    GREY
                ),
                (
                    Point::new(up.max.x - INSET, up.max.y - INSET),
                    apex_up,
                    GREY
                ),
                (
                    apex_up,
                    Point::new(up.min.x + INSET, up.max.y - INSET),
                    GREY
                ),
                (
                    Point::new(down.min.x + INSET, down.min.y + INSET),
                    Point::new(down.max.x - INSET, down.min.y + INSET),
                    TEXT_COLOR
                ),
                (
                    Point::new(down.max.x - INSET, down.min.y + INSET),
                    apex_down,
                    TEXT_COLOR
                ),
                (
                    apex_down,
                    Point::new(down.min.x + INSET, down.min.y + INSET),
                    TEXT_COLOR
                ),
            ]
        );
        assert_eq!(arrows(&screen), [GREY, TEXT_COLOR]);
        click_item(&mut screen, SCROLL_DOWN_ITEM);
        click_item(&mut screen, SCROLL_DOWN_ITEM);
        assert_eq!(arrows(&screen), [TEXT_COLOR, GREY]);
        assert_eq!(arrows(&screen_of(long(16))), [GREY, GREY]);
    }

    #[test]
    fn the_picture_and_description_are_read_as_the_selection_changes() {
        let art = art();
        let mut screen = screen_with(outfitter(), &art);
        assert_eq!(
            *art.asked.borrow(),
            [
                "PICT 6000",
                "PICT 6001",
                "PICT 6002",
                "PICT 6003",
                "dësc 3000"
            ],
            "every cell's picture, then the selected one's description"
        );
        press(&mut screen, Key::Right);
        let shown = words(&screen);
        assert!(shown.contains(&NO_PICTURE.to_owned()), "{shown:?}");
        assert!(shown.contains(&"dësc 3001.".to_owned()));
        press(&mut screen, Key::Right);
        let shown = words(&screen);
        assert!(
            !shown.iter().any(|text| text.starts_with("dësc")),
            "{shown:?}"
        );
        press(&mut screen, Key::Right);
        press(&mut screen, Key::Right);
        screen.set_outfitter(outfitter());
        assert_eq!(
            *art.asked.borrow(),
            [
                "PICT 6000",
                "PICT 6001",
                "PICT 6002",
                "PICT 6003",
                "dësc 3000",
                "dësc 3001",
                "dësc 3002",
                "dësc 3003",
                "PICT 6000",
                "PICT 6001",
                "PICT 6002",
                "PICT 6003",
            ],
            "the pictures once a list, each description once while it stays selected"
        );
        let picture = item(&screen, PICTURE_ITEM);
        assert_eq!(in_picture(&screen), no_picture_lines(picture));
    }

    /// The texts drawn in the picture item.
    fn in_picture(screen: &OutfitterScreen) -> Vec<(String, Point, Color)> {
        let picture = item(screen, PICTURE_ITEM);
        texts(&drawn(screen))
            .into_iter()
            .filter(|(_, at, _)| picture.contains(*at))
            .collect()
    }

    /// The picture item's two missing-picture lines as drawn: centred
    /// across it (`MonoMetrics` sets Geneva 10 at 5 a character), on
    /// baselines 6 either side of the floor of its middle, in white.
    fn no_picture_lines(picture: Bounds) -> Vec<(String, Point, Color)> {
        let mid_x = f32::midpoint(picture.min.x, picture.max.x).floor();
        let mid_y = f32::midpoint(picture.min.y, picture.max.y).floor();
        vec![
            (
                "No Picture".to_owned(),
                Point::new(mid_x - 25.0, grid::baseline_top(mid_y - 6.0)),
                grid::TEXT_COLOR,
            ),
            (
                "Available".to_owned(),
                Point::new(mid_x - 22.5, grid::baseline_top(mid_y + 6.0)),
                grid::TEXT_COLOR,
            ),
        ]
    }

    #[test]
    fn a_missing_picture_shows_no_picture_and_available_centred_in_white() {
        let mut screen = screen_with(outfitter(), &art());
        press(&mut screen, Key::Right);
        let picture = item(&screen, PICTURE_ITEM);
        assert_eq!(in_picture(&screen), no_picture_lines(picture));
    }

    #[test]
    fn an_outfits_picture_is_pict_6000_plus_its_index_when_it_exists() {
        assert_eq!(outfit_picture(OutfitId(128), |id| id == 6000), Some(6000));
        assert_eq!(outfit_picture(OutfitId(131), |id| id == 6003), Some(6003));
        assert_eq!(outfit_picture(OutfitId(131), |id| id == 6000), None);
        assert_eq!(
            outfit_picture(OutfitId(i16::MIN), |_| true),
            Some(6000 + (i16::MIN))
        );
        assert_eq!(outfit_picture(OutfitId(i16::MAX), |_| true), Some(i16::MAX));
    }

    /// The commands drawn inside `cell`'s bounds, by the point they start
    /// at.
    fn inside(commands: &[DrawCommand], cell: Bounds) -> Vec<DrawCommand> {
        commands
            .iter()
            .filter(|command| match command {
                DrawCommand::StretchedPicture { top_left, .. }
                | DrawCommand::Text {
                    origin: top_left, ..
                } => cell.contains(*top_left),
                DrawCommand::Line { from, to, .. } => cell.contains(*from) && cell.contains(*to),
                _ => false,
            })
            .cloned()
            .collect()
    }

    /// The black square in `cell`'s icon.
    fn black_square(cell: Bounds) -> Vec<DrawCommand> {
        let mut list = DrawList::new();
        fill_rect(&mut list, grid::icon_bounds(cell), Color::BLACK);
        list.iter().cloned().collect()
    }

    #[test]
    fn each_cell_shows_its_picture_and_a_missing_one_is_a_black_square() {
        let screen = screen();
        let commands = drawn(&screen);
        let first = screen.cell_bounds(0).expect("a cell");
        let picture = DrawCommand::StretchedPicture {
            image: ImageKey::picture(6000),
            top_left: grid::icon_bounds(first).min,
            width: 32.0,
            height: 32.0,
        };
        let in_first = inside(&commands, first);
        let at = |wanted: &DrawCommand| in_first.iter().position(|c| c == wanted);
        let picture_at = at(&picture).expect("outfit 128's PICT 6000 in its cell");
        let first_text = in_first
            .iter()
            .position(|c| matches!(c, DrawCommand::Text { .. }))
            .expect("its count and name");
        assert!(picture_at < first_text, "the picture under the text");
        for index in 1..4 {
            let cell = screen.cell_bounds(index).expect("a cell");
            let in_cell = inside(&commands, cell);
            assert!(
                !in_cell
                    .iter()
                    .any(|c| matches!(c, DrawCommand::StretchedPicture { .. })),
                "no PICT for cell {index}"
            );
            let square = black_square(cell);
            let start = in_cell
                .windows(square.len())
                .position(|window| window == square.as_slice())
                .unwrap_or_else(|| panic!("a black square in cell {index}"));
            let last_text = in_cell
                .iter()
                .rposition(|c| matches!(c, DrawCommand::Text { .. }))
                .expect("its name");
            assert!(start < last_text, "the name over the square");
        }
        let map = screen.cell_bounds(3).expect("a cell");
        let map_texts: Vec<String> = texts(&inside(&commands, map))
            .into_iter()
            .map(|(t, _, _)| t)
            .collect();
        assert_eq!(map_texts, ["1", "Map"], "its count, then its name");
    }

    #[test]
    fn a_new_list_reads_its_pictures_again() {
        let art = Rc::new(FakeArt {
            pictures: vec![6000, 6005],
            asked: RefCell::default(),
        });
        let mut screen = screen_with(outfitter(), &art);
        let mut changed = outfitter();
        changed.rows.push(row(133, "New", 10, 0, 0));
        screen.set_outfitter(changed);
        let fifth = screen.cell_bounds(4).expect("a cell");
        assert!(drawn(&screen).contains(&DrawCommand::StretchedPicture {
            image: ImageKey::picture(6005),
            top_left: grid::icon_bounds(fifth).min,
            width: 32.0,
            height: 32.0,
        }));
    }

    #[test]
    fn an_outfit_with_an_impossible_id_does_not_overflow() {
        let art = art();
        let odd = Outfitter {
            rows: vec![row(i16::MIN, "Odd", 1, 1, 0), row(i16::MAX, "Big", 1, 1, 0)],
            ..outfitter()
        };
        let mut screen = screen_with(odd, &art);
        press(&mut screen, Key::Right);
        assert_eq!(
            *art.asked.borrow(),
            ["PICT -26768", "PICT 32767", "dësc -29768", "dësc 32767"]
        );
    }

    #[test]
    fn the_available_line_is_the_clamped_free_mass_with_a_ton_at_exactly_1() {
        for (free_mass, amount) in [(-3, "0 tons"), (0, "0 tons"), (1, "1 ton"), (2, "2 tons")] {
            let shown = info_lines_of(Outfitter {
                free_mass,
                ..outfitter()
            });
            assert!(
                follows(&shown, AVAILABLE_LABEL, amount),
                "{free_mass}: {shown:?}"
            );
            assert!(!shown.iter().any(|(text, _)| text == "-3 tons"));
        }
    }

    #[test]
    fn the_item_mass_line_has_a_ton_at_exactly_1_and_no_line_without_mass() {
        let lines = |mass| {
            info_lines_of(Outfitter {
                rows: vec![row(128, "Widget", 100, mass, 0)],
                free_mass: 5,
                ..outfitter()
            })
        };
        let one = lines(1);
        assert!(follows(&one, MASS_LABEL, "1 ton"), "{one:?}");
        assert!(follows(&one, AVAILABLE_LABEL, "5 tons"), "{one:?}");
        let two = lines(2);
        assert!(follows(&two, MASS_LABEL, "2 tons"), "{two:?}");
        for mass in [0, -5] {
            let none = lines(mass);
            assert!(
                !none
                    .iter()
                    .any(|(line, _)| line == "Item Mass:" || line == "Available:"),
                "{mass}: {none:?}"
            );
            assert!(follows(&none, PRICE_LABEL, "100 cr"), "{none:?}");
            assert!(follows(&none, "You Have:", "5,000 cr"), "{none:?}");
        }
    }

    /// `lines` as texts placed from the info box's top left.
    fn placed(lines: &[(&str, f32, f32)]) -> Vec<(String, Point)> {
        lines
            .iter()
            .map(|&(text, x, y)| (text.to_owned(), Point::new(x, y)))
            .collect()
    }

    #[test]
    fn the_info_box_draws_its_lines_at_the_originals_rows_and_columns() {
        // Baselines 12, 24, 48 and 60 below the top, and 93 for the
        // refusal; MonoMetrics sets Geneva 10 with its top 10 above.
        let mut screen = screen();
        let drawn = info_drawn(&screen);
        assert!(drawn.iter().all(|(_, _, color)| *color == TEXT_COLOR));
        assert_eq!(
            info_placed(&screen),
            placed(&[
                ("Item Price:", 0.0, 2.0),
                ("1,000 cr", 70.0, 2.0),
                ("You Have:", 0.0, 14.0),
                ("5,000 cr", 70.0, 14.0),
                ("Item Mass:", 0.0, 38.0),
                ("1 ton", 70.0, 38.0),
                ("Available:", 0.0, 50.0),
                ("5 tons", 70.0, 50.0),
            ])
        );
        // The massless ammunition.
        press(&mut screen, Key::Right);
        assert_eq!(
            info_placed(&screen),
            placed(&[
                ("Item Price:", 0.0, 2.0),
                ("50 cr", 70.0, 2.0),
                ("You Have:", 0.0, 14.0),
                ("5,000 cr", 70.0, 14.0),
            ])
        );
        // The gun too heavy to hold.
        press(&mut screen, Key::Right);
        assert_eq!(
            info_placed(&screen),
            placed(&[
                ("Item Price:", 0.0, 2.0),
                ("9,000 cr", 70.0, 2.0),
                ("You Have:", 0.0, 14.0),
                ("5,000 cr", 70.0, 14.0),
                ("Item Mass:", 0.0, 38.0),
                ("10 tons", 70.0, 38.0),
                ("Available:", 0.0, 50.0),
                ("5 tons", 70.0, 50.0),
                (NO_SPACE_FOR_ANY, 0.0, 83.0),
            ])
        );
        // A massless outfit's refusal stays on its row; the Bible's sale
        // refusal is a line below it.
        let overloaded = Outfitter {
            rows: vec![OutfitRow {
                max: 1,
                ..row(128, "Expansion", 100, -5, 1)
            }],
            free_mass: 4,
            rules: Rulebook::default()
                .with_override(RuleKey::OutfitCount, RuleSource::Engine)
                .with_override(RuleKey::SaleRefusal, RuleSource::Bible),
            ..outfitter()
        };
        let screen = screen_of(overloaded);
        let drawn = info_drawn(&screen);
        assert!(drawn.iter().all(|(_, _, color)| *color == TEXT_COLOR));
        assert_eq!(
            info_placed(&screen),
            placed(&[
                ("Item Price:", 0.0, 2.0),
                ("100 cr", 70.0, 2.0),
                ("You Have:", 0.0, 14.0),
                ("5,000 cr", 70.0, 14.0),
                (MAX_OWNED, 0.0, 83.0),
                (NEGATIVE_FREE_MASS, 0.0, 95.0),
            ])
        );
    }

    #[test]
    fn you_have_is_the_cash_and_the_count_owned_is_only_in_the_grid() {
        // Two light blasters owned, 5000 credits.
        let mut screen = screen();
        let shown = info_placed(&screen);
        assert!(follows(&shown, "You Have:", "5,000 cr"), "{shown:?}");
        assert!(
            !shown
                .iter()
                .any(|(text, _)| text == "2" || text.starts_with("You Have: ")),
            "{shown:?}"
        );
        assert!(words(&screen).contains(&"2".to_owned()), "in the grid");
        for (cash, amount) in [(0, "0 cr"), (-250, "-250 cr")] {
            let shown = info_lines_of(Outfitter {
                cash,
                ..outfitter()
            });
            assert!(follows(&shown, "You Have:", amount), "{cash}: {shown:?}");
        }
        let mut bought = outfitter();
        bought.rows[0].owned = 3;
        bought.cash = 10;
        screen.set_outfitter(bought);
        let shown = info_placed(&screen);
        assert!(follows(&shown, "You Have:", "10 cr"), "{shown:?}");
    }

    #[test]
    fn every_amount_draws_with_draw_comma_numbers_separators() {
        let one = |price, mass, cash, free_mass| {
            info_lines_of(Outfitter {
                rows: vec![row(128, "Widget", price, mass, 0)],
                cash,
                free_mass,
                ..outfitter()
            })
        };
        let shown = one(12_345, 0, 5000, 5);
        assert!(follows(&shown, PRICE_LABEL, "12,345 cr"), "{shown:?}");
        let shown = one(999, 0, 5000, 5);
        assert!(follows(&shown, PRICE_LABEL, "999 cr"), "{shown:?}");
        let shown = one(100, 0, 1_234_567, 5);
        assert!(follows(&shown, "You Have:", "1.23M cr"), "{shown:?}");
        let shown = one(100, 1_000, 5000, 2_500);
        assert!(follows(&shown, MASS_LABEL, "1,000 tons"), "{shown:?}");
        assert!(follows(&shown, AVAILABLE_LABEL, "2,500 tons"), "{shown:?}");
        let shown = one(100, 1, 5000, 1);
        assert!(follows(&shown, AVAILABLE_LABEL, "1 ton"), "{shown:?}");
    }

    #[test]
    fn the_info_box_says_why_buy_or_sell_is_greyed_when_the_original_words_it() {
        let mut screen = screen();
        press(&mut screen, Key::Right);
        press(&mut screen, Key::Right);
        let shown = words(&screen);
        assert!(shown.contains(&NO_SPACE_FOR_ANY.to_owned()), "{shown:?}");
        assert!(shown.contains(&"9,000 cr".to_owned()));
        assert!(shown.contains(&"10 tons".to_owned()));
        press(&mut screen, Key::Right);
        let shown = words(&screen);
        assert!(
            !shown.iter().any(|text| text.starts_with("Can't")),
            "{shown:?}"
        );
        // Its one owned is its `Max`, and selling it would leave a ton
        // short of the 4 tons free.
        let overloaded = Outfitter {
            rows: vec![OutfitRow {
                max: 1,
                ..row(128, "Expansion", 100, -5, 1)
            }],
            free_mass: 4,
            rules: Rulebook::default()
                .with_override(RuleKey::OutfitCount, RuleSource::Engine)
                .with_override(RuleKey::SaleRefusal, RuleSource::Bible),
            ..outfitter()
        };
        let screen = screen_of(overloaded);
        let info = item(&screen, INFO_ITEM);
        let lines: Vec<_> = texts(&drawn(&screen))
            .into_iter()
            .filter(|(_, at, _)| info.contains(*at))
            .map(|(text, _, _)| text)
            .collect();
        assert_eq!(
            lines,
            [
                "Item Price:",
                "100 cr",
                "You Have:",
                "5,000 cr",
                MAX_OWNED,
                NEGATIVE_FREE_MASS,
            ]
        );
    }

    #[test]
    fn the_item_price_is_what_buying_one_costs() {
        let shown = info_lines(OutfitRow {
            price: 1500,
            buy_price: 1275,
            ..widget(0)
        });
        assert!(follows(&shown, PRICE_LABEL, "1,275 cr"), "{shown:?}");
    }

    /// Widget 128, 100 credits and a ton, `owned` owned.
    fn widget(owned: u16) -> OutfitRow {
        row(128, "Widget", 100, 1, owned)
    }

    /// The info box's texts for `widget`, in [`outfitter`]'s place.
    fn info_lines(widget: OutfitRow) -> Vec<(String, Point)> {
        info_lines_of(Outfitter {
            rows: vec![widget],
            ..outfitter()
        })
    }

    /// The info box's texts for the first row of `outfitter`, each where
    /// it starts from the box's top left.
    fn info_lines_of(outfitter: Outfitter) -> Vec<(String, Point)> {
        info_placed(&screen_of(outfitter))
    }

    /// The point `dx` across and `dy` down from the info box's top left.
    fn info_at(screen: &OutfitterScreen) -> impl Fn(f32, f32) -> Point {
        let info = item(screen, INFO_ITEM).min;
        move |dx, dy| Point::new(info.x + dx, info.y + dy)
    }

    /// The info box's texts, each where it starts from the box's top left,
    /// and its colour.
    fn info_drawn(screen: &OutfitterScreen) -> Vec<(String, Point, Color)> {
        let info = item(screen, INFO_ITEM);
        texts(&drawn(screen))
            .into_iter()
            .filter(|(_, at, _)| info.contains(*at))
            .map(|(text, at, color)| {
                let from = Point::new(at.x - info.min.x, at.y - info.min.y);
                (text, from, color)
            })
            .collect()
    }

    /// The info box's texts, each where it starts from the box's top left.
    fn info_placed(screen: &OutfitterScreen) -> Vec<(String, Point)> {
        info_drawn(screen)
            .into_iter()
            .map(|(text, at, _)| (text, at))
            .collect()
    }

    /// Whether `amount` is drawn next after `label`, on its row 70 right
    /// of it.
    fn follows(lines: &[(String, Point)], label: &str, amount: &str) -> bool {
        lines.windows(2).any(|pair| {
            let [(first, at), (second, then)] = pair else {
                return false;
            };
            first == label && second == amount && *then == Point::new(at.x + 70.0, at.y)
        })
    }

    /// The words on the info box's refusal row, when it says why buy is
    /// greyed: its baseline 93 below the top, at the left.
    fn said_why(lines: &[(String, Point)]) -> Option<&str> {
        lines
            .iter()
            .find(|(_, at)| *at == Point::new(0.0, 83.0))
            .map(|(text, _)| text.as_str())
    }

    #[test]
    fn the_info_box_words_a_max_refusal_by_the_count_owned() {
        // None allowed, one granted.
        let granted = info_lines(OutfitRow {
            max: 0,
            ..widget(1)
        });
        assert_eq!(said_why(&granted), Some(MAX_OWNED), "{granted:?}");
    }

    #[test]
    fn a_fighter_refused_for_full_bays_shows_the_mass_words_when_it_lacks_the_mass() {
        // 6 tons is past the 5 free.
        let bays_full = |mass, owned| {
            let mut fighter = OutfitRow {
                mass,
                ..widget(owned)
            };
            fighter.facts.fighter_room = Some(0);
            info_lines(fighter)
        };
        let none = bays_full(6, 0);
        assert_eq!(said_why(&none), Some(NO_SPACE_FOR_ANY), "{none:?}");
        let one = bays_full(6, 1);
        assert_eq!(said_why(&one), Some(NO_SPACE), "{one:?}");
        for owned in [0, 2] {
            let fits = bays_full(1, owned);
            assert!(fits.iter().any(|(text, _)| text == "Available:"));
            assert_eq!(said_why(&fits), None, "{owned}: {fits:?}");
        }
    }

    #[test]
    fn the_info_box_words_the_rows_words_not_its_buy_refusal() {
        let not_for_sale = |mass, max, owned| {
            let mut row = OutfitRow {
                mass,
                max,
                ..widget(owned)
            };
            row.facts.buy_tech = false;
            info_lines(row)
        };
        let heavy = not_for_sale(6, 10, 0);
        assert_eq!(said_why(&heavy), Some(NO_SPACE_FOR_ANY), "{heavy:?}");
        let at_max = not_for_sale(1, 1, 1);
        assert_eq!(said_why(&at_max), Some(MAX_OWNED), "{at_max:?}");
        // A mass expansion on a ship of negative `Holds`.
        let mut expansion = OutfitRow {
            mass: 6,
            ..widget(1)
        };
        expansion.facts.expansion = Some(5);
        let expansion = info_lines_of(Outfitter {
            rows: vec![expansion],
            holds: -1,
            ..outfitter()
        });
        assert_eq!(said_why(&expansion), Some(NO_SPACE), "{expansion:?}");
        // Refused for the cash: 6000 credits.
        let fits = info_lines(OutfitRow {
            price: 6000,
            buy_price: 6000,
            ..widget(0)
        });
        assert_eq!(said_why(&fits), None, "buy alone gets no words: {fits:?}");
    }

    #[test]
    fn with_nothing_listed_the_grid_says_so_and_both_are_greyed() {
        let art = art();
        let mut screen = screen_with(
            Outfitter {
                rows: Vec::new(),
                ..outfitter()
            },
            &art,
        );
        assert_eq!(screen.selected(), None);
        let grid = item(&screen, GRID_ITEM);
        let shown = texts(&drawn(&screen));
        assert!(shown.contains(&(
            NO_ITEMS.to_owned(),
            Point::new(grid.min.x + INSET, grid.min.y + INSET),
            TEXT_COLOR
        )));
        assert!(!shown.iter().any(|(text, _, _)| text.starts_with("Item")));
        assert_eq!(enabled(&screen), (false, false));
        press(&mut screen, BUY_KEY);
        press(&mut screen, SELL_KEY);
        press(&mut screen, Key::Down);
        assert_eq!(screen.take_order(), None);
        assert_eq!(*art.asked.borrow(), Vec::<String>::new(), "nothing to read");
        assert!(!words(&screen_of(outfitter())).contains(&NO_ITEMS.to_owned()));
    }

    #[test]
    fn buy_and_sell_ask_for_one_of_the_selected_outfit() {
        let mut screen = screen();
        click_item(&mut screen, BUY_ITEM);
        assert_eq!(screen.take_order(), Some(order(128, Direction::Buy)));
        assert_eq!(screen.take_order(), None, "once");
        click_item(&mut screen, SELL_ITEM);
        assert_eq!(screen.take_order(), Some(order(128, Direction::Sell)));
        press(&mut screen, Key::Right);
        press(&mut screen, BUY_KEY);
        assert_eq!(screen.take_order(), Some(order(129, Direction::Buy)));
        screen.input(&key(BUY_KEY, true, true));
        assert_eq!(
            screen.take_order(),
            Some(order(129, Direction::Buy)),
            "repeats too"
        );
        press(&mut screen, Key::Left);
        press(&mut screen, SELL_KEY);
        assert_eq!(screen.take_order(), Some(order(128, Direction::Sell)));
        screen.input(&key(SELL_KEY, true, true));
        assert_eq!(screen.take_order(), Some(order(128, Direction::Sell)));
        screen.input(&key(BUY_KEY, false, false));
        assert_eq!(screen.take_order(), None, "not on release");
    }

    #[test]
    fn buy_and_sell_are_greyed_when_one_could_not_go_that_way() {
        let mut screen = screen();
        assert_eq!(enabled(&screen), (true, true), "the blaster");
        press(&mut screen, Key::Right);
        assert_eq!(enabled(&screen), (true, false), "ammunition, none owned");
        press(&mut screen, Key::Right);
        assert_eq!(enabled(&screen), (false, false), "too heavy, none owned");
        click_item(&mut screen, BUY_ITEM);
        press(&mut screen, BUY_KEY);
        click_item(&mut screen, SELL_ITEM);
        press(&mut screen, SELL_KEY);
        assert_eq!(screen.take_order(), None, "greyed asks for nothing");
    }

    #[test]
    fn a_sell_only_row_can_be_selected_and_sold_but_not_bought() {
        let mut screen = screen();
        click_cell(&mut screen, 3);
        assert_eq!(screen.selected(), Some(3));
        assert_eq!(enabled(&screen), (false, true));
        press(&mut screen, BUY_KEY);
        assert_eq!(screen.take_order(), None);
        press(&mut screen, SELL_KEY);
        assert_eq!(screen.take_order(), Some(order(131, Direction::Sell)));
    }

    #[test]
    fn a_held_key_never_acts_on_an_outfit_it_did_not_start_on() {
        // A sell-only map (1 owned) next to a sellable blaster (2 owned).
        let both = || Outfitter {
            rows: vec![map(), row(128, "Light\\nBlaster", 1000, 1, 2)],
            ..outfitter()
        };
        let mut screen = screen_of(both());
        screen.input(&key(SELL_KEY, true, false));
        assert_eq!(screen.take_order(), Some(order(131, Direction::Sell)));
        // Sold, the map's row is gone: the blaster takes its cell.
        let mut sold = both();
        sold.rows.remove(0);
        screen.set_outfitter(sold);
        assert_eq!(screen.selected(), Some(0));
        for _ in 0..3 {
            screen.input(&key(SELL_KEY, true, true));
            assert_eq!(screen.take_order(), None, "the key was held for the map");
        }
        screen.input(&key(SELL_KEY, false, false));
        screen.input(&key(SELL_KEY, true, false));
        assert_eq!(
            screen.take_order(),
            Some(order(128, Direction::Sell)),
            "a fresh press acts on the new selection"
        );
        screen.input(&key(SELL_KEY, true, true));
        assert_eq!(
            screen.take_order(),
            Some(order(128, Direction::Sell)),
            "and its repeats too, while the blaster stays selected"
        );
    }

    #[test]
    fn a_repeat_acts_only_for_the_key_pressed_on_the_outfit_selected() {
        let mut screen = screen();
        screen.input(&key(BUY_KEY, true, true));
        assert_eq!(screen.take_order(), None, "no press before it");
        screen.input(&key(BUY_KEY, true, false));
        assert_eq!(screen.take_order(), Some(order(128, Direction::Buy)));
        screen.input(&key(SELL_KEY, true, true));
        assert_eq!(screen.take_order(), None, "S was never pressed");
        press(&mut screen, Key::Right);
        screen.input(&key(BUY_KEY, true, true));
        assert_eq!(screen.take_order(), None, "pressed on another outfit");
        press(&mut screen, Key::Left);
        screen.input(&key(BUY_KEY, true, true));
        assert_eq!(
            screen.take_order(),
            Some(order(128, Direction::Buy)),
            "back on the outfit it was pressed on"
        );
    }

    #[test]
    fn a_new_outfitter_shows_and_regreys_keeping_the_outfit_selected() {
        let mut screen = screen();
        press(&mut screen, Key::Right);
        let mut bought = outfitter();
        bought.rows[1].owned = 3;
        bought.cash = 10;
        screen.set_outfitter(bought.clone());
        assert_eq!(screen.outfitter(), &bought);
        assert_eq!(screen.selected(), Some(1));
        assert_eq!(enabled(&screen), (false, true));
        assert!(follows(&info_placed(&screen), "You Have:", "10 cr"));
        // Sold out of a sell-only map: the next cell along is selected.
        click_cell(&mut screen, 3);
        let mut sold = outfitter();
        sold.rows.remove(3);
        sold.rows.insert(0, row(127, "First", 1, 1, 0));
        screen.set_outfitter(sold.clone());
        assert_eq!(screen.selected(), Some(3), "the map's place, now the gun");
        sold.rows.remove(3);
        screen.set_outfitter(sold);
        assert_eq!(screen.selected(), Some(2), "clamped");
        let mut kept = screen_of(long(10));
        press(&mut kept, Key::Right);
        let mut moved = long(10);
        moved.rows.rotate_right(1);
        kept.set_outfitter(moved);
        assert_eq!(kept.selected(), Some(2), "followed");
        kept.set_outfitter(long(0));
        assert_eq!(kept.selected(), None);
        kept.set_outfitter(long(5));
        assert_eq!(kept.selected(), Some(0));
    }

    // After an order that changes the listing
    // (`RuleKey::RelistedSelection`).

    const BLASTER: OutfitId = OutfitId(128);

    /// `screen` after an order that reset the selection to the blaster.
    fn reset(mut screen: OutfitterScreen) -> OutfitterScreen {
        screen.reselect(Reselection::Reset(BLASTER));
        screen
    }

    /// Whether any cell shown is highlighted.
    fn any_highlight(screen: &OutfitterScreen) -> bool {
        let commands = drawn(screen);
        (0..COLUMNS * GRID_ROWS)
            .filter(|&index| screen.cell_bounds(index).is_some())
            .any(|index| contains(&commands, &bright_frame(screen, index)))
    }

    /// Whether outfit `outfit` can go `direction` by `screen`'s outfitter.
    fn can(screen: &OutfitterScreen, outfit: OutfitId, direction: Direction) -> bool {
        screen
            .outfitter()
            .can(OutfitOrder { outfit, direction })
            .is_ok()
    }

    #[test]
    fn after_a_reset_no_cell_is_highlighted_and_buy_and_sell_answer_for_the_outfit() {
        let mut screen = screen();
        press(&mut screen, Key::Right);
        assert!(any_highlight(&screen));
        let mut screen = reset(screen);
        assert_eq!(screen.selected(), None);
        assert_eq!(screen.selected_outfit(), Some(BLASTER));
        assert_eq!(
            enabled(&screen),
            (
                can(&screen, BLASTER, Direction::Buy),
                can(&screen, BLASTER, Direction::Sell)
            )
        );
        assert_eq!(enabled(&screen), (true, true));
        assert!(!any_highlight(&screen));
        press(&mut screen, BUY_KEY);
        assert_eq!(screen.take_order(), Some(order(128, Direction::Buy)));
        press(&mut screen, SELL_KEY);
        assert_eq!(screen.take_order(), Some(order(128, Direction::Sell)));
        click_item(&mut screen, BUY_ITEM);
        assert_eq!(screen.take_order(), Some(order(128, Direction::Buy)));
        assert_eq!(screen.selected(), None, "still no cell");
    }

    #[test]
    fn after_a_reset_the_picture_description_and_info_box_are_blank() {
        let screen = screen();
        let before = words(&screen);
        assert!(before.contains(&"dësc 3000.".to_owned()), "{before:?}");
        assert!(before.contains(&credits::YOU_HAVE.to_owned()));
        let pictured = |screen: &OutfitterScreen| {
            let area = item(screen, PICTURE_ITEM);
            drawn(screen).iter().any(|command| {
                matches!(command, DrawCommand::StretchedPicture { top_left, .. } if *top_left == area.min)
            })
        };
        assert!(pictured(&screen));
        let screen = reset(screen);
        let shown = words(&screen);
        for blank in [
            PRICE_LABEL,
            credits::YOU_HAVE,
            MASS_LABEL,
            AVAILABLE_LABEL,
            NO_PICTURE,
        ] {
            assert!(!shown.contains(&blank.to_owned()), "{blank}: {shown:?}");
        }
        assert!(!shown.iter().any(|text| text.starts_with("dësc")));
        assert!(!pictured(&screen));
    }

    /// An outfitter listing only the ammunition, with the blaster (2
    /// owned) not listed.
    fn unlisted_blaster() -> Outfitter {
        let mut blaster = row(128, "Light\\nBlaster", 1000, 1, 2);
        blaster.facts.listed = false;
        Outfitter {
            rows: vec![row(129, "*Ammo\\nPack", 50, 0, 0)],
            unlisted: Some(blaster),
            ..outfitter()
        }
    }

    #[test]
    fn a_reset_outfit_need_not_be_listed() {
        let mut screen = reset(screen_of(unlisted_blaster()));
        assert_eq!(screen.selected_outfit(), Some(BLASTER));
        assert_eq!(enabled(&screen), (true, true));
        press(&mut screen, SELL_KEY);
        assert_eq!(screen.take_order(), Some(order(128, Direction::Sell)));
        let commands = drawn(&screen);
        assert!(contains(&commands, &frame(&screen, 0)));
        assert!(!contains(&commands, &frame(&screen, 1)), "one cell");
        press(&mut screen, COUNT_KEY);
        press(&mut screen, SELL_KEY);
        assert_eq!(field_text(&screen), Some("2"), "the blaster's count");
    }

    #[test]
    fn a_reset_outfit_with_no_row_greys_both() {
        let mut screen = reset(screen_of(long(3)));
        assert_eq!(screen.selected_outfit(), Some(BLASTER));
        assert_eq!(enabled(&screen), (false, false));
        press(&mut screen, BUY_KEY);
        press(&mut screen, SELL_KEY);
        assert_eq!(screen.take_order(), None);
    }

    #[test]
    fn a_reset_scrolls_the_grid_to_its_top() {
        let mut screen = screen_of(long(30));
        for _ in 0..6 {
            press(&mut screen, Key::Down);
        }
        assert_eq!(screen.top_row(), 2);
        let screen = reset(screen);
        assert_eq!(screen.top_row(), 0);
        assert_eq!(names(&screen)[0], "Good 0");
    }

    #[test]
    fn after_a_reset_the_grid_reports_nothing_highlighted_until_an_arrow_picks_a_shown_cell() {
        for (k, picked) in [(Key::Right, 0), (Key::Left, 9)] {
            let mut screen = reset(screen_of(long(10)));
            assert!(!screen.grid.highlighted(), "{k:?}: the grid's own bit");
            assert_eq!(screen.grid.selected(10), None, "{k:?}");
            press(&mut screen, k);
            assert!(screen.grid.highlighted(), "{k:?}");
            assert_eq!(screen.grid.selected(10), Some(picked), "{k:?}");
            assert_eq!(screen.selected(), Some(picked), "{k:?}");
        }
    }

    #[test]
    fn a_click_or_an_arrow_ends_the_reset() {
        let mut clicked = reset(screen());
        click_cell(&mut clicked, 2);
        assert_eq!(clicked.selected(), Some(2));
        assert_eq!(clicked.selected_outfit(), Some(OutfitId(130)));
        assert!(contains(&drawn(&clicked), &bright_frame(&clicked, 2)));
        let mut empty = reset(screen());
        click_cell(&mut empty, 4);
        assert_eq!(empty.selected(), None, "an empty cell");
        assert_eq!(empty.selected_outfit(), None, "the blaster let go");
        let arrowed = |outfitter: Outfitter, scrolled: usize, k: Key| {
            let mut screen = reset(screen_of(outfitter));
            for _ in 0..scrolled {
                click_item(&mut screen, SCROLL_DOWN_ITEM);
            }
            press(&mut screen, k);
            (
                screen.selected(),
                screen.top_row(),
                screen.selected_outfit(),
            )
        };
        let at = |index: usize, top_row: usize| {
            (
                Some(index),
                top_row,
                Some(OutfitId(200 + i16::try_from(index).expect("few"))),
            )
        };
        for (k, first_or_last) in [
            (Key::Right, 0),
            (Key::Down, 0),
            (Key::PageDown, 0),
            (Key::Left, 19),
            (Key::Up, 19),
            (Key::PageUp, 19),
        ] {
            assert_eq!(arrowed(long(30), 0, k), at(first_or_last, 0), "{k:?}");
            assert_eq!(
                arrowed(long(30), 2, k),
                at(first_or_last + 8, 2),
                "{k:?} scrolled"
            );
        }
        for k in [Key::Left, Key::Up] {
            assert_eq!(arrowed(long(10), 0, k), at(9, 0), "{k:?}: the last");
            assert_eq!(
                arrowed(long(22), 1, k),
                at(21, 1),
                "{k:?}: the last, scrolled"
            );
        }
        let mut ended = reset(screen_of(long(30)));
        press(&mut ended, Key::Right);
        press(&mut ended, Key::Right);
        assert_eq!(ended.selected(), Some(1), "the arrows move as before");
    }

    // The grid's presses and arrows (`RuleKey::GridSelection`).

    /// `outfitter` with the grid's selection read the other way, the
    /// port's earlier behaviour.
    fn by_the_other_grid_reading(outfitter: Outfitter) -> Outfitter {
        Outfitter {
            rules: outfitter
                .rules
                .with_override(RuleKey::GridSelection, RuleSource::Bible),
            ..outfitter
        }
    }

    /// Whether an outfit's picture is drawn in the picture item, and
    /// which.
    fn picture_drawn(screen: &OutfitterScreen) -> Option<ImageKey> {
        let area = item(screen, PICTURE_ITEM);
        drawn(screen).iter().find_map(|command| match command {
            DrawCommand::StretchedPicture {
                image, top_left, ..
            } if *top_left == area.min => Some(*image),
            _ => None,
        })
    }

    /// Whether any description is drawn.
    fn described(screen: &OutfitterScreen) -> bool {
        words(screen).iter().any(|text| text.starts_with("dësc"))
    }

    /// Whether the info box draws any of its lines.
    fn any_info(screen: &OutfitterScreen) -> bool {
        let shown = words(screen);
        [PRICE_LABEL, credits::YOU_HAVE, MASS_LABEL, AVAILABLE_LABEL]
            .iter()
            .any(|label| shown.contains(&(*label).to_owned()))
    }

    #[test]
    fn by_the_engine_a_click_on_an_empty_cell_leaves_nothing_selected() {
        let mut screen = screen();
        click_cell(&mut screen, 1);
        assert!(any_info(&screen) && described(&screen));
        assert!(words(&screen).contains(&NO_PICTURE.to_owned()));
        click_cell(&mut screen, 4);
        assert_eq!(screen.take_sounds(), [], "the press reaches no button");
        assert_eq!(screen.selected(), None);
        assert_eq!(screen.selected_outfit(), None);
        assert_eq!(enabled(&screen), (false, false));
        assert!(!any_highlight(&screen));
        assert!(!any_info(&screen), "{:?}", words(&screen));
        assert!(!words(&screen).contains(&NO_PICTURE.to_owned()));
        assert_eq!(picture_drawn(&screen), None);
        assert!(!described(&screen));
        press(&mut screen, BUY_KEY);
        press(&mut screen, SELL_KEY);
        click_item(&mut screen, BUY_ITEM);
        click_item(&mut screen, SELL_ITEM);
        press(&mut screen, COUNT_KEY);
        press(&mut screen, BUY_KEY);
        press(&mut screen, SELL_KEY);
        assert_eq!(screen.take_order(), None);
        assert!(screen.quantity().is_none());
    }

    #[test]
    fn by_the_engine_a_click_on_an_empty_cell_after_a_reset_lets_the_outfit_go() {
        let mut screen = reset(screen());
        assert_eq!(enabled(&screen), (true, true));
        click_cell(&mut screen, 4);
        assert_eq!(screen.selected_outfit(), None);
        assert_eq!(enabled(&screen), (false, false));
        press(&mut screen, SELL_KEY);
        assert_eq!(screen.take_order(), None);
    }

    #[test]
    fn a_click_on_the_highlighted_cell_changes_nothing() {
        for outfitter in [outfitter(), by_the_other_grid_reading(outfitter())] {
            let art = art();
            let mut screen = screen_with(outfitter, &art);
            click_cell(&mut screen, 1);
            let asked = art.asked.borrow().len();
            let before = drawn(&screen);
            click_cell(&mut screen, 1);
            assert_eq!(art.asked.borrow().len(), asked, "no description read");
            assert_eq!(drawn(&screen), before);
            assert_eq!(screen.selected(), Some(1));
        }
    }

    #[test]
    fn after_an_empty_cell_an_arrow_picks_the_first_or_last_cell_shown() {
        let arrowed = |outfitter: Outfitter, scrolled: usize, empty: usize, k: Key| {
            let art = art();
            let mut screen = screen_with(outfitter, &art);
            for _ in 0..scrolled {
                click_item(&mut screen, SCROLL_DOWN_ITEM);
            }
            click_cell(&mut screen, empty);
            assert_eq!(screen.selected_outfit(), None, "{k:?}");
            art.asked.borrow_mut().clear();
            press(&mut screen, k);
            assert!(described(&screen), "{k:?}");
            let asked = art.asked.borrow().clone();
            (screen.selected(), screen.top_row(), asked)
        };
        let at = |index: usize, top_row: usize| {
            let description = 3072 + i16::try_from(index).expect("few");
            (Some(index), top_row, vec![format!("dësc {description}")])
        };
        for k in [Key::Right, Key::Down] {
            assert_eq!(arrowed(long(10), 0, 12, k), at(0, 0), "{k:?}");
            assert_eq!(arrowed(long(22), 1, 19, k), at(4, 1), "{k:?} scrolled");
        }
        for k in [Key::Left, Key::Up] {
            assert_eq!(arrowed(long(10), 0, 12, k), at(9, 0), "{k:?}");
            assert_eq!(arrowed(long(22), 1, 19, k), at(21, 1), "{k:?} scrolled");
        }
    }

    #[test]
    fn each_description_read_is_asked_to_load_and_shows_loaded() {
        let mut screen = screen();
        assert_eq!(
            screen.take_description_request(),
            Some(3000),
            "the first cell's"
        );
        assert_eq!(screen.take_description_request(), None, "once");
        screen.show_description(3000, "Loaded blaster.");
        assert!(words(&screen).contains(&"Loaded blaster.".to_owned()));
        press(&mut screen, Key::Right);
        assert_eq!(screen.take_description_request(), Some(3001));
        screen.show_description(3000, "Too late.");
        assert!(
            !words(&screen).contains(&"Too late.".to_owned()),
            "another outfit's is not shown"
        );
        assert!(words(&screen).contains(&"dësc 3001.".to_owned()));
        assert_eq!(outfit_desc_id(OutfitId(130)), 3002);
    }

    #[test]
    fn by_the_engine_an_arrow_onto_the_reset_outfit_asks_no_load() {
        let mut screen = reset(screen());
        let _ = screen.take_description_request();
        press(&mut screen, Key::Right);
        assert_eq!(screen.take_description_request(), None, "@0x58442-0x5844f");
    }

    #[test]
    fn by_the_engine_an_arrow_onto_the_reset_outfit_reads_no_description() {
        let art = art();
        let mut screen = reset(screen_with(outfitter(), &art));
        art.asked.borrow_mut().clear();
        press(&mut screen, Key::Right);
        assert_eq!(screen.selected(), Some(0));
        assert_eq!(screen.selected_outfit(), Some(BLASTER));
        assert!(contains(&drawn(&screen), &bright_frame(&screen, 0)));
        assert_eq!(picture_drawn(&screen), Some(ImageKey::picture(6000)));
        assert!(any_info(&screen));
        assert!(follows(&info_placed(&screen), PRICE_LABEL, "1,000 cr"));
        assert_eq!(enabled(&screen), (true, true));
        assert!(!described(&screen), "{:?}", words(&screen));
        assert_eq!(*art.asked.borrow(), Vec::<String>::new());
        press(&mut screen, Key::Right);
        assert_eq!(*art.asked.borrow(), ["dësc 3001"]);
        press(&mut screen, Key::Left);
        assert_eq!(*art.asked.borrow(), ["dësc 3001", "dësc 3000"]);
        assert!(words(&screen).contains(&"dësc 3000.".to_owned()));
    }

    #[test]
    fn by_the_engine_a_click_on_the_reset_outfits_cell_reads_its_description() {
        let mut screen = reset(screen());
        click_cell(&mut screen, 0);
        assert_eq!(screen.selected(), Some(0));
        assert!(words(&screen).contains(&"dësc 3000.".to_owned()));
    }

    #[test]
    fn by_the_other_reading_an_arrow_onto_the_reset_outfit_reads_its_description() {
        let mut screen = reset(screen_of(by_the_other_grid_reading(outfitter())));
        press(&mut screen, Key::Right);
        assert_eq!(screen.selected(), Some(0));
        assert!(words(&screen).contains(&"dësc 3000.".to_owned()));
    }

    #[test]
    fn by_the_other_reading_a_click_on_an_empty_cell_keeps_a_reset() {
        let mut screen = reset(screen_of(by_the_other_grid_reading(outfitter())));
        click_cell(&mut screen, 4);
        assert_eq!(screen.selected(), None);
        assert_eq!(screen.selected_outfit(), Some(BLASTER));
        assert_eq!(
            enabled(&screen),
            (
                can(&screen, BLASTER, Direction::Buy),
                can(&screen, BLASTER, Direction::Sell)
            )
        );
        press(&mut screen, SELL_KEY);
        assert_eq!(screen.take_order(), Some(order(128, Direction::Sell)));
    }

    #[test]
    fn a_new_outfitter_keeps_nothing_selected() {
        let mut screen = screen();
        click_cell(&mut screen, 4);
        screen.set_outfitter(outfitter());
        assert_eq!(screen.selected(), None);
        assert_eq!(screen.selected_outfit(), None);
        assert_eq!(enabled(&screen), (false, false));
        assert!(!any_highlight(&screen));
    }

    #[test]
    fn a_key_held_on_another_outfit_stops_when_a_reset_selects_128() {
        let mut screen = screen();
        click_cell(&mut screen, 3);
        press(&mut screen, SELL_KEY);
        assert_eq!(screen.take_order(), Some(order(131, Direction::Sell)));
        screen.reselect(Reselection::Reset(BLASTER));
        screen.set_outfitter(outfitter());
        for _ in 0..2 {
            screen.input(&key(SELL_KEY, true, true));
            assert_eq!(screen.take_order(), None, "the held key stops");
        }
        screen.input(&key(SELL_KEY, false, false));
        press(&mut screen, SELL_KEY);
        assert_eq!(screen.take_order(), Some(order(128, Direction::Sell)));
    }

    #[test]
    fn a_key_held_on_128_goes_on_when_a_reset_selects_128_again() {
        // The original has no held-key guard: `_OutfitFilter` asks Buy and
        // Sell of `*0xded58` on every event, repeats included
        // (@0x5806f-0x580ca, @0x5817a-0x581ab), so a held S keeps selling
        // `oütf` 128 when a reset selects it again. The port stops a held
        // key only when the outfit it answers for changes, and here it
        // does not.
        let mut screen = screen();
        press(&mut screen, SELL_KEY);
        assert_eq!(screen.take_order(), Some(order(128, Direction::Sell)));
        screen.reselect(Reselection::Reset(BLASTER));
        screen.set_outfitter(outfitter());
        assert_eq!(screen.selected(), None);
        screen.input(&key(SELL_KEY, true, true));
        assert_eq!(screen.take_order(), Some(order(128, Direction::Sell)));
        // Pressed during the reset, it goes on once an arrow picks 128's
        // cell: the outfit is the same.
        let mut reset = reset(screen_of(outfitter()));
        press(&mut reset, SELL_KEY);
        assert_eq!(reset.take_order(), Some(order(128, Direction::Sell)));
        press(&mut reset, Key::Right);
        assert_eq!(reset.selected(), Some(0));
        reset.input(&key(SELL_KEY, true, true));
        assert_eq!(reset.take_order(), Some(order(128, Direction::Sell)));
    }

    #[test]
    fn a_new_outfitter_keeps_the_reset() {
        let mut screen = reset(screen());
        let mut broke = outfitter();
        broke.cash = 10;
        screen.set_outfitter(broke.clone());
        assert_eq!(screen.outfitter(), &broke);
        assert_eq!(screen.selected(), None);
        assert_eq!(screen.selected_outfit(), Some(BLASTER));
        assert_eq!(enabled(&screen), (false, true), "regreyed");
        assert!(!any_highlight(&screen));
        // A scroll ends the reset by the engine; the other reading keeps it.
        let mut scrolled = reset(screen_of(by_the_other_grid_reading(long(30))));
        click_item(&mut scrolled, SCROLL_DOWN_ITEM);
        scrolled.set_outfitter(by_the_other_grid_reading(unlisted_blaster()));
        assert_eq!(scrolled.top_row(), 1, "the scroll kept");
        assert_eq!(scrolled.selected(), None);
        assert_eq!(enabled(&scrolled), (true, true), "the unlisted blaster");
        let mut dropped = reset(screen_of(long(30)));
        click_item(&mut dropped, SCROLL_DOWN_ITEM);
        dropped.set_outfitter(long(30));
        assert_eq!(dropped.top_row(), 1, "the scroll kept");
        assert_eq!(
            (dropped.selected(), dropped.selected_outfit()),
            (None, None)
        );
        assert_eq!(enabled(&dropped), (false, false));
    }

    #[test]
    fn an_order_that_keeps_the_listing_keeps_the_selection() {
        let mut screen = screen();
        press(&mut screen, Key::Right);
        let before = drawn(&screen);
        screen.reselect(Reselection::Kept);
        assert_eq!(screen.selected(), Some(1));
        assert_eq!(screen.selected_outfit(), Some(OutfitId(129)));
        assert_eq!(drawn(&screen), before);
        assert!(!screen.closed());
    }

    #[test]
    fn leave_closes_the_outfitter_once_its_alert_is_closed() {
        let mut screen = screen();
        assert!(!screen.closed());
        screen.reselect(Reselection::Leave);
        assert!(screen.closed());
        let mut alerted = alerting(OutfitRefusal::NegativeFreeMass);
        alerted.reselect(Reselection::Leave);
        assert!(!alerted.closed(), "the alert first");
        press(&mut alerted, Key::Escape);
        assert!(alerted.alert_dialog().is_none());
        assert!(alerted.closed());
    }

    #[test]
    fn done_return_and_escape_close_it() {
        let mut screen = screen();
        click_item(&mut screen, DONE_ITEM);
        assert!(screen.closed());
        for k in [Key::Enter, Key::Escape] {
            let mut screen = screen_of(outfitter());
            assert_eq!(screen.input(&key(k, true, false)), ScreenAction::None);
            assert!(screen.closed(), "{k:?}");
            assert_eq!(screen.take_order(), None);
        }
        let mut held = screen_of(outfitter());
        held.input(&key(Key::Escape, true, true));
        held.input(&key(Key::Space, true, false));
        held.tick(Duration::from_secs(1));
        assert!(!held.closed());
    }

    /// `_TrackOutfitButtons` @0x2a029 plays nothing under the pointer.
    #[test]
    fn pointer_click_is_silent_as_the_original() {
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
        let debug = format!("{screen:?}");
        assert!(debug.contains("TextMetrics"), "{debug}");
        assert!(debug.contains("OutfitterCatalog"), "{debug}");
    }

    fn problem(reason: &str) -> DrawCommand {
        DrawCommand::Text {
            text: format!(
                "Cannot show the outfitter: {reason}. Press Return or Escape to close it."
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
            let art = art();
            let mut screen = OutfitterScreen::new(
                Err("no DLOG 1002".to_owned()),
                Ok(quantity()),
                Ok(desc_template()),
                outfitter(),
                Rc::clone(&art) as Rc<dyn OutfitterCatalog>,
                ButtonStyle::STOCK,
            );
            assert_eq!(screen.problem(), Some("no DLOG 1002"));
            assert!(screen.dialog().is_none());
            assert_eq!(screen.cell_bounds(0), None);
            assert_eq!(drawn(&screen), [problem("no DLOG 1002")]);
            for other in [
                key(k, true, true),
                key(k, false, false),
                key(BUY_KEY, true, false),
                key(Key::Down, true, false),
            ] {
                screen.input(&other);
            }
            click(&mut screen, Point::new(500.0, 500.0));
            screen.cancel_pointer();
            assert!(!screen.closed());
            assert_eq!(screen.take_order(), None);
            assert_eq!(*art.asked.borrow(), Vec::<String>::new());
            screen.input(&key(k, true, false));
            assert!(screen.closed(), "{k:?}");
        }
    }

    // The quantity dialog, by the engine's count reading.

    /// The outfitter with 3000 credits: the light blaster's buy opens at
    /// 3, and its sale at the 2 owned.
    fn counting() -> Outfitter {
        Outfitter {
            cash: 3000,
            ..outfitter()
        }
    }

    fn counting_screen() -> OutfitterScreen {
        screen_of(counting())
    }

    fn quantity_item(screen: &OutfitterScreen, number: usize) -> Point {
        screen
            .quantity()
            .expect("asking")
            .dialog()
            .item_bounds(number)
            .expect("an item")
            .center()
    }

    fn quantity_texts(screen: &OutfitterScreen) -> Vec<String> {
        let mut list = DrawList::new();
        screen.quantity().expect("asking").draw(&mut list);
        let commands: Vec<DrawCommand> = list.iter().cloned().collect();
        texts(&commands)
            .into_iter()
            .map(|(text, _, _)| text)
            .collect()
    }

    fn field_text(screen: &OutfitterScreen) -> Option<&str> {
        screen.quantity().map(|asking| asking.field().text())
    }

    fn typed(screen: &mut OutfitterScreen, text: &str) {
        for c in text.chars() {
            screen.input(&key(Key::Char(c), true, false));
            screen.input(&Input::Text(c));
        }
    }

    /// Clicks `button` with Alt held, then lets Alt go.
    fn alt_click(screen: &mut OutfitterScreen, button: usize) {
        press(screen, COUNT_KEY);
        click_item(screen, button);
        screen.input(&key(COUNT_KEY, false, false));
    }

    #[test]
    fn with_alt_held_buy_opens_the_quantity_dialog_at_the_engines_maximum_titled_buy() {
        assert_eq!(COUNT_KEY, Key::Alt);
        let mut screen = counting_screen();
        assert!(screen.quantity().is_none());
        alt_click(&mut screen, BUY_ITEM);
        assert_eq!(screen.take_order(), None, "nothing yet");
        let asking = screen.quantity().expect("asking");
        assert_eq!(asking.field().text(), "3", "trunc(3000 / 1000)");
        assert_eq!(asking.field().selection(), 1);
        assert_eq!(
            asking.dialog().bounds().min,
            Point::new(10.0, 20.0),
            "laid out by its template"
        );
        assert_eq!(
            quantity_texts(&screen),
            ["Buy", "Enter quantity:", "Cancel", "3"]
        );
    }

    #[test]
    fn alt_s_opens_a_sell_dialog_at_the_count_owned_and_its_s_types_nothing() {
        let mut screen = counting_screen();
        press(&mut screen, COUNT_KEY);
        // As a Mac sends Option-S: the key, then the character it types.
        press(&mut screen, SELL_KEY);
        screen.input(&Input::Text('ß'));
        assert_eq!(
            quantity_texts(&screen),
            ["Sell", "Enter quantity:", "Cancel", "2"]
        );
        screen.input(&Input::Text('1'));
        assert_eq!(
            field_text(&screen),
            Some("1"),
            "only the first typed character is dropped"
        );
        // A click flushes nothing.
        let mut screen = counting_screen();
        alt_click(&mut screen, SELL_ITEM);
        screen.input(&Input::Text('1'));
        assert_eq!(field_text(&screen), Some("1"));
    }

    #[test]
    fn ok_records_the_count_confirmed() {
        let mut screen = counting_screen();
        alt_click(&mut screen, BUY_ITEM);
        typed(&mut screen, "2");
        let ok = quantity_item(&screen, 1);
        click(&mut screen, ok);
        assert_eq!(screen.take_order(), Some(counted(128, Direction::Buy, 2)));
        assert!(screen.quantity().is_none(), "closed");
        alt_click(&mut screen, BUY_ITEM);
        press(&mut screen, Key::Enter);
        assert_eq!(
            screen.take_order(),
            Some(counted(128, Direction::Buy, 3)),
            "the maximum untouched"
        );
        assert!(screen.quantity().is_none());
        click_item(&mut screen, BUY_ITEM);
        assert_eq!(screen.take_order(), Some(order(128, Direction::Buy)));
    }

    #[test]
    fn cancel_or_0_records_an_order_of_none_and_closes_the_dialog() {
        let mut screen = counting_screen();
        alt_click(&mut screen, SELL_ITEM);
        let cancel = quantity_item(&screen, 4);
        click(&mut screen, cancel);
        assert_eq!(screen.take_order(), Some(counted(128, Direction::Sell, 0)));
        assert!(screen.quantity().is_none(), "closed");
        alt_click(&mut screen, BUY_ITEM);
        typed(&mut screen, "0");
        press(&mut screen, Key::Enter);
        assert_eq!(screen.take_order(), Some(counted(128, Direction::Buy, 0)));
        assert!(screen.quantity().is_none(), "closed");
        assert!(!screen.closed());
    }

    #[test]
    fn alt_on_a_row_whose_maximum_is_none_records_one_and_opens_nothing() {
        // The map: one owned.
        let mut screen = counting_screen();
        click_cell(&mut screen, 3);
        alt_click(&mut screen, SELL_ITEM);
        assert!(screen.quantity().is_none());
        assert_eq!(screen.take_order(), Some(order(131, Direction::Sell)));
        // The big gun, which cannot be bought: nothing at all.
        click_cell(&mut screen, 2);
        alt_click(&mut screen, BUY_ITEM);
        assert!(screen.quantity().is_none());
        assert_eq!(screen.take_order(), None);
        // Cash for one light blaster.
        let mut screen = screen_of(Outfitter {
            cash: 1999,
            ..outfitter()
        });
        alt_click(&mut screen, BUY_ITEM);
        assert!(screen.quantity().is_none());
        assert_eq!(screen.take_order(), Some(order(128, Direction::Buy)));
    }

    #[test]
    fn with_shift_control_or_command_held_buy_records_one() {
        for modifier in [Key::Shift, Key::Control, Key::Command] {
            let mut screen = counting_screen();
            press(&mut screen, modifier);
            click_item(&mut screen, BUY_ITEM);
            assert_eq!(
                screen.take_order(),
                Some(order(128, Direction::Buy)),
                "{modifier:?}"
            );
            press(&mut screen, SELL_KEY);
            assert_eq!(screen.take_order(), Some(order(128, Direction::Sell)));
            assert!(screen.quantity().is_none());
        }
    }

    #[test]
    fn by_the_other_count_reading_alt_records_one_and_never_opens_the_dialog() {
        let mut screen = screen_of(Outfitter {
            rules: Rulebook::default().with_override(RuleKey::OutfitCount, RuleSource::Bible),
            ..counting()
        });
        alt_click(&mut screen, BUY_ITEM);
        assert!(screen.quantity().is_none());
        assert_eq!(screen.take_order(), Some(order(128, Direction::Buy)));
        press(&mut screen, COUNT_KEY);
        press(&mut screen, SELL_KEY);
        assert!(screen.quantity().is_none());
        assert_eq!(screen.take_order(), Some(order(128, Direction::Sell)));
    }

    #[test]
    fn while_the_dialog_is_open_b_escape_and_clicks_do_not_reach_the_outfitter() {
        let mut screen = counting_screen();
        alt_click(&mut screen, BUY_ITEM);
        for k in [BUY_KEY, SELL_KEY, Key::Escape, Key::Right, Key::Down] {
            press(&mut screen, k);
        }
        click_item(&mut screen, DONE_ITEM);
        click_item(&mut screen, SELL_ITEM);
        let cell = screen.cell_bounds(1).expect("a cell").center();
        click(&mut screen, cell);
        assert_eq!(screen.take_order(), None);
        assert!(!screen.closed());
        assert_eq!(screen.selected(), Some(0));
        assert_eq!(field_text(&screen), Some("3"), "still open, untouched");
    }

    #[test]
    fn without_its_template_the_dialog_opens_with_the_fallback() {
        let unusable = DialogTemplate {
            items: Vec::new(),
            ..quantity()
        };
        for template in [Err("no DLOG 1003".to_owned()), Ok(unusable)] {
            let mut screen = OutfitterScreen::new(
                Ok(layout()),
                template,
                Ok(desc_template()),
                counting(),
                art(),
                ButtonStyle::STOCK,
            );
            click_cell(&mut screen, 0);
            alt_click(&mut screen, BUY_ITEM);
            let asking = screen.quantity().expect("asking");
            assert_eq!(
                asking.dialog().bounds().min,
                Point::new((1024.0 - 172.0) / 2.0, (768.0 - 72.0) / 2.0),
                "centred"
            );
            assert_eq!(
                quantity_texts(&screen),
                ["Buy", "Enter quantity:", "Cancel", "3"]
            );
        }
    }

    #[test]
    fn the_dialog_is_drawn_over_the_outfitter_and_its_alert_comes_out() {
        let mut screen = counting_screen();
        let under = drawn(&screen);
        alt_click(&mut screen, BUY_ITEM);
        screen.take_sounds();
        let mut list = DrawList::new();
        screen.quantity().expect("asking").draw(&mut list);
        let mut expected = under;
        expected.extend(list.iter().cloned());
        assert_eq!(drawn(&screen), expected);
        typed(&mut screen, "9");
        press(&mut screen, Key::Enter);
        assert_eq!(screen.take_sounds(), [Sound::Ui(UiSound::Alert)]);
        assert_eq!(field_text(&screen), Some("3"), "reset to the maximum");
        let ok = quantity_item(&screen, 1);
        click(&mut screen, ok);
        assert_eq!(screen.take_sounds(), []);
        assert_eq!(screen.take_order(), Some(counted(128, Direction::Buy, 3)));
    }

    #[test]
    fn letting_go_of_the_keys_lets_go_of_alt() {
        let mut screen = counting_screen();
        press(&mut screen, COUNT_KEY);
        screen.release_keys();
        click_item(&mut screen, BUY_ITEM);
        assert!(screen.quantity().is_none());
        assert_eq!(screen.take_order(), Some(order(128, Direction::Buy)));
        // Let go while the dialog is open.
        press(&mut screen, COUNT_KEY);
        click_item(&mut screen, BUY_ITEM);
        screen.release_keys();
        let cancel = quantity_item(&screen, 4);
        click(&mut screen, cancel);
        click_item(&mut screen, BUY_ITEM);
        assert!(screen.quantity().is_none());
        assert_eq!(screen.take_order(), Some(order(128, Direction::Buy)));
    }

    #[test]
    fn alt_is_followed_while_the_dialog_is_open() {
        let mut screen = counting_screen();
        alt_click(&mut screen, BUY_ITEM);
        press(&mut screen, COUNT_KEY);
        let cancel = quantity_item(&screen, 4);
        click(&mut screen, cancel);
        click_item(&mut screen, BUY_ITEM);
        assert!(screen.quantity().is_some(), "Alt went down in the dialog");
        screen.input(&key(COUNT_KEY, false, false));
        let cancel = quantity_item(&screen, 4);
        click(&mut screen, cancel);
        click_item(&mut screen, BUY_ITEM);
        assert!(screen.quantity().is_none(), "and up");
        assert_eq!(screen.take_order(), Some(order(128, Direction::Buy)));
    }

    #[test]
    fn cancelling_the_pointer_abandons_a_click_in_the_dialog() {
        let mut screen = counting_screen();
        alt_click(&mut screen, BUY_ITEM);
        let at = quantity_item(&screen, 1);
        let button = |pressed| Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at,
        };
        screen.input(&button(true));
        screen.cancel_pointer();
        screen.input(&button(false));
        assert_eq!(screen.take_order(), None, "the click was abandoned");
        assert!(screen.quantity().is_some());
    }

    // The outfitter's mission offers.

    use crate::spaceport::offer::OfferChoice;
    use crate::spaceport::offer::fixture::{prompt, templates as offer_templates};

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    /// The outfitter, its offers given.
    fn offering() -> OutfitterScreen {
        counting_screen().with_offers(offer_templates())
    }

    #[test]
    fn the_outfitter_with_offers_asks_for_an_offer_as_it_opens() {
        let mut screen = offering();
        assert!(screen.take_offer_request(), "before any tick");
        assert!(!screen.take_offer_request(), "once");
        screen.tick(ms(10_000));
        assert!(!screen.take_offer_request(), "not again until rearmed");
    }

    #[test]
    fn without_offers_the_outfitter_never_asks() {
        let mut screen = counting_screen();
        assert!(!screen.take_offer_request());
        screen.tick(ms(10_000));
        assert!(!screen.take_offer_request());
        screen.open_offer(prompt());
        assert!(screen.offer_dialog().is_none());
    }

    #[test]
    fn an_offer_opens_over_the_outfitter_and_takes_its_input() {
        let mut screen = offering();
        screen.open_offer(prompt());
        let offer = screen.offer_dialog().expect("open");
        assert_eq!(offer.prompt(), &prompt());
        assert_eq!(offer.problem(), None, "laid out by the template given");
        let mut expected = drawn(&counting_screen());
        let mut over = DrawList::new();
        offer.draw(&mut over);
        expected.extend(over.iter().cloned());
        assert_eq!(drawn(&screen), expected, "drawn over the outfitter");
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
    fn the_outfitters_offer_pointer_goes_through_it() {
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
    fn after_a_rearm_the_outfitter_asks_again_after_the_gap() {
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
    fn by_the_other_reading_the_outfitter_waits_out_its_quantity_dialog() {
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
    fn an_offer_due_under_the_alert_waits_as_the_offer_clock_reads() {
        for (clock, wait) in [(RuleSource::Engine, 0), (RuleSource::Bible, 600)] {
            let mut screen = alerting(OutfitRefusal::NegativeFreeMass)
                .with_offers(offer_templates())
                .with_offer_clock(clock);
            assert!(screen.take_offer_request());
            screen.rearm(ms(600));
            screen.tick(ms(10_000));
            assert!(!screen.take_offer_request(), "{clock:?}: under the alert");
            press(&mut screen, Key::Escape);
            assert!(screen.alert_dialog().is_none());
            if wait > 0 {
                screen.tick(ms(wait - 1));
                assert!(!screen.take_offer_request(), "{clock:?}");
                screen.tick(ms(1));
            } else {
                screen.tick(ms(0));
            }
            assert!(screen.take_offer_request(), "{clock:?}");
        }
    }

    #[test]
    fn the_outfitters_gap_runs_from_the_ask_by_the_engine_and_the_answer_by_the_other() {
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
    fn covered_the_outfitter_runs_only_its_offer_clock() {
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

    // A sale refused in the loop (`RuleKey::SaleRefusal`).

    /// "Desc Dialog"'s shape, fixed at (0, 0): Done (1) at (100, 90), a
    /// parked picture (2) and a text box (3) wide enough for a sentence
    /// on a line.
    fn desc_template() -> DialogTemplate {
        let item = |bounds, enabled, kind| ItemTemplate {
            bounds,
            enabled,
            kind,
        };
        DialogTemplate {
            bounds: rect(0.0, 0.0, 200.0, 120.0),
            placement: Placement::Fixed,
            items: vec![
                item(rect(100.0, 90.0, 99.0, 25.0), true, ItemSpec::User),
                item(
                    rect(10.0, 150.0, 20.0, 20.0),
                    false,
                    ItemSpec::Picture(1431),
                ),
                item(rect(10.0, 10.0, 2000.0, 60.0), false, ItemSpec::User),
            ],
        }
    }

    /// Outfit 128, a rack (3 owned), refused only in the sell loop: by
    /// its negative mass, a raised `Max` (its widgets) or its launcher's
    /// ammunition (Vipers), with the info-box line the other reading
    /// shows.
    fn refused_in_the_loop() -> [(OutfitRow, &'static str); 3] {
        let mut rack = row(128, "Rack", 100, 0, 3);
        rack.facts.names = LcNames {
            singular: "rack".to_owned(),
            plural: "racks".to_owned(),
        };
        let negative = OutfitRow {
            mass: -10,
            ..rack.clone()
        };
        let mut raiser = rack.clone();
        raiser.facts.raises = vec![Raise {
            target: OutfitId(200),
            held: 5,
            target_max: Some(2),
            left_by_the_bible: 0,
            target_names: Some(widgets()),
        }];
        let mut launcher = rack;
        launcher.facts.launcher = Some(Launcher {
            max_ammo: 1,
            rounds: 4,
            ammo: Some(OutfitId(158)),
        });
        launcher.facts.ammo_names = Some(vipers());
        [
            (negative, NEGATIVE_FREE_MASS),
            (
                raiser,
                "You need to sell one widget before you can sell your rack.",
            ),
            (
                launcher,
                "You need to sell two Vipers before you can sell your rack.",
            ),
        ]
    }

    /// The outfitter of `row` alone, 5 tons free, `SaleRefusal` read as
    /// `source` says.
    fn loop_refusing(row: OutfitRow, source: RuleSource) -> Outfitter {
        Outfitter {
            rows: vec![row],
            rules: outfitter()
                .rules
                .with_override(RuleKey::SaleRefusal, source),
            ..outfitter()
        }
    }

    /// The info box's texts.
    fn info_box(screen: &OutfitterScreen) -> Vec<String> {
        info_placed(screen)
            .into_iter()
            .map(|(text, _)| text)
            .collect()
    }

    #[test]
    fn by_the_engine_sell_stays_enabled_for_a_row_refused_only_in_the_loop() {
        for (row, line) in refused_in_the_loop() {
            let screen = screen_of(loop_refusing(row, RuleSource::Engine));
            assert!(enabled(&screen).1, "{line}");
        }
    }

    #[test]
    fn by_the_engine_the_info_box_shows_no_sale_refusal_line() {
        for (row, line) in refused_in_the_loop() {
            let engine = info_box(&screen_of(loop_refusing(row.clone(), RuleSource::Engine)));
            assert!(!engine.contains(&line.to_owned()), "{engine:?}");
            // The price and the cash: the rack is massless, so it shows
            // no Item Mass or Available line.
            assert_eq!(engine.len(), 4, "{engine:?}");
            let other = info_box(&screen_of(loop_refusing(row, RuleSource::Bible)));
            assert_eq!(other.last().map(String::as_str), Some(line), "{other:?}");
        }
    }

    #[test]
    fn by_the_engine_sell_on_a_row_refused_in_the_loop_records_the_order() {
        for (row, line) in refused_in_the_loop() {
            let mut screen = screen_of(loop_refusing(row, RuleSource::Engine));
            click_item(&mut screen, SELL_ITEM);
            assert_eq!(
                screen.take_order(),
                Some(order(128, Direction::Sell)),
                "{line}"
            );
            press(&mut screen, SELL_KEY);
            assert_eq!(
                screen.take_order(),
                Some(order(128, Direction::Sell)),
                "{line}"
            );
        }
    }

    #[test]
    fn by_the_engine_alt_sell_on_a_row_refused_in_the_loop_opens_the_quantity_dialog() {
        for (row, line) in refused_in_the_loop() {
            let mut screen = screen_of(loop_refusing(row, RuleSource::Engine));
            alt_click(&mut screen, SELL_ITEM);
            assert_eq!(screen.take_order(), None, "{line}");
            assert_eq!(field_text(&screen), Some("3"), "{line}: the 3 owned");
        }
    }

    /// A sale of the rack refused with `refusal`, naming the rack, the
    /// widgets and the Vipers, `plural` or not.
    fn rack_alert(refusal: OutfitRefusal, plural: bool) -> SaleAlert {
        SaleAlert {
            outfit: OutfitId(128),
            refusal,
            names: BTreeMap::from([
                (
                    OutfitId(128),
                    LcNames {
                        singular: "rack".to_owned(),
                        plural: "racks".to_owned(),
                    },
                ),
                (OutfitId(200), widgets()),
                (OutfitId(158), vipers()),
            ]),
            plural,
        }
    }

    const RAISED: OutfitRefusal = OutfitRefusal::RaisedFirst {
        count: 2,
        target: OutfitId(200),
    };

    const AMMUNITION: OutfitRefusal = OutfitRefusal::AmmunitionFirst {
        rounds: 1,
        ammo: Some(OutfitId(158)),
    };

    #[test]
    fn the_alert_names_the_sold_outfit_in_the_plural_while_passes_remain() {
        assert_eq!(
            sale_alert_text(&rack_alert(RAISED, false)),
            "You need to sell two widgets before you can sell your rack."
        );
        assert_eq!(
            sale_alert_text(&rack_alert(RAISED, true)),
            "You need to sell two widgets before you can sell your racks."
        );
        assert_eq!(
            sale_alert_text(&rack_alert(AMMUNITION, false)),
            "You need to sell one Viper before you can sell your rack."
        );
        assert_eq!(
            sale_alert_text(&rack_alert(AMMUNITION, true)),
            "You need to sell one Viper before you can sell your racks."
        );
        let mut unnamed = rack_alert(
            OutfitRefusal::AmmunitionFirst {
                rounds: 3,
                ammo: None,
            },
            true,
        );
        assert_eq!(
            sale_alert_text(&unnamed),
            "You need to sell three units of ammunition before you can sell your racks."
        );
        unnamed.names.clear();
        unnamed.refusal = OutfitRefusal::RaisedFirst {
            count: 1,
            target: OutfitId(200),
        };
        assert_eq!(
            sale_alert_text(&unnamed),
            "You need to sell one  before you can sell your ."
        );
    }

    #[test]
    fn a_negative_free_mass_alert_is_207_alone() {
        for plural in [false, true] {
            assert_eq!(
                sale_alert_text(&rack_alert(OutfitRefusal::NegativeFreeMass, plural)),
                NEGATIVE_FREE_MASS
            );
        }
        assert_eq!(
            sale_alert_text(&rack_alert(OutfitRefusal::CannotAfford, false)),
            "",
            "no alert gives any other refusal"
        );
    }

    /// The engine's outfitter of the overloaded rack, an alert open over
    /// it for `refusal`.
    fn alerting(refusal: OutfitRefusal) -> OutfitterScreen {
        let [(row, _), ..] = refused_in_the_loop();
        let mut screen = screen_of(loop_refusing(row, RuleSource::Engine));
        screen.alert(&rack_alert(refusal, false));
        screen
    }

    fn alert_item(screen: &OutfitterScreen, number: usize) -> Point {
        screen
            .alert_dialog()
            .expect("an alert")
            .dialog()
            .item_bounds(number)
            .expect("an item")
            .center()
    }

    #[test]
    fn a_sale_alert_opens_its_words_in_the_desc_dialog_over_the_outfitter() {
        let screen = alerting(RAISED);
        let alert = screen.alert_dialog().expect("an alert");
        assert_eq!(
            alert.lines().join(" "),
            "You need to sell two widgets before you can sell your rack."
        );
        let mut list = DrawList::new();
        alert.draw(&mut list);
        let alert_drawn: Vec<DrawCommand> = list.iter().cloned().collect();
        let all = drawn(&screen);
        assert!(all.ends_with(&alert_drawn), "drawn last");
        assert!(all.len() > alert_drawn.len(), "over the outfitter");
    }

    #[test]
    fn after_a_part_way_sale_the_alert_opens_over_the_outfitter_as_it_now_is() {
        let [_, (raiser, _), _] = refused_in_the_loop();
        let mut screen = screen_of(loop_refusing(raiser.clone(), RuleSource::Engine));
        alt_click(&mut screen, SELL_ITEM);
        press(&mut screen, Key::Enter);
        assert_eq!(
            screen.take_order(),
            Some(counted(128, Direction::Sell, 3)),
            "the 3 owned"
        );
        // One rack sold, then the rest refused.
        screen.set_outfitter(loop_refusing(
            OutfitRow { owned: 2, ..raiser },
            RuleSource::Engine,
        ));
        screen.alert(&rack_alert(RAISED, true));
        let alert = screen.alert_dialog().expect("an alert");
        assert_eq!(
            alert.lines().join(" "),
            "You need to sell two widgets before you can sell your racks."
        );
        let mut list = DrawList::new();
        alert.draw(&mut list);
        let alert_drawn: Vec<DrawCommand> = list.iter().cloned().collect();
        assert!(drawn(&screen).ends_with(&alert_drawn), "drawn last");
        assert_eq!(screen.outfitter().rows[0].owned, 2);
        let shown = info_placed(&screen);
        assert!(follows(&shown, "You Have:", "5,000 cr"), "{shown:?}");
    }

    #[test]
    fn done_return_or_escape_closes_the_alert_and_leaves_the_outfitter_open() {
        for k in [Key::Enter, Key::Escape] {
            let mut screen = alerting(OutfitRefusal::NegativeFreeMass);
            press(&mut screen, k);
            assert!(screen.alert_dialog().is_none(), "{k:?}");
            assert!(!screen.closed(), "{k:?}");
        }
        let mut screen = alerting(OutfitRefusal::NegativeFreeMass);
        let done = alert_item(&screen, crate::ui::desc::DONE_ITEM);
        click(&mut screen, done);
        assert!(screen.alert_dialog().is_none());
        assert!(!screen.closed());
        press(&mut screen, Key::Escape);
        assert!(screen.closed(), "the next Escape closes the outfitter");
    }

    #[test]
    fn while_the_alert_is_open_it_takes_every_input() {
        let mut screen = alerting(OutfitRefusal::NegativeFreeMass);
        let selected = screen.selected();
        let sell = item(&screen, SELL_ITEM).center();
        let cell = screen.cell_bounds(0).expect("a cell").center();
        for k in [BUY_KEY, SELL_KEY, Key::Right, Key::Down, Key::Char('i')] {
            press(&mut screen, k);
        }
        click(&mut screen, sell);
        click(&mut screen, cell);
        alt_click(&mut screen, SELL_ITEM);
        assert_eq!(screen.take_order(), None);
        assert!(screen.quantity().is_none());
        assert_eq!(screen.selected(), selected);
        assert!(screen.alert_dialog().is_some(), "still open");
        assert!(!screen.closed());
        press(&mut screen, Key::Escape);
        assert!(screen.alert_dialog().is_none());
        assert!(!screen.closed(), "Escape closed the alert alone");
    }

    #[test]
    fn alt_let_go_while_the_alert_is_open_is_followed() {
        let [(row, _), ..] = refused_in_the_loop();
        let mut screen = screen_of(loop_refusing(row, RuleSource::Engine));
        press(&mut screen, COUNT_KEY);
        screen.alert(&rack_alert(OutfitRefusal::NegativeFreeMass, false));
        screen.input(&key(COUNT_KEY, false, false));
        press(&mut screen, Key::Escape);
        press(&mut screen, SELL_KEY);
        assert!(screen.quantity().is_none());
        assert_eq!(screen.take_order(), Some(order(128, Direction::Sell)));
    }

    #[test]
    fn without_the_desc_template_an_alert_opens_nothing() {
        let [(row, _), ..] = refused_in_the_loop();
        let mut screen = OutfitterScreen::new(
            Ok(layout()),
            Ok(quantity()),
            Err("no DLOG 3003".to_owned()),
            loop_refusing(row.clone(), RuleSource::Engine),
            art(),
            ButtonStyle::STOCK,
        );
        click_cell(&mut screen, 0);
        screen.alert(&rack_alert(OutfitRefusal::NegativeFreeMass, false));
        assert!(screen.alert_dialog().is_none());
        press(&mut screen, SELL_KEY);
        assert_eq!(screen.take_order(), Some(order(128, Direction::Sell)));
        let mut unlaid = OutfitterScreen::new(
            Err("no DLOG 1002".to_owned()),
            Ok(quantity()),
            Ok(desc_template()),
            loop_refusing(row, RuleSource::Engine),
            art(),
            ButtonStyle::STOCK,
        );
        unlaid.alert(&rack_alert(OutfitRefusal::NegativeFreeMass, false));
        assert!(unlaid.alert_dialog().is_none(), "no metrics to lay it out");
    }

    /// `_TrackTextButtons` @0x26bb0 plays nothing under the pointer.
    #[test]
    fn the_alerts_done_is_silent_under_the_pointer() {
        let mut screen = alerting(OutfitRefusal::NegativeFreeMass);
        let done = alert_item(&screen, crate::ui::desc::DONE_ITEM);
        screen.input(&Input::PointerButton {
            button: MouseButton::Left,
            pressed: true,
            at: done,
        });
        assert_eq!(screen.take_sounds(), []);
        screen.input(&Input::PointerButton {
            button: MouseButton::Left,
            pressed: false,
            at: done,
        });
        assert_eq!(screen.take_sounds(), []);
        assert!(screen.alert_dialog().is_none());
    }

    #[test]
    fn cancelling_the_pointer_abandons_a_click_in_the_alert() {
        let mut screen = alerting(OutfitRefusal::NegativeFreeMass);
        let done = alert_item(&screen, crate::ui::desc::DONE_ITEM);
        let button = |pressed| Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at: done,
        };
        screen.input(&button(true));
        screen.cancel_pointer();
        screen.release_keys();
        screen.input(&button(false));
        assert!(screen.alert_dialog().is_some(), "Done was not clicked");
    }

    #[test]
    fn the_mission_info_key_asks_for_the_mission_info_screen() {
        let i = crate::ui::mission_info::MISSION_INFO_KEY;
        let mut screen = counting_screen();
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
        let mut screen = alerting(RAISED);
        press(&mut screen, i);
        assert!(!screen.take_info_request(), "the alert takes it");
        assert!(screen.alert_dialog().is_some());
        let mut broken = OutfitterScreen::new(
            Err("no DLOG 1002".to_owned()),
            Ok(quantity()),
            Ok(desc_template()),
            outfitter(),
            art() as Rc<dyn OutfitterCatalog>,
            ButtonStyle::STOCK,
        );
        press(&mut broken, i);
        assert!(!broken.take_info_request(), "no dialog");
    }

    #[test]
    fn the_map_key_asks_for_the_map_on_a_press_not_in_the_quantity_dialog_or_the_alert() {
        let m = crate::flight::view::MAP_KEY;
        let mut screen = counting_screen();
        assert!(!screen.take_map_request());
        screen.input(&key(m, true, true));
        screen.input(&key(m, false, false));
        assert!(!screen.take_map_request(), "a repeat or a release");
        press(&mut screen, m);
        assert!(screen.take_map_request());
        assert!(!screen.take_map_request(), "once");
        assert!(!screen.take_info_request());
        assert!(!screen.closed());
        alt_click(&mut screen, BUY_ITEM);
        press(&mut screen, m);
        assert!(!screen.take_map_request(), "the quantity dialog takes it");
        assert!(screen.quantity().is_some());
        let mut screen = alerting(RAISED);
        press(&mut screen, m);
        assert!(!screen.take_map_request(), "the alert takes it");
        assert!(screen.alert_dialog().is_some());
        let mut broken = OutfitterScreen::new(
            Err("no DLOG 1002".to_owned()),
            Ok(quantity()),
            Ok(desc_template()),
            outfitter(),
            art() as Rc<dyn OutfitterCatalog>,
            ButtonStyle::STOCK,
        );
        press(&mut broken, m);
        assert!(!broken.take_map_request(), "no dialog");
        let mut screen = offering();
        screen.open_offer(prompt());
        press(&mut screen, m);
        assert!(!screen.take_map_request(), "the offer's");
        assert!(screen.take_offer_map_request().is_some());
    }

    #[test]
    fn the_mission_info_key_asks_from_its_offer_which_stays_open() {
        let mut screen = offering();
        screen.open_offer(prompt());
        press(&mut screen, crate::ui::mission_info::MISSION_INFO_KEY);
        assert!(screen.take_info_request());
        assert!(screen.offer_dialog().is_some());
    }

    /// The original's `_OutfitFilter` @0x58039 maps keys to items with no click sound.
    #[test]
    fn keyboard_activation_is_silent_as_the_original() {
        for k in [
            Key::Right,
            Key::Down,
            Key::Left,
            Key::Up,
            BUY_KEY,
            SELL_KEY,
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

    /// The outfitter's buttons take Tab then Space only with the focus
    /// ring, and its sale alert inherits the ring.
    #[test]
    fn tab_then_space_presses_the_focused_button_only_with_the_focus_ring() {
        let mut ringed = screen();
        press(&mut ringed, Key::Tab);
        press(&mut ringed, Key::Space);
        assert!(ringed.closed(), "by default");
        let mut off = screen().with_focus_ring(FocusRing::Off);
        press(&mut off, Key::Tab);
        press(&mut off, Key::Space);
        assert!(!off.closed());
        assert_eq!(off.take_order(), None);
        assert_eq!(off.dialog().and_then(Dialog::focus), None);
        for ring in [FocusRing::Tab, FocusRing::Off] {
            let [(row, _), ..] = refused_in_the_loop();
            let mut screen =
                screen_of(loop_refusing(row, RuleSource::Engine)).with_focus_ring(ring);
            screen.alert(&rack_alert(OutfitRefusal::NegativeFreeMass, false));
            press(&mut screen, Key::Tab);
            press(&mut screen, Key::Space);
            assert_eq!(
                screen.alert_dialog().is_none(),
                ring == FocusRing::Tab,
                "the alert's Done, {ring:?}"
            );
        }
    }

    /// The outfitter's mission offers take its focus ring, given before the
    /// offers as the spaceport gives it.
    #[test]
    fn its_offer_takes_tab_then_space_only_with_the_focus_ring() {
        for ring in [FocusRing::Tab, FocusRing::Off] {
            let mut screen = counting_screen()
                .with_focus_ring(ring)
                .with_offers(offer_templates());
            screen.open_offer(crate::spaceport::offer::fixture::prompt());
            press(&mut screen, Key::Tab);
            press(&mut screen, Key::Space);
            assert_eq!(
                screen.take_offer_choice().is_some(),
                ring == FocusRing::Tab,
                "{ring:?}"
            );
        }
    }
}
