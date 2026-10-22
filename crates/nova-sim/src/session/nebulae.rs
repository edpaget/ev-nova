//! The session's side of the nebulae (see the [`nebula`](crate::nebula)
//! module for the original's rules): their activity written at the
//! galaxy refresh points, their `OnExplore` run as an exploration reaches
//! them, and their explored state forgotten at every load.
//!
//! **Activity.** Every refresh point writes each nebula's activity from
//! its `ActiveOn` beside every system's visibility (the session's
//! `story` module's `Refresh`): a landing, a jump's arrival after its
//! explorations and before its days, a new pilot, a load, and, by the other reading of
//! [`RuleKey::VisibilityRefresh`](crate::RuleKey::VisibilityRefresh),
//! every settling after a script.
//!
//! **Exploring.** Each nebula registers one
//! [`Consumer::Nebulae`](crate::story::Consumer::Nebulae)
//! subscription at [`Session::fly`], keyed by its ID ([`nebula_hook`]), on
//! every `Explored` event, running its `OnExplore`; a blank one too, as
//! exploring is what makes a nebula drawn. Its consumer admits it only
//! for an `Explored` of a system its inset rectangle holds while it is
//! active and not yet explored ([`nebula::explores`]), so a control-bit
//! guard is never asked to express the geometry. Its effect marks it
//! explored before its script runs, as the original marks the slot
//! (@0x515a) before `OnExplore` (@0x519e), so a nested exploration finds
//! it explored; and tells [`SessionEvent::GalaxyChanged`], so the map
//! draws it. Activity and the explored flag are read as the event
//! is dispatched; a jump's and a new pilot's `OnStart` explorations are
//! dispatched at once, as the original explores them, so each reads the
//! activity of the last refresh (see the `jump_exploration` module).
//!
//! **Readings.** By the engine's reading of [`RuleKey::NebulaExploreOnce`]
//! the explored flag is never saved and exploring makes no save due; a
//! load forgets every flag, as `_ResetPlayer` does (@0x1da86), and its
//! sweep explores again. By the other reading, exploring keeps the
//! nebula's hook in the once-fired record, saved, and a load sets each
//! nebula kept there explored ([`nebula::kept`]). By the engine's reading
//! of [`RuleKey::NebulaNewPilot`], [`Session::begin`] withdraws the
//! `Explored` events [`Session::fly`] raised for a new pilot, as
//! `_DoNewPilot` explores its start system while every nebula is
//! inactive.
//!
//! **The load sweep.** [`Session::fly`] raises `Explored` for every
//! catalog system that is visible after the load's refresh and explored,
//! by ascending ID, as Open Pilot's `_MainScreenChoice` sweeps
//! (@0x1a8d4-0x1a906) once the pilot is loaded; `_OpenLastPilotFile`
//! sweeps the same way at launch (@0x765a2-0x765ce), which the launch's
//! load of the last pilot builds with the same sweep. A return from play
//! to the main screen keeps the session as play left it, so it sweeps
//! nothing and runs no `OnExplore`: `_DoMainScreen`'s setup, which
//! explores the player's system, runs once a launch, as
//! [`Session::open_at_launch`] does ([`MissionRun::explore_here`]); Open
//! Pilot doesn't run it. Nothing the sweep
//! decides reads a rule, so it does not wait on task
//! load-reads-rules-before-fly.
//!
//! **Gate exits** explore no nebula (`_PlayerEnterHypergate` 0x637bf,
//! `_PlayerEnterWormhole` 0x64005): their row raises no `Explored` (the
//! `entry` module).
//!
//! Left to others: the escape pod's exploration (roadmap player-death),
//! a jump's `EnteredSystem` hooks before the arrival is explored again
//! and populated (crons-news-ranks-fidelity phase 40), and the
//! explorations of scripts run outside the dispatcher
//! (crons-news-ranks-fidelity phase 43). A jump's scripts read the
//! arrival as the player's system (see the `jump_exploration` module).

