//! A stellar's outfitter as the player sees it ([`Outfitter`], its rows
//! [`OutfitRow`] and what its rules read of each, [`OutfitFacts`]), an
//! order and what it did ([`OutfitOrder`], [`OutfitRun`], [`SaleAlert`],
//! [`Reselection`]), why an order is refused ([`OutfitRefusal`]), the
//! visit's once-an-opening flags ([`OutfitVisit`], [`Bought`]) and the
//! ship's guns and turrets ([`Armed`], [`Hardpoints`]), from
//! `nova_sim_port::outfitter`, which re-exports them and keeps the shop's
//! building, its buy and sell checks
//! (`nova_sim_port::outfitter::OutfitterRules`), the prices and the
//! settling.

use std::collections::BTreeMap;

use nova_sim_base::rulebook::{RuleSource, Rulebook};
use nova_sim_base::rules::Context;
use nova_sim_catalog::catalog::{OutfitId, OutfitRecord};

use super::order_loop::OrderRun;
use crate::market::Direction;

/// The `oütf` `Flags` bits the outfitter reads (the Bible).
#[derive(Clone, Copy, Debug)]
pub struct OutfitFlags;

impl OutfitFlags {
    /// A fixed gun: counts against the ship's `MaxGun`.
    pub const GUN: u16 = 0x0001;
    /// A turret: counts against the ship's `MaxTur`.
    pub const TURRET: u16 = 0x0002;
    /// Stays with the player when they trade ships (persistent): see
    /// `nova_sim_port::shipyard`.
    pub const PERSISTENT: u16 = 0x0004;
    /// Can't be sold.
    pub const CANNOT_SELL: u16 = 0x0008;
    /// Removed after purchase: buying it only pays.
    pub const REMOVE_AFTER_PURCHASE: u16 = 0x0010;
    /// Stays with the player when a mission's set operator changes its
    /// ship (`H`): see `nova_sim_port::ship_change`.
    pub const MISSION_PERSISTENT: u16 = 0x0020;
    /// Not shown unless the player meets its `Require`, or owns one.
    pub const HIDE_UNLESS_REQUIRED: u16 = 0x0100;
    /// Its price is its `Cost` times the ship's `Mass`.
    pub const PRICE_BY_MASS: u16 = 0x0200;
    /// Its mass is the ship's `Mass` times its `Mass` over 100.
    pub const MASS_BY_MASS: u16 = 0x0400;
    /// Can be sold anywhere.
    pub const SELL_ANYWHERE: u16 = 0x0800;
    /// For sale, it takes higher-numbered outfits of equal `DispWeight`
    /// off sale.
    pub const HIDE_HIGHER: u16 = 0x1000;
    /// Listed among the ranks on the player info's Honors tab, not among
    /// its Extras: see `nova_sim_port::player_info`.
    pub const HONOR: u16 = 0x2000;
    /// Not shown unless its `Availability` holds, or the player owns one.
    pub const HIDE_UNLESS_AVAILABLE: u16 = 0x4000;
}

/// The `oütf` `ModType` that changes the ship's `MaxGun` by its `ModVal`.
pub const MORE_GUNS: i16 = 45;

/// The `oütf` `ModType` that changes the ship's `MaxTur` by its `ModVal`.
pub const MORE_TURRETS: i16 = 46;

/// The `oütf` `ModType` that explores a map: the outfitter sells one only
/// once an opening ([`Bought`]).
pub const EXPLORES_MAP: i16 = 16;

/// The `oütf` `ModType` that clears the player's legal record: the
/// outfitter sells one only once an opening ([`Bought`]).
pub const CLEAN_RECORD: i16 = 21;

/// The `oütf` `ModType` that paints the ship: an outfit with one never
/// sets the clean-record flag (`Bought::after_buying`).
pub const PAINT: i16 = 43;

/// One outfit the player asks to buy or sell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OutfitOrder {
    /// The outfit.
    pub outfit: OutfitId,
    /// Buying or selling.
    pub direction: Direction,
}

