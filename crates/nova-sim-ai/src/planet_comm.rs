//! The planet comm dialog's decisions: what a stellar the player hails
//! answers, as `_DoPlanetCommDialog` (@0x96949-0x97b96 in the `EV Nova`
//! executable), `_DrawPlanetCommDialogButtons` (@0x2893c) and
//! `_PlanetCommDialogUpdate` (@0x90a98) decide it. The session applies
//! what each press comes to (`nova_sim::Session`'s `planet_comm`
//! module), and the view draws [`PlanetCommView`].
//!
//! **The open** ([`PlanetComm::open`]): the variant every reply is said in
//! ([`VARIANTS`], `_responseRandom` @0x969c3); the bribe's price
//! ([`bribe_price`]); whether the stellar refuses the player landing
//! ([`refuses_landing`], `_hatesPlayer`); the system's bribe roll, drawn
//! only while the system has none ([`BRIBE_ROLL`], `_bribeRandom`
//! @0x96c66-0x96c7b), which serves every planet hail until the next
//! arrival; and whether the stellar takes a bribe ([`takes_bribe`],
//! `_willTakeBribe`). Its first reply greets the player
//! ([`PlanetReply::ChannelOpen`]), or, refusing and not dominated, asks
//! what it wants ([`PlanetReply::WhatDoYouWant`]).
//!
//! **The buttons** ([`PlanetCommView::second`],
//! [`PlanetCommView::third`]): item 2 is Greetings, or Offer Bribe while
//! the stellar refuses landing; item 3 is Demand Tribute, or Release while
//! it is dominated, and none for a stellar always dominated (`Flags2`
//! 0x0020). A press names the item ([`PlanetCommItem`]), as the
//! original's filter and track do, so a click where a hidden item 3
//! stands presses it.
//!
//! **A press** ([`PlanetComm::press`]) of item 2 or 3 first says "No
//! response." ([`PlanetReply::NoResponse`], @0x96ff7), which every branch
//! but item 3 of an always-dominated stellar replaces:
//!
//! - Item 2, not refusing: Greetings ([`PlanetReply::Greetings`]).
//!   Refusing: a dominated stellar wastes no time
//!   ([`PlanetReply::WastingTime`]); one that takes no bribe scoffs
//!   ([`PlanetReply::NoBribe`]); any other asks its price
//!   ([`PlanetReply::BribeAsked`], [`Pressed::Asked`]) and the player
//!   haggles ([`PlanetComm::haggle`], the ship bribe's haggle,
//!   [`Haggling`]). Short of cash it wastes no time; a price paid clears
//!   the player to land ([`BRIBE_PERMISSION`]); any other choice declines,
//!   setting the system's roll to 0 and raising the price by
//!   [`DECLINE_RAISE`] ([`PlanetReply::BribeDeclined`]).
//! - Item 3 on a dominated stellar: Release ([`Pressed::Released`],
//!   [`PlanetReply::Released`]), with the record write
//!   ([`record_after_offence`]).
//! - Item 3 on any other: Demand Tribute ([`Pressed::Demanded`]): the
//!   stellar refuses landing from then on, the record write, and
//!   [`demand`]'s outcome, each with its slaps ([`Demand::slaps`]).
//!   Every demand marks the dialog, so a second demand in it is fought.
//!
//! The sounds are the session's: `_beepSnd[4]` as it opens, `_beepSnd[1]`
//! for each press of item 2 or 3 and as the bribe's haggle ends, and
//! `_beepSnd[2]` as it closes.

use crate::catalog::{CommCatalog, StellarId, StringPatch};
use crate::chance::Chance;
use crate::hail::deal::{self, Haggle, Haggling, Settled};
use crate::hail::reply::COMM_STRINGS;

/// How many variants the dialog's replies are said in (`Rand(5)`
/// @0x969bc).
pub const VARIANTS: u32 = 5;
/// The system's bribe roll's draw (`Rand(100)` @0x96c6f).
pub const BRIBE_ROLL: u32 = 100;
/// The bribe roll a stellar must beat to take a bribe (@0x96c7e).
pub const BRIBE_ROLL_ABOVE: u8 = 30;
/// The price's draws per credit the player has (0xddad8).
pub const PRICE_STEPS_PER_CREDIT: f64 = 1e-6;
/// What each step drawn adds to the price.
pub const PRICE_STEP: i64 = 1000;
/// The price before the steps.
pub const PRICE_BASE: i64 = 3000;
/// What a greedy government's planets ask more (0xdd640).
pub const GREEDY_SHARE: f64 = 1.5;
/// The most of the player's cash a price may be (0xdd0c8).
pub const CASH_SHARE: f64 = 0.333;
/// What a price is rounded down to a multiple of.
pub const PRICE_ROUNDING: i64 = 1000;
/// The highest price (0xdbba0).
pub const MAX_PRICE: i64 = 900_000;
/// The lowest price.
pub const MIN_PRICE: i64 = 1000;
/// What declining raises the price by (@0x9728c), as a ship's decline
/// does.
pub const DECLINE_RAISE: i64 = deal::DECLINE_RAISE;
/// The haggle-win bound the bribe's haggle passes (@0x970ae), the ship
/// bribe's.
pub const HAGGLE_WINS_AT_MOST: u32 = deal::HAGGLE_WINS_AT_MOST;
/// The dock request's counter a bribe paid grants (@0x97117): past
/// `clearance::CLEARED`, so the land key lands at once.
pub const BRIBE_PERMISSION: i16 = 0x2ef;
/// The combat rating up to which a demand is laughed off (0x31ff,
/// @0x975da).
pub const LAUGH_AT_MOST: i64 = 12_799;
/// The slaps a fight that sends a wave, and a domination, make after the
/// first (@0x976e9, @0x97864).
pub const MORE_SLAPS: u32 = 5;
/// A government's `Flags` bit whose planets take bribes (gövt +0x21 &
/// 0x40, @0x96cbc).
pub const PLANETS_TAKE_BRIBES: u16 = 0x4000;
/// A `spöb` `Flags2` bit: always dominated, which hides item 3.
pub const ALWAYS_DOMINATED: u16 = 0x0020;
/// The `STR#` of the stellars' replies.
pub const STELLAR_STRINGS: i16 = 3002;
/// The `STR#` of the stellar type labels, #`Type` + 1 (@0x96b26).
pub const TYPE_LABELS: i16 = 1100;
/// The first `'STR '` patching a type label: 7000 + `Type` (@0x96b02).
pub const TYPE_LABEL_PATCHES: i16 = 7000;

