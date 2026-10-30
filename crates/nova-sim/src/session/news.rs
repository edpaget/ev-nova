//! The bar's news: the session's side of [`news`](crate::news).
//!
//! **Drawing it.** As the bar opens at a stellar with one
//! ([`Session::open_bar`]), the session draws the visit's news, as
//! `_RandomizeNewsDialog` (0x48238) does when `_DoBarDialog` opens
//! (@0x48f15), in this order:
//!
//! 1. The commercial line: a string of `STR#`
//!    [`COMMERCIALS`] (@0x4826a), else, with none or an empty one
//!    drawn (@0x4826f), `STR#` 2002 #190 (@0x48294), else nothing.
//! 2. The story starts as the `öops` story: with any price event in
//!    [`oops_pool`] (the local ones, else every active one), one of them,
//!    each equally likely (@0x482ff, @0x4833a), told by [`oops_story`]
//!    from `STR#` 2002's pieces ([`StoryWords`]) and the names; with
//!    none, empty and nothing drawn.
//! 3. With any government candidates ([`candidates`]), one of them, else
//!    with any independent candidates one of those, each equally likely
//!    (@0x486e2, @0x48704); that cron's `STR#` string **replaces** the
//!    story, empty when its list is missing or empty (@0x72f0f).
//! 4. With the story empty, a string of `STR#`
//!    [`GENERIC_NEWS`] (@0x48763), else, with none or an empty one
//!    drawn (@0x48768), `STR#` 2002 #191 (@0x4878d), else nothing.
//!
//! Each string is drawn uniformly among its list's, and every list is
//! read through the session's string lists ([`Session::with_strings`]).
//! The text is kept raw, its tags not filled, as `_NewsDialogUpdate`
//! draws it.
//!
//! **Its picture.** The news is drawn over the `PICT` of the docked
//! stellar's government ([`Session::news_picture`]), read when News is
//! pressed, as `_DoNewsDialog` does (@0x47f3b-0x47f98): the stellar's
//! current (overlaid) government, so a change of owner since the bar
//! opened shows. That government's `NewsPic`, else 9000 for an
//! independent stellar or a `NewsPic` of -1 ([`news_picture`]); the
//! fallback for a missing picture is the view's, which knows the
//! pictures.
//!
//! **Keeping it.** The visit's news ([`Session::news`]) is shown again
//! each time News is pressed, unchanged, until the bar opens again,
//! which draws afresh. It is forgotten as the ship's docking changes (a
//! landing, a move, a take-off), and never saved, as the original's
//! `_newsString` globals are not. The draws are made as the bar opens,
//! before anything can show them, so no ordering boundary is crossed.
//! News has no participant in the daily step and no trigger point of its
//! own.

use super::{Chart, Port, Session, Statics};
use crate::catalog::StellarId;
use crate::chance::Chance;
use crate::market;
use crate::news::{
    COMMERCIALS, GENERIC_NEWS, MISC_STRINGS, NO_NEWS, News, NewsFacts, OopsFacts, StoryWords,
    TOP_NEWS, candidates, news_picture, oops_pool, oops_story,
};

impl Session {
    /// Opens the bar of the stellar the ship is docked at: draws the
    /// visit's news on `chance` (see the module docs) and keeps it, and
    /// gives true. False, and nothing changes, when the ship has not
    /// landed at a stellar with a bar. It first saves the nav, as
    /// `_DoBarDialog` does (@0x48efe-0x48f11), for the bar's map to put
    /// back ([`Session::restore_bar_nav`]), and forgets the last race
    /// winner (@0x48f61-0x48f6c; see the `gamble` module).
    pub fn open_bar(&mut self, chance: &mut dyn Chance) -> bool {
        let Some(site) = self.chart.bar_site(self.mode.landed()) else {
            return false;
        };
        self.chart.bar_nav = Some(self.chart.nav);
        self.port.reset_race();
        let commercial = self
            .setup
            .pick_string(COMMERCIALS, chance)
            .or_else(|| self.setup.nth_string(MISC_STRINGS, TOP_NEWS))
            .unwrap_or_default();
        let mut story = self.disaster_story(site.id, chance);
        let found = candidates().of(&NewsFacts {
            crons: &self.statics.crons,
            states: &self.pilot.crons,
            stellar_govt: site.govt,
            govts: &self.statics.govts,
            rules: &self.setup.rules,
        });
        let pool = if found.government.is_empty() {
            found.independent
        } else {
            found.government
        };
        if !pool.is_empty() {
            let (_, list) = pool[pick(pool.len(), chance)];
            story = self.setup.random_string(list, chance).unwrap_or_default();
        }
        if story.is_empty() {
            story = self
                .setup
                .pick_string(GENERIC_NEWS, chance)
                .or_else(|| self.setup.nth_string(MISC_STRINGS, NO_NEWS))
                .unwrap_or_default();
        }
        self.port.bar_news = Some(News { commercial, story });
        true
    }

