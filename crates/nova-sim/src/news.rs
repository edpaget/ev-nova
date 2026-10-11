//! The spaceport news: which running crons have a story for the stellar
//! docked at, as the original's `_RandomizeNewsDialog` (0x48238) chooses
//! them.
//!
//! Nova has no `nëws` resource. A bar's news is a commercial line and one
//! story, drawn once as the bar opens (`_DoBarDialog` 0x48ef5 calls
//! `_RandomizeNewsDialog` @0x48f15) and shown unchanged by the News
//! button (`_DoNewsDialog` 0x47f0a) until the bar closes. The session
//! makes the draws ([`Session::open_bar`](crate::session::Session::open_bar));
//! this module holds the rules it draws among.
//!
//! # A cron's news fields
//!
//! Stock crons use 0 as well as -1 for an unused field, which nova-data
//! decodes raw, so the [`CronRecord`] carries them raw and this module
//! reads them as the engine's loader does (@0x7d8dd-0x7d90e):
//!
//! - A `NewsGovt` above 127 is a government; one of 127 or below is
//!   unused, and so is its slot ([`slots`], [`FIRST_GOVT`]).
//! - A used slot's `GovtNewsStr` is kept as it is, 0 included (the
//!   loader turns only values below -1 into -1). A used slot **matches**
//!   whatever its `STR#`; it **gives a story** only when its `STR#` is
//!   above 0 (`testw; cmovg` @0x4866b).
//! - `IndNewsStr` counts when it is above 0 ([`independent`]).
//!
//! # The candidates
//!
//! Over the crons by ascending ID ([`candidates`]), a cron counts only
//! while it is active, loaded and has a holdoff of 0 or below
//! ([`running`]; @0x485cf-0x485f1). Its `Duration` is not checked. For
//! each of its used slots whose government matches the docked stellar's
//! ([`news_govt_match`], read through
//! [`RuleKey::NewsGovtMatch`]; `_GovtAllies` @0x48640), the cron's pick
//! becomes that slot's `STR#`, so the last matching slot wins. The
//! government is the docked stellar's (@0x48627-0x48631), not the
//! system's. A cron whose pick is above 0 is a government candidate. A
//! cron with no matching slot whose `IndNewsStr` is above 0 is an
//! independent candidate (@0x4868f-0x486ad), at any stellar, governed or
//! not; a matching slot whose `STR#` is 0 or below still blocks it. An
//! independent stellar never matches a slot, so it sees only
//! independent news.
//!
//! A stellar's government of 127 or below loads as independent
//! (@0x77407-0x77416). The port keeps it as that ID, but no used slot's
//! government equals it and [`Governments::allies`] finds it no class,
//! so for news it reads as independent too.
//!
//! # The port's guard
//!
//! The engine sets its "some cron has government news" flag (-0x838) on
//! any matching slot whose `STR#` is above 0 and never clears it, but
//! keeps the last matching slot's `STR#` as the cron's pick
//! (@0x4866b-0x48676). A cron with a matching slot of `STR#` above 0
//! followed by a matching slot of `STR#` 0 or below sets the flag while
//! keeping a pick of 0 or below; if no other cron has a pick above 0,
//! the `Rand(512)` loop that draws among the government candidates
//! (@0x486e2-0x486f8) never ends. No stock cron can reach it, since none
//! has a used slot of `STR#` 0 or below. The port's government
//! candidates are the crons whose pick is above 0, and with none it goes
//! on to the independent candidates. A hang is not worth replicating,
//! so it has no Rulebook reading.
//!
//! # The draws
//!
//! What the session draws, in order (see [`Session::open_bar`](crate::session::Session::open_bar)):
//! the commercial, a string of `STR#` [`COMMERCIALS`] (@0x4826a), else
//! `STR#` [`MISC_STRINGS`] #[`TOP_NEWS`] (@0x48294); the `öops` story
//! (crons-news-ranks phase 8); then, with any government candidates, one
//! of them, else one of the independent candidates, each equally likely
//! (@0x486e2, @0x48704), whose `STR#` string **replaces** the story,
//! empty when that list is missing or empty (`_GetRandomIndString`
//! @0x72f0f); then, with the story empty, a string of `STR#`
//! [`GENERIC_NEWS`] (@0x48763), else `STR#` [`MISC_STRINGS`]
//! #[`NO_NEWS`] (@0x4878d). Each string is drawn uniformly
//! (`Rand(count)+1` @0x72f6e) and kept raw: no tag is filled
//! (`_NewsDialogUpdate` 0x47c57).
//!
//! # Left out
//!
//! - The `öops` story branch (@0x482a4-0x48586) is crons-news-ranks
//!   phase 8's; its slot is kept, empty.
//! - The bit loop at the top of `_RandomizeNewsDialog` (@0x48244-0x48257)
//!   has no effect and is not ported.

