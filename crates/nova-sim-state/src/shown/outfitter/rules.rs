//! The outfitter's rules ([`Outfitter`]'s questions): whether an order
//! goes through ([`Outfitter::can`], [`Outfitter::check_at`]), the info
//! box's words and lines, the reported cap, the refund, the counts asked
//! and the sale alert, as `_CanBuyOutfitItem`, `_HasMaxOfItem`,
//! `_OutfitDialogUpdate` and `_DoOutfitDialog` decide them, from
//! `nova_sim_port::outfitter`, whose module docs give each rule with its
//! addresses. They read only what the port settled on the outfitter's
//! rows when it built it ([`OutfitFacts`](super::OutfitFacts)), so they
//! sit here with the outfitter, below the layers.

use std::collections::BTreeMap;

use nova_sim_base::chance::NeverFires;
use nova_sim_base::rulebook::{RuleKey, RuleSource, Rulebook};
use nova_sim_base::rules::{Check, Checks, Context, Value};
use nova_sim_catalog::catalog::{OutfitId, OutfitRecord};

use super::{
    LcNames, OutfitFlags, OutfitOrder, OutfitRefusal, OutfitRow, Outfitter, Raise, SaleAlert,
};
use crate::market::Direction;
use crate::shown::has_max::{AtMax, ItemFacts, has_max_of_item};
use crate::shown::order_loop::Pass;
use crate::shown::wares::ShownTons;

/// The `oütf` `ModType` that multiplies the `Max` of the outfit its
/// `ModVal` names.
pub const RAISES_MAX: i16 = 27;

/// What an outfit owned when the outfitter opened sells back for, as a
/// percentage of its price: the engine's 0.5 (@0xdd128). One bought since
/// sells back in full by the engine ([`refund`]). The Bible does not say;
/// the community guide (evnova.miraheze.org, "Nova:Making Money") sells
/// the Scarab's Matter/Antimatter Reactor, whose stock `Cost` is
/// 5,000,000, "for 2.5 million credits".
pub const RESALE_PERCENT: i64 = 50;

/// [`Outfitter`]'s questions: whether an order goes through, the info
/// box's words and lines, the cap, the refund and the counts asked. They
/// run the outfitter's rules over a row, from the facts the port settled
/// when it built the outfitter.
impl Outfitter {
    /// The info box's "Available:" line: the free mass clamped at none,
    /// with "ton" at exactly 1 and "tons" otherwise.
    #[must_use]
    pub fn free_mass_shown(&self) -> ShownTons {
        free_mass_shown().of(self)
    }

    /// The info box's "Item Mass:" line for `outfit`: none for an outfit
    /// of `Mass` 0 or less, which shows no Item Mass or Available line;
    /// otherwise its mass for one, unclamped, with "ton" at exactly 1 and
    /// "tons" otherwise. None for an outfit with no row (neither listed
    /// nor [`unlisted`](Outfitter::unlisted)).
    #[must_use]
    pub fn item_mass_shown(&self, outfit: OutfitId) -> Option<ShownTons> {
        item_mass_shown().of(&asked_at(self, outfit, Pass::SINGLE)?)
    }

    /// Whether `order`'s button is enabled, or why not: an outfit that
    /// is neither listed nor the relisting's [`unlisted`](Outfitter::unlisted)
    /// row is [`OutfitRefusal::NotListed`], and the `unlisted` row is
    /// checked like a listed one (see
    /// [`RuleKey::RelistedSelection`]);
    /// a buy is refused by the
    /// first of `_CanBuyOutfitItem`'s checks that refuses it, in the
    /// port's order (see the module docs); a sale by the Sell flag's
    /// checks (some owned, sellable, sold here) and then, as
    /// [`RuleKey::SaleRefusal`](crate::RuleKey::SaleRefusal) says, by the
    /// engine nothing more, so a sale only the sell loop refuses leaves
    /// Sell enabled, or by the other reading the sell loop's checks too
    /// (the free mass, a raised `Max`, a launcher's ammunition).
    pub fn can(&self, order: OutfitOrder) -> Result<(), OutfitRefusal> {
        let asked = asked_at(self, order.outfit, Pass::SINGLE).ok_or(OutfitRefusal::NotListed)?;
        match order.direction {
            Direction::Buy => refusal(&can_buy(), &asked),
            Direction::Sell => {
                refusal(&sell_flag(), &asked)?;
                if sale_checks_on_the_button().of(&asked) {
                    refusal(&sale_checks(), &asked)
                } else {
                    Ok(())
                }
            }
        }
    }

    /// Whether `order` goes through when clicked, or why not, whatever
    /// the reading: [`check_at`](Self::check_at) at a single pass
    /// ([`Pass::SINGLE`]).
    pub fn check(&self, order: OutfitOrder) -> Result<(), OutfitRefusal> {
        self.check_at(order, Pass::SINGLE)
    }

    /// Whether `order` goes through at `pass` of the order loop, or why
    /// not, whatever the [`RuleKey::SaleRefusal`](crate::RuleKey::SaleRefusal)
    /// reading: an outfit with no row (neither listed nor
    /// [`unlisted`](Outfitter::unlisted)) is [`OutfitRefusal::NotListed`], and
    /// the `unlisted` row is checked like a listed one; a buy as [`can`](Self::can) says, whatever the pass; a
    /// sale by the Sell flag's checks, then the sell loop's (the free
    /// mass, a raised `Max`, a launcher's ammunition), as each pass of
    /// `_DoOutfitDialog`'s sell loop checks it (@0x5c735-0x5ce6c), a
    /// raised `Max` and a launcher's ammunition reading the pass as
    /// [`RuleKey::CountedSale`](crate::RuleKey::CountedSale) says.
    pub fn check_at(&self, order: OutfitOrder, pass: Pass) -> Result<(), OutfitRefusal> {
        let asked = asked_at(self, order.outfit, pass).ok_or(OutfitRefusal::NotListed)?;
        match order.direction {
            Direction::Buy => refusal(&can_buy(), &asked),
            Direction::Sell => {
                refusal(&sell_flag(), &asked)?;
                refusal(&sale_checks(), &asked)
            }
        }
    }

    /// Which sale refusal the info box words for `outfit` (`STR#` 2002
    /// #207-#212), as [`RuleKey::SaleRefusal`](crate::RuleKey::SaleRefusal)
    /// says: by the engine none, as `_OutfitDialogUpdate`'s only words are
    /// #213-#222 (@0x57a12-0x57e68); by the other reading the first of
    /// the sell loop's checks that refuses it, when the Sell flag is set.
    /// `None` too for an outfit with no row (neither listed nor
    /// [`unlisted`](Outfitter::unlisted)).
    #[must_use]
    pub fn sale_words(&self, outfit: OutfitId) -> Option<OutfitRefusal> {
        asked_at(self, outfit, Pass::SINGLE).and_then(|asked| sale_words().of(&asked))
    }

    /// The alert a sale refused inside the sell loop shows at `pass`, as
    /// [`RuleKey::SaleRefusal`](crate::RuleKey::SaleRefusal) says: by the
    /// engine, once an order at its first refused pass, the sell loop's
    /// refusal of a sale whose Sell flag is set, with the names its words need and
    /// the sold outfit named in the plural while passes remain
    /// (`_DoOutfitDialog` @0x5c735-0x5ce6c, @0x5c9ab-0x5c9dd, shown once
    /// an order by the flag -0x63e @0x5c6ca, in `DLOG` 3003 by
    /// `_ShowTextDialog` @0x5a8cf); by the other reading none. `None` for
    /// a buy, an outfit with no row (neither listed nor
    /// [`unlisted`](Outfitter::unlisted)), a Sell-flag refusal, a sale that goes
    /// through, and a pass after the order's first refused one
    /// ([`Pass::alerted`]).
    #[must_use]
    pub fn sale_alert(&self, order: OutfitOrder, pass: Pass) -> Option<SaleAlert> {
        let asked = asked_at(self, order.outfit, pass)?;
        sale_alert_value().of(&SaleAsk {
            asked,
            direction: order.direction,
        })
    }

    /// Which of the info box's words (`STR#` 2002 #219-#222) `outfit`
    /// gets, whatever [`can`](Self::can) says of a buy: the refusal
    /// `_HasMaxOfItem` makes, if any (a `Max`, none allowed, the
    /// ammunition cap, a gun or turret limit); otherwise
    /// [`OutfitRefusal::NoSpaceForAny`] with none owned or
    /// [`OutfitRefusal::NoSpace`] with some, when it lacks the mass;
    /// otherwise none (`_OutfitDialogUpdate` @0x57dba-0x57e6f, which never
    /// consults `_CanBuyOutfitItem`; see the module docs). `None` too when
    /// it has no row (neither listed nor [`unlisted`](Outfitter::unlisted)).
    #[must_use]
    pub fn words(&self, outfit: OutfitId) -> Option<OutfitRefusal> {
        asked_at(self, outfit, Pass::SINGLE).and_then(|asked| info_words().of(&asked))
    }

    /// The `Max` that `_HasMaxOfItem(item, 0, &cap)` reports out for
    /// `outfit` (@0x4512), which the quantity dialog's buy maximum reads:
    /// its `Max`, as the ammunition cap, a raised `Max` and the gun and
    /// turret limits change it (see [`count_max`](Self::count_max));
    /// `None` when it has no row (neither listed nor
    /// [`unlisted`](Outfitter::unlisted)).
    #[must_use]
    pub fn cap(&self, outfit: OutfitId) -> Option<i16> {
        asked_at(self, outfit, Pass::SINGLE).map(|asked| reported_cap().of(&asked))
    }

    /// What selling one of `outfit` refunds, as
    /// [`RuleKey::OutfitRefund`](crate::RuleKey::OutfitRefund) says: by
    /// the engine, the full price for one bought since the visit opened
    /// (its count owned above the [visit's](super::OutfitVisit) count on entry),
    /// otherwise half; by the other reading, always half (see
    /// [`refund`]). `None` when it has no row (neither listed nor
    /// [`unlisted`](Outfitter::unlisted)).
    #[must_use]
    pub fn refund(&self, outfit: OutfitId) -> Option<i64> {
        asked_at(self, outfit, Pass::SINGLE).map(|asked| refund_value().of(&asked))
    }

    /// The `LCName` and `LCPlural` of each outfit the words of
    /// [`check`](Self::check)'s refusal of `order` name (the click's,
    /// whatever the reading), worked out from that
    /// refusal: a launcher refused for its ammunition, and the outfit
    /// naming that ammunition when it has a record; a raiser refused for
    /// its target, and that target when it has a record. None for any
    /// other answer.
    #[must_use]
    pub fn lc_names(&self, order: OutfitOrder) -> BTreeMap<OutfitId, LcNames> {
        let Some(row) = self.row(order.outfit) else {
            return BTreeMap::new();
        };
        names_for(row, self.check(order))
    }

