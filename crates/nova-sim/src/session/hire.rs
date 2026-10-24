//! Hiring escorts in the bar and paying them: the session's side of
//! [`hire`](crate::hire).
//!
//! **The list.** Landed at a stellar with a bar, the session lists the
//! ships for hire ([`Session::escorts_for_hire`]) by the
//! [`hire`](crate::hire) rules, with the fee and wage its [`HireTerms`]
//! give ([`Session::with_hire_terms`], Nova's by default) and each ship's
//! `Availability` tested through its [`ControlBits`]
//! ([`Session::with_control_bits`]). Each class's roll for the day is
//! drawn by the daily step and kept across landings, or by the other
//! reading of [`RuleKey::DayRolls`](crate::RuleKey::DayRolls) drawn on the
//! caller's [`Chance`] the first time the list is built after a landing
//! and kept until the next; one not drawn yet is drawn when the list
//! asks for it. Hiring a class draws its roll again
//! when the list is next built. The rolls are never saved. The
//! outfitter's and the shipyard's `BuyRandom` rolls follow the same rule
//! ([`Session::outfitter`], [`Session::shipyard`]), buying a ship drawing
//! its class's roll again. Whether an unmet `Require` refuses a hire
//! follows the rule set's
//! [`RuleKey::HireRequire`](crate::RuleKey::HireRequire).
//!
//! **Hiring** ([`Session::hire`]) takes what the terms charge from the
//! cash and adds the ship to the fleet, an escort as a ship captured is
//! (see [`escorts`](super::escorts)), its reserves full as its class holds
//! them, no standing order, and the daily wage it was hired at
//! ([`Escort::wage`]): a change made in the spaceport, so a save is due.
//! Its NPC is put in the system at once, at the first free berth among the
//! landed system's ships, as `_SpawnEscort` (called from the bar
//! @0x5f1f1) takes the first free ship-table slot (@0x3fe8f) while the
//! system's ships stay in the table until take-off; a loaded pilot's
//! escorts not placed yet keep the berths they will take. So it keeps
//! that berth at take-off, and a mission escort set up then can come
//! before it in the formation. The berth must be below the spawn reserve
//! ([`Setup::spawn_reserve`](super::setup::Setup::spawn_reserve)), as `_SpawnEscort` passes 8 (@0x3fe73).
//! With none free, by the engine reading of
//! [`RuleKey::HireNoSlot`](crate::RuleKey::HireNoSlot), the hire still
//! takes the fee, so a save is due, and draws the class's roll again,
//! but no escort joins ([`Hired::joined`]): `_DoShipyardDialog` ignores
//! `_SpawnEscort`'s -1 (@0x5f1f1-0x5f221), and the original's escort is
//! its slot, so nothing is kept of it. By the other reading the hire is
//! refused ([`HireRefusal::NoSlot`]). A refused hire changes nothing.
//!
//! **Paying.** Each hired escort is paid its wage for each day of a jump,
//! once the date has moved on, and for one day at each take-off, whatever
//! days the stay passes
//! ([`RuleKey::StayDays`](crate::RuleKey::StayDays)), as the rule set
//! reads [`RuleKey::TakeOffPay`](crate::RuleKey::TakeOffPay);
//! the wage it is paid follows
//! [`RuleKey::EscortWage`](crate::RuleKey::EscortWage). One the cash does not cover defects,
//! its record and its NPC gone at once, so it is never placed in the next
//! system, and the flight is told how many defected
//! ([`Session::take_pay_notes`]), which makes a save due; paying alone
//! makes none, as paying a haggled price makes none. Hailing a hired
//! escort shows its pay line ([`EscortStatus`](crate::hail::EscortStatus)):
//! by the engine, as `_EscortDialogUpdate` works it out from the `Cost`
//! ([`hire::shown_wage`](crate::hire::shown_wage)), which can differ from
//! the wage paid in its unit and, for a `Cost` of -100 or below, its sign.
//!
//! The session keeps its [`HireTerms`] and [`ControlBits`] rather than
//! being passed them on each call, a deliberate departure from
//! [`BoardingRule`](crate::BoardingRule) and
//! [`Behaviour`](crate::Behaviour): see [`hire`](crate::hire).

use std::collections::BTreeSet;
use std::rc::Rc;

use super::control::Facts;
use super::{Chart, Outbox, Session, Setup, Space, Statics};
use crate::catalog::{LandingSite, ShipId, StellarId};
use crate::chance::Chance;
use crate::control::ControlBits;
use crate::hail::ShownWage;
use crate::hire::{Bar, HireList, HireRefusal, HireTerms, Hired, PayNote};
use crate::landing::StellarFlags;
use crate::pilot::{Escort, Pilot};
use crate::rulebook::{RuleKey, RuleSource};
use crate::traffic::SPAWN_RESERVE;
use crate::wares::Roll;

/// A port the session keeps, shared: equal to another when it is the
/// same one.
pub(super) struct Shared<T: ?Sized>(pub(super) Rc<T>);

impl<T: ?Sized> Clone for Shared<T> {
    fn clone(&self) -> Self {
        Self(Rc::clone(&self.0))
    }
}

impl<T: ?Sized + std::fmt::Debug> std::fmt::Debug for Shared<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl<T: ?Sized> PartialEq for Shared<T> {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl Session {
    /// This session with `terms` giving the fee and wage of a hire (see
    /// the module docs): Nova's ([`NovaHire`](crate::NovaHire)) by
    /// default.
    #[must_use]
    pub fn with_hire_terms(mut self, terms: Rc<dyn HireTerms>) -> Self {
        self.setup.hire_terms = Shared(terms);
        self
    }

    /// This session with `bits` testing a ship's `Availability` for hire
    /// and a person's `ActiveOn` against the pilot: Nova's
    /// ([`NovaBits`](crate::NovaBits)) by default.
    #[must_use]
    pub fn with_control_bits(mut self, bits: Rc<dyn ControlBits>) -> Self {
        self.setup.control_bits = Shared(bits);
        self
    }

    /// What paying the escorts did since this was last taken, in order;
    /// taking it empties the list.
    pub fn take_pay_notes(&mut self) -> Vec<PayNote> {
        self.out.take_pay_notes()
    }

    /// Pays every hired escort its wage for each of `days`, in fleet
    /// order (see [`hire`](crate::hire)): one the cash does not cover
    /// defects, a trader first taking its share of the fleet's cargo, its
    /// own holds among the fleet's, and the session resetting
    /// ([`Session::destroy_cargo_share`], `_DoEscortPayment` @0x40240),
    /// then leaving the fleet and the system at once. When any defected
    /// the flight is told how many, and a save is due.
    pub(super) fn pay_escorts(&mut self, days: u32) {
        let mut defected = 0_u32;
        for _ in 0..days {
            let mut index = 0;
            while index < self.pilot.escorts.len() {
                let escort = self.pilot.escorts[index];
                if !escort.hired() {
                    index += 1;
                    continue;
                }
                let wage = self.setup.paid_wage(&self.statics, &escort);
                if wage <= self.pilot.cash {
                    self.pilot.cash -= wage;
                    index += 1;
                    continue;
                }
                // A trader takes its share of the cargo, and the session
                // resets, while it is still in the fleet, its own holds
                // among the fleet's (@0x40225-0x40240).
                if let Some(holds) = self.statics.trader_holds(escort.ship) {
                    self.destroy_cargo_share(holds, false);
                }
                self.pilot.escorts.remove(index);
                if index < self.space.escorts.fleet.len()
                    && let Some(id) = self.space.escorts.fleet.remove(index)
                {
                    self.space.traffic.remove(id);
                }
                defected = defected.saturating_add(1);
            }
        }
        if defected > 0 {
            self.reform();
            self.out.pay_notes.push(PayNote::Defected(defected));
            self.out.save_due = true;
        }
    }

    /// The ships for hire in the bar of the stellar the ship is docked at
    /// (see the module docs), drawing on `chance` each class's roll for
    /// the day not drawn yet; `None` when it has not landed at a stellar
    /// with a bar.
    pub fn escorts_for_hire(&mut self, chance: &mut dyn Chance) -> Option<HireList> {
        let site = self.chart.bar_site(self.mode.landed())?;
        let facts = Facts {
            pilot: &self.pilot,
            ammo_outfits: &self.statics.ammo_outfits,
            armament: &self.ship.fit.armament,
        };
        let bar = Bar {
            ships: &self.statics.ships,
            site: &site,
            contributed: self.ship.contributed(&self.pilot, &self.statics),
            cash: self.pilot.cash,
            room: self.can_hire_escorts(),
            slot: self.space.hire_berth(&self.pilot, &self.setup).is_some()
                || self.setup.rules.source_for(RuleKey::HireNoSlot) == RuleSource::Engine,
            terms: &*self.setup.hire_terms.0,
            rank_mult: self.price_mults().second,
            control_bits: &*self.setup.control_bits.0,
            pilot: &facts,
            rules: self.setup.rules,
            arsenal: &self.statics.arsenal,
            outfits: &self.statics.outfits,
        };
        let rolls = &mut self.port.hire_rolls;
        Some(bar.list(|ship, percent| rolls.today(ship, Roll::Chance(percent), chance)))
    }

