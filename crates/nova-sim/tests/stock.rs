//! A flight session over the stock data: the first `chär` starts a session
//! with its ship's handling and reserves in a system that exists, and a
//! new pilot begins with the bits its blank `OnStart` sets (none); Port
//! Kane's exchange trades at its levels, an event at any stellar may be
//! placed there but not at uninhabited Reflex-ion, and its outfitter
//! sells what its tech levels allow on a day every roll fires, and the
//! Vell-os map only once its control bit is set, which explores the
//! systems around and is not added, and the Cheap Thorium Reactor, whose
//! `OnPurchase` and `OnSell` set and clear its bit; Viking's shipyard
//! sells what its tech levels and the ships' `BuyRandom` allow, and
//! trades the Shuttle in; on a day no roll fires, both sell only their
//! items of `BuyRandom` 100; Sirrusa's outfitter buys back the outfits
//! the player owns, whatever their tech, but not one that can't be sold;
//! the ships go by their names without the designers' notes. Port Kane sells
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
//! again; a Valkyrie hailed offers a new pilot its refuel mission in
//! place of the comm dialog. Skips, passing, when `NOVA_DATA` is unset.

mod common;

use nova_data::GameData;
use nova_data::records::character::Character;
use nova_data::records::ship::Ship;
use nova_data::records::stellar::Stellar;
use nova_sim::Test;
use nova_sim::fuel::FUEL_SCOOP;
use nova_sim::market::commodities;
use nova_sim::{
    Chance, Clearance, Direction, DisasterId, DisasterRecord, GameDate, GateKind, Gauge, Good,
    GovtId, JunkId, LandOutcome, LandPress, LandingRefusal, NeverFires, OutfitId, OutfitMod,
    OutfitOrder, OutfitRefusal, Pilot, PilotCatalog, RechargeRefusal, Service, Session, ShipFields,
    ShipId, ShipState, ShipStats, Standing, StartDate, StellarId, StellarShown, StringPatch,
    SystemId, TypeWidth, Vec2, check_landing, cleared, landing_reach, services,
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
    // The Shuttle carries no default items, and its one outfit, its Light
    // Blaster, changes no stat: its own fields are its stats.
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
            |site| cleared(
                site,
                Standing {
                    record: session.pilot().system_record(session.system()),
                    meets_require: true,
                    ..Standing::default()
                },
                RuleSource::Engine,
            ),
            landing_reach(
                kania,
                StellarShown {
                    frame: true,
                    type_width: TypeWidth::Frame,
                },
                RuleSource::Engine,
            ),
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

/// Buying one of `oütf` `id`.
fn buying(id: i16) -> OutfitOrder {
    OutfitOrder {
        outfit: OutfitId(id),
        direction: Direction::Buy,
    }
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
    // It buys the Ancient Vell-os Sculpture (`jünk` 138, base 500) high,
    // and by the engine sells it there too.
    let sculpture = market.row(Good::Junk(JunkId(138))).expect("listed");
    assert_eq!(
        (sculpture.price, sculpture.sold_here, sculpture.bought_here),
        (625, true, true)
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
    assert_eq!(
        strings.price_patches,
        <[StringPatch; 6]>::default(),
        "stock data has no 'STR '"
    );
    assert_eq!(
        strings.name_patches,
        <[StringPatch; 6]>::default(),
        "stock data has no 'STR '"
    );
    let prices: Vec<i64> = commodities(&strings)
        .into_iter()
        .map(|(_, commodity)| commodity.base_price)
        .collect();
    assert_eq!(prices, [75, 350, 750, 900, 200, 550]);
    let names: Vec<String> = commodities(&strings)
        .into_iter()
        .map(|(_, commodity)| commodity.name)
        .collect();
    assert_eq!(
        names,
        [
            "Food",
            "Industrial",
            "Medical Supplies",
            "Luxury Goods",
            "Metal",
            "Equipment"
        ]
    );
}

/// An `öops` at any stellar may be placed at Port Kane (`spöb` 137), but
/// not at uninhabited Reflex-ion (129).
#[test]
fn an_any_stellar_event_may_be_placed_at_port_kane_but_not_reflex_ion() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let candidates = nova_sim::market::event_stellars(&data.stellar_flags());
    assert!(candidates.contains(&StellarId(137)), "{candidates:?}");
    assert!(!candidates.contains(&StellarId(129)), "{candidates:?}");
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
    let outfitter = at_port_kane(&data)
        .outfitter(&mut Fires)
        .expect("an outfitter");
    let row = |id| outfitter.row(OutfitId(id));
    let battery = row(256).expect("the Battery Pack");
    assert_eq!(
        (battery.name.as_str(), battery.price, battery.mass),
        ("Battery Pack", 10_000, 3)
    );
    assert_eq!(outfitter.can(buying(256)), Ok(()));
    assert_eq!(row(228).map(|r| r.name.as_str()), Some("Solar Panels"));
    let fission = row(179).expect("the Fission Reactor");
    assert_eq!(
        outfitter.can(buying(179)),
        Err(OutfitRefusal::NoSpaceForAny)
    );
    assert!(fission.mass > 8, "{}", fission.mass);
    assert!(row(177).is_none(), "the Fusion Reactor is tech 8");
    assert!(row(180).is_some(), "Carbon Fiber");
    assert_eq!(outfitter.can(buying(180)), Err(OutfitRefusal::NotForSale));
    assert_eq!((outfitter.cash, outfitter.free_mass), (25_000, 8));
}

/// Carbon Fiber (`oütf` 180, `Mass` 1, `Flags` 0x0400) on the Shuttle
/// (`shïp` 128, `Mass` 15): Port Kane's outfitter weighs it at its own
/// ton by the engine (`_AdjustedItemMass` @0x28ba-0x28bc), and at
/// 15 × 1 / 100 = 0 by the other reading of `ScaledMass`.
#[test]
fn carbon_fiber_weighs_a_ton_on_the_shuttle_at_port_kane() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let weighed = |rules: nova_sim::Rulebook| {
        at_port_kane(&data)
            .with_rules(rules)
            .outfitter(&mut Fires)
            .expect("an outfitter")
            .row(OutfitId(180))
            .expect("Carbon Fiber")
            .mass
    };
    assert_eq!(weighed(nova_sim::Rulebook::default()), 1);
    assert_eq!(
        weighed(
            nova_sim::Rulebook::default()
                .with_override(nova_sim::RuleKey::ScaledMass, nova_sim::RuleSource::Bible)
        ),
        0
    );
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
    let outfitter = session.outfitter(&mut Fires).expect("an outfitter");
    assert!(outfitter.row(map).is_none(), "bit 9999 is clear");
    session.set_control_bit(nova_sim::Bit::new(9999).expect("a bit"), true);
    let outfitter = session.outfitter(&mut Fires).expect("an outfitter");
    let row = outfitter.row(map).expect("listed once bit 9999 is set");
    assert_eq!(row.name, "Area Map - Vell-os");
    assert_eq!(outfitter.can(buying(342)), Ok(()));
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
    let bought = |rules: nova_sim::Rulebook| {
        let mut session = at_port_kane(&data).with_rules(rules);
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
    let engine = bought(nova_sim::Rulebook::default());
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
    let bible = bought(
        nova_sim::Rulebook::default()
            .with_override(nova_sim::RuleKey::MapExplore, nova_sim::RuleSource::Bible),
    );
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
    // On a day every roll fires, as its `BuyRandom` is below 100.
    assert_eq!(session.outfit(order(Direction::Buy), &mut Fires), Ok(()));
    assert_eq!(session.pilot().owned(reactor), 1);
    assert!(session.control_bit(bit), "OnPurchase b9011");
    assert_eq!(session.outfit(order(Direction::Sell), &mut Fires), Ok(()));
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
    assert_eq!(session.outfit(battery, &mut Fires), Ok(()));
    let outfitter = session.outfitter(&mut Fires).expect("an outfitter");
    assert_eq!((outfitter.cash, outfitter.free_mass), (15_000, 5));
    assert_eq!(session.pilot().owned(OutfitId(256)), 1);
    assert_eq!(
        session.reserves().fuel,
        Gauge::full(400.0),
        "the new tank comes full"
    );
}

/// The Shuttle (`shïp` 128, `MaxGun` 2) carries one stock Light Blaster
/// (`oütf` 128, flagged a gun). At Port Kane it buys one more and then
/// no gun at all (`_HasMaxOfItem` @0x46c8-0x47b7).
#[test]
fn the_shuttle_at_port_kane_buys_one_more_gun_and_then_none() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let mut session = at_port_kane(&data);
    let blaster = OutfitOrder {
        outfit: OutfitId(128),
        direction: Direction::Buy,
    };
    assert_eq!(session.outfit(blaster, &mut Fires), Ok(()));
    assert_eq!(session.pilot().owned(OutfitId(128)), 2);
    let outfitter = session.outfitter(&mut Fires).expect("an outfitter");
    assert_eq!(outfitter.check(blaster), Err(OutfitRefusal::GunLimit));
    if outfitter.row(OutfitId(270)).is_some() {
        assert_eq!(
            outfitter.can(buying(270)),
            Err(OutfitRefusal::GunLimit),
            "the mining laser"
        );
    }
    assert!(outfitter.row(OutfitId(256)).is_some(), "the Battery Pack");
    assert_eq!(outfitter.can(buying(256)), Ok(()), "not a gun");
}

/// The Viper Bay (`oütf` 157) launches the Viper (`wëap` 149, `MaxAmmo`
/// 4), its own rounds, held by the Viper outfit (158).
#[test]
fn the_viper_bays_magazine_is_its_vipers() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let magazines = nova_sim::combat::armament::Arsenal::read(&data).magazines(&data.outfits());
    let bay = magazines.get(&OutfitId(157)).expect("a launcher");
    assert_eq!(
        (bay.weapon, bay.ammo, bay.max_ammo, bay.ammo_outfit),
        (WeaponId(149), WeaponId(149), 4, Some(OutfitId(158)))
    );
    assert!(bay.carried.is_some(), "it launches a ship");
}

/// A new stock pilot, docked at Viking (`spöb` 157 in Tichel, `sÿst`
/// 129, a jump from Kania), through a save that says so.
fn at_viking(data: &GameData) -> Session {
    at_viking_holding(data, &[])
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
    let mut session = at_viking(&data);
    let shipyard = session.shipyard(&mut Fires).expect("a shipyard");
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
        .filter(|ship| {
            ship.buy_random <= 0 && nova_sim::wares::ship_tech_allows(ship.tech_level, &site)
        })
        .map(|ship| ship.id)
        .collect();
    // Among them the Cargo Drone, `shïp` 130, of tech level 4.
    assert!(never.contains(&ShipId(130)), "{never:?}");
    for id in never {
        assert!(shipyard.row(id).is_none(), "{id:?}");
    }
    assert_eq!(
        shipyard.trade_in, 4700,
        "a quarter of the Shuttle and half its Light Blaster, 5000, through the \
         flux twice at the Shuttle's tech 3 against Viking's 4: 3 % off to 4850, \
         then to 4704, rounded down to 4700"
    );
    assert_eq!(shipyard.cash, 25_000);
    assert_eq!(shipyard.current, ShipId(128));
}

/// On a day no roll fires, Port Kane sells the Battery Pack (`oütf` 256,
/// `BuyRandom` 100) but not the Fission Reactor (179, 75) or Carbon Fiber
/// (180, 90), and Viking sells the Viper (`shïp` 167, 100) but not the
/// Terrapin (136, 90) or the Shuttle (128, 35), though the Shuttle is the
/// ship flown.
#[test]
fn port_kane_and_viking_sell_only_their_always_items_when_no_roll_fires() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let outfitter = at_port_kane(&data)
        .outfitter(&mut NeverFires)
        .expect("an outfitter");
    let listed = |id| outfitter.row(OutfitId(id)).is_some();
    assert!(listed(256), "the Battery Pack");
    assert!(!listed(179), "the Fission Reactor");
    assert!(!listed(180), "Carbon Fiber");
    let shipyard = at_viking(&data)
        .shipyard(&mut NeverFires)
        .expect("a shipyard");
    let listed = |id| shipyard.row(ShipId(id)).is_some();
    assert!(listed(167), "the Viper");
    assert!(!listed(136), "the Terrapin");
    assert!(!listed(128), "the Shuttle");
}

/// Sirrusa (`spöb` 156 in Archenar, `sÿst` 142; tech level 0, `Flags2`
/// 0x0400) lists the Battery Pack (`oütf` 256, tech 3) the player owns,
/// sell-only, but not the Fighter Bay License (259, flagged 0x0008, can't
/// be sold) they own, nor the Solar Panels (228, tech 3) they don't, on a
/// day every roll fires.
#[test]
fn sirrusas_outfitter_buys_back_owned_outfits_but_not_cannot_sell_ones() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let pilot = Pilot::new(&data, "Stock").expect("the stock first chär starts");
    let mut save: serde_json::Value =
        serde_json::from_str(&nova_sim::save::encode(&pilot)).expect("JSON");
    save["system"] = serde_json::json!(142);
    save["stellar"] = serde_json::json!(156);
    let outfits = save["outfits"].as_array_mut().expect("a list");
    outfits.push(serde_json::json!({"outfit": 256, "count": 1}));
    outfits.push(serde_json::json!({"outfit": 259, "count": 1}));
    let docked = nova_sim::save::decode(&save.to_string()).expect("a pilot");
    let mut session = Session::fly(&data, docked).expect("flies");
    assert_eq!(session.landed(), Some(StellarId(156)), "docked at Sirrusa");
    let outfitter = session.outfitter(&mut Fires).expect("an outfitter");
    let order = |id, direction| OutfitOrder {
        outfit: OutfitId(id),
        direction,
    };
    assert!(outfitter.row(OutfitId(256)).is_some(), "the Battery Pack");
    assert_eq!(outfitter.can(order(256, Direction::Sell)), Ok(()));
    assert_eq!(
        outfitter.can(order(256, Direction::Buy)),
        Err(OutfitRefusal::NotForSale)
    );
    assert!(
        outfitter.row(OutfitId(259)).is_none(),
        "the Fighter Bay License"
    );
    assert!(outfitter.row(OutfitId(228)).is_none(), "the Solar Panels");
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

/// Buying the Heavy Shuttle (17,500 credits, tech 4 at Viking's tech 4,
/// so no discount) trades the Shuttle and its Light Blaster in for 4,700
/// (5,000 through the flux twice at the Shuttle's tech 3) and leaves
/// 25,000 + 4,700 - 17,500 = 12,200 credits, 15 tons of cargo
/// space and 12 free, the Heavy Shuttle's own Light Blaster (`oütf` 128,
/// 3 tons) owned and its mass on top of its `FreeMass`.
#[test]
fn a_heavy_shuttle_trades_in_the_shuttle() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let mut session = at_viking(&data);
    let bought = session
        .buy_ship(ShipId(129), "Kestrel", &mut Fires)
        .expect("bought");
    assert_eq!((bought.price, bought.trade_in), (17_500, 4700));
    assert_eq!(session.ship(), ShipId(129));
    assert_eq!(session.pilot().cash(), 12_200);
    assert_eq!(session.capacity(), 15);
    assert_eq!(session.outfitter(&mut Fires).map(|o| o.free_mass), Some(12));
    let fields = data.ship_fields(ShipId(129)).expect("decodes");
    assert_eq!(session.stats(), ShipStats::new(fields, &[]));
    assert_eq!(
        session.pilot().outfits().collect::<Vec<_>>(),
        [(OutfitId(128), 1)]
    );
}

/// A new stock pilot owns its Shuttle's stock weapon, a Light Blaster
/// (`wëap` 128, held by `oütf` 128), as an outfit, and mounts it once.
#[test]
fn a_new_stock_pilot_owns_its_shuttles_light_blaster() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let pilot = Pilot::new(&data, "Stock").expect("the stock first chär starts");
    assert_eq!(pilot.ship(), ShipId(128));
    assert_eq!(pilot.outfits().collect::<Vec<_>>(), [(OutfitId(128), 1)]);
    let session = Session::fly(&data, pilot).expect("flies");
    assert_eq!(session.secondary(), None, "the Light Blaster is a primary");
}

