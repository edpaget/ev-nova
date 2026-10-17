//! The galaxy map over the stock data: every system, hyperlink, colour and
//! nebula, and every system selectable by clicking. Skips, passing, when
//! `NOVA_DATA` is unset.

use crate::common;

use std::collections::{BTreeMap, BTreeSet};

use nova_data::records::govt::Govt;
use nova_data::records::stellar::Stellar;
use nova_data::records::system::System;
use nova_data::{GameData, Record};
use nova_sim::RuleSource;
use nova_view::flight::radar::IRect;
use nova_view::galaxy::borders::draw_cells;
use nova_view::galaxy::map::{LESS_DARK_GREY, NAME_SIZE};
use nova_view::galaxy::view::Bounds;
use nova_view::galaxy::{GalaxyCatalog, GalaxyMap, NEUTRAL, SystemId};
use nova_view::{Blend, Color, DrawCommand, DrawList, Input, Key, MouseButton, Point, Screen};

fn open() -> Option<GameData> {
    let dir = common::nova_data()?;
    Some(GameData::open(&dir, None).expect("the stock data opens"))
}

/// Every stock system, decoded straight from the store: ID to record.
fn systems(data: &GameData) -> BTreeMap<i16, System> {
    data.records::<System>()
        .map(|(id, record)| (id, record.expect("every sÿst decodes").record.clone()))
        .collect()
}

/// How many stock links are drawn once each is resolved by place.
const RESOLVED_PAIRS: usize = 1079;

fn drawn(map: &GalaxyMap) -> DrawList {
    let mut list = DrawList::new();
    map.draw(&mut list);
    list
}

#[test]
fn the_stock_galaxy_reads_without_problems() {
    let Some(data) = open() else {
        return;
    };
    let galaxy = data.galaxy();
    assert_eq!(galaxy.problems, Vec::<String>::new());
    assert_eq!(galaxy.systems.len(), 545);
    assert_eq!(galaxy.systems.first().map(|s| s.id), Some(SystemId(128)));
    assert_eq!(galaxy.systems.last().map(|s| s.id), Some(SystemId(1127)));
    let bounds = Bounds::around(
        galaxy
            .systems
            .iter()
            .map(|s| Point::new(f32::from(s.x), f32::from(s.y))),
    );
    assert_eq!(
        bounds,
        Some(Bounds {
            min: Point::new(-358.0, -330.0),
            max: Point::new(587.0, 300.0),
        })
    );
    assert_eq!(galaxy.nebulae.len(), 4);
    for nebula in &galaxy.nebulae {
        assert_eq!(nebula.pictures.len(), 3, "{nebula:?}");
    }
    let unnamed: Vec<&str> = galaxy
        .systems
        .iter()
        .flat_map(|s| s.stellars.iter().map(|stellar| stellar.name.as_str()))
        .filter(|name| name.starts_with("spöb "))
        .collect();
    assert_eq!(unnamed, Vec::<&str>::new(), "every stellar has a name");
}

#[test]
fn the_stock_map_fits_at_75_percent_with_every_nebula_behind_the_systems() {
    let Some(data) = open() else {
        return;
    };
    let map = GalaxyMap::new(&data);
    assert_eq!(map.problems(), [] as [String; 0]);
    assert_eq!(map.view().expect("the viewer").zoom(), 2, "75%");
    let list = drawn(&map);
    // The borders' cells first, one rect each, then the nebulae.
    let mut cells = DrawList::new();
    draw_cells(
        &mut cells,
        map.border_cells(),
        (0, 0),
        IRect::new(0, 0, 1024, 608),
        map.model().govt_colors(),
    );
    let borders = cells.len();
    assert!(borders > 0);
    assert_eq!(
        list.iter().take(borders).collect::<Vec<_>>(),
        cells.iter().collect::<Vec<_>>()
    );
    let pictures: Vec<usize> = list
        .iter()
        .enumerate()
        .filter(|(_, c)| max_picture(c).is_some())
        .map(|(at, _)| at)
        .collect();
    let first_shape = list
        .iter()
        .skip(borders)
        .position(|c| matches!(c, DrawCommand::Line { .. }))
        .map(|at| at + borders);
    assert_eq!(pictures, (borders..borders + 4).collect::<Vec<_>>());
    assert_eq!(first_shape, Some(borders + 4));
}

