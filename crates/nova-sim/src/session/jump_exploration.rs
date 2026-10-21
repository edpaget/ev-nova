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
//! resolve it, before it enters and populates the system;
//! [`Session::begin`] dispatches the `OnStart`'s explorations before
//! `_PlayGame`'s refresh.
//!
//! **Ordering.** A jump's `EnteredSystem` hooks settle in the caller's
//! settling, after the re-explore and the population, where the original
//! runs them (@0x6c15d) before both; the jump's explorations and days see
//! the departure as the player's system, where the original writes the
//! arrival first (@0x6bdd4, @0x6be0b); and a map granted by a script run
//! outside the dispatcher (a crön's, a ship's hook, a stellar's `OnRegen`)
//! explores at the next settling. Each crosses a boundary the player can
//! see; crons-news-ranks-fidelity phase 40 covers them.
//!
//! [`RuleKey::StoryReentry`]: crate::RuleKey::StoryReentry

use crate::catalog::{PilotCatalog, SystemId, TrafficCatalog};
use crate::chance::Chance;
use crate::story::StoryEvent;

use super::Session;

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
}

#[cfg(test)]
pub(super) mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use super::super::crons::tests::scripted;
    use super::super::nebulae::tests::{around_131, nebulous};
    use super::super::tests::chained;
    use crate::catalog::{NebulaId, NebulaRecord, ShipId, StellarId, SystemId, SystemVisibility};
    use crate::chance::NeverFires;
    use crate::control::{Bit, Script, SetExpr, SetOp, Test};
    use crate::handling::ShipFields;
    use crate::hyperspace::MultiJumpRule;
    use crate::outfit_effects::MAP;
    use crate::pilot::Pilot;
    use crate::reserves::Gauge;
    use crate::session::Session;
    use crate::session::control::SetOpObserver;
    use crate::story::{
        Consumer, EventFilter, StoryEvent, StoryEventKind, Subscription, SubscriptionId,
    };
    use crate::testkit::{
        FAST, FakePilotCatalog, begin_jump_now, fly_out, jump, nebula, outfit, ship,
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
            [SystemId(130)],
            "its OnExplore ran before the entry wrote the system"
        );
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
