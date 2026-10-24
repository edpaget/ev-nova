//! The session's side of the spaceport's Recharge: whether its button is
//! enabled (`_DrawPlanetButtons` @0x297a3-0x297f4) and the refill
//! `_DoPortDialog` makes (@0x5fd5d-0x5fe25), as the [`recharge`] rules
//! say.
//!
//! The view can't know the cash or the tank, so it is told whether
//! Recharge is enabled ([`Session::recharge_enabled`]) after it is shown
//! and after each input it takes, before the next frame is drawn. The
//! button may thus lag one input behind a change the input made, which no
//! frame shows.
//!
//! The refill reads the landed stellar's domination from `Flags2` 0x0020
//! ([`market::DOMINATED`]), the start state of the original's +0x46 byte
//! (`_ResetPlayerRecord` @0x1dbcf-0x1dbef). Domination at run time is
//! not modelled yet (rdm:task/planet-domination).

use super::Session;
use crate::market;
use crate::recharge::{self, RechargeRefusal};
use crate::rulebook::RuleKey;

impl Session {
    /// Fills the tank at the stellar the ship is docked at, as
    /// `_DoPortDialog` does (see the [`recharge`] rules and
    /// [`RuleKey::RechargeDebt`]), and gives the price paid, which may be
    /// 0 or, for a tank above capacity, a refund. A refill that changes
    /// the tank or the cash is a change made in the spaceport, so a save
    /// is due. When the ship is not landed where fuel is sold nothing
    /// changes, and the refusal says so.
    pub fn recharge(&mut self) -> Result<i64, RechargeRefusal> {
        let stellar = self.mode.landed().ok_or(RechargeRefusal::NoFuel)?;
        let site = self
            .chart
            .sites
            .iter()
            .find(|site| site.id == stellar)
            .filter(|site| recharge::sells_fuel(site.flags))
            .ok_or(RechargeRefusal::NoFuel)?;
        let fill = recharge::fill(
            self.pilot.reserves.fuel,
            self.pilot.cash,
            site.flags2 & market::DOMINATED != 0,
            self.setup.rules.source_for(RuleKey::RechargeDebt),
        );
        let mut after = self.pilot.clone();
        recharge::settle(&mut after, fill);
        if after != self.pilot {
            self.transact(|pilot| *pilot = after);
        }
        Ok(fill.price)
    }

    /// Whether the spaceport's Recharge button is enabled: the ship is
    /// landed where fuel is sold, with cash above 0 and room in its tank
    /// ([`recharge::enabled`]). See the module docs for when the view is
    /// told.
    #[must_use]
    pub fn recharge_enabled(&self) -> bool {
        self.mode
            .landed()
            .and_then(|stellar| self.chart.sites.iter().find(|site| site.id == stellar))
            .is_some_and(|site| {
                recharge::enabled(site.flags, self.pilot.reserves.fuel, self.pilot.cash)
            })
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::super::tests::{exchange, half_empty_at_port};
    use super::super::*;
    use crate::chance::NeverFires;
    use crate::landing::StellarFlags;
    use crate::testkit::land_now;

    /// `session.recharge()` gives `price`, leaving the tank at `fuel` and
    /// the cash at `cash`, a save due or not by `saved`.
    fn recharged(mut session: Session, price: i64, fuel: f32, cash: i64, saved: bool) {
        assert_eq!(session.recharge(), Ok(price));
        assert_eq!(session.reserves().fuel.now, fuel);
        assert_eq!(session.pilot().cash(), cash);
        assert_eq!(session.take_save_due(), saved);
    }

    #[test]
    fn a_full_tank_recharges_nothing_and_makes_no_save_due() {
        let mut session = half_empty_at_port();
        session.pilot.reserves.fuel.now = 300.0;
        let before = session.clone();
        assert_eq!(session.recharge(), Ok(0));
        assert_eq!(session, before, "nothing changes");
        assert!(!session.take_save_due());
    }

    #[test]
    fn too_little_cash_fills_as_many_units_as_it_buys() {
        let mut session = half_empty_at_port();
        session.pilot.cash = 100;
        recharged(session, 100, 250.0, 0, true);
    }

    #[test]
    fn a_part_unit_is_truncated_even_with_no_cash() {
        let mut session = half_empty_at_port();
        session.pilot.reserves.fuel.now = 149.25;
        session.pilot.cash = 0;
        recharged(session, 0, 149.0, 0, true);
    }

    /// A session landed at planet 128 dominated from the start
    /// (`Flags2` 0x0020), with half a tank and `cash`.
    fn dominated(cash: i64) -> Session {
        let mut catalog = exchange();
        catalog.sites[0].1[0].flags2 |= market::DOMINATED;
        let mut session = Session::start(&catalog).expect("starts");
        land_now(&mut session).expect("lands");
        session.pilot.reserves.fuel.now = 150.0;
        session.pilot.cash = cash;
        session.take_save_due();
        session
    }

    #[test]
    fn a_dominated_stellar_fills_the_whole_tank_free() {
        recharged(dominated(0), 0, 300.0, 0, true);
        recharged(dominated(-50), 0, 300.0, -50, true);
    }

    fn in_debt(rules: Rulebook) -> Session {
        let mut session = half_empty_at_port().with_rules(rules);
        session.pilot.cash = -50;
        session
    }

    #[test]
    fn by_the_engine_a_debt_is_cleared_with_fuel() {
        recharged(in_debt(Rulebook::default()), -50, 100.0, 0, true);
    }

    #[test]
    fn by_the_other_reading_a_debt_stands_and_nothing_fills() {
        let rules = Rulebook::default().with_override(RuleKey::RechargeDebt, RuleSource::Bible);
        recharged(in_debt(rules), 0, 150.0, -50, false);
    }

    #[test]
    fn recharge_is_enabled_landed_where_fuel_is_sold_with_cash_and_room() {
        let session = half_empty_at_port();
        assert!(session.recharge_enabled());
        let mut full = half_empty_at_port();
        full.pilot.reserves.fuel.now = 300.0;
        assert!(!full.recharge_enabled(), "full");
        for cash in [0, -1] {
            let mut broke = half_empty_at_port();
            broke.pilot.cash = cash;
            assert!(!broke.recharge_enabled(), "{cash}");
        }
        let mut flying = half_empty_at_port();
        flying.take_off(&exchange(), &mut NeverFires);
        assert!(!flying.recharge_enabled(), "in flight");
        let mut catalog = exchange();
        catalog.sites[0].1[0].flags |= StellarFlags::UNINHABITED;
        let mut session = Session::start(&catalog).expect("starts");
        land_now(&mut session).expect("lands");
        session.pilot.reserves.fuel.now = 150.0;
        assert!(!session.recharge_enabled(), "no fuel sold");
    }

    #[test]
    fn dominated_or_not_recharge_needs_cash_to_be_enabled() {
        assert!(!dominated(0).recharge_enabled());
        assert!(dominated(1).recharge_enabled());
    }
}
