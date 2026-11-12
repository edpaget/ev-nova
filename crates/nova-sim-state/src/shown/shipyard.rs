//! A stellar's shipyard as the player sees it ([`Shipyard`], [`ShipRow`],
//! [`ShipFacts`]), why a ship can't be bought ([`ShipRefusal`]), what the
//! Shipyard Info panel shows of a class ([`ShipSpecs`], [`StockLine`],
//! [`Rating`]), the name prompt ([`ShipNaming`], [`SHIP_NAME_CHARS`]) and
//! what a purchase did ([`ShipPurchase`]), from `nova_sim_port::shipyard`,
//! which re-exports them and keeps the shipyard's building, its buy checks
//! (`nova_sim_port::shipyard::ShipyardRules`) and the purchase.

use std::collections::BTreeMap;

use nova_sim_base::handling::ShipFields;
use nova_sim_base::reserves::shield_points;
use nova_sim_base::rulebook::{RuleSource, Rulebook};
use nova_sim_base::rules::Context;
use nova_sim_catalog::catalog::{OutfitId, ShipId};

use super::wares::ShownTons;
use crate::market::Good;

/// A rating word on the Shipyard Info panel's Accel and Turn lines:
/// `STR#` 2002 #245-250, "Excellent" to "Terrible", or "N/A" (#396) for
/// none (`_ShipDescDialogUpdate` @0x4fc18-0x4fe12).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rating {
    /// "N/A" (#396): 0 or below.
    NotApplicable,
    /// "Terrible" (#250).
    Terrible,
    /// "Poor" (#249).
    Poor,
    /// "Average" (#248).
    Average,
    /// "Good" (#247).
    Good,
    /// "Very Good" (#246).
    VeryGood,
    /// "Excellent" (#245).
    Excellent,
}

/// The ratings above N/A, lowest first, as [`rated`] steps through them.
const RATINGS: [Rating; 6] = [
    Rating::Terrible,
    Rating::Poor,
    Rating::Average,
    Rating::Good,
    Rating::VeryGood,
    Rating::Excellent,
];

/// The rating of `value` on a ladder whose step n is reached strictly
/// above `bounds[n]`: N/A at or below the first bound, then
/// [`Rating::Terrible`] to [`Rating::Excellent`].
fn rated<T: PartialOrd + Copy>(value: T, bounds: [T; 6]) -> Rating {
    bounds
        .iter()
        .zip(RATINGS)
        .filter(|(bound, _)| value > **bound)
        .map(|(_, rating)| rating)
        .last()
        .unwrap_or(Rating::NotApplicable)
}

/// The Accel ladder's doubles, which `_ShipDescDialogUpdate` compares the
/// loader's single float of `Accel` / 10000 against, lowest first
/// (@0x4fc74-0x4fd05).
pub(crate) const ACCEL_THRESHOLDS: [f64; 6] = [0.0, 0.013, 0.025, 0.038, 0.055, 0.073];

/// `Accel` as the panel reads it by the engine: `Accel` / 10000 stored as
/// a single float by the loader (`_LoadObjectData` @0x7a0b3-0x7a0c7) and
/// widened back to a double.
pub(crate) fn engine_accel(accel: i16) -> f64 {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the loader's single float, as `cvtsd2ss` stores it"
    )]
    let stored = (f64::from(accel) / 10_000.0) as f32;
    f64::from(stored)
}

/// One line of the Shipyard Info panel's stock-weapons list
/// (`_ShipDescDialogUpdate` @0x50539-0x50c6d).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StockLine {
    /// How many: the weapon's count, or for a fighter bay its
    /// `AmmoLoad`, the fighters it carries.
    pub count: i64,
    /// The name of the outfit holding it
    /// (`nova_sim_port::shipyard::shown_outfit_name`), or for a bay of the one
    /// holding its fighters; empty with no such outfit.
    pub name: String,
    /// Its `AmmoLoad` when above 0; always none for a bay.
    pub ammo: Option<i64>,
}

