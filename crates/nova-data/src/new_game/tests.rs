//! The mapping, over the format's test galaxy (`fixture::galaxy`: one
//! faction, `league`; systems `alpha`, governed by it, and `beta`, linked
//! by the jump points `alpha/to-beta` at [100, 0] and `beta/to-alpha` at
//! [-100, 0], which `league` holds; stellars `alpha-prime` and
//! `alpha-yard`, a station it owns, both in `alpha`; the civilian hull
//! `skiff` and its variant `armed-skiff`; and the start `first-flight` in
//! the skiff in `alpha`) broken one file at a time.

use std::path::{Path, PathBuf};

use nova_galaxy_format::fixture;
use nova_galaxy_format::source::MemSource;

use super::fleet::{PX_PER_METRE, START_DATE, TRAFFIC_SHIPS};
use super::ids::CIVILIAN_DUDE;
use super::records::{
    neutral_character, neutral_dude, neutral_govt, neutral_ship, neutral_ship_anim, neutral_stellar,
};
use super::*;
use crate::decode::Record;
use crate::records::character::Character;
use crate::records::desc::Desc;
use crate::records::dude::Dude;
use crate::records::govt::Govt;
use crate::records::ship::Ship;
use crate::records::ship_anim::ShipAnim;
use crate::records::stellar::{Stellar, flags, flags2};
use crate::records::system::System;
use crate::wire::flags::{Flags16, Flags32};
use crate::wire::id::{GovtId, ShipId, StellarId, SystemId};
use nova_galaxy_format::id::PointRef;

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
    fixture::galaxy().with(
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
    let game = loaded(&fixture::galaxy());
    assert_eq!(problems(&game), Vec::<String>::new());
    assert_eq!(game.lfs_pointers, Vec::<String>::new());
}

#[test]
fn ids_count_up_from_128_in_id_order_per_kind_and_the_table_maps_back() {
    let game = loaded(&fixture::galaxy());
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
    let game = loaded(&fixture::galaxy());
    let alpha = game
        .data
        .get::<System>(128)
        .expect("present")
        .expect("decoded");
    assert_eq!(alpha.source.path, dir());
}

#[test]
fn a_faction_becomes_a_govt_with_its_name_and_colour() {
    let game = loaded(&fixture::galaxy());
    let league = game
        .data
        .get::<Govt>(128)
        .expect("present")
        .expect("decoded");
    assert_eq!(league.name, Some("League"));
    let expected = Govt {
        color: 0x00AA_8800,
        ship_color: 0x00AA_8800,
        ..neutral_govt()
    };
    assert_eq!(*league.record, expected);
}