/// Whether a stellar of `MinStatus` `min_status` refuses the player of
/// legal `record` landing as the dialog opens (`_hatesPlayer`,
/// @0x96ba9-0x96c57): when the record alone does not open it
/// ([`landing::open_by_record`](crate::landing::open_by_record)), unless
/// the stellar is `governed` and its government blesses the player with
/// landing (`blessed`, `_PlayerBlessedByGovt(govt, 1)` @0x96c15) or carries
/// the IFF scrambler's mark (`iff_marked`, gövt +0x83 @0x96c4d).
#[must_use]
pub fn refuses_landing(
    min_status: i16,
    record: i16,
    governed: bool,
    blessed: bool,
    iff_marked: bool,
) -> bool {
    !crate::landing::open_by_record(min_status, record) && !(governed && (blessed || iff_marked))
}

/// The bribe-taking traits of a stellar's government, from its `Flags`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Bribery {
    /// `Flags` 0x8000: its planets take a bribe whatever the roll, and
    /// ask half as much again.
    pub greedy: bool,
    /// `Flags` 0x4000 ([`PLANETS_TAKE_BRIBES`]).
    pub planets_take_bribes: bool,
}

/// Whether a stellar takes a bribe, the system's bribe roll at `roll`
/// (`_willTakeBribe`, @0x96c5a-0x96d0b): its government is greedy; or
/// the roll is above [`BRIBE_ROLL_ABOVE`] and it is independent (`govt`
/// none) or its government's planets take bribes.
#[must_use]
pub fn takes_bribe(roll: u8, govt: Option<Bribery>) -> bool {
    match govt {
        Some(govt) if govt.greedy => true,
        Some(govt) => roll > BRIBE_ROLL_ABOVE && govt.planets_take_bribes,
        None => roll > BRIBE_ROLL_ABOVE,
    }
}

/// The bribe's price for a player of `cash`, a `greedy` government's
/// half as much again, drawn on `chance` (@0x969f5-0x96ad1): a draw of
/// trunc(cash × [`PRICE_STEPS_PER_CREDIT`]) steps, at least 1, of
/// [`PRICE_STEP`] on [`PRICE_BASE`]; then at most trunc(cash ×
/// [`CASH_SHARE`]), rounded down to [`PRICE_ROUNDING`], at most
/// [`MAX_PRICE`] and at least [`MIN_PRICE`], in that order. So a player
/// of no cash is asked [`MIN_PRICE`].
pub fn bribe_price(cash: i64, greedy: bool, chance: &mut dyn Chance) -> i64 {
    let steps = u32::try_from((cash as f64 * PRICE_STEPS_PER_CREDIT) as i64)
        .unwrap_or(u32::MAX)
        .max(1);
    let mut price = i64::from(chance.below(steps)) * PRICE_STEP + PRICE_BASE;
    if greedy {
        price = (price as f64 * GREEDY_SHARE) as i64;
    }
    let price = price.min((cash as f64 * CASH_SHARE) as i64);
    let price = price / PRICE_ROUNDING * PRICE_ROUNDING;
    price.clamp(MIN_PRICE, MAX_PRICE)
}

/// The legal record a Release or a Demand Tribute leaves of `record`, at
/// a stellar of `MinStatus` `min_status` (@0x9747e-0x974ba,
/// @0x97567-0x97593): `MinStatus` - 1 when the record is at least
/// `MinStatus` and `MinStatus` is strictly between -32767 and 32767;
/// otherwise none, the record as it is.
#[must_use]
pub fn record_after_offence(record: i16, min_status: i16) -> Option<i16> {
    (record >= min_status && min_status > -32_767 && min_status < 32_767).then(|| min_status - 1)
}

/// What Demand Tribute reads (@0x975d5-0x97669).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DemandFacts {
    /// The player's combat rating (`_playerKills`).
    pub rating: i64,
    /// Whether cheat mode is on (`_cheatsActive`): the engine has none
    /// (rdm:task/cheat-mode-beyond-player-info), so the session passes
    /// `false`.
    pub cheats: bool,
    /// Whether tribute was demanded already in this dialog
    /// (`_hasDemandedTribute`).
    pub demanded: bool,
    /// The stellar's defence ships left (+0x466).
    pub defenders_left: i16,
    /// Whether the stellar is under attack (+0x47).
    pub under_attack: bool,
}

/// What Demand Tribute comes to (see [`demand`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Demand {
    /// The stellar laughs it off.
    Laugh,
    /// It fights, sending its defence fleet up when `send_wave`.
    Fight {
        /// Whether a wave goes up (+0x47 clear): with five more slaps.
        send_wave: bool,
    },
    /// It agrees to pay tribute: it is dominated.
    Dominate,
}

impl Demand {
    /// How many slaps it makes: the first, then [`MORE_SLAPS`] for a fight
    /// that sends a wave and for a domination.
    #[must_use]
    pub fn slaps(self) -> u32 {
        match self {
            Self::Laugh | Self::Fight { send_wave: false } => 1,
            Self::Fight { send_wave: true } | Self::Dominate => 1 + MORE_SLAPS,
        }
    }
}

/// What Demand Tribute comes to with `facts`: a laugh at a rating up to
/// [`LAUGH_AT_MOST`] with cheats off (@0x975d5-0x975ea); else a fight
/// when tribute was demanded already, the stellar has defence ships left
/// or it is under attack, sending a wave only when it is not under attack
/// (@0x9763b-0x976fd); else domination.
#[must_use]
pub fn demand(facts: DemandFacts) -> Demand {
    if facts.rating <= LAUGH_AT_MOST && !facts.cheats {
        Demand::Laugh
    } else if facts.demanded || facts.defenders_left > 0 || facts.under_attack {
        Demand::Fight {
            send_wave: !facts.under_attack,
        }
    } else {
        Demand::Dominate
    }
}