use super::{Chart, MissionRun, Outbox, Session, SessionEvent, Setup, Statics};
use crate::catalog::{NebulaId, SystemId};
use crate::nebula::{self, nebula_hook};
use crate::pilot::Pilot;
use crate::rulebook::{RuleKey, RuleSource};
use crate::story::{EventFilter, StoryEvent, StoryEventKind, Subscription};

impl Session {
    /// The load's sweep (see the module docs): `Explored` for every
    /// catalog system visible and explored, by ascending ID.
    pub(super) fn sweep_explored(&mut self) {
        let swept: Vec<SystemId> = self
            .chart
            .galaxy
            .catalog_systems()
            .filter(|&system| self.chart.galaxy.visible(system) && self.pilot.has_explored(system))
            .collect();
        for system in swept {
            self.run.raise(StoryEvent::Explored { system });
        }
    }
}

impl Chart {
    /// Whether `nebula`'s subscription runs on `event` now: an `Explored`
    /// of a system its inset rectangle holds while it is active and not
    /// yet explored ([`nebula::explores`]). Never for a nebula with no
    /// record.
    pub(super) fn nebula_admits(
        &self,
        statics: &Statics,
        nebula: NebulaId,
        event: &StoryEvent,
    ) -> bool {
        let StoryEvent::Explored { system } = *event else {
            return false;
        };
        let Some(record) = statics.nebulae.iter().find(|record| record.id == nebula) else {
            return false;
        };
        nebula::explores(
            record,
            self.galaxy.nebula_active(nebula),
            self.galaxy.nebula_explored(nebula),
            self.galaxy.position(system),
        )
    }

    /// `nebula`'s subscription's effect, before its `OnExplore`: marks it
    /// explored and tells [`SessionEvent::GalaxyChanged`]; by the other
    /// reading of [`RuleKey::NebulaExploreOnce`], keeps it in the
    /// once-fired record too, making a save due.
    pub(super) fn explore_nebula(
        &mut self,
        pilot: &mut Pilot,
        setup: &Setup,
        out: &mut Outbox,
        nebula: NebulaId,
    ) {
        pilot.galaxy.set_nebula_explored(nebula, true);
        self.galaxy.set_overlay(pilot.galaxy.clone());
        out.emit(SessionEvent::GalaxyChanged);
        if setup.rules.source_for(RuleKey::NebulaExploreOnce) == RuleSource::Bible {
            pilot.story_fired.insert(nebula_hook(nebula));
            out.save_due = true;
        }
    }
}

impl MissionRun {
    /// Registers each nebula's `OnExplore` subscription (see the module
    /// docs). A nebula registered already is left as it is.
    pub(super) fn register_nebulae(&mut self, statics: &Statics) {
        for record in statics.nebulae.clone().iter() {
            let _ = self.register_story(Subscription::new(
                nebula_hook(record.id),
                EventFilter::Any(StoryEventKind::Explored),
                record.on_explore.clone(),
            ));
        }
    }

    /// `_DoMainScreen`'s once-a-launch exploration of the player's system
    /// (`_ExploreNebulaeFromSystem(current)` @0x1affe), part of
    /// [`Session::open_at_launch`]: `Explored` for the current system,
    /// visible or not, explored or not. A nebula the load's sweep explored
    /// already is not admitted again as it settles.
    pub(super) fn explore_here(&mut self, pilot: &Pilot) {
        self.raise(StoryEvent::Explored {
            system: pilot.system,
        });
    }

    /// Withdraws the `Explored` events raised and not yet settled, keeping
    /// every other: a new pilot's, by the engine's reading of
    /// [`RuleKey::NebulaNewPilot`] (see the module docs).
    pub(super) fn withdraw_new_pilot_explored(&mut self, setup: &Setup) {
        if setup.rules.source_for(RuleKey::NebulaNewPilot) == RuleSource::Engine {
            self.withdraw_raised(StoryEventKind::Explored);
        }
    }
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use crate::catalog::{NebulaRecord, SystemVisibility};
    use crate::chance::NeverFires;
    use crate::control::{Bit, SetExpr, Test};
    use crate::pilot::Pilot;
    use crate::rulebook::Rulebook;
    use crate::save;
    use crate::testkit::{FakePilotCatalog, catalog, jump, land_now, nebula, star};
    use crate::{ScriptNote, StellarId};

