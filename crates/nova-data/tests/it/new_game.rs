//! The new game's folder adapter (`new_game::open`) over real folders:
//! the checked-in fixture (`tests/fixtures/new-game/`, published as
//! `new_game::fixture::folder`), the sample's cut
//! (`tests/fixtures/sample/`, `new_game::fixture::sample_folder`), and temporary
//! folders for a root that can't be listed and for media that are Git LFS
//! pointers.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use nova_data::new_game::fleet::PX_PER_METRE;
use nova_data::records::character::Character;
use nova_data::records::desc::Desc;
use nova_data::records::dude::Dude;
use nova_data::records::govt::Govt;
use nova_data::records::junk::Junk;
use nova_data::records::mission::Mission;
use nova_data::records::outfit::Outfit;
use nova_data::records::ship::Ship;
use nova_data::records::ship_anim::ShipAnim;
use nova_data::records::stellar::{Stellar, flags2};
use nova_data::records::string_list::StrList;
use nova_data::records::system::System;
use nova_data::records::weapon::Weapon;
use nova_data::store::GameData;
use nova_data::{
    GovtId, Record, Registered, ShipId, StandIn, StandInOf, StellarId, SystemId, new_game,
};
use nova_galaxy_format::fleet::Size;
use nova_galaxy_media::BARE;

/// The checked-in fixture (`new_game::fixture::folder`): one region; the
/// factions `guild` and `warden`; the systems `drift` (jump points alone),
/// `hearth` (the guild's) and `verge` (ungoverned); four stellars:
/// `hearth-dock`, a guild station with every service; `hearth-moon`, with
/// none; `verge-camp`, a warden outpost with a bar, an exchange and
/// no fuel; and `verge-workshop`, an unowned outpost, a lawless yard, with
/// an outfitter and fuel; seven goods, six standard and `vintage-wine` a `jünk`, which
/// the two exchanges price; jump
/// points two of which lead from `hearth` to `verge`, a dead pair between
/// `verge` and `drift`, and `drift`'s one-way back door to `verge`; the
/// civilian hull `hauler` (4000, sold nowhere), the crown hull `cutter`
/// (50,000 at `hearth-dock`) and the wardens' crown hull `warden-picket`
/// (unsold), in their colours, and the variant `racked-hauler`, the
/// hauler with cargo racks and a turret mass driver, sold at
/// `hearth-dock` with no price of its own; the outfits
/// `dock-pass` and `mass-driver`, a turret sold at `hearth-dock`; the mod
/// kits `cargo-racks` (legal), `gun-mount` (lawless, adding a turret
/// hardpoint) and `turret-ring` (legal, adding a turret hardpoint); the
/// start `dock-hand`, in the hauler in `hearth`; the
/// storyline `dock-work`,
/// whose four missions and two news items run over the conversations of
/// `dialog/dock-job.ink`, compiled, whose greeting `verge-camp` names.
fn fixture() -> PathBuf {
    new_game::fixture::folder()
}

/// Record `id`'s name and record.
fn named<T: Registered + Clone>(data: &GameData, id: i16) -> (String, T) {
    let entry = data.get::<T>(id).expect("present").expect("decoded");
    (entry.name.expect("named").to_owned(), entry.record.clone())
}

/// The bible id `text`.
fn id(text: &str) -> nova_galaxy_format::id::Id {
    nova_galaxy_format::id::Id::try_from(text.to_owned()).expect("an id")
}

/// `table` as `(id, nova)` pairs.
fn point(text: &str) -> nova_galaxy_format::id::PointRef {
    nova_galaxy_format::id::PointRef::try_from(text.to_owned()).expect("a point")
}

fn pairs<K: ToString>(table: &BTreeMap<K, i16>) -> Vec<(String, i16)> {
    table
        .iter()
        .map(|(id, &nova)| (id.to_string(), nova))
        .collect()
}

#[test]
fn the_fixture_folder_is_the_crates_own() {
    assert_eq!(
        fixture(),
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("new-game")
    );
}

// This and `the_sample_folder_loads_without_problems` also show each
// folder's `nova-ids.json` pins every record it maps: a table that left
// one out would be a problem.
#[test]
fn the_fixture_folder_loads_without_problems_or_pointers() {
    let game = new_game::open(&fixture()).expect("lists");
    let problems: Vec<_> = game.problems.iter().map(ToString::to_string).collect();
    assert_eq!(problems, Vec::<String>::new());
    assert_eq!(game.lfs_pointers, Vec::<String>::new());
}

