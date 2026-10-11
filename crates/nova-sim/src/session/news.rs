//! The bar's news: the session's side of [`news`](crate::news).
//!
//! **Drawing it.** As the bar opens at a stellar with one
//! ([`Session::open_bar`]), the session draws the visit's news, as
//! `_RandomizeNewsDialog` (0x48238) does when `_DoBarDialog` opens
//! (@0x48f15), in this order:
//!
//! 1. The commercial line: a string of `STR#`
//!    [`COMMERCIALS`] (@0x4826a), else
//!    `STR#` 2002 #190 (@0x48294), else nothing.
//! 2. The story starts as the `öops` story, empty until crons-news-ranks
//!    phase 8 fills it.
//! 3. With any government candidates ([`candidates`]), one of them, else
//!    with any independent candidates one of those, each equally likely
//!    (@0x486e2, @0x48704); that cron's `STR#` string **replaces** the
//!    story, empty when its list is missing or empty (@0x72f0f).
//! 4. With the story empty, a string of `STR#`
//!    [`GENERIC_NEWS`] (@0x48763), else
//!    `STR#` 2002 #191 (@0x4878d), else nothing.
//!
//! Each string is drawn uniformly among its list's, and every list is
//! read through the session's string lists ([`Session::with_strings`]).
//! The text is kept raw, its tags not filled, as `_NewsDialogUpdate`
//! draws it.
//!
//! **Keeping it.** The visit's news ([`Session::news`]) is shown again
//! each time News is pressed, unchanged, until the bar opens again,
//! which draws afresh. It is forgotten as the ship's docking changes (a
//! landing, a move, a take-off), and never saved, as the original's
//! `_newsString` globals are not. The draws are made as the bar opens,
//! before anything can show them, so no ordering boundary is crossed.
//! News has no participant in the daily step and no trigger point of its
//! own.

use super::Session;
use crate::chance::Chance;
use crate::news::{
    COMMERCIALS, GENERIC_NEWS, MISC_STRINGS, NO_NEWS, News, NewsFacts, TOP_NEWS, candidates,
};

impl Session {
    /// Opens the bar of the stellar the ship is docked at: draws the
    /// visit's news on `chance` (see the module docs) and keeps it, and
    /// gives true. False, and nothing changes, when the ship has not
    /// landed at a stellar with a bar.
    pub fn open_bar(&mut self, chance: &mut dyn Chance) -> bool {
        let Some(site) = self.bar_site() else {
            return false;
        };
        let commercial = self
            .random_string(COMMERCIALS, chance)
            .or_else(|| self.nth_string(MISC_STRINGS, TOP_NEWS))
            .unwrap_or_default();
        let mut story = self.disaster_story(chance);
        let found = candidates().of(&NewsFacts {
            crons: &self.crons,
            states: &self.pilot.crons,
            stellar_govt: site.govt,
            govts: &self.govts,
            rules: &self.rules,
        });
        let pool = if found.government.is_empty() {
            found.independent
        } else {
            found.government
        };
        if !pool.is_empty() {
            let (_, list) = pool[pick(pool.len(), chance)];
            story = self.random_string(list, chance).unwrap_or_default();
        }
        if story.is_empty() {
            story = self
                .random_string(GENERIC_NEWS, chance)
                .or_else(|| self.nth_string(MISC_STRINGS, NO_NEWS))
                .unwrap_or_default();
        }
        self.bar_news = Some(News { commercial, story });
        true
    }

    /// The news of the bar visit under way, as it was drawn: `None`
    /// before the bar opens, and after the ship's docking changes.
    #[must_use]
    pub fn news(&self) -> Option<&News> {
        self.bar_news.as_ref()
    }

    /// The `öops` story: the story the cron news replaces. Empty until
    /// crons-news-ranks phase 8 fills it (`_RandomizeNewsDialog`
    /// @0x482a4-0x48586).
    #[allow(clippy::unused_self)]
    fn disaster_story(&self, _chance: &mut dyn Chance) -> String {
        String::new()
    }

    /// A string of `STR#` `list` drawn on `chance`, each equally likely
    /// (`_GetRandomIndString` @0x72f6e); `None` when the list is missing
    /// or empty, drawing nothing.
    fn random_string(&self, list: i16, chance: &mut dyn Chance) -> Option<String> {
        let mut strings = self.strings.0.string_list(list);
        if strings.is_empty() {
            return None;
        }
        let index = pick(strings.len(), chance);
        Some(strings.swap_remove(index))
    }

    /// String `index` (1-based) of `STR#` `list`, if it has one.
    fn nth_string(&self, list: i16, index: usize) -> Option<String> {
        let mut strings = self.strings.0.string_list(list);
        let at = index.checked_sub(1)?;
        (at < strings.len()).then(|| strings.swap_remove(at))
    }
}

