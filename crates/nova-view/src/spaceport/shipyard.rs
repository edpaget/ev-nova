//! The Shipyard: the stellar's ships for sale, laid out by the interface
//! file's "Shipyard" dialog (`DLOG`/`DITL` 1004) over its background
//! picture.
//!
//! The stock dialog is 765 x 323, centred, and drawn over `PICT` 8501,
//! whose painted frames match its items: the grid of ships (item 5) on the
//! left, the selected ship's description (item 6) beside it, its picture
//! (item 8) and the info box (item 9) on the right, and along the bottom
//! the grid's scroll arrows (items 12 and 13), Info (10), Done (1) and Buy
//! Ship (7). The roles are read from the "Outfit" dialog's numbering and
//! the labels; the screen enables Done, Buy Ship, Info and the arrows
//! itself.
//!
//! The grid shows each ship listed ([`Shipyard`]) in a cell, as the
//! Outfitter's does ([`grid`](super::grid)): its picture (the same one as
//! the detail pane's, [`ship_picture`], shrunk to 32 x 32, or a black
//! square without one) near the cell's top, and its `ShortName`'s lines
//! centred at the bottom. "(current)" marks the cell of the class the
//! player flies, in its top-left corner: the port's own addition, as the
//! original's shipyard cell has no such mark (`_ShipyardDialogUpdate`
//! @0x58b30-0x58f91). With nothing listed, the grid says so (`STR#` 2002
//! #223).
//!
//! The selection follows
//! [`RuleKey::GridSelection`](nova_sim::RuleKey::GridSelection). By the
//! engine, as `_DoShipyardDialog` and `_ShipyardFilter` keep it, the
//! shipyard opens with nothing selected (@0x5e6d8-0x5e705); a press on an
//! empty cell selects nothing (@0x60a5d-0x60a73) and reaches no button;
//! and a scroll arrow that scrolls selects nothing (@0x5f580-0x5f5be,
//! @0x5f60b-0x5f670). With nothing selected, Buy Ship and Info are greyed
//! and no picture, info box or description is drawn. An arrow key then
//! highlights the first cell shown (Right, Down) or the last (Left, Up),
//! as `_ShipyardFilter` does from no cell (@0x60734-0x607b7); Page Up and
//! Page Down are not read here. By the other reading, the first cell is
//! selected as it opens, a press on an empty cell is ignored, and a scroll
//! keeps the selection. By both, a second press on the same cell position
//! within 15 Mac ticks of the first opens Info ([`grid_clicks`](super::grid_clicks),
//! @0x60cd7-0x60d12).
//!
//! The selected ship's picture is [`ship_picture`]'s: `PICT` 5000 plus its
//! ID less 128, or else that of the lowest-numbered ship sharing its
//! `shän` base image, as the Bible says target pictures are shared, or
//! "No Picture" and "Available" centred on the picture item in white. Its
//! description is `dësc` 13000 plus its ID less 128. The
//! info box shows, for any class but the one flown, four rows at the
//! original's baselines (`_ShipyardDialogUpdate` @0x592e4-0x596f6): "Ship
//! Price:", "Trade-In:", "Final Price:" (the price less the trade-in, at
//! least 0) and "You Have:" with the cash (#225-227, #217). Each amount is
//! drawn as `_DrawCommaNumber` draws it, followed by " cr"
//! ([`credits`](super::credits)). For the class flown it shows none
//! (@0x592d7-0x592de).
//!
//! Buy Ship, clicked or with B, requests the selected ship, and is greyed
//! when it cannot be bought, the class flown among them (`_CalcShipCanBuy`
//! @0x4f6f5-0x4f6fc; the original has no words for why). B with no ship
//! selected, or one that cannot be bought, beeps
//! ([`UiSound::BuyRefused`], `_beepSnd[3]`, @0x5f22a-0x5f256). B acts
//! on a press only, never a key repeat, unlike the Outfitter's. A held B
//! no longer re-buys the class just bought, which is now the class flown
//! and refused ([`ShipRefusal::AlreadyFlown`](nova_sim::shipyard::ShipRefusal::AlreadyFlown)),
//! but the purchase draws that class's roll again, and when the shipyard
//! given back no longer lists it, the selection falls on another class,
//! which a held B would otherwise trade the new ship and its outfits
//! for. The screen only records the request
//! ([`ShipyardScreen::take_request`]): whoever holds the session asks for
//! the new ship's name ([`ShipNaming`]) and opens the "Text Input" prompt
//! with it ([`ShipyardScreen::open_naming`], [`TextInputDialog`]), as the
//! original asks before it buys (`_DoShipyardDialog` @0x5eb6c). The
//! prompt takes every input while it is open; when B asked, the B typed
//! is dropped, as the original flushes it. OK records the order, the ship
//! and the name confirmed ([`ShipyardScreen::take_order`]): whoever holds
//! the session makes it and gives back the shipyard as it now is
//! ([`ShipyardScreen::set_shipyard`]). Cancel records the ship declined
//! ([`ShipyardScreen::take_declined`]), whose roll the session draws
//! again. Without the "Text Input" dialog, the prompt is laid out by
//! [`TextInputDialog::fallback`].
//!
//! Info, clicked or with Space ([`INFO_KEY`]), opens the "Shipyard Info" panel (`DLOG`/`DITL`
//! 1005 over `PICT` 8506) over the shipyard: the ship's name in item 3,
//! and in item 5 what `_ShipDescDialogUpdate` draws (@0x4fb4a-0x50f17),
//! each text placed by its baseline ([`spec_lines`]). Two columns of a
//! label in grey (`_rgbMedGrey`, [`LABEL_COLOR`]) and a value in white,
//! the labels at 0 and 130 right of item 5's left and the values at 45
//! and 175, in rows from baseline 12 in steps of 12: on the left Speed,
//! Accel and Turn (Accel and Turn as rating words, #245-250, or "N/A",
//! #396), Shields, Armor, Guns and Turrets ("Maximum of N", or "None",
//! #335); on the right Space, Cargo, Energy (its jumps), Length, Mass
//! and Crew (drawn only above 0). From baseline 108, "Standard Weapons:"
//! and the class's stock weapons, one a line at 8 right and 12 apart, or
//! "No standard weapons" (#241-243), all in white. A text whose top is at
//! or below item 5's bottom is not drawn, as the original clips to it.
//! The values are worked out in the core ([`ShipSpecs`]). Its Done,
//! Return and Escape close the panel alone. Without that dialog, Info is
//! greyed.
//!
//! Which "Shipyard Info" opens follows the class's description `Graphic`
//! (`_LoadDescription` @0x7186b-0x71892, read here when Info opens), as
//! `_DoShipyardDescription` picks it (@0x5b6db-0x5b77e, [`InfoVariant`]).
//! Above 127, when its `PICT` loads, "Shipyard Info" with a picture
//! (`DLOG` 1019) opens over `PICT` 8507, with the graphic stretched into
//! item 7 after Done (@0x50fd0-0x51043); when it does not load, `DLOG`
//! 1005 opens over `PICT` 8507 stretched into it, as the background
//! follows the ID test alone (@0x5b758-0x5b77e). While the ID test holds,
//! the title is the class's `Long Name`, whole (`_shipyardLongNames`,
//! @0x4fa33-0x4fa69), not its name. When the dialog picked is missing,
//! nothing opens (@0x5b749-0x5b752).
//!
//! While the ID test holds, item 5 draws no stock weapons
//! ([`spec_lines_for`]); they go into item 8 instead (@0x5061c-0x50ce0):
//! "Standard Weapons:", or "No standard weapons" with none, in white at
//! its left on baseline 12, then each weapon's line joined by ", " into
//! one sentence ([`weapons_sentence`]), set in a box from 16 below item
//! 8's top to its bottom, wrapped at word breaks, a line straddling the
//! bottom showing its top clipped to the box, in white on black
//! (`TETextBox`, then `InvertRect`). With none listed
//! there is no box. Without an item 8 (`DLOG` 1005 over `PICT` 8507,
//! where the original's rect is undefined), neither is drawn.
//!
//! In both variants the title is drawn in font 20, Times ([`TITLE_FONT`],
//! which the renderer draws in the bundled Noto Serif), at size 18, in
//! white, centred by `_DrawCenteredText` across item 3 on baseline item
//! 3's top + 18 (`_ShipDescDialogUpdate` @0x4f9f7-0x4faa3).
//!
//! Given the offer dialog's template ([`ShipyardScreen::with_offers`]),
//! the shipyard offers missions as `_DoShipyardDialog` and
//! `_ShipyardFilter` do while buying ships (the bar's escort hire is its
//! own screen, which offers none): it asks for an offer as it opens
//! ([`ShipyardScreen::take_offer_request`], `_DoShipyardDialog`
//! @0x5e8eb), and whoever holds the session answers with one, opening the
//! [`OfferDialog`] over it ([`ShipyardScreen::open_offer`]), or rearms it
//! ([`ShipyardScreen::rearm`]) with the gap drawn, after which it asks
//! again (`_ShipyardFilter` @0x60de5-0x60e1f). The offer takes every
//! input while it is open, and its choice is taken through the screen
//! ([`ShipyardScreen::take_offer_choice`]). The info panel and the name
//! prompt cover the shipyard, as the offer does: nothing is asked while
//! any is open, and the wait runs on or stops as
//! [`RuleKey::OfferClock`](nova_sim::RuleKey::OfferClock) reads it
//! ([`ShipyardScreen::with_offer_clock`]).
//!
//! Done, Return and Escape close the shipyard. Without its dialog, the
//! screen says why, and Return or Escape still closes it.

use std::rc::Rc;
use std::time::Duration;

use nova_sim::shipyard::SHIP_NAME_CHARS;
use nova_sim::{
    MissionId, OfferPrompt, Rating, RuleSource, ShipId, ShipNaming, ShipRow, ShipSpecs, Shipyard,
    StockLine,
};

use super::catalog::SpaceportCatalog;
use super::credits::{self, credits_text, draw_amount_rows, tons_text};
use super::grid::{
    self, GridColors, GridLooks, INSET, Shown, TEXT_COLOR, TEXT_FONT, TEXT_SIZE, TextStyle, text,
};
use super::layout::DONE_LABEL;
use super::modal::ModalHost;
use super::offer::{OfferChoice, OfferDialog, OfferTemplates, VenueOffers};
use super::shipyard_grid::{PressOutcome, ShipyardGrid};
use super::view::{PROBLEM_AT, PROBLEM_SIZE};
use crate::color::Color;
use crate::draw::{DrawList, fill_rect};
use crate::flight::PortScreen;
use crate::flight::view::is_map_press;
use crate::font::Font;
use crate::geometry::{Bounds, Point};
use crate::image::ImageKey;
use crate::input::{Input, Key};
use crate::movie::Movies;
use crate::screen::{Screen, ScreenAction};
use crate::sound::{Sound, UiSound};
use crate::text::TextMetrics;
use crate::ui::button::{ButtonSkin, ButtonStyle};
use crate::ui::catalog::{DescriptionGraphics, DescriptionSource};
use crate::ui::dialog::{Dialog, DialogEvent, DialogTemplate, Role};
use crate::ui::mission_info::is_mission_info_press;
use crate::ui::picture_variant::{VariantTemplates, shown_graphic};
use crate::ui::player_info::is_player_info_press;
use crate::ui::text_box::TextBox;
use crate::ui::text_input::{TextInputDialog, TextInputOutcome};

/// The "Shipyard" dialog's `DLOG` (and `DITL`) ID.
pub const SHIPYARD_DIALOG: i16 = 1004;
/// The "Shipyard Info" dialog's `DLOG` (and `DITL`) ID.
pub const SHIP_INFO_DIALOG: i16 = 1005;
/// The "Shipyard Info" dialog with a picture: its `DLOG` (and `DITL`) ID
/// (`_DoShipyardDescription` `GetNewDialog` 0x3fb, @0x5b70c-0x5b742).
pub const SHIP_PICTURE_INFO_DIALOG: i16 = 1019;

/// The picture the shipyard is drawn over: "Shipyard", in Nova Graphics.
pub const BACKGROUND: ImageKey = ImageKey::picture(8501);
/// The picture the info panel is drawn over.
pub const INFO_BACKGROUND: ImageKey = ImageKey::picture(8506);
/// The picture the info panel is drawn over while its graphic's ID is
/// above [`LAST_UNSHOWN_GRAPHIC`], "Ship description + pict"
/// (`_DoShipyardDescription` @0x5b758-0x5b77e).
pub const PICTURE_INFO_BACKGROUND: ImageKey = ImageKey::picture(8507);

/// Done's item.
pub const DONE_ITEM: usize = 1;
/// The grid's item.
pub const GRID_ITEM: usize = 5;
/// The description's item.
pub const DESCRIPTION_ITEM: usize = 6;
/// Buy Ship's item.
pub const BUY_ITEM: usize = 7;
/// The picture's item.
pub const PICTURE_ITEM: usize = 8;
/// The info box's item.
pub const INFO_BOX_ITEM: usize = 9;
/// Info's item.
pub const INFO_ITEM: usize = 10;
/// The grid's scroll-up arrow's item.
pub const SCROLL_UP_ITEM: usize = 12;
/// The grid's scroll-down arrow's item.
pub const SCROLL_DOWN_ITEM: usize = 13;

/// The info panel's Done.
pub const PANEL_DONE_ITEM: usize = 1;
/// The info panel's title.
pub const PANEL_TITLE_ITEM: usize = 3;
/// The info panel's text.
pub const PANEL_TEXT_ITEM: usize = 5;
/// The info panel title's font: QuickDraw font 20, Times
/// (`_ShipDescDialogUpdate` `TextFont(0x14)` @0x4f9f7).
pub const TITLE_FONT: Font = Font::Times;
/// The info panel title's size (`TextSize(0x12)` @0x4fa0a).
pub const TITLE_SIZE: f32 = 18.0;
/// The info panel title's baseline below item 3's top (+0x12,
/// @0x4fa0f-0x4faa3).
pub const TITLE_BASELINE: f32 = 18.0;
/// The picture info panel's graphic (`_ShipDescDialogUpdate`
/// @0x50fd0-0x51043).
pub const PANEL_GRAPHIC_ITEM: usize = 7;
/// The highest description `Graphic` that shows no picture: the engine
/// shows one only above it (`_DoShipyardDescription` @0x5b6db-0x5b6e4),
/// as it does for every `dësc`.
pub use crate::ui::desc::LAST_UNSHOWN_GRAPHIC;

/// Buys the selected ship, as Buy Ship does: the original's default.
pub const BUY_KEY: Key = Key::Char('b');
/// Opens the info panel, as Info does: Space, which `_ShipyardFilter`
/// maps to item 10 (@0x60526-0x60537). Nothing in it reads I, which asks
/// for the mission info screen
/// ([`MISSION_INFO_KEY`](crate::ui::mission_info::MISSION_INFO_KEY), pref
/// +0x50 @0x60947-0x6096d).
pub const INFO_KEY: Key = Key::Space;

/// The first ship's `PICT`; ship n's is n - 128 after it.
pub const FIRST_SHIP_PICTURE: i16 = 5000;
/// The first `shïp` ID.
const FIRST_SHIP: i16 = 128;

/// `STR#` 150 #4.
pub const BUY_SHIP_LABEL: &str = "Buy Ship";
/// `STR#` 150 #48.
pub const INFO_LABEL: &str = "Info";
/// `STR#` 2002 #223.
pub const NO_SHIPS: &str = "There are no ships available for purchase here.";
/// `STR#` 2002 #225.
pub const PRICE_LABEL: &str = "Ship Price:";
/// `STR#` 2002 #226.
pub const TRADE_IN_LABEL: &str = "Trade-In:";
/// `STR#` 2002 #227.
pub const FINAL_PRICE_LABEL: &str = "Final Price:";
/// The info box's rows' baselines below its top: Ship Price, Trade-In,
/// Final Price and You Have (@0x592f0, @0x593ba, @0x594db, @0x5954b).
pub const PRICE_ROWS: [f32; 4] = [12.0, 24.0, 48.0, 72.0];
/// `STR#` 2002 #229-237, the info panel's stat labels in order.
pub const STAT_LABELS: [&str; 9] = [
    "Speed:", "Accel:", "Turn:", "Guns:", "Turrets:", "Space:", "Length:", "Mass:", "Crew:",
];
/// `STR#` 2002 #110, the info panel's cargo space label.
pub const CARGO_LABEL: &str = "Cargo:";
/// `STR#` 2002 #238.
pub const MAXIMUM_OF: &str = "Maximum of";
/// `STR#` 2002 #239.
pub const JUMP: &str = "jump";
/// `STR#` 2002 #240.
pub const JUMPS: &str = "jumps";
/// `STR#` 2002 #11, the info panel's shields label (`_shieldString`,
/// `_LoadStrings` @0x71db5-0x71dd9).
pub const SHIELDS_LABEL: &str = "Shields:";
/// `STR#` 2002 #16, the info panel's armour label (`_armorString`,
/// @0x71ddb-0x71e01).
pub const ARMOR_LABEL: &str = "Armor:";
/// `STR#` 2002 #7, the info panel's fuel label (`_fuelString` + 0x400,
/// @0x71d8f-0x71db3).
pub const ENERGY_LABEL: &str = "Energy:";
/// `STR#` 2002 #335: a value of none.
pub const NONE: &str = "None";
/// `STR#` 2002 #396: an Accel or Turn of none.
pub const NOT_APPLICABLE: &str = "N/A";
/// `STR#` 2002 #245-250: the Accel and Turn ratings, Excellent first.
pub const RATINGS: [&str; 6] = [
    "Excellent",
    "Very Good",
    "Good",
    "Average",
    "Poor",
    "Terrible",
];
/// `STR#` 2002 #244: the Length's unit.
pub const METRES: &str = "m";
/// `STR#` 2002 #241: the stock-weapons list's heading.
pub const STANDARD_WEAPONS: &str = "Standard Weapons:";
/// `STR#` 2002 #242: the heading of a class without stock weapons.
pub const NO_STANDARD_WEAPONS: &str = "No standard weapons";
/// `STR#` 2002 #243: a stock weapon's ammunition.
pub const AMMO: &str = "ammo";
/// The info panel's label columns, right of item 5's left: the left and
/// the right (`_ShipDescDialogUpdate` `MoveTo` h +0 and +0x82).
pub const PANEL_LABEL_COLUMNS: [f32; 2] = [0.0, 130.0];
/// The info panel's value columns: the left and the right (h +0x2d and
/// +0xaf).
pub const PANEL_VALUE_COLUMNS: [f32; 2] = [45.0, 175.0];
/// The info panel's rows' baselines below item 5's top (v +0xc to
/// +0x54).
pub const PANEL_ROWS: [f32; 7] = [12.0, 24.0, 36.0, 48.0, 60.0, 72.0, 84.0];
/// The stock-weapons heading's baseline (v +0x6c, @0x505d6-0x505fd).
pub const WEAPONS_ROW: f32 = 108.0;
/// The first stock weapon's baseline (v +0x78, @0x5071f-0x50741).
pub const FIRST_WEAPON_ROW: f32 = 120.0;
/// Each later stock weapon's step down.
pub const WEAPON_STEP: f32 = 12.0;
/// The stock weapons' indent (h +8).
pub const WEAPON_INDENT: f32 = 8.0;
/// The picture info panel's stock weapons, while the ID test holds
/// (`_ShipDescDialogUpdate` @0x5061c-0x5066d).
pub const PANEL_WEAPONS_ITEM: usize = 8;
/// The weapons heading's baseline below item 8's top (v +0xc,
/// @0x5065c).
pub const WEAPONS_HEADING_ROW: f32 = 12.0;
/// How far below item 8's top the weapons sentence's box starts (+0x10,
/// @0x5066d).
pub const WEAPONS_BOX_DROP: f32 = 16.0;
/// What joins the weapons sentence's parts (@0x50c3c).
pub const WEAPONS_SEPARATOR: &str = ", ";
/// The info panel's labels' grey: `_rgbMedGrey`, 0x8000 in each channel
/// (`_InitColors` @0x1bf10-0x1bf21).
pub const LABEL_COLOR: Color = Color::rgba(128, 128, 128, 255);
/// The mark in the cell of the class the player flies.
pub const CURRENT_MARK: &str = "(current)";