#[test]
fn verge_camp_names_the_fixture_s_one_stellar_conversation() {
    let game = new_game::open(&fixture()).expect("lists");
    let camp = StellarId(game.ids.stellars[&id("verge-camp")]);
    let dialogs = game.data.dialogs();
    let places: Vec<_> = dialogs
        .stellars
        .iter()
        .map(|(&stellar, place)| (stellar, place.as_ref_text()))
        .collect();
    assert_eq!(
        places,
        [(camp, "dialog/dock-job.ink#camp_greeting".to_owned())],
        "the dock's job is a mission's, and the moon names none"
    );
    let (_, compiled) = dialogs.stellar(camp).expect("its compiled story");
    assert_eq!(
        &**compiled,
        nova_data::new_game::dialogs::fixture::DOCK_JOB_JSON
    );
}

#[test]
fn the_fixture_maps_four_missions_two_news_items_and_nine_jump_points() {
    let game = new_game::open(&fixture()).expect("lists");
    let storylines = game.data.storylines();
    let keys = |ids: Vec<&String>| ids.into_iter().cloned().collect::<Vec<_>>();
    assert_eq!(
        keys(storylines.missions.keys().collect()),
        [
            "clear-the-road",
            "dock-job",
            "side-road-closes",
            "warden-toll"
        ]
    );
    assert_eq!(
        keys(storylines.news.keys().collect()),
        ["plate-arrives", "side-road-closes"]
    );
    assert_eq!(storylines.points.len(), 9);
    let dock = StellarId(game.ids.stellars[&id("hearth-dock")]);
    let camp = StellarId(game.ids.stellars[&id("verge-camp")]);
    let job = &storylines.missions["dock-job"];
    let offer = job.offer.as_ref().expect("offered");
    assert_eq!(
        (offer.at, offer.dialog.as_ref_text()),
        (dock, "dialog/dock-job.ink#job_offer".to_owned()),
        "the dock job is offered at Hearth Dock"
    );
    assert_eq!(
        job.objective,
        Some(nova_data::MissionObjective::Deliver {
            cargo: "Hull plate".to_owned(),
            tons: 10,
            to: camp,
        })
    );
    assert_eq!(
        storylines.missions["side-road-closes"].offer, None,
        "an event"
    );
    assert_eq!(
        storylines.news["side-road-closes"].from,
        SystemId(game.ids.systems[&id("hearth")])
    );
    let road = &storylines.points["verge/hearth-road"];
    assert_eq!(road.owner.as_deref(), Some("warden"));
    assert_eq!(road.stability, nova_data::GateStability::Unstable);
}

#[test]
fn the_fixture_folder_holds_a_record_for_each_thing_it_maps() {
    let data = new_game::open(&fixture()).expect("lists").data;
    assert_eq!(data.ids(Govt::TYPE), [128, 129]);
    assert_eq!(data.ids(System::TYPE), [128, 129, 130]);
    assert_eq!(
        data.ids(Stellar::TYPE),
        (128..=140).collect::<Vec<_>>(),
        "three stellars, nine jump points' gates, then the workshop, pinned after them"
    );
    assert_eq!(
        data.ids(Desc::TYPE),
        [128, 130, 140],
        "the stellars with a body"
    );
    assert_eq!(
        data.ids(Ship::TYPE),
        [128, 129, 130, 131],
        "the three hulls and the variant"
    );
    assert_eq!(data.ids(ShipAnim::TYPE), [128, 129, 130, 131]);
    assert_eq!(
        data.ids(Dude::TYPE),
        [128, 129],
        "the civilian traffic, then the wardens' guards"
    );
    assert_eq!(data.ids(Mission::TYPE), [128], "clear-the-road's blockade");
    assert_eq!(data.ids(Character::TYPE), [128], "the start");
    assert_eq!(
        data.ids(Outfit::TYPE),
        [128, 129, 130, 131, 132],
        "the dock pass, the cargo racks kit, the mass driver, the gun mount kit, then the turret ring kit"
    );
    assert_eq!(data.ids(Weapon::TYPE), [128], "the mass driver");
    assert_eq!(
        data.ids(StrList::TYPE),
        [
            128, 134, 138, 1100, 2002, 3000, 3001, 3002, 4000, 4001, 4003, 4004, 5003
        ],
        "the engine's own lists around the standard goods' lists"
    );
    assert_eq!(data.ids(Junk::TYPE), [128], "the vintage wine");
    let [file] = data.files() else {
        panic!("one file: {:?}", data.files());
    };
    assert_eq!(file.path, fixture());
}

#[test]
fn the_factions_are_govts_with_their_colours() {
    let data = new_game::open(&fixture()).expect("lists").data;
    let (name, guild) = named::<Govt>(&data, 128);
    assert_eq!((name.as_str(), guild.color), ("The Guild", 0x00C0_8040));
    let (name, warden) = named::<Govt>(&data, 129);
    assert_eq!(
        (name.as_str(), warden.ship_color),
        ("The Wardens", 0x0033_66CC)
    );
}