/// The `PICT` a nebula is drawn from, when `command` is a nebula: a max
/// picture clipped to the map area.
fn max_picture(command: &DrawCommand) -> Option<i16> {
    let DrawCommand::Clipped { command, .. } = command else {
        return None;
    };
    match **command {
        DrawCommand::BlendedPicture {
            image,
            blend: Blend::Max,
            ..
        } => Some(image.id),
        _ => None,
    }
}

/// The `PICT` the course map draws each stock nebula from, by name, at
/// the zoom `steps_out` steps out from the first, 0.5625, centred on a
/// system inside each, by `source`'s reading of `nebula_picture`.
fn course_pictures(data: &GameData, steps_out: usize, source: RuleSource) -> Vec<(String, i16)> {
    let galaxy = data.galaxy();
    galaxy
        .nebulae
        .iter()
        .map(|nebula| {
            let inside = galaxy
                .systems
                .iter()
                .find(|system| {
                    (nebula.left..nebula.left + nebula.width).contains(&system.x)
                        && (nebula.top..nebula.top + nebula.height).contains(&system.y)
                })
                .unwrap_or_else(|| panic!("a system in {}", nebula.name));
            let mut map = GalaxyMap::course(data);
            map.set_nebula_pick(source);
            for _ in 0..steps_out {
                map.input(&Input::Key {
                    key: Key::Char('-'),
                    pressed: true,
                    repeat: false,
                });
            }
            map.centre_on_system(inside.id);
            let ids = nebula
                .pictures
                .iter()
                .map(|picture| picture.id)
                .collect::<Vec<_>>();
            let picture = drawn(&map)
                .iter()
                .filter_map(max_picture)
                .find(|id| ids.contains(id))
                .unwrap_or_else(|| panic!("{} drawn as a max picture", nebula.name));
            (nebula.name.clone(), picture)
        })
        .collect()
}

fn named(pictures: &[(&str, i16)]) -> Vec<(String, i16)> {
    pictures
        .iter()
        .map(|&(name, id)| (name.to_owned(), id))
        .collect()
}

#[test]
fn the_course_map_draws_each_stock_nebula_from_the_largest_picture_that_fits() {
    let Some(data) = open() else {
        return;
    };
    let names: Vec<String> = data
        .galaxy()
        .nebulae
        .iter()
        .map(|n| n.name.clone())
        .collect();
    assert_eq!(
        names,
        [
            "Holpa Nebula",
            "Obatta Nebula",
            "Rochak Dust Field",
            "L-1551"
        ]
    );
    let engine = RuleSource::Engine;
    // 0.5625: Holpa's rect is 446 × 529, so its "100%" 251 × 298, enlarged.
    assert_eq!(
        course_pictures(&data, 0, engine)[0],
        named(&[("Holpa Nebula", 9502)])[0]
    );
    // 0.9999995, two steps out: each rect its own size. Obatta's "100%"
    // is a little too tall and L-1551's twice too big, so each is drawn
    // from its "50%"; Rochak's "100%" is its rect's own 109 × 95, and
    // fits.
    assert_eq!(
        course_pictures(&data, 2, engine)[1..],
        named(&[
            ("Obatta Nebula", 9508),
            ("Rochak Dust Field", 9516),
            ("L-1551", 9522)
        ])
    );
    // 1.3333323, three steps out: the rects are 48 × 67, 81 × 71 and
    // 109 × 53. The largest that fits is each one's "50%" or "25%"; the
    // smallest that covers is the next one up.
    assert_eq!(
        course_pictures(&data, 3, engine)[1..],
        named(&[
            ("Obatta Nebula", 9508),
            ("Rochak Dust Field", 9515),
            ("L-1551", 9521)
        ])
    );
    assert_eq!(
        course_pictures(&data, 3, RuleSource::Bible)[1..],
        named(&[
            ("Obatta Nebula", 9509),
            ("Rochak Dust Field", 9516),
            ("L-1551", 9522)
        ]),
        "the smallest that covers each"
    );
    // 2.3703674…, five steps out: Holpa's rect is 105 × 125, under its
    // "50%".
    assert_eq!(
        course_pictures(&data, 5, engine)[0],
        named(&[("Holpa Nebula", 9500)])[0]
    );
}