    /// Hires a ship of class `ship` in the bar (see the module docs), the
    /// rolls not drawn yet drawn on `chance`, and gives what the hire did.
    ///
    /// # Errors
    ///
    /// The [`HireRefusal`] that applies, and nothing changes:
    /// [`HireRefusal::NoBar`] when not landed at a stellar with a bar,
    /// and, by the other reading of [`RuleKey::HireNoSlot`],
    /// [`HireRefusal::NoSlot`] when the system has no slot for it.
    pub fn hire(&mut self, ship: ShipId, chance: &mut dyn Chance) -> Result<Hired, HireRefusal> {
        let list = self.escorts_for_hire(chance).ok_or(HireRefusal::NoBar)?;
        list.check(ship)?;
        let record = self
            .statics
            .ship_record(ship)
            .cloned()
            .ok_or(HireRefusal::NotListed)?;
        let site = self
            .chart
            .bar_site(self.mode.landed())
            .ok_or(HireRefusal::NoBar)?;
        let fee = self.setup.hire_terms.0.charge(
            &record,
            &site,
            self.price_mults().second,
            self.pilot.cash,
        );
        let wage = self.setup.hire_terms.0.wage(&record);
        let reserves = self.setup.ship_kind(&self.statics, &record).stats.full();
        let berth = self.space.hire_berth(&self.pilot, &self.setup);
        self.transact(|pilot| {
            pilot.cash -= fee;
            if berth.is_some() {
                pilot.escorts.push(Escort {
                    ship,
                    reserves,
                    order: None,
                    carried: false,
                    wage: Some(wage),
                    person: None,
                    mark: None,
                });
            }
        });
        self.port.hire_rolls.redraw(&ship);
        if let Some(berth) = berth {
            let index = self.pilot.escorts.len() - 1;
            self.space.escorts.fleet.resize(index, None);
            let placed = self.place_escort_at(index, Some(berth), None, chance);
            // `_SpawnEscort` rolls its afterburner (@0x3ff51).
            if let Some(id) = placed {
                self.roll_escort_afterburner(id, chance);
            }
            self.space.escorts.fleet.push(placed);
            self.reform();
        }
        Ok(Hired {
            ship,
            fee,
            wage,
            joined: berth.is_some(),
        })
    }
}

impl Outbox {
    /// What paying the escorts did since this was last taken, in order;
    /// taking it empties the list.
    fn take_pay_notes(&mut self) -> Vec<PayNote> {
        std::mem::take(&mut self.pay_notes)
    }
}

impl Setup {
    /// The wage hired `escort` is paid a day (see
    /// [`RuleKey::EscortWage`]): by the engine, the terms' wage of
    /// its ship type's record, or the wage it was hired at when there is
    /// none; otherwise the wage it was hired at. None for an escort not
    /// hired.
    pub(super) fn paid_wage(&self, statics: &Statics, escort: &Escort) -> i64 {
        let kept = escort.wage.unwrap_or(0);
        match self.rules.source_for(RuleKey::EscortWage) {
            RuleSource::Engine => statics
                .ship_record(escort.ship)
                .map_or(kept, |record| self.hire_terms.0.wage(record)),
            RuleSource::Bible => kept,
        }
    }

