//! The rescues kept until the next refresh
//! ([`RuleKey::RescueUntilRefresh`]). All addresses are in `EV Nova.app`'s
//! `EV Nova` (i386).
//!
//! `_HandleStellarSystemVisibility`, at every rebuild point, writes the
//! player's system (+0x74) into the owner word (+0x14) of each stellar
//! that has none and that the player's system's `NavDefs` list
//! (@0x32c58-0x32ca1). It never overwrites an owner already set, and only
//! `_PropagateMissionBitEffects` clears the owners, every one, as its
//! first act (@0x9968a-0x9969f). So a rescue lasts until the next refresh:
//! a jump's arrival (@0x6bf3f), a landing (@0xa175f, @0xa1af2), a load
//! (@0x751ca, @0x7531f), a new pilot (@0x1925e) or the main screen's
//! play. A gate's or a wormhole's exit (@0x63e8c, @0x6473e), an `M` or
//! `N` (@0x159a3), the take-off (@0x674c7) and the escape pod (@0x69135)
//! rebuild with no refresh, so a stellar rescued in one system stays that
//! system's after the player has moved on: not shown in the new system,
//! though it lists it. The pilot file never holds +0x14 (`_WritePilotData`
//! @0x72580, @0x72ae2), and a load refreshes first, so nothing is saved.
//!
//! **Here.** The galaxy keeps the rescues
//! ([`OverlaidGalaxy::keep_rescues`](crate::story::OverlaidGalaxy::keep_rescues)),
//! and every owner-first reader takes them through
//! [`OverlaidGalaxy::holder`](crate::story::OverlaidGalaxy::holder). The
//! rescue pass runs at every rebuild point (`Session::rebuild_quietly`,
//! after the known pass and before the stellars shown, docked or not). The
//! rescues are forgotten at every refresh, whatever it changes:
//!
//! - `Session::refresh_galaxy` and `Session::refresh_galaxy_quietly`
//!   forget them first, then, after the refresh's move out of a hidden
//!   system, rescue for the player's system, standing in for the rebuild
//!   the original runs right after each of these refreshes. The engine
//!   runs a load's rebuild (`Session::fly`) and a landing's (`Session::land`'s
//!   dock) before their refreshes, where the original runs them after
//!   (@0x751ca then @0x751cf; @0xa1af2 then @0x5fbf0): the rescue after the
//!   refresh settles both before the frame is drawn or the spaceport
//!   opens. By `visibility_refresh = bible` the refresh after a settled
//!   script does the same, as a refresh point with its rebuild.
//! - A jump's arrival (`Session::arrive`) forgets them as it writes its
//!   refresh, before the jump's days, as the original does (@0x6bf3f, the
//!   days @0x6c144), and its entry's rebuild rescues for the arrival
//!   system. In between the owners are left unset, so the holder derives
//!   no rescue (see the `owners_unset` module).
//! - The developer tools' galaxy change (`Session::change_galaxy`) is no
//!   refresh, and forgets nothing.
//!
//! By the other reading (crons-news-ranks phase 13's) no rescue is kept:
//! the holder derives it for the player's current system alone, so a
//! rescued stellar follows the player. A session is built by the default
//! rulebook, whose rebuilds keep rescues, so `Session::with_rules`
//! forgets them when it installs this reading. The rescue plays no sound.

use super::{Chart, Setup, Statics};
use crate::pilot::Pilot;
use crate::rulebook::{RuleKey, RuleSource, Rulebook};

impl Chart {
    /// A rebuild point's rescue pass (see the module docs): by the engine
    /// reading of [`RuleKey::RescueUntilRefresh`], each stellar the
    /// player's system lists, with no owner and no rescue kept, is kept as
    /// that system's; by the other reading, nothing.
    pub(super) fn keep_rescues(&mut self, pilot: &Pilot, statics: &Statics, setup: &Setup) {
        if setup.rules.source_for(RuleKey::RescueUntilRefresh) == RuleSource::Engine {
            self.galaxy
                .keep_rescues(&statics.listed_stellars, pilot.system);
        }
    }

    /// A refresh's clear of every owner (`_PropagateMissionBitEffects`
    /// @0x9968a-0x9969f): every rescue kept is forgotten.
    pub(super) fn forget_rescues(&mut self) {
        self.galaxy.forget_rescues();
    }

    /// The session given `rules` (`Session::with_rules`): by the other
    /// reading of [`RuleKey::RescueUntilRefresh`], every rescue kept so far
    /// is forgotten, as the session was built by the default rulebook, whose
    /// rebuilds keep them, and the holder reads no kept rescue by that
    /// reading; nor are the owners left unset (see the `owners_unset`
    /// module).
    pub(super) fn read_rescue_rule(&mut self, rules: &Rulebook) {
        if rules.source_for(RuleKey::RescueUntilRefresh) == RuleSource::Bible {
            self.forget_rescues();
            self.galaxy.stop_awaiting_rebuild();
        }
    }
}

