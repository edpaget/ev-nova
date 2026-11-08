//! Builders for the new game's missions, for the story layer's and the
//! session's tests: each a plain value that tests change with
//! struct-update syntax.

use std::sync::Arc;

use crate::catalog::StellarId;
use crate::storylines::{
    DialogPlace, MissionObjective, MissionOffer, Requires, StoryEffects, StoryMission, Storylines,
};

/// The place `knot` in the Ink file `file`.
#[must_use]
pub fn place(file: &str, knot: &str) -> DialogPlace {
    DialogPlace {
        file: file.to_owned(),
        knot: knot.to_owned(),
    }
}

/// A mission offered at stellar `stellar` by the conversation at
/// `file`'s `knot`, requiring nothing, with no objective, no completion
/// and no effects.
#[must_use]
pub fn offered_at(stellar: i16, file: &str, knot: &str) -> StoryMission {
    StoryMission {
        offer: Some(MissionOffer {
            at: StellarId(stellar),
            dialog: place(file, knot),
        }),
        ..StoryMission::default()
    }
}

/// Effects that set the flags `names` and nothing else.
#[must_use]
pub fn setting(names: &[&str]) -> StoryEffects {
    StoryEffects {
        flags: texts(names),
        ..StoryEffects::default()
    }
}

/// Conditions that the flags `names` be set, and nothing else.
#[must_use]
pub fn requiring(names: &[&str]) -> Requires {
    Requires {
        flags: texts(names),
        ..Requires::default()
    }
}

/// A `deliver` objective: `tons` of `Plate` to stellar `to`.
#[must_use]
pub fn deliver(tons: u32, to: i16) -> MissionObjective {
    MissionObjective::Deliver {
        cargo: "Plate".to_owned(),
        tons,
        to: StellarId(to),
    }
}

/// `missions`, each by its id, with the Ink file `file`'s story compiled
/// to `compiled`.
#[must_use]
pub fn storylines(missions: &[(&str, StoryMission)], file: &str, compiled: &str) -> Storylines {
    Storylines {
        missions: missions
            .iter()
            .map(|(id, mission)| ((*id).to_owned(), mission.clone()))
            .collect(),
        ..Storylines::default()
    }
    .with_story(file, Arc::from(compiled))
}

fn texts(names: &[&str]) -> Vec<String> {
    names.iter().map(|&name| name.to_owned()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_offered_mission_is_offered_there_by_that_knot() {
        let mission = offered_at(130, "dialog/a.ink", "offer");
        let offer = mission.offer.expect("offered");
        assert_eq!(offer.at, StellarId(130));
        assert_eq!(offer.dialog, place("dialog/a.ink", "offer"));
        assert_eq!(mission.objective, None);
    }

    #[test]
    fn the_builders_fill_one_field_each() {
        assert_eq!(setting(&["a", "b"]).flags, ["a", "b"]);
        assert_eq!(requiring(&["c"]).flags, ["c"]);
        assert_eq!(
            deliver(10, 129),
            MissionObjective::Deliver {
                cargo: "Plate".to_owned(),
                tons: 10,
                to: StellarId(129),
            }
        );
    }

    #[test]
    fn storylines_hold_the_missions_and_the_story() {
        let held = storylines(
            &[("job", offered_at(128, "dialog/a.ink", "offer"))],
            "dialog/a.ink",
            "{a}",
        );
        assert_eq!(held.missions.keys().collect::<Vec<_>>(), ["job"]);
        assert_eq!(
            held.dialog(&place("dialog/a.ink", "offer"))
                .map(|at| at.compiled),
            Some("{a}")
        );
    }
}
