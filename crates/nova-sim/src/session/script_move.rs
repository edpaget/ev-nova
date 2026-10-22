//! What a queued `M` or `N` resolves to, and the parts of applying one
//! that every window shares: the settling after a set expression
//! ([`Session::settle_moves`]), a jump's window (the `jump_moves` module)
//! and a take-off's window (the `take_off_moves` module).
//!
//! - [`resolve_move`] decides, by [`ScriptEffectRules::missing_system_move`],
//!   whether a move goes to its system, runs the move's tail with no move,
//!   or does nothing.
//! - [`Statics::first_stellar`] is the system's first stellar and its
//!   position, read with no catalog, as the windows read it.
//! - [`Session::arrive_by_move`] is [`ScriptEffectRules::arrival`]'s other
//!   reading for a window's move.
//!
//! [`ScriptEffectRules::missing_system_move`]: super::ScriptEffectRules::missing_system_move
//! [`ScriptEffectRules::arrival`]: super::ScriptEffectRules::arrival

use super::script_effects::{ScriptEffectRules, ScriptMove};
use super::{Session, Statics};
use crate::catalog::{StellarId, SystemId};
use crate::geometry::Vec2;
use crate::pilot::ExploredLevel;
use crate::rulebook::RuleSource;
use crate::story::StoryEvent;

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

impl Statics {
    /// `system`'s first stellar's position, with no stellar to dock at,
    /// read with no catalog: where a window's `M` puts the ship
    /// ([`super::script_effects::move_placement`]'s `first`).
    pub(super) fn first_stellar(&self, system: SystemId) -> Option<(Vec2, Option<StellarId>)> {
        self.first_stellars
            .get(&system)
            .map(|&stellar| (self.stellar_position(stellar), None))
    }
}

impl Session {
    /// By [`ScriptEffectRules::arrival`]'s other reading, a window's move
    /// to `system` is an arrival too: the system explored at level 1, the
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
