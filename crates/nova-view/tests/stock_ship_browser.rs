//! The ship browser over the stock data: every ship, browsed by keyboard.
//! Skips, passing, when `NOVA_DATA` is unset.

mod common;

use std::collections::BTreeSet;
use std::time::Duration;

use nova_data::GameData;
use nova_view::ships::{ShipBrowser, ShipCatalog, ShipEntry, ShipId};
use nova_view::{DrawCommand, DrawList, ImageKey, Input, Key, Screen};

/// Every problem with the browser's view of `ship`.
fn problems(ship: &ShipEntry) -> Vec<String> {
    let id = ship.id.0;
    let mut problems = Vec::new();
    if let Err(err) = &ship.stats {
        problems.push(format!("shïp {id} stats: {err}"));
    }
    if let Err(err) = &ship.description {
        problems.push(format!("shïp {id} description: {err}"));
    }
    if let Err(err) = &ship.sprite {
        problems.push(format!("shïp {id} sprite: {err}"));
    }
    for layer in [&ship.glow, &ship.lights] {
        if let Some(Err(err)) = layer {
            problems.push(format!("shïp {id} layer: {err}"));
        }
    }
    problems
}

/// The `rlëD` IDs of the sprites drawn, in order.
fn sprite_ids(browser: &impl Screen) -> Vec<i16> {
    let mut list = DrawList::new();
    browser.draw(&mut list);
    list.iter()
        .filter_map(|command| match command {
            DrawCommand::Sprite {
                image: ImageKey { id, .. },
                ..
            } => Some(*id),
            _ => None,
        })
        .collect()
}

#[test]
fn every_stock_ship_is_browsed_with_every_frame_and_layer() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let mut browser = ShipBrowser::new(&data);
    let right = Input::Key {
        key: Key::Right,
        pressed: true,
        repeat: false,
    };

    let mut problems_seen = Vec::new();
    let mut visited = Vec::new();
    let (mut glows, mut lights, mut described) = (0, 0, 0);
    // One step per ship, so a browser that never wraps back to the first
    // ship fails the test instead of hanging it.
    let ship_count = data.ship_ids().len();
    let mut wrapped = false;
    for _ in 0..ship_count {
        let ship = browser.current().expect("a ship").clone();
        visited.push(ship.id);
        problems_seen.extend(problems(&ship));
        if let Ok(sprite) = ship.sprite {
            let mut expected = vec![sprite.image_id];
            for (layer, count) in [(&ship.glow, &mut glows), (&ship.lights, &mut lights)] {
                if let Some(Ok(layer)) = layer {
                    expected.push(layer.image_id);
                    *count += 1;
                }
            }
            if sprite_ids(&browser) != expected {
                problems_seen.push(format!("shïp {} draws the wrong sprites", ship.id.0));
            }
            let mut frames = BTreeSet::new();
            for _ in 0..sprite.frames.get() {
                frames.insert(browser.frame().expect("a frame"));
                browser.tick(Duration::from_secs(1) / 30);
            }
            if frames.len() != usize::from(sprite.frames.get()) {
                problems_seen.push(format!(
                    "shïp {} shows {} of its {} frames",
                    ship.id.0,
                    frames.len(),
                    sprite.frames
                ));
            }
        }
        described += usize::from(matches!(ship.description, Ok(Some(_))));
        browser.input(&right);
        if browser.selected() == Some(ShipId(128)) {
            wrapped = true;
            break;
        }
    }

    assert!(
        wrapped,
        "Right did not wrap back to the first ship within {ship_count} presses"
    );

    assert!(problems_seen.is_empty(), "{}", problems_seen.join("\n"));
    assert_eq!(visited.len(), 288);
    assert_eq!(visited.first(), Some(&ShipId(128)));
    assert_eq!(visited.last(), Some(&ShipId(895)));
    assert_eq!((glows, lights, described), (281, 107, 99));
}