#[cfg(test)]
pub(super) mod tests {
    use crate::catalog::{StellarId, SystemId};
    use crate::chance::NeverFires;
    use crate::control::SetExpr;
    use crate::gate::GateKind;
    use crate::pilot::Pilot;
    use crate::rulebook::{RuleKey, RuleSource, Rulebook};
    use crate::session::tests::gated;
    use crate::session::{LandPress, Session};
    use crate::testkit::{
        FakePilotCatalog, begin_jump_now, fly_out, hideable, hiding_bit, land_now, planet, star,
    };

    /// The stellar both 130 and 131 list.
    pub(in crate::session) const SHARED: StellarId = StellarId(150);

    /// What `engine` and `other` give by `source`.
    pub(in crate::session) fn by<T>(source: RuleSource, engine: T, other: T) -> T {
        match source {
            RuleSource::Engine => engine,
            RuleSource::Bible => other,
        }
    }

    /// [`gated`] with 129 at 130's place, visible, so 130 is shadowed and
    /// claims nothing, 131's planet 140 under hypergate 310, and planet 150
    /// listed by both 130 and 131; every system hideable.
    pub(in crate::session) fn shared() -> FakePilotCatalog {
        let mut catalog = gated();
        catalog.star_map.push(star(129, (0.0, 0.0), &[]));
        for (system, sites) in &mut catalog.sites {
            if *system == SystemId(131) {
                // Under hypergate 310's exit, so the ship can land there.
                *sites = vec![planet(140, 100.0, 200.0)];
            }
            if [SystemId(130), SystemId(131)].contains(system) {
                sites.push(planet(150, 400.0, 300.0));
            }
        }
        hideable(&mut catalog);
        catalog
    }

    /// `catalog`'s new pilot flown in 130 with 131 hidden by its bit,
    /// reading `rescue_until_refresh` as `source` from a refresh on: 150
    /// has no owner, so the load's refresh, and the refresh after it,
    /// rescue it into 130, which shows it. Then 131 is shown again as a
    /// refresh's move shows a chain head (`Some(true)`), which claims
    /// nothing, with no refresh: the galaxy change stands in for the move's
    /// reveal, so 131 is the hypergate's stand-in for 310 and still claims
    /// nothing.
    pub(in crate::session) fn rescued_in_130(
        catalog: &FakePilotCatalog,
        source: RuleSource,
    ) -> Session {
        let mut pilot = Pilot::new(catalog, "").expect("starts");
        pilot.bits_mut().set(hiding_bit(131));
        let mut session = Session::fly(catalog, pilot)
            .expect("flies")
            .with_rules(Rulebook::default().with_override(RuleKey::RescueUntilRefresh, source));
        assert!(!session.refresh_galaxy(catalog), "131 hidden already");
        assert_eq!(session.system(), SystemId(130), "shadowed, not hidden");
        assert!(session.change_galaxy(catalog, |overlay| {
            overlay.set_visible(SystemId(131), Some(true));
        }));
        assert_eq!(
            session.galaxy().rescued(SHARED),
            by(source, Some(SystemId(130)), None),
            "{source:?}"
        );
        assert!(
            session.stellars_shown().any(|id| id == SHARED),
            "{source:?}"
        );
        session
    }

    /// The system holding 150 and whether it is shown.
    fn shared_held(session: &Session) -> (Option<SystemId>, bool) {
        let listers = [SystemId(130), SystemId(131)];
        (
            session
                .galaxy()
                .holder(SHARED, &listers, session.pilot().system()),
            session.stellars_shown().any(|id| id == SHARED),
        )
    }

    /// `session`'s L pressed over hypergate 300, the dock approach run, L
    /// pressed again, then the exit into 131 taken.
    pub(in crate::session) fn gate_to_131(session: &mut Session, catalog: &FakePilotCatalog) {
        assert!(session.land().is_ok());
        session.dock_approach();
        assert_eq!(
            session.land(),
            Ok(LandPress::AtGate {
                stellar: StellarId(300),
                kind: GateKind::Hypergate,
            })
        );
        assert_eq!(
            session.enter_hypergate(Some(SystemId(131)), catalog, &mut NeverFires),
            Ok(SystemId(131))
        );
    }

    #[test]
    fn a_rescue_outlives_a_gate_exit_with_no_refresh_by_the_engine() {
        let catalog = shared();
        for source in RuleSource::ALL {
            let mut session = rescued_in_130(&catalog, source);
            gate_to_131(&mut session, &catalog);
            assert_eq!(
                shared_held(&session),
                by(
                    source,
                    (Some(SystemId(130)), false),
                    (Some(SystemId(131)), true)
                ),
                "{source:?}"
            );
        }
    }

    #[test]
    fn a_rescue_outlives_an_in_flight_m_by_the_engine() {
        let catalog = shared();
        for source in RuleSource::ALL {
            let mut session = rescued_in_130(&catalog, source);
            session.run_set(&SetExpr::parse("M131").expect("parses"), &mut NeverFires);
            let settled = session.settle_script(&catalog, &mut NeverFires);
            assert_eq!(settled.moved, Some(SystemId(131)), "{source:?}");
            assert_eq!(
                shared_held(&session),
                by(
                    source,
                    (Some(SystemId(130)), false),
                    (Some(SystemId(131)), true)
                ),
                "{source:?}"
            );
        }
    }

