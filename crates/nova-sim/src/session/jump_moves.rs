//! An in-flight `M` or `N` during a jump: the jump's own
//! (crons-news-ranks-fidelity phase 44). All addresses are in
//! `EV Nova.app`'s `EV Nova` (i386).
//!
//! **The original.** A jump's explorations (@0x6bddb, @0x6be12), its
//! days (@0x6c144), its enter-system hook (@0x6c15d) and its re-explore
//! (@0x6c1db) all run inside one `_HandlePlayer` call, before its setup
//! (@0x6c1f9). An `M` or `N` a script runs there acts at its operator, as
//! anywhere in flight (`_EvalSetExp` @0x1570e-0x159ad): it writes the
//! player's system (+0x74, @0x1572b, @0x158f2) and clears +0x6c; `M`
//! puts the ship on the first `NavDefs` slot that isn't -1
//! (@0x15756-0x157f5), its velocity zeroed and its speed along its
//! heading kept; both carry the escorts' +0x74 (@0x15807-0x1583e,
//! @0x1590b-0x1593d), set the five kill flags (@0x158a4-0x158cf,
//! @0x1596e-0x15993), rebuild the stellars (HSSV @0x159a3) and run
//! `_MissionHandlePlayerEnteredNewSystem` (@0x159a8). Neither sets the
//! system up. The jump's tail then sets up +0x74 as the moves left it:
//! the hook (@0x6c15d), the re-explore (@0x6c1db), the setup (@0x6c1f9),
//! the bounty hunter (@0x6c1fe) and its rebuild (@0x6c203). It places the
//! ship no more: the arrival block ran before (see the `jump_arrival`
//! module). So an `M` arrives at its first stellar, an `N` where the
//! arrival block put the ship.
//!
//! A hop's `OnExplore` move (@0x6bddb) is written over: the hop loop
//! writes the next hop's stand-in (@0x6bdd4), then the arrival (@0x6be0b),
//! over its +0x74. What stays of it is its placement, its rebuild (the
//! rescue for its target, which the jump's refresh then clears, @0x6bf3f),
//! its enter-system hook, its kill flags and `N`'s take-off hold. Only the
//! arrival's `OnExplore` (@0x6be12) and the days' moves keep their system;
//! the refresh's move out of a hidden system (@0x99918) acts on the system
//! as the move left it.
//!
//! **Here.** [`Session::arrive`] opens a window as it ends the jump and
//! closes it once it has entered the system ([`JumpWindow`], on the
//! [`Chart`]). While it is open, `Session::apply_op` hands each move a
//! set operator queues to [`Session::move_in_jump`], at the operator,
//! with no catalog:
//!
//! - A move to a system the galaxy holds writes the player's system and
//!   lets go of the nav selection. `M` stops the ship on the system's first
//!   stellar, read as the session starts, at its own position (the
//!   `spöb`'s `xPos`, `yPos`), or by [`ScriptEffectRules::starless`] at
//!   the centre with none; `N` keeps it, holding the next take-off's
//!   landing position by [`ScriptEffectRules::keep_flag`]'s engine
//!   reading, as a queued move does.
//! - A move to a system with no record, or an `N` out of range, is the
//!   tail with no move, by [`ScriptEffectRules::missing_system_move`]'s and
//!   [`ScriptEffectRules::out_of_range_move`]'s engine readings; by the
//!   other readings it does nothing.
//! - The rebuild's catalog-free parts run for the player's system: the
//!   course rewritten, the exploration shared by place, the known set
//!   worked out and the rescue kept, which ends the wait for the rebuild
//!   (see the `owners_unset` module).
//! - Under [`ScriptEffectRules::arrival`]'s other reading the move is an
//!   arrival too: its system explored, [`StoryEvent::Explored`] raised for
//!   the caller's settling and the course cleared; the jump's entry
//!   populates.
//! - The window notes that a move ran, which kills the explosions the
//!   jump made (see the `killed_explosions` module).
//! - [`StoryEvent::EnteredSystem`], entered by an in-flight move, is
//!   raised for the player's system and dispatched at once, events only,
//!   with no catalog; the ship goals wait for the jump's check after its
//!   setup (see the `mission_ships` module). A move a script of that
//!   dispatch runs only raises its own, which the running dispatch takes
//!   within its budget, so scripts that move each other end.
//!
//! The jump's tail then reads the player's system as the moves left it:
//! the refresh's move, the hook, the re-explore and the entry, which
//! populates once and places the ship no more by the engine's
//! `jump_arrival` (by the other reading, at rest on the last leg's side
//! unless a move placed it; after an `N`, or an `M` that keeps the ship,
//! the entry still places it).
//!
//! **Ordering.** The stellars shown, the stellar sprites and the sprite
//! pool wait for the entry's rebuild: nothing is drawn between the move
//! and the entry, so that crosses no boundary.
//!
//! **Left out.** The escorts' +0x74 carry: the jump's setup respawns them
//! (`_RespawnEscort` @0x42c79), which writes their +0x74 from the
//! player's (@0x3d565-0x3d571), and the entry brings in the ships the
//! player leads. The other four kill flags and +0x70 = -1 (@0x1599d) touch
//! nothing a jump's window holds.
//!
//! [`ScriptEffectRules::starless`]: super::ScriptEffectRules::starless
//! [`ScriptEffectRules::keep_flag`]: super::ScriptEffectRules::keep_flag
//! [`ScriptEffectRules::missing_system_move`]: super::ScriptEffectRules::missing_system_move
//! [`ScriptEffectRules::out_of_range_move`]: super::ScriptEffectRules::out_of_range_move
//! [`ScriptEffectRules::arrival`]: super::ScriptEffectRules::arrival

