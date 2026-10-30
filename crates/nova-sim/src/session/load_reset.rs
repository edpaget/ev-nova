//! What a load forgets, as the original's open resets the player.
//!
//! `_OpenPilotFile` (@0x7673e, then `_LoadPilotData` @0x7677e) and
//! `_OpenLastPilotFile` (@0x76502, then @0x76582) call `_ResetPlayer`
//! (@0x1d40d-0x1db6b) before they load. Ours builds a fresh `Session` on
//! every load, so only a saved [`Pilot`] field that `_LoadPilotData`
//! (@0x750b5-0x764ac) doesn't restore can diverge. The audit of every
//! `_ResetPlayer` write is in
//! rdm:phase/play-feel-5/phase-11-forget-on-load-what-reset-player-clears.
//! The fields it forgets that our save keeps, and where each is handled:
//!
//! - the course and its start (`_hyperRoute` @0x1d6bb-0x1d6cd), by
//!   [`RuleKey::LoadedCourse`] in the `course` module's `course_on_load`;
//! - the explored nebulae (@0x1da86), never saved and forgotten by
//!   `Session::load` (see the `nebulae` module);
//! - the offer roll table (@0x1d7e3, @0x1d940), drawn by `Session::resume`;
//! - the system last jumped from (player +0x94 @0x1d5b1-0x1d5b8), by
//!   [`RuleKey::LoadedJumpedFrom`] here ([`jumped_from_on_load`]).

use crate::pilot::Pilot;
use crate::rulebook::{RuleKey, RuleSource, Rulebook};

/// A load's system last jumped from, as [`RuleKey::LoadedJumpedFrom`]
/// reads it: by the engine it is forgotten, as `_ResetPlayer` sets player
/// +0x94 to -1 (@0x1d5b1-0x1d5b8) and nothing on the load path writes it
/// back. By the other reading the saved system is kept.
pub(super) fn jumped_from_on_load(pilot: &mut Pilot, rules: &Rulebook) {
    if rules.source_for(RuleKey::LoadedJumpedFrom) == RuleSource::Engine {
        pilot.jumped_from = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Session;
    use crate::catalog::SystemId;
    use crate::testkit::catalog;

    /// What `engine` and `bible` give by `source`.
    fn by<T>(source: RuleSource, engine: T, bible: T) -> T {
        match source {
            RuleSource::Engine => engine,
            RuleSource::Bible => bible,
        }
    }

    fn loaded_jumped_from(source: RuleSource) -> Rulebook {
        Rulebook::default().with_override(RuleKey::LoadedJumpedFrom, source)
    }

    /// A pilot that last jumped from `from`.
    fn jumped_from(from: Option<SystemId>) -> Pilot {
        let mut pilot = Pilot::new(&catalog(), "").expect("a pilot");
        pilot.jumped_from = from;
        pilot
    }

    #[test]
    fn a_load_forgets_the_system_jumped_from_by_the_engine_and_keeps_it_by_the_other_reading() {
        for source in RuleSource::ALL {
            let mut pilot = jumped_from(Some(SystemId(130)));
            jumped_from_on_load(&mut pilot, &loaded_jumped_from(source));
            assert_eq!(
                pilot.jumped_from,
                by(source, None, Some(SystemId(130))),
                "{source:?}"
            );
        }
    }

    #[test]
    fn a_load_leaves_no_jumped_from_as_none_under_both_readings() {
        for source in RuleSource::ALL {
            let mut pilot = jumped_from(None);
            jumped_from_on_load(&mut pilot, &loaded_jumped_from(source));
            assert_eq!(pilot.jumped_from, None, "{source:?}");
        }
    }

    #[test]
    fn a_saved_jumped_from_loads_as_the_rule_reads_and_the_save_keeps_it() {
        let catalog = catalog();
        for source in RuleSource::ALL {
            let pilot = jumped_from(Some(SystemId(130)));
            let saved = crate::save::decode(&crate::save::encode(&pilot)).expect("loads");
            assert_eq!(
                saved.jumped_from,
                Some(SystemId(130)),
                "the save keeps the system"
            );
            let session =
                Session::load(&catalog, saved, &loaded_jumped_from(source)).expect("flies");
            assert_eq!(
                session.pilot().jumped_from,
                by(source, None, Some(SystemId(130))),
                "{source:?}"
            );
        }
    }
}
