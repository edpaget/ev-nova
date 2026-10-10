//! Mission offers over the stock data: a new pilot docked at the first
//! landable stellar of its starting system, its offers rolled as the
//! session settles with every roll 1, is offered missions at the mission
//! computer, each of `AvailLoc` 0 and passing every availability field.
//! Skips, passing, when `NOVA_DATA` is unset.

mod common;

use nova_data::GameData;
use nova_sim::mission::Venue;
use nova_sim::{NeverFires, Pilot, PilotCatalog, Session, is_landable};

/// A new stock pilot docked at the first landable stellar of its system,
/// through a save that says so, flown and settled with every roll 1.
fn docked_new_pilot(data: &GameData) -> Session {
    let pilot = Pilot::new(data, "Stock").expect("the stock first chär starts");
    let site = data
        .landing_sites(pilot.system())
        .into_iter()
        .find(is_landable)
        .expect("a landable stellar where the pilot starts");
    let mut save: serde_json::Value =
        serde_json::from_str(&nova_sim::save::encode(&pilot)).expect("JSON");
    save["stellar"] = serde_json::json!(site.id.0);
    let docked = nova_sim::save::decode(&save.to_string()).expect("a pilot");
    let mut session = Session::fly(data, docked).expect("flies");
    assert_eq!(session.landed(), Some(site.id), "docked");
    let _ = session.settle_script(data, &mut NeverFires);
    session
}

#[test]
fn a_new_stock_pilot_is_offered_missions_that_pass_every_field() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let session = docked_new_pilot(&data);
    let offers = session.mission_offers(Venue::Computer);
    assert!(!offers.is_empty(), "the mission computer offers something");
    let missions = data.missions();
    for offer in offers {
        let record = missions
            .iter()
            .find(|record| record.id == offer.mission)
            .expect("a mïsn");
        assert_eq!(record.avail_loc, 0, "{:?} at the computer", offer.mission);
        assert_eq!(
            session.offer_refusals(offer.mission),
            Some(Vec::new()),
            "{:?} passes every field",
            offer.mission
        );
    }
}
