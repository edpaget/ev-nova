//! An `M` or `N` during a take-off: `_EvalSetExp`'s landed tail, at its
//! operator (crons-news-ranks-fidelity phase 46). All addresses are in
//! `EV Nova.app`'s `EV Nova` (i386).
//!
//! **The original.** `_PlayerLandOnStellar` sets `_playerIsLanded` at its
//! entry (@0x631e3-0x631ee) and clears it only at @0x6358e-0x63596.
//! `_EvalSetExp` chooses its tail by that flag (pointer 0x3b5060 ->
//! 0x23052c), so every move run in between is a landed one. In between
//! run, in this order:
//!
//! 1. the stay's days, inside `_DoPortDialog` (@0x63381;
//!    `_IncrementGameTime` @0x601fa, @0x6020e);
//! 2. the take-off's placement (@0x633a5-0x63417);
//! 3. the take-off's own day (@0x63440);
//! 4. the explored-level raise of +0x74 and its first co-located system
//!    (@0x63450-0x6349e);
//! 5. the walk, `_AutoSetExploration(+0x74)` (read @0x634c1, called
//!    @0x634c8);
//! 6. a second nebula pass, `_ExploreNebulaeFromSystem(+0x74)`
//!    (@0x634cd-0x634da).
//!
//! The landed tail writes +0x74 at the operator (`M` @0x1572b, `N`
//! @0x158f2) and sets +0x6c to -1 (@0x15733, @0x158fd); `M` makes the
//! first `NavDefs` slot that isn't -1 the +0x6c (@0x15756-0x157f5,
//! @0x156f2-0x15705) and writes no position (@0x1576c); `N` sets
//! `_dontMovePlayerAfterLanding` (pointer 0x3b5208 -> 0x316951,
//! @0x15903). Both carry the escorts' +0x74 (@0x15807-0x1583e,
//! @0x1590b-0x1593d) and fill `_dockedPortMissions` with -1 (pointer
//! 0x3b5204 -> 0x186c20, @0x1584b-0x15869, @0x15949-0x15967). Neither
//! sets a kill flag, runs `_HandleStellarSystemVisibility` or runs
//! `_MissionHandlePlayerEnteredNewSystem`. An out-of-range `M` (@0x1571d)
//! does nothing; an out-of-range `N` (@0x158e8 -> @0x1593f) only empties
//! the offers.
//!
//! The placement (@0x633a5-0x63417) zeroes the velocity, then keeps the
//! position and clears the flag when `_dontMovePlayerAfterLanding` is set
//! (@0x633ad-0x633b8, @0x63400), or else puts the ship on +0x6c's
//! `xPos`/`yPos` (@0x633bd-0x633eb), at (0,0) with none (@0x633f2-0x633fb).
//! So a stay day's `M` takes off at the target's first stellar and its
//! `N` where the ship touched down; a move after the placement leaves the
//! ship where the placement put it, in the system moved to, and its `N`'s
//! flag is cleared at the next landing's entry (@0x631f1-0x631f6) before
//! anything reads it.
//!
//! The raise and the walk read +0x74 after the take-off's day, so a move
//! on a stay day or on the take-off's day makes the target the system
//! raised to level 2 and walked, and @0x634da then has nothing new to
//! explore. Only a move from the walk's own `OnExplore` leaves the target
//! unraised: @0x634da then explores its nebulae, still landed, and raises
//! no level. The rest of the take-off reads +0x74 as the moves left it:
//! the save keeps +0x6c (@0x6354b), and `_HandlePlayerDockRequest`'s tail
//! paints +0x74's background (@0x670a6-0x670dd), skips the enter-system
//! hook (@0x66f2e, @0x67122-0x67153 -> @0x674ad), sets +0x74's ships up
//! once (@0x674c2), runs HSSV (@0x674c7) and lets go of the selection
//! (@0x674d2).
//!
//! **Here.** [`Session::lift_off`] opens a window ([`TakeOffWindow`], on
//! the [`Chart`]) as it takes the ship out of the spaceport and closes it
//! after the @0x634da pass. While it is open, `Session::apply_op` hands
//! each move a set operator queues to [`Session::move_at_take_off`], at
//! the operator, with no catalog:
//!
//! - A move to a system the galaxy holds writes the player's system and
//!   leaves the owner words unset for the take-off's rebuild (see the
//!   `owners_unset` module). Before the placement it places the ship as a
//!   landed move does: `M` on the system's first stellar, read as the
//!   session starts, or at rest at the centre with none; `N` holds the
//!   take-off's landing position, under either reading of
//!   [`ScriptEffectRules::keep_flag`]. After the placement it places
//!   nothing, and an `N` holds the next take-off's landing position only
//!   by [`ScriptEffectRules::keep_flag`]'s engine reading, as an `N` in
//!   flight does (phase 48 changes that key's default).
//! - Every move, and a move to a system with no record or an `N` out of
//!   range by the engine readings of
//!   [`ScriptEffectRules::missing_system_move`] and
//!   [`ScriptEffectRules::out_of_range_move`], empties the offers; by the
//!   other readings those two do nothing.
//! - No [`StoryEvent::EnteredSystem`] is raised, the chart is not rebuilt
//!   and nothing is killed.
//! - Under [`ScriptEffectRules::arrival`]'s other reading the move is an
//!   arrival too, as a landed move settled is: its system explored at
//!   level 1, [`StoryEvent::Explored`] raised, and the course cleared.
//!
//! The take-off then raises and walks the player's system as the moves
//! left it, explores the nebulae of the system a walk's move took it to
//! (@0x634da), and closes the window: after a move it docks the pilot's
//! stellar as a landed move does (the target's first stellar by `M`, when
//! it can be docked at, none by `N`) and shows the target's stellars,
//! before it restocks, restarts the auxiliary ships and populates, so
//! those act on the target once.
//!
//! **Ordering.** The pilot's stellar and the stellars shown are written
//! at the window's close rather than at the operator: nothing between the
//! two is drawn or saved.
//!
//! **Left out.** The escorts' +0x74 carry: the take-off's setup respawns
//! them (`_SetupShipsInSystem` @0x674c2, `_RespawnEscort` writing their
//! +0x74 from the player's, @0x3d565-0x3d571), and the take-off's
//! population brings in the fleet. The landed `M`'s +0x6c write
//! (@0x15705) and `N`'s -1 (@0x158fd) as the nav selection: the take-off
//! lets go of the selection (@0x674d2) before any frame.
//!
//! [`ScriptEffectRules::keep_flag`]: super::ScriptEffectRules::keep_flag
//! [`ScriptEffectRules::missing_system_move`]: super::ScriptEffectRules::missing_system_move
//! [`ScriptEffectRules::out_of_range_move`]: super::ScriptEffectRules::out_of_range_move
//! [`ScriptEffectRules::arrival`]: super::ScriptEffectRules::arrival
//! [`StoryEvent::EnteredSystem`]: crate::story::StoryEvent::EnteredSystem