    /// The `PICT` the news is drawn over (see the module docs): `None`
    /// unless the ship is docked at a stellar with a bar.
    #[must_use]
    pub fn news_picture(&self) -> Option<i16> {
        self.chart.news_picture(&self.statics, self.mode.landed())
    }

    /// The news of the bar visit under way, as it was drawn: `None`
    /// before the bar opens, and after the ship's docking changes.
    #[must_use]
    pub fn news(&self) -> Option<&News> {
        self.port.news()
    }

    /// The `öops` story at `docked`, the story the cron news replaces
    /// (`_RandomizeNewsDialog` @0x482a4-0x48586): one of the
    /// [`oops_pool`], drawn on `chance`, told by [`oops_story`]. Empty,
    /// and nothing drawn, with no event active; empty when the event
    /// drawn has no story.
    fn disaster_story(&self, docked: StellarId, chance: &mut dyn Chance) -> String {
        let pool = oops_pool().of(&OopsFacts {
            events: &self.pilot.events,
            disasters: &self.statics.goods.disasters,
            docked,
            rules: &self.setup.rules,
        });
        if pool.is_empty() {
            return String::new();
        }
        let id = pool[pick(pool.len(), chance)];
        let (Some(&active), Some(record)) = (
            self.pilot.events.get(&id),
            self.statics
                .goods
                .disasters
                .iter()
                .find(|record| record.id == id),
        ) else {
            return String::new();
        };
        let words = StoryWords::read(&*self.setup.strings.0);
        oops_story(record, market::place(active, record), &words, self).unwrap_or_default()
    }
}

impl Chart {
    /// The `PICT` the news is drawn over (see the module docs): `None`
    /// unless the ship is docked at a stellar with a bar.
    #[must_use]
    fn news_picture(&self, statics: &Statics, landed: Option<StellarId>) -> Option<i16> {
        let site = self.bar_site(landed)?;
        Some(news_picture(site.govt.and_then(|id| statics.govts.get(id))))
    }
}

impl Port {
    /// The news of the bar visit under way, as it was drawn: `None`
    /// before the bar opens, and after the ship's docking changes.
    #[must_use]
    fn news(&self) -> Option<&News> {
        self.bar_news.as_ref()
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
        CommCatalog, CronId, CronRecord, DisasterId, DisasterRecord, GovtId, GovtRecord,
        LandingSite, StellarId, SystemId,
    };
    use crate::chance::NeverFires;
    use crate::cron::CronState;
    use crate::landing::StellarFlags;
    use crate::market::ActiveEvent;
    use crate::news::CARGO_NAMES;
    use crate::rulebook::{RuleKey, RuleSource, Rulebook};
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

