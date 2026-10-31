//! The exchange dialog's words over the stock data. Skips, passing, when
//! `NOVA_DATA` is unset.
//!
//! - Stock `STR#` 2002's entries #1, #2, #197-#205, #363-#368, #391 and
//!   #392 are the words `TradeWords::STOCK` holds: "ton", "tons",
//!   "Commodity:", "In Fleet:", "In Hold:", "Price:", "Low", "Med",
//!   "High", "Higher", "Lower", "Other cargo:", "Free cargo space", "in
//!   your ship", "in your fleet", "mission cargo", "miscellaneous goods",
//!   "of" and "and".
//! - Stock stellars that two `jünk` name on one side list only the higher
//!   ID of those whose test holds, as `_DoTradeDialog` does: Rauther, 150
//!   over 136; Schmoo, 149 while b43 is set and 146 otherwise. Europa keeps
//!   both its rows, the bought one first.

use crate::common;

use nova_data::GameData;
use nova_sim::catalog::CommCatalog;
use nova_sim::control::Bit;
use nova_sim::news;
use nova_sim::trade_dialog::TradeWords;
use nova_sim::{Good, JunkId, Pilot, PilotCatalog, Session, StellarId, SystemId};

#[test]
fn the_exchanges_stock_words_are_str_2002s() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    assert_eq!(
        TradeWords::from_list(&data.string_list(news::MISC_STRINGS)),
        TradeWords::STOCK
    );
}

const RAUTHER: StellarId = StellarId(191);
const SCHMOO: StellarId = StellarId(322);
const EUROPA: StellarId = StellarId(160);

/// The `jünk` rows, as (ID, slot), of the exchange at `stellar` for a new
/// stock pilot with `bits` set, in the market's order.
fn junk_rows(data: &GameData, stellar: StellarId, bits: &[u16]) -> Vec<(i16, u8)> {
    let system = (128..2176)
        .map(SystemId)
        .find(|&system| {
            data.landing_sites(system)
                .iter()
                .any(|site| site.id == stellar)
        })
        .expect("a system holds the stellar");
    let mut pilot = Pilot::new(data, "Stock").expect("the stock first chär starts");
    for &bit in bits {
        pilot.bits_mut().set(Bit::new(bit).expect("a control bit"));
    }
    let mut save: serde_json::Value =
        serde_json::from_str(&nova_sim::save::encode(&pilot)).expect("JSON");
    save["system"] = serde_json::json!(system.0);
    save["stellar"] = serde_json::json!(stellar.0);
    let saved = nova_sim::save::decode(&save.to_string()).expect("a pilot");
    let mut session = Session::fly(data, saved).expect("flies");
    session.land_at(data, system, stellar).expect("lands");
    let market = session.market().expect("a trade center");
    market
        .rows
        .iter()
        .filter_map(|row| match row.good {
            Good::Junk(JunkId(id)) => Some((id, row.slot)),
            Good::Commodity(_) => None,
        })
        .collect()
}

#[test]
fn a_stock_stellar_lists_one_junk_a_side_the_highest_id_whose_test_holds() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    assert_eq!(
        junk_rows(&data, RAUTHER, &[]),
        [(150, 6)],
        "Chrotite Gas, not Ice-cat pelts"
    );
    assert_eq!(junk_rows(&data, SCHMOO, &[]), [(146, 6)], "Opals");
    assert_eq!(
        junk_rows(&data, SCHMOO, &[43]),
        [(149, 6)],
        "Durknen Girns, with b43"
    );
    assert_eq!(
        junk_rows(&data, EUROPA, &[]),
        [(136, 6), (134, 7)],
        "both rows, the bought one first"
    );
}
