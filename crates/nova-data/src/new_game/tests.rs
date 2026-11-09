//! The mapping, over the format's test galaxy (`fixture::galaxy`: one
//! faction, `league`; systems `alpha`, governed by it, and `beta`, linked
//! by the jump points `alpha/to-beta` at [100, 0] and `beta/to-alpha` at
//! [-100, 0], which `league` holds; stellars `alpha-prime` and
//! `alpha-yard`, a station it owns, both in `alpha`; the civilian hull
//! `skiff` and its variant `armed-skiff`; and the start `first-flight` in
//! the skiff in `alpha`) broken one file at a time.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use nova_galaxy_format::fixture;
use nova_galaxy_format::source::MemSource;

use super::fleet::{PX_PER_METRE, START_DATE, TRAFFIC_SHIPS, maneuver};
use super::ids::{CIVILIAN_DUDE, DEFAULT_INTERFACE};
use super::kits::{
    LAW_SCAN_FINE, LAW_SMUGGLE_PENALTY, LAWLESS_KIT_SCAN, YARD_TECH_LAWLESS, YARD_TECH_LEGAL,
};
use super::records::{
    ANY_RECORD, OUTFIT_TECH_NEVER, default_interface, neutral_character, neutral_dude,
    neutral_govt, neutral_ship, neutral_ship_anim, neutral_stellar,
};
use super::*;
use crate::decode::Record;
use crate::records::character::Character;
use crate::records::desc::Desc;
use crate::records::dude::Dude;
use crate::records::govt::Govt;
use crate::records::interface::Interface;
use crate::records::ship::Ship;
use crate::records::ship_anim::ShipAnim;
use crate::records::stellar::{Stellar, flags, flags2};
use crate::records::system::System;
use crate::wire::flags::{Flags16, Flags32};
use crate::wire::id::{GovtId, OutfitId, ShipId, StellarId, SystemId, WeaponId};
use nova_galaxy_format::id::PointRef;

/// The format's test galaxy with its mission's story compiled
/// (`dialog/errands.ink.json`), so it maps without problems.
pub(super) fn galaxy() -> MemSource {
    fixture::galaxy().with("dialog/errands.ink.json", "{errands}")
}

fn dir() -> PathBuf {
    Path::new("games").join("sample")
}

fn loaded(source: &MemSource) -> NewGame {
    load(source, &dir())
}

fn id(text: &str) -> nova_galaxy_format::id::Id {
    nova_galaxy_format::id::Id::try_from(text.to_owned()).expect("an id")
}

fn system(game: &NewGame, nova: i16) -> System {
    game.data
        .get::<System>(nova)
        .expect("present")
        .expect("decoded")
        .record
        .clone()
}

fn stellar(game: &NewGame, nova: i16) -> Stellar {
    game.data
        .get::<Stellar>(nova)
        .expect("present")
        .expect("decoded")
        .record
        .clone()
}

/// Record `nova` of type `T`, and its name.
fn named<T: crate::Registered + Clone>(game: &NewGame, nova: i16) -> (String, T) {
    let entry = game.data.get::<T>(nova).expect("present").expect("decoded");
    (entry.name.expect("named").to_owned(), entry.record.clone())
}

fn point(text: &str) -> PointRef {
    PointRef::try_from(text.to_owned()).expect("a point")
}

/// The problems as `file: message` lines.
fn problems(game: &NewGame) -> Vec<String> {
    game.problems.iter().map(ToString::to_string).collect()
}

/// The fixture galaxy with `alpha-prime` offering `services`.
fn alpha_prime_offering(services: &str) -> MemSource {
    galaxy().with(
        "stellars/alpha-prime.md",
        &format!(
            "---\nname: Alpha Prime\nkind: planet\nsystem: alpha\nposition: [50, 0]\n\
             services: [{services}]\n---\n"
        ),
    )
}

fn alpha_prime_flags(services: &str) -> u32 {
    stellar(&loaded(&alpha_prime_offering(services)), 128)
        .flags
        .bits()
}

#[test]
fn the_fixture_maps_without_problems() {
    let game = loaded(&galaxy());
    assert_eq!(problems(&game), Vec::<String>::new());
    assert_eq!(game.lfs_pointers, Vec::<String>::new());
}

#[test]
fn ids_count_up_from_128_in_id_order_per_kind_and_the_table_maps_back() {
    let game = loaded(&galaxy());
    assert_eq!(game.data.ids(Govt::TYPE), [128]);
    assert_eq!(game.data.ids(System::TYPE), [128, 129]);
    assert_eq!(game.data.ids(Stellar::TYPE), [128, 129, 130, 131]);
    let table = |map: &std::collections::BTreeMap<nova_galaxy_format::id::Id, i16>| {
        map.iter()
            .map(|(id, &n)| (id.to_string(), n))
            .collect::<Vec<_>>()
    };
    assert_eq!(table(&game.ids.factions), [("league".to_owned(), 128)]);
    assert_eq!(
        table(&game.ids.systems),
        [("alpha".to_owned(), 128), ("beta".to_owned(), 129)]
    );
    assert_eq!(
        table(&game.ids.stellars),
        [
            ("alpha-prime".to_owned(), 128),
            ("alpha-yard".to_owned(), 129)
        ]
    );
}

#[test]
fn every_record_comes_from_the_folder() {
    let game = loaded(&galaxy());
    let alpha = game
        .data
        .get::<System>(128)
        .expect("present")
        .expect("decoded");
    assert_eq!(alpha.source.path, dir());
}

#[test]
fn a_faction_becomes_a_govt_with_its_name_and_colour() {
    let game = loaded(&galaxy());
    let league = game
        .data
        .get::<Govt>(128)
        .expect("present")
        .expect("decoded");
    assert_eq!(league.name, Some("League"));
    let expected = Govt {
        color: 0x00AA_8800,
        ship_color: 0x00AA_8800,
        smug_penalty: LAW_SMUGGLE_PENALTY,
        scan_fine: LAW_SCAN_FINE,
        scan_mask: Flags16(LAWLESS_KIT_SCAN),
        ..neutral_govt()
    };
    assert_eq!(*league.record, expected, "it keeps the law");
    assert_eq!((LAW_SMUGGLE_PENALTY, LAW_SCAN_FINE), (10, 500));
}

/// The fixture galaxy with `league` lawless.
fn lawless_league() -> MemSource {
    galaxy().with(
        "factions/league.md",
        "---\nname: League\nkind: minor\ncolor: \"#aa8800\"\nlawless: true\n---\n",
    )
}

#[test]
fn a_lawless_faction_s_govt_is_neutral_but_its_colour() {
    let game = loaded(&lawless_league());
    let (_, league): (_, Govt) = named(&game, 128);
    let expected = Govt {
        color: 0x00AA_8800,
        ship_color: 0x00AA_8800,
        ..neutral_govt()
    };
    assert_eq!(league, expected);
}

#[test]
fn a_law_keeping_owner_s_stellar_lists_legal_kits_and_clears_by_record() {
    let yard = stellar(&loaded(&galaxy()), 129);
    assert_eq!((yard.tech_level, yard.min_status), (YARD_TECH_LEGAL, 0));
    assert_eq!(YARD_TECH_LEGAL, 1);
}

#[test]
fn a_lawless_or_missing_owner_s_stellar_lists_every_kit_and_clears_any_record() {
    let game = loaded(&lawless_league());
    let yard = stellar(&game, 129);
    assert_eq!(
        (yard.tech_level, yard.min_status),
        (YARD_TECH_LAWLESS, ANY_RECORD)
    );
    let prime = stellar(&loaded(&galaxy()), 128);
    assert_eq!(prime.govt, None);
    assert_eq!(
        (prime.tech_level, prime.min_status),
        (YARD_TECH_LAWLESS, ANY_RECORD)
    );
    assert_eq!(YARD_TECH_LAWLESS, 2);
}

#[test]
fn a_stellar_whose_owner_names_no_faction_is_lawless() {
    let source = galaxy().with(
        "stellars/alpha-yard.md",
        "---\nname: Alpha Yard\nkind: station\nsystem: alpha\nposition: [0, 0]\nowner: guild\n\
         visual: {asset: spire}\n---\n",
    );
    let yard = stellar(&loaded(&source), 129);
    assert_eq!(
        (yard.tech_level, yard.min_status),
        (YARD_TECH_LAWLESS, ANY_RECORD)
    );
}

#[test]
fn a_system_becomes_a_syst_with_its_name_position_government_and_stellars() {
    let game = loaded(&galaxy());
    let alpha = game
        .data
        .get::<System>(128)
        .expect("present")
        .expect("decoded");
    assert_eq!(alpha.name, Some("Alpha"));
    assert_eq!((alpha.record.x_pos, alpha.record.y_pos), (0, 0));
    assert_eq!(alpha.record.govt, Some(GovtId(128)));
    let mut nav_def = [None; 16];
    nav_def[0] = Some(StellarId(128));
    nav_def[1] = Some(StellarId(129));
    nav_def[2] = Some(StellarId(130));
    assert_eq!(alpha.record.nav_def, nav_def, "its stellars, then its gate");
    assert_eq!(alpha.record.con, [None; 16], "no hyperspace links");

    let beta = system(&game, 129);
    assert_eq!((beta.x_pos, beta.y_pos), (200, 50));
    assert_eq!(beta.govt, None, "ungoverned");
    let mut nav_def = [None; 16];
    nav_def[0] = Some(StellarId(131));
    assert_eq!(beta.nav_def, nav_def, "no stellars, its gate");
    assert_eq!(beta.con, [None; 16]);
}

