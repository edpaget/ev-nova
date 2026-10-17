//! The map model: the galaxy laid out for drawing and clicking, built once
//! from a [`Galaxy`].
//!
//! The catalog gives the static geometry: each system's position and each
//! nebula's rectangle. A flight's session overrides the rest through its
//! galaxy overlay ([`GalaxyModel::apply`]; see `nova_sim`'s `story` module,
//! "Galaxy overlay"): which systems are drawn and hit, each one's
//! government and so its colour, the hyperlinks, and which nebulae are
//! shown: those active and explored. The override rules are the
//! session's; the model only reads them. Until one is applied, the
//! catalog's galaxy is drawn as it is, every nebula included (the browse
//! map, with no session).
//!
//! - Each system's colour comes from its own government's map colour;
//!   independent systems, and systems whose government is missing, are
//!   [`NEUTRAL`].
//! - The hyperlinks are the [`StarMap`]'s [`StarMap::links`]: every pair
//!   either system lists, drawn once and undirected, as the original's
//!   `_DrawMap` (@0xe7a8) draws them, while the session's jumps follow its
//!   `HyperlinkRule` (by the engine's, one way along each system's own
//!   links); a link to a missing system is reported as a problem.
//! - The nebulae are kept for drawing ([`GalaxyModel::nebulae`],
//!   [`GalaxyModel::catalog_nebulae`]); how each is drawn, its rect and
//!   its picture, is the [`nebulae`](super::nebulae) module's.
//! - A click selects the nearest system within [`HIT_RADIUS`]. Several
//!   systems can share one position (alternate versions of a system that
//!   the game swaps in later); repeated clicks there cycle through them.

use std::collections::{BTreeMap, BTreeSet};

use nova_sim::story::OverlaidGalaxy;
use nova_sim::{StarMap, StarSystem, Vec2};

use super::catalog::{Galaxy, GovtId, NebulaEntry, SystemEntry, SystemId};
use super::view::{Bounds, Transform};
use crate::{Color, Point};

/// Independent systems' colour: a light grey unlike any stock government's.
pub const NEUTRAL: Color = Color::rgba(192, 192, 192, 255);
/// How near a click must be to a system's dot to select it, in logical
/// units at every zoom.
pub const HIT_RADIUS: f32 = 8.0;

/// The colour of a system owned by `govt`: its map colour, or [`NEUTRAL`]
/// for an independent system or a government with no colour.
#[must_use]
pub fn system_color(govt: Option<GovtId>, colors: &BTreeMap<GovtId, u32>) -> Color {
    govt.and_then(|govt| colors.get(&govt))
        .map_or(NEUTRAL, |&raw| Color::from_rgb24(raw))
}

/// A system as the map draws it.
#[derive(Clone, Debug, PartialEq)]
pub struct MapSystem {
    /// Everything the catalog says about it.
    pub entry: SystemEntry,
    /// Its dot's colour.
    pub color: Color,
}

impl MapSystem {
    /// Its map position.
    #[must_use]
    pub fn position(&self) -> Point {
        Point::new(f32::from(self.entry.x), f32::from(self.entry.y))
    }
}

/// `entry` as the star map takes it: its links stay raw, for the star map
/// to normalise. The map reads neither its stellars, its `Message` nor its
/// `Interference`.
pub(super) fn star_system(entry: &SystemEntry) -> StarSystem {
    StarSystem {
        id: entry.id,
        position: Vec2::new(entry.x.into(), entry.y.into()),
        links: entry.links.clone(),
        govt: entry.govt,
        stellars: Vec::new(),
        interference: 0,
        murk: 0,
        message: -1,
    }
}

/// The galaxy laid out for the map.
#[derive(Clone, Debug, PartialEq)]
pub struct GalaxyModel {
    /// Every system the catalog has, by ascending ID, coloured as it says.
    catalog: Vec<MapSystem>,
    /// Every system drawn, by ascending ID: the visible ones.
    systems: Vec<MapSystem>,
    /// Every hyperlink once, as (lower ID, higher ID), sorted.
    links: Vec<(SystemId, SystemId)>,
    /// Every government's map colour.
    govt_colors: BTreeMap<GovtId, u32>,
    /// Every nebula the catalog has that can be drawn.
    catalog_nebulae: Vec<NebulaEntry>,
    /// Every nebula drawn: the catalog's until a session's galaxy is
    /// applied, then those it shows.
    nebulae: Vec<NebulaEntry>,
    /// The catalog's problems, then the model's.
    problems: Vec<String>,
}