/// The Striker (`shïp` 262) stocks two `BioRelay` Lasers (`oütf` 154,
/// 250,000 each), a Wraith Cannon (151, 200,000) and its 30 Wraithii
/// (152, 1,000 each); its default items are the Nil'kemorya Jammer (245,
/// 125,000) and the Cloaking Organ (269, 1,000,000). A Striker from a
/// version 13 save owns its stock weapons once flown, and trades in for
/// a quarter of its 1,000,000 and half of its 1,855,000 in outfits,
/// 1,177,500, shown 1,177,000 after the shipyard flux's rounding to a
/// thousand. With 29 Wraithii fired it trades in for exactly the
/// original's logged 1,163,000 ("Striker (262) has a trade-in value of
/// 1163000"), the figure its loader's check reaches by counting a
/// Wraithii for each Wraith Cannon (`_LoadObjectData` @0x7aca2).
#[test]
fn the_strikers_stock_weapons_are_its_outfits_and_it_trades_in_as_the_original_logs() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let outfits = data.outfits();
    let ids = |pairs: &[(i16, u16)]| -> std::collections::BTreeMap<OutfitId, u16> {
        pairs.iter().map(|&(id, n)| (OutfitId(id), n)).collect()
    };
    assert_eq!(
        nova_sim::combat::armament::Arsenal::read(&data).stock_outfits(ShipId(262), &outfits),
        ids(&[(151, 1), (152, 30), (154, 2)])
    );
    let pilot = Pilot::new(&data, "Striker").expect("the stock first chär starts");
    let mut save: serde_json::Value =
        serde_json::from_str(&nova_sim::save::encode(&pilot)).expect("JSON");
    save["version"] = serde_json::json!(13);
    save.as_object_mut()
        .expect("an object")
        .remove("stock_weapons_fitted");
    save["system"] = serde_json::json!(129);
    save["stellar"] = serde_json::json!(157);
    save["ship"] = serde_json::json!(262);
    save["outfits"] = serde_json::json!([
        {"outfit": 245, "count": 1},
        {"outfit": 269, "count": 1}
    ]);
    let old = nova_sim::save::decode(&save.to_string()).expect("a version 13 pilot");
    let mut session = Session::fly(&data, old).expect("flies");
    assert_eq!(session.landed(), Some(StellarId(157)), "docked at Viking");
    let owned: std::collections::BTreeMap<OutfitId, u16> = session.pilot().outfits().collect();
    assert_eq!(
        owned,
        ids(&[(151, 1), (152, 30), (154, 2), (245, 1), (269, 1)])
    );
    let shipyard = session.shipyard(&mut Fires).expect("a shipyard");
    // 250,000 + 1,855,000 / 2 = 1,177,500, which the flux rounds down
    // to a thousand (the Striker is tech 19, so no discount, and the
    // second pass changes nothing).
    assert_eq!(shipyard.trade_in, 1_177_000);

    let mut save: serde_json::Value =
        serde_json::from_str(&nova_sim::save::encode(session.pilot())).expect("JSON");
    assert_eq!(save["stock_weapons_fitted"], true);
    let wraithii = save["outfits"]
        .as_array_mut()
        .expect("a list")
        .iter_mut()
        .find(|outfit| outfit["outfit"] == 152)
        .expect("the Wraithii");
    wraithii["count"] = serde_json::json!(1);
    let fired = nova_sim::save::decode(&save.to_string()).expect("a pilot");
    let mut session = Session::fly(&data, fired).expect("flies");
    assert_eq!(session.pilot().owned(OutfitId(152)), 1, "none added");
    let shipyard = session.shipyard(&mut Fires).expect("a shipyard");
    assert_eq!(shipyard.trade_in, 1_163_000);
}

/// `E143` makes a new stock pilot's ship the Fed Carrier (`shïp` 143),
/// adding its stock weapons as outfits: its Viper Bay (`wëap` 149,
/// `AmmoType` 144, load 4) and Anaconda Bay (150, `AmmoType` 145, load 2)
/// as their bay outfits (157 and 159). By the engine each load goes to
/// `wëap` 128 + its `AmmoType`, 272 and 273, which no `oütf` holds, so
/// no Vipers (158) or Anacondas (160) come with them (`_EvalSetExp`
/// @0x1551c-0x155de); by the other reading of `ship_change_bay_ammo` the
/// bays carry their 4 Vipers and 2 Anacondas, as a purchase loads them.
#[test]
fn e_to_the_fed_carrier_loses_its_fighters_by_the_engine() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let changed = |rules: nova_sim::Rulebook| {
        let pilot = Pilot::new(&data, "Stock").expect("the stock first chär starts");
        let mut session = Session::fly(&data, pilot).expect("flies").with_rules(rules);
        let expr = nova_sim::SetExpr::parse("E143").expect("parses");
        session.run_set(&expr, &mut NeverFires);
        assert_eq!(session.ship(), ShipId(143), "the Fed Carrier");
        let owned = |id| session.pilot().owned(OutfitId(id));
        [owned(157), owned(158), owned(159), owned(160)]
    };
    assert_eq!(
        changed(nova_sim::Rulebook::default()),
        [1, 0, 1, 0],
        "the bays, and no fighters"
    );
    let loaded = nova_sim::Rulebook::default().with_override(
        nova_sim::RuleKey::ShipChangeBayAmmo,
        nova_sim::RuleSource::Bible,
    );
    assert_eq!(changed(loaded), [1, 4, 1, 2], "4 Vipers and 2 Anacondas");
}

/// `E241` makes a new stock pilot's ship the Abomination (`shïp` 241),
/// which stocks the Hail Chaingun (`wëap` 155) with a load of 60 and the
/// Storm Chaingun (161) with a load of -1, both firing `wëap` 155's
/// rounds (`AmmoType` 27). By the engine the loads add signed
/// (`_EvalSetExp` @0x155c3), leaving 59 Chaingun Ammunition (`oütf`
/// 170); by the other reading of `ship_change_negative_stock` the -1 adds
/// nothing, leaving 60.
#[test]
fn e_to_the_abomination_nets_its_negative_load_by_the_engine() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let changed = |rules: nova_sim::Rulebook| {
        let pilot = Pilot::new(&data, "Stock").expect("the stock first chär starts");
        let mut session = Session::fly(&data, pilot).expect("flies").with_rules(rules);
        let expr = nova_sim::SetExpr::parse("E241").expect("parses");
        session.run_set(&expr, &mut NeverFires);
        assert_eq!(session.ship(), ShipId(241), "the Abomination");
        session.pilot().owned(OutfitId(170))
    };
    assert_eq!(changed(nova_sim::Rulebook::default()), 59, "60 - 1");
    let unsigned = nova_sim::Rulebook::default().with_override(
        nova_sim::RuleKey::ShipChangeNegativeStock,
        nova_sim::RuleSource::Bible,
    );
    assert_eq!(changed(unsigned), 60, "the -1 adds nothing");
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

use nova_sim::combat::armament::{Arsenal, LauncherRounds, Rounds};
use nova_sim::combat::flags::FlagField;
use nova_sim::combat::impact::TractorLock;
use nova_sim::combat::ion::Ion;
use nova_sim::combat::weapon::{Ammo, Guidance, WeaponSpec};
use nova_sim::combat::{Combat, Fighter, Rules, Shove, Strike};
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
    /// Its class's top speed, its `Speed` alone: what a push soft-caps
    /// against.
    class_speed: f32,
    /// Its top speed: what a push is clamped to.
    top_speed: f32,
    tractor: Option<TractorLock>,
    ion: Ion,
    trigger: Trigger,
    reserves: Reserves,
    condition: Condition,
    armament: Armament,
    rounds: LauncherRounds,
    /// The ship it attacks, which point defence reads; none by default.
    attacking: Option<ShipRef>,
    /// Whether it threatens the player, which its point defence reads;
    /// not by default.
    threatens_player: bool,
}

impl Combatant {
    fn fighter(&mut self) -> Fighter<'_> {
        Fighter {
            ship: self.id,
            ship_type: self.ship_type,
            fleet: self.id,
            govt: None,
            leader: None,
            booty: None,
            state: self.state,
            hull: self.hull,
            shield_regen: 0.0,
            armor_regen: 0.0,
            trigger: self.trigger,
            target: self.target,
            reserves: &mut self.reserves,
            condition: &mut self.condition,
            held_disabled: false,
            cloaked: false,
            attacking: self.attacking,
            threatens_player: self.threatens_player,
            emerging: false,
            cloak_scanner: false,
            has_cloak: false,
            jamming: nova_sim::combat::jamming::Jamming::NONE,
            class_speed: self.class_speed,
            top_speed: self.top_speed,
            jumping: false,
            tractor: &mut self.tractor,
            ion: &mut self.ion,
            armament: &mut self.armament,
            rounds: &mut self.rounds as &mut dyn Rounds,
            drawn_heading: self.state.heading,
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
        nova_sim::Field::default(),
        &mut [],
        &mut Straight,
    );
}

/// The player at the centre, at rest, facing right, firing one of
/// `weapon` with all the ammunition, fuel and armour it could want.
fn shooter(weapon: &WeaponSpec) -> Combatant {
    let mut rounds = LauncherRounds::default();
    if let Ammo::Rounds(ammo) = weapon.ammo {
        rounds.set(ammo, 100_000);
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
        class_speed: 0.0,
        top_speed: 0.0,
        tractor: None,
        ion: Ion::default(),
        trigger: Trigger {
            primary: !weapon.secondary(),
            secondary: weapon.secondary().then_some(weapon.id),
            only: None,
            turrets_only: false,
            bays: false,
            point_defence: false,
            keep: None,
        },
        reserves: Reserves::full(0.0, 1_000_000.0, 1_000_000.0),
        condition: Condition::Intact,
        armament: Armament::new([(*weapon, 1)]),
        rounds,
        attacking: None,
        threatens_player: false,
    }
}

/// What stock ship class `ship` gives a ship of it: its hull, its
/// reserves full as its stock fields and default items give them, its
/// class's top speed (its `Speed` alone) and its top speed.
struct Kind {
    hull: HullSpec,
    reserves: Reserves,
    class_speed: f32,
    top_speed: f32,
}

fn class(data: &GameData, arsenal: &Arsenal, ship: ShipId) -> Kind {
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
    let stats = ShipStats::new(record.fields, &mods);
    Kind {
        hull: arsenal.hull(ship),
        reserves: stats.full(),
        class_speed: record.fields.class_speed(),
        top_speed: stats.handling.max_speed,
    }
}

/// NPC 0, a ship of `ship` type `x` pixels right of the centre, at rest,
/// its reserves full as its stock fields and default items give them, and
/// its class and top speeds its own.
fn target(data: &GameData, arsenal: &Arsenal, ship: ShipId, x: f32) -> Combatant {
    let Kind {
        hull,
        reserves,
        class_speed,
        top_speed,
    } = class(data, arsenal, ship);
    Combatant {
        id: ShipRef::Npc(nova_sim::NpcId(0)),
        ship_type: ship,
        target: None,
        state: ShipState {
            position: Vec2::new(x, 0.0),
            ..ShipState::default()
        },
        hull,
        class_speed,
        top_speed,
        tractor: None,
        ion: Ion::default(),
        trigger: Trigger::default(),
        reserves,
        condition: Condition::Intact,
        armament: Armament::default(),
        rounds: LauncherRounds::default(),
        attacking: None,
        threatens_player: false,
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
/// lives `Count` ticks and flies `Speed`/100 x `Count`, its own reach
/// without its sub-munitions (a beam hits for `Count` ticks, and 15 −
/// `Falloff` more when it decays, and its cone reaches its `BeamLength`
/// and half the ship's cone radius), and it fires as often as item 7 of
/// the plan says.
#[test]
#[allow(clippy::float_cmp)]
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
            // A placeholder hull's 75 pixels: trunc(trunc(0.66 x 75) / 2).
            let reach = spec.beam_length + 24.0;
            let fade = if spec.decay > 0 {
                u32::try_from(15 - i32::from(spec.falloff)).unwrap_or(0)
            } else {
                0
            };
            assert_eq!(beam_hits(&spec, reach), spec.lifetime + fade, "{id}");
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
            assert!((spec.flight() - range).abs() < 1e-3, "{id}");
            if spec.guidance == Guidance::FreefallBomb {
                assert_eq!(spec.range(), 0.0, "{id}: a bomb reaches nothing");
            } else {
                assert!((spec.range() - range).abs() < 1e-3, "{id}: its own flight");
            }
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
            spec.flight().clamp(1.0, 60.0)
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
        class_speed: 0.0,
        top_speed: 0.0,
        tractor: None,
        ion: Ion::default(),
        trigger: Trigger::default(),
        reserves: Reserves::full(100_000.0, 100_000.0, 0.0),
        condition: Condition::Intact,
        armament: Armament::default(),
        rounds: LauncherRounds::default(),
        attacking: None,
        threatens_player: false,
    }
}

/// `shooter(weapon)` targeting NPC 0.
fn aiming(weapon: &WeaponSpec) -> Combatant {
    Combatant {
        target: Some(QUARRY),
        ..shooter(weapon)
    }
}

/// `aiming(weapon)` in a ship of stock class `ship`: its hull, mass
/// included, and its class and top speeds its own, which a tractor beam's
/// branch and its self-pull read; its reserves still the shooter's.
fn aiming_from(data: &GameData, arsenal: &Arsenal, ship: ShipId, weapon: &WeaponSpec) -> Combatant {
    let Kind {
        hull,
        class_speed,
        top_speed,
        ..
    } = class(data, arsenal, ship);
    Combatant {
        ship_type: ship,
        hull,
        class_speed,
        top_speed,
        ..aiming(weapon)
    }
}

/// A tick of `combat` among `ships`, sub-munitions read from `arsenal`,
/// each pushed ship's velocity then set from the fight's shoves as the
/// session sets them: the shoves.
fn fight_shoving(combat: &mut Combat, ships: &mut [Combatant], arsenal: &Arsenal) -> Vec<Shove> {
    fight_with(combat, ships, arsenal);
    let shoves = combat.take_shoves();
    for shove in &shoves {
        if let Some(ship) = ships.iter_mut().find(|ship| ship.id == shove.ship) {
            ship.state.velocity = shove.velocity;
        }
    }
    shoves
}

/// The push of `impact` from `from` on a ship of `mass` at rest at `at`,
/// under its caps: `impact` / `mass` along the whole-degree bearing.
fn push_of(from: Vec2, at: Vec2, impact: f32, mass: f32) -> Vec2 {
    nova_sim::flight::facing(f32::from(nova_sim::trig::intercept_angle(from, at))) * (impact / mass)
}

fn close(a: Vec2, b: Vec2) -> bool {
    (a - b).length() < 1e-5
}

/// The stock data has 55 weapons of positive `Impact` and one tractor
/// beam, 171, and no massless or planet-type ship class: so no stock
/// shot pulls, and every stock ship can be pushed.
#[test]
fn stock_impacts_are_55_pushes_and_one_tractor_beam_on_ships_that_all_move() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let arsenal = Arsenal::read(&data);
    let specs: Vec<WeaponSpec> = data.weapons().iter().map(WeaponSpec::new).collect();
    assert_eq!(specs.iter().filter(|spec| spec.impact > 0.0).count(), 55);
    let pulls: Vec<(i16, f32, Guidance, u32, u32)> = specs
        .iter()
        .filter(|spec| spec.impact < 0.0)
        .map(|spec| {
            (
                spec.id.0,
                spec.impact,
                spec.guidance,
                spec.lifetime,
                spec.decay,
            )
        })
        .collect();
    assert_eq!(
        pulls,
        [(171, -5.0, Guidance::Beam, 1, 0)],
        "one tick of life"
    );
    for record in data.ships() {
        let hull = arsenal.hull(record.id);
        assert!(hull.mass > 0.0, "{:?}", record.id);
        assert!(!hull.planet_type, "{:?}", record.id);
    }
}