#[test]
fn a_stellar_becomes_a_spob_with_its_name_position_and_owner() {
    let game = loaded(&galaxy());
    let yard = game
        .data
        .get::<Stellar>(129)
        .expect("present")
        .expect("decoded");
    assert_eq!(yard.name, Some("Alpha Yard"));
    assert_eq!((yard.record.x_pos, yard.record.y_pos), (0, 0));
    assert_eq!(yard.record.govt, Some(GovtId(128)));
    assert_eq!(yard.record.hyper_link, [None; 8]);
    let prime = stellar(&game, 128);
    assert_eq!((prime.x_pos, prime.y_pos), (50, 0));
    assert_eq!(prime.govt, None, "no owner");
}

#[test]
fn a_bar_sets_its_bit_and_landing() {
    let bits = alpha_prime_flags("bar, refuel");
    assert_eq!(bits, flags::CAN_LAND | flags::BAR);
}

#[test]
fn a_commodity_exchange_sets_its_bit_and_landing() {
    let bits = alpha_prime_flags("trade, refuel");
    assert_eq!(bits, flags::CAN_LAND | flags::TRADE_CENTER);
}

#[test]
fn an_outfitter_sets_its_bit_and_landing() {
    let bits = alpha_prime_flags("outfitter, refuel");
    assert_eq!(bits, flags::CAN_LAND | flags::OUTFITTER);
}

#[test]
fn a_shipyard_sets_its_bit_and_landing() {
    let bits = alpha_prime_flags("shipyard, refuel");
    assert_eq!(bits, flags::CAN_LAND | flags::SHIPYARD);
}

#[test]
fn refuel_alone_is_landing_and_inhabited() {
    assert_eq!(alpha_prime_flags("refuel"), flags::CAN_LAND);
}

#[test]
fn a_station_sets_the_station_bit() {
    let game = loaded(&galaxy());
    let yard = stellar(&game, 129);
    assert_eq!(yard.flags.bits(), flags::STATION | flags::UNINHABITED);
}

#[test]
fn a_planet_without_refuel_is_uninhabited() {
    assert_eq!(alpha_prime_flags(""), flags::UNINHABITED, "no services");
    assert_eq!(
        alpha_prime_flags("bar"),
        flags::CAN_LAND | flags::BAR | flags::UNINHABITED
    );
}

#[test]
fn a_stellar_s_body_is_its_landing_text_at_its_id() {
    let source = galaxy().with(
        "stellars/alpha-yard.md",
        "---\nname: Alpha Yard\nkind: station\nsystem: alpha\nposition: [0, 0]\n---\n\n\
         A yard.\n\nIt builds ships.\n\n",
    );
    let game = loaded(&source);
    let desc = game
        .data
        .get::<Desc>(129)
        .expect("present")
        .expect("decoded");
    assert_eq!(desc.record.text.as_str(), "A yard.\n\nIt builds ships.");
    assert_eq!(desc.record.graphic, None);
    assert_eq!(
        game.data.ids(Desc::TYPE),
        [129],
        "none for a stellar without a body"
    );
}

#[test]
fn a_blank_body_has_no_text() {
    let source = galaxy().with(
        "stellars/alpha-yard.md",
        "---\nname: Alpha Yard\nkind: station\nsystem: alpha\nposition: [0, 0]\n---\n \n\n",
    );
    assert_eq!(loaded(&source).data.ids(Desc::TYPE), Vec::<i16>::new());
}

#[test]
fn no_system_has_a_hyperspace_link() {
    let game = loaded(&galaxy());
    for (_, entry) in game.data.records::<System>() {
        assert_eq!(entry.expect("decoded").record.con, [None; 16]);
    }
}

// Jump points.

#[test]
fn each_jump_point_is_a_gate_numbered_after_the_last_stellar_in_system_and_point_order() {
    let game = loaded(&galaxy());
    assert_eq!(
        game.ids
            .gates
            .iter()
            .map(|(p, &n)| (p.to_string(), n))
            .collect::<Vec<_>>(),
        [
            ("alpha/to-beta".to_owned(), 130),
            ("beta/to-alpha".to_owned(), 131)
        ]
    );
    let source = galaxy().with(
        "systems/alpha.md",
        "---\nname: Alpha\nregion: rim\nmap: [0, 0]\njump_points:\n\
         \x20 - id: zed\n    position: [0, 0]\n    to: beta/to-alpha\n    stability: stable\n\
         \x20 - id: to-beta\n    position: [100, 0]\n    to: beta/to-alpha\n    stability: stable\n\
         ---\n",
    );
    let game = loaded(&source);
    let gate_of = |p: &str| game.ids.gates[&point(p)];
    assert_eq!(
        (
            gate_of("alpha/to-beta"),
            gate_of("alpha/zed"),
            gate_of("beta/to-alpha")
        ),
        (130, 131, 132),
        "by point id within a system, not as written"
    );
}

#[test]
fn adding_gates_renumbers_no_stellar() {
    let pointless = galaxy()
        .with(
            "systems/alpha.md",
            "---\nname: Alpha\nregion: rim\nmap: [0, 0]\n---\n",
        )
        .with(
            "systems/beta.md",
            "---\nname: Beta\nregion: rim\nmap: [200, 50]\n---\n",
        );
    let without = loaded(&pointless);
    assert!(without.ids.gates.is_empty());
    assert_eq!(loaded(&galaxy()).ids.stellars, without.ids.stellars);
}

#[test]
fn a_gate_sits_where_its_point_does() {
    let game = loaded(&galaxy());
    let (_, to_beta): (_, Stellar) = named(&game, 130);
    assert_eq!((to_beta.x_pos, to_beta.y_pos), (100, 0));
    let (_, to_alpha): (_, Stellar) = named(&game, 131);
    assert_eq!((to_alpha.x_pos, to_alpha.y_pos), (-100, 0));
}

#[test]
fn a_gate_is_owned_by_its_point_s_owner() {
    let game = loaded(&galaxy());
    assert_eq!(stellar(&game, 130).govt, None);
    assert_eq!(stellar(&game, 131).govt, Some(GovtId(128)));
}

#[test]
fn a_gate_can_be_landed_on_and_is_a_hypergate() {
    let game = loaded(&galaxy());
    let to_beta = stellar(&game, 130);
    assert_eq!(to_beta.flags, Flags32(flags::CAN_LAND));
    assert_eq!(to_beta.flags2, Flags16(flags2::HYPERGATE));
    assert_eq!(flags2::HYPERGATE, 0x1000);
}

#[test]
fn a_gate_links_to_its_partner_s_gate_alone() {
    let game = loaded(&galaxy());
    let mut links = [None; 8];
    links[0] = Some(StellarId(131));
    assert_eq!(stellar(&game, 130).hyper_link, links);
    links[0] = Some(StellarId(130));
    assert_eq!(stellar(&game, 131).hyper_link, links);
}

#[test]
fn a_gate_is_named_for_its_partner_s_system() {
    let game = loaded(&galaxy());
    assert_eq!(named::<Stellar>(&game, 130).0, "Beta");
    assert_eq!(named::<Stellar>(&game, 131).0, "Alpha");
}

#[test]
fn a_gate_heads_ships_out_at_random_and_sets_nothing_else() {
    let game = loaded(&galaxy());
    let mut hyper_link = [None; 8];
    hyper_link[0] = Some(StellarId(131));
    let expected = Stellar {
        x_pos: 100,
        y_pos: 0,
        flags: Flags32(flags::CAN_LAND),
        flags2: Flags16(flags2::HYPERGATE),
        hyper_link,
        min_status: ANY_RECORD,
        ..neutral_stellar()
    };
    assert_eq!(stellar(&game, 130), expected);
    assert_eq!(expected.cust_snd_id, -1, "a random exit heading");
}

#[test]
fn a_gate_clears_any_record() {
    let game = loaded(&galaxy());
    for gate in [130, 131] {
        assert_eq!(stellar(&game, gate).min_status, ANY_RECORD, "{gate}");
    }
    assert_eq!(ANY_RECORD, -32767);
}

/// The fixture galaxy with `gamma`, holding the one-way point `drop` to
/// `alpha/to-beta`.
fn with_a_drop() -> MemSource {
    galaxy().with(
        "systems/gamma.md",
        "---\nname: Gamma\nregion: rim\nmap: [0, 300]\njump_points:\n\
         \x20 - id: drop\n    position: [0, -1300]\n    to: alpha/to-beta\n    stability: unstable\n\
         \x20   one_way: true\n---\n",
    )
}

