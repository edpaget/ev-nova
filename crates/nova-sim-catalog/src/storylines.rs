//! The new game's missions, news and jump points, as the story layer runs
//! them, each mission's conversations joined with their compiled stories.
//!
//! The store keeps them as the loader mapped them (`nova_data::Storylines`):
//! the format's ids as text, a Nova ID only where the engine places
//! something, and each dialog as a place (`dialog/<file>.ink` and a knot).
//! They serve here as they are, re-exported, as `control` re-exports the
//! expression types. The catalog serves them through
//! [`PilotCatalog::storylines`], whose default is none: the stock game has
//! no storylines. [`Storylines::dialog`] finds a place's compiled story,
//! as [`StellarDialogs`](crate::dialogs::StellarDialogs) does a
//! stellar's. The rules that run them are the story layer's
//! (`nova_sim_story::storylines`).
//!
//! [`PilotCatalog::storylines`]: crate::catalog::PilotCatalog::storylines

use std::collections::BTreeMap;
use std::sync::Arc;

pub use nova_data::{
    DialogPlace, MissionCompletion, MissionGiver, MissionObjective, MissionOffer, MissionStoryline,
    NewsItem, Outcome, PointAt, PointState, Requires, StoryAct, StoryEffects, StoryMission,
};

/// A jump point's stability as missions test and set it: the data's own
/// (stable, then unstable, then dead).
pub use nova_data::GateStability as PointStability;

use nova_data::{GameData, GovtId};

use crate::dialogs::DialogAt;

/// The new game's missions, news and jump points, each by its id, and the
/// compiled stories their conversations play.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Storylines {
    /// Every mission, by id.
    pub missions: BTreeMap<String, StoryMission>,
    /// Every news item, by id.
    pub news: BTreeMap<String, NewsItem>,
    /// Every jump point, by `system/point`, with its static state.
    pub points: BTreeMap<String, PointAt>,
    /// Each faction's government, by its id: the gate's government a
    /// point's owner stands for in travel.
    pub factions: BTreeMap<String, GovtId>,
    /// Each compiled story a mission's conversation names, by its Ink
    /// file (`dialog/<file>.ink`).
    pub compiled: BTreeMap<String, Arc<str>>,
}

impl Storylines {
    /// These storylines with the Ink file `file`'s story compiled to
    /// `compiled`, replacing any it had.
    #[must_use]
    pub fn with_story(mut self, file: &str, compiled: Arc<str>) -> Self {
        self.compiled.insert(file.to_owned(), compiled);
        self
    }

    /// Where `place` is: its Ink file, its compiled story and its knot, or
    /// `None` when its story isn't held.
    #[must_use]
    pub fn dialog<'a>(&'a self, place: &'a DialogPlace) -> Option<DialogAt<'a>> {
        let (file, compiled) = self.compiled.get_key_value(&place.file)?;
        Some(DialogAt {
            file,
            compiled,
            knot: &place.knot,
        })
    }

    /// Whether there are no missions: nothing to offer or happen.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.missions.is_empty()
    }
}

/// The store's storylines, with the compiled stories their missions'
/// conversations name.
pub(crate) fn storylines(data: &GameData) -> Storylines {
    let stored = data.storylines();
    let compiled = &data.dialogs().compiled;
    let named = stored.missions.values().flat_map(|mission| {
        let offer = mission.offer.as_ref().map(|offer| &offer.dialog);
        let complete = mission.complete.as_ref().map(|complete| &complete.dialog);
        offer.into_iter().chain(complete)
    });
    let compiled = named
        .filter_map(|place| {
            let (file, text) = compiled.get_key_value(&place.file)?;
            Some((file.clone(), Arc::clone(text)))
        })
        .collect();
    Storylines {
        missions: stored.missions.clone(),
        news: stored.news.clone(),
        points: stored.points.clone(),
        factions: stored.factions.clone(),
        compiled,
    }
}

#[cfg(test)]
mod tests {
    use nova_data::{Dialogs, StellarId, SystemId};

    use super::*;
    use crate::catalog::PilotCatalog;
    use crate::fixture::{FakePilotCatalog, catalog};

    fn place(file: &str, knot: &str) -> DialogPlace {
        DialogPlace {
            file: file.to_owned(),
            knot: knot.to_owned(),
        }
    }

    fn offered(file: &str, knot: &str) -> StoryMission {
        StoryMission {
            offer: Some(MissionOffer {
                at: StellarId(128),
                giver: None,
                dialog: place(file, knot),
            }),
            ..StoryMission::default()
        }
    }

    fn stored() -> nova_data::Storylines {
        nova_data::Storylines {
            missions: BTreeMap::from([
                ("job".to_owned(), offered("dialog/a.ink", "offer")),
                ("lost".to_owned(), offered("dialog/gone.ink", "offer")),
            ]),
            news: BTreeMap::from([(
                "word".to_owned(),
                NewsItem {
                    headline: "Word".to_owned(),
                    text: "It spreads.".to_owned(),
                    from: SystemId(128),
                },
            )]),
            points: BTreeMap::from([(
                "a/b".to_owned(),
                PointAt {
                    gate: StellarId(140),
                    owner: None,
                    stability: PointStability::Dead,
                },
            )]),
            factions: BTreeMap::from([("guild".to_owned(), GovtId(128))]),
        }
    }

    fn data() -> GameData {
        GameData::default()
            .with_dialogs(Dialogs {
                compiled: BTreeMap::from([
                    ("dialog/a.ink".to_owned(), Arc::from("{a}")),
                    ("dialog/other.ink".to_owned(), Arc::from("{other}")),
                ]),
                stellars: BTreeMap::new(),
            })
            .with_storylines(stored())
    }

    #[test]
    fn game_data_serves_the_store_s_missions_news_points_and_factions() {
        let served = PilotCatalog::storylines(&data());
        let stored = stored();
        assert_eq!(served.missions, stored.missions);
        assert_eq!(served.news, stored.news);
        assert_eq!(served.points, stored.points);
        assert_eq!(served.factions, stored.factions);
        assert!(!served.is_empty());
    }

    #[test]
    fn a_place_joins_its_compiled_story() {
        let served = PilotCatalog::storylines(&data());
        assert_eq!(
            served.dialog(&place("dialog/a.ink", "offer")),
            Some(DialogAt {
                file: "dialog/a.ink",
                compiled: "{a}",
                knot: "offer",
            })
        );
    }

    #[test]
    fn a_place_whose_story_is_missing_or_unnamed_joins_nothing() {
        let served = PilotCatalog::storylines(&data());
        assert_eq!(served.dialog(&place("dialog/gone.ink", "offer")), None);
        assert_eq!(
            served.dialog(&place("dialog/other.ink", "greet")),
            None,
            "only the stories missions name are kept"
        );
    }

    #[test]
    fn a_story_given_is_joined() {
        let storylines = Storylines::default().with_story("dialog/b.ink", Arc::from("{b}"));
        assert_eq!(
            storylines
                .dialog(&place("dialog/b.ink", "knot"))
                .map(|at| at.compiled),
            Some("{b}")
        );
    }

    #[test]
    fn a_stock_store_and_the_fake_serve_none() {
        assert_eq!(
            PilotCatalog::storylines(&GameData::default()),
            Storylines::default()
        );
        assert!(Storylines::default().is_empty());
        assert_eq!(catalog().storylines(), Storylines::default());
        let given = Storylines {
            missions: stored().missions,
            ..Storylines::default()
        };
        let fake = FakePilotCatalog {
            storylines: given.clone(),
            ..catalog()
        };
        assert_eq!(fake.storylines(), given);
    }
}
