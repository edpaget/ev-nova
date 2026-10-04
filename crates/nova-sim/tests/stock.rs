//! A flight session over the stock data: the first `chär` starts a session
//! with its ship's handling and reserves in a system that exists, Port
//! Kane's exchange trades at its levels, and its outfitter sells what its
//! tech levels allow; Viking's shipyard sells what its tech levels and the
//! ships' `BuyRandom` allow, and trades the Shuttle in. Port Kane sells
//! fuel and uninhabited Reflex-ion sells none. Skips, passing,
//! when `NOVA_DATA` is unset.

mod common;

use nova_data::GameData;
use nova_data::records::character::Character;
use nova_data::records::ship::Ship;
use nova_data::records::stellar::Stellar;
use nova_sim::fuel::FUEL_SCOOP;
use nova_sim::{
    Direction, DisasterId, DisasterRecord, GameDate, Gauge, Good, JunkId, LandingRefusal, OutfitId,
    OutfitMod, OutfitOrder, OutfitRefusal, Pilot, PilotCatalog, RechargeRefusal, Service, Session,
    ShipFields, ShipId, ShipState, ShipStats, StartDate, StellarId, check_landing, services,
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
        mass: ship.mass,
        free_mass: ship.free_mass,
        contribute: ship.contribute.bits(),
    };
    assert_eq!(data.ship_fields(session.ship()), Ok(fields));
    // The Shuttle carries no default items: its own fields are its stats.
    let stats = ShipStats::new(fields, &[]);
    assert_eq!(session.stats(), stats);
    assert_eq!(session.handling(), stats.handling);
    assert_eq!(session.reserves(), stats.full());
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
/// no fuel; the Scarab's Matter/Antimatter Reactor (`oütf` 235), a fuel
/// scoop of 4, adds a unit every 4 ticks to its own every 10.
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
    let defaults = data.default_outfits(ShipId(162));
    assert!(defaults.contains(&(OutfitId(235), 1)), "{defaults:?}");
    let records = data.outfits();
    let mods: Vec<OutfitMod> = defaults
        .iter()
        .filter_map(|&(id, count)| {
            let record = records.iter().find(|record| record.id == id)?;
            Some(record.mods.map(|(mod_type, mod_val)| OutfitMod {
                mod_type,
                mod_val,
                count,
            }))
        })
        .flatten()
        .collect();
    assert!(
        mods.contains(&OutfitMod {
            mod_type: FUEL_SCOOP,
            mod_val: 4,
            count: 1,
        }),
        "{mods:?}"
    );
    let fields = data.ship_fields(ShipId(162)).expect("decodes");
    let regen = ShipStats::new(fields, &mods).fuel_regen;
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

/// Port Kane (tech level 4, special tech 6, 55, 57, 58 and 81, of the
/// Federation) lists the Battery Pack (`oütf` 256) and Solar Panels (228)
/// at tech level 3, the Fission Reactor (179) at its special tech 6 (too
/// heavy for the Shuttle's 8 free tons), and not the Fusion Reactor (177)
/// at 8. Carbon Fiber (180) is listed but
/// cannot be bought: its `Require` (0x800000001), scoped to the
/// Federation, wants a licence the Shuttle lacks.
#[test]
fn port_kanes_outfitter_sells_what_its_tech_levels_allow() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let outfitter = at_port_kane(&data).outfitter().expect("an outfitter");
    let row = |id| outfitter.row(OutfitId(id));
    let battery = row(256).expect("the Battery Pack");
    assert_eq!(
        (battery.name.as_str(), battery.price, battery.mass),
        ("Battery Pack", 10_000, 3)
    );
    assert_eq!(battery.buy, Ok(()));
    assert_eq!(row(228).map(|r| r.name.as_str()), Some("Solar Panels"));
    let fission = row(179).expect("the Fission Reactor");
    assert_eq!(fission.buy, Err(OutfitRefusal::NoSpaceForAny));
    assert!(fission.mass > 8, "{}", fission.mass);
    assert!(row(177).is_none(), "the Fusion Reactor is tech 8");
    let fiber = row(180).expect("Carbon Fiber");
    assert_eq!(fiber.buy, Err(OutfitRefusal::NotForSale));
    assert_eq!((outfitter.cash, outfitter.free_mass), (25_000, 8));
}

/// Buying a Battery Pack takes 10,000 of the Shuttle's 25,000 credits
/// and 3 of its 8 tons free, and raises its fuel from 300 to 400.
#[test]
fn a_battery_pack_adds_a_jump_of_fuel_to_the_shuttle() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let mut session = at_port_kane(&data);
    assert_eq!(session.reserves().fuel, Gauge::full(300.0));
    let battery = OutfitOrder {
        outfit: OutfitId(256),
        direction: Direction::Buy,
    };
    assert_eq!(session.outfit(battery), Ok(()));
    let outfitter = session.outfitter().expect("an outfitter");
    assert_eq!((outfitter.cash, outfitter.free_mass), (15_000, 5));
    assert_eq!(session.pilot().owned(OutfitId(256)), 1);
    assert_eq!(
        session.reserves().fuel,
        Gauge::full(400.0),
        "the new tank comes full"
    );
}

