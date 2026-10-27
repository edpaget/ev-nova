//! The session's side of `_DoAutoRefuel` (@0x41bc5-0x41c8c): the top-up
//! an auto-refueller makes as the ship docks at an inhabited stellar, as
//! the [`auto_refuel`](crate::auto_refuel) rules say.
//!
//! `Session::dock` runs it after the landing's precalced reset and before
//! the port's prices and `StoryEvent::Landed`, as `_PlayerLandOnStellar`
//! calls it (@0x632c5) after its reset (@0x63211) and before
//! `_DoPortDialog` (@0x63381). The original then marks the status bar for
//! a redraw (@0x41c6e-0x41c81, `_RedrawStatus` @0x632f4); the view draws
//! it from the session's state every frame, and the top-up is settled in
//! the same call that docks, before any frame or the spaceport's dialog,
//! so no boundary is crossed and nothing more is needed.

use super::{Setup, Statics};
use crate::auto_refuel::{carries_auto_refueller, fill};
use crate::outfitter::outfit_mods;
use crate::pilot::Pilot;
use crate::recharge;
use crate::rulebook::RuleKey;

/// Tops up `pilot`'s tank when it carries an auto-refueller
/// ([`AUTO_REFUELLER`](crate::auto_refuel::AUTO_REFUELLER)) and the tank is below capacity, charging its cash
/// a credit a unit, the debt branch read by [`RuleKey::RechargeDebt`].
/// It makes no sound. The caller runs it only at an inhabited stellar.
pub(super) fn auto_refuel(pilot: &mut Pilot, statics: &Statics, setup: &Setup) {
    if !carries_auto_refueller(&outfit_mods(&pilot.outfits, &statics.outfits)) {
        return;
    }
    let debt = setup.rules.source_for(RuleKey::RechargeDebt);
    if let Some(top_up) = fill(pilot.reserves.fuel, pilot.cash, debt) {
        recharge::settle(pilot, top_up);
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::super::Session;
    use super::super::tests::exchange;
    use super::*;
    use crate::auto_refuel::AUTO_REFUELLER;
    use crate::catalog::OutfitId;
    use crate::landing::StellarFlags;
    use crate::market;
    use crate::reserves::Gauge;
    use crate::rulebook::{RuleSource, Rulebook};
    use crate::sound::SimSound;
    use crate::testkit::{FakePilotCatalog, land_now, outfit};

    /// The auto-refueller's `oütf` ID in [`refuelling`].
    const REFUELLER: OutfitId = OutfitId(186);

    /// The exchange's catalog with an auto-refueller outfit, its
    /// [`AUTO_REFUELLER`] mod fourth, as the stock `;T` outfits have it.
    fn refuelling() -> FakePilotCatalog {
        let mut catalog = exchange();
        catalog.outfits.push(outfit(
            REFUELLER.0,
            &[(1, 0), (2, 0), (3, 0), (AUTO_REFUELLER, 0)],
        ));
        catalog
    }

    /// A session about to land at planet 128 with its fuel at `fuel` and
    /// `cash`, carrying an auto-refueller or not by `carries`, read by
    /// `rules`.
    fn about_to_land(
        catalog: &FakePilotCatalog,
        carries: bool,
        fuel: f32,
        cash: i64,
        rules: Rulebook,
    ) -> Session {
        let mut session = Session::start(catalog).expect("starts").with_rules(rules);
        if carries {
            session.pilot.outfits.insert(REFUELLER, 1);
        }
        session.pilot.reserves.fuel.now = fuel;
        session.pilot.cash = cash;
        let _ = session.take_sounds();
        session
    }

    fn landed(mut session: Session) -> Session {
        land_now(&mut session).expect("lands");
        session
    }

    #[test]
    fn landing_with_an_auto_refueller_tops_up_the_tank_at_a_credit_a_unit() {
        let session = landed(about_to_land(
            &refuelling(),
            true,
            50.5,
            1000,
            Rulebook::default(),
        ));
        let max = session.reserves().fuel.max;
        assert_eq!(session.reserves().fuel, Gauge::full(max));
        #[allow(clippy::cast_possible_truncation)]
        let units = (max - 50.0) as i64;
        assert_eq!(session.pilot().cash(), 1000 - units);
        assert!(
            !session.recharge_enabled(),
            "the tank full, Recharge has nothing to do"
        );
    }

    #[test]
    fn landing_without_one_leaves_the_tank_and_the_cash() {
        let session = landed(about_to_land(
            &refuelling(),
            false,
            50.5,
            1000,
            Rulebook::default(),
        ));
        assert_eq!(session.reserves().fuel.now, 50.5);
        assert_eq!(session.pilot().cash(), 1000);
    }

    #[test]
    fn an_uninhabited_stellar_tops_up_nothing_and_still_beeps_once() {
        let mut session = about_to_land(&refuelling(), true, 50.5, 1000, Rulebook::default());
        session.chart.sites[0].flags |= StellarFlags::UNINHABITED;
        let mut session = landed(session);
        assert_eq!(session.reserves().fuel.now, 50.5);
        assert_eq!(session.pilot().cash(), 1000);
        let sounds = session.take_sounds();
        let beeps = sounds.iter().filter(|s| **s == SimSound::Landed).count();
        assert_eq!(beeps, 1, "{sounds:?}");
    }

    #[test]
    fn the_top_up_makes_no_sound() {
        let sounds = |carries| {
            let mut session = landed(about_to_land(
                &refuelling(),
                carries,
                50.5,
                1000,
                Rulebook::default(),
            ));
            session.take_sounds()
        };
        let without = sounds(false);
        assert_eq!(sounds(true), without);
        assert!(!without.contains(&SimSound::Landed), "{without:?}");
    }

    #[test]
    fn a_dominated_stellar_still_charges() {
        let mut catalog = refuelling();
        catalog.sites[0].1[0].flags2 |= market::DOMINATED;
        let session = landed(about_to_land(
            &catalog,
            true,
            100.0,
            1000,
            Rulebook::default(),
        ));
        let max = session.reserves().fuel.max;
        assert_eq!(session.reserves().fuel, Gauge::full(max));
        #[allow(clippy::cast_possible_truncation)]
        let units = (max - 100.0) as i64;
        assert_eq!(session.pilot().cash(), 1000 - units);
    }

    #[test]
    fn by_the_engine_a_debt_is_cleared_with_fuel_by_the_other_reading_it_stands() {
        let session = landed(about_to_land(
            &refuelling(),
            true,
            100.5,
            -30,
            Rulebook::default(),
        ));
        assert_eq!(session.reserves().fuel.now, 70.0);
        assert_eq!(session.pilot().cash(), 0);
        let rules = Rulebook::default().with_override(RuleKey::RechargeDebt, RuleSource::Bible);
        let session = landed(about_to_land(&refuelling(), true, 100.5, -30, rules));
        assert_eq!(session.reserves().fuel.now, 100.0, "truncated only");
        assert_eq!(session.pilot().cash(), -30);
    }
}