/// The EMP Torp. (160), the Polaron Massive Torp. (199) and the Radar
/// Missile (234), homing, hit a Leviathan (`Mass` 10000) dead ahead and
/// push it once by `Impact` / 10000 straight on, their blasts sparing the
/// ship they hit; the EMP Torp.'s 16.7 on a Shuttle (`Mass` 15) is held
/// to the Shuttle's top speed.
#[test]
#[allow(clippy::float_cmp)]
fn stock_torpedoes_push_the_ship_they_hit_by_impact_over_its_mass() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let arsenal = Arsenal::read(&data);
    let spec = |id: i16| {
        data.weapons()
            .iter()
            .map(WeaponSpec::new)
            .find(|spec| spec.id.0 == id)
            .expect("a stock weapon")
    };
    let hit = |spec: &WeaponSpec, ship: i16| {
        let mut combat = Combat::default();
        let ahead = (spec.flight() / 2.0).min(300.0);
        let mut ships = [aiming(spec), target(&data, &arsenal, ShipId(ship), ahead)];
        let mut shoves = fight_shoving(&mut combat, &mut ships, &arsenal);
        ships[0].trigger = Trigger::default();
        for _ in 0..spec.lifetime {
            shoves.extend(fight_shoving(&mut combat, &mut ships, &arsenal));
        }
        (shoves, ships[1].top_speed)
    };
    for (id, impact, blast) in [(160, 250.0, 100.0), (199, 800.0, 20.0), (234, 650.0, 20.0)] {
        let torpedo = spec(id);
        assert_eq!(
            (torpedo.guidance, torpedo.impact),
            (Guidance::Homing, impact)
        );
        assert_eq!(torpedo.blast_radius, blast, "{id}");
        let (shoves, _) = hit(&torpedo, 131);
        assert_eq!(shoves.len(), 1, "{id}: the direct hit alone: {shoves:?}");
        assert_eq!(shoves[0].ship, QUARRY);
        let expected = Vec2::new(impact / 10_000.0, 0.0);
        assert!(close(shoves[0].velocity, expected), "{id}: {shoves:?}");
    }
    let (shoves, top) = hit(&spec(160), 128);
    assert_eq!(shoves.len(), 1, "{shoves:?}");
    assert!(top < 250.0 / 15.0, "{top}");
    assert!(
        close(shoves[0].velocity, Vec2::new(top, 0.0)),
        "held to the Shuttle's top speed {top}: {shoves:?}"
    );
}

/// The Thunderhead Lance (166, `Impact` 50, `Count` 10) held on a
/// Leviathan pushes it on each of its 10 hit ticks, 50 / 10000 a tick,
/// from the beam's start.
#[test]
fn a_stock_beam_pushes_its_ship_on_every_hit_tick() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let arsenal = Arsenal::read(&data);
    let lance = data
        .weapons()
        .iter()
        .map(WeaponSpec::new)
        .find(|spec| spec.id.0 == 166)
        .expect("the Thunderhead Lance");
    assert_eq!(
        (lance.guidance, lance.impact, lance.lifetime, lance.decay),
        (Guidance::Beam, 50.0, 10, 0)
    );
    let mut combat = Combat::default();
    let mut ships = [shooter(&lance), target(&data, &arsenal, ShipId(131), 60.0)];
    let mut shoves = fight_shoving(&mut combat, &mut ships, &arsenal);
    ships[0].trigger = Trigger::default();
    for _ in 0..12 {
        shoves.extend(fight_shoving(&mut combat, &mut ships, &arsenal));
    }
    assert_eq!(shoves.len(), 10, "{shoves:?}");
    assert!(
        close(
            ships[1].state.velocity,
            Vec2::new(10.0 * 50.0 / 10_000.0, 0.0)
        ),
        "{:?}",
        ships[1].state.velocity
    );
}

/// The Flower of Spring (171) fired by a ship of stock class `firer` from
/// the centre facing `heading` at a Shuttle at `at`, for one tick: the
/// shoves, the firer and the Shuttle after it, and the beam's start.
fn flower(
    data: &GameData,
    arsenal: &Arsenal,
    firer: i16,
    heading: f32,
    at: Vec2,
) -> (Vec<Shove>, [Combatant; 2], Vec2) {
    let tractor = data
        .weapons()
        .iter()
        .map(WeaponSpec::new)
        .find(|spec| spec.id.0 == 171)
        .expect("the Flower of Spring");
    let mut ships = [
        aiming_from(data, arsenal, ShipId(firer), &tractor),
        target(data, arsenal, ShipId(128), 0.0),
    ];
    ships[0].state.heading = heading;
    ships[1].state.position = at;
    let mut combat = Combat::default();
    let shoves = fight_shoving(&mut combat, &mut ships, arsenal);
    let start = combat.beams().first().map_or(Vec2::ZERO, |beam| beam.start);
    let hurt = ships[1].reserves.shield.now < class(data, arsenal, ShipId(128)).reserves.shield.now;
    assert!(hurt, "the beam hit the Shuttle");
    (shoves, ships, start)
}

/// A Vell-os Javelin (175, `Mass` 200) holds a Shuttle (`Mass` 15) in the
/// Flower of Spring, pulling it 5 / 15 toward the beam's start where the
/// Shuttle is off both of its axes, and not where it is dead ahead.
#[test]
#[allow(clippy::float_cmp)]
fn a_vell_os_javelins_flower_of_spring_holds_a_shuttle() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let arsenal = Arsenal::read(&data);
    assert_eq!(class(&data, &arsenal, ShipId(175)).hull.mass, 200.0);
    assert_eq!(class(&data, &arsenal, ShipId(128)).hull.mass, 15.0);
    let held = Some(TractorLock::by(ShipRef::Player));
    // Down the Javelin's 130-degree frame, 150 pixels out.
    let at = nova_sim::flight::facing(130.0) * 150.0;
    let (shoves, [javelin, shuttle], start) = flower(&data, &arsenal, 175, 130.0, at);
    assert_eq!(shuttle.tractor, held);
    assert_eq!(javelin.tractor, None);
    assert_eq!(shoves.len(), 1, "{shoves:?}");
    assert_eq!(shoves[0].ship, QUARRY);
    let pulled = push_of(start, at, -5.0, 15.0);
    assert!(close(shoves[0].velocity, pulled), "{shoves:?}, {pulled:?}");
    assert!(pulled.x < 0.0 && pulled.y < 0.0, "toward the Javelin");
    let (shoves, [_, shuttle], _) = flower(&data, &arsenal, 175, 90.0, Vec2::new(100.0, 0.0));
    assert_eq!(shuttle.tractor, held);
    assert_eq!(shoves, [], "dead ahead");
}

/// A Vell-os Dart (173, `Mass` 6, below 0.75 x 15) firing the Flower of
/// Spring at a Shuttle dead ahead pulls itself toward it by 5 / 6, holds
/// itself, and leaves the Shuttle where it was.
#[test]
#[allow(clippy::float_cmp)]
fn a_vell_os_darts_flower_of_spring_pulls_the_dart_to_a_shuttle() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let arsenal = Arsenal::read(&data);
    assert_eq!(class(&data, &arsenal, ShipId(173)).hull.mass, 6.0);
    let at = Vec2::new(100.0, 0.0);
    let (shoves, [dart, shuttle], start) = flower(&data, &arsenal, 173, 90.0, at);
    assert_eq!(dart.tractor, Some(TractorLock::by(ShipRef::Player)));
    assert_eq!(shuttle.tractor, None);
    assert_eq!(shuttle.state.velocity, Vec2::ZERO);
    assert_eq!(shoves.len(), 1, "{shoves:?}");
    assert_eq!(shoves[0].ship, ShipRef::Player);
    // The bearing runs from the Shuttle to the start; the pull back along
    // it, toward the Shuttle.
    let pulled = push_of(at, start, -5.0, 6.0);
    assert!(close(dart.state.velocity, pulled), "{shoves:?}, {pulled:?}");
    assert!(pulled.x > 0.8, "toward the Shuttle: {pulled:?}");
}

/// Stock weapon `id`.
fn stock_weapon(data: &GameData, id: i16) -> WeaponSpec {
    data.weapons()
        .iter()
        .map(WeaponSpec::new)
        .find(|spec| spec.id.0 == id)
        .expect("a stock weapon")
}

/// Exactly 18 stock weapons ionize, every one by a positive amount.
#[test]
fn stock_ionizing_weapons_are_18_all_positive() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let ionizing: Vec<(i16, i32)> = data
        .weapons()
        .iter()
        .map(WeaponSpec::new)
        .filter(|spec| spec.ionization != 0)
        .map(|spec| (spec.id.0, spec.ionization))
        .collect();
    let ids: Vec<i16> = ionizing.iter().map(|&(id, _)| id).collect();
    assert_eq!(
        ids,
        [
            137, 142, 146, 148, 160, 163, 164, 173, 174, 182, 184, 185, 198, 199, 201, 229, 230,
            232
        ]
    );
    assert!(
        ionizing.iter().all(|&(_, amount)| amount > 0),
        "{ionizing:?}"
    );
}

/// A stock fight over point defence: the player at the centre running
/// stock weapon `defence` as its point defence, its rounds full; NPC 1, a
/// Shuttle 200 pixels above facing down, firing stock `missile` at it on
/// the first tick; and `others`. Each tick's strikes and events, over
/// `ticks`.
fn defended_by(
    data: &GameData,
    arsenal: &Arsenal,
    (defence, missile): (i16, i16),
    others: Vec<Combatant>,
    ticks: u32,
) -> (Vec<Strike>, Vec<CombatEvent>) {
    let mut player = shooter(&stock_weapon(data, defence));
    player.state.heading = 0.0;
    player.trigger = Trigger {
        point_defence: true,
        ..Trigger::default()
    };
    let missile = stock_weapon(data, missile);
    let mut boat = target(data, arsenal, ShipId(128), 0.0);
    boat.id = ShipRef::Npc(nova_sim::NpcId(1));
    boat.state.position = Vec2::new(0.0, -200.0);
    boat.state.heading = 180.0;
    boat.target = Some(ShipRef::Player);
    boat.armament = Armament::new([(missile, 1)]);
    boat.rounds.set(missile.id, 10);
    boat.trigger = Trigger {
        primary: !missile.secondary(),
        secondary: missile.secondary().then_some(missile.id),
        ..Trigger::default()
    };
    let mut ships = vec![player, boat];
    ships.extend(others);
    let mut combat = Combat::default();
    let (mut strikes, mut events) = (Vec::new(), Vec::new());
    for _ in 0..ticks {
        fight_with(&mut combat, &mut ships, arsenal);
        ships[1].trigger = Trigger::default();
        strikes.extend(combat.take_strikes());
        events.extend(combat.take_events());
    }
    (strikes, events)
}

/// A stock Shuttle (128), NPC 0, 40 pixels above the centre: across the
/// path of the player's point defence at a missile from above.
fn in_the_way(data: &GameData, arsenal: &Arsenal) -> Combatant {
    bystander(data, arsenal, 0, Vec2::new(0.0, -40.0))
}

/// Whether `strikes` hold a hit by the player on NPC 0.
fn struck_in_the_way(strikes: &[Strike]) -> bool {
    strikes.iter().any(|strike| {
        strike.ship == ShipRef::Npc(nova_sim::NpcId(0)) && strike.by == Some(ShipRef::Player)
    })
}

/// The Storm Chaingun (161, `ProxRadius` 0) fired at an IR Missile flies
/// through a Shuttle's centre on its way and never hits it: point defence
/// meets a ship only by its proximity fuse.
#[test]
fn a_stock_storm_chaingun_shot_never_hits_a_ship() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let arsenal = Arsenal::read(&data);
    let (strikes, events) = defended_by(
        &data,
        &arsenal,
        (161, 134),
        vec![in_the_way(&data, &arsenal)],
        8,
    );
    let fired = events.iter().any(|event| {
        matches!(
            event,
            CombatEvent::Fired {
                ship: ShipRef::Player,
                weapon: WeaponId(161),
                ..
            }
        )
    });
    assert!(fired, "{events:?}");
    assert!(!struck_in_the_way(&strikes), "{strikes:?}");
}

/// The Quad Light Blaster Turret (133, `ProxRadius` 5) fired at an IR
/// Missile hits a Shuttle across its path by its proximity fuse.
#[test]
fn a_stock_quad_light_blaster_turret_shot_proximity_hits_a_ship() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let arsenal = Arsenal::read(&data);
    let (strikes, _) = defended_by(
        &data,
        &arsenal,
        (133, 134),
        vec![in_the_way(&data, &arsenal)],
        8,
    );
    assert!(struck_in_the_way(&strikes), "{strikes:?}");
}

/// The Polaron Torp. (148, `Flags` 0x0080, `Durability` 0) is never
/// chosen by point defence, but a Quad Light Blaster Turret shot fired at
/// an IR Missile behind it, whose sprite touches it, downs it in one hit,
/// shot down where the Quad's shot is.
#[test]
fn a_stock_point_defence_shot_downs_a_polaron_torpedo_it_touches_but_never_aims_at_one() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let arsenal = Arsenal::read(&data);
    let quad = stock_weapon(&data, 133);
    let torpedo = stock_weapon(&data, 148);
    let at_player = nova_sim::combat::projectile::Shot {
        target: Some(ShipRef::Player),
        aimed_at: Some(ShipRef::Player),
        position: Vec2::new(0.0, -100.0),
        ..nova_sim::combat::projectile::Shot::launch(
            torpedo,
            ShipRef::Npc(nova_sim::NpcId(2)),
            ShipRef::Npc(nova_sim::NpcId(2)),
            &ShipState::default(),
            180.0,
        )
    };
    let player = nova_sim::combat::defence::Side {
        ship: ShipRef::Player,
        fleet: ShipRef::Player,
        govt: None,
    };
    let shots = [at_player];
    let chosen = nova_sim::combat::defence::choose(
        player,
        &ShipState::default(),
        &HullSpec::default(),
        &quad,
        &shots,
        None,
        &nova_sim::Governments::default(),
    );
    assert_eq!(chosen, None, "never aimed at");
    // NPC 2, a Shuttle 100 above, fires the torpedo at the player as NPC
    // 1 fires the IR Missile from 200 above.
    let mut launcher = bystander(&data, &arsenal, 2, Vec2::new(0.0, -100.0));
    launcher.state.heading = 180.0;
    launcher.target = Some(ShipRef::Player);
    launcher.armament = Armament::new([(torpedo, 1)]);
    launcher.rounds.set(torpedo.id, 10);
    launcher.trigger = Trigger {
        primary: !torpedo.secondary(),
        secondary: torpedo.secondary().then_some(torpedo.id),
        ..Trigger::default()
    };
    let (strikes, events) = defended_by(&data, &arsenal, (133, 134), vec![launcher], 4);
    let downed: Vec<Vec2> = events
        .iter()
        .filter_map(|event| match *event {
            CombatEvent::ShotDown { at } => Some(at),
            _ => None,
        })
        .collect();
    // The torpedo, 13 pixels a tick, at -61 after the third tick; the
    // Quad's shot, 20 a tick, at -60: their 32- and 35-pixel sprites
    // overlap.
    assert_eq!(downed.len(), 1, "{events:?}");
    assert!(close(downed[0], Vec2::new(0.0, -60.0)), "{downed:?}");
    assert!(
        !strikes.iter().any(|strike| strike.ship == ShipRef::Player),
        "the torpedo never reached the player: {strikes:?}"
    );
}

/// The Quad Light Blaster Turret (133: `Reload` 5, `BurstCount` 4,
/// `BurstReload` 20) on two mounts fires every two and a half ticks, its
/// timer carrying what it counted past none: its burst's 8 shots leave on
/// ticks 0, 3, 5, 8, 10, 13, 15 and 18, then its 20-tick burst reload.
#[test]
fn a_stock_quad_light_blaster_turret_on_two_mounts_fires_every_two_and_a_half_ticks() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let mut armament = Armament::new([(stock_weapon(&data, 133), 2)]);
    let mut rounds = LauncherRounds::default();
    let mut fuel = Gauge::full(0.0);
    let mut reports = nova_sim::combat::report::Reports::default();
    let fired: Vec<u32> = (0..40)
        .filter(|_| {
            let fired = armament.fire_point_defence(
                Condition::Intact,
                nova_sim::combat::armament::Shooter::PLAYER,
                &mut rounds,
                &mut fuel,
                &mut Straight,
                &mut reports,
                &mut |_| {
                    Some((
                        nova_sim::combat::aim::Pointing::Along(0.0),
                        nova_sim::combat::projectile::ShotId(1),
                    ))
                },
            );
            armament.reload();
            fired.is_some()
        })
        .collect();
    assert_eq!(fired, [0, 3, 5, 8, 10, 13, 15, 18, 38]);
}

