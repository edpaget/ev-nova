//! The stock stand-off ships, as the traffic reads their ship types (see
//! `nova_sim::ai::stand_off`). Skips, passing, when `NOVA_DATA` is unset.
//!
//! - 142 of the 288 stock ship types have `shïp` `Flags2` 0x0002 and stand
//!   off: about 40 classes (the Enterprise, IDA Frigate, Federation
//!   destroyers, patrol boats and carriers, the Aurora Cruiser and Carrier,
//!   the Manticore, pirate capitals) and their variants.
//! - The IDA Frigate (140) reaches 1560 pixels with its stock weapons
//!   (`_ShipMaxWeaponRange`), so keeps within 1326, and 663 against a
//!   disabled target; the Federation Destroyer (141) reaches 1147, so
//!   keeps within 974, and 487.
//! - The Vell-os Javelin (175) reaches only with a fuel weapon: nothing
//!   with its tank dry.

use crate::common;

use nova_data::GameData;
use nova_sim::ai::stand_off::{band, max_weapon_range};
use nova_sim::combat::armament::Arsenal;
use nova_sim::stats::ShipStats;
use nova_sim::traffic::npc::Npc;
use nova_sim::{PilotCatalog, ShipId};

/// An NPC of stock ship type `ship` as the traffic arms it: its hull, its
/// stock weapons and their rounds, its tank holding `fuel`.
fn stock_npc(arsenal: &Arsenal, ship: ShipId, fuel: f32) -> Npc {
    let mut npc = nova_sim::traffic::fixture::npc(1, ShipStats::default());
    let (armament, rounds) = arsenal.npc(ship);
    npc.armament = armament;
    npc.rounds = rounds;
    npc.hull = arsenal.hull(ship);
    npc.reserves.fuel.now = fuel;
    npc
}

/// A full tank, for any stock ship.
const FULL: f32 = 1e6;

#[test]
fn stock_stand_off_ships_are_142_of_288() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let arsenal = Arsenal::read(&data);
    let ships = data.ships();
    let standing_off = ships
        .iter()
        .filter(|ship| arsenal.hull(ship.id).stands_off)
        .count();
    assert_eq!((standing_off, ships.len()), (142, 288));
    for id in [139, 140, 141, 142, 143, 154, 230] {
        assert!(arsenal.hull(ShipId(id)).stands_off, "{id}");
    }
}

#[test]
#[allow(clippy::float_cmp)]
fn stock_stand_off_ships_keep_085_of_their_longest_reach() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let arsenal = Arsenal::read(&data);
    let frigate = stock_npc(&arsenal, ShipId(140), FULL);
    assert_eq!(max_weapon_range(&frigate), 1560.0, "the IDA Frigate");
    assert_eq!(
        (band(&frigate, false), band(&frigate, true)),
        (Some(1326.0), Some(663.0))
    );
    let destroyer = stock_npc(&arsenal, ShipId(141), FULL);
    assert_eq!(max_weapon_range(&destroyer), 1147.0, "the Fed Destroyer");
    assert_eq!(
        (band(&destroyer, false), band(&destroyer, true)),
        (Some(974.0), Some(487.0))
    );
}

#[test]
#[allow(clippy::float_cmp)]
fn a_stock_javelin_reaches_only_with_fuel_in_its_tank() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let arsenal = Arsenal::read(&data);
    assert_eq!(
        max_weapon_range(&stock_npc(&arsenal, ShipId(175), FULL)),
        340.0
    );
    assert_eq!(
        max_weapon_range(&stock_npc(&arsenal, ShipId(175), 0.0)),
        0.0
    );
}