/// What the shipyard's Info shows of a ship class: from its record, its
/// Space and stock-weapons list worked out from its stock, and how its
/// Speed and Accel are read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShipSpecs {
    /// Its handling, reserve, cargo and mass fields.
    pub fields: ShipFields,
    /// Its `MaxGun`.
    pub max_gun: i16,
    /// Its `MaxTur`.
    pub max_tur: i16,
    /// Its `Length`, in metres.
    pub length: i16,
    /// Its `Crew`.
    pub crew: i16,
    /// The "Shipyard Info" panel's Space number, worked out when the list
    /// is built (`nova_sim_port::shipyard::space`).
    pub space: i64,
    /// How the panel reads `Speed` and `Accel`
    /// ([`RuleKey::ShipInfoRounding`](crate::RuleKey::ShipInfoRounding)), read
    /// when the list is built.
    pub rounding: RuleSource,
    /// The panel's stock-weapons list, worked out when the list is built
    /// (`nova_sim_port::shipyard::stock_lines`).
    pub weapons: Vec<StockLine>,
    /// Its `Long Name`, whole: `_LoadObjectData` copies it into
    /// `_shipyardLongNames` by `strcpy`, with no trim at ';'
    /// (@0x7b096-0x7b0ac), and `_ShipDescDialogUpdate` titles the
    /// picture variant of the panel with it (@0x4fa33-0x4fa69).
    pub long_name: String,
}

/// `value` when above 0, otherwise none: the panel's "None" (#335).
fn above_none(value: i64) -> Option<i64> {
    (value > 0).then_some(value)
}

