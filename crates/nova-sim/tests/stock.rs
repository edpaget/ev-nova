//! A flight session over the stock data: the first `chär` starts a session
//! with its ship's handling and reserves in a system that exists, and a
//! new pilot begins with the bits its blank `OnStart` sets (none); Port
//! Kane's exchange trades at its levels, and its outfitter sells what its
//! tech levels allow, and the Vell-os map only once its control bit is
//! set, which explores the systems around and is not added, and the Cheap
//! Thorium Reactor, whose `OnPurchase` and `OnSell` set and clear its
//! bit; Viking's shipyard sells what its tech levels and the
//! ships' `BuyRandom` allow, and trades the Shuttle in; the ships go by
//! their names without the designers' notes. Port Kane sells
//! fuel and uninhabited Reflex-ion sells none. The date reads with the
//! first `chär`'s affixes. HG-Kania leads to HG-Tichel. NPC traffic flies the
//! ships and governments its system's `düde`s and fleets give, and
//! Alphara's `DudeTypes` fleet comes when its roll fires. The governments
//! stand as their `gövt`s say, and in Fomalhaut the player's attack on a
//! Civvies trader puts it to flight, brings the Federation down on the
//! player and costs it 3 with each. A Federation ship hailed there
//! greets with its government's line, and one hunting a wanted player is
//! bought off; the ship comm strings, the 42-character advice lines and
//! the governments' hail flags are where hailing reads them. The fighter
//! bays launch their fighters as the extracted rules say. Viking's bar
//! hires the Shuttle at Nova's fee and wage. The persons read as their
//! `përs` say: Kania's Person slot and the person roll bring the UFS
//! Razorback on its ship and loadout where the Federation rules, and it
//! greets with its comm quote; Jack Folstam appears in Nesre Primus and,
//! by the engine's slip, in J'raphit; the Bounty Hunter's hail quote
//! names him and the pilot, and once gone for good he never appears
//! again. Skips, passing, when `NOVA_DATA` is unset.

mod common;

use nova_data::GameData;
use nova_data::records::character::Character;
use nova_data::records::ship::Ship;
use nova_data::records::stellar::Stellar;
use nova_sim::Test;
use nova_sim::fuel::FUEL_SCOOP;
use nova_sim::{
    Clearance, Direction, DisasterId, DisasterRecord, GameDate, GateKind, Gauge, Good, GovtId,
    JunkId, LandOutcome, LandPress, LandingRefusal, NeverFires, OutfitId, OutfitMod, OutfitOrder,
    OutfitRefusal, Pilot, PilotCatalog, RechargeRefusal, Service, Session, ShipFields, ShipId,
    ShipState, ShipStats, StartDate, StellarId, SystemId, Vec2, check_landing, services,
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

/// A new stock pilot, flown and begun, holds exactly the bits its first
/// `chär`'s `OnStart` writes: in the stock 1.0.10 data that `OnStart` is
/// blank, so none, and running it tells nothing.
#[test]
fn a_new_stock_pilot_begins_with_the_bits_its_chärs_on_start_sets() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let (_, first) = data.records::<Character>().next().expect("a chär");
    let on_start = first.expect("decodes").record.on_start.as_str().to_owned();
    assert_eq!(on_start, "", "the stock OnStart is blank");
    let writes = nova_sim::SetExpr::parse(&on_start)
        .expect("parses")
        .writes();
    let pilot = Pilot::new(&data, "Stock").expect("the stock first chär starts");
    let mut session = Session::fly(&data, pilot).expect("flies");
    session.begin(&data, &mut NeverFires);
    let set: Vec<nova_sim::Bit> = session.pilot().control_bits().iter().collect();
    let expected: Vec<nova_sim::Bit> = writes
        .iter()
        .filter(|(_, write)| *write == nova_sim::BitWrite::Set)
        .map(|(bit, _)| *bit)
        .collect();
    assert_eq!(set, expected);
    assert_eq!(session.take_script_notes(), []);
}

/// A stock session's date reads "June 23, 1177 NC": the first `chär`'s
/// empty `DatePrefix` and its `DateSuffix`, " NC".
#[test]
fn a_stock_session_shows_its_date_with_the_chärs_affixes() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let affixes = data.date_affixes();
    assert_eq!(
        (affixes.prefix.as_str(), affixes.suffix.as_str()),
        ("", " NC")
    );
    let session = Session::start(&data).expect("the stock first chär starts");
    assert_eq!(session.date_text(), "June 23, 1177 NC");
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
        flags2: ship.flags2.bits(),
        contribute: ship.contribute.bits(),
        shield_rech: ship.shield_rech,
        armor_rech: ship.armor_rech,
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
        check_landing(
            &parked,
            kania,
            session.star_map().govt(session.system()),
            |govt| session.pilot().legal_record(govt),
        ),
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
            activate_on: Test::default(),
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

/// Stock `oütf` 342, "Area Map - Vell-os", tech level 0 and hidden while
/// its `Availability`, `b9999`, does not hold: Port Kane's outfitter does
/// not list it to a new pilot, and lists it, for sale, once control bit
/// 9999 is set through the session's bit edit.
#[test]
fn port_kanes_outfitter_lists_the_vell_os_map_only_once_its_bit_is_set() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let mut session = at_port_kane(&data);
    let map = OutfitId(342);
    let outfitter = session.outfitter().expect("an outfitter");
    assert!(outfitter.row(map).is_none(), "bit 9999 is clear");
    session.set_control_bit(nova_sim::Bit::new(9999).expect("a bit"), true);
    let outfitter = session.outfitter().expect("an outfitter");
    let row = outfitter.row(map).expect("listed once bit 9999 is set");
    assert_eq!(row.name, "Area Map - Vell-os");
    assert_eq!(row.buy, Ok(()));
}

/// Buying the stock Vell-os map (`oütf` 342, `ModType` 16, `ModVal` 2, no
/// cost) at Port Kane, once bit 9999 is set, explores the systems within
/// 2 jumps of Port Kane's and does not add it. By the engine's depth-first
/// walk every system a jump away is explored and none beyond 2 jumps; by
/// the Bible's reading, exactly every system within 2 jumps, worked out
/// here breadth first along the stock hyperlinks.
#[test]
fn buying_the_stock_vell_os_map_explores_two_jumps_and_adds_nothing() {
    use std::collections::BTreeSet;
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let map = OutfitId(342);
    let record = data
        .outfits()
        .into_iter()
        .find(|outfit| outfit.id == map)
        .expect("the Vell-os map");
    assert_eq!(record.mods[0], (16, 2), "a map of 2 jumps");
    let bought = |rules: nova_sim::OutfitRules| {
        let mut session = at_port_kane(&data).with_outfit_rules(rules);
        session.set_control_bit(nova_sim::Bit::new(9999).expect("a bit"), true);
        let cash = session.pilot().cash();
        let order = OutfitOrder {
            outfit: map,
            direction: Direction::Buy,
        };
        assert_eq!(session.outfit(order, &mut NeverFires), Ok(()));
        assert_eq!(session.pilot().owned(map), 0, "not added");
        assert_eq!(session.pilot().cash(), cash, "it costs nothing");
        session
    };
    let engine = bought(nova_sim::OutfitRules::default());
    let home = engine.pilot().system();
    let stars = engine.star_map();
    let rule = nova_sim::HyperlinkRule::Engine;
    let one: BTreeSet<SystemId> = std::iter::once(home)
        .chain(stars.jumps(home, rule))
        .collect();
    let two: BTreeSet<SystemId> = one
        .iter()
        .flat_map(|&system| stars.jumps(system, rule))
        .chain(one.iter().copied())
        .collect();
    assert!(two.len() > one.len(), "{one:?} {two:?}");
    let explored: BTreeSet<SystemId> = engine.pilot().explored().collect();
    assert!(one.is_subset(&explored), "{explored:?}");
    assert!(explored.is_subset(&two), "{explored:?}");
    let bible = bought(nova_sim::OutfitRules {
        map_explore: nova_sim::RuleSource::Bible,
        ..nova_sim::OutfitRules::default()
    });
    assert_eq!(bible.pilot().explored().collect::<BTreeSet<_>>(), two);
}

/// Port Kane (special tech 57) sells the Cheap Thorium Reactor (`oütf`
/// 358), whose `OnPurchase` sets bit 9011 and whose `OnSell` clears it.
#[test]
fn the_stock_cheap_thorium_reactor_sets_its_bit_when_bought_and_clears_it_when_sold() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let mut session = at_port_kane(&data);
    assert!(session.transact(|pilot| pilot.set_cash(1_000_000)));
    let reactor = OutfitId(358);
    let bit = nova_sim::Bit::new(9011).expect("a bit");
    let order = |direction| OutfitOrder {
        outfit: reactor,
        direction,
    };
    assert_eq!(
        session.outfit(order(Direction::Buy), &mut NeverFires),
        Ok(())
    );
    assert_eq!(session.pilot().owned(reactor), 1);
    assert!(session.control_bit(bit), "OnPurchase b9011");
    assert_eq!(
        session.outfit(order(Direction::Sell), &mut NeverFires),
        Ok(())
    );
    assert!(!session.control_bit(bit), "OnSell !b9011");
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
    assert_eq!(session.outfit(battery, &mut NeverFires), Ok(()));
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

/// The stock ships go by their names without the designers' notes after
/// a ';': `shïp` 361, "Shuttle;Second-Hand - poor", is a "Shuttle".
#[test]
fn stock_ships_are_named_without_their_designer_notes() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let ships = data.ships();
    let name = |id| {
        let ship = ships.iter().find(|ship| ship.id == ShipId(id));
        ship.map(|ship| ship.name.as_str())
    };
    assert_eq!(name(361), Some("Shuttle"));
    assert_eq!(name(191), Some("Lightning"));
    let noted: Vec<_> = ships
        .iter()
        .filter(|ship| ship.name.contains(';'))
        .collect();
    assert!(noted.is_empty(), "{noted:?}");
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
    let bought = session
        .buy_ship(ShipId(129), &mut NeverFires)
        .expect("bought");
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

/// A small seeded generator (xorshift64*), so the stock traffic tests roll
/// the same dice every run; it never fires a percentage.
struct Seeded(u64);

impl nova_sim::Chance for Seeded {
    fn fires(&mut self, _percent: u8) -> bool {
        false
    }

    fn below(&mut self, n: u32) -> u32 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        let draw = self.0.wrapping_mul(0x2545_F491_4F6C_DD1D);
        u32::try_from(draw % u64::from(n.max(1))).expect("below n")
    }
}

/// Every (ship, government) system `id`'s traffic may fly, read straight
/// from the records: each `düde` its `DudeTypes` name, each fleet they
/// name, each fleet whose `LinkSyst` matches it, and each person its
/// `LinkSyst` or the system's Person slots allow.
fn allowed(data: &GameData, id: i16) -> Vec<(ShipId, Option<nova_sim::GovtId>)> {
    use nova_data::records::dude::Dude;
    use nova_data::records::fleet::Fleet;
    use nova_data::records::system::System;
    let system = data
        .get::<System>(id)
        .expect("present")
        .expect("decodes")
        .record;
    let mut allowed = Vec::new();
    for &value in &system.dude_types {
        if (128..=639).contains(&value) {
            let dude = data.get::<Dude>(value).expect("present").expect("decodes");
            let dude = dude.record;
            allowed.extend(
                dude.ship_type
                    .iter()
                    .flatten()
                    .map(|&ship| (ship, dude.govt)),
            );
        }
    }
    let named: Vec<i16> = system
        .dude_types
        .iter()
        .filter(|value| (-383..=-128).contains(*value))
        .map(|value| -value)
        .collect();
    for (fleet_id, fleet) in data.records::<Fleet>() {
        let Ok(fleet) = fleet else { continue };
        let fleet = fleet.record;
        let link = fleet.link_syst;
        let govt = system.govt.map(|govt| govt.0);
        let linked = link == -1
            || link == id
            || ((10_000..15_000).contains(&link) && govt == Some(link - 10_000 + 128))
            || ((20_000..25_000).contains(&link) && govt.is_some_and(|g| g != link - 20_000 + 128));
        if linked || named.contains(&fleet_id) {
            let ships = fleet
                .lead_ship_type
                .into_iter()
                .chain(fleet.escort_type.into_iter().flatten());
            allowed.extend(ships.map(|ship| (ship, fleet.govt)));
        }
    }
    // Each person whose `LinkSyst` (by the engine's slip too) or a Person
    // slot of the system allows it, flying its ship.
    let govts = nova_sim::Governments::read(data);
    for person in nova_sim::TrafficCatalog::persons(data) {
        let linked = nova_sim::person::PersonLink::decode(person.link_syst).matches(
            nova_sim::SystemId(id),
            system.govt,
            &govts,
            nova_sim::RuleSource::Engine,
        );
        let slotted = system.person.contains(&Some(person.id));
        if let Some(ship) = person.ship.filter(|_| linked || slotted) {
            allowed.push((ship, person.govt));
        }
    }
    allowed
}

