//! What the galaxy map draws, as the original's `_DrawMap`
//! (@0xe568-0xf5d0) decides it: the course, the links, the systems with
//! their colour codes and mission arrows, the current system, the
//! hypergate's arrows and the names ([`draw_map`], [`MapDrawing`]). The
//! view decides pixels, colours, pens and the order it draws them in;
//! this decides what is drawn. The session works its inputs out
//! (`Session::map_drawing`).
//!
//! Addresses are in `EV Nova.app/Contents/MacOS/EV Nova` (i386).
//!
//! # `_DrawMap`, in order
//!
//! 1. **The course**, off the hypergate map only (@0xe5e3): each pair of
//!    consecutive `_hyperRoute` entries, both ≠ −1, as raw IDs, never
//!    resolved by place (@0xe661-0xe7a2). The route's first entry is the
//!    course's start, slot 0 ([`MapState::course_start`]); with none (−1)
//!    the first pair is skipped and the line starts at the first hop. The
//!    player's position is never an end: after an `M` or `N` keeps the
//!    course, the line still starts at the system it was started from
//!    ([`MapDrawing::course`]).
//! 2. **The current system's links** (@0xe7a8-0xe986): each of its 16
//!    Con slots resolved by place (`_FindActiveCoLocatedSystem`), the
//!    known and explored flags never read ([`MapDrawing::current_links`]).
//!    Then the current system is flagged done (`_doneFlag`, @0xe9b0).
//! 3. **Every other link** (@0xe9da-0xebd2): each system, by ascending
//!    ID, that is visible (+0x1eb), explored (+0x90 > 0) and known
//!    (+0x1ec), draws each Con slot resolved by place to a system not yet
//!    flagged done, then is flagged done itself (@0xebb4). So a link
//!    listed only by the higher of its systems, after the lower one
//!    qualified, is never drawn, nor one listed only by another system
//!    into the current one: [`RuleKey::MapLinkPairing`]'s engine reading.
//!    By its other reading every link a qualifying system lists is drawn
//!    once. The current system's slot +0x6c in mode 3 is the selected one
//!    (`_dimGreen`, @0xea65-0xea86; [`MapLink::selected`]).
//! 4. **The mode-3 link** again (@0xebd8-0xed31): in mode 3 with +0x6c
//!    0-15, that Con slot of the current system resolved by place
//!    ([`MapDrawing::mode3_link`]).
//! 5. **The systems** (@0xed52-0xf077), by ascending ID: each visible one
//!    (+0x1eb) that exists (+0x1ed, the `sÿst` loaded) and is the current
//!    system (@0xed88); or explored itself (its own +0x90 > 0) with a
//!    system standing for it (@0xed97-0xedb8); or known (@0xedd8); or at
//!    the place of a `_missionSyst` entry, compared by the two places'
//!    heads (`_FindFirstCoLocatedSystem`, @0xede9-0xee25), which sets its
//!    red arrow; or at `_greenMissionSyst`'s place, compared the same way
//!    (@0xee27-0xee66), which sets its green arrow and clears its red one
//!    unless `_multiArrowsOnSingleSystem` is set ([`DrawnSystem`]). Its
//!    oval is framed in `_ColorCodeSystem`'s colour (@0xef8a-0xefad;
//!    [`iff::system_color_code`](crate::iff::system_color_code)).
//! 6. **The current system's dot** (@0xf07d-0xf190), with no visible
//!    test ([`MapDrawing::current`]).
//! 7. **The hypergate's arrows**, on the hypergate map with a gate
//!    (@0xf195-0xf499): one to each system the gate's links lead to, each
//!    link's stellar's owner (+0x14), else `_FindSystemFromStellar`,
//!    resolved by place ([`gate::hypergate_offers`]). Drawing one marks its
//!    system known (@0xf457-0xf45f; [`Session::mark_known`]).
//! 8. **The names** (@0xf49a-0xf5c0): each visible, explored system's
//!    ([`MapDrawing::named`]); +0x1ed and the known flag aren't read. The
//!    view draws every one at a zoom ≤ 1.1 and only the selection's above.
//!
//! `_MapDialogUpdate` then brackets the selection and draws the names a
//! second time, on the course and hypergate maps only
//! (@0xfc9c-0x1001f): the view's.
//!
//! The function plays no sound (no `PlaySound` or `_beepSnd` among its
//! calls) and makes no random draw.
//!
//! # `_RecalcMissionSystsForMap` (@0xe1b5-0xe2d1)
//!
//! The mission systems the red arrows mark (`_missionSyst`, 32 words):
//! cleared to −1, then for each of the 16 mission slots held, the travel
//! stellar's owner (+0x14) when the stellar is ≥ 0 and active, replaced
//! by the return stellar's owner when the return stellar is ≥ 0, is not
//! the travel stellar (raw IDs), the destination has been reached
//! (objectives +1) and it is active; pushed unless the mission's `Flags`
//! has [`HIDES_ARROW`] (@0xe1e1-0xe274). Then, with [`MARKS_SHIPS`] and
//! not [`HIDES_ARROW`], ships left (+0x6) and a ship system (+0x10) ≥ 0,
//! the system standing for the ship system is pushed too, when there is
//! one (@0xe278-0xe2b0). `_DoSystemMap` (@0x124a0) and `_ShowFloatingMap`
//! (@0x114ea) run it as each map opens (`Session::map_mission_systems`).
//!
//! `_greenMissionSyst` and `_multiArrowsOnSingleSystem` ([`MapMarks`])
//! are written only by the mission dialogs' maps, which set them and put
//! them back before returning (to −1 and to 1), and by `_InitMissions`
//! (@0x97bc9-0x97bd8) and `_ResetPlayer` (@0x1d8f5): on the in-flight
//! maps there is no green system, with multiple arrows allowed
//! ([`MapMarks::NONE`]). The offer, the mission computer, mission info
//! and the accept's briefing each give their map its marks
//! (`Session::offer_map_marks` and its kin), and the map passes them to
//! the drawing (`Session::map_drawing`) and to the click
//! (`Session::map_click`), which accepts the green system as it does a
//! mission system (@0x12f82-0x12f8a).
//!
//! [`RuleKey::MapLinkPairing`]: crate::RuleKey::MapLinkPairing
//! [`gate::hypergate_offers`]: crate::gate::hypergate_offers
//! [`Session::mark_known`]: crate::Session::mark_known

