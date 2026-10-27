//! Every `M` or `N` applied at its operator, as `_EvalSetExp` applies it
//! (crons-news-ranks-fidelity phase 47). All addresses are in
//! `EV Nova.app`'s `EV Nova` (i386).
//!
//! **The original.** Every script reaches `_EvalSetExp` (@0x150fc), and
//! `M` (@0x1570e) and `N` (@0x158d9) write the player's system (+0x74,
//! @0x1572b, @0x158f2) at the operator, then run one of two tails by
//! `_playerIsLanded` (tested @0x1576c, @0x15846, @0x15944), still inside
//! the operator:
//!
//! - **Landed** (set at `_PlayerLandOnStellar`'s entry @0x631ee, cleared
//!   at the take-off @0x63596): `M` makes the first `NavDefs` slot +0x6c
//!   (@0x156f2-0x15705) and writes no position, and both fill
//!   `_dockedPortMissions` with -1 (@0x1584b-0x15869, @0x15949-0x15967).
//!   No rebuild, no hook.
//! - **In flight:** `M` stops the ship on the first slot's stellar
//!   (@0x157a6-0x157da); both set the five kill flags (@0x158a4-0x158cf,
//!   @0x1596e-0x15993), let go of the target (+0x70 = -1, @0x1599d), run
//!   `_HandleStellarSystemVisibility` (@0x159a3) and run
//!   `_MissionHandlePlayerEnteredNewSystem` (@0x159a8).
//!
//! An out-of-range `N` (@0x158e8 -> @0x1593f) runs the same tail with no
//! system write; an out-of-range `M` (@0x1571d -> @0x15873) does nothing.
//! So a later operator of the same script reads the system moved to: a
//! `G` of an N-jump map walks from it (`_GrantOutfitItem` @0x44da6), and
//! `Y`'s owner test compares against it (@0x15a64-0x15a6c).
//!
//! **Here.** `Session::apply_op` queues each move and hands it at once to
//! [`Session::apply_moves_at_operator`], which applies it with no catalog,
//! by where the player is:
//!
//! - in a jump's window, as the jump's own (`Session::move_in_jump`, the
//!   `jump_moves` module);
//! - in a take-off's window, as the take-off's landed move
//!   ([`Session::move_at_take_off`], the `take_off_moves` module);
//! - docked otherwise, by [`Session::move_docked`]: the landed tail
//!   ([`Session::landed_tail`]), the ship placed as a landed move places
//!   it, `M` on the system's first stellar and `N` holding the take-off's
//!   landing position;
//! - in flight otherwise, by [`Session::move_in_flight`]: the in-flight
//!   tail's catalog-free part ([`Session::in_flight_tail`]), then the
//!   enter-system hook dispatched at once ([`Session::dispatch_move_entry`]).
//!
//! What needs a catalog (the stellars shown, the scene let go, the traffic
//! retargeted or populated, the pilot's docked stellar and
//! [`SessionEvent::SystemChanged`](super::SessionEvent::SystemChanged))
//! waits as a [`PendingMove`] for [`Session::settle_pending_move`], which
//! enters the system by the `entry` module's row for a move with the ship
//! kept where the operator put it: at the next settling with a catalog
//! (the end of the script, or for a script the story dispatcher nests,
//! after its operator), or first thing when another way in, a jump's
//! arrival or a take-off comes before that settling, so no entry
//! straddles another.
//!
//! The in-flight entry raises no second [`StoryEvent::EnteredSystem`]: the
//! operator dispatched it. A landed one still raises its engine-only
//! `EnteredSystem { landed: true }` at the completion, which no stock
//! consumer reads; the original runs no hook there.
//!
//! **Ordering.** The system write, the placement, the rebuild's
//! catalog-free parts and the in-flight hook run at the operator, as the
//! original runs them. The stellars shown and their sprites, the kill
//! flags, the traffic, the docked stellar and `SystemChanged` wait for the
//! completion, which every driver reaches after each input and tick,
//! before any frame is drawn, dialog opened or pilot saved. Inside one
//! tick, code after a combat `OnDestroy`'s move still sees the old
//! system's stellars shown until the settling; nothing of that is drawn.
//!
//! **Left out.** The escorts' +0x74 carry (@0x15807-0x1583e,
//! @0x1590b-0x1593d): the completion's retarget, or the take-off's
//! population docked, brings in the ships the player leads, and nothing
//! reads an escort's +0x74 in between. +0x70 = -1 (@0x1599d): the
//! completion's target reset lets it go before any frame. The landed
//! tail's +0x6c (@0x15705) as the nav selection is written at the
//! operator already (`Chart::select_for_landed_move`).
//!
//! [`StoryEvent::EnteredSystem`]: crate::story::StoryEvent::EnteredSystem

