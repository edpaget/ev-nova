//! A jump's nebula explorations, run at once in the original's order
//! (crons-news-ranks-fidelity phase 24). All addresses are in
//! `EV Nova.app`'s `EV Nova` (i386).
//!
//! **`_ExploreNebulaeFromSystem`** (0x508f-0x51bd) walks the 32 nëbu
//! slots (@0x50b2-0x51b0). A slot runs when it is active (+0x8, @0x50bc),
//! not yet explored (+0x9, @0x50c6), and its rectangle, inset by 8
//! (@0x50fb, @0x5116), holds the system (@0x5149). It is marked explored
//! (@0x515a), and its `OnExplore` runs at once (@0x519e), before the next
//! slot is tested. Only `_PropagateMissionBitEffects` writes +0x8
//! (@0x998b4-0x998ef), so an exploration reads the last refresh's
//! activity. The `nebulae` module builds every branch of it.
//!
//! **A jump** (`_HandlePlayer`) explores each hop's stand-in (@0x6bddb)
//! and the arrival (@0x6be12) before its fuel (@0x6be31), its refresh
//! (@0x6bf3f) and its days (`_IncrementGameTime` @0x6c144, which runs the
//! crons @0xb58b). So a nebula active before the refresh runs its
//! `OnExplore` even when the refresh leaves it inactive, the refresh
//! reads the bits those `OnExplore`s set, and so does a crön on the
//! jump's days. After the days and `_MissionHandlePlayerEnteredNewSystem`
//! (@0x6c15d), `_AutoSetExploration` (@0x6c1db) explores the arrival again
//! (@0xd98a), with the activity the refresh wrote, before
//! `_SetupShipsInSystem` (@0x6c1f9) populates the system, reading përs
//! `ActiveOn` live (@0x4334d).
//!
//! **A new pilot.** `_DoNewPilot` refreshes (@0x1925e) and then runs the
//! chär `OnStart` (@0x19345). A map it grants explores at once
//! (`_GrantOutfitItem` @0x44dcb, @0x44e12, @0x44e86, then @0xd98a), with
//! the activity from before the `OnStart`, and play's refresh
//! (`_PlayGame` @0x457e9) reads the bits its `OnExplore`s set.
//!
//! **Here.** [`Session::explore_now`] raises a system's
//! [`StoryEvent::Explored`] and dispatches it at once, through the story
//! dispatcher with the moves and a `Q`'s leave held back, so an
//! `OnExplore`'s `M`, `N` and `Q` wait for the caller's settling, as a
//! crön `OnStart`'s do (see the `crons` module). The events an `OnExplore`
//! raises are dispatched with it, as [`RuleKey::StoryReentry`] reads; the
//! events raised before it stay queued, in order, for the next settling.
//! `Session::arrive` explores each system passed and the arrival with it
//! before its refresh and days, and again the arrival, as the entry will
//! resolve it, before it enters and populates the system. A map grant
//! dispatches its own explorations at once, wherever it runs, with no
//! catalog ([`Session::grant_outfit`]): so an `OnStart`'s map explores
//! before `_PlayGame`'s refresh in [`Session::begin`], and a crön's before
//! the next crön of the pass.
//!
//! **The player's system** (crons-news-ranks-fidelity phase 40).
//! `_HandlePlayer` writes +0x74 to each hop's stand-in (@0x6bdd4, after
//! `_FindActiveCoLocatedSystem` @0x6bdc1) before exploring it, and to the
//! system the jump writes (@0x6be0b) before exploring that; the refresh's
//! move takes it out of a system the refresh hid (@0x99918) before the
//! days. So every `OnExplore` and every day script reads the hop or the
//! arrival as the player's system: a map's reveal origin, an `S`'s
//! whereabouts, a crön's `OnStart`. [`Chart::write_jump_system`] and
//! [`Chart::jump_refresh_move`] make the same writes. The owner words
//! don't change with them, so the galaxy awaits its rebuild from the
//! first write (see the `owners_unset` module). The stellars shown stay
//! the departure's until the entry: nothing is drawn before the
//! original's rebuild (@0x6c203), so that crosses no boundary.
//!
//! **The enter-system hook** (crons-news-ranks-fidelity phase 40). After
//! the days and the price flux (@0x6c153, @0x6c158),
//! `_MissionHandlePlayerEnteredNewSystem` (@0x6c15d, 0x99f11-0x9a017)
//! restarts each slot's appear timer and aux countdown; then the explored
//! levels are raised (@0x6c16c-0x6c1b8), the arrival is explored again
//! (@0x6c1db) and set up (@0x6c1f9). The hook runs no script, plays no
//! sound and checks no goal: `_MissionObjectivesCheck` runs from the
//! per-frame `_MissionHandler` (@0x9edb5). So
//! [`Session::enter_jump_hook_now`] raises the jump's
//! [`StoryEvent::EnteredSystem`] after the days and dispatches it at
//! once, as an exploration is, and [`Session::check_goals_after_jump`]
//! checks the goals once the system is set up, each slot's deadline
//! first, as `_MissionObjectivesCheck` does (@0x9eba5-0x9ec70).
//!
//! **Ordering.** The jump's `DayPassed`
//! events settle at the caller's settling, after the hook: the original
//! polls the deadlines per frame after the arrival, so that matches, and
//! the first goal check, which comes before that settling, checks each
//! slot's deadline itself before its goals.
//!
//! [`RuleKey::StoryReentry`]: crate::RuleKey::StoryReentry

