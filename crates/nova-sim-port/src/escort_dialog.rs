//! The escort dialog (`DLOG` 1022) and what it marks: hailing one of the
//! player's fleet escorts opens it in place of the comm dialog
//! (`_DoCommDialog` @0x95721-0x95747, `_DoEscortDialog` @0x951ee), and
//! leaving a shipyard settles its marks (`_DoEscortLand` @0x4031f).
//!
//! **The dialog.** Its title (`_EscortDialogUpdate` @0x94b40-0x94c65) is
//! "Hired Escort:" or "Captured Escort:" (`STR#` 2002 #166, #167), the
//! class's `CommName` and its `Subtitle` when it has one. Its info
//! (@0x94cc1-0x9502d) is three lines ([`EscortDialogView`]):
//!
//! - the upgrade: "Will be upgraded at next shipyard" (#292) while it is
//!   marked, else "Upgrade Cost:" (#293) and the class's `EscUpgrdCost`,
//!   or "This ship class cannot be upgraded." (#294);
//! - the sale, only for an escort that can be sold: "Will be sold off at
//!   next shipyard" (#295) while it is marked, else "Sell Price:" (#296)
//!   and the class's `EscSellValue`;
//! - a hired escort's pay (#297, #267).
//!
//! Only a captured escort can be sold ([`can_sell`], +0xbb 0,
//! @0x952e4); one can be upgraded when its class's `UpgradeTo` is a class
//! whose `Availability` holds ([`can_upgrade`], @0x952f6-0x9533e). Both
//! are worked out as the dialog opens. Upgrade and Sell toggle their
//! mark, and a mark set clears the other ([`toggle`],
//! @0x95517-0x95589), so the two never both stand. Neither buys or sells
//! anything in flight, and neither plays a sound.
//!
//! **Leaving a shipyard** ([`settle_marks`], `_DoEscortLand`
//! @0x4035f-0x40526): every escort marked for sale is sold first, the
//! cash gaining its class's `EscSellValue`; then every escort marked for
//! an upgrade whose class has an `UpgradeTo`, in slot order, while the
//! cash covers its `EscUpgrdCost`, becomes that class and loses its
//! mark. A mark the cash can't cover stays for the next shipyard. When
//! anything was sold or upgraded, a text dialog says so
//! ([`escort_land_text`], @0x405bb-0x407e8).

use crate::catalog::{ShipId, ShipRecord};
use crate::control::Gate;
use crate::fleet::{Escort, EscortMark};
use crate::hail::ShownWage;
use crate::player_info::ordinal_number;
use crate::rulebook::RuleSource;
use crate::text_tags::comma_number;

/// `STR#` 2002 #167: a captured escort's title.
pub const CAPTURED_ESCORT: &str = "Captured Escort";
/// `STR#` 2002 #292.
pub const WILL_BE_UPGRADED: &str = "Will be upgraded at next shipyard";
/// `STR#` 2002 #293.
pub const UPGRADE_COST: &str = "Upgrade Cost:";
/// `STR#` 2002 #294.
pub const CANNOT_BE_UPGRADED: &str = "This ship class cannot be upgraded.";
/// `STR#` 2002 #295.
pub const WILL_BE_SOLD: &str = "Will be sold off at next shipyard";
/// `STR#` 2002 #296.
pub const SELL_PRICE: &str = "Sell Price:";
/// `STR#` 2002 #298: after a count of one.
pub const ESCORT_WAS: &str = "escort was";
/// `STR#` 2002 #299: after any other count.
pub const ESCORTS_WERE: &str = "escorts were";
/// `STR#` 2002 #300.
pub const SOLD_FOR: &str = "sold for a profit of";
/// `STR#` 2002 #301.
pub const UPGRADED_FOR: &str = "upgraded at a cost of";
/// `_creditCString`, `STR#` 2002 #32: the report's unit for 1 or less
/// (@0x4068f).
pub const CREDIT: &str = "credit";
/// `_creditsCString`, `STR#` 2002 #33: the report's unit above 1.
pub const CREDITS: &str = "credits";
/// `STR#` 150 #32: the Release button.
pub const RELEASE: &str = "Release";
/// `STR#` 150 #52.
pub const UPGRADE_ESCORT: &str = "Upgrade Escort";
/// `STR#` 150 #53: the Upgrade button while the upgrade mark stands.
pub const CANCEL_UPGRADE: &str = "Cancel Upgrade";
/// `STR#` 150 #54.
pub const SELL_ESCORT: &str = "Sell Escort";
/// `STR#` 150 #55: the Sell button while the sell mark stands.
pub const CANCEL_SALE: &str = "Cancel Sale";