#[test]
fn the_systems_are_placed_governed_and_hold_their_stellars_then_gates_with_no_links() {
    let data = new_game::open(&fixture()).expect("lists").data;
    let ids = |nav_def: &[Option<StellarId>]| -> Vec<i16> {
        nav_def.iter().flatten().map(|id| id.0).collect()
    };
    let (name, drift) = named::<System>(&data, 128);
    assert_eq!(name, "Drift");
    assert_eq!(ids(&drift.nav_def), [131, 132, 133], "gates alone");
    let (name, hearth) = named::<System>(&data, 129);
    assert_eq!(name, "Hearth");
    assert_eq!((hearth.x_pos, hearth.y_pos), (0, 0));
    assert_eq!(hearth.govt, Some(GovtId(128)));
    assert_eq!(ids(&hearth.nav_def), [128, 129, 134, 135, 136]);
    let (name, verge) = named::<System>(&data, 130);
    assert_eq!(name, "Verge");
    assert_eq!((verge.x_pos, verge.y_pos), (300, -120));
    assert_eq!(verge.govt, None);
    assert_eq!(ids(&verge.nav_def), [130, 140, 137, 138, 139]);
    for system in [drift, hearth, verge] {
        assert_eq!(system.con, [None; 16], "the jump points are not links");
        assert_eq!(system.dude_types[0], 128, "the civilian traffic");
        assert_eq!(system.prob[0], 100);
        assert!(system.avg_ships > 0);
    }
}

#[test]
fn the_stellars_are_placed_owned_and_flagged_by_their_services() {
    let data = new_game::open(&fixture()).expect("lists").data;
    let (name, dock) = named::<Stellar>(&data, 128);
    assert_eq!(name, "Hearth Dock");
    assert_eq!((dock.x_pos, dock.y_pos), (200, -100));
    assert_eq!(dock.govt, Some(GovtId(128)));
    // Landing, the commodity exchange, the outfitter, the shipyard, the
    // station and the bar.
    assert_eq!(dock.flags.bits() & 0xFF, 0x5F);
    let (name, moon) = named::<Stellar>(&data, 129);
    assert_eq!(name, "Hearth's Moon");
    assert_eq!(moon.govt, None);
    assert_eq!(moon.flags.bits(), 0x20, "uninhabited, no landing");
    let (name, camp) = named::<Stellar>(&data, 130);
    assert_eq!(name, "Verge Camp");
    assert_eq!(camp.govt, Some(GovtId(129)));
    assert_eq!(
        camp.flags.bits() & 0xFF,
        0x73,
        "landing, the commodity exchange, a station, a bar, no fuel"
    );
    assert_eq!(camp.hyper_link, [None; 8]);
    let (name, workshop) = named::<Stellar>(&data, 140);
    assert_eq!(name, "Verge Workshop");
    assert_eq!(workshop.govt, None);
    assert_eq!(
        workshop.flags.bits() & 0xFF,
        0x15,
        "landing, an outfitter, a station"
    );
}

/// The `SpecialTech` slots of `spob`.
fn special_tech(spob: &Stellar) -> Vec<i16> {
    spob.special_tech1_3
        .iter()
        .chain(&spob.special_tech4_8)
        .copied()
        .collect()
}

#[test]
fn the_mass_driver_is_sold_at_hearth_dock_alone_and_verge_workshop_is_a_lawless_yard() {
    use nova_data::new_game::kits::{FitBits, YARD_TECH_LAWLESS, YARD_TECH_LEGAL};
    use nova_data::new_game::yards::SOLD_TECH_FIRST;
    use nova_galaxy_format::fleet::Mount;

    let game = new_game::open(&fixture()).expect("lists");
    let (name, driver) = named::<Outfit>(&game.data, game.ids.outfits[&id("mass-driver")]);
    assert_eq!(name, "Mass driver");
    assert_eq!(
        (driver.tech_level, driver.buy_random, driver.cost),
        (SOLD_TECH_FIRST, 100, 2500)
    );
    assert_eq!(driver.require.bits(), FitBits::mount_bit(Mount::Turret));
    let spob = |stellar: &str| named::<Stellar>(&game.data, game.ids.stellars[&id(stellar)]).1;
    let dock = spob("hearth-dock");
    assert_eq!(dock.tech_level, YARD_TECH_LEGAL);
    assert_eq!(special_tech(&dock), [3, -1, -1, -1, -1, -1, -1, -1]);
    for stellar in ["hearth-moon", "verge-camp", "verge-workshop"] {
        assert_eq!(special_tech(&spob(stellar)), [-1; 8], "{stellar}");
    }
    assert_eq!(spob("verge-workshop").tech_level, YARD_TECH_LAWLESS);
    let (_, mount) = named::<Outfit>(&game.data, game.ids.kits[&id("gun-mount")]);
    assert_eq!(mount.contribute.bits(), FitBits::mount_bit(Mount::Turret));
}

