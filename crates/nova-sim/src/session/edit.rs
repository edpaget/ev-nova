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
//!   kept, the bar's hire rolls and the shops' `BuyRandom` rolls drawn
//!   afresh, and the outfitter's opening started anew. The last system's
//!   traffic, fleet and fight are left behind, with the strikes, the
//!   target, the boarding, the hail and a gate's pending entry, and the
//!   next take-off populates the new system; a held `P` sound and the
//!   fighters out are kept. It enters the system by the relocation's row
//!   of the table in the `entry` module ([`Session::enter_system`]). It
//!   makes no sound and raises no message, and raises no `Landed`: the
//!   new stellar's offers are rolled at the next settling instead, if the
//!   ship is still landed there. It leaves the ship, its
//!   outfits, stats and reserves as they were. It tells
//!   [`SessionEvent::SystemChanged`](super::SessionEvent) into a system
//!   other than the one whose stellars are shown, and
//!   [`SessionEvent::StellarChanged`](super::SessionEvent) within it. It is
//!   refused ([`RelocateRefusal`]) in flight, a jump under way included,
//!   and for a system or stellar that is not there, a gate, or a stellar
//!   that cannot be landed on. A move to the stellar the ship is already
//!   docked at changes nothing but the save due, and tells nothing.
//! - A **landing** ([`Session::land_at`]) takes the ship to a stellar, as
//!   a move does, landed or in flight, and then docks it there as a
//!   landing does, raising [`StoryEvent::Landed`](crate::story::StoryEvent)
//!   once for the next settling ([`Session::settle_script`]) to dispatch:
//!   the offers rolled, and the active missions' landing hooks run, as
//!   the `missions` module says. It is what a storyline walker lands with,
//!   since a move raises no `Landed`. In flight it enters the system by
//!   the relocation's row too. At the stellar the ship is already docked
//!   at it moves nothing and docks again: the hooks run, and the offers,
//!   rolled there last, are not rolled again. It is refused as a move is,
//!   but in flight only while a jump is under way
//!   ([`RelocateRefusal::InFlight`]).
//! - A **reserve** ([`Session::set_reserve`]) is set within its gauge, at
//!   any time: no lower than none and no higher than the most the ship's
//!   stats hold, which the edit never changes, nor anything else the
//!   stats give (fuel regeneration among them). It is not damage nor a
//!   repair, so the ship's condition stays as it was.

use super::Session;
use super::entry::{Arrival, Entry, NoTraffic, Placement, ShipPlacement};
use crate::catalog::{PilotCatalog, StellarId, SystemId};
use crate::chance::NeverFires;
use crate::date::GameDate;
use crate::landing::is_dockable;
use crate::reserves::Reserve;

