//! A bought stock Starbridge's held stock (rdm play-feel-9 phase 16, see
//! the session's `held_readers` module). Skips, passing, when `NOVA_DATA`
//! is unset.
//!
//! The Starbridge (`shïp` 133) stocks two Medium Blasters (`oütf` 129),
//! whose second mod is a `ModType` 8 speed penalty. The purchase tops the
//! weapon array up with them, and the outfit counts the stats read hold
//! them only from the next fold, here a landing: until then the new ship
//! flies without the penalty.

use crate::common;

use nova_sim::handling::SPEED_PER_PIXEL_PER_TICK;
use nova_sim::stats::MORE_SPEED;
use nova_sim::{Chance, OutfitId, Pilot, PilotCatalog, Session, ShipId, StellarId, SystemId};

/// The Starbridge.
const STARBRIDGE: ShipId = ShipId(133);
/// The Medium Blaster, the Starbridge's stock gun.
const MEDIUM_BLASTER: OutfitId = OutfitId(129);
/// The Exotic Ships & Weapons License, which contributes the bits the
/// Starbridge's `Require` names (0xb00000000).
const LICENSE: OutfitId = OutfitId(265);
/// Earth (`spöb` 128) in Sol (`sÿst` 130), a shipyard of tech level 7.
const EARTH: (SystemId, StellarId) = (SystemId(130), StellarId(128));

/// Every chance fires: the shipyard lists each class it can.
struct Fires;

impl Chance for Fires {
    fn fires(&mut self, _percent: u8) -> bool {
        true
    }

    fn below(&mut self, _n: u32) -> u32 {
        0
    }

    fn roll(&mut self, _sides: u16) -> u16 {
        0
    }
}

#[test]
fn a_bought_stock_starbridge_flies_without_its_blasters_penalty_until_its_first_fold() {
    let Some(data) = common::opened() else {
        return;
    };
    let mut pilot = Pilot::new(&data, "Stock").expect("the stock first chär starts");
    pilot.cash = 1_000_000;
    pilot.outfits.insert(LICENSE, 1);
    let mut session = Session::fly(&data, pilot).expect("flies");
    let (system, earth) = EARTH;
    session
        .land_at(&data, system, earth)
        .expect("Earth is landable");
    session
        .buy_ship(STARBRIDGE, "Kestrel", &mut Fires)
        .expect("Earth sells the Starbridge");
    let blasters = session.pilot().owned(MEDIUM_BLASTER);
    assert_eq!(blasters, 2, "its stock guns");
    let penalty = data
        .outfits()
        .into_iter()
        .find(|record| record.id == MEDIUM_BLASTER)
        .and_then(|record| {
            record.mods[1..]
                .iter()
                .find(|&&(kind, _)| kind == MORE_SPEED)
                .map(|&(_, value)| value)
        })
        .expect("a speed mod past its first");
    assert!(penalty < 0, "{penalty}");
    let held = session.stats().handling.max_speed;
    session.land_at(&data, system, earth).expect("lands again");
    let folded = session.stats().handling.max_speed;
    let slowed = -f32::from(penalty) * f32::from(blasters) / SPEED_PER_PIXEL_PER_TICK;
    assert!(
        (held - folded - slowed).abs() < 1e-5,
        "held {held}, folded {folded}: two blasters' {penalty} each"
    );
}