impl ShipSpecs {
    /// The panel's "Speed:" number (`_ShipDescDialogUpdate`
    /// @0x4fb78-0x4fc13). By the engine, the loader stores `Speed` / 100
    /// as a single float (`_LoadObjectData` @0x7a0db-0x7a0ef) and the
    /// panel multiplies it by 100 and truncates (@0x4fbf0-0x4fbfe), so
    /// 190 shows 189; otherwise `Speed` as recorded
    /// ([`RuleKey::ShipInfoRounding`](crate::RuleKey::ShipInfoRounding)).
    #[must_use]
    pub fn speed_shown(&self) -> i64 {
        let speed = self.fields.speed;
        match self.rounding {
            RuleSource::Engine => {
                #[expect(
                    clippy::cast_possible_truncation,
                    reason = "the loader's single float, as `cvtsd2ss` stores it"
                )]
                let stored = (f64::from(speed) / 100.0) as f32;
                #[expect(
                    clippy::cast_possible_truncation,
                    reason = "`cvttsd2si` truncates; within i16 x 100"
                )]
                let shown = (f64::from(stored) * 100.0) as i64;
                shown
            }
            RuleSource::Bible => i64::from(speed),
        }
    }

    /// The panel's "Accel:" word (@0x4fc18-0x4fd0c). By the engine, the
    /// loader's single float of `Accel` / 10000 ([`engine_accel`]) above
    /// 0.073 is [`Rating::Excellent`], above 0.055 Very Good, 0.038 Good,
    /// 0.025 Average, 0.013 Poor and 0.0 Terrible, else N/A
    /// (@0x4fc74-0x4fd05): so 130 and 250 rate a step higher than exactly.
    /// Otherwise `Accel` itself against 730, 550, 380, 250, 130 and 0
    /// ([`RuleKey::ShipInfoRounding`](crate::RuleKey::ShipInfoRounding)).
    #[must_use]
    pub fn accel_shown(&self) -> Rating {
        let accel = self.fields.accel;
        match self.rounding {
            RuleSource::Engine => rated(engine_accel(accel), ACCEL_THRESHOLDS),
            RuleSource::Bible => rated(accel, [0, 130, 250, 380, 550, 730]),
        }
    }

    /// The panel's "Turn:" word (@0x4fd11-0x4fe12): `Maneuver` above 50 is
    /// [`Rating::Excellent`], above 40 Very Good, 30 Good, 20 Average, 10
    /// Poor and 0 Terrible, else N/A. The loader's float of `Maneuver` x
    /// 0.1 (@0x7a0f5-0x7a10b), compared as a float (@0x4fd72-0x4fe03),
    /// gives exactly these whole bounds for every `Maneuver`, so both
    /// readings agree.
    #[must_use]
    pub fn turn_shown(&self) -> Rating {
        rated(self.fields.maneuver, [0, 10, 20, 30, 40, 50])
    }

    /// The panel's "Shields:" number: [`shield_points`] of `Shield` when
    /// above 0, otherwise none, "None" (@0x4fe17-0x4fe9f).
    #[must_use]
    pub fn shields_shown(&self) -> Option<i64> {
        above_none(shield_points(self.fields.shield))
    }

    /// The panel's "Armor:" number: `Armor` when above 0, otherwise none
    /// (@0x4fea4-0x4ff2c).
    #[must_use]
    pub fn armor_shown(&self) -> Option<i64> {
        above_none(i64::from(self.fields.armor))
    }

    /// The panel's "Guns:" "Maximum of N": `MaxGun` when above 0,
    /// otherwise none (@0x4ff31-0x4fffb).
    #[must_use]
    pub fn guns_shown(&self) -> Option<i64> {
        above_none(i64::from(self.max_gun))
    }

    /// The panel's "Turrets:" "Maximum of N": `MaxTur` when above 0,
    /// otherwise none (@0x50000-0x500ca).
    #[must_use]
    pub fn turrets_shown(&self) -> Option<i64> {
        above_none(i64::from(self.max_tur))
    }

    /// The panel's "Energy:" jumps: none at `Fuel` exactly 0, otherwise
    /// `Fuel` / 100 truncated toward zero, so -150 is -1
    /// (@0x5044b-0x50534).
    #[must_use]
    pub fn jumps_shown(&self) -> Option<i64> {
        let fuel = i64::from(self.fields.fuel);
        (fuel != 0).then_some(fuel / 100)
    }

    /// The panel's "Crew:" number: `Crew` when above 0; otherwise the
    /// line is not drawn (@0x50e68-0x50f17).
    #[must_use]
    pub fn crew_shown(&self) -> Option<i64> {
        above_none(i64::from(self.crew))
    }

    /// The panel's "Space:" number with its unit: "ton" at exactly 1, else
    /// "tons" (`_ShipDescDialogUpdate` @0x50366-0x5037e).
    #[must_use]
    pub fn space_shown(&self) -> ShownTons {
        ShownTons::of(self.space)
    }

    /// The panel's "Cargo:" number with its unit: |`Holds`|, as the loader
    /// negates a negative one (`_LoadObjectData` @0x7a6ad-0x7a6b8), "ton"
    /// at exactly 1, else "tons" (`_ShipDescDialogUpdate`
    /// @0x503ea-0x50446).
    #[must_use]
    pub fn cargo_shown(&self) -> ShownTons {
        ShownTons::of(i64::from(self.fields.holds).abs())
    }

    /// The panel's "Mass:" number with its unit: `Mass`, "ton" at exactly
    /// 1, else "tons" (`_ShipDescDialogUpdate` @0x50e05-0x50e63).
    #[must_use]
    pub fn mass_shown(&self) -> ShownTons {
        ShownTons::of(i64::from(self.fields.mass))
    }
}

/// Why a ship cannot be bought.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShipRefusal {
    /// There is no shipyard: the ship is not landed, or the stellar has
    /// none.
    NoShipyard,
    /// The shipyard does not list the ship.
    NotListed,
    /// The ship is the class the player flies: `_CalcShipCanBuy` refuses
    /// it (@0x4f6f5-0x4f6fc), whatever the ship is named.
    AlreadyFlown,
    /// The ship is listed but cannot be bought here: its `Require` is not
    /// met, or its `Availability` does not hold.
    NotForSale,
    /// Its final price, at least none, is more than the cash: with the
    /// cash below 0, whatever the trade-in (`_CalcShipCanBuy`
    /// @0x4f7d8-0x4f7eb).
    CannotAfford,
}