/// Every NPC seen in `session`'s system over `ticks` ticks of traffic.
fn traffic_seen(
    data: &GameData,
    session: &mut Session,
    chance: &mut Seeded,
    ticks: u32,
) -> Vec<(ShipId, Option<nova_sim::GovtId>)> {
    let mut seen: Vec<_> = session
        .npcs()
        .iter()
        .map(|npc| (npc.ship, npc.govt))
        .collect();
    for _ in 0..ticks {
        session.tick_traffic(data, &nova_sim::ai::Peaceful, chance);
        seen.extend(session.npcs().iter().map(|npc| (npc.ship, npc.govt)));
    }
    seen
}

/// The new pilot's starting system and the first system linked to it, its
/// traffic populated on its first tick and when it arrives: every NPC
/// flies a ship of a `düde` of the system's, or of a fleet it names or its
/// `LinkSyst` matches, for its government.
#[test]
fn stock_traffic_flies_the_systems_dudes_and_fleets() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let mut chance = Seeded(0x5EED_CAFE);
    let mut session = Session::start(&data).expect("the stock first chär starts");
    let start = session.system();
    let seen = traffic_seen(&data, &mut session, &mut chance, 3000);
    assert!(!seen.is_empty(), "traffic in sÿst {}", start.0);
    let here = allowed(&data, start.0);
    for npc in &seen {
        assert!(here.contains(npc), "{npc:?} in sÿst {}", start.0);
    }
    // Jump to the first system linked to it, flying straight out.
    let next = first_link(&data, start.0);
    session.plot_course(next).expect("a route");
    let mut tries = 0;
    while session.player().position.length() < session.stats().jump_distance {
        session.tick(nova_sim::Controls {
            thrust: true,
            ..nova_sim::Controls::default()
        });
        tries += 1;
        assert!(tries < 10_000, "never got out");
    }
    session.begin_jump().expect("jumps");
    finish_pre_jump(&mut session);
    assert_eq!(session.arrive(&data, &mut chance), Some(next));
    let seen = traffic_seen(&data, &mut session, &mut chance, 3000);
    assert!(!seen.is_empty(), "traffic in sÿst {}", next.0);
    let there = allowed(&data, next.0);
    for npc in &seen {
        assert!(there.contains(npc), "{npc:?} in sÿst {}", next.0);
    }
}

/// The first hyperlink of `sÿst` `id`.
fn first_link(data: &GameData, id: i16) -> nova_sim::SystemId {
    use nova_data::records::system::System;
    let system = data
        .get::<System>(id)
        .expect("present")
        .expect("decodes")
        .record;
    system
        .con
        .into_iter()
        .flatten()
        .next()
        .expect("a hyperlink")
}

/// Draws its script, then 0 for ever.
struct Script(std::collections::VecDeque<u32>);

impl nova_sim::Chance for Script {
    fn fires(&mut self, _percent: u8) -> bool {
        false
    }

    fn below(&mut self, _n: u32) -> u32 {
        self.0.pop_front().unwrap_or(0)
    }
}

/// Draws by the bound asked, as listed, and 0 for any other bound.
struct ByBound(&'static [(u32, u32)]);

impl nova_sim::Chance for ByBound {
    fn fires(&mut self, _percent: u8) -> bool {
        false
    }

    fn below(&mut self, n: u32) -> u32 {
        self.0
            .iter()
            .find(|&&(bound, _)| bound == n)
            .map_or(0, |&(_, draw)| draw)
    }
}

/// Every person roll fires but lands on no stock person (1149, past the
/// last), and no Person slot below 100 % lists its person.
fn no_person() -> ByBound {
    ByBound(&[(1022, 1021), (100, 99)])
}

/// Alphara (`sÿst` 131) names `flët` 129 in its `DudeTypes`, at 20 %:
/// when an arrival's roll lands on 1 and the percentage roll is within
/// 20, the fleet's lead jumps in with its escorts, for its government.
#[test]
fn alpharas_named_fleet_comes_when_its_roll_fires() {
    use nova_data::records::fleet::Fleet;
    use nova_data::records::system::System;
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let alphara = data
        .get::<System>(131)
        .expect("present")
        .expect("decodes")
        .record;
    assert!(
        alphara
            .dude_types
            .iter()
            .zip(alphara.prob)
            .any(|pair| pair == (&-129, 20))
    );
    let fleet = data
        .get::<Fleet>(129)
        .expect("present")
        .expect("decodes")
        .record;
    let pilot = Pilot::new(&data, "Stock").expect("the stock first chär starts");
    let mut save: serde_json::Value =
        serde_json::from_str(&nova_sim::save::encode(&pilot)).expect("JSON");
    save["system"] = serde_json::json!(131);
    let pilot = nova_sim::save::decode(&save.to_string()).expect("a pilot");
    let mut session = Session::fly(&data, pilot).expect("flies");
    // Every setup pass draws a person, who never comes: no traffic yet.
    session.populate(&data, &mut no_person());
    assert_eq!(session.npcs(), []);
    // Rand(500) = 1, then Rand(100) + 1 = 1, within 20: the first named
    // fleet, flët 129.
    session.tick_traffic(
        &data,
        &nova_sim::ai::Peaceful,
        &mut Script(std::collections::VecDeque::from([1])),
    );
    let lead = session.npcs().first().expect("the fleet's lead");
    assert_eq!(Some(lead.ship), fleet.lead_ship_type);
    assert_eq!(lead.govt, fleet.govt);
    let escorts = session
        .npcs()
        .iter()
        .filter(|npc| npc.leader == Some(lead.id))
        .count();
    let least: i16 = fleet
        .escort_type
        .iter()
        .zip(fleet.min)
        .filter(|(ship, _)| ship.is_some())
        .map(|(_, min)| min.max(0))
        .sum();
    assert_eq!(
        escorts,
        usize::try_from(least).expect("count"),
        "each type's Min"
    );
}

// Combat over the stock data.

use std::collections::{BTreeMap, BTreeSet};

use nova_sim::combat::armament::{Arsenal, Rounds};
use nova_sim::combat::flags::FlagField;
use nova_sim::combat::weapon::{Ammo, Guidance, WeaponSpec};
use nova_sim::combat::{Combat, Fighter, Rules};
use nova_sim::{
    Armament, CombatCatalog, CombatEvent, Condition, HullSpec, Reserves, ShipRef, SimDiagnostic,
    Trigger, WeaponId,
};

/// Draws the middle outcome: every shot leaves straight ahead.
struct Straight;

impl nova_sim::Chance for Straight {
    fn fires(&mut self, _percent: u8) -> bool {
        false
    }

    fn below(&mut self, n: u32) -> u32 {
        n / 2
    }
}

/// A ship in a stock fight, owning what a fighter borrows.
struct Combatant {
    id: ShipRef,
    ship_type: ShipId,
    target: Option<ShipRef>,
    state: ShipState,
    hull: HullSpec,
    trigger: Trigger,
    reserves: Reserves,
    condition: Condition,
    armament: Armament,
    rounds: BTreeMap<WeaponId, u32>,
}

impl Combatant {
    fn fighter(&mut self) -> Fighter<'_> {
        Fighter {
            ship: self.id,
            ship_type: self.ship_type,
            fleet: self.id,
            govt: None,
            state: self.state,
            hull: self.hull,
            shield_regen: 0.0,
            armor_regen: 0.0,
            trigger: self.trigger,
            target: self.target,
            reserves: &mut self.reserves,
            condition: &mut self.condition,
            armament: &mut self.armament,
            rounds: &mut self.rounds as &mut dyn Rounds,
        }
    }
}

fn fight(combat: &mut Combat, ships: &mut [Combatant]) {
    fight_with(combat, ships, &Arsenal::default());
}

/// A tick of `combat` among `ships`, sub-munitions read from `arsenal`.
fn fight_with(combat: &mut Combat, ships: &mut [Combatant], arsenal: &Arsenal) {
    let mut fighters: Vec<Fighter> = ships.iter_mut().map(Combatant::fighter).collect();
    combat.tick(
        &mut fighters,
        arsenal,
        &nova_sim::Governments::default(),
        Rules::default(),
        &mut Straight,
    );
}

/// The player at the centre, at rest, facing right, firing one of
/// `weapon` with all the ammunition, fuel and armour it could want.
fn shooter(weapon: &WeaponSpec) -> Combatant {
    let mut rounds = BTreeMap::new();
    if let Ammo::Rounds(ammo) = weapon.ammo {
        rounds.insert(ammo, 100_000);
    }
    Combatant {
        id: ShipRef::Player,
        ship_type: ShipId(128),
        target: None,
        state: ShipState {
            heading: 90.0,
            ..ShipState::default()
        },
        hull: HullSpec::default(),
        trigger: Trigger {
            primary: !weapon.secondary(),
            secondary: weapon.secondary().then_some(weapon.id),
            only: None,
            turrets_only: false,
            bays: false,
        },
        reserves: Reserves::full(0.0, 1_000_000.0, 1_000_000.0),
        condition: Condition::Intact,
        armament: Armament::new([(*weapon, 1)]),
        rounds,
    }
}

/// NPC 0, a ship of `ship` type `x` pixels right of the centre, at rest,
/// its reserves full as its stock fields and default items give them.
fn target(data: &GameData, arsenal: &Arsenal, ship: ShipId, x: f32) -> Combatant {
    let record = data
        .ships()
        .into_iter()
        .find(|record| record.id == ship)
        .expect("a stock ship");
    let outfits = data.outfits();
    let mods: Vec<OutfitMod> = record
        .defaults
        .iter()
        .flat_map(|&(id, count)| {
            outfits
                .iter()
                .filter(move |outfit| outfit.id == id)
                .flat_map(move |outfit| {
                    outfit.mods.map(|(mod_type, mod_val)| OutfitMod {
                        mod_type,
                        mod_val,
                        count,
                    })
                })
        })
        .collect();
    Combatant {
        id: ShipRef::Npc(nova_sim::NpcId(0)),
        ship_type: ship,
        target: None,
        state: ShipState {
            position: Vec2::new(x, 0.0),
            ..ShipState::default()
        },
        hull: arsenal.hull(ship),
        trigger: Trigger::default(),
        reserves: ShipStats::new(record.fields, &mods).full(),
        condition: Condition::Intact,
        armament: Armament::default(),
        rounds: BTreeMap::new(),
    }
}

/// Every stock weapon this phase flies: guidance -1, 0, 5 and 6.
fn in_scope(data: &GameData) -> Vec<WeaponSpec> {
    let weapons: Vec<WeaponSpec> = data
        .weapons()
        .iter()
        .map(WeaponSpec::new)
        .filter(|spec| {
            matches!(
                spec.guidance,
                Guidance::Unguided | Guidance::Beam | Guidance::FreefallBomb | Guidance::Rocket
            )
        })
        .collect();
    assert_eq!(weapons.len(), 17, "the stock weapons in scope");
    weapons
}

/// The ticks a weapon fires on from rest over `ticks`, as `_FirePlayerWeapon`
/// paces one copy: every `Reload` ticks (at least every tick), and after
/// each `BurstCount` shots its `BurstReload`.
fn paced(spec: &WeaponSpec, ticks: u32) -> Vec<u32> {
    let mut fired = Vec::new();
    let mut tick = 0;
    while tick < ticks {
        fired.push(tick);
        let ending = spec.burst_count > 0
            && u32::try_from(fired.len()).expect("few") % spec.burst_count == 0;
        let wait = if ending {
            spec.burst_reload
        } else {
            spec.reload
        };
        tick += (wait.ceil() as u32).max(1);
    }
    fired
}

/// The ticks one beam of `spec`, fired once, damages a ship that soaks
/// up any damage `x` pixels ahead.
fn beam_hits(spec: &WeaponSpec, x: f32) -> u32 {
    let mut sponge = Combatant {
        id: ShipRef::Npc(nova_sim::NpcId(0)),
        reserves: Reserves::full(100_000.0, 100_000.0, 0.0),
        trigger: Trigger::default(),
        armament: Armament::default(),
        ..shooter(spec)
    };
    sponge.state.position = Vec2::new(x, 0.0);
    let mut combat = Combat::default();
    let mut ships = [shooter(spec), sponge];
    let mut hits = 0;
    for _ in 0..spec.lifetime + 5 {
        let before = ships[1].reserves;
        fight(&mut combat, &mut ships);
        ships[0].trigger = Trigger::default();
        hits += u32::from(ships[1].reserves != before);
    }
    hits
}