use super::entry::{Arrival, Entry, Placement, ShipPlacement};
use super::script_effects::{ScriptMove, move_placement};
use super::story::NoCatalog;
use super::{Chart, Session};
use crate::catalog::{PilotCatalog, SystemId, TrafficCatalog};
use crate::chance::Chance;
use crate::pilot::ExploredLevel;
use crate::rulebook::RuleSource;
use crate::story::{Entrance, StoryEvent, StoryEventKind};

/// A jump's window, from its end to its entry (see the module docs).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct JumpWindow {
    /// Whether a move into a system placed the ship in it: an `M` onto a
    /// stellar, or at the centre by [`ScriptEffectRules::starless`]'s
    /// other reading, not an `N` or a starless `M` that keeps it.
    ///
    /// [`ScriptEffectRules::starless`]: super::ScriptEffectRules::starless
    pub(super) placed: bool,
    /// Whether a move or a move's tail ran in it, setting the kill flags.
    pub(super) killed: bool,
    /// Whether a move's own enter-system dispatch is running.
    dispatching: bool,
}

impl Chart {
    /// Opens a jump's window (see the module docs).
    pub(super) fn open_jump_window(&mut self) {
        self.jump_window = Some(JumpWindow::default());
    }

    /// Closes the jump's window, giving what it noted.
    pub(super) fn close_jump_window(&mut self) -> JumpWindow {
        self.jump_window.take().unwrap_or_default()
    }

    /// Whether a jump's window is open.
    pub(super) fn in_jump_window(&self) -> bool {
        self.jump_window.is_some()
    }
}