/// A reply the stellar says: an `STR#` 3000 or 3002 entry, most picked by
/// the dialog's variant `rr`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlanetReply {
    /// `STR#` 3002 #(rr + 1) + the stellar's name + "." (@0x96d4e-0x96e01).
    ChannelOpen,
    /// `STR#` 3000 #(rr + 11) (`_LoadResponse(2)`, @0x96e03).
    WhatDoYouWant,
    /// `STR#` 3000 #(rr + 6) (`_LoadResponse(1)`, @0x96ff7).
    NoResponse,
    /// `STR#` 3000 #(rr + 46) (`_LoadResponse(9)`, @0x97305).
    Greetings,
    /// `STR#` 3002 #(rr + 21) (@0x97313, @0x972aa).
    WastingTime,
    /// `STR#` 3002 #(rr + 46) (@0x972d8).
    NoBribe,
    /// `STR#` 3002 #(rr + 41) (@0x97072).
    BribeAsked,
    /// `STR#` 3002 #(rr + 31) (@0x97267).
    BribeDeclined,
    /// `STR#` 3002 #(rr + 6) (@0x97604).
    Laugh,
    /// `STR#` 3002 #(rr + 11) (@0x9768b).
    Fight,
    /// `STR#` 3002 #27 for a station, else #26 (@0x9786a-0x978b9).
    Dominated {
        /// Whether the stellar is a station (`Flags` 0x10).
        station: bool,
    },
    /// `STR#` 3002 #37 for a station, else #36 (@0x973a2-0x973dc).
    Released {
        /// Whether the stellar is a station (`Flags` 0x10).
        station: bool,
    },
}

impl PlanetReply {
    /// The reply's `STR#` and its 1-based entry in `variant`.
    #[must_use]
    pub fn entry(self, variant: u8) -> (i16, u16) {
        let picked = |list, first: u16| (list, first + u16::from(variant));
        match self {
            Self::ChannelOpen => picked(STELLAR_STRINGS, 1),
            Self::WhatDoYouWant => picked(COMM_STRINGS, 11),
            Self::NoResponse => picked(COMM_STRINGS, 6),
            Self::Greetings => picked(COMM_STRINGS, 46),
            Self::WastingTime => picked(STELLAR_STRINGS, 21),
            Self::NoBribe => picked(STELLAR_STRINGS, 46),
            Self::BribeAsked => picked(STELLAR_STRINGS, 41),
            Self::BribeDeclined => picked(STELLAR_STRINGS, 31),
            Self::Laugh => picked(STELLAR_STRINGS, 6),
            Self::Fight => picked(STELLAR_STRINGS, 11),
            Self::Dominated { station } => (STELLAR_STRINGS, if station { 27 } else { 26 }),
            Self::Released { station } => (STELLAR_STRINGS, if station { 37 } else { 36 }),
        }
    }

    /// The reply's words in `variant`, read from `catalog`, the stellar
    /// named `name`: an entry missing is empty. Nothing is filled in, as
    /// the dialog calls no `_MungeBriefing`.
    #[must_use]
    pub fn say(self, variant: u8, name: &str, catalog: &(impl CommCatalog + ?Sized)) -> String {
        let (list, index) = self.entry(variant);
        let line = catalog
            .string_list(list)
            .into_iter()
            .nth(usize::from(index) - 1)
            .unwrap_or_default();
        if self == Self::ChannelOpen {
            format!("{line}{name}.")
        } else {
            line
        }
    }
}

/// The type label of a stellar of `Type` `graphic`, read from `catalog`
/// (@0x96afd-0x96b37): its `'STR '` 7000 + `Type` when a plug-in has one
/// (empty when that has no text or can't be read), else `STR#` 1100
/// #(`Type` + 1), empty when missing.
#[must_use]
pub fn type_label(graphic: i16, catalog: &(impl CommCatalog + ?Sized)) -> String {
    let patch = TYPE_LABEL_PATCHES
        .checked_add(graphic)
        .map_or(StringPatch::Absent, |id| catalog.plugin_string(id));
    match patch {
        StringPatch::Text(text) => text,
        StringPatch::Empty | StringPatch::Unreadable => String::new(),
        StringPatch::Absent => usize::try_from(graphic)
            .ok()
            .and_then(|at| catalog.string_list(TYPE_LABELS).into_iter().nth(at))
            .unwrap_or_default(),
    }
}

/// A button of the dialog, by what it says (`STR#` 150).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlanetCommButton {
    /// "Greetings" (#22).
    Greetings,
    /// "Offer Bribe" (#24).
    OfferBribe,
    /// "Demand Tribute" (#45).
    DemandTribute,
    /// "Release" (#32).
    Release,
}

/// "Greetings": `STR#` 150 #22.
pub const GREETINGS: &str = "Greetings";
/// "Offer Bribe": `STR#` 150 #24.
pub const OFFER_BRIBE: &str = "Offer Bribe";
/// "Demand Tribute": `STR#` 150 #45.
pub const DEMAND_TRIBUTE: &str = "Demand Tribute";
/// "Release": `STR#` 150 #32.
pub const RELEASE: &str = "Release";

impl PlanetCommButton {
    /// What it says.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Greetings => GREETINGS,
            Self::OfferBribe => OFFER_BRIBE,
            Self::DemandTribute => DEMAND_TRIBUTE,
            Self::Release => RELEASE,
        }
    }
}

/// An item of the dialog pressed, by its place (`DITL` 1009 items 1 to
/// 3), as the filter and the track give it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlanetCommItem {
    /// Item 1, "Close Channel".
    Close,
    /// Item 2, Greetings or Offer Bribe.
    Second,
    /// Item 3, Demand Tribute or Release, hidden for a stellar always
    /// dominated.
    Third,
}

/// The status line after "Status:" (`STR#` 2002 #196), as
/// `_PlanetCommDialogUpdate` draws it (@0x90fb6-0x910e1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlanetStatus {
    /// "Owned" (#171): always dominated, in bright green.
    Owned,
    /// "Dominated" (#172), in bright green.
    Dominated,
    /// "Forbidden" (#173): refusing landing with a record of 0 or more,
    /// bold orange.
    Forbidden,
    /// "Hostile" (#174): refusing landing with a record below 0, bold red.
    Hostile,
}

/// "Status:": `STR#` 2002 #196.
pub const STATUS_LABEL: &str = "Status:";
/// "Owned": `STR#` 2002 #171.
pub const OWNED: &str = "Owned";
/// "Dominated": `STR#` 2002 #172.
pub const DOMINATED_STATUS: &str = "Dominated";
/// "Forbidden": `STR#` 2002 #173.
pub const FORBIDDEN: &str = "Forbidden";
/// "Hostile": `STR#` 2002 #174.
pub const HOSTILE: &str = "Hostile";

impl PlanetStatus {
    /// What it says.
    #[must_use]
    pub fn text(self) -> &'static str {
        match self {
            Self::Owned => OWNED,
            Self::Dominated => DOMINATED_STATUS,
            Self::Forbidden => FORBIDDEN,
            Self::Hostile => HOSTILE,
        }
    }
}

