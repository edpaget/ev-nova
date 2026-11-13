//! The galaxy map's government borders over the stock data
//! (`nova_sim::map_borders`, `_AddSystemInfluenceToMap`,
//! `_DrawMapInfluence`). Skips, passing, when `NOVA_DATA` is unset.
//!
//! A new pilot's course map, on a real session, opened and ticked once,
//! shades the cells around the start system in the Federation's `Color`,
//! halved at a full value. The governments' border flags are
//! nova-view-it's `stock_map_borders`.
//!
//! That a new pilot's sources hold no government of no influence is
//! nova-sim's (`stock_galaxy_map`).

use crate::common;

use std::rc::Rc;
use std::time::Duration;

use nova_data::GameData;
use nova_flight::test_flight;
use nova_sim::{CombatCatalog, GovtId};
use nova_view::flight::radar::IRect;
use nova_view::galaxy::borders::{draw_cells, shade};
use nova_view::galaxy::dialog::map_area;
use nova_view::galaxy::{GalaxyCatalog, GalaxyMap};
use nova_view::{Color, DrawCommand, DrawList, Input, Key, Screen};

fn open() -> Option<GameData> {
    let dir = common::nova_data()?;
    Some(GameData::open(&dir, None).expect("the stock data opens"))
}

/// The stock Federation's ID: the government named so.
fn federation(data: &GameData) -> GovtId {
    data.governments()
        .into_iter()
        .find(|govt| govt.name == "Federation")
        .expect("the Federation")
        .id
}

#[test]
fn a_new_pilots_course_map_shades_the_start_in_the_federations_colour() {
    let Some(data) = open() else {
        return;
    };
    let fed = federation(&data);
    let data = Rc::new(data);
    let mut view = test_flight::new(Rc::clone(&data));
    let map_key = Input::Key {
        key: Key::KeyM,
        pressed: true,
        repeat: false,
    };
    view.input(&map_key);
    assert!(view.map_open());
    view.tick(Duration::from_millis(16));
    let map: &GalaxyMap = view.course_map();
    let centre = map
        .border_cells()
        .iter()
        .find(|cell| (cell.x, cell.y) == (114, 105))
        .copied()
        .expect("the start system's cell");
    assert_eq!(centre.step, 1, "the fine pass done in the tick");
    assert_eq!(centre.govt, fed);
    let [_, r, g, b] = data.galaxy().govt_colors[&fed].to_be_bytes();
    let shaded = Color::rgba(
        shade(r, centre.value),
        shade(g, centre.value),
        shade(b, centre.value),
        255,
    );
    let mut cells = DrawList::new();
    draw_cells(
        &mut cells,
        &[centre],
        (221, 136),
        IRect::from_bounds(map_area()),
        &data.galaxy().govt_colors,
    );
    let mut list = DrawList::new();
    map.draw(&mut list);
    let square = cells.iter().next().expect("drawn").clone();
    assert!(
        matches!(&square, DrawCommand::Line { color, .. } if *color == shaded),
        "{square:?}"
    );
    assert!(
        list.iter().any(|command| *command == square),
        "drawn in item 3"
    );
}