use super::entry::Dock;
use super::script_effects::{ScriptMove, move_placement};
use super::{Chart, Session, SessionEvent};
use crate::catalog::{PilotCatalog, SystemId};
use crate::landing::is_dockable_destroyed_or_not;
use crate::pilot::ExploredLevel;
use crate::rulebook::RuleSource;
use crate::story::StoryEvent;

/// The last move a take-off's window absorbed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct TakeOffMove {
    /// The system moved to.
    pub(super) system: SystemId,
    /// Whether it was an `N`, keeping the position.
    pub(super) keep: bool,
}

/// A take-off's window, from the ship leaving the spaceport to the
/// @0x634da pass (see the module docs).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct TakeOffWindow {
    /// Whether the take-off's placement has run (@0x633a5-0x63417).
    pub(super) placed: bool,
    /// The last move absorbed, if any.
    pub(super) moved: Option<TakeOffMove>,
}

impl Chart {
    /// Opens a take-off's window (see the module docs).
    pub(super) fn open_take_off_window(&mut self) {
        self.take_off_window = Some(TakeOffWindow::default());
    }

    /// Notes that the take-off's placement has run.
    pub(super) fn mark_take_off_placed(&mut self) {
        if let Some(window) = &mut self.take_off_window {
            window.placed = true;
        }
    }

