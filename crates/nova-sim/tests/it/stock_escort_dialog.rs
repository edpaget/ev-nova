//! Stock ship classes as the escort dialog reads them (`_LoadObjectData`
//! @0x7a3a5-0x7a44b; see `nova_sim::escort_dialog`). Skips, passing, when
//! `NOVA_DATA` is unset.
//!
//! - `shïp` 361 "Shuttle;Second-Hand - poor" (`Cost` 4995) upgrades to the
//!   Shuttle (128) for 6,000 and, captured, sells for 499: its
//!   `EscSellValue` is 0, so the sale is its `Cost` x 0.1, truncated.
//! - The dialog's strings and its report's are `STR#` 2002's and 150's
//!   where the engine names them.

use crate::common;

use nova_sim::escort_dialog::{
    CANCEL_SALE, CANCEL_UPGRADE, CANNOT_BE_UPGRADED, CAPTURED_ESCORT, CREDIT, CREDITS, ESCORT_WAS,
    ESCORTS_WERE, RELEASE, SELL_ESCORT, SELL_PRICE, SOLD_FOR, UPGRADE_COST, UPGRADE_ESCORT,
    UPGRADED_FOR, WILL_BE_SOLD, WILL_BE_UPGRADED,
};
use nova_sim::{CommCatalog, PilotCatalog, ShipId};

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

#[test]
fn the_escort_dialogs_strings_are_where_it_reads_them() {
    let Some(data) = common::opened() else {
        return;
    };
    let messages = CommCatalog::string_list(&data, 2002);
    for (n, text) in [
        (32, CREDIT),
        (33, CREDITS),
        (167, CAPTURED_ESCORT),
        (292, WILL_BE_UPGRADED),
        (293, UPGRADE_COST),
        (294, CANNOT_BE_UPGRADED),
        (295, WILL_BE_SOLD),
        (296, SELL_PRICE),
        (298, ESCORT_WAS),
        (299, ESCORTS_WERE),
        (300, SOLD_FOR),
        (301, UPGRADED_FOR),
    ] {
        assert_eq!(messages[n - 1], text, "STR# 2002 #{n}");
    }
    let buttons = CommCatalog::string_list(&data, 150);
    for (n, text) in [
        (32, RELEASE),
        (52, UPGRADE_ESCORT),
        (53, CANCEL_UPGRADE),
        (54, SELL_ESCORT),
        (55, CANCEL_SALE),
    ] {
        assert_eq!(buttons[n - 1], text, "STR# 150 #{n}");
    }
}
