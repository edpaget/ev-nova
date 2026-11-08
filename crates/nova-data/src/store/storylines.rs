//! The new game's missions, news and jump points, kept on the store as
//! the mission model reads them.
//!
//! The new game writes its story as missions (`missions/<id>.md`), which
//! test and set world state, and news (`news/<id>.md`), which missions
//! release. World state is the world flags and each jump point's owner and
//! stability. The loader (`crate::new_game`'s `missions`) maps them here,
//! then [`GameData::with_storylines`] sets them and
//! [`GameData::storylines`] answers them. Nothing here runs a mission: the
//! rules (offers, outcomes, effects, news spreading) are the story layer's
//! (`nova_sim_story::storylines`).
//!
//! The format's ids are kept as text: missions, news, flags, factions and
//! jump points (`system/point`), as they are permanent handles a save can
//! key by. A Nova ID appears only where the engine must place something: a
//! stellar to land at, a news item's `from` system, each jump point's
//! gate, and an outfit an effect grants or removes (effects aren't saved,
//! so a renumbered outfit can't strand one). Every reference here names something the store holds: the loader
//! leaves out a mission or news item that doesn't.
//!
//! The stock game's store holds none.

use std::collections::BTreeMap;

pub use nova_galaxy_format::model::Outcome;

use super::{DialogPlace, GameData, GateStability};
use crate::wire::id::{GovtId, OutfitId, StellarId, SystemId};

/// The new game's missions, news and jump points, each by its id.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Storylines {
    /// Every mission, by id (`missions/<id>.md`).
    pub missions: BTreeMap<String, StoryMission>,
    /// Every news item, by id (`news/<id>.md`).
    pub news: BTreeMap<String, NewsItem>,
    /// Every jump point, by `system/point`, with its static state.
    pub points: BTreeMap<String, PointAt>,
    /// Each faction's `gövt`, by its id: what a jump point's owner, as
    /// text, is in travel.
    pub factions: BTreeMap<String, GovtId>,
}

/// A mission: offered at a stellar once its `requires` hold, or, with no
/// offer, an event that happens by itself once they hold.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StoryMission {
    /// Its display title.
    pub title: String,
    /// What must hold before it is offered or happens.
    pub requires: Requires,
    /// Where it is offered and the offer's conversation; `None` for an
    /// event.
    pub offer: Option<MissionOffer>,
    /// What the player must do; `None` for a mission done as it is
    /// accepted (or as the event happens).
    pub objective: Option<MissionObjective>,
    /// The conversation where the objective is done, if it has one.
    pub complete: Option<MissionCompletion>,
    /// What the offer, or the event happening, sets.
    pub on_offer: StoryEffects,
    /// What accepting sets.
    pub on_accept: StoryEffects,
    /// What declining sets.
    pub on_decline: StoryEffects,
    /// What success sets.
    pub on_success: StoryEffects,
}

/// Where a mission is offered and the conversation that offers it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MissionOffer {
    /// The stellar it is offered at.
    pub at: StellarId,
    /// The offer's conversation. Every path ends in an outcome tag.
    pub dialog: DialogPlace,
}

/// The conversation where a mission's objective is done.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MissionCompletion {
    /// The stellar it plays at; `None` for the objective's own place.
    pub at: Option<StellarId>,
    /// The conversation.
    pub dialog: DialogPlace,
}

/// What a mission asks the player to do.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MissionObjective {
    /// Carry cargo to a stellar.
    Deliver {
        /// What the cargo is.
        cargo: String,
        /// How many tons.
        tons: u32,
        /// The stellar to deliver to.
        to: StellarId,
    },
    /// Take a jump point from the faction holding it.
    TakePoint {
        /// The point, `system/point`.
        point: String,
        /// The faction holding it.
        held_by: String,
    },
}

