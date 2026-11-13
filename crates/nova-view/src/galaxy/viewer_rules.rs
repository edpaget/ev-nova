//! The viewer's links by the rules: [`GalaxyMap::with_gate_links`] and
//! [`GalaxyMap::with_rules`].
//!
//! The viewer is built before any rulebook is known. Its catalog hands it
//! the `sÿst` Con links, and whoever builds it hands it a fresh pilot's
//! gate lanes ([`GalaxyMap::with_gate_links`]): the app passes
//! `nova_sim::viewer_lanes::gate_links`, which reads the story, so the
//! catalog can't. Once the rulebook arrives,
//! `nova_sim::viewer_lanes::viewer_links` picks the links by
//! `RuleKey::GateLanes` and their looks and the closed lanes by
//! `RuleKey::MapLaneStyle`; this module forwards that one call and
//! rebuilds the viewer's picture from it. The course and hypergate maps
//! take their links from the session's drawing, so neither the gate lanes
//! nor the rules change anything there.

use nova_sim::Rulebook;
use nova_sim::galaxy_map::MapLink;

use super::catalog::GateLinks;
use super::map::{GalaxyMap, MapMode};

impl GalaxyMap {
    /// The map with `gates` as a fresh pilot's gate lanes, which the
    /// viewer draws by the rules (see the module docs): none until
    /// handed in. Hand them in before the rules
    /// ([`with_rules`](Self::with_rules)), which pick the links drawn.
    #[must_use]
    pub fn with_gate_links(mut self, gates: GateLinks) -> Self {
        self.model.set_gate_links(gates);
        self
    }

    /// The viewer with its links drawn by `rules` (see the module docs);
    /// an in-flight map unchanged.
    #[must_use]
    pub fn with_rules(mut self, rules: &Rulebook) -> Self {
        if self.mode != MapMode::Viewer {
            return self;
        }
        let drawn = self.model.apply_viewer_rules(rules);
        if let Some(picture) = &mut self.picture {
            picture.links = drawn
                .links
                .iter()
                .map(|&(from, to)| MapLink {
                    from,
                    to,
                    selected: false,
                })
                .collect();
            picture.lanes = drawn.lanes;
            picture.closed_lanes = drawn.closed_lanes;
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use nova_sim::catalog::GateStability;
    use nova_sim::map_lanes::{Back, LaneLook};
    use nova_sim::{RuleKey, RuleSource};

    use super::*;
    use crate::draw::{DrawCommand, DrawList};
    use crate::galaxy::catalog::{GateLinks, SystemId};
    use crate::galaxy::lanes;
    use crate::galaxy::map::MapPicture;
    use crate::galaxy::testkit::{FakeCatalog, galaxy};
    use crate::screen::Screen;

    fn look(from: i16, to: i16, out: GateStability) -> LaneLook {
        LaneLook {
            from: SystemId(from),
            to: SystemId(to),
            out: Some(out),
            back: Back::Way(Some(out)),
        }
    }

    /// The testkit's galaxy, whose Con links join Alpha (128) to Beta
    /// (129) and Gamma (130).
    fn catalog() -> FakeCatalog {
        FakeCatalog::new(galaxy())
    }

    /// A stable gate lane from Alpha to Gamma and a dead closed one from
    /// Beta to Gamma.
    fn gates() -> GateLinks {
        GateLinks {
            links: vec![(SystemId(128), SystemId(130))],
            looks: vec![look(128, 130, GateStability::Stable)],
            closed: vec![look(129, 130, GateStability::Dead)],
        }
    }

    /// The viewer of [`catalog`], handed [`gates`].
    fn viewer() -> GalaxyMap {
        GalaxyMap::new(&catalog()).with_gate_links(gates())
    }

    fn new_game() -> Rulebook {
        Rulebook::default()
            .with_override(RuleKey::GateLanes, RuleSource::Bible)
            .with_override(RuleKey::MapLaneStyle, RuleSource::Bible)
    }

    fn drawn(map: &GalaxyMap) -> DrawList {
        let mut list = DrawList::new();
        map.draw(&mut list);
        list
    }

    fn colored(list: &DrawList, of: crate::Color) -> usize {
        list.iter()
            .filter(|command| matches!(command, DrawCommand::Line { color, .. } if *color == of))
            .count()
    }

    #[test]
    fn the_stock_rules_leave_the_viewers_picture_as_it_was() {
        let before = viewer();
        let after = viewer().with_rules(&Rulebook::default());
        assert_eq!(
            after.picture(),
            Some(&MapPicture::viewer(&before.model)),
            "today's viewer picture"
        );
        assert_eq!(after.picture(), before.picture());
        assert_eq!(after.model.links(), before.model.links());
    }

    #[test]
    fn the_new_games_rules_draw_the_gate_lanes_in_their_looks() {
        let map = viewer().with_rules(&new_game());
        let picture = map.picture().expect("the viewer's");
        let links: Vec<(SystemId, SystemId)> = picture
            .links
            .iter()
            .map(|link| (link.from, link.to))
            .collect();
        assert_eq!(links, [(SystemId(128), SystemId(130))]);
        assert_eq!(picture.lanes, [look(128, 130, GateStability::Stable)]);
        assert_eq!(picture.closed_lanes, [look(129, 130, GateStability::Dead)]);
        assert_eq!(map.model.links(), [(SystemId(128), SystemId(130))]);
    }

    #[test]
    fn the_catalog_alone_hands_the_viewer_no_gate_lanes() {
        let map = GalaxyMap::new(&catalog()).with_rules(&new_game());
        let picture = map.picture().expect("the viewer's");
        assert_eq!(picture.links, []);
        assert_eq!(picture.lanes, []);
        assert_eq!(picture.closed_lanes, []);
        assert_eq!(map.model.links(), []);
    }

    #[test]
    fn a_gate_lane_is_drawn_in_its_stabilitys_colour() {
        let stock = drawn(&viewer());
        let laned = drawn(&viewer().with_rules(&new_game()));
        assert_eq!(colored(&stock, lanes::STABLE), 0);
        assert!(colored(&laned, lanes::STABLE) > 0);
        assert!(colored(&laned, lanes::DEAD) > 0, "the closed lane");
    }

    #[test]
    fn a_course_maps_links_are_untouched_by_the_rules() {
        let before = GalaxyMap::course(&catalog()).with_gate_links(gates());
        let after = GalaxyMap::course(&catalog())
            .with_gate_links(gates())
            .with_rules(&new_game());
        assert_eq!(after.model.links(), before.model.links());
        assert_eq!(after.picture(), before.picture());
    }
}