    /// The pay line hailing hired `escort` shows (see
    /// [`RuleKey::EscortWage`]): by the engine, worked out from its ship
    /// type's record as `_EscortDialogUpdate` draws it
    /// ([`hire::shown_wage`](crate::hire::shown_wage)); otherwise, or with
    /// no record, the wage it was hired at.
    pub(super) fn shown_wage(&self, statics: &Statics, escort: &Escort) -> ShownWage {
        let kept = ShownWage::kept(escort.wage.unwrap_or(0));
        match self.rules.source_for(RuleKey::EscortWage) {
            RuleSource::Engine => statics
                .ship_record(escort.ship)
                .map_or(kept, crate::hire::shown_wage),
            RuleSource::Bible => kept,
        }
    }
}

impl Space {
    /// The berth a ship hired now takes: the first free one below the
    /// spawn reserve ([`Setup::spawn_reserve`](super::setup::Setup::spawn_reserve)), as `_SpawnEscort`
    /// passes 8 (@0x3fe73), skipping the berths a loaded pilot's escorts
    /// not placed yet will take, which the original's table already
    /// holds; `None` when there is none.
    fn hire_berth(&self, pilot: &Pilot, setup: &Setup) -> Option<u32> {
        let berths = self.fleet_berths(pilot);
        let unplaced: BTreeSet<u32> = berths
            .iter()
            .enumerate()
            .filter(|&(index, _)| self.escorts.fleet.get(index).copied().flatten().is_none())
            .map(|(_, &berth)| berth)
            .collect();
        self.traffic
            .free_berth_skipping(setup.spawn_reserve(SPAWN_RESERVE), &unplaced)
    }
}

impl Chart {
    /// The stellar the ship is docked at, if it has a bar.
    pub(super) fn bar_site(&self, landed: Option<StellarId>) -> Option<LandingSite> {
        let stellar = landed?;
        self.sites
            .iter()
            .find(|site| site.id == stellar)
            .filter(|site| site.flags & StellarFlags::BAR != 0)
            .copied()
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::ai::{Goal, Peaceful};
    use crate::board::MAX_ESCORTS;
    use crate::catalog::{LandingSite, ShipRecord, StellarId, SystemId};
    use crate::chance::NeverFires;
    use crate::combat::ShipRef;
    use crate::control::{PilotFacts, Test, TestExpr};
    use crate::escort::{EscortCommand, EscortDuty, EscortGroup, slot_position};
    use crate::hail::{CreditUnit, HailOptions, ShownWage};
    use crate::hire::{HireRow, NovaHire, PayNote};
    use crate::reserves::Reserves;
    use crate::rulebook::Rulebook;
    use crate::session::mission_ships::tests::crowd;
    use crate::stats::HYPERSPACE_DAYS;
    use crate::stats::ShipStats;
    use crate::targeting::TargetPick;
    use crate::testkit::{
        Draws, FAST, FakePilotCatalog, Scripted, bar, catalog, jump, land_now, planet, ship,
    };
    use crate::traffic::npc::AiType;

    /// The bar's stellar, 128, at the centre, under the ship as it
    /// starts.
    const BAR_AT: StellarId = StellarId(128);

    /// [`catalog`] with planet 128 a bar of tech level 7 and these ships
    /// besides the player's own (ship 128, never for hire).
    fn barred(ships: Vec<ShipRecord>) -> FakePilotCatalog {
        let mut records = vec![ship(128, FAST)];
        records.extend(ships);
        FakePilotCatalog {
            sites: vec![
                (SystemId(130), vec![bar(), planet(129, 2000.0, 0.0)]),
                (SystemId(131), vec![planet(140, 0.0, 0.0)]),
            ],
            ship_records: records,
            ..catalog()
        }
    }

    /// Ship `id` at 10,000 credits, tech level 1, for hire `hire_random`
    /// % of days.
    fn hireable(id: i16, hire_random: i16) -> ShipRecord {
        ShipRecord {
            hire_random,
            ..ship(id, FAST)
        }
    }

    /// A session of `catalog`'s with `cash` credits, landed at the bar.
    fn landed(catalog: &FakePilotCatalog, cash: i64) -> Session {
        let mut session = Session::start(catalog).expect("starts");
        session.pilot.cash = cash;
        assert_eq!(land_now(&mut session), Ok(BAR_AT));
        session.take_save_due();
        session
    }

    fn listed(list: &HireList) -> Vec<i16> {
        list.rows.iter().map(|row| row.id.0).collect()
    }

    fn list(session: &mut Session) -> HireList {
        session.escorts_for_hire(&mut NeverFires).expect("a bar")
    }

    fn row(list: &HireList, id: i16) -> &HireRow {
        list.row(ShipId(id)).expect("listed")
    }

    /// Terms of a fee of `fee`, a charge of `charge` and a wage of
    /// `wage` for every ship.
    #[derive(Debug)]
    struct FakeTerms {
        fee: i64,
        charge: i64,
        wage: i64,
    }

    impl HireTerms for FakeTerms {
        fn fee(&self, _ship: &ShipRecord, _site: &LandingSite, _mult: f32) -> i64 {
            self.fee
        }

        fn charge(&self, _ship: &ShipRecord, _site: &LandingSite, _mult: f32, _cash: i64) -> i64 {
            self.charge
        }

        fn wage(&self, _ship: &ShipRecord) -> i64 {
            self.wage
        }
    }

    /// Control bits refusing every test that reads one of its bits.
    #[derive(Debug)]
    struct Refusing(&'static [u16]);

    impl ControlBits for Refusing {
        fn allows(&self, test: &TestExpr, _pilot: &dyn PilotFacts) -> bool {
            !test.reads().iter().any(|bit| self.0.contains(&bit.get()))
        }
    }

    // The list.

    #[test]
    fn the_bar_offers_a_ship_for_hire_with_its_fee_and_daily_pay() {
        let catalog = barred(vec![hireable(129, 100)]);
        let mut session = landed(&catalog, 25_000);
        let list = list(&mut session);
        assert_eq!(listed(&list), [129], "not the player's own, never for hire");
        let record = hireable(129, 100);
        let nova = NovaHire::default();
        assert_eq!(
            list.rows[0],
            HireRow {
                id: ShipId(129),
                name: "Ship 129".to_owned(),
                short_name: "Ship\\n129".to_owned(),
                fee: nova.fee(&record, &bar(), 1.0),
                wage: nova.wage(&record),
                specs: crate::shipyard::ShipSpecs {
                    fields: FAST,
                    max_gun: 2,
                    max_tur: 1,
                    length: 20,
                    crew: 3,
                    space: 30,
                    rounding: RuleSource::Engine,
                    weapons: Vec::new(),
                    long_name: "The Ship 129".to_owned(),
                },
                hire: Ok(()),
            }
        );
        assert_eq!((list.rows[0].fee, list.rows[0].wage), (1000, 100));
        assert_eq!(list.cash, 25_000);
        assert!(list.room);
        assert!(!session.take_save_due(), "listing changes nothing");
    }

    #[test]
    fn the_bar_walks_each_space_over_the_sessions_stock() {
        // `FreeMass` -10 and a stock gun of 6 tons: -4 to start, and the
        // gun leaves 0.
        let mut catalog = barred(vec![ShipRecord {
            fields: crate::handling::ShipFields {
                free_mass: -10,
                ..FAST
            },
            ..hireable(129, 100)
        }]);
        catalog.outfits.push(crate::catalog::OutfitRecord {
            mass: 6,
            ..crate::testkit::outfit(310, &[(crate::combat::armament::MOD_WEAPON, 130)])
        });
        catalog.hulls.push(crate::catalog::HullRecord {
            weapons: vec![crate::catalog::StockWeapon {
                weapon: crate::catalog::WeaponId(130),
                count: 1,
                ammo: 0,
            }],
            ..crate::testkit::hull(129)
        });
        let mut session = landed(&catalog, 25_000);
        assert_eq!(
            row(&list(&mut session), 129).specs.space,
            0,
            "not its FreeMass of -10"
        );
    }

    #[test]
    fn the_fee_and_wage_are_the_hire_terms() {
        let catalog = barred(vec![hireable(129, 100)]);
        let mut session = landed(&catalog, 25_000).with_hire_terms(Rc::new(FakeTerms {
            fee: 7,
            charge: 9,
            wage: 3,
        }));
        let list = list(&mut session);
        assert_eq!((row(&list, 129).fee, row(&list, 129).wage), (7, 3));
    }

    #[test]
    fn a_ship_is_listed_when_its_roll_for_the_day_holds() {
        let catalog = barred(vec![hireable(129, 50)]);
        let mut session = landed(&catalog, 25_000);
        let mut fails = Scripted::answering(&[false]);
        let list = session.escorts_for_hire(&mut fails).expect("a bar");
        assert_eq!(listed(&list), Vec::<i16>::new());
        assert_eq!(fails.asked, [50]);

        let mut session = landed(&catalog, 25_000);
        let mut holds = Scripted::answering(&[true, false]);
        let list = session.escorts_for_hire(&mut holds).expect("a bar");
        assert_eq!(listed(&list), [129]);
        let again = session.escorts_for_hire(&mut holds).expect("a bar");
        assert_eq!(listed(&again), [129], "the roll is kept");
        assert_eq!(holds.asked, [50], "drawn once");
    }

    /// The original redraws the hire rolls only in its daily step
    /// (`_IncrementGameTime` @0xb7c4-0xb833), and no landing path writes
    /// them: the take-off's day draws the roll, and the landing keeps it.
    #[test]
    fn each_day_draws_the_rolls_again() {
        let catalog = barred(vec![hireable(129, 50)]);
        let mut session = landed(&catalog, 25_000);
        let mut listing = Scripted::answering(&[true]);
        assert_eq!(
            listed(&session.escorts_for_hire(&mut listing).expect("a bar")),
            [129]
        );
        let mut day = Scripted::answering(&[false]);
        session.take_off(&catalog, &mut day).expect("takes off");
        assert_eq!(day.asked, [50], "the take-off's day draws it");
        assert_eq!(land_now(&mut session), Ok(BAR_AT));
        let list = session.escorts_for_hire(&mut listing).expect("a bar");
        assert_eq!(listed(&list), Vec::<i16>::new(), "the day's roll");
        assert_eq!(listing.asked, [50], "the landing keeps it");
    }

    #[test]
    fn a_landing_with_no_day_keeps_the_hire_rolls() {
        use crate::rulebook::{RuleKey, RuleSource};
        let catalog = barred(vec![hireable(129, 50)]);
        let rules = Rulebook::default().with_override(RuleKey::StayDays, RuleSource::Bible);
        let mut session = landed(&catalog, 25_000).with_rules(rules);
        let mut chance = Scripted::answering(&[true, false]);
        assert_eq!(
            listed(&session.escorts_for_hire(&mut chance).expect("a bar")),
            [129]
        );
        session.take_off(&catalog, &mut chance).expect("takes off");
        assert_eq!(land_now(&mut session), Ok(BAR_AT));
        let list = session.escorts_for_hire(&mut chance).expect("a bar");
        assert_eq!(listed(&list), [129], "kept");
        assert_eq!(chance.asked, [50]);
    }

    #[test]
    fn by_the_other_reading_each_landing_draws_the_rolls_again() {
        use crate::rulebook::{RuleKey, RuleSource};
        let catalog = barred(vec![hireable(129, 50)]);
        let rules = Rulebook::default().with_override(RuleKey::DayRolls, RuleSource::Bible);
        let mut session = landed(&catalog, 25_000).with_rules(rules);
        let mut chance = Scripted::answering(&[true, false]);
        assert_eq!(
            listed(&session.escorts_for_hire(&mut chance).expect("a bar")),
            [129]
        );
        session
            .take_off(&catalog, &mut NeverFires)
            .expect("takes off");
        assert_eq!(session.escorts_for_hire(&mut chance), None, "in flight");
        assert_eq!(land_now(&mut session), Ok(BAR_AT));
        let list = session.escorts_for_hire(&mut chance).expect("a bar");
        assert_eq!(listed(&list), Vec::<i16>::new());
        assert_eq!(chance.asked, [50, 50]);
    }

    #[test]
    fn a_hire_random_of_none_is_never_listed_and_of_100_always_with_no_draw() {
        let catalog = barred(vec![
            hireable(129, 0),
            hireable(130, -1),
            hireable(131, 100),
            hireable(132, 250),
            hireable(133, 1),
            hireable(134, 99),
        ]);
        let mut session = landed(&catalog, 25_000);
        let mut chance = Scripted::answering(&[true, true]);
        let list = session.escorts_for_hire(&mut chance).expect("a bar");
        assert_eq!(listed(&list), [131, 132, 133, 134]);
        assert_eq!(chance.asked, [1, 99], "by ascending ID");
    }

    #[test]
    fn a_ship_above_the_stellars_tech_level_is_not_listed_and_draws_nothing() {
        let tech = |id, tech_level| ShipRecord {
            tech_level,
            ..hireable(id, 50)
        };
        let catalog = barred(vec![
            tech(129, 8),
            tech(130, 7),
            tech(131, -1),
            tech(132, 0),
        ]);
        let mut session = landed(&catalog, 25_000);
        let mut chance = Scripted::answering(&[true, true]);
        let list = session.escorts_for_hire(&mut chance).expect("a bar");
        assert_eq!(listed(&list), [130, 132], "a negative tech level never");
        assert_eq!(chance.asked, [50, 50]);

        let mut special = barred(vec![tech(129, 8), tech(131, -1)]);
        let mut site = bar();
        site.special_tech[0] = 8;
        site.special_tech[1] = -1;
        special.sites[0].1[0] = site;
        let mut session = landed(&special, 25_000);
        let list = session.escorts_for_hire(&mut Scripted::answering(&[true, true]));
        assert_eq!(listed(&list.expect("a bar")), [129], "its special tech");
    }

    #[test]
    fn a_special_tech_slot_of_0_lists_a_ship_of_tech_0_for_hire() {
        let tech = |id, tech_level| ShipRecord {
            tech_level,
            ..hireable(id, 100)
        };
        let mut catalog = barred(vec![tech(129, 0), tech(130, -1), tech(131, 1)]);
        let mut site = bar();
        site.tech_level = -5;
        site.special_tech[1] = -1;
        catalog.sites[0].1[0] = site;
        let mut session = landed(&catalog, 25_000);
        let list = session.escorts_for_hire(&mut NeverFires).expect("a bar");
        assert_eq!(listed(&list), [129], "slot 0 lists tech 0; -1 never");
    }

    #[test]
    fn a_ship_whose_availability_does_not_hold_is_hidden_or_refused() {
        let gated = |id, flags3| ShipRecord {
            availability: Test::parse("b99"),
            flags3,
            ..hireable(id, 100)
        };
        let catalog = barred(vec![gated(129, 0x0100), gated(130, 0), hireable(131, 100)]);
        let mut session = landed(&catalog, 25_000).with_control_bits(Rc::new(Refusing(&[99])));
        let list = list(&mut session);
        assert_eq!(listed(&list), [130, 131]);
        assert_eq!(row(&list, 130).hire, Err(HireRefusal::NotForHire));
        assert_eq!(row(&list, 131).hire, Ok(()));
    }

    #[test]
    fn through_the_real_control_bits_a_ships_availability_follows_its_bit() {
        let gated = |id, flags3| ShipRecord {
            availability: Test::parse("b99"),
            flags3,
            ..hireable(id, 100)
        };
        let catalog = barred(vec![gated(129, 0x0100), gated(130, 0), hireable(131, 100)]);
        let mut session = landed(&catalog, 25_000);
        let bit = crate::control::Bit::new(99).expect("in range");
        let clear = list(&mut session);
        assert_eq!(listed(&clear), [130, 131], "bit 99 clear");
        assert_eq!(row(&clear, 130).hire, Err(HireRefusal::NotForHire));
        session.set_control_bit(bit, true);
        let set = list_of(&mut session);
        assert_eq!(listed(&set), [129, 130, 131], "bit 99 set");
        assert_eq!(row(&set, 130).hire, Ok(()));
        session.set_control_bit(bit, false);
        assert_eq!(listed(&list_of(&mut session)), [130, 131], "clear again");
    }

    #[test]
    fn a_ship_whose_availability_is_malformed_is_never_for_hire() {
        let malformed = |id, flags3| ShipRecord {
            availability: Test::parse("b1 &"),
            flags3,
            ..hireable(id, 100)
        };
        let catalog = barred(vec![malformed(129, 0x0100), malformed(130, 0)]);
        let mut session =
            landed(&catalog, 25_000).with_control_bits(Rc::new(crate::testkit::AllowAll));
        let list = list(&mut session);
        assert_eq!(listed(&list), [130], "hidden when flagged");
        assert_eq!(row(&list, 130).hire, Err(HireRefusal::NotForHire));
    }

    fn list_of(session: &mut Session) -> HireList {
        list(session)
    }

    /// The list for a pilot whose ship contributes 0x1, of ships 129
    /// requiring 0x1000, 130 likewise hidden while it is unmet, and 131
    /// requiring 0x1, by `source`.
    fn required(source: RuleSource) -> HireList {
        let requiring = |id, require, flags3| ShipRecord {
            require,
            flags3,
            ..hireable(id, 100)
        };
        let catalog = barred(vec![
            requiring(129, 0x1000, 0),
            requiring(130, 0x1000, 0x0200),
            requiring(131, 0x1, 0x0200),
        ]);
        let mut session = landed(&catalog, 25_000)
            .with_rules(Rulebook::default().with_override(RuleKey::HireRequire, source));
        assert_eq!(session.rules().source_for(RuleKey::HireRequire), source);
        list(&mut session)
    }

    #[test]
    fn by_the_engine_an_unmet_require_only_hides_a_ship_flagged_so() {
        let list = required(RuleSource::Engine);
        assert_eq!(listed(&list), [129, 131]);
        assert_eq!(row(&list, 129).hire, Ok(()));
        assert_eq!(
            Session::start(&catalog())
                .expect("starts")
                .rules()
                .source_for(RuleKey::HireRequire),
            RuleSource::Engine,
            "by default"
        );
    }

    #[test]
    fn by_the_other_reading_an_unmet_require_refuses_a_hire() {
        let list = required(RuleSource::Bible);
        assert_eq!(listed(&list), [129, 131]);
        assert_eq!(row(&list, 129).hire, Err(HireRefusal::NotForHire));
        assert_eq!(row(&list, 131).hire, Ok(()));
    }

    #[test]
    fn a_ship_hiding_higher_ones_takes_those_of_its_weight_off_the_list() {
        let weighted = |id, disp_weight, flags3| ShipRecord {
            flags3,
            disp_weight,
            ..hireable(id, 100)
        };
        let catalog = barred(vec![
            weighted(129, 5, 0),
            weighted(130, 5, 0x4000),
            weighted(131, 5, 0),
            weighted(132, 6, 0),
            weighted(133, 5, 0x4000),
        ]);
        let mut session = landed(&catalog, 25_000);
        assert_eq!(
            listed(&list(&mut session)),
            [132, 129, 130],
            "by DispWeight, highest first, then by ID"
        );
    }

    #[test]
    fn a_ship_whose_fee_the_cash_does_not_cover_is_refused() {
        let catalog = barred(vec![hireable(129, 100)]);
        let mut session = landed(&catalog, 999);
        assert_eq!(
            row(&list(&mut session), 129).hire,
            Err(HireRefusal::CannotAfford)
        );
        let mut session = landed(&catalog, 1000);
        assert_eq!(row(&list(&mut session), 129).hire, Ok(()));
    }

    /// An escort of ship 129, `carried` or not, not hired.
    fn escort(carried: bool) -> Escort {
        Escort {
            ship: ShipId(129),
            reserves: Reserves::full(30.0, 45.0, 300.0),
            order: None,
            carried,
            wage: None,
            person: None,
            mark: None,
        }
    }

    #[test]
    fn a_full_fleet_has_no_room_and_its_fighters_do_not_count() {
        let catalog = barred(vec![hireable(129, 100), hireable(130, 100)]);
        let mut session = landed(&catalog, 25_000);
        session.pilot.escorts = vec![escort(false); MAX_ESCORTS];
        let full = list(&mut session);
        assert!(!full.room);
        assert!(
            full.rows
                .iter()
                .all(|row| row.hire == Err(HireRefusal::FleetFull))
        );
        let mut session = landed(&catalog, 25_000);
        session.pilot.escorts = vec![escort(false); MAX_ESCORTS - 1];
        session.pilot.escorts.extend([escort(true), escort(true)]);
        let list = list(&mut session);
        assert!(list.room);
        assert_eq!(row(&list, 129).hire, Ok(()));
    }

    /// `_CanHireEscorts` counts the escorts in the system (see the
    /// session's `capture` module): seven on file with two not placed
    /// leave room, and a sixth placed fills the fleet; the save keeps all
    /// seven.
    #[test]
    fn the_room_counts_the_escorts_in_the_system_so_a_fleet_can_pass_six() {
        let catalog = barred(vec![hireable(129, 100)]);
        let mut session = landed(&catalog, 25_000);
        session.pilot.escorts = vec![escort(false); MAX_ESCORTS + 1];
        session.space.escorts.fleet = (0..session.pilot.escorts.len())
            .map(|index| {
                (index < MAX_ESCORTS - 1)
                    .then(|| session.place_escort(index, &mut crate::chance::NeverFires))
                    .flatten()
            })
            .collect();
        assert!(list(&mut session).room, "five of the seven in the system");
        assert_eq!(row(&list(&mut session), 129).hire, Ok(()));
        session.space.escorts.fleet[MAX_ESCORTS - 1] =
            session.place_escort(MAX_ESCORTS - 1, &mut crate::chance::NeverFires);
        assert!(!list(&mut session).room, "six in the system");
        let saved = crate::save::decode(&crate::save::encode(session.pilot())).expect("reads");
        assert_eq!(saved.escorts().len(), MAX_ESCORTS + 1);
    }

    #[test]
    fn there_is_no_bar_in_flight_or_where_the_stellar_has_none() {
        let catalog = barred(vec![hireable(129, 100)]);
        let mut session = Session::start(&catalog).expect("starts");
        assert_eq!(session.escorts_for_hire(&mut NeverFires), None);
        let mut barless = barred(vec![hireable(129, 100)]);
        barless.sites[0].1[0].flags = StellarFlags::CAN_LAND | StellarFlags::SHIPYARD;
        let mut session = landed(&barless, 25_000);
        assert_eq!(session.escorts_for_hire(&mut NeverFires), None);
    }

    // Hiring.

    #[test]
    fn hiring_takes_the_fee_and_adds_the_ship_to_the_fleet_with_its_wage() {
        let catalog = barred(vec![hireable(129, 100)]);
        let mut session = landed(&catalog, 25_000);
        assert_eq!(
            session.hire(ShipId(129), &mut NeverFires),
            Ok(Hired {
                ship: ShipId(129),
                fee: 1000,
                wage: 100,
                joined: true,
            })
        );
        assert_eq!(session.pilot().cash(), 24_000);
        assert_eq!(
            session.pilot().escorts(),
            [Escort {
                ship: ShipId(129),
                reserves: ShipStats::new(FAST, &[]).full(),
                order: None,
                carried: false,
                wage: Some(100),
                person: None,
                mark: None,
            }]
        );
        assert!(session.take_save_due());
    }

    #[test]
    fn a_hired_escort_comes_with_its_classs_reserves_alone_by_the_engine() {
        // Ship 129 carries a tank (+100 fuel) by default.
        let mut catalog = barred(vec![ShipRecord {
            defaults: vec![(crate::catalog::OutfitId(330), 1)],
            ..hireable(129, 100)
        }]);
        catalog.outfits.push(crate::testkit::outfit(
            330,
            &[(crate::stats::MORE_FUEL, 100)],
        ));
        for (items, fuel) in [(RuleSource::Engine, 300.0), (RuleSource::Bible, 400.0)] {
            let mut session = landed(&catalog, 25_000)
                .with_rules(Rulebook::default().with_override(RuleKey::NpcDefaultItems, items));
            assert!(session.hire(ShipId(129), &mut NeverFires).is_ok());
            let reserves = session.pilot().escorts()[0].reserves;
            assert_eq!(
                (reserves.fuel.now, reserves.fuel.max),
                (fuel, fuel),
                "{items:?}"
            );
        }
    }

    #[test]
    fn a_hired_escort_rolls_its_skill_and_flies_of_no_government() {
        // `SkillVar` 37: 75 sides, the first 0.63.
        let catalog = barred(vec![ShipRecord {
            skill_var: 37,
            ..hireable(129, 100)
        }]);
        let mut session = landed(&catalog, 25_000);
        let mut chance = Draws::of(&[]).keyed(75, 0);
        assert!(session.hire(ShipId(129), &mut chance).is_ok());
        assert_eq!(chance.times_asked(75), 1);
        let id = session.space.escorts.fleet[0].expect("placed");
        let npc = session.space.npc(id).expect("in the system");
        assert_eq!((npc.skill, npc.skill_mult), ((0.63_f64) as f32, 1.0));
    }

    #[test]
    fn a_hired_escort_spawns_glowing_its_anim_owed_on_the_first_class() {
        use crate::traffic::ship_display::{AnimRoll, GLOW_SPAWN};
        let catalog = barred(vec![hireable(129, 100)]);
        let mut session = landed(&catalog, 25_000);
        assert!(session.hire(ShipId(129), &mut NeverFires).is_ok());
        let id = session.space.escorts.fleet[0].expect("placed");
        let npc = session.space.npc(id).expect("in the system");
        assert_eq!(npc.display.glow, GLOW_SPAWN, "`_SpawnEscort`'s spawn");
        assert_eq!(npc.display.rolls.anim, AnimRoll::FirstClass);
    }

    #[test]
    fn a_hire_takes_what_the_terms_charge_not_the_fee() {
        let catalog = barred(vec![hireable(129, 100)]);
        let mut session = landed(&catalog, 25_000).with_hire_terms(Rc::new(FakeTerms {
            fee: 1000,
            charge: 7,
            wage: 3,
        }));
        let hired = session.hire(ShipId(129), &mut NeverFires).expect("hires");
        assert_eq!((hired.fee, hired.wage), (7, 3));
        assert_eq!(session.pilot().cash(), 24_993);
        assert_eq!(session.pilot().escorts()[0].wage, Some(3));
    }

    /// `_DoBarDialog` resets as the hire screen closes (@0x49235), after
    /// a hire or none; the hire itself (`_DoShipyardDialog` in hire mode,
    /// @0x5f14a-0x5f221) resets nothing of its own.
    #[test]
    fn the_hire_screen_closing_resets_and_a_hire_alone_does_not() {
        use crate::scan::Latches;
        use crate::session::precalced::tests::{convicted, overloaded};
        let catalog = barred(vec![hireable(129, 100)]);
        let mut session = landed(&catalog, 25_000);
        convicted(&mut session);
        session.hire(ShipId(129), &mut NeverFires).expect("hires");
        assert_ne!(
            session.ship.latches.scan_latches,
            Latches::ARMED,
            "the hire alone"
        );
        let trimmed = overloaded(&mut session);
        session.close_hire();
        assert_eq!(
            session.ship.latches.scan_latches,
            Latches::ARMED,
            "@0x49235"
        );
        assert_eq!(
            session
                .pilot()
                .cargo()
                .collect::<std::collections::BTreeMap<_, _>>(),
            trimmed
        );
    }

    #[test]
    fn hiring_a_class_draws_its_roll_again() {
        let catalog = barred(vec![hireable(129, 50), hireable(130, 50)]);
        let mut session = landed(&catalog, 25_000);
        let mut chance = Scripted::answering(&[true, true, false]);
        session.hire(ShipId(129), &mut chance).expect("hires");
        let list = session.escorts_for_hire(&mut chance).expect("a bar");
        assert_eq!(listed(&list), [130]);
        assert_eq!(chance.asked, [50, 50, 50], "129, 130, then 129 again");
    }

    #[test]
    fn a_refused_hire_changes_nothing() {
        let gated = ShipRecord {
            availability: Test::parse("b99"),
            ..hireable(130, 100)
        };
        let catalog = barred(vec![hireable(129, 100), gated]);
        let refused = |session: &mut Session, ship: i16, refusal: HireRefusal| {
            let before = session.pilot().clone();
            assert_eq!(session.hire(ShipId(ship), &mut NeverFires), Err(refusal));
            assert_eq!(session.pilot(), &before);
            assert!(!session.take_save_due(), "{refusal:?}");
        };
        let mut flying = Session::start(&catalog).expect("starts");
        refused(&mut flying, 129, HireRefusal::NoBar);
        let mut session = landed(&catalog, 25_000).with_control_bits(Rc::new(Refusing(&[99])));
        refused(&mut session, 999, HireRefusal::NotListed);
        refused(&mut session, 128, HireRefusal::NotListed);
        refused(&mut session, 130, HireRefusal::NotForHire);
        let mut poor = landed(&catalog, 999);
        refused(&mut poor, 129, HireRefusal::CannotAfford);
        let mut full = landed(&catalog, 25_000);
        full.pilot.escorts = vec![escort(false); MAX_ESCORTS];
        refused(&mut full, 129, HireRefusal::FleetFull);
    }

    // The slot cap: `_SpawnEscort`'s reserve.

    #[test]
    fn a_hire_with_no_slot_takes_the_fee_and_redraws_the_roll_but_no_escort_joins() {
        let catalog = barred(vec![hireable(129, 50), hireable(130, 50)]);
        let mut session = landed(&catalog, 25_000);
        crowd(&mut session, 55);
        let npcs = session.npcs().len();
        let mut chance = Scripted::answering(&[true, true, false]);
        assert_eq!(
            session.hire(ShipId(129), &mut chance),
            Ok(Hired {
                ship: ShipId(129),
                fee: 1000,
                wage: 100,
                joined: false,
            }),
            "the -1 ignored (@0x5f1f1)"
        );
        assert_eq!(session.pilot().cash(), 24_000, "the fee is gone");
        assert_eq!(session.pilot().escorts(), [], "no escort record");
        assert_eq!(session.space.escorts.fleet, []);
        assert_eq!(session.npcs().len(), npcs, "nothing placed");
        assert!(session.take_save_due(), "the pilot changed (@0x5f1df)");
        let list = session.escorts_for_hire(&mut chance).expect("a bar");
        assert_eq!(listed(&list), [130]);
        assert_eq!(chance.asked, [50, 50, 50], "129's roll drawn again");
        assert_eq!(
            session.rules().source_for(RuleKey::HireNoSlot),
            RuleSource::Engine,
            "by default"
        );
    }

    #[test]
    fn by_the_other_reading_of_hire_no_slot_a_hire_with_no_slot_is_refused_and_changes_nothing() {
        let catalog = barred(vec![hireable(129, 50)]);
        let rules = Rulebook::default().with_override(RuleKey::HireNoSlot, RuleSource::Bible);
        let mut session = landed(&catalog, 25_000).with_rules(rules);
        crowd(&mut session, 55);
        let before = session.pilot().clone();
        let mut chance = Scripted::answering(&[true]);
        assert_eq!(
            session.hire(ShipId(129), &mut chance),
            Err(HireRefusal::NoSlot)
        );
        assert_eq!(session.pilot(), &before);
        assert!(!session.take_save_due());
        let list = session.escorts_for_hire(&mut chance).expect("a bar");
        assert_eq!(row(&list, 129).hire, Err(HireRefusal::NoSlot));
        assert_eq!(chance.asked, [50], "the roll kept");
        let mut poor = landed(&catalog, 999).with_rules(rules);
        crowd(&mut poor, 55);
        assert_eq!(
            row(&list_drawn(&mut poor), 129).hire,
            Err(HireRefusal::NoSlot),
            "before the fee"
        );
        let mut full = landed(&catalog, 25_000).with_rules(rules);
        full.pilot.escorts = vec![escort(false); MAX_ESCORTS];
        crowd(&mut full, 55);
        assert_eq!(
            row(&list_drawn(&mut full), 129).hire,
            Err(HireRefusal::FleetFull),
            "after the room"
        );
        let mut roomy = landed(&catalog, 25_000).with_rules(rules);
        crowd(&mut roomy, 54);
        assert_eq!(row(&list_drawn(&mut roomy), 129).hire, Ok(()));
    }

    /// The hire list, every roll holding.
    fn list_drawn(session: &mut Session) -> HireList {
        session
            .escorts_for_hire(&mut Scripted::answering(&[true; 8]))
            .expect("a bar")
    }

    #[test]
    fn a_hire_counts_a_loaded_escorts_berth_as_held() {
        let catalog = paying();
        let mut pilot = crate::Pilot::new(&catalog, "Ada").expect("starts");
        pilot.cash = 1_000_000;
        pilot.escorts = vec![paid(129, None, false)];
        pilot.stellar = Some(BAR_AT);
        let mut session = Session::fly(&catalog, pilot).expect("flies");
        assert_eq!(land_now(&mut session), Ok(BAR_AT));
        assert_eq!(session.space.escorts.fleet, [], "none placed yet");
        for berth in 2..=55 {
            let hulk = crate::testkit::npc(0, ShipStats::default());
            session.space.traffic.add_npc_at(hulk, berth);
        }
        let hired = session.hire(ShipId(129), &mut NeverFires).expect("hired");
        assert!(!hired.joined, "berth 1 is the loaded escort's");
        assert_eq!(session.pilot().escorts().len(), 1);
    }

    #[test]
    fn by_the_bible_reading_of_spawn_reserve_a_hire_takes_berth_56() {
        let catalog = barred(vec![hireable(129, 100)]);
        let rules = Rulebook::default().with_override(RuleKey::SpawnReserve, RuleSource::Bible);
        let mut session = landed(&catalog, 25_000).with_rules(rules);
        crowd(&mut session, 55);
        let hired = session.hire(ShipId(129), &mut NeverFires).expect("hired");
        assert!(hired.joined);
        let id = session.space.escorts.fleet[0].expect("placed");
        assert_eq!(session.space.npc(id).map(|npc| npc.berth), Some(56));
    }

    /// Adds a pirate at (`x`, `y`).
    fn pirate(session: &mut Session, x: f32, y: f32) -> crate::NpcId {
        let mut npc = crate::testkit::npc(0, ShipStats::new(FAST, &[]));
        npc.ship = ShipId(152);
        npc.ai_type = AiType::Interceptor;
        npc.state.position = crate::Vec2::new(x, y);
        session.space.traffic.add_npc(npc)
    }

    #[test]
    fn a_hired_escort_follows_the_player_as_any_escort() {
        let catalog = barred(vec![hireable(129, 100)]);
        let mut session = landed(&catalog, 25_000);
        session.hire(ShipId(129), &mut NeverFires).expect("hires");
        session
            .take_off(&catalog, &mut NeverFires)
            .expect("takes off");
        let placed = |session: &Session| {
            let id = session.space.escorts.fleet[0].expect("placed");
            session.space.npc(id).expect("in the system").clone()
        };
        let npc = placed(&session);
        assert_eq!(npc.ship, ShipId(129));
        let duty = npc.escort.expect("an escort");
        assert_eq!((duty.formation.slot, duty.formation.ships), (2, 2));
        assert_eq!(
            npc.state.position,
            slot_position(session.player(), 2, 2, duty.formation.spacing)
        );
        assert_eq!(npc.govt, None);
        assert_eq!(npc.goal, Goal::Formation { guard: None });
        assert_eq!(npc.reserves.shield, ShipStats::new(FAST, &[]).full().shield);
        assert_eq!(npc.reserves.armor, ShipStats::new(FAST, &[]).full().armor);

        assert_eq!(jump(&mut session, &catalog, 131), Some(SystemId(131)));
        let npc = placed(&session);
        assert_eq!(npc.ship, ShipId(129));
        assert_eq!(
            npc.escort.map(|duty: EscortDuty| duty.formation.slot),
            Some(2),
            "through the jump"
        );
        assert_eq!(
            npc.state.position,
            slot_position(session.player(), 2, 2, duty.formation.spacing)
        );

        let quarry = pirate(&mut session, 0.0, -400.0);
        assert_eq!(session.select_target(TargetPick::Nearest), Some(quarry));
        assert!(
            session
                .command_escorts(EscortGroup::All, EscortCommand::Attack, &mut NeverFires)
                .is_some()
        );
        assert_eq!(placed(&session).target, Some(ShipRef::Npc(quarry)));
    }

    // Paying the escorts.

    /// [`barred`] with ship 129 of `Cost` 10,000 and 130 of 30,000, both
    /// for hire, and outfit 300 adding a day to each jump.
    fn paying() -> FakePilotCatalog {
        let mut catalog = barred(vec![
            hireable(129, 100),
            ShipRecord {
                cost: 30_000,
                ..hireable(130, 100)
            },
        ]);
        catalog.outfits = vec![crate::testkit::outfit(300, &[(HYPERSPACE_DAYS, 1)])];
        catalog
    }

    /// An escort of `ship` at `wage`, `carried` or not.
    fn paid(ship: i16, wage: Option<i64>, carried: bool) -> Escort {
        Escort {
            ship: ShipId(ship),
            wage,
            ..escort(carried)
        }
    }

    /// The fleet: hired escorts at wages 100 and 300, a captured one and
    /// a carried fighter.
    fn payroll() -> Vec<Escort> {
        vec![
            paid(129, Some(100), false),
            paid(130, Some(300), false),
            paid(129, None, false),
            paid(129, None, true),
        ]
    }

    /// [`paying`]'s session holding `cash`, with `escorts`, flying with
    /// its fleet placed, `rule` choosing take-off pay, and each jump
    /// taking `days`.
    fn payer(cash: i64, escorts: Vec<Escort>, days: u16, rule: RuleSource) -> Session {
        let catalog = paying();
        let mut pilot = crate::Pilot::new(&catalog, "Ada").expect("starts");
        pilot.cash = cash;
        pilot.escorts = escorts;
        if days > 1 {
            pilot
                .outfits
                .insert(crate::catalog::OutfitId(300), days - 1);
        }
        let mut session = Session::fly(&catalog, pilot)
            .expect("flies")
            .with_rules(Rulebook::default().with_override(RuleKey::TakeOffPay, rule));
        session.tick_traffic(&catalog, &Peaceful, &mut NeverFires);
        assert_eq!(session.stats().jump_days, u32::from(days));
        session.take_save_due();
        session
    }

    fn ships(session: &Session) -> Vec<(i16, Option<i64>, bool)> {
        session
            .pilot()
            .escorts()
            .iter()
            .map(|escort| (escort.ship.0, escort.wage, escort.carried))
            .collect()
    }

    /// The ship types of the fleet's NPCs in the system, in fleet order.
    fn placed_ships(session: &Session) -> Vec<i16> {
        session
            .space
            .escorts
            .fleet
            .iter()
            .flatten()
            .filter_map(|&id| session.space.npc(id))
            .map(|npc| npc.ship.0)
            .collect()
    }

    #[test]
    fn each_day_of_a_jump_pays_every_hired_escort_its_wage() {
        let mut session = payer(1000, payroll(), 1, RuleSource::Engine);
        jump(&mut session, &paying(), 131).expect("arrives");
        assert_eq!(session.pilot().cash(), 600);
        assert_eq!(session.pilot().escorts(), payroll(), "all stay");
        assert_eq!(session.take_pay_notes(), []);
        assert!(!session.take_save_due(), "paying alone makes none");

        let mut session = payer(1000, payroll(), 2, RuleSource::Engine);
        jump(&mut session, &paying(), 131).expect("arrives");
        assert_eq!(session.pilot().cash(), 200, "two days");
    }

    /// A deadly stellar over `session`'s escort `index`, listed and
    /// shown, and the fight's tick that kills it: its NPC.
    fn kill_by_a_deadly_stellar(session: &mut Session, index: usize) -> crate::traffic::npc::NpcId {
        let id = session.space.escorts.fleet[index].expect("placed");
        let at = session.space.npc(id).expect("there").state.position;
        session.chart.sites = vec![LandingSite {
            flags2: crate::deadly_stellar::DEADLY_STELLAR,
            frame_size: Some((2, 2)),
            ..crate::testkit::planet(160, at.x, at.y)
        }];
        session.chart.listed = session.chart.sites.clone();
        session.tick_combat(crate::combat::Rules::default(), &mut NeverFires);
        assert!(session.space.npc(id).is_none(), "killed");
        id
    }

    /// A hired escort a deadly stellar frees is forgotten for good, as the
    /// original's save (`_WritePilotData` @0x72604, @0x7265e, @0x72728),
    /// system setup (`_SetupShipsInSystem` @0x42baf) and pay
    /// (`_DoEscortPayment` @0x40148) skip its inactive slot.
    #[test]
    fn a_hired_escort_a_deadly_stellar_kills_is_not_saved_placed_again_or_paid() {
        let mut session = payer(1000, payroll(), 1, RuleSource::Engine);
        kill_by_a_deadly_stellar(&mut session, 0);
        let left = [
            (130, Some(300), false),
            (129, None, false),
            (129, None, true),
        ];
        assert_eq!(ships(&session), left, "gone from the escorts");
        let saved = crate::save::decode(&crate::save::encode(session.pilot())).expect("reads");
        assert_eq!(saved.escorts(), session.pilot().escorts(), "nor saved");
        jump(&mut session, &paying(), 131).expect("arrives");
        assert_eq!(session.pilot().cash(), 700, "only the 300 wage is paid");
        assert_eq!(ships(&session), left);
        assert_eq!(placed_ships(&session), [130, 129, 129], "nor placed again");
    }

    /// The hire limit (`_CanHireEscorts` @0x57ad) counts the active slots
    /// the player leads, so one a deadly stellar frees makes room.
    #[test]
    fn a_hired_escort_a_deadly_stellar_kills_frees_its_place_under_the_hire_limit() {
        let fleet = vec![paid(129, Some(100), false); MAX_ESCORTS];
        let mut session = payer(1000, fleet, 1, RuleSource::Engine);
        assert!(!session.can_hire_escorts(), "full");
        kill_by_a_deadly_stellar(&mut session, 0);
        assert!(session.can_hire_escorts());
        assert_eq!(session.pilot().escorts().len(), MAX_ESCORTS - 1);
    }

    #[test]
    fn cash_equal_to_the_wages_pays_them_leaving_none() {
        let mut session = payer(400, payroll(), 1, RuleSource::Engine);
        jump(&mut session, &paying(), 131).expect("arrives");
        assert_eq!(session.pilot().cash(), 0);
        assert_eq!(session.pilot().escorts().len(), 4);
        assert_eq!(session.take_pay_notes(), []);
    }

    #[test]
    fn an_escort_the_player_cannot_pay_defects_and_leaves_the_fleet() {
        let mut session = payer(350, payroll(), 1, RuleSource::Engine);
        jump(&mut session, &paying(), 131).expect("arrives");
        assert_eq!(session.pilot().cash(), 250, "the first paid");
        assert_eq!(
            ships(&session),
            [
                (129, Some(100), false),
                (129, None, false),
                (129, None, true)
            ],
            "the second gone, the others in order"
        );
        assert_eq!(placed_ships(&session), [129, 129, 129]);
        assert!(
            session.npcs().iter().all(|npc| npc.ship != ShipId(130)),
            "not placed in the new system"
        );
        assert_eq!(session.take_pay_notes(), [PayNote::Defected(1)]);
        assert_eq!(session.take_pay_notes(), [], "taken");
        assert!(session.take_save_due());
    }

    #[test]
    fn each_day_pays_in_turn_and_those_unpaid_on_a_later_day_defect() {
        let mut session = payer(450, payroll(), 2, RuleSource::Engine);
        jump(&mut session, &paying(), 131).expect("arrives");
        assert_eq!(session.pilot().cash(), 50, "the first day paid both");
        assert_eq!(
            ships(&session),
            [(129, None, false), (129, None, true)],
            "both hired escorts defected on the second"
        );
        assert_eq!(session.take_pay_notes(), [PayNote::Defected(2)]);
    }

    #[test]
    fn a_defector_leaves_the_system_with_the_fleet_lined_up_with_its_records() {
        let mut session = payer(350, payroll(), 1, RuleSource::Engine);
        let defector = session.space.escorts.fleet[1].expect("placed");
        let others = [
            session.space.escorts.fleet[0],
            session.space.escorts.fleet[2],
            session.space.escorts.fleet[3],
        ];
        session.pay_escorts(1);
        assert_eq!(session.pilot().cash(), 250);
        assert_eq!(
            session.space.escorts.fleet, others,
            "the fleet lined up with the records"
        );
        assert!(
            session.space.npc(defector).is_none(),
            "gone from the system"
        );
        assert_eq!(placed_ships(&session), [129, 129, 129]);
        let slots: Vec<u8> = session
            .space
            .escorts
            .fleet
            .iter()
            .flatten()
            .filter_map(|&id| session.space.npc(id))
            .filter_map(|npc| npc.escort.map(|duty| duty.formation.slot))
            .collect();
        assert_eq!(slots, [2, 3, 4], "re-formed");
        assert_eq!(session.take_pay_notes(), [PayNote::Defected(1)]);
    }

    #[test]
    fn a_defector_takes_its_share_and_resets_while_still_in_the_fleet() {
        use crate::market::Good;
        use crate::scan::Latches;
        use crate::session::precalced::tests::convicted;
        for source in RuleSource::ALL {
            let mut session = payer(350, payroll(), 1, RuleSource::Engine)
                .with_rules(Rulebook::default().with_override(RuleKey::LostCargoShare, source));
            // 80 tons of holds: the player's 20, three escorts' 20 each,
            // and the carried fighter's none.
            assert_eq!(session.capacity(), 80);
            session.pilot.cargo = std::iter::once((Good::Commodity(0), 60)).collect();
            session
                .pilot
                .missions
                .put(0, crate::testkit::carrying(140, 30));
            convicted(&mut session);
            session.pay_escorts(1);
            assert_eq!(session.take_pay_notes(), [PayNote::Defected(1)]);
            assert_eq!(
                session.ship.latches.scan_latches,
                Latches::ARMED,
                "{source:?}: @0x40240"
            );
            assert_eq!(
                session.pilot().held(Good::Commodity(0)),
                45,
                "{source:?}: 20 of 80 lost, its own holds counted once; the \
                 reset, the defector still in the fleet, trims nothing, so 75 \
                 tons stay past the 60 left"
            );
            assert_eq!(session.capacity(), 60);
        }
    }

    #[test]
    fn defectors_before_the_fleet_is_placed_leave_their_records_alone() {
        let catalog = paying();
        let mut pilot = crate::Pilot::new(&catalog, "Ada").expect("starts");
        pilot.cash = 50;
        pilot.escorts = payroll();
        pilot.stellar = Some(BAR_AT);
        let mut session = Session::fly(&catalog, pilot).expect("flies");
        assert_eq!(land_now(&mut session), Ok(BAR_AT), "landed on resuming");
        assert_eq!(session.space.escorts.fleet, [], "none placed yet");
        session
            .take_off(&catalog, &mut NeverFires)
            .expect("takes off");
        assert_eq!(ships(&session), [(129, None, false), (129, None, true)]);
        assert_eq!(session.take_pay_notes(), [PayNote::Defected(2)]);
    }

    #[test]
    fn a_hire_after_loading_takes_the_berth_after_the_loaded_escorts() {
        let catalog = paying();
        let mut pilot = crate::Pilot::new(&catalog, "Ada").expect("starts");
        pilot.cash = 1_000_000;
        pilot.escorts = vec![paid(129, None, false), paid(130, None, false)];
        pilot.stellar = Some(BAR_AT);
        let mut session = Session::fly(&catalog, pilot).expect("flies");
        assert_eq!(land_now(&mut session), Ok(BAR_AT));
        assert_eq!(session.space.escorts.fleet, [], "none placed yet");
        session.hire(ShipId(129), &mut NeverFires).expect("hired");
        assert_eq!(session.space.escorts.fleet.len(), 3);
        let hired = session.space.escorts.fleet[2].expect("placed while landed");
        assert_eq!(session.space.npc(hired).map(|npc| npc.berth), Some(3));
        session
            .take_off(&catalog, &mut NeverFires)
            .expect("takes off");
        let berths: Vec<Option<u32>> = session
            .space
            .escorts
            .fleet
            .iter()
            .map(|id| id.and_then(|id| session.space.npc(id)).map(|npc| npc.berth))
            .collect();
        assert_eq!(berths, [Some(1), Some(2), Some(3)]);
    }

    #[test]
    fn with_no_hired_escort_nothing_is_paid() {
        let unpaid = vec![paid(129, None, false), paid(129, None, true)];
        let mut session = payer(10, unpaid.clone(), 1, RuleSource::Engine);
        jump(&mut session, &paying(), 131).expect("arrives");
        assert_eq!(session.pilot().cash(), 10);
        assert_eq!(session.pilot().escorts(), unpaid);
        assert_eq!(session.take_pay_notes(), []);
        assert!(!session.take_save_due());
    }

    /// [`payer`]'s session, landed at the bar.
    fn docked(cash: i64, rule: RuleSource) -> Session {
        let mut session = payer(cash, payroll(), 1, rule);
        assert_eq!(land_now(&mut session), Ok(BAR_AT));
        session.take_save_due();
        session
    }

    #[test]
    fn by_the_engine_each_take_off_pays_a_days_wages() {
        let mut session = docked(1000, RuleSource::Engine);
        assert_eq!(session.pilot().cash(), 1000, "landing pays nothing");
        session
            .take_off(&catalog(), &mut NeverFires)
            .expect("takes off");
        assert_eq!(session.pilot().cash(), 600);
        assert_eq!(session.take_pay_notes(), []);
        assert_eq!(
            Session::start(&catalog())
                .expect("starts")
                .rules()
                .source_for(RuleKey::TakeOffPay),
            RuleSource::Engine,
            "by default"
        );
    }

    #[test]
    fn a_take_off_pays_one_days_wage_whatever_the_stay_passes() {
        for (rule, cash) in [(RuleSource::Engine, 600), (RuleSource::Bible, 1000)] {
            let mut session = docked(1000, rule);
            let landed = session.date();
            session.port.stay.ship_bought = true;
            session
                .take_off(&catalog(), &mut NeverFires)
                .expect("takes off");
            assert_eq!(session.pilot().cash(), cash, "{rule:?}");
            let mut days = std::iter::successors(Some(landed), |date| Some(date.next_day()));
            assert_eq!(Some(session.date()), days.nth(5), "{rule:?}");
        }
    }

    #[test]
    fn by_the_engine_an_escort_unpaid_at_take_off_is_gone_before_the_fleet_is_placed() {
        let mut session = docked(350, RuleSource::Engine);
        assert_eq!(
            session.rules().source_for(RuleKey::TakeOffPay),
            RuleSource::Engine
        );
        session
            .take_off(&catalog(), &mut NeverFires)
            .expect("takes off");
        assert_eq!(session.pilot().cash(), 250);
        assert_eq!(session.pilot().escorts().len(), 3);
        assert_eq!(session.take_pay_notes(), [PayNote::Defected(1)]);
        session.tick_traffic(&paying(), &Peaceful, &mut NeverFires);
        assert_eq!(placed_ships(&session), [129, 129, 129]);
    }

    #[test]
    fn by_the_other_reading_only_the_days_of_a_jump_are_paid() {
        let mut session = docked(1000, RuleSource::Bible);
        assert_eq!(
            session.rules().source_for(RuleKey::TakeOffPay),
            RuleSource::Bible
        );
        session
            .take_off(&catalog(), &mut NeverFires)
            .expect("takes off");
        assert_eq!(session.pilot().cash(), 1000);
        session.tick_traffic(&paying(), &Peaceful, &mut NeverFires);
        jump(&mut session, &paying(), 131).expect("arrives");
        assert_eq!(session.pilot().cash(), 600);
    }

    /// A pilot who hired ship 129 (`Cost` 10,000, so a wage of 100) and
    /// was saved and loaded, flying with `catalog`, where its ship
    /// type's record may have changed, by `rule`; and its escort's NPC,
    /// none when its ship type has no record.
    fn rehired(catalog: &FakePilotCatalog, rule: RuleSource) -> (Session, Option<crate::NpcId>) {
        let original = barred(vec![hireable(129, 100)]);
        let mut session = landed(&original, 25_000);
        session.hire(ShipId(129), &mut NeverFires).expect("hires");
        let saved = crate::save::encode(session.pilot());
        let mut pilot = crate::save::decode(&saved).expect("loads");
        pilot.cash = 1000;
        pilot.stellar = None;
        let mut session = Session::fly(catalog, pilot)
            .expect("flies")
            .with_rules(Rulebook::default().with_override(RuleKey::EscortWage, rule));
        assert_eq!(session.rules().source_for(RuleKey::EscortWage), rule);
        session.tick_traffic(catalog, &Peaceful, &mut NeverFires);
        let id = session.space.escorts.fleet[0];
        (session, id)
    }

    /// The pay line the escort dialog shows for NPC `id`, hailed.
    fn status(
        session: &mut Session,
        catalog: &FakePilotCatalog,
        id: crate::NpcId,
    ) -> Option<ShownWage> {
        session.chart.target = Some(id);
        match session
            .hail_key(catalog, &HailOptions::default(), false, &mut NeverFires)
            .expect("answered")
        {
            crate::session::Hailed::Escort(view) => view.pay,
            other => panic!("{other:?}"),
        }
    }

    /// What one jump day pays, and the hail shows, for a hired escort
    /// whose ship type's `Cost` is now `cost` (none: no record), by
    /// `rule`; and its record's wage.
    fn wage_paid(cost: Option<i32>, rule: RuleSource) -> (i64, Option<ShownWage>, Option<i64>) {
        let mut catalog = barred(vec![hireable(129, 100)]);
        match cost {
            Some(cost) => catalog.ship_records[1].cost = cost,
            None => {
                catalog.ship_records.pop();
            }
        }
        let (mut session, id) = rehired(&catalog, rule);
        let shown = id.and_then(|id| status(&mut session, &catalog, id));
        session.press_escort(crate::escort_dialog::EscortButton::Close);
        jump(&mut session, &catalog, 131).expect("arrives");
        let wage = session.pilot().escorts()[0].wage;
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&crate::save::encode(session.pilot()))
                .expect("JSON")["escorts"][0]["wage"],
            100,
            "the wage it was hired at is kept"
        );
        (1000 - session.pilot().cash(), shown, wage)
    }