/// `ship`'s picture: its own `PICT` ([`FIRST_SHIP_PICTURE`] + ID - 128)
/// when `exists` says it is there; otherwise that of the lowest-numbered
/// ship in `bases` sharing its `shän` base image whose picture is there;
/// otherwise none. `bases` gives each ship's base image.
#[must_use]
pub fn ship_picture(
    ship: ShipId,
    bases: &[(ShipId, i16)],
    exists: impl Fn(i16) -> bool,
) -> Option<i16> {
    let picture = |id: ShipId| FIRST_SHIP_PICTURE.saturating_add(id.0.saturating_sub(FIRST_SHIP));
    let own = picture(ship);
    if exists(own) {
        return Some(own);
    }
    let (_, base) = bases.iter().find(|(id, _)| *id == ship)?;
    let mut sharers: Vec<ShipId> = bases
        .iter()
        .filter(|(id, other)| other == base && *id != ship)
        .map(|(id, _)| *id)
        .collect();
    sharers.sort();
    sharers.into_iter().map(picture).find(|&id| exists(id))
}

/// Which "Shipyard Info" opens, as `_DoShipyardDescription` picks it
/// (@0x5b6db-0x5b77e) from the selected class's description `Graphic`
/// ([`info_variant`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InfoVariant {
    /// No `Graphic` above [`LAST_UNSHOWN_GRAPHIC`]: `DLOG` 1005 over
    /// `PICT` 8506, titled with the class's name.
    Plain,
    /// A `Graphic` above it whose `PICT` loads: `DLOG` 1019 over `PICT`
    /// 8507, the graphic in item 7, titled with the `Long Name`.
    Picture(i16),
    /// A `Graphic` above it whose `PICT` does not load: `DLOG` 1005 over
    /// `PICT` 8507 stretched into it, titled with the `Long Name`.
    MissingPicture,
}

impl InfoVariant {
    /// The dialog that opens: 1019 when the graphic loaded, else 1005
    /// (`GetNewDialog` 0x3fb/0x3ed, @0x5b70c-0x5b742).
    #[must_use]
    pub fn dialog(self) -> i16 {
        match self {
            Self::Picture(_) => SHIP_PICTURE_INFO_DIALOG,
            Self::Plain | Self::MissingPicture => SHIP_INFO_DIALOG,
        }
    }

    /// The background, which follows the ID test alone: `PICT` 8507 above
    /// 127, else 8506 (@0x5b758-0x5b77e).
    #[must_use]
    pub fn background(self) -> ImageKey {
        if self.id_test() {
            PICTURE_INFO_BACKGROUND
        } else {
            INFO_BACKGROUND
        }
    }

    /// The graphic drawn in item 7, when it loaded (@0x50fd0-0x51043).
    #[must_use]
    pub fn graphic(self) -> Option<i16> {
        match self {
            Self::Picture(id) => Some(id),
            Self::Plain | Self::MissingPicture => None,
        }
    }

    /// Whether the `Graphic` is above 127, as `setg` records it
    /// (`_ShipDescDialogUpdate` @0x4f97b-0x4f986): it titles the panel
    /// with the `Long Name`.
    #[must_use]
    pub fn id_test(self) -> bool {
        !matches!(self, Self::Plain)
    }
}

/// The variant of "Shipyard Info" for a description whose `Graphic` is
/// `graphic`: [`InfoVariant::Plain`] without one above
/// [`LAST_UNSHOWN_GRAPHIC`]; otherwise [`InfoVariant::Picture`] when
/// `exists` says its `PICT` is there, as `GetPicture` loads it
/// (@0x5b6db-0x5b70a), and [`InfoVariant::MissingPicture`] when not.
/// `exists` is asked only above 127.
#[must_use]
pub fn info_variant(graphic: Option<i16>, exists: impl Fn(i16) -> bool) -> InfoVariant {
    match shown_graphic(graphic) {
        Some(id) if exists(id) => InfoVariant::Picture(id),
        Some(_) => InfoVariant::MissingPicture,
        None => InfoVariant::Plain,
    }
}

/// Both "Shipyard Info" templates, or why each is missing: `DLOG` 1005's
/// and `DLOG` 1019's.
#[derive(Clone, Debug)]
pub struct InfoTemplates {
    /// "Shipyard Info" ([`SHIP_INFO_DIALOG`]). Info is greyed without it.
    pub plain: Result<DialogTemplate, String>,
    /// "Shipyard Info" with a picture ([`SHIP_PICTURE_INFO_DIALOG`]).
    pub picture: Result<DialogTemplate, String>,
}

impl VariantTemplates for InfoTemplates {
    fn plain_template(&self) -> &Result<DialogTemplate, String> {
        &self.plain
    }

    fn picture_template(&self) -> &Result<DialogTemplate, String> {
        &self.picture
    }
}

impl InfoTemplates {
    /// The template of `variant`'s dialog, if it is there.
    fn of(&self, variant: InfoVariant) -> Option<&DialogTemplate> {
        self.template(variant.dialog() == SHIP_PICTURE_INFO_DIALOG)
            .as_ref()
            .ok()
    }
}

/// The info box's rows for `row`, each a label and its amount: none for
/// the class flown (`_ShipyardDialogUpdate` skips them,
/// @0x592d7-0x592de), otherwise the price, the trade-in, the final price
/// at least 0, and the cash.
#[must_use]
pub fn price_lines(shipyard: &Shipyard, row: &ShipRow) -> Vec<(&'static str, String)> {
    if row.id == shipyard.current {
        return Vec::new();
    }
    vec![
        (PRICE_LABEL, credits_text(row.price)),
        (TRADE_IN_LABEL, credits_text(shipyard.trade_in)),
        (
            FINAL_PRICE_LABEL,
            credits_text(shipyard.final_price_clamped(row)),
        ),
        (credits::YOU_HAVE, credits_text(shipyard.cash)),
    ]
}

/// One text of the info panel's item 5: drawn at `dx` right of its left
/// with its baseline `baseline` below its top, in `color`.
#[derive(Clone, Debug, PartialEq)]
pub struct PanelText {
    /// How far right of item 5's left.
    pub dx: f32,
    /// Its baseline, below item 5's top.
    pub baseline: f32,
    /// The words.
    pub text: String,
    /// Its colour: [`LABEL_COLOR`] for a label, white otherwise.
    pub color: Color,
}

/// A rating's word.
fn rating_text(rating: Rating) -> &'static str {
    let [excellent, very_good, good, average, poor, terrible] = RATINGS;
    match rating {
        Rating::NotApplicable => NOT_APPLICABLE,
        Rating::Terrible => terrible,
        Rating::Poor => poor,
        Rating::Average => average,
        Rating::Good => good,
        Rating::VeryGood => very_good,
        Rating::Excellent => excellent,
    }
}

/// A stock weapon's line: its count and name, "s" when the count is
/// above 1, and " + N ammo" with its ammunition.
fn stock_text(line: &StockLine) -> String {
    let plural = if line.count > 1 { "s" } else { "" };
    let ammo = line
        .ammo
        .map_or_else(String::new, |ammo| format!(" + {ammo} {AMMO}"));
    format!("{} {}{plural}{ammo}", line.count, line.name)
}

/// The stock weapons' heading: "Standard Weapons:", or "No standard
/// weapons" with none (#241-242).
fn weapons_heading(weapons: &[StockLine]) -> &'static str {
    if weapons.is_empty() {
        NO_STANDARD_WEAPONS
    } else {
        STANDARD_WEAPONS
    }
}

/// A row's grey label and white value at `baseline`, in the left column
/// (0) or the right (1).
fn labelled(column: usize, baseline: f32, label: &str, value: String) -> [PanelText; 2] {
    [
        PanelText {
            dx: PANEL_LABEL_COLUMNS[column],
            baseline,
            text: label.to_owned(),
            color: LABEL_COLOR,
        },
        PanelText {
            dx: PANEL_VALUE_COLUMNS[column],
            baseline,
            text: value,
            color: TEXT_COLOR,
        },
    ]
}

/// The info panel's texts for a ship of `specs`, in the order
/// `_ShipDescDialogUpdate` draws them (@0x4fb4a-0x50f17): Speed, Accel,
/// Turn, Shields, Armor, Guns and Turrets in the left column; Space,
/// Cargo and Energy in the right; the stock-weapons list; then Length,
/// Mass and Crew in the right column. Each row is a grey label and a
/// white value; the list is white.
#[must_use]
pub fn spec_lines(specs: &ShipSpecs) -> Vec<PanelText> {
    panel_texts(specs, true)
}

/// The info panel's item-5 texts for a ship of `specs` in `variant`:
/// [`spec_lines`] for [`InfoVariant::Plain`], and without the
/// stock-weapons heading and lines while the ID test holds, as the
/// weapons go into item 8 then (@0x50589-0x50590, @0x50613-0x5061a,
/// @0x50716).
#[must_use]
pub fn spec_lines_for(specs: &ShipSpecs, variant: InfoVariant) -> Vec<PanelText> {
    panel_texts(specs, !variant.id_test())
}

/// The stock weapons as one sentence, as `_ShipDescDialogUpdate` builds
/// it while the ID test holds (@0x50786-0x50c41): each weapon's line
/// ([`stock_text`]) in order, joined by [`WEAPONS_SEPARATOR`]; empty
/// with none.
#[must_use]
pub fn weapons_sentence(weapons: &[StockLine]) -> String {
    weapons
        .iter()
        .map(stock_text)
        .collect::<Vec<_>>()
        .join(WEAPONS_SEPARATOR)
}

/// [`spec_lines`]' texts, with the stock-weapons heading and lines only
/// when `with_weapons`.
fn panel_texts(specs: &ShipSpecs, with_weapons: bool) -> Vec<PanelText> {
    const LEFT: usize = 0;
    const RIGHT: usize = 1;
    let [speed, accel, turn, guns, turrets, space, length, mass, crew] = STAT_LABELS;
    let [r12, r24, r36, r48, r60, r72, r84] = PANEL_ROWS;
    let or_none = |value: Option<String>| value.unwrap_or_else(|| NONE.to_owned());
    let maximum = |value: Option<i64>| or_none(value.map(|n| format!("{MAXIMUM_OF} {n}")));
    let comma = |value: Option<i64>| or_none(value.map(credits::comma_number));
    let jumps = specs.jumps_shown().map(|jumps| {
        let unit = if jumps == 1 { JUMP } else { JUMPS };
        format!("{jumps} {unit}")
    });
    let mut texts: Vec<PanelText> = [
        labelled(LEFT, r12, speed, specs.speed_shown().to_string()),
        labelled(
            LEFT,
            r24,
            accel,
            rating_text(specs.accel_shown()).to_owned(),
        ),
        labelled(LEFT, r36, turn, rating_text(specs.turn_shown()).to_owned()),
        labelled(LEFT, r48, SHIELDS_LABEL, comma(specs.shields_shown())),
        labelled(LEFT, r60, ARMOR_LABEL, comma(specs.armor_shown())),
        labelled(LEFT, r72, guns, maximum(specs.guns_shown())),
        labelled(LEFT, r84, turrets, maximum(specs.turrets_shown())),
        labelled(RIGHT, r12, space, tons_text(specs.space_shown())),
        labelled(RIGHT, r24, CARGO_LABEL, tons_text(specs.cargo_shown())),
        labelled(RIGHT, r36, ENERGY_LABEL, or_none(jumps)),
    ]
    .into_iter()
    .flatten()
    .collect();
    if with_weapons {
        texts.push(PanelText {
            dx: PANEL_LABEL_COLUMNS[LEFT],
            baseline: WEAPONS_ROW,
            text: weapons_heading(&specs.weapons).to_owned(),
            color: TEXT_COLOR,
        });
        let mut baseline = FIRST_WEAPON_ROW;
        for line in &specs.weapons {
            texts.push(PanelText {
                dx: WEAPON_INDENT,
                baseline,
                text: stock_text(line),
                color: TEXT_COLOR,
            });
            baseline += WEAPON_STEP;
        }
    }
    texts.extend(labelled(
        RIGHT,
        r48,
        length,
        format!("{} {METRES}", specs.length),
    ));
    texts.extend(labelled(RIGHT, r60, mass, tons_text(specs.mass_shown())));
    if let Some(shown) = specs.crew_shown() {
        texts.extend(labelled(RIGHT, r72, crew, shown.to_string()));
    }
    texts
}

/// What the shipyard reads beyond the spaceport's port: each `shïp`'s
/// `shän` base image, raw.
pub trait ShipBaseImages {
    /// Each readable `shän`'s ship (`shän` ID = `shïp` ID) with its
    /// `BaseImageID`; an unreadable one is left out.
    fn ship_base_images(&self) -> Vec<(ShipId, i16)>;
}

/// What the shipyard reads about a ship: its description, through
/// [`DescriptionSource`], the graphic it names, through
/// [`DescriptionGraphics`], whether a picture exists, through
/// [`SpaceportCatalog::picture_exists`], and the ships' base images,
/// through [`ShipBaseImages`], and the grid's colours, through
/// [`GridLooks`]. Anything that is all five is one.
pub trait ShipyardCatalog:
    SpaceportCatalog + DescriptionSource + DescriptionGraphics + ShipBaseImages + GridLooks
{
}

impl<
    T: SpaceportCatalog
        + DescriptionSource
        + DescriptionGraphics
        + ShipBaseImages
        + GridLooks
        + ?Sized,
> ShipyardCatalog for T
{
}

/// The catalog, shared, with a `Debug` that shows nothing of it.
#[derive(Clone)]
struct CatalogHandle(Rc<dyn ShipyardCatalog>);

impl std::fmt::Debug for CatalogHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ShipyardCatalog")
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

/// The shipyard, laid out.
#[derive(Clone, Debug)]
struct Laid {
    dialog: Dialog,
    metrics: MetricsHandle,
    /// The ship whose picture and description are shown, and them.
    shown: Option<(ShipId, Shown)>,
    /// The info panel's templates.
    info: InfoTemplates,
    /// The name prompt's template, if there is one.
    text_input: Option<DialogTemplate>,
}

/// A ship to buy, and the name the player confirmed for it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShipOrder {
    /// The ship class.
    pub ship: ShipId,
    /// The name, as typed.
    pub name: String,
}

/// The info panel, open: "Shipyard Info" of its [`InfoVariant`] over
/// that variant's background, the ship's name centred in its title, its
/// lines in its text and, for [`InfoVariant::Picture`], its graphic in
/// item 7. The hire screen opens it too.
#[derive(Clone, Debug)]
pub(super) struct Panel {
    dialog: Dialog,
    variant: InfoVariant,
    title: String,
    /// Item 5's texts ([`spec_lines_for`]).
    texts: Vec<PanelText>,
    /// While the ID test holds, item 8's weapons: the heading
    /// ([`weapons_heading`]) and, when any stock weapon is listed, the
    /// sentence ([`weapons_sentence`]).
    weapons: Option<(&'static str, Option<String>)>,
}

impl Panel {
    /// The panel of `variant` laid out by `template`, titled `title` and
    /// drawing `texts` in item 5 and `weapons` in item 8, its Done
    /// enabled and labelled in `style`, its text measured by `metrics`.
    fn open(
        template: &DialogTemplate,
        variant: InfoVariant,
        title: String,
        texts: Vec<PanelText>,
        weapons: Option<(&'static str, Option<String>)>,
        metrics: &Rc<dyn TextMetrics>,
        style: ButtonStyle,
    ) -> Self {
        let roles = [(PANEL_DONE_ITEM, Role::Button(DONE_LABEL.to_owned()))];
        let mut template = template.clone();
        if let Some(done) = template.items.get_mut(PANEL_DONE_ITEM - 1) {
            done.enabled = true;
        }
        let dialog = Dialog::new(&template, &roles, Rc::clone(metrics))
            .with_buttons(ButtonSkin::NOVA, style)
            .with_default(Some(PANEL_DONE_ITEM))
            .with_cancel(Some(PANEL_DONE_ITEM));
        Self {
            dialog,
            variant,
            title,
            texts,
            weapons,
        }
    }

    /// Its dialog.
    pub(super) fn dialog(&self) -> &Dialog {
        &self.dialog
    }

    /// Its input, silent under the pointer as `_TrackShipyardButtons`
    /// @0x2a54e: whether Done, Return or Escape closed it.
    pub(super) fn input(&mut self, input: &Input) -> bool {
        self.dialog.input(input) == Some(DialogEvent::Item(PANEL_DONE_ITEM))
    }

    /// Abandons any click in progress on it.
    pub(super) fn cancel_pointer(&mut self) {
        self.dialog.cancel_pointer();
    }

    /// Draws it, its title measured by `metrics`: the background, drawn
    /// into the dialog's bounds as `_DrawPicHandle` stretches it into the
    /// port (@0x4f952-0x4f971), its title, the dialog, item 5's texts,
    /// and, after Done, the graphic stretched into item 7
    /// (@0x50fd0-0x51043).
    pub(super) fn draw(&self, list: &mut DrawList, metrics: &dyn TextMetrics) {
        let bounds = self.dialog.bounds();
        if self.variant == InfoVariant::MissingPicture {
            self.dialog
                .draw_stretched_over(list, self.variant.background());
        } else {
            list.picture(self.variant.background(), bounds.min);
        }
        if let Some(title) = self.dialog.item_bounds(PANEL_TITLE_ITEM) {
            let style = TextStyle {
                font: TITLE_FONT,
                size: TITLE_SIZE,
                color: TEXT_COLOR,
            };
            grid::draw_centred_text_in(
                list,
                metrics,
                style,
                title,
                title.min.y + TITLE_BASELINE,
                &self.title,
            );
        }
        self.dialog.draw(list);
        // The original clips to item 5 (`ClipRect` @0x4fb39); with no
        // clip here, a text whose top is at or below its bottom is not
        // drawn.
        if let Some(area) = self.dialog.item_bounds(PANEL_TEXT_ITEM) {
            for shown in &self.texts {
                let top = grid::baseline_top(area.min.y + shown.baseline);
                if top < area.max.y {
                    text(
                        list,
                        &shown.text,
                        Point::new(area.min.x + shown.dx, top),
                        shown.color,
                    );
                }
            }
        }
        if let (Some((heading, sentence)), Some(area)) =
            (&self.weapons, self.dialog.item_bounds(PANEL_WEAPONS_ITEM))
        {
            draw_weapons(list, metrics, area, heading, sentence.as_deref());
        }
        if let Some(id) = self.variant.graphic() {
            self.dialog.draw_picture_in(list, PANEL_GRAPHIC_ITEM, id);
        }
    }
}

/// Item 8's weapons, in `area`, as `_ShipDescDialogUpdate` draws them
/// while the ID test holds: the heading in white at its left on baseline
/// [`WEAPONS_HEADING_ROW`] (@0x5061c-0x5069b); then, with a `sentence`
/// (a weapon is listed), the box below [`WEAPONS_BOX_DROP`] (@0x5066d)
/// holding it, set by `TETextBox` and inverted (@0x50c73-0x50ce0): white
/// on black, wrapped at word breaks to its width through a [`TextBox`]:
/// every line whose top is inside the box, the one straddling its bottom
/// showing its top, clipped to the box. With none listed there is no box
/// (@0x50602).
fn draw_weapons(
    list: &mut DrawList,
    metrics: &dyn TextMetrics,
    area: Bounds,
    heading: &str,
    sentence: Option<&str>,
) {
    let top = grid::baseline_top(area.min.y + WEAPONS_HEADING_ROW);
    text(list, heading, Point::new(area.min.x, top), TEXT_COLOR);
    let Some(sentence) = sentence else {
        return;
    };
    let boxed = Bounds {
        min: Point::new(area.min.x, area.min.y + WEAPONS_BOX_DROP),
        max: area.max,
    };
    fill_rect(list, boxed, Color::BLACK);
    TextBox::new(sentence, TEXT_FONT, TEXT_SIZE, TEXT_COLOR, boxed, &metrics).draw(list);
}

/// The info panel on a class named `name`, of `specs`, whose description
/// is `dësc` `description`, laid out by `info`'s template for the
/// variant its `Graphic` picks ([`info_variant`]), read from `catalog`
/// when it opens. It is titled with the `Long Name` while the ID test
/// holds, and `name` otherwise (@0x4fa33-0x4fa9d). None when that
/// variant's dialog is missing, as nothing opens then (@0x5b749-0x5b752).
pub(super) fn open_panel(
    info: &InfoTemplates,
    catalog: &dyn ShipyardCatalog,
    description: Option<i16>,
    name: &str,
    specs: &ShipSpecs,
    metrics: &Rc<dyn TextMetrics>,
    style: ButtonStyle,
) -> Option<Panel> {
    let graphic = description.and_then(|id| catalog.description_graphic(id));
    let variant = info_variant(graphic, |id| catalog.picture_exists(id));
    let template = info.of(variant)?;
    let title = if variant.id_test() {
        specs.long_name.clone()
    } else {
        name.to_owned()
    };
    let weapons = variant.id_test().then(|| {
        let listed = !specs.weapons.is_empty();
        (
            weapons_heading(&specs.weapons),
            listed.then(|| weapons_sentence(&specs.weapons)),
        )
    });
    Some(Panel::open(
        template,
        variant,
        title,
        spec_lines_for(specs, variant),
        weapons,
        metrics,
        style,
    ))
}

/// The info screens the shipyard has asked for, until taken.
#[derive(Clone, Copy, Debug, Default)]
struct InfoAsks {
    /// Whether [`PLAYER_INFO_KEY`](crate::ui::player_info::PLAYER_INFO_KEY)
    /// asked for the player info screen.
    player_info: bool,
    /// Whether [`MISSION_INFO_KEY`](crate::ui::mission_info::MISSION_INFO_KEY)
    /// asked for the mission info screen.
    mission_info: bool,
    /// The map the map key asked for: the shipyard's, or the Info
    /// panel's.
    map: Option<PortScreen>,
}

/// The Shipyard of the stellar landed on.
#[derive(Clone, Debug)]
pub struct ShipyardScreen {
    /// The dialog, or why it cannot be shown.
    laid: Result<Laid, String>,
    shipyard: Shipyard,
    catalog: CatalogHandle,
    /// The grid's frame colours, read from the catalog once.
    colors: GridColors,
    /// Each ship's base image, read once.
    bases: Vec<(ShipId, i16)>,
    /// Each listed ship's picture, by row, read once a list (none without
    /// the dialog).
    pictures: Vec<Option<i16>>,
    style: ButtonStyle,
    /// The grid, its selection as `_ShipyardFilter` keeps it.
    grid: ShipyardGrid,
    /// The info panel, while it is open.
    panel: Option<Panel>,
    /// The ship requested, until it is taken, and whether B asked.
    request: Option<(ShipId, bool)>,
    /// Whether the request last taken was asked by B.
    asked_by_key: bool,
    /// The name prompt, while it is open, and the ship it names.
    naming: ModalHost<ShipId, TextInputDialog>,
    /// The ship ordered, named, until it is taken.
    order: Option<ShipOrder>,
    /// The ship declined at its name prompt, until it is taken.
    declined: Option<ShipId>,
    /// The mission offers, once given.
    offering: Option<VenueOffers>,
    /// The info screens asked for, until taken.
    asks: InfoAsks,
    closed: bool,
    /// The `dësc` shown since this was last taken, to be loaded.
    description_request: Option<i16>,
    sounds: Vec<Sound>,
}

impl ShipyardScreen {
    /// The shipyard `shipyard`, laid out by `layout`: the "Shipyard"
    /// dialog's template and the metrics its text is measured by, or why
    /// there are none; the info panel by `info`, "Shipyard Info" (Info is
    /// greyed without it); and the name prompt by `text_input`, "Text
    /// Input" (or else its fallback). Its buttons are labelled in `style`, and each
    /// ship's picture and description read from `catalog`, which gives
    /// the ships' base images once, now. By the engine nothing is
    /// selected (`_DoShipyardDialog` @0x5e6d8-0x5e705); by the other
    /// reading of [`RuleKey::GridSelection`](nova_sim::RuleKey::GridSelection),
    /// the first cell is.
    #[must_use]
    pub fn new(
        layout: Result<(DialogTemplate, Rc<dyn TextMetrics>), String>,
        info: InfoTemplates,
        text_input: Result<DialogTemplate, String>,
        shipyard: Shipyard,
        catalog: Rc<dyn ShipyardCatalog>,
        style: ButtonStyle,
    ) -> Self {
        let laid = layout.map(|(mut template, metrics)| {
            for item in [
                DONE_ITEM,
                BUY_ITEM,
                INFO_ITEM,
                SCROLL_UP_ITEM,
                SCROLL_DOWN_ITEM,
            ] {
                if let Some(found) = template.items.get_mut(item - 1) {
                    found.enabled = true;
                }
            }
            let roles = [
                (DONE_ITEM, Role::Button(DONE_LABEL.to_owned())),
                (BUY_ITEM, Role::Button(BUY_SHIP_LABEL.to_owned())),
                (INFO_ITEM, Role::Button(INFO_LABEL.to_owned())),
            ];
            let mut dialog = Dialog::new(&template, &roles, Rc::clone(&metrics))
                .with_buttons(ButtonSkin::NOVA, style)
                .with_default(Some(DONE_ITEM))
                .with_cancel(Some(DONE_ITEM));
            dialog.set_greyed(INFO_ITEM, info.plain.is_err());
            Laid {
                dialog,
                metrics: MetricsHandle(metrics),
                shown: None,
                info,
                text_input: text_input.ok(),
            }
        });
        let bases = if laid.is_ok() {
            catalog.ship_base_images()
        } else {
            Vec::new()
        };
        let grid = ShipyardGrid::open(&shipyard.rules, shipyard.rows.len());
        let mut screen = Self {
            laid,
            shipyard,
            colors: catalog.grid_colors(),
            catalog: CatalogHandle(catalog),
            bases,
            pictures: Vec::new(),
            style,
            grid,
            panel: None,
            request: None,
            asked_by_key: false,
            naming: ModalHost::default(),
            order: None,
            declined: None,
            offering: None,
            asks: InfoAsks::default(),
            closed: false,
            description_request: None,
            sounds: Vec::new(),
        };
        screen.read_pictures();
        screen.selected_changed();
        screen
    }