/// A new stock pilot, docked at Viking (`spöb` 157 in Tichel, `sÿst`
/// 129, a jump from Kania), through a save that says so.
fn at_viking(data: &GameData) -> Session {
    let pilot = Pilot::new(data, "Stock").expect("the stock first chär starts");
    let mut save: serde_json::Value =
        serde_json::from_str(&nova_sim::save::encode(&pilot)).expect("JSON");
    save["system"] = serde_json::json!(129);
    save["stellar"] = serde_json::json!(157);
    let docked = nova_sim::save::decode(&save.to_string()).expect("a pilot");
    let session = Session::fly(data, docked).expect("flies");
    assert_eq!(session.landed(), Some(StellarId(157)), "docked at Viking");
    session
}

/// Viking (tech level 4, special tech 81, 16, 12 and 10) sells the
/// Shuttle, the Heavy Shuttle, the Terrapin and the Viper, and none of
/// the ships whose `BuyRandom` is 0, though its tech levels allow them
/// (the Cargo Drone among them).
#[test]
fn vikings_shipyard_sells_what_its_tech_levels_and_buy_random_allow() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let session = at_viking(&data);
    let shipyard = session.shipyard().expect("a shipyard");
    for id in [128, 129, 136, 167] {
        assert!(shipyard.row(ShipId(id)).is_some(), "{id}: {shipyard:?}");
    }
    let site = data
        .landing_sites(nova_sim::SystemId(129))
        .into_iter()
        .find(|site| site.id == StellarId(157))
        .expect("Viking is in Tichel");
    let never: Vec<_> = data
        .ships()
        .into_iter()
        .filter(|ship| ship.buy_random <= 0 && nova_sim::wares::tech_allows(ship.tech_level, &site))
        .map(|ship| ship.id)
        .collect();
    // Among them the Cargo Drone, `shïp` 130, of tech level 4.
    assert!(never.contains(&ShipId(130)), "{never:?}");
    for id in never {
        assert!(shipyard.row(id).is_none(), "{id:?}");
    }
    assert_eq!(shipyard.trade_in, 2500, "a quarter of the Shuttle");
    assert_eq!(shipyard.cash, 25_000);
    assert_eq!(shipyard.current, ShipId(128));
}

/// Buying the Heavy Shuttle (17,500 credits) trades the Shuttle in for
/// 2,500 and leaves 10,000 credits, 15 tons of cargo space and 12 free.
#[test]
fn a_heavy_shuttle_trades_in_the_shuttle() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let mut session = at_viking(&data);
    let bought = session.buy_ship(ShipId(129)).expect("bought");
    assert_eq!((bought.price, bought.trade_in), (17_500, 2500));
    assert_eq!(session.ship(), ShipId(129));
    assert_eq!(session.pilot().cash(), 10_000);
    assert_eq!(session.capacity(), 15);
    assert_eq!(session.outfitter().map(|o| o.free_mass), Some(12));
    let fields = data.ship_fields(ShipId(129)).expect("decodes");
    assert_eq!(session.stats(), ShipStats::new(fields, &[]));
    assert_eq!(session.pilot().outfits().count(), 0);
}

/// A new stock pilot, docked at `stellar` in `system` with `fuel` units
/// in its tank, through a save that says so.
fn docked_with_fuel(data: &GameData, system: i16, stellar: i16, fuel: f32) -> Session {
    let pilot = Pilot::new(data, "Stock").expect("the stock first chär starts");
    let mut save: serde_json::Value =
        serde_json::from_str(&nova_sim::save::encode(&pilot)).expect("JSON");
    save["system"] = serde_json::json!(system);
    save["stellar"] = serde_json::json!(stellar);
    save["reserves"]["fuel"]["now"] = serde_json::json!(fuel);
    let docked = nova_sim::save::decode(&save.to_string()).expect("a pilot");
    let session = Session::fly(data, docked).expect("flies");
    assert_eq!(session.landed(), Some(StellarId(stellar)), "docked");
    session
}

/// At Port Kane (`spöb` 137 in Kania, `sÿst` 128) the Shuttle (300 fuel,
/// no regeneration), half empty, refills for 150 of its 25,000 credits.
#[test]
fn port_kane_refills_the_shuttle_for_a_credit_a_unit() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let mut session = docked_with_fuel(&data, 128, 137, 150.0);
    assert_eq!(session.pilot().system(), at_port_kane(&data).system());
    assert_eq!(session.pilot().cash(), 25_000);
    assert_eq!(session.recharge(), Ok(150));
    assert_eq!(session.reserves().fuel, Gauge::full(300.0));
    assert_eq!(session.pilot().cash(), 25_000 - 150);
}

/// Reflex-ion (`spöb` 129 in Agate, `sÿst` 196, flags `0x21`) is
/// uninhabited, so it sells no fuel.
#[test]
fn reflex_ion_sells_no_fuel() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let mut session = docked_with_fuel(&data, 196, 129, 150.0);
    assert_eq!(session.recharge(), Err(RechargeRefusal::NoFuel));
    assert_eq!(
        session.reserves().fuel,
        Gauge {
            now: 150.0,
            max: 300.0
        }
    );
}