    /// The count the quantity dialog opens at when `outfit` is bought or
    /// sold with Option held, as `_DoOutfitDialog` works it out; `None`
    /// when it opens none and the click moves one, as it does at a count
    /// of 1 or less (see the module docs' "Counted orders").
    ///
    /// A buy opens only while the Buy flag is set
    /// ([`can`](Self::can) allows it). Its count is the cash over the
    /// price, truncated, at most [`COUNT_CAP`] (that cap when the price
    /// is nothing or less, @0x5c0ca-0x5c11f); at most `Max` less those
    /// owned when `Max` is above those owned (@0x5c124-0x5c156); and at
    /// most the [reported cap](Self::cap) and that cap less those
    /// owned (@0x5c170-0x5c19d). Those owned read as 16 bits, signed.
    /// The engine also skips the dialog when the count is 1 or less
    /// before the cap is read (@0x5c159), which the cap could only lower.
    ///
    /// A sale opens while the Sell flag is set: some owned and not
    /// [`OutfitFlags::CANNOT_SELL`] (@0x5c563-0x5c595, and each time the
    /// list is redrawn, `_OutfitFilter` @0x5808c-0x580b1). Free mass, a
    /// raised `Max` and a launcher's ammunition refuse a sale only inside
    /// the sell loop, so a sale they refuse still opens the dialog. Its
    /// count is those owned, read as 16 bits, signed, at most
    /// [`COUNT_CAP`] (@0x5c5ed-0x5c617).
    #[must_use]
    pub fn count_max(&self, outfit: OutfitId, direction: Direction) -> Option<u32> {
        let row = self.row(outfit)?;
        let owned = i64::from(row.owned.cast_signed());
        let most = match direction {
            Direction::Buy => {
                self.can(OutfitOrder { outfit, direction }).ok()?;
                let mut most = if row.buy_price > 0 {
                    (self.cash / row.buy_price).min(COUNT_CAP)
                } else {
                    COUNT_CAP
                };
                let max = i64::from(row.max);
                if max > owned {
                    most = most.min(max - owned);
                }
                let cap = i64::from(self.cap(outfit)?);
                most.min(cap).min(cap - owned)
            }
            Direction::Sell => {
                let asked = asked_at(self, outfit, Pass::SINGLE)?;
                refusal(&sell_flag(), &asked).ok()?;
                owned.min(COUNT_CAP)
            }
        };
        u32::try_from(most).ok().filter(|&most| most > 1)
    }

    /// The count Option (Alt here) asks for when `outfit` is bought or
    /// sold: by the engine's reading of
    /// [`RuleKey::OutfitCount`](crate::RuleKey::OutfitCount), the quantity
    /// dialog opens at [`count_max`](Self::count_max), and with `None` the
    /// click moves one; by the other reading it never opens.
    #[must_use]
    pub fn count_asked(&self, outfit: OutfitId, direction: Direction) -> Option<u32> {
        if option_asks_count().of(self) {
            self.count_max(outfit, direction)
        } else {
            None
        }
    }
}

/// `outfit`'s row as the outfitter's rules ask about it at `pass` of
/// an order, if it has one: listed, or the
/// [`unlisted`](Outfitter::unlisted) row. The button, the info box and the cap
/// are not inside the order loop, so they ask at [`Pass::SINGLE`].
fn asked_at(outfitter: &Outfitter, outfit: OutfitId, pass: Pass) -> Option<RowOutfit<'_>> {
    outfitter.row(outfit).map(|row| RowOutfit {
        outfitter,
        row,
        pass,
    })
}

/// The `LCName` and `LCPlural` of each outfit the words of `refused`,
/// `row`'s refusal, name (see [`Outfitter::lc_names`]): a launcher
/// refused for its ammunition and the outfit naming that ammunition, or
/// a raiser refused for its target and that target, each when it has a
/// record. None for any other answer.
fn names_for(row: &OutfitRow, refused: Result<(), OutfitRefusal>) -> BTreeMap<OutfitId, LcNames> {
    let facts = &row.facts;
    let named = match refused {
        Err(OutfitRefusal::AmmunitionFirst { ammo, .. }) => ammo.zip(facts.ammo_names.clone()),
        Err(OutfitRefusal::RaisedFirst { target, .. }) => facts
            .raises
            .iter()
            .find(|raise| raise.target == target)
            .and_then(|raise| raise.target_names.clone())
            .map(|names| (target, names)),
        _ => return BTreeMap::new(),
    };
    [(row.id, facts.names.clone())]
        .into_iter()
        .chain(named)
        .collect()
}

/// The free mass the info box's "Available:" line shows
/// ([`Outfitter::free_mass_shown`]).
fn free_mass_shown() -> Value<Outfitter, ShownTons> {
    Value::plain(
        "free mass shown",
        "`_OutfitDialogUpdate` @0x57d66-0x57d9d: `_ShipFreeMass` read as `free_mass_clamped` \
         (`cmovs` @0x57d74), `_tonString` (`STR#` 2002 #1) at exactly 1 @0x57d8b-0x57d95, else \
         `_tonsString` (#2)",
        clamped_tons,
    )
}

/// The mass the info box's "Item Mass:" line shows
/// ([`Outfitter::item_mass_shown`]).
fn item_mass_shown<'a>() -> Value<RowOutfit<'a>, Option<ShownTons>> {
    Value::plain(
        "item mass shown",
        "`_OutfitDialogUpdate` @0x57c67-0x57d19: no Item Mass or Available line for an outfit of \
         `Mass` 0 or less (`jle` @0x57c78 to @0x57da2); otherwise `_AdjustedItemMass` unclamped \
         (@0x57cec), `_tonString` (`STR#` 2002 #1) at exactly 1 @0x57d07-0x57d19, else \
         `_tonsString` (#2)",
        row_tons,
    )
}

/// The row's mass with its unit, when its outfit's `Mass` is above none.
fn row_tons(asked: &RowOutfit) -> Option<ShownTons> {
    (asked.row.facts.mass_field > 0).then(|| ShownTons::of(asked.row.mass))
}

/// The clamped free mass, with its unit.
fn clamped_tons(outfitter: &Outfitter) -> ShownTons {
    ShownTons::of(outfitter.free_mass_clamped())
}

/// Whether Option (Alt here) on Buy or Sell asks for a count, as
/// [`RuleKey::OutfitCount`](crate::RuleKey::OutfitCount) says: by the
/// engine it opens the quantity dialog at [`Outfitter::count_max`]; by
/// the other reading each click moves one.
fn option_asks_count() -> Value<Outfitter, bool> {
    Value::disputed(
        "option asks count",
        "`_DoOutfitDialog` @0x5c0c0-0x5c1aa, @0x5c5e3-0x5c628",
        RuleKey::OutfitCount,
        asks_a_count,
        moves_one,
    )
}

/// The engine's Option: a count.
fn asks_a_count(_: &Outfitter) -> bool {
    true
}

/// The other reading's Option: one a click.
fn moves_one(_: &Outfitter) -> bool {
    false
}

/// The most the quantity dialog offers, bought or sold (`0x7d00`,
/// `_DoOutfitDialog` @0x5c116, @0x5c606).
pub const COUNT_CAP: i64 = 32_000;

/// One listed outfit, asked whether it can be bought or sold, its words
/// and its cap: the facts the outfitter's checks and value rules read,
/// all settled when the outfitter was built.
#[derive(Clone, Copy)]
struct RowOutfit<'a> {
    /// The outfitter: its cash, free mass, guns, holds, free cargo, the
    /// once-an-opening flags and the rule set.
    outfitter: &'a Outfitter,
    /// The row: its price, mass, count owned, `Max` and facts.
    row: &'a OutfitRow,
    /// The pass of the order it is asked at: [`Pass::SINGLE`] outside
    /// the order loop.
    pass: Pass,
}

impl Context for RowOutfit<'_> {
    fn rulebook(&self) -> &Rulebook {
        self.outfitter.rulebook()
    }
}

/// One listed outfit asked about one pass of an order (its row's
/// [`pass`](RowOutfit::pass)): what the sale alert reads
/// ([`Outfitter::sale_alert`]).
#[derive(Clone, Copy)]
struct SaleAsk<'a> {
    /// The outfit, as its row is asked.
    asked: RowOutfit<'a>,
    /// Buying or selling.
    direction: Direction,
}

impl Context for SaleAsk<'_> {
    fn rulebook(&self) -> &Rulebook {
        self.asked.rulebook()
    }
}

/// The first of `checks` that refuses `facts`, if any.
fn refusal<C: Context>(checks: &Checks<C, OutfitRefusal>, facts: &C) -> Result<(), OutfitRefusal> {
    // No outfitter check draws (`no_outfitter_check_draws`), so no chance
    // is ever asked.
    checks.first_refusal(facts, &mut NeverFires)
}

/// `Ok` when `holds`, else `refusal`.
fn holds(holds: bool, refusal: OutfitRefusal) -> Result<(), OutfitRefusal> {
    if holds { Ok(()) } else { Err(refusal) }
}

/// Whether Buy is enabled for an outfit, as `_CanBuyOutfitItem` has it
/// (@0x4e7c4-0x4ebbc), asking no list: its own tech test, [`has_max_checks`], the free mass, the bays' room, the two
/// mass-expansion tests, the once-an-opening flags, `Require`,
/// `Availability` and the cash.
fn can_buy<'a>() -> Checks<RowOutfit<'a>, OutfitRefusal> {
    let [range, ammunition, raised, guns, turrets] = has_max_checks();
    [
        Check::plain(
            "tech",
            "`_CanBuyOutfitItem` @0x4e7ff-0x4e83d, as `buy tech` reads it at build \
             (`TechLevel` -1 skips it @0x4e802)",
            tech_buys,
        ),
        range,
        ammunition,
        raised,
        guns,
        turrets,
        Check::plain(
            "free mass",
            "`_CanBuyOutfitItem` @0x4e88a-0x4e897, reads `free_mass_clamped` \
             (`_ShipFreeMass` @0x4e7e1)",
            mass_fits,
        ),
        Check::plain(
            "bay room",
            "`_CanBuyOutfitItem` @0x4e8f6-0x4e93f, `_CanBuyFighter` @0x5a82",
            bay_room,
        ),
        Check::plain(
            "expansion allowed",
            "`_CanBuyOutfitItem` @0x4e94f-0x4e97e",
            expansion_allowed,
        ),
        Check::plain(
            "cargo room",
            "`_CanBuyOutfitItem` @0x4e980-0x4e9a8",
            cargo_room,
        ),
        Check::disputed(
            "once an opening",
            "`_CanBuyOutfitItem` @0x4e9aa-0x4e9c5",
            RuleKey::OutfitLimit,
            not_bought_this_opening,
            never_limited,
        ),
        Check::plain(
            "required",
            "`_CanBuyOutfitItem` @0x4e9cb-0x4eb37",
            requirement_holds,
        ),
        Check::plain(
            "available",
            "`_CanBuyOutfitItem` @0x4eb51-0x4eb58",
            availability_holds,
        ),
        Check::plain(
            "cash",
            "`_CanBuyOutfitItem` @0x4eb95-0x4ebad",
            cash_for_the_price,
        ),
    ]
    .into()
}

/// `_HasMaxOfItem`'s checks (@0x4512), as a list of their own: the
/// refusal the info box's words read first ([`info_words`]).
fn has_max<'a>() -> Checks<RowOutfit<'a>, OutfitRefusal> {
    has_max_checks().into()
}

/// `_HasMaxOfItem(item, 0, 0)`'s checks, in its order: the range, the
/// ammunition cap, the raised `Max`, then the gun and turret limits (see
/// the module docs). Each reads the one decision
/// ([`has_max_of_item`]) and refuses only on its own limit, so the first
/// that refuses is the limit the original stops at.
fn has_max_checks<'a>() -> [Check<RowOutfit<'a>, OutfitRefusal>; 5] {
    [
        Check::plain("outfit range", "`_HasMaxOfItem` @0x451b-0x4547", in_range),
        Check::plain(
            "ammunition cap",
            "`_HasMaxOfItem` @0x457e-0x45e4, via `_CanBuyOutfitItem` @0x4e87d",
            under_ammo_cap,
        ),
        Check::plain(
            "raised max",
            "`_HasMaxOfItem` @0x45e9-0x46c3, the multiplier settled at build",
            under_raised_max,
        ),
        Check::plain(
            "gun limit",
            "`_HasMaxOfItem` @0x46c8-0x47b7",
            under_gun_limit,
        ),
        Check::plain(
            "turret limit",
            "`_HasMaxOfItem` @0x47c7-0x4866",
            under_turret_limit,
        ),
    ]
}

