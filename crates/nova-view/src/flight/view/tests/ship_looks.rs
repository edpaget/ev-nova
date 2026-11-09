//! The flight's second ship sprites
//! (rdm:phase/play-the-sample/phase-12-new-game-stock-sprite-look): a
//! ship type whose catalog sheet can't be read draws the looks' sheet
//! ([`FlightView::with_ship_looks`]) before its outline, for the player and
//! for traffic; a catalog sheet that reads wins; looks that can't answer
//! leave the outline and the crossed box as before. On [`FakeFlight`],
//! with a hand-written looks fake: the borrowed sheet's contents are
//! `StockShipLooks`' and nova-data's.

use std::rc::Rc;

use nova_data::ShipImage;
use nova_data::stock_look::Scale;

use super::faked_world::{world, world_list};
use super::*;
use crate::outline::{Outline, OutlineTriangle};
use crate::{ImageKind, SheetImage};

/// Answers the sheets it holds, by ship, and an error for every other;
/// records each ship asked. It never has an outline, and records any
/// asked, which none should be.
#[derive(Default)]
struct Looks {
    sheets: Vec<(i16, ShipSheet)>,
    asked: RefCell<Vec<ShipId>>,
    outlines_asked: RefCell<Vec<ShipId>>,
}

impl ShipSprites for Looks {
    fn ship_sheet(&self, id: ShipId) -> Result<ShipSheet, String> {
        self.asked.borrow_mut().push(id);
        self.sheets
            .iter()
            .find(|(ship, _)| *ship == id.0)
            .map(|(_, sheet)| *sheet)
            .ok_or_else(|| format!("no look for {}", id.0))
    }

    fn ship_outline(&self, id: ShipId) -> Option<Outline> {
        self.outlines_asked.borrow_mut().push(id);
        None
    }
}

/// The borrowed sheet of stock ship 300: 30 x 30, 36 rotations.
fn borrowed() -> ShipSheet {
    ShipSheet {
        image: SheetImage::Stock {
            ship: 300,
            image: ShipImage::Base,
            scale: Scale::of(3, 4),
        },
        frame_width: 30,
        frame_height: 30,
        image_size: (30, 30),
        ..crate::flight::fake_catalog::sheet()
    }
}

/// Looks borrowing [`borrowed`] for `ships`.
fn looks_for(ships: &[i16]) -> Rc<Looks> {
    Rc::new(Looks {
        sheets: ships.iter().map(|&ship| (ship, borrowed())).collect(),
        ..Looks::default()
    })
}

/// A triangle outline, so a ship drawn from it shows.
fn dart() -> Outline {
    Outline {
        triangles: vec![OutlineTriangle {
            corners: [at(0.0, 10.0), at(3.0, -4.0), at(-3.0, -4.0)],
            color: Color::rgba(255, 0, 0, 255),
        }],
    }
}

/// The fake catalog with ship 128's sheet unreadable and the dart as the
/// outline of 128 and 130, with ship 130's traffic.
fn sheetless() -> FakeCatalog {
    FakeCatalog {
        sheet: Err("no shän 128 for shïp 128".to_owned()),
        outlines: vec![(128, dart()), (130, dart())],
        ..trafficked(&[130], 1, 130, 3)
    }
}

/// The player at the centre and an NPC of ship 130 at (100, -100).
fn with_npc_130() -> FlightFrame {
    FlightFrame {
        traffic_ships: vec![ShipId(130)],
        ..in_sol_with(vec![
            the_player_at(Vec2::ZERO, 0.0),
            record::npc(0, 130, Vec2::new(100.0, -100.0), 90.0),
        ])
    }
}

/// The stock-ship keys `list` draws.
fn stock_keys(list: &DrawList) -> Vec<ImageKey> {
    list.iter()
        .filter_map(|command| match *command {
            DrawCommand::Sprite { image, .. } | DrawCommand::Colorized { image, .. } => Some(image),
            _ => None,
        })
        .filter(|key| matches!(key.kind, ImageKind::StockShip { .. }))
        .collect()
}

fn triangles(list: &DrawList) -> usize {
    list.iter()
        .filter(|command| matches!(command, DrawCommand::Triangle { .. }))
        .count()
}

#[test]
fn a_ship_with_no_catalog_sheet_draws_the_looks_sheet_and_reads_no_outline() {
    let looks = looks_for(&[128, 130]);
    let view = world(sheetless(), with_npc_130()).with_ship_looks(looks.clone());
    assert_eq!(view.sheet, Ok(borrowed()), "the player's");
    assert_eq!(view.outline, None);
    assert_eq!(
        view.npc_sheets[&ShipId(130)],
        Ok(borrowed()),
        "the traffic's"
    );
    assert!(!view.npc_outlines.contains_key(&ShipId(130)));
    let list = world_list(&view);
    let keys = stock_keys(&list);
    assert_eq!(keys.len(), 2, "the player and the npc: {keys:?}");
    assert!(keys.iter().all(|key| key.id == 300));
    assert_eq!(triangles(&list), 0, "no outline drawn");
    assert_eq!(*looks.outlines_asked.borrow(), []);
}

