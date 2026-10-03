//! The map model: the galaxy laid out for drawing and clicking, built once
//! from a [`Galaxy`].
//!
//! - Each system's colour comes from its own government's map colour;
//!   independent systems, and systems whose government is missing, are
//!   [`NEUTRAL`].
//! - Each hyperlink is kept once, however many of its two systems list it.
//! - Each nebula is drawn over its map rectangle at the current scale,
//!   from whichever of its pictures is closest in size without being
//!   enlarged.
//! - A click selects the nearest system within [`HIT_RADIUS`]. Several
//!   systems can share one position (alternate versions of a system that
//!   the game swaps in later); repeated clicks there cycle through them.

use std::collections::{BTreeMap, BTreeSet};

use super::catalog::{Galaxy, GovtId, NebulaEntry, NebulaPicture, SystemEntry, SystemId};
use super::view::{Bounds, MAP_HEIGHT, MapView};
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

/// Where a nebula goes on screen and which picture fills it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NebulaPlacement {
    /// The picture, stretched to fill the rectangle.
    pub picture: NebulaPicture,
    /// The rectangle's top-left corner on screen.
    pub top_left: Point,
    /// The rectangle's width on screen.
    pub width: f32,
    /// The rectangle's height on screen.
    pub height: f32,
}

/// Where `nebula` is drawn in `view`: its map rectangle scaled, filled by
/// the smallest picture at least as wide as that, or else the largest, so
/// a picture is shrunk rather than enlarged whenever possible. `None` when
/// it has no pictures or no area.
#[must_use]
pub fn placement(nebula: &NebulaEntry, view: &MapView) -> Option<NebulaPlacement> {
    if nebula.width <= 0 || nebula.height <= 0 {
        return None;
    }
    let scale = view.scale();
    let width = f32::from(nebula.width) * scale;
    let height = f32::from(nebula.height) * scale;
    let pictures = nebula.pictures.iter();
    let picture = pictures
        .clone()
        .filter(|picture| picture.width as f32 >= width)
        .min_by_key(|picture| picture.width)
        .or_else(|| pictures.max_by_key(|picture| picture.width))?;
    let corner = Point::new(f32::from(nebula.left), f32::from(nebula.top));
    Some(NebulaPlacement {
        picture: *picture,
        top_left: view.world_to_screen(corner),
        width,
        height,
    })
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

/// The galaxy laid out for the map.
#[derive(Clone, Debug, PartialEq)]
pub struct GalaxyModel {
    /// Every system, by ascending ID.
    systems: Vec<MapSystem>,
    /// Every hyperlink once, as (lower ID, higher ID), sorted.
    links: Vec<(SystemId, SystemId)>,
    /// Every nebula that can be drawn.
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
        let mut links = BTreeSet::new();
        for system in &systems {
            for &to in &system.links {
                if to == system.id {
                    continue;
                }
                if !known.contains(&to) {
                    problems.push(format!(
                        "sÿst {}: hyperlink to missing sÿst {}",
                        system.id.0, to.0
                    ));
                    continue;
                }
                links.insert((system.id.min(to), system.id.max(to)));
            }
        }
        let systems = systems
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
        let nebulae = nebulae
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
            systems,
            links: links.into_iter().collect(),
            nebulae,
            problems,
        }
    }

    /// Every system, by ascending ID.
    #[must_use]
    pub fn systems(&self) -> &[MapSystem] {
        &self.systems
    }

    /// System `id`, if there is one.
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

    /// Every nebula that can be drawn, by ascending ID.
    #[must_use]
    pub fn nebulae(&self) -> &[NebulaEntry] {
        &self.nebulae
    }

    /// The smallest rectangle holding every system, or `None` for none.
    #[must_use]
    pub fn bounds(&self) -> Option<Bounds> {
        Bounds::around(self.systems.iter().map(MapSystem::position))
    }

    /// Everything that could not be read or laid out, each a message
    /// ready to display: the catalog's, then the model's.
    #[must_use]
    pub fn problems(&self) -> &[String] {
        &self.problems
    }

    /// The system whose dot is nearest `at`, within [`HIT_RADIUS`]; on a
    /// tie, the lowest ID. Points outside the map area hit nothing.
    #[must_use]
    pub fn hit(&self, at: Point, view: &MapView) -> Option<SystemId> {
        if at.y >= MAP_HEIGHT {
            return None;
        }
        let mut nearest: Option<(f32, SystemId)> = None;
        // By ascending ID, and only a strictly nearer system replaces the
        // one found, so ties go to the lowest ID.
        for system in &self.systems {
            let dot = view.world_to_screen(system.position());
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
    pub fn click(&self, at: Point, view: &MapView, current: Option<SystemId>) -> Option<SystemId> {
        let hit = self.hit(at, view)?;
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
    use crate::galaxy::catalog::NebulaId;
    use crate::galaxy::view::{MAP_CENTER, MAP_HEIGHT, SCALES};

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
        while view.zoom() != 3 {
            view.zoom_in();
        }
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
        let model = model(vec![
            system(128, 0, 0, &[130, 128]),
            system(129, 0, 0, &[]),
            system(130, 0, 0, &[129, 129, 128]),
            system(131, 0, 0, &[128]),
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
            system(129, 0, 0, &[]),
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
    fn a_nebula_fills_its_scaled_rectangle_at_every_zoom() {
        let nebula = nebula(vec![picture(9500, 25)]);
        for steps in -3..=3 {
            let view = view_at(steps);
            let scale = view.scale();
            let placed = placement(&nebula, &view).expect("placed");
            assert_eq!(placed.top_left, view.world_to_screen(at(-20.0, -40.0)));
            assert_eq!(
                (placed.width, placed.height),
                (100.0 * scale, 100.0 * scale)
            );
            assert_eq!(placed.picture, picture(9500, 25), "the only picture");
        }
        let actual = placement(&nebula, &view_at(0)).expect("placed");
        assert_eq!(
            actual.top_left,
            at(MAP_CENTER.x - 20.0, MAP_CENTER.y - 40.0)
        );
    }

    #[test]
    fn the_picture_is_the_smallest_not_narrower_than_the_rectangle_or_else_the_largest() {
        // Out of size order, as nebula 131's are not.
        let nebula = nebula(vec![
            picture(9521, 50),
            picture(9522, 200),
            picture(9523, 100),
            picture(9524, 25),
        ]);
        let chosen = |steps| {
            placement(&nebula, &view_at(steps))
                .expect("placed")
                .picture
                .id
        };
        // 42.2 wide: 50 is the smallest that covers it.
        assert_eq!(chosen(-3), 9521);
        // 56.25 and 75 wide: 100.
        assert_eq!(chosen(-2), 9523);
        assert_eq!(chosen(-1), 9523);
        // 100 wide exactly: 100.
        assert_eq!(chosen(0), 9523);
        // 133.3, 177.8: 200; 237: none is that wide, so the largest.
        assert_eq!(chosen(1), 9522);
        assert_eq!(chosen(2), 9522);
        assert_eq!(chosen(3), 9522);
    }

    #[test]
    fn a_nebula_with_no_pictures_or_no_area_is_not_placed_or_kept() {
        let none = nebula(Vec::new());
        assert_eq!(placement(&none, &view_at(0)), None);
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
        assert_eq!(placement(&flat, &view_at(0)), None);
        assert_eq!(placement(&thin, &view_at(0)), None);
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
        assert_eq!(model.hit(MAP_CENTER, &view), Some(SystemId(128)));
        let beta = view.world_to_screen(at(100.0, 50.0));
        assert_eq!(model.hit(beta, &view), Some(SystemId(129)));
    }

    #[test]
    fn the_hit_radius_includes_its_edge_on_each_axis() {
        let model = model(vec![system(128, 0, 0, &[])]);
        let view = view_at(0);
        let c = MAP_CENTER;
        for (dx, dy) in [(1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)] {
            let edge = at(c.x + dx * HIT_RADIUS, c.y + dy * HIT_RADIUS);
            assert_eq!(model.hit(edge, &view), Some(SystemId(128)), "{edge:?}");
            let past = at(c.x + dx * (HIT_RADIUS + 0.5), c.y + dy * (HIT_RADIUS + 0.5));
            assert_eq!(model.hit(past, &view), None, "{past:?}");
        }
        // Diagonally, 6 and 6 is 8.49 away: a miss.
        assert_eq!(model.hit(at(c.x + 6.0, c.y + 6.0), &view), None);
        assert_eq!(
            model.hit(at(c.x + 4.8, c.y + 6.4), &view),
            Some(SystemId(128))
        );
    }

    #[test]
    fn the_nearer_of_two_systems_in_reach_wins_whichever_id_is_lower() {
        let model = model(vec![system(128, 0, 0, &[]), system(129, 10, 0, &[])]);
        let view = view_at(0);
        let c = MAP_CENTER;
        assert_eq!(model.hit(at(c.x + 4.0, c.y), &view), Some(SystemId(128)));
        assert_eq!(model.hit(at(c.x + 6.0, c.y), &view), Some(SystemId(129)));
        // Exactly between them: the lower ID.
        assert_eq!(model.hit(at(c.x + 5.0, c.y), &view), Some(SystemId(128)));
        let model = model_rev();
        assert_eq!(model.hit(at(c.x + 4.0, c.y), &view), Some(SystemId(129)));
        assert_eq!(model.hit(at(c.x + 6.0, c.y), &view), Some(SystemId(128)));
    }

    /// Systems 129 at the origin and 128 ten units right.
    fn model_rev() -> GalaxyModel {
        model(vec![system(129, 0, 0, &[]), system(128, 10, 0, &[])])
    }

    #[test]
    fn systems_at_one_position_hit_the_lowest_id() {
        let model = model(vec![system(531, 0, 0, &[]), system(130, 0, 0, &[])]);
        assert_eq!(model.hit(MAP_CENTER, &view_at(0)), Some(SystemId(130)));
    }

    #[test]
    fn empty_space_and_the_panel_hit_nothing() {
        let view = view_at(0);
        let model = model(vec![system(128, 0, 0, &[])]);
        assert_eq!(
            model.hit(at(MAP_CENTER.x + 40.0, MAP_CENTER.y), &view),
            None
        );
        assert_eq!(
            GalaxyModel::new(Galaxy::default()).hit(MAP_CENTER, &view),
            None
        );
        // A system just above the panel, clicked 4 units below it.
        let low = (MAP_HEIGHT - MAP_CENTER.y - 4.0) as i16;
        let model = model_with(system(128, 0, low, &[]));
        let dot = view.world_to_screen(at(0.0, f32::from(low)));
        assert_eq!(model.hit(dot, &view), Some(SystemId(128)));
        assert_eq!(
            model.hit(at(dot.x, MAP_HEIGHT - 0.5), &view),
            Some(SystemId(128))
        );
        assert_eq!(model.hit(at(dot.x, MAP_HEIGHT), &view), None);
    }

    fn model_with(system: SystemEntry) -> GalaxyModel {
        model(vec![system])
    }

    #[test]
    fn the_hit_radius_is_in_screen_units_at_every_zoom() {
        let model = model(vec![system(128, 0, 0, &[])]);
        // 10 map units right: 10 on screen at 100%, 7.5 at 75%.
        let offset = at(10.0, 0.0);
        let actual = view_at(0);
        assert_eq!(model.hit(actual.world_to_screen(offset), &actual), None);
        let small = view_at(-1);
        assert_eq!(
            model.hit(small.world_to_screen(offset), &small),
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
            selected = model.click(sol, &view, selected);
            visited.push(selected.expect("a system").0);
        }
        assert_eq!(visited, [130, 531, 130]);

        let triple = view.world_to_screen(at(50.0, 0.0));
        visited.clear();
        for _ in 0..4 {
            selected = model.click(triple, &view, selected);
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
        let selected = model.click(neighbour, &view, Some(SystemId(531)));
        assert_eq!(selected, Some(SystemId(132)));
        assert_eq!(model.click(neighbour, &view, selected), Some(SystemId(132)));
        // From the neighbour, the stack starts again at its lowest ID.
        assert_eq!(
            model.click(MAP_CENTER, &view, selected),
            Some(SystemId(130))
        );
        // Empty space selects nothing.
        let empty = at(MAP_CENTER.x, MAP_CENTER.y + 100.0);
        assert_eq!(model.click(empty, &view, Some(SystemId(130))), None);
    }
}