/// The hyperlinks, worked out here from the raw records, are each drawn
/// once between their systems' dots, each link to the system standing for
/// its target's place: with every system read visible, the lowest-ID
/// system there, as `_FindActiveCoLocatedSystem` resolves every Con slot
/// (`colocated_resolution` engine).
#[test]
fn the_stock_map_draws_every_hyperlink_once() {
    let Some(data) = open() else {
        return;
    };
    let raw = systems(&data);
    let map = GalaxyMap::new(&data);
    let list = drawn(&map);
    let directed: Vec<(i16, i16)> = raw
        .iter()
        .flat_map(|(&from, system)| system.con.iter().flatten().map(move |to| (from, to.0)))
        .collect();
    assert_eq!(directed.len(), 1812);
    let directed_set: BTreeSet<(i16, i16)> = directed.iter().copied().collect();
    let one_way = directed_set
        .iter()
        .filter(|&&(from, to)| !directed_set.contains(&(to, from)))
        .count();
    assert_eq!(one_way, 49);
    assert!(
        directed
            .iter()
            .all(|(from, to)| from != to && raw.contains_key(to))
    );
    let pairs: BTreeSet<(i16, i16)> = directed
        .iter()
        .map(|&(a, b)| (a.min(b), a.max(b)))
        .collect();
    assert_eq!(pairs.len(), 930);
    // The lowest-ID system at each system's place.
    let lowest = |id: i16| {
        let at = (raw[&id].x_pos, raw[&id].y_pos);
        raw.iter()
            .find(|(_, system)| (system.x_pos, system.y_pos) == at)
            .map_or(id, |(&lowest, _)| lowest)
    };
    let pairs: BTreeSet<(i16, i16)> = directed
        .iter()
        .map(|&(a, b)| (a, lowest(b)))
        .filter(|&(a, b)| a != b)
        .map(|(a, b)| (a.min(b), a.max(b)))
        .collect();
    assert_eq!(pairs.len(), RESOLVED_PAIRS);
    let model_pairs: BTreeSet<(i16, i16)> = map
        .model()
        .links()
        .iter()
        .map(|(a, b)| (a.0, b.0))
        .collect();
    assert_eq!(model_pairs, pairs);

    let picture = map.picture().expect("the viewer's picture");
    let picture_pairs: BTreeSet<(i16, i16)> = picture
        .links
        .iter()
        .map(|link| (link.from.0, link.to.0))
        .collect();
    assert_eq!(picture_pairs, pairs);
    let drawn_links = list
        .iter()
        .filter(|c| matches!(c, DrawCommand::Line { color, .. } if *color == LESS_DARK_GREY))
        .count();
    assert_eq!(drawn_links, RESOLVED_PAIRS, "each once, at pen 1");
}