/// What the planet comm dialog shows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanetCommView {
    /// The stellar hailed, whose sprite item 5 shows.
    pub stellar: StellarId,
    /// Its name, item 6's first line.
    pub name: String,
    /// Its type label, item 6's second line ([`type_label`]).
    pub type_label: String,
    /// Its reply, item 4.
    pub reply: String,
    /// Item 2.
    pub second: PlanetCommButton,
    /// Item 3: none while hidden.
    pub third: Option<PlanetCommButton>,
    /// The status line, if any.
    pub status: Option<PlanetStatus>,
    /// The bribe's price, while the haggle dialog asks it.
    pub asking: Option<i64>,
}

/// What a stellar and its government are as the dialog opens.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Opening {
    /// The stellar's `MinStatus`.
    pub min_status: i16,
    /// The player's legal record there.
    pub record: i16,
    /// Its government's bribery, none for an independent stellar.
    pub govt: Option<Bribery>,
    /// Whether its government blesses the player with landing.
    pub blessed: bool,
    /// Whether its government carries the IFF scrambler's mark.
    pub iff_marked: bool,
    /// The player's cash.
    pub cash: i64,
}

/// What a stellar is as a press reads it, and as the view shows it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Standing {
    /// Whether it is dominated (+0x46), by the player or always.
    pub dominated: bool,
    /// Whether it is always dominated (`Flags2` [`ALWAYS_DOMINATED`]).
    pub always_dominated: bool,
    /// Whether it is a station (`Flags` 0x10).
    pub station: bool,
    /// The player's legal record there.
    pub record: i16,
    /// Its `MinStatus`.
    pub min_status: i16,
}

/// What a press came to, for the session to apply.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pressed {
    /// Item 1: the dialog closes.
    Closed,
    /// A reply was said, and nothing more done.
    Said,
    /// The bribe's price is asked: the haggle dialog opens on it.
    Asked(i64),
    /// Release: the stellar is released, the record written when `record`
    /// is some.
    Released {
        /// The record to write ([`record_after_offence`]).
        record: Option<i16>,
    },
    /// Demand Tribute came to `demand`, the record written when `record`
    /// is some.
    Demanded {
        /// What it came to.
        demand: Demand,
        /// The record to write ([`record_after_offence`]).
        record: Option<i16>,
    },
}

/// The dialog open, what lasts for it (see the module docs).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlanetComm {
    /// The stellar hailed.
    stellar: StellarId,
    /// The variant its replies are said in (`_responseRandom`).
    variant: u8,
    /// The bribe's price (`_initialHaggleVal`).
    price: i64,
    /// The bribe's haggle.
    haggling: Haggling,
    /// Whether it refuses the player landing (`_hatesPlayer`).
    hates: bool,
    /// Whether it takes a bribe (`_willTakeBribe`).
    takes_bribe: bool,
    /// Whether tribute was demanded in this dialog (`_hasDemandedTribute`).
    demanded: bool,
    /// What it said last.
    reply: PlanetReply,
}

impl PlanetComm {
    /// The dialog opening on `stellar`, `dominated` or not, with `facts`,
    /// the system's bribe roll `roll` (none for the original's -1, drawn
    /// then), drawing on `chance` (see the module docs).
    pub fn open(
        stellar: StellarId,
        dominated: bool,
        facts: &Opening,
        roll: &mut Option<u8>,
        chance: &mut dyn Chance,
    ) -> Self {
        let variant = chance.below(VARIANTS) as u8;
        let price = bribe_price(
            facts.cash,
            facts.govt.is_some_and(|govt| govt.greedy),
            chance,
        );
        let hates = refuses_landing(
            facts.min_status,
            facts.record,
            facts.govt.is_some(),
            facts.blessed,
            facts.iff_marked,
        );
        let drawn = *roll.get_or_insert_with(|| chance.below(BRIBE_ROLL) as u8);
        let reply = if hates && !dominated {
            PlanetReply::WhatDoYouWant
        } else {
            PlanetReply::ChannelOpen
        };
        Self {
            stellar,
            variant,
            price,
            haggling: Haggling::default(),
            hates,
            takes_bribe: takes_bribe(drawn, facts.govt),
            demanded: false,
            reply,
        }
    }

    /// The stellar hailed.
    #[must_use]
    pub fn stellar(&self) -> StellarId {
        self.stellar
    }

    /// The variant its replies are said in.
    #[must_use]
    pub fn variant(&self) -> u8 {
        self.variant
    }

    /// What it said last.
    #[must_use]
    pub fn reply(&self) -> PlanetReply {
        self.reply
    }

    /// Whether it refuses the player landing.
    #[must_use]
    pub fn hates(&self) -> bool {
        self.hates
    }

    /// Whether it takes a bribe.
    #[must_use]
    pub fn takes_bribe(&self) -> bool {
        self.takes_bribe
    }

    /// Whether tribute was demanded in this dialog.
    #[must_use]
    pub fn demanded(&self) -> bool {
        self.demanded
    }

    /// The bribe's price, raised by each decline.
    #[must_use]
    pub fn price(&self) -> i64 {
        self.price
    }

    /// The price the haggle dialog asks, while it is open.
    #[must_use]
    pub fn asking(&self) -> Option<i64> {
        self.haggling.asking()
    }

    /// Presses `item` at a stellar standing as `standing` says, Demand
    /// Tribute reading `demand_facts` (its `demanded` the dialog's own),
    /// drawing on `chance` (see the module docs); none, and nothing
    /// changes, while the haggle dialog is open.
    pub fn press(
        &mut self,
        item: PlanetCommItem,
        standing: Standing,
        demand_facts: DemandFacts,
        chance: &mut dyn Chance,
    ) -> Option<Pressed> {
        if self.asking().is_some() {
            return None;
        }
        if item == PlanetCommItem::Close {
            return Some(Pressed::Closed);
        }
        self.reply = PlanetReply::NoResponse;
        Some(match item {
            PlanetCommItem::Close | PlanetCommItem::Second => self.second(standing, chance),
            PlanetCommItem::Third if standing.always_dominated => Pressed::Said,
            PlanetCommItem::Third if standing.dominated => {
                self.reply = PlanetReply::Released {
                    station: standing.station,
                };
                Pressed::Released {
                    record: record_after_offence(standing.record, standing.min_status),
                }
            }
            PlanetCommItem::Third => {
                self.hates = true;
                let record = record_after_offence(standing.record, standing.min_status);
                let demand = demand(DemandFacts {
                    demanded: self.demanded,
                    ..demand_facts
                });
                self.reply = match demand {
                    Demand::Laugh => PlanetReply::Laugh,
                    Demand::Fight { .. } => PlanetReply::Fight,
                    Demand::Dominate => PlanetReply::Dominated {
                        station: standing.station,
                    },
                };
                self.demanded = true;
                Pressed::Demanded { demand, record }
            }
        })
    }

