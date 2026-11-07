//! The new game's story state in a save, from version 40 (`story`): the
//! day of the game, where each mission is, the jump points play has
//! changed and the day each news item broke (see
//! [`storylines`](crate::storylines)).
//!
//! Everything is keyed by the data's ids, as text, each list ascending:
//!
//! ```json
//! "story": {
//!   "day": 4,
//!   "missions": [
//!     {"id": "dock-job", "state": "active", "cargo": 10, "met": false},
//!     {"id": "side-road-closes", "state": "done"}
//!   ],
//!   "points": [{"point": "hearth/side-road", "stability": "dead"}],
//!   "news": [{"id": "side-road-closes", "released": 3}]
//! }
//! ```
//!
//! A mission's `state` is `offered`, `active` (with its `cargo` aboard and
//! whether it is `met`), `done` or `declined`. A point keeps only the
//! fields play changed: its `owner`, a faction's id, and its `stability`,
//! as the data writes one. A state or stability the engine doesn't know
//! makes the save unusable. A stock pilot's story is empty.

use serde::{Deserialize, Serialize};

use super::SaveError;
use crate::storylines::state::{MissionState, PointChange, StoryState};
use nova_sim_catalog::storylines::PointStability;

/// The story state, as a save keeps it.
#[derive(Serialize, Deserialize)]
pub(super) struct SavedStory {
    day: u32,
    missions: Vec<SavedMissionState>,
    points: Vec<SavedPointChange>,
    news: Vec<SavedNews>,
}

/// One mission's state.
#[derive(Serialize, Deserialize)]
struct SavedMissionState {
    id: String,
    state: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    cargo: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    met: Option<bool>,
}

/// One jump point's change.
#[derive(Serialize, Deserialize)]
struct SavedPointChange {
    point: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    owner: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    stability: Option<String>,
}

/// One news item's release.
#[derive(Serialize, Deserialize)]
struct SavedNews {
    id: String,
    released: u32,
}

/// `story` as a save keeps it.
pub(super) fn saved_story(story: &StoryState) -> SavedStory {
    SavedStory {
        day: story.day,
        missions: story
            .missions
            .iter()
            .map(|(id, state)| {
                let (state, cargo, met) = match *state {
                    MissionState::Offered => ("offered", None, None),
                    MissionState::Active { cargo, met } => ("active", Some(cargo), Some(met)),
                    MissionState::Done => ("done", None, None),
                    MissionState::Declined => ("declined", None, None),
                };
                SavedMissionState {
                    id: id.clone(),
                    state: state.to_owned(),
                    cargo,
                    met,
                }
            })
            .collect(),
        points: story
            .points
            .iter()
            .map(|(point, change)| SavedPointChange {
                point: point.clone(),
                owner: change.owner.clone(),
                stability: change.stability.map(|s| s.as_str().to_owned()),
            })
            .collect(),
        news: story
            .news
            .iter()
            .map(|(id, &released)| SavedNews {
                id: id.clone(),
                released,
            })
            .collect(),
    }
}

/// The story state `saved` keeps, or why it can't be read.
pub(super) fn story_of(saved: SavedStory) -> Result<StoryState, SaveError> {
    let missions = saved
        .missions
        .into_iter()
        .map(|saved| {
            let state = match saved.state.as_str() {
                "offered" => MissionState::Offered,
                "active" => MissionState::Active {
                    cargo: saved.cargo.unwrap_or(0),
                    met: saved.met.unwrap_or(false),
                },
                "done" => MissionState::Done,
                "declined" => MissionState::Declined,
                other => {
                    return Err(SaveError::Unusable(format!(
                        "mission {}: no such state, {other:?}",
                        saved.id
                    )));
                }
            };
            Ok((saved.id, state))
        })
        .collect::<Result<_, _>>()?;
    let points = saved
        .points
        .into_iter()
        .map(|saved| {
            let stability = saved
                .stability
                .map(|text| {
                    stability_of(&text).ok_or_else(|| {
                        SaveError::Unusable(format!(
                            "point {}: no such stability, {text:?}",
                            saved.point
                        ))
                    })
                })
                .transpose()?;
            Ok((
                saved.point,
                PointChange {
                    owner: saved.owner,
                    stability,
                },
            ))
        })
        .collect::<Result<_, SaveError>>()?;
    Ok(StoryState {
        day: saved.day,
        missions,
        points,
        news: saved
            .news
            .into_iter()
            .map(|saved| (saved.id, saved.released))
            .collect(),
    })
}

/// The stability the data writes as `text`.
fn stability_of(text: &str) -> Option<PointStability> {
    [
        PointStability::Stable,
        PointStability::Unstable,
        PointStability::Dead,
    ]
    .into_iter()
    .find(|stability| stability.as_str() == text)
}

#[cfg(test)]
mod tests {
    use super::super::tests::seasoned;
    use super::super::{CURRENT, decode, encode, story_state_40};
    use super::*;
    use crate::pilot::Pilot;

