//! Tests of [`WeaponRecord::muzzle_reach`](crate::catalog::WeaponRecord::muzzle_reach)
//! over the game data: [`GameData::weapons`](crate::catalog::CombatCatalog::weapons)
//! serves each weapon the store's muzzle reach for it
//! (`nova_data::GameData::muzzle_reach`), which only a new game's store
//! holds.

use std::collections::BTreeMap;

use nova_data::records::weapon::Weapon;
use nova_data::{GameData, Record};

use crate::catalog::{CombatCatalog, WeaponId};
use crate::test_store::store;

/// Two `wëap`s, 128 and 129, of zeros.
fn two_weapons() -> GameData {
    let zeros = || vec![0; Weapon::SIZE.expect("fixed")];
    store(&[(Weapon::TYPE, 128, zeros()), (Weapon::TYPE, 129, zeros())])
}

fn reaches(data: &GameData) -> Vec<(WeaponId, Option<i16>)> {
    data.weapons()
        .iter()
        .map(|weapon| (weapon.id, weapon.muzzle_reach))
        .collect()
}

#[test]
fn a_weapon_carries_the_store_s_muzzle_reach_for_it() {
    let data = two_weapons().with_muzzle_reaches(BTreeMap::from([(WeaponId(129), 3)]));
    assert_eq!(
        reaches(&data),
        [(WeaponId(128), None), (WeaponId(129), Some(3))],
        "the unlisted weapon has none"
    );
}

#[test]
fn a_stock_store_s_weapons_have_no_muzzle_reach() {
    assert_eq!(
        reaches(&two_weapons()),
        [(WeaponId(128), None), (WeaponId(129), None)]
    );
}

#[test]
fn the_reach_leaves_the_rest_of_the_weapon_alone() {
    let stock = two_weapons().weapons();
    let mut reaching = two_weapons()
        .with_muzzle_reaches(BTreeMap::from([(WeaponId(128), 3)]))
        .weapons();
    reaching[0].muzzle_reach = None;
    assert_eq!(reaching, stock);
}