    /// Item 2 pressed (see the module docs).
    fn second(&mut self, standing: Standing, chance: &mut dyn Chance) -> Pressed {
        self.reply = if !self.hates {
            PlanetReply::Greetings
        } else if standing.dominated {
            PlanetReply::WastingTime
        } else if !self.takes_bribe {
            PlanetReply::NoBribe
        } else {
            self.reply = PlanetReply::BribeAsked;
            self.haggling.ask(self.price, chance);
            return Pressed::Asked(self.price);
        };
        Pressed::Said
    }

    /// Makes `choice` in the bribe's haggle dialog with the player's
    /// `cash`, the system's bribe roll `roll` set to 0 by a decline (see
    /// the module docs): what it came to, none with no price asked.
    pub fn haggle(&mut self, choice: Haggle, cash: i64, roll: &mut Option<u8>) -> Option<Settled> {
        let settled = self.haggling.haggle(choice, cash)?;
        match settled {
            Settled::Lowered => {}
            Settled::Short => self.reply = PlanetReply::WastingTime,
            Settled::Paid(_) => self.takes_bribe = false,
            Settled::Declined => {
                *roll = Some(0);
                self.price += DECLINE_RAISE;
                self.takes_bribe = false;
                self.reply = PlanetReply::BribeDeclined;
            }
        }
        Some(settled)
    }