    #[test]
    fn by_the_engine_a_hired_escort_is_paid_the_wage_its_type_gives_now() {
        assert_eq!(
            wage_paid(Some(30_000), RuleSource::Engine),
            (300, Some(ShownWage::kept(300)), Some(100))
        );
        assert_eq!(
            wage_paid(None, RuleSource::Engine).0,
            100,
            "with no record, the wage it was hired at"
        );
        assert_eq!(
            Session::start(&catalog())
                .expect("starts")
                .rules()
                .source_for(RuleKey::EscortWage),
            RuleSource::Engine,
            "by default"
        );
    }

    #[test]
    fn by_the_other_reading_a_hired_escort_is_paid_the_wage_it_was_hired_at() {
        assert_eq!(
            wage_paid(Some(30_000), RuleSource::Bible),
            (100, Some(ShownWage::kept(100)), Some(100))
        );
    }

    #[test]
    fn with_the_data_unchanged_both_readings_pay_alike() {
        for rule in RuleSource::ALL {
            assert_eq!(
                wage_paid(Some(10_000), rule),
                (100, Some(ShownWage::kept(100)), Some(100)),
                "{rule:?}"
            );
        }
    }

    /// The pay line hailing shows for a hired escort that keeps a wage
    /// of `kept` and whose ship type's `Cost` is now `cost`, by `rule`;
    /// and what one jump day pays it.
    fn line_and_pay(kept: i64, cost: i32, rule: RuleSource) -> (ShownWage, i64) {
        let mut catalog = barred(vec![hireable(129, 100)]);
        catalog.ship_records[1].cost = cost;
        let (mut session, id) = rehired(&catalog, rule);
        session.pilot.escorts[0].wage = Some(kept);
        let shown = id
            .and_then(|id| status(&mut session, &catalog, id))
            .expect("hired");
        session.press_escort(crate::escort_dialog::EscortButton::Close);
        let cash = session.pilot().cash();
        jump(&mut session, &catalog, 131).expect("arrives");
        (shown, cash - session.pilot().cash())
    }