    /// The shipyard with mission offers, each shown in the offer dialog of
    /// its variant laid out by `templates` (or saying why there is none): it asks for the
    /// first as it opens, and again once rearmed ([`ShipyardScreen::rearm`]), as
    /// the original's does ([`SHIPYARD`](nova_sim::mission::screens::SHIPYARD)).
    #[must_use]
    pub fn with_offers(self, templates: OfferTemplates) -> Self {
        Self {
            offering: Some(VenueOffers::asking(templates)),
            ..self
        }
    }

    /// Its offers' timer running as `clock` reads
    /// [`RuleKey::OfferClock`](nova_sim::RuleKey::OfferClock), for offers
    /// given ([`ShipyardScreen::with_offers`]) before it.
    #[must_use]
    pub fn with_offer_clock(self, clock: RuleSource) -> Self {
        Self {
            offering: self.offering.map(|offering| offering.with_clock(clock)),
            ..self
        }
    }

    /// Whether [`PLAYER_INFO_KEY`](crate::ui::player_info::PLAYER_INFO_KEY) has asked for the player info screen
    /// since this was last asked, in the shipyard, its Info panel or its
    /// offer (`_ShipyardFilter` @0x6091a, `_ShipDescFilter` @0x51132),
    /// never at the name prompt, which takes P as a letter.
    pub fn take_player_info_request(&mut self) -> bool {
        let offer = self
            .offering
            .as_mut()
            .is_some_and(VenueOffers::take_player_info_request);
        std::mem::take(&mut self.asks.player_info) || offer
    }

    /// The map the map key (`_keyPrefs` +0x12, a press) asked for since
    /// this was last asked, once: the shipyard's (`_ShipyardFilter` item 3,
    /// @0x608ed-0x6090b), or the ship description's while the Info panel
    /// is open (`_ShipDescFilter` item 4, @0x51108-0x51126). Never at the
    /// name prompt, which takes M as a letter, and never without the
    /// dialog. Whoever holds the session opens it over the spaceport.
    pub fn take_map_request(&mut self) -> Option<PortScreen> {
        self.asks.map.take()
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

    /// Whether the shipyard has asked for an offer since this was last
    /// asked: whoever holds the session answers with
    /// [`ShipyardScreen::open_offer`], or, with none, rearms it
    /// ([`ShipyardScreen::rearm`]).
    pub fn take_offer_request(&mut self) -> bool {
        self.offering
            .as_mut()
            .is_some_and(VenueOffers::take_request)
    }

    /// Opens the offer dialog over the shipyard on `prompt`.
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
    /// the shipyard. The time since the last press on the grid runs on
    /// too, as `_ShipyardFilter` measures it by the event's clock.
    pub(crate) fn tick_covered(&mut self, dt: Duration) {
        self.grid.advance(dt);
        if let Some(offering) = &mut self.offering {
            offering.tick(dt, true);
        }
    }

    /// The shipyard shown.
    #[must_use]
    pub fn shipyard(&self) -> &Shipyard {
        &self.shipyard
    }

    /// The selected cell's index: none when nothing is listed, or, by
    /// [`RuleKey::GridSelection`](nova_sim::RuleKey::GridSelection)'s
    /// engine reading, when nothing is selected (on opening, after a press
    /// on an empty cell or a scroll that moved).
    #[must_use]
    pub fn selected(&self) -> Option<usize> {
        self.grid.cells().selected(self.shipyard.rows.len())
    }

    /// The selected ship's row, if one is selected
    /// ([`ShipyardScreen::selected`]).
    fn selected_row(&self) -> Option<&ShipRow> {
        self.selected().map(|index| &self.shipyard.rows[index])
    }

    /// The grid's first row shown.
    #[must_use]
    pub fn top_row(&self) -> usize {
        self.grid.cells().top_row()
    }

    /// The dialog, if it could be laid out.
    #[must_use]
    pub fn dialog(&self) -> Option<&Dialog> {
        self.laid.as_ref().ok().map(|laid| &laid.dialog)
    }

    /// The info panel's dialog, while it is open.
    #[must_use]
    pub fn info_panel(&self) -> Option<&Dialog> {
        self.panel.as_ref().map(Panel::dialog)
    }

    /// Why the shipyard cannot be shown, if it cannot.
    #[must_use]
    pub fn problem(&self) -> Option<&str> {
        self.laid.as_ref().err().map(String::as_str)
    }

    /// Whether Done has been activated.
    #[must_use]
    pub fn closed(&self) -> bool {
        self.closed
    }

    /// The ship requested with Buy Ship since it was last taken, once.
    pub fn take_request(&mut self) -> Option<ShipId> {
        let (ship, by_key) = self.request.take()?;
        self.asked_by_key = by_key;
        Some(ship)
    }

    /// Opens the name prompt for `naming`'s ship over the shipyard, laid
    /// out by the "Text Input" dialog or its fallback. When B asked for
    /// it, the B typed is dropped. Nothing opens when the shipyard could
    /// not be laid out.
    pub fn open_naming(&mut self, naming: &ShipNaming) {
        let Ok(laid) = &self.laid else {
            return;
        };
        let metrics = Rc::clone(&laid.metrics.0);
        let built = laid.text_input.as_ref().and_then(|template| {
            TextInputDialog::new(
                template,
                &naming.prompt,
                &naming.default,
                SHIP_NAME_CHARS,
                self.style,
                Rc::clone(&metrics),
            )
            .ok()
        });
        let mut prompt = built.unwrap_or_else(|| {
            TextInputDialog::fallback(
                &naming.prompt,
                &naming.default,
                SHIP_NAME_CHARS,
                self.style,
                metrics,
            )
        });
        if self.asked_by_key {
            prompt.flush_typed_key();
        }
        self.naming.open(naming.ship, prompt);
    }

    /// The name prompt, while it is open.
    #[must_use]
    pub fn naming(&self) -> Option<&TextInputDialog> {
        self.naming.dialog()
    }

    /// The `dësc` of the ship the shipyard has shown since this was last
    /// taken, once, read raw until whoever flies the ship loads it as
    /// `_ShipyardFilter` does (@0x607fc, @0x60c33) and answers with
    /// `show_description`, in the same input.
    pub fn take_description_request(&mut self) -> Option<i16> {
        self.description_request.take()
    }

    /// Shows `text`, `dësc` `id` loaded, as the description of the ship
    /// shown, while it is the ship whose `dësc` that is.
    pub fn show_description(&mut self, id: i16, text: &str) {
        let Ok(Laid {
            dialog,
            metrics,
            shown: Some((ship, shown)),
            ..
        }) = &mut self.laid
        else {
            return;
        };
        if nova_data::ship_desc_id(*ship) == Some(id) {
            let area = dialog.item_bounds(DESCRIPTION_ITEM).unwrap_or(Bounds::at(
                Point::new(0.0, 0.0),
                0.0,
                0.0,
            ));
            shown.retext(text, area, &metrics.0);
        }
    }

    /// The ship ordered at its name prompt since it was last taken, once.
    pub fn take_order(&mut self) -> Option<ShipOrder> {
        self.order.take()
    }

    /// The ship declined at its name prompt since it was last taken, once.
    pub fn take_declined(&mut self) -> Option<ShipId> {
        self.declined.take()
    }

    /// Where cell `index` (counted from the grid's first row shown) is,
    /// if the grid could be laid out.
    #[must_use]
    pub fn cell_bounds(&self, index: usize) -> Option<Bounds> {
        let grid = self.dialog()?.item_bounds(GRID_ITEM)?;
        Some(grid::cell_bounds(grid, index))
    }

    /// Shows `shipyard`, the shipyard after an order, keeping the same
    /// ship selected when it is still listed, or the cell where it was;
    /// with nothing selected, nothing stays selected.
    pub fn set_shipyard(&mut self, shipyard: Shipyard) {
        let kept = self
            .selected_row()
            .map(|row| row.id)
            .and_then(|id| shipyard.rows.iter().position(|row| row.id == id));
        self.shipyard = shipyard;
        self.read_pictures();
        if self.grid.cells().highlighted() {
            self.select(kept.unwrap_or(self.grid.cells().selected_raw()));
        } else {
            self.selected_changed();
        }
    }

    /// Reads each listed ship's picture, when there is a dialog to show
    /// them in.
    fn read_pictures(&mut self) {
        if self.laid.is_err() {
            return;
        }
        let catalog = &self.catalog.0;
        self.pictures = self
            .shipyard
            .rows
            .iter()
            .map(|row| ship_picture(row.id, &self.bases, |id| catalog.picture_exists(id)))
            .collect();
    }

    /// Selects cell `index`, or the last when there is no such cell, and
    /// scrolls the grid to show it.
    fn select(&mut self, index: usize) {
        self.grid.select(index, self.shipyard.rows.len());
        self.selected_changed();
    }

    /// Greys Buy Ship when the selected ship cannot be bought, and Info
    /// with none selected (as `_DrawShipyardButtons` draws it,
    /// @0x2a3f7-0x2a405) or without its panel, and reads the ship's
    /// picture and description if they are not shown already. With none
    /// selected, none is shown (`_ShipyardDialogUpdate` @0x58fd0-0x58fda).
    fn selected_changed(&mut self) {
        let buys = self
            .selected_row()
            .is_some_and(|row| self.shipyard.can(row.id).is_ok());
        let selected = self.selected().map(|index| {
            let picture = self.pictures.get(index).copied().flatten();
            (self.shipyard.rows[index].id, picture)
        });
        let catalog = Rc::clone(&self.catalog.0);
        let Ok(laid) = &mut self.laid else {
            return;
        };
        laid.dialog.set_greyed(BUY_ITEM, !buys);
        laid.dialog
            .set_greyed(INFO_ITEM, laid.info.plain.is_err() || selected.is_none());
        let Some((ship, picture)) = selected else {
            laid.shown = None;
            return;
        };
        if laid.shown.as_ref().is_some_and(|(shown, _)| *shown == ship) {
            return;
        }
        let area = laid
            .dialog
            .item_bounds(DESCRIPTION_ITEM)
            .unwrap_or(Bounds::at(Point::new(0.0, 0.0), 0.0, 0.0));
        let description = nova_data::ship_desc_id(ship);
        self.description_request = description;
        let shown = Shown::read(&*catalog, picture, description, area, &laid.metrics.0);
        laid.shown = Some((ship, shown));
    }

    /// Requests the selected ship, if it can be bought, saying whether B
    /// asked; otherwise beeps ([`UiSound::BuyRefused`]), as
    /// `_DoShipyardDialog`'s item 1 does with no ship or one it cannot
    /// buy (@0x5e9bb-0x5e9d6, @0x5f22a-0x5f256).
    fn ask(&mut self, by_key: bool) {
        if let Some(row) = self
            .selected_row()
            .filter(|row| self.shipyard.can(row.id).is_ok())
        {
            self.request = Some((row.id, by_key));
        } else {
            self.sounds.push(Sound::Ui(UiSound::BuyRefused));
        }
    }

    /// Opens the info panel on the selected ship, if there is one and a
    /// panel to open.
    fn open_info(&mut self) {
        let Some(row) = self.selected_row() else {
            return;
        };
        let Ok(laid) = &self.laid else {
            return;
        };
        if laid.info.plain.is_err() {
            return;
        }
        self.panel = open_panel(
            &laid.info,
            &*self.catalog.0,
            nova_data::ship_desc_id(row.id),
            &row.name,
            &row.specs,
            &laid.metrics.0,
            self.style,
        );
    }

    /// Activates dialog item `item`: Done closes, Buy Ship asks, Info
    /// opens the panel, and the arrows scroll. Any other item does
    /// nothing.
    fn activate(&mut self, item: usize) {
        match item {
            DONE_ITEM => self.closed = true,
            BUY_ITEM => self.ask(false),
            INFO_ITEM => self.open_info(),
            SCROLL_UP_ITEM => self.scroll(false),
            SCROLL_DOWN_ITEM => self.scroll(true),
            _ => {}
        }
    }

    /// Scrolls the grid a row down, or up, never past either end; by the
    /// engine, a scroll that moves selects nothing and empties the
    /// description ([`ShipyardGrid::scroll`]).
    fn scroll(&mut self, down: bool) {
        if self.grid.scroll(down, self.shipyard.rows.len()) {
            self.selected_changed();
        }
    }

    /// A press on the grid at list index `index`, listed or not, and says
    /// whether the grid took it ([`ShipyardGrid::press`]): a listed cell is
    /// selected, and a double-click on it opens Info; an empty one selects
    /// nothing by the engine and is left to the dialog by the other
    /// reading.
    fn press_cell(&mut self, index: usize) -> bool {
        match self.grid.press(index, self.shipyard.rows.len()) {
            PressOutcome::Selected { double } => {
                self.selected_changed();
                if double {
                    self.open_info();
                }
            }
            PressOutcome::Cleared => self.selected_changed(),
            PressOutcome::FallThrough => return false,
        }
        true
    }

    /// The list index under `at` on the grid, listed or not
    /// ([`CellGrid::cell_under`](grid::CellGrid::cell_under)).
    fn cell_under(&self, at: Point) -> Option<usize> {
        let grid = self.dialog()?.item_bounds(GRID_ITEM)?;
        self.grid.cells().cell_under(grid, at)
    }

    /// The info panel's input: Done, Return and Escape close it.
    fn panel_input(&mut self, input: &Input) {
        let Some(panel) = &mut self.panel else {
            return;
        };
        if panel.input(input) {
            self.panel = None;
        }
    }

    /// The shown cells, each its picture, the current mark and its name,
    /// then its frame in `GridDim`; then, last, the selected cell framed
    /// again in `GridBright`. No cell is filled (`_ShipyardDialogUpdate`
    /// @0x58b30-0x58f91, @0x58f9d-0x58fbf).
    fn draw_grid(&self, laid: &Laid, list: &mut DrawList) {
        let metrics = &*laid.metrics.0;
        let Some(area) = laid.dialog.item_bounds(GRID_ITEM) else {
            return;
        };
        if self.shipyard.rows.is_empty() {
            let origin = Point::new(area.min.x + INSET, area.min.y + INSET);
            text(list, NO_SHIPS, origin, TEXT_COLOR);
            return;
        }
        let count = self.shipyard.rows.len();
        let selected = self.selected();
        self.grid.cells().draw_cells(
            list,
            area,
            count,
            selected,
            self.colors,
            |list, index, cell| {
                let row = &self.shipyard.rows[index];
                grid::draw_cell_picture(list, cell, self.pictures[index]);
                if row.id == self.shipyard.current {
                    grid::draw_corner_text(list, metrics, cell, CURRENT_MARK, false);
                }
                grid::draw_cell_name(list, metrics, cell, &row.short_name);
            },
        );
    }

    fn draw_selected(&self, laid: &Laid, list: &mut DrawList) {
        let Some(row) = self.selected_row() else {
            return;
        };
        if let (Some((_, shown)), Some(area)) = (&laid.shown, laid.dialog.item_bounds(PICTURE_ITEM))
        {
            shown.draw(list, area, &*laid.metrics.0);
        }
        let Some(info) = laid.dialog.item_bounds(INFO_BOX_ITEM) else {
            return;
        };
        let rows = PRICE_ROWS
            .into_iter()
            .zip(price_lines(&self.shipyard, row))
            .map(|(baseline, (label, amount))| (baseline, label, amount));
        draw_amount_rows(list, info, rows);
    }

    fn draw_panel(&self, laid: &Laid, list: &mut DrawList) {
        if let Some(panel) = &self.panel {
            panel.draw(list, &*laid.metrics.0);
        }
    }
}

impl Screen for ShipyardScreen {
    /// Every input goes to the shipyard, or to its name prompt or info
    /// panel while that is open; it never quits.
    fn input(&mut self, input: &Input) -> ScreenAction {
        if let Some(offering) = &mut self.offering
            && offering.input(input, &mut self.sounds)
        {
            return ScreenAction::None;
        }
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
        if self.naming.is_open() {
            // OK orders the ship, and Cancel declines it.
            if let Some((ship, outcome)) = self.naming.input(input, &mut self.sounds) {
                match outcome {
                    TextInputOutcome::Confirm(name) => {
                        self.order = Some(ShipOrder { ship, name });
                    }
                    TextInputOutcome::Cancel => self.declined = Some(ship),
                }
            }
            return ScreenAction::None;
        }
        if is_map_press(input) {
            self.asks.map = Some(if self.panel.is_some() {
                PortScreen::ShipDescription
            } else {
                PortScreen::Shipyard
            });
            return ScreenAction::None;
        }
        if is_player_info_press(input) {
            self.asks.player_info = true;
            return ScreenAction::None;
        }
        if is_mission_info_press(input) {
            self.asks.mission_info = true;
            return ScreenAction::None;
        }
        if self.panel.is_some() {
            self.panel_input(input);
            return ScreenAction::None;
        }
        let count = self.shipyard.rows.len();
        let arrow = match *input {
            Input::Key {
                key, pressed: true, ..
            } => self.grid.arrow(key, count),
            _ => false,
        };
        let pressed_once = |wanted: Key| matches!(*input, Input::Key { key, pressed: true, repeat: false } if key == wanted);
        let on_grid = match *input {
            Input::PointerButton {
                pressed: true, at, ..
            } => self
                .cell_under(at)
                .is_some_and(|index| self.press_cell(index)),
            _ => false,
        };
        if arrow {
            self.selected_changed();
        } else if pressed_once(BUY_KEY) {
            self.ask(true);
        } else if pressed_once(INFO_KEY) {
            self.open_info();
        } else if on_grid {
            // The grid took the press.
        } else if let Ok(laid) = &mut self.laid {
            let event = laid.dialog.input(input);
            if let Some(DialogEvent::Item(item)) = event {
                self.activate(item);
            }
        }
        ScreenAction::None
    }