/// The turret ring is a legal kit, so a law-keeping yard like Hearth Dock
/// fits it, that trades cargo for a turret hardpoint with no gun: the one
/// kit the racked hauler, which already holds its racks, can still take.
#[test]
fn the_turret_ring_is_a_legal_kit_adding_an_empty_turret_hardpoint() {
    use nova_data::new_game::kits::{FitBits, KIT_TECH_LEGAL, mod_type};
    use nova_galaxy_format::fleet::Mount;

    let game = new_game::open(&fixture()).expect("lists");
    let (name, ring) = named::<Outfit>(&game.data, game.ids.kits[&id("turret-ring")]);
    assert_eq!(name, "Turret ring");
    assert_eq!((ring.tech_level, ring.cost), (KIT_TECH_LEGAL, 1000));
    assert_eq!(ring.contribute.bits(), FitBits::mount_bit(Mount::Turret));
    assert_eq!(
        [
            (ring.mod_type, ring.mod_val),
            (ring.mod_type2, ring.mod_val2)
        ],
        [(mod_type::MORE_CARGO, -4), (mod_type::MORE_TURRETS, 1)]
    );
    assert_eq!(ring.on_purchase.as_str(), "", "no gun comes with it");
}

#[test]
fn the_cutter_and_the_racked_hauler_are_sold_at_hearth_dock_alone_and_the_hauler_nowhere() {
    use nova_data::new_game::records::OUTFIT_TECH_NEVER;
    use nova_data::new_game::yards::SOLD_TECH_FIRST;

    let game = new_game::open(&fixture()).expect("lists");
    let ship = |ship: &str| named::<Ship>(&game.data, game.ids.ships[&id(ship)]).1;
    let sale = |ship: &Ship| {
        (
            ship.cost,
            ship.tech_level,
            ship.buy_random,
            ship.hire_random,
        )
    };
    assert_eq!(sale(&ship("cutter")), (50_000, SOLD_TECH_FIRST, 100, 0));
    assert_eq!(
        sale(&ship("racked-hauler")),
        (6000, SOLD_TECH_FIRST, 100, 0),
        "the hauler's 4000 and its racks' 2000, at the mass driver's number: both \
         are sold at Hearth Dock alone"
    );
    assert_eq!(sale(&ship("hauler")), (4000, OUTFIT_TECH_NEVER, 0, 0));
    assert_eq!(sale(&ship("warden-picket")), (0, OUTFIT_TECH_NEVER, 0, 0));
    let spob = |stellar: &str| named::<Stellar>(&game.data, game.ids.stellars[&id(stellar)]).1;
    assert_eq!(
        special_tech(&spob("hearth-dock")),
        [3, -1, -1, -1, -1, -1, -1, -1]
    );
    for stellar in ["hearth-moon", "verge-camp", "verge-workshop"] {
        assert_eq!(special_tech(&spob(stellar)), [-1; 8], "{stellar}");
    }
}

#[test]
fn the_trade_stellars_price_the_six_standard_goods_in_their_nibbles() {
    use nova_data::records::stellar::flags::{HIGH_PRICE, LOW_PRICE, MEDIUM_PRICE, price_bits};
    let game = new_game::open(&fixture()).expect("lists");
    let slot = |good: &str| u8::try_from(game.ids.commodities[&id(good)]).expect("a slot");
    let price = |good: &str, level: u32| price_bits(slot(good), level);
    let nibbles = |stellar: &str| {
        let (_, record) = named::<Stellar>(&game.data, game.ids.stellars[&id(stellar)]);
        record.flags.bits() & !0xFF
    };
    assert_eq!(
        nibbles("hearth-dock"),
        price("alloys", MEDIUM_PRICE)
            | price("fuel-cells", LOW_PRICE)
            | price("grain", LOW_PRICE)
            | price("machinery", HIGH_PRICE)
            | price("medicine", MEDIUM_PRICE)
            | price("textiles", HIGH_PRICE)
    );
    assert_eq!(
        nibbles("verge-camp"),
        price("grain", HIGH_PRICE) | price("medicine", HIGH_PRICE)
    );
    assert_eq!(nibbles("hearth-moon"), 0, "it trades nothing");
}