use std::collections::BTreeSet;

use crate::catalog::{GovtId, NebulaId, SystemId};
use crate::flight::facing;
use crate::geometry::Vec2;
use crate::iff::SystemColorCode;
use crate::rulebook::RuleSource;
use crate::story::OverlaidGalaxy;
use crate::trig::intercept_angle;

/// How many Con slots a system has, `Con1-Con16`.
const CON_SLOTS: usize = 16;
/// How many entries `_hyperRoute` has.
const ROUTE_ENTRIES: usize = 32;

/// The mission `Flags` bit that keeps the mission's systems off the map's
/// arrows (`_RecalcMissionSystsForMap` @0xe25e, @0xe27d).
pub const HIDES_ARROW: u16 = 0x0002;
/// The mission `Flags` bit that marks the special ships' system with an
/// arrow too (@0xe278).
pub const MARKS_SHIPS: u16 = 0x0200;
/// The mission `Flags` bit that marks the mission's destination with the
/// green arrow on the map its dialog opens (`mïsn` +0x50 bit 8, read as
/// the copy's +0x19 or +0x59 bit 0): by the offer (`_OfferOneMission`
/// @0xa307c), the mission computer (`_DoMissionDialog` @0xa3d0d), mission
/// info (`_DoMissionInfoDialog` @0x9e40c) and the accept's briefing
/// (`_DoMissionAccept` @0xa1ea8). Nothing else reads it.
pub const MARKS_DESTINATION: u16 = 0x0100;

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
}