/// Conditions that must all hold, as the format writes them: every entry
/// of every list, with `all`, `any` and `none` nesting further groups. An
/// empty group holds.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Requires {
    /// Missions that must be done, by id.
    pub missions_done: Vec<String>,
    /// Flags that must be set.
    pub flags: Vec<String>,
    /// Flags that must not be set.
    pub flags_unset: Vec<String>,
    /// Jump points that must be in the given state.
    pub points: Vec<PointState>,
    /// The earliest day of the game.
    pub not_before_day: Option<u32>,
    /// Groups that must every one hold.
    pub all: Vec<Requires>,
    /// Groups of which at least one must hold.
    pub any: Vec<Requires>,
    /// Groups of which none may hold.
    pub none: Vec<Requires>,
}

/// World state a mission sets.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StoryEffects {
    /// Flags to set.
    pub flags: Vec<String>,
    /// Flags to clear.
    pub clear_flags: Vec<String>,
    /// Jump points whose owner or stability changes.
    pub points: Vec<PointState>,
    /// News released, by id.
    pub news: Vec<String>,
    /// Outfits granted, one for each entry, by their `oütf`: after the
    /// flags, points and news.
    pub grant_outfits: Vec<OutfitId>,
    /// Outfits removed, one for each entry, by their `oütf`: after the
    /// grants.
    pub remove_outfits: Vec<OutfitId>,
}

/// A jump point's owner, stability or both: a condition to test, or a
/// change to make. A field left `None` isn't tested or changed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PointState {
    /// The point, `system/point`.
    pub point: String,
    /// The owning faction, by id.
    pub owner: Option<String>,
    /// The stability.
    pub stability: Option<GateStability>,
}

/// A news item, released by a mission's effects.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NewsItem {
    /// Its headline.
    pub headline: String,
    /// The news as told: its file's body, trimmed.
    pub text: String,
    /// The system where it breaks.
    pub from: SystemId,
}

/// A jump point as the data writes it: its gate, and its static owner and
/// stability, which play overlays.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PointAt {
    /// The point's hypergate `spöb`.
    pub gate: StellarId,
    /// The faction holding it, by id, if any.
    pub owner: Option<String>,
    /// How well jumps through it work.
    pub stability: GateStability,
}

impl GameData {
    /// The store with `storylines`, replacing any it had.
    #[must_use]
    pub fn with_storylines(mut self, storylines: Storylines) -> Self {
        self.storylines = storylines;
        self
    }

    /// The new game's missions, news and jump points: none in a stock
    /// store (see the module docs).
    #[must_use]
    pub fn storylines(&self) -> &Storylines {
        &self.storylines
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn storylines() -> Storylines {
        Storylines {
            missions: BTreeMap::from([(
                "dock-job".to_owned(),
                StoryMission {
                    title: "Dock job".to_owned(),
                    ..StoryMission::default()
                },
            )]),
            news: BTreeMap::new(),
            points: BTreeMap::from([(
                "hearth/road".to_owned(),
                PointAt {
                    gate: StellarId(140),
                    owner: None,
                    stability: GateStability::Stable,
                },
            )]),
            factions: BTreeMap::from([("guild".to_owned(), GovtId(128))]),
        }
    }

    #[test]
    fn a_stock_store_keeps_no_storylines() {
        assert_eq!(GameData::default().storylines(), &Storylines::default());
        let data = GameData::from_decoded(PathBuf::from("/game"), Vec::new());
        assert_eq!(data.storylines(), &Storylines::default());
    }

    #[test]
    fn a_store_with_storylines_answers_them_and_new_ones_replace_them() {
        let data = GameData::default().with_storylines(storylines());
        assert_eq!(data.storylines(), &storylines());
        let data = data.with_storylines(Storylines::default());
        assert_eq!(data.storylines(), &Storylines::default());
    }

    #[test]
    fn outcomes_are_the_format_s() {
        assert_eq!(Outcome::parse("defer"), Some(Outcome::Defer));
    }
}
