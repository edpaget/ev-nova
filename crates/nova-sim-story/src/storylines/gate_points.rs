//! Each jump point's gate as travel reads it: its stability and its
//! owner's government, as play leaves the point.
//!
//! A jump point stands as the data writes it (`Storylines::points`) until
//! an effect changes it ([`StoryState::points`]), read field by field
//! ([`StoryState::point`]). Travel reads it through the point's gate, a
//! hypergate `spöb`: [`gate_points`] gives each gate's [`GatePoint`], and
//! the galaxy builds its lanes and stamps its sites from them
//! (`OverlaidGalaxy::set_gate_points`, in the `story` module).
//!
//! - The stability, as the gate's ([`GateStability::from`]): the galaxy
//!   combines each gate's with its link's, and a jump that isn't
//!   [open](GateStability::is_open) is refused.
//! - The owner, a faction id as text, as the gate's government: the
//!   faction's `gövt` (`Storylines::factions`). Travel reads a gate's
//!   government for its clearance, the NPCs' enemy test and the radar. A
//!   point with no owner, or one whose faction the data doesn't number,
//!   leaves the gate's own government; the loader never writes the
//!   latter.
//!
//! The stock game has no storylines, so no gate points: every stock gate
//! is stable and keeps its government.
//!
//! [`StoryState::points`]: super::state::StoryState::points
//! [`StoryState::point`]: super::state::StoryState::point

use std::collections::BTreeMap;

use nova_sim_catalog::storylines::Storylines;

use super::state::StoryState;
use crate::catalog::{GateStability, GovtId, StellarId};

/// A jump point's gate as travel reads it (see the module docs).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GatePoint {
    /// The point's stability now, as the gate's.
    pub stability: GateStability,
    /// The government of the faction holding it now, or `None` to leave
    /// the gate's own: no owner, or a faction the data doesn't number.
    pub govt: Option<GovtId>,
}

/// Each jump point's gate, by its `spöb`, as `story` leaves `storylines`'
/// points (see the module docs). Empty with no points.
#[must_use]
pub fn gate_points(storylines: &Storylines, story: &StoryState) -> BTreeMap<StellarId, GatePoint> {
    storylines
        .points
        .keys()
        .filter_map(|name| story.point(&storylines.points, name))
        .map(|now| {
            let govt = now
                .owner
                .and_then(|owner| storylines.factions.get(owner))
                .copied();
            (
                now.gate,
                GatePoint {
                    stability: GateStability::from(now.stability),
                    govt,
                },
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use nova_sim_catalog::storylines::{PointAt, PointStability};

    use super::*;
    use crate::storylines::state::PointChange;

    /// Two points: `hearth/road` at gate 140, the guild's, stable, and
    /// `verge/road` at 141, nobody's, unstable; the guild is `gövt` 128
    /// and the wardens 129.
    fn storylines() -> Storylines {
        let point = |gate, owner: Option<&str>, stability| PointAt {
            gate: StellarId(gate),
            owner: owner.map(str::to_owned),
            stability,
        };
        Storylines {
            points: BTreeMap::from([
                (
                    "hearth/road".to_owned(),
                    point(140, Some("guild"), PointStability::Stable),
                ),
                (
                    "verge/road".to_owned(),
                    point(141, None, PointStability::Unstable),
                ),
            ]),
            factions: BTreeMap::from([
                ("guild".to_owned(), GovtId(128)),
                ("warden".to_owned(), GovtId(129)),
            ]),
            ..Storylines::default()
        }
    }

    fn at(stability: GateStability, govt: Option<i16>) -> GatePoint {
        GatePoint {
            stability,
            govt: govt.map(GovtId),
        }
    }

    #[test]
    fn untouched_points_give_the_datas_stabilities_and_owners() {
        assert_eq!(
            gate_points(&storylines(), &StoryState::default()),
            BTreeMap::from([
                (StellarId(140), at(GateStability::Stable, Some(128))),
                (StellarId(141), at(GateStability::Unstable, None)),
            ])
        );
    }

    #[test]
    fn a_change_overrides_the_stability_and_the_owner() {
        let story = StoryState {
            points: BTreeMap::from([
                (
                    "hearth/road".to_owned(),
                    PointChange {
                        owner: Some("warden".to_owned()),
                        stability: Some(PointStability::Dead),
                    },
                ),
                (
                    "verge/road".to_owned(),
                    PointChange {
                        owner: None,
                        stability: Some(PointStability::Stable),
                    },
                ),
            ]),
            ..StoryState::default()
        };
        assert_eq!(
            gate_points(&storylines(), &story),
            BTreeMap::from([
                (StellarId(140), at(GateStability::Dead, Some(129))),
                (StellarId(141), at(GateStability::Stable, None)),
            ])
        );
    }

    #[test]
    fn an_owner_with_no_faction_leaves_the_gates_own_government() {
        let story = StoryState {
            points: BTreeMap::from([(
                "verge/road".to_owned(),
                PointChange {
                    owner: Some("nobody".to_owned()),
                    stability: None,
                },
            )]),
            ..StoryState::default()
        };
        assert_eq!(
            gate_points(&storylines(), &story)[&StellarId(141)],
            at(GateStability::Unstable, None)
        );
    }

    #[test]
    fn a_change_to_a_point_the_data_lacks_gives_nothing() {
        let story = StoryState {
            points: BTreeMap::from([(
                "far/road".to_owned(),
                PointChange {
                    owner: None,
                    stability: Some(PointStability::Dead),
                },
            )]),
            ..StoryState::default()
        };
        assert_eq!(gate_points(&storylines(), &story).len(), 2);
    }

    #[test]
    fn no_storylines_give_no_gate_points() {
        assert!(gate_points(&Storylines::default(), &StoryState::default()).is_empty());
    }
}