/// Each stock weapon in scope, fired from rest into empty space: a shot
/// leaves at `Speed`/100 (a rocket at its firer's rest, closing on that),
/// lives `Count` ticks and reaches `Speed`/100 x `Count` (a beam hits for
/// `Count` ticks and reaches its `BeamLength` and the ship's radius), and
/// it fires as often as item 7 of the plan says.
#[test]
fn every_stock_weapon_in_scope_flies_its_speed_life_and_range_at_its_reload() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    for spec in in_scope(&data) {
        let id = spec.id.0;
        let mut combat = Combat::default();
        let mut ships = [shooter(&spec)];
        fight(&mut combat, &mut ships);
        ships[0].trigger = Trigger::default();
        if spec.guidance == Guidance::Beam {
            assert!((spec.range() - spec.beam_length).abs() < 1e-6, "{id}");
            let reach = spec.beam_length + nova_sim::combat::hull::DEFAULT_HIT_RADIUS;
            assert_eq!(beam_hits(&spec, reach - 0.5), spec.lifetime.max(1), "{id}");
            assert_eq!(beam_hits(&spec, reach + 0.5), 0, "{id}: out of reach");
        } else {
            let shot = combat.shots()[0];
            let speed = shot.velocity.length();
            if spec.guidance == Guidance::Rocket {
                assert!(speed <= spec.speed * 0.05 + 1e-4, "{id}");
            } else {
                assert!((speed - spec.speed).abs() < 1e-3, "{id}: {speed}");
            }
            let mut lived = 1;
            let mut last = shot.position;
            while let Some(shot) = combat.shots().first() {
                last = shot.position;
                fight(&mut combat, &mut ships);
                lived += 1;
            }
            assert_eq!(lived, spec.lifetime.max(1), "{id}");
            // Seen last a tick before it flies its last tick and goes: a
            // tick short of its range.
            let flown = last.length() + spec.speed;
            let range = spec.speed * spec.lifetime as f32;
            assert!((spec.range() - range).abs() < 1e-3, "{id}");
            if spec.guidance == Guidance::Rocket {
                assert!(flown <= range + 1e-3, "{id}");
            } else {
                assert!((flown - range).abs() < 1e-2, "{id}: {flown}");
            }
        }
        let mut combat = Combat::default();
        let mut ships = [shooter(&spec)];
        let mut fired = Vec::new();
        for tick in 0..300 {
            fight(&mut combat, &mut ships);
            if combat
                .take_events()
                .iter()
                .any(|event| matches!(event, CombatEvent::Fired { .. }))
            {
                fired.push(tick);
            }
        }
        assert_eq!(fired, paced(&spec, 300), "wëap {id}");
    }
}

/// Each stock weapon in scope, fired at the stock ship with the most
/// shield: its shields fall first, and its armour only once they are
/// down; the shield-passing weapons (174 and 232) take only the armour.
#[test]
fn every_stock_weapon_in_scope_takes_the_shields_before_the_armour() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let arsenal = Arsenal::read(&data);
    let toughest = data
        .ships()
        .into_iter()
        .max_by_key(|record| (record.fields.shield, record.fields.armor))
        .expect("ships")
        .id;
    for spec in in_scope(&data) {
        let id = spec.id.0;
        let reach = if spec.guidance == Guidance::Beam {
            spec.beam_length / 2.0
        } else {
            spec.range().clamp(1.0, 60.0)
        };
        let mut combat = Combat::default();
        let mut ships = [shooter(&spec), target(&data, &arsenal, toughest, reach)];
        let full = ships[1].reserves;
        let mut seen = Vec::new();
        for _ in 0..400 {
            fight(&mut combat, &mut ships);
            seen.push(ships[1].reserves);
        }
        let hurt = seen.iter().position(|r| *r != full);
        let Some(hurt) = hurt else {
            // The Stellar Grenade (Speed 0) blows up where it is launched,
            // too far from anything to fire at.
            assert!(spec.speed.abs() < f32::EPSILON, "wëap {id} never hit");
            continue;
        };
        if spec.passes_shields() {
            assert!(seen.iter().all(|r| r.shield == full.shield), "wëap {id}");
            assert!(seen[hurt].armor.now < full.armor.now, "wëap {id}");
        } else {
            assert!(seen[hurt].shield.now < full.shield.now, "wëap {id}");
            assert_eq!(seen[hurt].armor, full.armor, "wëap {id}: shields first");
            for reserves in &seen {
                if reserves.armor.now < full.armor.now {
                    assert!(reserves.shield.now <= 0.0, "wëap {id}: {reserves:?}");
                    break;
                }
            }
        }
    }
}

/// Every stock weapon in scope, fired again and again in one fight:
/// exactly the Mining Blaster and the three Wraith Graviton Beams report
/// their x10 damage to asteroids (`Flags2` 0x8000), each once.
#[test]
fn only_the_asteroid_damage_flag_is_reported_each_once() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let mut combat = Combat::default();
    let mut ships: Vec<Combatant> = in_scope(&data).iter().map(shooter).collect();
    let mut diagnostics = Vec::new();
    for _ in 0..200 {
        fight(&mut combat, &mut ships);
        diagnostics.extend(combat.take_diagnostics());
    }
    let expected: BTreeSet<SimDiagnostic> = [181, 165, 167, 168]
        .into_iter()
        .map(|id| SimDiagnostic::UnimplementedWeaponFlag {
            weapon: WeaponId(id),
            field: FlagField::Flags2,
            bit: 0x8000,
        })
        .collect();
    assert_eq!(diagnostics.len(), 4, "{diagnostics:?}");
    assert_eq!(diagnostics.into_iter().collect::<BTreeSet<_>>(), expected);
}

/// Each ship type a trader `düde` (AI 1 or 2) flies, hit with Light
/// Blaster shots until it is disabled and then left alone, ends up
/// disabled, with armour left, and still there.
#[test]
fn a_stock_trader_beaten_in_combat_is_disabled_and_still_alive() {
    use nova_data::records::dude::Dude;
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let arsenal = Arsenal::read(&data);
    let blaster = *arsenal.weapon(WeaponId(128)).expect("the Light Blaster");
    let traders: BTreeSet<ShipId> = data
        .records::<Dude>()
        .filter_map(|(_, dude)| dude.ok())
        .filter(|dude| matches!(dude.record.ai_type, 1 | 2))
        .flat_map(|dude| dude.record.ship_type.into_iter().flatten())
        .collect();
    assert!(traders.len() > 5, "{traders:?}");
    for ship in traders {
        let mut combat = Combat::default();
        let mut ships = [shooter(&blaster), target(&data, &arsenal, ship, 100.0)];
        let mut ticks = 0;
        while ships[1].condition == Condition::Intact {
            fight(&mut combat, &mut ships);
            ticks += 1;
            assert!(ticks < 100_000, "shïp {} never disabled", ship.0);
        }
        ships[0].trigger = Trigger::default();
        for _ in 0..30 {
            fight(&mut combat, &mut ships);
        }
        let reserves = ships[1].reserves;
        assert_eq!(ships[1].condition, Condition::Disabled, "shïp {}", ship.0);
        assert!(reserves.armor.now > 0.0, "shïp {}: {reserves:?}", ship.0);
    }
}

// Guided weapons, turrets and point defence over the stock data.

use nova_sim::combat::aim::{BLIND_REAR, BLIND_SIDES};
use nova_sim::combat::defence::{engagement_range, pd_damage};
use nova_sim::flight::facing;

/// NPC 0, the target.
const QUARRY: ShipRef = ShipRef::Npc(nova_sim::NpcId(0));

/// Every stock weapon of `guidance`, by ID.
fn guided(data: &GameData, guidance: &[Guidance]) -> Vec<WeaponSpec> {
    data.weapons()
        .iter()
        .map(WeaponSpec::new)
        .filter(|spec| guidance.contains(&spec.guidance))
        .collect()
}

/// A ship that soaks up any damage, NPC 0 at (`x`, `y`), at rest.
fn sponge(x: f32, y: f32) -> Combatant {
    Combatant {
        id: QUARRY,
        ship_type: ShipId(128),
        target: None,
        state: ShipState {
            position: Vec2::new(x, y),
            ..ShipState::default()
        },
        hull: HullSpec::default(),
        trigger: Trigger::default(),
        reserves: Reserves::full(100_000.0, 100_000.0, 0.0),
        condition: Condition::Intact,
        armament: Armament::default(),
        rounds: BTreeMap::new(),
    }
}

/// `shooter(weapon)` targeting NPC 0.
fn aiming(weapon: &WeaponSpec) -> Combatant {
    Combatant {
        target: Some(QUARRY),
        ..shooter(weapon)
    }
}

/// Each stock homing weapon, fired at a stock ship dead ahead within its
/// range, flies at `Speed`/100 and hits it; fired at a ship that then
/// moves 90 degrees off, it flies straight for 15 ticks, then turns
/// `GuidedTurn`/10 degrees a tick.
#[test]
fn every_stock_homing_weapon_flies_its_speed_hits_its_target_and_turns_its_turn() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let arsenal = Arsenal::read(&data);
    let homing = guided(&data, &[Guidance::Homing]);
    let ids: Vec<i16> = homing.iter().map(|spec| spec.id.0).collect();
    assert_eq!(
        ids,
        [
            134, 135, 136, 137, 148, 160, 182, 184, 185, 199, 229, 230, 234
        ]
    );
    for spec in homing {
        let id = spec.id.0;
        let ahead = (spec.range() / 2.0).min(300.0);
        let mut combat = Combat::default();
        let mut ships = [aiming(&spec), target(&data, &arsenal, ShipId(128), ahead)];
        let full = ships[1].reserves;
        fight_with(&mut combat, &mut ships, &arsenal);
        ships[0].trigger = Trigger::default();
        let speed = combat.shots()[0].velocity.length();
        assert!((speed - spec.speed).abs() < 1e-3, "{id}: {speed}");
        for _ in 0..spec.lifetime {
            fight_with(&mut combat, &mut ships, &arsenal);
        }
        assert_ne!(ships[1].reserves, full, "wëap {id} hit");

        let mut combat = Combat::default();
        let mut ships = [aiming(&spec), sponge(5000.0, 0.0)];
        fight_with(&mut combat, &mut ships, &arsenal);
        ships[0].trigger = Trigger::default();
        ships[1].state.position = Vec2::new(0.0, 5000.0);
        let mut headings = vec![combat.shots()[0].heading];
        for _ in 0..17 {
            fight_with(&mut combat, &mut ships, &arsenal);
            headings.push(combat.shots()[0].heading);
        }
        assert!(
            headings[..15].iter().all(|&h| (h - 90.0).abs() < 1e-6),
            "{id}: {headings:?}"
        );
        for (tick, pair) in headings[14..].windows(2).enumerate() {
            let turned = pair[1] - pair[0];
            assert!(
                (turned - spec.turn).abs() < 1e-3,
                "{id} on {tick}: {headings:?}"
            );
        }
    }
}

/// Whether `spec`, on a ship of `hull`, facing right, fires at a ship
/// 100 pixels away at `bearing`, and which way: a shot's heading, or, for
/// a beam (over within the tick when it lasts a tick), the bearing when
/// it hit that ship.
fn fires_at(spec: &WeaponSpec, hull: HullSpec, bearing: f32) -> Option<f32> {
    let at = facing(bearing) * 100.0;
    let mut combat = Combat::default();
    let mut ships = [
        Combatant {
            hull,
            ..aiming(spec)
        },
        sponge(at.x, at.y),
    ];
    let full = ships[1].reserves;
    fight(&mut combat, &mut ships);
    let fired = combat
        .take_events()
        .iter()
        .any(|event| matches!(event, CombatEvent::Fired { .. }));
    if !fired {
        return None;
    }
    match combat.shots().first() {
        Some(shot) => Some(shot.heading),
        None => Some(if ships[1].reserves == full {
            f32::NAN
        } else {
            bearing
        }),
    }
}

/// Each stock turret (3 and 4) fires at a target astern unless it or its
/// ship is blind to the rear, and at one abeam unless it is blind to the
/// sides (162), which fires ahead.
#[test]
fn every_stock_turret_fires_at_its_target_outside_its_blind_spots() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let arsenal = Arsenal::read(&data);
    let rear_blind_hull = arsenal.hull(ShipId(141));
    assert_eq!(rear_blind_hull.blind_spots, BLIND_REAR, "the Fed Destroyer");
    let turrets = guided(&data, &[Guidance::TurretBeam, Guidance::Turret]);
    assert_eq!(turrets.len(), 17);
    for spec in turrets {
        let id = spec.id.0;
        let astern = fires_at(&spec, HullSpec::default(), 270.0);
        assert_eq!(astern.is_some(), spec.flags & BLIND_REAR == 0, "{id}");
        if let Some(heading) = astern {
            assert!((heading - 270.0).abs() < 1e-3, "{id}: {heading}");
        }
        assert_eq!(fires_at(&spec, rear_blind_hull, 270.0), None, "{id}");
        let abeam = fires_at(&spec, HullSpec::default(), 180.0);
        assert_eq!(abeam.is_some(), spec.flags & BLIND_SIDES == 0, "{id}");
        assert!(
            fires_at(&spec, HullSpec::default(), 90.0).is_some(),
            "{id} ahead"
        );
    }
}

