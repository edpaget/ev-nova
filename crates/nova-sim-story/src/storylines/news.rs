//! How the new game's news spreads: from the system it breaks in, one gate
//! lane further each day.
//!
//! A news item breaks on the day a mission's effects release it (kept in
//! [`StoryState::news`]) in its `from` system, and travels on ships: it
//! reaches a system [`NEWS_DAYS_PER_LANE`] days for each lane between them,
//! over the shortest way. A closed lane, unstable or dead, carries
//! nothing, as no ship crosses it, and a one-way lane carries news only
//! its own way.
//!
//! The lanes are the galaxy's ([`GateLanes`]), which follow play's
//! changes to the jump points, so they change at run time. Each item's
//! reach is frozen at its release, over the lanes then ([`freeze`], kept
//! in [`StoryState::reach`]), so a system that has heard an item never
//! "unhears" it when a lane closes, and an item released once a lane has
//! closed never crosses it. The session freezes after it hands the galaxy
//! an effect block's point changes, so an item released in the block that
//! closes a road never crosses that road. What a system has heard is
//! worked out from the release days and the reaches whenever it is asked
//! ([`heard`]).
//!
//! What the pilot has read is the pilot's, not a system's
//! ([`StoryState::read`]): an item becomes read when the player closes a
//! news log listing it in a system that has heard it ([`mark_read`]), and
//! each heard item says whether it is read, so the log opens at landing
//! only when something unread is heard.

use std::collections::{BTreeMap, VecDeque};

use nova_sim_catalog::storylines::Storylines;

use super::state::StoryState;
use crate::catalog::SystemId;
use crate::gate_lanes::GateLanes;

/// The days news takes to cross one gate lane: engine tuning, as the
/// format leaves news's speed open.
pub const NEWS_DAYS_PER_LANE: u32 = 1;

/// A news item a system has heard.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HeardNews {
    /// Its id.
    pub id: String,
    /// Its headline.
    pub headline: String,
    /// The news as told.
    pub text: String,
    /// The system where it broke.
    pub from: SystemId,
    /// How many days ago the system heard it.
    pub age: u32,
    /// Whether the pilot has read it ([`StoryState::read`]), here or in
    /// any other system.
    pub read: bool,
}

/// How many lanes news from `from` crosses to reach each system it
/// reaches, `from` itself at 0: a breadth-first walk over `lanes`' open
/// lanes ([`GateLanes::listed`]), as no ship crosses a closed one.
#[must_use]
pub fn arrivals(lanes: &GateLanes, from: SystemId) -> BTreeMap<SystemId, u32> {
    let mut reached = BTreeMap::from([(from, 0)]);
    let mut queue = VecDeque::from([from]);
    while let Some(at) = queue.pop_front() {
        let crossed = reached[&at] + 1;
        for &to in lanes.listed(at) {
            if reached.contains_key(&to) {
                continue;
            }
            reached.insert(to, crossed);
            queue.push_back(to);
        }
    }
    reached
}

/// Freezes the reach of each news item `story` has released and keeps no
/// reach for: its [`arrivals`] over `lanes` as they are now, kept in
/// [`StoryState::reach`]. An item with a reach keeps it, and one the data
/// lacks gets none.
pub fn freeze(storylines: &Storylines, story: &mut StoryState, lanes: &GateLanes) {
    for id in story.news.keys() {
        if story.reach.contains_key(id) {
            continue;
        }
        if let Some(item) = storylines.news.get(id) {
            story.reach.insert(id.clone(), arrivals(lanes, item.from));
        }
    }
}

/// The news `system` has heard by `story`'s day, over each item's frozen
/// reach ([`freeze`]), by the day each broke and then by id. An item with
/// no reach is heard nowhere.
#[must_use]
pub fn heard(storylines: &Storylines, story: &StoryState, system: SystemId) -> Vec<HeardNews> {
    let mut released: Vec<(&String, u32)> = story.news.iter().map(|(id, &day)| (id, day)).collect();
    released.sort_by_key(|&(id, day)| (day, id));
    released
        .into_iter()
        .filter_map(|(id, day)| {
            let item = storylines.news.get(id)?;
            let lanes = *story.reach.get(id)?.get(&system)?;
            let arrives = day.saturating_add(lanes.saturating_mul(NEWS_DAYS_PER_LANE));
            let age = story.day.checked_sub(arrives)?;
            Some(HeardNews {
                id: id.clone(),
                headline: item.headline.clone(),
                text: item.text.clone(),
                from: item.from,
                age,
                read: story.read.contains(id),
            })
        })
        .collect()
}