/// The homing weapons point defence cannot target (`Flags` 0x0080) are the
/// five polaron torpedoes, and every stock homing and point-defence weapon
/// has a sprite: the Quad Light Blaster Turret's 35 pixels square, the
/// Storm Chaingun's 16.
#[test]
fn stock_pd_immune_missiles_are_the_five_polarons_and_every_homing_and_pd_weapon_has_a_sprite() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let weapons = data.weapons();
    let immune: Vec<i16> = weapons
        .iter()
        .filter(|record| record.guidance == 1 && record.flags & 0x0080 != 0)
        .map(|record| record.id.0)
        .collect();
    assert_eq!(immune, [148, 182, 184, 185, 199]);
    let unseen: Vec<i16> = weapons
        .iter()
        .filter(|record| matches!(record.guidance, 1 | 9) && record.frame.is_none())
        .map(|record| record.id.0)
        .collect();
    assert_eq!(
        unseen, [0_i16; 0],
        "every homing and point-defence weapon drawn"
    );
    let frame = |id: i16| {
        weapons
            .iter()
            .find(|record| record.id.0 == id)
            .and_then(|record| record.frame)
            .map(|frame| (frame.width, frame.height))
    };
    assert_eq!(frame(133), Some((35, 35)));
    assert_eq!(frame(161), Some((16, 16)));
}

/// `ships[0]` fires once, its trigger then let go, and the fight runs on
/// for `ticks` more: the first explosion's place.
fn fire_once(ships: &mut [Combatant], arsenal: &Arsenal, ticks: u32) -> Option<Vec2> {
    let mut combat = Combat::default();
    let mut at = None;
    for tick in 0..=ticks {
        fight_with(&mut combat, ships, arsenal);
        if tick == 0 {
            ships[0].trigger = Trigger::default();
        }
        at = at.or_else(|| {
            combat.take_events().iter().find_map(|event| match *event {
                CombatEvent::Exploded { at, .. } => Some(at),
                _ => None,
            })
        });
    }
    at
}

/// A Shuttle (128), NPC `n`, at `at`.
fn bystander(data: &GameData, arsenal: &Arsenal, n: u32, at: Vec2) -> Combatant {
    let mut ship = target(data, arsenal, ShipId(128), 0.0);
    ship.id = ShipRef::Npc(nova_sim::NpcId(n));
    ship.state.position = at;
    ship
}

/// The EMP Torp. (160: `Ionization` 100, `IonizeColor` 0xd4ffff,
/// `BlastRadius` 100) hits a Leviathan (131) dead ahead and ionizes it by
/// 100 in its colour, once; a Shuttle 50 pixels from where it hit gains
/// 75, and one in the blast's square, 80 off on each axis, outside its
/// circle, none.
#[test]
#[allow(clippy::float_cmp)]
fn the_stock_emp_torpedo_ionizes_the_ship_it_hits_and_its_blasts_circle() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let arsenal = Arsenal::read(&data);
    let torpedo = stock_weapon(&data, 160);
    assert_eq!(
        (
            torpedo.guidance,
            torpedo.ionization,
            torpedo.ionize_colour,
            torpedo.blast_radius
        ),
        (Guidance::Homing, 100, 0x00d4_ffff, 100.0)
    );
    let ahead = 300.0;
    let leviathan = || target(&data, &arsenal, ShipId(131), ahead);
    // A first fight finds where the torpedo meets the Leviathan.
    let mut ships = [aiming(&torpedo), leviathan()];
    let at = fire_once(&mut ships, &arsenal, torpedo.lifetime).expect("it hit");
    assert_eq!(
        ships[1].ion,
        Ion {
            charge: 100.0,
            colour: 0x00d4_ffff
        }
    );
    let mut ships = [
        aiming(&torpedo),
        leviathan(),
        bystander(&data, &arsenal, 1, at + Vec2::new(0.0, 50.0)),
        bystander(&data, &arsenal, 2, at + Vec2::new(80.0, 80.0)),
    ];
    let again = fire_once(&mut ships, &arsenal, torpedo.lifetime);
    assert_eq!(again, Some(at), "the same hit");
    assert_eq!(ships[1].ion.charge, 100.0, "the direct hit alone");
    assert_eq!(
        ships[2].ion,
        Ion {
            charge: 75.0,
            colour: 0x00d4_ffff
        }
    );
    let shuttle = class(&data, &arsenal, ShipId(128)).reserves.shield.now;
    assert!(ships[3].reserves.shield.now < shuttle, "in the square");
    assert_eq!(ships[3].ion, Ion::default(), "outside the circle");
}

/// The Nanites (229, homing, `Ionization` 50, `Decay` 5, no blast) ionize
/// a Leviathan by the whole 50 whether they hit early or late in their
/// life, though late they do less damage.
#[test]
#[allow(clippy::float_cmp)]
fn stock_nanites_ionize_by_their_whole_ionization_however_decayed() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let arsenal = Arsenal::read(&data);
    let nanites = stock_weapon(&data, 229);
    assert_eq!(
        (
            nanites.guidance,
            nanites.ionization,
            nanites.decay,
            nanites.blast_radius
        ),
        (Guidance::Homing, 50, 5, 0.0)
    );
    let hit_from = |ahead: f32| {
        let mut ships = [
            aiming(&nanites),
            target(&data, &arsenal, ShipId(131), ahead),
        ];
        let mut combat = Combat::default();
        let mut damage = 0.0;
        for tick in 0..=nanites.lifetime {
            fight_with(&mut combat, &mut ships, &arsenal);
            if tick == 0 {
                ships[0].trigger = Trigger::default();
            }
            damage += combat
                .take_strikes()
                .iter()
                .filter(|strike| strike.ship == QUARRY)
                .map(|strike| strike.damage)
                .sum::<f32>();
        }
        (ships[1].ion.charge, damage)
    };
    let (early, early_damage) = hit_from(100.0);
    let (late, late_damage) = hit_from(1500.0);
    assert!(late_damage < early_damage, "{late_damage} < {early_damage}");
    assert_eq!((early, late), (50.0, 50.0));
}

/// The Pulse Laser (146, a beam, `Ionization` 4, 0xff00ff) held on a
/// Leviathan ionizes it by 4 on each tick it hits.
#[test]
#[allow(clippy::float_cmp)]
fn the_stock_pulse_laser_ionizes_by_4_on_every_hit_tick() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let arsenal = Arsenal::read(&data);
    let laser = stock_weapon(&data, 146);
    assert_eq!(
        (laser.guidance, laser.ionization, laser.ionize_colour),
        (Guidance::Beam, 4, 0x00ff_00ff)
    );
    let mut combat = Combat::default();
    let mut ships = [shooter(&laser), target(&data, &arsenal, ShipId(131), 60.0)];
    let mut hits = 0_u16;
    for tick in 0..=laser.lifetime + 2 {
        fight_with(&mut combat, &mut ships, &arsenal);
        if tick == 0 {
            ships[0].trigger = Trigger::default();
        }
        let strikes = combat.take_strikes();
        hits += u16::try_from(
            strikes
                .iter()
                .filter(|strike| strike.ship == QUARRY)
                .count(),
        )
        .expect("few");
    }
    assert!(hits > 1, "{hits}");
    assert_eq!(
        ships[1].ion,
        Ion {
            charge: 4.0 * f32::from(hits),
            colour: 0x00ff_00ff
        }
    );
}

/// The Ion Cannon (142, 0x34c2ff) and then the EMP Torp. (160, 0xd4ffff)
/// on one ship leave it the two colours OR-ed: 0xf4ffff.
#[test]
fn stock_ion_colours_or_together() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let arsenal = Arsenal::read(&data);
    let cannon = stock_weapon(&data, 142);
    let torpedo = stock_weapon(&data, 160);
    assert_eq!(
        (cannon.ionize_colour, torpedo.ionize_colour),
        (0x0034_c2ff, 0x00d4_ffff)
    );
    let mut ships = [aiming(&cannon), target(&data, &arsenal, ShipId(131), 100.0)];
    fire_once(&mut ships, &arsenal, 3);
    assert_eq!(ships[1].ion.colour, 0x0034_c2ff, "the cannon hit");
    ships[0] = aiming(&torpedo);
    fire_once(&mut ships, &arsenal, torpedo.lifetime);
    assert_eq!(ships[1].ion.colour, 0x00f4_ffff);
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
        let ahead = (spec.flight() / 2.0).min(300.0);
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

// The shot tick: decay, the proximity fuse and the missile lock.

/// The stock shots whose direct hits decay are the 22 non-beam weapons
/// of `Decay` above 0; the two beams of `Decay` (146 and 173) are beams,
/// which no decay reads.
#[test]
fn the_stock_shots_that_decay() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let decaying: Vec<i16> = data
        .weapons()
        .iter()
        .map(WeaponSpec::new)
        .filter(|spec| spec.decay > 0 && !spec.is_beam())
        .map(|spec| spec.id.0)
        .collect();
    assert_eq!(
        decaying,
        [
            143, 144, 145, 148, 156, 157, 158, 159, 161, 162, 163, 182, 183, 184, 185, 197, 198,
            199, 200, 219, 229, 233
        ]
    );
    let beams: Vec<i16> = data
        .weapons()
        .iter()
        .map(WeaponSpec::new)
        .filter(|spec| spec.decay > 0 && spec.is_beam())
        .map(|spec| spec.id.0)
        .collect();
    assert_eq!(beams, [146, 173]);
}

/// The shield a stock railgun slug takes off a sponge `x` pixels ahead,
/// and the tick it hit on.
fn slug_hit(spec: &WeaponSpec, x: f32) -> (f32, u32) {
    let mut combat = Combat::default();
    let mut ships = [aiming(spec), sponge(x, 0.0)];
    let full = ships[1].reserves.shield.now;
    for tick in 1..=spec.lifetime {
        fight(&mut combat, &mut ships);
        ships[0].trigger = Trigger::default();
        let now = ships[1].reserves.shield.now;
        if now < full {
            return (full - now, tick);
        }
    }
    panic!("wëap {} never hit at {x}", spec.id.0);
}

/// Each stock railgun (156-159), its slug fired at a ship near and far,
/// takes a point of its energy damage off the shield for each `Decay` + 1
/// ticks it has flown.
#[test]
// Whole points of damage and fields read whole.
#[allow(clippy::float_cmp)]
fn a_stock_railgun_slug_loses_a_point_of_mass_and_energy_every_decay_plus_one_ticks() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let arsenal = Arsenal::read(&data);
    for id in 156..=159 {
        let spec = *arsenal.weapon(WeaponId(id)).expect("a railgun");
        assert!(spec.decay > 0, "{id}");
        let (near, near_tick) = slug_hit(&spec, 100.0);
        let (far, far_tick) = slug_hit(&spec, spec.flight() * 0.75);
        let steps = |tick: u32| (tick / (spec.decay + 1)) as f32;
        assert_eq!(near, spec.energy_damage - steps(near_tick), "{id} near");
        assert_eq!(far, spec.energy_damage - steps(far_tick), "{id} far");
        assert!(steps(far_tick) >= 2.0, "{id}: {far_tick}");
        assert_eq!(near - far, steps(far_tick) - steps(near_tick), "{id}");
    }
}

/// The stock Nanite launcher (163, `ProxRadius` 0) touching a ship in its
/// path releases no Nanite (229); fired at a ship out of its reach, it
/// releases its Nanite at the end of its life, aimed at that ship.
#[test]
// Whole points of damage and fields read whole.
#[allow(clippy::float_cmp)]
fn stock_nanites_release_their_sub_229_only_at_expiry() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let arsenal = Arsenal::read(&data);
    let nanites = *arsenal.weapon(WeaponId(163)).expect("the Nanites");
    assert_eq!(nanites.prox_radius, 0.0);
    let released = |x: f32| {
        let mut combat = Combat::default();
        let mut ships = [aiming(&nanites), sponge(x, 0.0)];
        let full = ships[1].reserves;
        let mut subs = Vec::new();
        for _ in 0..=nanites.lifetime + 1 {
            fight_with(&mut combat, &mut ships, &arsenal);
            ships[0].trigger = Trigger::default();
            subs.extend(
                combat
                    .shots()
                    .iter()
                    .filter(|shot| shot.weapon.id == WeaponId(229))
                    .map(|shot| shot.aimed_at),
            );
        }
        (ships[1].reserves != full, subs)
    };
    let (hit, subs) = released(60.0);
    assert!(hit, "it touched the ship");
    assert_eq!(subs, [], "and released nothing");
    let (hit, subs) = released(5000.0);
    assert!(!hit);
    assert!(!subs.is_empty(), "released at the end of its life");
    assert!(subs.iter().all(|&aimed| aimed == Some(QUARRY)), "{subs:?}");
    // The stock shots whose contact hits no longer release theirs.
    let fused: Vec<i16> = data
        .weapons()
        .iter()
        .map(WeaponSpec::new)
        .filter(|spec| spec.prox_radius > 0.0 && spec.submunitions.is_some())
        .map(|spec| spec.id.0)
        .collect();
    assert_eq!(fused, [182, 185]);
}

/// Each stock homing missile, fired by the player at an NPC out of its
/// reach, strikes it once with no damage, a lock, on the tick its age
/// passes 30; the three of `Count` 20 (182, 185 and 199) never live that
/// long.
#[test]
fn every_stock_player_homing_missile_locks_its_target_at_31_ticks() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let homing = guided(&data, &[Guidance::Homing]);
    let ids: Vec<i16> = homing.iter().map(|spec| spec.id.0).collect();
    assert_eq!(
        ids,
        [
            134, 135, 136, 137, 148, 160, 182, 184, 185, 199, 229, 230, 234
        ]
    );
    let mut short = Vec::new();
    for spec in homing {
        let id = spec.id.0;
        let mut combat = Combat::default();
        let mut ships = [aiming(&spec), sponge(20_000.0, 0.0)];
        let full = ships[1].reserves;
        let mut locks = Vec::new();
        for tick in 1..=40 {
            fight(&mut combat, &mut ships);
            ships[0].trigger = Trigger::default();
            locks.extend(
                combat
                    .take_strikes()
                    .into_iter()
                    .map(|strike| (tick, strike)),
            );
        }
        if spec.lifetime < 30 {
            short.push(id);
            assert_eq!(locks, [], "wëap {id}");
            continue;
        }
        assert_eq!(locks.len(), 1, "wëap {id}: {locks:?}");
        let (tick, strike) = locks[0];
        assert_eq!(tick, 31, "wëap {id}");
        assert!(strike.lock && strike.direct && strike.intended, "{id}");
        assert_eq!((strike.ship, strike.by), (QUARRY, Some(ShipRef::Player)));
        assert_eq!((strike.damage, strike.downed), (0.0, None), "{id}");
        assert_eq!(ships[1].reserves, full, "wëap {id}");
    }
    assert_eq!(short, [182, 185, 199]);
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
        // The player's point defence runs every tick (`_HandlePlayer`
        // @0x68c74), whatever it holds.
        defender.trigger = Trigger {
            point_defence: true,
            ..Trigger::default()
        };
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
    use FlagField::{Flags2, Seeker};
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
    // The Zephyr family's `Flags2` 0x4000 (183-185, 199) fires while
    // cloaked, which `Armament::has_ammo` does, so it reports nothing.
    // The disable-only `Flags2` 0x1000 (142, 201, 233, 234) never
    // destroys, which `damage::take` does, so it reports nothing. The Ion
    // Cannons' `Flags3` 0x0010 (142, 201) fires from the exit point
    // nearest the target, which `exits` does, so they report nothing.
    // The sensor-interference `Seeker` 0x0008 (135, 137, 160, 230, 234) is
    // ported (see `projectile`), so it reports nothing either.
    let rows: [Row; 7] = [
        (&[134, 136], &[(Seeker, 0x2)]),
        (&[135, 160], &[(Seeker, 0x2)]),
        (&[137], &[(Seeker, 0x1)]),
        (&[148, 182, 184, 185, 199, 229], &[(Seeker, 0x1)]),
        (&[234], &[(Seeker, 0x2)]),
        (&[196, 197, 198], &[(Flags2, 0x8000)]),
        (&[230], &[(Seeker, 0x2), (Flags2, 0x8000)]),
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
    assert_eq!(expected.len(), 17);
    assert_eq!(diagnostics.len(), 17, "each once: {diagnostics:?}");
    assert_eq!(diagnostics.into_iter().collect::<BTreeSet<_>>(), expected);
}