/// Each stock front-quadrant turret (7) fires at a target 30 degrees off
/// its nose at the lead angle, and at one 90 degrees off straight ahead.
#[test]
fn every_stock_front_quadrant_turret_leads_a_target_in_its_arc_and_otherwise_fires_ahead() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let front = guided(&data, &[Guidance::FrontTurret]);
    let ids: Vec<i16> = front.iter().map(|spec| spec.id.0).collect();
    assert_eq!(ids, [143, 145, 155, 156, 157, 158, 183, 200, 219]);
    for spec in front {
        let id = spec.id.0;
        let in_arc = fires_at(&spec, HullSpec::default(), 120.0).expect("fires");
        assert!((in_arc - 120.0).abs() < 1e-3, "{id}: {in_arc}");
        let out = fires_at(&spec, HullSpec::default(), 180.0).expect("fires");
        assert!((out - 90.0).abs() < 1e-3, "{id}: {out}");
    }
}

/// Each stock point-defence turret (9) engages an IR Missile fired at its
/// ship once it is within its engagement range, with no target of its
/// own, and each hit takes its point-defence damage off the missile.
#[test]
fn every_stock_point_defence_turret_engages_an_ir_missile_in_range() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let arsenal = Arsenal::read(&data);
    let ir = *arsenal.weapon(WeaponId(134)).expect("the IR Missile");
    let defences = guided(&data, &[Guidance::PointDefence, Guidance::PointDefenceBeam]);
    let ids: Vec<i16> = defences.iter().map(|spec| spec.id.0).collect();
    assert_eq!(ids, [133, 161]);
    for spec in defences {
        let id = spec.id.0;
        let range = engagement_range(&spec);
        let mut defender = shooter(&spec);
        defender.trigger = Trigger::default();
        let mut attacker = Combatant {
            id: QUARRY,
            target: Some(ShipRef::Player),
            ..shooter(&ir)
        };
        attacker.state = ShipState {
            position: Vec2::new(range + 100.0, 0.0),
            velocity: Vec2::ZERO,
            heading: 270.0,
        };
        let mut combat = Combat::default();
        let mut ships = [defender, attacker];
        let mut engaged = None;
        let mut durability = ir.durability;
        let mut hits = 0;
        for tick in 0..60 {
            let before = combat
                .shots()
                .iter()
                .find(|shot| shot.weapon.id == WeaponId(134))
                .map(|shot| shot.position.length());
            fight_with(&mut combat, &mut ships, &arsenal);
            ships[1].trigger = Trigger::default();
            let fired = combat.take_events().iter().any(|event| {
                matches!(
                    event,
                    CombatEvent::Fired {
                        ship: ShipRef::Player,
                        ..
                    }
                )
            });
            if fired && engaged.is_none() {
                engaged = Some((tick, before));
            }
            match combat
                .shots()
                .iter()
                .find(|shot| shot.weapon.id == WeaponId(134))
            {
                Some(missile) if missile.durability < durability => {
                    assert!(
                        (durability - missile.durability - pd_damage(&spec)).abs() < 1e-3,
                        "{id}"
                    );
                    durability = missile.durability;
                    hits += 1;
                }
                Some(_) => {}
                None => break,
            }
        }
        let (tick, at) = engaged.expect("engaged");
        let at = at.expect("the missile in flight");
        assert!(at <= range, "{id}: engaged at {at} of {range}");
        assert!(at + ir.speed > range, "{id}: not before, on tick {tick}");
        assert!(hits >= 1, "{id}");
        assert!(
            combat
                .shots()
                .iter()
                .all(|shot| shot.weapon.id != WeaponId(134)),
            "{id}: shot down"
        );
        assert_eq!(
            ships[0].reserves.armor,
            Gauge::full(1_000_000.0),
            "{id}: never reached"
        );
    }
}

/// Stock weapons, and the unimplemented flags each reports.
type Row = (&'static [i16], &'static [(FlagField, u16)]);

/// Every stock guided weapon and turret, fired again and again at a ship
/// ahead, reports exactly its unimplemented flags, each once.
#[test]
fn every_stock_guided_weapon_and_turret_reports_its_unimplemented_flags_once() {
    use FlagField::{Flags2, Flags3, Seeker};
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let arsenal = Arsenal::read(&data);
    let weapons = guided(
        &data,
        &[
            Guidance::Homing,
            Guidance::TurretBeam,
            Guidance::Turret,
            Guidance::FrontTurret,
            Guidance::RearTurret,
            Guidance::PointDefence,
            Guidance::PointDefenceBeam,
        ],
    );
    let mut ships: Vec<Combatant> = weapons.iter().map(aiming).collect();
    ships.push(sponge(200.0, 0.0));
    let mut combat = Combat::default();
    let mut diagnostics = Vec::new();
    for _ in 0..200 {
        fight_with(&mut combat, &mut ships, &arsenal);
        diagnostics.extend(combat.take_diagnostics());
    }
    let rows: [Row; 11] = [
        (&[134, 136], &[(Seeker, 0x2)]),
        (&[135, 160], &[(Seeker, 0x2), (Seeker, 0x8)]),
        (&[137], &[(Seeker, 0x1), (Seeker, 0x8)]),
        (&[148, 182, 229], &[(Seeker, 0x1)]),
        (&[184, 185, 199], &[(Seeker, 0x1), (Flags2, 0x4000)]),
        (&[183], &[(Flags2, 0x4000)]),
        (&[142, 201], &[(Flags2, 0x1000), (Flags3, 0x10)]),
        (&[233], &[(Flags2, 0x1000)]),
        (&[234], &[(Seeker, 0x2), (Seeker, 0x8), (Flags2, 0x1000)]),
        (&[196, 197, 198], &[(Flags2, 0x8000)]),
        (&[230], &[(Seeker, 0x2), (Seeker, 0x8), (Flags2, 0x8000)]),
    ];
    let expected: BTreeSet<SimDiagnostic> = rows
        .iter()
        .flat_map(|(weapons, flags)| {
            weapons.iter().flat_map(move |&weapon| {
                flags.iter().map(
                    move |&(field, bit)| SimDiagnostic::UnimplementedWeaponFlag {
                        weapon: WeaponId(weapon),
                        field,
                        bit,
                    },
                )
            })
        })
        .collect();
    assert_eq!(expected.len(), 32);
    assert_eq!(diagnostics.len(), 32, "each once: {diagnostics:?}");
    assert_eq!(diagnostics.into_iter().collect::<BTreeSet<_>>(), expected);
}

/// Every stock weapon that releases sub-munitions, fired alone again and
/// again at a ship ahead, with its sub-munition's weapon mounted nowhere:
/// the sub-munitions released report their unimplemented flags, each
/// once.
#[test]
fn every_stock_sub_munition_reports_its_unimplemented_flags_once_when_released() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let arsenal = Arsenal::read(&data);
    let parents: Vec<WeaponSpec> = data
        .weapons()
        .iter()
        .map(WeaponSpec::new)
        .filter(|spec| spec.submunitions.is_some())
        .collect();
    let released: Vec<(i16, Vec<SimDiagnostic>)> = parents
        .iter()
        .map(|parent| {
            let mut ships = vec![aiming(parent), sponge(200.0, 0.0)];
            let mut combat = Combat::default();
            let mut diagnostics = Vec::new();
            for _ in 0..400 {
                fight_with(&mut combat, &mut ships, &arsenal);
                diagnostics.extend(combat.take_diagnostics());
            }
            diagnostics.retain(|diagnostic| match diagnostic {
                SimDiagnostic::UnimplementedWeaponFlag { weapon, .. }
                | SimDiagnostic::UnimplementedGuidance { weapon, .. } => *weapon != parent.id,
            });
            (parent.id.0, diagnostics)
        })
        .collect();
    let seeker = |weapon| {
        vec![SimDiagnostic::UnimplementedWeaponFlag {
            weapon: WeaponId(weapon),
            field: FlagField::Seeker,
            bit: 0x0001,
        }]
    };
    assert_eq!(
        released,
        [(163, seeker(229)), (182, seeker(148)), (185, seeker(148))]
    );
}

// Combat AI and legal status over the stock data.

use nova_sim::ai::{Goal, NovaAi};
use nova_sim::legal::{Crime, LegalCode, NovaLaw};
use nova_sim::rulebook::RuleSource;
use nova_sim::{Assigned, Assignment, Governments, NovaBoarding, Take, Taken};

const FEDERATION: GovtId = GovtId(128);
const PIRATES: GovtId = GovtId(137);
const CIVVIES: GovtId = GovtId(157);
const MARAUDERS: GovtId = GovtId(178);

/// The Federation and the Civvies are allies, the Federation and the
/// Pirates enemies, the Pirates and the Marauders xenophobes; disabling a
/// Civvies ship costs 3 with the Civvies and the Federation. By the
/// engine's law it also pleases the 20 governments not allied with the
/// Civvies whose own `DisabPenalty` is 2 or more (`gövt` 133, at 15, by
/// 7); by the Bible's it pleases none, for the Civvies' enemies all have a
/// `DisabPenalty` of 0.
#[test]
fn stock_governments_stand_as_the_gövts_say() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let govts = Governments::read(&data);
    assert!(govts.allies(Some(FEDERATION), Some(CIVVIES)));
    assert!(govts.enemies(Some(FEDERATION), Some(PIRATES)));
    assert!(govts.xenophobic(Some(PIRATES)));
    assert!(govts.xenophobic(Some(MARAUDERS)));
    assert!(!govts.xenophobic(Some(FEDERATION)));
    let changes = NovaLaw::default().penalties(Crime::Disable, Some(CIVVIES), &govts);
    assert!(changes.contains(&(CIVVIES, -3)), "{changes:?}");
    assert!(changes.contains(&(FEDERATION, -3)), "{changes:?}");
    let gains = |changes: &[(GovtId, i32)]| -> Vec<(GovtId, i32)> {
        changes
            .iter()
            .copied()
            .filter(|&(_, change)| change > 0)
            .collect()
    };
    let pleased = gains(&changes);
    assert_eq!(pleased.len(), 20, "{pleased:?}");
    assert!(pleased.contains(&(GovtId(133), 7)), "{pleased:?}");
    assert!(pleased.contains(&(GovtId(165), 5)), "{pleased:?}");
    assert!(
        pleased
            .iter()
            .all(|&(govt, _)| !govts.allies(Some(CIVVIES), Some(govt))),
        "{pleased:?}"
    );
    let bible = NovaLaw {
        crime_gains: RuleSource::Bible,
    }
    .penalties(Crime::Disable, Some(CIVVIES), &govts);
    assert_eq!(gains(&bible), [], "{bible:?}");
    let lowered: Vec<(GovtId, i32)> = changes.iter().copied().filter(|&(_, c)| c < 0).collect();
    assert_eq!(bible, lowered, "the same 24 lowered by both");
    assert_eq!(bible.len(), 24);
}

/// A new stock pilot flying the Fed Destroyer (`shïp` 141), in flight at
/// the centre of Fomalhaut (`sÿst` 136), its reserves full.
fn destroyer_in_fomalhaut(data: &GameData) -> Session {
    let pilot = Pilot::new(data, "Stock").expect("the stock first chär starts");
    let mut save: serde_json::Value =
        serde_json::from_str(&nova_sim::save::encode(&pilot)).expect("JSON");
    save["system"] = serde_json::json!(136);
    save["stellar"] = serde_json::Value::Null;
    save["ship"] = serde_json::json!(141);
    save["outfits"] = serde_json::Value::Null;
    for gauge in ["shield", "armor", "fuel"] {
        save["reserves"][gauge]["now"] = serde_json::json!(1_000_000.0);
        save["reserves"][gauge]["max"] = serde_json::json!(1_000_000.0);
    }
    let pilot = nova_sim::save::decode(&save.to_string()).expect("a pilot");
    let session = Session::fly(data, pilot).expect("flies");
    assert_eq!(session.landed(), None, "in flight");
    session
}

/// The NPC numbered `id`, if it is still in the system.
fn npc_of(session: &Session, id: nova_sim::NpcId) -> Option<&nova_sim::Npc> {
    session.npcs().iter().find(|npc| npc.id == id)
}

/// Whether no NPC but `quarry` is within 80 pixels of the line from the
/// player to it, so a shot at it hits nothing else on the way.
fn clear_shot(session: &Session, quarry: &nova_sim::Npc) -> bool {
    let from = session.player().position;
    let line = quarry.state.position - from;
    let length = line.length().max(1.0);
    session
        .npcs()
        .iter()
        .filter(|npc| npc.id != quarry.id && npc.condition.hittable())
        .all(|npc| {
            let off = npc.state.position - from;
            let along = ((off.x * line.x + off.y * line.y) / length).clamp(0.0, length);
            let nearest = from + line * (along / length);
            (npc.state.position - nearest).length() > 80.0
        })
}