#[test]
fn a_one_way_point_links_to_its_partner_and_the_partner_links_where_it_says() {
    let game = loaded(&with_a_drop());
    assert_eq!(problems(&game), Vec::<String>::new());
    let gate_of = |p: &str| game.ids.gates[&point(p)];
    let drop = stellar(&game, gate_of("gamma/drop"));
    assert_eq!(
        drop.hyper_link[0],
        Some(StellarId(gate_of("alpha/to-beta")))
    );
    let to_beta = stellar(&game, gate_of("alpha/to-beta"));
    assert_eq!(
        to_beta.hyper_link[0],
        Some(StellarId(gate_of("beta/to-alpha")))
    );
    assert_eq!((drop.x_pos, drop.y_pos), (0, -1300), "past 999, kept");
}

#[test]
fn a_point_leading_to_no_point_is_a_problem_and_its_gate_has_no_link() {
    for to in ["alpha/nowhere", "gamma/to-beta"] {
        let source = galaxy().with(
            "systems/beta.md",
            &format!(
                "---\nname: Beta\nregion: rim\nmap: [200, 50]\njump_points:\n\
                 \x20 - id: to-alpha\n    position: [-100, 0]\n    to: {to}\n    stability: stable\n\
                 ---\n"
            ),
        );
        let game = loaded(&source);
        assert_eq!(
            problems(&game),
            [format!(
                "systems/beta.md: its jump point `to-alpha` leads to `{to}`, which is no jump \
                 point; its gate has no link"
            )]
        );
        assert_eq!(stellar(&game, 131).hyper_link, [None; 8], "{to}");
    }
}

#[test]
fn a_gates_partner_in_a_system_that_failed_to_load_is_not_reported_again() {
    let source = galaxy().with("systems/alpha.md", "---\nname: Alpha\n---\n");
    let game = loaded(&source);
    let lines = problems(&game);
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(lines[0].starts_with("systems/alpha.md: "), "{lines:?}");
    assert_eq!(stellar(&game, 128).hyper_link, [None; 8], "beta's gate");
}

#[test]
fn a_full_system_leaves_out_its_stellars_past_the_slots_before_a_gate() {
    let game = loaded(&beta_holding(16));
    assert_eq!(
        problems(&game),
        [
            "systems/beta.md: it has 16 stellars and 1 jump point, more than a system's 16 nav \
             slots; `b15` has no place in it"
        ]
    );
    let beta = system(&game, 129);
    let to_alpha = game.ids.gates[&point("beta/to-alpha")];
    assert_eq!(beta.nav_def[15], Some(StellarId(to_alpha)), "the gate kept");
    assert_eq!(
        beta.nav_def[14],
        Some(StellarId(game.ids.stellars[&id("b14")]))
    );
}

#[test]
fn more_than_16_jump_points_keep_the_first_16_gates_and_no_stellar() {
    let points = (0..16).fold(String::new(), |mut points, n| {
        let _ = write!(
            points,
            "\x20 - id: p{n:02}\n    position: [{n}, 500]\n    to: alpha/to-beta\n    \
             stability: stable\n    one_way: true\n"
        );
        points
    });
    let source = beta_holding(1).with(
        "systems/beta.md",
        &format!(
            "---\nname: Beta\nregion: rim\nmap: [200, 50]\njump_points:\n\
             \x20 - id: to-alpha\n    position: [-100, 0]\n    to: alpha/to-beta\n    \
             stability: stable\n{points}---\n"
        ),
    );
    let game = loaded(&source);
    assert_eq!(
        problems(&game),
        [
            "systems/beta.md: it has 1 stellar and 17 jump points, more than a system's 16 nav \
             slots; `b00`, `beta/to-alpha` have no place in it"
        ]
    );
    let beta = system(&game, game.ids.systems[&id("beta")]);
    let gates: Vec<Option<StellarId>> = (0..16)
        .map(|n| Some(StellarId(game.ids.gates[&point(&format!("beta/p{n:02}"))])))
        .collect();
    assert_eq!(
        beta.nav_def.to_vec(),
        gates,
        "the first 16 gates, in ID order"
    );
    let b00 = StellarId(game.ids.stellars[&id("b00")]);
    assert!(!beta.nav_def.contains(&Some(b00)), "no stellar kept");
}

#[test]
fn a_government_naming_no_faction_is_a_problem_and_no_government() {
    let source = galaxy().with(
        "systems/beta.md",
        "---\nname: Beta\nregion: rim\ngovernment: nowhere\nmap: [200, 50]\njump_points:\n\
         \x20 - id: to-alpha\n    position: [-100, 0]\n    to: alpha/to-beta\n    stability: stable\n---\n",
    );
    let game = loaded(&source);
    assert_eq!(
        problems(&game),
        ["systems/beta.md: the faction `nowhere` is not in factions/; it has no government"]
    );
    assert_eq!(system(&game, 129).govt, None);
}

#[test]
fn an_owner_naming_no_faction_is_a_problem_and_no_government() {
    let source = galaxy().with(
        "stellars/alpha-yard.md",
        "---\nname: Alpha Yard\nkind: station\nsystem: alpha\nposition: [0, 0]\nowner: nobody\n---\n",
    );
    let game = loaded(&source);
    assert_eq!(
        problems(&game),
        ["stellars/alpha-yard.md: the faction `nobody` is not in factions/; it has no government"]
    );
    assert_eq!(stellar(&game, 129).govt, None);
}

#[test]
fn a_faction_that_failed_to_load_is_not_reported_again() {
    let source = galaxy().with("factions/league.md", "---\nname: League\n---\n");
    let game = loaded(&source);
    let lines = problems(&game);
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(lines[0].starts_with("factions/league.md: "), "{lines:?}");
    assert_eq!(system(&game, 128).govt, None);
    assert_eq!(stellar(&game, 129).govt, None);
}

#[test]
fn a_stellar_in_no_system_is_a_problem_and_left_out() {
    let source = galaxy().with(
        "stellars/alpha-yard.md",
        "---\nname: Alpha Yard\nkind: station\nsystem: gamma\nposition: [0, 0]\n---\n",
    );
    let game = loaded(&source);
    assert_eq!(
        problems(&game),
        ["stellars/alpha-yard.md: its system `gamma` is not in systems/; it is left out"]
    );
    assert_eq!(
        game.data.ids(Stellar::TYPE),
        [128, 129, 130],
        "alpha-prime and the gates"
    );
    assert!(!game.ids.stellars.contains_key(&id("alpha-yard")));
    let mut nav_def = [None; 16];
    nav_def[0] = Some(StellarId(128));
    nav_def[1] = Some(StellarId(129));
    assert_eq!(system(&game, 128).nav_def, nav_def);
}

#[test]
fn a_stellar_in_a_system_that_failed_to_load_is_left_out_unreported() {
    let source = galaxy().with("systems/alpha.md", "---\nname: Alpha\n---\n");
    let game = loaded(&source);
    let lines = problems(&game);
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(lines[0].starts_with("systems/alpha.md: "), "{lines:?}");
    assert_eq!(game.data.ids(Stellar::TYPE), [128], "beta's gate alone");
    assert_eq!(game.data.ids(System::TYPE), [128], "beta");
}

/// The fixture galaxy loaded with `systems/` marked as a folder that
/// failed to list, as the format's loader leaves it.
pub(super) fn with_systems_unlisted(source: &MemSource) -> nova_galaxy_format::load::Galaxy {
    let (mut galaxy, _) = nova_galaxy_format::load::load(source);
    galaxy.systems.clear();
    galaxy.unreadable.insert("systems/".to_owned());
    galaxy
}

#[test]
fn with_systems_unlisted_no_reference_into_it_is_reported() {
    let mapped = map(&with_systems_unlisted(&galaxy()));
    let lines: Vec<_> = mapped.problems.iter().map(ToString::to_string).collect();
    assert_eq!(
        lines,
        Vec::<String>::new(),
        "the stellars' and start's systems"
    );
}

#[test]
fn a_map_position_outside_i16_is_a_problem_and_moved_to_the_nearest() {
    let source = galaxy().with(
        "systems/beta.md",
        "---\nname: Beta\nregion: rim\nmap: [40000, -40000]\njump_points:\n\
         \x20 - id: to-alpha\n    position: [-100, 0]\n    to: alpha/to-beta\n    stability: stable\n---\n",
    );
    let game = loaded(&source);
    assert_eq!(
        problems(&game),
        [
            "systems/beta.md: its map position [40000, -40000] is outside -32768 to 32767; it is \
             moved to [32767, -32768]"
        ]
    );
    let beta = system(&game, 129);
    assert_eq!((beta.x_pos, beta.y_pos), (32_767, -32_768));
}

