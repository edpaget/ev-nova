//! How the new game's news spreads: from the system it breaks in, one gate
//! lane further each day.
//!
//! A news item breaks on the day a mission's effects release it (kept in
//! [`StoryState::news`]) in its `from` system, and travels on ships: it
//! reaches a system [`NEWS_DAYS_PER_LANE`] days for each lane between them,
//! over the shortest way. A dead lane carries nothing, and a one-way lane
//! carries news only its own way. Nothing is stored per system: what a
//! system has heard is worked out from the release days and the lanes
//! whenever it is asked ([`heard`]).
//!
//! The lanes are the galaxy's as the data writes them
//! ([`GateLanes`]); play's changes to a jump point don't reach them yet
//! (`play-the-sample` phase 1). Once they do, a lane closing could make a
//! system "unhear" an item; freezing each item's arrivals at its release
//! would then be the fix.

use std::collections::{BTreeMap, VecDeque};

use nova_sim_catalog::storylines::Storylines;

use super::state::StoryState;
use crate::catalog::{GateStability, SystemId};
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
}

/// How many lanes news from `from` crosses to reach each system it
/// reaches, `from` itself at 0: a breadth-first walk over `lanes`, never
/// over a dead lane.
#[must_use]
pub fn arrivals(lanes: &GateLanes, from: SystemId) -> BTreeMap<SystemId, u32> {
    let mut reached = BTreeMap::from([(from, 0)]);
    let mut queue = VecDeque::from([from]);
    while let Some(at) = queue.pop_front() {
        let crossed = reached[&at] + 1;
        for &to in lanes.listed(at) {
            if lanes.stability(at, to) == Some(GateStability::Dead) || reached.contains_key(&to) {
                continue;
            }
            reached.insert(to, crossed);
            queue.push_back(to);
        }
    }
    reached
}

/// The news `system` has heard by `story`'s day, over `lanes`, by the day
/// each broke and then by id.
#[must_use]
pub fn heard(
    storylines: &Storylines,
    story: &StoryState,
    lanes: &GateLanes,
    system: SystemId,
) -> Vec<HeardNews> {
    let mut released: Vec<(&String, u32)> = story.news.iter().map(|(id, &day)| (id, day)).collect();
    released.sort_by_key(|&(id, day)| (day, id));
    released
        .into_iter()
        .filter_map(|(id, day)| {
            let item = storylines.news.get(id)?;
            let lanes = *arrivals(lanes, item.from).get(&system)?;
            let arrives = day.saturating_add(lanes.saturating_mul(NEWS_DAYS_PER_LANE));
            let age = story.day.checked_sub(arrives)?;
            Some(HeardNews {
                id: id.clone(),
                headline: item.headline.clone(),
                text: item.text.clone(),
                from: item.from,
                age,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use nova_sim_catalog::storylines::NewsItem;

    use super::*;

    const HEARTH: i16 = 1;
    const VERGE: i16 = 2;
    const DRIFT: i16 = 3;
    const FAR: i16 = 4;

    /// The new-game fixture's lanes: Hearth and Verge both ways, Hearth
    /// and Drift both ways, Verge to Drift dead, Drift to Verge unstable
    /// (its back door), and Far reached by nothing.
    fn lanes() -> GateLanes {
        use GateStability::{Dead, Stable, Unstable};
        GateLanes::fixture(&[
            (HEARTH, VERGE, Stable),
            (VERGE, HEARTH, Unstable),
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

    /// What `system` has heard on `day`, as (id, age).
    fn heard_in(system: i16, day: u32, released: &[(&str, u32)]) -> Vec<(String, u32)> {
        heard(
            &storylines(),
            &story(day, released),
            &lanes(),
            SystemId(system),
        )
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
    fn a_one_way_lane_carries_one_way() {
        let lanes = GateLanes::fixture(&[(DRIFT, VERGE, GateStability::Unstable)]);
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
        let news = heard(
            &storylines(),
            &story(10, &[("plate", 5)]),
            &lanes(),
            SystemId(DRIFT),
        );
        assert_eq!(
            news,
            [HeardNews {
                id: "plate".to_owned(),
                headline: "Plate".to_owned(),
                text: "Plate, told.".to_owned(),
                from: SystemId(VERGE),
                age: 3,
            }]
        );
    }

    #[test]
    fn a_lane_is_a_day() {
        assert_eq!(NEWS_DAYS_PER_LANE, 1);
    }
}