/// The player's controls to face `quarry` and close in, and whether to
/// fire: facing it, with a clear shot and none of its own shots in
/// flight.
fn closing_on(session: &Session, quarry: &nova_sim::Npc) -> (nova_sim::Controls, bool) {
    let player = *session.player();
    let off = quarry.state.position - player.position;
    let turn = nova_sim::flight::shortest_turn(player.heading, nova_sim::flight::heading_of(off));
    let controls = nova_sim::Controls {
        thrust: turn.abs() < 10.0 && off.length() > 150.0,
        turn: if turn > 2.0 {
            nova_sim::Turn::Right
        } else if turn < -2.0 {
            nova_sim::Turn::Left
        } else {
            nova_sim::Turn::None
        },
        reverse: false,
    };
    let quiet = session
        .shots()
        .iter()
        .all(|shot| shot.firer != ShipRef::Player);
    (
        controls,
        turn.abs() < 5.0 && quiet && clear_shot(session, quarry),
    )
}

/// The Fomalhaut fight: the session, the trader attacked and the
/// Federation ship that came, the dice, and what was seen.
struct Fomalhaut {
    session: Session,
    trader: nova_sim::NpcId,
    police: nova_sim::NpcId,
    chance: Seeded,
    fled: bool,
    answered: bool,
    disabled: bool,
}

/// The Done scenario up to the disabling: in Fomalhaut, where a Civvies
/// trader (`düde` 129, AI 1) and a Lone Federation Ship (`düde` 128, AI
/// 4) fly, the player picks a seed that brings a trader of 1 to 6 crew
/// (so it can be boarded, and the Destroyer's 50 crew cap the odds) and
/// the Federation, attacks the trader until it is disabled and the
/// Federation answers, then holds its fire for 30 ticks.
fn trader_disabled_in_fomalhaut(data: &GameData) -> Fomalhaut {
    let crews: BTreeMap<ShipId, i16> = PilotCatalog::ships(data)
        .into_iter()
        .map(|record| (record.id, record.crew))
        .collect();
    let mut session = destroyer_in_fomalhaut(data);
    let mut chance = Seeded(0x5EED_F00D);
    let (trader, police) = (1..=200)
        .find_map(|seed| {
            session.populate(data, &mut Seeded(seed));
            let find = |govt, wanted: &[nova_sim::AiType]| {
                session
                    .npcs()
                    .iter()
                    .find(|npc| {
                        npc.govt == Some(govt)
                            && wanted.contains(&npc.ai_type)
                            && (govt != CIVVIES
                                || (1..=6).contains(&crews.get(&npc.ship).copied().unwrap_or(0)))
                    })
                    .map(|npc| npc.id)
            };
            let trader = find(CIVVIES, &[nova_sim::AiType::WimpyTrader])?;
            let police = find(
                FEDERATION,
                &[nova_sim::AiType::Warship, nova_sim::AiType::Interceptor],
            )?;
            Some((trader, police))
        })
        .expect("a seed brings a Civvies trader with a crew and the Federation");
    let (mut fled, mut answered, mut disabled) = (false, false, false);
    for _ in 0..3000 {
        let Some(quarry) = npc_of(&session, trader) else {
            break;
        };
        let (controls, fire) = closing_on(&session, quarry);
        disabled |= quarry.condition == Condition::Disabled;
        session.hold_fire(fire && !disabled, false);
        session.tick(controls);
        session.tick_combat(nova_sim::Rules::default(), &mut chance);
        session.tick_traffic(data, &NovaAi::default(), &mut chance);
        fled |= npc_of(&session, trader).is_some_and(|npc| npc.goal == Goal::Flee(ShipRef::Player));
        answered |= session
            .npcs()
            .iter()
            .any(|npc| npc.govt == Some(FEDERATION) && npc.goal == Goal::Attack(ShipRef::Player));
        if disabled && answered {
            break;
        }
    }
    for _ in 0..30 {
        session.hold_fire(false, false);
        session.tick(nova_sim::Controls::default());
        session.tick_combat(nova_sim::Rules::default(), &mut chance);
    }
    Fomalhaut {
        session,
        trader,
        police,
        chance,
        fled,
        answered,
        disabled,
    }
}

/// The Done scenario, short of boarding: the player's attack on the
/// trader puts it to flight and brings the Federation down on the player;
/// disabling the trader costs 3 with the Civvies and with the Federation,
/// and it is left alive. By the engine's law (the default) the record
/// also changes with the Civvies' other allies (down 3) and rises with
/// the 20 governments not allied with them that it pleases.
#[test]
fn attacking_a_stock_trader_puts_it_to_flight_brings_the_police_and_costs_3() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let all: Vec<GovtId> = Governments::read(&data).ids().collect();
    let records = |session: &Session| -> Vec<i16> {
        all.iter()
            .map(|&govt| session.pilot().legal_record(govt))
            .collect()
    };
    let before_all = records(&destroyer_in_fomalhaut(&data));
    let fight = trader_disabled_in_fomalhaut(&data);
    assert!(fight.fled, "the trader fled from the player");
    assert!(
        fight.answered,
        "the Federation (NPC {}) attacked the player",
        fight.police.0
    );
    assert!(fight.disabled, "the trader was disabled");
    let session = &fight.session;
    assert!(npc_of(session, fight.trader).is_some(), "not destroyed");
    let after = [CIVVIES, FEDERATION].map(|govt| session.pilot().legal_record(govt));
    assert_eq!(after, [-3, -3]);
    let changed: Vec<(GovtId, i32)> = all
        .iter()
        .zip(records(session).into_iter().zip(before_all))
        .filter(|(_, (after, before))| after != before)
        .map(|(&govt, (after, before))| (govt, i32::from(after) - i32::from(before)))
        .collect();
    assert_eq!(changed.len(), 44, "{changed:?}");
    assert!(changed.contains(&(GovtId(133), 7)), "{changed:?}");
    assert_eq!(
        changed,
        NovaLaw::default().penalties(Crime::Disable, Some(CIVVIES), &Governments::read(&data))
    );
}

/// The player's controls to come over `quarry` at its velocity, closing
/// in more slowly the nearer it is, then to face its heading or the
/// reverse, whichever is nearer.
fn boarding(session: &Session, quarry: &nova_sim::Npc) -> nova_sim::Controls {
    use nova_sim::flight::{heading_of, shortest_turn};
    let player = *session.player();
    let accel = session.handling().accel;
    let off = quarry.state.position - player.position;
    let distance = off.length();
    let wanted = if distance > 0.0 {
        off * ((distance * 0.02).min(1.5) / distance)
    } else {
        nova_sim::Vec2::ZERO
    };
    let error = wanted - (player.velocity - quarry.state.velocity);
    let toward = |heading: f32| {
        let turn = shortest_turn(player.heading, heading);
        let side = if turn > 1.5 {
            nova_sim::Turn::Right
        } else if turn < -1.5 {
            nova_sim::Turn::Left
        } else {
            nova_sim::Turn::None
        };
        (turn, side)
    };
    if error.length() > accel || distance > quarry.hull.board_reach / 3.0 {
        let (turn, side) = toward(heading_of(error));
        return nova_sim::Controls {
            thrust: turn.abs() < 10.0 && error.length() > accel / 2.0,
            turn: side,
            reverse: false,
        };
    }
    let ahead = toward(quarry.state.heading);
    let back = toward((quarry.state.heading + 180.0) % 360.0);
    let (turn, side) = if ahead.0.abs() <= back.0.abs() {
        ahead
    } else {
        back
    };
    nova_sim::Controls {
        turn: if turn.abs() <= 20.0 {
            nova_sim::Turn::None
        } else {
            side
        },
        ..nova_sim::Controls::default()
    }
}

/// The Done scenario, to the end: once the trader is disabled, the player
/// flies over it and boards it, which costs 5 more with the Civvies and
/// with the Federation (the Civvies' `BoardPenalty`); its plunder offers
/// at least 1000 credits and some cargo, and the credits taken go to the
/// cash; then, at odds of 75 (the Destroyer's crew of 50 against a
/// trader's), the player captures it into the fleet, and the pilot saves
/// and opens again as it is.
#[test]
fn attacking_a_stock_trader_then_boarding_and_capturing_it() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let mut fight = trader_disabled_in_fomalhaut(&data);
    assert!(fight.fled, "the trader fled from the player");
    assert!(
        fight.answered,
        "the Federation (NPC {}) attacked the player",
        fight.police.0
    );
    assert!(fight.disabled, "the trader was disabled");
    let session = &mut fight.session;
    session.select_target(nova_sim::TargetPick::Nearest);
    while session.target().is_some_and(|npc| npc.id != fight.trader) {
        session.select_target(nova_sim::TargetPick::Next);
    }
    assert_eq!(session.target().map(|npc| npc.id), Some(fight.trader));
    let class = session.target().map(|npc| npc.ship).expect("targeted");
    let mut opened = None;
    for _ in 0..3000 {
        let tried = session.board(
            &NovaLaw::default(),
            &NovaBoarding::default(),
            &mut fight.chance,
        );
        if let Ok(boarding) = tried {
            opened = Some(boarding);
            break;
        }
        let Some(quarry) = npc_of(session, fight.trader) else {
            break;
        };
        let controls = boarding(session, quarry);
        session.tick(controls);
        session.tick_traffic(&data, &NovaAi::default(), &mut fight.chance);
    }
    let Some(nova_sim::Boarding::Opened(view)) = opened else {
        panic!("boarded: {opened:?}");
    };
    assert_eq!(
        [CIVVIES, FEDERATION].map(|govt| session.pilot().legal_record(govt)),
        [-8, -8],
        "3 for the disabling, 5 for the boarding"
    );
    assert!(view.credits >= 1000, "{view:?}");
    assert!(view.cargo.is_some(), "{view:?}");
    assert_eq!(view.odds, 75);
    let cash = session.pilot().cash();
    assert_eq!(
        session.plunder(Take::Credits, &NovaBoarding::default(), &mut fight.chance),
        Taken::Credits(view.credits)
    );
    assert_eq!(session.pilot().cash(), cash + view.credits);
    // No self-destruct (99), the capture (1 against 75), and not the
    // capture's 1 in 10 (1).
    let mut capture = Script([99, 1, 1].into());
    assert_eq!(
        session.plunder(Take::Capture, &NovaBoarding::default(), &mut capture),
        Taken::Captured
    );
    assert_eq!(
        session.assign(Assignment::Escort, &mut fight.chance),
        Some(Assigned::Escort)
    );
    assert_eq!(
        session
            .pilot()
            .escorts()
            .iter()
            .map(|escort| escort.ship)
            .collect::<Vec<_>>(),
        [class]
    );
    let saved = nova_sim::save::decode(&nova_sim::save::encode(session.pilot())).expect("reads");
    assert_eq!(&saved, session.pilot());
    // The trader flies with the player as its escort, through a jump.
    assert!(session.is_escort(fight.trader), "it joined where it was");
    let next = first_link(&data, 136);
    session.plot_course(next).expect("a route");
    let mut tries = 0;
    while session.player().position.length() < session.stats().jump_distance {
        session.tick(nova_sim::Controls {
            thrust: true,
            ..nova_sim::Controls::default()
        });
        session.tick_traffic(&data, &NovaAi::default(), &mut fight.chance);
        tries += 1;
        assert!(tries < 10_000, "never got out");
    }
    session.begin_jump().expect("jumps");
    finish_pre_jump(session);
    assert_eq!(session.arrive(&data, &mut fight.chance), Some(next));
    let arrived: Vec<_> = session
        .npcs()
        .iter()
        .filter(|npc| npc.escort.is_some())
        .collect();
    assert_eq!(arrived.len(), 1, "the trader came along");
    assert_eq!(arrived[0].ship, class);
    assert!(session.is_escort(arrived[0].id));
    let off = arrived[0].state.position - session.player().position;
    assert!(off.length() < 100.0, "beside the player: {off:?}");
}

/// The stock ships' escort classes: the Shuttle (`shïp` 128) a freighter,
/// the Starbridge (133) a medium ship, the Lightning (135) a fighter and
/// the Fed Destroyer (141) a warship.
#[test]
fn stock_ships_have_their_escort_classes() {
    use nova_sim::escort::EscortClass;
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let ships = PilotCatalog::ships(&data);
    let class = |id: i16| {
        let record = ships
            .iter()
            .find(|record| record.id == ShipId(id))
            .expect("a stock shïp");
        EscortClass::of(record.escort_type, record.inherent_ai, record.fields.mass)
    };
    assert_eq!(
        [128, 133, 135, 141].map(class),
        [
            EscortClass::Freighter,
            EscortClass::Medium,
            EscortClass::Fighter,
            EscortClass::Warship
        ]
    );
    assert!(
        ships
            .iter()
            .all(|record| (0..=3).contains(&record.escort_type)),
        "every stock shïp names its class"
    );
}