    /// The time since the last press on the grid runs on, and the wait
    /// for the next offer runs down, the Info panel or the name prompt
    /// covering the shipyard while either is open.
    fn tick(&mut self, dt: Duration) {
        self.grid.advance(dt);
        let covered = self.panel.is_some() || self.naming.is_open();
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
                        "Cannot show the shipyard: {reason}. Press Return or Escape to close it."
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
        let count = self.shipyard.rows.len();
        grid::draw_arrows(
            list,
            laid.dialog.item_bounds(SCROLL_UP_ITEM),
            laid.dialog.item_bounds(SCROLL_DOWN_ITEM),
            self.grid.cells().can_scroll(count),
        );
        laid.dialog.draw(list);
        self.draw_panel(laid, list);
        self.naming.draw(list);
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
        if self.naming.cancel_pointer() {
            return;
        }
        if let Some(panel) = &mut self.panel {
            panel.cancel_pointer();
        } else if let Ok(laid) = &mut self.laid {
            laid.dialog.cancel_pointer();
        }
    }

    /// The offer's and the naming dialog's sounds, in order. The buttons
    /// are silent under the pointer, as `_TrackShipyardButtons` @0x2a54e
    /// plays nothing.
    fn take_sounds(&mut self) -> Vec<Sound> {
        std::mem::take(&mut self.sounds)
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::cell::RefCell;

    use nova_sim::{RuleSource, Rulebook, ShipFacts, ShipFields, ShipSpecs, StockLine};

    use super::*;
    use crate::draw::{DrawCommand, frame_rect};
    use crate::flight::view::MAP_KEY;
    use crate::input::MouseButton;
    use crate::sound::UiSound;
    use crate::spaceport::catalog::{PortRecord, StellarId};
    use crate::spaceport::credits::AMOUNT_COLUMN;
    use crate::spaceport::grid::GREY;
    use crate::spaceport::grid::TEXT_SIZE;
    use crate::text::fixture::MonoMetrics;
    use crate::ui::dialog::{ItemSpec, ItemTemplate, Placement};

    fn rect(x: f32, y: f32, w: f32, h: f32) -> Bounds {
        Bounds::at(Point::new(x, y), w, h)
    }

    /// `items`, each (x, y, w, h, enabled), as user items.
    fn items(items: &[(f32, f32, f32, f32, bool)]) -> Vec<ItemTemplate> {
        items
            .iter()
            .map(|&(x, y, w, h, enabled)| ItemTemplate {
                bounds: rect(x, y, w, h),
                enabled,
                kind: ItemSpec::User,
            })
            .collect()
    }

    /// Stock "Shipyard": `DLOG` 1004, 765 x 323 and centred, and its
    /// thirteen user items, where 2, 3, 4 and 11 are parked outside it.
    fn template() -> DialogTemplate {
        DialogTemplate {
            bounds: rect(100.0, 100.0, 765.0, 323.0),
            placement: Placement::Center,
            items: items(&[
                (365.0, 289.0, 109.0, 25.0, true),
                (248.0, 440.0, 68.0, 30.0, false),
                (251.0, 491.0, 29.0, 21.0, false),
                (394.0, 489.0, 99.0, 25.0, false),
                (9.0, 8.0, 333.0, 271.0, true),
                (354.0, 10.0, 192.0, 267.0, false),
                (480.0, 289.0, 109.0, 25.0, false),
                (557.0, 8.0, 200.0, 200.0, false),
                (614.0, 214.0, 143.0, 100.0, false),
                (253.0, 289.0, 89.0, 25.0, false),
                (170.0, 449.0, 32.0, 32.0, false),
                (141.0, 288.0, 25.0, 25.0, false),
                (171.0, 288.0, 25.0, 25.0, false),
            ]),
        }
    }

    /// Stock "Shipyard Info": `DLOG` 1005, 250 x 285 and centred, with
    /// Done (1), the title (3) and the text (5).
    fn info_template() -> DialogTemplate {
        DialogTemplate {
            bounds: rect(0.0, 0.0, 250.0, 285.0),
            placement: Placement::Center,
            items: items(&[
                (86.0, 253.0, 74.0, 25.0, false),
                (300.0, 0.0, 1.0, 1.0, false),
                (3.0, 3.0, 240.0, 24.0, false),
                (300.0, 0.0, 1.0, 1.0, false),
                (9.0, 32.0, 234.0, 214.0, false),
            ]),
        }
    }

    /// Stock "Shipyard Info" with a picture: `DLOG` 1019, 614 x 537 and
    /// centred, with Done (1), the title (3), the text (5), the graphic
    /// (7) and the weapons (8); items 2, 4 and 6 parked outside it.
    fn picture_info_template() -> DialogTemplate {
        DialogTemplate {
            bounds: rect(0.0, 0.0, 614.0, 537.0),
            placement: Placement::Center,
            items: items(&[
                (503.0, 507.0, 99.0, 25.0, false),
                (700.0, 0.0, 1.0, 1.0, false),
                (7.0, 413.0, 600.0, 24.0, false),
                (700.0, 0.0, 1.0, 1.0, false),
                (11.0, 442.0, 256.0, 92.0, false),
                (700.0, 0.0, 1.0, 1.0, false),
                (7.0, 6.0, 600.0, 400.0, false),
                (266.0, 442.0, 342.0, 59.0, false),
            ]),
        }
    }

    /// Both info templates, stock.
    fn info_templates() -> InfoTemplates {
        InfoTemplates {
            plain: Ok(info_template()),
            picture: Ok(picture_info_template()),
        }
    }

    /// Where the dialog goes: (1024 - 765) / 2 and (768 - 323) / 2,
    /// floored.
    const ORIGIN: Point = Point::new(129.0, 222.0);

    /// Ship 128's picture (`PICT` 5000) exists, and 5002's; ships 128,
    /// 130 and 131 share base image 1000, ship 129 has its own, 1001.
    /// Every description is "dësc n.". Records what it is asked.
    struct FakeArt {
        pictures: Vec<i16>,
        bases: Vec<(ShipId, i16)>,
        /// Each `dësc` with a `Graphic`, and it.
        graphics: Vec<(i16, i16)>,
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

    impl DescriptionSource for FakeArt {
        fn description(&self, id: i16) -> Result<String, String> {
            self.asked.borrow_mut().push(format!("dësc {id}"));
            Ok(format!("dësc {id}."))
        }

        fn button_style(&self) -> ButtonStyle {
            panic!("the style is given")
        }
    }

    impl crate::spaceport::grid::GridLooks for FakeArt {
        fn grid_colors(&self) -> crate::spaceport::grid::GridColors {
            crate::spaceport::grid::TEST_GRID_COLORS
        }
    }

    impl ShipBaseImages for FakeArt {
        fn ship_base_images(&self) -> Vec<(ShipId, i16)> {
            self.asked.borrow_mut().push("bases".to_owned());
            self.bases.clone()
        }
    }

    impl DescriptionGraphics for FakeArt {
        fn description_graphic(&self, id: i16) -> Option<i16> {
            self.asked.borrow_mut().push(format!("graphic {id}"));
            self.graphics
                .iter()
                .find(|(desc, _)| *desc == id)
                .map(|(_, graphic)| *graphic)
        }

        /// Names no movie: these screens never play one.
        fn description_movie(&self, _id: i16) -> Option<nova_sim::DescMovie> {
            None
        }
    }

    fn art() -> Rc<FakeArt> {
        art_with(Vec::new())
    }

    /// [`art`], with these `dësc` graphics.
    fn art_with(graphics: Vec<(i16, i16)>) -> Rc<FakeArt> {
        Rc::new(FakeArt {
            pictures: vec![5000, 5002],
            bases: vec![
                (ShipId(128), 1000),
                (ShipId(129), 1001),
                (ShipId(130), 1000),
                (ShipId(131), 1000),
            ],
            graphics,
            asked: RefCell::default(),
        })
    }

    /// The specs of a ship of speed 300, 12 tons free and of Space, 15
    /// tons of cargo, mass 25, fuel 300.
    const SPECS: ShipSpecs = ShipSpecs {
        fields: ShipFields {
            speed: 300,
            accel: 350,
            maneuver: 20,
            shield: 0,
            armor: 0,
            fuel: 300,
            fuel_regen: 0,
            holds: 15,
            mass: 25,
            free_mass: 12,
            contribute: 0,
            shield_rech: 0,
            armor_rech: 0,
            flags: 0,
            flags2: 0,
            ionize_max: 0,
            deionize: 0,
        },
        max_gun: 2,
        max_tur: 1,
        length: 26,
        crew: 4,
        space: 12,
        rounding: nova_sim::RuleSource::Engine,
        weapons: Vec::new(),
        long_name: String::new(),
    };

    fn row(id: i16, short_name: &str, price: i64) -> ShipRow {
        ShipRow {
            id: ShipId(id),
            name: format!("Ship {id}"),
            short_name: short_name.to_owned(),
            price,
            specs: SPECS,
            facts: ShipFacts { buyable: true },
        }
    }

    /// The Shuttle (flown), a Heavy Shuttle, a Courier too dear, and a
    /// Viper whose `Require` is unmet, with 25,000 credits and a 2,500
    /// trade-in.
    fn shipyard() -> Shipyard {
        Shipyard {
            rows: vec![
                row(128, "Shuttle", 10_000),
                row(129, "Heavy\\nShuttle", 17_500),
                row(130, "*Courier", 90_000),
                ShipRow {
                    facts: ShipFacts { buyable: false },
                    ..row(131, "Viper", 20_000)
                },
            ],
            trade_in: 2500,
            cash: 25_000,
            current: ShipId(128),
            rules: Rulebook::default(),
        }
    }

    fn layout() -> (DialogTemplate, Rc<dyn TextMetrics>) {
        (template(), Rc::new(MonoMetrics))
    }

    /// The shipyard as it opens, with nothing selected by the engine.
    fn opened_with(shipyard: Shipyard, art: &Rc<FakeArt>) -> ShipyardScreen {
        let catalog: Rc<dyn ShipyardCatalog> = Rc::clone(art) as Rc<dyn ShipyardCatalog>;
        ShipyardScreen::new(
            Ok(layout()),
            info_templates(),
            Ok(text_input_template()),
            shipyard,
            catalog,
            ButtonStyle::STOCK,
        )
    }

    /// The shipyard opened, then the first cell selected, as Right picks
    /// it from no selection (an arrow, so no press is remembered for a
    /// double-click).
    fn screen_with(shipyard: Shipyard, art: &Rc<FakeArt>) -> ShipyardScreen {
        let mut screen = opened_with(shipyard, art);
        if screen.selected().is_none() {
            press(&mut screen, Key::Right);
        }
        screen
    }

    fn screen_of(shipyard: Shipyard) -> ShipyardScreen {
        screen_with(shipyard, &art())
    }

    fn screen() -> ShipyardScreen {
        screen_of(shipyard())
    }

    fn drawn(screen: &ShipyardScreen) -> Vec<DrawCommand> {
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

    fn words(screen: &ShipyardScreen) -> Vec<String> {
        texts(&drawn(screen))
            .into_iter()
            .map(|(text, _, _)| text)
            .collect()
    }

    fn item(screen: &ShipyardScreen, number: usize) -> Bounds {
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

    fn press(screen: &mut ShipyardScreen, k: Key) {
        screen.input(&key(k, true, false));
    }

    fn click(screen: &mut ShipyardScreen, at: Point) {
        for pressed in [true, false] {
            screen.input(&Input::PointerButton {
                button: MouseButton::Left,
                pressed,
                at,
            });
        }
    }

    fn click_item(screen: &mut ShipyardScreen, number: usize) {
        let at = item(screen, number).center();
        click(screen, at);
    }

    fn click_cell(screen: &mut ShipyardScreen, index: usize) {
        let at = screen.cell_bounds(index).expect("a cell").center();
        click(screen, at);
    }

    /// Whether `label` draws enabled.
    fn enabled(screen: &ShipyardScreen, label: &str) -> bool {
        texts(&drawn(screen))
            .into_iter()
            .find_map(|(text, _, color)| (text == label).then_some(color))
            .is_some_and(|color| color != ButtonStyle::STOCK.grey)
    }

    /// The lines drawn in the info box.
    fn info_box(screen: &ShipyardScreen) -> Vec<String> {
        let info = item(screen, INFO_BOX_ITEM);
        texts(&drawn(screen))
            .into_iter()
            .filter(|(_, at, _)| info.contains(*at))
            .map(|(text, _, _)| text)
            .collect()
    }

    #[test]
    fn the_named_values() {
        assert_eq!(
            (SHIPYARD_DIALOG, SHIP_INFO_DIALOG, SHIP_PICTURE_INFO_DIALOG),
            (1004, 1005, 1019)
        );
        assert_eq!(BACKGROUND, ImageKey::picture(8501));
        assert_eq!(INFO_BACKGROUND, ImageKey::picture(8506));
        assert_eq!(PICTURE_INFO_BACKGROUND, ImageKey::picture(8507));
        assert_eq!((PANEL_GRAPHIC_ITEM, LAST_UNSHOWN_GRAPHIC), (7, 127));
        assert_eq!(
            (TITLE_FONT, TITLE_SIZE, TITLE_BASELINE),
            (Font::Times, 18.0, 18.0)
        );
        assert_eq!(
            (
                PANEL_WEAPONS_ITEM,
                WEAPONS_HEADING_ROW,
                WEAPONS_BOX_DROP,
                WEAPONS_SEPARATOR
            ),
            (8, 12.0, 16.0, ", ")
        );
        assert_eq!(
            [
                DONE_ITEM,
                GRID_ITEM,
                DESCRIPTION_ITEM,
                BUY_ITEM,
                PICTURE_ITEM,
                INFO_BOX_ITEM,
                INFO_ITEM,
                SCROLL_UP_ITEM,
                SCROLL_DOWN_ITEM
            ],
            [1, 5, 6, 7, 8, 9, 10, 12, 13]
        );
        assert_eq!(
            (PANEL_DONE_ITEM, PANEL_TITLE_ITEM, PANEL_TEXT_ITEM),
            (1, 3, 5)
        );
        assert_eq!((BUY_KEY, INFO_KEY), (Key::Char('b'), Key::Space));
        assert_eq!(FIRST_SHIP_PICTURE, 5000);
        assert_eq!(
            [BUY_SHIP_LABEL, INFO_LABEL, PRICE_LABEL, TRADE_IN_LABEL],
            ["Buy Ship", "Info", "Ship Price:", "Trade-In:"]
        );
        assert_eq!(FINAL_PRICE_LABEL, "Final Price:");
        assert_eq!(
            (AMOUNT_COLUMN, PRICE_ROWS),
            (70.0, [12.0, 24.0, 48.0, 72.0])
        );
        assert_eq!(NO_SHIPS, "There are no ships available for purchase here.");
        assert_eq!((MAXIMUM_OF, JUMP, JUMPS), ("Maximum of", "jump", "jumps"));
        assert_eq!(
            (SHIELDS_LABEL, ARMOR_LABEL, ENERGY_LABEL),
            ("Shields:", "Armor:", "Energy:")
        );
        assert_eq!((NONE, NOT_APPLICABLE, METRES), ("None", "N/A", "m"));
        assert_eq!(
            RATINGS,
            [
                "Excellent",
                "Very Good",
                "Good",
                "Average",
                "Poor",
                "Terrible"
            ]
        );
        assert_eq!(
            (STANDARD_WEAPONS, NO_STANDARD_WEAPONS, AMMO),
            ("Standard Weapons:", "No standard weapons", "ammo")
        );
        assert_eq!(
            (PANEL_LABEL_COLUMNS, PANEL_VALUE_COLUMNS),
            ([0.0, 130.0], [45.0, 175.0])
        );
        assert_eq!(PANEL_ROWS, [12.0, 24.0, 36.0, 48.0, 60.0, 72.0, 84.0]);
        assert_eq!(
            (WEAPONS_ROW, FIRST_WEAPON_ROW, WEAPON_STEP, WEAPON_INDENT),
            (108.0, 120.0, 12.0, 8.0)
        );
        assert_eq!(LABEL_COLOR, Color::rgba(128, 128, 128, 255));
    }

    // The picture.

    /// Pictures 5000 and 5003 exist.
    fn exists(id: i16) -> bool {
        [5000, 5003].contains(&id)
    }

    #[test]
    fn a_ship_with_its_own_picture_shows_it() {
        let bases = [(ShipId(128), 1000), (ShipId(131), 1000)];
        assert_eq!(ship_picture(ShipId(128), &bases, exists), Some(5000));
        assert_eq!(ship_picture(ShipId(131), &bases, exists), Some(5003));
        assert_eq!(ship_picture(ShipId(131), &[], exists), Some(5003));
    }

    #[test]
    fn a_ship_without_one_shows_the_lowest_numbered_sharers_picture() {
        let bases = [
            (ShipId(361), 1000),
            (ShipId(131), 1000),
            (ShipId(128), 1000),
            (ShipId(129), 1001),
        ];
        assert_eq!(ship_picture(ShipId(361), &bases, exists), Some(5000));
    }

    #[test]
    fn a_sharer_without_a_picture_is_skipped() {
        let bases = [
            (ShipId(361), 1000),
            (ShipId(129), 1000),
            (ShipId(131), 1000),
        ];
        assert_eq!(ship_picture(ShipId(361), &bases, exists), Some(5003));
    }

    #[test]
    fn a_ship_with_no_base_image_or_no_sharer_with_a_picture_has_none() {
        let bases = [(ShipId(361), 1000), (ShipId(129), 1000)];
        assert_eq!(ship_picture(ShipId(362), &bases, exists), None);
        assert_eq!(ship_picture(ShipId(361), &bases, exists), None);
        assert_eq!(
            ship_picture(
                ShipId(361),
                &[(ShipId(361), 1000), (ShipId(128), 999)],
                exists
            ),
            None,
            "another base image"
        );
    }

    #[test]
    fn an_impossible_ship_id_does_not_overflow() {
        assert_eq!(ship_picture(ShipId(i16::MIN), &[], |_| false), None);
        assert_eq!(
            ship_picture(ShipId(i16::MAX), &[], |id| id == i16::MAX),
            Some(i16::MAX)
        );
    }

    // The grid.

    #[test]
    fn it_is_centred_with_the_first_cell_selected() {
        let screen = screen();
        assert_eq!(screen.problem(), None);
        assert_eq!(screen.dialog().expect("laid out").bounds().min, ORIGIN);
        assert_eq!(screen.selected(), Some(0));
        assert_eq!(screen.top_row(), 0);
        assert!(!screen.closed());
        assert_eq!(screen.shipyard(), &shipyard());
        assert!(screen.info_panel().is_none());
        assert_eq!(
            screen.cell_bounds(5),
            Some(rect(138.0 + 83.0, 230.0 + 54.0, 84.0, 55.0))
        );
    }

    #[test]
    fn the_grid_shows_the_names_and_marks_the_ship_flown() {
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
        let flown = screen.cell_bounds(0).expect("a cell");
        let shown = texts(&commands);
        for expected in [
            ("Shuttle".to_owned(), name(0, "Shuttle", 6.0), TEXT_COLOR),
            (
                CURRENT_MARK.to_owned(),
                Point::new(flown.min.x + 3.0, flown.min.y + 12.0 - 10.0),
                TEXT_COLOR,
            ),
            ("Heavy".to_owned(), name(1, "Heavy", 14.0), TEXT_COLOR),
            ("Shuttle".to_owned(), name(1, "Shuttle", 3.0), TEXT_COLOR),
            ("*Courier".to_owned(), name(2, "*Courier", 6.0), GREY),
            ("Viper".to_owned(), name(3, "Viper", 6.0), TEXT_COLOR),
        ] {
            assert!(shown.contains(&expected), "{expected:?}: {shown:?}");
        }
        let marks = shown
            .iter()
            .filter(|(text, _, _)| text == CURRENT_MARK)
            .count();
        assert_eq!(marks, 1, "only the ship flown");
        assert!(contains(&commands, &bright_frame(&screen, 0)));
        assert!(contains(&commands, &frame(&screen, 3)));
        assert!(
            !contains(&commands, &frame(&screen, 4)),
            "no frame on an empty cell"
        );
        assert!(!words(&screen).contains(&NO_SHIPS.to_owned()));
    }

    /// Shown cell `index`'s frame in `color`.
    fn framed(screen: &ShipyardScreen, index: usize, color: Color) -> Vec<DrawCommand> {
        let mut list = DrawList::new();
        frame_rect(&mut list, screen.cell_bounds(index).expect("a cell"), color);
        list.iter().cloned().collect()
    }

    /// Shown cell `index`'s frame in the fake's `GridDim`.
    fn frame(screen: &ShipyardScreen, index: usize) -> Vec<DrawCommand> {
        framed(screen, index, grid::TEST_GRID_COLORS.dim)
    }

    /// Shown cell `index`'s selection: framed in the fake's `GridBright`.
    fn bright_frame(screen: &ShipyardScreen, index: usize) -> Vec<DrawCommand> {
        framed(screen, index, grid::TEST_GRID_COLORS.bright)
    }

    fn contains(commands: &[DrawCommand], part: &[DrawCommand]) -> bool {
        commands.windows(part.len()).any(|window| window == part)
    }

    /// The frame rule itself is `grid::CellGrid::draw_cells`'s, tested
    /// there; this checks the shipyard hands it its grid item, rows,
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
    }

    #[test]
    fn a_press_is_hit_tested_on_the_84_by_55_division() {
        let mut screen = screen_of(long(22));
        let grid = item(&screen, GRID_ITEM);
        click_cell(&mut screen, 6);
        click(
            &mut screen,
            Point::new(grid.min.x + 83.0, grid.min.y + 54.0),
        );
        assert_eq!(screen.selected(), Some(0), "83 right, 54 down is cell 0");
        click(
            &mut screen,
            Point::new(grid.min.x + 84.0, grid.min.y + 55.0),
        );
        assert_eq!(screen.selected(), Some(5));
        click(&mut screen, Point::new(grid.max.x - 0.5, grid.max.y - 0.5));
        assert_eq!(
            screen.selected(),
            Some(19),
            "the last cell reaches the corner"
        );
        click(&mut screen, Point::new(grid.max.x, grid.min.y + 1.0));
        assert_eq!(screen.selected(), Some(19), "the right edge is out");
        click(&mut screen, Point::new(grid.min.x + 1.0, grid.max.y));
        assert_eq!(screen.selected(), Some(19), "the bottom edge is out");
    }

    /// `count` ships, numbered from 200.
    fn long(count: usize) -> Shipyard {
        Shipyard {
            rows: (0..count)
                .map(|n| {
                    let id = 200 + i16::try_from(n).expect("few");
                    row(id, &format!("Hull {n}"), 10)
                })
                .collect(),
            ..shipyard()
        }
    }

    #[test]
    fn the_selection_moves_by_click_and_arrows_and_the_grid_scrolls() {
        let mut screen = screen_of(long(22));
        press(&mut screen, Key::Right);
        assert_eq!(screen.selected(), Some(1));
        press(&mut screen, Key::Down);
        assert_eq!(screen.selected(), Some(5));
        press(&mut screen, Key::Up);
        press(&mut screen, Key::Left);
        assert_eq!(screen.selected(), Some(0));
        click_cell(&mut screen, 6);
        assert_eq!(screen.selected(), Some(6));
        click_item(&mut screen, SCROLL_DOWN_ITEM);
        click_item(&mut screen, SCROLL_DOWN_ITEM);
        assert_eq!(screen.top_row(), 1, "never past the end");
        let names: Vec<_> = words(&screen)
            .into_iter()
            .filter(|w| w.starts_with("Hull "))
            .collect();
        assert_eq!(names.first().map(String::as_str), Some("Hull 4"));
        assert_eq!(names.last().map(String::as_str), Some("Hull 21"));
        click_cell(&mut screen, 0);
        assert_eq!(screen.selected(), Some(4));
        click_item(&mut screen, SCROLL_UP_ITEM);
        assert_eq!(screen.top_row(), 0);
        click_cell(&mut screen, 4);
        assert_eq!(screen.selected(), Some(4));
        let grid = item(&screen, GRID_ITEM);
        click(&mut screen, Point::new(grid.min.x - 1.0, grid.min.y + 1.0));
        assert_eq!(screen.selected(), Some(4), "outside the grid");
        let mut short = screen_of(shipyard());
        click_cell(&mut short, 5);
        assert_eq!(short.selected(), None, "an empty cell selects nothing");
        assert_eq!(short.take_sounds(), [], "cells are silent");
    }

    #[test]
    fn the_arrows_are_white_only_when_the_grid_can_scroll_that_way() {
        let mut screen = screen_of(long(22));
        let up = item(&screen, SCROLL_UP_ITEM);
        let colors = |screen: &ShipyardScreen| -> Vec<Color> {
            drawn(screen)
                .into_iter()
                .filter_map(|command| match command {
                    DrawCommand::Line { from, color, .. } => Some((from, color)),
                    _ => None,
                })
                .filter(|(from, _)| {
                    up.contains(*from) || item(screen, SCROLL_DOWN_ITEM).contains(*from)
                })
                .step_by(3)
                .map(|(_, color)| color)
                .collect()
        };
        assert_eq!(colors(&screen), [GREY, TEXT_COLOR]);
        click_item(&mut screen, SCROLL_DOWN_ITEM);
        click_item(&mut screen, SCROLL_DOWN_ITEM);
        assert_eq!(colors(&screen), [TEXT_COLOR, GREY]);
    }

    /// The texts drawn in the picture item.
    fn in_picture(screen: &ShipyardScreen) -> Vec<(String, Point, Color)> {
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
        let mut screen = screen_with(shipyard(), &art());
        press(&mut screen, Key::Right);
        let picture = item(&screen, PICTURE_ITEM);
        assert_eq!(in_picture(&screen), no_picture_lines(picture));
    }

    #[test]
    fn each_description_read_is_asked_to_load_and_shows_loaded() {
        let mut screen = screen();
        assert_eq!(screen.take_description_request(), Some(13000));
        assert_eq!(screen.take_description_request(), None, "once");
        screen.show_description(13000, "Loaded.");
        assert!(words(&screen).contains(&"Loaded.".to_owned()));
        press(&mut screen, Key::Right);
        assert_eq!(screen.take_description_request(), Some(13001));
        screen.show_description(13000, "Stale.");
        assert!(
            !words(&screen).contains(&"Stale.".to_owned()),
            "not shown now"
        );
    }

    #[test]
    fn the_picture_and_description_are_read_as_the_selection_changes() {
        let art = art();
        let mut screen = screen_with(shipyard(), &art);
        let picture = item(&screen, PICTURE_ITEM);
        assert!(drawn(&screen).contains(&DrawCommand::StretchedPicture {
            image: ImageKey::picture(5000),
            top_left: picture.min,
            width: 200.0,
            height: 200.0,
        }));
        assert!(words(&screen).contains(&"dësc 13000.".to_owned()));
        press(&mut screen, Key::Right);
        assert_eq!(in_picture(&screen), no_picture_lines(picture));
        press(&mut screen, Key::Right);
        assert!(
            drawn(&screen).contains(&DrawCommand::StretchedPicture {
                image: ImageKey::picture(5002),
                top_left: picture.min,
                width: 200.0,
                height: 200.0,
            }),
            "its own"
        );
        press(&mut screen, Key::Right);
        assert!(
            drawn(&screen).contains(&DrawCommand::StretchedPicture {
                image: ImageKey::picture(5000),
                top_left: picture.min,
                width: 200.0,
                height: 200.0,
            }),
            "the Shuttle's, sharing its base image"
        );
        screen.set_shipyard(shipyard());
        assert_eq!(
            *art.asked.borrow(),
            [
                "bases",
                "PICT 5000",
                "PICT 5001",
                "PICT 5002",
                "PICT 5003",
                "PICT 5000",
                "dësc 13000",
                "dësc 13001",
                "dësc 13002",
                "dësc 13003",
                "PICT 5000",
                "PICT 5001",
                "PICT 5002",
                "PICT 5003",
                "PICT 5000",
            ],
            "the bases once, the pictures once a list, and each description once \
             while it stays selected"
        );
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

    /// `PICT` `id` in `cell`'s icon.
    fn icon(cell: Bounds, id: i16) -> DrawCommand {
        DrawCommand::StretchedPicture {
            image: ImageKey::picture(id),
            top_left: grid::icon_bounds(cell).min,
            width: 32.0,
            height: 32.0,
        }
    }

    #[test]
    fn each_cell_shows_its_ships_picture_and_a_missing_one_is_a_black_square() {
        let screen = screen();
        let commands = drawn(&screen);
        let cell = |index: usize| screen.cell_bounds(index).expect("a cell");
        for (index, id, why) in [
            (0, 5000, "the Shuttle's own"),
            (2, 5002, "the Courier's own"),
            (3, 5000, "the Shuttle's, sharing the Viper's base image"),
        ] {
            let in_cell = inside(&commands, cell(index));
            let at = in_cell
                .iter()
                .position(|c| *c == icon(cell(index), id))
                .unwrap_or_else(|| panic!("{why}"));
            let text = in_cell
                .iter()
                .position(|c| matches!(c, DrawCommand::Text { .. }))
                .expect("its name");
            assert!(at < text, "the picture under the text: {why}");
        }
        let heavy = inside(&commands, cell(1));
        assert!(
            !heavy
                .iter()
                .any(|c| matches!(c, DrawCommand::StretchedPicture { .. })),
            "no PICT of its own or shared"
        );
        let mut square = DrawList::new();
        fill_rect(&mut square, grid::icon_bounds(cell(1)), Color::BLACK);
        let square: Vec<DrawCommand> = square.iter().cloned().collect();
        let start = heavy
            .windows(square.len())
            .position(|window| window == square.as_slice())
            .expect("a black square");
        let names: Vec<(usize, String)> = heavy
            .iter()
            .enumerate()
            .filter_map(|(n, c)| match c {
                DrawCommand::Text { text, .. } => Some((n, text.clone())),
                _ => None,
            })
            .collect();
        assert_eq!(
            names.iter().map(|(_, t)| t.as_str()).collect::<Vec<_>>(),
            ["Heavy", "Shuttle"]
        );
        assert!(start < names[0].0, "the name over the square");
        let flown = inside(&commands, cell(0));
        let mark = flown
            .iter()
            .position(|c| matches!(c, DrawCommand::Text { text, .. } if text == CURRENT_MARK))
            .expect("the mark");
        let name = flown
            .iter()
            .position(|c| matches!(c, DrawCommand::Text { text, .. } if text == "Shuttle"))
            .expect("the name");
        assert!(mark < name, "the mark, then the name");
    }

    #[test]
    fn a_new_list_reads_its_pictures_again() {
        let art = Rc::new(FakeArt {
            pictures: vec![5000, 5002, 5004],
            bases: vec![(ShipId(128), 1000), (ShipId(132), 1004)],
            graphics: Vec::new(),
            asked: RefCell::default(),
        });
        let mut screen = screen_with(shipyard(), &art);
        let mut changed = shipyard();
        changed.rows.push(row(132, "New", 10));
        screen.set_shipyard(changed);
        let fifth = screen.cell_bounds(4).expect("a cell");
        assert!(drawn(&screen).contains(&icon(fifth, 5004)));
    }

    #[test]
    fn the_class_flown_shows_no_price_lines() {
        let mut screen = screen();
        assert!(info_box(&screen).is_empty());
        let flown = shipyard();
        assert!(price_lines(&flown, &flown.rows[0]).is_empty());
        assert!(
            drawn(&screen).contains(&DrawCommand::StretchedPicture {
                image: ImageKey::picture(5000),
                top_left: item(&screen, PICTURE_ITEM).min,
                width: 200.0,
                height: 200.0,
            }),
            "the picture is still drawn"
        );
        press(&mut screen, Key::Right);
        assert!(!info_box(&screen).is_empty(), "the Heavy Shuttle's lines");
        press(&mut screen, Key::Left);
        assert!(info_box(&screen).is_empty(), "none again for the Shuttle");
    }

    /// The shipyard's screen with the Heavy Shuttle (not flown) selected.
    fn heavy_selected(shipyard: Shipyard) -> ShipyardScreen {
        let mut screen = screen_of(shipyard);
        press(&mut screen, Key::Right);
        screen
    }

    #[test]
    fn the_info_box_shows_the_price_the_trade_in_the_final_price_and_the_cash() {
        let screen = heavy_selected(shipyard());
        assert_eq!(
            info_box(&screen),
            [
                "Ship Price:",
                "17,500 cr",
                "Trade-In:",
                "2,500 cr",
                "Final Price:",
                "15,000 cr",
                "You Have:",
                "25,000 cr"
            ]
        );
        // Each label at the box's left, its amount 70 right of it, on
        // baselines 12, 24, 48 and 72 below its top.
        let info = item(&screen, INFO_BOX_ITEM);
        let placed: Vec<_> = texts(&drawn(&screen))
            .into_iter()
            .filter(|(_, at, _)| info.contains(*at))
            .map(|(_, origin, color)| (origin, color))
            .collect();
        let at = |x: f32, baseline: f32| {
            (
                Point::new(info.min.x + x, info.min.y + baseline - TEXT_SIZE),
                TEXT_COLOR,
            )
        };
        assert_eq!(
            placed,
            [
                at(0.0, 12.0),
                at(70.0, 12.0),
                at(0.0, 24.0),
                at(70.0, 24.0),
                at(0.0, 48.0),
                at(70.0, 48.0),
                at(0.0, 72.0),
                at(70.0, 72.0),
            ]
        );
    }

    /// The amount drawn after `label` in the info box.
    fn amount_after(screen: &ShipyardScreen, label: &str) -> String {
        let lines = info_box(screen);
        let at = lines
            .iter()
            .position(|line| line == label)
            .unwrap_or_else(|| panic!("{label}"));
        lines[at + 1].clone()
    }

    #[test]
    fn the_final_price_shown_is_at_least_0() {
        let traded = |trade_in| {
            heavy_selected(Shipyard {
                trade_in,
                ..shipyard()
            })
        };
        let paid = traded(40_000);
        assert_eq!(amount_after(&paid, FINAL_PRICE_LABEL), "0 cr");
        assert_eq!(amount_after(&paid, TRADE_IN_LABEL), "40,000 cr");
        assert_eq!(amount_after(&traded(17_500), FINAL_PRICE_LABEL), "0 cr");
        assert_eq!(amount_after(&traded(17_499), FINAL_PRICE_LABEL), "1 cr");
    }

    #[test]
    fn the_amounts_draw_as_draw_comma_number_does() {
        let in_debt = heavy_selected(Shipyard {
            cash: -25_000,
            ..shipyard()
        });
        assert_eq!(amount_after(&in_debt, credits::YOU_HAVE), "-25000 cr");
        let mut dear = Shipyard {
            cash: 1_234_567,
            ..shipyard()
        };
        dear.rows[1].price = 2_500_000;
        let dear = heavy_selected(dear);
        assert_eq!(amount_after(&dear, PRICE_LABEL), "2.50M cr");
        assert_eq!(amount_after(&dear, credits::YOU_HAVE), "1.23M cr");
        let broke = heavy_selected(Shipyard {
            cash: 0,
            ..shipyard()
        });
        assert_eq!(amount_after(&broke, credits::YOU_HAVE), "0 cr");
    }

    #[test]
    fn buy_ship_is_greyed_when_the_ship_cannot_be_bought() {
        let mut screen = screen();
        assert!(!enabled(&screen, BUY_SHIP_LABEL), "the class flown");
        press(&mut screen, BUY_KEY);
        assert_eq!(
            screen.take_request(),
            None,
            "the class flown asks for nothing"
        );
        press(&mut screen, Key::Right);
        assert!(enabled(&screen, BUY_SHIP_LABEL));
        press(&mut screen, Key::Right);
        assert!(!enabled(&screen, BUY_SHIP_LABEL), "too dear");
        click_item(&mut screen, BUY_ITEM);
        press(&mut screen, BUY_KEY);
        press(&mut screen, Key::Right);
        assert!(!enabled(&screen, BUY_SHIP_LABEL), "not for sale");
        press(&mut screen, BUY_KEY);
        assert_eq!(screen.take_request(), None, "greyed asks for nothing");
        assert!(enabled(&screen, DONE_LABEL));
        assert!(enabled(&screen, INFO_LABEL));
    }

    #[test]
    fn buy_ship_requests_the_selected_ship_and_b_only_on_a_press() {
        let mut screen = screen();
        press(&mut screen, Key::Right);
        click_item(&mut screen, BUY_ITEM);
        assert_eq!(screen.take_request(), Some(ShipId(129)));
        assert_eq!(screen.take_request(), None, "once");
        press(&mut screen, BUY_KEY);
        assert_eq!(screen.take_request(), Some(ShipId(129)));
        for _ in 0..3 {
            screen.input(&key(BUY_KEY, true, true));
            assert_eq!(screen.take_request(), None, "a repeat buys nothing");
        }
        screen.input(&key(BUY_KEY, false, false));
        assert_eq!(screen.take_request(), None, "nor a release");
    }

    #[test]
    fn a_new_shipyard_shows_and_regreys_keeping_the_ship_selected() {
        let mut screen = screen();
        press(&mut screen, Key::Right);
        let mut bought = shipyard();
        bought.current = ShipId(129);
        bought.cash = 0;
        bought.rows.remove(0);
        screen.set_shipyard(bought.clone());
        assert_eq!(screen.shipyard(), &bought);
        assert_eq!(screen.selected(), Some(0), "followed");
        assert!(!enabled(&screen, BUY_SHIP_LABEL));
        screen.set_shipyard(long(0));
        assert_eq!(screen.selected(), None);
        assert!(!enabled(&screen, BUY_SHIP_LABEL));
    }

    #[test]
    fn with_nothing_listed_the_grid_says_so_and_buy_and_info_do_nothing() {
        let art = art();
        let mut screen = screen_with(long(0), &art);
        assert_eq!(screen.selected(), None);
        let grid = item(&screen, GRID_ITEM);
        assert!(texts(&drawn(&screen)).contains(&(
            NO_SHIPS.to_owned(),
            Point::new(grid.min.x + INSET, grid.min.y + INSET),
            TEXT_COLOR
        )));
        assert!(info_box(&screen).is_empty());
        assert!(!enabled(&screen, BUY_SHIP_LABEL));
        press(&mut screen, BUY_KEY);
        press(&mut screen, INFO_KEY);
        click_item(&mut screen, INFO_ITEM);
        assert_eq!(screen.take_request(), None);
        assert!(screen.info_panel().is_none());
        assert_eq!(*art.asked.borrow(), ["bases"], "nothing else to read");
    }

    // The selection as `_ShipyardFilter` keeps it (`RuleKey::GridSelection`).

    /// The Bible's reading of the grid's selection: the port's, where the
    /// selection is the highlighted cell.
    fn bible() -> Rulebook {
        Rulebook::default().with_override(nova_sim::RuleKey::GridSelection, RuleSource::Bible)
    }

    /// Whether the screen shows no ship: Buy Ship and Info greyed, and no
    /// cell bright, no picture, no info box and no description drawn.
    fn shows_no_ship(screen: &ShipyardScreen) -> bool {
        let commands = drawn(screen);
        let bright = (0..20).any(|index| contains(&commands, &bright_frame(screen, index)));
        let picture = item(screen, PICTURE_ITEM);
        let pictured = commands.iter().any(|c| {
            matches!(c, DrawCommand::StretchedPicture { top_left, .. } if *top_left == picture.min)
        });
        !enabled(screen, BUY_SHIP_LABEL)
            && !enabled(screen, INFO_LABEL)
            && !bright
            && !pictured
            && in_picture(screen).is_empty()
            && info_box(screen).is_empty()
            && !words(screen).iter().any(|w| w.starts_with("dësc"))
    }

    #[test]
    fn by_the_engine_it_opens_with_nothing_selected() {
        let art = art();
        let mut screen = opened_with(shipyard(), &art);
        assert_eq!(screen.selected(), None);
        assert!(shows_no_ship(&screen));
        assert_eq!(screen.take_description_request(), None, "none to load");
        assert_eq!(
            *art.asked.borrow(),
            [
                "bases",
                "PICT 5000",
                "PICT 5001",
                "PICT 5002",
                "PICT 5003",
                "PICT 5000"
            ]
        );
    }

    #[test]
    fn by_the_engine_an_empty_cell_pressed_selects_nothing() {
        let mut screen = opened_with(shipyard(), &art());
        click_cell(&mut screen, 1);
        assert_eq!(screen.selected(), Some(1));
        assert!(!shows_no_ship(&screen));
        click_cell(&mut screen, 5);
        assert_eq!(screen.selected(), None, "the empty position past the last");
        assert!(shows_no_ship(&screen));
        let mut first_empty = opened_with(shipyard(), &art());
        click_cell(&mut first_empty, 1);
        click_cell(&mut first_empty, 4);
        assert_eq!(first_empty.selected(), None, "the first empty position");
        assert_eq!(screen.take_request(), None, "it reaches no button");
        assert!(!screen.closed());
        assert_eq!(screen.take_sounds(), [], "silent");
    }

    #[test]
    fn by_the_engine_a_scroll_that_moves_selects_nothing() {
        let mut screen = opened_with(long(30), &art());
        click_cell(&mut screen, 5);
        assert_eq!(screen.selected(), Some(5));
        click_item(&mut screen, SCROLL_DOWN_ITEM);
        assert_eq!((screen.top_row(), screen.selected()), (1, None));
        assert!(shows_no_ship(&screen));
        click_item(&mut screen, SCROLL_DOWN_ITEM);
        click_item(&mut screen, SCROLL_DOWN_ITEM);
        assert_eq!(screen.top_row(), 3, "30 listed: the last first row");
        click_cell(&mut screen, 2);
        assert_eq!(screen.selected(), Some(14));
        click_item(&mut screen, SCROLL_DOWN_ITEM);
        assert_eq!(
            (screen.top_row(), screen.selected()),
            (3, Some(14)),
            "at the bottom it neither scrolls nor clears"
        );
        click_item(&mut screen, SCROLL_UP_ITEM);
        assert_eq!((screen.top_row(), screen.selected()), (2, None));
        click_item(&mut screen, SCROLL_UP_ITEM);
        click_item(&mut screen, SCROLL_UP_ITEM);
        click_cell(&mut screen, 3);
        click_item(&mut screen, SCROLL_UP_ITEM);
        assert_eq!(
            (screen.top_row(), screen.selected()),
            (0, Some(3)),
            "at the top it neither scrolls nor clears"
        );
    }

    #[test]
    fn by_the_other_reading_it_opens_on_the_first_cell_and_keeps_it() {
        let mut screen = opened_with(
            Shipyard {
                rules: bible(),
                ..long(30)
            },
            &art(),
        );
        assert_eq!(screen.selected(), Some(0), "the first cell");
        click_cell(&mut screen, 5);
        click_item(&mut screen, SCROLL_DOWN_ITEM);
        assert_eq!(
            (screen.top_row(), screen.selected()),
            (1, Some(5)),
            "a scroll keeps it"
        );
        let mut short = opened_with(
            Shipyard {
                rules: bible(),
                ..shipyard()
            },
            &art(),
        );
        click_cell(&mut short, 1);
        click_cell(&mut short, 5);
        assert_eq!(short.selected(), Some(1), "an empty cell is ignored");
        assert!(enabled(&short, BUY_SHIP_LABEL));
    }

    #[test]
    fn a_second_press_on_a_cell_within_15_mac_ticks_opens_info_by_either_reading() {
        use nova_sim::mission::screens::mac_ticks;
        for rules in [Rulebook::default(), bible()] {
            let opened = || {
                opened_with(
                    Shipyard {
                        rules,
                        ..shipyard()
                    },
                    &art(),
                )
            };
            let mut screen = opened();
            click_cell(&mut screen, 1);
            assert!(screen.info_panel().is_none(), "one press");
            screen.tick(mac_ticks(15));
            click_cell(&mut screen, 1);
            assert!(screen.info_panel().is_some(), "{rules:?}");
            assert!(words(&screen).contains(&"Ship 129".to_owned()));

            let mut slow = opened();
            click_cell(&mut slow, 1);
            slow.tick(mac_ticks(16));
            click_cell(&mut slow, 1);
            assert!(slow.info_panel().is_none(), "16 ticks apart");

            let mut covered = opened();
            click_cell(&mut covered, 1);
            covered.tick_covered(mac_ticks(16));
            click_cell(&mut covered, 1);
            assert!(
                covered.info_panel().is_none(),
                "the time covered counts, as `when` is the clock's"
            );

            let mut moved = opened();
            click_cell(&mut moved, 0);
            click_cell(&mut moved, 1);
            assert!(moved.info_panel().is_none(), "another cell");
            click_cell(&mut moved, 1);
            assert!(moved.info_panel().is_some(), "then the same one");

            let mut broken = opened();
            click_cell(&mut broken, 1);
            click_cell(&mut broken, 5);
            click_cell(&mut broken, 1);
            assert!(broken.info_panel().is_none(), "an empty press between");
        }
    }

    #[test]
    fn a_double_click_is_by_position_so_one_across_a_scroll_opens_the_ship_now_shown() {
        let mut screen = opened_with(long(30), &art());
        click_cell(&mut screen, 5);
        click_item(&mut screen, SCROLL_DOWN_ITEM);
        assert!(screen.info_panel().is_none());
        click_cell(&mut screen, 5);
        assert_eq!(screen.selected(), Some(9), "the ship now at position 5");
        assert!(screen.info_panel().is_some());
        assert!(words(&screen).contains(&"Ship 209".to_owned()));
    }

    #[test]
    fn b_refused_beeps_and_space_with_nothing_selected_is_silent() {
        for rules in [Rulebook::default(), bible()] {
            let mut screen = opened_with(
                Shipyard {
                    rules,
                    ..shipyard()
                },
                &art(),
            );
            if rules == Rulebook::default() {
                press(&mut screen, BUY_KEY);
                assert_eq!(screen.take_request(), None, "nothing selected");
                assert_eq!(screen.take_sounds(), [Sound::Ui(UiSound::BuyRefused)]);
                press(&mut screen, INFO_KEY);
                assert!(screen.info_panel().is_none());
                assert_eq!(screen.take_sounds(), [], "Info with no ship is silent");
            }
            click_cell(&mut screen, 0);
            press(&mut screen, BUY_KEY);
            assert_eq!(screen.take_request(), None, "the class flown");
            assert_eq!(
                screen.take_sounds(),
                [Sound::Ui(UiSound::BuyRefused)],
                "{rules:?}"
            );
            press(&mut screen, Key::Right);
            press(&mut screen, BUY_KEY);
            assert_eq!(screen.take_request(), Some(ShipId(129)));
            assert_eq!(screen.take_sounds(), [], "a buy does not beep");
        }
    }

    #[test]
    fn by_the_engine_an_arrow_from_nothing_selected_picks_the_first_or_last_shown() {
        let mut screen = opened_with(long(30), &art());
        press(&mut screen, Key::Right);
        assert_eq!(screen.selected(), Some(0));
        click_item(&mut screen, SCROLL_DOWN_ITEM);
        press(&mut screen, Key::Left);
        assert_eq!(screen.selected(), Some(23), "the last shown");
        assert!(enabled(&screen, BUY_SHIP_LABEL));
        let mut short = opened_with(shipyard(), &art());
        click_cell(&mut short, 5);
        press(&mut short, Key::Up);
        assert_eq!(short.selected(), Some(3), "the last listed");
    }

    #[test]
    fn by_the_engine_a_new_shipyard_keeps_nothing_selected() {
        let mut screen = opened_with(shipyard(), &art());
        screen.set_shipyard(shipyard());
        assert_eq!(screen.selected(), None);
        assert!(shows_no_ship(&screen));
    }

    // Naming the ship.

    /// Stock "Text Input": `DLOG` 3001, 360 x 138 and centred, with OK
    /// (1), a parked picture (2), the prompt (3), a picture (4), the field
    /// (5) and Cancel (6).
    fn text_input_template() -> DialogTemplate {
        let item = |x, y, w, h, enabled, kind| ItemTemplate {
            bounds: rect(x, y, w, h),
            enabled,
            kind,
        };
        DialogTemplate {
            bounds: rect(0.0, 0.0, 360.0, 138.0),
            placement: Placement::Center,
            items: vec![
                item(
                    252.0,
                    106.0,
                    70.0,
                    20.0,
                    true,
                    ItemSpec::Button("OK".into()),
                ),
                item(7.0, 147.0, 32.0, 32.0, false, ItemSpec::Picture(129)),
                item(
                    52.0,
                    5.0,
                    295.0,
                    50.0,
                    false,
                    ItemSpec::StaticText(String::new()),
                ),
                item(7.0, 5.0, 32.0, 32.0, false, ItemSpec::Picture(130)),
                item(
                    91.0,
                    64.0,
                    200.0,
                    16.0,
                    true,
                    ItemSpec::EditText(String::new()),
                ),
                item(
                    170.0,
                    106.0,
                    70.0,
                    20.0,
                    true,
                    ItemSpec::Button("Cancel".into()),
                ),
            ],
        }
    }

    fn naming() -> ShipNaming {
        ShipNaming {
            ship: ShipId(129),
            prompt: "Please name your new The Heavy Shuttle: ".to_owned(),
            default: "Ship 129 491".to_owned(),
        }
    }

    fn typed(screen: &mut ShipyardScreen, text: &str) {
        for c in text.chars() {
            press(screen, Key::Char(c.to_ascii_lowercase()));
            screen.input(&Input::Text(c));
        }
    }

    /// The screen with B pressed on ship 129 and its prompt open.
    fn naming_by_key() -> ShipyardScreen {
        let mut screen = screen();
        press(&mut screen, Key::Right);
        press(&mut screen, BUY_KEY);
        assert_eq!(screen.take_request(), Some(ShipId(129)));
        screen.open_naming(&naming());
        screen
    }

    fn naming_click(screen: &mut ShipyardScreen, item: usize) {
        let at = screen
            .naming()
            .expect("open")
            .dialog()
            .item_bounds(item)
            .expect("an item")
            .center();
        click(screen, at);
    }

    #[test]
    fn opening_the_naming_routes_input_to_it_and_ok_orders_the_ship_with_the_name() {
        let mut screen = naming_by_key();
        let prompt = screen.naming().expect("open");
        assert_eq!(prompt.field().text(), "Ship 129 491");
        screen.input(&Input::Text('b'));
        assert_eq!(
            screen.naming().expect("open").field().text(),
            "Ship 129 491",
            "the B that asked types nothing"
        );
        typed(&mut screen, "Kestrel");
        press(&mut screen, INFO_KEY);
        assert!(screen.info_panel().is_none(), "the prompt takes the keys");
        assert_eq!(screen.take_order(), None);
        press(&mut screen, Key::Enter);
        assert_eq!(
            screen.take_order(),
            Some(ShipOrder {
                ship: ShipId(129),
                name: "Kestrel".to_owned(),
            })
        );
        assert_eq!(screen.take_order(), None, "once");
        assert!(screen.naming().is_none(), "closed");
        assert!(!screen.closed(), "Return went to the prompt alone");
        assert_eq!(screen.take_declined(), None);
    }

    /// `_TrackTextButtons` @0x26bb0 plays nothing under the pointer.
    #[test]
    fn ok_clicked_orders_the_ship_silently() {
        let mut screen = naming_by_key();
        screen.take_sounds();
        naming_click(&mut screen, crate::ui::text_input::OK_ITEM);
        assert_eq!(screen.take_sounds(), []);
        assert_eq!(
            screen.take_order(),
            Some(ShipOrder {
                ship: ShipId(129),
                name: "Ship 129 491".to_owned(),
            })
        );
    }

    #[test]
    fn a_name_too_long_alerts_and_keeps_the_prompt_open() {
        let mut screen = naming_by_key();
        typed(&mut screen, &"x".repeat(65));
        screen.take_sounds();
        press(&mut screen, Key::Enter);
        assert_eq!(screen.take_sounds(), [Sound::Ui(UiSound::Alert)]);
        assert_eq!(screen.take_order(), None);
        assert!(screen.naming().is_some());
    }

    #[test]
    fn cancel_declines_the_ship() {
        let mut screen = naming_by_key();
        press(&mut screen, Key::Escape);
        assert!(screen.naming().is_some(), "Escape does nothing");
        assert!(!screen.closed());
        naming_click(&mut screen, crate::ui::text_input::CANCEL_ITEM);
        assert_eq!(screen.take_declined(), Some(ShipId(129)));
        assert_eq!(screen.take_declined(), None, "once");
        assert_eq!(screen.take_order(), None);
        assert!(screen.naming().is_none(), "closed");
        press(&mut screen, Key::Escape);
        assert!(screen.closed(), "the shipyard takes the keys again");
    }

    #[test]
    fn a_click_on_buy_ship_does_not_flush_text() {
        let mut screen = screen();
        press(&mut screen, Key::Right);
        click_item(&mut screen, BUY_ITEM);
        assert_eq!(screen.take_request(), Some(ShipId(129)));
        screen.open_naming(&naming());
        screen.input(&Input::Text('K'));
        assert_eq!(screen.naming().expect("open").field().text(), "K");
    }

    #[test]
    fn the_naming_falls_back_without_the_text_input_dialog() {
        let mut screen = ShipyardScreen::new(
            Ok(layout()),
            info_templates(),
            Err("no DLOG 3001".to_owned()),
            shipyard(),
            art(),
            ButtonStyle::STOCK,
        );
        press(&mut screen, Key::Right);
        click_item(&mut screen, BUY_ITEM);
        screen.take_request();
        screen.open_naming(&naming());
        let prompt = screen.naming().expect("open");
        let bounds = prompt.dialog().bounds();
        assert_eq!((bounds.width(), bounds.height()), (360.0, 138.0));
        press(&mut screen, Key::Enter);
        assert_eq!(
            screen.take_order().map(|order| order.name),
            Some("Ship 129 491".to_owned())
        );
    }

    #[test]
    fn the_prompt_is_drawn_over_the_shipyard() {
        let mut screen = screen();
        let under = drawn(&screen);
        screen.open_naming(&naming());
        let commands = drawn(&screen);
        assert_eq!(commands[..under.len()], under[..]);
        let mut prompt = DrawList::new();
        screen.naming().expect("open").draw(&mut prompt);
        assert_eq!(
            commands[under.len()..],
            prompt.iter().cloned().collect::<Vec<_>>()[..]
        );
    }

    #[test]
    fn cancelling_the_pointer_abandons_a_click_on_the_prompt() {
        let mut screen = naming_by_key();
        let at = screen
            .naming()
            .expect("open")
            .dialog()
            .item_bounds(crate::ui::text_input::OK_ITEM)
            .expect("OK")
            .center();
        let button = |pressed| Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at,
        };
        screen.input(&button(true));
        screen.cancel_pointer();
        screen.input(&button(false));
        assert_eq!(screen.take_order(), None);
        assert!(screen.naming().is_some());
    }

    // The info panel.

    /// The panel's grey, for its labels.
    const LABEL_GREY: Color = Color::rgba(128, 128, 128, 255);

    /// A text of item 5 at (`dx`, `baseline`) in `color`.
    fn at(dx: f32, baseline: f32, text: &str, color: Color) -> PanelText {
        PanelText {
            dx,
            baseline,
            text: text.to_owned(),
            color,
        }
    }

    /// A grey label at (`dx`, `baseline`).
    fn label(dx: f32, baseline: f32, text: &str) -> PanelText {
        at(dx, baseline, text, LABEL_GREY)
    }

    /// A white value at (`dx`, `baseline`).
    fn value(dx: f32, baseline: f32, text: &str) -> PanelText {
        at(dx, baseline, text, Color::WHITE)
    }

    /// The texts of the rows above the weapons list, for [`SPECS`].
    fn upper_rows() -> Vec<PanelText> {
        vec![
            label(0.0, 12.0, "Speed:"),
            value(45.0, 12.0, "300"),
            label(0.0, 24.0, "Accel:"),
            value(45.0, 24.0, "Average"),
            label(0.0, 36.0, "Turn:"),
            value(45.0, 36.0, "Poor"),
            label(0.0, 48.0, "Shields:"),
            value(45.0, 48.0, "None"),
            label(0.0, 60.0, "Armor:"),
            value(45.0, 60.0, "None"),
            label(0.0, 72.0, "Guns:"),
            value(45.0, 72.0, "Maximum of 2"),
            label(0.0, 84.0, "Turrets:"),
            value(45.0, 84.0, "Maximum of 1"),
            label(130.0, 12.0, "Space:"),
            value(175.0, 12.0, "12 tons"),
            label(130.0, 24.0, "Cargo:"),
            value(175.0, 24.0, "15 tons"),
            label(130.0, 36.0, "Energy:"),
            value(175.0, 36.0, "3 jumps"),
        ]
    }

    /// The texts of the rows below the weapons list, for [`SPECS`].
    fn lower_rows() -> Vec<PanelText> {
        vec![
            label(130.0, 48.0, "Length:"),
            value(175.0, 48.0, "26 m"),
            label(130.0, 60.0, "Mass:"),
            value(175.0, 60.0, "25 tons"),
            label(130.0, 72.0, "Crew:"),
            value(175.0, 72.0, "4"),
        ]
    }

    #[test]
    fn the_panel_draws_the_originals_two_columns_for_a_class_without_stock_weapons() {
        let mut expected = upper_rows();
        expected.push(value(0.0, 108.0, "No standard weapons"));
        expected.extend(lower_rows());
        assert_eq!(expected.len(), 27);
        assert_eq!(spec_lines(&SPECS), expected);
        assert_eq!(
            spec_lines_for(&row(129, "Heavy", 1).specs, InfoVariant::Plain),
            expected,
            "what a plain panel draws"
        );
    }

    /// A stock line of `count` of `name`, with `ammo`.
    fn stock(count: i64, name: &str, ammo: Option<i64>) -> StockLine {
        StockLine {
            count,
            name: name.to_owned(),
            ammo,
        }
    }

    #[test]
    fn the_panel_lists_stock_weapons_from_row_108() {
        let specs = ShipSpecs {
            weapons: vec![
                stock(2, "Light Blaster", None),
                stock(1, "Missile Launcher", Some(10)),
                stock(4, "Viper", None),
                stock(1, "Lone", None),
                stock(2, "", None),
            ],
            ..SPECS
        };
        let mut expected = upper_rows();
        expected.extend([
            value(0.0, 108.0, "Standard Weapons:"),
            value(8.0, 120.0, "2 Light Blasters"),
            value(8.0, 132.0, "1 Missile Launcher + 10 ammo"),
            value(8.0, 144.0, "4 Vipers"),
            value(8.0, 156.0, "1 Lone"),
            value(8.0, 168.0, "2 s"),
        ]);
        expected.extend(lower_rows());
        assert_eq!(spec_lines(&specs), expected);
    }

    #[test]
    fn each_panel_value_follows_its_condition() {
        let shown = |specs: ShipSpecs, label: &str| {
            let lines = spec_lines(&specs);
            let at = lines
                .iter()
                .position(|text| text.text == label)
                .unwrap_or_else(|| panic!("{label}: {lines:?}"));
            lines[at + 1].text.clone()
        };
        let fielded = |fields: ShipFields| ShipSpecs { fields, ..SPECS };
        let shields = |shield| {
            shown(
                fielded(ShipFields {
                    shield,
                    ..SPECS.fields
                }),
                "Shields:",
            )
        };
        assert_eq!(shields(30), "30");
        assert_eq!(shields(-4), "20");
        assert_eq!(shields(1500), "1,500");
        let armour = fielded(ShipFields {
            armor: 45,
            ..SPECS.fields
        });
        assert_eq!(shown(armour, "Armor:"), "45");
        let armed = ShipSpecs {
            max_gun: 0,
            max_tur: -1,
            ..SPECS
        };
        assert_eq!(shown(armed.clone(), "Guns:"), "None");
        assert_eq!(shown(armed, "Turrets:"), "None");
        let energy = |fuel| {
            shown(
                fielded(ShipFields {
                    fuel,
                    ..SPECS.fields
                }),
                "Energy:",
            )
        };
        assert_eq!(energy(0), "None");
        assert_eq!(energy(100), "1 jump");
        assert_eq!(energy(199), "1 jump");
        assert_eq!(energy(99), "0 jumps");
        assert_eq!(energy(-150), "-1 jumps");
        let handling = |accel, maneuver| {
            let specs = fielded(ShipFields {
                accel,
                maneuver,
                ..SPECS.fields
            });
            (shown(specs.clone(), "Accel:"), shown(specs, "Turn:"))
        };
        assert_eq!(handling(0, 51), ("N/A".to_owned(), "Excellent".to_owned()));
        assert_eq!(handling(731, 0), ("Excellent".to_owned(), "N/A".to_owned()));
        let ratings = [
            (1, "Terrible"),
            (11, "Poor"),
            (21, "Average"),
            (31, "Good"),
            (41, "Very Good"),
        ];
        for (maneuver, word) in ratings {
            assert_eq!(handling(0, maneuver).1, word, "{maneuver}");
        }
        let speed = |rounding| {
            shown(
                ShipSpecs {
                    fields: ShipFields {
                        speed: 190,
                        ..SPECS.fields
                    },
                    rounding,
                    ..SPECS
                },
                "Speed:",
            )
        };
        assert_eq!(speed(RuleSource::Engine), "189");
        assert_eq!(speed(RuleSource::Bible), "190");
        let crewless = spec_lines(&ShipSpecs { crew: 0, ..SPECS });
        assert!(!crewless.iter().any(|text| text.text == "Crew:"));
        assert!(
            !crewless
                .iter()
                .any(|text| (text.dx, text.baseline) == (175.0, 72.0))
        );
        assert_eq!(crewless.len(), 25);
    }

    #[test]
    fn the_space_cargo_and_mass_lines_have_a_ton_at_exactly_1() {
        let lines = |space, holds, mass| {
            spec_lines(&ShipSpecs {
                fields: ShipFields {
                    holds,
                    mass,
                    ..SPECS.fields
                },
                space,
                ..SPECS
            })
        };
        let has = |lines: &[PanelText], dx: f32, baseline: f32, text: &str| {
            assert!(
                lines.contains(&value(dx, baseline, text)),
                "{text} at ({dx}, {baseline}): {lines:?}"
            );
        };
        let one = lines(1, 1, 1);
        has(&one, 175.0, 12.0, "1 ton");
        has(&one, 175.0, 24.0, "1 ton");
        has(&one, 175.0, 60.0, "1 ton");
        let none = lines(0, 0, 0);
        has(&none, 175.0, 12.0, "0 tons");
        has(&none, 175.0, 24.0, "0 tons");
        has(&none, 175.0, 60.0, "0 tons");
        has(&lines(0, -15, 0), 175.0, 24.0, "15 tons");
        has(&lines(-3, 0, 0), 175.0, 12.0, "-3 tons");
    }

    /// The panel's item `number`.
    fn panel_item(screen: &ShipyardScreen, number: usize) -> Bounds {
        screen
            .info_panel()
            .expect("open")
            .item_bounds(number)
            .expect("an item")
    }

    #[test]
    fn a_panel_text_wholly_below_item_5_is_not_drawn() {
        let mut shipyard = shipyard();
        shipyard.rows[1].specs.weapons = (0..10)
            .map(|n| stock(1, &format!("Gun {n}"), None))
            .collect();
        let mut screen = screen_of(shipyard);
        press(&mut screen, Key::Right);
        press(&mut screen, INFO_KEY);
        let area = panel_item(&screen, PANEL_TEXT_ITEM);
        assert_eq!(area.height(), 214.0);
        let drawn = texts(&drawn(&screen));
        let top = |line: &str| {
            drawn
                .iter()
                .find(|(text, _, _)| text == line)
                .map(|(_, at, _)| at.y - area.min.y)
        };
        for n in 0..9 {
            assert_eq!(
                top(&format!("1 Gun {n}")),
                Some(110.0 + 12.0 * n as f32),
                "{n}"
            );
        }
        assert_eq!(top("1 Gun 8"), Some(206.0));
        assert_eq!(top("1 Gun 9"), None, "its top is 218");
        assert_eq!(
            top("Crew:"),
            Some(62.0),
            "the rows after the list still draw"
        );
        // In an item 5 218 high, line 9's top is exactly at its bottom:
        // still not drawn.
        let mut info = info_template();
        info.items[PANEL_TEXT_ITEM - 1].bounds = rect(9.0, 32.0, 234.0, 218.0);
        let mut shipyard = self::shipyard();
        shipyard.rows[1].specs.weapons = (0..10)
            .map(|n| stock(1, &format!("Gun {n}"), None))
            .collect();
        let catalog: Rc<dyn ShipyardCatalog> = art();
        let mut taller = ShipyardScreen::new(
            Ok(layout()),
            InfoTemplates {
                plain: Ok(info),
                ..info_templates()
            },
            Ok(text_input_template()),
            shipyard,
            catalog,
            ButtonStyle::STOCK,
        );
        press(&mut taller, Key::Right);
        press(&mut taller, Key::Right);
        press(&mut taller, INFO_KEY);
        assert_eq!(panel_item(&taller, PANEL_TEXT_ITEM).height(), 218.0);
        let words: Vec<String> = texts(&self::drawn(&taller))
            .into_iter()
            .map(|(text, _, _)| text)
            .collect();
        assert!(words.contains(&"1 Gun 8".to_owned()));
        assert!(!words.contains(&"1 Gun 9".to_owned()), "its top is 218");
    }

    #[test]
    fn info_and_i_open_the_panel_with_the_ships_name_and_stats() {
        let mut screen = screen();
        press(&mut screen, Key::Right);
        click_item(&mut screen, INFO_ITEM);
        let panel = screen.info_panel().expect("open");
        assert_eq!(
            panel.bounds().min,
            Point::new((1024.0 - 250.0) / 2.0, ((768.0 - 285.0) / 2.0_f32).floor())
        );
        let commands = drawn(&screen);
        let area = panel_item(&screen, PANEL_TEXT_ITEM);
        let placed: Vec<(String, Point, Color)> = spec_lines(&SPECS)
            .into_iter()
            .map(|text| {
                (
                    text.text,
                    Point::new(area.min.x + text.dx, area.min.y + text.baseline - 10.0),
                    text.color,
                )
            })
            .collect();
        assert_eq!(placed.len(), 27);
        let all = texts(&commands);
        let start = all
            .iter()
            .position(|text| *text == placed[0])
            .expect("the panel's texts");
        assert_eq!(all[start..start + placed.len()], placed[..]);
        let background = commands
            .iter()
            .position(|c| {
                *c == DrawCommand::Picture {
                    image: INFO_BACKGROUND,
                    top_left: panel.bounds().min,
                }
            })
            .expect("drawn");
        let title = commands
            .iter()
            .position(|c| matches!(c, DrawCommand::Text { text, .. } if text == "Ship 129"))
            .expect("the name");
        let done_buy = commands
            .iter()
            .position(|c| matches!(c, DrawCommand::Text { text, .. } if text == BUY_SHIP_LABEL))
            .expect("the shipyard under it");
        assert!(done_buy < background && background < title);
        let first_text = commands
            .iter()
            .position(|c| matches!(c, DrawCommand::Text { text, origin, .. } if *text == placed[0].0 && *origin == placed[0].1))
            .expect("drawn");
        assert!(title < first_text, "the texts after the title");
        let DrawCommand::Text {
            origin, font, size, ..
        } = &commands[title]
        else {
            unreachable!()
        };
        let area = panel_item(&screen, PANEL_TITLE_ITEM);
        let width = MonoMetrics.width(Font::Times, 18.0, "Ship 129");
        assert_eq!(
            *origin,
            Point::new(area.center().x.floor() - width / 2.0, area.min.y),
            "centred across the title, on baseline 18"
        );
        assert_eq!((*font, *size), (Font::Times, 18.0));
        // Its Done closes it alone.
        let done = panel_item(&screen, PANEL_DONE_ITEM).center();
        click(&mut screen, done);
        assert!(screen.info_panel().is_none());
        assert!(!screen.closed());
        assert_eq!(screen.take_sounds(), [], "Info and Done are silent");
        press(&mut screen, INFO_KEY);
        assert!(screen.info_panel().is_some(), "Space opens it");
        screen.input(&key(INFO_KEY, true, true));
        assert!(screen.info_panel().is_some());
    }

    #[test]
    fn p_asks_for_the_player_info_from_the_shipyard_and_its_panel_not_its_prompt() {
        let p = crate::ui::player_info::PLAYER_INFO_KEY;
        let mut screen = screen();
        assert!(!screen.take_player_info_request());
        screen.input(&key(p, true, true));
        assert!(!screen.take_player_info_request(), "not a repeat");
        press(&mut screen, p);
        assert!(screen.take_player_info_request());
        assert!(!screen.take_player_info_request(), "once");
        press(&mut screen, INFO_KEY);
        press(&mut screen, p);
        assert!(screen.take_player_info_request(), "the Info panel");
        assert!(screen.info_panel().is_some(), "which stays open");
        let mut screen = naming_by_key();
        press(&mut screen, p);
        assert!(!screen.take_player_info_request(), "the prompt takes it");
        assert!(screen.naming().is_some());
    }

    #[test]
    fn the_map_key_asks_for_the_map_from_the_shipyard_and_its_panel_not_its_prompt() {
        let mut screen = screen();
        assert_eq!(screen.take_map_request(), None);
        screen.input(&key(MAP_KEY, true, true));
        screen.input(&key(MAP_KEY, false, false));
        assert_eq!(screen.take_map_request(), None, "a repeat or a release");
        press(&mut screen, MAP_KEY);
        assert_eq!(screen.take_map_request(), Some(PortScreen::Shipyard));
        assert_eq!(screen.take_map_request(), None, "once");
        assert!(!screen.closed());
        press(&mut screen, INFO_KEY);
        press(&mut screen, MAP_KEY);
        assert_eq!(
            screen.take_map_request(),
            Some(PortScreen::ShipDescription),
            "the Info panel"
        );
        assert!(screen.info_panel().is_some(), "which stays open");
        let mut screen = naming_by_key();
        press(&mut screen, MAP_KEY);
        assert_eq!(screen.take_map_request(), None, "the prompt takes it");
        assert!(screen.naming().is_some());
    }

    #[test]
    fn the_map_key_asks_nothing_from_a_shipyard_without_its_dialog_or_under_an_offer() {
        let mut screen = ShipyardScreen::new(
            Err("no DLOG 1004".to_owned()),
            info_templates(),
            Ok(text_input_template()),
            shipyard(),
            Rc::clone(&art()) as Rc<dyn ShipyardCatalog>,
            ButtonStyle::STOCK,
        );
        press(&mut screen, MAP_KEY);
        assert_eq!(screen.take_map_request(), None, "without its dialog");
        let mut screen = screen_with_offers();
        screen.open_offer(crate::spaceport::offer::fixture::prompt());
        press(&mut screen, MAP_KEY);
        assert_eq!(screen.take_map_request(), None, "the offer's");
        assert!(screen.take_offer_map_request().is_some());
    }

    fn screen_with_offers() -> ShipyardScreen {
        screen().with_offers(crate::spaceport::offer::fixture::templates())
    }

    #[test]
    fn return_and_escape_close_the_panel_alone_and_it_takes_every_input() {
        for k in [Key::Enter, Key::Escape] {
            let mut screen = screen();
            press(&mut screen, INFO_KEY);
            press(&mut screen, BUY_KEY);
            press(&mut screen, Key::Right);
            assert_eq!(screen.take_request(), None, "the panel takes it");
            assert_eq!(screen.selected(), Some(0));
            screen.input(&key(k, true, true));
            assert!(screen.info_panel().is_some(), "not a repeat");
            press(&mut screen, k);
            assert!(screen.info_panel().is_none(), "{k:?}");
            assert!(!screen.closed());
            press(&mut screen, k);
            assert!(screen.closed());
        }
        let mut screen = screen();
        press(&mut screen, INFO_KEY);
        let done = panel_item(&screen, PANEL_DONE_ITEM).center();
        let button = |pressed| Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at: done,
        };
        screen.input(&button(true));
        screen.cancel_pointer();
        screen.input(&button(false));
        assert!(screen.info_panel().is_some(), "the click was abandoned");
    }

    #[test]
    fn info_variant_shows_a_graphic_above_127_that_exists() {
        let never = |id: i16| -> bool { panic!("asked whether PICT {id} exists") };
        for graphic in [None, Some(0), Some(127), Some(-5)] {
            assert_eq!(
                info_variant(graphic, never),
                InfoVariant::Plain,
                "{graphic:?}"
            );
        }
        let all = |_: i16| true;
        assert_eq!(info_variant(Some(128), all), InfoVariant::Picture(128));
        assert_eq!(info_variant(Some(20128), all), InfoVariant::Picture(20128));
        let asked = RefCell::new(Vec::new());
        let missing = |id: i16| {
            asked.borrow_mut().push(id);
            false
        };
        assert_eq!(
            info_variant(Some(20128), missing),
            InfoVariant::MissingPicture
        );
        assert_eq!(*asked.borrow(), [20128]);
        let accessors = |variant: InfoVariant| {
            (
                variant.dialog(),
                variant.background(),
                variant.graphic(),
                variant.id_test(),
            )
        };
        assert_eq!(
            accessors(InfoVariant::Plain),
            (1005, ImageKey::picture(8506), None, false)
        );
        assert_eq!(
            accessors(InfoVariant::Picture(20128)),
            (1019, ImageKey::picture(8507), Some(20128), true)
        );
        assert_eq!(
            accessors(InfoVariant::MissingPicture),
            (1005, ImageKey::picture(8507), None, true)
        );
    }

    /// The shipyard with the Heavy Shuttle (129) named "The Long 129" in
    /// full, and selected, with `art`.
    fn long_named(art: &Rc<FakeArt>) -> ShipyardScreen {
        let mut shipyard = shipyard();
        shipyard.rows[1].specs.long_name = "The Long 129".to_owned();
        let mut screen = screen_with(shipyard, art);
        press(&mut screen, Key::Right);
        screen
    }

    #[test]
    fn info_on_a_class_with_a_graphic_opens_the_picture_panel() {
        let art = art_with(vec![(13_001, 5002)]);
        let mut screen = long_named(&art);
        art.asked.borrow_mut().clear();
        press(&mut screen, INFO_KEY);
        let panel = screen.info_panel().expect("open");
        assert_eq!(panel.bounds(), rect(205.0, 115.0, 614.0, 537.0));
        assert_eq!(
            *art.asked.borrow(),
            ["graphic 13001".to_owned(), "PICT 5002".to_owned()]
        );
        let commands = drawn(&screen);
        let background = commands
            .iter()
            .position(|c| {
                *c == DrawCommand::Picture {
                    image: PICTURE_INFO_BACKGROUND,
                    top_left: Point::new(205.0, 115.0),
                }
            })
            .expect("the picture background");
        let graphic_area = panel_item(&screen, PANEL_GRAPHIC_ITEM);
        assert_eq!(graphic_area, rect(212.0, 121.0, 600.0, 400.0));
        let graphic = commands
            .iter()
            .position(|c| {
                *c == DrawCommand::StretchedPicture {
                    image: ImageKey::picture(5002),
                    top_left: graphic_area.min,
                    width: 600.0,
                    height: 400.0,
                }
            })
            .expect("the graphic in item 7");
        let done = commands
            .iter()
            .rposition(|c| matches!(c, DrawCommand::Text { text, .. } if text == DONE_LABEL))
            .expect("the panel's Done");
        assert!(
            background < done && done < graphic,
            "the graphic after Done"
        );
        assert!(
            !commands.iter().any(
                |c| matches!(c, DrawCommand::Picture { image, .. } if *image == INFO_BACKGROUND)
            )
        );
        let words = words(&screen);
        assert!(words.contains(&"The Long 129".to_owned()), "{words:?}");
        assert!(!words.contains(&"Ship 129".to_owned()), "{words:?}");
        // Its Done closes it alone.
        let done = panel_item(&screen, PANEL_DONE_ITEM).center();
        click(&mut screen, done);
        assert!(screen.info_panel().is_none());
        assert!(!screen.closed());
    }

    #[test]
    fn info_on_a_class_whose_graphic_is_missing_opens_1005_over_the_picture_background() {
        let art = art_with(vec![(13_001, 20_130)]);
        let mut screen = long_named(&art);
        press(&mut screen, INFO_KEY);
        let panel = screen.info_panel().expect("open");
        assert_eq!(panel.bounds(), rect(387.0, 241.0, 250.0, 285.0));
        let commands = drawn(&screen);
        assert!(commands.contains(&DrawCommand::StretchedPicture {
            image: PICTURE_INFO_BACKGROUND,
            top_left: Point::new(387.0, 241.0),
            width: 250.0,
            height: 285.0,
        }));
        assert!(!commands.iter().any(|c| matches!(
            c,
            DrawCommand::Picture { image, .. } if *image == INFO_BACKGROUND
        )));
        assert!(!commands.iter().any(|c| matches!(
            c,
            DrawCommand::Picture { image, .. } | DrawCommand::StretchedPicture { image, .. }
                if image.id == 20_130
        )));
        let words = words(&screen);
        assert!(words.contains(&"The Long 129".to_owned()), "{words:?}");
        assert!(!words.contains(&"Ship 129".to_owned()), "{words:?}");
    }

    #[test]
    fn without_the_picture_dialog_a_class_with_a_graphic_opens_nothing() {
        let art = art_with(vec![(13_001, 5002)]);
        let mut shipyard = shipyard();
        shipyard.rows[1].specs.long_name = "The Long 129".to_owned();
        let mut screen = ShipyardScreen::new(
            Ok(layout()),
            InfoTemplates {
                picture: Err("no DLOG 1019".to_owned()),
                ..info_templates()
            },
            Ok(text_input_template()),
            shipyard,
            Rc::clone(&art) as Rc<dyn ShipyardCatalog>,
            ButtonStyle::STOCK,
        );
        press(&mut screen, Key::Right);
        assert!(enabled(&screen, INFO_LABEL));
        press(&mut screen, Key::Right);
        press(&mut screen, INFO_KEY);
        assert!(screen.info_panel().is_none());
        // A class without a graphic still opens 1005.
        press(&mut screen, Key::Left);
        press(&mut screen, INFO_KEY);
        let panel = screen.info_panel().expect("open");
        assert_eq!(panel.bounds(), rect(387.0, 241.0, 250.0, 285.0));
    }

    /// The five stock lines of phase 29's list test.
    fn five_weapons() -> Vec<StockLine> {
        vec![
            stock(2, "Light Blaster", None),
            stock(1, "Missile Launcher", Some(10)),
            stock(4, "Viper", None),
            stock(1, "Lone", None),
            stock(2, "", None),
        ]
    }

    #[test]
    fn the_weapons_sentence_joins_each_stock_line_with_a_comma() {
        assert_eq!(weapons_sentence(&[]), "");
        assert_eq!(
            weapons_sentence(&[stock(2, "Light Blaster", None)]),
            "2 Light Blasters"
        );
        assert_eq!(
            weapons_sentence(&five_weapons()),
            "2 Light Blasters, 1 Missile Launcher + 10 ammo, 4 Vipers, 1 Lone, 2 s"
        );
    }

    #[test]
    fn spec_lines_for_drops_the_weapons_only_while_the_id_test_holds() {
        let specs = ShipSpecs {
            weapons: five_weapons(),
            ..SPECS
        };
        assert_eq!(
            spec_lines_for(&specs, InfoVariant::Plain),
            spec_lines(&specs)
        );
        let mut expected = upper_rows();
        expected.extend(lower_rows());
        for variant in [InfoVariant::Picture(5002), InfoVariant::MissingPicture] {
            assert_eq!(spec_lines_for(&specs, variant), expected, "{variant:?}");
        }
    }

    /// The Heavy Shuttle's picture panel, open, its stock weapons
    /// `weapons`.
    fn picture_panel(weapons: Vec<StockLine>) -> ShipyardScreen {
        let art = art_with(vec![(13_001, 5002)]);
        let mut shipyard = shipyard();
        shipyard.rows[1].specs.long_name = "The Long 129".to_owned();
        shipyard.rows[1].specs.weapons = weapons;
        let mut screen = screen_with(shipyard, &art);
        press(&mut screen, Key::Right);
        press(&mut screen, INFO_KEY);
        screen
    }

    /// The black box under item 8's heading: item 8 less its top 16.
    fn weapons_box(screen: &ShipyardScreen) -> Bounds {
        let weapons = panel_item(screen, PANEL_WEAPONS_ITEM);
        Bounds {
            min: Point::new(weapons.min.x, weapons.min.y + 16.0),
            max: weapons.max,
        }
    }

    /// The panel's item-5 texts at phase 29's weapons places, (0, 108)
    /// and (8, 120).
    fn at_item_5s_weapons(screen: &ShipyardScreen) -> Vec<(String, Point, Color)> {
        let area = panel_item(screen, PANEL_TEXT_ITEM);
        let places = [
            Point::new(area.min.x, area.min.y + 98.0),
            Point::new(area.min.x + 8.0, area.min.y + 110.0),
        ];
        texts(&drawn(screen))
            .into_iter()
            .filter(|(_, at, _)| places.contains(at))
            .collect()
    }

    #[test]
    fn the_picture_panel_draws_the_weapons_heading_and_sentence_in_item_8() {
        let screen = picture_panel(five_weapons());
        let weapons = panel_item(&screen, PANEL_WEAPONS_ITEM);
        assert_eq!(weapons, rect(471.0, 557.0, 342.0, 59.0));
        let commands = drawn(&screen);
        let heading = commands
            .iter()
            .position(|c| {
                *c == DrawCommand::Text {
                    text: STANDARD_WEAPONS.to_owned(),
                    font: Font::Geneva,
                    origin: Point::new(weapons.min.x, weapons.min.y + 12.0 - 10.0),
                    size: TEXT_SIZE,
                    wrap_width: None,
                    color: Color::WHITE,
                }
            })
            .expect("the heading in item 8");
        let boxed = weapons_box(&screen);
        assert_eq!(boxed.height(), 43.0);
        let mut black = DrawList::new();
        fill_rect(&mut black, boxed, Color::BLACK);
        let fill = commands
            .iter()
            .position(|c| Some(c) == black.iter().next())
            .expect("the box filled black");
        let lines = crate::text::wrap(
            &MonoMetrics,
            Font::Geneva,
            10.0,
            &weapons_sentence(&five_weapons()),
            342.0,
        );
        assert!(!lines.is_empty());
        for (n, line) in lines.iter().enumerate() {
            let expected = DrawCommand::Text {
                text: line.clone(),
                font: Font::Geneva,
                origin: Point::new(boxed.min.x, boxed.min.y + 12.0 * n as f32),
                size: TEXT_SIZE,
                wrap_width: None,
                color: Color::WHITE,
            };
            let at = commands
                .iter()
                .position(|c| *c == expected)
                .unwrap_or_else(|| panic!("{expected:?}: {commands:?}"));
            assert!(
                heading < fill && fill < at,
                "the heading, the box, then {line}"
            );
        }
        assert_eq!(at_item_5s_weapons(&screen), []);
        let words = words(&screen);
        for row in ["Speed:", "Crew:"] {
            assert!(words.contains(&row.to_owned()), "{row}: {words:?}");
        }
    }

    #[test]
    fn the_weapons_sentence_clips_its_straddling_line_to_the_box() {
        let weapons: Vec<StockLine> = (0..30)
            .map(|n| stock(1, &format!("Gun{n:02}"), None))
            .collect();
        let lines = crate::text::wrap(
            &MonoMetrics,
            Font::Geneva,
            10.0,
            &weapons_sentence(&weapons),
            342.0,
        );
        assert_eq!(lines.len(), 5, "{lines:?}");
        let screen = picture_panel(weapons);
        let boxed = weapons_box(&screen);
        let commands = drawn(&screen);
        let line = |n: usize| DrawCommand::Text {
            text: lines[n].clone(),
            font: Font::Geneva,
            origin: Point::new(boxed.min.x, boxed.min.y + 12.0 * n as f32),
            size: TEXT_SIZE,
            wrap_width: None,
            color: Color::WHITE,
        };
        for (n, text) in lines[..3].iter().enumerate() {
            assert!(commands.contains(&line(n)), "{text}: {commands:?}");
        }
        assert!(
            commands.contains(&DrawCommand::Clipped {
                clip: boxed,
                command: Box::new(line(3)),
            }),
            "its top is 36 and its bottom 48, past the box's 43: {commands:?}"
        );
        assert!(!commands.contains(&line(3)), "not drawn unclipped");
        let anywhere = |text: &str| {
            commands.iter().any(|c| match c {
                DrawCommand::Text { text: t, .. } => t == text,
                DrawCommand::Clipped { command, .. } => {
                    matches!(&**command, DrawCommand::Text { text: t, .. } if t == text)
                }
                _ => false,
            })
        };
        assert!(!anywhere(&lines[4]), "its top is 48, below the box's 43");
    }

    #[test]
    fn the_picture_panel_without_stock_weapons_says_so_in_item_8_with_no_box() {
        let screen = picture_panel(Vec::new());
        let weapons = panel_item(&screen, PANEL_WEAPONS_ITEM);
        let drawn_texts = texts(&drawn(&screen));
        assert!(drawn_texts.contains(&(
            NO_STANDARD_WEAPONS.to_owned(),
            Point::new(weapons.min.x, weapons.min.y + 2.0),
            Color::WHITE
        )));
        let mut black = DrawList::new();
        fill_rect(&mut black, weapons_box(&screen), Color::BLACK);
        assert!(
            !drawn(&screen)
                .iter()
                .any(|c| Some(c) == black.iter().next())
        );
        assert!(
            !drawn_texts
                .iter()
                .any(|(_, at, _)| weapons_box(&screen).contains(*at))
        );
        assert_eq!(at_item_5s_weapons(&screen), []);
    }

    #[test]
    fn the_missing_picture_panel_draws_no_weapons() {
        let art = art_with(vec![(13_001, 20_130)]);
        let mut shipyard = shipyard();
        shipyard.rows[1].specs.weapons = five_weapons();
        let mut screen = screen_with(shipyard, &art);
        press(&mut screen, Key::Right);
        press(&mut screen, INFO_KEY);
        assert_eq!(
            screen.info_panel().expect("open").bounds(),
            rect(387.0, 241.0, 250.0, 285.0)
        );
        let words = words(&screen);
        for gone in [STANDARD_WEAPONS, NO_STANDARD_WEAPONS, "2 Light Blasters"] {
            assert!(!words.contains(&gone.to_owned()), "{gone}: {words:?}");
        }
        assert!(
            !words.iter().any(|word| word.contains("Light Blaster")),
            "{words:?}"
        );
        assert!(words.contains(&"Speed:".to_owned()), "{words:?}");
    }

    /// The panel's title, as drawn: its words, font, size, origin and
    /// colour.
    fn title_of(screen: &ShipyardScreen, words: &str) -> (Font, f32, Point, Color) {
        drawn(screen)
            .into_iter()
            .find_map(|c| match c {
                DrawCommand::Text {
                    text,
                    font,
                    size,
                    origin,
                    color,
                    ..
                } if text == words => Some((font, size, origin, color)),
                _ => None,
            })
            .unwrap_or_else(|| panic!("no title {words:?}"))
    }

    #[test]
    fn the_panel_title_is_times_18_centred_on_baseline_18() {
        for (art, title) in [
            (art(), "Ship 129"),
            (art_with(vec![(13_001, 5002)]), "The Long 129"),
        ] {
            let mut screen = long_named(&art);
            press(&mut screen, INFO_KEY);
            let area = panel_item(&screen, PANEL_TITLE_ITEM);
            let middle = f32::midpoint(area.min.x, area.max.x).floor();
            let width = MonoMetrics.width(Font::Times, 18.0, title);
            assert_eq!(
                title_of(&screen, title),
                (
                    Font::Times,
                    18.0,
                    Point::new(middle - width / 2.0, area.min.y + 18.0 - 18.0),
                    Color::WHITE
                ),
                "{title}"
            );
        }
    }

    #[test]
    fn without_the_info_dialog_info_is_greyed() {
        // Ship 129's `dësc` 13001 has a graphic whose picture dialog is
        // there: greyed Info still opens nothing for it.
        for (art, select) in [(art(), 0), (art_with(vec![(13_001, 5002)]), 1)] {
            let mut screen = ShipyardScreen::new(
                Ok(layout()),
                InfoTemplates {
                    plain: Err("no DLOG 1005".to_owned()),
                    picture: Ok(picture_info_template()),
                },
                Ok(text_input_template()),
                shipyard(),
                art,
                ButtonStyle::STOCK,
            );
            assert!(!enabled(&screen, INFO_LABEL));
            for _ in 0..=select {
                press(&mut screen, Key::Right);
            }
            assert_eq!(screen.selected(), Some(select));
            press(&mut screen, INFO_KEY);
            click_item(&mut screen, INFO_ITEM);
            assert!(screen.info_panel().is_none(), "{select}");
        }
    }

    // Closing.

    #[test]
    fn done_return_and_escape_close_it() {
        let mut screen = screen();
        click_item(&mut screen, DONE_ITEM);
        assert!(screen.closed());
        for k in [Key::Enter, Key::Escape] {
            let mut screen = screen_of(shipyard());
            assert_eq!(screen.input(&key(k, true, false)), ScreenAction::None);
            assert!(screen.closed(), "{k:?}");
            assert_eq!(screen.take_request(), None);
        }
        let mut held = screen_of(shipyard());
        held.input(&key(Key::Escape, true, true));
        held.tick(Duration::from_secs(1));
        assert!(!held.closed());
    }

    /// `_TrackShipyardButtons` @0x2a54e plays nothing under the pointer.
    #[test]
    fn pointer_click_is_silent_as_the_original_and_cancelling_the_pointer_abandons_a_click() {
        let mut screen = screen();
        press(&mut screen, Key::Right);
        let buy = item(&screen, BUY_ITEM).center();
        let button = |pressed| Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at: buy,
        };
        screen.input(&button(true));
        assert_eq!(screen.take_sounds(), []);
        screen.cancel_pointer();
        screen.input(&button(false));
        assert_eq!(screen.take_request(), None);
        press(&mut screen, BUY_KEY);
        press(&mut screen, Key::Enter);
        assert_eq!(screen.take_sounds(), [], "keys are silent");
        let debug = format!("{screen:?}");
        assert!(debug.contains("ShipyardCatalog"), "{debug}");
        assert!(debug.contains("TextMetrics"), "{debug}");
    }

    fn problem(reason: &str) -> DrawCommand {
        DrawCommand::Text {
            text: format!(
                "Cannot show the shipyard: {reason}. Press Return or Escape to close it."
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
            let mut screen = ShipyardScreen::new(
                Err("no DLOG 1004".to_owned()),
                info_templates(),
                Ok(text_input_template()),
                shipyard(),
                Rc::clone(&art) as Rc<dyn ShipyardCatalog>,
                ButtonStyle::STOCK,
            );
            assert_eq!(screen.problem(), Some("no DLOG 1004"));
            assert!(screen.dialog().is_none());
            assert_eq!(screen.cell_bounds(0), None);
            assert_eq!(drawn(&screen), [problem("no DLOG 1004")]);
            for other in [
                key(k, true, true),
                key(k, false, false),
                key(BUY_KEY, true, false),
                key(INFO_KEY, true, false),
            ] {
                screen.input(&other);
            }
            click(&mut screen, Point::new(500.0, 500.0));
            screen.cancel_pointer();
            assert!(!screen.closed());
            assert_eq!(screen.take_request(), None);
            assert!(screen.info_panel().is_none());
            screen.open_naming(&naming());
            assert!(screen.naming().is_none(), "nothing to open it over");
            assert_eq!(*art.asked.borrow(), Vec::<String>::new());
            screen.input(&key(k, true, false));
            assert!(screen.closed(), "{k:?}");
        }
    }

    // The shipyard's mission offers.

    use crate::spaceport::offer::OfferChoice;
    use crate::spaceport::offer::fixture::{prompt, templates as offer_templates};

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    /// The shipyard, its offers given.
    fn offering() -> ShipyardScreen {
        screen().with_offers(offer_templates())
    }

    #[test]
    fn the_shipyard_with_offers_asks_for_an_offer_as_it_opens() {
        let mut screen = offering();
        assert!(screen.take_offer_request(), "before any tick");
        assert!(!screen.take_offer_request(), "once");
        screen.tick(ms(10_000));
        assert!(!screen.take_offer_request(), "not again until rearmed");
    }

    #[test]
    fn without_offers_the_shipyard_never_asks() {
        let mut screen = screen();
        assert!(!screen.take_offer_request());
        screen.tick(ms(10_000));
        assert!(!screen.take_offer_request());
        screen.open_offer(prompt());
        assert!(screen.offer_dialog().is_none());
    }

    #[test]
    fn an_offer_opens_over_the_shipyard_and_takes_its_input() {
        let mut screen = offering();
        screen.open_offer(prompt());
        let offer = screen.offer_dialog().expect("open");
        assert_eq!(offer.prompt(), &prompt());
        assert_eq!(offer.problem(), None, "laid out by the template given");
        let mut expected = drawn(&self::screen());
        let mut over = DrawList::new();
        offer.draw(&mut over);
        expected.extend(over.iter().cloned());
        assert_eq!(drawn(&screen), expected, "drawn over the shipyard");
        press(&mut screen, BUY_KEY);
        assert_eq!(screen.take_request(), None, "B went to the offer");
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
    fn the_shipyards_offer_pointer_goes_through_it() {
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
    fn after_a_rearm_the_shipyard_asks_again_after_the_gap() {
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
    fn by_the_other_reading_the_shipyard_waits_out_its_info_panel_and_name_prompt() {
        let mut screen = offering().with_offer_clock(RuleSource::Bible);
        assert!(screen.take_offer_request());
        screen.rearm(ms(600));
        press(&mut screen, INFO_KEY);
        assert!(screen.info_panel().is_some());
        screen.tick(ms(10_000));
        assert!(!screen.take_offer_request(), "paused under the info panel");
        press(&mut screen, Key::Escape);
        assert!(screen.info_panel().is_none());
        screen.tick(ms(300));
        screen.open_naming(&naming());
        assert!(screen.naming().is_some());
        screen.tick(ms(10_000));
        assert!(!screen.take_offer_request(), "paused under the name prompt");
        naming_click(&mut screen, crate::ui::text_input::CANCEL_ITEM);
        assert!(screen.naming().is_none());
        screen.tick(ms(299));
        assert!(!screen.take_offer_request());
        screen.tick(ms(1));
        assert!(
            screen.take_offer_request(),
            "the wait goes on once it closes"
        );
    }

    #[test]
    fn by_the_engine_an_offer_due_under_the_info_panel_or_name_prompt_is_asked_for_as_it_closes() {
        let mut screen = offering();
        assert!(screen.take_offer_request());
        screen.rearm(ms(600));
        press(&mut screen, INFO_KEY);
        assert!(screen.info_panel().is_some());
        screen.tick(ms(10_000));
        assert!(!screen.take_offer_request(), "not under the info panel");
        press(&mut screen, Key::Escape);
        assert!(screen.info_panel().is_none());
        screen.tick(ms(0));
        assert!(screen.take_offer_request(), "at once once it closes");
        screen.rearm(ms(600));
        screen.open_naming(&naming());
        assert!(screen.naming().is_some());
        screen.tick(ms(10_000));
        assert!(!screen.take_offer_request(), "not under the name prompt");
        naming_click(&mut screen, crate::ui::text_input::CANCEL_ITEM);
        assert!(screen.naming().is_none());
        screen.tick(ms(0));
        assert!(screen.take_offer_request(), "at once once it closes");
    }

    #[test]
    fn the_shipyards_gap_runs_from_the_ask_by_the_engine_and_the_answer_by_the_other() {
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
    fn covered_the_shipyard_runs_only_its_offer_clock() {
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
    fn the_mission_info_key_asks_from_the_shipyard_and_its_panel_not_its_prompt() {
        let i = crate::ui::mission_info::MISSION_INFO_KEY;
        let mut screen = screen();
        assert!(!screen.take_info_request());
        screen.input(&key(i, true, true));
        assert!(!screen.take_info_request(), "not a repeat");
        press(&mut screen, i);
        assert!(screen.take_info_request());
        assert!(!screen.take_info_request(), "once");
        assert!(screen.info_panel().is_none(), "I no longer opens the panel");
        assert!(!screen.closed());
        press(&mut screen, INFO_KEY);
        assert!(screen.info_panel().is_some(), "Space does");
        press(&mut screen, i);
        assert!(screen.take_info_request(), "the Info panel");
        assert!(screen.info_panel().is_some(), "which stays open");
        let mut screen = naming_by_key();
        press(&mut screen, i);
        assert!(!screen.take_info_request(), "the prompt takes it");
        assert!(screen.naming().is_some());
    }

    #[test]
    fn the_mission_info_key_asks_nothing_without_the_dialog() {
        let art = art();
        let mut screen = ShipyardScreen::new(
            Err("no DLOG 1004".to_owned()),
            info_templates(),
            Ok(text_input_template()),
            shipyard(),
            Rc::clone(&art) as Rc<dyn ShipyardCatalog>,
            ButtonStyle::STOCK,
        );
        press(&mut screen, crate::ui::mission_info::MISSION_INFO_KEY);
        assert!(!screen.take_info_request());
    }

    #[test]
    fn the_mission_info_key_asks_from_its_offer_which_stays_open() {
        let mut screen = offering();
        screen.open_offer(prompt());
        press(&mut screen, crate::ui::mission_info::MISSION_INFO_KEY);
        assert!(screen.take_info_request());
        assert!(screen.offer_dialog().is_some());
    }

    /// The original's `_ShipyardFilter` @0x60449 maps keys to items with no click sound; the arrows also cover `grid`.
    #[test]
    fn keyboard_activation_is_silent_as_the_original() {
        for k in [
            Key::Right,
            Key::Down,
            Key::Left,
            Key::Up,
            BUY_KEY,
            INFO_KEY,
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
            let refused: &[Sound] = if k == BUY_KEY {
                &[Sound::Ui(UiSound::BuyRefused)]
            } else {
                &[]
            };
            assert_eq!(
                screen.take_sounds(),
                refused,
                "{k:?} makes no click; B on the class flown beeps"
            );
        }
    }
}