    /// `STR#` 2002 with its #190 and #191, and the price story's #60,
    /// #181 and #192-#194.
    fn misc() -> Vec<&'static str> {
        let mut strings = vec!["-"; 194];
        strings[59] = "on";
        strings[180] = "the price of";
        strings[189] = "Today's top news:";
        strings[190] = "No news is good news";
        strings[191] = "has";
        strings[192] = "raised";
        strings[193] = "lowered";
        strings
    }

    /// The usual lists: one commercial, one generic story, `STR#` 2002
    /// and the cargo names.
    fn lists() -> Lists {
        Lists::default()
            .with(COMMERCIALS, &["Buy Bibble"])
            .with(GENERIC_NEWS, &["Nothing happened"])
            .with(MISC_STRINGS, &misc())
            .with(CARGO_NAMES, &["food", "industrial"])
    }

    /// Govt 130 (class 1, allied with class 2, news picture 9002) and
    /// 131 (class 2, no news picture).
    fn govts() -> Vec<GovtRecord> {
        vec![
            GovtRecord {
                classes: [1, -1, -1, -1],
                allies: [2, -1, -1, -1],
                news_pic: 9002,
                ..govt(130)
            },
            GovtRecord {
                classes: [2, -1, -1, -1],
                news_pic: -1,
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
    fn an_empty_commercial_or_generic_story_drawn_falls_back_to_string_190_or_191() {
        let catalog = barred(Some(130), vec![]);
        let lists = Lists::default()
            .with(COMMERCIALS, &["", "Drink Fizz"])
            .with(GENERIC_NEWS, &["", "G2"])
            .with(MISC_STRINGS, &misc());
        let mut session = landed(&catalog, lists);
        let mut chance = Scripted::rolling(&[0, 0]);
        let news = opened(&mut session, &mut chance);
        assert_eq!(news.commercial, "Today's top news:", "#190");
        assert_eq!(news.story, "No news is good news", "#191");
        assert_eq!(chance.sides_asked, [2, 2], "commercial, generic story");
    }

    #[test]
    fn an_empty_cron_story_drawn_falls_through_to_generic_news() {
        let catalog = barred(Some(130), vec![newsy(128, &[(130, 15_000)], 0)]);
        let mut session = landed(&catalog, lists().with(15_000, &["", "Cron news"]));
        let mut chance = Scripted::rolling(&[0, 0, 0, 0]);
        assert_eq!(opened(&mut session, &mut chance).story, "Nothing happened");
        assert_eq!(chance.sides_asked, [1, 1, 2, 1], "then the generic story");
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
        session.take_off(&catalog, &mut NeverFires);
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

    /// A food event `id` named `name`, its record at `stellar`.
    fn oops(id: i16, name: &str, stellar: i16) -> DisasterRecord {
        DisasterRecord {
            id: DisasterId(id),
            name: name.to_owned(),
            stellar,
            price_delta: 15,
            duration: 10,
            ..DisasterRecord::default()
        }
    }

    /// [`barred`] with these crons and `öops`, stellar 128 "Earth" and
    /// 140 "Mars".
    fn priced(crons: Vec<CronRecord>, disasters: Vec<DisasterRecord>) -> FakePilotCatalog {
        let mut catalog = barred(Some(130), crons);
        catalog.disasters = disasters;
        catalog.place_names.stellars = BTreeMap::from([
            (StellarId(128), "Earth".to_owned()),
            (StellarId(140), "Mars".to_owned()),
        ]);
        catalog
    }

    /// [`landed`], with these events active (ID, stellar, days left).
    fn eventful(catalog: &FakePilotCatalog, lists: Lists, events: &[(i16, i16, u16)]) -> Session {
        let mut session = landed(catalog, lists);
        session.pilot.events = events
            .iter()
            .map(|&(id, stellar, days)| {
                let stellar = Some(StellarId(stellar));
                (DisasterId(id), ActiveEvent { days, stellar })
            })
            .collect();
        session
    }

    #[test]
    fn the_bar_tells_an_active_local_price_event() {
        let catalog = priced(vec![], vec![oops(130, "A drought", 128)]);
        let mut session = eventful(&catalog, lists(), &[(130, 128, 5)]);
        let mut chance = Scripted::rolling(&[]);
        assert_eq!(
            opened(&mut session, &mut chance).story,
            "Today's top news: A drought has raised the price of food on Earth."
        );
        assert_eq!(
            chance.sides_asked,
            [1, 1],
            "commercial, the event; no generic story"
        );
    }

    #[test]
    fn a_local_event_is_chosen_over_one_elsewhere() {
        let catalog = priced(
            vec![],
            vec![oops(129, "A flood", 140), oops(130, "A drought", 128)],
        );
        let mut session = eventful(&catalog, lists(), &[(129, 140, 5), (130, 128, 5)]);
        for roll in [0, 1] {
            let news = opened(&mut session, &mut Scripted::rolling(&[0, roll]));
            assert_eq!(
                news.story, "Today's top news: A drought has raised the price of food on Earth.",
                "roll {roll}"
            );
        }
        let mut session = eventful(&catalog, lists(), &[(129, 140, 5)]);
        assert_eq!(
            opened(&mut session, &mut NeverFires).story,
            "Today's top news: A flood has raised the price of food on Mars.",
            "with none local, one elsewhere"
        );
    }

    #[test]
    fn among_several_local_events_the_roll_picks() {
        let catalog = priced(
            vec![],
            vec![oops(130, "A drought", 128), oops(131, "A glut", 128)],
        );
        let mut session = eventful(&catalog, lists(), &[(130, 128, 5), (131, 128, 5)]);
        let mut chance = Scripted::rolling(&[0, 1]);
        assert_eq!(
            opened(&mut session, &mut chance).story,
            "Today's top news: A glut has raised the price of food on Earth."
        );
        assert_eq!(chance.sides_asked, [1, 2], "commercial, the event");
        let mut chance = Scripted::rolling(&[0, 0]);
        assert_eq!(
            opened(&mut session, &mut chance).story,
            "Today's top news: A drought has raised the price of food on Earth."
        );
    }

    #[test]
    fn with_no_active_event_no_draw_is_made_for_the_story() {
        let catalog = priced(vec![], vec![oops(130, "A drought", 128)]);
        let mut session = eventful(&catalog, lists(), &[(130, 128, 1)]);
        let mut chance = Scripted::rolling(&[]);
        assert_eq!(opened(&mut session, &mut chance).story, "Nothing happened");
        assert_eq!(chance.sides_asked, [1, 1], "commercial, generic story");
    }

    #[test]
    fn cron_news_replaces_the_price_story() {
        let catalog = priced(
            vec![newsy(128, &[], 15_100)],
            vec![oops(130, "A drought", 128)],
        );
        let lists = lists().with(15_100, &["Pirates lose"]);
        let mut session = eventful(&catalog, lists, &[(130, 128, 5)]);
        let mut chance = Scripted::rolling(&[]);
        assert_eq!(opened(&mut session, &mut chance).story, "Pirates lose");
        assert_eq!(
            chance.sides_asked,
            [1, 1, 1, 1],
            "commercial, event, cron, string"
        );
    }

    #[test]
    fn an_event_with_no_story_falls_through_to_generic_news() {
        let negative = DisasterRecord {
            commodity: -1,
            ..oops(130, "A drought", 128)
        };
        let catalog = priced(vec![], vec![negative]);
        let mut session = eventful(&catalog, lists(), &[(130, 128, 5)]);
        assert_eq!(
            opened(&mut session, &mut NeverFires).story,
            "Nothing happened"
        );
    }

    #[test]
    fn the_news_picture_is_the_bars_governments() {
        let catalog = barred(Some(130), vec![]);
        let session = landed(&catalog, lists());
        assert_eq!(session.news_picture(), Some(9002));
        let catalog = barred(None, vec![]);
        let session = landed(&catalog, lists());
        assert_eq!(session.news_picture(), Some(9000), "independent");
    }

    #[test]
    fn there_is_no_news_picture_away_from_a_bar() {
        let catalog = barred(Some(130), vec![]);
        let session = Session::start(&catalog).expect("starts");
        assert_eq!(session.news_picture(), None, "not landed");
        let mut session = landed(&catalog, lists());
        session.mode = crate::session::Mode::Landed(StellarId(140));
        assert_eq!(session.news_picture(), None, "no bar");
    }

    #[test]
    fn the_news_picture_follows_the_stellars_owner_when_news_is_pressed() {
        let catalog = barred(Some(130), vec![]);
        let mut session = landed(&catalog, lists());
        assert!(session.open_bar(&mut NeverFires));
        session.change_galaxy(&catalog, |overlay| {
            overlay.set_stellar_govt(StellarId(128), Some(Some(GovtId(131))));
        });
        assert_eq!(session.news_picture(), Some(9000), "131's NewsPic is -1");
    }

    #[test]
    fn the_bars_map_puts_back_the_nav_the_bar_opened_with() {
        use crate::navigation::NavMode;
        let catalog = barred(Some(130), Vec::new());
        let mut session = landed(&catalog, lists());
        let opening = session.nav_mode();
        assert_eq!(opening, NavMode::Stellar(Some(StellarId(128))), "landed");
        assert!(session.open_bar(&mut NeverFires));

        session.plot_course(SystemId(131)).expect("a route");
        assert_eq!(session.nav_mode(), NavMode::Hyperspace(Some(SystemId(131))));
        session.restore_bar_nav();
        assert_eq!(session.nav_mode(), opening, "put back");
        assert_eq!(session.course_shown(), [SystemId(131)], "the route kept");

        session.plot_course(SystemId(131)).expect("a route");
        session.restore_bar_nav();
        assert_eq!(session.nav_mode(), opening, "put back again");
    }

    #[test]
    fn taking_off_or_landing_again_forgets_the_bars_nav() {
        let catalog = barred(Some(130), vec![]);
        let mut session = landed(&catalog, lists());
        assert!(session.open_bar(&mut NeverFires));
        session.take_off(&catalog, &mut NeverFires);
        assert_eq!(session.chart.bar_nav, None, "took off");
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        assert!(session.open_bar(&mut NeverFires));
        session
            .land_at(&catalog, SystemId(130), StellarId(128))
            .expect("lands");
        assert_eq!(session.chart.bar_nav, None, "landed again");
        assert!(session.open_bar(&mut NeverFires));
        session
            .relocate(&catalog, SystemId(131), StellarId(140))
            .expect("moves");
        assert_eq!(session.chart.bar_nav, None, "moved");
    }

    #[test]
    fn without_a_bar_opened_the_bars_restore_changes_nothing() {
        use crate::navigation::NavMode;
        let catalog = barred(Some(130), Vec::new());
        let mut session = landed(&catalog, lists());
        session.plot_course(SystemId(131)).expect("a route");
        session.restore_bar_nav();
        assert_eq!(session.nav_mode(), NavMode::Hyperspace(Some(SystemId(131))));
    }
}
