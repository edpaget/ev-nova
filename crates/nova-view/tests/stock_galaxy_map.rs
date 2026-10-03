//! The galaxy map over the stock data: every system, hyperlink, colour and
//! nebula, and every system selectable by clicking. Skips, passing, when
//! `NOVA_DATA` is unset.

mod common;

use std::collections::{BTreeMap, BTreeSet};

use nova_data::records::govt::Govt;
use nova_data::records::stellar::Stellar;
use nova_data::records::system::System;
use nova_data::{GameData, Record};
use nova_view::galaxy::map::{DOT_SIZE, LINK};
use nova_view::galaxy::view::Bounds;
use nova_view::galaxy::{GalaxyCatalog, GalaxyMap, NEUTRAL, SystemId};
use nova_view::{Color, DrawCommand, DrawList, Input, MouseButton, Point, Screen};

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

/// Where `system`'s dot is drawn on `map`.
fn dot_at(map: &GalaxyMap, system: &System) -> Point {
    map.view()
        .world_to_screen(Point::new(f32::from(system.x_pos), f32::from(system.y_pos)))
}

#[test]
fn the_stock_map_fits_at_75_percent_with_every_nebula_behind_the_systems() {
    let Some(data) = open() else {
        return;
    };
    let map = GalaxyMap::new(&data);
    assert_eq!(map.problems(), [] as [String; 0]);
    assert_eq!(map.view().zoom(), 2, "75%");
    let list = drawn(&map);
    let pictures: Vec<usize> = list
        .iter()
        .enumerate()
        .filter(|(_, c)| matches!(c, DrawCommand::StretchedPicture { .. }))
        .map(|(at, _)| at)
        .collect();
    let first_shape = list
        .iter()
        .position(|c| matches!(c, DrawCommand::Line { .. } | DrawCommand::Dot { .. }));
    assert_eq!(pictures, [0, 1, 2, 3]);
    assert_eq!(first_shape, Some(4));
}

/// The hyperlinks, worked out here from the raw records, are each drawn
/// once between their systems' dots.
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
    let model_pairs: BTreeSet<(i16, i16)> = map
        .model()
        .links()
        .iter()
        .map(|(a, b)| (a.0, b.0))
        .collect();
    assert_eq!(model_pairs, pairs);

    let screen = |id: i16| dot_at(&map, &raw[&id]);
    let key = |p: Point| (p.x.to_bits(), p.y.to_bits());
    let unordered = |a: Point, b: Point| {
        let (a, b) = (key(a), key(b));
        (a.min(b), a.max(b))
    };
    let mut expected_lines: Vec<_> = pairs
        .iter()
        .map(|&(a, b)| unordered(screen(a), screen(b)))
        .collect();
    let mut drawn_lines: Vec<_> = list
        .iter()
        .filter_map(|c| match c {
            DrawCommand::Line {
                from, to, color, ..
            } if *color == LINK => Some(unordered(*from, *to)),
            _ => None,
        })
        .collect();
    expected_lines.sort_unstable();
    drawn_lines.sort_unstable();
    assert_eq!(drawn_lines.len(), 930);
    assert_eq!(drawn_lines, expected_lines);
}

/// Every system's dot, in its own government's colour worked out here
/// from the raw records, highest ID first.
#[test]
fn the_stock_map_draws_every_system_in_its_governments_colour() {
    let Some(data) = open() else {
        return;
    };
    let raw = systems(&data);
    let map = GalaxyMap::new(&data);
    let list = drawn(&map);
    let colors: BTreeMap<i16, u32> = data
        .records::<Govt>()
        .map(|(id, govt)| (id, govt.expect("every gövt decodes").record.color))
        .collect();
    let expected_dots: Vec<(Point, Color)> = raw
        .iter()
        .rev()
        .map(|(_, system)| {
            let color = system.govt.map_or(NEUTRAL, |govt| {
                let [_, r, g, b] = colors[&govt.0].to_be_bytes();
                Color::rgba(r, g, b, 255)
            });
            (dot_at(&map, system), color)
        })
        .collect();
    let fills: Vec<(Point, Color)> = list
        .iter()
        .filter_map(|c| match c {
            DrawCommand::Dot {
                center,
                size,
                color,
            } if *size == DOT_SIZE => Some((*center, *color)),
            _ => None,
        })
        .collect();
    assert_eq!(fills.len(), 545);
    assert_eq!(fills, expected_dots);
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
    let view = *map.view();
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