    /// [`seasoned`] on day 9 with a mission in each state, two points
    /// changed and two news items, each added out of order.
    fn storied() -> Pilot {
        let mut pilot = seasoned();
        let story = &mut pilot.story;
        story.day = 9;
        for (id, state) in [
            ("toll", MissionState::Declined),
            (
                "job",
                MissionState::Active {
                    cargo: 10,
                    met: true,
                },
            ),
            ("road", MissionState::Offered),
            ("close", MissionState::Done),
        ] {
            story.missions.insert(id.to_owned(), state);
        }
        story.points.insert(
            "verge/side-road".to_owned(),
            PointChange {
                owner: None,
                stability: Some(PointStability::Dead),
            },
        );
        story.points.insert(
            "hearth/side-road".to_owned(),
            PointChange {
                owner: Some("guild".to_owned()),
                stability: Some(PointStability::Unstable),
            },
        );
        story.news.insert("plate".to_owned(), 7);
        story.news.insert("closed".to_owned(), 3);
        pilot
    }

    fn json(text: &str) -> serde_json::Value {
        serde_json::from_str(text).expect("JSON")
    }

    #[test]
    fn the_story_survives_a_round_trip() {
        let pilot = storied();
        assert_eq!(decode(&encode(&pilot)), Ok(pilot));
    }

    #[test]
    fn the_story_is_saved_ascending_with_only_what_each_entry_holds() {
        let value = json(&encode(&storied()));
        assert_eq!(value["version"], CURRENT);
        assert_eq!(CURRENT, 40);
        assert_eq!(
            value["story"],
            serde_json::json!({
                "day": 9,
                "missions": [
                    {"id": "close", "state": "done"},
                    {"id": "job", "state": "active", "cargo": 10, "met": true},
                    {"id": "road", "state": "offered"},
                    {"id": "toll", "state": "declined"}
                ],
                "points": [
                    {"point": "hearth/side-road", "owner": "guild", "stability": "unstable"},
                    {"point": "verge/side-road", "stability": "dead"}
                ],
                "news": [
                    {"id": "closed", "released": 3},
                    {"id": "plate", "released": 7}
                ]
            })
        );
    }

    #[test]
    fn a_stock_pilot_saves_an_empty_story() {
        let value = json(&encode(&seasoned()));
        assert_eq!(
            value["story"],
            serde_json::json!({"day": 0, "missions": [], "points": [], "news": []})
        );
    }

    #[test]
    fn a_version_39_save_loads_with_an_empty_story() {
        let mut value = json(&encode(&storied()));
        value["version"] = serde_json::Value::from(39);
        value.as_object_mut().expect("an object").remove("story");
        let pilot = decode(&value.to_string()).expect("loads");
        assert_eq!(pilot, seasoned());
    }

    #[test]
    fn the_upgrade_from_39_adds_an_empty_story() {
        let mut value = serde_json::json!({"name": "Ada"});
        story_state_40(&mut value);
        assert_eq!(
            value,
            serde_json::json!({
                "name": "Ada",
                "story": {"day": 0, "missions": [], "points": [], "news": []}
            })
        );
    }

    /// Why the current save of `storied`, edited by `edit`, is unusable.
    fn unusable(edit: impl FnOnce(&mut serde_json::Value)) -> String {
        let mut value = json(&encode(&storied()));
        edit(&mut value);
        match decode(&value.to_string()) {
            Err(SaveError::Unusable(reason)) => reason,
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_current_save_without_a_story_is_unusable() {
        let reason = unusable(|value| {
            value.as_object_mut().expect("an object").remove("story");
        });
        assert!(reason.contains("story"), "{reason}");
    }

    #[test]
    fn an_unknown_state_or_stability_is_unusable() {
        let reason = unusable(|value| value["story"]["missions"][0]["state"] = "lost".into());
        assert_eq!(reason, "mission close: no such state, \"lost\"");
        let reason = unusable(|value| value["story"]["points"][1]["stability"] = "wobbly".into());
        assert_eq!(
            reason,
            "point verge/side-road: no such stability, \"wobbly\""
        );
    }

    #[test]
    fn an_active_mission_missing_its_cargo_and_met_loads_empty_and_unmet() {
        let mut value = json(&encode(&storied()));
        let job = &mut value["story"]["missions"][1];
        let job = job.as_object_mut().expect("an object");
        job.remove("cargo");
        job.remove("met");
        let pilot = decode(&value.to_string()).expect("loads");
        assert_eq!(
            pilot.story.mission("job"),
            Some(MissionState::Active {
                cargo: 0,
                met: false
            })
        );
    }

    #[test]
    fn every_stability_reads_as_written() {
        for stability in [
            PointStability::Stable,
            PointStability::Unstable,
            PointStability::Dead,
        ] {
            assert_eq!(stability_of(stability.as_str()), Some(stability));
        }
        assert_eq!(stability_of("Dead"), None);
    }
}
