//! Whether a mission's `requires` hold: over the world flags, the missions
//! done, the jump points as they stand and the day of the game.

use nova_sim_catalog::storylines::{PointState, Requires, Storylines};

use super::state::StoryState;
use crate::flags::WorldFlags;

/// Whether `requires` hold, with `flags` set, in `story`, over the jump
/// points of `storylines`:
///
/// - each of `missions_done` is done, each of `flags` set and each of
///   `flags_unset` unset;
/// - each of `points` stands as given, in each field given, the data's
///   state overlaid by play's;
/// - the day is `not_before_day` or later;
/// - every group of `all` holds, one of `any` holds (when it has any) and
///   none of `none` holds.
///
/// An empty group holds.
#[must_use]
pub fn holds(
    requires: &Requires,
    storylines: &Storylines,
    flags: &WorldFlags,
    story: &StoryState,
) -> bool {
    let group = |group: &Requires| holds(group, storylines, flags, story);
    requires.missions_done.iter().all(|id| story.is_done(id))
        && requires.flags.iter().all(|flag| flags.is_set(flag))
        && requires.flags_unset.iter().all(|flag| !flags.is_set(flag))
        && requires
            .points
            .iter()
            .all(|wanted| stands(wanted, storylines, story))
        && requires.not_before_day.is_none_or(|day| story.day >= day)
        && requires.all.iter().all(group)
        && (requires.any.is_empty() || requires.any.iter().any(group))
        && !requires.none.iter().any(group)
}