    const N128: NebulaId = NebulaId(128);

    /// [`catalog`] with `nebulae`.
    pub(in crate::session) fn nebulous(nebulae: Vec<NebulaRecord>) -> FakePilotCatalog {
        let mut catalog = FakePilotCatalog {
            nebulae,
            ..catalog()
        };
        catalog.systems.push(SystemId(132));
        catalog
    }

    /// Nebula 128 around 131 (600, 0) alone, active by `active_on`,
    /// setting bit 7 as it is explored.
    pub(in crate::session) fn around_131(active_on: &str) -> NebulaRecord {
        nebula(128, (500, -100, 200, 200), active_on, "b7")
    }

    fn rules(key: RuleKey, source: RuleSource) -> Rulebook {
        Rulebook::default().with_override(key, source)
    }

    /// `catalog`'s new pilot, flown by `rules`.
    fn flown(catalog: &FakePilotCatalog, rules: Rulebook) -> Session {
        Session::fly(catalog, Pilot::new(catalog, "").expect("starts"))
            .expect("flies")
            .with_rules(rules)
    }

    fn run(session: &mut Session, script: &str) {
        session.run_set(&SetExpr::parse(script).expect("parses"), &mut NeverFires);
    }

    fn settle(session: &mut Session, catalog: &FakePilotCatalog) {
        let _ = session.settle_script(catalog, &mut NeverFires);
    }

    fn bit(session: &Session, n: u16) -> bool {
        session.pilot().control_bit(Bit::new(n).expect("a bit"))
    }

    fn active(session: &Session, id: i16) -> bool {
        session.galaxy().nebula_active(NebulaId(id))
    }

    fn explored(session: &Session, id: i16) -> bool {
        session.galaxy().nebula_explored(NebulaId(id))
    }

    /// Puts the ship at rest at its system's centre and lands there.
    fn land_at_centre(session: &mut Session) -> StellarId {
        session.ship.player.position = crate::geometry::Vec2::ZERO;
        session.ship.player.velocity = crate::geometry::Vec2::ZERO;
        land_now(session).expect("lands")
    }

    // Activity at the refresh points.

    #[test]
    fn a_flown_pilots_nebulae_are_active_by_their_bits_and_a_zero_size_one_is_not() {
        let catalog = nebulous(vec![
            nebula(128, (0, 0, 100, 100), "b5", ""),
            nebula(129, (0, 0, 0, 100), "", ""),
            nebula(130, (0, 0, 100, 100), "", ""),
        ]);
        let mut session = Session::start(&catalog).expect("starts");
        assert!(!active(&session, 128), "b5 clear");
        assert!(!active(&session, 129), "no width");
        assert!(active(&session, 130), "a blank test");
        run(&mut session, "b5");
        let flown = Session::fly(&catalog, session.pilot().clone()).expect("flies");
        assert!(active(&flown, 128), "b5 set as it is flown");
        assert!(!active(&flown, 129));
    }

    #[test]
    fn a_load_refresh_writing_both_a_system_and_a_nebula_writes_each() {
        let mut catalog = nebulous(vec![nebula(128, (0, 0, 100, 100), "b5", "")]);
        catalog.visibility.push(SystemVisibility {
            id: SystemId(132),
            visibility: Test::parse("b50"),
        });
        let session = Session::start(&catalog).expect("starts");
        assert!(!session.galaxy().visible(SystemId(132)), "hidden");
        assert!(!active(&session, 128), "inactive");
    }

