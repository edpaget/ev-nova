//! The stock look's role defaults over the player's own stock data.
//!
//! The stock data is copyrighted and never committed, so these tests get
//! its location from `common` and skip, passing, without `NOVA_DATA`. They
//! write nothing and assert in memory only.

use std::collections::BTreeMap;

use nova_data::stock_look::{ROLE_DEFAULTS, resolve};
use nova_data::store::GameData;
use nova_data::{ShipId, ShipLine, StockArt};

use crate::common::nova_data;

fn stock_art() -> Option<StockArt> {
    let dir = nova_data()?;
    Some(StockArt::new(
        GameData::open(&dir, None).expect("the stock data opens"),
    ))
}

#[test]
fn every_role_default_is_a_stock_ship_whose_sheet_reads() {
    let Some(art) = stock_art() else { return };
    let unread: Vec<_> = ROLE_DEFAULTS
        .iter()
        .filter_map(|&(role, stock)| {
            let sheet = art.ship_sprite_info(ShipId(stock));
            sheet
                .err()
                .map(|error| format!("{role:?} {stock}: {error}"))
        })
        .collect();
    assert_eq!(unread, Vec::<String>::new());
}

#[test]
fn a_hull_of_each_role_borrows_its_default() {
    let Some(art) = stock_art() else { return };
    let lines: BTreeMap<_, _> = ROLE_DEFAULTS
        .iter()
        .zip(128..)
        .map(|(&(role, _), id)| {
            let line = ShipLine {
                chain: vec![format!("{role:?}")],
                role: Some(role),
            };
            (ShipId(id), line)
        })
        .collect();
    let game = GameData::default().with_ship_lines(lines);
    let resolved = resolve(&game, &art, &BTreeMap::new());
    assert_eq!(resolved.warnings, Vec::<String>::new());
    let borrowed: Vec<i16> = resolved.borrows.values().map(|b| b.stock.0).collect();
    let defaults: Vec<i16> = ROLE_DEFAULTS.iter().map(|&(_, stock)| stock).collect();
    assert_eq!(borrowed, defaults);
}
