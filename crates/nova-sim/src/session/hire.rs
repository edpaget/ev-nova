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
//! when the list is next built. The rolls are never saved. Whether an
//! unmet `Require` refuses a hire follows
//! [`Session::with_hire_require`].
//!
//! **Hiring** ([`Session::hire`]) takes what the terms charge from the
//! cash and adds the ship to the fleet, an escort as a ship captured is
//! (see [`escorts`](super::escorts)), its reserves full as its class holds
//! them, no standing order, and the daily wage it was hired at
//! ([`Escort::wage`]): a change made in the spaceport, so a save is due.
//! A refused hire changes nothing.
//!
//! The session keeps its [`HireTerms`] and [`ControlBits`] rather than
//! being passed them on each call, a deliberate departure from
//! [`BoardingRule`](crate::BoardingRule) and
//! [`Behaviour`](crate::Behaviour): see [`hire`](crate::hire).

use std::rc::Rc;

use super::Session;
use crate::board::MAX_ESCORTS;
use crate::catalog::ShipId;
use crate::chance::Chance;
use crate::hire::{Bar, ControlBits, HireList, HireRefusal, HireTerms, Hired};
use crate::landing::StellarFlags;
use crate::pilot::Escort;
use crate::rulebook::RuleSource;
use crate::traffic::table;
use crate::wares;

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

    /// This session with `bits` testing a ship's `Availability` for hire:
    /// [`NoControlBits`](crate::NoControlBits), which lets every one
    /// hold, by default.
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

    /// The ships for hire in the bar of the stellar the ship is docked at
    /// (see the module docs), drawing on `chance` each class's roll for
    /// the day not drawn yet; `None` when it has not landed at a stellar
    /// with a bar.
    pub fn escorts_for_hire(&mut self, chance: &mut dyn Chance) -> Option<HireList> {
        let stellar = self.landed?;
        let site = self.sites.iter().find(|site| site.id == stellar)?;
        if site.flags & StellarFlags::BAR == 0 {
            return None;
        }
        let bar = Bar {
            ships: &self.ships,
            site,
            contributed: wares::contributed(
                self.fields.contribute,
                &self.pilot.outfits,
                &self.outfits,
            ),
            cash: self.pilot.cash,
            room: self.pilot.escort_count() < MAX_ESCORTS,
            terms: &*self.hire_terms.0,
            control_bits: &*self.control_bits.0,
            hire_require: self.hire_require,
        };
        let rolls = &mut self.hire_rolls;
        Some(bar.list(|ship, percent| *rolls.entry(ship).or_insert_with(|| chance.fires(percent))))
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
        let site = self
            .landed
            .and_then(|stellar| self.sites.iter().find(|site| site.id == stellar))
            .copied()
            .ok_or(HireRefusal::NoBar)?;
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
            });
        });
        self.hire_rolls.remove(&ship);
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
    use crate::escort::{EscortCommand, EscortDuty, EscortGroup, slot_position};
    use crate::hire::{HireRow, NovaHire};
    use crate::reserves::Reserves;
    use crate::stats::ShipStats;
    use crate::targeting::TargetPick;
    use crate::testkit::{FAST, FakePilotCatalog, Scripted, catalog, jump, planet, ship};
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
        assert_eq!(session.land(), Ok(BAR_AT));
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

    /// Control bits where the expressions named do not hold.
    #[derive(Debug)]
    struct Refusing(&'static [&'static str]);

    impl ControlBits for Refusing {
        fn allows(&self, expression: &str) -> bool {
            !self.0.contains(&expression)
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
        assert_eq!(session.land(), Ok(BAR_AT));
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
            availability: "b99".to_owned(),
            flags3,
            ..hireable(id, 100)
        };
        let catalog = barred(vec![gated(129, 0x0100), gated(130, 0), hireable(131, 100)]);
        let mut session = landed(&catalog, 25_000).with_control_bits(Rc::new(Refusing(&["b99"])));
        let list = list(&mut session);
        assert_eq!(listed(&list), [130, 131]);
        assert_eq!(row(&list, 130).hire, Err(HireRefusal::NotForHire));
        assert_eq!(row(&list, 131).hire, Ok(()));
        let mut open = landed(&catalog, 25_000);
        assert_eq!(listed(&list_of(&mut open)), [129, 130, 131], "by default");
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
            availability: "b99".to_owned(),
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
        let mut session = landed(&catalog, 25_000).with_control_bits(Rc::new(Refusing(&["b99"])));
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