    #[test]
    fn a_landing_refresh_activates_a_nebula_its_bits_now_pass() {
        let catalog = nebulous(vec![nebula(128, (0, 0, 100, 100), "b5", "")]);
        let mut session = Session::start(&catalog).expect("starts");
        settle(&mut session, &catalog);
        run(&mut session, "b5");
        settle(&mut session, &catalog);
        assert!(!active(&session, 128), "not before a refresh point");
        land_now(&mut session).expect("lands");
        settle(&mut session, &catalog);
        assert!(active(&session, 128), "at the landing");
    }

    #[test]
    fn a_jumps_arrival_writes_nebula_activity() {
        let catalog = nebulous(vec![nebula(128, (0, 0, 100, 100), "b5", "")]);
        let mut session = Session::start(&catalog).expect("starts");
        run(&mut session, "b5");
        settle(&mut session, &catalog);
        assert!(!active(&session, 128));
        jump(&mut session, &catalog, 131).expect("arrives");
        assert!(active(&session, 128), "at the arrival");
    }

    #[test]
    fn by_the_other_visibility_refresh_reading_a_settled_script_refreshes_activity() {
        let catalog = nebulous(vec![nebula(128, (0, 0, 100, 100), "b5", "")]);
        for (source, refreshed) in [(RuleSource::Engine, false), (RuleSource::Bible, true)] {
            let mut session = flown(&catalog, rules(RuleKey::VisibilityRefresh, source));
            settle(&mut session, &catalog);
            session.take_events();
            run(&mut session, "b5");
            settle(&mut session, &catalog);
            assert_eq!(active(&session, 128), refreshed, "{source:?}");
            assert_eq!(
                session.take_events().contains(&SessionEvent::GalaxyChanged),
                refreshed,
                "{source:?}"
            );
        }
    }

    // Exploring.

    #[test]
    fn exploring_a_system_inside_a_nebula_runs_its_on_explore_and_shows_it() {
        let catalog = nebulous(vec![around_131("")]);
        let mut session = Session::start(&catalog).expect("starts");
        settle(&mut session, &catalog);
        assert!(!explored(&session, 128), "130 is outside it");
        assert!(!bit(&session, 7));
        session.take_events();
        session.run.raise(StoryEvent::Explored {
            system: SystemId(131),
        });
        settle(&mut session, &catalog);
        assert!(bit(&session, 7), "its OnExplore ran");
        assert!(session.galaxy().nebula_shown(N128));
        assert_eq!(session.take_events(), [SessionEvent::GalaxyChanged]);
    }

    /// Whether nebula 128 is explored as each operator applies.
    #[derive(Debug, Default)]
    struct Seen(std::cell::RefCell<Vec<bool>>);

    impl crate::session::control::SetOpObserver for Seen {
        fn saw(&self, _op: &crate::control::SetOp, session: &Session) {
            self.0
                .borrow_mut()
                .push(session.galaxy().nebula_explored(N128));
        }
    }

    #[test]
    fn a_nebula_is_explored_before_its_on_explore_runs() {
        let catalog = nebulous(vec![nebula(128, (500, -100, 200, 200), "", "X132")]);
        let seen = std::rc::Rc::new(Seen::default());
        let mut session = Session::start(&catalog)
            .expect("starts")
            .with_set_op_observer(seen.clone());
        settle(&mut session, &catalog);
        jump(&mut session, &catalog, 131).expect("arrives");
        settle(&mut session, &catalog);
        assert_eq!(seen.0.take(), [true], "X132, once, the nebula explored");
    }

    #[test]
    fn a_nebula_explored_runs_no_more_this_flight() {
        let catalog = nebulous(vec![around_131("")]);
        let mut session = Session::start(&catalog).expect("starts");
        jump(&mut session, &catalog, 131).expect("arrives");
        settle(&mut session, &catalog);
        assert!(bit(&session, 7));
        run(&mut session, "!b7");
        settle(&mut session, &catalog);
        land_at_centre(&mut session);
        settle(&mut session, &catalog);
        session
            .take_off(&catalog, &mut NeverFires)
            .expect("takes off");
        settle(&mut session, &catalog);
        assert!(!bit(&session, 7), "explored already");
        assert!(explored(&session, 128));
    }