impl Session {
    /// Enters the player's system as the jump's tail does, `how`, from
    /// the `departure`, its last leg `from` one system `at` another,
    /// reading `catalog` and drawing on `chance` to populate (see the
    /// module docs): the ship placed by the entry only by the other
    /// reading of `jump_arrival`, and then only when no move placed it (an
    /// `N` keeps the entry's placement). The window closes once it has
    /// entered, and the explosions the jump made are killed when a move or
    /// a move's tail ran in it.
    pub(super) fn enter_jump_system<C>(
        &mut self,
        how: Entry,
        (departure, from, at): (SystemId, SystemId, SystemId),
        catalog: &C,
        chance: &mut (impl Chance + ?Sized),
    ) where
        C: PilotCatalog + TrafficCatalog,
    {
        let system = self.pilot.system;
        let placed = self.chart.jump_window.is_some_and(|window| window.placed);
        let ship = if placed {
            ShipPlacement::Keep
        } else {
            self.jump_entry_placement(from, at)
        };
        let entered = Arrival {
            system,
            sites: self.chart.read_sites(catalog, system),
            placement: Placement {
                ship,
                stellar: None,
                hold: false,
            },
        };
        // The entry leaves the departure, handed in: the player's system
        // is the one the jump and its moves wrote, and the entry writes its
        // own (`set_player_system`).
        self.enter_system(how, departure, entered, catalog, catalog, chance);
        // The jump's explosions, killed when a move ran in it (see the
        // `killed_explosions` module).
        let window = self.chart.close_jump_window();
        self.kill_jump_explosions(window);
    }

    /// Applies as the jump's own, at once, each move queued from `from`
    /// on, while a jump's window is open (see the module docs), drawing on
    /// `chance` for what the moves' dispatch runs. Otherwise the moves
    /// stay queued for the caller's settling.
    pub(super) fn absorb_moves(&mut self, from: usize, chance: &mut dyn Chance) {
        if !self.chart.in_jump_window() || self.run.queued.moves.len() <= from {
            return;
        }
        for step in self.run.queued.moves.split_off(from) {
            self.move_in_jump(step, chance);
        }
    }

    /// An in-flight `M` or `N`, `step`, during a jump's window (see the
    /// module docs).
    fn move_in_jump(&mut self, step: ScriptMove, chance: &mut dyn Chance) {
        let rules = self.setup.script_effect_rules();
        let to = match step {
            ScriptMove::To(system) | ScriptMove::KeepPosition(system)
                if self.chart.galaxy.exists(system) =>
            {
                Some((system, matches!(step, ScriptMove::KeepPosition(_))))
            }
            ScriptMove::To(_) | ScriptMove::KeepPosition(_)
                if rules.missing_system_move != RuleSource::Engine =>
            {
                return;
            }
            ScriptMove::To(_) | ScriptMove::KeepPosition(_) | ScriptMove::Stay => None,
        };
        let mut placed = false;
        if let Some((system, keep)) = to {
            self.pilot.system = system;
            self.chart.deselect();
            let first = self
                .statics
                .first_stellars
                .get(&system)
                .map(|&stellar| (self.statics.stellar_position(stellar), None));
            let placement = move_placement(first, keep, false, rules);
            placed = placement.ship != ShipPlacement::Keep;
            self.place(placement);
        }
        self.chart
            .rebuild_chart(&mut self.pilot, &self.statics, &self.setup);
        if to.is_some() && rules.arrival == RuleSource::Bible {
            let system = self.pilot.system;
            self.chart.explore_place(
                &mut self.pilot,
                &self.setup,
                system,
                ExploredLevel::Explored,
            );
            self.pilot.clear_course();
            self.run.raise(StoryEvent::Explored { system });
        }
        if let Some(window) = &mut self.chart.jump_window {
            window.placed |= placed;
            window.killed = true;
        }
        self.enter_in_jump(chance);
    }

