//! What the exchange's dialog shows beside its prices, as
//! `_TradeDialogUpdate` (0x4ceea-0x4e092 in the `EV Nova` executable)
//! decides it: the header's "In Fleet:" or "In Hold:" ([`held_in`]), each
//! row's price marker ([`PriceMarker`]), the cargo summary under the rows
//! ([`CargoSummary`], item 12), and the `STR#` 2002 words they are told
//! in ([`TradeWords`]).
//!
//! The exchange's [`Market`](crate::market::Market) carries them; the
//! view draws them. The original keeps its counts and tons in 16 bits
//! (@0x4d6d6, @0x4d74d); they are kept wider here. Nothing here plays a
//! sound: `_TradeDialogUpdate` calls neither `PlaySound` nor `_beepSnd`.

use std::borrow::Cow;
use std::collections::BTreeMap;

use crate::catalog::{DisasterId, DisasterRecord, StellarId};
use crate::market::{ActiveEvent, JunkSide, PriceLevel, event_commodity, place};

/// The word a row is marked with, at right-70 (@0x4d3ce-0x4d521).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PriceMarker {
    /// #201 "Low": a commodity at level 1, or the `SoldAt` `jünk` row.
    Low,
    /// #202 "Med": a commodity at level 2.
    Medium,
    /// #203 "High": a commodity at level 4, or the `BoughtAt` `jünk` row.
    High,
    /// #204 "Higher": an event here raises the commodity.
    Higher,
    /// #205 "Lower": an event here lowers it.
    Lower,
}

/// A commodity row's marker (@0x4d3ec-0x4d4f2): with an event here on
/// it, of `PriceDelta` `first_event_delta` ([`first_local_delta`]),
/// "Higher" above 0, "Lower" below, and nothing at 0, with no level
/// marker either (`jns` @0x4d472); with none, its price `level`'s.
#[must_use]
pub fn commodity_marker(
    first_event_delta: Option<i64>,
    level: Option<PriceLevel>,
) -> Option<PriceMarker> {
    match first_event_delta {
        Some(delta) if delta > 0 => Some(PriceMarker::Higher),
        Some(delta) if delta < 0 => Some(PriceMarker::Lower),
        Some(_) => None,
        None => level.map(|level| match level {
            PriceLevel::Low => PriceMarker::Low,
            PriceLevel::Medium => PriceMarker::Medium,
            PriceLevel::High => PriceMarker::High,
        }),
    }
}

/// A `jünk` row's marker, by the side it was listed from
/// (@0x4d4f6-0x4d521): row 6, `BoughtAt`, "High", and row 7, `SoldAt`,
/// "Low", its price level.
#[must_use]
pub const fn junk_marker(side: JunkSide) -> PriceMarker {
    match side {
        JunkSide::BoughtAt => PriceMarker::High,
        JunkSide::SoldAt => PriceMarker::Low,
    }
}

/// The `PriceDelta` of the first event, by ID, that marks `commodity`'s
/// row at `stellar` (@0x4d3ec-0x4d43f): one under way, with days left,
/// at `stellar`, on `commodity` ([`event_commodity`]). An event with no
/// record is skipped.
#[must_use]
pub fn first_local_delta(
    events: &BTreeMap<DisasterId, ActiveEvent>,
    disasters: &[DisasterRecord],
    stellar: StellarId,
    commodity: u8,
) -> Option<i64> {
    events.iter().find_map(|(id, active)| {
        let event = disasters.iter().find(|event| event.id == *id)?;
        (active.days > 0
            && place(*active, event) == Some(stellar)
            && event_commodity(event.commodity) == Some(commodity))
        .then_some(i64::from(event.price_delta))
    })
}

/// Where the header says the held tons are (@0x4d034-0x4d07a).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HeldIn {
    /// #199 "In Hold:": the ship alone.
    Hold,
    /// #198 "In Fleet:": escorts add holds.
    Fleet,
}