impl GalaxyModel {
    /// Lays out `galaxy`.
    #[must_use]
    pub fn new(galaxy: Galaxy) -> Self {
        let Galaxy {
            mut systems,
            govt_colors,
            nebulae,
            mut problems,
        } = galaxy;
        systems.sort_by_key(|system| system.id);
        let known: BTreeSet<SystemId> = systems.iter().map(|system| system.id).collect();
        for system in &systems {
            for &to in &system.links {
                if to != system.id && !known.contains(&to) {
                    problems.push(format!(
                        "sÿst {}: hyperlink to missing sÿst {}",
                        system.id.0, to.0
                    ));
                }
            }
        }
        let links = StarMap::new(systems.iter().map(star_system).collect()).links();
        let systems: Vec<MapSystem> = systems
            .into_iter()
            .map(|entry| {
                if let Some(govt) = entry.govt
                    && !govt_colors.contains_key(&govt)
                {
                    problems.push(format!("sÿst {}: no gövt {}", entry.id.0, govt.0));
                }
                let color = system_color(entry.govt, &govt_colors);
                MapSystem { entry, color }
            })
            .collect();
        let nebulae: Vec<NebulaEntry> = nebulae
            .into_iter()
            .filter(|nebula| {
                let problem = if nebula.width <= 0 || nebula.height <= 0 {
                    format!(
                        "its {} x {} rectangle has no area",
                        nebula.width, nebula.height
                    )
                } else if nebula.pictures.is_empty() {
                    "no pictures to draw it with".to_owned()
                } else {
                    return true;
                };
                problems.push(format!("nëbu {}: {problem}", nebula.id.0));
                false
            })
            .collect();
        Self {
            catalog: systems.clone(),
            systems,
            links,
            govt_colors,
            catalog_nebulae: nebulae.clone(),
            nebulae,
            problems,
        }
    }

    /// Draws the session's `galaxy` over the catalog's geometry: only its
    /// visible systems, each coloured by its government there, its
    /// hyperlinks, and only the nebulae it shows, each active and
    /// explored, as `_MapDialogUpdate` draws them (@0xf951-0xf95f). The
    /// problems are the catalog's, as built.
    pub fn apply(&mut self, galaxy: &OverlaidGalaxy) {
        self.systems = self
            .catalog
            .iter()
            .filter(|system| galaxy.visible(system.entry.id))
            .map(|system| MapSystem {
                entry: system.entry.clone(),
                color: system_color(galaxy.govt(system.entry.id), &self.govt_colors),
            })
            .collect();
        self.links = galaxy.map().links();
        self.nebulae = self
            .catalog_nebulae
            .iter()
            .filter(|nebula| galaxy.nebula_shown(nebula.id))
            .cloned()
            .collect();
    }

    /// Every system clicked, by ascending ID: every visible one.
    #[must_use]
    pub fn systems(&self) -> &[MapSystem] {
        &self.systems
    }

    /// Every system the catalog has, by ascending ID, visible or not,
    /// coloured by its government in the catalog.
    #[must_use]
    pub fn catalog_systems(&self) -> &[MapSystem] {
        &self.catalog
    }

    /// System `id` as the catalog has it, visible or not.
    #[must_use]
    pub fn catalog_system(&self, id: SystemId) -> Option<&MapSystem> {
        let at = self
            .catalog
            .binary_search_by_key(&id, |system| system.entry.id)
            .ok()?;
        Some(&self.catalog[at])
    }

    /// System `id`'s map position, visible or not, if the catalog has it:
    /// where the galaxy map draws it, as `_DrawMap` reads any system's
    /// position, hidden or not.
    #[must_use]
    pub fn position(&self, id: SystemId) -> Option<Point> {
        self.catalog_system(id).map(MapSystem::position)
    }