#[test]
fn traffic_read_after_the_looks_are_given_draws_them_too() {
    let looks = looks_for(&[130]);
    let mut view = world(
        sheetless(),
        in_sol_with(vec![the_player_at(Vec2::ZERO, 0.0)]),
    )
    .with_ship_looks(looks);
    assert!(!view.npc_sheets.contains_key(&ShipId(130)));
    super::faked_hud::shows(&mut view, with_npc_130());
    assert_eq!(view.npc_sheets[&ShipId(130)], Ok(borrowed()));
    assert!(!view.npc_outlines.contains_key(&ShipId(130)));
}

#[test]
fn a_catalog_sheet_that_reads_wins_over_the_looks() {
    let looks = looks_for(&[128, 129]);
    let view = world(
        trafficked(&[129], 1, 129, 3),
        among(vec![npc_129(0, Vec2::new(100.0, -100.0), 90.0)]),
    )
    .with_ship_looks(looks.clone());
    assert_eq!(view.sheet, Ok(crate::flight::fake_catalog::sheet()));
    assert!(view.npc_sheets[&ShipId(129)].is_ok());
    assert_ne!(view.npc_sheets[&ShipId(129)], Ok(borrowed()));
    assert_eq!(*looks.asked.borrow(), [], "never asked");
    assert_eq!(stock_keys(&world_list(&view)), []);
}

#[test]
fn when_the_looks_have_no_sheet_either_the_outline_draws_with_the_catalog_s_reason() {
    let looks = looks_for(&[]);
    let view = world(sheetless(), with_npc_130()).with_ship_looks(looks.clone());
    assert_eq!(view.sheet, Err("no shän 128 for shïp 128".to_owned()));
    assert_eq!(view.outline, Some(dart()));
    assert_eq!(view.npc_outlines.get(&ShipId(130)), Some(&dart()));
    assert_eq!(triangles(&world_list(&view)), 2, "both darts");
    assert_eq!(*looks.asked.borrow(), [ShipId(128), ShipId(130)]);
}

#[test]
fn looks_given_after_the_opening_read_the_player_s_and_the_read_traffic_s_sheets_again() {
    let mut view = world(sheetless(), with_npc_130());
    assert_eq!(view.outline, Some(dart()), "the stand-in at first");
    assert!(view.npc_outlines.contains_key(&ShipId(130)));
    view = view.with_ship_looks(looks_for(&[128, 130]));
    assert_eq!(view.sheet, Ok(borrowed()));
    assert_eq!(view.outline, None);
    assert_eq!(view.npc_sheets[&ShipId(130)], Ok(borrowed()));
    assert!(!view.npc_outlines.contains_key(&ShipId(130)));
}

#[test]
fn without_looks_or_with_looks_that_answer_nothing_the_flight_draws_as_before() {
    for catalog in [sheetless, catalog] {
        let plain = world(catalog(), with_npc_130());
        let empty = world(catalog(), with_npc_130()).with_ship_looks(looks_for(&[]));
        assert_eq!(world_list(&plain), world_list(&empty));
        let (mut plain, mut empty) = (plain, empty);
        let (mut drawn, mut also) = (DrawList::new(), DrawList::new());
        plain.draw(&mut drawn);
        empty.draw(&mut also);
        assert_eq!(drawn, also);
        plain.tick(TICK);
        empty.tick(TICK);
    }
}

#[test]
fn the_flight_is_driven_the_same_with_looks_or_without() {
    let run = |looks: Option<Rc<Looks>>| {
        let mut view = world(sheetless(), with_npc_130());
        if let Some(looks) = looks {
            view = view.with_ship_looks(looks);
        }
        for _ in 0..5 {
            view.tick(TICK);
        }
        view.input(&Input::Key {
            key: Key::ArrowUp,
            pressed: true,
            repeat: false,
        });
        for _ in 0..5 {
            view.tick(TICK);
        }
        let sim = view.sim();
        (
            sim.applied().to_vec(),
            sim.edits().to_vec(),
            sim.stepped_calls().to_vec(),
            sim.asks().to_vec(),
            sim.screens_asked().to_vec(),
        )
    };
    let without = run(None);
    let with = run(Some(looks_for(&[128, 130])));
    assert!(!without.2.is_empty(), "the flight was stepped");
    assert_eq!(with, without);
}