/// Whether the Sell flag is set for an outfit, as `_DoOutfitDialog`
/// (@0x5c563-0x5c595) and `_OutfitFilter` (@0x5808c-0x580b1) set it: some
/// owned and not [`OutfitFlags::CANNOT_SELL`]; and, the port's own, the
/// outfit sold here.
fn sell_flag<'a>() -> Checks<RowOutfit<'a>, OutfitRefusal> {
    [
        Check::plain(
            "owned",
            "`_DoOutfitDialog` @0x5c563-0x5c570; `_OutfitFilter` @0x5808c-0x58096",
            owned,
        ),
        Check::plain(
            "can be sold",
            "`_DoOutfitDialog` @0x5c572-0x5c595; `_OutfitFilter` @0x58098-0x580b1",
            sellable,
        ),
        Check::plain(
            "sold here",
            "port-only: the row as listed (`_SetupPortAvailableItems` @0xc0a8-0xc0ba \
             lists an owned `SELL_ANYWHERE` outfit). The original's Sell flag asks no list",
            sold_here,
        ),
    ]
    .into()
}

/// The checks of `_DoOutfitDialog`'s sell loop, made on each pass and
/// not before the quantity dialog opens, in its order: the free mass,
/// a raised `Max`, then a launcher's ammunition (see the module docs).
fn sale_checks<'a>() -> Checks<RowOutfit<'a>, OutfitRefusal> {
    [
        Check::disputed(
            "free mass",
            "`_DoOutfitDialog` @0x5c735-0x5c7ac. The engine reads `free_mass_clamped` \
             (`_ShipFreeMass` @0x5c740), and the other reading reads `free_mass`",
            RuleKey::SaleMass,
            clamped_mass_left,
            mass_left,
        ),
        Check::disputed(
            "raised first",
            "`_DoOutfitDialog` @0x5c7be-0x5ca6f. The engine's left is \
             `target_max x raisers_left`, and the other's is `left_by_the_bible`",
            RuleKey::RaisedMax,
            engine_raised_first,
            bible_raised_first,
        ),
        Check::disputed(
            "ammunition first",
            "`_DoOutfitDialog` @0x5ca75-0x5cbe0, through \
             `Launcher::excess_left(launchers_left, Engine / Bible)`",
            RuleKey::LauncherSale,
            engine_ammunition_first,
            bible_ammunition_first,
        ),
    ]
    .into()
}

/// Whether the sell loop's checks ([`sale_checks`]) also decide Sell's
/// enable, as [`RuleKey::SaleRefusal`] says: by the engine they run only
/// inside the sell loop, so the Sell flag alone enables Sell; by the
/// other reading they grey it too. Each check keeps its own descriptor
/// and key: this value only says whether the list runs.
fn sale_checks_on_the_button<'a>() -> Value<RowOutfit<'a>, bool> {
    Value::disputed(
        "sale checks on the button",
        "`_DoOutfitDialog` @0x5c563-0x5c595 and `_OutfitFilter` @0x5808c-0x580b1 set \
         the Sell flag by the count owned and `CANNOT_SELL` alone; the sale checks run \
         only in the sell loop, @0x5c735-0x5ce6c",
        RuleKey::SaleRefusal,
        loop_only,
        before_the_loop,
    )
}

/// The engine's Sell enable: the sale checks run in the loop only.
fn loop_only(_: &RowOutfit) -> bool {
    false
}

/// The other reading's Sell enable: the sale checks grey Sell too.
fn before_the_loop(_: &RowOutfit) -> bool {
    true
}

/// The info box's sale words for an outfit ([`Outfitter::sale_words`]).
fn sale_words<'a>() -> Value<RowOutfit<'a>, Option<OutfitRefusal>> {
    Value::disputed(
        "sale words",
        "`_OutfitDialogUpdate` @0x57a12-0x57e68: only #213-#222",
        RuleKey::SaleRefusal,
        no_sale_words,
        first_sale_refusal,
    )
}

/// The engine's info box: no sale words.
fn no_sale_words(_: &RowOutfit) -> Option<OutfitRefusal> {
    None
}

/// The other reading's info box: the first sale check's refusal, when
/// the Sell flag is set.
fn first_sale_refusal(asked: &RowOutfit) -> Option<OutfitRefusal> {
    refusal(&sell_flag(), asked).ok()?;
    refusal(&sale_checks(), asked).err()
}

/// The alert a refused sale shows ([`Outfitter::sale_alert`]).
fn sale_alert_value<'a>() -> Value<SaleAsk<'a>, Option<SaleAlert>> {
    Value::disputed(
        "sale alert",
        "`_DoOutfitDialog` @0x5c735-0x5ce6c, -0x63e @0x5c6ca; `_ShowTextDialog` @0x5a8cf",
        RuleKey::SaleRefusal,
        alert_at_first_refusal,
        never_alerts,
    )
}

/// The engine's alert: at the order's first refused pass, whatever its
/// index, the sell loop's refusal of a sale whose Sell flag is set. A
/// pass after it ([`Pass::alerted`], flag -0x63e) is refused silently.
fn alert_at_first_refusal(ask: &SaleAsk) -> Option<SaleAlert> {
    let asked = &ask.asked;
    if ask.direction != Direction::Sell || asked.pass.alerted {
        return None;
    }
    refusal(&sell_flag(), asked).ok()?;
    let refused = refusal(&sale_checks(), asked).err()?;
    Some(SaleAlert {
        outfit: asked.row.id,
        refusal: refused,
        names: names_for(asked.row, Err(refused)),
        plural: passes_remain(asked.pass),
    })
}

/// The other reading's alert: none.
fn never_alerts(_: &SaleAsk) -> Option<SaleAlert> {
    None
}

/// Whether passes remain after `pass`: pass < count - 1
/// (@0x5c9ab-0x5c9dd), none past the count.
fn passes_remain(pass: Pass) -> bool {
    pass.count.saturating_sub(pass.index) > 1
}

/// `_CanBuyOutfitItem`'s own tech test passes the outfit; refused as
/// [`OutfitRefusal::NotForSale`].
fn tech_buys(asked: &RowOutfit) -> Result<(), OutfitRefusal> {
    holds(asked.row.facts.buy_tech, OutfitRefusal::NotForSale)
}

/// The outfit's `Require` is met where it applies; refused as
/// [`OutfitRefusal::NotForSale`].
fn requirement_holds(asked: &RowOutfit) -> Result<(), OutfitRefusal> {
    holds(asked.row.facts.required, OutfitRefusal::NotForSale)
}

/// The outfit's `Availability` holds; refused as
/// [`OutfitRefusal::NotForSale`].
fn availability_holds(asked: &RowOutfit) -> Result<(), OutfitRefusal> {
    holds(asked.row.facts.available, OutfitRefusal::NotForSale)
}

/// What [`has_max_of_item`] reads of the row: its count owned, its `Max`
/// as raised, its ammunition cap and flags, and the ship's guns and
/// turrets, all settled when the outfitter was built.
fn item_facts(asked: &RowOutfit) -> ItemFacts {
    let row = asked.row;
    ItemFacts {
        id: row.id,
        owned: i64::from(row.owned),
        max: row.facts.raised.unwrap_or(i64::from(row.max)),
        ammo_cap: row.facts.ammo_cap,
        flags: row.facts.flags,
        armed: asked.outfitter.armed,
    }
}

/// The refusal each of `_HasMaxOfItem`'s limits gives the row: none
/// allowed out of range or for a `Max` of none or less, `Max` owned at
/// the ammunition cap or the `Max`, and the gun or turret limit. The
/// match is exhaustive on purpose: between them the outfitter's checks
/// act on every limit [`AtMax`] names.
fn at_max_refusal(at: AtMax, row: &OutfitRow) -> OutfitRefusal {
    match at {
        AtMax::OutOfRange => OutfitRefusal::NoneAllowed,
        AtMax::Max(_) if row.max <= 0 => OutfitRefusal::NoneAllowed,
        AtMax::AmmoCap(_) | AtMax::Max(_) => OutfitRefusal::MaxOwned,
        AtMax::Guns(_) => OutfitRefusal::GunLimit,
        AtMax::Turrets(_) => OutfitRefusal::TurretLimit,
    }
}

/// Refused ([`at_max_refusal`]) when `_HasMaxOfItem` stops at the limit
/// `limit` picks out; any other limit is another check's to refuse.
fn refused_at(asked: &RowOutfit, limit: fn(AtMax) -> bool) -> Result<(), OutfitRefusal> {
    match has_max_of_item(&item_facts(asked)) {
        Some(at) if limit(at) => Err(at_max_refusal(at, asked.row)),
        _ => Ok(()),
    }
}

/// The outfit's ID is in 128-639 (@0x451b-0x4547); refused as none
/// allowed.
fn in_range(asked: &RowOutfit) -> Result<(), OutfitRefusal> {
    refused_at(asked, |at| at == AtMax::OutOfRange)
}

/// The count owned is below the ammunition cap, if it has one
/// (`_HasMaxOfItem` @0x45cd-0x45d1); refused as `Max` owned.
fn under_ammo_cap(asked: &RowOutfit) -> Result<(), OutfitRefusal> {
    refused_at(asked, |at| matches!(at, AtMax::AmmoCap(_)))
}

/// The count owned is below the raised `Max` (its `Max` when nothing
/// raises it); refused as none allowed when the `Max` is none or less.
fn under_raised_max(asked: &RowOutfit) -> Result<(), OutfitRefusal> {
    refused_at(asked, |at| matches!(at, AtMax::Max(_)))
}

/// A gun fits the ship's gun limit.
fn under_gun_limit(asked: &RowOutfit) -> Result<(), OutfitRefusal> {
    refused_at(asked, |at| matches!(at, AtMax::Guns(_)))
}

/// A turret fits the ship's turret limit.
fn under_turret_limit(asked: &RowOutfit) -> Result<(), OutfitRefusal> {
    refused_at(asked, |at| matches!(at, AtMax::Turrets(_)))
}

/// The outfit fits the free mass ([`lacks_the_mass`]).
fn mass_fits(asked: &RowOutfit) -> Result<(), OutfitRefusal> {
    lacks_the_mass(asked).map_or(Ok(()), Err)
}

/// The mass words, when the outfit's mass is above the free mass clamped
/// at none ([`Outfitter::free_mass_clamped`]): so one of mass 0 or less
/// always fits, even when the free mass is negative (`_CanBuyOutfitItem`
/// @0x4e88a-0x4e892, `_OutfitDialogUpdate` @0x57e3c-0x57e45). By the
/// count owned: [`OutfitRefusal::NoSpaceForAny`] with none,
/// [`OutfitRefusal::NoSpace`] with some.
fn lacks_the_mass(asked: &RowOutfit) -> Option<OutfitRefusal> {
    (asked.row.mass > asked.outfitter.free_mass_clamped()).then_some(if asked.row.owned == 0 {
        OutfitRefusal::NoSpaceForAny
    } else {
        OutfitRefusal::NoSpace
    })
}