    /// Closes the take-off's window, giving what it noted.
    pub(super) fn close_take_off_window(&mut self) -> TakeOffWindow {
        self.take_off_window.take().unwrap_or_default()
    }

    /// Whether a take-off's window is open.
    pub(super) fn in_take_off_window(&self) -> bool {
        self.take_off_window.is_some()
    }
}

impl Session {
    /// A move, `step`, during a take-off's window: `_EvalSetExp`'s landed
    /// tail, at its operator, with no catalog (see the module docs).
    pub(super) fn move_at_take_off(&mut self, step: ScriptMove) {
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
        // `_dockedPortMissions` filled with -1 (@0x1584b, @0x15949).
        self.run.empty_offers();
        let Some((system, keep)) = to else {
            return;
        };
        self.pilot.system = system;
        // The owners wait for the take-off's rebuild (@0x674c7).
        self.chart.leave_owners_unset(&self.setup);
        let placed = self
            .chart
            .take_off_window
            .is_some_and(|window| window.placed);
        if placed {
            // The flag an `N` sets now is cleared at the next landing
            // (@0x631f1-0x631f6) before it is read; the engine's
            // `keep_flag` keeps it to the next take-off, as in flight.
            if keep && rules.keep_flag == RuleSource::Engine {
                self.port.hold_position = true;
            }
        } else {
            let first = self
                .statics
                .first_stellars
                .get(&system)
                .map(|&stellar| (self.statics.stellar_position(stellar), None));
            self.place(move_placement(first, keep, true, rules));
        }
        if rules.arrival == RuleSource::Bible {
            self.chart.explore_place(
                &mut self.pilot,
                &self.setup,
                system,
                ExploredLevel::Explored,
            );
            self.pilot.clear_course();
            self.run.raise(StoryEvent::Explored { system });
        }
        if let Some(window) = &mut self.chart.take_off_window {
            window.moved = Some(TakeOffMove { system, keep });
        }
    }

