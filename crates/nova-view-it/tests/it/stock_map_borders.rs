//! The galaxy map's government borders over the stock data
//! (`nova_sim::map_borders`, `_AddSystemInfluenceToMap`,
//! `_DrawMapInfluence`). Skips, passing, when `NOVA_DATA` is unset.
//!
//! - `gövt` `Flags2` 0x0004, no influence: Rimerta (142) and Temmin Shard
//!   (185) among others, both with 0x0002 too; 0x0002 alone, the smaller
//!   influence: the Rebellion (141) and the Auroran Empire (156) among
//!   others; the Federation (128) neither.
//!
//! A new pilot's course map over a real session is nova-flight's
//! `stock_map_borders`. That a new pilot's sources hold no government of
//! no influence is nova-sim's (`stock_galaxy_map`).

use crate::common;

use nova_data::GameData;
use nova_sim::govt::{NO_INFLUENCE, SMALL_INFLUENCE};
use nova_sim::{CombatCatalog, Governments, GovtId};
use nova_view::galaxy::GalaxyCatalog;

fn open() -> Option<GameData> {
    let dir = common::nova_data()?;
    Some(GameData::open(&dir, None).expect("the stock data opens"))
}

/// The stock Federation's ID: the government named so.
fn federation(data: &GameData) -> GovtId {
    data.governments()
        .into_iter()
        .find(|govt| govt.name == "Federation")
        .expect("the Federation")
        .id
}

#[test]
fn the_stock_governments_hold_the_borders_flags2_bits() {
    let Some(data) = open() else {
        return;
    };
    let govts = Governments::read(&data);
    // Both have 0x0002 too (`Flags2` 0x27 and 0x26): it never counts.
    for id in [142, 185] {
        assert!(govts.flag2(Some(GovtId(id)), NO_INFLUENCE), "{id}");
    }
    for id in [141, 156] {
        assert!(govts.flag2(Some(GovtId(id)), SMALL_INFLUENCE), "{id}");
        assert!(!govts.flag2(Some(GovtId(id)), NO_INFLUENCE), "{id}");
    }
    let fed = Some(federation(&data));
    assert!(!govts.flag2(fed, NO_INFLUENCE) && !govts.flag2(fed, SMALL_INFLUENCE));
    assert_eq!(data.galaxy().govts, govts, "the map's catalog reads them");
}