    #[test]
    fn an_inactive_or_zero_size_nebula_is_never_explored() {
        let catalog = nebulous(vec![
            around_131("b5"),
            nebula(129, (500, -100, 0, 200), "", "b8"),
        ]);
        let mut session = Session::start(&catalog).expect("starts");
        jump(&mut session, &catalog, 131).expect("arrives");
        settle(&mut session, &catalog);
        assert!(!bit(&session, 7));
        assert!(!bit(&session, 8));
        assert!(!explored(&session, 128));
        assert!(!explored(&session, 129));
    }

    #[test]
    fn a_nebula_inactive_when_passed_explores_once_a_refresh_activates_it() {
        let catalog = nebulous(vec![around_131("b5")]);
        let mut session = Session::start(&catalog).expect("starts");
        jump(&mut session, &catalog, 131).expect("arrives");
        settle(&mut session, &catalog);
        assert!(!explored(&session, 128), "inactive");
        run(&mut session, "b5");
        settle(&mut session, &catalog);
        land_at_centre(&mut session);
        settle(&mut session, &catalog);
        assert!(active(&session, 128), "the landing's refresh");
        assert!(!explored(&session, 128), "not yet");
        session
            .take_off(&catalog, &mut NeverFires)
            .expect("takes off");
        settle(&mut session, &catalog);
        assert!(explored(&session, 128), "the take-off's exploration");
        assert!(bit(&session, 7));
    }

    #[test]
    fn by_the_engine_exploring_makes_no_save_due_and_by_the_other_reading_one() {
        let catalog = nebulous(vec![nebula(128, (500, -100, 200, 200), "", "")]);
        for (source, due) in [(RuleSource::Engine, false), (RuleSource::Bible, true)] {
            let mut session = flown(&catalog, rules(RuleKey::NebulaExploreOnce, source));
            settle(&mut session, &catalog);
            session.take_save_due();
            session.run.raise(StoryEvent::Explored {
                system: SystemId(131),
            });
            settle(&mut session, &catalog);
            assert!(explored(&session, 128), "{source:?}");
            assert_eq!(session.take_save_due(), due, "{source:?}");
            assert_eq!(
                session.pilot().story_fired.contains(&nebula_hook(N128)),
                due,
                "{source:?}"
            );
        }
    }

    #[test]
    fn a_nebula_with_no_record_or_another_event_is_never_admitted() {
        let catalog = nebulous(vec![around_131("")]);
        let session = Session::start(&catalog).expect("starts");
        let at_131 = StoryEvent::Explored {
            system: SystemId(131),
        };
        assert!(session.chart.nebula_admits(&session.statics, N128, &at_131));
        assert!(
            !session
                .chart
                .nebula_admits(&session.statics, NebulaId(129), &at_131),
            "no record"
        );
        assert!(
            !session.chart.nebula_admits(
                &session.statics,
                N128,
                &StoryEvent::EnteredSystem {
                    system: SystemId(131),
                    how: crate::story::Entrance::Jump,
                }
            ),
            "not an exploration"
        );
        assert!(
            !session.chart.nebula_admits(
                &session.statics,
                N128,
                &StoryEvent::Explored {
                    system: SystemId(999),
                }
            ),
            "no such system"
        );
    }

    // Loads.

    /// [`around_131`]'s session after jumping 130 to 131 and on to 132,
    /// settling after each, with bit 7 cleared again.
    fn travelled(catalog: &FakePilotCatalog) -> Pilot {
        let mut session = Session::start(catalog).expect("starts");
        settle(&mut session, catalog);
        jump(&mut session, catalog, 131).expect("arrives");
        settle(&mut session, catalog);
        jump(&mut session, catalog, 132).expect("arrives");
        settle(&mut session, catalog);
        assert!(bit(&session, 7));
        run(&mut session, "!b7");
        session.pilot().clone()
    }