use super::entry::{Arrival, Entry, Placement, ShipPlacement};
use super::script_effects::{ScriptMove, move_placement};
use super::script_move::first_site;
use super::{Chart, Session};
use crate::catalog::{PilotCatalog, SystemId, TrafficCatalog};
use crate::chance::Chance;

/// The entry a move applied at its operator, outside both windows, still
/// owes (see the module docs).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct PendingMove {
    /// The system before the first such move: the system the entry leaves.
    pub(super) left: SystemId,
    /// Whether the first such move was a landed one.
    pub(super) landed: bool,
    /// Whether any moved into a system, rather than running the tail with
    /// no move.
    pub(super) moved: bool,
    /// Whether the last that moved into a system was an `N`.
    pub(super) keep: bool,
}

impl Chart {
    /// Notes a move applied at its operator from `left`, `landed` or not,
    /// into a system when `moved`, an `N` when `keep`: the first one opens
    /// the pending entry, and a later one that moved updates it.
    fn note_move(&mut self, left: SystemId, landed: bool, moved: bool, keep: bool) {
        match &mut self.pending_move {
            None => {
                self.pending_move = Some(PendingMove {
                    left,
                    landed,
                    moved,
                    keep,
                });
            }
            Some(pending) => {
                if moved {
                    pending.moved = true;
                    pending.keep = keep;
                }
            }
        }
    }
}

impl Session {
    /// Applies every move the set operator just applied queued, at its
    /// operator, drawing on `chance` for what an in-flight move's hook runs
    /// (see the module docs).
    pub(super) fn apply_moves_at_operator(&mut self, chance: &mut dyn Chance) {
        for step in std::mem::take(&mut self.run.queued.moves) {
            if self.chart.in_jump_window() {
                self.move_in_jump(step, chance);
            } else if self.chart.in_take_off_window() {
                self.move_at_take_off(step);
            } else if self.mode.landed().is_some() {
                self.move_docked(step);
            } else {
                self.move_in_flight(step, chance);
            }
        }
    }

    /// A docked move, `step`, outside the take-off's window: the landed
    /// tail, the ship placed as a landed move places it, and the entry
    /// noted for the completion (see the module docs). A move that runs
    /// the tail with no move only empties the offers and owes nothing.
    pub(super) fn move_docked(&mut self, step: ScriptMove) {
        let left = self.pilot.system;
        let Some((system, keep)) = self.landed_tail(step) else {
            return;
        };
        let rules = self.setup.script_effect_rules();
        let first = self.statics.first_stellar(system);
        self.place(move_placement(first, keep, true, rules));
        self.arrive_by_move(system, rules);
        self.chart.note_move(left, true, true, keep);
    }

    /// An in-flight move, `step`, outside a jump's window: the in-flight
    /// tail, the entry noted for the completion, then the enter-system
    /// hook dispatched at once (see the module docs).
    pub(super) fn move_in_flight(&mut self, step: ScriptMove, chance: &mut dyn Chance) {
        let left = self.pilot.system;
        let Some(tail) = self.in_flight_tail(step) else {
            return;
        };
        self.chart.note_move(left, false, tail.moved, tail.keep);
        self.dispatch_move_entry(chance);
    }