#[test]
fn the_standard_goods_are_named_and_priced_in_their_slots() {
    let game = new_game::open(&fixture()).expect("lists");
    assert_eq!(
        pairs(&game.ids.commodities),
        [
            ("alloys".to_owned(), 0),
            ("fuel-cells".to_owned(), 1),
            ("grain".to_owned(), 2),
            ("machinery".to_owned(), 3),
            ("medicine".to_owned(), 4),
            ("textiles".to_owned(), 5)
        ]
    );
    let strings = |list: i16| -> Vec<String> {
        let (_, record) = named::<StrList>(&game.data, list);
        record.strings.iter().map(ToString::to_string).collect()
    };
    assert_eq!(
        strings(4000),
        [
            "Alloys",
            "Fuel cells",
            "Grain",
            "Machinery",
            "Medicine",
            "Textiles"
        ]
    );
    assert_eq!(strings(4004), ["400", "150", "100", "600", "800", "250"]);
}

#[test]
fn vintage_wine_is_a_junk_sold_at_verge_camp_and_bought_at_hearth_dock() {
    let game = new_game::open(&fixture()).expect("lists");
    assert_eq!(pairs(&game.ids.junk), [("vintage-wine".to_owned(), 128)]);
    let (name, wine) = named::<Junk>(&game.data, 128);
    assert_eq!(name, "Vintage wine");
    assert_eq!(wine.base_price, 1200);
    let at = |stellar: &str| Some(StellarId(game.ids.stellars[&id(stellar)]));
    let side = |first| {
        let mut side = [None; 8];
        side[0] = first;
        side
    };
    assert_eq!(wine.sold_at, side(at("verge-camp")));
    assert_eq!(wine.bought_at, side(at("hearth-dock")));
}

#[test]
fn each_jump_point_is_a_hypergate_linked_to_its_partner_s() {
    let game = new_game::open(&fixture()).expect("lists");
    assert_eq!(
        pairs(&game.ids.gates),
        [
            ("drift/back-door".to_owned(), 131),
            ("drift/hearth-road".to_owned(), 132),
            ("drift/verge-road".to_owned(), 133),
            ("hearth/drift-road".to_owned(), 134),
            ("hearth/side-road".to_owned(), 135),
            ("hearth/verge-road".to_owned(), 136),
            ("verge/drift-road".to_owned(), 137),
            ("verge/hearth-road".to_owned(), 138),
            ("verge/side-road".to_owned(), 139),
        ]
    );
    let link = |gate: i16| {
        let (_, stellar) = named::<Stellar>(&game.data, gate);
        assert_eq!(stellar.flags2.bits(), flags2::HYPERGATE, "{gate}");
        assert_eq!(stellar.flags.bits(), 0x01, "{gate}: landable");
        assert_eq!(stellar.hyper_link[1..], [None; 7], "{gate}: one link");
        stellar.hyper_link[0].map(|id| id.0)
    };
    // Two roads from hearth to verge, each to its own partner.
    assert_eq!(link(136), Some(138));
    assert_eq!(link(135), Some(139));
    assert_eq!(link(138), Some(136));
    assert_eq!(link(139), Some(135));
    // The dead pair, travelled until stability is read.
    assert_eq!(link(133), Some(137));
    assert_eq!(link(137), Some(133));
    // The one-way back door reaches verge's side road, which leads to
    // hearth.
    assert_eq!(link(131), Some(139));
    assert_eq!(link(132), Some(134));
    assert_eq!(link(134), Some(132));
    let (name, side) = named::<Stellar>(&game.data, 135);
    assert_eq!(name, "Verge", "named for the partner's system");
    assert_eq!((side.x_pos, side.y_pos), (0, 1300));
    let (_, owned) = named::<Stellar>(&game.data, 138);
    assert_eq!(owned.govt, Some(GovtId(129)), "the wardens hold it");
}

#[test]
fn each_hull_is_a_ship_with_its_size_and_the_civilian_one_is_traffic() {
    let game = new_game::open(&fixture()).expect("lists");
    assert_eq!(
        pairs(&game.ids.ships),
        [
            ("cutter".to_owned(), 128),
            ("hauler".to_owned(), 129),
            ("racked-hauler".to_owned(), 131),
            ("warden-picket".to_owned(), 130)
        ]
    );
    let (name, hauler) = named::<Ship>(&game.data, 129);
    assert_eq!(name, "Hauler");
    assert_eq!(
        (hauler.speed, hauler.shield, hauler.armor, hauler.holds),
        (300, 100, 200, 50)
    );
    assert_eq!((hauler.accel, hauler.maneuver, hauler.fuel), (300, 40, 300));
    let (_, anim) = named::<ShipAnim>(&game.data, 129);
    assert_eq!((anim.base_x_size, anim.base_y_size), (12, 30));
    let (name, dude) = named::<Dude>(&game.data, 128);
    assert_eq!(name, "Civilian traffic");
    assert_eq!(dude.ship_type[..2], [Some(ShipId(129)), None]);
    assert_eq!(dude.ai_type, 1);
}

