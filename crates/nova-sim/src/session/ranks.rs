//! The ranks on the session (see [`rank`](crate::rank)).
//!
//! `K` and `L` apply at once, inside the script that runs them, as the
//! original's `_EvalSetExp` calls `_ActivateRank` and `_DeactivateRank`
//! as it meets them (@0x159e1, @0x15a17); neither runs a script. The
//! sweeps read the session's
//! [`RankUnaffiliatedGroup`](crate::rulebook::RuleKey::RankUnaffiliatedGroup)
//! rule. A change to the active ranks makes a save due, as any change a
//! set expression makes to the pilot does. The recent rank `<RRK>` reads
//! is the session's, never saved, and none on every
//! [`Session::fly`](crate::Session::fly), as the original's `ResetPlayer`
//! (@0x1db44) and `ResetPlayerRecord` (@0x1dbf1) reset it.
//!
//! The pilot keeps its active ranks by ID. A pilot flown from a save
//! naming a rank the data no longer has forgets it, so the next save
//! leaves it out, as the original's `_LoadPilotData` drops an active flag
//! whose `ränk` is gone (@0x75e47-0x75eb6).

use crate::catalog::RankId;
use crate::rank::{self, Held};
use crate::rulebook::RuleKey;
use crate::session::Session;

impl Session {
    /// The pilot's ranks and the recent rank, as `K` and `L` change them.
    fn held_ranks(&mut self) -> Held<'_> {
        Held {
            active: &mut self.pilot.ranks,
            recent: &mut self.recent_rank,
        }
    }

    /// `K`: activates rank `rank` (see [`rank::activate`]), unaffiliated
    /// ranks grouped by the session's `rank_unaffiliated_group` rule.
    pub(super) fn activate_rank(&mut self, rank: RankId) {
        let unaffiliated = self.rules.source_for(RuleKey::RankUnaffiliatedGroup);
        let records = std::rc::Rc::clone(&self.ranks);
        rank::activate(&records, self.held_ranks(), rank, unaffiliated);
    }

    /// `L`: deactivates rank `rank` (see [`rank::deactivate`]),
    /// unaffiliated ranks grouped by the session's
    /// `rank_unaffiliated_group` rule.
    pub(super) fn deactivate_rank(&mut self, rank: RankId) {
        let unaffiliated = self.rules.source_for(RuleKey::RankUnaffiliatedGroup);
        let records = std::rc::Rc::clone(&self.ranks);
        rank::deactivate(&records, self.held_ranks(), rank, unaffiliated);
    }
}

#[cfg(test)]
mod tests {
    use crate::catalog::{RankId, RankRecord};
    use crate::control::SetExpr;
    use crate::pilot::Pilot;
    use crate::rank::{SWEEP_GOVT_ON_ACTIVATE, SWEEP_GOVT_ON_DEACTIVATE};
    use crate::rulebook::{RuleKey, RuleSource, Rulebook};
    use crate::save;
    use crate::session::Session;
    use crate::testkit::{FakePilotCatalog, Scripted, catalog, rank};

    fn set(text: &str) -> SetExpr {
        SetExpr::parse(text).expect("parses")
    }

    /// A session over `ranks`.
    fn ranked(ranks: Vec<RankRecord>) -> Session {
        let catalog = FakePilotCatalog { ranks, ..catalog() };
        Session::start(&catalog).expect("starts")
    }

    /// The session's active ranks.
    fn active(session: &Session) -> Vec<i16> {
        session.pilot().ranks().iter().map(|id| id.0).collect()
    }

    #[test]
    fn flying_a_pilot_forgets_a_rank_the_data_no_longer_has() {
        let catalog = FakePilotCatalog {
            ranks: vec![rank(128)],
            ..catalog()
        };
        let mut pilot = Pilot::new(&catalog, "Ada").expect("starts");
        pilot.ranks.extend([RankId(128), RankId(200)]);
        let pilot = save::decode(&save::encode(&pilot)).expect("loads");
        assert_eq!(pilot.ranks().len(), 2, "the save keeps it");
        let session = Session::fly(&catalog, pilot).expect("flies");
        assert_eq!(
            session.pilot().ranks().iter().collect::<Vec<_>>(),
            [&RankId(128)]
        );
        let saved = save::decode(&save::encode(session.pilot())).expect("loads");
        assert_eq!(saved.ranks().iter().collect::<Vec<_>>(), [&RankId(128)]);
    }

    #[test]
    fn k_and_l_apply_at_once_inside_a_script() {
        let mut session = ranked(vec![rank(200)]);
        let mut seen = Vec::new();
        session.run_ops(
            &set("K200 L200 K201"),
            &mut Scripted::default(),
            &mut |session, _| seen.push((active(session), session.recent_rank)),
        );
        assert_eq!(
            seen,
            [
                (vec![200], Some(RankId(200))),
                (vec![], None),
                (vec![201], Some(RankId(201))),
            ],
            "each applied before the next operator; 201 has no record"
        );
        assert_eq!(session.take_script_notes(), []);
    }

    #[test]
    fn a_rank_change_makes_a_save_due() {
        let mut session = ranked(vec![rank(200)]);
        session.run_set(&set("K200"), &mut Scripted::default());
        assert!(session.take_save_due());
        session.run_set(&set("K200"), &mut Scripted::default());
        assert!(!session.take_save_due(), "already active");
        session.run_set(&set("L200"), &mut Scripted::default());
        assert!(session.take_save_due());
    }

    #[test]
    fn the_sweeps_read_the_sessions_rank_unaffiliated_group() {
        let ranks = vec![
            RankRecord {
                flags: SWEEP_GOVT_ON_ACTIVATE,
                ..rank(200)
            },
            RankRecord {
                flags: SWEEP_GOVT_ON_DEACTIVATE,
                ..rank(201)
            },
            rank(202),
        ];
        for (source, after_k, after_l) in [
            (RuleSource::Engine, vec![200], vec![]),
            (RuleSource::Bible, vec![200, 201, 202], vec![200, 202]),
        ] {
            let mut session = ranked(ranks.clone()).with_rules(
                Rulebook::default().with_override(RuleKey::RankUnaffiliatedGroup, source),
            );
            session.run_set(&set("K201 K202 K200"), &mut Scripted::default());
            assert_eq!(active(&session), after_k, "{source:?}");
            session.run_set(&set("K201 K202 L201"), &mut Scripted::default());
            assert_eq!(active(&session), after_l, "{source:?}");
        }
    }
}
