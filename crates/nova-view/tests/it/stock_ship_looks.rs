//! The borrowed stock look over the player's own stock data: each role
//! default's sheet, borrowed and scaled, keeps the sprite port's sheet
//! contract.
//!
//! The stock data is copyrighted and never committed, so this test gets
//! its location from `common` and skips, passing, without `NOVA_DATA`. It
//! writes nothing and asserts in memory only.

use std::collections::BTreeMap;
use std::rc::Rc;

use nova_data::stock_look::{Borrow, ROLE_DEFAULTS, Scale};
use nova_data::store::GameData;
use nova_data::{ShipId, StockArt};
use nova_view::flight::sheet_check::check_sheets;
use nova_view::flight::{ShipSprites, StockShipLooks};

use crate::common::nova_data;

#[test]
fn stock_ship_looks_of_every_role_default_keep_the_sheet_contract() {
    let Some(dir) = nova_data() else { return };
    let art = Rc::new(StockArt::new(
        GameData::open(&dir, None).expect("the stock data opens"),
    ));
    // Each default at its own size, and at a hull's size of 30 px.
    let borrows: BTreeMap<ShipId, Borrow> = ROLE_DEFAULTS
        .iter()
        .zip(128..)
        .flat_map(|(&(_, stock), id)| {
            let side = art
                .ship_sprite_info(ShipId(stock))
                .map(|sprite| {
                    sprite
                        .sheet
                        .info
                        .frame_width
                        .max(sprite.sheet.info.frame_height)
                })
                .expect("the default's sheet reads");
            [
                (
                    ShipId(id),
                    Borrow {
                        stock: ShipId(stock),
                        scale: Scale::ONE,
                    },
                ),
                (
                    ShipId(id + 100),
                    Borrow {
                        stock: ShipId(stock),
                        scale: Scale::of(30, side),
                    },
                ),
            ]
        })
        .collect();
    let ships: Vec<ShipId> = borrows.keys().copied().collect();
    let looks = StockShipLooks::new(art, borrows);
    for &ship in &ships {
        assert!(looks.ship_sheet(ship).is_ok(), "shïp {}", ship.0);
    }
    assert_eq!(check_sheets(&looks, &ships), Ok(()));
}