    /// Raises the move's [`StoryEvent::EnteredSystem`] for the player's
    /// system and dispatches it at once, events only and with no catalog,
    /// unless a move's own dispatch is running already, which takes it
    /// (see the module docs).
    fn enter_in_jump(&mut self, chance: &mut dyn Chance) {
        let event = StoryEvent::EnteredSystem {
            system: self.pilot.system,
            how: Entrance::ScriptMove { landed: false },
        };
        let Some(window) = &mut self.chart.jump_window else {
            return;
        };
        if window.dispatching {
            self.run.raise(event);
            return;
        }
        window.dispatching = true;
        let mark = self.run.story_mark();
        self.run.raise(event);
        self.dispatch_since(&NoCatalog, chance, mark, StoryEventKind::EnteredSystem);
        if let Some(window) = &mut self.chart.jump_window {
            window.dispatching = false;
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use super::super::crons::tests::scripted;
    use super::super::kept_rescues::tests::by;
    use crate::catalog::{
        CronRecord, DudeId, DudeRecord, NebulaRecord, ShipId, StellarId, SystemId, SystemTraffic,
    };
    use crate::chance::NeverFires;
    use crate::control::{Script, ScriptNote, SetOp};
    use crate::geometry::Vec2;
    use crate::hyperspace::MultiJumpRule;
    use crate::pilot::{ExploredLevel, Pilot};
    use crate::rulebook::{RuleKey, RuleSource, Rulebook};
    use crate::session::Session;
    use crate::session::control::SetOpObserver;
    use crate::session::tests::chained;
    use crate::stats::INERTIALESS_HULL;
    use crate::story::{Consumer, EventFilter, StoryEvent, Subscription, SubscriptionId};
    use crate::testkit::{
        FAST, FakePilotCatalog, begin_jump_now, catalog, fly_out, hideable, hiding_bit, jump,
        nebula, planet, ship, star,
    };

    /// Where 133's first stellar, 150, is.
    const FIRST_150: Vec2 = Vec2::new(300.0, -200.0);

    /// [`catalog`] (130 to 131 to 132) with 132 and 133 (at (-600, 0),
    /// unlinked): 133's first stellar 150 at [`FIRST_150`], then 151, and
    /// two ships of traffic, düde 128 flying ship 129; with `nebulae` and
    /// `crons`.
    fn moving(nebulae: Vec<NebulaRecord>, crons: Vec<CronRecord>) -> FakePilotCatalog {
        let mut dude_types = [(-1, 0); 8];
        dude_types[0] = (128, 100);
        let mut catalog = FakePilotCatalog {
            nebulae,
            crons,
            traffic: vec![(
                SystemId(133),
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
            ..catalog()
        };
        catalog.systems.extend([SystemId(132), SystemId(133)]);
        catalog.sites.push((
            SystemId(133),
            vec![planet(150, FIRST_150.x, FIRST_150.y), planet(151, 0.0, 0.0)],
        ));
        catalog
    }

    /// The nebula around 131 running `on_explore`.
    fn over_131(on_explore: &str) -> NebulaRecord {
        nebula(128, (500, -100, 200, 200), "", on_explore)
    }

    /// `catalog`'s new pilot flown by `rules`, by `MultiJumpRule::Engine`,
    /// its load settled.
    fn flown(catalog: &FakePilotCatalog, rules: Rulebook) -> Session {
        let mut session = Session::fly(catalog, Pilot::new(catalog, "").expect("starts"))
            .expect("flies")
            .with_rules(rules)
            .with_multi_jump(MultiJumpRule::Engine);
        let _ = session.settle_script(catalog, &mut NeverFires);
        session
    }

    /// The NPCs in the system as each `A` operator applies, with the
    /// mission it names.
    #[derive(Debug, Default)]
    struct Hooks(RefCell<Vec<(i16, usize)>>);

    impl Hooks {
        fn seen(&self) -> Vec<(i16, usize)> {
            self.0.borrow().clone()
        }
    }

    impl SetOpObserver for Hooks {
        fn saw(&self, op: &SetOp, session: &Session) {
            if let SetOp::AbortMission(mission) = op {
                self.0.borrow_mut().push((mission.0, session.npcs().len()));
            }
        }
    }

    /// `session` with a fixture subscription, `record`, to `system`'s
    /// `EnteredSystem` running `A{record}`, a mission never active, which
    /// the [`Hooks`] given back sees.
    fn watching_entries(mut session: Session, record: i16, system: i16) -> (Session, Rc<Hooks>) {
        session
            .register_story(Subscription::new(
                SubscriptionId {
                    consumer: Consumer::Fixture,
                    record,
                    hook: 0,
                },
                EventFilter::EnteredSystem(SystemId(system)),
                Script::parse(&format!("A{record}")),
            ))
            .expect("registers");
        let seen = Rc::new(Hooks::default());
        (session.with_set_op_observer(seen.clone()), seen)
    }

    fn settle(session: &mut Session, catalog: &FakePilotCatalog) -> Option<SystemId> {
        session.settle_script(catalog, &mut NeverFires).moved
    }

    #[test]
    fn a_crons_m_on_a_jumps_day_enters_and_populates_the_system_moved_to() {
        let catalog = moving(Vec::new(), vec![scripted(128, 1, "M133", "")]);
        let (mut session, seen) = watching_entries(flown(&catalog, Rulebook::default()), 601, 133);
        assert_eq!(jump(&mut session, &catalog, 131), Some(SystemId(133)));
        assert_eq!(
            seen.seen(),
            [(601, 0), (601, 0)],
            "the move's hook (@0x159a8), then the jump's (@0x6c15d), both before the setup"
        );
        assert_eq!(session.system(), SystemId(133));
        assert_eq!(session.npcs().len(), 2, "populated once");
        assert_eq!(session.player().position, FIRST_150);
        assert_eq!(session.player().velocity, Vec2::ZERO);
        assert_eq!(settle(&mut session, &catalog), None, "nothing left");
        assert_eq!(seen.seen().len(), 2);
    }

    #[test]
    fn a_crons_m_on_a_jumps_day_keeps_the_arrivals_speed_along_the_heading() {
        // The arrival block sets an inertialess ship's +0x48 to its top
        // speed (@0x6bcf5) and `M` never writes it (C6).
        let mut catalog = moving(Vec::new(), vec![scripted(128, 1, "M133", "")]);
        catalog.ships = vec![(
            ShipId(128),
            Ok(crate::handling::ShipFields {
                flags2: INERTIALESS_HULL,
                ..FAST
            }),
        )];
        let mut session = flown(&catalog, Rulebook::default());
        assert_eq!(jump(&mut session, &catalog, 131), Some(SystemId(133)));
        assert_eq!(session.player().position, FIRST_150);
        assert_eq!(session.player().velocity, Vec2::ZERO);
        assert_eq!(session.player_speed(), session.top_speed());
    }

    #[test]
    fn a_crons_n_on_a_jumps_day_keeps_the_jumps_placement() {
        for source in RuleSource::ALL {
            let rules = Rulebook::default().with_override(RuleKey::MoveKeepFlag, source);
            let plain = moving(Vec::new(), Vec::new());
            let mut session = flown(&plain, rules);
            assert_eq!(jump(&mut session, &plain, 131), Some(SystemId(131)));
            let placed = (*session.player(), session.player_speed());
            assert!(!session.port.hold_position);

            let catalog = moving(Vec::new(), vec![scripted(128, 1, "N133", "")]);
            let mut session = flown(&catalog, rules);
            assert_eq!(jump(&mut session, &catalog, 131), Some(SystemId(133)));
            assert_eq!((*session.player(), session.player_speed()), placed);
            assert_eq!(session.npcs().len(), 2, "populated");
            assert_eq!(
                session.port.hold_position,
                source == RuleSource::Engine,
                "{source:?}"
            );
        }
    }

    #[test]
    fn by_the_at_rest_arrival_a_move_in_the_jump_keeps_its_placement() {
        // The other reading of `jump_arrival` places the ship at the entry,
        // but not over a move's placement.
        let rules = Rulebook::default().with_override(RuleKey::JumpArrival, RuleSource::Bible);
        let catalog = moving(Vec::new(), vec![scripted(128, 1, "M133", "")]);
        let mut session = flown(&catalog, rules);
        assert_eq!(jump(&mut session, &catalog, 131), Some(SystemId(133)));
        assert_eq!(session.player().position, FIRST_150);
        let catalog = moving(Vec::new(), Vec::new());
        let mut session = flown(&catalog, rules);
        assert_eq!(jump(&mut session, &catalog, 131), Some(SystemId(131)));
        assert_eq!(
            session.player().position,
            Vec2::new(-1001.0, 0.0),
            "with no move, at rest outside the zone"
        );
    }

    #[test]
    fn by_the_at_rest_arrival_a_move_that_places_nothing_keeps_the_arrival() {
        // An `N`, or an `M` to a starless system by the engine's
        // `starless`, keeps the ship where it is: by the other reading of
        // `jump_arrival` that is where the entry puts it, at rest outside
        // the zone, not at its departure's coordinates.
        let at_rest = Rulebook::default().with_override(RuleKey::JumpArrival, RuleSource::Bible);
        for (script, system) in [("N133", 133), ("M132", 132)] {
            let catalog = moving(Vec::new(), vec![scripted(128, 1, script, "")]);
            let mut session = flown(&catalog, at_rest);
            assert_eq!(jump(&mut session, &catalog, 131), Some(SystemId(system)));
            assert_eq!(
                session.player().position,
                Vec2::new(-1001.0, 0.0),
                "{script}: the at-rest arrival"
            );
            assert_eq!(session.player().velocity, Vec2::ZERO, "{script}");
        }
        // By the other reading of `starless` the `M` stops the ship at the
        // centre, which the entry keeps.
        let catalog = moving(Vec::new(), vec![scripted(128, 1, "M132", "")]);
        let rules = at_rest.with_override(RuleKey::MoveStarless, RuleSource::Bible);
        let mut session = flown(&catalog, rules);
        assert_eq!(jump(&mut session, &catalog, 131), Some(SystemId(132)));
        assert_eq!(session.player().position, Vec2::ZERO);
    }

    #[test]
    fn by_the_engines_arrival_an_n_in_the_jump_keeps_the_early_placement() {
        // The arrival block places the ship before the window opens
        // (@0x6bcf5), and an `N` keeps it there, as a `M` to a starless
        // system does.
        let plain = moving(Vec::new(), Vec::new());
        let mut session = flown(&plain, Rulebook::default());
        assert_eq!(jump(&mut session, &plain, 131), Some(SystemId(131)));
        let placed = *session.player();
        for script in ["N133", "M132"] {
            let catalog = moving(Vec::new(), vec![scripted(128, 1, script, "")]);
            let mut session = flown(&catalog, Rulebook::default());
            assert!(jump(&mut session, &catalog, 131).is_some());
            assert_eq!(*session.player(), placed, "{script}");
        }
    }

    #[test]
    fn a_hops_on_explore_move_is_written_over_but_keeps_its_placement() {
        // 130 to 132 by 131, whose nebula's `OnExplore` moves to 133: the
        // hop loop then writes 132 (@0x6be0b), but the ship stays where the
        // move put it, and the move's hook ran.
        let catalog = chained(moving(vec![over_131("M133")], Vec::new()), 2);
        let (mut session, seen) = watching_entries(flown(&catalog, Rulebook::default()), 601, 133);
        session.plot_course(SystemId(132)).expect("a route");
        fly_out(&mut session);
        assert_eq!(begin_jump_now(&mut session), Ok(SystemId(131)));
        assert_eq!(
            session.arrive(&catalog, &mut NeverFires),
            Some(SystemId(132))
        );
        assert_eq!(session.player().position, FIRST_150);
        assert_eq!(session.player().velocity, Vec2::ZERO);
        assert_eq!(seen.seen(), [(601, 0)], "the move's hook");
        assert_eq!(settle(&mut session, &catalog), None);
        assert_eq!(session.system(), SystemId(132));
    }

    #[test]
    fn an_arrival_on_explores_move_into_a_system_the_refresh_hides_is_moved_by_the_refresh() {
        // 131's `OnExplore` moves to 133, which the jump's refresh hides;
        // 135, at its place, stands for it (@0x99918).
        let mut catalog = moving(vec![over_131("M133")], Vec::new());
        catalog.star_map.push(star(135, (-600.0, 0.0), &[]));
        hideable(&mut catalog);
        let mut session = flown(&catalog, Rulebook::default());
        session.pilot.bits_mut().set(hiding_bit(133));
        assert_eq!(jump(&mut session, &catalog, 131), Some(SystemId(135)));
        assert_eq!(settle(&mut session, &catalog), None);
    }

    /// [`moving`] with 129 shadowing 133 at its place, so no system owns
    /// 133's stellars, and a crön moving to 133 on the jump's day, then
    /// aborting 601, a mission never active.
    fn shadowed_133() -> FakePilotCatalog {
        let mut catalog = moving(Vec::new(), vec![scripted(128, 1, "M133 A601", "")]);
        catalog.star_map.push(star(129, (-600.0, 0.0), &[]));
        catalog
    }

    /// Who holds stellar 150 as each `A` operator applies.
    #[derive(Debug, Default)]
    struct Holders(RefCell<Vec<(Option<SystemId>, Option<SystemId>)>>);

    impl SetOpObserver for Holders {
        fn saw(&self, op: &SetOp, session: &Session) {
            if matches!(op, SetOp::AbortMission(_)) {
                let galaxy = session.galaxy();
                self.0.borrow_mut().push((
                    galaxy.holder(StellarId(150), &[SystemId(133)], session.system()),
                    galaxy.rescued(StellarId(150)),
                ));
            }
        }
    }

    #[test]
    fn a_move_on_a_jumps_day_rescues_for_the_system_moved_to() {
        let catalog = shadowed_133();
        for source in RuleSource::ALL {
            let rules = Rulebook::default().with_override(RuleKey::RescueUntilRefresh, source);
            let seen = Rc::new(Holders::default());
            let mut session = flown(&catalog, rules).with_set_op_observer(seen.clone());
            assert_eq!(jump(&mut session, &catalog, 131), Some(SystemId(133)));
            // By the other reading the holder derives the player's system,
            // 133 once the move has written it.
            assert_eq!(
                *seen.0.borrow(),
                [(Some(SystemId(133)), by(source, Some(SystemId(133)), None))],
                "{source:?}: before the entry"
            );
        }
    }

    #[test]
    fn a_move_to_no_system_or_an_n_out_of_range_on_a_jumps_day_stays() {
        for (script, key) in [
            ("M999", RuleKey::MissingSystemMove),
            ("N3000", RuleKey::OutOfRangeMove),
        ] {
            for source in RuleSource::ALL {
                let catalog = moving(Vec::new(), vec![scripted(128, 1, script, "")]);
                let rules = Rulebook::default().with_override(key, source);
                let (mut session, seen) = watching_entries(flown(&catalog, rules), 601, 131);
                assert_eq!(jump(&mut session, &catalog, 131), Some(SystemId(131)));
                let hooks = seen.seen().len();
                assert_eq!(
                    hooks,
                    by(source, 2, 1),
                    "{script} {source:?}: the tail's hook, then the jump's"
                );
                assert_eq!(
                    session.chart.jump_window, None,
                    "{script} {source:?}: closed"
                );
            }
        }
    }

    #[test]
    fn by_the_bible_a_move_on_a_jumps_day_is_an_arrival_too() {
        // Two hops a jump along 131, 132, 134: by the engine the tail
        // spools the arrival's hop (@0x6c4c8); by the Bible the move
        // clears the course, and the tail drops nothing more.
        let catalog = chained(moving(Vec::new(), vec![scripted(128, 1, "M133", "")]), 2);
        for source in RuleSource::ALL {
            let rules = Rulebook::default().with_override(RuleKey::MoveArrival, source);
            let mut session = flown(&catalog, rules);
            session.plot_course(SystemId(134)).expect("a route");
            fly_out(&mut session);
            begin_jump_now(&mut session).expect("jumps");
            assert_eq!(
                session.arrive(&catalog, &mut NeverFires),
                Some(SystemId(133))
            );
            assert_eq!(
                session.course(),
                by(source, vec![SystemId(134)], Vec::new()),
                "{source:?}"
            );
            let explored = StoryEvent::Explored {
                system: SystemId(133),
            };
            assert_eq!(
                session.story_raised().contains(&explored),
                source == RuleSource::Bible,
                "{source:?}: raised for the settling"
            );
            assert_eq!(
                session.pilot().explored_level(SystemId(133)),
                ExploredLevel::Explored,
                "{source:?}: the jump's entry explores it anyway"
            );
        }
    }

    #[test]
    fn a_jumps_course_after_a_days_move_is_the_originals() {
        // One jump along 131, 132: the tail clears a course whose next hop
        // stands elsewhere than the system the move left (@0x6c472-0x6c4c6);
        // the other reading drops the hop made.
        let catalog = moving(Vec::new(), vec![scripted(128, 1, "M133", "")]);
        for source in RuleSource::ALL {
            let rules = Rulebook::default().with_override(RuleKey::RouteRewrite, source);
            let mut session = flown(&catalog, rules);
            session.plot_course(SystemId(132)).expect("a route");
            fly_out(&mut session);
            begin_jump_now(&mut session).expect("jumps");
            session.arrive(&catalog, &mut NeverFires).expect("arrives");
            assert_eq!(
                session.course(),
                by(source, Vec::new(), vec![SystemId(132)]),
                "{source:?}"
            );
        }
        // Two hops a jump along 131, 132, 134: the multi-jump branch spools
        // the arrival's hop whatever the move (@0x6c4c8).
        let catalog = chained(moving(Vec::new(), vec![scripted(128, 1, "M133", "")]), 2);
        for source in RuleSource::ALL {
            let rules = Rulebook::default().with_override(RuleKey::RouteRewrite, source);
            let mut session = flown(&catalog, rules);
            session.plot_course(SystemId(134)).expect("a route");
            fly_out(&mut session);
            begin_jump_now(&mut session).expect("jumps");
            assert_eq!(
                session.arrive(&catalog, &mut NeverFires),
                Some(SystemId(133))
            );
            assert_eq!(session.course(), [SystemId(134)], "{source:?}");
        }
    }

    #[test]
    fn an_entered_system_script_that_moves_again_during_a_jump_ends() {
        let catalog = moving(Vec::new(), vec![scripted(128, 1, "M133", "")]);
        let mut session = flown(&catalog, Rulebook::default());
        session
            .register_story(Subscription::new(
                SubscriptionId {
                    consumer: Consumer::Fixture,
                    record: 1,
                    hook: 0,
                },
                EventFilter::EnteredSystem(SystemId(133)),
                Script::parse("M133"),
            ))
            .expect("registers");
        assert_eq!(jump(&mut session, &catalog, 131), Some(SystemId(133)));
        assert!(
            session
                .take_script_notes()
                .iter()
                .any(|note| matches!(note, ScriptNote::EventsDropped { .. })),
            "the budget's drop"
        );
    }

    #[test]
    fn a_move_out_of_a_jump_still_waits_for_the_settling() {
        let catalog = moving(Vec::new(), Vec::new());
        let mut session = flown(&catalog, Rulebook::default());
        session.run_set(
            &crate::control::SetExpr::parse("M133").expect("parses"),
            &mut NeverFires,
        );
        assert_eq!(session.system(), SystemId(130), "queued");
        assert_eq!(settle(&mut session, &catalog), Some(SystemId(133)));
    }
}