/// Why an outfit cannot be bought or sold. A buy gives the first refusal
/// in `_CanBuyOutfitItem`'s order (see the module docs). The original
/// names none: it only disables its Buy button (0xded08, @0x5c1c4-0x5c1cc).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutfitRefusal {
    /// There is no outfitter: the ship is not landed, or the stellar has
    /// none.
    NoOutfitter,
    /// The outfitter has no row for the outfit: it is neither listed nor
    /// the relisting's [`Outfitter::unlisted`] row, which is checked like
    /// a listed one (see
    /// [`RuleKey::RelistedSelection`](crate::RuleKey::RelistedSelection)).
    NotListed,
    /// The outfit is listed but cannot be bought here:
    /// `_CanBuyOutfitItem`'s own tech test refuses it, or its `Require` is
    /// not met, or its `Availability` does not hold.
    NotForSale,
    /// The player owns its `Max` already.
    MaxOwned,
    /// Its `Max` is none.
    NoneAllowed,
    /// It is a fighter, and the ship's bays have no room for another
    /// (`_CanBuyFighter` @0x5a82). The original reaches that check only from
    /// `_CanBuyOutfitItem` (@0x4e938), not from `_HasMaxOfItem`, so its info
    /// box gives this none of the #219/#220 words the other `Max` refusals get;
    /// it still shows the mass words #221/#222 when the fighter also lacks the
    /// mass (`nova_sim_port::outfitter::OutfitterRules::words`).
    BaysFull,
    /// The ship has not the free mass for another.
    NoSpace,
    /// The ship has not the free mass for one.
    NoSpaceForAny,
    /// It is a mass expansion (the last of its mods of
    /// [`MORE_CARGO`](crate::market::MORE_CARGO) has a negative `ModVal`), and
    /// the ship's `Holds` is negative, which forbids that (`_CanBuyOutfitItem`
    /// @0x4e94f-0x4e97e).
    NoExpansion,
    /// It is a mass expansion, and the ship's own free cargo (its cargo
    /// space less every ton held) is below the size of its `ModVal`
    /// (`_CanBuyOutfitItem` @0x4e980-0x4e9a8).
    NoCargoRoom,
    /// It explores a map ([`EXPLORES_MAP`]) and a map was the last outfit
    /// bought since the outfitter opened (`_boughtMap`), or it clears the
    /// legal record ([`CLEAN_RECORD`]) and a clean record was
    /// (`_boughtID`; `_CanBuyOutfitItem` @0x4e9aa-0x4e9c5). See
    /// [`Bought`].
    BoughtThisOpening,
    /// The player cannot pay for it.
    CannotAfford,
    /// The player owns none.
    NoneOwned,
    /// It can't be sold.
    CannotSell,
    /// It is not for sale here, and cannot be sold anywhere.
    NotBoughtHere,
    /// The ship lacks the free mass to sell it: by the engine, it is of
    /// negative mass and the free mass clamped at 0 plus its mass is
    /// below 0; by the other reading, selling it would leave the free
    /// mass below 0 (see [`RuleKey::SaleMass`](crate::RuleKey::SaleMass)).
    NegativeFreeMass,
    /// The ship's guns are at its `MaxGun`, as `ModType` 45 changes it.
    GunLimit,
    /// The ship's turrets are at its `MaxTur`, as `ModType` 46 changes it.
    TurretLimit,
    /// It is a launcher, and `rounds` of its ammunition must be sold
    /// first.
    AmmunitionFirst {
        /// The rounds to sell first.
        rounds: u32,
        /// The outfit that names the ammunition, if any.
        ammo: Option<OutfitId>,
    },
    /// It raises the `Max` of `target` (`ModType` 27), and `count` of that
    /// must be sold first.
    RaisedFirst {
        /// How many of `target` to sell first.
        count: u32,
        /// The outfit whose `Max` it raises.
        target: OutfitId,
    },
}

/// The refusal text a sale refused inside the sell loop shows (`DLOG`
/// 3003, `_DoOutfitDialog` @0x5c735-0x5ce6c; see
/// `nova_sim_port::outfitter::OutfitterRules::sale_alert`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SaleAlert {
    /// The outfit sold.
    pub outfit: OutfitId,
    /// Why: [`OutfitRefusal::NegativeFreeMass`],
    /// [`OutfitRefusal::RaisedFirst`] or [`OutfitRefusal::AmmunitionFirst`].
    pub refusal: OutfitRefusal,
    /// The names the words need, as
    /// `nova_sim_port::outfitter::OutfitterRules::lc_names` gives them: the
    /// sold outfit's, and the ammunition's or the target's.
    pub names: BTreeMap<OutfitId, LcNames>,
    /// Whether the sold outfit is named in `LCPlural`: passes remain after
    /// the refused one (pass < count - 1, @0x5c9ab-0x5c9dd, @0x5cd53).
    pub plural: bool,
}