use std::collections::BTreeMap;

use crate::catalog::{CronId, CronRecord, GovtId};
use crate::cron::CronState;
use crate::govt::Governments;
use crate::rulebook::{RuleKey, Rulebook};
use crate::rules::{Context, Value};

/// The lowest `NewsGovt` that names a government; below it a slot is
/// unused (@0x7d8dd).
pub const FIRST_GOVT: i16 = 128;
/// `STR#` 8100 "Commercials": the commercial line's strings (@0x4826a).
pub const COMMERCIALS: i16 = 8100;
/// `STR#` 8101 "Generic News": the story with no other (@0x48763).
pub const GENERIC_NEWS: i16 = 8101;
/// `STR#` 2002, the miscellaneous strings the fallbacks come from.
pub const MISC_STRINGS: i16 = 2002;
/// `STR#` 2002 #190 (1-based) "Today's top news:": the commercial line
/// when `STR#` 8100 is empty (@0x48294).
pub const TOP_NEWS: usize = 190;
/// `STR#` 2002 #191 (1-based) "No news is good news": the story when
/// `STR#` 8101 is empty (@0x4878d).
pub const NO_NEWS: usize = 191;

/// A bar visit's news: the commercial line and the story, raw, their
/// tags not filled.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct News {
    /// The commercial line.
    pub commercial: String,
    /// The story.
    pub story: String,
}

/// `record`'s used news slots, each a government (`NewsGovt` of
/// [`FIRST_GOVT`] or above) with its `GovtNewsStr` raw: the engine's load
/// reading (@0x7d8dd-0x7d90e).
pub fn slots(record: &CronRecord) -> impl Iterator<Item = (GovtId, i16)> + '_ {
    record
        .news_govt
        .iter()
        .zip(record.govt_news)
        .filter(|&(&govt, _)| govt >= FIRST_GOVT)
        .map(|(&govt, news)| (GovtId(govt), news))
}

/// `record`'s `IndNewsStr` when it is above 0.
#[must_use]
pub fn independent(record: &CronRecord) -> Option<i16> {
    (record.ind_news > 0).then_some(record.ind_news)
}

/// Whether a cron with these counters has news: active, and with a
/// holdoff of 0 or below (@0x485cf-0x485f1).
#[must_use]
pub fn running(state: CronState) -> bool {
    state.active() && state.holdoff <= 0
}

/// What the news candidates read: the crons, their states, the docked
/// stellar's government, the governments and the rule set.
pub struct NewsFacts<'a> {
    /// Every crön.
    pub crons: &'a [CronRecord],
    /// The crons running or holding off, by ID.
    pub states: &'a BTreeMap<CronId, CronState>,
    /// The docked stellar's government; `None` for an independent.
    pub stellar_govt: Option<GovtId>,
    /// The governments.
    pub govts: &'a Governments,
    /// The rule set in force.
    pub rules: &'a Rulebook,
}

impl Context for NewsFacts<'_> {
    fn rulebook(&self) -> &Rulebook {
        self.rules
    }
}

/// What [`news_govt_match`] reads: one used slot's government, the docked
/// stellar's, the governments and the rule set.
pub struct SlotFacts<'a> {
    /// The slot's `NewsGovt`.
    pub news_govt: GovtId,
    /// The docked stellar's government; `None` for an independent.
    pub stellar_govt: Option<GovtId>,
    /// The governments.
    pub govts: &'a Governments,
    /// The rule set in force.
    pub rules: &'a Rulebook,
}

