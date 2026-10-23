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
//!
//! **The candidates** ([`Chart::pick_candidates`]) of the land key and
//! F5 are every stellar the system's `NavDefs` list that is active
//! (+0x44, @0x27c3-0x27c7): its holder, with the player in the system, is
//! visible. `_FindNearestStellar` reads no owner (+0x14), so a stellar
//! another visible system owns, which the system lists but doesn't show,
//! is a candidate. The land key selects it and its dock request refuses
//! it as not landable (+0x14 ≠ +0x74, @0x6699a-0x669a4), with its
//! not-landable line and `snd ` 153; held, a second L is refused again.
//! F5 drops it (@0x69e73-0x69e94): nothing is written and nothing beeps.
//!
//! Left to rdm:phase/stellar-fidelity-followups/phase-5-nearest-and-slot-picks-over-navdefs:
//! the tutorial's land hint and `_DoNewPilot`'s pick still read the
//! stellars shown.

use super::{Chart, Statics};
use crate::catalog::LandingSite;
use crate::pilot::Pilot;

impl Chart {
    /// The stellars the land key's and F5's nearest pick walk, in
    /// `NavDefs` order: each of `listed` that is shown here, or whose
    /// holder, with the player in their system, is visible (the +0x44 the
    /// walk tests, @0x27c3-0x27c7). So every stellar of `sites` is one.
    ///
    /// The original sets +0x44 at the rebuild points
    /// (`_HandleStellarSystemVisibility`); this reads the holder as the
    /// galaxy stands, as `Session::stellar_active` does for the stellar
    /// guns.
    pub(super) fn pick_candidates(&self, pilot: &Pilot, statics: &Statics) -> Vec<LandingSite> {
        self.listed
            .iter()
            .filter(|site| {
                self.sites.iter().any(|shown| shown.id == site.id)
                    || self
                        .galaxy
                        .active_holder(statics.listers(site.id), pilot.system)
                        .is_some()
            })
            .copied()
            .collect()
    }
}

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

    // Candidates: the listed, active stellars, and F5's drop.

    /// Stellar 200, listed by every system of the testkit catalog at
    /// (500, 500): 130, the lowest visible lister, owns it.
    const UNOWNED: i16 = 200;

    /// The `chart_ownership` tests' catalog, stellar [`UNOWNED`] built by
    /// `make`, every system hideable.
    fn twice_listed(make: fn(i16, f32, f32) -> LandingSite) -> FakePilotCatalog {
        let mut catalog = catalog();
        for (_, sites) in &mut catalog.sites {
            sites.push(make(UNOWNED, 500.0, 500.0));
        }
        crate::testkit::hideable(&mut catalog);
        catalog
    }

    /// The session flown into 131 through its own system entry by
    /// `rules`, the ship at rest at `at`, nothing selected and no sound
    /// pending: 131 lists its own 140 at the centre and 200, which 130
    /// owns.
    fn in_131(catalog: &FakePilotCatalog, rules: Rulebook, at: Vec2) -> Session {
        let mut session = Session::start(catalog).expect("starts").with_rules(rules);
        crate::testkit::jump(&mut session, catalog, 131).expect("arrives");
        assert_eq!(
            session
                .chart
                .listed
                .iter()
                .map(|site| site.id.0)
                .collect::<Vec<_>>(),
            [140, UNOWNED]
        );
        assert_eq!(
            session.stellars_shown().collect::<Vec<_>>(),
            [StellarId(140)]
        );
        session.ship.player.position = at;
        session.ship.player.velocity = Vec2::ZERO;
        session.chart.nav = crate::navigation::NavMode::Off;
        session.chart.nav_fly_in.settle();
        session.port.dock = None;
        session.take_sounds();
        session.take_messages();
        session
    }

    /// 141 from the wormhole at (500, 500), 566 from 140 at the centre.
    const NEAR_200: Vec2 = Vec2::new(400.0, 400.0);
    /// 566 from the wormhole, 141 from 140.
    const NEAR_140: Vec2 = Vec2::new(100.0, 100.0);
    /// 566 from the wormhole, 1273 from 140.
    const PAST_200: Vec2 = Vec2::new(900.0, 900.0);

    fn refused_200() -> Result<LandPress, crate::landing::LandingRefusal> {
        Err(crate::landing::LandingRefusal::NotLandable {
            stellar: StellarId(UNOWNED),
            station: false,
        })
    }

    #[test]
    fn l_within_500_of_a_wormhole_another_system_owns_selects_it_and_is_refused() {
        let catalog = twice_listed(wormhole);
        let mut session = in_131(&catalog, engine(), NEAR_200);
        assert_eq!(session.land(), refused_200());
        assert_eq!(session.nav_target(), Some(StellarId(UNOWNED)));
        let mut started = crate::brackets::FlyIn::default();
        started.start();
        assert_eq!(session.chart.nav_fly_in, started, "the fly-in started");
        assert_eq!(session.take_sounds(), [SimSound::Refused], "@0x67e8f");
        assert_eq!(session.port.dock, None, "no dock request");

        // Held, it is refused again rather than passed over.
        assert_eq!(session.land(), refused_200());
        assert_eq!(session.nav_target(), Some(StellarId(UNOWNED)));
        assert_eq!(session.take_sounds(), [SimSound::Refused]);
        assert_eq!(session.port.dock, None);
    }

    #[test]
    fn f5_drops_a_pick_another_system_owns_in_silence() {
        let catalog = twice_listed(wormhole);
        let mut session = in_131(&catalog, engine(), NEAR_200);
        assert_eq!(session.select_stellar(StellarPick::Nearest), None);
        assert_eq!(session.nav_mode(), crate::navigation::NavMode::Off);
        assert_eq!(session.take_sounds(), [], "no beep (@0x69e88-0x69e94)");

        let mut session = in_131(&catalog, engine(), NEAR_200);
        session.select_stellar(StellarPick::Slot(0));
        assert_eq!(session.nav_target(), Some(StellarId(140)));
        session.take_sounds();
        session.chart.nav_fly_in.settle();
        let dock = Some(crate::clearance::DockRequest {
            stellar: StellarId(140),
            permission: 0,
        });
        session.port.dock = dock;
        assert_eq!(session.select_stellar(StellarPick::Nearest), None);
        assert_eq!(session.nav_target(), Some(StellarId(140)), "kept");
        assert_eq!(session.take_sounds(), []);
        assert_eq!(session.port.dock, dock, "the dock request kept");
        assert_eq!(
            session.chart.nav_fly_in,
            crate::brackets::FlyIn::default(),
            "no fly-in"
        );
    }

    #[test]
    fn outside_500_l_and_f5_pick_the_systems_own_stellar() {
        let catalog = twice_listed(wormhole);
        let mut session = in_131(&catalog, engine(), NEAR_140);
        assert!(session.land().is_ok());
        assert_eq!(session.nav_target(), Some(StellarId(140)));
        let mut session = in_131(&catalog, engine(), PAST_200);
        assert!(session.land().is_ok());
        assert_eq!(session.nav_target(), Some(StellarId(140)));
        assert_eq!(
            session.select_stellar(StellarPick::Nearest),
            Some(StellarId(140))
        );
    }

    #[test]
    fn by_the_other_reading_outside_500_l_picks_the_wormhole_and_f5_does_nothing() {
        let catalog = twice_listed(wormhole);
        let mut session = in_131(&catalog, bible(), PAST_200);
        assert_eq!(session.land(), refused_200());
        assert_eq!(session.nav_target(), Some(StellarId(UNOWNED)));
        let mut session = in_131(&catalog, bible(), PAST_200);
        assert_eq!(session.select_stellar(StellarPick::Nearest), None);
        assert_eq!(session.nav_target(), None);
        assert_eq!(session.take_sounds(), []);
    }

    #[test]
    fn l_picks_a_planet_another_system_owns_at_any_distance_and_is_refused() {
        let catalog = twice_listed(planet);
        let mut session = in_131(&catalog, engine(), Vec2::new(2000.0, 2000.0));
        assert_eq!(session.land(), refused_200());
        assert_eq!(session.nav_target(), Some(StellarId(UNOWNED)));
        assert_eq!(session.take_sounds(), [SimSound::Refused]);
    }
}