/// [`HeldIn::Fleet`] when the fleet's `fleet` tons are more than the
/// ship's own `ship` (`_TotalFleetHolds` > `_ShipTotalHolds`, `jle`
/// @0x4d05f), else [`HeldIn::Hold`].
#[must_use]
pub const fn held_in(ship: u32, fleet: u32) -> HeldIn {
    if fleet > ship {
        HeldIn::Fleet
    } else {
        HeldIn::Hold
    }
}

/// The `STR#` 2002 words the exchange's dialog is told in. A missing one
/// reads empty, as `GetIndString` gives nothing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TradeWords {
    /// #1 "ton" (`_tonString`).
    pub ton: Cow<'static, str>,
    /// #2 "tons" (`_tonsString`).
    pub tons: Cow<'static, str>,
    /// #197 "Commodity:".
    pub commodity: Cow<'static, str>,
    /// #198 "In Fleet:".
    pub in_fleet: Cow<'static, str>,
    /// #199 "In Hold:".
    pub in_hold: Cow<'static, str>,
    /// #200 "Price:".
    pub price: Cow<'static, str>,
    /// #201 "Low".
    pub low: Cow<'static, str>,
    /// #202 "Med".
    pub medium: Cow<'static, str>,
    /// #203 "High".
    pub high: Cow<'static, str>,
    /// #204 "Higher".
    pub higher: Cow<'static, str>,
    /// #205 "Lower".
    pub lower: Cow<'static, str>,
    /// #363 "Other cargo:".
    pub other_cargo: Cow<'static, str>,
    /// #364 "Free cargo space".
    pub free_cargo_space: Cow<'static, str>,
    /// #365 "in your ship".
    pub in_your_ship: Cow<'static, str>,
    /// #366 "in your fleet".
    pub in_your_fleet: Cow<'static, str>,
    /// #367 "mission cargo".
    pub mission_cargo: Cow<'static, str>,
    /// #368 "miscellaneous goods".
    pub misc_goods: Cow<'static, str>,
    /// #391 "of".
    pub of: Cow<'static, str>,
    /// #392 "and".
    pub and: Cow<'static, str>,
}

impl TradeWords {
    /// Stock `STR#` 2002's.
    pub const STOCK: Self = Self {
        ton: Cow::Borrowed("ton"),
        tons: Cow::Borrowed("tons"),
        commodity: Cow::Borrowed("Commodity:"),
        in_fleet: Cow::Borrowed("In Fleet:"),
        in_hold: Cow::Borrowed("In Hold:"),
        price: Cow::Borrowed("Price:"),
        low: Cow::Borrowed("Low"),
        medium: Cow::Borrowed("Med"),
        high: Cow::Borrowed("High"),
        higher: Cow::Borrowed("Higher"),
        lower: Cow::Borrowed("Lower"),
        other_cargo: Cow::Borrowed("Other cargo:"),
        free_cargo_space: Cow::Borrowed("Free cargo space"),
        in_your_ship: Cow::Borrowed("in your ship"),
        in_your_fleet: Cow::Borrowed("in your fleet"),
        mission_cargo: Cow::Borrowed("mission cargo"),
        misc_goods: Cow::Borrowed("miscellaneous goods"),
        of: Cow::Borrowed("of"),
        and: Cow::Borrowed("and"),
    };

    /// The words of `list`, `STR#` 2002 read whole, numbered from 1.
    #[must_use]
    pub fn from_list(list: &[String]) -> Self {
        let at = |n: usize| Cow::Owned(list.get(n - 1).cloned().unwrap_or_default());
        Self {
            ton: at(1),
            tons: at(2),
            commodity: at(197),
            in_fleet: at(198),
            in_hold: at(199),
            price: at(200),
            low: at(201),
            medium: at(202),
            high: at(203),
            higher: at(204),
            lower: at(205),
            other_cargo: at(363),
            free_cargo_space: at(364),
            in_your_ship: at(365),
            in_your_fleet: at(366),
            mission_cargo: at(367),
            misc_goods: at(368),
            of: at(391),
            and: at(392),
        }
    }