#[test]
fn one_number_outside_i16_is_enough_for_a_problem() {
    for at in ["[40000, 0]", "[0, 40000]"] {
        let source = galaxy().with(
            "stellars/alpha-prime.md",
            &format!("---\nname: Alpha Prime\nkind: planet\nsystem: alpha\nposition: {at}\n---\n"),
        );
        let lines = problems(&loaded(&source));
        assert_eq!(lines.len(), 1, "{at}: {lines:?}");
        assert!(
            lines[0].starts_with("stellars/alpha-prime.md: its position "),
            "{lines:?}"
        );
    }
}

/// The fixture galaxy with `count` more stellars in `beta`, `b00` up.
fn beta_holding(count: usize) -> MemSource {
    (0..count).fold(galaxy(), |source, n| {
        source.with(
            &format!("stellars/b{n:02}.md"),
            &format!("---\nname: B{n}\nkind: moon\nsystem: beta\nposition: [{n}, 0]\n---\n"),
        )
    })
}

#[test]
fn fifteen_stellars_and_a_gate_fill_a_system() {
    let game = loaded(&beta_holding(15));
    assert_eq!(problems(&game), Vec::<String>::new());
    let beta = system(&game, 129);
    assert!(
        beta.nav_def.iter().all(Option::is_some),
        "{:?}",
        beta.nav_def
    );
}

#[test]
fn more_than_16_stellars_in_a_system_is_a_problem_and_the_rest_have_no_place() {
    let game = loaded(&beta_holding(18));
    assert_eq!(
        problems(&game),
        [
            "systems/beta.md: it has 18 stellars and 1 jump point, more than a system's 16 nav \
             slots; `b15`, `b16`, `b17` have no place in it"
        ]
    );
    let beta = system(&game, 129);
    let first: Vec<_> = beta.nav_def.iter().map(|s| s.map(|s| s.0)).collect();
    let b = |n: &str| game.ids.stellars[&id(n)];
    assert_eq!(first[0], Some(b("b00")));
    assert_eq!(first[14], Some(b("b14")));
    assert_eq!(
        game.data.ids(Stellar::TYPE).len(),
        22,
        "each still has a record, and the gates theirs"
    );
}

#[test]
fn a_faction_past_the_last_govt_id_is_a_problem_and_left_out() {
    let source = (0..256).fold(galaxy(), |source, n| {
        source.with(
            &format!("factions/f{n:03}.md"),
            "---\nname: F\nkind: minor\ncolor: \"#000000\"\n---\n",
        )
    });
    let game = loaded(&source);
    assert_eq!(
        problems(&game),
        ["factions/league.md: it is past the last gövt ID, 383; it is left out"]
    );
    assert_eq!(game.data.ids(Govt::TYPE).len(), 256);
    assert_eq!(system(&game, 128).govt, None, "not reported again");
}

#[test]
fn a_system_past_the_last_syst_id_is_a_problem_and_left_out_with_its_stellars() {
    let source = (0..2047).fold(
        galaxy().with(
            "stellars/beta-rock.md",
            "---\nname: Beta Rock\nkind: moon\nsystem: beta\nposition: [0, 0]\n---\n",
        ),
        |source, n| {
            source.with(
                &format!("systems/a{n:04}.md"),
                "---\nname: S\nregion: rim\nmap: [0, 0]\n---\n",
            )
        },
    );
    let game = loaded(&source);
    // `beta`'s stellar goes with it, without a second report.
    assert_eq!(
        problems(&game),
        ["systems/beta.md: it is past the last sÿst ID, 2175; it is left out"]
    );
    assert_eq!(game.data.ids(System::TYPE).len(), 2048);
    assert_eq!(game.ids.systems[&id("alpha")], 2175);
    assert!(!game.ids.stellars.contains_key(&id("beta-rock")));
    assert_eq!(
        game.data.ids(Stellar::TYPE).len(),
        3,
        "alpha's two and its gate"
    );
    assert_eq!(
        stellar(&game, game.ids.gates[&point("alpha/to-beta")]).hyper_link,
        [None; 8],
        "its partner went with beta"
    );
}

#[test]
fn a_stellar_past_the_last_spob_id_is_a_problem_and_left_out() {
    let source = (0..2047).fold(galaxy(), |source, n| {
        source.with(
            &format!("stellars/a{n:04}.md"),
            "---\nname: S\nkind: moon\nsystem: beta\nposition: [0, 0]\n---\n",
        )
    });
    let game = loaded(&source);
    let past: Vec<_> = problems(&game)
        .into_iter()
        .filter(|line| line.contains("past the last"))
        .collect();
    assert_eq!(
        past,
        [
            "stellars/alpha-yard.md: it is past the last spöb ID, 2175; it is left out",
            "systems/alpha.md: its jump point `to-beta` is past the last spöb ID, 2175; it is \
             left out",
            "systems/beta.md: its jump point `to-alpha` is past the last spöb ID, 2175; it is \
             left out"
        ]
    );
    assert_eq!(game.data.ids(Stellar::TYPE).len(), 2048);
    assert!(game.ids.gates.is_empty());
}

#[test]
fn load_problems_come_through_unchanged_before_the_mapping_s() {
    let source = galaxy()
        .with("systems/Gamma.md", "---\nname: G\n---\n")
        .with(
            "systems/beta.md",
            "---\nname: Beta\nregion: rim\ngovernment: nowhere\nmap: [200, 50]\njump_points:\n\
         \x20 - id: to-alpha\n    position: [-100, 0]\n    to: alpha/to-beta\n    stability: stable\n---\n",
        );
    let (_, load_problems) = nova_galaxy_format::load::load(&source);
    let game = loaded(&source);
    assert_eq!(game.problems[..1], load_problems[..]);
    assert_eq!(game.problems.len(), 2);
    assert_eq!(game.problems[1].file, "systems/beta.md");
}

#[test]
fn export_media_holding_lfs_pointers_are_listed_and_the_data_still_loads() {
    let pointer = b"version https://git-lfs.github.com/spec/v1\noid sha256:00\nsize 1\n";
    let source = galaxy()
        .with_bytes("media/export/pack/hull.glb", pointer)
        .with_bytes("media/export/pack/surface.png", b"\x89PNG");
    let game = loaded(&source);
    assert_eq!(game.lfs_pointers, ["media/export/pack/hull.glb"]);
    assert_eq!(game.data.ids(System::TYPE), [128, 129]);
}

// Ships.

/// The fixture galaxy with the skiff holding every stat.
fn skiff_with_stats() -> MemSource {
    galaxy().with(
        "ships/skiff.md",
        "---\nname: Skiff\nstyle: civilian\nrole: shuttle\nmodel: hull\n\
         size: {length: 8, beam: 4, collision_radius: 4}\n\
         stats: {hull: 50, shields: 30, speed: 300, handling: 60, cargo: 10}\n---\n",
    )
}

/// The ship `ship` of `game`.
fn ship_of(game: &NewGame, ship: &str) -> Ship {
    named::<Ship>(game, game.ids.ships[&id(ship)]).1
}

fn skiff(source: &MemSource) -> Ship {
    ship_of(&loaded(source), "skiff")
}

#[test]
fn each_ship_is_a_ship_numbered_from_128_with_its_name() {
    let game = loaded(&galaxy());
    assert_eq!(game.data.ids(Ship::TYPE), [128, 129]);
    assert_eq!(
        game.ids
            .ships
            .iter()
            .map(|(s, &n)| (s.to_string(), n))
            .collect::<Vec<_>>(),
        [("armed-skiff".to_owned(), 128), ("skiff".to_owned(), 129)],
        "a variant is a ship too"
    );
    assert_eq!(named::<Ship>(&game, 128).0, "Armed skiff");
    assert_eq!(named::<Ship>(&game, 129).0, "Skiff");
}

#[test]
fn a_ship_with_mods_but_no_base_maps_but_is_no_traffic() {
    let source = galaxy().with(
        "ships/kitted.md",
        "---\nname: Kitted\nstyle: civilian\nrole: shuttle\nmodel: hull\n\
         size: {length: 8, beam: 4, collision_radius: 4}\nmods: [gun-mount]\n---\n",
    );
    let game = loaded(&source);
    assert!(game.ids.ships.contains_key(&id("kitted")));
    let (_, dude): (_, Dude) = named(&game, CIVILIAN_DUDE);
    let skiff = Some(ShipId(game.ids.ships[&id("skiff")]));
    assert_eq!(dude.ship_type[..2], [skiff, None], "the skiff alone");
}

#[test]
fn a_hull_s_speed_is_its_speed() {
    assert_eq!(skiff(&skiff_with_stats()).speed, 300);
}

#[test]
fn a_hull_s_acceleration_is_its_speed() {
    assert_eq!(skiff(&skiff_with_stats()).accel, 300);
}

#[test]
fn a_hull_s_turn_rate_is_its_handling_on_the_turn_scale() {
    assert_eq!(skiff(&skiff_with_stats()).maneuver, 60);
}