/// Every stock `shïp` has a `shän` whose weapon exit points reach its
/// hull, 279 of the 288 with one off the centre; 55 of the 81 `wëap` fire
/// from a set of them. The Shuttle (128) fires its Light Blasters from
/// guns 3 pixels either side of its nose, 10 ahead, squashed to 71% up the
/// screen and lifted 2 by its z: at (3, -5.1) and then (-3, -5.1).
#[test]
fn stock_shans_carry_their_exit_points_to_the_hull() {
    use nova_sim::combat::exits::ExitKind;
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let arsenal = Arsenal::read(&data);
    let hulls = data.hulls();
    let backed: Vec<_> = hulls.iter().filter(|hull| hull.size.is_some()).collect();
    assert_eq!(backed.len(), 288);
    assert!(
        backed
            .iter()
            .all(|hull| hull.exits.is_some() && arsenal.hull(hull.id).exits.is_some()),
        "every shän-backed hull carries its exits"
    );
    let off_centre = backed
        .iter()
        .filter_map(|hull| hull.exits)
        .filter(|exits| exits.points.iter().flatten().any(|&point| point != (0, 0)))
        .count();
    assert_eq!(off_centre, 279);
    let weapons: Vec<WeaponSpec> = data.weapons().iter().map(WeaponSpec::new).collect();
    assert_eq!(weapons.len(), 81);
    assert_eq!(
        weapons.iter().filter(|spec| spec.exit.is_some()).count(),
        55
    );
    let shuttle = hulls
        .iter()
        .find(|hull| hull.id == ShipId(128))
        .and_then(|hull| hull.exits)
        .expect("the Shuttle's exits");
    assert_eq!(shuttle.points[0], [(3, 10), (-3, 10), (3, 10), (-3, 10)]);
    assert_eq!(shuttle.z[0], [-2; 4]);
    assert_eq!(
        (shuttle.up_compress, shuttle.down_compress),
        ((100, 71), (100, 71))
    );
    let blaster = *arsenal.weapon(WeaponId(128)).expect("the Light Blaster");
    assert_eq!(blaster.exit, Some(ExitKind::Gun));
    let mut ships = [Combatant {
        hull: arsenal.hull(ShipId(128)),
        state: ShipState::default(),
        ..shooter(&blaster)
    }];
    let mut combat = Combat::default();
    let mut starts = Vec::new();
    for _ in 0..30 {
        fight_with(&mut combat, &mut ships, &arsenal);
        if let Some(shot) = combat.shots().iter().find(|shot| shot.age == 1) {
            starts.push(shot.position - shot.velocity);
        }
    }
    assert!(starts.len() >= 2, "{starts:?}");
    let near = |a: Vec2, b: Vec2| (a - b).length() < 1e-3;
    assert!(near(starts[0], Vec2::new(3.0, -5.1)), "{starts:?}");
    assert!(near(starts[1], Vec2::new(-3.0, -5.1)), "{starts:?}");
}

/// The only stock beams that decay, the Pulse Laser (146) and the Autumn
/// Petal (173), say Falloff 0, which loads as 16 (`_LoadObjectData`
/// @0x795e2-0x795f0), so neither fades: the Pulse Laser hits its Count,
/// 15 ticks, and the Autumn Petal, fired every tick, keeps one beam at a
/// time, hitting every tick.
#[test]
fn the_stock_pulse_laser_and_autumn_petal_never_fade() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let arsenal = Arsenal::read(&data);
    let decaying: Vec<i16> = data
        .weapons()
        .iter()
        .map(WeaponSpec::new)
        .filter(|spec| spec.beam_length > 0.0 && spec.decay > 0)
        .map(|spec| spec.id.0)
        .collect();
    assert_eq!(decaying, [146, 173]);
    let pulse = *arsenal.weapon(WeaponId(146)).expect("the Pulse Laser");
    let petal = *arsenal.weapon(WeaponId(173)).expect("the Autumn Petal");
    assert_eq!((pulse.falloff, petal.falloff), (16, 16));
    assert_eq!(beam_hits(&pulse, 300.0), 15);
    let mut ships = [aiming(&petal), sponge(100.0, 0.0)];
    let mut combat = Combat::default();
    let mut most = 0;
    for tick in 0..20 {
        let before = ships[1].reserves;
        fight(&mut combat, &mut ships);
        assert_ne!(ships[1].reserves, before, "tick {tick}");
        assert!(combat.beams().iter().all(|beam| beam.fade == 0));
        most = most.max(combat.beams().len());
    }
    assert_eq!(most, 1);
}

/// Draws its value, or the last below that, every time.
struct Drawing(u32);

impl nova_sim::Chance for Drawing {
    fn fires(&mut self, _percent: u8) -> bool {
        false
    }

    fn below(&mut self, n: u32) -> u32 {
        self.0.min(n.saturating_sub(1))
    }
}

/// The Thunderhead Lance (166, `Inaccuracy` 2), held ahead, and the Ion
/// Cannon (142, `Inaccuracy` 4), a turret, each turn by `Rand(2i) − i`
/// every tick of their life: along their frame or bearing, d − i off for
/// each tick's draw d.
#[test]
#[allow(clippy::float_cmp)]
fn the_stock_thunderhead_lance_and_ion_cannon_jitter_every_tick() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let arsenal = Arsenal::read(&data);
    let lance = *arsenal
        .weapon(WeaponId(166))
        .expect("the Thunderhead Lance");
    assert_eq!((lance.inaccuracy, lance.lifetime), (2, 10));
    let ion = WeaponSpec {
        lifetime: 10,
        ..*arsenal.weapon(WeaponId(142)).expect("the Ion Cannon")
    };
    assert_eq!(ion.inaccuracy, 4);
    // The sponge lies straight up from the Ion Cannon, at bearing 0.
    for (spec, sponge_at, aim) in [(lance, (300.0, 0.0), 90.0), (ion, (0.0, -100.0), 0.0)] {
        let id = spec.id.0;
        let mut ships = [aiming(&spec), sponge(sponge_at.0, sponge_at.1)];
        let mut combat = Combat::default();
        let mut headings = Vec::new();
        for draw in [0, 3, 1, 2, 7, 5, 6, 4, 0, 1] {
            let mut fighters: Vec<Fighter> = ships.iter_mut().map(Combatant::fighter).collect();
            combat.tick(
                &mut fighters,
                &Arsenal::default(),
                &nova_sim::Governments::default(),
                Rules::default(),
                nova_sim::Field::default(),
                &mut [],
                &mut Drawing(draw),
            );
            drop(fighters);
            ships[0].trigger = Trigger::default();
            let beam = combat.beams().first().copied().expect("a beam");
            let inaccuracy = spec.inaccuracy;
            headings.push(beam.heading - aim);
            let off = f32::from(u16::try_from(draw.min(2 * inaccuracy - 1)).unwrap_or(0))
                - inaccuracy as f32;
            assert_eq!(beam.heading - aim, off, "{id}: draw {draw}");
        }
        assert!(
            headings.iter().any(|&off| off != headings[0]),
            "{id}: {headings:?}"
        );
    }
}

/// Only the stock Ion Cannons (142, 201) fire from the exit point nearest
/// their target (`Flags3` 0x0010). A Manticore (146), whose beam exits
/// stand ahead, 30 pixels to starboard, behind and 30 to port, fires its
/// Ion Cannon from the starboard exit at a ship to starboard and from the
/// port exit at a ship to port.
#[test]
fn the_stock_ion_cannons_fire_from_the_beam_exit_nearest_their_target() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let arsenal = Arsenal::read(&data);
    let nearest: Vec<i16> = data
        .weapons()
        .iter()
        .map(WeaponSpec::new)
        .filter(|spec| spec.nearest_exit)
        .map(|spec| spec.id.0)
        .collect();
    assert_eq!(nearest, [142, 201]);
    // Kept past its one tick of life, so the beam can be seen.
    let ion = WeaponSpec {
        lifetime: 10,
        ..*arsenal.weapon(WeaponId(142)).expect("the Ion Cannon")
    };
    for (x, exit, at) in [(100.0, 1, 30.0), (-100.0, 3, -30.0)] {
        let mut ships = [
            Combatant {
                hull: arsenal.hull(ShipId(146)),
                state: ShipState::default(),
                ..aiming(&ion)
            },
            sponge(x, 0.0),
        ];
        let mut combat = Combat::default();
        fight_with(&mut combat, &mut ships, &arsenal);
        let beam = combat.beams().first().copied().expect("a beam");
        assert_eq!(beam.exit, Some(exit), "{x}");
        assert!(
            (beam.start - Vec2::new(at, 0.0)).length() < 1e-3,
            "{x}: {beam:?}"
        );
    }
}

/// Each stock disable-only weapon (`Flags2` 0x1000): the Ion Cannons
/// (142, 201, turreted beams), the Fusion Pulse Turret (233) and the
/// Radar Missile (234, both blasting 20 pixels), fired again and again at
/// a ship with no shields and little armour, and a bystander beside it
/// for the blasting two: each is left at 1 armour, disabled, and never
/// breaks up (`_DamageShip` @0x3abd6, @0x3ac64).
#[test]
#[allow(clippy::float_cmp)]
fn every_stock_disable_only_weapon_leaves_its_victims_at_1_armour() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let arsenal = Arsenal::read(&data);
    let bystander = ShipRef::Npc(nova_sim::NpcId(1));
    let frail = |id, x, y| Combatant {
        id,
        hull: HullSpec {
            hit_radius: 10.0,
            ..HullSpec::default()
        },
        reserves: Reserves::full(0.0, 20.0, 0.0),
        ..sponge(x, y)
    };
    for (id, blasts) in [(142, false), (201, false), (233, true), (234, true)] {
        let spec = *arsenal.weapon(WeaponId(id)).expect("a stock weapon");
        assert!(spec.disable_only(), "wëap {id}");
        assert_eq!(spec.blast_radius > 0.0, blasts, "wëap {id}");
        let mut ships = vec![aiming(&spec), frail(QUARRY, 100.0, 0.0)];
        if blasts {
            ships.push(Combatant {
                hull: HullSpec::default(),
                ..frail(bystander, 100.0, 15.0)
            });
        }
        let mut combat = Combat::default();
        let mut events = Vec::new();
        for _ in 0..600 {
            fight_with(&mut combat, &mut ships, &arsenal);
            events.extend(combat.take_events());
        }
        for victim in &ships[1..] {
            assert_eq!(victim.reserves.armor.now, 1.0, "wëap {id}: {:?}", victim.id);
            assert_eq!(victim.condition, Condition::Disabled, "wëap {id}");
        }
        let downed = events.iter().any(|event| {
            matches!(
                event,
                CombatEvent::BreakingUp { .. } | CombatEvent::Destroyed { .. }
            )
        });
        assert!(!downed, "wëap {id}: {events:?}");
    }
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
                SimDiagnostic::CronRunsCapped { .. } => true,
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
/// the centre of Fomalhaut (`sÿst` 136), its reserves full, owning the
/// Destroyer's default items and stock weapons: a save that has read
/// neither, which flying reads.
fn destroyer_in_fomalhaut(data: &GameData) -> Session {
    let pilot = Pilot::new(data, "Stock").expect("the stock first chär starts");
    let mut save: serde_json::Value =
        serde_json::from_str(&nova_sim::save::encode(&pilot)).expect("JSON");
    save["system"] = serde_json::json!(136);
    save["stellar"] = serde_json::Value::Null;
    save["ship"] = serde_json::json!(141);
    save["outfits"] = serde_json::Value::Null;
    save["stock_weapons_fitted"] = serde_json::json!(false);
    for gauge in ["shield", "armor", "fuel"] {
        save["reserves"][gauge]["now"] = serde_json::json!(1_000_000.0);
        save["reserves"][gauge]["max"] = serde_json::json!(1_000_000.0);
    }
    let pilot = nova_sim::save::decode(&save.to_string()).expect("a pilot");
    let session = Session::fly(data, pilot).expect("flies");
    assert_eq!(session.landed(), None, "in flight");
    session
}

/// [`destroyer_in_fomalhaut`] with its two Quad Light Blaster Turrets
/// (`oütf` 133) taken off. Its point defence would fire at the Federation
/// fighters that answer the player (`_HandleShipPointDefense`
/// @0x394ce-0x3969d: every Lone Federation Ship the seeds bring is of
/// `Flags2` 0x0008), each hit a crime of its own, where the scenario
/// measures the disabling's.
fn destroyer_in_fomalhaut_without_point_defence(data: &GameData) -> Session {
    let flown = destroyer_in_fomalhaut(data);
    let mut save: serde_json::Value =
        serde_json::from_str(&nova_sim::save::encode(flown.pilot())).expect("JSON");
    let outfits = save["outfits"].as_array_mut().expect("a list");
    let before = outfits.len();
    outfits.retain(|outfit| outfit["outfit"] != 133);
    assert_eq!(outfits.len(), before - 1, "the Quads were fitted");
    let pilot = nova_sim::save::decode(&save.to_string()).expect("a pilot");
    Session::fly(data, pilot).expect("flies")
}

/// The NPC numbered `id`, if it is still in the system.
fn npc_of(session: &Session, id: nova_sim::NpcId) -> Option<&nova_sim::Npc> {
    session.npcs().iter().find(|npc| npc.id == id)
}

