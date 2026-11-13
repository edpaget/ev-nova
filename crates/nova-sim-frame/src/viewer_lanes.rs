//! The links the developer galaxy viewer draws, by `RuleKey::GateLanes`
//! and `RuleKey::MapLaneStyle`, as plain data and the pure rule that
//! picks them: a fresh pilot's gate lanes ([`GateLinks`]), what the
//! viewer draws ([`ViewerLinks`]) and the pick by the rulebook
//! ([`viewer_links`]). From `nova_sim::viewer_lanes`, which re-exports
//! them and keeps the builder that reads a fresh story's gate lanes
//! (`nova_sim::viewer_lanes::gate_links`, which the app calls and hands
//! the viewer), with the module docs on what each reading draws.

use nova_sim_base::rulebook::{RuleKey, RuleSource, Rulebook};
use nova_sim_catalog::catalog::SystemId;

use crate::map_lanes::LaneLook;

/// A fresh pilot's gate lanes, rule-free: the data [`viewer_links`]
/// chooses from.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GateLinks {
    /// The gate-lane links, each pair once, lower ID first, sorted.
    pub links: Vec<(SystemId, SystemId)>,
    /// One look per link, in the same order.
    pub looks: Vec<LaneLook>,
    /// The closed lanes' looks: each pair once, none a link.
    pub closed: Vec<LaneLook>,
}

/// What the viewer draws: its links, each link's look (empty to draw
/// them all grey) and the closed lanes.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ViewerLinks {
    /// The links, each pair once.
    pub links: Vec<(SystemId, SystemId)>,
    /// One look per link, in order, or none.
    pub lanes: Vec<LaneLook>,
    /// The closed lanes' looks.
    pub closed_lanes: Vec<LaneLook>,
}

/// What the viewer draws by `rules`, from the Con links `con` and the
/// gate lanes `gates` (see `nova_sim::viewer_lanes`' module docs).
#[must_use]
pub fn viewer_links(
    con: &[(SystemId, SystemId)],
    gates: &GateLinks,
    rules: &Rulebook,
) -> ViewerLinks {
    if rules.source_for(RuleKey::GateLanes) == RuleSource::Engine {
        return ViewerLinks {
            links: con.to_vec(),
            ..ViewerLinks::default()
        };
    }
    let styled = rules.source_for(RuleKey::MapLaneStyle) == RuleSource::Bible;
    ViewerLinks {
        links: gates.links.clone(),
        lanes: if styled {
            gates.looks.clone()
        } else {
            Vec::new()
        },
        closed_lanes: if styled {
            gates.closed.clone()
        } else {
            Vec::new()
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map_lanes::Back;
    use nova_sim_catalog::catalog::GateStability;

    fn id(n: i16) -> SystemId {
        SystemId(n)
    }

    fn look(from: i16, to: i16) -> LaneLook {
        LaneLook {
            from: id(from),
            to: id(to),
            out: Some(GateStability::Stable),
            back: Back::OneWay,
        }
    }

    fn gates() -> GateLinks {
        GateLinks {
            links: vec![(id(1), id(2))],
            looks: vec![look(1, 2)],
            closed: vec![look(2, 3)],
        }
    }

    const CON: [(SystemId, SystemId); 1] = [(SystemId(5), SystemId(6))];

    fn rules(lanes: RuleSource, style: RuleSource) -> Rulebook {
        Rulebook::default()
            .with_override(RuleKey::GateLanes, lanes)
            .with_override(RuleKey::MapLaneStyle, style)
    }

    #[test]
    fn the_engine_reading_draws_the_con_links_grey() {
        let drawn = viewer_links(&CON, &gates(), &Rulebook::default());
        assert_eq!(
            drawn,
            ViewerLinks {
                links: CON.to_vec(),
                ..ViewerLinks::default()
            }
        );
        let explicit = viewer_links(
            &CON,
            &gates(),
            &rules(RuleSource::Engine, RuleSource::Engine),
        );
        assert_eq!(explicit, drawn);
    }

    #[test]
    fn the_lane_style_alone_still_draws_the_con_links_and_no_closed_lanes() {
        let drawn = viewer_links(
            &CON,
            &gates(),
            &rules(RuleSource::Engine, RuleSource::Bible),
        );
        assert_eq!(
            drawn,
            ViewerLinks {
                links: CON.to_vec(),
                ..ViewerLinks::default()
            }
        );
    }

    #[test]
    fn the_gate_lanes_alone_draw_the_gate_links_grey() {
        let drawn = viewer_links(
            &CON,
            &gates(),
            &rules(RuleSource::Bible, RuleSource::Engine),
        );
        assert_eq!(
            drawn,
            ViewerLinks {
                links: gates().links,
                ..ViewerLinks::default()
            }
        );
    }

    #[test]
    fn both_keys_draw_the_gate_links_in_their_looks_and_the_closed_lanes() {
        let drawn = viewer_links(&CON, &gates(), &rules(RuleSource::Bible, RuleSource::Bible));
        assert_eq!(
            drawn,
            ViewerLinks {
                links: gates().links,
                lanes: gates().looks,
                closed_lanes: gates().closed,
            }
        );
    }
}