/// What an outfit order did: the order loop's run, the alert its refused sale
/// shows, if any (`nova_sim_port::outfitter::OutfitterRules::sale_alert`), and
/// what it leaves the outfitter's selection at.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutfitRun {
    /// The passes that went through, and the first refusal.
    pub run: OrderRun<OutfitRefusal>,
    /// The alert the first refused pass shows, if any.
    pub alert: Option<SaleAlert>,
    /// What the order leaves the outfitter's selection at (see
    /// [`RuleKey::RelistedSelection`](crate::RuleKey::RelistedSelection)).
    pub after: Reselection,
    /// Whether the order's Buy or Sell button passed its own check
    /// (`nova_sim_port::outfitter::OutfitterRules::can`), so `_DoOutfitDialog`
    /// falls through its loop to the buy's reset and draw (@0x5c2fc-0x5c2ff,
    /// @0x5c305, @0x5c5a1) or the sale's draw (@0x5d328), however many passes
    /// went through.
    pub button_passed: bool,
}

/// What an order leaves the outfitter's selection at, as
/// [`RuleKey::RelistedSelection`](crate::RuleKey::RelistedSelection)
/// reads it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Reselection {
    /// The selection is kept: the order left the listing as it was, or
    /// the rule follows its other reading.
    #[default]
    Kept,
    /// The listing changed (`_SetupPortAvailableItems` true,
    /// @0xc216-0xc23d, after the buy loop @0x5c314 or a sale @0x5d00a),
    /// so this outfit, index 0 (`*0xded58` = 0, @0x5c336, @0x5d01c), is
    /// selected, listed or not, with no cell highlighted and the grid at
    /// its top (`*0xded5c` = -1, `*0xdeda8` = 0, @0x5c32c-0x5c33b,
    /// @0x5d021-0x5d02b).
    Reset(OutfitId),
    /// The listing changed and nothing is listed: the leave flag -0x63d
    /// is set (@0x5c40c, @0x5d128), and the outfitter closes once the
    /// click is handled (@0x5da33).
    Leave,
}

/// An outfit's lower-case names, as the outfitter's words use them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LcNames {
    /// Its `LCName`.
    pub singular: String,
    /// Its `LCPlural`.
    pub plural: String,
}

/// One outfit in a stellar's outfitter.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutfitRow {
    /// The outfit.
    pub id: OutfitId,
    /// Its name.
    pub name: String,
    /// Its `ShortName`, raw.
    pub short_name: String,
    /// Its price here, for one: what its resale, the trade-in and a
    /// refund at half read.
    pub price: i64,
    /// What buying one costs: [`price`](Self::price), scaled by the ranks'
    /// factor by the other reading of
    /// [`RuleKey::RankPriceScope`](crate::RuleKey::RankPriceScope)
    /// (`nova_sim_story::rank::scaled`).
    pub buy_price: i64,
    /// Its mass, for one, in tons.
    pub mass: i64,
    /// How many the player owns.
    pub owned: u16,
    /// Its `Max`.
    pub max: i16,
    /// What the outfitter's rules read about the outfit, settled when the
    /// outfitter was built.
    pub facts: OutfitFacts,
}