    /// Completes the entry the moves applied at their operators owe, if
    /// any (see the module docs): enters the player's system from the
    /// system the first of them left, its stellars read from `stellars`,
    /// the traffic from `traffic`, populating on `chance` where the rules
    /// say, with the ship kept where the operators put it. In flight it is
    /// a move's entry, or the tail's with no move ([`Entry::ScriptStay`]);
    /// docked, a landed move's, the pilot docked at the system's first
    /// stellar by `M` when it can be docked at ([`first_site`]), at none by
    /// `N`. The system moved to, or `None` for the tail with no move.
    ///
    /// Within a tick, what runs between the operator and this completion
    /// (code after a combat `OnDestroy`'s move) still sees the old
    /// system's stellars shown: nothing of it is drawn before the settling
    /// that completes it.
    pub(super) fn settle_pending_move<S, C, R>(
        &mut self,
        stellars: &S,
        traffic: &C,
        chance: &mut R,
    ) -> Option<SystemId>
    where
        S: PilotCatalog + ?Sized,
        C: TrafficCatalog + ?Sized,
        R: Chance + ?Sized,
    {
        debug_assert!(
            self.run.queued.moves.is_empty(),
            "every move is applied at its operator"
        );
        let pending = self.chart.pending_move.take()?;
        let system = self.pilot.system;
        let sites = self.chart.read_sites(stellars, system);
        let (how, stellar) = if pending.landed {
            let rules = self.setup.script_effect_rules();
            let first = first_site(&sites);
            let stellar = move_placement(first, pending.keep, true, rules).stellar;
            (Entry::ScriptMove { landed: true }, stellar)
        } else if pending.moved {
            (Entry::ScriptMove { landed: false }, None)
        } else {
            (Entry::ScriptStay, None)
        };
        let arrival = Arrival {
            system,
            sites,
            placement: Placement {
                ship: ShipPlacement::Keep,
                stellar,
                hold: false,
            },
        };
        self.enter_system(how, pending.left, arrival, stellars, traffic, chance);
        pending.moved.then_some(self.pilot.system)
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use super::super::jump_moves::tests::{FIRST_150, flown, moving, watching_entries};
    use super::super::kept_rescues::tests::by;
    use super::super::map_walk::tests::{charted, galaxy, line};
    use super::super::tests::{buy, outfitted, outfitting};
    use crate::catalog::{
        BoomId, Explosion, LandingSite, OutfitId, OutfitRecord, SoundId, StellarId, StellarLife,
        SystemId,
    };
    use crate::chance::NeverFires;
    use crate::combat::CombatEvent;
    use crate::control::{Bit, Script, ScriptNote, SetExpr, SetOp};
    use crate::landing::HYPERGATE;
    use crate::rulebook::{RuleKey, RuleSource, Rulebook};
    use crate::session::Session;
    use crate::session::control::SetOpObserver;
    use crate::session::crons::tests::scripted;
    use crate::sound::SimSound;
    use crate::story::{Consumer, EventFilter, StoryEvent, Subscription, SubscriptionId};
    use crate::testkit::{
        FakePilotCatalog, begin_jump_now, fly_out, land_now, life, outfit, planet, star,
    };

    fn run(session: &mut Session, script: &str) {
        session.run_set(&SetExpr::parse(script).expect("parses"), &mut NeverFires);
    }

    /// `session` with a fixture subscription, `record`, to `system`'s
    /// `EnteredSystem` running `script`.
    fn on_entering(session: &mut Session, record: i16, system: i16, script: &str) {
        session
            .register_story(Subscription::new(
                SubscriptionId {
                    consumer: Consumer::Fixture,
                    record,
                    hook: 0,
                },
                EventFilter::EnteredSystem(SystemId(system)),
                Script::parse(script),
            ))
            .expect("registers");
    }

    fn bit(n: u16) -> Bit {
        Bit::new(n).expect("a bit")
    }

    /// The player's system and whether b5 is set, as each `A` operator
    /// applies.
    #[derive(Debug, Default)]
    struct Seen(RefCell<Vec<(SystemId, bool)>>);

    impl SetOpObserver for Seen {
        fn saw(&self, op: &SetOp, session: &Session) {
            if matches!(op, SetOp::AbortMission(_)) {
                self.0
                    .borrow_mut()
                    .push((session.system(), session.control_bit(bit(5))));
            }
        }
    }

    #[test]
    fn an_in_flight_move_writes_the_system_and_runs_its_hook_before_the_next_operator() {
        let catalog = moving(Vec::new(), Vec::new());
        let seen = Rc::new(Seen::default());
        let mut session = flown(&catalog, Rulebook::default()).with_set_op_observer(seen.clone());
        on_entering(&mut session, 1, 133, "b5");
        run(&mut session, "N133 A601");
        assert_eq!(
            *seen.0.borrow(),
            [(SystemId(133), true)],
            "+0x74 written (@0x158f2) and the hook run (@0x159a8) inside the operator"
        );
    }

    #[test]
    fn scripts_that_move_each_other_end_within_the_budget() {
        let mut catalog = moving(Vec::new(), Vec::new());
        catalog.star_map.push(star(134, (-1200.0, 0.0), &[]));
        catalog.systems.push(SystemId(134));
        let mut session = flown(&catalog, Rulebook::default());
        on_entering(&mut session, 1, 133, "N134");
        on_entering(&mut session, 2, 134, "N133");
        run(&mut session, "N133");
        assert!(
            session
                .take_script_notes()
                .iter()
                .any(|note| matches!(note, ScriptNote::EventsDropped { .. })),
            "the budget's drop"
        );
        assert!([SystemId(133), SystemId(134)].contains(&session.system()));
        let moved = session.settle_script(&catalog, &mut NeverFires).moved;
        assert_eq!(moved, Some(session.system()));
        assert_eq!(session.chart.pending_move, None);
    }

    /// [`moving`] with 128 exploding in `bööm` 132, which sounds `snd `
    /// 302.
    fn exploding_128() -> FakePilotCatalog {
        let mut catalog = moving(Vec::new(), Vec::new());
        catalog.boom_sounds = vec![(BoomId(132), SoundId(302))];
        catalog.living([StellarLife {
            strength: 100,
            explosion: Explosion::decode(BOOM),
            ..life(128, 2)
        }])
    }

    #[test]
    fn an_in_flight_moves_entry_completes_at_the_settling() {
        let catalog = exploding_128();
        let (mut session, seen) = watching_entries(flown(&catalog, Rulebook::default()), 601, 133);
        session.chart.target = Some(crate::traffic::npc::NpcId(7));
        session.take_combat_events();
        run(&mut session, "Y128 M133");
        assert_eq!(session.system(), SystemId(133));
        assert_eq!(
            session.player().position,
            FIRST_150,
            "placed at the operator"
        );
        assert_eq!(seen.seen().len(), 1, "the hook, at the operator");
        assert!(
            session.stellars_shown().any(|id| id == StellarId(128)),
            "130's until the entry"
        );
        let settled = session.settle_script(&catalog, &mut NeverFires);
        assert_eq!(settled.moved, Some(SystemId(133)));
        assert_eq!(
            session.stellars_shown().collect::<Vec<_>>(),
            [StellarId(150), StellarId(151)]
        );
        assert_eq!(session.chart.target, None, "let go");
        assert_eq!(session.npcs(), [], "130's traffic left behind");
        assert_eq!(seen.seen().len(), 1, "no second hook");
        assert!(
            !session
                .take_combat_events()
                .iter()
                .any(|event| matches!(event, CombatEvent::ExplodedHeard { .. })),
            "the `Y`'s explosion killed by the move (C2)"
        );
        assert_eq!(session.settle_script(&catalog, &mut NeverFires).moved, None);
    }

    #[test]
    fn a_docked_moves_entry_docks_the_pilot_at_the_first_stellar_it_can_dock_at() {
        let catalog = moving(Vec::new(), Vec::new());
        let mut gated = moving(Vec::new(), Vec::new());
        for (system, sites) in &mut gated.sites {
            if *system == SystemId(133) {
                sites[0].flags2 = HYPERGATE;
            }
        }
        for (catalog, script, docked) in [
            (&catalog, "M133", Some(StellarId(150))),
            (&gated, "M133", None),
            (&catalog, "N133", None),
        ] {
            let mut session = flown(catalog, Rulebook::default());
            assert_eq!(land_now(&mut session), Ok(StellarId(128)));
            let _ = session.settle_script(catalog, &mut NeverFires);
            run(&mut session, script);
            assert_eq!(session.system(), SystemId(133), "{script}");
            assert!(session.chart.pending_move.is_some(), "{script}");
            let settled = session.settle_script(catalog, &mut NeverFires);
            assert_eq!(settled.moved, Some(SystemId(133)), "{script}");
            assert_eq!(session.pilot().stellar, docked, "{script} {docked:?}");
            assert_eq!(
                session.landed(),
                Some(StellarId(128)),
                "{script}: the spaceport still serves 128"
            );
        }
    }

    #[test]
    fn a_later_docked_n_merged_into_a_pending_m_docks_at_none() {
        let catalog = moving(Vec::new(), Vec::new());
        let mut session = flown(&catalog, Rulebook::default());
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        let _ = session.settle_script(&catalog, &mut NeverFires);
        run(&mut session, "M131 N133");
        assert_eq!(session.system(), SystemId(133));
        let settled = session.settle_script(&catalog, &mut NeverFires);
        assert_eq!(settled.moved, Some(SystemId(133)));
        assert_eq!(session.pilot().stellar, None, "the last move's `N`");
    }

    #[test]
    fn a_later_in_flight_move_merged_into_a_pending_stay_completes_as_a_move() {
        let catalog = moving(Vec::new(), Vec::new());
        let rules = Rulebook::default().with_override(RuleKey::OutOfRangeMove, RuleSource::Engine);
        let mut session = flown(&catalog, rules);
        session.take_events();
        run(&mut session, "N9999 M133");
        assert_eq!(session.system(), SystemId(133));
        let settled = session.settle_script(&catalog, &mut NeverFires);
        assert_eq!(settled.moved, Some(SystemId(133)));
        assert_eq!(
            session.stellars_shown().collect::<Vec<_>>(),
            [StellarId(150), StellarId(151)]
        );
        assert_eq!(
            session
                .take_events()
                .iter()
                .filter(|event| **event == crate::session::SessionEvent::SystemChanged)
                .count(),
            1
        );
    }

    #[test]
    fn an_in_flight_move_to_no_system_runs_the_tail_where_the_player_is() {
        for (script, key) in [
            ("N9999", RuleKey::OutOfRangeMove),
            ("M999", RuleKey::MissingSystemMove),
        ] {
            for source in RuleSource::ALL {
                let catalog = moving(Vec::new(), Vec::new());
                let rules = Rulebook::default().with_override(key, source);
                let (mut session, seen) = watching_entries(flown(&catalog, rules), 601, 130);
                session.take_events();
                run(&mut session, script);
                assert_eq!(
                    seen.seen().len(),
                    by(source, 1, 0),
                    "{script} {source:?}: the hook at the operator"
                );
                let settled = session.settle_script(&catalog, &mut NeverFires);
                assert_eq!(settled.moved, None, "{script} {source:?}");
                assert_eq!(session.system(), SystemId(130), "{script} {source:?}");
                assert_eq!(seen.seen().len(), by(source, 1, 0), "{script} {source:?}");
                assert_eq!(
                    session
                        .take_events()
                        .contains(&crate::session::SessionEvent::SystemChanged),
                    source == RuleSource::Engine,
                    "{script} {source:?}: the stay's entry"
                );
            }
        }
    }

    #[test]
    fn as_an_arrival_the_operator_explores_once_and_the_entry_populates() {
        let rules = Rulebook::default().with_override(RuleKey::MoveArrival, RuleSource::Bible);
        let catalog = moving(Vec::new(), Vec::new());
        let mut session = flown(&catalog, rules);
        session.plot_course(SystemId(131)).expect("a route");
        session.run.drop_raised();
        run(&mut session, "M133");
        assert_eq!(session.course(), [], "cleared at the operator");
        let explored = |session: &Session| {
            session
                .story_raised()
                .iter()
                .filter(|event| {
                    **event
                        == StoryEvent::Explored {
                            system: SystemId(133),
                        }
                })
                .count()
        };
        assert_eq!(explored(&session), 1, "at the operator");
        session.settle_moves(
            &catalog,
            &mut NeverFires,
            &mut super::super::Settled::default(),
        );
        assert_eq!(explored(&session), 1, "not again at the entry");
        assert_eq!(session.npcs().len(), 2, "133's traffic");
    }

    #[test]
    fn a_take_off_completes_a_docked_moves_entry_first() {
        let catalog = moving(Vec::new(), Vec::new());
        let mut session = flown(&catalog, Rulebook::default());
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        let _ = session.settle_script(&catalog, &mut NeverFires);
        run(&mut session, "M133");
        session
            .take_off(&catalog, &mut NeverFires)
            .expect("takes off");
        assert_eq!(session.chart.pending_move, None);
        assert_eq!(session.pilot().stellar, Some(StellarId(150)), "docked at");
        assert_eq!(
            session.stellars_shown().collect::<Vec<_>>(),
            [StellarId(150), StellarId(151)]
        );
    }

    #[test]
    fn a_jumps_arrival_completes_an_in_flight_moves_entry_first() {
        // A move made in hyperspace, not yet settled as the jump ends: its
        // entry comes first and gives up the jump, as it does at a
        // settling, so no entry straddles the jump's window.
        let catalog = moving(Vec::new(), Vec::new());
        let mut session = flown(&catalog, Rulebook::default());
        session.plot_course(SystemId(131)).expect("a route");
        fly_out(&mut session);
        assert_eq!(begin_jump_now(&mut session), Ok(SystemId(131)));
        run(&mut session, "N133");
        assert_eq!(session.arrive(&catalog, &mut NeverFires), None);
        assert_eq!(session.chart.pending_move, None);
        assert!(!session.chart.in_jump_window());
        assert_eq!(session.system(), SystemId(133));
        assert_eq!(session.settle_script(&catalog, &mut NeverFires).moved, None);
    }

    #[test]
    fn another_way_in_completes_a_moves_entry_first() {
        let catalog = moving(Vec::new(), Vec::new());
        let mut session = flown(&catalog, Rulebook::default());
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        let _ = session.settle_script(&catalog, &mut NeverFires);
        run(&mut session, "M133");
        session
            .relocate(&catalog, SystemId(131), StellarId(140))
            .expect("moves");
        assert_eq!(session.chart.pending_move, None);
        assert_eq!(session.system(), SystemId(131));
        assert_eq!(session.settle_script(&catalog, &mut NeverFires).moved, None);
    }

    #[test]
    fn a_site_list_gives_its_first_with_the_stellar_to_dock_at() {
        let gate = LandingSite {
            flags2: HYPERGATE,
            ..planet(151, 1.0, 2.0)
        };
        assert_eq!(super::first_site(&[]), None);
        assert_eq!(
            super::first_site(&[planet(150, 3.0, 4.0), gate]),
            Some((crate::geometry::Vec2::new(3.0, 4.0), Some(StellarId(150))))
        );
        assert_eq!(
            super::first_site(&[gate]),
            Some((crate::geometry::Vec2::new(1.0, 2.0), None))
        );
    }

    /// The explosion sounds among `sounds`.
    fn explosions(sounds: &[SimSound]) -> Vec<SimSound> {
        sounds
            .iter()
            .copied()
            .filter(|sound| matches!(sound, SimSound::Explosion { .. }))
            .collect()
    }

    #[test]
    fn an_in_flight_days_n_then_map_charts_around_the_system_moved_to() {
        // 130 - 131 - 132 - 133. A crön's `OnStart` on a day in flight,
        // outside a jump: the `N` writes 133 at its operator (@0x158f2),
        // and the 1-jump map walks from 133 (`_GrantOutfitItem` @0x44da6
        // reads +0x74), charting 132, not 131.
        let catalog = FakePilotCatalog {
            crons: vec![scripted(128, 1, "N133 G301", "")],
            ..galaxy(line(), Vec::new(), 3, 1)
        };
        let mut session = flown(&catalog, Rulebook::default());
        let before = charted(&session);
        session.let_days_pass(1, &mut NeverFires);
        assert_eq!(session.system(), SystemId(133), "at the operator");
        let after = charted(&session);
        assert!(after.contains(&132), "{before:?} -> {after:?}");
        assert!(!after.contains(&131), "{before:?} -> {after:?}");
        let _ = session.settle_script(&catalog, &mut NeverFires);
        assert_eq!(session.system(), SystemId(133));

        // A ship's `OnCapture`, run through `run_script` in flight.
        let catalog = galaxy(line(), Vec::new(), 3, 1);
        let mut session = flown(&catalog, Rulebook::default());
        session.run_script(&Script::parse("N133 G301"), &mut NeverFires);
        let after = charted(&session);
        assert!(after.contains(&132) && !after.contains(&131), "{after:?}");
    }

    /// `bööm` 132, explosion type 4.
    const BOOM: i16 = 4;

    /// [`outfitting`] with 132 and 133, 133 listing 150 at [`FIRST_150`]
    /// and 151, 150 exploding in `bööm` 132, which sounds `snd ` 302, and
    /// outfit 310, whose `OnPurchase` is `on_purchase`.
    fn exploding_150(on_purchase: &str) -> FakePilotCatalog {
        let mut catalog = outfitting();
        catalog.systems.extend([SystemId(132), SystemId(133)]);
        catalog.sites.push((
            SystemId(133),
            vec![planet(150, FIRST_150.x, FIRST_150.y), planet(151, 0.0, 0.0)],
        ));
        catalog.outfits.push(OutfitRecord {
            on_purchase: Script::parse(on_purchase),
            ..outfit(310, &[])
        });
        catalog.boom_sounds = vec![(BoomId(132), SoundId(302))];
        catalog.living([StellarLife {
            strength: 100,
            explosion: Explosion::decode(BOOM),
            ..life(150, 2)
        }])
    }

    #[test]
    fn a_docked_scripts_m_then_y_explodes_the_stellar_by_its_owner() {
        // Docked at 128, an outfit's `OnPurchase` (`_DoOutfitDialog`
        // @0x5c2cb, landed) moves to 133 and destroys 150, which 133 owns:
        // `Y`'s owner test reads +0x74 as the `M` wrote it (@0x15a64-0x15a6c).
        let catalog = exploding_150("M133 Y150");
        let mut session = outfitted(&catalog);
        session.take_sounds();
        session.take_combat_events();
        session
            .outfit(buy(OutfitId(310)), &mut NeverFires)
            .expect("bought");
        assert_eq!(session.system(), SystemId(133));
        assert_eq!(
            explosions(&session.take_sounds()).len(),
            1,
            "heard at once, docked"
        );
        assert!(session.galaxy().destroyed(StellarId(150)));
        assert!(
            !session
                .take_combat_events()
                .iter()
                .any(|event| matches!(event, CombatEvent::ExplodedHeard { .. })),
            "never drawn docked"
        );

        // 150 only rescued into 133: 129 shadows 133 at its place, so no
        // system owns it, and the landed tail runs no HSSV (the owners
        // stay unset), so nothing explodes.
        let mut catalog = exploding_150("M133 Y150");
        catalog.star_map.push(star(129, (-600.0, 0.0), &[]));
        let mut session = outfitted(&catalog);
        session.take_sounds();
        session
            .outfit(buy(OutfitId(310)), &mut NeverFires)
            .expect("bought");
        assert_eq!(session.system(), SystemId(133));
        assert_eq!(explosions(&session.take_sounds()), []);
        assert!(session.galaxy().destroyed(StellarId(150)));
    }
}