/// One ship class in a stellar's shipyard.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShipRow {
    /// The ship class.
    pub id: ShipId,
    /// Its name.
    pub name: String,
    /// Its `ShortName`, raw.
    pub short_name: String,
    /// Its price.
    pub price: i64,
    /// What the shipyard's Info shows of it.
    pub specs: ShipSpecs,
    /// What the shipyard's rules read about the class, settled when the
    /// shipyard was built.
    pub facts: ShipFacts,
}

/// What the shipyard's rules read about one ship class, settled when the
/// shipyard is built: whatever needs the gate or the day's roll is worked
/// out then, so the rules answer from these alone.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ShipFacts {
    /// Whether it can be bought here at all: for sale here today (its tech
    /// level, the day's roll and `HideHigher` as listed), its `Require` met and
    /// its `Availability` holding (see the module docs). A relisting
    /// (`nova_sim_port::shipyard::Yard::relisted`) takes the class's tech, roll
    /// and `HideHigher` as the opening listed them and reads `Require` and
    /// `Availability` now, as `_CalcShipCanBuy` does at each draw
    /// (`_ShipyardFilter` @0x6047b, @0x60829, @0x60c48).
    pub buyable: bool,
}

/// A stellar's shipyard, as the player sees it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Shipyard {
    /// The ships listed, highest `DispWeight` first, then by ID.
    pub rows: Vec<ShipRow>,
    /// What the player's ship trades in for, whichever is bought.
    pub trade_in: i64,
    /// The player's credits.
    pub cash: i64,
    /// The ship class flown.
    pub current: ShipId,
    /// The rule set the shipyard was built by, which its questions read.
    pub rules: Rulebook,
}

impl Shipyard {
    /// `ship`'s row, if it is listed.
    #[must_use]
    pub fn row(&self, ship: ShipId) -> Option<&ShipRow> {
        self.rows.iter().find(|row| row.id == ship)
    }

    /// What buying `row`'s ship costs after the trade-in: negative when
    /// the player is paid. The purchase settles this, unclamped; the cash
    /// check and the info box read
    /// [`final_price_clamped`](Self::final_price_clamped).
    #[must_use]
    pub fn final_price(&self, row: &ShipRow) -> i64 {
        row.price.saturating_sub(self.trade_in)
    }

    /// The final price, none when the trade-in exceeds the price: what the
    /// buy's cash check (`_CalcShipCanBuy` `cmovs` @0x4f7e2) and the info
    /// box's "Final Price:" line (`_ShipyardDialogUpdate` @0x594cf-0x594d8)
    /// read. The purchase itself settles the unclamped
    /// [`final_price`](Self::final_price).
    #[must_use]
    pub fn final_price_clamped(&self, row: &ShipRow) -> i64 {
        self.final_price(row).max(0)
    }
}

impl Context for Shipyard {
    fn rulebook(&self) -> &Rulebook {
        &self.rules
    }
}

/// The longest name the original's name prompt takes, in characters
/// (`_EVTextInputDialog`'s most, 0x40 @0x5eb5d, checked @0x554b3).
pub const SHIP_NAME_CHARS: usize = 64;

/// The prompt for naming a ship about to be bought.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShipNaming {
    /// The ship class to buy.
    pub ship: ShipId,
    /// The prompt's text: `nova_sim_port::shipyard::NAME_PROMPT_LIST`
    /// #`nova_sim_port::shipyard::NAME_PROMPT_INDEX` (empty when missing), a
    /// space, the class's `LongName`, then ": ".
    pub prompt: String,
    /// The name the prompt opens with: the class's name, a space and three
    /// digits from 1 to 9.
    pub default: String,
}