/// One of `count` (above 0) equal outcomes, drawn on `chance`.
fn pick(count: usize, chance: &mut dyn Chance) -> usize {
    let sides = u16::try_from(count).unwrap_or(u16::MAX);
    usize::from(chance.roll(sides)).min(count - 1)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::rc::Rc;

    use super::*;
    use crate::catalog::{
        CommCatalog, CronId, CronRecord, GovtId, GovtRecord, LandingSite, StellarId, SystemId,
    };
    use crate::chance::NeverFires;
    use crate::cron::CronState;
    use crate::landing::StellarFlags;
    use crate::rulebook::{RuleKey, RuleSource, Rulebook};
    use crate::session::entry::NoTraffic;
    use crate::testkit::{FakePilotCatalog, Scripted, catalog, cron, govt, land_now, planet};

    const RUNNING: CronState = CronState {
        days: 5,
        holdoff: 0,
    };

    /// String lists by ID, as a test sets them.
    #[derive(Default)]
    struct Lists(Vec<(i16, Vec<String>)>);

    impl Lists {
        fn with(mut self, id: i16, strings: &[&str]) -> Self {
            self.0
                .push((id, strings.iter().map(|&text| text.to_owned()).collect()));
            self
        }
    }

    impl CommCatalog for Lists {
        fn string_list(&self, id: i16) -> Vec<String> {
            self.0
                .iter()
                .find(|(list, _)| *list == id)
                .map_or_else(Vec::new, |(_, strings)| strings.clone())
        }
    }

    /// `STR#` 2002 with its #190 and #191 and nothing else of note.
    fn misc() -> Vec<&'static str> {
        let mut strings = vec!["-"; 191];
        strings[189] = "Today's top news:";
        strings[190] = "No news is good news";
        strings
    }

    /// The usual lists: one commercial, one generic story, and `STR#`
    /// 2002.
    fn lists() -> Lists {
        Lists::default()
            .with(COMMERCIALS, &["Buy Bibble"])
            .with(GENERIC_NEWS, &["Nothing happened"])
            .with(MISC_STRINGS, &misc())
    }

    /// Govt 130 (class 1, allied with class 2) and 131 (class 2).
    fn govts() -> Vec<GovtRecord> {
        vec![
            GovtRecord {
                classes: [1, -1, -1, -1],
                allies: [2, -1, -1, -1],
                ..govt(130)
            },
            GovtRecord {
                classes: [2, -1, -1, -1],
                ..govt(131)
            },
        ]
    }

    /// Stellar 128 at the centre, of government `govt`, with these
    /// `flags`.
    fn stellar(govt: Option<i16>, flags: u32) -> LandingSite {
        LandingSite {
            flags,
            govt: govt.map(GovtId),
            ..planet(128, 0.0, 0.0)
        }
    }

    /// Crön `id` with these news slots and `IndNews`.
    fn newsy(id: i16, slots: &[(i16, i16)], ind_news: i16) -> CronRecord {
        let mut record = cron(id);
        for (slot, &(govt, news)) in slots.iter().enumerate() {
            record.news_govt[slot] = govt;
            record.govt_news[slot] = news;
        }
        record.ind_news = ind_news;
        record
    }

    /// A catalog of these crons and the governments, its stellar 128 a
    /// bar of `govt`.
    fn barred(govt: Option<i16>, crons: Vec<CronRecord>) -> FakePilotCatalog {
        FakePilotCatalog {
            sites: vec![
                (
                    SystemId(130),
                    vec![stellar(govt, StellarFlags::CAN_LAND | StellarFlags::BAR)],
                ),
                (SystemId(131), vec![planet(140, 0.0, 0.0)]),
            ],
            govts: govts(),
            crons,
            ..catalog()
        }
    }

    /// A session of `catalog`'s reading `lists`, every crön running,
    /// landed at stellar 128.
    fn landed(catalog: &FakePilotCatalog, lists: Lists) -> Session {
        let mut session = Session::start(catalog)
            .expect("starts")
            .with_strings(Rc::new(lists));
        session.pilot.crons = catalog
            .crons
            .iter()
            .map(|record| (record.id, RUNNING))
            .collect::<BTreeMap<_, _>>();
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        session
    }

    fn opened(session: &mut Session, chance: &mut dyn Chance) -> News {
        assert!(session.open_bar(chance), "a bar");
        session.news().expect("drawn").clone()
    }

    #[test]
    fn government_news_beats_independent_news() {
        let catalog = barred(
            Some(130),
            vec![newsy(128, &[], 15_100), newsy(129, &[(131, 15_000)], 0)],
        );
        let lists = lists()
            .with(15_000, &["Rebels win"])
            .with(15_100, &["Pirates lose"]);
        let mut session = landed(&catalog, lists);
        for roll in [0, 1, 2] {
            let news = opened(&mut session, &mut Scripted::rolling(&[0, roll, 0]));
            assert_eq!(news.story, "Rebels win", "roll {roll}");
        }
    }

    #[test]
    fn independent_news_shows_with_no_government_news() {
        let catalog = barred(Some(130), vec![newsy(128, &[], 15_100)]);
        let mut session = landed(&catalog, lists().with(15_100, &["Pirates lose"]));
        assert_eq!(opened(&mut session, &mut NeverFires).story, "Pirates lose");
    }

    #[test]
    fn the_chosen_crons_story_is_drawn_among_its_lists_strings() {
        let catalog = barred(
            Some(130),
            vec![
                newsy(128, &[(130, 15_000)], 0),
                newsy(129, &[(130, 15_001)], 0),
            ],
        );
        let lists = lists()
            .with(15_000, &["A1", "A2"])
            .with(15_001, &["B1", "B2", "B3"]);
        let mut session = landed(&catalog, lists);
        let mut chance = Scripted::rolling(&[0, 1, 2]);
        assert_eq!(opened(&mut session, &mut chance).story, "B3");
        assert_eq!(chance.sides_asked, [1, 2, 3], "commercial, cron, string");
        let mut chance = Scripted::rolling(&[0, 0, 1]);
        assert_eq!(opened(&mut session, &mut chance).story, "A2");
    }

    #[test]
    fn with_no_candidates_the_story_is_generic_news_else_string_191() {
        let catalog = barred(Some(130), vec![]);
        let mut session = landed(&catalog, lists());
        assert_eq!(
            opened(&mut session, &mut NeverFires).story,
            "Nothing happened"
        );
        let lists = Lists::default().with(MISC_STRINGS, &misc());
        let mut session = landed(&catalog, lists);
        assert_eq!(
            opened(&mut session, &mut NeverFires).story,
            "No news is good news"
        );
        let mut session = landed(&catalog, Lists::default());
        assert_eq!(opened(&mut session, &mut NeverFires).story, "");
    }

    #[test]
    fn generic_news_is_drawn_among_its_strings() {
        let catalog = barred(Some(130), vec![]);
        let lists = Lists::default()
            .with(COMMERCIALS, &["Buy Bibble"])
            .with(GENERIC_NEWS, &["G1", "G2"]);
        let mut session = landed(&catalog, lists);
        let mut chance = Scripted::rolling(&[0, 1]);
        assert_eq!(opened(&mut session, &mut chance).story, "G2");
    }

    #[test]
    fn a_chosen_cron_whose_list_is_missing_or_empty_falls_through_to_generic_news() {
        let catalog = barred(Some(130), vec![newsy(128, &[(130, 15_000)], 0)]);
        let mut session = landed(&catalog, lists());
        assert_eq!(
            opened(&mut session, &mut NeverFires).story,
            "Nothing happened",
            "missing"
        );
        let mut session = landed(&catalog, lists().with(15_000, &[]));
        assert_eq!(
            opened(&mut session, &mut NeverFires).story,
            "Nothing happened",
            "empty"
        );
    }

    #[test]
    fn the_commercial_comes_from_str_8100_else_string_190() {
        let catalog = barred(Some(130), vec![]);
        let lists = Lists::default()
            .with(COMMERCIALS, &["Buy Bibble", "Drink Fizz"])
            .with(MISC_STRINGS, &misc());
        let mut session = landed(&catalog, lists);
        let mut chance = Scripted::rolling(&[1]);
        assert_eq!(opened(&mut session, &mut chance).commercial, "Drink Fizz");
        let mut session = landed(&catalog, Lists::default().with(MISC_STRINGS, &misc()));
        assert_eq!(
            opened(&mut session, &mut NeverFires).commercial,
            "Today's top news:"
        );
        let short = Lists::default().with(MISC_STRINGS, &misc()[..189]);
        let mut session = landed(&catalog, short);
        let news = opened(&mut session, &mut NeverFires);
        assert_eq!(news.commercial, "", "STR# 2002 without #190");
        assert_eq!(news.story, "", "nor #191");
        let mut session = landed(&catalog, Lists::default());
        assert_eq!(opened(&mut session, &mut NeverFires).commercial, "");
    }

    #[test]
    fn the_text_is_kept_raw() {
        let catalog = barred(Some(130), vec![]);
        let lists = Lists::default()
            .with(COMMERCIALS, &["<PN>'s ad"])
            .with(GENERIC_NEWS, &["<PN> wins"]);
        let mut session = landed(&catalog, lists);
        assert_eq!(
            opened(&mut session, &mut NeverFires),
            News {
                commercial: "<PN>'s ad".to_owned(),
                story: "<PN> wins".to_owned(),
            }
        );
    }

    #[test]
    fn the_same_visit_shows_the_same_news_drawing_nothing_more() {
        let catalog = barred(Some(130), vec![]);
        let mut session = landed(&catalog, lists());
        let mut chance = Scripted::rolling(&[]);
        assert!(session.open_bar(&mut chance));
        let asked = chance.sides_asked.len();
        let first = session.news().cloned();
        assert_eq!(session.news().cloned(), first);
        assert_eq!(session.news().cloned(), first);
        assert_eq!(chance.sides_asked.len(), asked, "reading draws nothing");
        assert!(first.is_some());
    }

    #[test]
    fn opening_the_bar_again_draws_afresh() {
        let catalog = barred(Some(130), vec![]);
        let lists = Lists::default()
            .with(COMMERCIALS, &["C1", "C2"])
            .with(GENERIC_NEWS, &["G1", "G2"]);
        let mut session = landed(&catalog, lists);
        let first = opened(&mut session, &mut Scripted::rolling(&[0, 0]));
        let second = opened(&mut session, &mut Scripted::rolling(&[1, 1]));
        assert_eq!(
            (first.commercial.as_str(), first.story.as_str()),
            ("C1", "G1")
        );
        assert_eq!(
            (second.commercial.as_str(), second.story.as_str()),
            ("C2", "G2")
        );
    }

    #[test]
    fn there_is_no_news_without_a_bar() {
        let catalog = FakePilotCatalog {
            sites: vec![
                (
                    SystemId(130),
                    vec![stellar(Some(130), StellarFlags::CAN_LAND)],
                ),
                (SystemId(131), vec![planet(140, 0.0, 0.0)]),
            ],
            ..barred(Some(130), vec![])
        };
        let mut session = landed(&catalog, lists());
        let mut chance = Scripted::rolling(&[]);
        assert!(!session.open_bar(&mut chance));
        assert_eq!(session.news(), None);
        assert_eq!(chance.sides_asked, [] as [u16; 0], "nothing drawn");
        let catalog = barred(Some(130), vec![]);
        let mut session = Session::start(&catalog)
            .expect("starts")
            .with_strings(Rc::new(lists()));
        assert!(!session.open_bar(&mut NeverFires), "in flight");
        assert_eq!(session.news(), None);
    }

    #[test]
    fn taking_off_or_landing_again_forgets_the_news() {
        let catalog = barred(Some(130), vec![]);
        let mut session = landed(&catalog, lists());
        assert!(session.open_bar(&mut NeverFires));
        session.take_off(&NoTraffic, &mut NeverFires);
        assert_eq!(session.news(), None, "took off");
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        assert!(session.open_bar(&mut NeverFires));
        session
            .land_at(&catalog, SystemId(130), StellarId(128))
            .expect("lands");
        assert_eq!(session.news(), None, "landed again");
        assert!(session.open_bar(&mut NeverFires));
        session
            .relocate(&catalog, SystemId(131), StellarId(140))
            .expect("moves");
        assert_eq!(session.news(), None, "moved");
    }

    #[test]
    fn the_days_passing_leave_the_news_alone() {
        let catalog = barred(Some(130), vec![]);
        let mut session = landed(&catalog, lists());
        let news = opened(&mut session, &mut NeverFires);
        session.let_days_pass(3, &mut NeverFires);
        assert_eq!(session.news(), Some(&news));
    }

    #[test]
    fn the_cron_news_reads_the_docked_stellars_government_by_the_rule_set() {
        let catalog = barred(Some(131), vec![newsy(128, &[(130, 15_000)], 0)]);
        let lists = || lists().with(15_000, &["Allied news"]);
        let mut session = landed(&catalog, lists());
        assert_eq!(opened(&mut session, &mut NeverFires).story, "Allied news");
        let rules = Rulebook::default().with_override(RuleKey::NewsGovtMatch, RuleSource::Bible);
        let mut session = landed(&catalog, lists()).with_rules(rules);
        assert_eq!(
            opened(&mut session, &mut NeverFires).story,
            "Nothing happened"
        );
    }

    #[test]
    fn a_cron_in_its_holdoff_gives_no_news() {
        let catalog = barred(Some(130), vec![newsy(128, &[(130, 15_000)], 0)]);
        let mut session = landed(&catalog, lists().with(15_000, &["Held news"]));
        session.pilot.crons.insert(
            CronId(128),
            CronState {
                days: 5,
                holdoff: 2,
            },
        );
        assert_eq!(
            opened(&mut session, &mut NeverFires).story,
            "Nothing happened"
        );
    }
}
