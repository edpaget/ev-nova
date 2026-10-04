//! A flight session over the stock data: the first `chär` starts a session
//! with its ship's handling and reserves in a system that exists. Skips,
//! passing, when `NOVA_DATA` is unset.

mod common;

use nova_data::GameData;
use nova_data::records::character::Character;
use nova_data::records::ship::Ship;
use nova_data::records::stellar::Stellar;
use nova_sim::{
    Handling, LandingRefusal, PilotCatalog, Reserves, Service, Session, ShipFields, ShipState,
    check_landing, services,
};

#[test]
fn the_first_chär_starts_a_session_in_one_of_its_systems() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let session = Session::start(&data).expect("the stock first chär starts");
    let (_, first) = data.records::<Character>().next().expect("a chär");
    let character = first.expect("decodes").record;
    assert_eq!(Some(session.ship()), character.ship_type);
    assert!(character.system.contains(&Some(session.system())));
    let ship = data
        .get::<Ship>(session.ship().0)
        .expect("present")
        .expect("decodes")
        .record;
    let fields = ShipFields {
        speed: ship.speed,
        accel: ship.accel,
        maneuver: ship.maneuver,
        shield: ship.shield,
        armor: ship.armor,
        fuel: ship.fuel,
        fuel_regen: ship.fuel_regen,
    };
    assert_eq!(session.handling(), Handling::from_fields(fields));
    assert_eq!(session.player().reserves, Reserves::from_fields(fields));
    assert!(
        session.player().reserves.shield.max > 0.0,
        "{:?}",
        session.player().reserves
    );
    assert!(
        session.handling().max_speed > 0.0,
        "{:?}",
        session.handling()
    );
}

/// The start system's stellars land as the stock data says: a new pilot
/// parked over HG-Kania, a station that needs a legal record of 32767,
/// is denied, and Port Kane (`spöb` 137) offers its trade center,
/// outfitter, bar and mission BBS, but no shipyard.
#[test]
fn stock_landing_sites_follow_their_flags_and_min_status() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let session = Session::start(&data).expect("the stock first chär starts");
    let sites = data.landing_sites(session.system());
    let kania = sites
        .iter()
        .find(|site| {
            data.get::<Stellar>(site.id.0)
                .and_then(Result::ok)
                .and_then(|entry| entry.name)
                == Some("HG-Kania")
        })
        .expect("HG-Kania is in the start system");
    assert_eq!(kania.min_status, 32767);
    let parked = ShipState {
        position: kania.position,
        ..ShipState::default()
    };
    assert_eq!(
        check_landing(&parked, &sites, session.legal_record()),
        Err(LandingRefusal::Denied {
            stellar: kania.id,
            station: true,
            min_status: 32767,
        })
    );

    let port_kane = data.get::<Stellar>(137).expect("present").expect("decodes");
    assert_eq!(port_kane.name, Some("Port Kane"));
    assert_eq!(
        services(port_kane.record.flags.bits()),
        [
            Service::TradeCenter,
            Service::Outfitter,
            Service::Bar,
            Service::MissionBbs,
        ]
    );
}