    fn shown(credits: i64, unit: CreditUnit) -> ShownWage {
        ShownWage { credits, unit }
    }

    #[test]
    fn by_the_engine_the_pay_line_is_worked_out_from_the_cost_now() {
        let engine = RuleSource::Engine;
        assert_eq!(
            line_and_pay(100, 100, engine),
            (shown(1, CreditUnit::Credit), 1)
        );
        assert_eq!(
            line_and_pay(100, 150, engine),
            (shown(1, CreditUnit::Credits), 1)
        );
        assert_eq!(
            line_and_pay(100, -150, engine),
            (shown(-1, CreditUnit::Credits), 0),
            "the line is not clamped, the pay is"
        );
    }

    #[test]
    fn by_the_other_reading_the_pay_line_shows_the_wage_kept() {
        let bible = RuleSource::Bible;
        assert_eq!(
            line_and_pay(100, 30_000, bible),
            (shown(100, CreditUnit::Credits), 100)
        );
        assert_eq!(
            line_and_pay(1, 150, bible),
            (shown(1, CreditUnit::Credit), 1)
        );
    }

    #[test]
    fn with_no_record_the_pay_line_shows_the_wage_kept() {
        let mut catalog = barred(vec![hireable(129, 100)]);
        catalog.ship_records.pop();
        let (mut session, id) = rehired(&catalog, RuleSource::Engine);
        assert_eq!(id, None, "no NPC to hail");
        session.pilot.escorts[0].wage = Some(1);
        assert_eq!(
            session
                .setup
                .shown_wage(&session.statics, &session.pilot.escorts[0]),
            shown(1, CreditUnit::Credit)
        );
    }

    #[test]
    fn a_shared_port_is_equal_only_to_itself() {
        let one: Rc<dyn HireTerms> = Rc::new(NovaHire::default());
        let other: Rc<dyn HireTerms> = Rc::new(NovaHire::default());
        assert_eq!(Shared(Rc::clone(&one)), Shared(Rc::clone(&one)));
        assert_ne!(Shared(Rc::clone(&one)), Shared(other));
        assert_eq!(
            format!("{:?}", Shared(one)),
            "NovaHire { hire_fee: Engine }"
        );
    }
}
