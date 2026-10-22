//! What a queued `M` or `N` resolves to, and the parts of applying one
//! at its operator that every path shares: outside both windows (the
//! `operator_moves` module), a jump's window (the `jump_moves` module) and
//! a take-off's window (the `take_off_moves` module). All addresses are
//! in `EV Nova.app`'s `EV Nova` (i386).
//!
//! - [`resolve_move`] decides, by [`ScriptEffectRules::missing_system_move`],
//!   whether a move goes to its system, runs the move's tail with no move,
//!   or does nothing.
//! - [`Statics::first_stellar`] is the system's first stellar and its
//!   position, read with no catalog, where an `M` puts the ship at its
//!   operator; [`first_site`] is the same read from a catalog's stellars,
//!   with the stellar a landed `M` docks at, for the entry a move owes.
//! - [`Session::landed_tail`] is `_EvalSetExp`'s landed tail (@0x1584b,
//!   @0x15949): the offers emptied, the system written and the owner
//!   words left unset (see the `owners_unset` module).
//! - [`Session::in_flight_tail`] is its in-flight tail's catalog-free part
//!   (@0x157a6-0x157da, @0x159a3): the system written, the selection let
//!   go, the ship placed and the chart rebuilt.
//! - [`Session::dispatch_move_entry`] is its enter-system hook
//!   (`_MissionHandlePlayerEnteredNewSystem` @0x159a8), dispatched at the
//!   operator.
//! - [`Session::arrive_by_move`] is [`ScriptEffectRules::arrival`]'s other
//!   reading for a move at its operator.
//!
//! [`ScriptEffectRules::missing_system_move`]: super::ScriptEffectRules::missing_system_move
//! [`ScriptEffectRules::arrival`]: super::ScriptEffectRules::arrival

use super::entry::ShipPlacement;
use super::script_effects::{ScriptEffectRules, ScriptMove, move_placement};
use super::story::NoCatalog;
use super::{Session, Statics};
use crate::catalog::{LandingSite, StellarId, SystemId};
use crate::chance::Chance;
use crate::geometry::Vec2;
use crate::landing::is_dockable_destroyed_or_not;
use crate::pilot::ExploredLevel;
use crate::rulebook::{RuleKey, RuleSource};
use crate::story::dispatch::Reentry;
use crate::story::{Entrance, StoryEvent, StoryEventKind};

/// What a queued move resolves to ([`resolve_move`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum MoveTarget {
    /// To `system`, keeping the position when `keep` (`N`).
    To {
        /// The system moved to.
        system: SystemId,
        /// Whether it is an `N`, keeping the position.
        keep: bool,
    },
    /// The move's tail with no move: an `N` out of range, or a move to a
    /// system with no record by the engine reading of
    /// [`ScriptEffectRules::missing_system_move`].
    Stay,
    /// Nothing at all: a move to a system with no record by the other
    /// reading of [`ScriptEffectRules::missing_system_move`].
    Nothing,
}

/// What `step` resolves to by `rules`, `exists` telling whether the
/// galaxy holds a system's record.
pub(super) fn resolve_move(
    step: ScriptMove,
    exists: impl FnOnce(SystemId) -> bool,
    rules: ScriptEffectRules,
) -> MoveTarget {
    let (system, keep) = match step {
        ScriptMove::To(system) => (system, false),
        ScriptMove::KeepPosition(system) => (system, true),
        ScriptMove::Stay => return MoveTarget::Stay,
    };
    if exists(system) {
        MoveTarget::To { system, keep }
    } else if rules.missing_system_move == RuleSource::Engine {
        MoveTarget::Stay
    } else {
        MoveTarget::Nothing
    }
}

/// The first of `sites`, a system's stellars read from a catalog, with
/// the stellar a landed `M` docks at: itself when it can be docked at
/// ([`is_dockable_destroyed_or_not`]), none otherwise (see the
/// `script_effects` module).
pub(super) fn first_site(sites: &[LandingSite]) -> Option<(Vec2, Option<StellarId>)> {
    sites.first().map(|site| {
        let dock = is_dockable_destroyed_or_not(site).then_some(site.id);
        (site.position, dock)
    })
}

