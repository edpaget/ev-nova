//! The stock auto-refuellers (rdm play-feel-2 phase 20, see
//! `nova_sim::auto_refuel` and the session's `auto_refuel`). Skips,
//! passing, when `NOVA_DATA` is unset.
//!
//! - The stock `oütf`s with `ModType` 19 among their four mods: 186
//!   "Auto-recharger" and 251 "Vell-os Area Map" (first mod), and the
//!   hidden ";T" strength outfits 207, 209, 336 and 337 (fourth mod).
//!   Each counts as an auto-refueller.
//! - A new stock pilot owning the Auto-recharger, landing at Port Kane
//!   (`spöb` 137, inhabited, in the start system) with its tank low, is
//!   topped up at a credit a unit; without it nothing changes.

use crate::common;

use std::collections::BTreeMap;

use nova_data::GameData;
use nova_sim::auto_refuel::{AUTO_REFUELLER, carries_auto_refueller};
use nova_sim::outfitter::outfit_mods;
use nova_sim::{OutfitId, Pilot, PilotCatalog, Session, StellarId};

#[test]
fn the_stock_auto_refuellers_are_the_outfits_of_mod_type_19() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let outfits = data.outfits();
    let refuellers: Vec<i16> = outfits
        .iter()
        .filter(|record| record.mods.iter().any(|&(kind, _)| kind == AUTO_REFUELLER))
        .map(|record| record.id.0)
        .collect();
    assert_eq!(refuellers, [186, 207, 209, 251, 336, 337]);
    for id in [186, 251] {
        let owned = BTreeMap::from([(OutfitId(id), 1)]);
        assert!(
            carries_auto_refueller(&outfit_mods(&owned, &outfits)),
            "oütf {id}"
        );
    }
}

/// A new stock pilot with its tank at 10 and 10,000 credits, owning the
/// Auto-recharger (`oütf` 186) or not by `carries`, landed at Port Kane:
/// its tank and its cash after.
fn landed_at_port_kane(data: &GameData, carries: bool) -> (f32, f32, i64) {
    let mut pilot = Pilot::new(data, "Stock").expect("the stock first chär starts");
    if carries {
        pilot.outfits.insert(OutfitId(186), 1);
    }
    pilot.reserves.fuel.now = 10.0;
    pilot.cash = 10_000;
    let system = pilot.system;
    let mut session = Session::fly(data, pilot).expect("flies");
    session
        .land_at(data, system, StellarId(137))
        .expect("Port Kane is landable");
    let fuel = session.reserves().fuel;
    (fuel.now, fuel.max, session.pilot().cash())
}

#[test]
fn landing_at_port_kane_with_the_auto_recharger_tops_up_the_tank() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let (now, max, cash) = landed_at_port_kane(&data, true);
    assert!(max > 10.0, "{max}");
    assert_eq!(now.to_bits(), max.to_bits(), "full");
    #[allow(clippy::cast_possible_truncation)]
    let units = (max - 10.0) as i64;
    assert_eq!(cash, 10_000 - units);
    let (now, _, cash) = landed_at_port_kane(&data, false);
    assert_eq!(now.to_bits(), 10.0_f32.to_bits(), "no outfit, no top-up");
    assert_eq!(cash, 10_000);
}
