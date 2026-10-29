//! The stellar defence guns over the stock data (stellar-fidelity phase
//! 13, `_HandleStellarWeapons` @0x2ef3a): 95 stock `spöb` carry a gun, and
//! Spacedock I in Gateway fires at a criminal near it, while Earth, which
//! waits for its defence ships (`Flags2` 0x0200), never fires. Skips,
//! passing, when `NOVA_DATA` is unset.

use std::collections::BTreeMap;

use crate::common;

use nova_data::GameData;
use nova_sim::combat::stellar_guns::GUARDED_BY_DEFENDERS;
use nova_sim::pilot::Pilot;
use nova_sim::{CombatEvent, PilotCatalog, Session, ShipRef, StellarId, SystemId, WeaponId};

/// A new stock pilot flying in `system` from `stellar`, its record there
/// `record`: at rest on the stellar's centre, as a load puts it
/// (`_LoadPilotData` @0x75329-0x75354), well within its gun's range.
fn beside(data: &GameData, system: i16, stellar: i16, record: i16) -> Session {
    let pilot = Pilot::new(data, "Stock").expect("the stock first chär starts");
    let mut save: serde_json::Value =
        serde_json::from_str(&nova_sim::save::encode(&pilot)).expect("JSON");
    save["stellar"] = serde_json::json!(stellar);
    save["system"] = serde_json::json!(system);
    save["system_legal"] = serde_json::json!([{"system": system, "record": record}]);
    let there = nova_sim::save::decode(&save.to_string()).expect("a pilot");
    let session = Session::fly(data, there).expect("flies");
    let site = data
        .landing_sites(SystemId(system))
        .into_iter()
        .find(|site| site.id == StellarId(stellar))
        .expect("the system lists it");
    assert_eq!(session.player().position, site.position, "on the stellar");
    session
}

/// The stellars that fire on a pass of `data`'s session, and the shots
/// then in flight, each with the ship it is aimed at.
fn a_tick(
    session: &mut Session,
    data: &GameData,
) -> (Vec<StellarId>, Vec<(WeaponId, Option<ShipRef>)>) {
    crate::passes::work(session, data, nova_sim::Controls::default());
    let fired = session
        .take_combat_events()
        .into_iter()
        .filter_map(|event| match event {
            CombatEvent::StellarFired { stellar, .. } => Some(stellar),
            _ => None,
        })
        .collect();
    let shots = session
        .shots()
        .iter()
        .map(|shot| (shot.weapon.id, shot.aimed_at))
        .collect();
    (fired, shots)
}

#[test]
fn ninety_five_stock_stellars_carry_a_gun() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let guns = data.stellar_weapons();
    assert_eq!(guns.len(), 95);
    let mut by_weapon: BTreeMap<i16, usize> = BTreeMap::new();
    for (_, weapon) in &guns {
        *by_weapon.entry(weapon.0).or_default() += 1;
    }
    assert_eq!(
        by_weapon,
        BTreeMap::from([
            (131, 1),
            (159, 22),
            (196, 15),
            (197, 27),
            (198, 9),
            (199, 21)
        ])
    );
    assert!(
        guns.contains(&(StellarId(184), WeaponId(196))),
        "Spacedock I"
    );
    let guarded: Vec<StellarId> = data
        .stellar_lives()
        .into_iter()
        .filter(|life| life.flags2 & GUARDED_BY_DEFENDERS != 0)
        .map(|life| life.id)
        .filter(|id| guns.iter().any(|&(stellar, _)| stellar == *id))
        .collect();
    assert_eq!(
        guarded,
        [
            StellarId(128),
            StellarId(137),
            StellarId(161),
            StellarId(508)
        ],
        "Earth, Port Kane, Pyrogenesis I and New Ireland"
    );
}

#[test]
fn spacedock_i_fires_at_a_criminal_beside_it_and_holds_fire_at_a_clean_record() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let mut criminal = beside(&data, 159, 184, -7);
    assert_eq!(
        a_tick(&mut criminal, &data),
        (
            vec![StellarId(184)],
            vec![(WeaponId(196), Some(ShipRef::Player))]
        )
    );
    let mut clean = beside(&data, 159, 184, -6);
    assert_eq!(a_tick(&mut clean, &data), (vec![], vec![]), "at -CrimeTol");
}

#[test]
fn earth_waits_for_its_defence_ships_and_never_fires() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    assert!(
        data.stellar_weapons()
            .iter()
            .any(|&(stellar, _)| stellar == StellarId(128)),
        "Earth is armed"
    );
    let sol = data
        .star_map()
        .into_iter()
        .find(|system| {
            data.landing_sites(system.id)
                .iter()
                .any(|site| site.id == StellarId(128))
        })
        .expect("a system lists Earth")
        .id;
    let mut wanted = beside(&data, sol.0, 128, -1000);
    assert_eq!(a_tick(&mut wanted, &data).0, []);
}