/// A fighter's bays have room for another.
fn bay_room(asked: &RowOutfit) -> Result<(), OutfitRefusal> {
    holds(
        asked.row.facts.fighter_room != Some(0),
        OutfitRefusal::BaysFull,
    )
}

/// A mass expansion is allowed: the ship's `Holds` is not negative.
fn expansion_allowed(asked: &RowOutfit) -> Result<(), OutfitRefusal> {
    let forbidden = asked.row.facts.expansion.is_some() && asked.outfitter.holds < 0;
    holds(!forbidden, OutfitRefusal::NoExpansion)
}

/// A mass expansion has the ship's own free cargo of its size.
fn cargo_room(asked: &RowOutfit) -> Result<(), OutfitRefusal> {
    let short = asked
        .row
        .facts
        .expansion
        .is_some_and(|size| asked.outfitter.free_cargo < i64::from(size));
    holds(!short, OutfitRefusal::NoCargoRoom)
}

/// The engine's limit: the once-an-opening flag that limits the outfit,
/// if any, is clear.
fn not_bought_this_opening(asked: &RowOutfit) -> Result<(), OutfitRefusal> {
    let refused = asked
        .outfitter
        .visit
        .bought
        .refuses(asked.row.facts.opening_limit);
    holds(!refused, OutfitRefusal::BoughtThisOpening)
}

/// The other reading: no limit.
#[allow(clippy::unnecessary_wraps)] // A check's type, the engine's reading's.
fn never_limited(_: &RowOutfit) -> Result<(), OutfitRefusal> {
    Ok(())
}

/// The player can pay the price.
fn cash_for_the_price(asked: &RowOutfit) -> Result<(), OutfitRefusal> {
    holds(
        asked.row.buy_price <= asked.outfitter.cash,
        OutfitRefusal::CannotAfford,
    )
}

/// The player owns some.
fn owned(asked: &RowOutfit) -> Result<(), OutfitRefusal> {
    holds(asked.row.owned > 0, OutfitRefusal::NoneOwned)
}

/// The outfit is not flagged [`OutfitFlags::CANNOT_SELL`].
fn sellable(asked: &RowOutfit) -> Result<(), OutfitRefusal> {
    holds(
        asked.row.facts.flags & OutfitFlags::CANNOT_SELL == 0,
        OutfitRefusal::CannotSell,
    )
}

/// The outfit can be sold here: for sale, or sold anywhere.
fn sold_here(asked: &RowOutfit) -> Result<(), OutfitRefusal> {
    holds(asked.row.facts.sold_here, OutfitRefusal::NotBoughtHere)
}

/// The engine's sale mass: the free mass clamped at none, plus the
/// outfit's mass, is not below none (`_DoOutfitDialog` @0x5c740-0x5c747).
/// The clamped free mass is never below none, so only an outfit of
/// negative mass is refused, and the engine's sign test (@0x5c735-0x5c73e)
/// needs no test of its own.
fn clamped_mass_left(asked: &RowOutfit) -> Result<(), OutfitRefusal> {
    holds(
        asked.outfitter.free_mass_clamped() + asked.row.mass >= 0,
        OutfitRefusal::NegativeFreeMass,
    )
}

/// The other reading's sale mass: the sale leaves the unclamped free mass
/// not below none.
fn mass_left(asked: &RowOutfit) -> Result<(), OutfitRefusal> {
    holds(
        asked.outfitter.free_mass + asked.row.mass >= 0,
        OutfitRefusal::NegativeFreeMass,
    )
}

/// The engine's raised `Max` on a sale: the target held is no more than
/// its raw `Max` x the raisers left ([`raisers_left`]: owned - 1 for a
/// single sale), other raisers ignored and with no floor; none is left of
/// a target with no record.
fn engine_raised_first(asked: &RowOutfit) -> Result<(), OutfitRefusal> {
    let left = raisers_left().of(asked);
    raised_first_past(asked, |raise| {
        raise.target_max.map_or(0, |max| i64::from(max) * left)
    })
}

/// How many of the raiser a raised-`Max` check counts as left at a pass,
/// as [`RuleKey::CountedSale`] says: by the engine, owned now less the
/// whole count (@0x5c82d-0x5c85e); by the other reading, owned now less
/// one. The same at a count of 1.
fn raisers_left<'a>() -> Value<RowOutfit<'a>, i64> {
    Value::disputed(
        "raisers left",
        "`_DoOutfitDialog` @0x5c82d-0x5c85e: owned now - the whole count (-0x68c)",
        RuleKey::CountedSale,
        owned_less_the_count,
        owned_less_one,
    )
}

/// The engine's raisers left: owned now less the whole count, signed.
fn owned_less_the_count(asked: &RowOutfit) -> i64 {
    i64::from(asked.row.owned) - i64::from(asked.pass.count)
}

/// The other reading's raisers left: owned now less one, as one sale.
fn owned_less_one(asked: &RowOutfit) -> i64 {
    i64::from(asked.row.owned) - 1
}

/// The launchers whose `MaxAmmo` a launcher-sale check counts as kept at
/// a pass, as [`RuleKey::CountedSale`] says: by the engine, owned now less
/// the pass index less one, signed (@0x5cb9c-0x5cbc7); by the other
/// reading, owned now less one, at least none. The same at pass 0 for
/// some owned.
fn launchers_left<'a>() -> Value<RowOutfit<'a>, i64> {
    Value::disputed(
        "launchers left",
        "`_DoOutfitDialog` @0x5cb9c-0x5cbc7: owned now - the pass index (-0x63a) - 1",
        RuleKey::CountedSale,
        owned_less_the_pass,
        one_launcher_sold,
    )
}

/// The engine's launchers left: owned now less the pass index less one,
/// signed.
fn owned_less_the_pass(asked: &RowOutfit) -> i64 {
    i64::from(asked.row.owned) - i64::from(asked.pass.index) - 1
}

/// The other reading's launchers left: owned now less one, at least none.
fn one_launcher_sold(asked: &RowOutfit) -> i64 {
    i64::from(asked.row.owned.saturating_sub(1))
}

/// The Bible's raised `Max` on a sale: the target held is no more than
/// its raised `Max` once one of the raiser is sold.
fn bible_raised_first(asked: &RowOutfit) -> Result<(), OutfitRefusal> {
    raised_first_past(asked, |raise| raise.left_by_the_bible)
}

/// The first of the outfit's raises, in slot order, whose target held is
/// more than `left` of it leaves, refusing with that excess to be sold
/// first.
fn raised_first_past(asked: &RowOutfit, left: impl Fn(&Raise) -> i64) -> Result<(), OutfitRefusal> {
    asked
        .row
        .facts
        .raises
        .iter()
        .find_map(|raise| {
            let excess = i64::from(raise.held) - left(raise);
            (excess > 0).then(|| OutfitRefusal::RaisedFirst {
                count: u32::try_from(excess).unwrap_or(u32::MAX),
                target: raise.target,
            })
        })
        .map_or(Ok(()), Err)
}

/// The engine's launcher sale ([`Launcher::excess_left`](super::Launcher::excess_left)).
fn engine_ammunition_first(asked: &RowOutfit) -> Result<(), OutfitRefusal> {
    ammunition_first(asked, RuleSource::Engine)
}

/// The other reading's launcher sale ([`Launcher::excess_left`](super::Launcher::excess_left)).
fn bible_ammunition_first(asked: &RowOutfit) -> Result<(), OutfitRefusal> {
    ammunition_first(asked, RuleSource::Bible)
}

/// A launcher sells when no rounds must be sold first, as `source` reads
/// [`Launcher::excess_left`](super::Launcher::excess_left) of the [`launchers_left`].
fn ammunition_first(asked: &RowOutfit, source: RuleSource) -> Result<(), OutfitRefusal> {
    asked
        .row
        .facts
        .launcher
        .and_then(|launcher| {
            let rounds = launcher.excess_left(launchers_left().of(asked), source)?;
            Some(OutfitRefusal::AmmunitionFirst {
                rounds,
                ammo: launcher.ammo,
            })
        })
        .map_or(Ok(()), Err)
}

/// The info box's words for an outfit ([`Outfitter::words`]).
fn info_words<'a>() -> Value<RowOutfit<'a>, Option<OutfitRefusal>> {
    Value::plain(
        "words",
        "`_OutfitDialogUpdate` @0x57dba-0x57e6f: `_HasMaxOfItem` @0x57dd6, then the mass \
         @0x57e33-0x57e45 against `free_mass_clamped`",
        has_max_or_mass,
    )
}

/// [`has_max`]'s refusal, else [`lacks_the_mass`].
fn has_max_or_mass(asked: &RowOutfit) -> Option<OutfitRefusal> {
    refusal(&has_max(), asked)
        .err()
        .or_else(|| lacks_the_mass(asked))
}

/// The cap `_HasMaxOfItem(item, 0, &cap)` reports out for an outfit
/// ([`Outfitter::cap`]).
fn reported_cap<'a>() -> Value<RowOutfit<'a>, i16> {
    Value::plain(
        "cap",
        "`_HasMaxOfItem` out-parameter @0x4512: ammunition @0x45b2-0x45c1, raised \
         @0x4681-0x46a0, guns @0x4798-0x47a7, turrets @0x482b-0x483a",
        cap_reported,
    )
}

/// What a sale of an outfit refunds ([`Outfitter::refund`]).
fn refund_value<'a>() -> Value<RowOutfit<'a>, i64> {
    Value::disputed(
        "refund",
        "`_DoOutfitDialog` @0x5ceee-0x5cf08",
        RuleKey::OutfitRefund,
        full_since_the_visit_opened,
        always_half,
    )
}

/// The engine's refund: what was paid when the count owned is above the
/// visit's count on entry, otherwise half the price.
fn full_since_the_visit_opened(asked: &RowOutfit) -> i64 {
    refund(
        asked.row.price,
        asked.row.buy_price,
        asked.row.owned,
        asked.outfitter.visit.opened_with(asked.row.id),
        RuleSource::Engine,
    )
}

/// The other reading's refund: always half.
fn always_half(asked: &RowOutfit) -> i64 {
    resale(asked.row.price)
}

/// Its `Max`, lowered to a smaller ammunition cap (@0x45b2-0x45c1);
/// replaced by the raised `Max` when more than one raiser counts
/// (@0x4681-0x46a0: the engine also replaces it when the raised `Max` is
/// the smaller, which with a multiplier of 1 it never is); then, for a
/// gun, lowered to the ship's whole gun limit (@0x4798-0x47a7) and, for a
/// turret, to its whole turret limit (@0x482b-0x483a). Each is held in 16
/// bits ([`saturate_i16`]), where the engine's `imulw` and `addw` wrap.
fn cap_reported(asked: &RowOutfit) -> i16 {
    let facts = &asked.row.facts;
    let armed = asked.outfitter.armed;
    let mut cap = asked.row.max;
    if let Some(ammo) = facts.ammo_cap {
        cap = cap.min(saturate_i16(ammo));
    }
    if let Some(raised) = facts.raised {
        cap = saturate_i16(raised);
    }
    if facts.flags & OutfitFlags::GUN != 0 {
        cap = cap.min(saturate_i16(i64::from(armed.gun_limit)));
    }
    if facts.flags & OutfitFlags::TURRET != 0 {
        cap = cap.min(saturate_i16(i64::from(armed.turret_limit)));
    }
    cap
}

/// What one outfit priced at `price` sells back for at half.
#[must_use]
pub fn resale(price: i64) -> i64 {
    price.saturating_mul(RESALE_PERCENT) / 100
}

