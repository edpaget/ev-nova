//! The board reach over the stock data (rdm ship-handling-fidelity phase
//! 17): every stock ship has a `shän` of positive `BaseXSize` and
//! `BaseYSize`, its board reach is half of each on its axis, and no stock
//! class's sizes differ, so its reach is the same on both axes. Skips,
//! passing, when `NOVA_DATA` is unset.

use crate::common;

use nova_data::GameData;
use nova_sim::combat::hull::HullSpec;
use nova_sim::{CombatCatalog, Vec2};

#[test]
#[allow(clippy::float_cmp)]
fn every_stock_ship_reaches_half_its_shän_sizes_and_the_same_on_both_axes() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let hulls = data.hulls();
    assert!(!hulls.is_empty());
    for record in hulls {
        let (Some(width), Some(height)) = (record.size, record.height) else {
            panic!("{:?} has no shän sizes", record.id);
        };
        assert!(width > 0 && height > 0, "{:?}", record.id);
        let reach = HullSpec::new(&record).board_reach;
        assert_eq!(
            reach,
            Vec2::new(f32::from(width) / 2.0, f32::from(height) / 2.0),
            "{:?}",
            record.id
        );
        assert_eq!(reach.x, reach.y, "{:?}", record.id);
    }
}