/// What the outfitter's rules read about one outfit, settled when the
/// outfitter is built: whatever needs the gate, the day's rolls or every
/// record is worked out then, so the rules answer from these alone.
// Each flag is its own fact, settled at build and read by its own check.
#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct OutfitFacts {
    /// Its own `LCName` and `LCPlural`.
    pub names: LcNames,
    /// Its `Flags` ([`OutfitFlags`]): the rules read
    /// [`GUN`](OutfitFlags::GUN), [`TURRET`](OutfitFlags::TURRET) and
    /// [`CANNOT_SELL`](OutfitFlags::CANNOT_SELL).
    pub flags: u16,
    /// Whether it is in the list: false only for
    /// [`Outfitter::unlisted`].
    pub listed: bool,
    /// Whether `_CanBuyOutfitItem`'s own tech test passes it
    /// (@0x4e7ff-0x4e83d), as [`RuleKey::TechLevel`](crate::RuleKey::TechLevel)
    /// reads it.
    pub buy_tech: bool,
    /// Whether its `Require` is met where its `RequireGovt` applies it
    /// (`_CanBuyOutfitItem` @0x4e9cb-0x4eb37).
    pub required: bool,
    /// Whether its `Availability` holds (`_CanBuyOutfitItem`
    /// @0x4eb51-0x4eb58).
    pub available: bool,
    /// Whether it can be sold here at all: for sale here today, by tech or as
    /// an owned outfit at a stellar of `Flags2`
    /// `nova_sim_port::outfitter::BUYS_OWNED_OUTFITS`, or flagged
    /// [`OutfitFlags::SELL_ANYWHERE`].
    pub sold_here: bool,
    /// Its ammunition cap, when it has one (`_HasMaxOfItem`
    /// @0x457e-0x45e4; see the module docs and
    /// `nova_sim_port::has_max::ammo_cap`), a wide product.
    pub ammo_cap: Option<i64>,
    /// Its `Max` as the `nova_sim_port::outfitter::RAISES_MAX` outfits owned
    /// raise it, when the multiplier is above 1; `None` when it is its `Max`.
    pub raised: Option<i64>,
    /// When it is a fighter's ammunition, the fighters the ship's bays can
    /// still take.
    pub fighter_room: Option<u32>,
    /// When it is a mass expansion, the cargo space it takes
    /// (`mass_expansion`).
    pub expansion: Option<u16>,
    /// Which once-an-opening flag limits it, if any (`opening_limit`).
    pub opening_limit: Option<OpeningLimit>,
    /// When the player owns it as a launcher, that launcher and the
    /// ammunition it holds.
    pub launcher: Option<Launcher>,
    /// The names of the outfit that names its launcher's ammunition, when
    /// that has a record.
    pub ammo_names: Option<LcNames>,
    /// One for each of its `nova_sim_port::outfitter::RAISES_MAX` mods, in slot
    /// order.
    pub raises: Vec<Raise>,
    /// Its `Mass`, raw: the info box draws its Item Mass and Available
    /// lines only above none (`_OutfitDialogUpdate` @0x57c72).
    pub mass_field: i16,
}

/// One `nova_sim_port::outfitter::RAISES_MAX` mod of an outfit: the outfit
/// whose `Max` it raises, and what a sale of the raiser reads of it
/// (`_DoOutfitDialog` @0x5c7be-0x5ca6f; see the module docs).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Raise {
    /// The outfit its raw `ModVal` names.
    pub target: OutfitId,
    /// How many of the target the player owns.
    pub held: u16,
    /// The target's `Max`; `None` when it has no record.
    pub target_max: Option<i16>,
    /// The target's raised `Max` by the Bible once one of the raiser is
    /// sold; none when it has no record.
    pub left_by_the_bible: i64,
    /// The target's names, when it has a record.
    pub target_names: Option<LcNames>,
}

/// A stellar's outfitter, as the player sees it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Outfitter {
    /// The outfits listed, highest `DispWeight` first, then by ID.
    pub rows: Vec<OutfitRow>,
    /// The row of the outfit asked for when it is not listed: in no cell, and
    /// found only through [`row`](Self::row). It is the outfit a relisting
    /// selects, `oütf` 128 (`nova_sim_port::outfitter::FIRST_OUTFIT`), which
    /// Buy and Sell answer for listed or not (see
    /// [`RuleKey::RelistedSelection`](crate::RuleKey::RelistedSelection)).
    pub unlisted: Option<OutfitRow>,
    /// The player's credits.
    pub cash: i64,
    /// The ship's free mass, in tons; negative when it carries more than
    /// it has space for.
    pub free_mass: i64,
    /// The ship's guns and turrets, and its limits for each.
    pub armed: Armed,
    /// The ship class's `Holds`: below none forbids a mass expansion.
    pub holds: i16,
    /// The ship's own free cargo: its cargo space less every ton held
    /// (`_ShipTotalHolds() - _TotalCargo(player)`, @0x4e980-0x4e9a8).
    pub free_cargo: i64,
    /// The visit it was built in: the once-an-opening flags as they
    /// stand, and the counts owned on entry ([`OutfitVisit`]).
    pub visit: OutfitVisit,
    /// The rule set the outfitter was built by, which its questions read.
    pub rules: Rulebook,
    /// Buy is not drawn: the stellar has no tech
    /// (`nova_sim_port::wares::has_no_tech`), the flag `_DoOutfitDialog` sets
    /// at `*0xded64` (@0x5bb67-0x5bb9d) and `_DrawOutfitButtons` reads as Buy's
    /// visibility (@0x29ecf-0x29f11). Buy still takes a press and its key
    /// (`_TrackOutfitButtons` @0x2a1ec-0x2a202, `_OutfitFilter`
    /// @0x5817a-0x58190).
    pub buy_hidden: bool,
}

