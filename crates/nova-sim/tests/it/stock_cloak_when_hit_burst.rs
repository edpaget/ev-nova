//! The stock ship types that cloak when hit (`shïp` `Flags2` 0x4000) and
//! the burst weapons `_AIForceBurstReload` starts on their burst reload as
//! they do (rdm combat-ai-fidelity phase 8, see
//! `nova_sim::combat::armament::Armament::force_burst_reload`). Skips,
//! passing, when `NOVA_DATA` is unset.
//!
//! - 45 stock ship types cloak when hit; 25 of them carry a burst weapon
//!   (`BurstCount` and `BurstReload` both above 0).
//! - A pirate Raven with a cloak (263) carries a cloak and two 146 Pulse
//!   Lasers (burst 15, reload 15): hit, both start on their burst reload.

use crate::common;

use nova_data::GameData;
use nova_sim::catalog::WeaponId;
use nova_sim::cloak::{CLOAK_WHEN_HIT, CloakGear};
use nova_sim::combat::armament::Arsenal;
use nova_sim::{PilotCatalog, ShipId};

/// The pirate Raven with a cloak.
const RAVEN: ShipId = ShipId(263);
/// Its Pulse Laser.
const PULSE_LASER: WeaponId = WeaponId(146);

fn opened() -> Option<GameData> {
    let dir = common::nova_data()?;
    Some(GameData::open(&dir, None).expect("the stock data opens"))
}

#[test]
fn the_stock_ship_types_that_cloak_when_hit_and_carry_a_burst_weapon() {
    let Some(data) = opened() else {
        return;
    };
    let arsenal = Arsenal::read(&data);
    let cloaking: Vec<ShipId> = data
        .ships()
        .iter()
        .filter(|ship| ship.fields.flags2 & CLOAK_WHEN_HIT != 0)
        .map(|ship| ship.id)
        .collect();
    assert_eq!(cloaking.len(), 45);
    let bursting = cloaking
        .iter()
        .filter(|&&ship| {
            arsenal
                .of_class(ship)
                .mounts()
                .iter()
                .any(|mount| mount.spec.burst_count > 0 && mount.spec.burst_reload > 0.0)
        })
        .count();
    assert_eq!(bursting, 25);
}

#[test]
fn a_cloaking_raven_hit_starts_its_pulse_lasers_on_their_burst_reload() {
    let Some(data) = opened() else {
        return;
    };
    let raven = data
        .ships()
        .into_iter()
        .find(|ship| ship.id == RAVEN)
        .expect("the Raven");
    let items = raven.defaults.iter().copied().collect();
    let gear = CloakGear::of(raven.fields.flags2, &items, &data.outfits());
    assert_ne!(gear.flags2 & CLOAK_WHEN_HIT, 0);
    assert!(gear.device, "a cloak among its default items");
    let arsenal = Arsenal::read(&data);
    let (mut armament, _) = arsenal.npc(RAVEN);
    armament.force_burst_reload(&arsenal.of_class(RAVEN));
    let laser = armament
        .mounts()
        .iter()
        .find(|mount| mount.spec.id == PULSE_LASER)
        .expect("its Pulse Lasers");
    assert_eq!(laser.count, 2);
    assert_eq!((laser.burst, laser.reload), (0, 15.0));
}