/// Why [`Session::relocate`] refused to move the pilot, or
/// [`Session::land_at`] to land it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum RelocateRefusal {
    /// The ship is in flight, or jumping: it must be landed to move. A
    /// landing is refused only while a jump is under way.
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
        self.out.save_due = true;
    }

    /// Sets the date to `date`, landed or in flight, with no days going by
    /// (see the module docs); a save is due.
    pub fn set_date(&mut self, date: GameDate) {
        self.pilot.date = date;
        self.out.save_due = true;
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
            self.out.save_due = true;
            return Ok(());
        }
        self.enter_at(catalog, system, stellar)?;
        self.run.offer_roll_due = Some(stellar);
        Ok(())
    }

    /// Lands the ship at `stellar` in `system`, read from `catalog`,
    /// landed or in flight (see the module docs).
    ///
    /// # Errors
    ///
    /// The first [`RelocateRefusal`] that applies, in its order; nothing
    /// changes and no save is due.
    pub fn land_at(
        &mut self,
        catalog: &(impl PilotCatalog + ?Sized),
        system: SystemId,
        stellar: StellarId,
    ) -> Result<(), RelocateRefusal> {
        if self.jump.is_some() {
            return Err(RelocateRefusal::InFlight);
        }
        let here = self
            .landed
            .is_some_and(|landed| (system, stellar) == (self.pilot.system, landed));
        if !here {
            self.enter_at(catalog, system, stellar)?;
        }
        self.dock(stellar);
        Ok(())
    }

    /// Moves the ship to `stellar` in `system`, read from `catalog`, and
    /// docks it there by the relocation's row of the entry table.
    fn enter_at(
        &mut self,
        catalog: &(impl PilotCatalog + ?Sized),
        system: SystemId,
        stellar: StellarId,
    ) -> Result<(), RelocateRefusal> {
        if !catalog.system_exists(system) {
            return Err(RelocateRefusal::NoSystem);
        }
        let sites = self.chart.read_sites(catalog, system);
        let site = sites
            .iter()
            .find(|site| site.id == stellar)
            .ok_or(RelocateRefusal::NoStellar)?;
        if !is_dockable(site) {
            return Err(RelocateRefusal::NotLandable);
        }
        let placement = Placement {
            ship: ShipPlacement::AtRest(site.position),
            stellar: Some(stellar),
            hold: false,
        };
        let arrival = Arrival {
            system,
            sites,
            placement,
        };
        self.enter_system(
            Entry::Relocate,
            arrival,
            catalog,
            &NoTraffic,
            &mut NeverFires,
        );
        Ok(())
    }

    /// Sets the ship's `reserve` to `amount`, held within its gauge (none
    /// for NaN), landed or in flight, and gives what it holds; a save is
    /// due.
    pub fn set_reserve(&mut self, reserve: Reserve, amount: f32) -> f32 {
        let gauge = self.pilot.reserves.get_mut(reserve);
        gauge.now = amount.max(0.0).min(gauge.max);
        self.out.save_due = true;
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
    use crate::gate::{GateKind, HYPERGATE, WORMHOLE};
    use crate::geometry::Vec2;
    use crate::hyperspace::{JUMP_FUEL, JumpRefusal};
    use crate::pilot::Escort;
    use crate::pilot::Pilot;
    use crate::reserves::Reserve;
    use crate::reserves::Reserves;
    use crate::save;
    use crate::session::SessionEvent;
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
        session.ship.condition = Condition::Disabled;
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
        session.pilot.events.insert(
            DisasterId(200),
            crate::market::ActiveEvent {
                days: 5,
                stellar: None,
            },
        );
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
        preparing.ship.player.velocity = Vec2::new(5.0, 0.0);
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
        session.ship.player.heading = 1.25;
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
    fn a_moves_dock_ends_a_warp_up_playing() {
        // The move docks by the entry table's `Reset::Dock`, not through
        // a landing, so it ends the warp-ups itself (see `warp::WarpUps`),
        // for the session only: no landing in the original halts them,
        // so it raises no halt.
        let catalog = atlas();
        let mut session = landed(&catalog);
        session.ship.warp.warp_ups.start(false, 364);
        session.ship.warp.warp_ups.start(true, 252);
        session.take_sounds();
        session
            .relocate(&catalog, SystemId(131), StellarId(140))
            .expect("moves");
        assert_eq!(session.ship.warp.warp_ups, crate::warp::WarpUps::default());
        assert_eq!(session.take_sounds(), [], "silently");
    }

    /// The original redraws the rolls only in its daily step
    /// (`_IncrementGameTime` @0xb7c4-0xb833), and no landing path writes
    /// them: a move, passing no day, keeps them, and only the redraw due
    /// on landing (`_DoShipyardDialog` @0x5f0f8) is applied.
    #[test]
    fn a_move_keeps_the_day_rolls_as_a_landing_does() {
        use crate::catalog::OutfitId;
        use crate::wares::DayRolls;
        let catalog = atlas();
        let mut session = landed(&catalog);
        session.port.hire_rolls.hold(ShipId(129));
        session.port.outfit_rolls.hold(OutfitId(128));
        session.port.ship_rolls.hold(ShipId(128));
        session.port.ship_rolls.hold(ShipId(129));
        session.port.ship_redraws.insert(ShipId(129));
        session.port.outfit_visit.bought = crate::outfitter::Bought::after_buying(
            &crate::testkit::outfit(128, &[(crate::outfit_effects::MAP, 1)]),
        );
        let (hire, outfits) = (
            session.port.hire_rolls.clone(),
            session.port.outfit_rolls.clone(),
        );
        session
            .relocate(&catalog, SystemId(131), StellarId(140))
            .expect("moves");
        session.chart.assert_docked_among_shown(session.landed);
        assert_eq!(session.port.hire_rolls, hire, "kept");
        assert_eq!(session.port.outfit_rolls, outfits, "kept");
        let mut kept = DayRolls::default();
        kept.hold(ShipId(128));
        assert_eq!(session.port.ship_rolls, kept, "the redraw due applied");
        assert!(session.port.ship_redraws.is_empty());
        assert_eq!(
            session.port.outfit_visit.bought,
            crate::outfitter::Bought::default()
        );
    }

    #[test]
    fn by_the_other_reading_a_move_clears_the_day_rolls_as_a_landing_does() {
        use crate::rulebook::{RuleKey, RuleSource, Rulebook};
        let catalog = atlas();
        let rules = Rulebook::default().with_override(RuleKey::DayRolls, RuleSource::Bible);
        let mut session = landed(&catalog).with_rules(rules);
        session.port.hire_rolls.hold(ShipId(129));
        session
            .port
            .outfit_rolls
            .hold(crate::catalog::OutfitId(128));
        session.port.ship_rolls.hold(ShipId(129));
        session.port.ship_redraws.insert(ShipId(129));
        session.port.outfit_visit.bought = crate::outfitter::Bought::after_buying(
            &crate::testkit::outfit(128, &[(crate::outfit_effects::MAP, 1)]),
        );
        session
            .relocate(&catalog, SystemId(131), StellarId(140))
            .expect("moves");
        session.chart.assert_docked_among_shown(session.landed);
        assert_eq!(session.port.hire_rolls, crate::wares::DayRolls::default());
        assert_eq!(session.port.outfit_rolls, crate::wares::DayRolls::default());
        assert_eq!(session.port.ship_rolls, crate::wares::DayRolls::default());
        assert!(session.port.ship_redraws.is_empty());
        assert_eq!(
            session.port.outfit_visit.bought,
            crate::outfitter::Bought::default()
        );
    }

    #[test]
    fn a_move_to_the_stellar_already_landed_on_changes_only_the_save_due() {
        let catalog = atlas();
        let mut session = landed(&catalog);
        session.plot_course(SystemId(132)).expect("a route");
        session.port.hire_rolls.hold(ShipId(129));
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
        session.chart.assert_docked_among_shown(session.landed);
        assert_eq!(session.landed(), Some(StellarId(129)));
        assert_eq!(session.player().position, Vec2::new(2000.0, 0.0));
    }

    #[test]
    fn a_move_tells_whether_the_system_or_only_the_stellar_changed() {
        let catalog = atlas();
        for ((system, stellar), told) in [
            ((131, 140), vec![SessionEvent::SystemChanged]),
            ((130, 129), vec![SessionEvent::StellarChanged]),
            ((130, 128), vec![]),
            ((131, 141), vec![]),
        ] {
            let mut session = landed(&catalog);
            let _ = session.relocate(&catalog, SystemId(system), StellarId(stellar));
            assert_eq!(session.take_events(), told, "{system} {stellar}");
            session.take_off(&catalog, &mut NeverFires);
            assert_eq!(session.take_events(), [], "{system} {stellar}: taken off");
        }
    }

    #[test]
    fn a_move_after_a_landed_move_is_told_against_the_system_shown() {
        let catalog = atlas();
        for ((system, stellar), told) in [
            ((131, 140), SessionEvent::SystemChanged),
            ((130, 129), SessionEvent::StellarChanged),
        ] {
            let mut session = landed(&catalog);
            session.run_set(
                &crate::control::SetExpr::parse("M131").expect("parses"),
                &mut NeverFires,
            );
            let _ = session.settle_script(&catalog, &mut NeverFires);
            assert_eq!(session.take_events(), []);
            session
                .relocate(&catalog, SystemId(system), StellarId(stellar))
                .expect("moves");
            session.chart.assert_docked_among_shown(session.landed);
            assert_eq!(session.take_events(), [told], "{system} {stellar}");
            session.take_off(&catalog, &mut NeverFires);
            assert_eq!(session.take_events(), [], "{system} {stellar}: taken off");
        }
    }

    #[test]
    fn after_a_move_taking_off_populates_the_new_system() {
        let catalog = atlas();
        let mut session = landed(&catalog);
        session
            .relocate(&catalog, SystemId(131), StellarId(140))
            .expect("moves");
        session.chart.assert_docked_among_shown(session.landed);
        session.take_off(&catalog, &mut NeverFires);
        session.tick_traffic(&catalog, &Peaceful, &mut NeverFires);
        assert_eq!(session.npcs().len(), 3);
    }

    /// `_ResetPlayer` disarms the timer (@0x1d73f), and relocation's
    /// `TrafficStep::Reset` builds a fresh traffic, which holds it
    /// disarmed. No flight can show it, as every take-off arms it again,
    /// so it is checked as state.
    #[test]
    fn a_relocation_disarms_an_armed_escort_strength_timer() {
        let catalog = atlas();
        let mut session = landed(&catalog);
        session.space.arm_strength_timer(&mut NeverFires);
        assert!(
            session.space.traffic.strength_armed(),
            "armed before the move"
        );
        session
            .relocate(&catalog, SystemId(131), StellarId(140))
            .expect("moves");
        assert!(!session.space.traffic.strength_armed(), "a move");
        let mut session = landed(&catalog);
        session
            .take_off(&catalog, &mut NeverFires)
            .expect("takes off");
        assert!(
            session.space.traffic.strength_armed(),
            "armed by the take-off"
        );
        session
            .land_at(&catalog, SystemId(131), StellarId(140))
            .expect("lands");
        assert!(
            !session.space.traffic.strength_armed(),
            "a landing from flight"
        );
    }

    #[test]
    fn a_moved_pilot_saved_flies_again_on_the_new_stellar() {
        let catalog = atlas();
        let mut session = landed(&catalog);
        session
            .relocate(&catalog, SystemId(131), StellarId(140))
            .expect("moves");
        session.chart.assert_docked_among_shown(session.landed);
        let saved = save::decode(&save::encode(session.pilot())).expect("loads");
        let flown = Session::fly(&catalog, saved).expect("flies");
        assert_eq!(flown.system(), SystemId(131));
        assert_eq!(crate::testkit::resumed_on(&flown), Some(StellarId(140)));
        assert_eq!(flown.player().position, Vec2::new(50.0, 60.0));
    }

    /// [`catalog`] with `missions`.
    fn with_missions(missions: Vec<crate::catalog::MissionRecord>) -> FakePilotCatalog {
        FakePilotCatalog {
            missions,
            ..catalog()
        }
    }

    /// A session of `catalog` landed on 128 and settled, every roll 1.
    fn docked(catalog: &FakePilotCatalog) -> Session {
        let mut session = Session::start(catalog).expect("starts");
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        let _ = session.settle_script(catalog, &mut NeverFires);
        session
    }

    /// Whether mission `id` is offered at the mission computer.
    fn offered(session: &Session, id: i16) -> bool {
        session
            .mission_offers(crate::mission::Venue::Computer)
            .iter()
            .any(|offer| offer.mission == crate::catalog::MissionId(id))
    }

    /// Mission `id`, offered at 128, returning to `to`, paying 1000 and
    /// setting b10 on success.
    fn errand(id: i16, to: i16) -> crate::catalog::MissionRecord {
        crate::catalog::MissionRecord {
            avail_stel: 128,
            return_stel: to,
            pay_val: 1000,
            on_success: crate::control::Script::parse("b10"),
            ..crate::testkit::mission(id)
        }
    }

    /// Whether bit `n` is set.
    fn bit_set(session: &Session, n: u16) -> bool {
        session
            .pilot()
            .control_bits()
            .get(crate::control::Bit::new(n).expect("a bit"))
    }

    /// The `Landed` events raised and not yet settled.
    fn landings(session: &Session) -> Vec<crate::story::StoryEvent> {
        session
            .story_raised()
            .into_iter()
            .filter(|event| matches!(event, crate::story::StoryEvent::Landed { .. }))
            .collect()
    }

    #[test]
    fn a_landing_at_another_stellar_raises_landed_once_and_rolls_its_offers() {
        let catalog = with_missions(vec![crate::catalog::MissionRecord {
            avail_stel: 129,
            ..crate::testkit::mission(200)
        }]);
        let mut session = docked(&catalog);
        assert!(!offered(&session, 200));
        assert_eq!(
            session.land_at(&catalog, SystemId(130), StellarId(129)),
            Ok(())
        );
        session.chart.assert_docked_among_shown(session.landed);
        assert_eq!(session.landed(), Some(StellarId(129)));
        assert_eq!(
            landings(&session),
            [crate::story::StoryEvent::Landed {
                stellar: StellarId(129)
            }]
        );
        assert!(session.take_save_due());
        let _ = session.settle_script(&catalog, &mut NeverFires);
        assert!(offered(&session, 200));
    }

    #[test]
    fn a_move_to_another_stellar_rolls_its_offers_as_it_settles() {
        let catalog = with_missions(vec![crate::catalog::MissionRecord {
            avail_stel: 129,
            ..crate::testkit::mission(200)
        }]);
        let mut session = docked(&catalog);
        assert_eq!(
            session.relocate(&catalog, SystemId(130), StellarId(129)),
            Ok(())
        );
        assert!(!offered(&session, 200));
        assert!(landings(&session).is_empty(), "a move raises no Landed");
        let _ = session.settle_script(&catalog, &mut NeverFires);
        assert!(offered(&session, 200));
    }

    #[test]
    fn a_move_to_the_stellar_docked_at_rolls_nothing() {
        let catalog = with_missions(vec![
            crate::testkit::mission(200),
            crate::testkit::mission(201),
        ]);
        let mut session = docked(&catalog);
        let slot = session
            .accept_mission(crate::catalog::MissionId(200), &mut NeverFires)
            .expect("accepted");
        // An abort forgets the offers' stellar, so only the early return
        // keeps the settling from rolling the visit's offers again.
        session
            .abort_mission(slot, &mut NeverFires)
            .expect("aborted");
        session
            .run
            .offers
            .set_roll(crate::catalog::MissionId(201), 77);
        assert_eq!(
            session.relocate(&catalog, SystemId(130), StellarId(128)),
            Ok(())
        );
        let _ = session.settle_script(&catalog, &mut NeverFires);
        assert_eq!(
            session.offer_rolls().roll(crate::catalog::MissionId(201)),
            Some(77),
            "the visit's offers not rolled again"
        );
    }

    #[test]
    fn a_move_then_a_take_off_rolls_nothing_as_it_settles() {
        let catalog = with_missions(vec![crate::catalog::MissionRecord {
            avail_stel: 129,
            ..crate::testkit::mission(200)
        }]);
        let mut session = docked(&catalog);
        assert_eq!(
            session.relocate(&catalog, SystemId(130), StellarId(129)),
            Ok(())
        );
        assert_eq!(
            session.take_off(&catalog, &mut NeverFires),
            Some(StellarId(129))
        );
        let _ = session.settle_script(&catalog, &mut NeverFires);
        assert!(!offered(&session, 200), "rolled only while landed there");
    }

    #[test]
    fn a_landing_at_a_missions_return_completes_it() {
        let catalog = with_missions(vec![errand(200, 140)]);
        let mut session = docked(&catalog);
        let cash = session.pilot().cash();
        let slot = session
            .accept_mission(crate::catalog::MissionId(200), &mut NeverFires)
            .expect("accepted");
        session
            .land_at(&catalog, SystemId(131), StellarId(140))
            .expect("lands");
        let _ = session.settle_script(&catalog, &mut NeverFires);
        assert_eq!(session.system(), SystemId(131));
        assert_eq!(session.pilot().missions().get(slot), None, "freed");
        assert_eq!(session.pilot().cash(), cash + 1000);
        assert!(bit_set(&session, 10));
    }

    #[test]
    fn landing_again_where_docked_completes_a_mission_returning_there_rolling_nothing() {
        let catalog = with_missions(vec![errand(200, 128), crate::testkit::mission(201)]);
        let mut session = docked(&catalog);
        let slot = session
            .accept_mission(crate::catalog::MissionId(200), &mut NeverFires)
            .expect("accepted");
        session
            .run
            .offers
            .set_roll(crate::catalog::MissionId(201), 77);
        session.drop_raised();
        assert_eq!(
            session.land_at(&catalog, SystemId(130), StellarId(128)),
            Ok(())
        );
        assert_eq!(
            landings(&session),
            [crate::story::StoryEvent::Landed {
                stellar: StellarId(128)
            }]
        );
        let _ = session.settle_script(&catalog, &mut NeverFires);
        assert_eq!(session.pilot().missions().get(slot), None, "completed");
        assert!(bit_set(&session, 10));
        assert_eq!(
            session.offer_rolls().roll(crate::catalog::MissionId(201)),
            Some(77),
            "the offers not rolled again"
        );
    }

    #[test]
    fn a_landing_from_flight_docks_at_the_stellar() {
        let catalog = with_missions(vec![crate::catalog::MissionRecord {
            avail_stel: 140,
            ..crate::testkit::mission(200)
        }]);
        for (system, stellar) in [(131, 140), (130, 129), (130, 128)] {
            let mut session = docked(&catalog);
            session
                .take_off(&catalog, &mut NeverFires)
                .expect("takes off");
            let _ = session.settle_script(&catalog, &mut NeverFires);
            session.ship.player.velocity = Vec2::new(5.0, 0.0);
            session.drop_raised();
            assert_eq!(
                session.land_at(&catalog, SystemId(system), StellarId(stellar)),
                Ok(()),
                "{stellar}"
            );
            session.chart.assert_docked_among_shown(session.landed);
            assert_eq!(session.system(), SystemId(system));
            assert_eq!(session.landed(), Some(StellarId(stellar)));
            assert_eq!(session.pilot().stellar(), Some(StellarId(stellar)));
            assert_eq!(session.player().velocity, Vec2::ZERO);
            assert_eq!(
                landings(&session),
                [crate::story::StoryEvent::Landed {
                    stellar: StellarId(stellar)
                }]
            );
            let _ = session.settle_script(&catalog, &mut NeverFires);
            assert_eq!(offered(&session, 200), stellar == 140, "{stellar}");
        }
    }

    fn assert_landing_refused(
        mut session: Session,
        catalog: &FakePilotCatalog,
        (system, stellar): (i16, i16),
        refusal: RelocateRefusal,
    ) {
        session.take_save_due();
        let before = session.clone();
        assert_eq!(
            session.land_at(catalog, SystemId(system), StellarId(stellar)),
            Err(refusal),
            "{system} {stellar}"
        );
        assert_eq!(session, before, "nothing changes");
        assert!(!session.take_save_due());
    }

    #[test]
    fn a_landing_where_a_move_is_refused_is_refused_alike() {
        let catalog = atlas();
        let flying =
            Session::fly(&catalog, Pilot::new(&catalog, "Ada").expect("starts")).expect("flies");
        // Boxed: two sessions side by side outgrow clippy's stack-array limit.
        for session in [Box::new(landed(&catalog)), Box::new(flying)] {
            for (to, refusal) in [
                ((132, 140), RelocateRefusal::NoSystem),
                ((999, 140), RelocateRefusal::NoSystem),
                ((131, 128), RelocateRefusal::NoStellar),
                ((131, 999), RelocateRefusal::NoStellar),
                ((131, 141), RelocateRefusal::NotLandable),
                ((131, 142), RelocateRefusal::NotLandable),
                ((131, 143), RelocateRefusal::NotLandable),
            ] {
                assert_landing_refused((*session).clone(), &catalog, to, refusal);
            }
        }
    }

    #[test]
    fn a_landing_while_a_jump_is_under_way_is_refused() {
        let catalog = atlas();
        let session =
            Session::fly(&catalog, Pilot::new(&catalog, "Ada").expect("starts")).expect("flies");
        let mut preparing = session.clone();
        preparing.plot_course(SystemId(131)).expect("a route");
        fly_out(&mut preparing);
        preparing.ship.player.velocity = Vec2::new(5.0, 0.0);
        preparing.begin_jump().expect("accepted");
        assert!(preparing.preparing_jump().is_some());
        assert_landing_refused(preparing, &catalog, (131, 140), RelocateRefusal::InFlight);
        let mut jumping = session;
        jumping.plot_course(SystemId(131)).expect("a route");
        fly_out(&mut jumping);
        begin_jump_now(&mut jumping).expect("jumps");
        assert_landing_refused(jumping, &catalog, (131, 140), RelocateRefusal::InFlight);
    }

    #[test]
    fn a_move_raises_no_landing() {
        let catalog = atlas();
        for (system, stellar) in [(131, 140), (130, 129), (130, 128)] {
            let mut session = landed(&catalog);
            session.drop_raised();
            session
                .relocate(&catalog, SystemId(system), StellarId(stellar))
                .expect("moves");
            assert_eq!(landings(&session), [], "{stellar}");
        }
    }

    /// [`atlas`], the player's ship carrying a blaster, outfit 250
    /// holding it.
    fn armed_atlas() -> FakePilotCatalog {
        use crate::catalog::{HullRecord, StockWeapon, WeaponId, WeaponRecord};
        use crate::testkit::{hull, weapon};
        FakePilotCatalog {
            weapons: vec![WeaponRecord {
                reload: 2,
                count: 30,
                speed: 2000,
                ..weapon(128)
            }],
            hulls: vec![HullRecord {
                weapons: vec![StockWeapon {
                    weapon: WeaponId(128),
                    count: 1,
                    ammo: 0,
                }],
                ..hull(128)
            }],
            outfits: vec![outfit(250, &[(crate::combat::armament::MOD_WEAPON, 128)])],
            ..atlas()
        }
    }

    #[test]
    fn a_move_lets_go_of_the_scene_and_the_fleet_and_keeps_a_held_sound_and_the_fighters_out() {
        let catalog = armed_atlas();
        let escort = |carried| Escort {
            ship: ShipId(129),
            reserves: Reserves::full(30.0, 45.0, 300.0),
            order: None,
            carried,
            wage: None,
            person: None,
        };
        let mut pilot = Pilot::new(&catalog, "Ada").expect("starts");
        pilot.escorts = vec![escort(false), escort(true)];
        let mut session = Session::fly(&catalog, pilot).expect("flies");
        session.populate(&catalog, &mut NeverFires);
        session.hold_fire(true, false);
        session.tick_combat(crate::combat::Rules::default(), &mut NeverFires);
        session.hold_fire(false, false);
        let combat = session.space.combat.clone();
        assert_eq!(combat.shots().len(), 1);
        land_now(&mut session).expect("lands");
        session.space.combat = combat;
        session.chart.nav = crate::navigation::NavMode::Stellar(Some(StellarId(129)));
        session.ship.warp.gate = Some(StellarId(129));
        session.stage_scene(session.npcs()[0].id);
        assert_eq!(session.space.escorts.fleet.len(), 2);
        session.run_set(
            &crate::control::SetExpr::parse("P300").expect("parses"),
            &mut NeverFires,
        );
        session.take_sounds();
        session.take_fighter_notes();
        session
            .relocate(&catalog, SystemId(131), StellarId(140))
            .expect("moves");
        session.chart.assert_docked_among_shown(session.landed);
        assert_eq!(session.shots(), []);
        assert_eq!(session.nav_target(), None);
        assert_eq!(session.ship.warp.gate, None);
        session.assert_scene_left("a relocation");
        assert_eq!(session.space.escorts.fleet, []);
        assert!(session.space.traffic_due);
        assert_eq!(session.pilot().escorts().len(), 2, "the fighter still out");
        assert_eq!(session.take_fighter_notes(), []);
        session.take_off(&catalog, &mut NeverFires);
        assert_eq!(session.take_sounds(), [crate::sound::SimSound::TookOff]);
        session.tick(crate::flight::Controls::default());
        assert_eq!(
            session.take_sounds(),
            [crate::sound::SimSound::Script {
                sound: crate::catalog::SoundId(300),
                exclusive: true,
            }]
        );
    }
}