    /// System `id`, if it is drawn.
    #[must_use]
    pub fn system(&self, id: SystemId) -> Option<&MapSystem> {
        let at = self
            .systems
            .binary_search_by_key(&id, |system| system.entry.id)
            .ok()?;
        Some(&self.systems[at])
    }

    /// Every hyperlink once, as (lower ID, higher ID), sorted.
    #[must_use]
    pub fn links(&self) -> &[(SystemId, SystemId)] {
        &self.links
    }

    /// Every nebula drawn, by ascending ID: every one that can be drawn,
    /// or, once a session's galaxy is applied, every one it shows, active
    /// and explored.
    #[must_use]
    pub fn nebulae(&self) -> &[NebulaEntry] {
        &self.nebulae
    }

    /// Every nebula the catalog has that can be drawn, by ascending ID,
    /// whatever a session's galaxy says: active or not, explored or not.
    /// The viewer draws these.
    #[must_use]
    pub fn catalog_nebulae(&self) -> &[NebulaEntry] {
        &self.catalog_nebulae
    }

    /// The smallest rectangle holding every system the catalog has, drawn
    /// or not, or `None` for none.
    #[must_use]
    pub fn bounds(&self) -> Option<Bounds> {
        Bounds::around(self.catalog.iter().map(MapSystem::position))
    }

    /// Everything that could not be read or laid out, each a message
    /// ready to display: the catalog's, then the model's.
    #[must_use]
    pub fn problems(&self) -> &[String] {
        &self.problems
    }

    /// The system whose dot, drawn through `transform`, is nearest `at`,
    /// within [`HIT_RADIUS`]; on a tie, the lowest ID. Whether `at` is in
    /// the map area at all is the map's to say.
    #[must_use]
    pub fn hit(&self, at: Point, transform: &Transform) -> Option<SystemId> {
        let mut nearest: Option<(f32, SystemId)> = None;
        // By ascending ID, and only a strictly nearer system replaces the
        // one found, so ties go to the lowest ID.
        for system in &self.systems {
            let dot = transform.world_to_screen(system.position());
            let distance = (dot.x - at.x).hypot(dot.y - at.y);
            let nearer = nearest.is_none_or(|(best, _)| distance < best);
            if distance <= HIT_RADIUS && nearer {
                nearest = Some((distance, system.entry.id));
            }
        }
        nearest.map(|(_, id)| id)
    }

    /// Every system at system `id`'s position, `id` included, by ascending
    /// ID; empty if there is no system `id`.
    #[must_use]
    pub fn stack(&self, id: SystemId) -> Vec<SystemId> {
        let Some(system) = self.system(id) else {
            return Vec::new();
        };
        let (x, y) = (system.entry.x, system.entry.y);
        self.systems
            .iter()
            .filter(|other| (other.entry.x, other.entry.y) == (x, y))
            .map(|other| other.entry.id)
            .collect()
    }