/// Whether `escort` can be sold: only one not hired, a captured one
/// (`_canSellEscort`, +0xbb 0, @0x952e4).
#[must_use]
pub fn can_sell(escort: &Escort) -> bool {
    !escort.hired()
}

/// Whether an escort of `record`'s class can be upgraded
/// (`_canUpgradeEscort`, @0x952f6-0x9533e): its `UpgradeTo` is a class
/// `ships` has a record of, and that class's `Availability` holds through
/// `gate`. An upgrade class with no record can't be upgraded to, since
/// the engine can't fly it.
#[must_use]
pub fn can_upgrade<'a>(
    record: &ShipRecord,
    ships: impl Fn(ShipId) -> Option<&'a ShipRecord>,
    gate: &Gate<'_>,
) -> bool {
    record
        .upgrade_to
        .and_then(ships)
        .is_some_and(|upgraded| gate.allows(&upgraded.availability))
}

/// The mark after `pressed` (Upgrade or Sell) on an escort marked `mark`
/// (@0x95517-0x95589): `mark` as it is when the press is not `allowed`;
/// else the press's mark toggled, a mark set clearing the other.
#[must_use]
pub fn toggle(mark: Option<EscortMark>, pressed: EscortMark, allowed: bool) -> Option<EscortMark> {
    if !allowed {
        return mark;
    }
    (mark != Some(pressed)).then_some(pressed)
}

/// A button of the escort dialog, its item (`_EscortFilter` @0x950e9).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EscortButton {
    /// Item 1, "Close Channel": the dialog closes.
    Close,
    /// Item 2, "Release": the escort is released at once, and the dialog
    /// closes.
    Release,
    /// Item 3, "Upgrade Escort" or "Cancel Upgrade".
    Upgrade,
    /// Item 4, "Sell Escort" or "Cancel Sale".
    Sell,
}

/// The escort dialog's upgrade line (@0x94d0d-0x94e0e).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UpgradeLine {
    /// "Will be upgraded at next shipyard", in light grey.
    Marked,
    /// "Upgrade Cost:" in medium grey, then the cost in white.
    Cost(ShownWage),
    /// "This ship class cannot be upgraded.", in light grey.
    Cannot,
}

/// The escort dialog's sale line (@0x94e13-0x94f15).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SaleLine {
    /// "Will be sold off at next shipyard", in light grey.
    Marked,
    /// "Sell Price:" in medium grey, then the price in white.
    Price(ShownWage),
}

/// What the escort dialog shows of an escort (see the module docs).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EscortDialogView {
    /// Its ship class, whose picture the dialog shows (class +0xa0a).
    pub ship: ShipId,
    /// Whether it was hired: the title's "Hired Escort", else "Captured
    /// Escort".
    pub hired: bool,
    /// Its class's `CommName`, the title's second line.
    pub comm_name: String,
    /// Its class's `Subtitle`, the title's third line; none when empty.
    pub subtitle: Option<String>,
    /// The upgrade line.
    pub upgrade: UpgradeLine,
    /// The sale line; none for an escort that can't be sold.
    pub sale: Option<SaleLine>,
    /// A hired escort's pay line; none for any other.
    pub pay: Option<ShownWage>,
    /// Whether the Upgrade button is enabled.
    pub can_upgrade: bool,
    /// Whether the Sell button is enabled.
    pub can_sell: bool,
    /// Its mark, which names the buttons "Cancel Upgrade" or "Cancel
    /// Sale".
    pub mark: Option<EscortMark>,
}