    #[test]
    fn a_jump_forgets_the_rescue_and_the_arrival_rescues_for_its_system() {
        let catalog = shared();
        for source in RuleSource::ALL {
            let mut session = rescued_in_130(&catalog, source);
            session.plot_course(SystemId(131)).expect("a route");
            fly_out(&mut session, &catalog);
            assert_eq!(begin_jump_now(&mut session, &catalog), Ok(SystemId(131)));
            assert_eq!(
                session.arrive(&catalog, &mut NeverFires),
                Some(SystemId(131)),
                "{source:?}: hidden again, the chain head made visible"
            );
            assert_eq!(
                shared_held(&session),
                (Some(SystemId(131)), true),
                "{source:?}"
            );
            assert_eq!(
                session.galaxy().rescued(SHARED),
                by(source, Some(SystemId(131)), None),
                "{source:?}"
            );
        }
    }

    #[test]
    fn a_landing_after_a_gate_exit_rescues_for_the_landing_system() {
        let catalog = shared();
        for source in RuleSource::ALL {
            let mut session = rescued_in_130(&catalog, source);
            gate_to_131(&mut session, &catalog);
            // Brought to rest over 140, as the dock approach needs.
            session.ship.player.velocity = crate::geometry::Vec2::ZERO;
            assert_eq!(land_now(&mut session), Ok(StellarId(140)), "{source:?}");
            assert_eq!(
                session.galaxy().rescued(SHARED),
                by(source, Some(SystemId(130)), None),
                "{source:?}: the dock's rebuild keeps it"
            );
            // The landing's refresh, dispatched as the step settles
            // (@0xa1af2), forgets it, and the rescue after it keeps it in 131.
            let _ = session.settle_script(&catalog, &mut NeverFires);
            assert_eq!(
                session.galaxy().rescued(SHARED),
                by(source, Some(SystemId(131)), None),
                "{source:?}"
            );
            session.take_off(&catalog, &mut NeverFires);
            assert_eq!(
                shared_held(&session),
                (Some(SystemId(131)), true),
                "{source:?}"
            );
        }
    }

    #[test]
    fn a_session_given_the_bible_reading_after_its_load_keeps_no_rescue() {
        // Built as the app builds a flight: flown, so the load's rebuild and
        // refresh run by the default rulebook and rescue 150 into 130, then
        // given its rules.
        let catalog = shared();
        let mut pilot = Pilot::new(&catalog, "").expect("starts");
        pilot.bits_mut().set(hiding_bit(131));
        let mut session = Session::fly(&catalog, pilot).expect("flies").with_rules(
            Rulebook::default().with_override(RuleKey::RescueUntilRefresh, RuleSource::Bible),
        );
        assert_eq!(session.system(), SystemId(130));
        assert_eq!(session.galaxy().rescued(SHARED), None);
        assert!(session.change_galaxy(&catalog, |overlay| {
            overlay.set_visible(SystemId(131), Some(true));
        }));
        gate_to_131(&mut session, &catalog);
        assert_eq!(
            shared_held(&session),
            (Some(SystemId(131)), true),
            "the rescue follows the player"
        );
    }

    #[test]
    fn a_loads_refresh_forgets_a_rescue_kept_for_another_system() {
        // By the engine, 150 kept in 130 after the gate's exit into 131: the
        // load's refresh hides 131 again and shows it as its chain head, so
        // 150 has no owner and the rescue after the refresh keeps it in 131,
        // not in 130, which the stale rescue would give.
        let catalog = shared();
        let mut session = rescued_in_130(&catalog, RuleSource::Engine);
        gate_to_131(&mut session, &catalog);
        assert_eq!(session.galaxy().rescued(SHARED), Some(SystemId(130)));
        session.refresh_galaxy_quietly(&catalog);
        assert_eq!(session.system(), SystemId(131));
        assert_eq!(session.galaxy().rescued(SHARED), Some(SystemId(131)));
        assert_eq!(shared_held(&session), (Some(SystemId(131)), true));
    }

    #[test]
    fn a_load_keeps_no_rescue_and_rescues_for_the_loaded_system_only() {
        let catalog = shared();
        for source in RuleSource::ALL {
            let mut session = rescued_in_130(&catalog, source);
            gate_to_131(&mut session, &catalog);
            let pilot: Pilot = session.pilot().clone();
            let loaded = Session::fly(&catalog, pilot)
                .expect("flies")
                .with_rules(Rulebook::default().with_override(RuleKey::RescueUntilRefresh, source));
            assert_eq!(loaded.system(), SystemId(131), "{source:?}");
            assert_eq!(
                loaded.galaxy().rescued(SHARED),
                by(source, Some(SystemId(131)), None),
                "{source:?}: flown by the engine's default, then read"
            );
            assert_eq!(
                shared_held(&loaded),
                (Some(SystemId(131)), true),
                "{source:?}"
            );
        }
    }
}
