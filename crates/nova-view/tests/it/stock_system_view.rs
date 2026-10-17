//! The system view over the stock data: every system's scene matches its
//! raw records, and every system can be entered from the galaxy map and
//! left again. Skips, passing, when `NOVA_DATA` is unset.

// Positions here are small whole numbers, exact in floating point.
#![allow(clippy::float_cmp)]

mod common;

use std::collections::BTreeMap;

use nova_data::records::spin::Spin;
use nova_data::records::stellar::Stellar;
use nova_data::records::system::System;
use nova_data::{GameData, SystemId};
use nova_view::system::SystemScene;
use nova_view::{
    DrawCommand, DrawList, ImageKey, Input, Key, MouseButton, Navigator, Point, Screen,
};

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

/// A stellar's raw record.
fn stellar(data: &GameData, id: i16) -> Stellar {
    data.get::<Stellar>(id)
        .expect("present")
        .expect("decodes")
        .record
        .clone()
}

/// Every system's scene has its stellars, in navigation order, at the
/// positions their raw `spöb`s give, with the sheets their raw `spïn`s
/// name. Only the Wormholes whose `AnimDelay` is 2 animate.
#[test]
fn every_stock_systems_scene_matches_its_records() {
    let Some(data) = open() else {
        return;
    };
    let raw = systems(&data);
    assert_eq!(raw.len(), 545);
    let mut problems = Vec::new();
    let mut animated = BTreeMap::new();
    for (&id, system) in &raw {
        let scene = SystemScene::load(&data, SystemId(id));
        if !scene.problems().is_empty() {
            problems.push(format!("sÿst {id}: {:?}", scene.problems()));
        }
        let nav: Vec<i16> = system.nav_def.iter().flatten().map(|s| s.0).collect();
        let ids: Vec<i16> = scene.stellars().iter().map(|s| s.id.0).collect();
        if ids != nav {
            problems.push(format!("sÿst {id} has {ids:?}, not {nav:?}"));
        }
        for placed in scene.stellars() {
            let record = stellar(&data, placed.id.0);
            let position = Point::new(f32::from(record.x_pos), f32::from(record.y_pos));
            if placed.position != position {
                problems.push(format!(
                    "spöb {} at {:?}, not {position:?}",
                    placed.id.0, placed.position
                ));
            }
            let spin = data
                .get::<Spin>(1000 + record.graphic_type)
                .expect("present")
                .expect("decodes");
            match &placed.sprite {
                Ok(sheet) if sheet.image_id == spin.record.sprites_id => {}
                other => problems.push(format!(
                    "spöb {}: {other:?}, not rlëD {}",
                    placed.id.0, spin.record.sprites_id
                )),
            }
            animated.insert(placed.id.0, (placed.animation.is_animated(), record));
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
    let moving: Vec<&Stellar> = animated
        .values()
        .filter(|(is, _)| *is)
        .map(|(_, record)| record)
        .collect();
    assert_eq!(moving.len(), 23);
    assert!(
        moving
            .iter()
            .all(|r| r.anim_delay == 2 && r.graphic_type == 59),
        "only Wormholes with a delay of 2 animate"
    );
}

fn input(navigator: &mut Navigator<&GameData>, event: Input) {
    navigator.input(&event);
}

fn click(navigator: &mut Navigator<&GameData>, at: Point) {
    for pressed in [true, false] {
        input(
            navigator,
            Input::PointerButton {
                button: MouseButton::Left,
                pressed,
                at,
            },
        );
    }
}

fn press(navigator: &mut Navigator<&GameData>, key: Key) {
    input(
        navigator,
        Input::Key {
            key,
            pressed: true,
            repeat: false,
        },
    );
}

/// Every system, in turn, is selected on the map (by clicking its dot at
/// most once per system there), entered with Return, drawn with one sprite
/// per stellar at its position, and left with Escape, which keeps it
/// selected.
#[test]
fn every_stock_system_can_be_entered_from_the_map_and_left() {
    let Some(data) = open() else {
        return;
    };
    let raw = systems(&data);
    let mut navigator = Navigator::new(&data);
    let mut problems = Vec::new();
    for &id in raw.keys() {
        let map = navigator.map();
        let system = map.model().system(SystemId(id)).expect("on the map");
        let dot = map.view().world_to_screen(system.position());
        let stack = map.model().stack(SystemId(id)).len();
        // Start from empty space, then click until it is selected.
        click(&mut navigator, Point::new(-100.0, -100.0));
        for _ in 0..stack {
            click(&mut navigator, dot);
            if navigator.map().selected() == Some(SystemId(id)) {
                break;
            }
        }
        if navigator.map().selected() != Some(SystemId(id)) {
            problems.push(format!("sÿst {id} cannot be selected"));
            continue;
        }

        press(&mut navigator, Key::Enter);
        let Some(view) = navigator.system() else {
            problems.push(format!("sÿst {id}: Return opened nothing"));
            continue;
        };
        if view.scene().id() != SystemId(id) {
            problems.push(format!("sÿst {id} opened {:?}", view.scene().id()));
        }
        let mut list = DrawList::new();
        navigator.draw(&mut list);
        let drawn: Vec<(ImageKey, Point)> = list
            .iter()
            .filter_map(|c| match c {
                DrawCommand::Sprite { image, center, .. } => Some((*image, *center)),
                _ => None,
            })
            .collect();
        let expected: Vec<Point> = raw[&id]
            .nav_def
            .iter()
            .flatten()
            .map(|s| {
                let record = stellar(&data, s.0);
                Point::new(
                    512.0 + f32::from(record.x_pos),
                    384.0 + f32::from(record.y_pos),
                )
            })
            .collect();
        let centres: Vec<Point> = drawn.iter().map(|(_, at)| *at).collect();
        if centres != expected {
            problems.push(format!("sÿst {id} drew {centres:?}, not {expected:?}"));
        }

        press(&mut navigator, Key::Escape);
        if navigator.system().is_some() || navigator.map().selected() != Some(SystemId(id)) {
            problems.push(format!("sÿst {id}: Escape did not go back to it"));
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}