/// What the galaxy map's model shows of the galaxy as overlaid, as plain
/// data ([`GalaxyShown::of`]): which systems are drawn and hit, each
/// one's government, so its colour, the hyperlinks and the nebulae drawn.
/// The places are the catalog's, which the view's model is built from,
/// so it holds none. The default shows nothing: a flight that failed to
/// start answers no galaxy.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GalaxyShown {
    /// The visible systems by ascending ID, each with its overlaid
    /// government ([`OverlaidGalaxy::govt`]), `None` for an independent
    /// one.
    pub systems: Vec<(SystemId, Option<GovtId>)>,
    /// The overlaid map's links, as
    /// [`StarMap::links`](crate::StarMap::links) gives them: every pair
    /// once, lower ID first, sorted.
    pub links: Vec<(SystemId, SystemId)>,
    /// The nebulae shown, active and explored, by ascending ID
    /// ([`OverlaidGalaxy::shown_nebulae`]).
    pub nebulae: Vec<NebulaId>,
}

impl GalaxyShown {
    /// What the map shows of `galaxy` (see [`GalaxyShown`]).
    #[must_use]
    pub fn of(galaxy: &OverlaidGalaxy) -> Self {
        Self {
            systems: galaxy
                .catalog_systems()
                .filter(|&id| galaxy.visible(id))
                .map(|id| (id, galaxy.govt(id)))
                .collect(),
            links: galaxy.map().links(),
            nebulae: galaxy.shown_nebulae().collect(),
        }
    }

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

/// What [`draw_map`] reads: the galaxy, the player and the map.
#[derive(Clone, Copy, Debug)]
pub struct MapState<'a> {
    /// The galaxy as overlaid: its catalog's systems, which are visible,
    /// their Con slots and places.
    pub galaxy: &'a OverlaidGalaxy,
    /// The systems explored (+0x90 > 0).
    pub explored: &'a BTreeSet<SystemId>,
    /// The systems known (+0x1ec).
    pub known: &'a BTreeSet<SystemId>,
    /// The system the player is in.
    pub current: SystemId,
    /// The course's start, `_hyperRoute` slot 0: the system it was last
    /// started or advanced from (`Pilot::course_start`). `None` is −1,
    /// which skips the first pair.
    pub course_start: Option<SystemId>,
    /// The course's hops after its start, raw: slots 1 on.
    pub course: &'a [SystemId],
    /// The current system's Con slot selected in mode 3 (+0x6c), if any.
    pub hyper_slot: Option<usize>,
    /// The systems the hypergate's links lead to, on the hypergate map;
    /// `None` off it.
    pub hypergate: Option<&'a [SystemId]>,
    /// The mission systems (`_missionSyst`), raw.
    pub missions: &'a [SystemId],
    /// The green mission system (`_greenMissionSyst`), if any.
    pub green: Option<SystemId>,
    /// Whether a system with a green arrow keeps its red one
    /// (`_multiArrowsOnSingleSystem`).
    pub multi_arrows: bool,
    /// How links are paired ([`RuleKey::MapLinkPairing`](crate::RuleKey::MapLinkPairing)).
    pub pairing: RuleSource,
}

impl MapState<'_> {
    /// The system standing at the place `slot` names: none for a system
    /// the catalog doesn't hold or with none visible at its place
    /// (`_FindActiveCoLocatedSystem` @0x4be0).
    fn resolve(&self, slot: SystemId) -> Option<SystemId> {
        if self.galaxy.exists(slot) {
            self.galaxy.same_place(slot)
        } else {
            None
        }
    }

    /// The head of `system`'s place, itself when it is at none
    /// (`_FindFirstCoLocatedSystem` @0x4baa).
    fn head(&self, system: SystemId) -> SystemId {
        self.galaxy.map().places().head(system).unwrap_or(system)
    }

    /// `system`'s first 16 Con slots, raw.
    fn slots(&self, system: SystemId) -> impl Iterator<Item = SystemId> + '_ {
        self.galaxy
            .con_slots(system)
            .iter()
            .copied()
            .take(CON_SLOTS)
    }
}

