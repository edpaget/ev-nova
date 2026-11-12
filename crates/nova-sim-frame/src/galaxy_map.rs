//! What the galaxy map draws, as plain data: the drawing `_DrawMap`
//! (@0xe568-0xf5d0) decides ([`MapDrawing`], with its [`MapLink`]s and
//! [`DrawnSystem`]s), the hypergate arrows' heads ([`arrow_head`]), what
//! the map's model shows of the galaxy as overlaid ([`GalaxyShown`]), and
//! what a map marks beside the mission systems ([`MapMarks`],
//! [`MARKS_DESTINATION`]). From `nova_sim::galaxy_map`, which re-exports
//! them and keeps the builders that read the overlaid galaxy:
//! `nova_sim::galaxy_map::draw_map` over its `MapState`, and
//! `GalaxyShown::of` (`nova_sim::galaxy_map::GalaxyShownOf`). Its module
//! docs walk `_DrawMap` in order.
//!
//! Addresses are in `EV Nova.app/Contents/MacOS/EV Nova` (i386).

use nova_sim_base::flight::facing;
use nova_sim_base::geometry::Vec2;
use nova_sim_base::trig::intercept_angle;
use nova_sim_catalog::catalog::{GovtId, NebulaId, SystemId};
use nova_sim_state::shown::iff::SystemColorCode;

use crate::map_lanes::LaneLook;

/// What a map marks beside the mission systems: the green mission system
/// (`_greenMissionSyst`) and whether a system with the green arrow keeps
/// its red one (`_multiArrowsOnSingleSystem`). Only the mission dialogs'
/// maps set them, each for the map it opens, and put them back before
/// returning; `_InitMissions` (@0x97bc9-0x97bd8) and `_ResetPlayer`
/// (@0x1d8f5) leave [`MapMarks::NONE`]. So ours holds neither between
/// maps: each map opens with the marks its opener gives, and every other
/// map with none.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MapMarks {
    /// The system the green arrow marks, raw, if any.
    pub green: Option<SystemId>,
    /// Whether the green system keeps its red arrow.
    pub multi_arrows: bool,
}

impl MapMarks {
    /// What the in-flight maps mark, and what the resets leave: no green
    /// system (−1), multiple arrows allowed (1).
    pub const NONE: Self = Self {
        green: None,
        multi_arrows: true,
    };
}

/// The mission `Flags` bit that marks the mission's destination with the
/// green arrow on the map its dialog opens (`mïsn` +0x50 bit 8, read as
/// the copy's +0x19 or +0x59 bit 0): by the offer (`_OfferOneMission`
/// @0xa307c), the mission computer (`_DoMissionDialog` @0xa3d0d), mission
/// info (`_DoMissionInfoDialog` @0x9e40c) and the accept's briefing
/// (`_DoMissionAccept` @0xa1ea8). Nothing else reads it.
pub const MARKS_DESTINATION: u16 = 0x0100;

/// How long the hypergate arrows' head strokes are (8.0, @0xf339).
pub const ARROW_HEAD: f32 = 8.0;

/// The ends of a hypergate arrow's two head strokes, drawn from its `tip`
/// (the system it points to) back toward its `tail` (the current system),
/// as `_DrawMap` works them out (@0xf371-0xf452): the heading from the
/// tip toward the tail ([`intercept_angle`], `_CalcInterceptAngle`), then
/// [`ARROW_HEAD`] from the tip along that heading plus 45 and along it
/// plus 315, mod 360 (`_Accel`). The points are the map's, unrounded; the
/// view truncates them as the original's `MoveTo` / `LineTo` do.
#[must_use]
pub fn arrow_head(tip: Vec2, tail: Vec2) -> [Vec2; 2] {
    let back = i32::from(intercept_angle(tip, tail));
    [back + 45, (back + 315) % 360].map(|angle| {
        // In 0..405.
        #[allow(clippy::cast_precision_loss)]
        let angle = angle as f32;
        tip + facing(angle) * ARROW_HEAD
    })
}

/// A link `_DrawMap` draws between two systems of the galaxy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MapLink {
    /// The system listing it.
    pub from: SystemId,
    /// The system standing at the place it names.
    pub to: SystemId,
    /// Whether it is the current system's Con slot selected in mode 3,
    /// drawn in `_dimGreen` (@0xea65-0xea86).
    pub selected: bool,
}

/// A system `_DrawMap` draws an oval for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DrawnSystem {
    /// The system.
    pub id: SystemId,
    /// What its oval is framed in.
    pub code: SystemColorCode,
    /// Whether a red mission arrow (`cicn` 15000) goes up and left of it.
    pub red_arrow: bool,
    /// Whether a green mission arrow (`cicn` 15001) goes up and right of
    /// it.
    pub green_arrow: bool,
}