/// Whether no NPC but `quarry`, breaking up or not, is within 80 pixels
/// of the line from the player to it, so a shot at it hits nothing else
/// on the way.
fn clear_shot(session: &Session, quarry: &nova_sim::Npc) -> bool {
    let from = session.player().position;
    let line = quarry.state.position - from;
    let length = line.length().max(1.0);
    session
        .npcs()
        .iter()
        .filter(|npc| npc.id != quarry.id && npc.condition.active())
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
/// Federation answers, then holds its fire for 30 ticks. The player's
/// Destroyer flies without its point defence
/// ([`destroyer_in_fomalhaut_without_point_defence`]).
fn trader_disabled_in_fomalhaut(data: &GameData) -> Fomalhaut {
    let crews: BTreeMap<ShipId, i16> = PilotCatalog::ships(data)
        .into_iter()
        .map(|record| (record.id, record.crew))
        .collect();
    let mut session = destroyer_in_fomalhaut_without_point_defence(data);
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
    // The player targets the trader, as a player would: only a hit on the
    // ship it means to hit brings the police (`_DamageShip` @0x3bed2).
    session.select_target(nova_sim::TargetPick::Nearest);
    for _ in 0..session.npcs().len() {
        if session.target().is_some_and(|npc| npc.id == trader) {
            break;
        }
        session.select_target(nova_sim::TargetPick::Next);
    }
    assert_eq!(session.target().map(|npc| npc.id), Some(trader), "targeted");
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
                advice(RuleSource::Engine).say(variant, &data, ""),
                comm[45 + usize::from(variant)],
                "STR# {list} #{index}"
            );
            assert_eq!(advice(RuleSource::Bible).say(variant, &data, ""), line);
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
    // Derived afresh from the Federation's, as for an older save.
    save["system_legal"] = serde_json::Value::Null;
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
    let (_, rounds) = arsenal.npc(ShipId(143));
    assert_eq!(rounds.slot(WeaponId(149)), 4, "the Fed Carrier's Vipers");
    assert_eq!(rounds.slot(WeaponId(150)), 2, "and Anacondas");
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
    assert_eq!(nova.fee(shuttle, &viking, 1.0), 970);
    assert_eq!(nova.fee(shuttle, &earth, 1.0), 1000);
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

/// A stock pilot docked at Viking holding `ranks`.
fn at_viking_holding(data: &GameData, ranks: &[i16]) -> Session {
    let pilot = Pilot::new(data, "Stock").expect("the stock first chär starts");
    let mut save: serde_json::Value =
        serde_json::from_str(&nova_sim::save::encode(&pilot)).expect("JSON");
    save["system"] = serde_json::json!(129);
    save["stellar"] = serde_json::json!(157);
    save["ranks"] = serde_json::json!(ranks);
    let docked = nova_sim::save::decode(&save.to_string()).expect("a pilot");
    let session = Session::fly(data, docked).expect("flies");
    assert_eq!(session.landed(), Some(StellarId(157)), "docked at Viking");
    session
}

/// The Viper's price, the Heavy Shuttle's, the trade-in and the
/// Shuttle's hire fee at `session`'s Viking.
fn viking_prices(session: &mut Session) -> (i64, i64, i64, i64) {
    let shipyard = session.shipyard(&mut Always).expect("a shipyard");
    let price = |id| shipyard.row(ShipId(id)).expect("for sale").price;
    let fee = session
        .escorts_for_hire(&mut Always)
        .expect("Viking has a bar")
        .row(ShipId(128))
        .expect("the Shuttle is for hire")
        .fee;
    (price(167), price(129), shipyard.trade_in, fee)
}

/// `ränk` 128, the Federation Naval Rank of Commander, has a `PriceMod`
/// of 85 and the Federation's government (128), Viking's. A pilot docked
/// at Viking (tech level 4) holding it pays 85 % in the shipyard: the
/// Viper (80,000, tech 4) 68,000; the Heavy Shuttle (17,500, tech 4)
/// 14,875, rounded down to 14,800. The Shuttle (tech 3) trades in raw for
/// 5000, through the flux twice: 3 % off to 4850, × 0.85 to 4122, rounded
/// to 4120; then 3 % off to 3996, × 0.85 to 3396, rounded to 3390. The
/// Shuttle's hire fee is a tenth of 10,000 less 3 % (9700), × 0.85 to
/// 8245, rounded to 8240: 824. By the engine's `rank_price_scope` the
/// outfitter's prices are unchanged.
#[test]
fn a_federation_commander_pays_85_percent_at_viking() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let commander = PilotCatalog::ranks(&data)
        .into_iter()
        .find(|rank| rank.id == nova_sim::RankId(128))
        .expect("ränk 128");
    assert_eq!(commander.name, "Federation Naval Rank of Commander");
    assert_eq!(commander.price_mod, 85);
    assert_eq!(commander.affil_govt, Some(nova_sim::GovtId(128)));
    let viking = stellar_site(&data, 157);
    assert_eq!(
        (viking.govt, viking.tech_level),
        (Some(nova_sim::GovtId(128)), 4)
    );
    let viper = PilotCatalog::ships(&data)
        .into_iter()
        .find(|ship| ship.id == ShipId(167))
        .expect("the Viper");
    assert_eq!(
        (viper.cost, viper.tech_level, viper.buy_random),
        (80_000, 4, 100)
    );

    let mut unranked = at_viking_holding(&data, &[]);
    assert_eq!(viking_prices(&mut unranked), (80_000, 17_500, 4700, 970));
    let mut ranked = at_viking_holding(&data, &[128]);
    assert_eq!(viking_prices(&mut ranked), (68_000, 14_800, 3390, 824));

    let outfit_prices = |session: &mut Session| {
        session
            .outfitter(&mut Always)
            .expect("Viking has an outfitter")
            .rows
            .iter()
            .map(|row| (row.id, row.price, row.buy_price))
            .collect::<Vec<_>>()
    };
    let unranked_outfits = outfit_prices(&mut unranked);
    assert!(!unranked_outfits.is_empty());
    assert_eq!(
        outfit_prices(&mut ranked),
        unranked_outfits,
        "the engine's scope"
    );
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
            unit_mass: nova_sim::outfitter::unit_mass(
                outfit,
                fields.mass,
                &nova_sim::Rulebook::default(),
            ),
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
        if arsenal.weapon(slot.weapon).is_some() {
            assert_eq!(
                fitted.rounds.slot(slot.weapon),
                valkyrie.rounds.slot(slot.weapon) + i32::from(slot.ammo),
                "the rounds on wëap {id}, its own slot"
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

/// The swap of a person for its mission's ship on an accept on its hail
/// admits exactly 141 persons, `Flags` 0x0040 and the one ship of its
/// `LinkMission` left at accept: the 78 Terrapins of escort mission 132,
/// whose type (ship 136) its `düde` flies under its `Flags` 0x0800, and
/// the refuel traders of rescue missions 141, 650, 651 and 652. Each of
/// those missions has `ShipStart` 0 and follows the player
/// (`ShipSyst` -6).
#[test]
fn only_the_terrapins_and_the_refuel_traders_swap_for_their_missions_ship() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let persons = nova_sim::TrafficCatalog::persons(&data);
    let mut by_mission = std::collections::BTreeMap::<i16, usize>::new();
    for person in &persons {
        let left = person.mission_ship.map_or(0, |ship| ship.count);
        if nova_sim::person::swaps_for_mission_ship(person.flags, left) {
            *by_mission
                .entry(person.link_mission.expect("linked"))
                .or_default() += 1;
        }
    }
    assert_eq!(
        by_mission,
        [(132, 78), (141, 27), (650, 13), (651, 10), (652, 13)].into()
    );
    let missions = nova_sim::PilotCatalog::missions(&data);
    for &id in by_mission.keys() {
        let record = missions
            .iter()
            .find(|mission| mission.id == nova_sim::MissionId(id))
            .expect("a mission");
        let ships = record.ships;
        assert_eq!((ships.count, ships.start, ships.syst), (1, 0, -6), "{id}");
        assert_eq!(
            ships.goal,
            if id == 132 { 3 } else { 5 },
            "{id}: escort or rescue"
        );
    }
    let terrapins = missions
        .iter()
        .find(|mission| mission.id == nova_sim::MissionId(132))
        .expect("132");
    let dude = nova_sim::TrafficCatalog::dude(&data, terrapins.ships.dude.expect("a düde"))
        .expect("its düde");
    let dude_ships: Vec<_> = dude.ships.iter().map(|&(ship, _)| ship).collect();
    assert_eq!(
        nova_sim::mission::ships::swapped_ship_type(
            terrapins.flags,
            &dude_ships,
            person_record(&data, 128).ship.expect("a ship")
        ),
        Some(nova_sim::ShipId(136)),
        "a Terrapin is swapped for its own type"
    );
    let refuel = missions
        .iter()
        .find(|mission| mission.id == nova_sim::MissionId(141))
        .expect("141");
    assert_eq!(
        refuel.flags & nova_sim::mission::ships::UNIFORM_SHIP_TYPE,
        0,
        "a refuel trader's type is drawn"
    );
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

/// A pilot of combat rating `rating` flying in `sÿst` `system`, as
/// [`flying_in`] flies one by `rules`.
fn rated_flying_in(data: &GameData, system: i16, rating: i64, rules: NovaPersons) -> Session {
    let pilot = flying_in(data, system, &[], rules).pilot().clone();
    let mut save: serde_json::Value =
        serde_json::from_str(&nova_sim::save::encode(&pilot)).expect("JSON");
    save["combat_rating"] = serde_json::json!(rating);
    let pilot = nova_sim::save::decode(&save.to_string()).expect("a pilot");
    Session::fly(data, pilot)
        .expect("flies")
        .with_person_rules(std::rc::Rc::new(rules))
}

/// Hailed in Kania under `person_join`'s other reading, the Terrapin
/// (`përs` 128), whose mission 132 asks a combat rating of 10 and an
/// `AvailRandom` of 40 (the roll still undrawn, 0), lists Use As Escort
/// and joins the fleet as itself; saved and flown again it is placed as
/// the Terrapin, and no other Terrapin comes. By the engine it offers
/// mission 132 in place of the comm dialog.
#[test]
fn the_terrapin_hailed_in_kania_joins_the_player_under_person_join() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let mut session = rated_flying_in(&data, 128, 10, NovaPersons::default());
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
    let mut offering = session.clone();
    assert!(
        matches!(
            offering.open_hail(&data, &engine, &mut ByBound(&[])),
            Ok(nova_sim::Hailed::Offer(nova_sim::VenueOffer::Prompt(
                nova_sim::OfferPrompt {
                    mission: nova_sim::MissionId(132),
                    ..
                }
            )))
        ),
        "by the engine, the offer"
    );
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

/// Every setup pass's person roll lands on the Valkyrie `përs` 225, and
/// no Person slot below 100 % lists its person.
fn valkyrie_roll() -> ByBound {
    ByBound(&[(7, 0), (1022, 97), (100, 99)])
}

/// A new pilot in a system the Valkyries' `gövt` 134 governs, where the
/// Valkyrie `përs` 225 (`LinkSyst` 10006) appears, hails it: it offers
/// "Refuel Trader", mission 141 (`AvailLoc` 2), in place of the comm
/// dialog, and accepting it makes 141 active. The Terrapin's mission 132,
/// which asks a combat rating of 10, is not available to the new pilot,
/// so hailing the Terrapin in Kania opens the comm dialog.
#[test]
fn a_new_pilot_hailing_a_valkyrie_is_offered_its_refuel_mission() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let valkyries = data
        .star_map()
        .into_iter()
        .find(|system| system.govt == Some(GovtId(134)))
        .expect("a Valkyrie system");
    let mut session = flying_in(&data, valkyries.id.0, &[], NovaPersons::default());
    session.populate(&data, &mut valkyrie_roll());
    assert_eq!(persons_here(&session), [(225, false)]);
    let valkyrie = session
        .npcs()
        .iter()
        .find(|npc| npc.person.is_some())
        .expect("the Valkyrie")
        .id;
    while session.select_target(nova_sim::TargetPick::Next) != Some(valkyrie) {}
    let options = nova_sim::HailOptions::default();
    let hailed = session.open_hail(&data, &options, &mut ByBound(&[]));
    let Ok(nova_sim::Hailed::Offer(nova_sim::VenueOffer::Prompt(prompt))) = hailed else {
        panic!("an offer: {hailed:?}");
    };
    assert_eq!(prompt.mission, nova_sim::MissionId(141));
    assert!(prompt.can_refuse());
    assert!(session.hailing(&data, &options).is_none(), "no comm dialog");
    assert_eq!(
        session.accept_mission(nova_sim::MissionId(141), &mut ByBound(&[])),
        Ok(0)
    );
    assert_eq!(
        session.pilot().missions().missions(),
        [nova_sim::MissionId(141)]
    );

    let mut session = flying_in(&data, 128, &[], NovaPersons::default());
    session.populate(&data, &mut terrapin_roll());
    let terrapin = session
        .npcs()
        .iter()
        .find(|npc| npc.person.is_some())
        .expect("the Terrapin")
        .id;
    while session.select_target(nova_sim::TargetPick::Next) != Some(terrapin) {}
    assert!(
        matches!(
            session.open_hail(&data, &options, &mut ByBound(&[])),
            Ok(nova_sim::Hailed::Comm(_))
        ),
        "mission 132 is not available to a new pilot"
    );
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
        ship_name: "Kestrel",
        ranks: nova_sim::rank::RankNames::none(),
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
/// which leaves the ship at the gate's centre. It holds rank 147, "Have
/// Access to Hypergate System", which blesses landing at the Hypergate
/// government's (183) stellars: no record clears HG-Kania's `MinStatus`
/// of 32767 (`_HandlePlayerDockRequest` @0x67915).
fn over_hg_kania(data: &GameData) -> Session {
    let pilot = Pilot::new(data, "Stock").expect("the stock first chär starts");
    let mut save: serde_json::Value =
        serde_json::from_str(&nova_sim::save::encode(&pilot)).expect("JSON");
    save["stellar"] = serde_json::json!(1404);
    save["ranks"] = serde_json::json!([147]);
    let parked = nova_sim::save::decode(&save.to_string()).expect("a pilot");
    let mut session = Session::fly(data, parked).expect("flies");
    assert_eq!(
        session.take_off(data, &mut NeverFires),
        Some(StellarId(1404))
    );
    assert_eq!(session.player().position, Vec2::new(-70.0, 250.0));
    session
}

/// L, L over HG-Kania enters it, offering Tichel, Dani and Koria; picking
/// Tichel brings the Shuttle out of HG-Tichel at (-400, -500), heading
/// 120° at half its top speed, a day on, the take-off's
/// (`_PlayerLandOnStellar` @0x63440), and with a full tank.
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
    assert!((speed - session.top_speed() / 2.0).abs() < 1e-4, "{speed}");
    assert_eq!(session.date_text(), "June 24, 1177 NC");
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

/// A chance that fires every roll and draws 0: every `BuyRandom` from 1 to
/// 99 rolls on.
struct Fires;

impl Chance for Fires {
    fn fires(&mut self, _percent: u8) -> bool {
        true
    }

    fn below(&mut self, _n: u32) -> u32 {
        0
    }
}

/// Every stock weapon, ammunition and cleaned-government `ModVal` is
/// already an ID the loader leaves alone, in the fold's `wëap` 128 to 383
/// (or -1, every government, for a clean): rebasing a low `ModVal` at load
/// changes no stock outfit, which comes out of the catalog with its mods
/// exactly as its `oütf` has them.
#[test]
fn stock_weapon_mod_vals_load_unchanged_inside_the_folded_weapons() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let records = data.outfits();
    assert_eq!(records.len(), 242);
    let mut counted = [0; 3];
    for record in &records {
        let raw = data
            .get::<nova_data::records::outfit::Outfit>(record.id.0)
            .expect("present")
            .expect("decodes")
            .record;
        assert_eq!(
            record.mods,
            [
                (raw.mod_type, raw.mod_val),
                (raw.mod_type2, raw.mod_val2),
                (raw.mod_type3, raw.mod_val3),
                (raw.mod_type4, raw.mod_val4),
            ],
            "{:?}",
            record.id
        );
        for (mod_type, mod_val) in record.mods {
            let slot = match mod_type {
                1 => 0,
                3 => 1,
                21 => 2,
                _ => continue,
            };
            counted[slot] += 1;
            assert!(
                nova_sim::ship_change::FOLDED_WEAPONS.contains(&mod_val)
                    || (mod_type == 21 && mod_val == -1),
                "{:?}: ModType {mod_type}, ModVal {mod_val}",
                record.id
            );
        }
    }
    assert_eq!(counted, [83, 43, 0], "weapon, ammunition and clean mods");
}

#[test]
fn stock_repair_droids_are_the_only_repair_system_and_no_ship_class_carries_them() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let outfits = data.outfits();
    let repairing: Vec<OutfitId> = outfits
        .iter()
        .filter(|record| {
            record
                .mods
                .iter()
                .any(|&(kind, _)| kind == nova_sim::self_repair::MOD_REPAIR)
        })
        .map(|record| record.id)
        .collect();
    assert_eq!(repairing, [OutfitId(437)], "Repair Droids");
    let ships = data.ships();
    assert!(ships.len() > 200);
    for ship in &ships {
        let defaults: std::collections::BTreeMap<OutfitId, u16> =
            ship.defaults.iter().copied().collect();
        assert!(
            !nova_sim::self_repair::has_gear(&defaults, &outfits),
            "{:?} would repair itself",
            ship.id
        );
    }
    let droids = std::collections::BTreeMap::from([(OutfitId(437), 1)]);
    assert!(nova_sim::self_repair::has_gear(&droids, &outfits));
}

// Which missile an NPC fires (`_AIFireMissile`), over the stock arsenal.

/// What a stock NPC of class `ship`, armed as the traffic arms it, fires
/// at the player 1000 pixels ahead flying stock class `target` at rest
/// ([`stock_pick_at`]).
fn stock_missile_at(data: &GameData, ship: ShipId, target: ShipId) -> Option<nova_sim::WeaponId> {
    stock_pick_at(data, ship, target, 1000.0)
}

/// What a stock NPC of class `ship`, armed as the traffic arms it, fires
/// at the player `distance` pixels ahead flying stock class `target` at
/// rest, its reserves too strong for any damage aimed to suffice. The NPC
/// is a stock one of Fomalhaut's traffic made over into the class.
fn stock_pick_at(
    data: &GameData,
    ship: ShipId,
    target: ShipId,
    distance: f32,
) -> Option<nova_sim::WeaponId> {
    let mut session = destroyer_in_fomalhaut(data);
    let mut chance = NeverFires;
    session.tick_traffic(data, &NovaAi::default(), &mut chance);
    let mut npc = session
        .npcs()
        .first()
        .expect("Fomalhaut has traffic")
        .clone();
    let arsenal = Arsenal::read(data);
    let (armament, rounds) = arsenal.npc(ship);
    let fields = data.ship_fields(ship).expect("the shïp reads");
    npc.ship = ship;
    npc.stats = ShipStats::new(fields, &[]);
    npc.reserves = npc.stats.full();
    npc.hull = arsenal.hull(ship);
    npc.armament = armament;
    npc.rounds = rounds;
    npc.cloak = nova_sim::cloak::Cloak::default();
    npc.goal = Goal::Attack(ShipRef::Player);
    npc.state = ShipState::default();
    let quarry = data.ship_fields(target).expect("the target reads");
    let strong = nova_sim::Reserves::full(1_000_000.0, 1_000_000.0, 100.0);
    let player = nova_sim::ai::PlayerSide {
        state: ShipState {
            position: Vec2::new(0.0, -distance),
            ..ShipState::default()
        },
        condition: Condition::Intact,
        reserves: strong,
        hull: arsenal.hull(target),
        handling: ShipStats::new(quarry, &[]).handling,
        rating: 0,
        yardstick: 0,
        untargetable: false,
        blessing: nova_sim::rank::Blessing::NONE,
    };
    let npcs = [npc.clone()];
    let around = nova_sim::ai::Surroundings {
        player: Some(player),
        ..nova_sim::ai::Surroundings::new(&[], &npcs)
    };
    nova_sim::ai::fire::trigger(&npc, &around, nova_sim::ai::fire::FireRules::default()).only
}

/// The Manticore (`shïp` 146) carries the EMP Torpedo (`wëap` 160) in
/// slot 2 and the IR Missile (134) in slot 3, both reaching 1350 pixels.
/// At a slow Cargo Drone (130, `Maneuver` 20) it fires the IR Missile,
/// the lower weapon ID, as the original's walk by weapon index does. At
/// the Shuttle (128, `Maneuver` 40), turning 4 degrees a tick, the EMP
/// Torpedo (`GuidedTurn` 20, 2 degrees) would be unsuitable, and the IR
/// Missile still flies.
#[test]
fn a_stock_manticore_fires_its_ir_missile_before_its_emp_torpedo() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let manticore = ShipId(146);
    let slotted: Vec<i16> = nova_sim::CombatCatalog::hulls(&data)
        .into_iter()
        .find(|hull| hull.id == manticore)
        .expect("the Manticore")
        .weapons
        .iter()
        .map(|slot| slot.weapon.0)
        .collect();
    let emp = slotted.iter().position(|&id| id == 160).expect("the EMP");
    let ir = slotted.iter().position(|&id| id == 134).expect("the IR");
    assert!(emp < ir, "slotted EMP first: {slotted:?}");
    let ir_missile = Some(nova_sim::WeaponId(134));
    assert_eq!(stock_missile_at(&data, manticore, ShipId(130)), ir_missile);
    assert_eq!(stock_missile_at(&data, manticore, ShipId(128)), ir_missile);
}

