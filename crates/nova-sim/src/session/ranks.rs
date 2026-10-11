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
//! The daily step's eighth participant pays the active ranks' salaries
//! ([`rank::pay_salaries`]), reading the session's
//! [`SalaryCap`](crate::rulebook::RuleKey::SalaryCap) rule; a change to
//! the cash makes a save due.
//!
//! The pilot keeps its active ranks by ID. A pilot flown from a save
//! naming a rank the data no longer has forgets it, so the next save
//! leaves it out, as the original's `_LoadPilotData` drops an active flag
//! whose `ränk` is gone (@0x75e47-0x75eb6).

use std::borrow::Cow;

use crate::catalog::RankId;
use crate::news::MISC_STRINGS;
use crate::person::NO_RANK;
use crate::rank::{self, Held, NO_RANK_STRING, RankNames};
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

    /// What the rank tags read: the pilot's active ranks, the recent
    /// rank, and `STR#` 2002 #341 from the session's string lists as the
    /// fallback, [`NO_RANK`] when the lists lack it (none are wired by
    /// default).
    pub(super) fn rank_names(&self) -> RankNames<'_> {
        RankNames {
            records: &self.ranks,
            active: &self.pilot.ranks,
            recent: self.recent_rank,
            fallback: self
                .nth_string(MISC_STRINGS, NO_RANK_STRING)
                .map_or(Cow::Borrowed(NO_RANK), Cow::Owned),
        }
    }

    /// The day's eighth participant: the active ranks' salaries (see
    /// [`rank::pay_salaries`]), capped by the session's `salary_cap`
    /// rule. A change to the cash makes a save due.
    pub(super) fn ranks_day(&mut self) {
        let cap = self.rules.source_for(RuleKey::SalaryCap);
        let before = self.pilot.cash;
        rank::pay_salaries(&self.ranks, &self.pilot.ranks, &mut self.pilot.cash, cap);
        if self.pilot.cash != before {
            self.save_due = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::catalog::{OutfitId, OutfitRecord, RankId, RankRecord, ShipId, ShipRecord};
    use crate::chance::NeverFires;
    use crate::control::SetExpr;
    use crate::outfitter::OutfitFlags;
    use crate::pilot::Pilot;
    use crate::rank::{SWEEP_GOVT_ON_ACTIVATE, SWEEP_GOVT_ON_DEACTIVATE};
    use crate::rulebook::{RuleKey, RuleSource, Rulebook};
    use crate::save;
    use crate::session::Session;
    use crate::shipyard::ShipFlags3;
    use crate::testkit::{FakePilotCatalog, Scripted, catalog, outfit, rank, ship};

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

    #[test]
    fn letting_a_day_pass_pays_each_active_ranks_salary() {
        let mut session = ranked(vec![
            RankRecord {
                salary: 100,
                ..rank(200)
            },
            RankRecord {
                salary: 50,
                salary_cap: 700,
                ..rank(201)
            },
            RankRecord {
                salary: 1000,
                ..rank(202)
            },
        ]);
        session.run_set(&set("K200 K201"), &mut Scripted::default());
        session.pilot.cash = 500;
        let _ = session.take_save_due();
        session.let_days_pass(1, &mut NeverFires);
        assert_eq!(session.pilot().cash(), 650);
        assert!(session.take_save_due());
        session.let_days_pass(2, &mut NeverFires);
        assert_eq!(session.pilot().cash(), 850, "201 stops at its cap");
    }

    #[test]
    fn the_day_reads_the_sessions_salary_cap() {
        let ranks = vec![RankRecord {
            salary: 200,
            salary_cap: 1100,
            ..rank(200)
        }];
        for (source, cash) in [(RuleSource::Engine, 1200), (RuleSource::Bible, 1100)] {
            let mut session = ranked(ranks.clone())
                .with_rules(Rulebook::default().with_override(RuleKey::SalaryCap, source));
            session.run_set(&set("K200"), &mut Scripted::default());
            session.pilot.cash = 1000;
            session.let_days_pass(1, &mut NeverFires);
            assert_eq!(session.pilot().cash(), cash, "{source:?}");
        }
    }

    #[test]
    fn granting_a_rank_adds_its_contribute_bits_to_the_players() {
        let mut session = ranked(vec![RankRecord {
            contribute: 0x0100_0000_0004,
            ..rank(200)
        }]);
        assert_eq!(session.contributed() & 0x0100_0000_0004, 0);
        session.run_set(&set("K200"), &mut Scripted::default());
        assert_eq!(session.contributed() & 0x0100_0000_0004, 0x0100_0000_0004);
        session.run_set(&set("L200"), &mut Scripted::default());
        assert_eq!(session.contributed() & 0x0100_0000_0004, 0);
    }

    #[test]
    fn an_active_ranks_contribute_lists_a_required_ship_and_outfit() {
        let mut catalog = crate::session::tests::shipbuying();
        catalog.ranks = vec![RankRecord {
            contribute: 0x4,
            ..rank(200)
        }];
        catalog.ship_records.push(ShipRecord {
            require: 0x4,
            flags3: ShipFlags3::HIDE_UNLESS_REQUIRED,
            ..ship(130, catalog.ship_records[0].fields)
        });
        catalog.outfits.push(OutfitRecord {
            flags: OutfitFlags::HIDE_UNLESS_REQUIRED,
            require: 0x4,
            require_govt: -1,
            tech_level: 1,
            ..outfit(310, &[])
        });
        let mut session = crate::session::tests::outfitted(&catalog);
        let listed = |session: &mut Session| {
            let yard = session.shipyard(&mut NeverFires).expect("a shipyard");
            let shop = session.outfitter(&mut NeverFires).expect("an outfitter");
            (
                yard.row(ShipId(130)).is_some(),
                shop.row(OutfitId(310)).is_some(),
            )
        };
        assert_eq!(listed(&mut session), (false, false), "not held");
        session.run_set(&set("K200"), &mut Scripted::default());
        assert_eq!(listed(&mut session), (true, true), "held");
    }

    /// Rank 128, "Commander" of government 128, "Cmdr" in conversation.
    fn commander() -> RankRecord {
        RankRecord {
            name: "Commander".to_owned(),
            conv_name: "Cmdr".to_owned(),
            short_name: "Cdr".to_owned(),
            affil_govt: Some(crate::catalog::GovtId(128)),
            ..rank(128)
        }
    }

    #[test]
    fn ranks_survive_save_and_reload_and_rrk_reads_captain_after_a_load() {
        let catalog = FakePilotCatalog {
            ranks: vec![commander()],
            ..catalog()
        };
        let mut session = Session::start(&catalog).expect("starts");
        assert_eq!(session.plain_text("<PRK> <RRK>"), "captain captain");
        session.run_set(&set("K128"), &mut Scripted::default());
        assert_eq!(
            session.plain_text("<PRK> <SRK> <RRK> <PRK128>"),
            "Cmdr Cdr Commander Cdr"
        );
        let pilot = save::decode(&save::encode(session.pilot())).expect("loads");
        let session = Session::fly(&catalog, pilot).expect("flies");
        assert_eq!(active(&session), [128]);
        assert_eq!(
            session.plain_text("<PRK> <RRK>"),
            "Cmdr captain",
            "the recent rank is not saved"
        );
    }

    /// `STR#` 2002 with #341 `fallback`.
    struct Misc(&'static str);

    impl crate::catalog::CommCatalog for Misc {
        fn string_list(&self, id: i16) -> Vec<String> {
            if id == crate::news::MISC_STRINGS {
                let mut strings = vec![String::new(); 400];
                strings[340] = self.0.to_owned();
                strings
            } else {
                Vec::new()
            }
        }
    }

    #[test]
    fn the_rank_fallback_reads_str_2002_341_when_the_lists_have_it() {
        let session = ranked(vec![commander()]).with_strings(std::rc::Rc::new(Misc("skipper")));
        assert_eq!(
            session.plain_text("<PRK> <SRK> <RRK>"),
            "skipper skipper skipper"
        );
        assert_eq!(session.plain_text("<PRK128>"), "skipper");
    }
}