    #[test]
    fn on_explore_runs_again_after_a_load_by_the_engine_and_once_a_pilot_by_the_other_reading() {
        for source in RuleSource::ALL {
            let catalog = nebulous(vec![around_131("")]);
            let once = rules(RuleKey::NebulaExploreOnce, source);
            let mut session = Session::start(&catalog).expect("starts").with_rules(once);
            settle(&mut session, &catalog);
            for to in [131, 132] {
                jump(&mut session, &catalog, to).expect("arrives");
                settle(&mut session, &catalog);
            }
            assert!(bit(&session, 7), "{source:?}");
            run(&mut session, "!b7");
            let pilot = save::decode(&save::encode(session.pilot())).expect("loads");
            assert_eq!(pilot.system, SystemId(132), "the player is elsewhere");
            let mut loaded = Session::fly(&catalog, pilot)
                .expect("flies")
                .with_rules(once);
            settle(&mut loaded, &catalog);
            assert_eq!(
                bit(&loaded, 7),
                source == RuleSource::Engine,
                "{source:?}: again by the engine"
            );
            assert!(loaded.galaxy().nebula_shown(N128), "{source:?}");
        }
    }

    #[test]
    fn a_load_forgets_explored_nebulae_but_keeps_those_fired_under_the_other_reading() {
        let catalog = nebulous(vec![around_131(""), nebula(129, (0, 0, 9, 9), "", "")]);
        let mut pilot = travelled(&catalog);
        pilot.galaxy.set_nebula_explored(N128, true);
        pilot.galaxy.set_nebula_explored(NebulaId(129), true);
        pilot.galaxy.set_nebula_active(NebulaId(129), Some(false));
        let flown = Session::fly(&catalog, pilot.clone()).expect("flies");
        assert!(!explored(&flown, 128), "forgotten");
        assert!(!explored(&flown, 129), "forgotten");
        pilot.story_fired.insert(nebula_hook(N128));
        let flown = Session::fly(&catalog, pilot).expect("flies");
        assert!(explored(&flown, 128), "kept by its hook");
        assert!(!explored(&flown, 129));
    }

    #[test]
    fn a_load_sweeps_the_visible_explored_systems_by_ascending_id() {
        let mut catalog = nebulous(vec![around_131("")]);
        catalog.visibility.push(SystemVisibility {
            id: SystemId(131),
            visibility: Test::parse("!b40"),
        });
        let mut pilot = Pilot::new(&catalog, "").expect("starts");
        for system in [132, 131, 133] {
            pilot.explore(SystemId(system));
        }
        pilot.bits.set(Bit::new(40).expect("a bit"));
        pilot.system = SystemId(132);
        let flown = Session::fly(&catalog, pilot).expect("flies");
        assert_eq!(
            flown.story_raised(),
            [130, 132, 133].map(|system| StoryEvent::Explored {
                system: SystemId(system),
            }),
            "131 hidden"
        );
    }

    #[test]
    fn a_load_sweep_over_many_explored_systems_drops_no_event() {
        let mut catalog = nebulous(vec![nebula(128, (-100, -100, 200, 200), "", "b7")]);
        let mut pilot = Pilot::new(&catalog, "").expect("starts");
        for n in 0..70 {
            let id = 1000 + n;
            catalog
                .star_map
                .push(star(id, (5000.0 + f32::from(n) * 10.0, 0.0), &[]));
            pilot.explore(SystemId(id));
        }
        let mut session = Session::fly(&catalog, pilot).expect("flies");
        assert!(session.story_raised().len() > 64);
        settle(&mut session, &catalog);
        assert!(
            !session
                .take_script_notes()
                .iter()
                .any(|note| matches!(note, ScriptNote::EventsDropped { .. }))
        );
        assert!(bit(&session, 7));
    }

    // The launch.