impl EscortDialogView {
    /// The view of `escort`, of class `record`, which can be upgraded when
    /// `can_upgrade` says so ([`can_upgrade`]), its pay line `pay` when
    /// hired. The cost and price show as [`ShownWage::kept`]: "credit" at
    /// exactly 1, the integer test `_EscortDialogUpdate` makes
    /// (@0x94dd5).
    #[must_use]
    pub fn of(
        escort: &Escort,
        record: &ShipRecord,
        can_upgrade: bool,
        pay: Option<ShownWage>,
    ) -> Self {
        let can_sell = can_sell(escort);
        let upgrade = match (can_upgrade, escort.mark) {
            (false, _) => UpgradeLine::Cannot,
            (true, Some(EscortMark::Upgrade)) => UpgradeLine::Marked,
            (true, _) => UpgradeLine::Cost(ShownWage::kept(record.esc_upgrd_cost)),
        };
        let sale = can_sell.then(|| match escort.mark {
            Some(EscortMark::Sell) => SaleLine::Marked,
            _ => SaleLine::Price(ShownWage::kept(record.esc_sell_value)),
        });
        Self {
            ship: escort.ship,
            hired: escort.hired(),
            comm_name: record.comm_name.clone(),
            subtitle: Some(record.subtitle.clone()).filter(|subtitle| !subtitle.is_empty()),
            upgrade,
            sale,
            pay: pay.filter(|_| escort.hired()),
            can_upgrade,
            can_sell,
            mark: escort.mark,
        }
    }
}

/// What leaving a shipyard sold and upgraded.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EscortLanding {
    /// How many escorts were sold.
    pub sold: u32,
    /// What they were sold for, in all.
    pub sold_for: i64,
    /// How many escorts were upgraded.
    pub upgraded: u32,
    /// What the upgrades cost, in all.
    pub paid: i64,
}

/// The text dialog `_DoEscortLand` shows for `landing`
/// (@0x405bb-0x407e8), its numbers grouped as `millions` reads
/// `RuleKey::PayMillions`; none when nothing was sold or upgraded.
#[must_use]
pub fn escort_land_text(landing: &EscortLanding, millions: RuleSource) -> Option<String> {
    let sentence = |count: u32, verb: &str, credits: i64| {
        let unit = if credits > 1 { CREDITS } else { CREDIT };
        let was = if count == 1 { ESCORT_WAS } else { ESCORTS_WERE };
        format!(
            "{} {was} {verb} {} {unit}.",
            ordinal_number(i64::from(count), true),
            comma_number(credits, millions)
        )
    };
    let sold = (landing.sold > 0).then(|| sentence(landing.sold, SOLD_FOR, landing.sold_for));
    let upgraded =
        (landing.upgraded > 0).then(|| sentence(landing.upgraded, UPGRADED_FOR, landing.paid));
    match (sold, upgraded) {
        (Some(sold), Some(upgraded)) => Some(format!("{sold}\n\n{upgraded}")),
        (sold, upgraded) => sold.or(upgraded),
    }
}

/// What leaving a shipyard does with the fleet's marks
/// ([`settle_marks`]). Every index is the fleet's before any removal.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MarkSettlement {
    /// The escorts sold, by ascending index.
    pub sold: Vec<usize>,
    /// The escorts upgraded, each with its new class, in the order taken.
    pub upgraded: Vec<(usize, ShipId)>,
    /// The cash after both.
    pub cash: i64,
    /// The counts and totals.
    pub landing: EscortLanding,
}