impl Context for SlotFacts<'_> {
    fn rulebook(&self) -> &Rulebook {
        self.rules
    }
}

/// Whether a used slot's government matches the docked stellar's: by
/// the engine, `_GovtAllies` (@0x48640) finds them allied; by the other
/// reading, they are the same government.
#[must_use]
pub fn news_govt_match<'a>() -> Value<SlotFacts<'a>, bool> {
    Value::disputed(
        "news government",
        "_GovtAllies @0x48640",
        RuleKey::NewsGovtMatch,
        |facts: &SlotFacts| {
            facts
                .govts
                .allies(Some(facts.news_govt), facts.stellar_govt)
        },
        |facts: &SlotFacts| Some(facts.news_govt) == facts.stellar_govt,
    )
}

/// The crons with a story for the docked stellar, each with its `STR#`,
/// by ascending ID.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Candidates {
    /// The crons with government news: the `STR#` of each one's last
    /// matching slot, above 0.
    pub government: Vec<(CronId, i16)>,
    /// The crons with independent news: no matching slot, an
    /// `IndNewsStr` above 0.
    pub independent: Vec<(CronId, i16)>,
}

/// The news candidates (see the module docs).
#[must_use]
pub fn candidates<'a>() -> Value<NewsFacts<'a>, Candidates> {
    Value::plain(
        "news candidates",
        "_RandomizeNewsDialog @0x4858b-0x486c4",
        choose,
    )
}

