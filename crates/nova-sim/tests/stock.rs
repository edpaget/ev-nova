//! A flight session over the stock data: the first `chär` starts a session
//! with its ship's handling and reserves in a system that exists, and
//! Port Kane's exchange trades at its levels. Skips, passing, when
//! `NOVA_DATA` is unset.

mod common;

use nova_data::GameData;
use nova_data::records::character::Character;
use nova_data::records::ship::Ship;
use nova_data::records::stellar::Stellar;
use nova_sim::fuel::FUEL_SCOOP;
use nova_sim::{
    DisasterId, DisasterRecord, GameDate, Good, Handling, JunkId, LandingRefusal, OutfitMod, Pilot,
    PilotCatalog, Reserves, Service, Session, ShipFields, ShipId, ShipState, StartDate, StellarId,
    check_landing, fuel_regen_per_tick, services,
};

/// A new pilot starts with the first `chär`'s ship, cash, location (its
/// first starting system, which exists) and date, 23 June 1177.
#[test]
fn a_new_pilot_starts_as_the_first_chär_says() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let pilot = Pilot::new(&data, "Stock").expect("the stock first chär starts");
    let (_, first) = data.records::<Character>().next().expect("a chär");
    let character = first.expect("decodes").record;
    assert_eq!(Some(pilot.ship()), character.ship_type);
    assert_eq!(pilot.cash(), i64::from(character.cash));
    assert_eq!(Some(pilot.system()), character.system[0]);
    assert_eq!(
        pilot.date(),
        GameDate::from_start(StartDate {
            day: character.unknown_0x134,
            month: character.unknown_0x136,
            year: character.start_year,
        })
    );
    assert_eq!(
        (
            pilot.date().day(),
            pilot.date().month(),
            pilot.date().year()
        ),
        (23, 6, 1177)
    );
    assert_eq!(pilot.stellar(), None);
    assert_eq!(pilot.explored().collect::<Vec<_>>(), [pilot.system()]);
}

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
        holds: ship.holds,
    };
    assert_eq!(session.handling(), Handling::from_fields(fields));
    assert_eq!(session.reserves(), Reserves::from_fields(fields));
    assert!(
        session.reserves().shield.max > 0.0,
        "{:?}",
        session.reserves()
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
/// outfitter, bar and mission BBS, but no shipyard, and lands to
/// "Federation Station.SFIL" (`snd ` 10032).
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
    let port_kane_site = sites
        .iter()
        .find(|site| site.id.0 == 137)
        .expect("Port Kane is in the start system");
    assert_eq!(
        port_kane_site.landing_sound,
        Some(nova_sim::SoundId(10_032)),
        "Federation Station.SFIL"
    );
    assert_eq!(kania.landing_sound, None, "a hypergate's angle, 120");
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

/// The stock starting Shuttle carries no default outfits and regenerates
/// no fuel; the Scarab's Matter/Antimatter Reactor, a fuel scoop of 4,
/// adds a unit every 4 ticks to its own every 10.
#[test]
fn stock_default_outfits_give_their_fuel_regeneration() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let session = Session::start(&data).expect("the stock first chär starts");
    let shuttle = data
        .get::<Ship>(session.ship().0)
        .expect("present")
        .expect("decodes");
    assert_eq!(shuttle.name, Some("Shuttle"));
    assert_eq!(data.default_outfits(session.ship()), []);
    assert!(
        session.fuel_regen_per_tick().abs() < f32::EPSILON,
        "{}",
        session.fuel_regen_per_tick()
    );

    let scarab = data.get::<Ship>(162).expect("present").expect("decodes");
    assert_eq!(scarab.name, Some("Scarab"));
    let reactor = OutfitMod {
        mod_type: FUEL_SCOOP,
        mod_val: 4,
        count: 1,
    };
    let outfits = data.default_outfits(ShipId(162));
    assert!(outfits.contains(&reactor), "{outfits:?}");
    let regen = fuel_regen_per_tick(scarab.record.fuel_regen, &outfits);
    assert!((regen - (0.1 + 0.25)).abs() < 1e-6, "{regen}");
}

/// A new stock pilot, docked at Port Kane (`spöb` 137 in Kania, where it
/// starts), through a save that says so.
fn at_port_kane(data: &GameData) -> Session {
    let pilot = Pilot::new(data, "Stock").expect("the stock first chär starts");
    let mut save: serde_json::Value =
        serde_json::from_str(&nova_sim::save::encode(&pilot)).expect("JSON");
    save["stellar"] = serde_json::json!(137);
    let docked = nova_sim::save::decode(&save.to_string()).expect("a pilot");
    let session = Session::fly(data, docked).expect("flies");
    assert_eq!(
        session.landed(),
        Some(StellarId(137)),
        "docked at Port Kane"
    );
    session
}

/// Port Kane trades every standard commodity: food and medical supplies
/// high, industrial, luxury goods and metal medium, equipment low, at the
/// community's table of prices.
#[test]
fn port_kanes_exchange_trades_at_its_levels() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let market = at_port_kane(&data).market().expect("a trade center");
    let standard: Vec<_> = market
        .rows
        .iter()
        .filter(|row| matches!(row.good, Good::Commodity(_)))
        .map(|row| (row.name.as_str(), row.price))
        .collect();
    assert_eq!(
        standard,
        [
            ("Food", 93),
            ("Industrial", 350),
            ("Medical Supplies", 937),
            ("Luxury Goods", 900),
            ("Metal", 200),
            ("Equipment", 440),
        ]
    );
    // It buys the Ancient Vell-os Sculpture (`jünk` 138, base 500) high.
    let sculpture = market.row(Good::Junk(JunkId(138))).expect("listed");
    assert_eq!(
        (sculpture.price, sculpture.sold_here, sculpture.bought_here),
        (625, false, true)
    );
    assert_eq!(market.events, Vec::<String>::new(), "no event yet");
}

/// The Shuttle, the first `chär`'s ship, holds 10 tons, all free.
#[test]
fn the_shuttle_holds_10_tons() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let session = at_port_kane(&data);
    assert_eq!(session.ship(), ShipId(128));
    assert_eq!(data.ship_fields(ShipId(128)).map(|f| f.holds), Ok(10));
    assert_eq!(session.capacity(), 10);
    assert_eq!(session.market().map(|m| m.free), Some(10));
}

/// `öops` 128, "An enormous food surplus", lowers food at Port Kane by 15
/// for 30 days, with a 35 % chance a day.
#[test]
fn the_food_surplus_targets_port_kane() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let disasters = data.disasters();
    assert_eq!(disasters.len(), 19);
    let surplus = disasters
        .iter()
        .find(|event| event.id == DisasterId(128))
        .expect("öops 128");
    assert_eq!(
        surplus,
        &DisasterRecord {
            id: DisasterId(128),
            name: "An enormous food surplus".to_owned(),
            stellar: 137,
            commodity: 0,
            price_delta: -15,
            duration: 30,
            freq: 35,
            activate_on: String::new(),
        }
    );
    assert_eq!(data.junk().len(), 23);
    let strings = data.commodity_strings();
    assert_eq!(
        strings.base_prices,
        ["75", "350", "750", "900", "200", "550"]
    );
}