impl Outfitter {
    /// `outfit`'s row, if it is listed or is the [`unlisted`](Self::unlisted)
    /// row asked for.
    #[must_use]
    pub fn row(&self, outfit: OutfitId) -> Option<&OutfitRow> {
        self.rows
            .iter()
            .chain(self.unlisted.as_ref())
            .find(|row| row.id == outfit)
    }

    /// The ship's free mass as `_ShipFreeMass` reads it, clamped at none
    /// (@0xb506-0xb50a): what the buy's free mass check, the info box's mass
    /// words, its "Available:" line
    /// (`nova_sim_port::outfitter::OutfitterRules::free_mass_shown`) and the
    /// engine's reading of a sale read.
    #[must_use]
    pub fn free_mass_clamped(&self) -> i64 {
        self.free_mass.max(0)
    }
}

impl Context for Outfitter {
    fn rulebook(&self) -> &Rulebook {
        &self.rules
    }
}

/// Which of the once-an-opening flags limits an outfit
/// (`nova_sim_port::outfitter::opening_limit`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpeningLimit {
    /// The map flag, `_boughtMap`.
    Map,
    /// The clean-record flag, `_boughtID`.
    Record,
}

/// The once-an-opening flags: whether the last outfit bought since the
/// outfitter opened explored a map (`_boughtMap`) or cleared the legal
/// record (`_boughtID`). The original clears both each time the
/// outfitter dialog opens (`_DoOutfitDialog` @0x5bb37-0x5bb44) and sets
/// them on every grant (`Bought::after_buying`); see the module docs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Bought {
    /// `_boughtMap`. `pub` for `nova_sim_port::outfitter`'s tests, which
    /// set the flags as they stand.
    pub map: bool,
    /// `_boughtID`. `pub` for `nova_sim_port::outfitter`'s tests, which
    /// set the flags as they stand.
    pub record: bool,
}

impl Bought {
    /// The flags after `record` is bought, whatever they were
    /// (`_GrantOutfitItem` @0x44d4f): both cleared (@0x44d67-0x44d7d); then the
    /// map flag set when any slot has an [`EXPLORES_MAP`] mod, whatever its
    /// `ModVal` (@0x44ea9-0x44eaf); otherwise the record flag set when any slot
    /// has a [`CLEAN_RECORD`] mod and none a [`PAINT`] mod (@0x44ecc-0x44f8b).
    /// Unlike `nova_sim_port::outfitter::opening_limit`, an ammunition or cargo
    /// mod plays no part.
    #[must_use]
    pub fn after_buying(record: &OutfitRecord) -> Self {
        let has = |kind: i16| record.mods.iter().any(|&(mod_type, _)| mod_type == kind);
        let map = has(EXPLORES_MAP);
        Self {
            map,
            record: !map && has(CLEAN_RECORD) && !has(PAINT),
        }
    }

    /// Whether these flags refuse a buy of an outfit that `limit` limits
    /// (`nova_sim_port::outfitter::opening_limit`). `pub` for
    /// `nova_sim_port::outfitter`'s buy checks, which read it.
    #[must_use]
    pub fn refuses(self, limit: Option<OpeningLimit>) -> bool {
        match limit {
            Some(OpeningLimit::Map) => self.map,
            Some(OpeningLimit::Record) => self.record,
            None => false,
        }
    }
}

