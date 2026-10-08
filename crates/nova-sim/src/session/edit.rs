//! Plain edits of the session's pilot, for tests and the developer tools:
//! putting a pilot into a given state without flying it there.
//!
//! Each edit keeps the session consistent with itself, and makes a save
//! due, as a change made in the spaceport does; a refused one changes
//! nothing. None asks the game's rules (cash, legal records or a
//! stellar's `MinStatus`): these are overrides.
//!
//! - **Credits** ([`Session::set_credits`]) and the **date**
//!   ([`Session::set_date`]) are set as given, at any time. Setting the
//!   date lets no days go by: no planetary event steps and no escort is
//!   paid, and an earlier date is allowed.
//! - A **move** ([`Session::relocate`]) takes a landed pilot to a stellar
//!   of any system and docks it there, as if it had landed: the system
//!   explored, its stellars read, the course and the navigation target
//!   cleared, the ship at rest at the stellar's centre with its heading
//!   kept, and the bar's hire rolls drawn afresh. The last system's
//!   traffic and fight are left behind, and the next take-off populates
//!   the new system. It makes no sound and raises no message, and leaves
//!   the ship, its outfits, stats and reserves as they were. It is
//!   refused ([`RelocateRefusal`]) in flight, a jump under way included,
//!   and for a system or stellar that is not there, a gate, or a stellar
//!   that cannot be landed on. A move to the stellar the ship is already
//!   docked at changes nothing but the save due.
//! - A **reserve** ([`Session::set_reserve`]) is set within its gauge, at
//!   any time: no lower than none and no higher than the most the ship's
//!   stats hold, which the edit never changes, nor anything else the
//!   stats give (fuel regeneration among them). It is not damage nor a
//!   repair, so the ship's condition stays as it was.

use super::Session;
use crate::catalog::{PilotCatalog, StellarId, SystemId};
use crate::date::GameDate;
use crate::gate::GateKind;
use crate::geometry::Vec2;
use crate::landing::is_landable;
use crate::reserves::Reserve;
use crate::traffic::Traffic;

/// Why [`Session::relocate`] refused to move the pilot.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum RelocateRefusal {
    /// The ship is in flight, or jumping: it must be landed to move.
    #[error("the ship must be landed to move")]
    InFlight,
    /// The system does not exist.
    #[error("no such system")]
    NoSystem,
    /// The stellar is not one of the system's.
    #[error("no such stellar in that system")]
    NoStellar,
    /// The stellar is a hypergate or wormhole, or cannot be landed on.
    #[error("that stellar cannot be landed on")]
    NotLandable,
}

impl Session {
    /// Sets the pilot's cash to `credits`, landed or in flight; a save is
    /// due.
    pub fn set_credits(&mut self, credits: i64) {
        self.pilot.cash = credits;
        self.save_due = true;
    }

    /// Sets the date to `date`, landed or in flight, with no days going by
    /// (see the module docs); a save is due.
    pub fn set_date(&mut self, date: GameDate) {
        self.pilot.date = date;
        self.save_due = true;
    }

    /// Moves the landed pilot to `stellar` in `system`, read from
    /// `catalog`, and docks it there (see the module docs).
    ///
    /// # Errors
    ///
    /// The first [`RelocateRefusal`] that applies, in its order; nothing
    /// changes and no save is due.
    pub fn relocate(
        &mut self,
        catalog: &(impl PilotCatalog + ?Sized),
        system: SystemId,
        stellar: StellarId,
    ) -> Result<(), RelocateRefusal> {
        let Some(landed) = self.landed else {
            return Err(RelocateRefusal::InFlight);
        };
        if (system, stellar) == (self.pilot.system, landed) {
            self.save_due = true;
            return Ok(());
        }
        if !catalog.system_exists(system) {
            return Err(RelocateRefusal::NoSystem);
        }
        let sites = catalog.landing_sites(system);
        let site = sites
            .iter()
            .find(|site| site.id == stellar)
            .ok_or(RelocateRefusal::NoStellar)?;
        if GateKind::of(site.flags2).is_some() || !is_landable(site) {
            return Err(RelocateRefusal::NotLandable);
        }
        self.player.position = site.position;
        self.player.velocity = Vec2::ZERO;
        let pilot = &mut self.pilot;
        pilot.system = system;
        pilot.stellar = Some(stellar);
        pilot.explore(system);
        pilot.course.clear();
        self.sites = sites;
        self.landed = Some(stellar);
        self.nav_target = None;
        self.gate = None;
        self.traffic = Traffic::new();
        self.traffic_due = true;
        self.fleet.clear();
        self.combat.clear();
        self.strikes.clear();
        self.target = None;
        self.aboard = None;
        self.talk = None;
        self.hire_rolls.clear();
        self.save_due = true;
        Ok(())
    }

