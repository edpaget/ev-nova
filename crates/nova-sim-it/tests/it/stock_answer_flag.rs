//! What `_DamageShip`'s answer flag reads of the stock data (rdm
//! weapons-fidelity phase 26, see `nova_sim::ai::answer_flag`): the
//! `ShootPenalty` that decides whether the player's stray hit may be
//! forgiven by the roll, and the `Reload` a strike weighs, which the
//! engine reads clamped at none. Skips, passing, when `NOVA_DATA` is
//! unset.

use crate::common;

use nova_data::GameData;
use nova_sim::{CombatCatalog, GovtId};

/// Stock `gövt` `id`'s `ShootPenalty`.
fn shoot_penalty(data: &GameData, id: i16) -> i16 {
    data.governments()
        .into_iter()
        .find(|govt| govt.id == GovtId(id))
        .expect("the government reads")
        .penalties
        .shoot
}

#[test]
fn the_federation_never_forgives_a_stray_hit_by_the_roll_and_the_aurorans_may() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    // Above 0: no roll (@0x3b821); at or below 0: forgiven 3 times in 4.
    assert_eq!(shoot_penalty(&data, 128), 5, "the Federation");
    assert_eq!(shoot_penalty(&data, 129), 0, "the Aurorans");
}

#[test]
fn no_stock_weapon_has_a_negative_reload() {
    // A strike's weight is the weapon's `Reload` as the engine reads it,
    // clamped at none, where `_LoadObjectData` keeps the raw value
    // (@0x792cf-0x792d6); stock has no negative one to tell them apart.
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let weapons = data.weapons();
    assert!(!weapons.is_empty());
    let negative: Vec<_> = weapons
        .iter()
        .filter(|weapon| weapon.reload < 0)
        .map(|weapon| (weapon.id, weapon.reload))
        .collect();
    assert_eq!(negative, []);
}