/// One visit to the outfitter: what lasts from the outfitter opening to its
/// next opening. The once-an-opening flags ([`Bought`]), which limit what is
/// sold as [`RuleKey::OutfitLimit`](crate::RuleKey::OutfitLimit) reads, and how
/// many of each outfit the player owned on entry, which a sale refunds against
/// (`nova_sim_port::outfitter::refund`). The session opens one each time the
/// outfitter dialog opens (`_DoOutfitDialog` @0x5bb37-0x5bb44), on a landing
/// and as it starts. None of it is saved: the original keeps the flags in
/// globals `_DoOutfitDialog` clears on opening, and the counts in its locals.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct OutfitVisit {
    /// The once-an-opening flags (`_boughtMap`, `_boughtID`).
    pub bought: Bought,
    /// How many of each outfit the player owned when the outfitter
    /// opened, as `_DoOutfitDialog` copies them on entry
    /// (@0x5badd-0x5bafa). `pub` for `nova_sim_port::outfitter`'s tests,
    /// which build a visit as a static.
    pub opened_with: BTreeMap<OutfitId, u16>,
}

impl OutfitVisit {
    /// A visit opened over `owned`: the flags clear and the counts
    /// copied.
    #[must_use]
    pub fn open(owned: &BTreeMap<OutfitId, u16>) -> Self {
        Self {
            bought: Bought::default(),
            opened_with: owned.clone(),
        }
    }

    /// `record` is bought: the flags are set afresh
    /// ([`Bought::after_buying`]); the counts on entry stay.
    pub fn bought(&mut self, record: &OutfitRecord) {
        self.bought = Bought::after_buying(record);
    }

    /// How many of `outfit` the player owned when the visit opened; none
    /// when it owned none. `pub` for `nova_sim_port::outfitter`'s refund,
    /// which reads it.
    #[must_use]
    pub fn opened_with(&self, outfit: OutfitId) -> u16 {
        self.opened_with.get(&outfit).copied().unwrap_or(0)
    }
}

/// A ship class's room for guns and turrets: its `MaxGun` and `MaxTur`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Hardpoints {
    /// Its `MaxGun`.
    pub guns: i16,
    /// Its `MaxTur`.
    pub turrets: i16,
}

/// A launcher outfit the player owns, and the ammunition it holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Launcher {
    /// Its weapon's `MaxAmmo`; none for no limit of its own.
    pub max_ammo: u32,
    /// The rounds of its ammunition held, the fighters out counted.
    pub rounds: u32,
    /// The outfit that names its ammunition, if any.
    pub ammo: Option<OutfitId>,
}

impl Launcher {
    /// The rounds that must be sold before one of `owned` launchers can
    /// be, as `source` reads the rule
    /// ([`RuleKey::LauncherSale`](crate::RuleKey::LauncherSale)); `None`
    /// when it can be sold. By the engine, the rounds the remaining
    /// launchers' `MaxAmmo` cannot hold, only when `MaxAmmo` is above 0
    /// (`_DoOutfitDialog` @0x5cad0, @0x5cb9c-0x5cbe0); by the other
    /// reading, every round held.
    #[cfg(test)]
    pub(crate) fn excess(&self, owned: u16, source: RuleSource) -> Option<u32> {
        self.excess_left(i64::from(owned.saturating_sub(1)), source)
    }

    /// The rounds that must be sold before a launcher can be, `left`
    /// launchers' `MaxAmmo` kept, as `source` reads the rule
    /// ([`RuleKey::LauncherSale`](crate::RuleKey::LauncherSale)); `None`
    /// when it can be sold. By the engine, rounds - `MaxAmmo` x `left`,
    /// signed with no floor, so `left` below none names more rounds than
    /// are held (@0x5cbc7-0x5cbcf), only when `MaxAmmo` is above 0
    /// (@0x5cad0) and some rounds are held; by the other reading, every
    /// round held. `pub` for `nova_sim_port::outfitter`'s sale checks,
    /// which read it.
    #[must_use]
    pub fn excess_left(&self, left: i64, source: RuleSource) -> Option<u32> {
        if self.rounds == 0 {
            return None;
        }
        match source {
            RuleSource::Engine => (self.max_ammo > 0).then(|| {
                let kept = i64::from(self.max_ammo).saturating_mul(left);
                let excess = i64::from(self.rounds).saturating_sub(kept);
                u32::try_from(excess.max(0)).unwrap_or(u32::MAX)
            }),
            RuleSource::Bible => Some(self.rounds),
        }
        .filter(|&excess| excess > 0)
    }
}