/// What `_DrawMap` draws of `state` (see the module docs), each system
/// drawn framed in what `code` gives it.
pub fn draw_map(
    state: &MapState<'_>,
    mut code: impl FnMut(SystemId) -> SystemColorCode,
) -> MapDrawing {
    let galaxy = state.galaxy;
    let current = state.current;
    let course = if state.hypergate.is_some() {
        Vec::new()
    } else {
        let route: Vec<Option<SystemId>> = std::iter::once(state.course_start)
            .chain(state.course.iter().copied().map(Some))
            .take(ROUTE_ENTRIES)
            .collect();
        route
            .windows(2)
            .filter_map(|pair| Some((pair[0]?, pair[1]?)))
            .collect()
    };
    let current_links = state
        .slots(current)
        .filter_map(|slot| state.resolve(slot))
        .map(|to| (current, to))
        .collect();
    let mut links = Vec::new();
    let mut done = BTreeSet::from([current]);
    let mut paired = BTreeSet::new();
    for from in galaxy.catalog_systems() {
        if !galaxy.visible(from) || !state.explored.contains(&from) || !state.known.contains(&from)
        {
            continue;
        }
        for (slot, to) in state.slots(from).enumerate() {
            let Some(to) = state.resolve(to) else {
                continue;
            };
            let drawn = match state.pairing {
                RuleSource::Engine => !done.contains(&to),
                RuleSource::Bible => paired.insert((from.min(to), from.max(to))),
            };
            if drawn {
                links.push(MapLink {
                    from,
                    to,
                    selected: from == current && state.hyper_slot == Some(slot),
                });
            }
        }
        done.insert(from);
    }
    let mode3_link = state
        .hyper_slot
        .and_then(|slot| state.slots(current).nth(slot))
        .and_then(|slot| state.resolve(slot))
        .map(|to| (current, to));
    let mut systems = Vec::new();
    for id in galaxy.catalog_systems().filter(|&id| galaxy.visible(id)) {
        let head = state.head(id);
        let mut red_arrow = state.missions.iter().any(|&m| state.head(m) == head);
        let green_arrow = state.green.is_some_and(|green| state.head(green) == head);
        if green_arrow && !state.multi_arrows {
            red_arrow = false;
        }
        let shown = id == current
            || (galaxy.same_place(id).is_some() && state.explored.contains(&id))
            || state.known.contains(&id);
        if shown || red_arrow || green_arrow {
            systems.push(DrawnSystem {
                id,
                code: code(id),
                red_arrow,
                green_arrow,
            });
        }
    }
    let named = galaxy
        .catalog_systems()
        .filter(|&id| galaxy.visible(id) && state.explored.contains(&id))
        .collect();
    MapDrawing {
        hypergate: state.hypergate.is_some(),
        course,
        current_links,
        links,
        mode3_link,
        systems,
        current,
        gate_arrows: state
            .hypergate
            .map(<[SystemId]>::to_vec)
            .unwrap_or_default(),
        named,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::story::GalaxyOverlay;
    use crate::testkit::star;

    fn ids(list: &[i16]) -> Vec<SystemId> {
        list.iter().copied().map(SystemId).collect()
    }

    fn set(list: &[i16]) -> BTreeSet<SystemId> {
        list.iter().copied().map(SystemId).collect()
    }

    /// `systems` with those in `hidden` hidden.
    fn galaxy(systems: Vec<crate::catalog::StarSystem>, hidden: &[i16]) -> OverlaidGalaxy {
        let mut overlay = GalaxyOverlay::default();
        overlay.set_hidden(&set(hidden));
        OverlaidGalaxy::new(systems, overlay)
    }

    /// A state of `galaxy` in 130, every other input empty.
    fn state<'a>(
        galaxy: &'a OverlaidGalaxy,
        explored: &'a BTreeSet<SystemId>,
        known: &'a BTreeSet<SystemId>,
    ) -> MapState<'a> {
        MapState {
            galaxy,
            explored,
            known,
            current: SystemId(130),
            course_start: None,
            course: &[],
            hyper_slot: None,
            hypergate: None,
            missions: &[],
            green: None,
            multi_arrows: true,
            pairing: RuleSource::Engine,
        }
    }

    fn uncoded(_: SystemId) -> SystemColorCode {
        SystemColorCode::Uninhabited
    }

    fn drawn_ids(drawing: &MapDrawing) -> Vec<SystemId> {
        drawing.systems.iter().map(|system| system.id).collect()
    }

    fn link_pairs(drawing: &MapDrawing) -> Vec<(i16, i16)> {
        drawing
            .links
            .iter()
            .map(|link| (link.from.0, link.to.0))
            .collect()
    }

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

    // The systems.

    /// 130 (current) and 131-136 apart; 137 hidden; 138 at 139's place
    /// (138 the head), 139 visible.
    fn spread() -> OverlaidGalaxy {
        galaxy(
            vec![
                star(130, (0.0, 0.0), &[]),
                star(131, (1.0, 0.0), &[]),
                star(132, (2.0, 0.0), &[]),
                star(133, (3.0, 0.0), &[]),
                star(134, (4.0, 0.0), &[]),
                star(135, (5.0, 0.0), &[]),
                star(136, (6.0, 0.0), &[]),
                star(137, (7.0, 0.0), &[]),
                star(138, (8.0, 0.0), &[]),
                star(139, (8.0, 0.0), &[]),
            ],
            &[137],
        )
    }

    #[test]
    fn a_system_is_drawn_current_explored_known_or_at_a_mission_place() {
        let galaxy = spread();
        let (explored, known) = (set(&[131, 137]), set(&[132, 137]));
        let missions = ids(&[133]);
        let state = MapState {
            missions: &missions,
            green: Some(SystemId(134)),
            ..state(&galaxy, &explored, &known)
        };
        let drawing = draw_map(&state, uncoded);
        assert_eq!(
            drawn_ids(&drawing),
            ids(&[130, 131, 132, 133, 134]),
            "current, explored, known, red and green; 135 and 136 neither, \
             137 hidden"
        );
    }

    #[test]
    fn an_explored_system_with_no_stand_in_is_not_drawn_for_it() {
        // 131 hidden at 132's place: none stands for 131, so it is hidden
        // and never drawn; but 132 explored, standing for itself, is.
        let galaxy = galaxy(
            vec![
                star(130, (0.0, 0.0), &[]),
                star(131, (5.0, 5.0), &[]),
                star(132, (9.0, 9.0), &[]),
            ],
            &[131],
        );
        let (explored, known) = (set(&[131, 132]), set(&[]));
        let drawing = draw_map(&state(&galaxy, &explored, &known), uncoded);
        assert_eq!(drawn_ids(&drawing), ids(&[130, 132]));
    }

    #[test]
    fn an_unknown_unexplored_system_is_left_out() {
        let galaxy = spread();
        let (explored, known) = (set(&[]), set(&[]));
        let drawing = draw_map(&state(&galaxy, &explored, &known), uncoded);
        assert_eq!(drawn_ids(&drawing), ids(&[130]), "only the current one");
        assert_eq!(drawing.current, SystemId(130));
    }

    #[test]
    fn each_system_drawn_is_framed_in_its_code() {
        let galaxy = spread();
        let (explored, known) = (set(&[131]), set(&[132]));
        let drawing = draw_map(&state(&galaxy, &explored, &known), |id| {
            if id == SystemId(131) {
                SystemColorCode::Hostile
            } else {
                SystemColorCode::Cleared
            }
        });
        let codes: Vec<SystemColorCode> = drawing.systems.iter().map(|s| s.code).collect();
        assert_eq!(
            codes,
            [
                SystemColorCode::Cleared,
                SystemColorCode::Hostile,
                SystemColorCode::Cleared
            ]
        );
    }

    #[test]
    fn the_arrows_compare_the_places_heads() {
        // 139, a mission system, shares 138's place: both get the red
        // arrow; 140, not in the catalog, marks nothing.
        let galaxy = spread();
        let (explored, known) = (set(&[]), set(&[]));
        let missions = ids(&[139, 140]);
        let state = MapState {
            missions: &missions,
            ..state(&galaxy, &explored, &known)
        };
        let drawing = draw_map(&state, uncoded);
        let red: Vec<SystemId> = drawing
            .systems
            .iter()
            .filter(|system| system.red_arrow)
            .map(|system| system.id)
            .collect();
        assert_eq!(red, ids(&[138, 139]));
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

    #[test]
    fn a_green_arrow_clears_the_red_one_unless_multiple_arrows_are_allowed() {
        let galaxy = spread();
        let (explored, known) = (set(&[]), set(&[]));
        let missions = ids(&[133]);
        for multi_arrows in [true, false] {
            let state = MapState {
                missions: &missions,
                green: Some(SystemId(133)),
                multi_arrows,
                ..state(&galaxy, &explored, &known)
            };
            let drawing = draw_map(&state, uncoded);
            let system = drawing.systems[1];
            assert_eq!(system.id, SystemId(133));
            assert!(system.green_arrow);
            assert_eq!(system.red_arrow, multi_arrows);
        }
    }

    // The links.

    /// 130 (current) lists 131; 131 lists 132 and 140 (no system); 132
    /// lists 133; 133 lists nothing; 134, unexplored, lists 135.
    fn chain() -> OverlaidGalaxy {
        galaxy(
            vec![
                star(130, (0.0, 0.0), &[131]),
                star(131, (1.0, 0.0), &[132, 140]),
                star(132, (2.0, 0.0), &[133]),
                star(133, (3.0, 0.0), &[]),
                star(134, (4.0, 0.0), &[135]),
                star(135, (5.0, 0.0), &[]),
            ],
            &[],
        )
    }

    #[test]
    fn links_leave_from_explored_known_systems_and_may_reach_unexplored_ones() {
        let galaxy = chain();
        let (explored, known) = (set(&[131, 134]), set(&[131, 132, 133]));
        let drawing = draw_map(&state(&galaxy, &explored, &known), uncoded);
        assert_eq!(drawing.current_links, [(SystemId(130), SystemId(131))]);
        assert_eq!(
            link_pairs(&drawing),
            [(131, 132)],
            "into 132, unexplored; none from 132 (unexplored), 134 (unknown) \
             or 130 (unexplored, its links the current system's)"
        );
    }

    #[test]
    fn the_current_systems_links_are_drawn_whatever_is_known_or_explored() {
        let galaxy = chain();
        let (explored, known) = (set(&[]), set(&[]));
        let drawing = draw_map(&state(&galaxy, &explored, &known), uncoded);
        assert_eq!(drawing.current_links, [(SystemId(130), SystemId(131))]);
        assert!(drawing.links.is_empty());
    }

    /// 130 (current) lists 131; 131 lists 130 and 132; 132 lists 131 and
    /// 133; 133 lists 131, which doesn't list it back. All explored and
    /// known.
    fn ring() -> OverlaidGalaxy {
        galaxy(
            vec![
                star(130, (0.0, 0.0), &[131]),
                star(131, (1.0, 0.0), &[130, 132]),
                star(132, (2.0, 0.0), &[131, 133]),
                star(133, (3.0, 0.0), &[131]),
            ],
            &[],
        )
    }

    #[test]
    fn each_pair_is_drawn_once_and_a_link_listed_only_by_the_higher_id_never() {
        let galaxy = ring();
        let all = set(&[130, 131, 132, 133]);
        let drawing = draw_map(&state(&galaxy, &all, &all), uncoded);
        assert_eq!(
            link_pairs(&drawing),
            [(130, 131), (131, 132), (132, 133)],
            "131-130 meets the current system done; 133-131 meets 131 done"
        );
    }

    #[test]
    fn by_the_other_reading_every_link_listed_is_drawn_once() {
        let galaxy = ring();
        let all = set(&[130, 131, 132, 133]);
        let state = MapState {
            pairing: RuleSource::Bible,
            ..state(&galaxy, &all, &all)
        };
        let drawing = draw_map(&state, uncoded);
        assert_eq!(
            link_pairs(&drawing),
            [(130, 131), (131, 132), (132, 133), (133, 131)]
        );
    }

    #[test]
    fn the_current_systems_selected_slot_is_flagged_and_drawn_again() {
        // 130 lists 132 then 131 (its slot 1, selected); 131 is at 133's
        // place, hidden, so it stands for 133.
        let galaxy = galaxy(
            vec![
                star(130, (0.0, 0.0), &[132, 131]),
                star(131, (3.0, 0.0), &[]),
                star(132, (2.0, 0.0), &[]),
                star(133, (3.0, 0.0), &[]),
            ],
            &[131],
        );
        let all = set(&[130, 132, 133]);
        let state = MapState {
            hyper_slot: Some(1),
            ..state(&galaxy, &all, &all)
        };
        let drawing = draw_map(&state, uncoded);
        let selected: Vec<bool> = drawing.links.iter().map(|link| link.selected).collect();
        assert_eq!(link_pairs(&drawing), [(130, 132), (130, 133)]);
        assert_eq!(selected, [false, true]);
        assert_eq!(drawing.mode3_link, Some((SystemId(130), SystemId(133))));
        let state = MapState {
            hyper_slot: Some(5),
            ..state
        };
        assert_eq!(draw_map(&state, uncoded).mode3_link, None, "no such slot");
    }

    // The course, the gate and the names.

    #[test]
    fn the_course_is_drawn_raw_from_its_start() {
        // In 131, the course started from 130 and kept: the line still
        // starts at 130, never at the player. 133 is hidden: the course
        // still runs through it.
        let galaxy = galaxy(chain_systems(), &[133]);
        let (explored, known) = (set(&[]), set(&[]));
        let course = ids(&[131, 133, 140]);
        let state = MapState {
            current: SystemId(131),
            course_start: Some(SystemId(130)),
            course: &course,
            ..state(&galaxy, &explored, &known)
        };
        let drawing = draw_map(&state, uncoded);
        assert_eq!(
            drawing.course,
            [(130, 131), (131, 133), (133, 140)].map(|(a, b)| (SystemId(a), SystemId(b)))
        );
        assert!(!drawing.hypergate);
        assert!(drawing.gate_arrows.is_empty());
    }

    #[test]
    fn a_course_with_no_start_is_drawn_from_its_first_hop() {
        // Slot 0 is −1: the first pair is skipped, the rest drawn
        // (@0xe66a).
        let galaxy = galaxy(chain_systems(), &[]);
        let (explored, known) = (set(&[]), set(&[]));
        let course = ids(&[131, 133, 140]);
        let state = MapState {
            course: &course,
            ..state(&galaxy, &explored, &known)
        };
        assert_eq!(
            draw_map(&state, uncoded).course,
            [(131, 133), (133, 140)].map(|(a, b)| (SystemId(a), SystemId(b)))
        );
        let one = ids(&[131]);
        let state = MapState {
            course: &one,
            ..state
        };
        assert!(draw_map(&state, uncoded).course.is_empty());
    }

    #[test]
    fn the_course_is_capped_at_the_routes_32_entries() {
        let galaxy = galaxy(chain_systems(), &[]);
        let (explored, known) = (set(&[]), set(&[]));
        let course: Vec<SystemId> = (1..=40).map(SystemId).collect();
        let state = MapState {
            course_start: Some(SystemId(0)),
            course: &course,
            ..state(&galaxy, &explored, &known)
        };
        let drawn = draw_map(&state, uncoded).course;
        assert_eq!(drawn.len(), ROUTE_ENTRIES - 1);
        assert_eq!(drawn.first(), Some(&(SystemId(0), SystemId(1))));
        assert_eq!(drawn.last(), Some(&(SystemId(30), SystemId(31))));
    }

    fn chain_systems() -> Vec<crate::catalog::StarSystem> {
        vec![
            star(130, (0.0, 0.0), &[131]),
            star(131, (1.0, 0.0), &[132]),
            star(132, (2.0, 0.0), &[]),
            star(133, (3.0, 0.0), &[]),
        ]
    }

    #[test]
    fn the_hypergate_map_has_no_course_and_arrows_its_destinations() {
        let galaxy = galaxy(chain_systems(), &[]);
        let (explored, known) = (set(&[]), set(&[]));
        let course = ids(&[131]);
        let offered = ids(&[133, 132]);
        let state = MapState {
            course_start: Some(SystemId(130)),
            course: &course,
            hypergate: Some(&offered),
            ..state(&galaxy, &explored, &known)
        };
        let drawing = draw_map(&state, uncoded);
        assert!(drawing.hypergate);
        assert!(drawing.course.is_empty());
        assert_eq!(drawing.gate_arrows, offered);
    }

    #[test]
    fn every_visible_explored_system_is_named_known_or_not() {
        let galaxy = galaxy(chain_systems(), &[133]);
        let (explored, known) = (set(&[131, 133]), set(&[132]));
        let drawing = draw_map(&state(&galaxy, &explored, &known), uncoded);
        assert_eq!(
            drawing.named,
            ids(&[131]),
            "not 132, known, nor 133, hidden"
        );
        assert_eq!(drawn_ids(&drawing), ids(&[130, 131, 132]));
    }

    // What the map's model shows of the galaxy.

    /// 130, 131 and 132, linked in a ring, of governments 128, none and
    /// 128 in the catalog; 133 alone, of 128.
    fn governed_systems() -> Vec<crate::catalog::StarSystem> {
        let govern = |system: crate::catalog::StarSystem, govt| crate::catalog::StarSystem {
            govt,
            ..system
        };
        vec![
            govern(star(130, (0.0, 0.0), &[131, 132]), Some(GovtId(128))),
            govern(star(131, (10.0, 0.0), &[132]), None),
            govern(star(132, (20.0, 0.0), &[130]), Some(GovtId(128))),
            govern(star(133, (30.0, 0.0), &[]), Some(GovtId(128))),
        ]
    }

    #[test]
    fn the_galaxy_shown_leaves_out_a_hidden_system_and_takes_the_overlays_governments() {
        let mut overlay = GalaxyOverlay::default();
        overlay.set_visible(SystemId(131), Some(false));
        overlay.set_govt(SystemId(130), Some(Some(GovtId(129))));
        overlay.set_govt(SystemId(132), Some(None));
        let galaxy = OverlaidGalaxy::new(governed_systems(), overlay);
        let shown = GalaxyShown::of(&galaxy);
        assert_eq!(
            shown.systems,
            [
                (SystemId(130), Some(GovtId(129))),
                (SystemId(132), None),
                (SystemId(133), Some(GovtId(128))),
            ],
            "131 hidden; 130 and 132 the overlay's, 133 the catalog's"
        );
        assert!(shown.visible(SystemId(130)));
        assert!(!shown.visible(SystemId(131)), "hidden");
        assert!(!shown.visible(SystemId(140)), "not in the catalog");
        assert_eq!(shown.govt(SystemId(130)), Some(GovtId(129)));
        assert_eq!(shown.govt(SystemId(132)), None, "made independent");
        assert_eq!(shown.govt(SystemId(131)), None, "not drawn");
    }

    #[test]
    fn the_galaxy_shown_lists_the_overlaid_maps_links() {
        let mut overlay = GalaxyOverlay::default();
        overlay.set_visible(SystemId(131), Some(false));
        let galaxy = OverlaidGalaxy::new(governed_systems(), overlay);
        let links = galaxy.map().links();
        assert_eq!(
            links,
            [(SystemId(130), SystemId(132))],
            "precondition: 131's links gone with it"
        );
        assert_eq!(GalaxyShown::of(&galaxy).links, links);
    }

    #[test]
    fn the_galaxy_shown_lists_a_nebula_only_once_explored_and_while_active() {
        let mut overlay = GalaxyOverlay::default();
        overlay.set_nebula_explored(NebulaId(128), true);
        overlay.set_nebula_explored(NebulaId(129), true);
        overlay.set_nebula_active(NebulaId(129), Some(false));
        overlay.set_nebula_active(NebulaId(130), Some(true));
        let galaxy = OverlaidGalaxy::new(governed_systems(), overlay);
        let shown = GalaxyShown::of(&galaxy);
        assert_eq!(shown.nebulae, [NebulaId(128)]);
        assert!(shown.nebula_shown(NebulaId(128)));
        assert!(!shown.nebula_shown(NebulaId(129)), "inactive");
        assert!(!shown.nebula_shown(NebulaId(130)), "unexplored");
        assert_eq!(GalaxyShown::default().nebulae, [], "the default none");
    }
}