    /// Closes the take-off's window (see the module docs): after a move,
    /// the pilot's stellar docked as a landed move docks it, and the
    /// stellars of the system moved to shown, read from `catalog`, telling
    /// [`SessionEvent::SystemChanged`] when they are another system's.
    pub(super) fn close_take_off_window(&mut self, catalog: &(impl PilotCatalog + ?Sized)) {
        let Some(moved) = self.chart.close_take_off_window().moved else {
            return;
        };
        let sites = self.chart.read_sites(catalog, moved.system);
        let first = sites.first().map(|site| {
            let dock = is_dockable_destroyed_or_not(site).then_some(site.id);
            (site.position, dock)
        });
        let rules = self.setup.script_effect_rules();
        self.pilot.stellar = move_placement(first, moved.keep, true, rules).stellar;
        if self.set_player_system(moved.system, sites, Dock::AsItWas) {
            self.out.emit(SessionEvent::SystemChanged);
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::super::crons::tests::scripted;
    use super::super::jump_exploration::tests::watching_explored;
    use super::super::jump_moves::tests::{FIRST_150, flown, moving, watching_entries};
    use super::super::map_walk::tests::{around, charted, galaxy, line};
    use crate::catalog::{MissionId, MissionRecord, StellarId, SystemId};
    use crate::chance::NeverFires;
    use crate::control::{Bit, SetExpr};
    use crate::geometry::Vec2;
    use crate::mission::Venue;
    use crate::pilot::ExploredLevel;
    use crate::rulebook::{RuleKey, RuleSource, Rulebook};
    use crate::session::{Session, SessionEvent};
    use crate::testkit::{FakePilotCatalog, land_now, mission};

    /// `session` landed and settled on `catalog`, then b5 set, which
    /// makes [`over_130`]'s nebula active only once the new pilot's first
    /// exploration has passed.
    fn landed(mut session: Session, catalog: &FakePilotCatalog) -> Session {
        land_now(&mut session).expect("lands");
        session.run_set(&SetExpr::parse("b5").expect("parses"), &mut NeverFires);
        let _ = session.settle_script(catalog, &mut NeverFires);
        session.take_events();
        session
    }

    /// `session` landed, an outfit traded so a stay day passes before the
    /// take-off's, then left.
    fn left_after_a_stay_day(mut session: Session, catalog: &FakePilotCatalog) -> Session {
        session = landed(session, catalog);
        session.port.stay.outfit_traded = true;
        let _ = session
            .leave_port(catalog, &mut NeverFires)
            .expect("leaves");
        session
    }

    /// `session` landed and left.
    fn left(session: Session, catalog: &FakePilotCatalog) -> Session {
        let mut session = landed(session, catalog);
        let _ = session
            .leave_port(catalog, &mut NeverFires)
            .expect("leaves");
        session
    }

    /// Where a take-off from [`moving`] with no move puts the ship: on the
    /// stellar landed on.
    fn placed_with_no_move(rules: Rulebook) -> Vec2 {
        let plain = moving(Vec::new(), Vec::new());
        left(flown(&plain, rules), &plain).player().position
    }

    /// The nebula around 130 running `on_explore`, active once b5 is set.
    fn over_130(on_explore: &str) -> crate::catalog::NebulaRecord {
        around(128, (0, 0), "b5", on_explore)
    }

    #[test]
    fn a_take_offs_on_explore_n_then_map_charts_around_the_system_moved_to() {
        // 130 - 131 - 132 - 133. The take-off's walk from 130 runs the
        // OnExplore: the `N` writes 133 at its operator, and the 1-jump
        // map walks from 133 (`_GrantOutfitItem` @0x44da6 reads +0x74),
        // charting 132. The outer 130 frame then walks 131 under the
        // shared reach of 1 (phase 45's N2).
        let catalog = galaxy(line(), vec![over_130("N133 G301")], 3, 1);
        let session = left(flown(&catalog, Rulebook::default()), &catalog);
        assert_eq!(session.system(), SystemId(133));
        assert_eq!(charted(&session), [130, 131, 132, 133]);
    }

    #[test]
    fn a_take_offs_on_explore_m_takes_off_in_the_system_moved_to_as_a_landed_move() {
        let placed = placed_with_no_move(Rulebook::default());
        let catalog = moving(vec![over_130("M133")], Vec::new());
        let (session, seen) = watching_entries(flown(&catalog, Rulebook::default()), 601, 133);
        let mut session = landed(session, &catalog);
        let _ = session
            .leave_port(&catalog, &mut NeverFires)
            .expect("leaves");
        assert_eq!(session.system(), SystemId(133));
        assert_eq!(seen.seen(), [], "no enter-system hook");
        assert_eq!(
            session.player().position,
            placed,
            "the take-off's placement"
        );
        assert_eq!(session.player().velocity, Vec2::ZERO);
        assert_eq!(
            session.stellars_shown().collect::<Vec<_>>(),
            [StellarId(150), StellarId(151)]
        );
        assert!(session.take_events().contains(&SessionEvent::SystemChanged));
        assert_eq!(session.npcs().len(), 2, "133's traffic");
        assert_eq!(session.pilot().stellar, Some(StellarId(150)));
        assert_eq!(session.settle_script(&catalog, &mut NeverFires).moved, None);
    }

    #[test]
    fn a_stay_days_m_takes_off_at_the_targets_first_stellar() {
        let catalog = moving(Vec::new(), vec![scripted(128, 1, "M133", "")]);
        let session = left_after_a_stay_day(flown(&catalog, Rulebook::default()), &catalog);
        assert_eq!(session.system(), SystemId(133));
        assert_eq!(session.player().position, FIRST_150);
        assert_eq!(session.player().velocity, Vec2::ZERO);
        assert_eq!(
            session.pilot().explored_level(SystemId(133)),
            ExploredLevel::Charted,
            "raised and walked (@0x6346d-0x6349e, @0x634c8)"
        );
        // 132 has no stellar: at rest at the centre (@0x633f2-0x633fb).
        let catalog = moving(Vec::new(), vec![scripted(128, 1, "M132", "")]);
        let session = left_after_a_stay_day(flown(&catalog, Rulebook::default()), &catalog);
        assert_eq!(session.system(), SystemId(132));
        assert_eq!(session.player().position, Vec2::ZERO);
        assert_eq!(session.player_speed(), 0.0);
        assert_eq!(session.pilot().stellar, None);
    }

    #[test]
    fn a_stay_days_n_keeps_the_touchdown_position() {
        for source in RuleSource::ALL {
            let rules = Rulebook::default().with_override(RuleKey::MoveKeepFlag, source);
            let catalog = moving(Vec::new(), vec![scripted(128, 1, "N133", "")]);
            let mut session = landed(flown(&catalog, rules), &catalog);
            let touchdown = session.port.touchdown;
            session.port.stay.outfit_traded = true;
            let _ = session
                .leave_port(&catalog, &mut NeverFires)
                .expect("leaves");
            assert_eq!(session.system(), SystemId(133), "{source:?}");
            assert_eq!(session.player().position, touchdown, "{source:?}");
            assert_ne!(touchdown, placed_with_no_move(rules), "{source:?}");
            assert_eq!(session.pilot().stellar, None, "{source:?}");
        }
    }

    #[test]
    fn an_n_after_the_placement_holds_only_by_the_engine_keep_flag() {
        for source in RuleSource::ALL {
            let rules = Rulebook::default().with_override(RuleKey::MoveKeepFlag, source);
            let placed = placed_with_no_move(rules);
            let crons = moving(Vec::new(), vec![scripted(128, 1, "N133", "")]);
            let walk = moving(vec![over_130("N133")], Vec::new());
            for (what, catalog) in [("the take-off's day", crons), ("the walk", walk)] {
                let session = left(flown(&catalog, rules), &catalog);
                assert_eq!(session.system(), SystemId(133), "{what} {source:?}");
                assert_eq!(session.player().position, placed, "{what} {source:?}");
                assert_eq!(
                    session.port.hold_position,
                    source == RuleSource::Engine,
                    "{what} {source:?}"
                );
            }
        }
    }

    #[test]
    fn the_take_offs_own_day_runs_after_the_placement() {
        let placed = placed_with_no_move(Rulebook::default());
        let catalog = moving(Vec::new(), vec![scripted(128, 1, "M133", "")]);
        let session = left(flown(&catalog, Rulebook::default()), &catalog);
        assert_eq!(session.system(), SystemId(133));
        assert_eq!(session.player().position, placed, "not 150's (@0x63440)");
        assert_eq!(session.pilot().stellar, Some(StellarId(150)));
    }

    #[test]
    fn the_take_off_explores_the_nebulae_of_the_system_moved_to() {
        // The walk's OnExplore moves to 133; @0x634da then explores 133's
        // nebulae, still inside the take-off, raising no level.
        let catalog = moving(
            vec![over_130("M133"), around(129, (-600, 0), "", "b9")],
            Vec::new(),
        );
        let (session, seen) = watching_explored(flown(&catalog, Rulebook::default()), &[130, 133]);
        let mut session = landed(session, &catalog);
        session
            .take_off(&catalog, &mut NeverFires)
            .expect("takes off");
        assert!(session.pilot().control_bit(Bit::new(9).expect("a bit")));
        assert_eq!(
            seen.systems(),
            [SystemId(130), SystemId(133)],
            "the walk's, then @0x634da's"
        );
        assert!(!session.pilot().has_explored(SystemId(133)), "not raised");
        // With no move, @0x634da finds every nebula explored.
        let catalog = moving(vec![over_130("")], Vec::new());
        let (session, seen) = watching_explored(flown(&catalog, Rulebook::default()), &[130]);
        let mut session = landed(session, &catalog);
        session
            .take_off(&catalog, &mut NeverFires)
            .expect("takes off");
        assert_eq!(seen.systems(), [SystemId(130)], "no second Explored");
    }

    #[test]
    fn a_crons_m_on_a_take_offs_day_is_a_landed_move() {
        let catalog = moving(Vec::new(), vec![scripted(128, 1, "M133", "")]);
        let (session, seen) = watching_entries(flown(&catalog, Rulebook::default()), 601, 133);
        let session = left(session, &catalog);
        assert_eq!(session.system(), SystemId(133));
        assert_eq!(seen.seen(), [], "no enter-system hook");
        assert_eq!(session.npcs().len(), 2, "133's traffic, once");
        assert_eq!(
            session.pilot().explored_level(SystemId(133)),
            ExploredLevel::Charted,
            "the take-off's day comes before the raise and the walk"
        );
    }

    /// Mission 200, offered at 128's mission computer.
    fn errand() -> MissionRecord {
        MissionRecord {
            avail_stel: 128,
            ..mission(200)
        }
    }

    fn offered(session: &Session) -> bool {
        session
            .mission_offers(Venue::Computer)
            .iter()
            .any(|offer| offer.mission == MissionId(200))
    }

    #[test]
    fn a_take_off_move_to_no_system_only_empties_the_offers() {
        for source in RuleSource::ALL {
            let rules = Rulebook::default().with_override(RuleKey::MissingSystemMove, source);
            let mut catalog = moving(Vec::new(), vec![scripted(128, 1, "M999", "")]);
            catalog.missions = vec![errand()];
            let (session, seen) = watching_entries(flown(&catalog, rules), 601, 130);
            let session = landed(session, &catalog);
            assert!(offered(&session), "{source:?}: offered at the landing");
            let mut session = session;
            let _ = session
                .leave_port(&catalog, &mut NeverFires)
                .expect("leaves");
            assert_eq!(session.system(), SystemId(130), "{source:?}");
            assert_eq!(seen.seen(), [], "{source:?}: no enter-system hook");
            assert_eq!(
                offered(&session),
                source != RuleSource::Engine,
                "{source:?}"
            );
        }
    }

    #[test]
    fn a_take_off_move_by_the_bible_is_an_arrival_too() {
        let rules = Rulebook::default().with_override(RuleKey::MoveArrival, RuleSource::Bible);
        let catalog = moving(vec![over_130("M133")], Vec::new());
        let mut session = landed(flown(&catalog, rules), &catalog);
        session.plot_course(SystemId(131)).expect("a route");
        let _ = session
            .leave_port(&catalog, &mut NeverFires)
            .expect("leaves");
        assert_eq!(session.system(), SystemId(133));
        assert!(session.pilot().has_explored(SystemId(133)));
        assert_eq!(session.course(), []);
    }

    #[test]
    fn the_window_is_closed_after_the_take_off() {
        let catalog = moving(vec![over_130("M133")], Vec::new());
        let session = left(flown(&catalog, Rulebook::default()), &catalog);
        assert!(!session.chart.in_take_off_window());
        assert_eq!(session.chart.take_off_window, None);
    }
}