/// What selling one outfit priced at `price`, bought here for `paid`,
/// refunds, the player owning `owned` of it before the sale and having
/// owned `opened_with` when the outfitter opened, as `source` says
/// ([`RuleKey::OutfitRefund`](crate::RuleKey::OutfitRefund)).
///
/// By the engine ([`RuleSource::Engine`]), `_DoOutfitDialog` compares the
/// count owned before the sale with its copy of the counts taken on
/// opening (@0x5ceee-0x5cefa): one bought since, the count above the copy,
/// refunds in full what was paid, `paid` (the price, unless the other
/// reading of [`RuleKey::RankPriceScope`](crate::RuleKey::RankPriceScope)
/// scaled it); any other refunds [`resale`], the price times
/// 0.5 (the double at @0xdd128) truncated toward zero (@0x5cefc-0x5cf08).
/// By the other reading, the port's earlier behaviour, every sale refunds
/// [`resale`].
#[must_use]
pub fn refund(price: i64, paid: i64, owned: u16, opened_with: u16, source: RuleSource) -> i64 {
    if source == RuleSource::Engine && owned > opened_with {
        paid
    } else {
        resale(price)
    }
}

/// `record`'s `Max` as the [`RAISES_MAX`] outfits `owned` raise it, read
/// as `source` says ([`RuleKey::RaisedMax`](crate::RuleKey::RaisedMax)).
/// By the engine, the multiplier is the sum, over the outfits owned, of
/// the count owned times the number of its mods naming `record`; by the
/// Bible, the count owned of each outfit with any such mod. Either way it
/// is at least 1 (`_HasMaxOfItem` @0x45e9-0x46c3), so a `Max` of 0 or less
/// stays so. The engine's sums and product are 16-bit (`addw`/`imulw`)
/// and wrap; here they are wide and saturate.
#[must_use]
pub fn raised_max(
    record: &OutfitRecord,
    owned: &BTreeMap<OutfitId, u16>,
    records: &[OutfitRecord],
    source: RuleSource,
) -> i64 {
    i64::from(record.max).saturating_mul(raise_multiplier(record, owned, records, source))
}

/// What [`raised_max`] multiplies `record`'s `Max` by: at least 1. `pub`
/// for `nova_sim_port::outfitter`'s listing, whose raised-`Max` value
/// reads it.
#[must_use]
pub fn raise_multiplier(
    record: &OutfitRecord,
    owned: &BTreeMap<OutfitId, u16>,
    records: &[OutfitRecord],
    source: RuleSource,
) -> i64 {
    records
        .iter()
        .filter_map(|raiser| Some((raiser, *owned.get(&raiser.id)?)))
        .map(|(raiser, count)| {
            let mods: i64 = raiser
                .mods
                .iter()
                .filter(|&&(kind, val)| kind == RAISES_MAX && val == record.id.0)
                .map(|_| 1)
                .sum();
            let mods = match source {
                RuleSource::Engine => mods,
                RuleSource::Bible => mods.min(1),
            };
            i64::from(count) * mods
        })
        .fold(0, i64::saturating_add)
        .max(1)
}

