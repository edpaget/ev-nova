//! Hiring escorts in the bar and paying them: the session's side of
//! [`hire`](crate::hire).
//!
//! **The list.** Landed at a stellar with a bar, the session lists the
//! ships for hire ([`Session::escorts_for_hire`]) by the
//! [`hire`](crate::hire) rules, with the fee and wage its [`HireTerms`]
//! give ([`Session::with_hire_terms`], Nova's by default) and each ship's
//! `Availability` tested through its [`ControlBits`]
//! ([`Session::with_control_bits`]). Each class's roll for the day is
//! drawn on the caller's [`Chance`] the first time the list is built after
//! a landing, and kept until the next; hiring a class draws its roll again
//! when the list is next built. The rolls are never saved. The
//! outfitter's and the shipyard's `BuyRandom` rolls follow the same rule
//! ([`Session::outfitter`], [`Session::shipyard`]), buying a ship drawing
//! its class's roll again. Whether an unmet `Require` refuses a hire
//! follows
//! [`Session::with_hire_require`].
//!
//! **Hiring** ([`Session::hire`]) takes what the terms charge from the
//! cash and adds the ship to the fleet, an escort as a ship captured is
//! (see [`escorts`](super::escorts)), its reserves full as its class holds
//! them, no standing order, and the daily wage it was hired at
//! ([`Escort::wage`]): a change made in the spaceport, so a save is due.
//! A refused hire changes nothing.
//!
//! **Paying.** Each hired escort is paid its wage for each day of a jump,
//! once the date has moved on, and for one day at each take-off as
//! [`Session::with_take_off_pay`] says; the wage it is paid follows
//! [`Session::with_escort_wage`]. One the cash does not cover defects,
//! its record and its NPC gone at once, so it is never placed in the next
//! system, and the flight is told how many defected
//! ([`Session::take_pay_notes`]), which makes a save due; paying alone
//! makes none, as paying a haggled price makes none. Hailing a hired
//! escort shows the wage it is paid ([`EscortStatus`](crate::hail::EscortStatus)).
//!
//! The session keeps its [`HireTerms`] and [`ControlBits`] rather than
//! being passed them on each call, a deliberate departure from
//! [`BoardingRule`](crate::BoardingRule) and
//! [`Behaviour`](crate::Behaviour): see [`hire`](crate::hire).

use std::rc::Rc;