    /// What the dialog shows: the stellar `named`, of type label
    /// `type_label`, standing as `standing` says, its reply read from
    /// `catalog`.
    #[must_use]
    pub fn view(
        &self,
        named: &str,
        type_label: String,
        standing: Standing,
        catalog: &(impl CommCatalog + ?Sized),
    ) -> PlanetCommView {
        let third = if !standing.dominated {
            Some(PlanetCommButton::DemandTribute)
        } else if standing.always_dominated {
            None
        } else {
            Some(PlanetCommButton::Release)
        };
        let status = if standing.dominated {
            Some(if standing.always_dominated {
                PlanetStatus::Owned
            } else {
                PlanetStatus::Dominated
            })
        } else if self.hates {
            Some(if standing.record >= 0 {
                PlanetStatus::Forbidden
            } else {
                PlanetStatus::Hostile
            })
        } else {
            None
        };
        PlanetCommView {
            stellar: self.stellar,
            name: named.to_owned(),
            type_label,
            reply: self.reply.say(self.variant, named, catalog),
            second: if self.hates {
                PlanetCommButton::OfferBribe
            } else {
                PlanetCommButton::Greetings
            },
            third,
            status,
            asking: self.asking(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nova_sim_base::fixture::Draws;

    const EARTH: StellarId = StellarId(128);

    /// `STR#` 3000 is "c<n>" and 3002 "s<n>" for each n up to 60; 1100
    /// "[t<n>]" up to 10; `'STR '` 7003 patches type 3 to "Plug".
    struct Strings;

    impl CommCatalog for Strings {
        fn string_list(&self, id: i16) -> Vec<String> {
            match id {
                3000 => (1..=60).map(|n| format!("c{n}")).collect(),
                3002 => (1..=60).map(|n| format!("s{n}")).collect(),
                1100 => (1..=10).map(|n| format!("[t{n}]")).collect(),
                _ => Vec::new(),
            }
        }

        fn plugin_string(&self, id: i16) -> StringPatch {
            match id {
                7003 => StringPatch::Text("Plug".to_owned()),
                7004 => StringPatch::Empty,
                7005 => StringPatch::Unreadable,
                _ => StringPatch::Absent,
            }
        }
    }

    /// No strings at all.
    struct Nothing;

    impl CommCatalog for Nothing {
        fn string_list(&self, _id: i16) -> Vec<String> {
            Vec::new()
        }
    }

    #[test]
    fn a_record_short_of_min_status_refuses_landing() {
        assert!(
            refuses_landing(0x7fff, 32_767, false, false, false),
            "never"
        );
        assert!(
            !refuses_landing(-32_767, -32_768, false, false, false),
            "any"
        );
        assert!(refuses_landing(10, 9, false, false, false), "below");
        assert!(!refuses_landing(10, 10, false, false, false), "equal");
        assert!(!refuses_landing(10, 11, false, false, false), "above");
    }

    #[test]
    fn a_blessing_or_an_iff_mark_clears_a_government_stellars_refusal() {
        assert!(!refuses_landing(10, 0, true, true, false), "blessed");
        assert!(!refuses_landing(10, 0, true, false, true), "marked");
        assert!(refuses_landing(10, 0, true, false, false), "neither");
        assert!(
            refuses_landing(10, 0, false, true, true),
            "an independent is never cleared"
        );
        assert!(refuses_landing(0x7fff, 0, false, true, true));
        assert!(
            !refuses_landing(0x7fff, 0, true, true, false),
            "even 0x7fff"
        );
    }

    #[test]
    fn a_stellar_takes_a_bribe_above_a_roll_of_thirty_or_for_a_greedy_government() {
        let plain = Bribery::default();
        let planets = Bribery {
            planets_take_bribes: true,
            ..plain
        };
        let greedy = Bribery {
            greedy: true,
            ..plain
        };
        assert!(takes_bribe(31, None), "an independent");
        assert!(!takes_bribe(30, None));
        assert!(takes_bribe(31, Some(planets)));
        assert!(!takes_bribe(30, Some(planets)));
        assert!(!takes_bribe(99, Some(plain)), "no 0x4000");
        assert!(takes_bribe(0, Some(greedy)), "greedy, whatever the roll");
    }

    /// The price for `cash`, `greedy` or not, its draw `step`.
    fn price(cash: i64, greedy: bool, step: u32) -> (i64, Vec<u32>) {
        let mut chance = Draws::of(&[step]);
        (bribe_price(cash, greedy, &mut chance), chance.asked)
    }

    #[test]
    fn the_price_is_capped_at_a_third_of_the_cash_before_its_floor() {
        assert_eq!(price(0, false, 0), (1000, vec![1]), "no cash asks 1000");
        assert_eq!(price(10_000, false, 0).0, 3000, "a third of 10,000");
        assert_eq!(price(8000, false, 0).0, 2000, "2,664 rounds down");
        assert_eq!(price(2000, false, 0).0, 1000, "666 rounds to 0, then 1000");
    }

    #[test]
    fn the_price_draws_a_step_a_million_credits() {
        assert_eq!(price(5_000_000, false, 4), (7000, vec![5]));
        assert_eq!(price(1_999_999, false, 0).1, vec![1]);
    }

    #[test]
    fn a_greedy_governments_price_is_half_as_much_again_rounded_down() {
        assert_eq!(price(5_000_000, true, 0).0, 4000, "4,500");
        assert_eq!(price(5_000_000, true, 1).0, 6000, "6,000");
    }

    #[test]
    fn the_price_is_at_most_nine_hundred_thousand() {
        assert_eq!(price(100_000_000_000, false, 99_999).0, MAX_PRICE);
    }

    #[test]
    fn an_offence_drops_a_record_at_or_above_min_status_to_one_below_it() {
        assert_eq!(record_after_offence(10, 10), Some(9));
        assert_eq!(record_after_offence(500, 10), Some(9));
        assert_eq!(record_after_offence(9, 10), None, "below");
        assert_eq!(record_after_offence(0, -32_767), None, "any record");
        assert_eq!(record_after_offence(0, -32_766), Some(-32_767));
        assert_eq!(record_after_offence(32_767, 32_767), None, "never");
        assert_eq!(record_after_offence(32_767, 32_766), Some(32_765));
        assert_eq!(record_after_offence(32_765, 32_766), None);
    }

    fn facts(rating: i64) -> DemandFacts {
        DemandFacts {
            rating,
            ..DemandFacts::default()
        }
    }

    #[test]
    fn a_weak_pilot_is_laughed_off_unless_cheating() {
        assert_eq!(demand(facts(12_799)), Demand::Laugh);
        assert_eq!(demand(facts(-5)), Demand::Laugh);
        assert_eq!(demand(facts(12_800)), Demand::Dominate);
        let cheating = DemandFacts {
            cheats: true,
            ..facts(0)
        };
        assert_eq!(demand(cheating), Demand::Dominate);
    }

    #[test]
    fn a_defended_stellar_fights_sending_a_wave_only_when_not_under_attack() {
        let strong = facts(12_800);
        let left = DemandFacts {
            defenders_left: 1,
            ..strong
        };
        assert_eq!(demand(left), Demand::Fight { send_wave: true });
        let attacked = DemandFacts {
            under_attack: true,
            ..strong
        };
        assert_eq!(demand(attacked), Demand::Fight { send_wave: false });
        let again = DemandFacts {
            demanded: true,
            ..strong
        };
        assert_eq!(demand(again), Demand::Fight { send_wave: true });
        let negative = DemandFacts {
            defenders_left: -3,
            ..strong
        };
        assert_eq!(demand(negative), Demand::Dominate, "none left");
    }

    #[test]
    fn each_outcome_slaps_once_and_a_wave_or_a_domination_five_times_more() {
        assert_eq!(Demand::Laugh.slaps(), 1);
        assert_eq!(Demand::Fight { send_wave: false }.slaps(), 1);
        assert_eq!(Demand::Fight { send_wave: true }.slaps(), 6);
        assert_eq!(Demand::Dominate.slaps(), 6);
    }

    #[test]
    fn each_reply_reads_its_entry_by_the_variant() {
        let cases = [
            (PlanetReply::ChannelOpen, (3002, 1)),
            (PlanetReply::WhatDoYouWant, (3000, 11)),
            (PlanetReply::NoResponse, (3000, 6)),
            (PlanetReply::Greetings, (3000, 46)),
            (PlanetReply::WastingTime, (3002, 21)),
            (PlanetReply::NoBribe, (3002, 46)),
            (PlanetReply::BribeAsked, (3002, 41)),
            (PlanetReply::BribeDeclined, (3002, 31)),
            (PlanetReply::Laugh, (3002, 6)),
            (PlanetReply::Fight, (3002, 11)),
        ];
        for (reply, (list, first)) in cases {
            assert_eq!(reply.entry(0), (list, first), "{reply:?}");
            assert_eq!(reply.entry(4), (list, first + 4), "{reply:?}");
        }
        for variant in [0, 4] {
            let fixed = |station| {
                (
                    PlanetReply::Dominated { station }.entry(variant),
                    PlanetReply::Released { station }.entry(variant),
                )
            };
            assert_eq!(fixed(false), ((3002, 26), (3002, 36)));
            assert_eq!(fixed(true), ((3002, 27), (3002, 37)));
        }
    }

    #[test]
    fn the_greeting_names_the_stellar_and_ends_with_a_full_stop() {
        assert_eq!(
            PlanetReply::ChannelOpen.say(2, "Earth", &Strings),
            "s3Earth."
        );
        assert_eq!(PlanetReply::Greetings.say(2, "Earth", &Strings), "c48");
        assert_eq!(
            PlanetReply::Released { station: true }.say(2, "Earth", &Strings),
            "s37"
        );
        assert_eq!(
            PlanetReply::Greetings.say(0, "Earth", &Nothing),
            "",
            "missing"
        );
    }

    #[test]
    fn a_type_label_is_its_plugin_string_or_its_str_1100_entry() {
        assert_eq!(type_label(0, &Strings), "[t1]");
        assert_eq!(type_label(9, &Strings), "[t10]");
        assert_eq!(type_label(10, &Strings), "", "past the list");
        assert_eq!(type_label(3, &Strings), "Plug");
        assert_eq!(type_label(4, &Strings), "", "an empty patch");
        assert_eq!(type_label(5, &Strings), "", "an unreadable patch");
        assert_eq!(type_label(-1, &Strings), "", "no type");
    }

    /// An opening at `MinStatus` 0 with a record of `record` and 1,000,000
    /// in cash, independent.
    fn opening(record: i16) -> Opening {
        Opening {
            record,
            cash: 1_000_000,
            ..Opening::default()
        }
    }

    /// The dialog opened on Earth with `facts`, the system's `roll`, its
    /// variant `variant` and the price's step 0, the roll's draw `drawn`.
    fn opened(
        facts: &Opening,
        dominated: bool,
        roll: &mut Option<u8>,
        variant: u32,
        drawn: u32,
    ) -> (PlanetComm, Vec<u32>) {
        let mut chance = Draws::of(&[]).keyed(5, variant).keyed(100, drawn);
        let comm = PlanetComm::open(EARTH, dominated, facts, roll, &mut chance);
        (comm, chance.asked)
    }

    #[test]
    fn opening_draws_the_variant_the_price_and_the_systems_bribe_roll_once() {
        let mut roll = None;
        let (comm, asked) = opened(&opening(0), false, &mut roll, 3, 31);
        assert_eq!(comm.variant(), 3);
        assert_eq!(comm.price(), 3000);
        assert_eq!(roll, Some(31));
        assert_eq!(asked, [5, 1, 100]);
        assert!(comm.takes_bribe());
        let (again, asked) = opened(&opening(0), false, &mut roll, 0, 0);
        assert_eq!(asked, [5, 1], "the roll kept");
        assert!(again.takes_bribe());
        assert_eq!(again.stellar(), EARTH);
        assert!(!again.demanded());
        assert_eq!(again.asking(), None);
    }

    #[test]
    fn a_refusing_stellar_asks_what_the_player_wants_unless_dominated() {
        let refusing = Opening {
            min_status: 1,
            ..opening(0)
        };
        let (comm, _) = opened(&refusing, false, &mut Some(0), 0, 0);
        assert!(comm.hates());
        assert_eq!(comm.reply(), PlanetReply::WhatDoYouWant);
        let (dominated, _) = opened(&refusing, true, &mut Some(0), 0, 0);
        assert!(dominated.hates());
        assert_eq!(dominated.reply(), PlanetReply::ChannelOpen);
        let (open, _) = opened(&opening(0), false, &mut Some(0), 0, 0);
        assert!(!open.hates());
        assert_eq!(open.reply(), PlanetReply::ChannelOpen);
    }

    #[test]
    fn opening_reads_the_governments_bribery_and_its_greed() {
        let greedy = Opening {
            govt: Some(Bribery {
                greedy: true,
                planets_take_bribes: false,
            }),
            ..opening(0)
        };
        let (comm, _) = opened(&greedy, false, &mut Some(0), 0, 0);
        assert!(comm.takes_bribe());
        assert_eq!(comm.price(), 4000, "4,500 rounded down");
        let blessed = Opening {
            min_status: 5,
            blessed: true,
            govt: Some(Bribery::default()),
            ..opening(0)
        };
        assert!(!opened(&blessed, false, &mut Some(0), 0, 0).0.hates());
        let marked = Opening {
            blessed: false,
            iff_marked: true,
            ..blessed
        };
        assert!(!opened(&marked, false, &mut Some(0), 0, 0).0.hates());
    }

    fn standing(record: i16) -> Standing {
        Standing {
            record,
            ..Standing::default()
        }
    }

    /// Presses `item` on `comm` standing as `standing` says, demanding with
    /// `facts`, drawing the haggle roll `haggle_roll`.
    fn press(
        comm: &mut PlanetComm,
        item: PlanetCommItem,
        standing: Standing,
        facts: DemandFacts,
        haggle_roll: u32,
    ) -> Option<Pressed> {
        let mut chance = Draws::of(&[haggle_roll]);
        comm.press(item, standing, facts, &mut chance)
    }

    #[test]
    fn greetings_greets_a_stellar_that_does_not_refuse() {
        let (mut comm, _) = opened(&opening(0), false, &mut Some(0), 1, 0);
        let pressed = press(&mut comm, PlanetCommItem::Second, standing(0), facts(0), 0);
        assert_eq!(pressed, Some(Pressed::Said));
        assert_eq!(comm.reply(), PlanetReply::Greetings);
        assert_eq!(
            press(&mut comm, PlanetCommItem::Close, standing(0), facts(0), 0),
            Some(Pressed::Closed)
        );
    }

    /// A refusing stellar (`MinStatus` 1, record 0) opened with the
    /// system's `roll`.
    fn refusing(roll: u8) -> (PlanetComm, Option<u8>) {
        let facts = Opening {
            min_status: 1,
            ..opening(0)
        };
        let mut roll = Some(roll);
        let (comm, _) = opened(&facts, false, &mut roll, 0, 0);
        (comm, roll)
    }

    fn refused() -> Standing {
        Standing {
            min_status: 1,
            ..standing(0)
        }
    }

    #[test]
    fn offer_bribe_asks_the_price_of_a_stellar_that_takes_one() {
        let (mut comm, _) = refusing(31);
        let pressed = press(&mut comm, PlanetCommItem::Second, refused(), facts(0), 0);
        assert_eq!(pressed, Some(Pressed::Asked(3000)));
        assert_eq!(comm.reply(), PlanetReply::BribeAsked);
        assert_eq!(comm.asking(), Some(3000));
        assert_eq!(
            press(&mut comm, PlanetCommItem::Third, refused(), facts(0), 0),
            None,
            "no press while haggling"
        );
    }

    #[test]
    fn offer_bribe_is_scoffed_at_without_the_roll_and_wasted_on_a_dominated_stellar() {
        let (mut comm, _) = refusing(30);
        let pressed = press(&mut comm, PlanetCommItem::Second, refused(), facts(0), 0);
        assert_eq!(pressed, Some(Pressed::Said));
        assert_eq!(comm.reply(), PlanetReply::NoBribe);
        assert_eq!(comm.asking(), None);
        let (mut comm, _) = refusing(31);
        let dominated = Standing {
            dominated: true,
            ..refused()
        };
        press(&mut comm, PlanetCommItem::Second, dominated, facts(0), 0);
        assert_eq!(comm.reply(), PlanetReply::WastingTime);
        assert_eq!(comm.asking(), None);
    }

    #[test]
    fn a_won_haggle_lowers_the_bribe_and_accepting_pays_it() {
        let (mut comm, mut roll) = refusing(31);
        press(&mut comm, PlanetCommItem::Second, refused(), facts(0), 35);
        assert_eq!(
            comm.haggle(Haggle::LowerPrice, 1_000_000, &mut roll),
            Some(Settled::Lowered)
        );
        assert_eq!(comm.asking(), Some(2200));
        assert_eq!(
            comm.haggle(Haggle::Accept, 1_000_000, &mut roll),
            Some(Settled::Paid(2200))
        );
        assert!(!comm.takes_bribe());
        assert_eq!(roll, Some(31), "the roll kept");
        assert_eq!(comm.asking(), None);
    }

    #[test]
    fn a_lost_haggle_declines_zeroing_the_roll_and_raising_the_price() {
        let (mut comm, mut roll) = refusing(31);
        press(&mut comm, PlanetCommItem::Second, refused(), facts(0), 36);
        assert_eq!(
            comm.haggle(Haggle::LowerPrice, 1_000_000, &mut roll),
            Some(Settled::Declined)
        );
        assert_eq!(roll, Some(0));
        assert_eq!(comm.price(), 4000);
        assert!(!comm.takes_bribe());
        assert_eq!(comm.reply(), PlanetReply::BribeDeclined);
        press(&mut comm, PlanetCommItem::Second, refused(), facts(0), 0);
        assert_eq!(comm.reply(), PlanetReply::NoBribe, "declined once");
        assert_eq!(
            comm.haggle(Haggle::Accept, 0, &mut roll),
            None,
            "none asked"
        );
    }

    #[test]
    fn short_cash_wastes_the_stellars_time_whatever_the_choice() {
        for choice in [Haggle::Accept, Haggle::LowerPrice] {
            let (mut comm, mut roll) = refusing(31);
            press(&mut comm, PlanetCommItem::Second, refused(), facts(0), 99);
            assert_eq!(comm.haggle(choice, 2999, &mut roll), Some(Settled::Short));
            assert_eq!(comm.reply(), PlanetReply::WastingTime);
            assert_eq!((roll, comm.price()), (Some(31), 3000), "nothing else");
            assert!(comm.takes_bribe());
        }
    }

    #[test]
    fn demand_tribute_refuses_landing_writes_the_record_and_marks_the_dialog() {
        let (mut comm, _) = opened(&opening(10), false, &mut Some(0), 0, 0);
        let at = Standing {
            min_status: 5,
            ..standing(10)
        };
        let pressed = press(&mut comm, PlanetCommItem::Third, at, facts(0), 0);
        assert_eq!(
            pressed,
            Some(Pressed::Demanded {
                demand: Demand::Laugh,
                record: Some(4),
            })
        );
        assert!(comm.hates());
        assert!(comm.demanded());
        assert_eq!(comm.reply(), PlanetReply::Laugh);
        let strong = facts(20_000);
        let again = press(&mut comm, PlanetCommItem::Third, at, strong, 0);
        assert_eq!(
            again,
            Some(Pressed::Demanded {
                demand: Demand::Fight { send_wave: true },
                record: Some(4),
            }),
            "demanded already in this dialog"
        );
        assert_eq!(comm.reply(), PlanetReply::Fight);
    }

    #[test]
    fn domination_says_the_planet_or_the_station_pays_tribute() {
        for station in [false, true] {
            let (mut comm, _) = opened(&opening(0), false, &mut Some(0), 0, 0);
            let at = Standing {
                station,
                ..standing(0)
            };
            let pressed = press(&mut comm, PlanetCommItem::Third, at, facts(12_800), 0);
            assert_eq!(
                pressed,
                Some(Pressed::Demanded {
                    demand: Demand::Dominate,
                    record: Some(-1),
                })
            );
            assert_eq!(comm.reply(), PlanetReply::Dominated { station });
        }
    }

    #[test]
    fn release_releases_a_dominated_stellar_without_marking_the_dialog() {
        let (mut comm, _) = opened(&opening(0), true, &mut Some(0), 0, 0);
        let at = Standing {
            dominated: true,
            station: true,
            ..standing(-5)
        };
        let pressed = press(&mut comm, PlanetCommItem::Third, at, facts(0), 0);
        assert_eq!(pressed, Some(Pressed::Released { record: None }));
        assert_eq!(comm.reply(), PlanetReply::Released { station: true });
        assert!(!comm.demanded());
        assert!(!comm.hates(), "Release leaves the refusal as it was");
    }

    #[test]
    fn item_three_of_an_always_dominated_stellar_says_no_response_and_does_nothing() {
        let (mut comm, _) = opened(&opening(0), true, &mut Some(0), 0, 0);
        let owned = Standing {
            dominated: true,
            always_dominated: true,
            ..standing(0)
        };
        let pressed = press(&mut comm, PlanetCommItem::Third, owned, facts(0), 0);
        assert_eq!(pressed, Some(Pressed::Said));
        assert_eq!(comm.reply(), PlanetReply::NoResponse);
        assert!(!comm.demanded());
    }

    #[test]
    fn the_view_shows_the_buttons_by_the_refusal_and_the_domination() {
        let (comm, _) = opened(&opening(0), false, &mut Some(0), 2, 0);
        let view = comm.view("Earth", "[Earth]".to_owned(), standing(0), &Strings);
        assert_eq!(
            view,
            PlanetCommView {
                stellar: EARTH,
                name: "Earth".to_owned(),
                type_label: "[Earth]".to_owned(),
                reply: "s3Earth.".to_owned(),
                second: PlanetCommButton::Greetings,
                third: Some(PlanetCommButton::DemandTribute),
                status: None,
                asking: None,
            }
        );
        let dominated = Standing {
            dominated: true,
            ..standing(0)
        };
        let view = comm.view("Earth", String::new(), dominated, &Strings);
        assert_eq!(view.third, Some(PlanetCommButton::Release));
        assert_eq!(view.status, Some(PlanetStatus::Dominated));
        let owned = Standing {
            always_dominated: true,
            ..dominated
        };
        let view = comm.view("Earth", String::new(), owned, &Strings);
        assert_eq!(view.third, None);
        assert_eq!(view.status, Some(PlanetStatus::Owned));
    }

    #[test]
    fn a_refusing_stellars_status_is_forbidden_or_hostile_by_the_records_sign() {
        let (mut comm, _) = refusing(31);
        let view = comm.view("Earth", String::new(), refused(), &Strings);
        assert_eq!(view.second, PlanetCommButton::OfferBribe);
        assert_eq!(view.status, Some(PlanetStatus::Forbidden));
        assert_eq!(view.reply, "c11");
        let hostile = Standing {
            record: -1,
            ..refused()
        };
        let view = comm.view("Earth", String::new(), hostile, &Strings);
        assert_eq!(view.status, Some(PlanetStatus::Hostile));
        press(&mut comm, PlanetCommItem::Second, refused(), facts(0), 0);
        let view = comm.view("Earth", String::new(), refused(), &Strings);
        assert_eq!(view.asking, Some(3000));
    }

    #[test]
    fn the_buttons_and_statuses_say_their_strings() {
        assert_eq!(
            [
                PlanetCommButton::Greetings,
                PlanetCommButton::OfferBribe,
                PlanetCommButton::DemandTribute,
                PlanetCommButton::Release,
            ]
            .map(PlanetCommButton::label),
            ["Greetings", "Offer Bribe", "Demand Tribute", "Release"]
        );
        assert_eq!(
            [
                PlanetStatus::Owned,
                PlanetStatus::Dominated,
                PlanetStatus::Forbidden,
                PlanetStatus::Hostile,
            ]
            .map(PlanetStatus::text),
            ["Owned", "Dominated", "Forbidden", "Hostile"]
        );
    }
}
