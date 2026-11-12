//! The new game's story state in a save, from version 40 (`story`): the
//! day of the game, where each mission is, the jump points play has
//! changed and the day each news item broke, and from version 41 the news
//! items the pilot has read (see [`storylines`](nova_sim_story::storylines)).
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
//!   "news": [{"id": "side-road-closes", "released": 3, "reach": [
//!     {"system": 129, "lanes": 0}, {"system": 130, "lanes": 1}
//!   ]}],
//!   "read": ["side-road-closes"]
//! }
//! ```
//!
//! A mission's `state` is `offered`, `active` (with its `cargo` aboard and
//! whether it is `met`), `done` or `declined`. A point keeps only the
//! fields play changed: its `owner`, a faction's id, and its `stability`,
//! as the data writes one. A state or stability the engine doesn't know
//! makes the save unusable. A news item keeps its `reach`, frozen at its
//! release: the lanes it crosses to each system it reaches, by system ID,
//! ascending. A save made before reaches were kept has none, and the load
//! freezes each then (see the session's `gate_points` module). The news
//! read (`read`) are their ids, ascending: the pilot's, not a system's.
//! The current version requires the list; a version 40 save had none, so
//! the upgrade gives `[]`, nothing read. An id the data no longer has
//! stays as saved and matches nothing. A stock pilot's story is empty.

use serde::{Deserialize, Serialize};

use super::SaveError;
use nova_sim_catalog::catalog::SystemId;
use nova_sim_catalog::storylines::PointStability;
use nova_sim_story::storylines::state::{MissionState, PointChange, StoryState};

/// The story state, as a save keeps it.
#[derive(Serialize, Deserialize)]
pub(super) struct SavedStory {
    day: u32,
    missions: Vec<SavedMissionState>,
    points: Vec<SavedPointChange>,
    news: Vec<SavedNews>,
    read: Vec<String>,
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

/// One news item's release, and its reach once frozen.
#[derive(Serialize, Deserialize)]
struct SavedNews {
    id: String,
    released: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    reach: Vec<SavedReach>,
}

/// How many lanes a news item crosses to reach one system.
#[derive(Serialize, Deserialize)]
struct SavedReach {
    system: i16,
    lanes: u32,
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
                reach: story
                    .reach
                    .get(id)
                    .into_iter()
                    .flatten()
                    .map(|(system, &lanes)| SavedReach {
                        system: system.0,
                        lanes,
                    })
                    .collect(),
            })
            .collect(),
        read: story.read.iter().cloned().collect(),
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
    let reach = saved
        .news
        .iter()
        .filter(|saved| !saved.reach.is_empty())
        .map(|saved| {
            let reach = saved
                .reach
                .iter()
                .map(|reach| (SystemId(reach.system), reach.lanes))
                .collect();
            (saved.id.clone(), reach)
        })
        .collect();
    Ok(StoryState {
        day: saved.day,
        missions,
        points,
        news: saved
            .news
            .into_iter()
            .map(|saved| (saved.id, saved.released))
            .collect(),
        reach,
        read: saved.read.into_iter().collect(),
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
    use super::super::upgrade::{news_read_41, story_state_40};
    use super::super::{CURRENT, decode, encode};
    use super::*;
    use nova_sim_story::pilot::Pilot;

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

    /// [`storied`] with both news items read, marked out of order.
    fn read() -> Pilot {
        let mut pilot = storied();
        pilot.story.read.insert("plate".to_owned());
        pilot.story.read.insert("closed".to_owned());
        pilot
    }

    #[test]
    fn the_news_read_survives_a_round_trip() {
        let pilot = read();
        assert_eq!(decode(&encode(&pilot)), Ok(pilot));
    }

    #[test]
    fn the_news_read_is_saved_ascending() {
        let value = json(&encode(&read()));
        assert_eq!(
            value["story"]["read"],
            serde_json::json!(["closed", "plate"])
        );
    }

    #[test]
    fn a_version_40_save_loads_with_nothing_read() {
        let mut value = json(&encode(&read()));
        value["version"] = serde_json::Value::from(40);
        value["story"]
            .as_object_mut()
            .expect("an object")
            .remove("read");
        let pilot = decode(&value.to_string()).expect("loads");
        assert_eq!(pilot, storied());
    }

    #[test]
    fn the_upgrade_from_40_reads_nothing() {
        let mut value = serde_json::json!({
            "name": "Ada",
            "story": {"day": 2, "missions": [], "points": [], "news": []}
        });
        news_read_41(&mut value);
        assert_eq!(
            value,
            serde_json::json!({
                "name": "Ada",
                "story": {"day": 2, "missions": [], "points": [], "news": [], "read": []}
            })
        );
    }

    #[test]
    fn a_current_save_without_its_news_read_is_unusable() {
        let reason = unusable(|value| {
            value["story"]
                .as_object_mut()
                .expect("an object")
                .remove("read");
        });
        assert!(reason.contains("read"), "{reason}");
    }

    #[test]
    fn the_story_is_saved_ascending_with_only_what_each_entry_holds() {
        let value = json(&encode(&storied()));
        assert_eq!(value["version"], CURRENT);
        assert_eq!(CURRENT, 41);
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
                ],
                "read": []
            })
        );
    }

    /// [`storied`] with `closed` frozen to reach 130 at once and 131 a
    /// lane on.
    fn reached() -> Pilot {
        let mut pilot = storied();
        pilot.story.reach.insert(
            "closed".to_owned(),
            std::collections::BTreeMap::from([
                (nova_sim_catalog::catalog::SystemId(131), 1),
                (nova_sim_catalog::catalog::SystemId(130), 0),
            ]),
        );
        pilot
    }

    #[test]
    fn a_news_items_reach_survives_a_round_trip() {
        let pilot = reached();
        assert_eq!(decode(&encode(&pilot)), Ok(pilot));
    }

    #[test]
    fn a_reach_is_saved_by_ascending_system_on_its_item_alone() {
        let value = json(&encode(&reached()));
        assert_eq!(
            value["story"]["news"],
            serde_json::json!([
                {"id": "closed", "released": 3, "reach": [
                    {"system": 130, "lanes": 0},
                    {"system": 131, "lanes": 1}
                ]},
                {"id": "plate", "released": 7}
            ])
        );
    }

    #[test]
    fn a_news_item_saved_without_a_reach_loads_with_none() {
        let mut value = json(&encode(&reached()));
        value["story"]["news"][0]
            .as_object_mut()
            .expect("an object")
            .remove("reach");
        let pilot = decode(&value.to_string()).expect("loads");
        assert!(pilot.story.reach.is_empty());
        assert_eq!(pilot.story.news, reached().story.news);
    }

    #[test]
    fn a_stock_pilot_saves_an_empty_story() {
        let value = json(&encode(&seasoned()));
        assert_eq!(
            value["story"],
            serde_json::json!({"day": 0, "missions": [], "points": [], "news": [], "read": []})
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
