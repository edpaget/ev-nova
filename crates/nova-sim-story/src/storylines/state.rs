//! The pilot's story state: the day of the game, where each mission is,
//! how play has changed each jump point, and when each news item broke.
//! Everything is keyed by the data's own ids, as text (see the
//! [module docs](super)).

use std::collections::BTreeMap;

use crate::catalog::StellarId;
use nova_sim_catalog::storylines::{PointAt, PointStability};

/// The new game's story, as a pilot has played it. A new pilot's is
/// empty: day 0, no mission seen, no point changed, no news.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StoryState {
    /// The day of the game: 0 for a new pilot, one more for each day
    /// passed.
    pub day: u32,
    /// Each mission that has been offered or has happened, by id. A
    /// mission it doesn't hold hasn't been seen.
    pub missions: BTreeMap<String, MissionState>,
    /// Each jump point play has changed, by `system/point`.
    pub points: BTreeMap<String, PointChange>,
    /// Each news item released, by id, with the day it broke.
    pub news: BTreeMap<String, u32>,
}

/// Where a mission is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MissionState {
    /// Offered, and neither accepted nor declined: it is offered again
    /// while its `requires` hold.
    Offered,
    /// Accepted (or happened), its objective not yet done.
    Active {
        /// The tons of its cargo aboard.
        cargo: u32,
        /// Whether its objective is met: its cargo is delivered.
        met: bool,
    },
    /// Done: it succeeded. The only state `missions_done` counts.
    Done,
    /// Declined: never offered again.
    Declined,
}

/// How play has changed a jump point: each field set overrides the data's.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PointChange {
    /// The faction holding it now, by id.
    pub owner: Option<String>,
    /// Its stability now.
    pub stability: Option<PointStability>,
}

/// A jump point as it stands: the data's state, overlaid by play's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PointNow<'a> {
    /// Its gate's `spöb`.
    pub gate: StellarId,
    /// The faction holding it, if any.
    pub owner: Option<&'a str>,
    /// How well jumps through it work.
    pub stability: PointStability,
}

impl StoryState {
    /// Where mission `id` is, or `None` when it hasn't been seen.
    #[must_use]
    pub fn mission(&self, id: &str) -> Option<MissionState> {
        self.missions.get(id).copied()
    }

    /// Whether mission `id` is done.
    #[must_use]
    pub fn is_done(&self, id: &str) -> bool {
        self.mission(id) == Some(MissionState::Done)
    }

    /// The tons of every active mission's cargo aboard.
    #[must_use]
    pub fn cargo_aboard(&self) -> i64 {
        self.missions
            .values()
            .map(|state| match state {
                MissionState::Active { cargo, .. } => i64::from(*cargo),
                _ => 0,
            })
            .sum()
    }

    /// The jump point `point` (`system/point`) as it stands: its data in
    /// `points`, each field play has changed overridden. `None` when the
    /// data has no such point.
    #[must_use]
    pub fn point<'a>(
        &'a self,
        points: &'a BTreeMap<String, PointAt>,
        point: &str,
    ) -> Option<PointNow<'a>> {
        let data = points.get(point)?;
        let change = self.points.get(point);
        Some(PointNow {
            gate: data.gate,
            owner: change
                .and_then(|change| change.owner.as_deref())
                .or(data.owner.as_deref()),
            stability: change
                .and_then(|change| change.stability)
                .unwrap_or(data.stability),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn points() -> BTreeMap<String, PointAt> {
        BTreeMap::from([(
            "hearth/road".to_owned(),
            PointAt {
                gate: StellarId(140),
                owner: Some("warden".to_owned()),
                stability: PointStability::Stable,
            },
        )])
    }

    #[test]
    fn a_new_story_is_day_0_with_nothing_seen() {
        let story = StoryState::default();
        assert_eq!(story.day, 0);
        assert_eq!(story.mission("job"), None);
        assert!(!story.is_done("job"));
        assert_eq!(story.cargo_aboard(), 0);
    }

    #[test]
    fn only_done_is_done() {
        let mut story = StoryState::default();
        for state in [
            MissionState::Offered,
            MissionState::Active {
                cargo: 0,
                met: true,
            },
            MissionState::Declined,
        ] {
            story.missions.insert("job".to_owned(), state);
            assert!(!story.is_done("job"), "{state:?}");
        }
        story.missions.insert("job".to_owned(), MissionState::Done);
        assert!(story.is_done("job"));
    }

    #[test]
    fn the_cargo_aboard_is_every_active_mission_s() {
        let mut story = StoryState::default();
        story.missions.insert(
            "a".to_owned(),
            MissionState::Active {
                cargo: 10,
                met: false,
            },
        );
        story.missions.insert(
            "b".to_owned(),
            MissionState::Active {
                cargo: u32::MAX,
                met: false,
            },
        );
        story.missions.insert("c".to_owned(), MissionState::Done);
        assert_eq!(story.cargo_aboard(), i64::from(u32::MAX) + 10, "no wrap");
    }

    #[test]
    fn a_point_stands_as_its_data_until_play_changes_it() {
        let mut story = StoryState::default();
        let points = points();
        assert_eq!(
            story.point(&points, "hearth/road"),
            Some(PointNow {
                gate: StellarId(140),
                owner: Some("warden"),
                stability: PointStability::Stable,
            })
        );
        assert_eq!(story.point(&points, "hearth/nowhere"), None);
        story.points.insert(
            "hearth/road".to_owned(),
            PointChange {
                owner: None,
                stability: Some(PointStability::Dead),
            },
        );
        let now = story.point(&points, "hearth/road").expect("a point");
        assert_eq!(
            (now.owner, now.stability),
            (Some("warden"), PointStability::Dead)
        );
        story.points.insert(
            "hearth/road".to_owned(),
            PointChange {
                owner: Some("guild".to_owned()),
                stability: None,
            },
        );
        let now = story.point(&points, "hearth/road").expect("a point");
        assert_eq!(
            (now.owner, now.stability),
            (Some("guild"), PointStability::Stable)
        );
    }
}
