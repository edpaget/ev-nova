//! The mapping, over the format's test galaxy (`fixture::galaxy`: one
//! faction, `league`; systems `alpha`, governed by it, and `beta`; and
//! stellars `alpha-prime` and `alpha-yard`, a station it owns, both in
//! `alpha`) broken one file at a time.

use std::path::{Path, PathBuf};

use nova_galaxy_format::fixture;
use nova_galaxy_format::source::MemSource;

use super::records::{flags, neutral_govt};
use super::*;
use crate::decode::Record;
use crate::records::desc::Desc;
use crate::records::govt::Govt;
use crate::records::stellar::Stellar;
use crate::records::system::System;
use crate::wire::id::{GovtId, StellarId};

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
    assert_eq!(game.data.ids(Stellar::TYPE), [128, 129]);
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
    assert_eq!(alpha.record.nav_def, nav_def);
    assert_eq!(alpha.record.con, [None; 16], "no hyperspace links");

    let beta = system(&game, 129);
    assert_eq!((beta.x_pos, beta.y_pos), (200, 50));
    assert_eq!(beta.govt, None, "ungoverned");
    assert_eq!(beta.nav_def, [None; 16], "no stellars");
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
fn jump_points_make_no_spob_and_no_link() {
    let game = loaded(&fixture::galaxy());
    assert_eq!(
        game.data.ids(Stellar::TYPE).len(),
        2,
        "the two stellars only"
    );
    for (_, entry) in game.data.records::<Stellar>() {
        assert_eq!(entry.expect("decoded").record.hyper_link, [None; 8]);
    }
    for (_, entry) in game.data.records::<System>() {
        assert_eq!(entry.expect("decoded").record.con, [None; 16]);
    }
}

#[test]
fn a_government_naming_no_faction_is_a_problem_and_no_government() {
    let source = fixture::galaxy().with(
        "systems/beta.md",
        "---\nname: Beta\nregion: rim\ngovernment: nowhere\nmap: [200, 50]\n---\n",
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
    assert_eq!(game.data.ids(Stellar::TYPE), [128], "alpha-prime alone");
    assert!(!game.ids.stellars.contains_key(&id("alpha-yard")));
    let mut nav_def = [None; 16];
    nav_def[0] = Some(StellarId(128));
    assert_eq!(system(&game, 128).nav_def, nav_def);
}

#[test]
fn a_stellar_in_a_system_that_failed_to_load_is_left_out_unreported() {
    let source = fixture::galaxy().with("systems/alpha.md", "---\nname: Alpha\n---\n");
    let game = loaded(&source);
    let lines = problems(&game);
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(lines[0].starts_with("systems/alpha.md: "), "{lines:?}");
    assert_eq!(game.data.ids(Stellar::TYPE), Vec::<i16>::new());
    assert_eq!(game.data.ids(System::TYPE), [128], "beta");
}

#[test]
fn a_map_position_outside_i16_is_a_problem_and_moved_to_the_nearest() {
    let source = fixture::galaxy().with(
        "systems/beta.md",
        "---\nname: Beta\nregion: rim\nmap: [40000, -40000]\n---\n",
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
fn sixteen_stellars_fill_a_system() {
    let game = loaded(&beta_holding(16));
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
            "systems/beta.md: it has 18 stellars, more than a system's 16; `b16`, `b17` have no \
             place in it"
        ]
    );
    let beta = system(&game, 129);
    let first: Vec<_> = beta.nav_def.iter().map(|s| s.map(|s| s.0)).collect();
    let b = |n: &str| game.ids.stellars[&id(n)];
    assert_eq!(first[0], Some(b("b00")));
    assert_eq!(first[15], Some(b("b15")));
    assert_eq!(
        game.data.ids(Stellar::TYPE).len(),
        20,
        "each still has a record"
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
    let source = (0..2047).fold(fixture::galaxy(), |source, n| {
        source.with(
            &format!("systems/a{n:04}.md"),
            "---\nname: S\nregion: rim\nmap: [0, 0]\n---\n",
        )
    });
    let game = loaded(&source);
    assert_eq!(
        problems(&game),
        ["systems/beta.md: it is past the last sÿst ID, 2175; it is left out"]
    );
    assert_eq!(game.data.ids(System::TYPE).len(), 2048);
    assert_eq!(game.ids.systems[&id("alpha")], 2175);
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
        ["stellars/alpha-yard.md: it is past the last spöb ID, 2175; it is left out"]
    );
    assert_eq!(game.data.ids(Stellar::TYPE).len(), 2048);
}

#[test]
fn load_problems_come_through_unchanged_before_the_mapping_s() {
    let source = fixture::galaxy()
        .with("systems/Gamma.md", "---\nname: G\n---\n")
        .with(
            "systems/beta.md",
            "---\nname: Beta\nregion: rim\ngovernment: nowhere\nmap: [200, 50]\n---\n",
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