    /// Sets the ship's `reserve` to `amount`, held within its gauge (none
    /// for NaN), landed or in flight, and gives what it holds; a save is
    /// due.
    pub fn set_reserve(&mut self, reserve: Reserve, amount: f32) -> f32 {
        let gauge = self.pilot.reserves.get_mut(reserve);
        gauge.now = amount.max(0.0).min(gauge.max);
        self.save_due = true;
        gauge.now
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::ai::Peaceful;
    use crate::catalog::{
        DisasterId, DudeId, DudeRecord, GovtId, LandingSite, OutfitId, ShipId, StellarId, SystemId,
        SystemTraffic,
    };
    use crate::chance::NeverFires;
    use crate::combat::hull::Condition;
    use crate::date::GameDate;
    use crate::gate::{HYPERGATE, WORMHOLE};
    use crate::geometry::Vec2;
    use crate::hyperspace::{JUMP_FUEL, JumpRefusal};
    use crate::pilot::Escort;
    use crate::pilot::Pilot;
    use crate::reserves::Reserve;
    use crate::reserves::Reserves;
    use crate::save;
    use crate::stats::MORE_FUEL;
    use crate::testkit::{
        FAST, FakePilotCatalog, begin_jump_now, catalog, fly_out, land_now, outfit, planet, ship,
    };

    fn session() -> Session {
        let catalog = catalog();
        Session::fly(&catalog, Pilot::new(&catalog, "Ada").expect("starts")).expect("flies")
    }

    #[test]
    fn credits_are_set_in_flight_and_landed_and_a_save_is_due() {
        let mut session = session();
        session.set_credits(12_345);
        assert_eq!(session.pilot().cash(), 12_345);
        assert!(session.take_save_due());
        land_now(&mut session).expect("lands");
        session.take_save_due();
        session.set_credits(-7);
        assert_eq!(session.pilot().cash(), -7);
        assert!(session.take_save_due());
    }

    #[test]
    fn edited_credits_are_saved() {
        let mut session = session();
        session.set_credits(9_876_543_210);
        let saved = save::decode(&save::encode(session.pilot())).expect("loads");
        assert_eq!(saved.cash(), 9_876_543_210);
    }

    #[test]
    fn a_reserve_is_set_within_its_gauge_and_gives_what_it_holds() {
        let mut session = session();
        let stats = session.stats();
        for (reserve, max) in [
            (Reserve::Shield, stats.shield),
            (Reserve::Armor, stats.armor),
            (Reserve::Fuel, stats.fuel),
        ] {
            assert_eq!(session.set_reserve(reserve, 10.0), 10.0, "{reserve:?}");
            assert_eq!(session.reserves().get(reserve).now, 10.0, "{reserve:?}");
            assert_eq!(
                session.set_reserve(reserve, max + 500.0),
                max,
                "{reserve:?}"
            );
            assert_eq!(session.set_reserve(reserve, -3.0), 0.0, "{reserve:?}");
            assert_eq!(session.set_reserve(reserve, f32::NAN), 0.0, "{reserve:?}");
            assert_eq!(session.reserves().get(reserve).now, 0.0, "{reserve:?}");
            assert_eq!(session.reserves().get(reserve).max, max, "{reserve:?}");
        }
        assert_eq!(session.stats(), stats, "the stats are untouched");
    }

    #[test]
    fn each_reserve_edit_changes_only_its_own_gauge() {
        let mut session = session();
        let full = session.reserves();
        session.set_reserve(Reserve::Shield, 1.0);
        assert_eq!(session.reserves().armor, full.armor);
        assert_eq!(session.reserves().fuel, full.fuel);
        session.set_reserve(Reserve::Armor, 2.0);
        assert_eq!(session.reserves().shield.now, 1.0);
        assert_eq!(session.reserves().fuel, full.fuel);
        session.set_reserve(Reserve::Fuel, 3.0);
        assert_eq!(session.reserves().shield.now, 1.0);
        assert_eq!(session.reserves().armor.now, 2.0);
    }

    #[test]
    fn a_tank_outfits_capacity_is_the_fuel_edits_cap() {
        let catalog = FakePilotCatalog {
            outfits: vec![outfit(300, &[(MORE_FUEL, 100)])],
            ..catalog()
        };
        let mut pilot = Pilot::new(&catalog, "Ada").expect("starts");
        pilot.outfits.insert(OutfitId(300), 1);
        let mut session = Session::fly(&catalog, pilot).expect("flies");
        assert_eq!(session.set_reserve(Reserve::Fuel, 1000.0), 400.0);
        assert_eq!(session.reserves().fuel.max, 400.0);
    }

    #[test]
    fn the_fuel_set_decides_whether_the_ship_can_jump() {
        let mut session = session();
        session.plot_course(SystemId(131)).expect("a route");
        fly_out(&mut session);
        session.set_reserve(Reserve::Fuel, 0.0);
        assert_eq!(session.begin_jump(), Err(JumpRefusal::NoFuel { fuel: 0.0 }));
        session.set_reserve(Reserve::Fuel, JUMP_FUEL);
        assert_eq!(session.begin_jump(), Ok(SystemId(131)));
    }

    #[test]
    fn a_reserve_edit_neither_disables_nor_repairs_and_a_save_is_due() {
        let mut session = session();
        session.set_reserve(Reserve::Armor, 0.0);
        session.set_reserve(Reserve::Shield, 0.0);
        assert_eq!(session.player_condition(), Condition::Intact);
        assert!(session.take_save_due());
        session.condition = Condition::Disabled;
        session.set_reserve(Reserve::Armor, 1000.0);
        assert_eq!(session.player_condition(), Condition::Disabled);
        assert!(session.take_save_due());
    }

    fn date(year: i32, month: u8, day: u8) -> GameDate {
        GameDate::new(year, month, day).expect("a date")
    }

    #[test]
    fn the_date_is_set_later_or_earlier_and_reads_as_set() {
        let mut session = session();
        session.set_date(date(1180, 2, 29));
        assert_eq!(session.date(), date(1180, 2, 29));
        assert_eq!(session.date_text(), "February 29, 1180 NC");
        assert!(session.take_save_due());
        session.set_date(date(1100, 1, 1));
        assert_eq!(session.date(), date(1100, 1, 1));
        assert!(session.take_save_due());
        let saved = save::decode(&save::encode(session.pilot())).expect("loads");
        assert_eq!(saved.date(), date(1100, 1, 1));
    }

    #[test]
    fn setting_the_date_lets_no_days_go_by() {
        let mut session = session();
        session.pilot.events.insert(DisasterId(200), 5);
        session.pilot.escorts.push(Escort {
            ship: ShipId(128),
            reserves: Reserves::full(30.0, 45.0, 300.0),
            order: None,
            carried: false,
            wage: Some(100),
            person: None,
        });
        session.set_credits(1000);
        session.set_date(date(1178, 6, 23));
        assert_eq!(
            session.pilot().events().collect::<Vec<_>>(),
            [(DisasterId(200), 5)],
            "no event steps"
        );
        assert_eq!(session.pilot().cash(), 1000, "no wages paid");
        assert_eq!(session.take_pay_notes(), []);
        assert_eq!(session.pilot().escort_count(), 1);
    }

    /// A system of `avg` ships of düde 128, ship 129, for govt 140.
    fn traffic(system: i16, avg: i16) -> (SystemId, SystemTraffic) {
        let mut dude_types = [(-1, 0); 8];
        dude_types[0] = (128, 100);
        (
            SystemId(system),
            SystemTraffic {
                dude_types,
                avg_ships: avg,
                persons: Default::default(),
            },
        )
    }

    /// [`catalog`] with traffic, two ships in 130 and three in 131, and
    /// in 131 a planet, 140, off the centre; a hypergate, 141; a
    /// wormhole, 142; and a stellar that cannot be landed on, 143.
    fn atlas() -> FakePilotCatalog {
        let gate = |id, flags2| LandingSite {
            flags2,
            ..planet(id, 0.0, 500.0)
        };
        let base = catalog();
        let mut sites = base.sites.clone();
        sites[1].1 = vec![
            planet(140, 50.0, 60.0),
            gate(141, HYPERGATE),
            gate(142, WORMHOLE),
            LandingSite {
                flags: 0,
                ..planet(143, -300.0, 0.0)
            },
        ];
        FakePilotCatalog {
            sites,
            traffic: vec![traffic(130, 2), traffic(131, 3)],
            dudes: vec![(
                DudeId(128),
                DudeRecord {
                    ai_type: 1,
                    govt: Some(GovtId(140)),
                    ships: vec![(ShipId(129), 1)],
                    booty: 0,
                    info_types: 0,
                },
            )],
            ship_records: vec![ship(129, FAST)],
            ..base
        }
    }

    /// A session landed on planet 128 in system 130, among its traffic.
    fn landed(catalog: &FakePilotCatalog) -> Session {
        let mut session =
            Session::fly(catalog, Pilot::new(catalog, "Ada").expect("starts")).expect("flies");
        session.populate(catalog, &mut NeverFires);
        land_now(&mut session).expect("lands");
        session.take_save_due();
        session.take_sounds();
        session
    }

    fn assert_refused(
        mut session: Session,
        catalog: &FakePilotCatalog,
        (system, stellar): (i16, i16),
        refusal: RelocateRefusal,
    ) {
        let before = session.clone();
        assert_eq!(
            session.relocate(catalog, SystemId(system), StellarId(stellar)),
            Err(refusal)
        );
        assert_eq!(session, before, "nothing changes");
        assert!(!session.take_save_due());
    }

    #[test]
    fn a_move_in_flight_is_refused() {
        let catalog = atlas();
        let session =
            Session::fly(&catalog, Pilot::new(&catalog, "Ada").expect("starts")).expect("flies");
        assert_refused(
            session.clone(),
            &catalog,
            (131, 140),
            RelocateRefusal::InFlight,
        );
        let mut preparing = session.clone();
        preparing.plot_course(SystemId(131)).expect("a route");
        fly_out(&mut preparing);
        preparing.player.velocity = Vec2::new(5.0, 0.0);
        preparing.begin_jump().expect("accepted");
        assert!(preparing.preparing_jump().is_some());
        assert_refused(preparing, &catalog, (131, 140), RelocateRefusal::InFlight);
        let mut jumping = session;
        jumping.plot_course(SystemId(131)).expect("a route");
        fly_out(&mut jumping);
        begin_jump_now(&mut jumping).expect("jumps");
        assert_refused(jumping, &catalog, (131, 140), RelocateRefusal::InFlight);
    }

    #[test]
    fn a_move_to_a_system_or_stellar_that_is_not_there_is_refused() {
        let catalog = atlas();
        let session = landed(&catalog);
        assert_refused(
            session.clone(),
            &catalog,
            (132, 140),
            RelocateRefusal::NoSystem,
        );
        assert_refused(
            session.clone(),
            &catalog,
            (999, 140),
            RelocateRefusal::NoSystem,
        );
        assert_refused(
            session.clone(),
            &catalog,
            (131, 128),
            RelocateRefusal::NoStellar,
        );
        assert_refused(session, &catalog, (131, 999), RelocateRefusal::NoStellar);
    }

    #[test]
    fn a_move_to_a_gate_or_a_stellar_that_cannot_be_landed_on_is_refused() {
        let catalog = atlas();
        let session = landed(&catalog);
        for stellar in [141, 142, 143] {
            assert_refused(
                session.clone(),
                &catalog,
                (131, stellar),
                RelocateRefusal::NotLandable,
            );
        }
    }

    #[test]
    fn each_refusal_says_why() {
        assert_eq!(
            RelocateRefusal::InFlight.to_string(),
            "the ship must be landed to move"
        );
        assert_eq!(RelocateRefusal::NoSystem.to_string(), "no such system");
        assert_eq!(
            RelocateRefusal::NoStellar.to_string(),
            "no such stellar in that system"
        );
        assert_eq!(
            RelocateRefusal::NotLandable.to_string(),
            "that stellar cannot be landed on"
        );
    }

    #[test]
    fn a_move_docks_the_ship_at_the_stellar_in_the_new_system() {
        let catalog = atlas();
        let mut session = landed(&catalog);
        session.plot_course(SystemId(132)).expect("a route");
        session.player.heading = 1.25;
        let (stats, reserves) = (session.stats(), session.reserves());
        assert_eq!(session.npcs().len(), 2);
        assert_eq!(
            session.relocate(&catalog, SystemId(131), StellarId(140)),
            Ok(())
        );
        assert_eq!(session.system(), SystemId(131));
        assert_eq!(session.pilot().stellar(), Some(StellarId(140)));
        assert_eq!(session.landed(), Some(StellarId(140)));
        assert!(session.pilot().has_explored(SystemId(131)));
        assert_eq!(session.course(), []);
        assert_eq!(session.player().position, Vec2::new(50.0, 60.0));
        assert_eq!(session.player().velocity, Vec2::ZERO);
        assert_eq!(session.player().heading, 1.25, "heading kept");
        assert_eq!(session.nav_target(), None);
        assert_eq!(session.npcs(), [], "the last system's traffic is gone");
        assert_eq!(session.target(), None);
        assert_eq!((session.stats(), session.reserves()), (stats, reserves));
        assert_eq!(session.gate_kind(StellarId(141)), Some(GateKind::Hypergate));
        assert!(session.market().is_none());
        assert!(session.take_save_due());
        assert_eq!(session.take_sounds(), [], "silently");
        assert_eq!(session.take_messages(), []);
    }

    #[test]
    fn a_move_clears_the_hire_rolls_as_a_landing_does() {
        let catalog = atlas();
        let mut session = landed(&catalog);
        session.hire_rolls.insert(ShipId(129), true);
        session
            .relocate(&catalog, SystemId(131), StellarId(140))
            .expect("moves");
        assert!(session.hire_rolls.is_empty());
    }

    #[test]
    fn a_move_to_the_stellar_already_landed_on_changes_only_the_save_due() {
        let catalog = atlas();
        let mut session = landed(&catalog);
        session.plot_course(SystemId(132)).expect("a route");
        session.hire_rolls.insert(ShipId(129), true);
        let before = session.clone();
        assert_eq!(
            session.relocate(&catalog, SystemId(130), StellarId(128)),
            Ok(())
        );
        assert!(session.take_save_due());
        assert_eq!(session, before, "the course, rolls and traffic kept");
        assert_eq!(session.course(), [SystemId(131), SystemId(132)]);
    }

    #[test]
    fn a_move_to_another_stellar_of_the_same_system_docks_there() {
        let catalog = atlas();
        let mut session = landed(&catalog);
        session
            .relocate(&catalog, SystemId(130), StellarId(129))
            .expect("moves");
        assert_eq!(session.landed(), Some(StellarId(129)));
        assert_eq!(session.player().position, Vec2::new(2000.0, 0.0));
    }

    #[test]
    fn after_a_move_taking_off_populates_the_new_system() {
        let catalog = atlas();
        let mut session = landed(&catalog);
        session
            .relocate(&catalog, SystemId(131), StellarId(140))
            .expect("moves");
        session.take_off();
        session.tick_traffic(&catalog, &Peaceful, &mut NeverFires);
        assert_eq!(session.npcs().len(), 3);
    }

    #[test]
    fn a_moved_pilot_saved_flies_again_docked_at_the_new_stellar() {
        let catalog = atlas();
        let mut session = landed(&catalog);
        session
            .relocate(&catalog, SystemId(131), StellarId(140))
            .expect("moves");
        let saved = save::decode(&save::encode(session.pilot())).expect("loads");
        let flown = Session::fly(&catalog, saved).expect("flies");
        assert_eq!(flown.system(), SystemId(131));
        assert_eq!(flown.landed(), Some(StellarId(140)));
        assert_eq!(flown.player().position, Vec2::new(50.0, 60.0));
    }
}