/// The stock escort strings: `STR#` 2002's menu, command and message
/// strings, `STR#` 3000's opening to an escort, `STR#` 3001's farewell
/// and `STR#` 150's Release.
#[test]
fn stock_escort_strings_are_where_escorts_read_them() {
    use nova_sim::escort::{ESCORT_COMMANDS, NEW_ORDERS, NO_ESCORTS, WILL_ATTACK, order_label};
    use nova_sim::{EscortCommand, EscortGroup, EscortOrder};
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let messages = nova_sim::CommCatalog::string_list(&data, 2002);
    let commands = [
        EscortCommand::Recall,
        EscortCommand::Hold,
        EscortCommand::Defend,
        EscortCommand::Attack,
    ];
    let orders = [
        Some(EscortOrder::Defend),
        Some(EscortOrder::Attack),
        Some(EscortOrder::Hold),
    ];
    let strings = [(51, NO_ESCORTS), (133, ESCORT_COMMANDS), (134, NEW_ORDERS)]
        .into_iter()
        .chain(
            [139, 135, 136, 137, 138]
                .into_iter()
                .zip(EscortGroup::ALL.map(EscortGroup::message_form)),
        )
        .chain(
            [144, 140, 141, 142, 143]
                .into_iter()
                .zip(EscortGroup::ALL.map(EscortGroup::menu_label)),
        )
        .chain([145, 146, 147].into_iter().zip(orders.map(order_label)))
        .chain([(149, order_label(None)), (154, WILL_ATTACK)])
        .chain(
            [156, 157, 158, 159]
                .into_iter()
                .zip(commands.map(EscortCommand::doing)),
        );
    for (n, text) in strings {
        assert_eq!(messages[n - 1], text, "STR# 2002 #{n}");
    }
    assert_eq!(
        nova_sim::CommCatalog::string_list(&data, 3000)[20],
        "What can I do for you?"
    );
    assert_eq!(
        nova_sim::CommCatalog::string_list(&data, 3001)[0],
        "Goodbye, captain."
    );
    assert_eq!(
        nova_sim::CommCatalog::string_list(&data, 150)[31],
        nova_sim::hail::nova::RELEASE
    );
}

/// The stock `düde` and `shïp` fields boarding reads: the Civvies
/// trader's booty names all six commodities and money (127), and the
/// Cargo Drone (`shïp` 130) has no crew, so it cannot be boarded.
#[test]
fn stock_boarding_reads_the_civvies_booty_and_the_drones_crew() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let dude = nova_sim::TrafficCatalog::dude(&data, nova_sim::DudeId(129)).expect("düde 129");
    assert_eq!(dude.booty, 127);
    let drone = PilotCatalog::ships(&data)
        .into_iter()
        .find(|record| record.id == ShipId(130))
        .expect("shïp 130");
    assert_eq!(drone.crew, 0);
}

// Hailing.

use nova_sim::hail::nova::{BEG_FOR_MERCY, GREETINGS, REQUEST_ASSISTANCE};
use nova_sim::hail::{
    BRIBABLE_TRADERS, BRIBABLE_WARSHIPS, GREEDY, Haggle, HailOptions, QUIET, UNTALKATIVE,
};
use nova_sim::{CommCatalog, Reply, TargetPick};

const HYPERGATE: GovtId = GovtId(183);

/// The stock ship comm strings, hail lines and button labels hailing
/// reads: `STR#` 3000's first, "Nice to meet you." (group 9) and last
/// strings, the Federation's first hail line, and `STR#` 150's labels.
#[test]
fn stock_hail_strings_are_where_hailing_reads_them() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let comm = data.string_list(3000);
    assert_eq!(comm[0], "Channel open.");
    assert_eq!(comm[45], "Nice to meet you.");
    assert_eq!(comm[185], "Goodbye, captain.");
    assert_eq!(
        data.string_list(7000)[0],
        "We wish only to help.  If you need anything, just call us."
    );
    let labels = data.string_list(150);
    assert_eq!(labels[20], "Close Channel");
    assert_eq!(labels[21], GREETINGS);
    assert_eq!(labels[22], REQUEST_ASSISTANCE);
    assert_eq!(labels[24], BEG_FOR_MERCY);
}

/// The three stock advice lines exactly 42 characters long read "Nice to
/// meet you." in the conversation's variant by the engine, and are shown
/// as written by the other reading of `long_advice`.
#[test]
fn the_three_stock_42_character_advice_lines_follow_long_advice() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let comm = data.string_list(3000);
    for (list, index) in [(7009, 8), (7018, 4), (7041, 1)] {
        let line = data.string_list(list)[usize::from(index) - 1].clone();
        assert_eq!(line.chars().count(), 42, "{line:?}");
        for variant in 0..5 {
            let advice = |long_advice| Reply::Advice {
                list,
                index,
                greet_if_blank: false,
                long_advice,
            };
            assert_eq!(
                advice(RuleSource::Engine).say(variant, &data),
                comm[45 + usize::from(variant)],
                "STR# {list} #{index}"
            );
            assert_eq!(advice(RuleSource::Bible).say(variant, &data), line);
        }
    }
}

/// The stock governments' flags hailing reads: the Federation (`Flags`
/// 0xE2B0) is greedy and its warships and traders take bribes, the
/// pirates' warships take them, and the Hypergates are quiet but
/// talkative.
#[test]
fn stock_governments_hail_as_their_flags_say() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let govts = Governments::read(&data);
    assert_eq!(govts.get(FEDERATION).map(|govt| govt.flags), Some(0xE2B0));
    assert!(govts.flag(Some(FEDERATION), GREEDY));
    assert!(govts.flag(Some(FEDERATION), BRIBABLE_WARSHIPS));
    assert!(govts.flag(Some(FEDERATION), BRIBABLE_TRADERS));
    assert!(govts.flag(Some(PIRATES), BRIBABLE_WARSHIPS));
    assert!(govts.flag2(Some(HYPERGATE), QUIET));
    assert!(!govts.flag2(Some(HYPERGATE), UNTALKATIVE));
}

/// A new stock pilot flying the Fed Destroyer in flight at the centre of
/// Fomalhaut, its record with the Federation `record`, and the
/// Federation warship or interceptor (of some aggression) that some seed
/// brings, targeted.
fn hailing_the_federation(data: &GameData, record: i16) -> (Session, nova_sim::NpcId) {
    let mut session = destroyer_in_fomalhaut(data);
    let mut save: serde_json::Value =
        serde_json::from_str(&nova_sim::save::encode(session.pilot())).expect("JSON");
    save["legal"] = serde_json::json!([{ "govt": 128, "record": record }]);
    let pilot = nova_sim::save::decode(&save.to_string()).expect("a pilot");
    session = Session::fly(data, pilot).expect("flies");
    let fed = (1..=200)
        .find_map(|seed| {
            session.populate(data, &mut Seeded(seed));
            session
                .npcs()
                .iter()
                .find(|npc| {
                    npc.govt == Some(FEDERATION) && npc.aggression > 0 && !npc.ai_type.trades()
                })
                .map(|npc| npc.id)
        })
        .expect("a seed brings the Federation");
    for _ in 0..=session.npcs().len() {
        if session.select_target(TargetPick::Next) == Some(fed) {
            return (session, fed);
        }
    }
    panic!("never targeted NPC {}", fed.0);
}

/// In Fomalhaut, a Federation ship hailed by a pilot of clean record
/// opens with group 0 ("Channel open." or one of its variants) and greets
/// with one of the Federation's hail lines.
#[test]
fn a_federation_ship_hailed_in_fomalhaut_greets_with_its_governments_line() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let (mut session, fed) = hailing_the_federation(&data, 0);
    let options = HailOptions::default();
    let mut chance = Seeded(7);
    let view = session
        .hail(&data, &options, &mut chance)
        .expect("answered");
    assert_eq!(view.npc, fed);
    let channel_open = &data.string_list(3000)[..5];
    assert_eq!(channel_open[0], "Channel open.");
    assert!(channel_open.contains(&view.reply), "{:?}", view.reply);
    assert_eq!(view.govt_name.as_deref(), Some("Federation"));
    assert_eq!(view.options[0].label, GREETINGS);
    assert_eq!(view.options[1].label, REQUEST_ASSISTANCE);
    let view = session
        .answer(0, &data, &options, &mut chance)
        .expect("open");
    assert!(
        data.string_list(7000).contains(&view.reply),
        "{:?}",
        view.reply
    );
}

/// A wanted pilot hailing the Federation warship hunting it in Fomalhaut
/// can beg for mercy at a price of whole thousands from 1000 to 20,000;
/// paid, the ship does not attack it over the next 300 ticks.
#[test]
fn a_wanted_pilot_buys_off_a_hunting_federation_ship() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let (mut session, fed) = hailing_the_federation(&data, -100);
    let mut chance = Seeded(0x5EED);
    let hunting = (0..600).any(|_| {
        session.tick_traffic(&data, &NovaAi::default(), &mut chance);
        npc_of(&session, fed).is_some_and(|npc| npc.goal == Goal::Attack(ShipRef::Player))
    });
    assert!(hunting, "the Federation hunts the wanted pilot");
    let options = HailOptions::default();
    let view = session
        .hail(&data, &options, &mut chance)
        .expect("answered");
    let labels: Vec<&str> = view
        .options
        .iter()
        .map(|button| button.label.as_str())
        .collect();
    assert_eq!(labels, [GREETINGS, BEG_FOR_MERCY]);
    let view = session
        .answer(1, &data, &options, &mut chance)
        .expect("open");
    let price = view.asking.expect("a price");
    assert!(
        (1000..=20_000).contains(&price) && price % 1000 == 0,
        "{price}"
    );
    let cash = session.pilot().cash();
    session
        .haggle(Haggle::Accept, &data, &options)
        .expect("open");
    assert_eq!(session.pilot().cash(), cash - price);
    session.hang_up();
    for _ in 0..300 {
        session.tick(nova_sim::Controls::default());
        session.tick_combat(nova_sim::Rules::default(), &mut chance);
        session.tick_traffic(&data, &NovaAi::default(), &mut chance);
        if let Some(npc) = npc_of(&session, fed) {
            assert!(npc.goal.attacking().is_none(), "{:?}", npc.goal);
            assert_ne!(npc.target, Some(ShipRef::Player));
        }
    }
}

/// The stock fighter bays: the Viper Bay (149) carries the Fed Viper
/// (144), four a bay, its rounds the Viper outfit (158); the Fed Carrier
/// (143) starts with 4 Vipers and 2 Anacondas aboard; a Viper docks
/// within 100 pixels and a Thunderhead within 250; every stock fighter
/// holds a jump's fuel; and the Return to Hangar and abandoned-fighter
/// strings are `STR#` 2002's.
#[test]
#[allow(clippy::float_cmp)]
fn stock_fighter_bays_launch_their_fighters_as_the_extracted_rules_say() {
    use nova_sim::bay::{ABANDONED_MANY, ABANDONED_ONE, capacity, dock_window};
    use nova_sim::combat::armament::Arsenal;
    use nova_sim::combat::weapon::{Ammo, Guidance, WeaponSpec};
    use nova_sim::escort::order_label;
    use nova_sim::{CombatCatalog, CommCatalog, EscortCommand, EscortOrder, WeaponId};
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let arsenal = Arsenal::read(&data);
    let viper_bay = *arsenal.weapon(WeaponId(149)).expect("the Viper Bay");
    assert_eq!(viper_bay.guidance, Guidance::FighterBay);
    assert_eq!(viper_bay.carried, Some(ShipId(144)));
    assert_eq!(viper_bay.ammo, Ammo::Rounds(WeaponId(149)));
    assert_eq!((viper_bay.max_ammo, viper_bay.reload), (4, 60.0));
    assert!(viper_bay.secondary());
    let outfits = data.outfits();
    let (_, rounds) = arsenal.npc(ShipId(143), &BTreeMap::new(), &outfits);
    assert_eq!(
        rounds.get(&WeaponId(149)),
        Some(&4),
        "the Fed Carrier's Vipers"
    );
    assert_eq!(rounds.get(&WeaponId(150)), Some(&2), "and Anacondas");
    assert!(Arsenal::ammo_outfits(&outfits).contains(&(WeaponId(149), OutfitId(158))));
    let vipers = outfits
        .iter()
        .find(|record| record.id == OutfitId(158))
        .expect("the Viper outfit");
    assert_eq!(capacity(viper_bay.max_ammo, 1, vipers.max), 4);
    let ships = data.ships();
    let fields = |id: i16| {
        ships
            .iter()
            .find(|record| record.id == ShipId(id))
            .map(|record| record.fields)
            .expect("a stock shïp")
    };
    assert_eq!(dock_window(fields(144).maneuver), 100.0, "the Fed Viper");
    assert_eq!(dock_window(fields(157).maneuver), 250.0, "the Thunderhead");
    let bays: Vec<WeaponSpec> = data
        .weapons()
        .iter()
        .map(WeaponSpec::new)
        .filter(WeaponSpec::is_bay)
        .collect();
    assert!(bays.len() >= 20, "{}", bays.len());
    for bay in &bays {
        let carried = bay.carried.expect("a bay carries a ship");
        assert!(
            fields(carried.0).fuel >= 100,
            "{:?} launches {carried:?}",
            bay.id
        );
    }
    let messages = CommCatalog::string_list(&data, 2002);
    assert_eq!(messages[148 - 1], order_label(Some(EscortOrder::Dock)));
    assert_eq!(messages[155 - 1], EscortCommand::Dock.doing());
    assert_eq!(messages[164 - 1], ABANDONED_ONE);
    assert_eq!(messages[165 - 1], ABANDONED_MANY);
}