/// An NPC carrying the EMP Torpedo alone of its missiles never fires it
/// at a stock ship of `Maneuver` 40 or more (a long gun may fire
/// instead), and does at a slower one.
#[test]
fn a_stock_emp_torpedo_is_never_fired_at_an_agile_ship() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let arsenal = Arsenal::read(&data);
    // The first stock class whose homing weapons are the EMP Torpedo alone.
    let carrier = data
        .ships()
        .iter()
        .map(|ship| ship.id)
        .find(|&ship| {
            let homing: Vec<i16> = arsenal
                .of_class(ship)
                .mounts()
                .iter()
                .filter(|mount| mount.spec.guidance == Guidance::Homing)
                .map(|mount| mount.spec.id.0)
                .collect();
            homing == [160]
        })
        .expect("a class carries the EMP Torpedo alone");
    let agile: Vec<ShipId> = data
        .ships()
        .iter()
        .filter(|ship| ship.fields.maneuver >= 40)
        .map(|ship| ship.id)
        .collect();
    assert!(agile.len() > 100, "{}", agile.len());
    let emp = Some(nova_sim::WeaponId(160));
    for &target in &agile {
        assert_ne!(stock_missile_at(&data, carrier, target), emp, "{target:?}");
    }
    assert_eq!(
        stock_missile_at(&data, carrier, ShipId(130)),
        emp,
        "at the Cargo Drone"
    );
}

// An NPC's rounds on each launcher (`_WeaponHasAmmo` @0xbab4-0xbac2).

/// The stock classes whose stock slots carry both 155 Hail and 161 Storm,
/// which fire 155's rounds alike, are 241, 248, 250, 294 and 395. As an
/// NPC each keeps every launcher's own load (`_RandomShipSpawn`
/// @0x3c7de-0x3c848): 241's Storm, loaded -1, never fires its point
/// defence, and 248's and 250's Hails, loaded -1, hold nothing though
/// their Storms hold hundreds. Firing the Storm as point defence until it
/// stops spends its own load to none and leaves the Hail's whole.
#[test]
fn stock_classes_carrying_the_hail_and_storm_chainguns_keep_separate_rounds() {
    use nova_sim::combat::armament::Shooter;
    use nova_sim::combat::projectile::ShotId;
    use nova_sim::combat::report::Reports;
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let arsenal = Arsenal::read(&data);
    let (hail, storm) = (nova_sim::WeaponId(155), nova_sim::WeaponId(161));
    let hulls = nova_sim::CombatCatalog::hulls(&data);
    let both: Vec<i16> = hulls
        .iter()
        .filter(|hull| {
            let carries = |weapon| hull.weapons.iter().any(|slot| slot.weapon == weapon);
            carries(hail) && carries(storm)
        })
        .map(|hull| hull.id.0)
        .collect();
    assert_eq!(both, [241, 248, 250, 294, 395]);
    for hull in hulls.iter().filter(|hull| both.contains(&hull.id.0)) {
        let load = |weapon| {
            hull.weapons
                .iter()
                .find(|slot| slot.weapon == weapon)
                .map_or(0, |slot| i32::from(slot.ammo))
        };
        let (mut armament, mut rounds) = arsenal.npc(hull.id);
        assert_eq!(rounds.slot(hail), load(hail), "{:?}", hull.id);
        assert_eq!(rounds.slot(storm), load(storm), "{:?}", hull.id);
        let npc = Shooter {
            npc: true,
            ..Shooter::PLAYER
        };
        let mut fuel = Gauge::full(0.0);
        let mut shots = 0_u32;
        for _ in 0..100_000 {
            let fired = armament.fire_point_defence(
                Condition::Intact,
                npc,
                &mut rounds,
                &mut fuel,
                &mut NeverFires,
                &mut Reports::default(),
                &mut |_| Some((nova_sim::combat::aim::Pointing::Along(0.0), ShotId(1))),
            );
            shots += u32::from(fired.is_some());
            armament.reload();
        }
        assert_eq!(
            shots > 0,
            load(storm) > 0,
            "{:?}: the Storm fires on its own load alone",
            hull.id
        );
        assert_eq!(rounds.slot(storm), load(storm).min(0), "{:?}", hull.id);
        assert_eq!(
            rounds.slot(hail),
            load(hail),
            "{:?}: the Hail's whole",
            hull.id
        );
    }
}

/// The stock scans behind the per-launcher rounds: no class names one
/// weapon in two stock slots, so summing and replacing agree; the only
/// negative stock loads on a weapon that spends rounds are 241's 161 and
/// 248's and 250's 155, each -1; no stock count is negative; and no class
/// carries a weapon or ammunition outfit among its `DefaultItems`, which
/// arm no NPC (`_LoadObjectData` @0x7aa25-0x7ad0f).
#[test]
fn stock_classes_load_one_slot_a_weapon_and_name_no_weapon_among_their_default_items() {
    use nova_sim::PilotCatalog;
    use nova_sim::combat::armament::{MOD_AMMO, MOD_WEAPON};
    use nova_sim::combat::weapon::Ammo;
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let arsenal = Arsenal::read(&data);
    let hulls = nova_sim::CombatCatalog::hulls(&data);
    let twice: Vec<i16> = hulls
        .iter()
        .filter(|hull| {
            let mut weapons: Vec<_> = hull.weapons.iter().map(|slot| slot.weapon).collect();
            weapons.sort_unstable();
            weapons.windows(2).any(|pair| pair[0] == pair[1])
        })
        .map(|hull| hull.id.0)
        .collect();
    assert_eq!(twice, Vec::<i16>::new());
    let negative: Vec<(i16, i16, i16)> = hulls
        .iter()
        .flat_map(|hull| hull.weapons.iter().map(move |slot| (hull.id, slot)))
        .filter(|(_, slot)| {
            slot.ammo < 0
                && arsenal
                    .weapon(slot.weapon)
                    .is_some_and(|spec| matches!(spec.ammo, Ammo::Rounds(_)) || spec.guidance_99())
        })
        .map(|(ship, slot)| (ship.0, slot.weapon.0, slot.ammo))
        .collect();
    assert_eq!(negative, [(241, 161, -1), (248, 155, -1), (250, 155, -1)]);
    assert!(
        hulls
            .iter()
            .flat_map(|hull| &hull.weapons)
            .all(|slot| slot.count >= 0)
    );
    let outfits = data.outfits();
    let arming: Vec<i16> = data
        .ships()
        .iter()
        .filter(|ship| {
            data.default_outfits(ship.id).iter().any(|(outfit, _)| {
                outfits.iter().any(|record| {
                    record.id == *outfit
                        && record
                            .mods
                            .iter()
                            .any(|&(kind, _)| kind == MOD_WEAPON || kind == MOD_AMMO)
                })
            })
        })
        .map(|ship| ship.id.0)
        .collect();
    assert_eq!(arming, Vec::<i16>::new());
}

// Every weapon's reach down its sub-munition chain (`_LoadObjectData`
// @0x79e8c-0x79fa6).

/// The Nanites (163) reach 2437.5 pixels down their chain to the Nanite
/// swarm (229), and the Polaron Multi-Torpedoes (182, 185) 3510 down
/// theirs to the Polaron Torpedo (148), each sub-munition at its own
/// flight. Among the stock weapons that are not beams, those whose range
/// is not their own flight are these three and every fighter bay, which
/// reaches nothing.
#[test]
// Whole and half pixels, exactly.
#[allow(clippy::float_cmp)]
fn stock_nanites_and_polaron_multi_torpedoes_reach_down_their_sub_munition_chain() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let arsenal = Arsenal::read(&data);
    let range = |id| {
        arsenal
            .weapon(nova_sim::WeaponId(id))
            .map(WeaponSpec::range)
            .expect("read")
    };
    let flight = |id| {
        arsenal
            .weapon(nova_sim::WeaponId(id))
            .map(WeaponSpec::flight)
            .expect("read")
    };
    assert_eq!((flight(163), range(163)), (187.5, 2437.5));
    assert_eq!((flight(182), range(182)), (260.0, 3510.0));
    assert_eq!(range(185), 3510.0);
    assert_eq!(range(229), flight(229));
    assert_eq!(range(148), flight(148));
    let differing: Vec<i16> = arsenal
        .weapons()
        .filter(|spec| !spec.is_beam() && (spec.range() - spec.flight()).abs() > 1e-3)
        .map(|spec| spec.id.0)
        .collect();
    assert_eq!(
        differing,
        [
            149, 150, 151, 152, 153, 154, 163, 169, 175, 176, 177, 178, 179, 180, 182, 185, 186,
            187, 188, 189, 190, 191, 192, 193, 194, 195
        ]
    );
    for spec in arsenal.weapons().filter(|spec| spec.guidance_99()) {
        assert_eq!(spec.range(), 0.0, "{:?}", spec.id);
    }
}

/// A stock Raven (164) fires its Polaron Multi-Torpedo (182, flying 260
/// pixels) at a ship 3450 pixels off, within its 3510-pixel chain
/// (3510² ≥ 0.95 d² up to about 3601), and no torpedo at 3700: there its
/// pick is the fighter bay (154) it launches, which its gun pass, finding
/// nothing, fires as its kept pick (`_AILaunchFighter` @0x81411,
/// `_AIFireGun` @0x80213), and which is reloading by then.
#[test]
fn a_stock_raven_fires_its_polaron_multi_torpedo_from_beyond_its_own_flight() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let raven = ShipId(164);
    let carried: Vec<i16> = nova_sim::CombatCatalog::hulls(&data)
        .into_iter()
        .find(|hull| hull.id == raven)
        .expect("the Raven")
        .weapons
        .iter()
        .map(|slot| slot.weapon.0)
        .collect();
    assert_eq!(carried, [146, 182, 154], "a beam, the torpedo and a bay");
    let torpedo = Some(nova_sim::WeaponId(182));
    assert_eq!(stock_pick_at(&data, raven, ShipId(130), 3450.0), torpedo);
    let bay = Some(nova_sim::WeaponId(154));
    assert_eq!(stock_pick_at(&data, raven, ShipId(130), 3700.0), bay);
}

/// A stock Krypt Pod (176), whose one weapon is the Nanites turret (163,
/// flying 187.5 pixels), turns them on a ship 2000 pixels off, within its
/// 2437.5-pixel chain and the margin, and not on one 2500 off.
#[test]
fn a_stock_krypt_pod_turns_its_nanites_on_a_ship_beyond_their_own_flight() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let pod = ShipId(176);
    let nanites = Some(nova_sim::WeaponId(163));
    assert_eq!(stock_pick_at(&data, pod, ShipId(130), 2000.0), nanites);
    assert_ne!(stock_pick_at(&data, pod, ShipId(130), 2500.0), nanites);
}

/// The stock reads behind `_FireAIShipWeapon`'s port
/// (`combat::ai_fire`): `Flags` 0x0040 (copies fire together) is set on
/// fighter bays alone, each guidance 99; no weapon has a `Recoil` other
/// than 0 or -1; none is exclusive (`Flags3` 0x0020); and every fighter bay is a secondary (`Flags` 0x0002), so an NPC keeps
/// a bay it picked.
#[test]
fn stock_weapons_fire_ai_ship_weapon_reads() {
    use nova_sim::CombatCatalog;
    use nova_sim::combat::weapon::{EXCLUSIVE, SIMULTANEOUS};
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let weapons = data.weapons();
    let ids = |keep: &dyn Fn(&nova_sim::WeaponRecord) -> bool| -> Vec<i16> {
        weapons.iter().filter(|w| keep(w)).map(|w| w.id.0).collect()
    };
    let simultaneous = ids(&|w| w.flags & SIMULTANEOUS != 0);
    assert_eq!(simultaneous, [151, 152, 186, 187, 188, 189]);
    assert!(
        weapons
            .iter()
            .filter(|w| simultaneous.contains(&w.id.0))
            .all(|w| w.guidance == 99)
    );
    assert_eq!(ids(&|w| !matches!(w.recoil, 0 | -1)), [0_i16; 0]);
    assert_eq!(ids(&|w| w.flags3 & EXCLUSIVE != 0), [0_i16; 0]);
    let bays = ids(&|w| WeaponSpec::new(w).is_bay());
    assert!(bays.len() >= 20, "{bays:?}");
    assert_eq!(
        ids(&|w| WeaponSpec::new(w).is_bay() && w.flags & 0x0002 == 0),
        [0_i16; 0]
    );
}

// The turret miss and the bomb and rocket launch (weapons-fidelity
// phase 10: `_SpawnShot` @0x3e89e-0x3e963, @0x3e73a-0x3e783,
// @0x3ede8-0x3ee0a; `_ShotCanHitShip` @0x448ad-0x448d5).

/// Draws as [`Straight`], every shot leaving straight ahead, but a draw in
/// `class`, a miss roll's range, draws `roll - 1`; each range asked is kept.
struct Rolling {
    class: u32,
    roll: u32,
    asked: Vec<u32>,
}

impl nova_sim::Chance for Rolling {
    fn fires(&mut self, _percent: u8) -> bool {
        false
    }

    fn below(&mut self, n: u32) -> u32 {
        self.asked.push(n);
        if n == self.class {
            self.roll - 1
        } else {
            n / 2
        }
    }
}

/// NPC 1, a ship of stock class `ship` at the centre, at rest, facing
/// right, targeting NPC 0, firing one of `weapon` with all the rounds,
/// fuel and armour it could want.
fn npc_shooter(data: &GameData, arsenal: &Arsenal, ship: ShipId, weapon: &WeaponSpec) -> Combatant {
    let Kind {
        hull,
        class_speed,
        top_speed,
        ..
    } = class(data, arsenal, ship);
    let mut rounds = LauncherRounds::default();
    rounds.set(weapon.id, 100_000);
    Combatant {
        id: ShipRef::Npc(nova_sim::NpcId(1)),
        ship_type: ship,
        target: Some(QUARRY),
        hull,
        class_speed,
        top_speed,
        rounds,
        ..shooter(weapon)
    }
}

/// Whether the shot of `weapon` that a ship of stock class `firer` fires
/// at a ship of stock class `quarry` 100 pixels ahead, `condition`
/// (disabled with its armour down to a tenth), hits it directly, its miss
/// roll scripted to `roll`; and the ranges drawn in.
fn turret_hits(
    data: &GameData,
    arsenal: &Arsenal,
    weapon: &WeaponSpec,
    (firer, quarry): (i16, i16),
    condition: Condition,
    roll: u32,
) -> (bool, Vec<u32>) {
    let shooter = npc_shooter(data, arsenal, ShipId(firer), weapon);
    let class = shooter.hull.miss_class;
    let mut quarry = target(data, arsenal, ShipId(quarry), 100.0);
    quarry.condition = condition;
    if condition == Condition::Disabled {
        quarry.reserves.armor.now = quarry.reserves.armor.max / 10.0;
    }
    let mut ships = [shooter, quarry];
    let mut chance = Rolling {
        class,
        roll,
        asked: Vec::new(),
    };
    let mut combat = Combat::default();
    let mut hit = false;
    // Long enough for the slowest to pass the quarry, short of any life's
    // end, so no expiry blast or sub-munition is counted.
    for tick in 0..12 {
        let mut fighters: Vec<Fighter> = ships.iter_mut().map(Combatant::fighter).collect();
        combat.tick(
            &mut fighters,
            arsenal,
            &nova_sim::Governments::default(),
            Rules::default(),
            nova_sim::Field::default(),
            &mut [],
            &mut chance,
        );
        drop(fighters);
        if tick == 0 {
            ships[0].trigger = Trigger::default();
        }
        hit |= combat
            .take_strikes()
            .iter()
            .any(|strike| strike.ship == QUARRY && strike.direct);
    }
    (hit, chance.asked)
}