#[test]
fn the_wardens_hold_their_road_with_pickets_in_their_colours() {
    let game = new_game::open(&fixture()).expect("lists");
    let (name, guards) = named::<Dude>(&game.data, 129);
    assert_eq!(name, "The Wardens guards");
    assert_eq!(guards.govt, Some(GovtId(game.ids.factions[&id("warden")])));
    assert_eq!(
        guards.ship_type[..2],
        [Some(ShipId(game.ids.ships[&id("warden-picket")])), None],
        "the picket wears the wardens' colours; the cutter wears none"
    );
    let (name, blockade) = named::<Mission>(&game.data, game.ids.blockades[&id("clear-the-road")]);
    assert_eq!(name, "Clear the hearth road blockade");
    assert_eq!(blockade.ship_dude.map(|dude| dude.0), Some(129));
    assert_eq!(blockade.ship_syst, game.ids.systems[&id("verge")]);
    let (_, verge) = named::<System>(&game.data, game.ids.systems[&id("verge")]);
    let road = StellarId(game.ids.gates[&point("verge/hearth-road")]);
    let slot = verge
        .nav_def
        .iter()
        .position(|s| *s == Some(road))
        .expect("a slot");
    assert_eq!(i32::from(blockade.ship_start), -(slot as i32) - 1);
}

#[test]
fn the_harrow_band_blockades_the_far_gate_with_its_gunships() {
    let game = new_game::open(&new_game::fixture::sample_folder()).expect("lists");
    let (name, guards) = named::<Dude>(&game.data, 129);
    assert_eq!(name, "The Harrow band guards");
    assert_eq!(
        guards.govt,
        Some(GovtId(game.ids.factions[&id("harrow-band")]))
    );
    let gunship = game.ids.ships[&id("harrow-gunship")];
    assert_eq!(guards.ship_type[..2], [Some(ShipId(gunship)), None]);
    let (_, ship) = named::<Ship>(&game.data, gunship);
    assert!(
        ship.weap_type1_4[0].is_some(),
        "armed: its gun mount's turret"
    );
    let (name, blockade) = named::<Mission>(&game.data, game.ids.blockades[&id("take-the-point")]);
    assert_eq!(name, "Take the point blockade");
    assert_eq!(blockade.ship_dude.map(|dude| dude.0), Some(129));
    assert_eq!(blockade.ship_syst, game.ids.systems[&id("marrow")]);
}

#[test]
fn the_sample_s_light_freighter_and_its_gunship_collide_at_the_freighter_s_radius() {
    let folder = new_game::fixture::sample_folder();
    let (galaxy, _) =
        nova_galaxy_format::load::load(&nova_galaxy_format::source::DirSource::new(&folder));
    let size = galaxy.ships[&id("light-freighter")]
        .data
        .size
        .expect("the freighter's size");
    #[allow(clippy::cast_possible_truncation)]
    let px = (size.collision_radius * PX_PER_METRE).round() as i16;
    let game = new_game::open(&folder).expect("lists");
    for ship in ["light-freighter", "harrow-gunship"] {
        assert_eq!(
            game.data
                .collision_radius(ShipId(game.ids.ships[&id(ship)])),
            Some(px),
            "{ship}"
        );
    }
}

/// The size box `beam` by `length` metres, at `PX_PER_METRE`, in `color`.
fn size_box(length: f64, beam: f64, color: [u8; 3]) -> StandIn {
    let size = Size {
        length,
        beam,
        collision_radius: 0.0,
    };
    StandIn::Outline(nova_galaxy_media::size_box(size, color).scaled(PX_PER_METRE))
}

#[test]
fn each_hull_draws_as_its_size_box_on_its_placeholder_model() {
    // Both hulls show `hauler-model`, a placeholder with no files.
    let game = new_game::open(&fixture()).expect("lists");
    let ship = |nova| game.data.stand_in(StandInOf::Ship(ShipId(nova))).cloned();
    assert_eq!(
        ship(129),
        Some(size_box(30.0, 12.0, [0xc0, 0x80, 0x40])),
        "the hauler in guild paint"
    );
    assert_eq!(
        ship(128),
        Some(size_box(24.0, 10.0, BARE)),
        "the cutter, bare"
    );
}