/// The candidates of [`candidates`].
fn choose(facts: &NewsFacts) -> Candidates {
    let matches = news_govt_match();
    let mut found = Candidates::default();
    for (&id, &state) in facts.states {
        if !running(state) {
            continue;
        }
        let Some(record) = facts.crons.iter().find(|record| record.id == id) else {
            continue;
        };
        let pick = slots(record)
            .filter(|&(news_govt, _)| {
                matches.of(&SlotFacts {
                    news_govt,
                    stellar_govt: facts.stellar_govt,
                    govts: facts.govts,
                    rules: facts.rulebook(),
                })
            })
            .last()
            .map(|(_, news)| news);
        match pick {
            Some(news) if news > 0 => found.government.push((id, news)),
            Some(_) => {}
            None => {
                if let Some(news) = independent(record) {
                    found.independent.push((id, news));
                }
            }
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::GovtRecord;
    use crate::rulebook::RuleSource;
    use crate::testkit::{cron, govt};

    const RUNNING: CronState = CronState {
        days: 5,
        holdoff: 0,
    };

    /// Govt 130 (class 1, allied with class 2), 131 (class 2), 132
    /// (class 3, no allies), and 133 (class 2, derelict).
    fn govts() -> Governments {
        Governments::new([
            GovtRecord {
                classes: [1, -1, -1, -1],
                allies: [2, -1, -1, -1],
                ..govt(130)
            },
            GovtRecord {
                classes: [2, -1, -1, -1],
                ..govt(131)
            },
            GovtRecord {
                classes: [3, -1, -1, -1],
                ..govt(132)
            },
            GovtRecord {
                classes: [2, -1, -1, -1],
                flags: 0x0800,
                ..govt(133)
            },
        ])
    }

    /// Crön `id` with these slots and `IndNews`.
    fn newsy(id: i16, slots: &[(i16, i16)], ind_news: i16) -> CronRecord {
        let mut record = cron(id);
        for (slot, &(govt, news)) in slots.iter().enumerate() {
            record.news_govt[slot] = govt;
            record.govt_news[slot] = news;
        }
        record.ind_news = ind_news;
        record
    }

    fn states(entries: &[(i16, CronState)]) -> BTreeMap<CronId, CronState> {
        entries
            .iter()
            .map(|&(id, state)| (CronId(id), state))
            .collect()
    }

    /// The candidates of `crons`, all running, at a stellar of
    /// `stellar_govt`, by `source`'s reading.
    fn found(crons: &[CronRecord], stellar_govt: Option<i16>, source: RuleSource) -> Candidates {
        let running = states(
            &crons
                .iter()
                .map(|record| (record.id.0, RUNNING))
                .collect::<Vec<_>>(),
        );
        found_with(crons, &running, stellar_govt, source)
    }

    fn found_with(
        crons: &[CronRecord],
        states: &BTreeMap<CronId, CronState>,
        stellar_govt: Option<i16>,
        source: RuleSource,
    ) -> Candidates {
        let rules = Rulebook::default().with_override(RuleKey::NewsGovtMatch, source);
        let govts = govts();
        candidates().of(&NewsFacts {
            crons,
            states,
            stellar_govt: stellar_govt.map(GovtId),
            govts: &govts,
            rules: &rules,
        })
    }

    #[test]
    fn a_slot_is_used_from_govt_128_its_str_kept_raw() {
        let record = newsy(
            128,
            &[(0, 15_000), (-1, 15_001), (127, 15_002), (128, 0)],
            0,
        );
        assert_eq!(slots(&record).collect::<Vec<_>>(), [(GovtId(128), 0)]);
        let record = newsy(128, &[(131, -1), (0, 0), (200, 15_003)], 0);
        assert_eq!(
            slots(&record).collect::<Vec<_>>(),
            [(GovtId(131), -1), (GovtId(200), 15_003)]
        );
    }

    #[test]
    fn independent_news_counts_above_0() {
        assert_eq!(independent(&newsy(128, &[], 0)), None);
        assert_eq!(independent(&newsy(128, &[], -1)), None);
        assert_eq!(independent(&newsy(128, &[], 1)), Some(1));
        assert_eq!(independent(&newsy(128, &[], 15_100)), Some(15_100));
    }

    #[test]
    fn government_news_shows_at_an_allied_stellar_only_by_the_engine() {
        let crons = [newsy(128, &[(130, 15_000)], 0)];
        assert_eq!(
            found(&crons, Some(131), RuleSource::Engine).government,
            [(CronId(128), 15_000)],
            "131 is of a class 130 is allied with"
        );
        assert_eq!(
            found(&crons, Some(131), RuleSource::Bible),
            Candidates::default()
        );
    }

    #[test]
    fn government_news_shows_at_its_own_government_by_either_reading() {
        let crons = [newsy(128, &[(130, 15_000)], 0)];
        for source in [RuleSource::Engine, RuleSource::Bible] {
            assert_eq!(
                found(&crons, Some(130), source).government,
                [(CronId(128), 15_000)]
            );
        }
    }

    #[test]
    fn government_news_does_not_show_at_an_unallied_or_derelict_stellar() {
        let crons = [newsy(128, &[(130, 15_000)], 0)];
        assert_eq!(
            found(&crons, Some(132), RuleSource::Engine),
            Candidates::default()
        );
        assert_eq!(
            found(&crons, Some(133), RuleSource::Engine),
            Candidates::default(),
            "a derelict government is allied with none"
        );
    }

    #[test]
    fn the_last_matching_slot_wins() {
        let crons = [newsy(128, &[(130, 15_000), (131, 15_001)], 0)];
        assert_eq!(
            found(&crons, Some(130), RuleSource::Engine).government,
            [(CronId(128), 15_001)]
        );
        let crons = [newsy(128, &[(130, 15_000), (132, 15_001)], 0)];
        assert_eq!(
            found(&crons, Some(130), RuleSource::Engine).government,
            [(CronId(128), 15_000)],
            "a later slot that does not match keeps the earlier pick"
        );
    }

    #[test]
    fn a_matching_slot_without_a_story_blocks_independent_news() {
        for news in [0, -1] {
            let crons = [newsy(128, &[(130, news)], 15_100)];
            assert_eq!(
                found(&crons, Some(130), RuleSource::Engine),
                Candidates::default(),
                "STR# {news}"
            );
        }
    }

    #[test]
    fn independent_news_shows_at_a_governed_stellar_no_slot_matches() {
        let crons = [newsy(128, &[(132, 15_000)], 15_100)];
        assert_eq!(
            found(&crons, Some(130), RuleSource::Engine),
            Candidates {
                government: vec![],
                independent: vec![(CronId(128), 15_100)],
            }
        );
    }

    #[test]
    fn an_independent_stellar_sees_only_independent_news() {
        let crons = [
            newsy(128, &[(130, 15_000)], 15_100),
            newsy(129, &[(130, 15_001)], 0),
        ];
        for source in [RuleSource::Engine, RuleSource::Bible] {
            assert_eq!(
                found(&crons, None, source),
                Candidates {
                    government: vec![],
                    independent: vec![(CronId(128), 15_100)],
                }
            );
        }
    }

    #[test]
    fn the_candidates_come_by_ascending_id() {
        let crons = [
            newsy(140, &[(130, 15_001)], 0),
            newsy(129, &[(130, 15_000)], 0),
            newsy(135, &[], 15_101),
            newsy(131, &[], 15_100),
        ];
        assert_eq!(
            found(&crons, Some(130), RuleSource::Engine),
            Candidates {
                government: vec![(CronId(129), 15_000), (CronId(140), 15_001)],
                independent: vec![(CronId(131), 15_100), (CronId(135), 15_101)],
            }
        );
    }

    #[test]
    fn a_cron_in_its_holdoff_or_inactive_has_no_news() {
        let crons = [newsy(128, &[(130, 15_000)], 0), newsy(129, &[], 15_100)];
        for state in [
            CronState {
                days: 3,
                holdoff: 2,
            },
            CronState {
                days: -1,
                holdoff: 1,
            },
        ] {
            let held = states(&[(128, state), (129, state)]);
            assert_eq!(
                found_with(&crons, &held, Some(130), RuleSource::Engine),
                Candidates::default(),
                "{state:?}"
            );
        }
        assert_eq!(
            found_with(&crons, &states(&[]), Some(130), RuleSource::Engine),
            Candidates::default(),
            "no state, inactive"
        );
        let ending = states(&[(
            128,
            CronState {
                days: 0,
                holdoff: -1,
            },
        )]);
        assert_eq!(
            found_with(&crons, &ending, Some(130), RuleSource::Engine).government,
            [(CronId(128), 15_000)],
            "a holdoff below 0 with days left runs"
        );
    }

    #[test]
    fn running_is_active_with_no_holdoff_left() {
        assert!(running(CronState {
            days: 0,
            holdoff: 0
        }));
        assert!(running(CronState {
            days: 4,
            holdoff: -3
        }));
        assert!(!running(CronState {
            days: 4,
            holdoff: 1
        }));
        assert!(
            !running(CronState {
                days: -1,
                holdoff: -1
            }),
            "inactive"
        );
    }

    #[test]
    fn a_state_with_no_record_is_skipped() {
        let crons = [newsy(129, &[], 15_100)];
        let running = states(&[(128, RUNNING), (129, RUNNING)]);
        assert_eq!(
            found_with(&crons, &running, Some(130), RuleSource::Engine).independent,
            [(CronId(129), 15_100)]
        );
    }

    #[test]
    fn a_stellar_of_govt_127_or_below_gets_no_government_news() {
        let crons = [newsy(128, &[(130, 15_000)], 15_100)];
        for stellar in [0, 127] {
            for source in [RuleSource::Engine, RuleSource::Bible] {
                assert_eq!(
                    found(&crons, Some(stellar), source),
                    Candidates {
                        government: vec![],
                        independent: vec![(CronId(128), 15_100)],
                    },
                    "govt {stellar}"
                );
            }
        }
    }

    #[test]
    fn a_story_kept_from_an_earlier_slot_is_not_a_candidate_when_a_later_one_has_none() {
        let crons = [
            newsy(128, &[(130, 15_000), (131, 0)], 0),
            newsy(129, &[], 15_100),
        ];
        assert_eq!(
            found(&crons, Some(130), RuleSource::Engine),
            Candidates {
                government: vec![],
                independent: vec![(CronId(129), 15_100)],
            },
            "the port's guard against the engine's endless Rand(512) loop"
        );
    }

    #[test]
    fn the_descriptors_say_what_each_rule_reads_and_neither_draws() {
        let about = *candidates().about();
        assert_eq!(about.reads(), None);
        assert!(!about.draws());
        let about = *news_govt_match().about();
        assert_eq!(about.reads(), Some(RuleKey::NewsGovtMatch));
        assert!(!about.draws());
    }
}
