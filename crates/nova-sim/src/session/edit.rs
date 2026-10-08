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
//! - A **reserve** ([`Session::set_reserve`]) is set within its gauge, at
//!   any time: no lower than none and no higher than the most the ship's
//!   stats hold, which the edit never changes, nor anything else the
//!   stats give (fuel regeneration among them). It is not damage nor a
//!   repair, so the ship's condition stays as it was.

use super::Session;
use crate::date::GameDate;
use crate::reserves::Reserve;

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
    use crate::catalog::{DisasterId, OutfitId, ShipId, SystemId};
    use crate::combat::hull::Condition;
    use crate::date::GameDate;
    use crate::hyperspace::{JUMP_FUEL, JumpRefusal};
    use crate::pilot::Escort;
    use crate::pilot::Pilot;
    use crate::reserves::Reserve;
    use crate::reserves::Reserves;
    use crate::save;
    use crate::stats::MORE_FUEL;
    use crate::testkit::{FakePilotCatalog, catalog, fly_out, land_now, outfit};

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
}
