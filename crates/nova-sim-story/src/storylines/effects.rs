//! Applying a mission's effect block: flags set and cleared, jump points
//! changed, and news released.

use nova_sim_catalog::storylines::StoryEffects;

use super::state::StoryState;
use crate::flags::WorldFlags;

/// Applies `effects` to `flags` and `story`:
///
/// - sets each of `flags`, then clears each of `clear_flags`, so a flag in
///   both ends clear;
/// - changes each point's owner and stability, each field given on its
///   own, over any change made before;
/// - releases each news item on today's day. An item released before
///   keeps the day it first broke.
pub fn apply(effects: &StoryEffects, flags: &mut WorldFlags, story: &mut StoryState) {
    for flag in &effects.flags {
        flags.set(flag);
    }
    for flag in &effects.clear_flags {
        flags.clear(flag);
    }
    for change in &effects.points {
        let point = story.points.entry(change.point.clone()).or_default();
        if let Some(owner) = &change.owner {
            point.owner = Some(owner.clone());
        }
        if let Some(stability) = change.stability {
            point.stability = Some(stability);
        }
    }
    let today = story.day;
    for news in &effects.news {
        story.news.entry(news.clone()).or_insert(today);
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use nova_sim_catalog::fixture::storylines::setting;
    use nova_sim_catalog::storylines::{PointStability, PointState};

    use super::*;
    use crate::storylines::state::PointChange;

    fn texts(names: &[&str]) -> Vec<String> {
        names.iter().map(|&name| name.to_owned()).collect()
    }

    fn change(owner: Option<&str>, stability: Option<PointStability>) -> StoryEffects {
        StoryEffects {
            points: vec![PointState {
                point: "hearth/road".to_owned(),
                owner: owner.map(str::to_owned),
                stability,
            }],
            ..StoryEffects::default()
        }
    }

    #[test]
    fn flags_are_set_then_cleared() {
        let mut flags: WorldFlags = ["old".to_owned(), "both".to_owned()].into_iter().collect();
        let mut story = StoryState::default();
        let effects = StoryEffects {
            clear_flags: texts(&["old", "both"]),
            ..setting(&["new", "both"])
        };
        apply(&effects, &mut flags, &mut story);
        assert_eq!(flags.iter().collect::<Vec<_>>(), ["new"]);
        assert_eq!(story, StoryState::default());
    }

    #[test]
    fn a_point_s_owner_and_stability_change_independently() {
        let mut flags = WorldFlags::default();
        let mut story = StoryState::default();
        apply(
            &change(None, Some(PointStability::Dead)),
            &mut flags,
            &mut story,
        );
        assert_eq!(
            story.points["hearth/road"],
            PointChange {
                owner: None,
                stability: Some(PointStability::Dead),
            }
        );
        apply(&change(Some("guild"), None), &mut flags, &mut story);
        assert_eq!(
            story.points["hearth/road"],
            PointChange {
                owner: Some("guild".to_owned()),
                stability: Some(PointStability::Dead),
            },
            "the stability stays"
        );
        apply(
            &change(Some("warden"), Some(PointStability::Unstable)),
            &mut flags,
            &mut story,
        );
        assert_eq!(
            story.points["hearth/road"],
            PointChange {
                owner: Some("warden".to_owned()),
                stability: Some(PointStability::Unstable),
            }
        );
    }

    #[test]
    fn news_breaks_today_and_once() {
        let mut flags = WorldFlags::default();
        let mut story = StoryState {
            day: 3,
            ..StoryState::default()
        };
        let effects = StoryEffects {
            news: texts(&["closed"]),
            ..StoryEffects::default()
        };
        apply(&effects, &mut flags, &mut story);
        story.day = 5;
        apply(
            &StoryEffects {
                news: texts(&["closed", "opened"]),
                ..StoryEffects::default()
            },
            &mut flags,
            &mut story,
        );
        assert_eq!(
            story.news,
            BTreeMap::from([("closed".to_owned(), 3), ("opened".to_owned(), 5)])
        );
    }
}