#[test]
fn each_stellar_draws_as_a_disc_and_each_gate_as_a_ring() {
    let game = new_game::open(&fixture()).expect("lists");
    let stellar = |id: &str| {
        let nova =
            game.ids.stellars[&nova_galaxy_format::id::Id::try_from(id.to_owned()).expect("an id")];
        game.data
            .stand_in(StandInOf::Stellar(StellarId(nova)))
            .cloned()
    };
    assert_eq!(
        stellar("hearth-dock"),
        Some(StandIn::Disc {
            color: [0xc0, 0x80, 0x40]
        }),
        "in the guild's colour"
    );
    assert_eq!(stellar("hearth-moon"), Some(StandIn::Disc { color: BARE }));
    assert_eq!(
        stellar("verge-camp"),
        Some(StandIn::Disc {
            color: [0x33, 0x66, 0xcc]
        }),
        "in the wardens' colour"
    );
    assert!(!game.ids.gates.is_empty());
    for &nova in game.ids.gates.values() {
        assert_eq!(
            game.data.stand_in(StandInOf::Stellar(StellarId(nova))),
            Some(&StandIn::Ring { color: BARE })
        );
    }
}

#[test]
fn the_start_is_a_char_in_the_hauler_in_hearth() {
    let game = new_game::open(&fixture()).expect("lists");
    assert_eq!(pairs(&game.ids.starts), [("dock-hand".to_owned(), 128)]);
    let (name, start) = named::<Character>(&game.data, 128);
    assert_eq!(name, "Dock hand");
    assert_eq!(start.ship_type, Some(ShipId(129)));
    assert_eq!(start.system[0], Some(SystemId(129)));
    assert_eq!(start.cash, 10_000);
}

#[test]
fn a_stellar_s_body_is_its_landing_text() {
    let data = new_game::open(&fixture()).expect("lists").data;
    let (_, dock) = named::<Desc>(&data, 128);
    assert_eq!(
        dock.text.as_str(),
        "The Guild's great dock.\n\nShips are built and sold here."
    );
}

#[test]
fn the_dock_pass_is_an_outfit_the_dock_job_lends_and_takes_back() {
    let game = new_game::open(&fixture()).expect("lists");
    assert_eq!(
        pairs(&game.ids.outfits),
        [
            ("dock-pass".to_owned(), 128),
            ("mass-driver".to_owned(), 130)
        ]
    );
    let (name, pass) = named::<Outfit>(&game.data, 128);
    assert_eq!(name, "Dock pass");
    assert_eq!(pass.cost, 0);
    assert_eq!(
        pass.require.bits(),
        nova_data::new_game::kits::FitBits::mount_bit(nova_galaxy_format::fleet::Mount::Fixed),
        "its `fixed` mount"
    );
    let job = &game.data.storylines().missions["dock-job"];
    assert_eq!(job.on_accept.grant_outfits, [nova_data::OutfitId(128)]);
    assert_eq!(job.on_accept.remove_outfits, []);
    assert_eq!(job.on_success.grant_outfits, []);
    assert_eq!(job.on_success.remove_outfits, [nova_data::OutfitId(128)]);
}

#[test]
fn the_ids_map_back_to_the_bible_s() {
    let ids = new_game::open(&fixture()).expect("lists").ids;
    assert_eq!(
        pairs(&ids.stellars),
        [
            ("hearth-dock".to_owned(), 128),
            ("hearth-moon".to_owned(), 129),
            ("verge-camp".to_owned(), 130),
            ("verge-workshop".to_owned(), 140)
        ]
    );
    assert_eq!(
        pairs(&ids.systems),
        [
            ("drift".to_owned(), 128),
            ("hearth".to_owned(), 129),
            ("verge".to_owned(), 130)
        ]
    );
}

#[test]
fn the_sample_folder_is_the_crates_own() {
    assert_eq!(
        new_game::fixture::sample_folder(),
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("sample")
    );
}

#[test]
fn each_fixture_holds_a_table_pinning_exactly_what_it_maps() {
    for folder in [fixture(), new_game::fixture::sample_folder()] {
        let bytes = fs::read(folder.join(new_game::pins::FILE)).expect("the folder's ID table");
        let table = new_game::pins::parse(&bytes).expect("parses");
        assert_eq!(table.problems, Vec::<String>::new(), "{}", folder.display());
        let game = new_game::open(&folder).expect("lists");
        assert_eq!(table.pins, game.ids, "{}", folder.display());
        assert_eq!(
            new_game::pins::text(&table.pins).into_bytes(),
            bytes,
            "{}: as the pin command writes it",
            folder.display()
        );
    }
}

#[test]
fn the_sample_folder_loads_without_problems() {
    let game = new_game::open(&new_game::fixture::sample_folder()).expect("lists");
    let problems: Vec<_> = game.problems.iter().map(ToString::to_string).collect();
    assert_eq!(problems, Vec::<String>::new());
    assert_eq!(game.lfs_pointers, Vec::<String>::new());
}