#[test]
fn a_system_becomes_a_syst_with_its_name_position_government_and_stellars() {
    let game = loaded(&fixture::galaxy());
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
    let game = loaded(&fixture::galaxy());
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
    let game = loaded(&fixture::galaxy());
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
    let source = fixture::galaxy().with(
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
    let source = fixture::galaxy().with(
        "stellars/alpha-yard.md",
        "---\nname: Alpha Yard\nkind: station\nsystem: alpha\nposition: [0, 0]\n---\n \n\n",
    );
    assert_eq!(loaded(&source).data.ids(Desc::TYPE), Vec::<i16>::new());
}

#[test]
fn no_system_has_a_hyperspace_link() {
    let game = loaded(&fixture::galaxy());
    for (_, entry) in game.data.records::<System>() {
        assert_eq!(entry.expect("decoded").record.con, [None; 16]);
    }
}

// Jump points.

#[test]
fn each_jump_point_is_a_gate_numbered_after_the_last_stellar_in_system_and_point_order() {
    let game = loaded(&fixture::galaxy());
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
    let source = fixture::galaxy().with(
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
    let pointless = fixture::galaxy()
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
    assert_eq!(
        loaded(&fixture::galaxy()).ids.stellars,
        without.ids.stellars
    );
}

#[test]
fn a_gate_sits_where_its_point_does() {
    let game = loaded(&fixture::galaxy());
    let (_, to_beta): (_, Stellar) = named(&game, 130);
    assert_eq!((to_beta.x_pos, to_beta.y_pos), (100, 0));
    let (_, to_alpha): (_, Stellar) = named(&game, 131);
    assert_eq!((to_alpha.x_pos, to_alpha.y_pos), (-100, 0));
}

#[test]
fn a_gate_is_owned_by_its_point_s_owner() {
    let game = loaded(&fixture::galaxy());
    assert_eq!(stellar(&game, 130).govt, None);
    assert_eq!(stellar(&game, 131).govt, Some(GovtId(128)));
}

#[test]
fn a_gate_can_be_landed_on_and_is_a_hypergate() {
    let game = loaded(&fixture::galaxy());
    let to_beta = stellar(&game, 130);
    assert_eq!(to_beta.flags, Flags32(flags::CAN_LAND));
    assert_eq!(to_beta.flags2, Flags16(flags2::HYPERGATE));
    assert_eq!(flags2::HYPERGATE, 0x1000);
}

#[test]
fn a_gate_links_to_its_partner_s_gate_alone() {
    let game = loaded(&fixture::galaxy());
    let mut links = [None; 8];
    links[0] = Some(StellarId(131));
    assert_eq!(stellar(&game, 130).hyper_link, links);
    links[0] = Some(StellarId(130));
    assert_eq!(stellar(&game, 131).hyper_link, links);
}

#[test]
fn a_gate_is_named_for_its_partner_s_system() {
    let game = loaded(&fixture::galaxy());
    assert_eq!(named::<Stellar>(&game, 130).0, "Beta");
    assert_eq!(named::<Stellar>(&game, 131).0, "Alpha");
}

#[test]
fn a_gate_heads_ships_out_at_random_and_sets_nothing_else() {
    let game = loaded(&fixture::galaxy());
    let mut hyper_link = [None; 8];
    hyper_link[0] = Some(StellarId(131));
    let expected = Stellar {
        x_pos: 100,
        y_pos: 0,
        flags: Flags32(flags::CAN_LAND),
        flags2: Flags16(flags2::HYPERGATE),
        hyper_link,
        ..neutral_stellar()
    };
    assert_eq!(stellar(&game, 130), expected);
    assert_eq!(expected.cust_snd_id, -1, "a random exit heading");
}

/// The fixture galaxy with `gamma`, holding the one-way point `drop` to
/// `alpha/to-beta`.
fn with_a_drop() -> MemSource {
    fixture::galaxy().with(
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
        let source = fixture::galaxy().with(
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
    let source = fixture::galaxy().with("systems/alpha.md", "---\nname: Alpha\n---\n");
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
fn a_government_naming_no_faction_is_a_problem_and_no_government() {
    let source = fixture::galaxy().with(
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
    let source = fixture::galaxy().with(
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
    let source = fixture::galaxy().with("factions/league.md", "---\nname: League\n---\n");
    let game = loaded(&source);
    let lines = problems(&game);
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(lines[0].starts_with("factions/league.md: "), "{lines:?}");
    assert_eq!(system(&game, 128).govt, None);
    assert_eq!(stellar(&game, 129).govt, None);
}

#[test]
fn a_stellar_in_no_system_is_a_problem_and_left_out() {
    let source = fixture::galaxy().with(
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
    let source = fixture::galaxy().with("systems/alpha.md", "---\nname: Alpha\n---\n");
    let game = loaded(&source);
    let lines = problems(&game);
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(lines[0].starts_with("systems/alpha.md: "), "{lines:?}");
    assert_eq!(game.data.ids(Stellar::TYPE), [128], "beta's gate alone");
    assert_eq!(game.data.ids(System::TYPE), [128], "beta");
}

#[test]
fn a_map_position_outside_i16_is_a_problem_and_moved_to_the_nearest() {
    let source = fixture::galaxy().with(
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
        let source = fixture::galaxy().with(
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
    (0..count).fold(fixture::galaxy(), |source, n| {
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
    let source = (0..256).fold(fixture::galaxy(), |source, n| {
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
        fixture::galaxy().with(
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
    let source = (0..2047).fold(fixture::galaxy(), |source, n| {
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
    let source = fixture::galaxy()
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
    let source = fixture::galaxy()
        .with_bytes("media/export/pack/hull.glb", pointer)
        .with_bytes("media/export/pack/surface.png", b"\x89PNG");
    let game = loaded(&source);
    assert_eq!(game.lfs_pointers, ["media/export/pack/hull.glb"]);
    assert_eq!(game.data.ids(System::TYPE), [128, 129]);
}

// Ships.

/// The fixture galaxy with the skiff holding every stat.
fn skiff_with_stats() -> MemSource {
    fixture::galaxy().with(
        "ships/skiff.md",
        "---\nname: Skiff\nstyle: civilian\nrole: shuttle\nmodel: hull\n\
         size: {length: 8, beam: 4, collision_radius: 4}\n\
         stats: {hull: 50, shields: 30, speed: 300, handling: 60, cargo: 10}\n---\n",
    )
}

fn skiff(source: &MemSource) -> Ship {
    named::<Ship>(&loaded(source), 128).1
}

#[test]
fn each_hull_is_a_ship_numbered_from_128_with_its_name() {
    let game = loaded(&fixture::galaxy());
    assert_eq!(game.data.ids(Ship::TYPE), [128], "the skiff alone");
    assert_eq!(
        game.ids
            .ships
            .iter()
            .map(|(s, &n)| (s.to_string(), n))
            .collect::<Vec<_>>(),
        [("skiff".to_owned(), 128)]
    );
    assert_eq!(named::<Ship>(&game, 128).0, "Skiff");
}

#[test]
fn a_variant_is_not_mapped_and_is_no_problem() {
    let game = loaded(&fixture::galaxy());
    assert!(!game.ids.ships.contains_key(&id("armed-skiff")));
    assert_eq!(problems(&game), Vec::<String>::new());
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
fn a_hull_s_turn_rate_is_a_quarter_of_its_handling() {
    assert_eq!(skiff(&skiff_with_stats()).maneuver, 15);
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
        maneuver: 15,
        shield: 30,
        armor: 50,
        holds: 10,
        fuel: 300,
        crew: 1,
        ..neutral_ship()
    };
    assert_eq!(skiff(&skiff_with_stats()), expected);
}

#[test]
fn a_stat_left_out_is_zero() {
    let ship = skiff(&fixture::galaxy());
    assert_eq!(
        (ship.speed, ship.accel, ship.maneuver, ship.shield),
        (0, 0, 0, 0)
    );
    assert_eq!((ship.armor, ship.holds), (50, 10));
}

#[test]
fn a_hull_s_anim_sets_only_its_size() {
    let game = loaded(&fixture::galaxy());
    let (_, anim): (_, ShipAnim) = named(&game, 128);
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
    let source = (0..768).fold(fixture::galaxy(), |source, n| {
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
    assert_eq!(game.data.ids(Ship::TYPE).len(), 768);
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
    fixture::galaxy()
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
    let game = loaded(&fixture::galaxy());
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
    let source = fixture::galaxy().with(
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
    let game = loaded(&fixture::galaxy());
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
        ship_type: Some(ShipId(128)),
        system,
        unknown_0x134: day,
        unknown_0x136: month,
        start_year: year,
        ..neutral_character()
    };
    assert_eq!(character, expected);
}

#[test]
fn a_start_in_a_ship_that_is_no_mapped_hull_is_a_problem_and_left_out() {
    for ship in ["armed-skiff", "barge"] {
        let source = fixture::galaxy().with(
            "starts/first-flight.md",
            &format!("---\nname: First flight\nsystem: alpha\nship: {ship}\ncash: 1000\n---\n"),
        );
        let game = loaded(&source);
        assert_eq!(
            problems(&game),
            [format!(
                "starts/first-flight.md: its ship `{ship}` is no hull in ships/; it is left out"
            )]
        );
        assert_eq!(game.data.ids(Character::TYPE), Vec::<i16>::new());
        assert!(game.ids.starts.is_empty());
    }
}

#[test]
fn a_start_in_a_system_that_is_not_there_is_a_problem_and_left_out() {
    let source = fixture::galaxy().with(
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