use crate::catalog::{PilotCatalog, SystemId, TrafficCatalog};
use crate::chance::Chance;
use crate::pilot::Pilot;
use crate::story::{Entrance, StoryEvent, StoryEventKind};

use super::{Chart, Outbox, Session, Setup};

impl Session {
    /// Explores `system`'s nebulae at once (`_ExploreNebulaeFromSystem`;
    /// see the module docs): raises its [`StoryEvent::Explored`] and
    /// dispatches it now, reading `catalog` and drawing on `chance`, the
    /// events raised before it left queued.
    pub(super) fn explore_now<C>(&mut self, catalog: &C, chance: &mut dyn Chance, system: SystemId)
    where
        C: PilotCatalog + TrafficCatalog + ?Sized,
    {
        let mark = self.run.story_mark();
        self.run.raise(StoryEvent::Explored { system });
        self.dispatch_explored_since(catalog, chance, mark);
    }

    /// Raises the jump's [`StoryEvent::EnteredSystem`] for the player's
    /// system and dispatches it at once, as the explorations are (see the
    /// module docs' "The enter-system hook"), reading `catalog` and drawing
    /// on `chance`: after the days, before the re-explore and the
    /// population (`_MissionHandlePlayerEnteredNewSystem` @0x6c15d).
    pub(super) fn enter_jump_hook_now<C>(&mut self, catalog: &C, chance: &mut dyn Chance)
    where
        C: PilotCatalog + TrafficCatalog + ?Sized,
    {
        let mark = self.run.story_mark();
        self.run.raise(StoryEvent::EnteredSystem {
            system: self.pilot.system,
            how: Entrance::Jump,
        });
        self.dispatch_since(catalog, chance, mark, StoryEventKind::EnteredSystem);
    }

    /// The jump's first check of each mission's ship goals, once the
    /// system is set up, drawing on `chance`: the stand-in for the
    /// per-frame `_MissionObjectivesCheck` (`_MissionHandler` @0x9edb5),
    /// which the enter-system hook, run before the setup, leaves to here
    /// (see the `mission_ships` module). Slot by slot, as the original
    /// does, the deadline comes first (@0x9eba5-0x9ec61): a mission whose
    /// days ran out on the jump's days fails before its goals are checked,
    /// so its goals-met moment is skipped (@0x9ec70), though the jump's
    /// `DayPassed` deadline hooks settle only later, at the caller's
    /// settling, where they find it failed already.
    pub(super) fn check_goals_after_jump(&mut self, chance: &mut dyn Chance) {
        let slots: Vec<u8> = self.pilot.missions.iter().map(|(slot, _)| slot).collect();
        for slot in slots {
            self.mission_deadline(slot, chance);
            self.check_ship_goals(slot, false, chance);
        }
    }
}

impl Chart {
    /// Writes `system` as `pilot`'s system during a jump, before it is
    /// explored: a hop's stand-in (@0x6bdd4) or the system the jump writes
    /// (@0x6be0b; see the module docs' "The player's system"). The owner
    /// words don't change with it, so the galaxy awaits its rebuild from
    /// the first write, as [`RuleKey::RescueUntilRefresh`] reads in
    /// `setup` (see the `owners_unset` module): the holder derives no
    /// rescue for the hop or the arrival, and keeps the departure's.
    ///
    /// [`RuleKey::RescueUntilRefresh`]: crate::RuleKey::RescueUntilRefresh
    pub(super) fn write_jump_system(&mut self, pilot: &mut Pilot, setup: &Setup, system: SystemId) {
        pilot.system = system;
        self.leave_owners_unset(setup);
    }

    /// The jump's refresh's move (@0x99918): `pilot` is moved out of
    /// `written` when the refresh hid it, as the entry will resolve it
    /// ([`Chart::show_entry`]), a chain head it moves to made visible
    /// (@0x999a4), which makes a save due and tells
    /// [`SessionEvent::GalaxyChanged`](super::SessionEvent::GalaxyChanged)
    /// through `out`. Gives the system the player is left in, the
    /// player's system through the days, the enter-system hook, the
    /// re-explore and the population.
    pub(super) fn jump_refresh_move(
        &mut self,
        pilot: &mut Pilot,
        setup: &Setup,
        out: &mut Outbox,
        written: SystemId,
    ) -> SystemId {
        let moved_to = self.show_entry(pilot, setup, out, written);
        pilot.system = moved_to;
        moved_to
    }
}