#[test]
fn the_turn_scale_reads_handling_as_maneuver_so_every_bible_hull_turns_above_the_floor() {
    // `Maneuver` 10 is the player's 1° a tick floor: the bible's hulls,
    // handling 15 to 85, all turn above it, and a kit's -10 shows.
    assert_eq!(
        (maneuver(15), maneuver(35), maneuver(85), maneuver(-10)),
        (15, 35, 85, -10)
    );
}

#[test]
fn a_hull_s_shield_is_its_shields() {
    assert_eq!(skiff(&skiff_with_stats()).shield, 30);
}

#[test]
fn a_hull_s_armour_is_its_hull() {
    assert_eq!(skiff(&skiff_with_stats()).armor, 50);
}

#[test]
fn a_hull_s_holds_are_its_cargo() {
    assert_eq!(skiff(&skiff_with_stats()).holds, 10);
}

#[test]
fn a_hull_holds_300_fuel_and_a_crew_of_one() {
    let ship = skiff(&skiff_with_stats());
    assert_eq!((ship.fuel, ship.crew), (300, 1));
}

#[test]
fn a_hull_sets_nothing_else() {
    let expected = Ship {
        speed: 300,
        accel: 300,
        maneuver: 60,
        shield: 30,
        armor: 50,
        holds: 10,
        fuel: 300,
        crew: 1,
        contribute: crate::Flags64(1),
        tech_level: OUTFIT_TECH_NEVER,
        ..neutral_ship()
    };
    assert_eq!(skiff(&skiff_with_stats()), expected);
}

/// The fixture galaxy with `alpha-prime` a shipyard and the skiff priced
/// at `price`, sold at `sold_at` (a YAML list's insides, none for no
/// field).
fn skiff_for_sale(price: &str, sold_at: Option<&str>) -> MemSource {
    let sold_at = sold_at.map_or(String::new(), |at| format!("sold_at: [{at}]\n"));
    alpha_prime_offering("shipyard").with(
        "ships/skiff.md",
        &format!(
            "---\nname: Skiff\nstyle: civilian\nrole: shuttle\nmodel: hull\n\
             size: {{length: 8, beam: 4, collision_radius: 4}}\nprice: {price}\n{sold_at}---\n"
        ),
    )
}

#[test]
fn a_hull_sold_at_a_yard_costs_its_price_at_its_set_s_tech_always_offered() {
    let game = loaded(&skiff_for_sale("40000", Some("alpha-prime")));
    assert_eq!(problems(&game), Vec::<String>::new());
    let ship = ship_of(&game, "skiff");
    assert_eq!(
        (
            ship.cost,
            ship.tech_level,
            ship.buy_random,
            ship.hire_random
        ),
        (40000, 3, 100, 0)
    );
}

#[test]
fn a_hull_sold_nowhere_is_never_offered_but_keeps_its_price_for_the_trade_in() {
    let game = loaded(&skiff_for_sale("40000", None));
    assert_eq!(problems(&game), Vec::<String>::new());
    let ship = ship_of(&game, "skiff");
    assert_eq!(
        (
            ship.cost,
            ship.tech_level,
            ship.buy_random,
            ship.hire_random
        ),
        (40000, OUTFIT_TECH_NEVER, 0, 0)
    );
}

#[test]
fn a_ship_with_no_price_costs_nothing() {
    assert_eq!(skiff(&skiff_with_stats()).cost, 0);
}

#[test]
fn a_variant_costs_its_resolved_price() {
    // The gun mount costs 100 (the format's fixture).
    let game = loaded(&skiff_for_sale("40000", None));
    assert_eq!(ship_of(&game, "armed-skiff").cost, 40100);
}

#[test]
fn a_price_beyond_i32_is_held_at_the_most() {
    let game = loaded(&skiff_for_sale("4000000000", None));
    assert_eq!(problems(&game), Vec::<String>::new());
    assert_eq!(ship_of(&game, "skiff").cost, i32::MAX);
    assert_eq!(ship_of(&game, "armed-skiff").cost, i32::MAX);
}

#[test]
fn a_stellar_s_special_tech_holds_the_hulls_sold_there() {
    let game = loaded(&skiff_for_sale("40000", Some("alpha-prime")));
    let prime = stellar(&game, game.ids.stellars[&id("alpha-prime")]);
    assert_eq!(prime.special_tech1_3, [3, -1, -1]);
    assert_eq!(prime.special_tech4_8, [-1; 5]);
    let yard = stellar(&game, game.ids.stellars[&id("alpha-yard")]);
    assert_eq!(yard.special_tech1_3, [-1; 3], "sells nothing");
}

/// The fixture galaxy with a variant of the skiff, `technical`, that sets
/// its speed and adds the gun mount, and a variant of that, `heavy`, that
/// sets its hull.
fn variants() -> MemSource {
    skiff_with_stats()
        .with(
            "ships/technical.md",
            "---\nname: Technical\nbase: skiff\nstats: {speed: 350}\nmods: [gun-mount]\n---\n",
        )
        .with(
            "ships/heavy.md",
            "---\nname: Heavy\nbase: technical\nstats: {hull: 90}\n---\n",
        )
}

#[test]
fn a_variant_has_its_base_s_stats_with_its_own_over_them() {
    let game = loaded(&variants());
    assert_eq!(problems(&game), Vec::<String>::new());
    let technical = ship_of(&game, "technical");
    assert_eq!(
        (
            technical.speed,
            technical.accel,
            technical.armor,
            technical.holds
        ),
        (350, 350, 50, 10)
    );
    let heavy = ship_of(&game, "heavy");
    assert_eq!(
        (heavy.speed, heavy.armor, heavy.shield, heavy.maneuver),
        (350, 90, 30, 60),
        "two steps down"
    );
}

#[test]
fn a_variant_s_kits_are_its_default_items_before_their_effects() {
    let game = loaded(&variants());
    let mount = Some(OutfitId(game.ids.kits[&id("gun-mount")]));
    for name in ["technical", "heavy"] {
        let ship = ship_of(&game, name);
        assert_eq!(ship.default_items1_4, [mount, None, None, None], "{name}");
        assert_eq!(ship.item_count1_4, [1, 0, 0, 0], "{name}");
        assert_eq!(
            ship.holds, 10,
            "{name}: the kit's cargo comes from the item"
        );
    }
}

#[test]
fn a_variant_draws_at_its_base_s_size_with_its_kits_mounts() {
    let game = loaded(&variants());
    let (_, anim): (_, ShipAnim) = named(&game, game.ids.ships[&id("technical")]);
    assert_eq!((anim.base_x_size, anim.base_y_size), (4, 8));
    assert_eq!(anim.turret_pos_x, [0; 4]);
    assert_eq!(
        anim.turret_pos_y, [-1; 4],
        "the gun mount's bed in every turret point"
    );
    assert_eq!(
        anim.gun_pos_y, [0; 4],
        "the skiff with stats has no nose gun"
    );
}

#[test]
fn a_hull_s_hardpoints_are_its_slots_its_weapons_and_its_firing_positions() {
    let game = loaded(&galaxy());
    let skiff = ship_of(&game, "skiff");
    assert_eq!((skiff.max_gun, skiff.max_tur), (1, 0), "its nose");
    let blaster = Some(WeaponId(game.ids.weapons[&id("blaster")]));
    assert_eq!(skiff.weap_type1_4, [blaster, None, None, None]);
    assert_eq!(skiff.weap_count1_4, [1, 0, 0, 0]);
    let (_, anim): (_, ShipAnim) = named(&game, game.ids.ships[&id("skiff")]);
    assert_eq!((anim.gun_pos_x, anim.gun_pos_y), ([0; 4], [3; 4]));
    let armed = ship_of(&game, "armed-skiff");
    assert_eq!(
        (armed.max_gun, armed.max_tur),
        (1, 0),
        "the kit's turret slot is its effect, not the chain's"
    );
    assert_eq!(armed.weap_type1_4, [blaster, None, None, None]);
}

#[test]
fn a_kit_s_hardpoint_outfit_is_a_stock_weapon_of_the_ship_it_is_fitted_to() {
    let game = loaded(&galaxy().with(
        "mods/gun-mount.md",
        "---\nname: Gun mount\nfits: [shuttle]\nfitted_at: lawless\nprice: 100\n\
         hardpoints:\n  - id: bed\n    mount: turret\n    position: [0, -1]\n    outfit: blaster\n---\n",
    ));
    let armed = ship_of(&game, "armed-skiff");
    let blaster = Some(WeaponId(game.ids.weapons[&id("blaster")]));
    assert_eq!(armed.weap_type1_4, [blaster, None, None, None]);
    assert_eq!(
        armed.weap_count1_4,
        [2, 0, 0, 0],
        "the nose's and the bed's"
    );
}