/// 25 stock ships are Fighter class, 106 Medium, 106 Warships and 51
/// Freighters, so their hulls' miss classes are 80, 90, 100 and 100.
#[test]
fn stock_hulls_take_their_miss_class_from_their_escort_class() {
    use nova_sim::CombatCatalog;
    use nova_sim::escort::EscortClass;
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let arsenal = Arsenal::read(&data);
    let mut counts: BTreeMap<EscortClass, usize> = BTreeMap::new();
    for record in data.hulls() {
        *counts.entry(record.escort_class).or_default() += 1;
    }
    assert_eq!(
        counts.into_iter().collect::<Vec<_>>(),
        [
            (EscortClass::Fighter, 25),
            (EscortClass::Medium, 106),
            (EscortClass::Warship, 106),
            (EscortClass::Freighter, 51)
        ]
    );
    let miss = |id| arsenal.hull(ShipId(id)).miss_class;
    assert_eq!([135, 133, 141, 128].map(miss), [80, 90, 100, 100]);
}

/// Every stock turret (4), fired by a Fed Destroyer (141, a Warship),
/// rolls its miss below 100: it flies through an intact Lightning (135, a
/// Fighter) on 81 and hits it on 80, hits a disabled one on 100, flies
/// through an intact Starbridge (133, Medium) on 91 and hits it on 90,
/// and hits a Fed Destroyer and a Shuttle (128, a Freighter) on 100. Fired
/// by a Lightning, it rolls below 80.
#[test]
fn every_stock_turret_misses_a_light_ship_by_its_roll_and_hits_a_disabled_one() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let arsenal = Arsenal::read(&data);
    let turrets = guided(&data, &[Guidance::Turret]);
    let ids: Vec<i16> = turrets.iter().map(|spec| spec.id.0).collect();
    assert_eq!(
        ids,
        [130, 131, 132, 139, 144, 159, 162, 163, 196, 197, 198, 233]
    );
    let intact = Condition::Intact;
    for spec in &turrets {
        let id = spec.id.0;
        let hits = |quarry, condition, roll| {
            turret_hits(&data, &arsenal, spec, (141, quarry), condition, roll).0
        };
        assert!(!hits(135, intact, 81), "{id}: a fighter on 81");
        assert!(hits(135, intact, 80), "{id}: a fighter on 80");
        assert!(hits(135, Condition::Disabled, 100), "{id}: disabled");
        assert!(!hits(133, intact, 91), "{id}: a medium ship on 91");
        assert!(hits(133, intact, 90), "{id}: a medium ship on 90");
        assert!(hits(141, intact, 100), "{id}: a warship");
        assert!(hits(128, intact, 100), "{id}: a freighter");
        let (_, asked) = turret_hits(&data, &arsenal, spec, (141, 141), intact, 1);
        assert!(asked.contains(&100), "{id}: {asked:?}");
        let (_, asked) = turret_hits(&data, &arsenal, spec, (135, 141), intact, 1);
        assert!(
            asked.contains(&80) && !asked.contains(&100),
            "{id}: {asked:?}"
        );
    }
}

/// Each stock point-defence turret (9: 133, 161), on a Fed Destroyer,
/// rolls its miss below 100 as it engages an IR Missile.
#[test]
fn every_stock_point_defence_shot_rolls_its_miss_in_its_ships_class() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let arsenal = Arsenal::read(&data);
    let ir = *arsenal.weapon(WeaponId(134)).expect("the IR Missile");
    let defences = guided(&data, &[Guidance::PointDefence]);
    let ids: Vec<i16> = defences.iter().map(|spec| spec.id.0).collect();
    assert_eq!(ids, [133, 161]);
    for spec in defences {
        let id = spec.id.0;
        let mut defender = npc_shooter(&data, &arsenal, ShipId(141), &spec);
        defender.trigger = Trigger {
            point_defence: true,
            ..Trigger::default()
        };
        let mut attacker = Combatant {
            id: QUARRY,
            target: Some(defender.id),
            ..shooter(&ir)
        };
        attacker.state = ShipState {
            position: Vec2::new(engagement_range(&spec) - 10.0, 0.0),
            velocity: Vec2::ZERO,
            heading: 270.0,
        };
        let mut ships = [defender, attacker];
        let mut chance = Rolling {
            class: 100,
            roll: 37,
            asked: Vec::new(),
        };
        let mut combat = Combat::default();
        let mut fighters: Vec<Fighter> = ships.iter_mut().map(Combatant::fighter).collect();
        combat.tick(
            &mut fighters,
            &arsenal,
            &nova_sim::Governments::default(),
            Rules::default(),
            nova_sim::Field::default(),
            &mut [],
            &mut chance,
        );
        let rolls: Vec<Option<u32>> = combat
            .shots()
            .iter()
            .filter(|shot| shot.weapon.id == spec.id)
            .map(|shot| shot.miss_roll)
            .collect();
        assert_eq!(rolls, [Some(37)], "{id}");
        let missile = combat
            .shots()
            .iter()
            .find(|shot| shot.weapon.id == WeaponId(134))
            .expect("the missile");
        assert_eq!(missile.miss_roll, None, "{id}: a missile never rolls");
    }
}

// Point defence against ships (`_HandleShipPointDefense` @0x394ce-0x3969d).
// No stock weapon is a point-defence beam (10): only 133 and 161 are point
// defence at all (`every_stock_point_defence_turret_engages_an_ir_missile_in_range`
// pins them), so a point-defence beam on a ship has unit tests alone.

/// The stock ships point defence fires on (`Flags2` 0x0008) and the stock
/// ships carrying point defence (133 or 161 in a stock slot).
#[test]
fn stock_point_defence_targets_and_carriers() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let arsenal = Arsenal::read(&data);
    let hulls = data.hulls();
    let targets: Vec<i16> = hulls
        .iter()
        .filter(|hull| hull.flags2 & 0x0008 != 0)
        .map(|hull| hull.id.0)
        .collect();
    let carriers: Vec<i16> = hulls
        .iter()
        .filter(|hull| {
            hull.weapons
                .iter()
                .any(|slot| matches!(slot.weapon.0, 133 | 161))
        })
        .map(|hull| hull.id.0)
        .collect();
    assert_eq!((targets.len(), carriers.len()), (TARGETS, CARRIERS));
    for id in [128, 144, 157, 167, 279] {
        assert!(targets.contains(&id), "{id} a target");
        assert!(arsenal.hull(ShipId(id)).pd_target, "{id}");
    }
    for id in [141, 143, 153, 181] {
        assert!(carriers.contains(&id), "{id} a carrier");
    }
    assert!(!targets.contains(&141), "the Fed Destroyer is no target");
    assert!(!arsenal.hull(ShipId(141)).pd_target);
}

/// How many stock ships point defence fires on.
const TARGETS: usize = 129;
/// How many stock ships carry point defence.
const CARRIERS: usize = 71;

/// The stock `count` of `weapon` in ship `ship`'s stock slots.
fn stock_count(data: &GameData, ship: i16, weapon: i16) -> u32 {
    data.hulls()
        .iter()
        .find(|hull| hull.id == ShipId(ship))
        .expect("a stock ship")
        .weapons
        .iter()
        .filter(|slot| slot.weapon == WeaponId(weapon))
        .map(|slot| u32::try_from(slot.count).expect("a count"))
        .sum()
}

/// A stock carrier of `ship` (NPC 1) at the centre facing up, running its
/// stock `defence` as point defence alone, and a stock `attacker` (NPC 0)
/// at `at`, moving `velocity` a tick, attacking it.
fn defending(
    data: &GameData,
    arsenal: &Arsenal,
    (ship, defence): (i16, i16),
    attacker: i16,
    (at, velocity): (Vec2, Vec2),
) -> [Combatant; 2] {
    let spec = stock_weapon(data, defence);
    let mut carrier = npc_shooter(data, arsenal, ShipId(ship), &spec);
    carrier.armament = Armament::new([(spec, stock_count(data, ship, defence))]);
    carrier.target = None;
    carrier.state.heading = 0.0;
    carrier.trigger = Trigger {
        point_defence: true,
        ..Trigger::default()
    };
    let mut fighter = target(data, arsenal, ShipId(attacker), 0.0);
    fighter.state.position = at;
    fighter.state.velocity = velocity;
    fighter.attacking = Some(carrier.id);
    [carrier, fighter]
}

/// Each tick of `ships` over `ticks`, the attacker moving on: the
/// point-defence shots of `weapon` fired, each with where it left from,
/// its heading and the attacker as it was, and the strikes.
fn defended_against(
    arsenal: &Arsenal,
    ships: &mut [Combatant; 2],
    weapon: i16,
    ticks: u32,
) -> (Vec<(Vec2, f32, ShipState)>, Vec<Strike>) {
    let mut combat = Combat::default();
    let (mut fired, mut strikes) = (Vec::new(), Vec::new());
    for _ in 0..ticks {
        let seen: Vec<_> = combat.shots().iter().map(|shot| shot.id).collect();
        let attacker = ships[1].state;
        fight_with(&mut combat, ships, arsenal);
        fired.extend(
            combat
                .shots()
                .iter()
                .filter(|shot| shot.weapon.id == WeaponId(weapon) && !seen.contains(&shot.id))
                .map(|shot| (shot.position - shot.velocity, shot.heading, attacker)),
        );
        strikes.extend(combat.take_strikes());
        let state = &mut ships[1].state;
        state.position = state.position + state.velocity;
    }
    (fired, strikes)
}

/// A stock Fed Destroyer (141) runs its two Quads (133) as point defence;
/// a stock Viper (167, `Flags2` 0x0089) attacks it, crossing at its top
/// speed 150 pixels off, within the Quad's 240-pixel flight. The Quad
/// leads it from its exit, which matters at that speed, and a Quad shot
/// strikes it.
#[test]
fn a_stock_fed_destroyers_quad_leads_and_hits_a_viper_attacking_it() {
    use nova_sim::combat::aim::{bearing, lead};
    use nova_sim::combat::projectile::Target;
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let arsenal = Arsenal::read(&data);
    let speed = target(&data, &arsenal, ShipId(167), 0.0).top_speed;
    let mut ships = defending(
        &data,
        &arsenal,
        (141, 133),
        167,
        (Vec2::new(-100.0, -150.0), Vec2::new(speed, 0.0)),
    );
    let quad = stock_weapon(&data, 133);
    let (fired, strikes) = defended_against(&arsenal, &mut ships, 133, 40);
    let &(start, heading, viper) = fired.first().expect("the Quad fires");
    let quarry = Target {
        ship: QUARRY,
        ship_type: ShipId(167),
        fleet: QUARRY,
        ties: nova_sim::combat::sides::Ties::default(),
        position: viper.position,
        velocity: viper.velocity,
        radius: 0.0,
        sprite_x: 0.0,
        condition: Condition::Intact,
        miss_class: 0,
    };
    let from = ShipState {
        position: start,
        ..ShipState::default()
    };
    let led = lead(&from, &quarry, quad.speed);
    assert!((heading - led).abs() < 1e-3, "{heading} led {led}");
    let aimed = bearing(start, viper.position);
    assert!((led - aimed).abs() > 5.0, "the lead matters: {led} {aimed}");
    assert_ne!(start, Vec2::ZERO, "from its exit");
    assert!(
        strikes
            .iter()
            .any(|strike| strike.ship == QUARRY && strike.by == Some(ships[0].id) && strike.direct),
        "{strikes:?}"
    );
}

/// A stock Aurora Carrier (153) runs its Storm Chaingun (161, `ProxRadius`
/// 0) as point defence; a stock Thunderhead (157, `Flags2` 0x0089)
/// attacks it. A Storm Chaingun shot strikes it, which it can only by
/// contact: one fired at a ship is in the shot layer.
#[test]
fn a_stock_aurora_carriers_storm_chaingun_hits_a_thunderhead_attacking_it_by_contact() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let arsenal = Arsenal::read(&data);
    assert!(stock_weapon(&data, 161).prox_radius <= 0.0, "no fuse");
    let mut ships = defending(
        &data,
        &arsenal,
        (153, 161),
        157,
        (Vec2::new(0.0, -150.0), Vec2::ZERO),
    );
    let (fired, strikes) = defended_against(&arsenal, &mut ships, 161, 40);
    assert!(!fired.is_empty(), "the Storm Chaingun fires");
    assert!(
        strikes
            .iter()
            .any(|strike| strike.ship == QUARRY && strike.by == Some(ships[0].id)),
        "{strikes:?}"
    );
}

/// A stock Rebel Destroyer (181) runs its Quads as point defence; a stock
/// Fed Destroyer (141, no `Flags2` 0x0008) attacking it draws no fire.
#[test]
fn a_stock_point_defence_holds_fire_at_a_destroyer_attacking_it() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let arsenal = Arsenal::read(&data);
    let mut ships = defending(
        &data,
        &arsenal,
        (181, 133),
        141,
        (Vec2::new(0.0, -150.0), Vec2::ZERO),
    );
    let (fired, _) = defended_against(&arsenal, &mut ships, 133, 20);
    assert_eq!(fired, []);
}

/// The Raven Rocket (138) is in the stock slots of 31 ships. Fired by one
/// of them, an Auroran Carrier (137), as an NPC moving (2, 1) and facing
/// right, it leaves at that and its 15 pixels a tick to the right; fired
/// by the player in the same ship, at its ship's velocity alone. Each
/// then closes on its top speed as a rocket does.
#[test]
fn a_stock_raven_rocket_leaves_an_npc_at_its_speed_and_the_player_at_its_ships() {
    use nova_sim::CombatCatalog;
    use nova_sim::combat::projectile::{ROCKET_KEEP, ROCKET_PUSH};
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let arsenal = Arsenal::read(&data);
    let carriers: Vec<i16> = data
        .hulls()
        .iter()
        .filter(|hull| hull.weapons.iter().any(|slot| slot.weapon == WeaponId(138)))
        .map(|hull| hull.id.0)
        .collect();
    assert_eq!(carriers.len(), 31, "{carriers:?}");
    assert_eq!(carriers[0], 137);
    let raven = stock_weapon(&data, 138);
    assert_eq!((raven.guidance, raven.speed), (Guidance::Rocket, 15.0));
    let v = Vec2::new(2.0, 1.0);
    let flown = |id: ShipRef| {
        let mut ship = npc_shooter(&data, &arsenal, ShipId(137), &raven);
        ship.id = id;
        ship.state.velocity = v;
        if id == ShipRef::Player {
            ship.rounds.set(WeaponId(10), 100_000);
        }
        let mut ships = [ship];
        let mut combat = Combat::default();
        fight_with(&mut combat, &mut ships, &arsenal);
        combat
            .shots()
            .iter()
            .find(|shot| shot.weapon.id == WeaponId(138))
            .expect("a Raven Rocket")
            .velocity
    };
    let ahead = facing(90.0) * (15.0 * ROCKET_PUSH);
    let npc = flown(ShipRef::Npc(nova_sim::NpcId(1)));
    let thrown = (v + Vec2::new(15.0, 0.0)) * ROCKET_KEEP + ahead;
    assert!(close(npc, thrown), "{npc:?} {thrown:?}");
    let player = flown(ShipRef::Player);
    let dropped = v * ROCKET_KEEP + ahead;
    assert!(close(player, dropped), "{player:?} {dropped:?}");
}

/// No stock weapon is a freefall bomb (5), none releases one as its
/// sub-munition, and none spreads its sub-munitions in a starburst (a
/// negative `SubTheta`): the bomb launch, the weathervane, a bomb
/// sub-munition's speed and the starburst's whole-degree halving reach no
/// stock weapon.
#[test]
fn no_stock_weapon_is_a_bomb_or_a_starburst() {
    use nova_sim::CombatCatalog;
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let weapons = data.weapons();
    assert!(weapons.iter().all(|weapon| weapon.guidance != 5));
    assert!(weapons.iter().all(|weapon| weapon.sub_theta >= 0));
}
