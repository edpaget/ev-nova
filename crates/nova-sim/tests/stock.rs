//! A flight session over the stock data: the first `chär` starts a session
//! with its ship's handling and reserves in a system that exists, Port
//! Kane's exchange trades at its levels, and its outfitter sells what its
//! tech levels allow; Viking's shipyard sells what its tech levels and the
//! ships' `BuyRandom` allow, and trades the Shuttle in; the ships go by
//! their names without the designers' notes. Port Kane sells
//! fuel and uninhabited Reflex-ion sells none. NPC traffic flies the
//! ships and governments its system's `düde`s and fleets give, and
//! Alphara's `DudeTypes` fleet comes when its roll fires. The governments
//! stand as their `gövt`s say, and in Fomalhaut the player's attack on a
//! Civvies trader puts it to flight, brings the Federation down on the
//! player and costs it 3 with each. Skips, passing, when `NOVA_DATA` is
//! unset.

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
            &sites,
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
/// name, and each fleet whose `LinkSyst` matches it.
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
    // Every setup pass draws a person: no traffic yet.
    session.populate(&data, &mut Script(std::collections::VecDeque::new()));
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
    Trigger, Vec2, WeaponId,
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
use nova_sim::{Governments, GovtId};

const FEDERATION: GovtId = GovtId(128);
const PIRATES: GovtId = GovtId(137);
const CIVVIES: GovtId = GovtId(157);
const MARAUDERS: GovtId = GovtId(178);

/// The Federation and the Civvies are allies, the Federation and the
/// Pirates enemies, the Pirates and the Marauders xenophobes; disabling a
/// Civvies ship costs 3 with the Civvies and the Federation.
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
    let changes = NovaLaw.penalties(Crime::Disable, Some(CIVVIES), &govts);
    assert!(changes.contains(&(CIVVIES, -3)), "{changes:?}");
    assert!(changes.contains(&(FEDERATION, -3)), "{changes:?}");
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

/// The Done scenario, short of boarding: in Fomalhaut, where a Civvies
/// trader (`düde` 129, AI 1) and a Lone Federation Ship (`düde` 128, AI
/// 4) fly, the player's attack on the trader puts it to flight and
/// brings the Federation down on the player; disabling the trader costs
/// 3 with the Civvies and with the Federation, and it is left alive.
#[test]
fn attacking_a_stock_trader_puts_it_to_flight_brings_the_police_and_costs_3() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let mut session = destroyer_in_fomalhaut(&data);
    let mut chance = Seeded(0x5EED_F00D);
    let (trader, police) = (1..=200)
        .find_map(|seed| {
            session.populate(&data, &mut Seeded(seed));
            let find = |govt, wanted: &[nova_sim::AiType]| {
                session
                    .npcs()
                    .iter()
                    .find(|npc| npc.govt == Some(govt) && wanted.contains(&npc.ai_type))
                    .map(|npc| npc.id)
            };
            let trader = find(CIVVIES, &[nova_sim::AiType::WimpyTrader])?;
            let police = find(
                FEDERATION,
                &[nova_sim::AiType::Warship, nova_sim::AiType::Interceptor],
            )?;
            Some((trader, police))
        })
        .expect("a seed brings a Civvies trader and the Federation");
    let before = [CIVVIES, FEDERATION].map(|govt| session.pilot().legal_record(govt));
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
        session.tick_traffic(&data, &NovaAi::default(), &mut chance);
        fled |= npc_of(&session, trader).is_some_and(|npc| npc.goal == Goal::Flee(ShipRef::Player));
        answered |= session
            .npcs()
            .iter()
            .any(|npc| npc.govt == Some(FEDERATION) && npc.goal == Goal::Attack(ShipRef::Player));
        if disabled && answered {
            break;
        }
    }
    assert!(fled, "the trader fled from the player");
    assert!(
        answered,
        "the Federation (NPC {}) attacked the player",
        police.0
    );
    assert!(disabled, "the trader was disabled");
    for _ in 0..30 {
        session.hold_fire(false, false);
        session.tick(nova_sim::Controls::default());
        session.tick_combat(nova_sim::Rules::default(), &mut chance);
    }
    assert!(npc_of(&session, trader).is_some(), "not destroyed");
    let after = [CIVVIES, FEDERATION].map(|govt| session.pilot().legal_record(govt));
    assert_eq!(after, before.map(|record| record - 3));
}