/// What buying a ship did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShipPurchase {
    /// The new ship's price.
    pub price: i64,
    /// What the old ship traded in for.
    pub trade_in: i64,
    /// The persistent outfits that did not carry over, each with how many,
    /// sold back, by the other reading of
    /// [`RuleKey::PurchaseCarry`](crate::RuleKey::PurchaseCarry); empty by
    /// the engine.
    pub sold_back: BTreeMap<OutfitId, u16>,
    /// What they were sold back for, credited, by the other reading of
    /// [`RuleKey::PurchaseCarry`](crate::RuleKey::PurchaseCarry); 0 by the
    /// engine.
    pub refund: i64,
    /// The cargo that did not fit, each good with its tons, left behind.
    pub left_behind: BTreeMap<Good, u32>,
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::super::wares::TonUnit;
    use super::*;
    use nova_sim_base::fixture::FAST;

    /// The specs of a class of `space`, `holds` and `mass`.
    fn specs(space: i64, holds: i16, mass: i16) -> ShipSpecs {
        ShipSpecs {
            fields: ShipFields {
                holds,
                mass,
                ..FAST
            },
            max_gun: 0,
            max_tur: 0,
            length: 0,
            crew: 0,
            space,
            rounding: RuleSource::Engine,
            weapons: Vec::new(),
            long_name: String::new(),
        }
    }

    /// The specs of a class of `fields`, read by `rounding`.
    fn handling(fields: ShipFields, rounding: RuleSource) -> ShipSpecs {
        ShipSpecs {
            fields,
            rounding,
            ..specs(0, 0, 0)
        }
    }

    #[test]
    fn the_spec_lines_values_take_a_ton_at_exactly_one() {
        let shown = |tons, unit| ShownTons { tons, unit };
        assert_eq!(specs(1, 0, 0).space_shown(), shown(1, TonUnit::Ton));
        assert_eq!(specs(0, 0, 0).space_shown(), shown(0, TonUnit::Tons));
        assert_eq!(specs(-3, 0, 0).space_shown(), shown(-3, TonUnit::Tons));
        assert_eq!(specs(0, 1, 0).cargo_shown(), shown(1, TonUnit::Ton));
        assert_eq!(specs(0, -1, 0).cargo_shown(), shown(1, TonUnit::Ton));
        assert_eq!(specs(0, -15, 0).cargo_shown(), shown(15, TonUnit::Tons));
        assert_eq!(specs(0, 0, 1).mass_shown(), shown(1, TonUnit::Ton));
        assert_eq!(specs(0, 0, 25).mass_shown(), shown(25, TonUnit::Tons));
    }

    #[test]
    fn the_speed_shown_goes_through_single_precision_by_the_engine() {
        let shown =
            |speed, rounding| handling(ShipFields { speed, ..FAST }, rounding).speed_shown();
        for (speed, engine) in [
            (190, 189),
            (130, 129),
            (115, 114),
            (355, 354),
            (485, 484),
            (300, 300),
            (400, 400),
            (600, 600),
            (0, 0),
            (-190, -189),
        ] {
            assert_eq!(shown(speed, RuleSource::Engine), engine, "{speed}");
        }
        for speed in [190, 485, -190] {
            assert_eq!(shown(speed, RuleSource::Bible), i64::from(speed), "{speed}");
        }
    }

    #[test]
    fn the_accel_is_rated_on_the_loaders_float_by_the_engine() {
        let rated =
            |accel, rounding| handling(ShipFields { accel, ..FAST }, rounding).accel_shown();
        for (accel, rating) in [
            (-5, Rating::NotApplicable),
            (0, Rating::NotApplicable),
            (1, Rating::Terrible),
            (129, Rating::Terrible),
            (130, Rating::Poor),
            (249, Rating::Poor),
            (250, Rating::Average),
            (380, Rating::Average),
            (381, Rating::Good),
            (550, Rating::Good),
            (551, Rating::VeryGood),
            (730, Rating::VeryGood),
            (731, Rating::Excellent),
            (i16::MAX, Rating::Excellent),
        ] {
            assert_eq!(rated(accel, RuleSource::Engine), rating, "{accel}");
        }
        for (accel, rating) in [
            (0, Rating::NotApplicable),
            (1, Rating::Terrible),
            (130, Rating::Terrible),
            (131, Rating::Poor),
            (250, Rating::Poor),
            (251, Rating::Average),
            (380, Rating::Average),
            (381, Rating::Good),
            (550, Rating::Good),
            (551, Rating::VeryGood),
            (730, Rating::VeryGood),
            (731, Rating::Excellent),
        ] {
            assert_eq!(rated(accel, RuleSource::Bible), rating, "{accel}");
        }
    }

    #[test]
    fn no_accel_float_equals_a_rating_threshold() {
        // So `>` and `>=` against the engine's double thresholds above
        // 0.0 choose alike for every `Accel`; 0.0 is met by `Accel` 0.
        assert_eq!(engine_accel(0), ACCEL_THRESHOLDS[0]);
        for accel in i16::MIN..=i16::MAX {
            let float = engine_accel(accel);
            assert!(
                !ACCEL_THRESHOLDS[1..].contains(&float),
                "{accel} widens to a threshold"
            );
        }
    }

    #[test]
    fn the_turn_is_rated_by_whole_maneuver_under_both_readings() {
        for rounding in RuleSource::ALL {
            let rated = |maneuver| handling(ShipFields { maneuver, ..FAST }, rounding).turn_shown();
            for (maneuver, rating) in [
                (-3, Rating::NotApplicable),
                (0, Rating::NotApplicable),
                (1, Rating::Terrible),
                (10, Rating::Terrible),
                (11, Rating::Poor),
                (20, Rating::Poor),
                (21, Rating::Average),
                (30, Rating::Average),
                (31, Rating::Good),
                (40, Rating::Good),
                (41, Rating::VeryGood),
                (50, Rating::VeryGood),
                (51, Rating::Excellent),
            ] {
                assert_eq!(rated(maneuver), rating, "{maneuver} by {rounding:?}");
            }
        }
    }

    #[test]
    fn shields_armour_guns_and_turrets_are_none_at_0_or_below() {
        let fielded = |shield, armor| {
            handling(
                ShipFields {
                    shield,
                    armor,
                    ..FAST
                },
                RuleSource::Engine,
            )
        };
        assert_eq!(fielded(0, 0).shields_shown(), None);
        assert_eq!(fielded(30, 0).shields_shown(), Some(30));
        assert_eq!(fielded(-4, 0).shields_shown(), Some(20));
        assert_eq!(fielded(0, 0).armor_shown(), None);
        assert_eq!(fielded(0, -1).armor_shown(), None);
        assert_eq!(fielded(0, 45).armor_shown(), Some(45));
        let armed = |max_gun, max_tur| ShipSpecs {
            max_gun,
            max_tur,
            ..specs(0, 0, 0)
        };
        assert_eq!(armed(0, 0).guns_shown(), None);
        assert_eq!(armed(-1, 0).guns_shown(), None);
        assert_eq!(armed(2, 0).guns_shown(), Some(2));
        assert_eq!(armed(0, 0).turrets_shown(), None);
        assert_eq!(armed(0, 1).turrets_shown(), Some(1));
    }

    #[test]
    fn the_jumps_are_whole_hundreds_and_none_only_at_0() {
        let jumps = |fuel| handling(ShipFields { fuel, ..FAST }, RuleSource::Engine).jumps_shown();
        assert_eq!(jumps(0), None);
        assert_eq!(jumps(300), Some(3));
        assert_eq!(jumps(199), Some(1));
        assert_eq!(jumps(100), Some(1));
        assert_eq!(jumps(99), Some(0));
        assert_eq!(jumps(-50), Some(0));
        assert_eq!(jumps(-150), Some(-1));
    }

    #[test]
    fn the_crew_is_shown_only_above_0() {
        let crew = |crew| {
            ShipSpecs {
                crew,
                ..specs(0, 0, 0)
            }
            .crew_shown()
        };
        assert_eq!(crew(4), Some(4));
        assert_eq!(crew(0), None);
        assert_eq!(crew(-2), None);
    }
}