/// Whether the jump point `wanted` names stands as it says, in each field
/// it gives.
fn stands(wanted: &PointState, storylines: &Storylines, story: &StoryState) -> bool {
    let Some(now) = story.point(&storylines.points, &wanted.point) else {
        return false;
    };
    wanted
        .owner
        .as_deref()
        .is_none_or(|owner| now.owner == Some(owner))
        && wanted
            .stability
            .is_none_or(|stability| now.stability == stability)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use nova_sim_catalog::fixture::storylines::requiring;
    use nova_sim_catalog::storylines::{PointAt, PointStability};

    use super::*;
    use crate::catalog::StellarId;
    use crate::storylines::state::{MissionState, PointChange};

    /// Storylines with one jump point, `hearth/road`, stable and held by
    /// the wardens.
    fn storylines() -> Storylines {
        Storylines {
            points: BTreeMap::from([(
                "hearth/road".to_owned(),
                PointAt {
                    gate: StellarId(140),
                    owner: Some("warden".to_owned()),
                    stability: PointStability::Stable,
                },
            )]),
            ..Storylines::default()
        }
    }

    fn flags(names: &[&str]) -> WorldFlags {
        names.iter().map(|&name| name.to_owned()).collect()
    }

    fn check(requires: &Requires, flags: &WorldFlags, story: &StoryState) -> bool {
        holds(requires, &storylines(), flags, story)
    }

    fn point(owner: Option<&str>, stability: Option<PointStability>) -> Requires {
        Requires {
            points: vec![PointState {
                point: "hearth/road".to_owned(),
                owner: owner.map(str::to_owned),
                stability,
            }],
            ..Requires::default()
        }
    }

    fn texts(names: &[&str]) -> Vec<String> {
        names.iter().map(|&name| name.to_owned()).collect()
    }

    #[test]
    fn nothing_required_holds() {
        assert!(check(
            &Requires::default(),
            &WorldFlags::default(),
            &StoryState::default()
        ));
    }

    #[test]
    fn flags_must_each_be_set() {
        let story = StoryState::default();
        let requires = requiring(&["a", "b"]);
        assert!(check(&requires, &flags(&["a", "b", "c"]), &story));
        assert!(!check(&requires, &flags(&["a"]), &story));
        assert!(!check(&requires, &flags(&["b"]), &story));
    }

    #[test]
    fn flags_unset_must_each_be_unset() {
        let story = StoryState::default();
        let requires = Requires {
            flags_unset: texts(&["a", "b"]),
            ..Requires::default()
        };
        assert!(check(&requires, &flags(&["c"]), &story));
        assert!(!check(&requires, &flags(&["a"]), &story));
        assert!(!check(&requires, &flags(&["b"]), &story));
    }

    #[test]
    fn missions_done_counts_done_only() {
        let requires = Requires {
            missions_done: texts(&["job", "toll"]),
            ..Requires::default()
        };
        let mut story = StoryState::default();
        story.missions.insert("job".to_owned(), MissionState::Done);
        assert!(!check(&requires, &WorldFlags::default(), &story));
        for state in [
            MissionState::Offered,
            MissionState::Active {
                cargo: 0,
                met: true,
            },
            MissionState::Declined,
        ] {
            story.missions.insert("toll".to_owned(), state);
            assert!(
                !check(&requires, &WorldFlags::default(), &story),
                "{state:?}"
            );
        }
        story.missions.insert("toll".to_owned(), MissionState::Done);
        assert!(check(&requires, &WorldFlags::default(), &story));
    }

    #[test]
    fn a_point_s_owner_stability_or_both_must_stand() {
        let none = WorldFlags::default();
        let story = StoryState::default();
        assert!(check(&point(Some("warden"), None), &none, &story));
        assert!(!check(&point(Some("guild"), None), &none, &story));
        assert!(check(
            &point(None, Some(PointStability::Stable)),
            &none,
            &story
        ));
        assert!(!check(
            &point(None, Some(PointStability::Dead)),
            &none,
            &story
        ));
        assert!(check(
            &point(Some("warden"), Some(PointStability::Stable)),
            &none,
            &story
        ));
        assert!(!check(
            &point(Some("warden"), Some(PointStability::Dead)),
            &none,
            &story
        ));
        assert!(!check(
            &point(Some("guild"), Some(PointStability::Stable)),
            &none,
            &story
        ));
    }

    #[test]
    fn a_point_play_has_changed_is_read_as_it_stands() {
        let mut story = StoryState::default();
        story.points.insert(
            "hearth/road".to_owned(),
            PointChange {
                owner: Some("guild".to_owned()),
                stability: Some(PointStability::Dead),
            },
        );
        let none = WorldFlags::default();
        assert!(check(
            &point(Some("guild"), Some(PointStability::Dead)),
            &none,
            &story
        ));
        assert!(!check(&point(Some("warden"), None), &none, &story));
    }

    #[test]
    fn a_point_the_data_lacks_never_stands() {
        let requires = Requires {
            points: vec![PointState {
                point: "hearth/nowhere".to_owned(),
                owner: None,
                stability: None,
            }],
            ..Requires::default()
        };
        assert!(!check(
            &requires,
            &WorldFlags::default(),
            &StoryState::default()
        ));
    }

    #[test]
    fn not_before_day_holds_from_that_day() {
        let requires = Requires {
            not_before_day: Some(3),
            ..Requires::default()
        };
        let mut story = StoryState {
            day: 2,
            ..StoryState::default()
        };
        assert!(!check(&requires, &WorldFlags::default(), &story));
        story.day = 3;
        assert!(check(&requires, &WorldFlags::default(), &story));
        story.day = 4;
        assert!(check(&requires, &WorldFlags::default(), &story));
    }

    #[test]
    fn all_needs_every_group() {
        let requires = Requires {
            all: vec![requiring(&["a"]), requiring(&["b"])],
            ..Requires::default()
        };
        let story = StoryState::default();
        assert!(check(&requires, &flags(&["a", "b"]), &story));
        assert!(!check(&requires, &flags(&["a"]), &story));
        assert!(!check(&requires, &flags(&["b"]), &story));
    }

    #[test]
    fn any_needs_one_group() {
        let requires = Requires {
            any: vec![requiring(&["a"]), requiring(&["b"])],
            ..Requires::default()
        };
        let story = StoryState::default();
        assert!(!check(&requires, &flags(&[]), &story), "none");
        assert!(check(&requires, &flags(&["a"]), &story), "one");
        assert!(check(&requires, &flags(&["b"]), &story), "the other");
        assert!(check(&requires, &flags(&["a", "b"]), &story), "all");
    }

    #[test]
    fn none_fails_on_any_one_group() {
        let requires = Requires {
            none: vec![requiring(&["a"]), requiring(&["b"])],
            ..Requires::default()
        };
        let story = StoryState::default();
        assert!(check(&requires, &flags(&["c"]), &story));
        assert!(!check(&requires, &flags(&["a"]), &story));
        assert!(!check(&requires, &flags(&["b"]), &story));
    }

    #[test]
    fn empty_groups_hold() {
        let requires = Requires {
            all: vec![Requires::default()],
            any: vec![Requires::default()],
            ..Requires::default()
        };
        assert!(check(
            &requires,
            &WorldFlags::default(),
            &StoryState::default()
        ));
    }

    #[test]
    fn groups_nest_two_deep() {
        // all: [any: [a, none: [b]]]: a is set, or b is not.
        let requires = Requires {
            all: vec![Requires {
                any: vec![
                    requiring(&["a"]),
                    Requires {
                        none: vec![requiring(&["b"])],
                        ..Requires::default()
                    },
                ],
                ..Requires::default()
            }],
            ..Requires::default()
        };
        let story = StoryState::default();
        assert!(check(&requires, &flags(&[]), &story));
        assert!(check(&requires, &flags(&["a", "b"]), &story));
        assert!(!check(&requires, &flags(&["b"]), &story));
    }

    #[test]
    fn every_leaf_of_a_group_must_hold() {
        let requires = Requires {
            flags: texts(&["a"]),
            not_before_day: Some(5),
            ..Requires::default()
        };
        let story = StoryState {
            day: 5,
            ..StoryState::default()
        };
        assert!(check(&requires, &flags(&["a"]), &story));
        assert!(!check(&requires, &flags(&[]), &story));
        assert!(!check(&requires, &flags(&["a"]), &StoryState::default()));
    }
}