/// What an in-flight tail did ([`Session::in_flight_tail`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct InFlightTail {
    /// Whether it moved into a system, rather than running with no move.
    pub(super) moved: bool,
    /// Whether the move was an `N`, keeping the position.
    pub(super) keep: bool,
    /// Whether it placed the ship: an `M` onto a stellar, or at the
    /// centre by [`ScriptEffectRules::starless`]'s other reading.
    ///
    /// [`ScriptEffectRules::starless`]: super::ScriptEffectRules::starless
    pub(super) placed: bool,
}

impl Statics {
    /// `system`'s first stellar's position, with no stellar to dock at,
    /// read with no catalog: where an `M` puts the ship at its operator
    /// ([`move_placement`]'s `first`).
    pub(super) fn first_stellar(&self, system: SystemId) -> Option<(Vec2, Option<StellarId>)> {
        self.first_stellars
            .get(&system)
            .map(|&stellar| (self.stellar_position(stellar), None))
    }
}

impl Session {
    /// `_EvalSetExp`'s landed tail for `step` (see the module docs): the
    /// offers emptied (`_dockedPortMissions` filled with -1, @0x1584b,
    /// @0x15949), then for a move into a system the galaxy holds, the
    /// player's system written (@0x1572b, @0x158f2) and the owner words
    /// left unset for the next rebuild (see the `owners_unset` module).
    /// The system moved to and whether the move keeps the position (`N`);
    /// `None` for a move that runs the tail with no move, and for one that
    /// does nothing, which empties nothing either.
    pub(super) fn landed_tail(&mut self, step: ScriptMove) -> Option<(SystemId, bool)> {
        let rules = self.setup.script_effect_rules();
        let to = match resolve_move(step, |system| self.chart.galaxy.exists(system), rules) {
            MoveTarget::To { system, keep } => Some((system, keep)),
            MoveTarget::Stay => None,
            MoveTarget::Nothing => return None,
        };
        self.run.empty_offers();
        let (system, keep) = to?;
        self.pilot.system = system;
        self.chart.leave_owners_unset(&self.setup);
        Some((system, keep))
    }

    /// `_EvalSetExp`'s in-flight tail for `step`, but its kill flags and
    /// its enter-system hook (see the module docs): for a move into a
    /// system the galaxy holds, the player's system written (@0x1572b,
    /// @0x158f2), the selection let go (@0x157e3, @0x158fd), and the ship
    /// placed as [`move_placement`] says in flight, `M` on the system's
    /// first stellar; then for any move but one that does nothing, the
    /// chart's catalog-free rebuild (HSSV @0x159a3: the course rewritten,
    /// the exploration shared, the known set and the rescue), and for a
    /// move into a system, [`Session::arrive_by_move`]. `None` for a move
    /// that does nothing.
    pub(super) fn in_flight_tail(&mut self, step: ScriptMove) -> Option<InFlightTail> {
        let rules = self.setup.script_effect_rules();
        let to = match resolve_move(step, |system| self.chart.galaxy.exists(system), rules) {
            MoveTarget::To { system, keep } => Some((system, keep)),
            MoveTarget::Stay => None,
            MoveTarget::Nothing => return None,
        };
        let mut tail = InFlightTail::default();
        if let Some((system, keep)) = to {
            self.pilot.system = system;
            self.chart.deselect();
            let first = self.statics.first_stellar(system);
            let placement = move_placement(first, keep, false, rules);
            tail = InFlightTail {
                moved: true,
                keep,
                placed: placement.ship != ShipPlacement::Keep,
            };
            self.place(placement);
        }
        self.chart
            .rebuild_chart(&mut self.pilot, &self.statics, &self.setup);
        if tail.moved {
            self.arrive_by_move(self.pilot.system, rules);
        }
        Some(tail)
    }