#[test]
fn the_sample_s_stars_stand_in_as_dots_of_one_to_three_pixels_in_several_greys() {
    let game = new_game::open(&new_game::fixture::sample_folder()).expect("lists");
    let dots = game.data.star_dots();
    assert!(!dots.is_empty());
    assert!(
        dots.iter().all(|dot| (1..=3).contains(&dot.side)),
        "{dots:?}"
    );
    assert!(dots.iter().any(|dot| dot.grey != dots[0].grey), "{dots:?}");
}

#[test]
fn the_sample_maps_four_missions_one_news_item_and_six_jump_points() {
    let game = new_game::open(&new_game::fixture::sample_folder()).expect("lists");
    let storylines = game.data.storylines();
    let keys = |ids: Vec<&String>| ids.into_iter().cloned().collect::<Vec<_>>();
    assert_eq!(
        keys(storylines.missions.keys().collect()),
        [
            "supply-run",
            "take-the-point",
            "the-crossroads-offer",
            "the-first-failure"
        ]
    );
    assert_eq!(
        keys(storylines.news.keys().collect()),
        ["luyten-road-fails"]
    );
    assert_eq!(
        keys(storylines.points.keys().collect()),
        [
            "brightwater/luyten-road",
            "brightwater/marrow-gate",
            "luyten-crossing/association-road",
            "luyten-crossing/marrow-road",
            "marrow/far-gate",
            "marrow/luyten-road"
        ]
    );
    let failure = &storylines.missions["the-first-failure"];
    assert_eq!(failure.offer, None, "an event");
    assert_eq!(failure.requires.not_before_day, Some(60));
    assert_eq!(
        storylines.missions["take-the-point"].objective,
        Some(nova_data::MissionObjective::TakePoint {
            point: "marrow/far-gate".to_owned(),
            held_by: "harrow-band".to_owned(),
            blockade: Some(nova_data::MissionId(
                game.ids.blockades[&id("take-the-point")]
            )),
        })
    );
    assert_eq!(
        storylines.news["luyten-road-fails"].from,
        SystemId(game.ids.systems[&id("luyten-crossing")])
    );
}

#[test]
fn a_missing_root_cannot_be_listed() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let gone = dir.path().join("gone");
    let error = new_game::open(&gone).expect_err("missing");
    assert_eq!(error.path, gone);
    assert_eq!(error.source.kind(), std::io::ErrorKind::NotFound);
    assert!(
        error
            .to_string()
            .starts_with(&format!("listing {}: ", gone.display())),
        "{error}"
    );
}

#[test]
fn a_root_that_is_a_file_cannot_be_listed() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let file = dir.path().join("data.md");
    fs::write(&file, "not a folder").expect("writes");
    let error = new_game::open(&file).expect_err("a file");
    assert_eq!(error.path, file);
}

/// Copies the folder `from` into `to`, recursively.
fn copy(from: &Path, to: &Path) {
    fs::create_dir_all(to).expect("creates");
    for entry in fs::read_dir(from).expect("lists") {
        let entry = entry.expect("an entry");
        let target = to.join(entry.file_name());
        if entry.file_type().expect("a type").is_dir() {
            copy(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).expect("copies");
        }
    }
}

#[test]
fn a_non_utf8_system_file_is_reported_and_the_good_systems_still_map() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let root = dir.path().join("data");
    copy(&fixture(), &root);
    fs::write(root.join("systems").join("garbled.md"), b"---\xff\xfe\n").expect("writes");
    let game = new_game::open(&root).expect("lists");
    assert_eq!(game.data.ids(System::TYPE), [128, 129, 130]);
    // The format's relative path, written with `/` on every OS.
    assert!(
        game.problems.iter().any(|p| p.file.contains("garbled.md")),
        "{:?}",
        game.problems
    );
}

#[test]
fn media_holding_lfs_pointers_are_reported_and_the_data_still_loads() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let root = dir.path().join("data");
    copy(&fixture(), &root);
    fs::create_dir_all(root.join("assets")).expect("creates");
    fs::write(
        root.join("assets").join("hull.md"),
        "---\nkind: model\nplaceholder: true\nfiles: [media/export/pack/hull.glb, \
         media/export/pack/real.glb]\n---\n",
    )
    .expect("writes");
    let pack = root.join("media").join("export").join("pack");
    fs::create_dir_all(&pack).expect("creates");
    fs::write(
        pack.join("hull.glb"),
        "version https://git-lfs.github.com/spec/v1\noid sha256:00\nsize 1\n",
    )
    .expect("writes");
    fs::write(pack.join("real.glb"), b"glTF\x02").expect("writes");
    let game = new_game::open(&root).expect("lists");
    assert_eq!(game.lfs_pointers, ["media/export/pack/hull.glb"]);
    assert_eq!(game.data.ids(System::TYPE), [128, 129, 130]);
    assert!(game.problems.is_empty(), "{:?}", game.problems);
}