/// Every system's oval, framed in its own government's colour worked out
/// here from the raw records, by ascending ID.
#[test]
fn the_stock_map_draws_every_system_in_its_governments_colour() {
    let Some(data) = open() else {
        return;
    };
    let raw = systems(&data);
    let map = GalaxyMap::new(&data);
    let colors: BTreeMap<i16, u32> = data
        .records::<Govt>()
        .map(|(id, govt)| (id, govt.expect("every gövt decodes").record.color))
        .collect();
    let expected: Vec<(i16, Color)> = raw
        .iter()
        .map(|(&id, system)| {
            let color = system.govt.map_or(NEUTRAL, |govt| {
                let [_, r, g, b] = colors[&govt.0].to_be_bytes();
                Color::rgba(r, g, b, 255)
            });
            (id, color)
        })
        .collect();
    let frames: Vec<(i16, Color)> = map
        .picture()
        .expect("the viewer's picture")
        .systems
        .iter()
        .map(|system| (system.id.0, system.frame))
        .collect();
    assert_eq!(frames.len(), 545);
    assert_eq!(frames, expected);
    let names = |map: &GalaxyMap| {
        drawn(map)
            .iter()
            .filter(|c| matches!(c, DrawCommand::Text { size, .. } if *size == NAME_SIZE))
            .count()
    };
    assert_eq!(names(&map), 0, "at 75%, above a zoom of 1.1, none selected");
    let mut map = map;
    map.input(&Input::Key {
        key: nova_view::Key::Char('='),
        pressed: true,
        repeat: false,
    });
    assert_eq!(names(&map), 2 * 545, "every name, twice, at 100%");
    let independent = raw.values().filter(|s| s.govt.is_none()).count();
    assert_eq!(independent, 91);
    let governments: BTreeSet<i16> = raw.values().filter_map(|s| s.govt).map(|g| g.0).collect();
    assert_eq!(governments.len(), 23);
}

/// Clicks the left button at `at`.
fn click(map: &mut GalaxyMap, at: Point) {
    for pressed in [true, false] {
        map.input(&Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at,
        });
    }
}

