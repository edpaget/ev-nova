//! The galaxy map's government borders over the stock data
//! (`nova_sim::map_borders`, `_AddSystemInfluenceToMap`,
//! `_DrawMapInfluence`). Skips, passing, when `NOVA_DATA` is unset.
//!
//! - `gövt` `Flags2` 0x0004, no influence: Rimerta (142) and Temmin Shard
//!   (185) among others, both with 0x0002 too; 0x0002 alone, the smaller
//!   influence: the Rebellion (141) and the Auroran Empire (156) among
//!   others; the Federation (128) neither.
//! - A new pilot's course map, opened and ticked once, shades the cells
//!   around the start system in the Federation's `Color`, halved at a full
//!   value.

use crate::common;

use std::time::Duration;

use nova_data::GameData;
use nova_sim::fixture::SessionBuilder;
use nova_sim::govt::{NO_INFLUENCE, SMALL_INFLUENCE};
use nova_sim::{CombatCatalog, Governments, GovtId};
use nova_view::flight::FlightView;
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
fn the_stock_governments_hold_the_borders_flags2_bits() {
    let Some(data) = open() else {
        return;
    };
    let govts = Governments::read(&data);
    // Both have 0x0002 too (`Flags2` 0x27 and 0x26): it never counts.
    for id in [142, 185] {
        assert!(govts.flag2(Some(GovtId(id)), NO_INFLUENCE), "{id}");
    }
    for id in [141, 156] {
        assert!(govts.flag2(Some(GovtId(id)), SMALL_INFLUENCE), "{id}");
        assert!(!govts.flag2(Some(GovtId(id)), NO_INFLUENCE), "{id}");
    }
    let fed = Some(federation(&data));
    assert!(!govts.flag2(fed, NO_INFLUENCE) && !govts.flag2(fed, SMALL_INFLUENCE));
    assert_eq!(data.galaxy().govts, govts, "the map's catalog reads them");
}

#[test]
fn a_new_pilots_sources_hold_no_government_of_no_influence() {
    let Some(data) = open() else {
        return;
    };
    let session = SessionBuilder::flying(&data).build();
    let sources = session.map_border_sources();
    assert!(!sources.coarse.is_empty());
    let govts = Governments::read(&data);
    for source in sources
        .coarse
        .iter()
        .chain(sources.walk.iter().map(|(_, source)| source))
    {
        assert!(!govts.flag2(Some(source.govt), NO_INFLUENCE), "{source:?}");
    }
    let start = session.system();
    assert!(
        sources.coarse.iter().any(|source| source.system == start),
        "the start system adds"
    );
}

#[test]
fn a_new_pilots_course_map_shades_the_start_in_the_federations_colour() {
    let Some(data) = open() else {
        return;
    };
    let fed = federation(&data);
    let mut view = FlightView::new(&data);
    let map_key = Input::Key {
        key: Key::Char('m'),
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