#[test]
fn each_hardpoint_fires_from_its_kind_s_points_in_whole_pixels() {
    let game = loaded(&galaxy().with(
        "ships/skiff.md",
        "---\nname: Skiff\nstyle: civilian\nrole: shuttle\nmodel: hull\n\
         size: {length: 8, beam: 4, collision_radius: 4}\n\
         hardpoints:\n  - id: a\n    mount: fixed\n    position: [2.4, 3.6]\n\
         \x20 - id: c\n    mount: launcher\n    position: [0, -3]\n\
         \x20 - id: d\n    mount: bay\n    position: [5, 5]\n---\n",
    ));
    let (_, anim): (_, ShipAnim) = named(&game, game.ids.ships[&id("skiff")]);
    assert_eq!(
        (anim.gun_pos_x, anim.gun_pos_y),
        ([2; 4], [4; 4]),
        "whole pixels"
    );
    assert_eq!((anim.guided_pos_x, anim.guided_pos_y), ([0; 4], [-3; 4]));
    assert_eq!(
        (anim.turret_pos_x, anim.turret_pos_y),
        ([0; 4], [0; 4]),
        "a bay fires from none, and a kind with none keeps the centre"
    );
    let skiff = ship_of(&game, "skiff");
    assert_eq!((skiff.max_gun, skiff.max_tur), (2, 0), "a bay is no slot");
}

#[test]
fn more_than_four_spots_of_a_kind_are_a_problem() {
    let hardpoints = (0..5)
        .map(|n| format!("  - id: t{n}\n    mount: turret\n    position: [{n}, 0]\n"))
        .collect::<Vec<_>>()
        .concat();
    let game = loaded(&galaxy().with(
        "ships/skiff.md",
        &format!(
            "---\nname: Skiff\nstyle: civilian\nrole: shuttle\n\
             size: {{length: 8, beam: 4, collision_radius: 4}}\nhardpoints:\n{hardpoints}---\n"
        ),
    ));
    let lines = problems(&game);
    assert!(
        lines.contains(
            &"ships/skiff.md: it has more than four turret hardpoints; the first four are its \
              firing positions"
                .to_owned()
        ),
        "{lines:?}"
    );
}

#[test]
fn more_than_eight_default_items_is_a_problem_and_the_rest_are_left_out() {
    let kits: Vec<String> = (0..9).map(|n| format!("k{n}")).collect();
    let source = kits.iter().fold(galaxy(), |source, kit| {
        source.with(
            &format!("mods/{kit}.md"),
            "---\nname: K\nfits: [shuttle]\nfitted_at: legal\nprice: 1\n---\n",
        )
    });
    let game = loaded(&source.with(
        "ships/armed-skiff.md",
        &format!(
            "---\nname: Armed skiff\nbase: skiff\nmods: [{}]\n---\n",
            kits.join(", ")
        ),
    ));
    assert_eq!(
        problems(&game),
        ["ships/armed-skiff.md: it has more than eight default items; 1 are left out"]
    );
    let armed = ship_of(&game, "armed-skiff");
    let kit = |n: usize| Some(OutfitId(game.ids.kits[&id(&kits[n])]));
    assert_eq!(armed.default_items5_8, [kit(4), kit(5), kit(6), kit(7)]);
    assert_eq!(armed.item_count5_8, [1; 4]);
}

#[test]
fn a_mod_kit_or_hardpoint_outfit_that_is_not_there_is_a_problem() {
    let game = loaded(
        &galaxy()
            .with(
                "ships/armed-skiff.md",
                "---\nname: Armed skiff\nbase: skiff\nmods: [gun-mount, gone]\n---\n",
            )
            .with(
                "ships/skiff.md",
                "---\nname: Skiff\nstyle: civilian\nrole: shuttle\n\
                 hardpoints:\n  - id: nose\n    mount: fixed\n    position: [0, 3]\n    outfit: lost\n---\n",
            ),
    );
    assert_eq!(
        problems(&game),
        [
            "ships/armed-skiff.md: its mod kit `gone` is not in mods/; it is left out",
            "ships/skiff.md: its hardpoint `nose`'s outfit `lost` is not in outfits/; it is left out",
        ],
        "each against the file that names it, once"
    );
}

#[test]
fn a_ship_contributes_the_bit_of_every_kit_set_holding_its_role() {
    let game = loaded(&galaxy());
    assert_eq!(
        skiff(&skiff_with_stats()).contribute.bits(),
        1,
        "the gun mount fits a shuttle"
    );
    let mount = game.ids.kits[&id("gun-mount")];
    let (_, kit): (_, crate::records::outfit::Outfit) = named(&game, mount);
    assert_eq!(kit.require.bits(), 1);
    let crown = galaxy().with(
        "ships/skiff.md",
        "---\nname: Skiff\nstyle: crown\nrole: cruiser\n---\n",
    );
    assert_eq!(skiff(&crown).contribute.bits(), 0);
}

#[test]
fn a_ship_contributes_the_mount_bit_of_each_of_its_chain_s_hardpoints() {
    use super::kits::FitBits;
    use nova_galaxy_format::fleet::Mount;

    let game = loaded(&galaxy());
    let fixed = FitBits::mount_bit(Mount::Fixed);
    assert_eq!(
        ship_of(&game, "skiff").contribute.bits(),
        1 | fixed,
        "its role's set and its nose gun"
    );
    assert_eq!(
        ship_of(&game, "armed-skiff").contribute.bits(),
        1 | fixed,
        "the gun mount's turret bed comes with the kit's own Contribute"
    );
    let all = galaxy().with(
        "ships/skiff.md",
        "---\nname: Skiff\nstyle: civilian\nrole: cruiser\nmodel: hull\n\
         size: {length: 8, beam: 4, collision_radius: 4}\n\
         hardpoints:\n  - id: a\n    mount: turret\n    position: [0, 1]\n\
         \x20 - id: b\n    mount: launcher\n    position: [0, 2]\n\
         \x20 - id: c\n    mount: bay\n    position: [0, 3]\n---\n",
    );
    assert_eq!(
        skiff(&all).contribute.bits(),
        FitBits::mount_bit(Mount::Turret)
            | FitBits::mount_bit(Mount::Launcher)
            | FitBits::mount_bit(Mount::Bay),
        "no role set, and no fixed hardpoint"
    );
}

#[test]
fn a_chain_that_loops_or_names_no_base_is_a_problem_and_left_out() {
    let game = loaded(
        &galaxy()
            .with("ships/loop-a.md", "---\nname: A\nbase: loop-b\n---\n")
            .with("ships/loop-b.md", "---\nname: B\nbase: loop-a\n---\n")
            .with("ships/lost.md", "---\nname: Lost\nbase: zeppelin\n---\n"),
    );
    assert_eq!(
        problems(&game),
        [
            "ships/loop-a.md: its `base` chain loops (loop-a -> loop-b -> loop-a); it is left out",
            "ships/loop-b.md: its `base` chain loops (loop-b -> loop-a -> loop-b); it is left out",
            "ships/lost.md: its base `zeppelin` is not in ships/; it is left out",
        ]
    );
    for ship in ["loop-a", "loop-b", "lost"] {
        assert!(!game.ids.ships.contains_key(&id(ship)), "{ship}");
    }
}

#[test]
fn a_base_that_failed_to_load_leaves_its_variant_out_unreported() {
    let game = loaded(&galaxy().with("ships/skiff.md", "---\nname: Skiff\nstyle: nowhere\n---\n"));
    assert!(!game.ids.ships.contains_key(&id("armed-skiff")));
    let lines = problems(&game);
    assert!(
        !lines
            .iter()
            .any(|line| line.starts_with("ships/armed-skiff.md")),
        "{lines:?}"
    );
}

#[test]
fn the_side_table_holds_each_class_s_chain_spots() {
    let game = loaded(&galaxy());
    let nose = vec![crate::Spot {
        kind: crate::SpotKind::Gun,
        at: (0, 3),
    }];
    let hulls = &game.data.fittings().hulls;
    assert_eq!(hulls[&ShipId(game.ids.ships[&id("skiff")])], nose);
    assert_eq!(
        hulls[&ShipId(game.ids.ships[&id("armed-skiff")])],
        nose,
        "its base's, not its kit's"
    );
}

#[test]
fn a_stat_left_out_is_zero() {
    let ship = skiff(&galaxy());
    assert_eq!(
        (ship.speed, ship.accel, ship.maneuver, ship.shield),
        (0, 0, 0, 0)
    );
    assert_eq!((ship.armor, ship.holds), (50, 10));
}

#[test]
fn a_hull_s_anim_sets_only_its_size() {
    let game = loaded(&skiff_with_stats());
    let (_, anim): (_, ShipAnim) = named(&game, game.ids.ships[&id("skiff")]);
    #[allow(clippy::cast_possible_truncation)]
    let px = |metres: f64| (metres * PX_PER_METRE).round() as i16;
    let expected = ShipAnim {
        base_x_size: px(4.0),
        base_y_size: px(8.0),
        ..neutral_ship_anim()
    };
    assert_eq!(anim, expected, "the beam across, the length along");
    assert!((PX_PER_METRE - 1.0).abs() < f64::EPSILON, "provisional");
}

