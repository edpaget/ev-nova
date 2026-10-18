//! The stock miners, as the traffic reads their ship types (see
//! `nova_sim::ai::miner`). Skips, passing, when `NOVA_DATA` is unset.
//!
//! - The Asteroid Miner (379) has `Flags3` 0x0003, the economy shuttles
//!   (391-393) and the Sprite (394) 0x0002, the Zephyrs (375, 376)
//!   0x0001, and no other stock ship type is a miner.
//! - Only the Asteroid Miner carries an asteroid scoop among its default
//!   items (`_ShipCanScoop`), so it mines (state 16, a stop without
//!   asteroids); the shuttles and the Sprite, which can't scoop, shuttle
//!   to their nearest stellar.

use crate::common;

use nova_data::GameData;
use nova_sim::combat::armament::Arsenal;
use nova_sim::traffic::table::{ShipKind, kind};
use nova_sim::{PilotCatalog, ShipId};

/// Each stock ship type that is a miner, with its miner bits and
/// whether it scoops, as the traffic spawns it.
fn stock_miners(data: &GameData) -> Vec<(i16, u16, bool)> {
    let outfits = data.outfits();
    let arsenal = Arsenal::read(data);
    data.ships()
        .iter()
        .map(|record| (record.id, kind(record, &outfits, &arsenal)))
        .filter(|(_, kind): &(ShipId, ShipKind)| kind.miner != 0)
        .map(|(id, kind)| (id.0, kind.miner, kind.can_scoop))
        .collect()
}

#[test]
fn the_stock_miners_are_the_asteroid_miner_the_shuttles_the_sprite_and_the_zephyrs() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    assert_eq!(
        stock_miners(&data),
        [
            (375, 0x0001, false),
            (376, 0x0001, false),
            (379, 0x0003, true),
            (391, 0x0002, false),
            (392, 0x0002, false),
            (393, 0x0002, false),
            (394, 0x0002, false),
        ]
    );
}