/// What leaving a shipyard with `cash` does to `escorts` (see the module
/// docs), the upgrades taken in `order` (every fleet index, in slot
/// order) and each class read from `ships`. An escort whose class has no
/// record is left as it is. It changes nothing: the session applies it.
#[must_use]
pub fn settle_marks<'a>(
    escorts: &[Escort],
    order: &[usize],
    cash: i64,
    ships: impl Fn(ShipId) -> Option<&'a ShipRecord>,
) -> MarkSettlement {
    let mut settled = MarkSettlement {
        cash,
        ..MarkSettlement::default()
    };
    for (index, escort) in escorts.iter().enumerate() {
        if escort.mark != Some(EscortMark::Sell) {
            continue;
        }
        let Some(record) = ships(escort.ship) else {
            continue;
        };
        settled.cash += record.esc_sell_value;
        settled.landing.sold += 1;
        settled.landing.sold_for += record.esc_sell_value;
        settled.sold.push(index);
    }
    for &index in order {
        let Some(escort) = escorts.get(index) else {
            continue;
        };
        if escort.mark != Some(EscortMark::Upgrade) {
            continue;
        }
        let Some((record, to)) =
            ships(escort.ship).and_then(|record| Some((record, record.upgrade_to?)))
        else {
            continue;
        };
        if settled.cash < record.esc_upgrd_cost {
            continue;
        }
        settled.cash -= record.esc_upgrd_cost;
        settled.landing.upgraded += 1;
        settled.landing.paid += record.esc_upgrd_cost;
        settled.upgraded.push((index, to));
    }
    settled
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::ShipRecord;
    use crate::control::{FreshPilot, Test};
    use crate::hail::CreditUnit;
    use crate::handling::ShipFields;
    use crate::reserves::Reserves;
    use nova_sim_catalog::fixture::{AllowAll, RefuseBits, ship};

    /// A fleet escort of class `ship`, hired at `wage` (none: captured).
    fn escort(ship: i16, wage: Option<i64>) -> Escort {
        Escort {
            ship: ShipId(ship),
            reserves: Reserves::full(10.0, 10.0, 100.0),
            order: None,
            carried: false,
            wage,
            person: None,
            mark: None,
        }
    }

    /// Class `id`, upgrading to `to` for `cost`, selling for `value`.
    fn class(id: i16, to: Option<i16>, cost: i64, value: i64) -> ShipRecord {
        ShipRecord {
            comm_name: format!("class {id}"),
            subtitle: format!("Subtitle {id}"),
            upgrade_to: to.map(ShipId),
            esc_upgrd_cost: cost,
            esc_sell_value: value,
            ..ship(id, ShipFields::default())
        }
    }

    fn gate(bits: &'static dyn crate::control::ControlBits) -> Gate<'static> {
        Gate {
            control_bits: bits,
            pilot: &FreshPilot,
        }
    }

    #[test]
    fn only_an_escort_not_hired_can_be_sold() {
        assert!(can_sell(&escort(128, None)));
        assert!(!can_sell(&escort(128, Some(0))));
        assert!(!can_sell(&escort(128, Some(100))));
    }

    #[test]
    fn an_escort_can_be_upgraded_to_an_available_class_with_a_record() {
        let upgraded = ShipRecord {
            availability: Test::parse("b7"),
            ..class(129, None, 0, 0)
        };
        let ships = [class(128, Some(129), 6_000, 499), upgraded];
        let find = |id: ShipId| ships.iter().find(|record| record.id == id);
        assert!(can_upgrade(&ships[0], find, &gate(&AllowAll)));
        assert!(
            !can_upgrade(&ships[0], find, &gate(&RefuseBits(&[7]))),
            "the upgrade class's Availability fails"
        );
        assert!(
            !can_upgrade(&class(128, None, 6_000, 499), find, &gate(&AllowAll)),
            "no UpgradeTo"
        );
        assert!(
            !can_upgrade(&class(128, Some(140), 6_000, 499), find, &gate(&AllowAll)),
            "an upgrade class with no record"
        );
    }

    #[test]
    fn a_press_toggles_its_mark_and_a_mark_set_clears_the_other() {
        use EscortMark::{Sell, Upgrade};
        assert_eq!(toggle(None, Upgrade, true), Some(Upgrade));
        assert_eq!(toggle(Some(Upgrade), Upgrade, true), None);
        assert_eq!(toggle(Some(Sell), Upgrade, true), Some(Upgrade));
        assert_eq!(toggle(None, Sell, true), Some(Sell));
        assert_eq!(toggle(Some(Sell), Sell, true), None);
        assert_eq!(toggle(Some(Upgrade), Sell, true), Some(Sell));
        for mark in [None, Some(Upgrade), Some(Sell)] {
            for pressed in [Upgrade, Sell] {
                assert_eq!(toggle(mark, pressed, false), mark, "refused");
            }
        }
    }

    #[test]
    fn a_hired_escort_shows_its_title_upgrade_cost_no_sale_and_its_pay() {
        let pay = ShownWage::kept(50);
        let view = EscortDialogView::of(
            &escort(128, Some(50)),
            &class(128, Some(129), 6_000, 499),
            true,
            Some(pay),
        );
        assert_eq!(
            view,
            EscortDialogView {
                ship: ShipId(128),
                hired: true,
                comm_name: "class 128".to_owned(),
                subtitle: Some("Subtitle 128".to_owned()),
                upgrade: UpgradeLine::Cost(ShownWage::kept(6_000)),
                sale: None,
                pay: Some(pay),
                can_upgrade: true,
                can_sell: false,
                mark: None,
            }
        );
    }

    #[test]
    fn a_captured_escort_shows_its_sell_price_and_no_pay() {
        let record = ShipRecord {
            subtitle: String::new(),
            ..class(130, None, 0, 499)
        };
        let view = EscortDialogView::of(&escort(130, None), &record, false, None);
        assert!(!view.hired);
        assert_eq!(view.subtitle, None, "an empty subtitle is none");
        assert_eq!(view.upgrade, UpgradeLine::Cannot);
        assert_eq!(view.sale, Some(SaleLine::Price(ShownWage::kept(499))));
        assert_eq!(view.pay, None);
        assert!(view.can_sell);
        assert!(!view.can_upgrade);
    }

    #[test]
    fn a_marked_escort_shows_what_the_next_shipyard_will_do() {
        let record = class(128, Some(129), 6_000, 499);
        let marked = |mark| Escort {
            mark: Some(mark),
            ..escort(128, None)
        };
        let upgrading = EscortDialogView::of(&marked(EscortMark::Upgrade), &record, true, None);
        assert_eq!(upgrading.upgrade, UpgradeLine::Marked);
        assert_eq!(upgrading.sale, Some(SaleLine::Price(ShownWage::kept(499))));
        assert_eq!(upgrading.mark, Some(EscortMark::Upgrade));
        let selling = EscortDialogView::of(&marked(EscortMark::Sell), &record, true, None);
        assert_eq!(selling.upgrade, UpgradeLine::Cost(ShownWage::kept(6_000)));
        assert_eq!(selling.sale, Some(SaleLine::Marked));
        assert_eq!(
            EscortDialogView::of(&marked(EscortMark::Upgrade), &record, false, None).upgrade,
            UpgradeLine::Cannot,
            "a class that can't be upgraded says so, marked or not"
        );
    }

    #[test]
    fn a_cost_of_exactly_1_reads_credit() {
        let unit = |cost| {
            let view = EscortDialogView::of(
                &escort(128, None),
                &class(128, None, cost, cost),
                true,
                None,
            );
            match (view.upgrade, view.sale) {
                (UpgradeLine::Cost(cost), Some(SaleLine::Price(price))) => (cost.unit, price.unit),
                other => panic!("{other:?}"),
            }
        };
        assert_eq!(unit(1), (CreditUnit::Credit, CreditUnit::Credit));
        assert_eq!(unit(2), (CreditUnit::Credits, CreditUnit::Credits));
        assert_eq!(unit(0), (CreditUnit::Credits, CreditUnit::Credits));
    }

    fn text(sold: u32, sold_for: i64, upgraded: u32, paid: i64) -> Option<String> {
        escort_land_text(
            &EscortLanding {
                sold,
                sold_for,
                upgraded,
                paid,
            },
            RuleSource::Engine,
        )
    }

    #[test]
    fn the_report_counts_the_escorts_sold_and_upgraded_and_their_credits() {
        assert_eq!(
            text(1, 499, 0, 0).as_deref(),
            Some("One escort was sold for a profit of 499 credits.")
        );
        assert_eq!(
            text(11, 5_489, 0, 0).as_deref(),
            Some("11 escorts were sold for a profit of 5,489 credits.")
        );
        assert_eq!(
            text(1, 1, 0, 0).as_deref(),
            Some("One escort was sold for a profit of 1 credit.")
        );
        assert_eq!(
            text(1, 0, 0, 0).as_deref(),
            Some("One escort was sold for a profit of 0 credit."),
            "0 reads credit too (@0x4068f)"
        );
        assert_eq!(
            text(0, 0, 2, 12_000).as_deref(),
            Some("Two escorts were upgraded at a cost of 12,000 credits.")
        );
        assert_eq!(
            text(1, 499, 1, 6_000).as_deref(),
            Some(
                "One escort was sold for a profit of 499 credits.\n\nOne escort was upgraded \
                 at a cost of 6,000 credits."
            )
        );
        assert_eq!(text(0, 0, 0, 0), None);
    }

    #[test]
    fn a_million_reads_as_pay_millions_says() {
        let landing = EscortLanding {
            sold: 2,
            sold_for: 1_234_567,
            ..EscortLanding::default()
        };
        assert_eq!(
            escort_land_text(&landing, RuleSource::Engine).as_deref(),
            Some("Two escorts were sold for a profit of 1.23M credits.")
        );
        assert_eq!(
            escort_land_text(&landing, RuleSource::Bible).as_deref(),
            Some("Two escorts were sold for a profit of 1,234,567 credits.")
        );
    }

    fn marked(ship: i16, mark: Option<EscortMark>) -> Escort {
        Escort {
            mark,
            ..escort(ship, None)
        }
    }

    #[test]
    fn sales_come_first_then_upgrades_in_slot_order_while_the_cash_lasts() {
        use EscortMark::{Sell, Upgrade};
        let ships = [
            class(128, Some(129), 1_000, 400),
            class(129, Some(130), 2_000, 700),
            class(130, None, 0, 50),
        ];
        let find = |id: ShipId| ships.iter().find(|record| record.id == id);
        let escorts = [
            marked(128, Some(Upgrade)),
            marked(130, Some(Sell)),
            marked(129, Some(Upgrade)),
            marked(129, Some(Sell)),
            marked(130, Some(Upgrade)),
            marked(128, None),
        ];
        let before = escorts;
        let settled = settle_marks(&escorts, &[2, 0, 1, 3, 4, 5], 1_500, find);
        assert_eq!(settled.sold, [1, 3], "ascending, by the old indices");
        assert_eq!(
            settled.upgraded,
            [(2, ShipId(130))],
            "the sales' 750 make 2,250: index 2 first, by slot, then 0 is short; \
             130 has no UpgradeTo"
        );
        assert_eq!(settled.cash, 250);
        assert_eq!(
            settled.landing,
            EscortLanding {
                sold: 2,
                sold_for: 750,
                upgraded: 1,
                paid: 2_000,
            }
        );
        assert_eq!(escorts, before, "it changes nothing");
    }

    #[test]
    fn an_escort_of_no_record_or_unmarked_is_left_as_it_is() {
        let ships = [class(128, Some(129), 0, 400)];
        let find = |id: ShipId| ships.iter().find(|record| record.id == id);
        let escorts = [
            marked(140, Some(EscortMark::Sell)),
            marked(140, Some(EscortMark::Upgrade)),
            marked(128, None),
        ];
        let settled = settle_marks(&escorts, &[0, 1, 2], 10, find);
        assert_eq!(
            settled,
            MarkSettlement {
                cash: 10,
                ..MarkSettlement::default()
            }
        );
        let free = settle_marks(&[marked(128, Some(EscortMark::Upgrade))], &[0], 0, find);
        assert_eq!(
            free.upgraded,
            [(0, ShipId(129))],
            "a cost of 0 with no cash"
        );
    }
}