/// `value` held in the engine's 16 bits, at the least or most they hold
/// rather than wrapped past it.
fn saturate_i16(value: i64) -> i16 {
    let held = value.clamp(i64::from(i16::MIN), i64::from(i16::MAX));
    i16::try_from(held).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shown::outfitter::{
        Armed, Bought, Launcher, OpeningLimit, OutfitFacts, OutfitVisit,
    };
    use crate::shown::wares::TonUnit;
    use nova_sim_base::rules::Descriptor;
    use nova_sim_catalog::fixture::outfit;

    /// Row 128, listed, bought and sold here, its every buy fact holding,
    /// at `price`, `owned` of a `Max` of `max`, of mass none.
    fn row(price: i64, owned: u16, max: i16) -> OutfitRow {
        OutfitRow {
            id: OutfitId(128),
            name: String::new(),
            short_name: String::new(),
            price,
            buy_price: price,
            mass: 0,
            owned,
            max,
            facts: OutfitFacts {
                listed: true,
                buy_tech: true,
                required: true,
                available: true,
                sold_here: true,
                ..OutfitFacts::default()
            },
        }
    }

    /// An outfitter of `row` alone, with 1,000 credits, 100 tons free,
    /// room for two guns and two turrets and 50 tons of cargo free.
    fn shop(row: OutfitRow) -> Outfitter {
        Outfitter {
            rows: vec![row],
            cash: 1_000,
            free_mass: 100,
            armed: Armed {
                gun_limit: 2,
                turret_limit: 2,
                ..Armed::default()
            },
            holds: 10,
            free_cargo: 50,
            ..Outfitter::default()
        }
    }

    /// `outfitter` with `key` read as `source`.
    fn reading(outfitter: Outfitter, key: RuleKey, source: RuleSource) -> Outfitter {
        Outfitter {
            rules: Rulebook::default().with_override(key, source),
            ..outfitter
        }
    }

    /// An order of outfit 128.
    fn order(direction: Direction) -> OutfitOrder {
        OutfitOrder {
            outfit: OutfitId(128),
            direction,
        }
    }

    fn buy(outfitter: &Outfitter) -> Result<(), OutfitRefusal> {
        outfitter.can(order(Direction::Buy))
    }

    fn pass(index: u32, count: u32) -> Pass {
        Pass {
            index,
            count,
            alerted: false,
        }
    }

    /// `row` with its facts changed by `facts`.
    fn with(row: OutfitRow, facts: impl FnOnce(&mut OutfitFacts)) -> OutfitRow {
        let mut row = row;
        facts(&mut row.facts);
        row
    }

    /// Each check of `checks` with its citation, in order.
    fn listed_checks<C>(checks: &Checks<C, OutfitRefusal>) -> Vec<(&'static str, &'static str)> {
        checks
            .descriptors()
            .map(|about| (about.name(), about.reproduces()))
            .collect()
    }

    /// The disputed rule each check of `checks` reads, in order.
    fn keys<C>(checks: &Checks<C, OutfitRefusal>) -> Vec<Option<RuleKey>> {
        checks.descriptors().map(Descriptor::reads).collect()
    }

    #[test]
    fn the_buy_checks_are_canbuyoutfititems_in_its_order() {
        assert_eq!(
            listed_checks(&can_buy()),
            [
                (
                    "tech",
                    "`_CanBuyOutfitItem` @0x4e7ff-0x4e83d, as `buy tech` reads it at build \
                     (`TechLevel` -1 skips it @0x4e802)"
                ),
                ("outfit range", "`_HasMaxOfItem` @0x451b-0x4547"),
                (
                    "ammunition cap",
                    "`_HasMaxOfItem` @0x457e-0x45e4, via `_CanBuyOutfitItem` @0x4e87d"
                ),
                (
                    "raised max",
                    "`_HasMaxOfItem` @0x45e9-0x46c3, the multiplier settled at build"
                ),
                ("gun limit", "`_HasMaxOfItem` @0x46c8-0x47b7"),
                ("turret limit", "`_HasMaxOfItem` @0x47c7-0x4866"),
                (
                    "free mass",
                    "`_CanBuyOutfitItem` @0x4e88a-0x4e897, reads `free_mass_clamped` \
                     (`_ShipFreeMass` @0x4e7e1)"
                ),
                (
                    "bay room",
                    "`_CanBuyOutfitItem` @0x4e8f6-0x4e93f, `_CanBuyFighter` @0x5a82"
                ),
                ("expansion allowed", "`_CanBuyOutfitItem` @0x4e94f-0x4e97e"),
                ("cargo room", "`_CanBuyOutfitItem` @0x4e980-0x4e9a8"),
                ("once an opening", "`_CanBuyOutfitItem` @0x4e9aa-0x4e9c5"),
                ("required", "`_CanBuyOutfitItem` @0x4e9cb-0x4eb37"),
                ("available", "`_CanBuyOutfitItem` @0x4eb51-0x4eb58"),
                ("cash", "`_CanBuyOutfitItem` @0x4eb95-0x4ebad"),
            ]
        );
    }

    #[test]
    fn the_has_max_checks_are_hasmaxofitems_in_order() {
        let has_max = listed_checks(&has_max());
        assert_eq!(
            has_max.iter().map(|&(name, _)| name).collect::<Vec<_>>(),
            [
                "outfit range",
                "ammunition cap",
                "raised max",
                "gun limit",
                "turret limit"
            ]
        );
        assert_eq!(has_max, listed_checks(&can_buy())[1..6]);
    }

    #[test]
    fn only_the_limit_hasmaxofitem_stops_at_refuses() {
        // Owned at its `Max` and past the gun limit: `_HasMaxOfItem`
        // returns at the `Max` (@0x46a5-0x46c3) and never reaches the gun
        // limit, so only that check refuses.
        let refusing = |owned: u16| {
            let outfitter = Outfitter {
                rows: vec![OutfitRow {
                    facts: OutfitFacts {
                        flags: OutfitFlags::GUN,
                        ..row(0, owned, 10).facts
                    },
                    ..row(0, owned, 10)
                }],
                armed: Armed {
                    guns: i32::from(owned),
                    gun_limit: 1,
                    ..Armed::default()
                },
                ..Outfitter::default()
            };
            let asked = asked_at(&outfitter, OutfitId(128), Pass::SINGLE).expect("listed");
            has_max()
                .all_refusals(&asked)
                .iter()
                .map(|(about, _)| about.name())
                .collect::<Vec<_>>()
        };
        assert_eq!(refusing(10), ["raised max"]);
        assert_eq!(refusing(1), ["gun limit"]);
        assert_eq!(refusing(0), Vec::<&str>::new());
    }

    #[test]
    fn the_sell_flag_is_owned_and_sellable_then_sold_here() {
        assert_eq!(
            listed_checks(&sell_flag()),
            [
                (
                    "owned",
                    "`_DoOutfitDialog` @0x5c563-0x5c570; `_OutfitFilter` @0x5808c-0x58096"
                ),
                (
                    "can be sold",
                    "`_DoOutfitDialog` @0x5c572-0x5c595; `_OutfitFilter` @0x58098-0x580b1"
                ),
                (
                    "sold here",
                    "port-only: the row as listed (`_SetupPortAvailableItems` @0xc0a8-0xc0ba \
                     lists an owned `SELL_ANYWHERE` outfit). The original's Sell flag asks no list"
                ),
            ]
        );
    }

    #[test]
    fn the_sale_checks_are_the_sell_loops_in_order() {
        assert_eq!(
            listed_checks(&sale_checks()),
            [
                (
                    "free mass",
                    "`_DoOutfitDialog` @0x5c735-0x5c7ac. The engine reads `free_mass_clamped` \
                     (`_ShipFreeMass` @0x5c740), and the other reading reads `free_mass`"
                ),
                (
                    "raised first",
                    "`_DoOutfitDialog` @0x5c7be-0x5ca6f. The engine's left is \
                     `target_max x raisers_left`, and the other's is `left_by_the_bible`"
                ),
                (
                    "ammunition first",
                    "`_DoOutfitDialog` @0x5ca75-0x5cbe0, through \
                     `Launcher::excess_left(launchers_left, Engine / Bible)`"
                ),
            ]
        );
    }

    #[test]
    fn the_refund_and_count_values_read_their_keys() {
        assert_eq!(refund_value().about().reads(), Some(RuleKey::OutfitRefund));
        assert_eq!(
            refund_value().about().reproduces(),
            "`_DoOutfitDialog` @0x5ceee-0x5cf08"
        );
        assert_eq!(
            option_asks_count().about().reads(),
            Some(RuleKey::OutfitCount)
        );
        assert_eq!(
            option_asks_count().about().reproduces(),
            "`_DoOutfitDialog` @0x5c0c0-0x5c1aa, @0x5c5e3-0x5c628"
        );
    }

    #[test]
    fn the_free_mass_shown_is_clamped_at_none_with_a_ton_at_exactly_one() {
        for (free_mass, tons, unit) in [
            (-15, 0, TonUnit::Tons),
            (0, 0, TonUnit::Tons),
            (1, 1, TonUnit::Ton),
            (2, 2, TonUnit::Tons),
            (30, 30, TonUnit::Tons),
        ] {
            let outfitter = Outfitter {
                free_mass,
                ..Outfitter::default()
            };
            assert_eq!(
                outfitter.free_mass_shown(),
                ShownTons { tons, unit },
                "{free_mass}"
            );
        }
    }

    #[test]
    fn the_free_mass_shown_names_what_it_reproduces() {
        let value = free_mass_shown();
        let about = value.about();
        assert_eq!(about.name(), "free mass shown");
        assert!(about.reproduces().contains("@0x57d66-0x57d9d"));
        assert!(about.reproduces().contains("free_mass_clamped"));
        assert_eq!(about.reads(), None);
    }

    #[test]
    fn the_item_mass_shown_is_none_without_mass_and_unclamped_with_it() {
        let shown = |mass_field, mass| {
            Outfitter {
                rows: vec![OutfitRow {
                    mass,
                    facts: OutfitFacts {
                        mass_field,
                        ..OutfitFacts::default()
                    },
                    ..row(0, 0, 10)
                }],
                ..Outfitter::default()
            }
            .item_mass_shown(OutfitId(128))
        };
        assert_eq!(shown(0, 0), None);
        assert_eq!(shown(-5, -5), None);
        let tons = |tons, unit| Some(ShownTons { tons, unit });
        assert_eq!(shown(1, 1), tons(1, TonUnit::Ton));
        assert_eq!(shown(1, 2), tons(2, TonUnit::Tons));
        assert_eq!(
            shown(1, 0),
            tons(0, TonUnit::Tons),
            "mass-scaled on a light class"
        );
        assert_eq!(
            Outfitter::default().item_mass_shown(OutfitId(128)),
            None,
            "not listed"
        );
    }

    #[test]
    fn the_item_mass_shown_names_what_it_reproduces() {
        let value = item_mass_shown();
        let about = value.about();
        assert_eq!(about.name(), "item mass shown");
        assert!(about.reproduces().contains("@0x57c67-0x57d19"));
        assert!(about.reproduces().contains("@0x57da2"));
        assert_eq!(about.reads(), None);
    }

    #[test]
    fn the_free_mass_checks_name_the_fact_they_read() {
        let named = |checks: &Checks<RowOutfit, OutfitRefusal>, name: &str| {
            checks
                .descriptors()
                .find(|about| about.name() == name)
                .map(Descriptor::reproduces)
                .expect("listed")
        };
        assert!(named(&can_buy(), "free mass").contains("reads `free_mass_clamped`"));
        let sale = named(&sale_checks(), "free mass");
        assert!(
            sale.contains("The engine reads `free_mass_clamped`"),
            "{sale}"
        );
        assert!(
            sale.contains("the other reading reads `free_mass`"),
            "{sale}"
        );
        let words = info_words().about().reproduces();
        assert!(words.contains("against `free_mass_clamped`"), "{words}");
    }

    #[test]
    fn the_buttons_sale_checks_read_the_sale_refusal_rule_and_the_checks_keep_their_own() {
        let button = sale_checks_on_the_button();
        assert_eq!(button.about().name(), "sale checks on the button");
        assert_eq!(button.about().reads(), Some(RuleKey::SaleRefusal));
        assert_eq!(
            button.about().reproduces(),
            "`_DoOutfitDialog` @0x5c563-0x5c595 and `_OutfitFilter` @0x5808c-0x580b1 set \
             the Sell flag by the count owned and `CANNOT_SELL` alone; the sale checks run \
             only in the sell loop, @0x5c735-0x5ce6c"
        );
        assert!(!button.about().draws());
        // The enable runs these two lists, each check under its own key.
        assert_eq!(keys(&sell_flag()), [None; 3]);
        assert_eq!(
            keys(&sale_checks()),
            [
                Some(RuleKey::SaleMass),
                Some(RuleKey::RaisedMax),
                Some(RuleKey::LauncherSale),
            ]
        );
        for about in [*sale_words().about(), *sale_alert_value().about()] {
            assert_eq!(
                about.reads(),
                Some(RuleKey::SaleRefusal),
                "{}",
                about.name()
            );
            assert!(!about.draws(), "{}", about.name());
        }
        assert_eq!(sale_words().about().name(), "sale words");
        assert_eq!(
            sale_words().about().reproduces(),
            "`_OutfitDialogUpdate` @0x57a12-0x57e68: only #213-#222"
        );
        assert_eq!(sale_alert_value().about().name(), "sale alert");
        assert_eq!(
            sale_alert_value().about().reproduces(),
            "`_DoOutfitDialog` @0x5c735-0x5ce6c, -0x63e @0x5c6ca; `_ShowTextDialog` @0x5a8cf"
        );
    }

    #[test]
    fn passes_remain_after_a_pass_but_the_last() {
        assert!(!passes_remain(pass(0, 1)));
        assert!(passes_remain(pass(0, 2)));
        assert!(passes_remain(pass(1, 3)));
        assert!(!passes_remain(pass(2, 3)));
        assert!(!passes_remain(pass(3, 3)), "past the count");
    }

    #[test]
    fn the_counted_sale_rules_read_the_counted_sale_key() {
        for (about, name, reproduces) in [
            (
                *raisers_left().about(),
                "raisers left",
                "`_DoOutfitDialog` @0x5c82d-0x5c85e: owned now - the whole count (-0x68c)",
            ),
            (
                *launchers_left().about(),
                "launchers left",
                "`_DoOutfitDialog` @0x5cb9c-0x5cbc7: owned now - the pass index (-0x63a) - 1",
            ),
        ] {
            assert_eq!(about.name(), name);
            assert_eq!(about.reads(), Some(RuleKey::CountedSale), "{name}");
            assert_eq!(about.reproduces(), reproduces, "{name}");
            assert!(!about.draws(), "{name}");
        }
    }

    #[test]
    fn the_value_rules_cite_the_originals_addresses() {
        assert_eq!(
            [
                info_words().about().reproduces(),
                reported_cap().about().reproduces(),
            ],
            [
                "`_OutfitDialogUpdate` @0x57dba-0x57e6f: `_HasMaxOfItem` @0x57dd6, then the mass \
                 @0x57e33-0x57e45 against `free_mass_clamped`",
                "`_HasMaxOfItem` out-parameter @0x4512: ammunition @0x45b2-0x45c1, raised \
                 @0x4681-0x46a0, guns @0x4798-0x47a7, turrets @0x482b-0x483a",
            ]
        );
    }

    #[test]
    fn each_disputed_outfitter_rule_names_its_key() {
        let mut buying = vec![None; 14];
        buying[10] = Some(RuleKey::OutfitLimit);
        assert_eq!(keys(&can_buy()), buying);
        assert_eq!(keys(&has_max()), [None; 5]);
        assert_eq!(keys(&sell_flag()), [None; 3]);
        assert_eq!(
            keys(&sale_checks()),
            [
                Some(RuleKey::SaleMass),
                Some(RuleKey::RaisedMax),
                Some(RuleKey::LauncherSale),
            ]
        );
        for plain in [*info_words().about(), *reported_cap().about()] {
            assert_eq!(plain.reads(), None, "{}", plain.name());
        }
    }

    #[test]
    fn no_outfitter_check_draws() {
        let lists = [can_buy(), has_max(), sell_flag(), sale_checks()];
        for about in lists.iter().flat_map(Checks::descriptors) {
            assert!(!about.draws(), "{}", about.name());
        }
        for about in [
            *info_words().about(),
            *reported_cap().about(),
            *option_asks_count().about(),
            *free_mass_shown().about(),
            *refund_value().about(),
        ] {
            assert!(!about.draws(), "{}", about.name());
        }
    }

    #[test]
    fn an_outfit_with_no_row_is_not_listed_and_has_no_answers() {
        let outfitter = shop(row(10, 1, 10));
        let other = OutfitOrder {
            outfit: OutfitId(129),
            direction: Direction::Buy,
        };
        assert_eq!(outfitter.can(other), Err(OutfitRefusal::NotListed));
        assert_eq!(outfitter.check(other), Err(OutfitRefusal::NotListed));
        assert_eq!(
            outfitter.check_at(other, pass(0, 3)),
            Err(OutfitRefusal::NotListed)
        );
        let sale = OutfitOrder {
            direction: Direction::Sell,
            ..other
        };
        assert_eq!(outfitter.can(sale), Err(OutfitRefusal::NotListed));
        assert_eq!(outfitter.check(sale), Err(OutfitRefusal::NotListed));
        assert_eq!(outfitter.sale_alert(sale, Pass::SINGLE), None);
        assert_eq!(outfitter.lc_names(sale), BTreeMap::new());
        let id = OutfitId(129);
        assert_eq!(outfitter.words(id), None);
        assert_eq!(outfitter.sale_words(id), None);
        assert_eq!(outfitter.item_mass_shown(id), None);
        assert_eq!(outfitter.cap(id), None);
        assert_eq!(outfitter.refund(id), None);
        assert_eq!(outfitter.count_max(id, Direction::Buy), None);
        assert_eq!(outfitter.count_max(id, Direction::Sell), None);
        assert_eq!(outfitter.count_asked(id, Direction::Buy), None);
    }

    #[test]
    fn the_unlisted_row_is_asked_like_a_listed_one() {
        let outfitter = Outfitter {
            rows: Vec::new(),
            unlisted: Some(row(10, 1, 10)),
            ..shop(row(10, 1, 10))
        };
        assert_eq!(buy(&outfitter), Ok(()));
        assert_eq!(
            outfitter.refund(OutfitId(128)),
            Some(10),
            "owned above the visit's none on entry"
        );
    }

    #[test]
    fn each_buy_fact_refuses_with_its_own_refusal() {
        let refused = |facts: fn(&mut OutfitFacts)| buy(&shop(with(row(10, 1, 10), facts)));
        assert_eq!(refused(|_| ()), Ok(()));
        assert_eq!(
            refused(|facts| facts.buy_tech = false),
            Err(OutfitRefusal::NotForSale)
        );
        assert_eq!(
            refused(|facts| facts.ammo_cap = Some(1)),
            Err(OutfitRefusal::MaxOwned)
        );
        assert_eq!(refused(|facts| facts.ammo_cap = Some(2)), Ok(()));
        assert_eq!(
            refused(|facts| facts.raised = Some(1)),
            Err(OutfitRefusal::MaxOwned)
        );
        assert_eq!(
            refused(|facts| facts.fighter_room = Some(0)),
            Err(OutfitRefusal::BaysFull)
        );
        assert_eq!(refused(|facts| facts.fighter_room = Some(1)), Ok(()));
        assert_eq!(
            refused(|facts| facts.required = false),
            Err(OutfitRefusal::NotForSale)
        );
        assert_eq!(
            refused(|facts| facts.available = false),
            Err(OutfitRefusal::NotForSale)
        );
    }

    #[test]
    fn the_range_and_the_max_refuse_as_none_allowed_or_max_owned() {
        let outside = Outfitter {
            rows: vec![OutfitRow {
                id: OutfitId(127),
                ..row(10, 0, 10)
            }],
            ..shop(row(10, 0, 10))
        };
        let order = OutfitOrder {
            outfit: OutfitId(127),
            direction: Direction::Buy,
        };
        assert_eq!(outside.can(order), Err(OutfitRefusal::NoneAllowed));
        assert_eq!(buy(&shop(row(10, 10, 10))), Err(OutfitRefusal::MaxOwned));
        assert_eq!(buy(&shop(row(10, 9, 10))), Ok(()));
        assert_eq!(buy(&shop(row(10, 0, 0))), Err(OutfitRefusal::NoneAllowed));
        assert_eq!(buy(&shop(row(10, 0, -1))), Err(OutfitRefusal::NoneAllowed));
    }

    #[test]
    fn a_gun_or_turret_is_refused_at_its_limit() {
        let flagged = |flags| shop(with(row(10, 0, 10), |facts| facts.flags = flags));
        let at = |flags, guns, turrets| {
            let mut outfitter = flagged(flags);
            outfitter.armed.guns = guns;
            outfitter.armed.turrets = turrets;
            buy(&outfitter)
        };
        assert_eq!(at(OutfitFlags::GUN, 1, 9), Ok(()));
        assert_eq!(at(OutfitFlags::GUN, 2, 0), Err(OutfitRefusal::GunLimit));
        assert_eq!(at(OutfitFlags::TURRET, 9, 1), Ok(()));
        assert_eq!(
            at(OutfitFlags::TURRET, 0, 2),
            Err(OutfitRefusal::TurretLimit)
        );
        assert_eq!(
            at(OutfitFlags::GUN | OutfitFlags::TURRET, 2, 2),
            Err(OutfitRefusal::GunLimit),
            "the gun limit first"
        );
        assert_eq!(at(0, 9, 9), Ok(()), "the flag decides");
    }

    #[test]
    fn the_mass_must_fit_the_free_mass_clamped_at_none() {
        let massed = |mass, owned, free_mass| {
            buy(&Outfitter {
                free_mass,
                ..shop(OutfitRow {
                    mass,
                    ..row(10, owned, 10)
                })
            })
        };
        assert_eq!(massed(100, 0, 100), Ok(()), "exactly the free mass");
        assert_eq!(massed(101, 0, 100), Err(OutfitRefusal::NoSpaceForAny));
        assert_eq!(massed(101, 1, 100), Err(OutfitRefusal::NoSpace));
        assert_eq!(massed(0, 0, -20), Ok(()), "none always fits");
        assert_eq!(massed(1, 0, -20), Err(OutfitRefusal::NoSpaceForAny));
    }

    #[test]
    fn a_mass_expansion_needs_holds_of_none_or_more_and_the_cargo_free() {
        let expanding = |holds, free_cargo| {
            buy(&Outfitter {
                holds,
                free_cargo,
                ..shop(with(row(10, 0, 10), |facts| facts.expansion = Some(5)))
            })
        };
        assert_eq!(expanding(0, 5), Ok(()));
        assert_eq!(expanding(-1, 50), Err(OutfitRefusal::NoExpansion));
        assert_eq!(expanding(0, 4), Err(OutfitRefusal::NoCargoRoom));
        let plain = Outfitter {
            holds: -1,
            free_cargo: 0,
            ..shop(row(10, 0, 10))
        };
        assert_eq!(buy(&plain), Ok(()), "only an expansion");
    }

    #[test]
    fn once_an_opening_refuses_by_the_engine_and_never_by_the_other_reading() {
        let limited = |bought| Outfitter {
            visit: OutfitVisit {
                bought,
                ..OutfitVisit::default()
            },
            ..shop(with(row(10, 0, 10), |facts| {
                facts.opening_limit = Some(OpeningLimit::Map);
            }))
        };
        let map = Bought {
            map: true,
            record: false,
        };
        assert_eq!(buy(&limited(map)), Err(OutfitRefusal::BoughtThisOpening));
        assert_eq!(buy(&limited(Bought::default())), Ok(()));
        assert_eq!(
            buy(&reading(
                limited(map),
                RuleKey::OutfitLimit,
                RuleSource::Bible
            )),
            Ok(())
        );
    }

    #[test]
    fn the_cash_pays_the_buy_price() {
        let priced = |buy_price, cash| {
            buy(&Outfitter {
                cash,
                ..shop(OutfitRow {
                    buy_price,
                    ..row(10, 0, 10)
                })
            })
        };
        assert_eq!(priced(1_000, 1_000), Ok(()));
        assert_eq!(priced(1_001, 1_000), Err(OutfitRefusal::CannotAfford));
    }

    #[test]
    fn the_sell_flag_refuses_none_owned_a_cannot_sell_outfit_and_one_not_sold_here() {
        let sell = |row| shop(row).can(order(Direction::Sell));
        assert_eq!(sell(row(10, 1, 10)), Ok(()));
        assert_eq!(sell(row(10, 0, 10)), Err(OutfitRefusal::NoneOwned));
        assert_eq!(
            sell(with(row(10, 1, 10), |facts| facts.flags =
                OutfitFlags::CANNOT_SELL)),
            Err(OutfitRefusal::CannotSell)
        );
        assert_eq!(
            sell(with(row(10, 1, 10), |facts| facts.sold_here = false)),
            Err(OutfitRefusal::NotBoughtHere)
        );
    }

    /// A sale of row 128, `mass` and one owned, with `free_mass`.
    fn massed_sale(mass: i64, free_mass: i64) -> Outfitter {
        Outfitter {
            free_mass,
            ..shop(OutfitRow {
                mass,
                ..row(10, 1, 10)
            })
        }
    }

    #[test]
    fn the_sale_mass_reads_the_clamped_free_mass_by_the_engine_and_the_free_mass_by_the_other() {
        let sale = order(Direction::Sell);
        let refused = Err(OutfitRefusal::NegativeFreeMass);
        assert_eq!(massed_sale(-5, 5).check(sale), Ok(()));
        assert_eq!(massed_sale(-6, 5).check(sale), refused);
        assert_eq!(massed_sale(0, -10).check(sale), Ok(()), "clamped");
        let bible = |outfitter| reading(outfitter, RuleKey::SaleMass, RuleSource::Bible);
        assert_eq!(bible(massed_sale(0, -10)).check(sale), refused);
        assert_eq!(bible(massed_sale(10, -10)).check(sale), Ok(()));
        assert_eq!(bible(massed_sale(-5, 5)).check(sale), Ok(()));
    }

    #[test]
    fn by_the_engine_sell_is_enabled_by_the_sell_flag_alone_and_by_the_other_the_checks_grey_it() {
        let sale = order(Direction::Sell);
        let refused = Err(OutfitRefusal::NegativeFreeMass);
        assert_eq!(
            massed_sale(-6, 5).can(sale),
            Ok(()),
            "only the loop refuses"
        );
        let other = reading(massed_sale(-6, 5), RuleKey::SaleRefusal, RuleSource::Bible);
        assert_eq!(other.can(sale), refused);
        assert_eq!(other.check(sale), refused);
    }

    /// Row 128, `owned` of it, raising 200's `Max` of 2, five held, which
    /// leaves four by the Bible once one is sold.
    fn raiser(owned: u16) -> Outfitter {
        shop(with(row(10, owned, 10), |facts| {
            facts.raises = vec![Raise {
                target: OutfitId(200),
                held: 5,
                target_max: Some(2),
                left_by_the_bible: 4,
                target_names: Some(LcNames {
                    singular: "widget".into(),
                    plural: "widgets".into(),
                }),
            }];
        }))
    }

    #[test]
    fn a_raiser_is_sold_only_while_its_targets_held_fit_the_max_it_leaves() {
        let sale = order(Direction::Sell);
        let first = |count| {
            Err(OutfitRefusal::RaisedFirst {
                count,
                target: OutfitId(200),
            })
        };
        assert_eq!(raiser(2).check(sale), first(3), "2 x 1 left");
        assert_eq!(raiser(4).check(sale), Ok(()), "2 x 3 left");
        let bible = reading(raiser(2), RuleKey::RaisedMax, RuleSource::Bible);
        assert_eq!(bible.check(sale), first(1), "the Bible's 4 left");
        assert_eq!(
            raiser(4).check_at(sale, pass(0, 3)),
            first(3),
            "by the engine, owned less the whole count"
        );
        let one_at_a_time = reading(raiser(4), RuleKey::CountedSale, RuleSource::Bible);
        assert_eq!(one_at_a_time.check_at(sale, pass(0, 3)), Ok(()));
        let one_at_a_time = reading(raiser(3), RuleKey::CountedSale, RuleSource::Bible);
        assert_eq!(
            one_at_a_time.check_at(sale, pass(0, 3)),
            first(1),
            "owned less one: 2 x 2 left of 5"
        );
        let exactly = shop(with(row(10, 3, 10), |facts| {
            facts.raises = vec![Raise {
                target: OutfitId(200),
                held: 4,
                target_max: Some(2),
                left_by_the_bible: 4,
                target_names: None,
            }];
        }));
        assert_eq!(exactly.check(sale), Ok(()), "held exactly what is left");
        let unknown = shop(with(row(10, 2, 10), |facts| {
            facts.raises = vec![Raise {
                target: OutfitId(200),
                held: 1,
                target_max: None,
                left_by_the_bible: 0,
                target_names: None,
            }];
        }));
        assert_eq!(unknown.check(sale), first(1), "none left of no record");
    }

    /// Row 128, `owned` launchers of `MaxAmmo` 4 holding six rounds of
    /// outfit 201.
    fn launcher(owned: u16) -> Outfitter {
        shop(with(row(10, owned, 10), |facts| {
            facts.launcher = Some(Launcher {
                max_ammo: 4,
                rounds: 6,
                ammo: Some(OutfitId(201)),
            });
            facts.ammo_names = Some(LcNames {
                singular: "round".into(),
                plural: "rounds".into(),
            });
        }))
    }

    #[test]
    fn a_launcher_is_sold_only_once_the_rest_hold_its_rounds() {
        let sale = order(Direction::Sell);
        let first = |rounds| {
            Err(OutfitRefusal::AmmunitionFirst {
                rounds,
                ammo: Some(OutfitId(201)),
            })
        };
        assert_eq!(launcher(1).check(sale), first(6));
        assert_eq!(launcher(2).check(sale), first(2));
        assert_eq!(launcher(3).check(sale), Ok(()));
        let any_round = reading(launcher(3), RuleKey::LauncherSale, RuleSource::Bible);
        assert_eq!(any_round.check(sale), first(6));
        assert_eq!(
            launcher(3).check_at(sale, pass(1, 2)),
            first(2),
            "by the engine, the pass index counts"
        );
        let one_at_a_time = reading(launcher(3), RuleKey::CountedSale, RuleSource::Bible);
        assert_eq!(one_at_a_time.check_at(sale, pass(1, 2)), Ok(()));
    }

    #[test]
    fn the_words_are_the_has_max_refusal_then_the_mass_whatever_the_cash() {
        let words = |outfitter: Outfitter| outfitter.words(OutfitId(128));
        assert_eq!(words(shop(row(10, 1, 10))), None);
        assert_eq!(words(shop(row(10, 10, 10))), Some(OutfitRefusal::MaxOwned));
        let heavy = |owned, max| OutfitRow {
            mass: 200,
            ..row(10, owned, max)
        };
        assert_eq!(
            words(shop(heavy(0, 10))),
            Some(OutfitRefusal::NoSpaceForAny)
        );
        assert_eq!(words(shop(heavy(1, 10))), Some(OutfitRefusal::NoSpace));
        assert_eq!(
            words(shop(heavy(10, 10))),
            Some(OutfitRefusal::MaxOwned),
            "the max first"
        );
        let poor = Outfitter {
            cash: 0,
            ..shop(row(10, 1, 10))
        };
        assert_eq!(words(poor), None, "no cash words");
    }

    #[test]
    fn the_sale_words_are_none_by_the_engine_and_the_first_sale_check_by_the_other() {
        assert_eq!(massed_sale(-6, 5).sale_words(OutfitId(128)), None);
        let other = |outfitter| reading(outfitter, RuleKey::SaleRefusal, RuleSource::Bible);
        assert_eq!(
            other(massed_sale(-6, 5)).sale_words(OutfitId(128)),
            Some(OutfitRefusal::NegativeFreeMass)
        );
        assert_eq!(other(massed_sale(0, 5)).sale_words(OutfitId(128)), None);
        let none_owned = shop(OutfitRow {
            mass: -6,
            ..row(10, 0, 10)
        });
        assert_eq!(
            other(none_owned).sale_words(OutfitId(128)),
            None,
            "the Sell flag refuses first"
        );
    }

    #[test]
    fn by_the_engine_a_sale_refused_in_the_loop_alerts_once_naming_the_outfits() {
        let sale = order(Direction::Sell);
        let alert = launcher(1)
            .sale_alert(sale, Pass::SINGLE)
            .expect("an alert");
        assert_eq!(alert.outfit, OutfitId(128));
        assert_eq!(
            alert.refusal,
            OutfitRefusal::AmmunitionFirst {
                rounds: 6,
                ammo: Some(OutfitId(201)),
            }
        );
        assert_eq!(
            alert.names.keys().copied().collect::<Vec<_>>(),
            [OutfitId(128), OutfitId(201)]
        );
        assert!(!alert.plural, "a plain click");
        let plural = |index, count| {
            launcher(1)
                .sale_alert(sale, pass(index, count))
                .map(|alert| alert.plural)
        };
        assert_eq!(plural(0, 3), Some(true));
        assert_eq!(plural(1, 3), Some(true));
        assert_eq!(plural(2, 3), Some(false));
        let alerted = Pass {
            alerted: true,
            ..pass(1, 3)
        };
        assert_eq!(launcher(1).sale_alert(sale, alerted), None, "once an order");
        assert_eq!(launcher(3).sale_alert(sale, Pass::SINGLE), None, "sold");
        assert_eq!(
            launcher(1).sale_alert(order(Direction::Buy), Pass::SINGLE),
            None,
            "a buy"
        );
        let other = reading(launcher(1), RuleKey::SaleRefusal, RuleSource::Bible);
        assert_eq!(other.sale_alert(sale, Pass::SINGLE), None);
        let none_owned = launcher(0);
        assert_eq!(
            none_owned.sale_alert(sale, Pass::SINGLE),
            None,
            "the Sell flag"
        );
    }

    #[test]
    fn the_names_are_a_refused_launchers_and_its_ammunitions_or_a_raisers_and_its_targets() {
        let sale = order(Direction::Sell);
        assert_eq!(
            launcher(1)
                .lc_names(sale)
                .keys()
                .copied()
                .collect::<Vec<_>>(),
            [OutfitId(128), OutfitId(201)]
        );
        assert_eq!(launcher(3).lc_names(sale), BTreeMap::new(), "sold");
        let raised = raiser(2).lc_names(sale);
        assert_eq!(
            raised
                .get(&OutfitId(200))
                .map(|names| names.plural.as_str()),
            Some("widgets")
        );
        assert_eq!(raised.len(), 2);
        assert_eq!(
            shop(row(10, 10, 10)).lc_names(order(Direction::Buy)),
            BTreeMap::new(),
            "any other refusal"
        );
    }

    #[test]
    fn the_cap_is_the_max_as_the_ammunition_cap_the_raise_and_the_limits_change_it() {
        let cap =
            |facts: fn(&mut OutfitFacts)| shop(with(row(10, 0, 10), facts)).cap(OutfitId(128));
        assert_eq!(cap(|_| ()), Some(10));
        assert_eq!(cap(|facts| facts.ammo_cap = Some(4)), Some(4));
        assert_eq!(cap(|facts| facts.ammo_cap = Some(40)), Some(10));
        assert_eq!(cap(|facts| facts.raised = Some(30)), Some(30));
        assert_eq!(cap(|facts| facts.raised = Some(100_000)), Some(i16::MAX));
        assert_eq!(cap(|facts| facts.flags = OutfitFlags::GUN), Some(2));
        assert_eq!(cap(|facts| facts.flags = OutfitFlags::TURRET), Some(2));
        let mut wide = shop(with(row(10, 0, 10), |facts| {
            facts.flags = OutfitFlags::TURRET;
        }));
        wide.armed.turret_limit = 5;
        assert_eq!(wide.cap(OutfitId(128)), Some(5));
    }

    #[test]
    fn a_sale_refunds_in_full_one_bought_this_visit_by_the_engine_and_half_otherwise() {
        let bought = |opened_with| Outfitter {
            visit: OutfitVisit {
                opened_with: BTreeMap::from([(OutfitId(128), opened_with)]),
                ..OutfitVisit::default()
            },
            ..shop(OutfitRow {
                buy_price: 12,
                ..row(11, 2, 10)
            })
        };
        assert_eq!(bought(1).refund(OutfitId(128)), Some(12), "the price paid");
        assert_eq!(bought(2).refund(OutfitId(128)), Some(5), "half, truncated");
        let other = reading(bought(1), RuleKey::OutfitRefund, RuleSource::Bible);
        assert_eq!(other.refund(OutfitId(128)), Some(5));
        assert_eq!(refund(11, 12, 2, 1, RuleSource::Engine), 12);
        assert_eq!(refund(11, 12, 1, 1, RuleSource::Engine), 5);
        assert_eq!(refund(11, 12, 2, 1, RuleSource::Bible), 5);
        assert_eq!(resale(-11), -5, "toward zero");
        assert_eq!(resale(i64::MAX), i64::MAX / 100, "saturating");
    }

    #[test]
    fn the_buy_count_is_the_cash_over_the_price_within_the_max_and_the_cap() {
        let most = |price, cash, owned, max| {
            Outfitter {
                cash,
                ..shop(row(price, owned, max))
            }
            .count_max(OutfitId(128), Direction::Buy)
        };
        assert_eq!(most(10, 1_000, 0, 1_000), Some(100));
        assert_eq!(most(10, 1_000, 0, 20), Some(20), "the max");
        assert_eq!(most(10, 1_000, 15, 20), Some(5), "the max less those owned");
        assert_eq!(most(0, 0, 0, i16::MAX), Some(i64_to_u32(COUNT_CAP)));
        assert_eq!(most(1, i64::MAX, 0, i16::MAX), Some(i64_to_u32(COUNT_CAP)));
        assert_eq!(most(10, 15, 0, 20), None, "one opens no dialog");
        assert_eq!(most(10, 5, 0, 20), None, "refused");
        let capped = Outfitter {
            cash: 1_000,
            ..shop(with(row(10, 2, 20), |facts| facts.ammo_cap = Some(6)))
        };
        assert_eq!(capped.count_max(OutfitId(128), Direction::Buy), Some(4));
    }

    #[test]
    fn the_buy_count_keeps_to_the_max_less_those_owned_only_while_the_max_is_above_them() {
        // A raised cap above the `Max`: only the `Max` test limits it.
        let most = |owned, max| {
            Outfitter {
                cash: 1_000_000,
                ..shop(with(row(10, owned, max), |facts| facts.raised = Some(30)))
            }
            .count_max(OutfitId(128), Direction::Buy)
        };
        assert_eq!(most(15, 20), Some(5), "the max less those owned");
        assert_eq!(
            most(20, 20),
            Some(10),
            "at the max: the cap less those owned"
        );
        assert_eq!(
            most(25, 20),
            Some(5),
            "above the max: the cap less those owned"
        );
    }

    fn i64_to_u32(value: i64) -> u32 {
        u32::try_from(value).expect("a count")
    }

    #[test]
    fn the_sale_count_is_those_owned_within_the_cap_while_the_sell_flag_is_set() {
        let most = |owned| shop(row(10, owned, i16::MAX)).count_max(OutfitId(128), Direction::Sell);
        assert_eq!(most(7), Some(7));
        assert_eq!(most(1), None);
        assert_eq!(most(32_001), Some(32_000));
        assert_eq!(most(40_000), None, "read as 16 bits, signed");
        let sale_refused = shop(OutfitRow {
            mass: -200,
            ..row(10, 7, 10)
        });
        assert_eq!(
            sale_refused.count_max(OutfitId(128), Direction::Sell),
            Some(7),
            "the loop's checks do not close it"
        );
        let unsellable = shop(with(row(10, 7, 10), |facts| {
            facts.flags = OutfitFlags::CANNOT_SELL;
        }));
        assert_eq!(unsellable.count_max(OutfitId(128), Direction::Sell), None);
    }

    #[test]
    fn option_asks_the_count_by_the_engine_and_never_by_the_other_reading() {
        let outfitter = shop(row(10, 7, 100));
        assert_eq!(
            outfitter.count_asked(OutfitId(128), Direction::Sell),
            Some(7)
        );
        assert_eq!(
            outfitter.count_asked(OutfitId(128), Direction::Buy),
            Some(93)
        );
        let other = reading(outfitter, RuleKey::OutfitCount, RuleSource::Bible);
        assert_eq!(other.count_asked(OutfitId(128), Direction::Sell), None);
        assert_eq!(other.count_asked(OutfitId(128), Direction::Buy), None);
    }

    #[test]
    fn a_raised_max_is_the_max_times_the_raisers_owned_at_least_once() {
        let target = OutfitRecord {
            max: 3,
            ..outfit(200, &[])
        };
        let records = [
            target.clone(),
            outfit(300, &[(RAISES_MAX, 200)]),
            outfit(301, &[(RAISES_MAX, 200), (RAISES_MAX, 200)]),
            outfit(302, &[(RAISES_MAX, 999)]),
        ];
        let max = |owned: &[(i16, u16)], source| {
            let owned = owned.iter().map(|&(id, n)| (OutfitId(id), n)).collect();
            raised_max(&target, &owned, &records, source)
        };
        assert_eq!(max(&[], RuleSource::Engine), 3, "at least once");
        assert_eq!(max(&[(302, 4)], RuleSource::Engine), 3, "another target");
        assert_eq!(max(&[(300, 2)], RuleSource::Engine), 6);
        assert_eq!(max(&[(301, 2)], RuleSource::Engine), 12, "each mod counts");
        assert_eq!(max(&[(301, 2)], RuleSource::Bible), 6, "each item once");
        assert_eq!(max(&[(300, 1), (301, 1)], RuleSource::Engine), 9);
        let none = OutfitRecord {
            max: 0,
            ..target.clone()
        };
        let owned = BTreeMap::from([(OutfitId(300), 5)]);
        assert_eq!(raised_max(&none, &owned, &records, RuleSource::Engine), 0);
        let big = BTreeMap::from([(OutfitId(300), u16::MAX)]);
        let huge = OutfitRecord {
            max: i16::MAX,
            ..target
        };
        assert_eq!(
            raised_max(&huge, &big, &records, RuleSource::Engine),
            i64::from(i16::MAX) * i64::from(u16::MAX)
        );
    }
}