    /// "ton" for exactly 1, else "tons".
    #[must_use]
    pub fn tons_for(&self, n: i64) -> &str {
        if n == 1 { &self.ton } else { &self.tons }
    }

    /// The header's label for where the held tons are.
    #[must_use]
    pub fn held_in(&self, held_in: HeldIn) -> &str {
        match held_in {
            HeldIn::Hold => &self.in_hold,
            HeldIn::Fleet => &self.in_fleet,
        }
    }

    /// `marker`'s word.
    #[must_use]
    pub fn marker(&self, marker: PriceMarker) -> &str {
        match marker {
            PriceMarker::Low => &self.low,
            PriceMarker::Medium => &self.medium,
            PriceMarker::High => &self.high,
            PriceMarker::Higher => &self.higher,
            PriceMarker::Lower => &self.lower,
        }
    }
}

impl Default for TradeWords {
    fn default() -> Self {
        Self::STOCK
    }
}

/// One mission slot's cargo, as the summary counts it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MissionLoad {
    /// Whether its cargo is aboard (+0x35).
    pub aboard: bool,
    /// Its cargo type, raw: none for -1 (+0x12).
    pub cargo_type: Option<i16>,
    /// Its tons (+0x14).
    pub tons: i64,
}

/// The cargo summary under the exchange's rows, item 12
/// (@0x4d5cb-0x4dc93): the other cargo aboard and the free space.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CargoSummary {
    /// The missions counted: aboard, with a cargo type and not below 0
    /// tons.
    pub mission_kinds: i64,
    /// Their tons.
    pub mission_tons: i64,
    /// The `jünk` held but not listed here.
    pub misc_kinds: i64,
    /// Their tons held.
    pub misc_tons: i64,
    /// The ship's own cargo space (`_ShipTotalHolds`).
    pub ship_holds: i64,
    /// The fleet's (`_TotalFleetHolds`).
    pub fleet_holds: i64,
    /// Every ton aboard (`_TotalCargo`).
    pub total_cargo: i64,
}

/// The summary of `missions`, the 16 slots' cargo, and `unlisted_junk`,
/// the tons held of each `jünk` neither of the exchange's `jünk` rows
/// names, for a ship of `ship` tons in a fleet of `fleet` with `total`
/// aboard (@0x4d68a-0x4d767). A mission counts when its cargo is aboard,
/// it has a cargo type and its tons are not below 0; a `jünk` when some
/// is held.
#[must_use]
pub fn cargo_summary(
    missions: &[MissionLoad],
    unlisted_junk: impl IntoIterator<Item = i64>,
    ship: u32,
    fleet: u32,
    total: i64,
) -> CargoSummary {
    let counted: Vec<i64> = missions
        .iter()
        .filter(|load| load.aboard && load.cargo_type.is_some() && load.tons >= 0)
        .map(|load| load.tons)
        .collect();
    let junk: Vec<i64> = unlisted_junk.into_iter().filter(|&held| held > 0).collect();
    CargoSummary {
        mission_kinds: count(&counted),
        mission_tons: counted.iter().sum(),
        misc_kinds: count(&junk),
        misc_tons: junk.iter().sum(),
        ship_holds: i64::from(ship),
        fleet_holds: i64::from(fleet),
        total_cargo: total,
    }
}

/// How many `items` there are.
fn count(items: &[i64]) -> i64 {
    i64::try_from(items.len()).unwrap_or(i64::MAX)
}

impl CargoSummary {
    /// Whether escorts add holds: the ship's are fewer than the fleet's.
    #[must_use]
    pub const fn has_fleet(&self) -> bool {
        self.ship_holds < self.fleet_holds
    }

    /// The tons free in the ship itself, never below 0 (@0x4da26-0x4da5d):
    /// with a fleet, its holds less the mission tons and the goods the
    /// escorts' holds cannot take; without one, its holds less every ton
    /// aboard.
    #[must_use]
    pub fn free_in_ship(&self) -> i64 {
        let used = if self.has_fleet() {
            let goods = self.total_cargo - self.mission_tons;
            let spilled = (goods - (self.fleet_holds - self.ship_holds)).max(0);
            self.mission_tons + spilled
        } else {
            self.total_cargo
        };
        (self.ship_holds - used).max(0)
    }