#[test]
fn a_hull_past_the_last_ship_id_is_a_problem_and_left_out() {
    let source = (0..767).fold(galaxy(), |source, n| {
        source.with(
            &format!("ships/a{n:03}.md"),
            "---\nname: A\nstyle: civilian\nrole: shuttle\nmodel: hull\n\
             size: {length: 8, beam: 4, collision_radius: 4}\n---\n",
        )
    });
    let game = loaded(&source);
    let past: Vec<_> = problems(&game)
        .into_iter()
        .filter(|line| line.contains("past the last"))
        .collect();
    assert_eq!(
        past,
        ["ships/skiff.md: it is past the last shïp ID, 895; it is left out"]
    );
    assert_eq!(
        game.data.ids(Ship::TYPE).len(),
        768,
        "the armed skiff among them"
    );
    assert!(game.ids.starts.is_empty(), "the skiff's start goes with it");
    assert!(
        !problems(&game)
            .iter()
            .any(|line| line.starts_with("starts/")),
        "not reported again"
    );
}

// Traffic.

/// The fixture galaxy with a second civilian hull, `barge`, and a crown
/// hull, `cutter`.
fn three_hulls() -> MemSource {
    let hull = |name: &str, style: &str| {
        format!(
            "---\nname: {name}\nstyle: {style}\nrole: freighter\nmodel: hull\n\
             size: {{length: 20, beam: 10, collision_radius: 10}}\n---\n"
        )
    };
    galaxy()
        .with("ships/barge.md", &hull("Barge", "civilian"))
        .with("ships/cutter.md", &hull("Cutter", "crown"))
}

#[test]
fn the_civilian_traffic_dude_flies_every_civilian_hull_at_even_odds() {
    let game = loaded(&three_hulls());
    let ship = |s: &str| Some(ShipId(game.ids.ships[&id(s)]));
    let (name, dude): (_, Dude) = named(&game, CIVILIAN_DUDE);
    assert_eq!(name, "Civilian traffic");
    let mut ship_type = [None; 16];
    ship_type[0] = ship("barge");
    ship_type[1] = ship("skiff");
    let mut probability = [0; 16];
    probability[0] = 50;
    probability[1] = 50;
    let expected = Dude {
        ai_type: 1,
        ship_type,
        probability,
        ..neutral_dude()
    };
    assert_eq!(dude, expected, "a wimpy trader of no government");
}

#[test]
fn every_system_names_the_civilian_traffic() {
    let game = loaded(&galaxy());
    for nova in [128, 129] {
        let system = system(&game, nova);
        assert_eq!(system.dude_types[0], CIVILIAN_DUDE, "{nova}");
        assert_eq!(system.prob[0], 100, "{nova}");
        assert_eq!(system.dude_types[1..], [-1; 7], "{nova}");
        assert_eq!(system.avg_ships, TRAFFIC_SHIPS, "{nova}");
    }
    assert_eq!(TRAFFIC_SHIPS, 4);
}

#[test]
fn without_a_civilian_hull_there_is_no_traffic() {
    let source = galaxy().with(
        "ships/skiff.md",
        "---\nname: Skiff\nstyle: crown\nrole: shuttle\nmodel: hull\n---\n",
    );
    let game = loaded(&source);
    assert_eq!(game.data.ids(Dude::TYPE), Vec::<i16>::new());
    let alpha = system(&game, 128);
    assert_eq!((alpha.dude_types[0], alpha.avg_ships), (-1, 0));
}

// Starts.

#[test]
fn a_start_is_a_char_with_its_name_ship_system_cash_and_the_placeholder_date() {
    let game = loaded(&galaxy());
    assert_eq!(
        game.ids
            .starts
            .iter()
            .map(|(s, &n)| (s.to_string(), n))
            .collect::<Vec<_>>(),
        [("first-flight".to_owned(), 128)]
    );
    let (name, character): (_, Character) = named(&game, 128);
    assert_eq!(name, "First flight");
    let mut system = [None; 4];
    system[0] = Some(SystemId(128));
    let (day, month, year) = START_DATE;
    let expected = Character {
        cash: 1000,
        ship_type: Some(ShipId(game.ids.ships[&id("skiff")])),
        system,
        unknown_0x134: day,
        unknown_0x136: month,
        start_year: year,
        ..neutral_character()
    };
    assert_eq!(character, expected);
}

#[test]
fn a_start_in_a_ship_that_is_not_there_is_a_problem_and_left_out() {
    let source = galaxy().with(
        "starts/first-flight.md",
        "---\nname: First flight\nsystem: alpha\nship: barge\ncash: 1000\n---\n",
    );
    let game = loaded(&source);
    assert_eq!(
        problems(&game),
        ["starts/first-flight.md: its ship `barge` is not in ships/; it is left out"]
    );
    assert_eq!(game.data.ids(Character::TYPE), Vec::<i16>::new());
    assert!(game.ids.starts.is_empty());
}

#[test]
fn a_start_in_a_variant_maps() {
    let source = galaxy().with(
        "starts/first-flight.md",
        "---\nname: First flight\nsystem: alpha\nship: armed-skiff\ncash: 1000\n---\n",
    );
    let game = loaded(&source);
    assert_eq!(problems(&game), Vec::<String>::new());
    let (_, character): (_, Character) = named(&game, 128);
    assert_eq!(
        character.ship_type,
        Some(ShipId(game.ids.ships[&id("armed-skiff")]))
    );
}

#[test]
fn a_start_in_a_ship_left_out_for_its_chain_is_left_out_unreported_again() {
    let source = galaxy()
        .with("ships/lost.md", "---\nname: Lost\nbase: zeppelin\n---\n")
        .with(
            "starts/first-flight.md",
            "---\nname: First flight\nsystem: alpha\nship: lost\ncash: 1000\n---\n",
        );
    let game = loaded(&source);
    assert_eq!(
        problems(&game),
        ["ships/lost.md: its base `zeppelin` is not in ships/; it is left out"]
    );
    assert!(game.ids.starts.is_empty());
}

#[test]
fn a_start_in_a_system_that_is_not_there_is_a_problem_and_left_out() {
    let source = galaxy().with(
        "starts/first-flight.md",
        "---\nname: First flight\nsystem: gamma\nship: skiff\ncash: 1000\n---\n",
    );
    let game = loaded(&source);
    assert_eq!(
        problems(&game),
        ["starts/first-flight.md: its system `gamma` is not in systems/; it is left out"]
    );
    assert_eq!(game.data.ids(Character::TYPE), Vec::<i16>::new());
}

#[test]
fn a_start_in_a_ship_or_system_that_failed_to_load_is_left_out_unreported() {
    for (file, text) in [
        ("ships/skiff.md", "---\nname: Skiff\nstyle: nowhere\n---\n"),
        ("systems/alpha.md", "---\nname: Alpha\n---\n"),
    ] {
        let game = loaded(&galaxy().with(file, text));
        assert!(game.ids.starts.is_empty(), "{file}");
        let lines = problems(&game);
        assert!(
            !lines.iter().any(|line| line.starts_with("starts/")),
            "{file}: {lines:?}"
        );
    }
}

#[test]
fn the_storylines_map_each_faction_to_its_govt() {
    let game = open(&super::fixture::folder()).expect("the fixture lists");
    let factions: Vec<(String, GovtId)> = game
        .data
        .storylines()
        .factions
        .iter()
        .map(|(id, &govt)| (id.clone(), govt))
        .collect();
    let expected: Vec<(String, GovtId)> = game
        .ids
        .factions
        .iter()
        .map(|(id, &nova)| (id.to_string(), GovtId(nova)))
        .collect();
    assert_eq!(factions, expected);
    assert_eq!(
        factions
            .iter()
            .map(|(id, _)| id.as_str())
            .collect::<Vec<_>>(),
        ["guild", "warden"]
    );
}

#[test]
fn each_gate_keeps_its_own_point_s_stability() {
    let game = loaded(&with_a_drop());
    let kept = |p: &str| {
        let at = &game.data.storylines().points[p];
        assert_eq!(at.gate, StellarId(game.ids.gates[&point(p)]), "{p}");
        at.stability
    };
    assert_eq!(kept("gamma/drop"), crate::GateStability::Unstable);
    assert_eq!(kept("alpha/to-beta"), crate::GateStability::Stable);
}

#[test]
fn the_fixture_s_gates_keep_their_points_stabilities() {
    use crate::GateStability::{Dead, Stable, Unstable};
    let game = open(&super::fixture::folder()).expect("the fixture lists");
    let gate_of = |p: &str| StellarId(game.ids.gates[&point(p)]);
    for (gate, stability) in [
        ("drift/hearth-road", Stable),
        ("drift/verge-road", Dead),
        ("drift/back-door", Unstable),
        ("hearth/verge-road", Stable),
        ("hearth/side-road", Stable),
        ("hearth/drift-road", Stable),
        ("verge/hearth-road", Unstable),
        ("verge/side-road", Stable),
        ("verge/drift-road", Dead),
    ] {
        let at = &game.data.storylines().points[gate];
        assert_eq!(at.gate, gate_of(gate), "{gate}");
        assert_eq!(at.stability, stability, "{gate}");
    }
}

