//! Spawn availability over the stock data: accepting mïsn 754
//! (Tutorial 006) at its stellar sets b9208 after the landing's galaxy
//! refresh, yet përs 642 (the Pirate Viper, `ActiveOn b9208`), in
//! Rautherion's person slots, is among the ships at the take-off right
//! after, as a person slot tests its `ActiveOn` now
//! (`_SetupShipsInSystem` @0x4334d), past the refresh's cache. Skips,
//! passing, when `NOVA_DATA` is unset.

use crate::common;

use nova_data::GameData;
use nova_sim::fixture::bits_allowing;
use nova_sim::mission::Venue;
use nova_sim::{Bit, MissionId, NeverFires, Pilot, PilotCatalog, Session, StellarId, SystemId};

/// Tutorial 006.
const MISSION: MissionId = MissionId(754);
/// Its `AvailStel`.
const STELLAR: StellarId = StellarId(191);
/// Rautherion, which lists it.
const RAUTHERION: SystemId = SystemId(166);
/// The Pirate Viper, `ActiveOn b9208`.
const PIRATE_VIPER: i16 = 642;

/// A new stock pilot docked at 191 in Rautherion with the bits mïsn
/// 754's `AvailBits` asks, settled with every roll 1.
fn docked_for_754(data: &GameData) -> Session {
    let record = data
        .missions()
        .into_iter()
        .find(|record| record.id == MISSION)
        .expect("mïsn 754");
    let mut pilot = Pilot::new(data, "Stock").expect("the stock first chär starts");
    for (bit, on) in bits_allowing(&record.avail_bits).expect("bits alone open it") {
        if on {
            pilot.bits_mut().set(bit);
        } else {
            pilot.bits_mut().clear(bit);
        }
    }
    let mut save: serde_json::Value =
        serde_json::from_str(&nova_sim::save::encode(&pilot)).expect("JSON");
    save["system"] = serde_json::json!(RAUTHERION.0);
    save["stellar"] = serde_json::json!(STELLAR.0);
    let saved = nova_sim::save::decode(&save.to_string()).expect("a pilot");
    let mut session = Session::fly(data, saved).expect("flies");
    session.land_at(data, RAUTHERION, STELLAR).expect("lands");
    let _ = session.settle_script(data, &mut NeverFires);
    session
}

#[test]
fn the_pirate_viper_of_tutorial_006_appears_at_the_take_off_after_its_accept() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let mut session = docked_for_754(&data);
    let b9208 = Bit::new(9208).expect("a bit");
    assert!(
        !session.control_bit(b9208),
        "clear at the landing's refresh"
    );
    let offered = [Venue::Computer, Venue::Outside].into_iter().any(|venue| {
        session
            .mission_offers(venue)
            .iter()
            .any(|offer| offer.mission == MISSION)
    });
    assert!(offered, "754 is offered at 191");
    session
        .accept_mission(MISSION, &mut NeverFires)
        .expect("accepted");
    let _ = session.settle_script(&data, &mut NeverFires);
    assert!(session.control_bit(b9208), "OnAccept set it");
    session.take_off(&data, &mut NeverFires).expect("takes off");
    let persons: Vec<i16> = session
        .npcs()
        .iter()
        .filter_map(|npc| npc.person.as_ref().map(|person| person.id.0))
        .collect();
    assert!(persons.contains(&PIRATE_VIPER), "{persons:?}");
}
