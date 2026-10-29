//! Tests of the outfits Use As My Ship leaves the player
//! (`Session::take_over_captured`): the captured ship's **live** weapons
//! and rounds, merged as `_DoShipCapture` does (@0x41849-0x41a3e; see the
//! `captured_loadout` module of `nova_sim`), not its class's stock topped
//! up. The rounds it fired are gone, a person's extra weapons come over
//! and a carrier's fighters aboard are its bay's rounds.

use std::collections::BTreeMap;

use super::Session;
use super::capture_naming::tests::use_as_my_ship;
use super::tests::{captured, kitted};
use crate::board::{Assigned, FIGHTER_BAY};
use crate::catalog::StockWeapon;
use crate::catalog::{OutfitId, ShipId, WeaponId, WeaponRecord};
use crate::combat::armament::{Armament, MOD_AMMO, MOD_WEAPON};
use crate::combat::weapon::WeaponSpec;
use crate::testkit::{Draws, FakePilotCatalog};
use nova_sim_catalog::fixture::{outfit, weapon};

/// The outfits the player owns, as pairs of raw IDs and counts.
fn owned(session: &Session) -> Vec<(i16, u16)> {
    session
        .pilot()
        .outfits()
        .map(|(id, count)| (id.0, count))
        .collect()
}

/// Use As My Ship on `session`, named "Prize".
fn take_over(session: &mut Session) {
    assert_eq!(
        use_as_my_ship(session, "Prize", &mut Draws::of(&[0])),
        Some(Assigned::MyShip)
    );
}

/// [`kitted`] with the trader (129) stocking `weapons`.
fn stocking(mut catalog: FakePilotCatalog, weapons: &[StockWeapon]) -> FakePilotCatalog {
    for hull in &mut catalog.hulls {
        if hull.id == ShipId(129) {
            hull.weapons = weapons.to_vec();
        }
    }
    catalog
}

#[test]
fn a_captured_ship_hands_over_the_rounds_it_has_left() {
    // The trader stocks a homing missile (134, held by 252) firing its
    // own rounds (`AmmoType` 6, held by 403), loaded 6, and has fired 2.
    let mut catalog = stocking(
        kitted(),
        &[StockWeapon {
            weapon: WeaponId(134),
            count: 1,
            ammo: 6,
        }],
    );
    for record in &mut catalog.weapons {
        if record.id == WeaponId(134) {
            record.ammo_type = 6;
        }
    }
    catalog.outfits.push(outfit(403, &[(MOD_AMMO, 134)]));
    let mut session = captured(&catalog);
    let trader = &mut session.space.traffic.npcs_mut()[0];
    assert_eq!(trader.ship, ShipId(129));
    assert_eq!(trader.rounds.slot(WeaponId(134)), 6, "its stock load");
    trader.rounds.set(WeaponId(134), 4);
    session.pilot.outfits = BTreeMap::from([(OutfitId(403), 1)]);
    take_over(&mut session);
    assert_eq!(
        owned(&session),
        [(252, 1), (402, 1), (403, 4)],
        "the 4 it has left, the player's own round zeroed, no stock top-up to 6"
    );
}

#[test]
fn a_captured_persons_ship_hands_over_its_persons_weapons() {
    // The unarmed trader carries three point defences (133, held by 251)
    // its class does not stock, as a person's fitted ship does.
    let catalog = kitted();
    let mut session = captured(&catalog);
    let defence = *session
        .statics
        .arsenal
        .weapon(WeaponId(133))
        .expect("point defence");
    session.space.traffic.npcs_mut()[0].armament = Armament::new([(defence, 3)]);
    take_over(&mut session);
    assert_eq!(owned(&session), [(251, 3), (402, 1)]);
}

#[test]
fn a_captured_carrier_hands_over_the_fighters_aboard() {
    // A bay of ship 144 (150, held by 254), its fighters held by 311,
    // stocked with 4 and 3 aboard.
    let mut catalog = stocking(
        kitted(),
        &[StockWeapon {
            weapon: WeaponId(150),
            count: 1,
            ammo: 4,
        }],
    );
    catalog.weapons.push(WeaponRecord {
        guidance: FIGHTER_BAY,
        ammo_type: 144,
        ..weapon(150)
    });
    catalog.outfits.push(outfit(254, &[(MOD_WEAPON, 150)]));
    catalog.outfits.push(outfit(311, &[(MOD_AMMO, 150)]));
    let mut session = captured(&catalog);
    let bay = session
        .statics
        .arsenal
        .weapon(WeaponId(150))
        .copied()
        .map(|spec: WeaponSpec| spec.guidance_99());
    assert_eq!(bay, Some(true));
    session.space.traffic.npcs_mut()[0]
        .rounds
        .set(WeaponId(150), 3);
    take_over(&mut session);
    assert_eq!(owned(&session), [(254, 1), (311, 3), (402, 1)]);
}