use nova_sim::hire::{
    DEFECTED_ONE, DEFECTED_SOME, ESCORT, HIRE_ESCORT, HIRED_ESCORT, HIRING_PRICE, NONE_FOR_HIRE,
    PAY_LABEL, PER_DAY, YOU_HAVE,
};
use nova_sim::{HireTerms, LandingSite, NovaHire};

/// Stellar `id`, as its system's landing sites give it.
fn stellar_site(data: &GameData, id: i16) -> LandingSite {
    data.star_map()
        .into_iter()
        .flat_map(|system| data.landing_sites(system.id))
        .find(|site| site.id == StellarId(id))
        .expect("a stock stellar")
}

/// A chance that always fires.
struct Always;

impl nova_sim::Chance for Always {
    fn fires(&mut self, _percent: u8) -> bool {
        true
    }

    fn below(&mut self, _n: u32) -> u32 {
        0
    }
}

/// The Shuttle (128) is for hire 40 % of days; hired at Viking (tech
/// level 4) its fee is 970, a tenth of its 10,000 `Cost` less the
/// low-tech discount, and at Earth (tech level 7) 1000; its wage is 100 a
/// day. Docked at Viking, with every roll firing, the bar lists it so.
#[test]
fn the_shuttles_fee_and_wage_are_nova_hires() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let ships = data.ships();
    let shuttle = ships
        .iter()
        .find(|record| record.id == ShipId(128))
        .expect("the Shuttle");
    assert_eq!(shuttle.hire_random, 40);
    assert_eq!((shuttle.cost, shuttle.tech_level), (10_000, 3));
    let viking = stellar_site(&data, 157);
    let earth = stellar_site(&data, 128);
    assert_eq!((viking.tech_level, earth.tech_level), (4, 7));
    let nova = NovaHire::default();
    assert_eq!(nova.fee(shuttle, &viking), 970);
    assert_eq!(nova.fee(shuttle, &earth), 1000);
    assert_eq!(nova.wage(shuttle), 100);
    let heavy = ships
        .iter()
        .find(|record| record.id == ShipId(129))
        .expect("the Heavy Shuttle");
    assert_eq!(nova.wage(heavy), 175);

    let mut session = at_viking(&data);
    let list = session
        .escorts_for_hire(&mut Always)
        .expect("Viking has a bar");
    let row = list.row(ShipId(128)).expect("the Shuttle is for hire");
    assert_eq!((row.fee, row.wage), (970, 100));
}

/// The bar's and the hire dialog's descriptions and strings are where
/// hiring reads them.
#[test]
fn the_hire_descriptions_and_strings_are_where_hiring_reads_them() {
    use nova_data::records::desc::Desc;
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    for id in [14_000, 10_029] {
        assert!(matches!(data.get::<Desc>(id), Some(Ok(_))), "dësc {id}");
    }
    assert_eq!(CommCatalog::string_list(&data, 150)[13 - 1], HIRE_ESCORT);
    let messages = CommCatalog::string_list(&data, 2002);
    for (n, text) in [
        (166, HIRED_ESCORT),
        (168, ESCORT),
        (217, YOU_HAVE),
        (224, NONE_FOR_HIRE),
        (228, HIRING_PRICE),
        (267, PER_DAY),
        (297, PAY_LABEL),
        (302, DEFECTED_ONE),
        (303, DEFECTED_SOME),
    ] {
        assert_eq!(messages[n - 1], text, "#{n}");
    }
}

// Persons over the stock data.

use nova_data::records::person::Person;
use nova_sim::person::{ESCAPE_POD, HAIL_QUOTES, NovaPersons, QuoteTags, expand_tags};
use nova_sim::traffic::table::SpawnTable;
use nova_sim::{PersonId, RuleKey, Rulebook, TrafficCatalog};

/// `përs` `id`'s record.
fn person_record(data: &GameData, id: i16) -> nova_sim::PersonRecord {
    data.persons()
        .into_iter()
        .find(|person| person.id == PersonId(id))
        .expect("a stock person")
}

#[test]
fn the_stock_persons_records_read_as_the_bible_lays_them_out() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    assert_eq!(data.records::<Person>().count(), 516);
    let razorback = person_record(&data, 510);
    assert_eq!(razorback.name, "UFS Razorback");
    assert_eq!(razorback.link_syst, 10_000);
    assert_eq!(razorback.ship, Some(ShipId(143)));
    assert_eq!(
        razorback
            .weapons
            .iter()
            .map(|slot| (slot.weapon.0, slot.count))
            .collect::<Vec<_>>(),
        [(132, 2), (133, 2)]
    );
    assert_eq!(
        (razorback.shield_mod, razorback.comm_quote, razorback.flags),
        (250, 24, 0x0003)
    );
    let hunter = person_record(&data, 151);
    assert_eq!(hunter.name, "Bounty Hunter");
    assert_eq!((hunter.flags, hunter.hail_quote), (0x0090, 8));
    let kania = data.system_traffic(nova_sim::SystemId(128)).expect("Kania");
    assert_eq!(kania.persons[0], (Some(PersonId(510)), 50));
    assert_eq!(
        nova_sim::CommCatalog::string_list(&data, 7100)[23],
        "Greetings from the U.F.S. Razorback, the newest and latest capital ship to enter \
         Federation service."
    );
    assert_eq!(
        nova_sim::CommCatalog::string_list(&data, HAIL_QUOTES)[7],
        "<OSN>: Prepare to die, <PN>!"
    );
    let ralph = person_record(&data, 162);
    assert_eq!(
        (ralph.grant_class, ralph.grant_count, ralph.grant_prob),
        (25, 1, 50),
        "Dr Ralph grants one of class 25, half the time"
    );
    let persons = nova_sim::TrafficCatalog::persons(&data);
    assert_eq!(
        persons
            .iter()
            .filter(|person| person.grant_class == 0)
            .count(),
        511
    );
    let outfits = data.outfits();
    let map = outfits
        .iter()
        .find(|outfit| outfit.id == OutfitId(272))
        .expect("Dr Ralph's map");
    assert_eq!(map.item_class, 25);
    assert_eq!(map.lc_name, "Dr Ralph's exploration map");
    assert_eq!(
        outfits
            .iter()
            .filter(|outfit| outfit.item_class == 25)
            .map(|outfit| outfit.id)
            .collect::<Vec<_>>(),
        [OutfitId(272)],
        "the only outfit of class 25"
    );
}

/// Boarding Dr Ralph (`përs` 162) grants his exploration map when
/// `Rand(100)` is 49, within his odds of 50, and nothing at 50.
#[test]
fn boarding_dr_ralph_grants_his_map_half_the_time() {
    use nova_sim::BoardingRule;
    use nova_sim::grant::{GrantStock, Granted, PersonGrant};
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let grant = PersonGrant::of(&person_record(&data, 162)).expect("Dr Ralph grants");
    let pilot = Pilot::new(&data, "Stock").expect("the stock first chär starts");
    let fields = data.ship_fields(pilot.ship()).expect("its ship");
    let free = fields.free_mass;
    let stock: Vec<GrantStock> = data
        .outfits()
        .iter()
        .map(|outfit| GrantStock {
            outfit: outfit.id,
            item_class: outfit.item_class,
            mass: outfit.mass,
            unit_mass: nova_sim::outfitter::unit_mass(outfit, fields.mass),
            owned: 0,
            max: outfit.max,
        })
        .collect();
    let rule = nova_sim::NovaBoarding::default();
    let granted = |odds| {
        let draws: &'static [(u32, u32)] = if odds == 49 {
            &[(100, 49)]
        } else {
            &[(100, 50)]
        };
        rule.grant(
            &grant,
            &stock,
            i64::from(free),
            RuleSource::Engine,
            &mut ByBound(draws),
        )
    };
    assert_eq!(
        granted(49),
        Some(Granted {
            outfit: OutfitId(272),
            count: 1,
        })
    );
    assert_eq!(granted(50), None);
}

/// A new pilot, "Stock", flying in `sÿst` `system`, with `gone` gone for
/// good, its persons appearing as `rules` say.
fn flying_in(data: &GameData, system: i16, gone: &[i16], rules: NovaPersons) -> Session {
    let pilot = Pilot::new(data, "Stock").expect("the stock first chär starts");
    let mut save: serde_json::Value =
        serde_json::from_str(&nova_sim::save::encode(&pilot)).expect("JSON");
    save["system"] = serde_json::json!(system);
    save["stellar"] = serde_json::Value::Null;
    save["gone_persons"] = serde_json::json!(gone);
    let pilot = nova_sim::save::decode(&save.to_string()).expect("a pilot");
    Session::fly(data, pilot)
        .expect("flies")
        .with_person_rules(std::rc::Rc::new(rules))
}

/// The persons `sÿst` `system` is populated with, drawing as `chance`
/// says, by `rules`, with `gone` gone for good.
fn persons_in(
    data: &GameData,
    system: i16,
    gone: &[i16],
    rules: NovaPersons,
    chance: &mut ByBound,
) -> Vec<i16> {
    persons_with_bits(data, system, gone, rules, &[], chance)
}

/// The persons `sÿst` `system` is populated with, as [`persons_in`] says,
/// with the control bits `bits` set.
fn persons_with_bits(
    data: &GameData,
    system: i16,
    gone: &[i16],
    rules: NovaPersons,
    bits: &[u16],
    chance: &mut ByBound,
) -> Vec<i16> {
    let mut session = flying_in(data, system, gone, rules);
    for &bit in bits {
        session.set_control_bit(nova_sim::Bit::new(bit).expect("a bit"), true);
    }
    session.populate(data, chance);
    session
        .npcs()
        .iter()
        .filter_map(|npc| Some(npc.person?.id.0))
        .collect()
}

/// `sÿst` `system`'s spawn table, by the engine.
fn table_of(data: &GameData, system: i16) -> SpawnTable {
    use nova_data::records::system::System;
    let govt = data
        .get::<System>(system)
        .expect("present")
        .expect("decodes")
        .record
        .govt;
    SpawnTable::resolve(
        data,
        nova_sim::SystemId(system),
        govt,
        &nova_sim::Governments::read(data),
        &data.ships(),
        &data.outfits(),
        &nova_sim::combat::armament::Arsenal::read(data),
        &NovaPersons::default(),
        &std::collections::BTreeSet::new(),
    )
}

/// The count of each weapon `kind` carries.
fn counts(kind: &nova_sim::traffic::table::ShipKind) -> BTreeMap<i16, u32> {
    kind.armament
        .mounts()
        .iter()
        .map(|mount| (mount.spec.id.0, mount.count))
        .collect()
}

#[test]
fn kanias_person_slot_brings_the_ufs_razorback_on_its_ship_and_loadout() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    // No person roll fires; the slot's 50 % lists it.
    let mut session = flying_in(&data, 128, &[], NovaPersons::default());
    session.populate(&data, &mut ByBound(&[(7, 1), (100, 49)]));
    let persons: Vec<_> = session
        .npcs()
        .iter()
        .filter(|npc| npc.person.is_some())
        .collect();
    assert_eq!(persons.len(), 1, "{persons:?}");
    let razorback = persons[0];
    assert_eq!(razorback.person.expect("a person").id, PersonId(510));
    assert_eq!(session.npc_name(razorback), Some("UFS Razorback"));
    assert_eq!(razorback.govt, Some(nova_sim::GovtId(128)));
    assert_eq!(razorback.aggression, 4);
    let table = table_of(&data, 128);
    let carrier = &table.ships[&ShipId(143)];
    let mut more = counts(carrier);
    *more.entry(132).or_default() += 2;
    *more.entry(133).or_default() += 2;
    assert_eq!(
        counts(&nova_sim::traffic::table::ShipKind {
            armament: razorback.armament.clone(),
            ..carrier.clone()
        }),
        more
    );
    assert_eq!((more[&132], more[&133]), (4, 4), "with the stock ones");
    assert!((razorback.stats.shield - 2.5 * carrier.stats.shield).abs() < 1e-3);
    assert!((razorback.stats.armor - 2.5 * carrier.stats.armor).abs() < 1e-3);
}