fn texts(list: &DrawList) -> Vec<String> {
    list.iter()
        .filter_map(|c| match c {
            DrawCommand::Text { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect()
}

/// Every system, in turn, is selected by clicking its dot at the default
/// view, at most once per system sharing its position, and the panel then
/// shows its name and each of its stellars.
#[test]
fn every_stock_system_can_be_selected_and_shows_its_stellars() {
    let Some(data) = open() else {
        return;
    };
    let raw = systems(&data);
    let mut map = GalaxyMap::new(&data);
    let view = *map.view().expect("the viewer");
    let mut at_position: BTreeMap<(i16, i16), Vec<i16>> = BTreeMap::new();
    for (&id, system) in &raw {
        at_position
            .entry((system.x_pos, system.y_pos))
            .or_default()
            .push(id);
    }
    let shared = at_position.values().filter(|ids| ids.len() > 1).count();
    assert_eq!(shared, 128, "positions holding more than one system");

    let mut problems = Vec::new();
    for (&id, system) in &raw {
        let stack = &at_position[&(system.x_pos, system.y_pos)];
        let dot =
            view.world_to_screen(Point::new(f32::from(system.x_pos), f32::from(system.y_pos)));
        // Start from empty space, then click: the first click picks the
        // lowest ID there.
        click(&mut map, Point::new(-100.0, -100.0));
        click(&mut map, dot);
        if map.selected() != Some(SystemId(stack[0])) {
            problems.push(format!(
                "a first click on sÿst {id} selects {:?}",
                map.selected()
            ));
            continue;
        }
        // At most one more click per other system in the stack.
        for _ in 1..stack.len() {
            if map.selected() == Some(SystemId(id)) {
                break;
            }
            click(&mut map, dot);
        }
        if map.selected() != Some(SystemId(id)) {
            problems.push(format!("sÿst {id} cannot be selected"));
            continue;
        }
        let shown = texts(&drawn(&map));
        let entry = &map.model().system(SystemId(id)).expect("a system").entry;
        let title = format!("{} (sÿst {id})", entry.name);
        if !shown.contains(&title) {
            problems.push(format!("sÿst {id}: no {title:?} in {shown:?}"));
        }
        let nav: Vec<i16> = system.nav_def.iter().flatten().map(|s| s.0).collect();
        let listed: Vec<i16> = entry.stellars.iter().map(|s| s.id.0).collect();
        if listed != nav {
            problems.push(format!("sÿst {id} lists {listed:?}, not {nav:?}"));
        }
        for stellar in &nav {
            let name = data
                .resource(Stellar::TYPE, *stellar)
                .and_then(|res| res.resource.name())
                .expect("every stock spöb is named");
            let name = name.split(';').next().unwrap_or_default().trim().to_owned();
            if !shown.contains(&name) {
                problems.push(format!("sÿst {id}: no stellar {name:?} in {shown:?}"));
            }
        }
        if nav.is_empty() && !shown.contains(&"No stellars".to_owned()) {
            problems.push(format!("sÿst {id}: no \"No stellars\" in {shown:?}"));
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

// Find and the Done and Find buttons.

/// The stock viewer, measured, with `query` found by its Find.
fn found(map: &mut GalaxyMap, query: &str) -> Option<SystemId> {
    let key = |key, pressed| Input::Key {
        key,
        pressed,
        repeat: false,
    };
    map.input(&key(nova_view::Key::Char('s'), true));
    map.input(&Input::Text('s'));
    for c in query.chars() {
        map.input(&Input::Text(c));
    }
    map.input(&key(nova_view::Key::Enter, true));
    assert!(!map.finding());
    map.selected()
}

#[test]
fn the_stock_viewers_find_searches_every_system_by_name() {
    let Some(data) = open() else {
        return;
    };
    let mut map =
        GalaxyMap::new(&data).with_metrics(std::rc::Rc::new(nova_view::text::fixture::MonoMetrics));
    let name = |map: &GalaxyMap, id| {
        map.model()
            .catalog_system(SystemId(id))
            .map(|system| system.entry.name.clone())
    };
    assert_eq!(name(&map, 130).as_deref(), Some("Sol"));
    assert_eq!(name(&map, 531).as_deref(), Some("Sol"));
    assert_eq!(found(&mut map, "sol"), Some(SystemId(130)), "the lower Sol");
    assert_eq!(
        found(&mut map, "kanius"),
        Some(SystemId(461)),
        "of 461 and 653"
    );
    assert_eq!(found(&mut map, "nemor"), Some(SystemId(205)), "Ne'mor");
}

#[test]
fn str_150_holds_the_maps_six_button_labels() {
    use nova_data::records::string_list::StrList;
    use nova_view::galaxy::buttons::{
        CLEAR_ROUTE_LABEL, DONE_LABEL, FIND_LABEL, HIDE_BORDERS_LABEL, SHOW_BORDERS_LABEL,
        ZOOM_IN_LABEL, ZOOM_OUT_LABEL,
    };
    let Some(data) = open() else {
        return;
    };
    let list = data
        .get::<StrList>(150)
        .expect("STR# 150 is in the stock data")
        .expect("decodes")
        .record
        .clone();
    assert_eq!(list.strings[4].as_str(), DONE_LABEL);
    assert_eq!(
        list.strings[16].as_str(),
        ZOOM_OUT_LABEL,
        "an ASCII hyphen-minus"
    );
    assert_eq!(list.strings[17].as_str(), ZOOM_IN_LABEL);
    assert_eq!(list.strings[48].as_str(), CLEAR_ROUTE_LABEL);
    assert_eq!(list.strings[55].as_str(), SHOW_BORDERS_LABEL);
    assert_eq!(list.strings[56].as_str(), HIDE_BORDERS_LABEL);
    assert_eq!(list.strings[59].as_str(), FIND_LABEL);
}

#[test]
fn pict_8509_the_maps_background_is_the_dialogs_own_size() {
    use nova_data::graphics::{PICT, decode_pict};
    use nova_view::galaxy::dialog::{BACKGROUND, DIALOG_SIZE};
    let Some(data) = open() else {
        return;
    };
    let resource = data.resource(PICT, BACKGROUND).expect("present").resource;
    let image = decode_pict(resource.data()).expect("decodes");
    #[allow(clippy::cast_precision_loss)] // A picture's size.
    let size = (image.width() as f32, image.height() as f32);
    assert_eq!(size, DIALOG_SIZE, "601 x 513: the stretch changes nothing");
}
