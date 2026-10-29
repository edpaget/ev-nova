//! The exploration walk on the session (crons-news-ranks-fidelity
//! phase 45): `_AutoSetExploration` and `_RecursiveAutoExplore`, as the
//! [`map_walk`](crate::map_walk) module ports them, over the session's
//! galaxy and pilot. All addresses are in `EV Nova.app`'s `EV Nova`
//! (i386).
//!
//! **The walks.** [`Session::auto_explore`] runs one, by the engine
//! reading of [`RuleKey::MapExplore`](crate::RuleKey::MapExplore) depth
//! first, by the other breadth first. Its callers:
//!
//! - a map grant ([`Session::grant_outfit`]): a walk for each origin
//!   [`map_walks`](crate::exploration::map_walks) gives, at level 2
//!   (@0x44dcb, @0x44e12, @0x44e86);
//! - the take-off, reaching 0 at level 2 (`_PlayerLandOnStellar`
//!   @0x634b1-0x634c8);
//! - a jump's re-explore of the arrival, reaching 0 at level 1
//!   (`_HandlePlayer` @0x6c1cb-0x6c1db), with the jump's catalog.
//!
//! The escape pod's (@0x690cb) is left to roadmap player-death.
//!
//! **Each system** reached raises its [`StoryEvent::Explored`] and
//! dispatches it at once ([`Session::explore_now`]), so its nebulae's
//! `OnExplore`s run before its level is raised and before its neighbours
//! are walked; then it and, by the engine reading of
//! [`RuleKey::ExploredByPlace`](crate::RuleKey::ExploredByPlace), its
//! place's head are raised ([`Chart::explore_place`]); then its links
//! are read from the overlaid map, under the session's hyperlinks. The
//! walk ends in a known pass with no clear ([`Chart::add_known`]).
//!
//! **The shared state.** The visited set and the reach are one
//! [`MapWalk`] on the [`Chart`], as the original's globals (0xde93c,
//! 0xde944) are one: a map granted by an `OnExplore` inside a walk runs
//! its own walk over the same state, so the outer walk goes on with the
//! inner one's reach and visits (see the [`map_walk`](crate::map_walk)
//! module). It is not saved, as no save routine touches either global.
//!
//! **Ordering.** The take-off's `OnExplore`s run at once, inside the
//! take-off, before its aux restart (@0x634df) and its setup (@0x674c2),
//! through an events-only settling with no catalog
//! ([`NoCatalog`](super::story::NoCatalog)). The moves they make are the
//! take-off's landed moves, applied at their operator (see the
//! `take_off_moves` module); a map grant's are held for the caller's
//! settling.
//!
//! [`StoryEvent::Explored`]: crate::story::StoryEvent::Explored

use std::collections::BTreeSet;

use super::story::SettlingCatalog;
use super::{Chart, Session};
use crate::catalog::SystemId;
use crate::chance::Chance;
use crate::map_walk::{WalkSite, auto_set_exploration, breadth_first};
use crate::pilot::ExploredLevel;
use crate::rulebook::RuleSource;

/// The walk's shared state, the original's visited array (0xde93c) and
/// reach (0xde944): what the last walk begun left.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct MapWalk {
    /// The systems visited since the last walk began.
    visited: BTreeSet<SystemId>,
    /// The last walk's reach.
    reach: u16,
}

impl Chart {
    /// Begins a walk reaching `reach`: the visited set cleared
    /// (`_AutoSetExploration` @0xdada) and the reach written (@0xdae9).
    fn begin_walk(&mut self, reach: u16) {
        self.walk.visited.clear();
        self.walk.reach = reach;
    }

    /// The reach the last walk begun wrote.
    fn walk_reach(&self) -> u16 {
        self.walk.reach
    }

    /// Marks `system` visited (@0xd983): whether it was not already.
    fn visit(&mut self, system: SystemId) -> bool {
        self.walk.visited.insert(system)
    }
}

/// The session as a walk's [`WalkSite`], dispatching its explorations
/// with `catalog` and drawing on `chance`.
struct SessionWalk<'a, C: ?Sized> {
    session: &'a mut Session,
    catalog: &'a C,
    chance: &'a mut dyn Chance,
}

impl<C: SettlingCatalog + ?Sized> WalkSite for SessionWalk<'_, C> {
    fn begin(&mut self, reach: u16) {
        self.session.chart.begin_walk(reach);
    }

    fn reach(&self) -> u16 {
        self.session.chart.walk_reach()
    }

    fn visit(&mut self, system: SystemId) -> bool {
        self.session.chart.visit(system)
    }

    fn explore(&mut self, system: SystemId) {
        self.session
            .explore_now(self.catalog, &mut *self.chance, system);
    }

    fn raise(&mut self, system: SystemId, level: ExploredLevel) {
        let session = &mut *self.session;
        session
            .chart
            .explore_place(&mut session.pilot, &session.setup, system, level);
    }

    fn links(&self, system: SystemId) -> Vec<SystemId> {
        self.session
            .chart
            .galaxy
            .map()
            .ordered_jumps(system, self.session.setup.hyperlinks)
    }

    fn known_pass(&mut self) {
        self.session.chart.add_known(&self.session.pilot);
    }
}