#[cfg(test)]
pub(super) mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use super::super::crons::tests::scripted;
    use super::super::kept_rescues::tests::{by, rescued_in_130, shared};
    use super::super::nebulae::tests::{around_131, nebulous};
    use super::super::tests::{alternates, chained};
    use crate::catalog::{
        DudeId, DudeRecord, MissionRecord, NebulaId, NebulaRecord, OutfitId, ShipId, StellarId,
        SystemId, SystemTraffic, SystemVisibility,
    };
    use crate::chance::NeverFires;
    use crate::control::{Bit, Script, SetExpr, SetOp, Test};
    use crate::handling::ShipFields;
    use crate::hyperspace::MultiJumpRule;
    use crate::outfit_effects::MAP;
    use crate::pilot::{ExploredLevel, Pilot};
    use crate::reserves::Gauge;
    use crate::rulebook::RuleSource;
    use crate::session::Session;
    use crate::session::control::SetOpObserver;
    use crate::stats::MULTI_JUMP;
    use crate::story::{
        Consumer, EventFilter, StoryEvent, StoryEventKind, Subscription, SubscriptionId,
    };
    use crate::testkit::{
        FAST, FakePilotCatalog, begin_jump_now, fly_out, hideable, hiding_bit, jump, mission,
        nebula, outfit, planet, ship, star,
    };

    fn bit(n: u16) -> Bit {
        Bit::new(n).expect("a bit")
    }

    fn on(session: &Session, n: u16) -> bool {
        session.pilot().control_bit(bit(n))
    }

    fn active(session: &Session, id: i16) -> bool {
        session.galaxy().nebula_active(NebulaId(id))
    }

    fn explored(session: &Session, id: i16) -> bool {
        session.galaxy().nebula_explored(NebulaId(id))
    }

    fn run(session: &mut Session, script: &str) {
        session.run_set(&SetExpr::parse(script).expect("parses"), &mut NeverFires);
    }

    fn settle(session: &mut Session, catalog: &FakePilotCatalog) {
        let _ = session.settle_script(catalog, &mut NeverFires);
    }

    /// `catalog`'s pilot flown with `bits` set, by `rule`, its load
    /// settled.
    fn flown(catalog: &FakePilotCatalog, bits: &[u16], rule: MultiJumpRule) -> Session {
        let mut pilot = Pilot::new(catalog, "").expect("starts");
        for &n in bits {
            pilot.bits.set(bit(n));
        }
        let mut session = Session::fly(catalog, pilot)
            .expect("flies")
            .with_multi_jump(rule);
        settle(&mut session, catalog);
        session
    }

    /// Plots a course to `to`, flies out and begins the jump, then runs
    /// `script` and settles it: by the engine's `visibility_refresh`, no
    /// refresh until the jump's.
    fn bound(session: &mut Session, catalog: &FakePilotCatalog, to: i16, script: &str) {
        session.plot_course(SystemId(to)).expect("a route");
        fly_out(session);
        begin_jump_now(session).expect("jumps");
        run(session, script);
        settle(session, catalog);
    }

    /// The player's system as each set operator applies, but a bit's,
    /// which no observer sees.
    #[derive(Debug, Default)]
    struct Systems(RefCell<Vec<SystemId>>);

    /// The systems whose `Explored` [`watching_explored`] saw dispatched,
    /// in order.
    #[derive(Debug, Default)]
    pub(in crate::session) struct Explorations(RefCell<Vec<SystemId>>);

    impl Explorations {
        /// The systems explored, in order.
        pub(in crate::session) fn systems(&self) -> Vec<SystemId> {
            self.0.borrow().clone()
        }
    }

    impl SetOpObserver for Explorations {
        fn saw(&self, op: &SetOp, _session: &Session) {
            if let SetOp::AbortMission(mission) = op {
                self.0.borrow_mut().push(SystemId(mission.0));
            }
        }
    }

    /// `session` watching the `Explored` events of `systems` dispatched: a
    /// fixture subscription to each runs `A` of the system's ID, a mission
    /// never active, which the [`Explorations`] given back sees.
    pub(in crate::session) fn watching_explored(
        mut session: Session,
        systems: &[i16],
    ) -> (Session, Rc<Explorations>) {
        for &system in systems {
            session
                .register_story(Subscription::new(
                    SubscriptionId {
                        consumer: Consumer::Fixture,
                        record: system,
                        hook: 0,
                    },
                    EventFilter::Explored(SystemId(system)),
                    Script::parse(&format!("A{system}")),
                ))
                .expect("registers");
        }
        let seen = Rc::new(Explorations::default());
        (session.with_set_op_observer(seen.clone()), seen)
    }

    /// [`around_131`] whose `OnExplore` also explores 132, an operator a
    /// [`Systems`] sees.
    fn observed_around_131(active_on: &str) -> NebulaRecord {
        nebula(128, (500, -100, 200, 200), active_on, "b7 X132")
    }

    impl SetOpObserver for Systems {
        fn saw(&self, _op: &SetOp, session: &Session) {
            self.0.borrow_mut().push(session.system());
        }
    }

    #[test]
    fn a_jump_runs_the_on_explore_of_a_nebula_active_before_its_refresh_and_inactive_after() {
        // Passed through: 130 to 132 by 131, the nebula around 131.
        let catalog = chained(nebulous(vec![around_131("b5")]), 2);
        let mut session = flown(&catalog, &[5], MultiJumpRule::Engine);
        assert!(active(&session, 128), "b5 at the load's refresh");
        bound(&mut session, &catalog, 132, "!b5");
        assert!(active(&session, 128), "no refresh since");
        assert_eq!(
            session.arrive(&catalog, &mut NeverFires),
            Some(SystemId(132))
        );
        assert!(!active(&session, 128), "the jump's refresh");
        assert!(explored(&session, 128), "131 passed while it was active");
        assert!(on(&session, 7), "its OnExplore ran");

        // Arrived in: 131 itself.
        let catalog = nebulous(vec![around_131("b5")]);
        let mut session = flown(&catalog, &[5], MultiJumpRule::Engine);
        bound(&mut session, &catalog, 131, "!b5");
        session.arrive(&catalog, &mut NeverFires).expect("arrives");
        assert!(!active(&session, 128), "the jump's refresh");
        assert!(explored(&session, 128));
        assert!(on(&session, 7));
    }

    #[test]
    fn a_nebula_the_jumps_refresh_activates_is_explored_only_over_the_arrival() {
        // Passed through: not explored, as the original explores only the
        // arrival again.
        let catalog = chained(nebulous(vec![around_131("b5")]), 2);
        let mut session = flown(&catalog, &[], MultiJumpRule::Engine);
        bound(&mut session, &catalog, 132, "b5");
        assert!(!active(&session, 128), "no refresh since");
        session.arrive(&catalog, &mut NeverFires).expect("arrives");
        assert!(active(&session, 128), "the jump's refresh");
        settle(&mut session, &catalog);
        assert!(!explored(&session, 128), "131 passed while inactive");
        assert!(!on(&session, 7));

        // Arrived in: the re-explore runs its OnExplore, once.
        let catalog = nebulous(vec![observed_around_131("b5")]);
        let mut session = flown(&catalog, &[], MultiJumpRule::Engine);
        bound(&mut session, &catalog, 131, "b5");
        let seen = Rc::new(Systems::default());
        let mut session = session.with_set_op_observer(seen.clone());
        session.arrive(&catalog, &mut NeverFires).expect("arrives");
        assert!(explored(&session, 128));
        assert!(on(&session, 7));
        settle(&mut session, &catalog);
        assert_eq!(seen.0.borrow().len(), 1, "X132, once");
    }

    #[test]
    fn the_arrivals_explore_again_runs_before_the_system_is_entered_and_populated() {
        let catalog = nebulous(vec![observed_around_131("b5")]);
        let mut session = flown(&catalog, &[], MultiJumpRule::Engine);
        bound(&mut session, &catalog, 131, "b5");
        let seen = Rc::new(Systems::default());
        let mut session = session.with_set_op_observer(seen.clone());
        session.arrive(&catalog, &mut NeverFires).expect("arrives");
        assert_eq!(session.system(), SystemId(131));
        assert_eq!(
            *seen.0.borrow(),
            [SystemId(131)],
            "its OnExplore ran in the arrival, before the entry"
        );
    }

    #[test]
    fn an_on_explore_during_a_jump_reads_the_arrival_as_the_players_system() {
        // Arrived in 131: the original writes it before exploring it
        // (@0x6be0b, @0x6be12).
        let catalog = nebulous(vec![observed_around_131("")]);
        let mut session = flown(&catalog, &[], MultiJumpRule::Engine);
        let seen = Rc::new(Systems::default());
        session = session.with_set_op_observer(seen.clone());
        jump(&mut session, &catalog, 131).expect("arrives");
        assert_eq!(*seen.0.borrow(), [SystemId(131)]);

        // 130 to 132 by 131: each hop's stand-in written before it is
        // explored (@0x6bdd4, @0x6bddb), then the arrival.
        let catalog = chained(
            nebulous(vec![
                observed_around_131(""),
                nebula(129, (500, 500, 200, 200), "", "X133"),
            ]),
            2,
        );
        let mut session = flown(&catalog, &[], MultiJumpRule::Engine);
        bound(&mut session, &catalog, 132, "");
        let seen = Rc::new(Systems::default());
        let mut session = session.with_set_op_observer(seen.clone());
        assert_eq!(
            session.arrive(&catalog, &mut NeverFires),
            Some(SystemId(132))
        );
        assert_eq!(*seen.0.borrow(), [SystemId(131), SystemId(132)]);
    }

    #[test]
    fn a_map_an_on_explore_grants_during_a_jump_reveals_around_the_arrival() {
        // A map of 1 jump: from 131, 130 and 132; from the departure 130,
        // only 131.
        let catalog = FakePilotCatalog {
            outfits: vec![outfit(300, &[(MAP, 1)])],
            ..nebulous(vec![nebula(128, (500, -100, 200, 200), "", "G300")])
        };
        let mut session = flown(&catalog, &[], MultiJumpRule::Engine);
        assert_eq!(
            session.pilot().explored_level(SystemId(132)),
            ExploredLevel::Unexplored
        );
        jump(&mut session, &catalog, 131).expect("arrives");
        assert_eq!(
            session.pilot().explored_level(SystemId(132)),
            ExploredLevel::Charted
        );
    }

    #[test]
    fn a_cron_on_a_jumps_day_reads_the_arrival_as_the_players_system() {
        let catalog = FakePilotCatalog {
            crons: vec![scripted(128, 1, "X132", "")],
            ..nebulous(Vec::new())
        };
        let mut session = flown(&catalog, &[], MultiJumpRule::Engine);
        let seen = Rc::new(Systems::default());
        session = session.with_set_op_observer(seen.clone());
        jump(&mut session, &catalog, 131).expect("arrives");
        assert_eq!(*seen.0.borrow(), [SystemId(131)], "the crön's X132");
    }

    #[test]
    fn a_jump_into_a_system_its_refresh_hides_reads_the_system_moved_to_on_its_days() {
        // 134 shares 131's place; the jump's refresh hides 131, and its move
        // takes the player to 134 (@0x99918) before the days (@0x6c144).
        let mut catalog = alternates();
        catalog.crons.push(scripted(128, 1, "X132", ""));
        let mut session = flown(&catalog, &[], MultiJumpRule::Engine);
        session.plot_course(SystemId(131)).expect("a route");
        fly_out(&mut session);
        assert_eq!(begin_jump_now(&mut session), Ok(SystemId(131)));
        session.pilot.bits_mut().set(hiding_bit(131));
        let seen = Rc::new(Systems::default());
        let mut session = session.with_set_op_observer(seen.clone());
        assert_eq!(
            session.arrive(&catalog, &mut NeverFires),
            Some(SystemId(134))
        );
        assert_eq!(*seen.0.borrow(), [SystemId(134)], "the crön's X132");
    }

    /// The missions each `A` operator aborts, in order, which an
    /// [`EventFilter`]'s fixture script names to say it ran.
    #[derive(Debug, Default)]
    struct Aborts(RefCell<Vec<i16>>);

    impl SetOpObserver for Aborts {
        fn saw(&self, op: &SetOp, _session: &Session) {
            if let SetOp::AbortMission(mission) = op {
                self.0.borrow_mut().push(mission.0);
            }
        }
    }

    /// `session` with a fixture subscription, `record`, to `filter`,
    /// running `A{record}`, which an [`Aborts`] sees: a mission (128 to
    /// 1127) never active.
    fn aborting_on(mut session: Session, record: i16, filter: EventFilter) -> Session {
        session
            .register_story(Subscription::new(
                SubscriptionId {
                    consumer: Consumer::Fixture,
                    record,
                    hook: 0,
                },
                filter,
                Script::parse(&format!("A{record}")),
            ))
            .expect("registers");
        session
    }

    #[test]
    fn a_jump_into_a_system_its_refresh_hides_enters_the_system_moved_to_before_exploring_it_again()
    {
        // As above: the refresh hides 131, the system the jump wrote, and
        // its move takes the player to 134. The enter-system hook (@0x6c15d)
        // names 134, before the re-explore of 134 (@0x6c1db); 131's never
        // runs.
        let catalog = alternates();
        let mut session = flown(&catalog, &[], MultiJumpRule::Engine);
        session.plot_course(SystemId(131)).expect("a route");
        fly_out(&mut session);
        assert_eq!(begin_jump_now(&mut session), Ok(SystemId(131)));
        session.pilot.bits_mut().set(hiding_bit(131));
        let session = aborting_on(session, 601, EventFilter::EnteredSystem(SystemId(134)));
        let session = aborting_on(session, 602, EventFilter::EnteredSystem(SystemId(131)));
        let session = aborting_on(session, 603, EventFilter::Explored(SystemId(134)));
        let seen = Rc::new(Aborts::default());
        let mut session = session.with_set_op_observer(seen.clone());
        assert_eq!(
            session.arrive(&catalog, &mut NeverFires),
            Some(SystemId(134))
        );
        assert_eq!(
            *seen.0.borrow(),
            [601, 603],
            "134's hook during the arrival, then its re-explore; never 131's hook"
        );
    }

    /// [`shared`] with 128, shadowed by 127 at (-600, 600), and 131
    /// listing 160, which no system owns or rescues: the lowest visible
    /// lister, 128, is `_FindSystemFromStellar`'s. 150 is kept rescued in
    /// the departure 130 by [`rescued_in_130`]. The `OnExplore` of the
    /// nebula around 131 starts 141, to 160, and 142, to 150.
    fn unowned_160() -> FakePilotCatalog {
        let mut catalog = shared();
        catalog.star_map.push(star(127, (-600.0, 600.0), &[]));
        catalog.star_map.push(star(128, (-600.0, 600.0), &[]));
        catalog.systems.push(SystemId(128));
        catalog
            .sites
            .push((SystemId(128), vec![planet(160, 400.0, 300.0)]));
        for (system, sites) in &mut catalog.sites {
            if *system == SystemId(131) {
                sites.push(planet(160, 400.0, 300.0));
            }
        }
        hideable(&mut catalog);
        for (id, stellar) in [(141, 160), (142, 150)] {
            catalog.missions.push(MissionRecord {
                travel_stel: stellar,
                return_stel: stellar,
                ..mission(id)
            });
        }
        catalog
            .nebulae
            .push(nebula(128, (500, -100, 200, 200), "", "S141 S142"));
        catalog
    }

    /// [`unowned_160`]'s pilot, reading `rescue_until_refresh` as
    /// `source`, jumped to `to` from 130 along the course.
    fn jumped_from_130(catalog: &FakePilotCatalog, source: RuleSource, to: i16) -> Session {
        let mut session = rescued_in_130(catalog, source);
        session.plot_course(SystemId(to)).expect("a route");
        fly_out(&mut session);
        assert_eq!(begin_jump_now(&mut session), Ok(SystemId(131)));
        assert_eq!(
            session.arrive(catalog, &mut NeverFires),
            Some(SystemId(to)),
            "{source:?}"
        );
        session
    }

    /// The system of the destination of the mission held in `slot`.
    fn destination(session: &Session, slot: u8) -> Option<SystemId> {
        session
            .pilot()
            .missions()
            .get(slot)
            .expect("held")
            .destination
            .and_then(|stop| stop.system)
    }

    #[test]
    fn a_jumps_explorations_derive_no_rescue_for_the_arrival() {
        let catalog = unowned_160();
        for source in RuleSource::ALL {
            let session = jumped_from_130(&catalog, source, 131);
            // By the other reading the holder derives the player's system,
            // 131 now, as that reading always does.
            assert_eq!(
                destination(&session, 0),
                by(source, Some(SystemId(128)), Some(SystemId(131))),
                "{source:?}: 160 unowned while the rebuild is awaited"
            );
            assert_eq!(
                destination(&session, 1),
                by(source, Some(SystemId(129)), Some(SystemId(131))),
                "{source:?}: 150 kept rescued in 130, which 129 stands for"
            );
        }
    }

    /// [`unowned_160`] with 132 and ship 128 jumping two systems a jump,
    /// so a jump to 132 passes 131, whose `OnExplore` runs on the hop.
    fn hopping_over_131() -> FakePilotCatalog {
        let mut catalog = unowned_160();
        catalog.systems.push(SystemId(132));
        catalog.defaults = vec![(ShipId(128), vec![(OutfitId(320), 1)])];
        catalog.outfits.push(outfit(320, &[(MULTI_JUMP, 2)]));
        catalog
    }

    #[test]
    fn a_hops_exploration_derives_no_rescue_for_the_hop() {
        // 130 to 132 by 131: the hop's stand-in, 131, is written
        // (@0x6bdd4) before it is explored (@0x6bddb), so the galaxy
        // awaits its rebuild from there, and 160 stays unowned.
        let catalog = hopping_over_131();
        for source in RuleSource::ALL {
            let session = jumped_from_130(&catalog, source, 132);
            // By the other reading the holder derives the player's system,
            // the hop's 131 as its `OnExplore` runs.
            assert_eq!(
                destination(&session, 0),
                by(source, Some(SystemId(128)), Some(SystemId(131))),
                "{source:?}: 160 unowned while the rebuild is awaited"
            );
        }
    }

    #[test]
    fn a_hops_exploration_keeps_the_departures_rescue() {
        // The departure 130's rescue of 150 holds through the hop over
        // 131: nothing is rescued for the hop.
        let catalog = hopping_over_131();
        for source in RuleSource::ALL {
            let session = jumped_from_130(&catalog, source, 132);
            assert_eq!(
                destination(&session, 1),
                by(source, Some(SystemId(129)), Some(SystemId(131))),
                "{source:?}: 150 kept rescued in 130, which 129 stands for"
            );
        }
    }

    /// The system each `X` names, with the NPCs in the system as it
    /// applies.
    #[derive(Debug, Default)]
    struct Populated(RefCell<Vec<(i16, usize)>>);

    impl SetOpObserver for Populated {
        fn saw(&self, op: &SetOp, session: &Session) {
            if let SetOp::Explore(system) = op {
                self.0.borrow_mut().push((system.0, session.npcs().len()));
            }
        }
    }

    #[test]
    fn a_jumps_entered_system_hook_settles_before_the_arrivals_re_explore_and_population() {
        // `_MissionHandlePlayerEnteredNewSystem` (@0x6c15d) before the
        // re-explore (@0x6c1db) and the setup (@0x6c1f9). The nebula
        // around 131 is active only from the jump's refresh, so only the
        // re-explore runs its `X132`; 131's traffic is two ships.
        let mut dude_types = [(-1, 0); 8];
        dude_types[0] = (128, 100);
        let catalog = FakePilotCatalog {
            traffic: vec![(
                SystemId(131),
                SystemTraffic {
                    dude_types,
                    avg_ships: 2,
                    persons: Default::default(),
                },
            )],
            dudes: vec![(
                DudeId(128),
                DudeRecord {
                    ai_type: 1,
                    govt: None,
                    ships: vec![(ShipId(129), 1)],
                    booty: 0,
                    info_types: 0,
                },
            )],
            ship_records: vec![ship(129, FAST)],
            ..nebulous(vec![observed_around_131("b5")])
        };
        let mut session = flown(&catalog, &[], MultiJumpRule::Engine);
        session
            .register_story(Subscription::new(
                SubscriptionId {
                    consumer: Consumer::Fixture,
                    record: 1,
                    hook: 0,
                },
                EventFilter::EnteredSystem(SystemId(131)),
                Script::parse("X133"),
            ))
            .expect("registers");
        bound(&mut session, &catalog, 131, "b5");
        let seen = Rc::new(Populated::default());
        let mut session = session.with_set_op_observer(seen.clone());
        session.arrive(&catalog, &mut NeverFires).expect("arrives");
        assert_eq!(
            *seen.0.borrow(),
            [(133, 0), (132, 0)],
            "the hook, then the re-explore, both before the setup"
        );
        assert_eq!(session.npcs().len(), 2, "the setup");
        settle(&mut session, &catalog);
        assert_eq!(seen.0.borrow().len(), 2, "nothing left to settle");
    }

    #[test]
    fn a_cron_on_the_jumps_day_reads_a_bit_an_on_explore_set_on_that_jump() {
        let mut reads_b7 = scripted(128, 1, "b10", "");
        reads_b7.enable_on = Test::parse("b7");
        let catalog = FakePilotCatalog {
            crons: vec![reads_b7],
            ..nebulous(vec![around_131("")])
        };
        let mut session = flown(&catalog, &[], MultiJumpRule::Engine);
        jump(&mut session, &catalog, 131).expect("arrives");
        assert!(on(&session, 7), "the arrival's OnExplore");
        assert!(on(&session, 10), "the crön started on the jump's day");
    }

    #[test]
    fn by_per_hop_a_cron_on_a_further_hops_day_reads_a_bit_an_on_explore_set_on_that_jump() {
        // 130 to 132 by 131, a day each hop: crön 129 starts on the first
        // day, setting b11, so crön 128 can start only on the second, the
        // further hop's; it reads b7, which 132's OnExplore sets.
        let mut second_day = scripted(128, 1, "b10", "");
        second_day.enable_on = Test::parse("b7 & b11");
        let first_day = scripted(129, 5, "b11", "");
        let catalog = FakePilotCatalog {
            crons: vec![second_day, first_day],
            ..chained(
                nebulous(vec![nebula(128, (500, 500, 200, 200), "", "b7")]),
                1,
            )
        };
        let mut session = flown(&catalog, &[], MultiJumpRule::PerHop);
        bound(&mut session, &catalog, 132, "");
        let left = session.date();
        assert_eq!(
            session.arrive(&catalog, &mut NeverFires),
            Some(SystemId(132))
        );
        assert_eq!(session.date(), left.next_day().next_day(), "two hops' days");
        assert!(on(&session, 11), "the first day's crön");
        assert!(on(&session, 10), "the second day's crön read b7");
    }

    #[test]
    fn a_jumps_fuel_is_taken_after_an_on_explore_holds_it_to_a_smaller_class() {
        // 300 fuel; 131's OnExplore changes to ship 130, of 200 most. The
        // original holds the fuel to 200 there (@0x6be12), then takes the
        // jump's 100 (@0x6be31): 100, where taking it first leaves 200.
        let mut catalog = nebulous(vec![nebula(128, (500, -100, 200, 200), "", "C130")]);
        let small = ShipFields { fuel: 200, ..FAST };
        catalog.ships.push((ShipId(130), Ok(small)));
        catalog.ship_records.push(ship(130, small));
        for rule in [MultiJumpRule::Engine, MultiJumpRule::PerHop] {
            let mut session = flown(&catalog, &[], rule);
            assert_eq!(session.reserves().fuel, Gauge::full(300.0));
            jump(&mut session, &catalog, 131).expect("arrives");
            assert_eq!(
                session.reserves().fuel,
                Gauge {
                    now: 100.0,
                    max: 200.0
                },
                "{rule:?}"
            );
        }
    }

    #[test]
    fn an_on_explore_bit_reaches_the_jumps_refresh() {
        let mut catalog = nebulous(vec![around_131("")]);
        catalog.systems.push(SystemId(133));
        catalog.visibility.push(SystemVisibility {
            id: SystemId(133),
            visibility: Test::parse("b7"),
        });
        let mut session = flown(&catalog, &[], MultiJumpRule::Engine);
        assert!(!session.galaxy().visible(SystemId(133)));
        jump(&mut session, &catalog, 131).expect("arrives");
        assert!(session.galaxy().visible(SystemId(133)), "b7 refreshed");
    }

    #[test]
    fn events_raised_before_a_jump_wait_and_an_on_explores_move_waits_for_the_settling() {
        let mut catalog = nebulous(vec![nebula(128, (500, -100, 200, 200), "", "b7 M133")]);
        catalog.systems.push(SystemId(133));
        let mut session = flown(&catalog, &[], MultiJumpRule::Engine);
        let landing = SubscriptionId {
            consumer: Consumer::Fixture,
            record: 1,
            hook: 0,
        };
        session
            .register_story(Subscription::new(
                landing,
                EventFilter::Any(StoryEventKind::Landed),
                Script::parse("b20"),
            ))
            .expect("registers");
        let landed = StoryEvent::Landed {
            stellar: StellarId(128),
        };
        session.run.raise(landed);
        jump(&mut session, &catalog, 131).expect("arrives");
        assert!(on(&session, 7), "the OnExplore ran");
        assert!(!on(&session, 20), "the landing waits");
        let raised = session.story_raised();
        assert_eq!(raised.first(), Some(&landed), "first, as raised");
        assert!(
            !raised
                .iter()
                .any(|event| event.kind() == StoryEventKind::Explored),
            "{raised:?}"
        );
        assert_eq!(session.system(), SystemId(131), "M133 waits");
        let settled = session.settle_script(&catalog, &mut NeverFires);
        assert_eq!(settled.moved, Some(SystemId(133)));
        assert!(on(&session, 20));
    }

    #[test]
    fn only_the_explorations_raised_since_the_mark_are_dispatched_at_once() {
        // No script operator raises a story event of another kind, so a
        // landing and a take-off stand for one raised before the mark and
        // one raised since, beside the exploration.
        let catalog = nebulous(vec![around_131("")]);
        let mut session = flown(&catalog, &[], MultiJumpRule::Engine);
        for (record, kind, script) in [
            (1, StoryEventKind::Landed, "b20"),
            (2, StoryEventKind::TookOff, "b21"),
        ] {
            session
                .register_story(Subscription::new(
                    SubscriptionId {
                        consumer: Consumer::Fixture,
                        record,
                        hook: 0,
                    },
                    EventFilter::Any(kind),
                    Script::parse(script),
                ))
                .expect("registers");
        }
        let (mut session, seen) = watching_explored(session, &[131]);
        let landed = StoryEvent::Landed {
            stellar: StellarId(128),
        };
        let took_off = StoryEvent::TookOff {
            stellar: StellarId(128),
        };
        session.run.raise(landed);
        let mark = session.run.story_mark();
        session.run.raise(took_off);
        session.run.raise(StoryEvent::Explored {
            system: SystemId(131),
        });
        session.dispatch_explored_since(&catalog, &mut NeverFires, mark);
        assert_eq!(seen.systems(), [SystemId(131)], "the exploration, now");
        assert!(on(&session, 7), "its OnExplore ran");
        assert!(!on(&session, 20) && !on(&session, 21), "the others wait");
        assert_eq!(session.story_raised(), [landed, took_off], "in order");
        settle(&mut session, &catalog);
        assert!(on(&session, 20) && on(&session, 21));
    }

    /// One message, `Go.`, in `STR#` 25048.
    #[derive(Debug)]
    struct Go;

    impl crate::catalog::CommCatalog for Go {
        fn string_list(&self, id: i16) -> Vec<String> {
            if id == 25048 {
                vec!["Go.".to_owned()]
            } else {
                Vec::new()
            }
        }
    }

    #[test]
    fn an_on_explores_leave_waits_for_the_settling() {
        let catalog = nebulous(vec![nebula(128, (500, -100, 200, 200), "", "b7 Q25048")]);
        let mut session = flown(&catalog, &[], MultiJumpRule::Engine).with_strings(Rc::new(Go));
        jump(&mut session, &catalog, 131).expect("arrives");
        assert!(on(&session, 7), "the OnExplore ran");
        let settled = session.settle_script(&catalog, &mut NeverFires);
        assert_eq!(settled.message.as_deref(), Some("Go."), "its Q, held");
    }

    #[test]
    fn a_new_pilots_start_exploration_kept_by_the_other_reading_waits_for_the_settling() {
        use crate::rulebook::{RuleKey, RuleSource, Rulebook};
        let catalog = nebulous(vec![nebula(128, (-100, -100, 200, 200), "", "b7")]);
        let rules = Rulebook::default().with_override(RuleKey::NebulaNewPilot, RuleSource::Bible);
        let mut session = Session::fly(&catalog, Pilot::new(&catalog, "").expect("starts"))
            .expect("flies")
            .with_rules(rules);
        session.begin(&catalog, &mut NeverFires);
        assert!(!on(&session, 7), "raised before the OnStart: it waits");
        assert!(session.story_raised().contains(&StoryEvent::Explored {
            system: SystemId(130)
        }));
        settle(&mut session, &catalog);
        assert!(on(&session, 7));
    }

    #[test]
    fn a_new_pilots_on_start_map_explores_with_the_activity_from_before_it() {
        // OnStart sets b5 and maps 130, 131 and 132: the nebula around 131,
        // active by b5, was inactive at `_DoNewPilot`'s refresh; the one
        // around 132 was active, and its b8 shows 133 at `_PlayGame`'s.
        let mut catalog = FakePilotCatalog {
            outfits: vec![outfit(300, &[(MAP, 2)])],
            ..nebulous(vec![
                around_131("b5"),
                nebula(129, (500, 500, 200, 200), "", "b8"),
            ])
        };
        if let Ok(start) = &mut catalog.character {
            start.on_start = Script::parse("b5 G300");
        }
        catalog.systems.push(SystemId(133));
        catalog.visibility.push(SystemVisibility {
            id: SystemId(133),
            visibility: Test::parse("b8"),
        });
        let mut session =
            Session::fly(&catalog, Pilot::new(&catalog, "").expect("starts")).expect("flies");
        session.begin(&catalog, &mut NeverFires);
        assert!(explored(&session, 129));
        assert!(on(&session, 8));
        assert!(session.galaxy().visible(SystemId(133)), "b8 refreshed");
        assert!(active(&session, 128), "b5 at play's refresh");
        settle(&mut session, &catalog);
        assert!(!explored(&session, 128), "inactive as it was mapped");
        assert!(!on(&session, 7));
    }
}