/// What `_DrawMap` draws, in its order (see the module docs).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MapDrawing {
    /// Whether it is the hypergate map's: no course, and the links in
    /// `_rgbRealDarkGrey`.
    pub hypergate: bool,
    /// The course's segments, as raw IDs: none on the hypergate map.
    pub course: Vec<(SystemId, SystemId)>,
    /// The current system's links, each to the system standing at the
    /// place its Con slot names.
    pub current_links: Vec<(SystemId, SystemId)>,
    /// Every other link drawn, in the order drawn.
    pub links: Vec<MapLink>,
    /// The link to the system selected in mode 3, drawn again.
    pub mode3_link: Option<(SystemId, SystemId)>,
    /// The systems drawn, by ascending ID.
    pub systems: Vec<DrawnSystem>,
    /// The current system, whose dot is drawn over the systems.
    pub current: SystemId,
    /// The systems the hypergate's arrows point to from the current one.
    pub gate_arrows: Vec<SystemId>,
    /// The systems that may be named, by ascending ID: every visible,
    /// explored one.
    pub named: Vec<SystemId>,
    /// How each link of [`current_links`](Self::current_links), then of
    /// [`links`](Self::links), looks, one each in that order, by
    /// [`RuleKey::MapLaneStyle`](nova_sim_base::rulebook::RuleKey::MapLaneStyle)'s other
    /// reading ([`map_lanes`](crate::map_lanes)); empty by the engine's,
    /// which draws every link grey.
    pub lanes: Vec<LaneLook>,
    /// The closed gate lanes drawn beside the links, each in its look, by
    /// [`RuleKey::MapLaneStyle`](nova_sim_base::rulebook::RuleKey::MapLaneStyle)'s other
    /// reading: each pair once, none drawn as a link, none on the
    /// hypergate map ([`map_lanes`](crate::map_lanes)). Empty by the
    /// engine's.
    pub closed_lanes: Vec<LaneLook>,
}

/// What the galaxy map's model shows of the galaxy as overlaid, as plain
/// data (`GalaxyShown::of`, of `nova_sim::galaxy_map::GalaxyShownOf`,
/// builds it from the overlaid galaxy): which systems are drawn and hit, each
/// one's government, so its colour, the hyperlinks and the nebulae drawn.
/// The places are the catalog's, which the view's model is built from,
/// so it holds none. The default shows nothing: a flight that failed to
/// start answers no galaxy.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GalaxyShown {
    /// The visible systems by ascending ID, each with its overlaid
    /// government (`nova_sim::story::OverlaidGalaxy::govt`), `None` for an independent
    /// one.
    pub systems: Vec<(SystemId, Option<GovtId>)>,
    /// The overlaid map's links, as
    /// `nova_sim::StarMap::links` gives them: every pair
    /// once, lower ID first, sorted.
    pub links: Vec<(SystemId, SystemId)>,
    /// The nebulae shown, active and explored, by ascending ID
    /// (`nova_sim::story::OverlaidGalaxy::shown_nebulae`).
    pub nebulae: Vec<NebulaId>,
}

impl GalaxyShown {
    /// Whether `system` is drawn: whether it is among the visible
    /// systems.
    #[must_use]
    pub fn visible(&self, system: SystemId) -> bool {
        self.find(system).is_some()
    }

    /// `system`'s government as overlaid: `None` when it is independent
    /// or not drawn.
    #[must_use]
    pub fn govt(&self, system: SystemId) -> Option<GovtId> {
        self.find(system).and_then(|at| self.systems[at].1)
    }

    /// Whether `nebula` is drawn: active and explored.
    #[must_use]
    pub fn nebula_shown(&self, nebula: NebulaId) -> bool {
        self.nebulae.binary_search(&nebula).is_ok()
    }

    /// Where `system` stands among the visible systems, if it does.
    fn find(&self, system: SystemId) -> Option<usize> {
        self.systems
            .binary_search_by_key(&system, |&(id, _)| id)
            .ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // The hypergate arrows' heads.

    fn close(a: Vec2, b: Vec2) -> bool {
        (a - b).length() < 1e-4
    }

    #[test]
    fn an_arrows_head_strokes_go_back_from_its_tip_45_degrees_either_side() {
        let half = 8.0 * std::f32::consts::FRAC_1_SQRT_2;
        // Pointing up: the tail straight below the tip.
        let [left, right] = arrow_head(Vec2::new(10.0, 10.0), Vec2::new(10.0, 50.0));
        assert!(close(left, Vec2::new(10.0 - half, 10.0 + half)), "{left:?}");
        assert!(
            close(right, Vec2::new(10.0 + half, 10.0 + half)),
            "{right:?}"
        );
        // Pointing right: the tail to the left.
        let [up, down] = arrow_head(Vec2::new(10.0, 10.0), Vec2::new(-30.0, 10.0));
        assert!(close(up, Vec2::new(10.0 - half, 10.0 - half)), "{up:?}");
        assert!(close(down, Vec2::new(10.0 - half, 10.0 + half)), "{down:?}");
    }

    #[test]
    fn an_arrows_heading_is_the_tables_whole_degree() {
        // From (0, 0) toward (1, 1) the table gives 134, not 135, so the
        // strokes are at 179 and 89.
        let [first, second] = arrow_head(Vec2::new(0.0, 0.0), Vec2::new(1.0, 1.0));
        assert!(close(first, facing(179.0) * ARROW_HEAD), "{first:?}");
        assert!(close(second, facing(89.0) * ARROW_HEAD), "{second:?}");
    }

    #[test]
    fn no_marks_are_no_green_system_with_multiple_arrows_allowed() {
        assert_eq!(
            MapMarks::NONE,
            MapMarks {
                green: None,
                multi_arrows: true,
            }
        );
        assert_eq!(MARKS_DESTINATION, 0x0100);
    }
}