/// `base` changed by the `ModVal` of the first mod of `mod_type` of each
/// outfit owned, once per outfit whatever the count
/// (`_HasMaxOfItem` @0x472b-0x4796).
fn raised(
    base: i16,
    mod_type: i16,
    owned: &BTreeMap<OutfitId, u16>,
    records: &[OutfitRecord],
) -> i32 {
    let mods: i32 = records
        .iter()
        .filter(|record| owned.get(&record.id).is_some_and(|&count| count > 0))
        .filter_map(|record| record.mods.iter().find(|&&(kind, _)| kind == mod_type))
        .map(|&(_, mod_val)| i32::from(mod_val))
        .sum();
    i32::from(base) + mods
}

/// How many of the outfits `owned` are flagged `flag`.
fn flagged(flag: u16, owned: &BTreeMap<OutfitId, u16>, records: &[OutfitRecord]) -> i32 {
    records
        .iter()
        .filter(|record| record.flags & flag != 0)
        .filter_map(|record| owned.get(&record.id))
        .map(|&count| i32::from(count))
        .sum()
}

/// The guns and turrets a ship carries, and its limits for each
/// (`_HasMaxOfItem` @0x46c8-0x4866; see the module docs).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Armed {
    /// The outfits owned flagged [`OutfitFlags::GUN`].
    pub guns: i32,
    /// The outfits owned flagged [`OutfitFlags::TURRET`].
    pub turrets: i32,
    /// `MaxGun`, as [`MORE_GUNS`] changes it.
    pub gun_limit: i32,
    /// `MaxTur`, as [`MORE_TURRETS`] changes it.
    pub turret_limit: i32,
}