    /// Raises an in-flight move's [`StoryEvent::EnteredSystem`] for the
    /// player's system and dispatches it at once, events only and with no
    /// catalog (`_MissionHandlePlayerEnteredNewSystem` @0x159a8, inside
    /// the operator). A move inside a move's own dispatch only raises its
    /// event, which the running dispatch takes within its budget, so
    /// scripts that move each other end. By the other reading of
    /// [`RuleKey::StoryReentry`], never in the middle of a script, it only
    /// raises it, for the next settling to dispatch in turn.
    pub(super) fn dispatch_move_entry(&mut self, chance: &mut dyn Chance) {
        let event = StoryEvent::EnteredSystem {
            system: self.pilot.system,
            how: Entrance::ScriptMove { landed: false },
        };
        let queued =
            Reentry::from(self.setup.rules.source_for(RuleKey::StoryReentry)) == Reentry::Queued;
        if self.chart.move_dispatching || queued {
            self.run.raise(event);
            return;
        }
        self.chart.move_dispatching = true;
        let mark = self.run.story_mark();
        self.run.raise(event);
        self.dispatch_since(&NoCatalog, chance, mark, StoryEventKind::EnteredSystem);
        self.chart.move_dispatching = false;
    }

    /// By [`ScriptEffectRules::arrival`]'s other reading, a move at its
    /// operator to `system` is an arrival too: the system explored at level 1, the
    /// course cleared and [`StoryEvent::Explored`] raised. By the engine
    /// reading, nothing.
    ///
    /// [`ScriptEffectRules::arrival`]: super::ScriptEffectRules::arrival
    pub(super) fn arrive_by_move(&mut self, system: SystemId, rules: ScriptEffectRules) {
        if rules.arrival != RuleSource::Bible {
            return;
        }
        self.chart.explore_place(
            &mut self.pilot,
            &self.setup,
            system,
            ExploredLevel::Explored,
        );
        self.pilot.clear_course();
        self.run.raise(StoryEvent::Explored { system });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rules(missing_system_move: RuleSource) -> ScriptEffectRules {
        ScriptEffectRules {
            missing_system_move,
            ..ScriptEffectRules::default()
        }
    }

    const HELD: SystemId = SystemId(130);

    fn held(system: SystemId) -> bool {
        system == HELD
    }

    #[test]
    fn an_m_to_a_held_system_moves_there_without_keeping_the_position() {
        for source in [RuleSource::Engine, RuleSource::Bible] {
            assert_eq!(
                resolve_move(ScriptMove::To(HELD), held, rules(source)),
                MoveTarget::To {
                    system: HELD,
                    keep: false
                }
            );
        }
    }

    #[test]
    fn an_n_to_a_held_system_moves_there_keeping_the_position() {
        assert_eq!(
            resolve_move(
                ScriptMove::KeepPosition(HELD),
                held,
                rules(RuleSource::Bible)
            ),
            MoveTarget::To {
                system: HELD,
                keep: true
            }
        );
    }

    #[test]
    fn a_move_to_a_system_with_no_record_stays_by_the_engine_reading() {
        for step in [
            ScriptMove::To(SystemId(9)),
            ScriptMove::KeepPosition(SystemId(9)),
        ] {
            assert_eq!(
                resolve_move(step, held, rules(RuleSource::Engine)),
                MoveTarget::Stay
            );
        }
    }

    #[test]
    fn a_move_to_a_system_with_no_record_does_nothing_by_the_other_reading() {
        for step in [
            ScriptMove::To(SystemId(9)),
            ScriptMove::KeepPosition(SystemId(9)),
        ] {
            assert_eq!(
                resolve_move(step, held, rules(RuleSource::Bible)),
                MoveTarget::Nothing
            );
        }
    }

    #[test]
    fn a_queued_stay_stays_under_either_reading_without_asking_the_galaxy() {
        for source in [RuleSource::Engine, RuleSource::Bible] {
            assert_eq!(
                resolve_move(
                    ScriptMove::Stay,
                    |_| panic!("a stay names no system"),
                    rules(source)
                ),
                MoveTarget::Stay
            );
        }
    }
}
