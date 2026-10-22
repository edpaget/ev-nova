//! The stock ship types carrying each AI-cloak bit of `Flags2` that
//! `_AIHandleCloaking` tests (rdm cloaking-fidelity phase 3, see
//! `nova_sim::cloak`), and the reload term on a Wraith's weapon. Skips,
//! passing, when `NOVA_DATA` is unset.
//!
//! - Ship types carrying each bit, all and with a cloak: 0x0100 31/28,
//!   0x0200 45/41, 0x0400 3/2, 0x0800 2/2, 0x1000 35/32, 0x2000 0/0.
//! - A Wraith Adult (168) at the end of a burst of its weapon 165
//!   (`BurstCount` 1, `BurstReload` 45 past its `Reload`) reloads past
//!   `Reload` by the engine's reading, so its 0x0100 term holds; the
//!   pirate Raven's Pulse Lasers (146, `Reload`, `BurstCount` and
//!   `BurstReload` all 15) don't, but do by the Bible's. A burst's end
//!   puts the timer at `BurstReload`, as a cloak-when-hit's
//!   `Armament::force_burst_reload` does, so that stands in for a shot.
//! - No stock outfit has an area cloak (`ModVal` 0x1000 of a cloak mod,
//!   `_HasAreaCloak`), so no stock ship type carries one.

use crate::common;

use nova_data::GameData;
use nova_sim::catalog::WeaponId;
use nova_sim::cloak::{
    CLOAK_IDLE, CLOAK_JUMPING, CLOAK_PRESSING, CLOAK_RELOADING, CLOAK_RETREATING, CLOAK_TRAVELLING,
    CloakGear,
};
use nova_sim::combat::armament::Arsenal;
use nova_sim::rulebook::RuleSource;
use nova_sim::{PilotCatalog, ShipId};

/// The Wraith Adult.
const WRAITH: ShipId = ShipId(168);
/// The pirate Raven with a cloak.
const RAVEN: ShipId = ShipId(263);

fn opened() -> Option<GameData> {
    let dir = common::nova_data()?;
    Some(GameData::open(&dir, None).expect("the stock data opens"))
}

/// The cloak gear of every stock ship type.
fn gears(data: &GameData) -> Vec<(ShipId, CloakGear)> {
    let outfits = data.outfits();
    data.ships()
        .into_iter()
        .map(|ship| {
            let items = ship.defaults.iter().copied().collect();
            (ship.id, CloakGear::of(ship.fields.flags2, &items, &outfits))
        })
        .collect()
}

#[test]
fn the_stock_ship_types_carrying_each_ai_cloak_bit() {
    let Some(data) = opened() else {
        return;
    };
    let gears = gears(&data);
    let counts = |bit: u16| {
        let carrying: Vec<&CloakGear> = gears
            .iter()
            .filter(|(_, gear)| gear.flags2 & bit != 0)
            .map(|(_, gear)| gear)
            .collect();
        let cloaked = carrying.iter().filter(|gear| gear.device).count();
        (carrying.len(), cloaked)
    };
    assert_eq!(counts(CLOAK_RELOADING), (31, 28), "0x0100");
    assert_eq!(counts(CLOAK_RETREATING), (45, 41), "0x0200");
    assert_eq!(counts(CLOAK_JUMPING), (3, 2), "0x0400");
    assert_eq!(counts(CLOAK_TRAVELLING), (2, 2), "0x0800");
    assert_eq!(counts(CLOAK_PRESSING), (35, 32), "0x1000");
    assert_eq!(counts(CLOAK_IDLE), (0, 0), "0x2000");
    let jumping: Vec<ShipId> = gears
        .iter()
        .filter(|(_, gear)| gear.flags2 & CLOAK_JUMPING != 0)
        .map(|&(id, _)| id)
        .collect();
    assert_eq!(jumping, [ShipId(168), ShipId(169), ShipId(185)]);
}

#[test]
fn a_wraith_at_the_end_of_a_burst_reloads_past_reload_and_a_ravens_pulse_laser_does_not() {
    let Some(data) = opened() else {
        return;
    };
    let gears = gears(&data);
    let wraith = gears.iter().find(|(id, _)| *id == WRAITH).expect("168").1;
    assert!(wraith.device);
    assert_ne!(wraith.flags2 & CLOAK_RELOADING, 0);
    let arsenal = Arsenal::read(&data);
    let (mut armament, _) = arsenal.npc(WRAITH);
    assert!(
        armament
            .mounts()
            .iter()
            .any(|mount| mount.spec.id == WeaponId(165)),
        "its weapon 165"
    );
    assert!(!armament.reloading(RuleSource::Engine), "ready");
    armament.force_burst_reload(&arsenal.of_class(WRAITH));
    assert!(armament.reloading(RuleSource::Engine));
    assert!(armament.reloading(RuleSource::Bible));

    let (mut raven, _) = arsenal.npc(RAVEN);
    raven.force_burst_reload(&arsenal.of_class(RAVEN));
    assert!(!raven.reloading(RuleSource::Engine));
    assert!(raven.reloading(RuleSource::Bible));
}

#[test]
fn no_stock_outfit_has_an_area_cloak() {
    // `_HasAreaCloak` (@0x3d49-0x3e24) reads `ModVal` 0x1000 of a
    // `ModType` 17 mod: the stock cloaks are 211 (0xe), 234 (0x6), 266
    // (0x2), 268 (0x80b), 269 (0x409) and 347 (0x6), none with it.
    use nova_sim::catalog::OutfitId;
    use nova_sim::cloak::{AREA_CLOAK, MOD_CLOAK};
    let Some(data) = opened() else {
        return;
    };
    let cloaks: Vec<(OutfitId, i16)> = data
        .outfits()
        .iter()
        .flat_map(|record| {
            record
                .mods
                .iter()
                .filter(|&&(mod_type, _)| mod_type == MOD_CLOAK)
                .map(move |&(_, mod_val)| (record.id, mod_val))
        })
        .collect();
    assert_eq!(
        cloaks,
        [
            (OutfitId(211), 0xe),
            (OutfitId(234), 0x6),
            (OutfitId(266), 0x2),
            (OutfitId(268), 0x80b),
            (OutfitId(269), 0x409),
            (OutfitId(347), 0x6),
        ]
    );
    assert!(cloaks.iter().all(|&(_, mod_val)| mod_val & AREA_CLOAK == 0));
    assert!(
        gears(&data).iter().all(|(_, gear)| !gear.area),
        "no ship type"
    );
}