impl Armed {
    /// The guns and turrets of a ship of `hardpoints` owning `owned`.
    /// `pub` for `nova_sim_port::outfitter`'s shop and
    /// `nova_sim_port::has_max`, which build it.
    #[must_use]
    pub fn of(
        hardpoints: Hardpoints,
        owned: &BTreeMap<OutfitId, u16>,
        records: &[OutfitRecord],
    ) -> Self {
        Self {
            guns: flagged(OutfitFlags::GUN, owned, records),
            turrets: flagged(OutfitFlags::TURRET, owned, records),
            gun_limit: raised(hardpoints.guns, MORE_GUNS, owned, records),
            turret_limit: raised(hardpoints.turrets, MORE_TURRETS, owned, records),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::market::MORE_CARGO;
    use crate::stats::MORE_FUEL;
    use nova_sim_catalog::fixture::outfit;

    /// The flags with only the map one set, or only the record one.
    const MAP_BOUGHT: Bought = Bought {
        map: true,
        record: false,
    };
    const RECORD_BOUGHT: Bought = Bought {
        map: false,
        record: true,
    };

    #[test]
    fn the_mission_persistent_flag_is_0x0020() {
        assert_eq!(OutfitFlags::MISSION_PERSISTENT, 0x0020);
    }

    #[test]
    fn a_launchers_excess_is_read_as_its_rule_says() {
        let excess = |max_ammo, rounds, owned, source| {
            Launcher {
                max_ammo,
                rounds,
                ammo: None,
            }
            .excess(owned, source)
        };
        for (max_ammo, rounds, owned, engine, bible) in [
            (4, 2, 1, Some(2), Some(2)),
            (4, 0, 1, None, None),
            (4, 4, 2, None, Some(4)),
            (4, 5, 2, Some(1), Some(5)),
            (0, 1, 1, None, Some(1)),
            (0, 0, 1, None, None),
            (4, 9, 3, Some(1), Some(9)),
            (4, 8, 3, None, Some(8)),
            (4, 3, 0, Some(3), Some(3)),
            (u32::MAX, u32::MAX, u16::MAX, None, Some(u32::MAX)),
            (4, u32::MAX, 1, Some(u32::MAX), Some(u32::MAX)),
            (u32::MAX, 1, 2, None, Some(1)),
        ] {
            let case = format!("{max_ammo} {rounds} {owned}");
            assert_eq!(
                excess(max_ammo, rounds, owned, RuleSource::Engine),
                engine,
                "{case}"
            );
            assert_eq!(
                excess(max_ammo, rounds, owned, RuleSource::Bible),
                bible,
                "{case}"
            );
        }
    }

    #[test]
    fn the_clamped_free_mass_is_never_below_none() {
        let clamped = |free_mass| {
            Outfitter {
                free_mass,
                ..Outfitter::default()
            }
            .free_mass_clamped()
        };
        assert_eq!(clamped(-15), 0);
        assert_eq!(clamped(0), 0);
        assert_eq!(clamped(7), 7);
    }

    /// The flags as `_GrantOutfitItem` leaves them after buying an outfit
    /// with `mods`.
    fn after(mods: &[(i16, i16)]) -> Bought {
        Bought::after_buying(&outfit(128, mods))
    }

    #[test]
    fn buying_a_map_sets_the_map_flag_and_clears_the_record_flag() {
        // `_GrantOutfitItem` clears both (@0x44d67-0x44d7d), then sets
        // `_boughtMap` for a `ModType` 16 whatever its `ModVal`
        // (@0x44ea9-0x44eaf).
        assert_eq!(after(&[(EXPLORES_MAP, 5)]), MAP_BOUGHT);
        assert_eq!(after(&[(MORE_FUEL, 1), (EXPLORES_MAP, 0)]), MAP_BOUGHT);
        assert_eq!(after(&[(EXPLORES_MAP, -2)]), MAP_BOUGHT);
        assert_eq!(after(&[(CLEAN_RECORD, 128), (EXPLORES_MAP, 5)]), MAP_BOUGHT);
        // A cargo mod does not stop the grant setting it.
        assert_eq!(after(&[(MORE_CARGO, -10), (EXPLORES_MAP, 5)]), MAP_BOUGHT);
    }

    #[test]
    fn buying_a_clean_record_without_paint_sets_the_record_flag() {
        // @0x44ecc-0x44f8b: a `ModType` 21, with no map and no paint mod.
        assert_eq!(after(&[(CLEAN_RECORD, 128)]), RECORD_BOUGHT);
        assert_eq!(after(&[(MORE_FUEL, 1), (CLEAN_RECORD, 128)]), RECORD_BOUGHT);
        assert_eq!(after(&[(CLEAN_RECORD, 128), (PAINT, 3)]), Bought::default());
        assert_eq!(after(&[(PAINT, 3), (CLEAN_RECORD, 128)]), Bought::default());
    }

    #[test]
    fn buying_any_other_outfit_clears_both_flags() {
        assert_eq!(after(&[(MORE_FUEL, 5)]), Bought::default());
        assert_eq!(after(&[]), Bought::default());
        assert_eq!(after(&[(PAINT, 3)]), Bought::default());
    }

    #[test]
    fn opening_a_visit_clears_the_flags_and_copies_the_counts() {
        let owned = BTreeMap::from([(OutfitId(128), 2), (OutfitId(130), 1)]);
        let visit = OutfitVisit::open(&owned);
        assert_eq!(visit.bought, Bought::default());
        assert_eq!(visit.opened_with, owned);
        assert_eq!(visit.opened_with(OutfitId(128)), 2);
        assert_eq!(visit.opened_with(OutfitId(130)), 1);
        assert_eq!(visit.opened_with(OutfitId(129)), 0, "not owned on entry");
    }

    #[test]
    fn buying_sets_the_visits_flags_afresh_and_keeps_its_counts() {
        let owned = BTreeMap::from([(OutfitId(128), 2)]);
        let mut visit = OutfitVisit::open(&owned);
        visit.bought(&outfit(129, &[(EXPLORES_MAP, 5)]));
        assert_eq!(visit.bought, MAP_BOUGHT);
        visit.bought(&outfit(130, &[(CLEAN_RECORD, 128)]));
        assert_eq!(visit.bought, RECORD_BOUGHT);
        visit.bought(&outfit(128, &[(MORE_FUEL, 5)]));
        assert_eq!(visit.bought, Bought::default());
        assert_eq!(visit.opened_with, owned, "the counts on entry kept");
    }
}