use super::Session;
use super::control::Facts;
use crate::board::MAX_ESCORTS;
use crate::catalog::{LandingSite, ShipId};
use crate::chance::Chance;
use crate::control::ControlBits;
use crate::hire::{Bar, HireList, HireRefusal, HireTerms, Hired, PayNote};
use crate::landing::StellarFlags;
use crate::pilot::Escort;
use crate::rulebook::RuleSource;
use crate::traffic::table;
use crate::wares::{self, Roll};

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
        self.hire_terms = Shared(terms);
        self
    }

    /// This session with `bits` testing a ship's `Availability` for hire
    /// and a person's `ActiveOn` against the pilot: Nova's
    /// ([`NovaBits`](crate::NovaBits)) by default.
    #[must_use]
    pub fn with_control_bits(mut self, bits: Rc<dyn ControlBits>) -> Self {
        self.control_bits = Shared(bits);
        self
    }

    /// This session with an unmet `Require` refusing a hire, or not, as
    /// `source` says ([`RuleKey::HireRequire`](crate::RuleKey::HireRequire)):
    /// by the engine's default it does not.
    #[must_use]
    pub fn with_hire_require(mut self, source: RuleSource) -> Self {
        self.hire_require = source;
        self
    }

    /// Whether an unmet `Require` refuses a hire: not by the engine
    /// ([`RuleSource::Engine`]), and so by the other reading.
    #[must_use]
    pub fn hire_require(&self) -> RuleSource {
        self.hire_require
    }

    /// This session with each take-off paying the hired escorts a day's
    /// wages, or not, as `source` says
    /// ([`RuleKey::TakeOffPay`](crate::RuleKey::TakeOffPay)): by the
    /// engine's default it does.
    #[must_use]
    pub fn with_take_off_pay(mut self, source: RuleSource) -> Self {
        self.take_off_pay = source;
        self
    }

    /// Whether each take-off pays the hired escorts a day's wages: by the
    /// engine ([`RuleSource::Engine`]) it does; otherwise only a jump's
    /// days are paid.
    #[must_use]
    pub fn take_off_pay(&self) -> RuleSource {
        self.take_off_pay
    }

    /// This session with a hired escort paid, and showing when hailed,
    /// the wage `source` says
    /// ([`RuleKey::EscortWage`](crate::RuleKey::EscortWage)): by the
    /// engine's default, the wage its ship type's record gives now.
    #[must_use]
    pub fn with_escort_wage(mut self, source: RuleSource) -> Self {
        self.escort_wage = source;
        self
    }

    /// Which wage a hired escort is paid: by the engine
    /// ([`RuleSource::Engine`]), its ship type's now; otherwise the wage
    /// it was hired at.
    #[must_use]
    pub fn escort_wage(&self) -> RuleSource {
        self.escort_wage
    }

    /// What paying the escorts did since this was last taken, in order;
    /// taking it empties the list.
    pub fn take_pay_notes(&mut self) -> Vec<PayNote> {
        std::mem::take(&mut self.pay_notes)
    }

    /// The wage hired `escort` is paid a day (see
    /// [`Session::with_escort_wage`]): by the engine, the terms' wage of
    /// its ship type's record, or the wage it was hired at when there is
    /// none; otherwise the wage it was hired at. None for an escort not
    /// hired.
    pub(super) fn paid_wage(&self, escort: &Escort) -> i64 {
        let kept = escort.wage.unwrap_or(0);
        match self.escort_wage {
            RuleSource::Engine => self
                .ship_record(escort.ship)
                .map_or(kept, |record| self.hire_terms.0.wage(record)),
            RuleSource::Bible => kept,
        }
    }

    /// Pays every hired escort its wage for each of `days`, in fleet
    /// order (see [`hire`](crate::hire)): one the cash does not cover
    /// defects, leaving the fleet and the system at once. When any
    /// defected the flight is told how many, and a save is due.
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
                let wage = self.paid_wage(&escort);
                if wage <= self.pilot.cash {
                    self.pilot.cash -= wage;
                    index += 1;
                    continue;
                }
                self.pilot.escorts.remove(index);
                if index < self.fleet.len()
                    && let Some(id) = self.fleet.remove(index)
                {
                    self.traffic.remove(id);
                }
                defected = defected.saturating_add(1);
            }
        }
        if defected > 0 {
            self.reform();
            self.pay_notes.push(PayNote::Defected(defected));
            self.save_due = true;
        }
    }

    /// The ships for hire in the bar of the stellar the ship is docked at
    /// (see the module docs), drawing on `chance` each class's roll for
    /// the day not drawn yet; `None` when it has not landed at a stellar
    /// with a bar.
    pub fn escorts_for_hire(&mut self, chance: &mut dyn Chance) -> Option<HireList> {
        let site = self.bar_site()?;
        let facts = Facts {
            pilot: &self.pilot,
            ammo_outfits: &self.ammo_outfits,
            armament: &self.armament,
        };
        let bar = Bar {
            ships: &self.ships,
            site: &site,
            contributed: wares::contributed(
                self.fields.contribute,
                &self.pilot.outfits,
                &self.outfits,
            ),
            cash: self.pilot.cash,
            room: self.pilot.escort_count() < MAX_ESCORTS,
            terms: &*self.hire_terms.0,
            control_bits: &*self.control_bits.0,
            pilot: &facts,
            hire_require: self.hire_require,
        };
        let rolls = &mut self.hire_rolls;
        Some(bar.list(|ship, percent| rolls.today(ship, Roll::Chance(percent), chance)))
    }

    /// The stellar the ship is docked at, if it has a bar.
    fn bar_site(&self) -> Option<LandingSite> {
        let stellar = self.landed?;
        self.sites
            .iter()
            .find(|site| site.id == stellar)
            .filter(|site| site.flags & StellarFlags::BAR != 0)
            .copied()
    }

    /// Hires a ship of class `ship` in the bar (see the module docs), the
    /// rolls not drawn yet drawn on `chance`, and gives what the hire did.
    ///
    /// # Errors
    ///
    /// The [`HireRefusal`] that applies, and nothing changes:
    /// [`HireRefusal::NoBar`] when not landed at a stellar with a bar.
    pub fn hire(&mut self, ship: ShipId, chance: &mut dyn Chance) -> Result<Hired, HireRefusal> {
        let list = self.escorts_for_hire(chance).ok_or(HireRefusal::NoBar)?;
        list.check(ship)?;
        let record = self
            .ship_record(ship)
            .cloned()
            .ok_or(HireRefusal::NotListed)?;
        let site = self.bar_site().ok_or(HireRefusal::NoBar)?;
        let fee = self.hire_terms.0.charge(&record, &site, self.pilot.cash);
        let wage = self.hire_terms.0.wage(&record);
        let reserves = table::kind(&record, &self.outfits, &self.arsenal)
            .stats
            .full();
        self.transact(|pilot| {
            pilot.cash -= fee;
            pilot.escorts.push(Escort {
                ship,
                reserves,
                order: None,
                carried: false,
                wage: Some(wage),
                person: None,
            });
        });
        self.hire_rolls.redraw(&ship);
        Ok(Hired { ship, fee, wage })
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::ai::{Goal, Peaceful};
    use crate::catalog::{LandingSite, ShipRecord, StellarId, SystemId};
    use crate::chance::NeverFires;
    use crate::combat::ShipRef;
    use crate::control::{PilotFacts, Test, TestExpr};
    use crate::escort::{EscortCommand, EscortDuty, EscortGroup, slot_position};
    use crate::hail::{EscortStatus, HailOptions};
    use crate::hire::{HireRow, NovaHire, PayNote};
    use crate::reserves::Reserves;
    use crate::stats::HYPERSPACE_DAYS;
    use crate::stats::ShipStats;
    use crate::targeting::TargetPick;
    use crate::testkit::{FAST, FakePilotCatalog, Scripted, catalog, jump, land_now, planet, ship};
    use crate::traffic::npc::AiType;

    /// The bar's stellar, 128, at the centre, under the ship as it
    /// starts.
    const BAR_AT: StellarId = StellarId(128);

    /// A bar of tech level 7 (above 5, so no low-tech discount), at the
    /// centre, so a ship taking off flies out as one starting.
    fn bar() -> LandingSite {
        LandingSite {
            flags: StellarFlags::CAN_LAND | StellarFlags::BAR,
            tech_level: 7,
            ..planet(128, 0.0, 0.0)
        }
    }

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
        fn fee(&self, _ship: &ShipRecord, _site: &LandingSite) -> i64 {
            self.fee
        }

        fn charge(&self, _ship: &ShipRecord, _site: &LandingSite, _cash: i64) -> i64 {
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
                fee: nova.fee(&record, &bar()),
                wage: nova.wage(&record),
                specs: crate::shipyard::ShipSpecs {
                    fields: FAST,
                    max_gun: 2,
                    max_tur: 1,
                    length: 20,
                    crew: 3,
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

    #[test]
    fn each_landing_draws_the_rolls_again() {
        let catalog = barred(vec![hireable(129, 50)]);
        let mut session = landed(&catalog, 25_000);
        let mut chance = Scripted::answering(&[true, false]);
        assert_eq!(
            listed(&session.escorts_for_hire(&mut chance).expect("a bar")),
            [129]
        );
        session.take_off().expect("takes off");
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
        let mut session = landed(&catalog, 25_000).with_hire_require(source);
        assert_eq!(session.hire_require(), source);
        list(&mut session)
    }

    #[test]
    fn by_the_engine_an_unmet_require_only_hides_a_ship_flagged_so() {
        let list = required(RuleSource::Engine);
        assert_eq!(listed(&list), [129, 131]);
        assert_eq!(row(&list, 129).hire, Ok(()));
        assert_eq!(
            Session::start(&catalog()).expect("starts").hire_require(),
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
                wage: 100
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
            }]
        );
        assert!(session.take_save_due());
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

    /// Adds a pirate at (`x`, `y`).
    fn pirate(session: &mut Session, x: f32, y: f32) -> crate::NpcId {
        let mut npc = crate::testkit::npc(0, ShipStats::new(FAST, &[]));
        npc.ship = ShipId(152);
        npc.ai_type = AiType::Interceptor;
        npc.state.position = crate::Vec2::new(x, y);
        session.traffic.add_npc(npc)
    }

    #[test]
    fn a_hired_escort_follows_the_player_as_any_escort() {
        let catalog = barred(vec![hireable(129, 100)]);
        let mut session = landed(&catalog, 25_000);
        session.hire(ShipId(129), &mut NeverFires).expect("hires");
        session.take_off().expect("takes off");
        session.tick_traffic(&catalog, &Peaceful, &mut NeverFires);
        let placed = |session: &Session| {
            let id = session.fleet[0].expect("placed");
            session.npc(id).expect("in the system").clone()
        };
        let npc = placed(&session);
        assert_eq!(npc.ship, ShipId(129));
        let duty = npc.escort.expect("an escort");
        assert_eq!((duty.slot, duty.ships), (2, 2));
        assert_eq!(
            npc.state.position,
            slot_position(session.player(), 2, 2, duty.spacing)
        );
        assert_eq!(npc.govt, None);
        assert_eq!(npc.goal, Goal::Formation { guard: None });
        assert_eq!(npc.reserves.shield, ShipStats::new(FAST, &[]).full().shield);
        assert_eq!(npc.reserves.armor, ShipStats::new(FAST, &[]).full().armor);

        assert_eq!(jump(&mut session, &catalog, 131), Some(SystemId(131)));
        let npc = placed(&session);
        assert_eq!(npc.ship, ShipId(129));
        assert_eq!(
            npc.escort.map(|duty: EscortDuty| duty.slot),
            Some(2),
            "through the jump"
        );
        assert_eq!(
            npc.state.position,
            slot_position(session.player(), 2, 2, duty.spacing)
        );

        let quarry = pirate(&mut session, 0.0, -400.0);
        assert_eq!(session.select_target(TargetPick::Nearest), Some(quarry));
        assert!(
            session
                .command_escorts(EscortGroup::All, EscortCommand::Attack)
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
            .with_take_off_pay(rule);
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
            .fleet
            .iter()
            .flatten()
            .filter_map(|&id| session.npc(id))
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
        let defector = session.fleet[1].expect("placed");
        let others = [session.fleet[0], session.fleet[2], session.fleet[3]];
        session.pay_escorts(1);
        assert_eq!(session.pilot().cash(), 250);
        assert_eq!(session.fleet, others, "the fleet lined up with the records");
        assert!(session.npc(defector).is_none(), "gone from the system");
        assert_eq!(placed_ships(&session), [129, 129, 129]);
        let slots: Vec<u8> = session
            .fleet
            .iter()
            .flatten()
            .filter_map(|&id| session.npc(id))
            .filter_map(|npc| npc.escort.map(|duty| duty.slot))
            .collect();
        assert_eq!(slots, [2, 3, 4], "re-formed");
        assert_eq!(session.take_pay_notes(), [PayNote::Defected(1)]);
    }

    #[test]
    fn defectors_before_the_fleet_is_placed_leave_their_records_alone() {
        let catalog = paying();
        let mut pilot = crate::Pilot::new(&catalog, "Ada").expect("starts");
        pilot.cash = 50;
        pilot.escorts = payroll();
        pilot.stellar = Some(BAR_AT);
        let mut session = Session::fly(&catalog, pilot).expect("flies");
        assert_eq!(session.landed(), Some(BAR_AT), "resumed docked");
        assert_eq!(session.fleet, [], "none placed yet");
        session.take_off().expect("takes off");
        assert_eq!(ships(&session), [(129, None, false), (129, None, true)]);
        assert_eq!(session.take_pay_notes(), [PayNote::Defected(2)]);
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
        session.take_off().expect("takes off");
        assert_eq!(session.pilot().cash(), 600);
        assert_eq!(session.take_pay_notes(), []);
        assert_eq!(
            Session::start(&catalog()).expect("starts").take_off_pay(),
            RuleSource::Engine,
            "by default"
        );
    }

    #[test]
    fn by_the_engine_an_escort_unpaid_at_take_off_is_gone_before_the_fleet_is_placed() {
        let mut session = docked(350, RuleSource::Engine);
        assert_eq!(session.take_off_pay(), RuleSource::Engine);
        session.take_off().expect("takes off");
        assert_eq!(session.pilot().cash(), 250);
        assert_eq!(session.pilot().escorts().len(), 3);
        assert_eq!(session.take_pay_notes(), [PayNote::Defected(1)]);
        session.tick_traffic(&paying(), &Peaceful, &mut NeverFires);
        assert_eq!(placed_ships(&session), [129, 129, 129]);
    }

    #[test]
    fn by_the_other_reading_only_the_days_of_a_jump_are_paid() {
        let mut session = docked(1000, RuleSource::Bible);
        assert_eq!(session.take_off_pay(), RuleSource::Bible);
        session.take_off().expect("takes off");
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
            .with_escort_wage(rule);
        assert_eq!(session.escort_wage(), rule);
        session.tick_traffic(catalog, &Peaceful, &mut NeverFires);
        let id = session.fleet[0];
        (session, id)
    }

    /// What hailing NPC `id` says of it as an escort.
    fn status(
        session: &mut Session,
        catalog: &FakePilotCatalog,
        id: crate::NpcId,
    ) -> Option<EscortStatus> {
        session.target = Some(id);
        session
            .hail(catalog, &HailOptions::default(), &mut NeverFires)
            .expect("answered")
            .escort
    }

    /// What one jump day pays, and the hail shows, for a hired escort
    /// whose ship type's `Cost` is now `cost` (none: no record), by
    /// `rule`; and its record's wage.
    fn wage_paid(cost: Option<i32>, rule: RuleSource) -> (i64, Option<EscortStatus>, Option<i64>) {
        let mut catalog = barred(vec![hireable(129, 100)]);
        match cost {
            Some(cost) => catalog.ship_records[1].cost = cost,
            None => {
                catalog.ship_records.pop();
            }
        }
        let (mut session, id) = rehired(&catalog, rule);
        let shown = id.and_then(|id| status(&mut session, &catalog, id));
        session.hang_up();
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
            (300, Some(EscortStatus { wage: Some(300) }), Some(100))
        );
        assert_eq!(
            wage_paid(None, RuleSource::Engine).0,
            100,
            "with no record, the wage it was hired at"
        );
        assert_eq!(
            Session::start(&catalog()).expect("starts").escort_wage(),
            RuleSource::Engine,
            "by default"
        );
    }

    #[test]
    fn by_the_other_reading_a_hired_escort_is_paid_the_wage_it_was_hired_at() {
        assert_eq!(
            wage_paid(Some(30_000), RuleSource::Bible),
            (100, Some(EscortStatus { wage: Some(100) }), Some(100))
        );
    }

    #[test]
    fn with_the_data_unchanged_both_readings_pay_alike() {
        for rule in RuleSource::ALL {
            assert_eq!(
                wage_paid(Some(10_000), rule),
                (100, Some(EscortStatus { wage: Some(100) }), Some(100)),
                "{rule:?}"
            );
        }
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