impl Session {
    /// Runs the walk from `from` reaching `reach` jumps, raising each
    /// system to `level` (see the module docs), dispatching its
    /// explorations with `catalog` and drawing on `chance`. It coordinates
    /// the [`Chart`], the pilot and the story's dispatch, so it lives on
    /// the session.
    pub(super) fn auto_explore<C>(
        &mut self,
        catalog: &C,
        chance: &mut dyn Chance,
        from: SystemId,
        reach: u16,
        level: ExploredLevel,
    ) where
        C: SettlingCatalog + ?Sized,
    {
        let source = self.setup.outfit_rules().map_explore;
        let mut site = SessionWalk {
            session: self,
            catalog,
            chance,
        };
        match source {
            RuleSource::Engine => auto_set_exploration(&mut site, from, reach, level),
            RuleSource::Bible => breadth_first(&mut site, from, reach, level),
        }
    }
}

#[cfg(test)]
pub(super) mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use super::super::jump_exploration::tests::watching_explored;
    use crate::catalog::{NebulaRecord, StarSystem, SystemId};
    use crate::chance::NeverFires;
    use crate::control::{Bit, SetExpr, SetOp};
    use crate::outfit_effects::{INHABITED_INDEPENDENT, MAP};
    use crate::pilot::Pilot;
    use crate::session::Session;
    use crate::session::control::SetOpObserver;
    use crate::story::StoryEvent;
    use crate::testkit::{
        FakePilotCatalog, begin_jump_now, catalog, fly_out, land_now, nebula, outfit, star,
    };

    /// A nebula, `id`, around the system at `at` alone, active by
    /// `active_on`, running `on_explore`.
    pub(in crate::session) fn around(
        id: i16,
        at: (i16, i16),
        active_on: &str,
        on_explore: &str,
    ) -> NebulaRecord {
        nebula(
            id,
            (at.0 - 100, at.1 - 100, 200, 200),
            active_on,
            on_explore,
        )
    }

    /// [`catalog`] on `stars`, each of them a system, with `nebulae`, and
    /// outfit 300 a map of `outer`, 301 one of `inner`.
    pub(in crate::session) fn galaxy(
        stars: Vec<StarSystem>,
        nebulae: Vec<NebulaRecord>,
        outer: i16,
        inner: i16,
    ) -> FakePilotCatalog {
        FakePilotCatalog {
            systems: stars.iter().map(|system| system.id).collect(),
            star_map: stars,
            nebulae,
            outfits: vec![outfit(300, &[(MAP, outer)]), outfit(301, &[(MAP, inner)])],
            ..catalog()
        }
    }

    /// 130 - 131 - 132 - 133 in a line, 600 apart, each linking the
    /// next and back.
    pub(in crate::session) fn line() -> Vec<StarSystem> {
        vec![
            star(130, (0.0, 0.0), &[131]),
            star(131, (600.0, 0.0), &[130, 132]),
            star(132, (1200.0, 0.0), &[131, 133]),
            star(133, (1800.0, 0.0), &[132]),
        ]
    }

    /// `catalog`'s new pilot flown from 130, with `bits` set, its load
    /// settled.
    fn flown(catalog: &FakePilotCatalog, bits: &[u16]) -> Session {
        let mut pilot = Pilot::new(catalog, "").expect("starts");
        for &n in bits {
            pilot.bits.set(Bit::new(n).expect("a bit"));
        }
        let mut session = Session::fly(catalog, pilot).expect("flies");
        let _ = session.settle_script(catalog, &mut NeverFires);
        session
    }

    fn run(session: &mut Session, script: &str) {
        session.run_set(&SetExpr::parse(script).expect("parses"), &mut NeverFires);
    }

    pub(in crate::session) fn charted(session: &Session) -> Vec<i16> {
        session.pilot().charted().map(|system| system.0).collect()
    }

    fn order(seen: &[SystemId]) -> Vec<i16> {
        seen.iter().map(|system| system.0).collect()
    }

    #[test]
    fn a_map_granted_inside_the_walk_sets_the_reach_the_outer_walk_goes_on_with() {
        // A 3-jump map from 130; 131's OnExplore grants a 1-jump map. Its
        // walk clears the visited set and walks 130 and 131 again, and
        // the outer 131 frame then tests 132 against its reach of 1
        // (0xde944 read @0xd95a-0xd967): 132 and 133 are never reached.
        let catalog = galaxy(line(), vec![around(128, (600, 0), "", "G301")], 3, 1);
        let (mut session, seen) = watching_explored(flown(&catalog, &[]), &[130, 131, 132, 133]);
        run(&mut session, "G300");
        // Each watcher runs after the nebula's OnExplore on the same
        // event: the outer 131's is seen after the inner walk's.
        assert_eq!(order(&seen.systems()), [130, 130, 131, 131]);
        assert_eq!(charted(&session), [130, 131]);
        assert!(!session.pilot().has_explored(SystemId(132)));
    }

    /// 130 - 131 and 140 - 141, each pair linking both ways, 140 an
    /// inhabited independent system and none of the others.
    fn pairs() -> Vec<StarSystem> {
        vec![
            star(130, (0.0, 0.0), &[131]),
            star(131, (600.0, 0.0), &[130]),
            StarSystem {
                stellars: vec![Some((0x0001, 0))],
                ..star(140, (3000.0, 0.0), &[141])
            },
            star(141, (3600.0, 0.0), &[140]),
        ]
    }

    #[test]
    fn a_map_granted_inside_an_independents_walk_extends_it_and_wipes_its_visits() {
        // The independents' map walks 140 at reach 0; 140's OnExplore
        // grants a 2-jump map from the player at 130. The 140 frame then
        // walks 141 under the inner reach of 2, and 140 again, its visit
        // wiped by the inner clear (@0xdada).
        let catalog = galaxy(
            pairs(),
            vec![around(128, (3000, 0), "", "G301")],
            INHABITED_INDEPENDENT,
            2,
        );
        let (mut session, seen) = watching_explored(flown(&catalog, &[]), &[130, 131, 140, 141]);
        run(&mut session, "G300");
        // The outer 140's watcher runs after its OnExplore's whole walk.
        assert_eq!(order(&seen.systems()), [130, 131, 140, 141, 140]);
        assert_eq!(charted(&session), [130, 131, 140, 141]);
    }

    /// Whether 132 was explored and 131 charted as each set operator
    /// applied.
    #[derive(Debug, Default)]
    struct Levels(RefCell<Vec<(bool, bool)>>);

    impl SetOpObserver for Levels {
        fn saw(&self, _op: &SetOp, session: &Session) {
            let pilot = session.pilot();
            self.0.borrow_mut().push((
                pilot.has_explored(SystemId(132)),
                pilot.has_charted(SystemId(131)),
            ));
        }
    }

    #[test]
    fn a_systems_on_explore_runs_before_its_level_and_before_its_neighbours() {
        // A 2-jump map from 130: 131's OnExplore runs before 131 is
        // charted (@0xd9a6-0xd9af) and before 132 is walked (@0xdaa3).
        let catalog = galaxy(line(), vec![around(128, (600, 0), "", "A999")], 2, 1);
        let levels = Rc::new(Levels::default());
        let mut session = flown(&catalog, &[]).with_set_op_observer(levels.clone());
        run(&mut session, "G300");
        assert_eq!(
            *levels.0.borrow(),
            [(false, false), (false, false)],
            "G300, then the OnExplore's A999"
        );
        assert_eq!(charted(&session), [130, 131, 132]);
    }

    #[test]
    fn a_map_granted_in_a_jumps_re_explore_walks_the_arrivals_neighbours() {
        // 130 lists 131, 131 lists 132; 133 links to none. The nebula
        // around 131 is inactive until the jump's refresh (b5 set after
        // the load's), so its OnExplore runs in the re-explore's walk
        // (@0x6c1db), not at the arrival's first explore (@0x6be12). It
        // moves the player to 133 and grants a 1-jump map there; the
        // re-explore's 131 frame then walks 132 under that reach, at its
        // own level 1: 132 explored, not charted, and 133 charted.
        let mut stars = catalog().star_map;
        stars.truncate(4);
        let catalog = galaxy(stars, vec![around(128, (600, 0), "b5", "N133 G300")], 1, 1);
        let mut session = flown(&catalog, &[]);
        session.plot_course(SystemId(131)).expect("a route");
        fly_out(&mut session, &catalog);
        begin_jump_now(&mut session, &catalog).expect("jumps");
        run(&mut session, "b5");
        let _ = session.settle_script(&catalog, &mut NeverFires);
        let (mut session, seen) = watching_explored(session, &[131, 132, 133]);
        session.arrive(&catalog, &mut NeverFires).expect("arrives");
        // The arrival's first explore, then the inner walk's 133, inside
        // the re-explore's 131, then 132.
        assert_eq!(order(&seen.systems()), [131, 133, 131, 132]);
        assert!(session.pilot().has_explored(SystemId(132)));
        assert!(!session.pilot().has_charted(SystemId(132)));
        assert!(session.pilot().has_charted(SystemId(133)));
    }

    #[test]
    fn a_take_offs_on_explore_runs_inside_the_take_off() {
        // `_PlayerLandOnStellar`'s walk (@0x634b1-0x634c8) runs the
        // OnExplore at once, before the caller's settling.
        let catalog = galaxy(line(), vec![around(128, (0, 0), "", "b9")], 1, 1);
        let mut session = flown(&catalog, &[]);
        land_now(&mut session).expect("lands");
        let _ = session.settle_script(&catalog, &mut NeverFires);
        session
            .take_off(&catalog, &mut NeverFires)
            .expect("takes off");
        assert!(session.pilot().control_bit(Bit::new(9).expect("a bit")));
        assert!(
            !session
                .story_raised()
                .iter()
                .any(|event| matches!(event, StoryEvent::Explored { .. })),
            "none left for the settling"
        );
    }
}
