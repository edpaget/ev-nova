//! Applying a mission's effect block: flags set and cleared, jump points
//! changed, and news released, and its outfit grants and removals handed
//! to the caller.
//!
//! An outfit lives on the pilot's ship, which the story doesn't hold, so
//! a block's outfit changes are collected as [`OutfitChange`]s for the
//! session to make (through the `G` and `D` set operators, so the stock
//! rules for each apply).

use nova_sim_catalog::storylines::StoryEffects;

use super::state::StoryState;
use crate::catalog::OutfitId;
use crate::flags::WorldFlags;

/// An outfit change an effect block hands to the caller to make.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutfitChange {
    /// Grant one of the outfit.
    Grant(OutfitId),
    /// Remove one of the outfit, if owned.
    Remove(OutfitId),
}

/// Applies `effects` to `flags` and `story`:
///
/// - sets each of `flags`, then clears each of `clear_flags`, so a flag in
///   both ends clear;
/// - changes each point's owner and stability, each field given on its
///   own, over any change made before;
/// - releases each news item on today's day. An item released before
///   keeps the day it first broke;
/// - pushes onto `outfits` a grant for each of `grant_outfits`, then a
///   removal for each of `remove_outfits`, in list order, a repeated id
///   once for each time it is listed.
pub fn apply(
    effects: &StoryEffects,
    flags: &mut WorldFlags,
    story: &mut StoryState,
    outfits: &mut Vec<OutfitChange>,
) {
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
    let grants = effects
        .grant_outfits
        .iter()
        .map(|&id| OutfitChange::Grant(id));
    let removals = effects
        .remove_outfits
        .iter()
        .map(|&id| OutfitChange::Remove(id));
    outfits.extend(grants.chain(removals));
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use nova_sim_catalog::fixture::storylines::setting;
    use nova_sim_catalog::storylines::{PointStability, PointState};

    use super::*;
    use crate::catalog::OutfitId;
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
        apply(&effects, &mut flags, &mut story, &mut Vec::new());
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
            &mut Vec::new(),
        );
        assert_eq!(
            story.points["hearth/road"],
            PointChange {
                owner: None,
                stability: Some(PointStability::Dead),
            }
        );
        apply(
            &change(Some("guild"), None),
            &mut flags,
            &mut story,
            &mut Vec::new(),
        );
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
            &mut Vec::new(),
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
        apply(&effects, &mut flags, &mut story, &mut Vec::new());
        story.day = 5;
        apply(
            &StoryEffects {
                news: texts(&["closed", "opened"]),
                ..StoryEffects::default()
            },
            &mut flags,
            &mut story,
            &mut Vec::new(),
        );
        assert_eq!(
            story.news,
            BTreeMap::from([("closed".to_owned(), 3), ("opened".to_owned(), 5)])
        );
    }

    fn outfits(ids: &[i16]) -> Vec<OutfitId> {
        ids.iter().copied().map(OutfitId).collect()
    }

    #[test]
    fn outfits_are_granted_then_removed_in_list_order() {
        let effects = StoryEffects {
            grant_outfits: outfits(&[130, 128, 130]),
            remove_outfits: outfits(&[129, 128]),
            ..StoryEffects::default()
        };
        let mut changes = vec![OutfitChange::Remove(OutfitId(140))];
        apply(
            &effects,
            &mut WorldFlags::default(),
            &mut StoryState::default(),
            &mut changes,
        );
        assert_eq!(
            changes,
            [
                OutfitChange::Remove(OutfitId(140)),
                OutfitChange::Grant(OutfitId(130)),
                OutfitChange::Grant(OutfitId(128)),
                OutfitChange::Grant(OutfitId(130)),
                OutfitChange::Remove(OutfitId(129)),
                OutfitChange::Remove(OutfitId(128)),
            ],
            "pushed after what was there, a repeated id twice"
        );
    }

    #[test]
    fn a_block_without_outfits_hands_none() {
        let mut changes = Vec::new();
        apply(
            &setting(&["a"]),
            &mut WorldFlags::default(),
            &mut StoryState::default(),
            &mut changes,
        );
        assert_eq!(changes, []);
    }
}