#[test]
fn the_sample_cut_maps_its_kits_weapon_and_yards_without_problems() {
    let game = open(&super::fixture::sample_folder()).expect("the sample lists");
    assert_eq!(problems(&game), Vec::<String>::new());
    let kits: Vec<_> = game.ids.kits.keys().map(ToString::to_string).collect();
    assert_eq!(kits, ["armour-plating", "gun-mount"]);
    assert_eq!(
        game.ids
            .weapons
            .keys()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        ["mass-driver"]
    );
    let yard = |id_text: &str| stellar(&game, game.ids.stellars[&id(id_text)]);
    assert_eq!(yard("luyten-exchange").tech_level, YARD_TECH_LEGAL);
    assert_eq!(yard("marrow-yard").tech_level, YARD_TECH_LAWLESS);
    assert!(yard("luyten-exchange").flags.bits() & flags::OUTFITTER != 0);
    assert!(yard("marrow-yard").flags.bits() & flags::OUTFITTER != 0);
}

/// The fixture galaxy with the skiff carrying `hardpoints` (YAML list
/// items) and no other fields but its size, its armed variant fitted with
/// no kit.
fn skiff_with_hardpoints(hardpoints: &str) -> MemSource {
    galaxy()
        .with(
            "ships/skiff.md",
            &format!(
                "---\nname: Skiff\nstyle: civilian\nrole: shuttle\n\
                 size: {{length: 8, beam: 4, collision_radius: 4}}\nhardpoints:\n{hardpoints}---\n"
            ),
        )
        .with(
            "ships/armed-skiff.md",
            "---\nname: Armed skiff\nbase: skiff\n---\n",
        )
}

#[test]
fn a_hull_s_turret_hardpoint_is_a_turret_slot_and_not_a_gun_slot() {
    let skiff = skiff(&skiff_with_hardpoints(
        "  - id: top\n    mount: turret\n    position: [0, 0]\n",
    ));
    assert_eq!((skiff.max_gun, skiff.max_tur), (0, 1));
}

#[test]
fn exactly_four_spots_of_a_kind_fill_the_points_with_no_problem() {
    let hardpoints = (0..4)
        .map(|n| format!("  - id: t{n}\n    mount: turret\n    position: [{n}, 0]\n"))
        .collect::<Vec<_>>()
        .concat();
    let game = loaded(&skiff_with_hardpoints(&hardpoints));
    assert_eq!(problems(&game), Vec::<String>::new());
    let (_, anim): (_, ShipAnim) = named(&game, game.ids.ships[&id("skiff")]);
    assert_eq!(anim.turret_pos_x, [0, 1, 2, 3]);
}

/// The fixture galaxy with `count` weapon outfits, `w0`..., each on one of
/// the skiff's hardpoints, four of each mount in turn.
fn skiff_with_weapons(count: usize) -> MemSource {
    let hardpoints = (0..count)
        .map(|n| {
            let mount = ["fixed", "turret", "launcher"][n / 4];
            format!("  - id: h{n}\n    mount: {mount}\n    position: [0, 0]\n    outfit: w{n}\n")
        })
        .collect::<Vec<_>>()
        .concat();
    (0..count).fold(skiff_with_hardpoints(&hardpoints), |source, n| {
        source.with(
            &format!("outfits/w{n}.md"),
            "---\nname: W\nmount: fixed\ndamage: 1\nrange: 10\nreload: 1\nprice: 1\n---\n",
        )
    })
}

#[test]
fn eight_stock_weapons_fill_both_weapon_arrays_with_no_problem() {
    let game = loaded(&skiff_with_weapons(8));
    assert_eq!(problems(&game), Vec::<String>::new());
    let skiff = ship_of(&game, "skiff");
    let weapon = |n: usize| Some(WeaponId(game.ids.weapons[&id(&format!("w{n}"))]));
    assert_eq!(
        skiff.weap_type1_4,
        [weapon(0), weapon(1), weapon(2), weapon(3)]
    );
    assert_eq!(
        skiff.weap_type5_8,
        [weapon(4), weapon(5), weapon(6), weapon(7)]
    );
    assert_eq!(skiff.weap_count5_8, [1; 4]);
}

#[test]
fn a_ninth_stock_weapon_is_a_problem_and_left_out() {
    let game = loaded(&skiff_with_weapons(9));
    assert_eq!(
        problems(&game),
        [
            "ships/armed-skiff.md: it has more than eight weapons; 1 are left out",
            "ships/skiff.md: it has more than eight weapons; 1 are left out",
        ],
        "the variant inherits the hardpoints"
    );
}

#[test]
fn eight_default_items_are_no_problem() {
    let kits: Vec<String> = (0..8).map(|n| format!("k{n}")).collect();
    let source = kits.iter().fold(galaxy(), |source, kit| {
        source.with(
            &format!("mods/{kit}.md"),
            "---\nname: K\nfits: [shuttle]\nfitted_at: legal\nprice: 1\n---\n",
        )
    });
    let game = loaded(&source.with(
        "ships/armed-skiff.md",
        &format!(
            "---\nname: Armed skiff\nbase: skiff\nmods: [{}]\n---\n",
            kits.join(", ")
        ),
    ));
    assert_eq!(problems(&game), Vec::<String>::new());
}

#[test]
fn a_kit_s_missing_hardpoint_outfit_is_reported_against_the_kit_alone_whatever_its_id() {
    // The kit shares its id with the ship that fits it: the ship still
    // reports only its own hardpoints' outfits.
    let game = loaded(
        &galaxy()
            .with(
                "mods/armed-skiff.md",
                "---\nname: Bed\nfits: [shuttle]\nfitted_at: legal\nprice: 1\n\
                 hardpoints:\n  - id: bed\n    mount: turret\n    position: [0, -1]\n    outfit: lost\n---\n",
            )
            .with(
                "ships/armed-skiff.md",
                "---\nname: Armed skiff\nbase: skiff\nmods: [armed-skiff]\n---\n",
            ),
    );
    assert_eq!(
        problems(&game),
        [
            "mods/armed-skiff.md: its hardpoint `bed`'s outfit `lost` is not in outfits/; it isn't given with the kit"
        ]
    );
}

#[test]
fn the_new_game_maps_the_placeholder_status_bar() {
    let (name, intf) = named::<Interface>(&loaded(&galaxy()), DEFAULT_INTERFACE);
    assert_eq!(name, "Default status bar");
    assert_eq!(intf, default_interface());
}

#[test]
fn an_empty_galaxy_maps_the_placeholder_status_bar_too() {
    let game = loaded(&MemSource::default());
    assert_eq!(game.data.ids(Interface::TYPE), [DEFAULT_INTERFACE]);
}

/// Ship `ship`'s lineage in `game`'s store.
fn line_of(game: &NewGame, ship: &str) -> Option<crate::ShipLine> {
    let nova = game.ids.ships[&id(ship)];
    game.data.ship_line(ShipId(nova)).cloned()
}

fn chain(ids: &[&str]) -> Vec<String> {
    ids.iter().map(|&id| id.to_owned()).collect()
}

#[test]
fn a_hull_s_line_is_itself_and_its_role() {
    use nova_galaxy_format::fleet::Role;
    let line = line_of(&loaded(&galaxy()), "skiff").expect("a line");
    assert_eq!(line.chain, chain(&["skiff"]));
    assert_eq!(line.role, Some(Role::Shuttle));
}

#[test]
fn a_variant_s_line_is_itself_then_its_bases_with_the_role_inherited() {
    use nova_galaxy_format::fleet::Role;
    let game = loaded(&variants());
    let heavy = line_of(&game, "heavy").expect("a line");
    assert_eq!(heavy.chain, chain(&["heavy", "technical", "skiff"]));
    assert_eq!(heavy.role, Some(Role::Shuttle), "the skiff's");
    let armed = line_of(&game, "armed-skiff").expect("a line");
    assert_eq!(armed.chain, chain(&["armed-skiff", "skiff"]));
}

#[test]
fn a_ship_that_does_not_resolve_has_no_line() {
    let source = galaxy().with("ships/lost.md", "---\nname: Lost\nbase: zeppelin\n---\n");
    let (galaxy, _) = nova_galaxy_format::load::load(&source);
    let mut ids = IdTable::default();
    ids.ships.insert(id("lost"), 140);
    ids.ships.insert(id("skiff"), 141);
    let lines = looks::ship_lines(&galaxy, &ids);
    assert_eq!(lines.keys().copied().collect::<Vec<_>>(), [ShipId(141)]);
}

#[test]
fn the_fixture_folder_s_hulls_have_their_roles() {
    use nova_galaxy_format::fleet::Role;
    let game = open(&super::fixture::folder()).expect("lists");
    let hauler = line_of(&game, "hauler").expect("a line");
    assert_eq!(
        (hauler.chain, hauler.role),
        (chain(&["hauler"]), Some(Role::Freighter))
    );
    let cutter = line_of(&game, "cutter").expect("a line");
    assert_eq!(cutter.role, Some(Role::Corvette));
}