    /// The tons free in the fleet, never below 0 (@0x4db09-0x4dc2d).
    #[must_use]
    pub fn free_in_fleet(&self) -> i64 {
        (self.fleet_holds - self.total_cargo).max(0)
    }

    /// The summary's text in `words`, its lines broken by `\r`
    /// (@0x4d769-0x4dc32): "Other cargo: N tons of mission cargo and
    /// miscellaneous goods" and a blank line when there is any, then
    /// "Free cargo space[ in your ship]: N tons", and with a fleet "Free
    /// cargo space in your fleet: M tons".
    #[must_use]
    pub fn text(&self, words: &TradeWords) -> String {
        let other = if self.mission_kinds + self.misc_kinds > 0 {
            let tons = self.mission_tons + self.misc_tons;
            let count = if tons > 0 {
                format!("{tons} {} {} ", words.tons_for(tons), words.of)
            } else {
                String::new()
            };
            let kinds = match (self.mission_kinds > 0, self.misc_kinds > 0) {
                (true, true) => {
                    format!("{} {} {}", words.mission_cargo, words.and, words.misc_goods)
                }
                (true, false) => words.mission_cargo.to_string(),
                (false, _) => words.misc_goods.to_string(),
            };
            format!("{} {count}{kinds}\r\r", words.other_cargo)
        } else {
            String::new()
        };
        let free = self.free_in_ship();
        let (ship, fleet) = if self.has_fleet() {
            let in_fleet = self.free_in_fleet();
            (
                format!(" {}", words.in_your_ship),
                format!(
                    "\r{} {}: {in_fleet} {}",
                    words.free_cargo_space,
                    words.in_your_fleet,
                    words.tons_for(in_fleet)
                ),
            )
        } else {
            (String::new(), String::new())
        };
        format!(
            "{other}{}{ship}: {free} {}{fleet}",
            words.free_cargo_space,
            words.tons_for(free)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::control::Test;

    #[test]
    fn the_header_says_in_fleet_only_when_the_fleet_holds_more_than_the_ship() {
        assert_eq!(held_in(10, 15), HeldIn::Fleet);
        assert_eq!(held_in(10, 10), HeldIn::Hold);
        assert_eq!(held_in(10, 5), HeldIn::Hold);
        let words = TradeWords::STOCK;
        assert_eq!(words.held_in(HeldIn::Fleet), "In Fleet:");
        assert_eq!(words.held_in(HeldIn::Hold), "In Hold:");
    }

    #[test]
    fn an_event_marks_its_row_higher_or_lower_and_a_zero_delta_marks_nothing() {
        let low = Some(PriceLevel::Low);
        assert_eq!(commodity_marker(Some(15), low), Some(PriceMarker::Higher));
        assert_eq!(commodity_marker(Some(1), None), Some(PriceMarker::Higher));
        assert_eq!(commodity_marker(Some(-15), low), Some(PriceMarker::Lower));
        assert_eq!(commodity_marker(Some(-1), None), Some(PriceMarker::Lower));
        assert_eq!(commodity_marker(Some(0), low), None, "no level fallback");
    }

    #[test]
    fn with_no_event_the_level_marks_the_row() {
        let marked = [PriceLevel::Low, PriceLevel::Medium, PriceLevel::High]
            .map(|level| commodity_marker(None, Some(level)));
        assert_eq!(
            marked,
            [
                Some(PriceMarker::Low),
                Some(PriceMarker::Medium),
                Some(PriceMarker::High)
            ]
        );
        assert_eq!(commodity_marker(None, None), None);
    }

    #[test]
    fn a_junk_row_is_marked_by_its_side() {
        assert_eq!(junk_marker(JunkSide::BoughtAt), PriceMarker::High);
        assert_eq!(junk_marker(JunkSide::SoldAt), PriceMarker::Low);
    }

    #[test]
    fn each_marker_has_its_word() {
        let words = TradeWords::STOCK;
        let all = [
            PriceMarker::Low,
            PriceMarker::Medium,
            PriceMarker::High,
            PriceMarker::Higher,
            PriceMarker::Lower,
        ]
        .map(|marker| words.marker(marker).to_owned());
        assert_eq!(all, ["Low", "Med", "High", "Higher", "Lower"]);
    }

    const EARTH: StellarId = StellarId(140);

    fn disaster(id: i16, stellar: i16, commodity: i16, delta: i16) -> DisasterRecord {
        DisasterRecord {
            id: DisasterId(id),
            name: format!("Event {id}"),
            stellar,
            commodity,
            price_delta: delta,
            duration: 5,
            freq: 1,
            activate_on: Test::default(),
        }
    }

    fn under_way(days: u16) -> ActiveEvent {
        ActiveEvent {
            days,
            stellar: None,
        }
    }

    #[test]
    fn the_first_local_event_by_id_marks_the_row() {
        let disasters = [
            disaster(127, 140, 0, -7),
            disaster(126, 141, 0, -8),
            disaster(125, 140, 1, -9),
            disaster(128, 140, 0, 15),
            disaster(130, 140, 0, -15),
        ];
        let events = BTreeMap::from([
            (DisasterId(127), under_way(0)),
            (DisasterId(126), under_way(3)),
            (DisasterId(125), under_way(3)),
            (DisasterId(130), under_way(3)),
            (DisasterId(128), under_way(3)),
            (DisasterId(124), under_way(3)),
        ]);
        assert_eq!(first_local_delta(&events, &disasters, EARTH, 0), Some(15));
        assert_eq!(first_local_delta(&events, &disasters, EARTH, 1), Some(-9));
        assert_eq!(first_local_delta(&events, &disasters, EARTH, 2), None);
        let moved = BTreeMap::from([(
            DisasterId(126),
            ActiveEvent {
                days: 1,
                stellar: Some(EARTH),
            },
        )]);
        assert_eq!(
            first_local_delta(&moved, &disasters, EARTH, 0),
            Some(-8),
            "placed here"
        );
        let equipment = [disaster(131, 140, 9, 4)];
        let events = BTreeMap::from([(DisasterId(131), under_way(1))]);
        assert_eq!(
            first_local_delta(&events, &equipment, EARTH, 5),
            Some(4),
            "above 5 is equipment"
        );
    }

    #[test]
    fn trade_words_are_read_by_string_number() {
        let list: Vec<String> = (1..=392).map(|n| format!("#{n}")).collect();
        let words = TradeWords::from_list(&list);
        let read = [
            &words.ton,
            &words.tons,
            &words.commodity,
            &words.in_fleet,
            &words.in_hold,
            &words.price,
            &words.low,
            &words.medium,
            &words.high,
            &words.higher,
            &words.lower,
            &words.other_cargo,
            &words.free_cargo_space,
            &words.in_your_ship,
            &words.in_your_fleet,
            &words.mission_cargo,
            &words.misc_goods,
            &words.of,
            &words.and,
        ]
        .map(ToString::to_string);
        assert_eq!(
            read,
            [
                "#1", "#2", "#197", "#198", "#199", "#200", "#201", "#202", "#203", "#204", "#205",
                "#363", "#364", "#365", "#366", "#367", "#368", "#391", "#392"
            ]
        );
        let short = TradeWords::from_list(&list[..200]);
        assert_eq!((&*short.price, &*short.low), ("#200", ""));
        assert_eq!(TradeWords::default(), TradeWords::STOCK);
    }

    fn load(cargo_type: i16, tons: i64) -> MissionLoad {
        MissionLoad {
            aboard: true,
            cargo_type: Some(cargo_type),
            tons,
        }
    }

    /// The text for a 10-ton ship alone, with `total` aboard.
    fn alone(missions: &[MissionLoad], junk: &[i64], total: i64) -> String {
        cargo_summary(missions, junk.iter().copied(), 10, 10, total).text(&TradeWords::STOCK)
    }

    #[test]
    fn no_other_cargo_shows_only_the_free_space() {
        assert_eq!(alone(&[], &[], 0), "Free cargo space: 10 tons");
        assert_eq!(alone(&[], &[0], 3), "Free cargo space: 7 tons");
    }

    #[test]
    fn one_ton_of_mission_cargo() {
        assert_eq!(
            alone(&[load(0, 1)], &[], 1),
            "Other cargo: 1 ton of mission cargo\r\rFree cargo space: 9 tons"
        );
    }

    #[test]
    fn misc_goods_alone() {
        assert_eq!(
            alone(&[], &[2, 3], 5),
            "Other cargo: 5 tons of miscellaneous goods\r\rFree cargo space: 5 tons"
        );
    }

    #[test]
    fn mission_cargo_and_misc_goods() {
        assert_eq!(
            alone(&[load(0, 2), load(1, 1)], &[3], 6),
            "Other cargo: 6 tons of mission cargo and miscellaneous goods\r\r\
             Free cargo space: 4 tons"
        );
    }

    #[test]
    fn typed_missions_of_zero_tons_name_the_kind_without_a_count() {
        assert_eq!(
            alone(&[load(0, 0)], &[], 0),
            "Other cargo: mission cargo\r\rFree cargo space: 10 tons"
        );
    }

    #[test]
    fn untyped_negative_or_unloaded_missions_are_not_counted() {
        let missions = [
            MissionLoad {
                cargo_type: None,
                ..load(0, 2)
            },
            load(0, -1),
            MissionLoad {
                aboard: false,
                ..load(0, 2)
            },
        ];
        let summary = cargo_summary(&missions, [], 10, 10, 2);
        assert_eq!((summary.mission_kinds, summary.mission_tons), (0, 0));
        assert_eq!(summary.text(&TradeWords::STOCK), "Free cargo space: 8 tons");
        let counted = cargo_summary(&[load(0, 2), load(0, 3)], [4, 0, 1], 10, 10, 10);
        assert_eq!(
            (
                counted.mission_kinds,
                counted.mission_tons,
                counted.misc_kinds,
                counted.misc_tons
            ),
            (2, 5, 2, 5)
        );
    }

    #[test]
    fn a_fleet_shows_ship_and_fleet_free_space() {
        // Ship 20, fleet 50, 5 tons of missions, 40 aboard: 35 tons of
        // goods, 30 of them in the escorts, so 5 spill into the ship.
        let summary = cargo_summary(&[load(0, 5)], [], 20, 50, 40);
        assert_eq!((summary.free_in_ship(), summary.free_in_fleet()), (10, 10));
        assert_eq!(
            summary.text(&TradeWords::STOCK),
            "Other cargo: 5 tons of mission cargo\r\r\
             Free cargo space in your ship: 10 tons\r\
             Free cargo space in your fleet: 10 tons"
        );
        let unspilled = cargo_summary(&[load(0, 5)], [], 20, 50, 25);
        assert_eq!(unspilled.free_in_ship(), 15, "the escorts take the goods");
        assert_eq!(unspilled.free_in_fleet(), 25);
    }

    #[test]
    fn free_space_never_below_zero_and_one_ton_is_singular() {
        assert_eq!(alone(&[], &[], 12), "Free cargo space: 0 tons");
        assert_eq!(alone(&[], &[], 9), "Free cargo space: 1 ton");
        let over = cargo_summary(&[load(0, 30)], [], 20, 50, 60);
        assert_eq!((over.free_in_ship(), over.free_in_fleet()), (0, 0));
        let one = cargo_summary(&[], [], 20, 50, 49);
        assert_eq!(
            one.text(&TradeWords::STOCK),
            "Free cargo space in your ship: 1 ton\rFree cargo space in your fleet: 1 ton"
        );
    }
}