/// Marks read each of `ids` that `system` has [`heard`] and the pilot
/// hasn't read yet, in [`StoryState::read`]: the player closed a news log
/// listing them there. An id `system` hasn't heard (not released, not
/// arrived yet, or not in the data) is left alone. Whether it marked any.
pub fn mark_read(
    storylines: &Storylines,
    story: &mut StoryState,
    system: SystemId,
    ids: &[String],
) -> bool {
    let unread: Vec<String> = heard(storylines, story, system)
        .into_iter()
        .filter(|item| !item.read && ids.contains(&item.id))
        .map(|item| item.id)
        .collect();
    let marked = !unread.is_empty();
    story.read.extend(unread);
    marked
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use nova_sim_catalog::storylines::NewsItem;

    use super::*;
    use crate::catalog::GateStability;

    const HEARTH: i16 = 1;
    const VERGE: i16 = 2;
    const DRIFT: i16 = 3;
    const FAR: i16 = 4;

    /// Lanes shaped like the new-game fixture's: Hearth and Verge both
    /// ways, Hearth and Drift both ways, Verge to Drift dead, Drift to
    /// Verge unstable (its back door), and Far reached by nothing.
    fn lanes() -> GateLanes {
        use GateStability::{Dead, Stable, Unstable};
        GateLanes::fixture(&[
            (HEARTH, VERGE, Stable),
            (VERGE, HEARTH, Stable),
            (HEARTH, DRIFT, Stable),
            (DRIFT, HEARTH, Stable),
            (VERGE, DRIFT, Dead),
            (DRIFT, VERGE, Unstable),
        ])
    }

    fn storylines() -> Storylines {
        let item = |headline: &str, from: i16| NewsItem {
            headline: headline.to_owned(),
            text: format!("{headline}, told."),
            from: SystemId(from),
        };
        Storylines {
            news: BTreeMap::from([
                ("closed".to_owned(), item("Closed", HEARTH)),
                ("plate".to_owned(), item("Plate", VERGE)),
                ("drift".to_owned(), item("Drift", DRIFT)),
            ]),
            ..Storylines::default()
        }
    }

    fn story(day: u32, released: &[(&str, u32)]) -> StoryState {
        StoryState {
            day,
            news: released
                .iter()
                .map(|&(id, day)| (id.to_owned(), day))
                .collect(),
            ..StoryState::default()
        }
    }

    /// [`story`] with every item frozen over [`lanes`].
    fn frozen(day: u32, released: &[(&str, u32)]) -> StoryState {
        let mut story = story(day, released);
        freeze(&storylines(), &mut story, &lanes());
        story
    }

    /// What `system` has heard on `day`, as (id, age).
    fn heard_in(system: i16, day: u32, released: &[(&str, u32)]) -> Vec<(String, u32)> {
        heard_by(&frozen(day, released), system)
    }

    /// What `system` has heard of `story`, as (id, age).
    fn heard_by(story: &StoryState, system: i16) -> Vec<(String, u32)> {
        heard(&storylines(), story, SystemId(system))
            .into_iter()
            .map(|news| (news.id, news.age))
            .collect()
    }

    fn ids(heard: &[(String, u32)]) -> Vec<&str> {
        heard.iter().map(|(id, _)| id.as_str()).collect()
    }

    #[test]
    fn news_is_heard_where_it_breaks_on_its_day() {
        assert_eq!(
            heard_in(HEARTH, 3, &[("closed", 3)]),
            [("closed".to_owned(), 0)]
        );
        assert_eq!(heard_in(HEARTH, 2, &[("closed", 3)]), []);
    }

    #[test]
    fn news_travels_a_lane_a_day() {
        assert_eq!(heard_in(VERGE, 3, &[("closed", 3)]), []);
        assert_eq!(
            heard_in(VERGE, 4, &[("closed", 3)]),
            [("closed".to_owned(), 0)]
        );
        assert_eq!(
            heard_in(DRIFT, 4, &[("closed", 3)]),
            [("closed".to_owned(), 0)]
        );
        assert_eq!(
            heard_in(VERGE, 9, &[("closed", 3)]),
            [("closed".to_owned(), 5)]
        );
    }

    #[test]
    fn a_dead_lane_carries_nothing_but_a_way_round_does() {
        assert_eq!(
            arrivals(&lanes(), SystemId(VERGE)),
            BTreeMap::from([
                (SystemId(VERGE), 0),
                (SystemId(HEARTH), 1),
                (SystemId(DRIFT), 2),
            ])
        );
        assert_eq!(heard_in(DRIFT, 6, &[("plate", 5)]), []);
        assert_eq!(
            heard_in(DRIFT, 7, &[("plate", 5)]),
            [("plate".to_owned(), 0)]
        );
    }

    #[test]
    fn an_unstable_lane_carries_nothing() {
        assert_eq!(
            arrivals(&lanes(), SystemId(DRIFT)),
            BTreeMap::from([
                (SystemId(DRIFT), 0),
                (SystemId(HEARTH), 1),
                (SystemId(VERGE), 2),
            ]),
            "round by hearth, not by the back door"
        );
        let lanes = GateLanes::fixture(&[(DRIFT, VERGE, GateStability::Unstable)]);
        assert_eq!(
            arrivals(&lanes, SystemId(DRIFT)),
            BTreeMap::from([(SystemId(DRIFT), 0)])
        );
    }

    #[test]
    fn a_one_way_lane_carries_one_way() {
        let lanes = GateLanes::fixture(&[(DRIFT, VERGE, GateStability::Stable)]);
        assert_eq!(
            arrivals(&lanes, SystemId(DRIFT)),
            BTreeMap::from([(SystemId(DRIFT), 0), (SystemId(VERGE), 1)])
        );
        assert_eq!(
            arrivals(&lanes, SystemId(VERGE)),
            BTreeMap::from([(SystemId(VERGE), 0)])
        );
    }

    #[test]
    fn an_unreachable_system_never_hears() {
        assert_eq!(heard_in(FAR, 1000, &[("closed", 0), ("plate", 0)]), []);
    }

    #[test]
    fn news_not_released_or_not_in_the_data_is_not_heard() {
        assert_eq!(heard_in(HEARTH, 10, &[]), []);
        assert_eq!(heard_in(HEARTH, 10, &[("gone", 0)]), []);
    }

    #[test]
    fn heard_news_is_by_release_day_then_id() {
        let heard = heard_in(HEARTH, 20, &[("plate", 4), ("drift", 4), ("closed", 7)]);
        assert_eq!(ids(&heard), ["drift", "plate", "closed"]);
    }

    #[test]
    fn heard_news_carries_its_item_and_its_age() {
        let news = heard(&storylines(), &frozen(10, &[("plate", 5)]), SystemId(DRIFT));
        assert_eq!(
            news,
            [HeardNews {
                id: "plate".to_owned(),
                headline: "Plate".to_owned(),
                text: "Plate, told.".to_owned(),
                from: SystemId(VERGE),
                age: 3,
                read: false,
            }]
        );
    }

    #[test]
    fn freezing_keeps_each_released_items_reach_over_the_lanes_then() {
        let story = frozen(3, &[("closed", 3), ("gone", 1)]);
        assert_eq!(
            story.reach,
            BTreeMap::from([("closed".to_owned(), arrivals(&lanes(), SystemId(HEARTH)))]),
            "an item the data lacks has no reach"
        );
    }

    #[test]
    fn an_item_frozen_before_a_lane_closes_is_still_heard_beyond_it() {
        let mut story = frozen(3, &[("closed", 3)]);
        // Hearth's lanes all close; the item, frozen, still reaches verge.
        let closed = GateLanes::fixture(&[(VERGE, HEARTH, GateStability::Stable)]);
        story.news.insert("plate".to_owned(), 3);
        freeze(&storylines(), &mut story, &closed);
        story.day = 4;
        assert_eq!(
            heard_by(&story, VERGE),
            [("closed".to_owned(), 0), ("plate".to_owned(), 1)]
        );
        assert_eq!(
            heard_by(&story, HEARTH),
            [("closed".to_owned(), 1), ("plate".to_owned(), 0)],
            "plate, released after, crosses verge's open lane"
        );
        assert_eq!(heard_by(&story, DRIFT), [("closed".to_owned(), 0)]);
    }

    #[test]
    fn an_item_released_after_a_lane_closes_is_not_heard_beyond_it() {
        let mut story = story(5, &[("closed", 4)]);
        freeze(
            &storylines(),
            &mut story,
            &GateLanes::fixture(&[(HEARTH, VERGE, GateStability::Dead)]),
        );
        assert_eq!(heard_by(&story, HEARTH), [("closed".to_owned(), 1)]);
        assert_eq!(heard_by(&story, VERGE), []);
    }

    #[test]
    fn freezing_leaves_a_kept_reach_alone() {
        let mut story = frozen(3, &[("closed", 3)]);
        let kept = story.reach.clone();
        freeze(&storylines(), &mut story, &GateLanes::default());
        assert_eq!(story.reach, kept);
    }

    #[test]
    fn an_item_not_frozen_is_heard_nowhere() {
        assert_eq!(heard_by(&story(9, &[("closed", 3)]), HEARTH), []);
    }

    #[test]
    fn a_lane_is_a_day() {
        assert_eq!(NEWS_DAYS_PER_LANE, 1);
    }

    /// What `system` has heard of `story`, as (id, read).
    fn read_flags(story: &StoryState, system: i16) -> Vec<(String, bool)> {
        heard(&storylines(), story, SystemId(system))
            .into_iter()
            .map(|news| (news.id, news.read))
            .collect()
    }

    #[test]
    fn heard_news_is_read_exactly_when_the_story_holds_it_read() {
        let mut story = frozen(9, &[("closed", 3), ("plate", 4)]);
        assert_eq!(
            read_flags(&story, HEARTH),
            [("closed".to_owned(), false), ("plate".to_owned(), false)]
        );
        story.read.insert("plate".to_owned());
        assert_eq!(
            read_flags(&story, HEARTH),
            [("closed".to_owned(), false), ("plate".to_owned(), true)]
        );
    }

    #[test]
    fn marking_a_heard_item_reads_it_once() {
        let mut story = frozen(9, &[("closed", 3)]);
        let ids = ["closed".to_owned()];
        assert!(mark_read(&storylines(), &mut story, SystemId(HEARTH), &ids));
        assert_eq!(story.read, BTreeSet::from(["closed".to_owned()]));
        assert!(
            !mark_read(&storylines(), &mut story, SystemId(HEARTH), &ids),
            "already read"
        );
        assert_eq!(story.read, BTreeSet::from(["closed".to_owned()]));
    }

    #[test]
    fn marking_marks_only_what_the_system_has_heard() {
        // closed breaks in hearth on day 3 and reaches verge on day 4;
        // plate isn't released; gone isn't in the data.
        let mut story = frozen(3, &[("closed", 3), ("gone", 1)]);
        let ids = ["closed", "plate", "gone"].map(str::to_owned);
        assert!(
            !mark_read(&storylines(), &mut story, SystemId(VERGE), &ids),
            "nothing has arrived in verge yet"
        );
        assert_eq!(story.read, BTreeSet::new());
        assert!(mark_read(&storylines(), &mut story, SystemId(HEARTH), &ids));
        assert_eq!(
            story.read,
            BTreeSet::from(["closed".to_owned()]),
            "only the heard item is marked"
        );
    }

    #[test]
    fn an_item_read_in_one_system_is_read_when_heard_in_another() {
        let mut story = frozen(3, &[("closed", 3)]);
        assert_eq!(read_flags(&story, VERGE), [], "not in verge yet");
        assert!(mark_read(
            &storylines(),
            &mut story,
            SystemId(HEARTH),
            &["closed".to_owned()]
        ));
        story.day = 4;
        assert_eq!(read_flags(&story, VERGE), [("closed".to_owned(), true)]);
    }
}
