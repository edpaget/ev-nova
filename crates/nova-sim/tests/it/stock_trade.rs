//! The exchange dialog's words over the stock data. Skips, passing, when
//! `NOVA_DATA` is unset.
//!
//! - Stock `STR#` 2002's entries #1, #2, #197-#205, #363-#368, #391 and
//!   #392 are the words `TradeWords::STOCK` holds: "ton", "tons",
//!   "Commodity:", "In Fleet:", "In Hold:", "Price:", "Low", "Med",
//!   "High", "Higher", "Lower", "Other cargo:", "Free cargo space", "in
//!   your ship", "in your fleet", "mission cargo", "miscellaneous goods",
//!   "of" and "and".

use crate::common;

use nova_data::GameData;
use nova_sim::catalog::CommCatalog;
use nova_sim::news;
use nova_sim::trade_dialog::TradeWords;

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