    /// [`around_131`]'s pilot loaded at 131, its load's sweep settled
    /// (which explores nebula 128, as the load explores 131), with bit 7
    /// cleared again.
    fn loaded_at_131() -> (FakePilotCatalog, Session) {
        let catalog = nebulous(vec![around_131("")]);
        let mut pilot = Pilot::new(&catalog, "").expect("starts");
        pilot.system = SystemId(131);
        let mut session = Session::fly(&catalog, pilot).expect("flies");
        settle(&mut session, &catalog);
        assert!(explored(&session, 128), "the sweep's");
        run(&mut session, "!b7");
        let _ = session.take_save_due();
        (catalog, session)
    }

    #[test]
    fn the_launch_explores_the_players_system() {
        let (_, mut session) = loaded_at_131();
        session.open_at_launch(&mut NeverFires);
        assert_eq!(
            session.story_raised(),
            [StoryEvent::Explored {
                system: SystemId(131)
            }]
        );
    }

    #[test]
    fn the_launch_runs_the_on_explore_of_a_nebula_over_the_players_system_left_unexplored() {
        let (catalog, mut session) = loaded_at_131();
        session.pilot.galaxy.set_nebula_explored(N128, false);
        session
            .chart
            .galaxy
            .set_overlay(session.pilot.galaxy.clone());
        session.open_at_launch(&mut NeverFires);
        assert!(!session.take_save_due(), "the launch makes no save due");
        settle(&mut session, &catalog);
        assert!(explored(&session, 128));
        assert!(bit(&session, 7), "its OnExplore ran");
    }

    #[test]
    fn a_nebula_the_sweep_explored_runs_no_more_at_the_launch() {
        let (catalog, mut session) = loaded_at_131();
        session.open_at_launch(&mut NeverFires);
        settle(&mut session, &catalog);
        assert!(!bit(&session, 7), "once in all: the sweep's");
        assert!(!session.take_save_due(), "no save is due");
    }

    // A new pilot and the gates.

    #[test]
    fn a_new_pilot_explores_no_nebula_until_its_first_take_off_by_the_engine() {
        let catalog = nebulous(vec![nebula(128, (-100, -100, 200, 200), "", "b7")]);
        for source in RuleSource::ALL {
            let mut session = flown(&catalog, rules(RuleKey::NebulaNewPilot, source));
            session.begin(&catalog, &mut NeverFires);
            settle(&mut session, &catalog);
            let at_start = source == RuleSource::Bible;
            assert_eq!(explored(&session, 128), at_start, "{source:?}");
            assert_eq!(bit(&session, 7), at_start, "{source:?}");
            land_now(&mut session).expect("lands");
            settle(&mut session, &catalog);
            session
                .take_off(&catalog, &mut NeverFires)
                .expect("takes off");
            settle(&mut session, &catalog);
            assert!(explored(&session, 128), "{source:?}");
            assert!(bit(&session, 7), "{source:?}");
        }
    }

    #[test]
    fn a_new_pilots_begin_keeps_its_landing() {
        let catalog = nebulous(Vec::new());
        let mut session = flown(&catalog, Rulebook::default());
        session.run.raise(StoryEvent::Landed {
            stellar: StellarId(128),
        });
        session.begin(&catalog, &mut NeverFires);
        assert_eq!(
            session.story_raised(),
            [StoryEvent::Landed {
                stellar: StellarId(128),
            }]
        );
    }

    #[test]
    fn a_gate_exit_explores_no_nebula() {
        use crate::session::tests::{at_gate, gated};
        let catalog = FakePilotCatalog {
            nebulae: vec![around_131("")],
            ..gated()
        };
        for arrival in [
            crate::gate::GateArrivalRule::default(),
            crate::gate::GateArrivalRule::LikeJump,
        ] {
            let mut session = at_gate(&catalog).with_gate_arrival(arrival);
            settle(&mut session, &catalog);
            session
                .enter_hypergate(Some(SystemId(131)), &catalog, &mut NeverFires)
                .expect("enters");
            settle(&mut session, &catalog);
            assert_eq!(session.pilot().system, SystemId(131), "{arrival:?}");
            assert!(!explored(&session, 128), "{arrival:?}");
            assert!(!bit(&session, 7), "{arrival:?}");
        }
    }
}