#[test]
fn the_person_roll_brings_the_ufs_razorback_where_the_federation_rules() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let rolled = || ByBound(&[(7, 0), (1022, 382), (100, 99)]);
    let engine = NovaPersons::default();
    assert_eq!(
        persons_in(&data, 128, &[], engine, &mut rolled()),
        [510],
        "Kania"
    );
    assert_eq!(
        persons_in(&data, 130, &[], engine, &mut rolled()),
        [510],
        "Sol"
    );
    assert_eq!(
        persons_in(&data, 260, &[], engine, &mut rolled()),
        Vec::<i16>::new(),
        "J'raphit, Polaris"
    );
}

#[test]
fn jack_folstam_appears_in_nesre_primus_and_by_the_engines_slip_in_jraphit() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let rolled = || ByBound(&[(7, 0), (1022, 3), (100, 99)]);
    let engine = NovaPersons::default();
    // His `ActiveOn` is `b0 & !b8`.
    let active = |system, rules, bits: &[u16]| {
        persons_with_bits(&data, system, &[], rules, bits, &mut rolled())
    };
    assert_eq!(active(132, engine, &[0]), [131]);
    assert_eq!(active(260, engine, &[0]), [131], "the slip");
    let bible = NovaPersons::from_rulebook(
        &Rulebook::default().with_override(RuleKey::LinkSystSlip, RuleSource::Bible),
    );
    assert_eq!(
        active(260, bible, &[0]),
        Vec::<i16>::new(),
        "no slip by the Bible"
    );
    assert_eq!(active(132, engine, &[]), Vec::<i16>::new(), "bit 0 clear");
    assert_eq!(active(132, engine, &[0, 8]), Vec::<i16>::new(), "bit 8 set");
    let jack = person_record(&data, 131);
    let table = table_of(&data, 132);
    let valkyrie = &table.ships[&ShipId(279)];
    let fitted = &table.persons[&PersonId(131)].kind;
    let (before, after) = (counts(valkyrie), counts(fitted));
    let arsenal = nova_sim::combat::armament::Arsenal::read(&data);
    for slot in &jack.weapons {
        let id = slot.weapon.0;
        let was = before.get(&id).copied().unwrap_or(0);
        assert_eq!(
            after.get(&id).copied().unwrap_or(0),
            was + u32::try_from(slot.count).expect("more"),
            "wëap {id}"
        );
        if let Some(nova_sim::combat::weapon::Ammo::Rounds(ammo)) =
            arsenal.weapon(slot.weapon).map(|spec| spec.ammo)
            && slot.ammo > 0
        {
            assert_eq!(
                fitted.rounds.get(&ammo).copied().unwrap_or(0),
                valkyrie.rounds.get(&ammo).copied().unwrap_or(0)
                    + u32::try_from(slot.ammo).expect("more"),
                "the rounds of wëap {id}"
            );
        }
    }
    assert_eq!(
        jack.weapons
            .iter()
            .map(|slot| (slot.weapon.0, slot.count, slot.ammo))
            .collect::<Vec<_>>(),
        [(133, 1, 0), (131, 1, 0), (129, 1, 0), (135, 2, 50)],
        "three guns more and two missile launchers with 50 rounds"
    );
}

#[test]
fn the_ufs_razorback_hailed_greets_with_its_comm_quote() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let mut session = flying_in(&data, 128, &[], NovaPersons::default());
    session.populate(&data, &mut ByBound(&[(7, 1), (100, 49)]));
    let razorback = session
        .npcs()
        .iter()
        .find(|npc| npc.person.is_some())
        .expect("the Razorback")
        .id;
    while session.select_target(nova_sim::TargetPick::Next) != Some(razorback) {}
    let options = nova_sim::HailOptions::default();
    let opened = session
        .hail(&data, &options, &mut ByBound(&[]))
        .expect("answers");
    assert_eq!(opened.reply, "Channel open.");
    let greeted = session
        .answer(0, &data, &options, &mut ByBound(&[]))
        .expect("answers");
    assert_eq!(
        greeted.reply,
        "Greetings from the U.F.S. Razorback, the newest and latest capital ship to enter \
         Federation service."
    );
}

/// Exactly the 78 "Terrapin" persons, whose `LinkMission` 132 has one
/// escort ship, may join the player under `person_join`'s other reading;
/// the Valkyrie's refuel mission and the Razorback's lack of one keep
/// them out.
#[test]
fn only_the_terrapins_may_join_the_player() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let persons = nova_sim::TrafficCatalog::persons(&data);
    let joining: Vec<_> = persons
        .iter()
        .filter(|person| nova_sim::person::offers_to_join(person))
        .collect();
    assert_eq!(joining.len(), 78);
    assert!(
        joining
            .iter()
            .all(|person| person.name == "Terrapin" && person.link_mission == Some(132))
    );
    assert!(nova_sim::person::offers_to_join(&person_record(&data, 128)));
    assert!(!nova_sim::person::offers_to_join(&person_record(
        &data, 225
    )));
    assert!(!nova_sim::person::offers_to_join(&person_record(
        &data, 510
    )));
}

/// Every setup pass's person roll lands on the Terrapin `përs` 128, and
/// the Razorback's Person slot does not list it.
fn terrapin_roll() -> ByBound {
    ByBound(&[(7, 0), (1022, 0), (100, 99)])
}

/// The persons in `session`'s system, with whether each is an escort.
fn persons_here(session: &Session) -> Vec<(i16, bool)> {
    session
        .npcs()
        .iter()
        .filter_map(|npc| Some((npc.person?.id.0, npc.escort.is_some())))
        .collect()
}

/// Hailed in Kania under `person_join`'s other reading, the Terrapin
/// (`përs` 128) lists Use As Escort and joins the fleet as itself; saved
/// and flown again it is placed as the Terrapin, and no other Terrapin
/// comes. By the engine it lists no Use As Escort.
#[test]
fn the_terrapin_hailed_in_kania_joins_the_player_under_person_join() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let mut session = flying_in(&data, 128, &[], NovaPersons::default());
    session.populate(&data, &mut terrapin_roll());
    assert_eq!(persons_here(&session), [(128, false)]);
    let terrapin = session
        .npcs()
        .iter()
        .find(|npc| npc.person.is_some())
        .expect("the Terrapin")
        .id;
    while session.select_target(nova_sim::TargetPick::Next) != Some(terrapin) {}
    let labels = |view: &nova_sim::HailView| {
        view.options
            .iter()
            .map(|button| button.label.clone())
            .collect::<Vec<_>>()
    };
    let engine = nova_sim::HailOptions::default();
    let opened = session
        .hail(&data, &engine, &mut ByBound(&[]))
        .expect("answers");
    assert!(!labels(&opened).contains(&"Use As Escort".to_owned()));
    let bible = nova_sim::HailOptions::nova(
        &nova_sim::Rulebook::default()
            .with_override(nova_sim::RuleKey::PersonJoin, nova_sim::RuleSource::Bible),
    );
    let opened = session
        .hail(&data, &bible, &mut ByBound(&[]))
        .expect("answers");
    let pick = labels(&opened)
        .iter()
        .position(|label| label == "Use As Escort")
        .expect("listed");
    let joined = session
        .answer(pick, &data, &bible, &mut ByBound(&[]))
        .expect("answers");
    assert_eq!(joined.reply, "Okay, I'm on my way.");
    assert_eq!(
        session
            .pilot()
            .escorts()
            .iter()
            .map(|escort| escort.person.map(|person| person.0))
            .collect::<Vec<_>>(),
        [Some(128)]
    );
    session.hang_up();
    let pilot =
        nova_sim::save::decode(&nova_sim::save::encode(session.pilot())).expect("reads again");
    assert_eq!(pilot.escorts()[0].person, Some(PersonId(128)));
    let mut again = Session::fly(&data, pilot).expect("flies");
    again.populate(&data, &mut terrapin_roll());
    assert_eq!(
        persons_here(&again),
        [(128, true)],
        "the escort, and no other Terrapin"
    );
    let escort = again
        .npcs()
        .iter()
        .find(|npc| npc.escort.is_some())
        .expect("placed");
    assert_eq!(again.npc_name(escort), Some("Terrapin"));
}

#[test]
fn the_bounty_hunters_hail_quote_names_him_and_the_pilot() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let line = &nova_sim::CommCatalog::string_list(&data, HAIL_QUOTES)[7];
    let tags = QuoteTags {
        person: "Bounty Hunter",
        pilot: "Stock",
        ship_type: "Shuttle",
    };
    assert_eq!(
        expand_tags(line, &tags),
        "Bounty Hunter: Prepare to die, Stock!"
    );
}

#[test]
fn the_bounty_hunter_gone_for_good_never_appears_again() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    assert_eq!(person_record(&data, 151).flags & ESCAPE_POD, 0, "unique");
    assert_ne!(person_record(&data, 510).flags & ESCAPE_POD, 0);
    let rolled = || ByBound(&[(7, 0), (1022, 23), (100, 99)]);
    let engine = NovaPersons::default();
    let start = Session::start(&data).expect("starts").system().0;
    assert_eq!(persons_in(&data, start, &[], engine, &mut rolled()), [151]);
    for system in [start, 128, 130, 132, 260] {
        assert_eq!(
            persons_in(&data, system, &[151], engine, &mut rolled()),
            Vec::<i16>::new(),
            "sÿst {system}"
        );
    }
}

/// A new stock pilot in Kania, at rest over HG-Kania (`spöb` 1404, at
/// (-70, 250)): docked there through a save that says so, then taken off,
/// which leaves the ship at the gate's centre. Its record with the
/// Hypergate government (183) is 32767, HG-Kania's `MinStatus`: the
/// original never lets a record pass 32767 (task
/// `landing-minstatus-32767-never`), which `check_landing` does not model
/// yet.
fn over_hg_kania(data: &GameData) -> Session {
    let mut pilot = Pilot::new(data, "Stock").expect("the stock first chär starts");
    pilot.set_legal_record(GovtId(183), 32767);
    let mut save: serde_json::Value =
        serde_json::from_str(&nova_sim::save::encode(&pilot)).expect("JSON");
    save["stellar"] = serde_json::json!(1404);
    let parked = nova_sim::save::decode(&save.to_string()).expect("a pilot");
    let mut session = Session::fly(data, parked).expect("flies");
    assert_eq!(session.take_off(), Some(StellarId(1404)));
    assert_eq!(session.player().position, Vec2::new(-70.0, 250.0));
    session
}

/// L, L over HG-Kania enters it, offering Tichel, Dani and Koria; picking
/// Tichel brings the Shuttle out of HG-Tichel at (-400, -500), heading
/// 120° at half its top speed, on the same day and with a full tank.
#[test]
fn hg_kania_leads_to_hg_tichel() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let mut session = over_hg_kania(&data);
    assert_eq!(session.system(), SystemId(128));
    assert_eq!(
        session.land(),
        Ok(LandOutcome::Selected {
            stellar: StellarId(1404),
            station: true,
            clearance: Clearance::Granted,
        }
        .into())
    );
    assert_eq!(
        session.land(),
        Ok(LandPress::AtGate {
            stellar: StellarId(1404),
            kind: GateKind::Hypergate,
        })
    );
    assert_eq!(
        session.open_hypergate(&data),
        Ok(vec![SystemId(129), SystemId(298), SystemId(483)])
    );
    let full = session.reserves().fuel;
    assert_eq!(
        session.enter_hypergate(Some(SystemId(129)), &data, &mut NeverFires),
        Ok(SystemId(129))
    );
    let player = *session.player();
    assert_eq!(player.position, Vec2::new(-400.0, -500.0));
    assert!(
        (player.heading - 120.0).abs() < f32::EPSILON,
        "{}",
        player.heading
    );
    let speed = player.velocity.length();
    assert!(
        (speed - session.handling().max_speed / 2.0).abs() < 1e-4,
        "{speed}"
    );
    assert_eq!(session.date_text(), "June 23, 1177 NC");
    assert_eq!(session.reserves().fuel, full);
    assert_eq!(full, Gauge::full(300.0));
}

/// Ticks `session` through the pre-jump turn and braking until the jump
/// itself begins.
fn finish_pre_jump(session: &mut Session) {
    for _ in 0..10_000 {
        if session.jumping().is_some() {
            return;
        }
        session.tick(nova_sim::Controls::default());
    }
    panic!("never began the jump");
}
