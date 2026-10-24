//! Stock ship classes as the escort dialog reads them (`_LoadObjectData`
//! @0x7a3a5-0x7a44b; see `nova_sim::escort_dialog`). Skips, passing, when
//! `NOVA_DATA` is unset.
//!
//! - `shïp` 361 "Shuttle;Second-Hand - poor" (`Cost` 4995) upgrades to the
//!   Shuttle (128) for 6,000 and, captured, sells for 499: its
//!   `EscSellValue` is 0, so the sale is its `Cost` x 0.1, truncated.

use crate::common;

use nova_sim::{PilotCatalog, ShipId};

#[test]
fn the_second_hand_shuttle_upgrades_to_the_shuttle_and_sells_for_a_tenth_of_its_cost() {
    let Some(data) = common::opened() else {
        return;
    };
    let ship = data
        .ships()
        .into_iter()
        .find(|record| record.id == ShipId(361))
        .expect("shïp 361 reads");
    assert_eq!(ship.cost, 4_995);
    assert_eq!(ship.upgrade_to, Some(ShipId(128)));
    assert_eq!(ship.esc_upgrd_cost, 6_000);
    assert_eq!(ship.esc_sell_value, 499);
}
