//! The nearest-stellar pick, `_FindNearestStellar` (@0x276c-0x2888), as
//! the session's callers make it: the land key with no target
//! (`Session::land`), F5 (`Session::select_stellar`), the tutorial's land
//! hint and a new pilot's saved stellar (`Session::begin`). See the
//! `story` module's "Built by stellar-fidelity phase 14".
//!
//! Each reads its gate limit from
//! [`RuleKey::GatePickRange`](crate::RuleKey::GatePickRange): by the
//! engine, a hypergate or wormhole counts only strictly within 500 pixels
//! (a squared distance below 250000.0, @0x2838-0x2853); by the other
//! reading, at any distance.

#[cfg(test)]
pub(super) mod tests {
    use crate::catalog::{LandingSite, StellarId, SystemId};
    use crate::geometry::Vec2;
    use crate::landing::{HYPERGATE, LandOutcome, StellarFlags, WORMHOLE};
    use crate::rulebook::Rulebook;
    use crate::session::{LandPress, Session};
    use crate::sound::SimSound;
    use crate::testkit::{FakePilotCatalog, catalog, planet};
    use crate::{RuleKey, RuleSource, StellarPick};

    /// The wormhole of [`gates`]: can-land and uninhabited (`Flags`
    /// 0x21), `Flags2` 0x2000, as stock wormholes are.
    const WORMHOLE_ID: i16 = 150;
    /// The hypergate of [`gates`]: can-land, `Flags2` 0x1000.
    const HYPERGATE_ID: i16 = 151;
    /// The ordinary planet of [`gates`].
    const PLANET_ID: i16 = 128;

    pub(in crate::session) fn wormhole(id: i16, x: f32, y: f32) -> LandingSite {
        LandingSite {
            flags: StellarFlags::CAN_LAND | StellarFlags::UNINHABITED,
            flags2: WORMHOLE,
            ..planet(id, x, y)
        }
    }

    pub(in crate::session) fn hypergate(id: i16, x: f32, y: f32) -> LandingSite {
        LandingSite {
            flags2: HYPERGATE,
            ..planet(id, x, y)
        }
    }

    /// The testkit catalog with system 130 holding a wormhole at (0, 600),
    /// a hypergate at (0, -600) and a planet at (800, 0), in that order.
    fn gates() -> FakePilotCatalog {
        let mut catalog = catalog();
        catalog.sites[0].1 = vec![
            wormhole(WORMHOLE_ID, 0.0, 600.0),
            hypergate(HYPERGATE_ID, 0.0, -600.0),
            planet(PLANET_ID, 800.0, 0.0),
        ];
        catalog
    }

    /// The session in 130 by `rules`, the ship at rest at `at`, nothing
    /// selected and no sound pending.
    fn at(catalog: &FakePilotCatalog, rules: Rulebook, at: Vec2) -> Session {
        let mut session = Session::start(catalog).expect("starts").with_rules(rules);
        assert_eq!(session.system(), SystemId(130));
        session.ship.player.position = at;
        session.ship.player.velocity = Vec2::ZERO;
        session.chart.nav = crate::navigation::NavMode::Off;
        session.take_sounds();
        session
    }

    fn engine() -> Rulebook {
        Rulebook::default()
    }

    fn bible() -> Rulebook {
        Rulebook::default().with_override(RuleKey::GatePickRange, RuleSource::Bible)
    }

    /// What an untargeted L selects from `at` by `rules`.
    fn l_selects(rules: Rulebook, from: Vec2) -> Option<StellarId> {
        let mut session = at(&gates(), rules, from);
        let _ = session.land();
        session.nav_target()
    }

    /// What F5 selects from `at` by `rules`, and the sounds it made.
    fn f5_selects(rules: Rulebook, from: Vec2) -> (Option<StellarId>, Vec<SimSound>) {
        let mut session = at(&gates(), rules, from);
        let picked = session.select_stellar(StellarPick::Nearest);
        assert_eq!(picked, session.nav_target());
        (picked, session.take_sounds())
    }

    #[test]
    fn by_the_engine_l_and_f5_pass_over_gates_600_away_for_the_planet() {
        let mut session = at(&gates(), engine(), Vec2::ZERO);
        assert_eq!(
            session.land(),
            Ok(LandPress::Outcome(LandOutcome::Requested {
                stellar: StellarId(PLANET_ID),
                station: false,
            })),
            "requested as the planet would be"
        );
        assert_eq!(session.nav_target(), Some(StellarId(PLANET_ID)));
        assert_eq!(
            f5_selects(engine(), Vec2::ZERO),
            (Some(StellarId(PLANET_ID)), vec![SimSound::NavSelected])
        );
    }

    #[test]
    fn by_the_engine_a_wormhole_within_500_is_picked() {
        // 450 from the wormhole, 750 from the hypergate, 813 from the planet.
        let near = Vec2::new(0.0, 150.0);
        assert_eq!(l_selects(engine(), near), Some(StellarId(WORMHOLE_ID)));
        assert_eq!(
            f5_selects(engine(), near),
            (Some(StellarId(WORMHOLE_ID)), vec![SimSound::NavSelected])
        );
    }

    #[test]
    fn by_the_engine_a_hypergate_within_500_is_picked() {
        let near = Vec2::new(0.0, -150.0);
        assert_eq!(l_selects(engine(), near), Some(StellarId(HYPERGATE_ID)));
        assert_eq!(
            f5_selects(engine(), near),
            (Some(StellarId(HYPERGATE_ID)), vec![SimSound::NavSelected])
        );
    }

    #[test]
    fn by_the_other_reading_l_and_f5_pick_a_gate_600_away() {
        // Both gates are 600 away: the wormhole is listed first.
        assert_eq!(l_selects(bible(), Vec2::ZERO), Some(StellarId(WORMHOLE_ID)));
        assert_eq!(
            f5_selects(bible(), Vec2::ZERO).0,
            Some(StellarId(WORMHOLE_ID))
        );
        let nearer_the_hypergate = Vec2::new(0.0, -10.0);
        assert_eq!(
            l_selects(bible(), nearer_the_hypergate),
            Some(StellarId(HYPERGATE_ID))
        );
        assert_eq!(
            f5_selects(bible(), nearer_the_hypergate).0,
            Some(StellarId(HYPERGATE_ID))
        );
    }
}
