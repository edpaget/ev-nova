//! The stock swarming ships, as the traffic reads their ship types (see
//! `nova_sim::ai::swarm`). Skips, passing, when `NOVA_DATA` is unset.

use crate::common;

use nova_data::GameData;
use nova_sim::PilotCatalog;
use nova_sim::combat::armament::Arsenal;

#[test]
fn stock_swarming_ships_are_counted() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let arsenal = Arsenal::read(&data);
    let ships = data.ships();
    let swarming: Vec<String> = ships
        .iter()
        .filter(|ship| arsenal.hull(ship.id).swarms)
        .map(|ship| format!("{} {}", ship.id.0, ship.name))
        .collect();
    assert_eq!((swarming.len(), ships.len()), (155, 288), "{swarming:#?}");
}