    /// What a click at `at` selects when `current` is selected: the system
    /// it hits, except that clicking again on the position of the current
    /// system selects the next system there (by ID, wrapping round). `None`
    /// when it hits nothing.
    #[must_use]
    pub fn click(
        &self,
        at: Point,
        transform: &Transform,
        current: Option<SystemId>,
    ) -> Option<SystemId> {
        let hit = self.hit(at, transform)?;
        let stack = self.stack(hit);
        let next = current
            .and_then(|current| stack.iter().position(|&id| id == current))
            .map(|index| stack[(index + 1) % stack.len()]);
        Some(next.unwrap_or(hit))
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::galaxy::catalog::{NebulaId, NebulaPicture};
    use crate::galaxy::view::{MAP_CENTER, MapView, SCALES};
    use nova_sim::story::GalaxyOverlay;

    fn at(x: f32, y: f32) -> Point {
        Point::new(x, y)
    }

    /// System `id` at (`x`, `y`), independent, with these raw links.
    fn system(id: i16, x: i16, y: i16, links: &[i16]) -> SystemEntry {
        SystemEntry {
            id: SystemId(id),
            name: format!("System {id}"),
            x,
            y,
            links: links.iter().copied().map(SystemId).collect(),
            govt: None,
            stellars: Vec::new(),
        }
    }

    fn galaxy(systems: Vec<SystemEntry>) -> Galaxy {
        Galaxy {
            systems,
            ..Galaxy::default()
        }
    }

    fn model(systems: Vec<SystemEntry>) -> GalaxyModel {
        GalaxyModel::new(galaxy(systems))
    }

    fn ids(ids: &[i16]) -> Vec<SystemId> {
        ids.iter().copied().map(SystemId).collect()
    }

    /// A view at 100% centred on the origin, over bounds wide enough not to
    /// clamp, zoomed `steps` in (negative: out).
    fn view_at(steps: i32) -> MapView {
        let limits = Bounds {
            min: at(-2000.0, -2000.0),
            max: at(2000.0, 2000.0),
        };
        let mut view = MapView::fit(Some(limits));
        view.pan(
            -view.center().x * view.scale(),
            -view.center().y * view.scale(),
        );
        // At most one step per scale, so a view that never zooms in fails
        // the assertion instead of hanging the test.
        for _ in 0..SCALES.len() {
            if view.zoom() == 3 {
                break;
            }
            view.zoom_in();
        }
        assert_eq!(view.zoom(), 3, "100%");
        for _ in 0..steps.abs() {
            if steps > 0 {
                view.zoom_in();
            } else {
                view.zoom_out();
            }
        }
        assert_eq!(view.center(), at(0.0, 0.0));
        view
    }

    // Colours.

    #[test]
    fn a_governed_system_has_its_governments_colour_without_the_top_byte() {
        let colors = BTreeMap::from([(GovtId(128), 0xFF2C_2CAF), (GovtId(142), 0)]);
        assert_eq!(
            system_color(Some(GovtId(128)), &colors),
            Color::rgba(0x2C, 0x2C, 0xAF, 255)
        );
        assert_eq!(system_color(Some(GovtId(142)), &colors), Color::BLACK);
    }

    #[test]
    fn independent_and_unknown_governments_are_neutral() {
        let colors = BTreeMap::from([(GovtId(128), 0x00CF_0C0C)]);
        assert_eq!(system_color(None, &colors), NEUTRAL);
        assert_eq!(system_color(Some(GovtId(129)), &colors), NEUTRAL);
    }

    #[test]
    fn each_system_takes_its_own_governments_colour() {
        let mut systems = vec![
            system(128, 0, 0, &[]),
            system(129, 10, 0, &[]),
            system(130, 20, 0, &[]),
            system(131, 30, 0, &[]),
        ];
        systems[0].govt = Some(GovtId(128));
        systems[1].govt = Some(GovtId(129));
        systems[3].govt = Some(GovtId(150));
        let model = GalaxyModel::new(Galaxy {
            systems,
            govt_colors: BTreeMap::from([(GovtId(128), 0x002C_2CAF), (GovtId(129), 0x00CF_0C0C)]),
            problems: vec!["from the catalog".to_owned()],
            ..Galaxy::default()
        });
        let colors: Vec<Color> = model.systems().iter().map(|s| s.color).collect();
        assert_eq!(
            colors,
            [
                Color::from_rgb24(0x002C_2CAF),
                Color::from_rgb24(0x00CF_0C0C),
                NEUTRAL,
                NEUTRAL
            ]
        );
        assert_eq!(
            model.problems(),
            ["from the catalog", "sÿst 131: no gövt 150"]
        );
    }

    // Systems and bounds.

    #[test]
    fn systems_are_kept_by_ascending_id_and_looked_up_by_id() {
        let model = model(vec![
            system(130, -358, 300, &[]),
            system(128, 587, -330, &[]),
            system(129, 0, 0, &[]),
        ]);
        let order: Vec<SystemId> = model.systems().iter().map(|s| s.entry.id).collect();
        assert_eq!(order, ids(&[128, 129, 130]));
        let found = model.system(SystemId(130)).expect("system 130");
        assert_eq!(
            (found.entry.id, found.position()),
            (SystemId(130), at(-358.0, 300.0))
        );
        assert_eq!(model.system(SystemId(131)), None);
        assert_eq!(
            model.bounds(),
            Some(Bounds {
                min: at(-358.0, -330.0),
                max: at(587.0, 300.0)
            })
        );
        assert_eq!(GalaxyModel::new(Galaxy::default()).bounds(), None);
    }

    // Hyperlinks.

    #[test]
    fn a_link_listed_by_both_systems_is_kept_once() {
        let model = model(vec![system(128, 0, 0, &[129]), system(129, 9, 9, &[128])]);
        assert_eq!(model.links(), [(SystemId(128), SystemId(129))]);
        assert_eq!(model.problems(), [] as [String; 0]);
    }

    #[test]
    fn one_way_duplicate_and_self_links_are_kept_once_or_dropped() {
        // Each at its own place: a link names the system standing for its
        // target's place.
        let model = model(vec![
            system(128, 0, 0, &[130, 128]),
            system(129, 10, 0, &[]),
            system(130, 20, 0, &[129, 129, 128]),
            system(131, 30, 0, &[128]),
        ]);
        assert_eq!(
            model.links(),
            [
                (SystemId(128), SystemId(130)),
                (SystemId(128), SystemId(131)),
                (SystemId(129), SystemId(130)),
            ]
        );
        assert_eq!(model.problems(), [] as [String; 0]);
    }

    #[test]
    fn a_link_to_a_missing_system_is_dropped_with_a_problem() {
        let model = model(vec![
            system(128, 0, 0, &[4000, 129]),
            system(129, 10, 0, &[]),
        ]);
        assert_eq!(model.links(), [(SystemId(128), SystemId(129))]);
        assert_eq!(
            model.problems(),
            ["sÿst 128: hyperlink to missing sÿst 4000"]
        );
    }

    // Nebulae.

    fn picture(id: i16, width: u32) -> NebulaPicture {
        NebulaPicture {
            id,
            slot: 0,
            width,
            height: width,
        }
    }

    /// Nebula 128 covering (-20, -40) to (80, 60) at 100%, with these
    /// pictures.
    fn nebula(pictures: Vec<NebulaPicture>) -> NebulaEntry {
        NebulaEntry {
            id: NebulaId(128),
            name: "Holpa Nebula".to_owned(),
            left: -20,
            top: -40,
            width: 100,
            height: 100,
            pictures,
        }
    }

    #[test]
    fn a_nebula_with_no_pictures_or_no_area_is_not_kept() {
        let none = nebula(Vec::new());
        let flat = NebulaEntry {
            id: NebulaId(129),
            height: 0,
            ..nebula(vec![picture(9507, 10)])
        };
        let thin = NebulaEntry {
            id: NebulaId(130),
            width: -5,
            ..nebula(vec![picture(9514, 10)])
        };
        let sliver = NebulaEntry {
            id: NebulaId(131),
            width: 1,
            height: 1,
            ..nebula(vec![picture(9521, 10)])
        };
        let model = GalaxyModel::new(Galaxy {
            nebulae: vec![none, flat, thin, sliver.clone()],
            ..Galaxy::default()
        });
        assert_eq!(model.nebulae(), [sliver]);
        assert_eq!(
            model.problems(),
            [
                "nëbu 128: no pictures to draw it with",
                "nëbu 129: its 100 x 0 rectangle has no area",
                "nëbu 130: its -5 x 100 rectangle has no area",
            ]
        );
    }

    // Hit-testing.

    #[test]
    fn a_click_on_a_dot_hits_its_system() {
        let model = model(vec![system(128, 0, 0, &[]), system(129, 100, 50, &[])]);
        let view = view_at(0);
        assert_eq!(
            model.hit(MAP_CENTER, &view.transform()),
            Some(SystemId(128))
        );
        let beta = view.world_to_screen(at(100.0, 50.0));
        assert_eq!(model.hit(beta, &view.transform()), Some(SystemId(129)));
    }

    #[test]
    fn the_hit_radius_includes_its_edge_on_each_axis() {
        let model = model(vec![system(128, 0, 0, &[])]);
        let view = view_at(0);
        let c = MAP_CENTER;
        for (dx, dy) in [(1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)] {
            let edge = at(c.x + dx * HIT_RADIUS, c.y + dy * HIT_RADIUS);
            assert_eq!(
                model.hit(edge, &view.transform()),
                Some(SystemId(128)),
                "{edge:?}"
            );
            let past = at(c.x + dx * (HIT_RADIUS + 0.5), c.y + dy * (HIT_RADIUS + 0.5));
            assert_eq!(model.hit(past, &view.transform()), None, "{past:?}");
        }
        // Diagonally, 6 and 6 is 8.49 away: a miss.
        assert_eq!(model.hit(at(c.x + 6.0, c.y + 6.0), &view.transform()), None);
        assert_eq!(
            model.hit(at(c.x + 4.8, c.y + 6.4), &view.transform()),
            Some(SystemId(128))
        );
    }

    #[test]
    fn the_nearer_of_two_systems_in_reach_wins_whichever_id_is_lower() {
        let model = model(vec![system(128, 0, 0, &[]), system(129, 10, 0, &[])]);
        let view = view_at(0);
        let c = MAP_CENTER;
        assert_eq!(
            model.hit(at(c.x + 4.0, c.y), &view.transform()),
            Some(SystemId(128))
        );
        assert_eq!(
            model.hit(at(c.x + 6.0, c.y), &view.transform()),
            Some(SystemId(129))
        );
        // Exactly between them: the lower ID.
        assert_eq!(
            model.hit(at(c.x + 5.0, c.y), &view.transform()),
            Some(SystemId(128))
        );
        let model = model_rev();
        assert_eq!(
            model.hit(at(c.x + 4.0, c.y), &view.transform()),
            Some(SystemId(129))
        );
        assert_eq!(
            model.hit(at(c.x + 6.0, c.y), &view.transform()),
            Some(SystemId(128))
        );
    }

    /// Systems 129 at the origin and 128 ten units right.
    fn model_rev() -> GalaxyModel {
        model(vec![system(129, 0, 0, &[]), system(128, 10, 0, &[])])
    }

    #[test]
    fn systems_at_one_position_hit_the_lowest_id() {
        let model = model(vec![system(531, 0, 0, &[]), system(130, 0, 0, &[])]);
        assert_eq!(
            model.hit(MAP_CENTER, &view_at(0).transform()),
            Some(SystemId(130))
        );
    }

    // Which points are in the map area at all is the map's to say
    // (`GalaxyMap::in_map`): its `a_press_in_the_panel_starts_nothing`
    // and `a_click_released_in_the_panel_keeps_the_selection` pin the
    // viewer's panel edge.
    #[test]
    fn empty_space_hits_nothing() {
        let view = view_at(0);
        let model = model(vec![system(128, 0, 0, &[])]);
        assert_eq!(
            model.hit(at(MAP_CENTER.x + 40.0, MAP_CENTER.y), &view.transform()),
            None
        );
        assert_eq!(
            GalaxyModel::new(Galaxy::default()).hit(MAP_CENTER, &view.transform()),
            None
        );
    }

    #[test]
    fn the_hit_radius_is_in_screen_units_at_every_zoom() {
        let model = model(vec![system(128, 0, 0, &[])]);
        // 10 map units right: 10 on screen at 100%, 7.5 at 75%.
        let offset = at(10.0, 0.0);
        let actual = view_at(0);
        assert_eq!(
            model.hit(actual.world_to_screen(offset), &actual.transform()),
            None
        );
        let small = view_at(-1);
        assert_eq!(
            model.hit(small.world_to_screen(offset), &small.transform()),
            Some(SystemId(128))
        );
        assert_eq!(SCALES[small.zoom()], 0.75);
    }

    // Stacked systems.

    /// Sol (130) and its alternate (531) at one position, Earth's neighbour
    /// 132 elsewhere, and a three-way stack at (50, 0).
    fn stacked() -> GalaxyModel {
        model(vec![
            system(130, 0, 0, &[]),
            system(132, 30, 0, &[]),
            system(531, 0, 0, &[]),
            system(700, 50, 0, &[]),
            system(600, 50, 0, &[]),
            system(650, 50, 0, &[]),
        ])
    }

    #[test]
    fn a_stack_is_every_system_at_one_position_by_id() {
        let model = stacked();
        assert_eq!(model.stack(SystemId(531)), ids(&[130, 531]));
        assert_eq!(model.stack(SystemId(130)), ids(&[130, 531]));
        assert_eq!(model.stack(SystemId(132)), ids(&[132]));
        assert_eq!(model.stack(SystemId(650)), ids(&[600, 650, 700]));
        assert_eq!(model.stack(SystemId(999)), ids(&[]));
    }

    #[test]
    fn clicking_a_stack_again_cycles_through_it_and_wraps() {
        let model = stacked();
        let view = view_at(0);
        let sol = MAP_CENTER;
        let mut selected = None;
        let mut visited = Vec::new();
        for _ in 0..3 {
            selected = model.click(sol, &view.transform(), selected);
            visited.push(selected.expect("a system").0);
        }
        assert_eq!(visited, [130, 531, 130]);

        let triple = view.world_to_screen(at(50.0, 0.0));
        visited.clear();
        for _ in 0..4 {
            selected = model.click(triple, &view.transform(), selected);
            visited.push(selected.expect("a system").0);
        }
        assert_eq!(visited, [600, 650, 700, 600]);
    }

    #[test]
    fn clicking_another_position_selects_what_it_hits() {
        let model = stacked();
        let view = view_at(0);
        let neighbour = view.world_to_screen(at(30.0, 0.0));
        // From the alternate, a click on a lone neighbour selects it, and
        // its own repeated clicks keep it.
        let selected = model.click(neighbour, &view.transform(), Some(SystemId(531)));
        assert_eq!(selected, Some(SystemId(132)));
        assert_eq!(
            model.click(neighbour, &view.transform(), selected),
            Some(SystemId(132))
        );
        // From the neighbour, the stack starts again at its lowest ID.
        assert_eq!(
            model.click(MAP_CENTER, &view.transform(), selected),
            Some(SystemId(130))
        );
        // Empty space selects nothing.
        let empty = at(MAP_CENTER.x, MAP_CENTER.y + 100.0);
        assert_eq!(
            model.click(empty, &view.transform(), Some(SystemId(130))),
            None
        );
    }

    // The session's galaxy.

    /// 128 at (0, 0) lists 129; 129 at (50, 50), of government 128; 130 at
    /// 129's place; 131 at (90, 0) lists 129. Nebulae 140 and 141 can be
    /// drawn.
    fn session_galaxy() -> Galaxy {
        let mut systems = vec![
            system(128, 0, 0, &[129]),
            system(129, 50, 50, &[]),
            system(130, 50, 50, &[]),
            system(131, 90, 0, &[129]),
        ];
        systems[1].govt = Some(GovtId(128));
        let picture = NebulaPicture {
            id: 9500,
            slot: 0,
            width: 30,
            height: 30,
        };
        let entry = |id| NebulaEntry {
            id: NebulaId(id),
            ..nebula(vec![picture])
        };
        Galaxy {
            systems,
            govt_colors: BTreeMap::from([(GovtId(128), 0x002C_2CAF), (GovtId(129), 0x00CF_0C0C)]),
            nebulae: vec![entry(140), entry(141)],
            problems: vec!["from the catalog".to_owned()],
        }
    }

    /// The session's galaxy over `galaxy`'s systems, with `change` made.
    fn overlaid(galaxy: &Galaxy, change: impl FnOnce(&mut GalaxyOverlay)) -> OverlaidGalaxy {
        let mut overlay = GalaxyOverlay::default();
        change(&mut overlay);
        OverlaidGalaxy::new(galaxy.systems.iter().map(star_system).collect(), overlay)
    }

    fn system_ids(model: &GalaxyModel) -> Vec<SystemId> {
        model
            .systems()
            .iter()
            .map(|system| system.entry.id)
            .collect()
    }

    fn nebula_ids(model: &GalaxyModel) -> Vec<NebulaId> {
        model.nebulae().iter().map(|nebula| nebula.id).collect()
    }

    #[test]
    fn with_nothing_explored_the_sessions_galaxy_draws_the_catalogs_systems_and_no_nebula() {
        let galaxy = session_galaxy();
        let mut model = GalaxyModel::new(galaxy.clone());
        let fresh = model.clone();
        assert_eq!(nebula_ids(&fresh), [NebulaId(140), NebulaId(141)]);
        model.apply(&overlaid(&galaxy, |_| {}));
        assert_eq!(model.systems(), fresh.systems());
        assert_eq!(model.links(), fresh.links());
        assert_eq!(model.problems(), fresh.problems());
        assert_eq!(nebula_ids(&model), [] as [NebulaId; 0], "none explored");
        assert_eq!(
            model.catalog_nebulae(),
            fresh.nebulae(),
            "the catalog's are every one"
        );
    }

    #[test]
    fn a_nebula_is_drawn_only_while_active_and_explored() {
        let galaxy = session_galaxy();
        let mut model = GalaxyModel::new(galaxy.clone());
        model.apply(&overlaid(&galaxy, |overlay| {
            overlay.set_nebula_explored(NebulaId(140), true);
        }));
        assert_eq!(nebula_ids(&model), [NebulaId(140)], "141 unexplored");
        model.apply(&overlaid(&galaxy, |overlay| {
            overlay.set_nebula_explored(NebulaId(140), true);
            overlay.set_nebula_active(NebulaId(140), Some(false));
            overlay.set_nebula_explored(NebulaId(141), true);
        }));
        assert_eq!(nebula_ids(&model), [NebulaId(141)], "140 inactive");
    }

    #[test]
    fn a_hidden_system_is_neither_drawn_nor_hit_and_its_links_go_to_its_place() {
        let galaxy = session_galaxy();
        let mut model = GalaxyModel::new(galaxy.clone());
        let bounds = model.bounds();
        model.apply(&overlaid(&galaxy, |overlay| {
            overlay.set_visible(SystemId(129), Some(false));
        }));
        assert_eq!(system_ids(&model), ids(&[128, 130, 131]));
        assert_eq!(model.system(SystemId(129)), None);
        assert_eq!(model.stack(SystemId(130)), ids(&[130]), "alone there now");
        let view = view_at(0);
        let place = view.world_to_screen(at(50.0, 50.0));
        assert_eq!(model.hit(place, &view.transform()), Some(SystemId(130)));
        assert_eq!(
            model.links(),
            [
                (SystemId(128), SystemId(130)),
                (SystemId(130), SystemId(131))
            ]
        );
        assert_eq!(model.bounds(), bounds, "the geometry is the catalog's");
        assert_eq!(model.problems(), ["from the catalog"], "unchanged");
    }

    #[test]
    fn an_overridden_government_recolours_its_system() {
        let galaxy = session_galaxy();
        let mut model = GalaxyModel::new(galaxy.clone());
        model.apply(&overlaid(&galaxy, |overlay| {
            overlay.set_govt(SystemId(128), Some(Some(GovtId(129))));
            overlay.set_govt(SystemId(129), Some(None));
        }));
        let colors: Vec<Color> = model.systems().iter().map(|s| s.color).collect();
        assert_eq!(
            colors,
            [Color::from_rgb24(0x00CF_0C0C), NEUTRAL, NEUTRAL, NEUTRAL]
        );
    }

    #[test]
    fn an_inactive_nebula_is_not_drawn_and_shows_again_once_active() {
        let galaxy = session_galaxy();
        let mut model = GalaxyModel::new(galaxy.clone());
        let explored = |overlay: &mut GalaxyOverlay| {
            overlay.set_nebula_explored(NebulaId(140), true);
            overlay.set_nebula_explored(NebulaId(141), true);
        };
        model.apply(&overlaid(&galaxy, |overlay| {
            explored(overlay);
            overlay.set_nebula_active(NebulaId(140), Some(false));
        }));
        assert_eq!(nebula_ids(&model), [NebulaId(141)]);
        model.apply(&overlaid(&galaxy, explored));
        assert_eq!(
            nebula_ids(&model),
            [NebulaId(140), NebulaId(141)],
            "both again"
        );
    }
}
