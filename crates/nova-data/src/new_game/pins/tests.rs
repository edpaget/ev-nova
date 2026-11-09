//! The table's reader and writer, and the numbering under its pins, over
//! the format's test galaxy (`super::super::tests::galaxy`): `league`
//! 128; `alpha` 128 and `beta` 129; the stellars `alpha-prime` 128 and
//! `alpha-yard` 129, then the gates `alpha/to-beta` 130 and
//! `beta/to-alpha` 131; the ships `armed-skiff` 128 and `skiff` 129; the
//! start `first-flight` 128; the outfit `blaster` 128, its weapon 128;
//! and the kit `gun-mount` 129, after the blaster.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use nova_galaxy_format::id::{Id, PointRef};
use nova_galaxy_format::source::MemSource;

use super::super::tests::galaxy;
use super::super::{NewGame, load};
use super::*;

fn dir() -> PathBuf {
    Path::new("games").join("sample")
}

fn loaded(source: &MemSource) -> NewGame {
    load(source, &dir())
}

fn id(text: &str) -> Id {
    Id::try_from(text.to_owned()).expect("an id")
}

fn point(text: &str) -> PointRef {
    PointRef::try_from(text.to_owned()).expect("a point")
}

fn problems(game: &NewGame) -> Vec<String> {
    game.problems.iter().map(ToString::to_string).collect()
}

/// The problems against the table, as `file: message` lines.
fn table_problems(game: &NewGame) -> Vec<String> {
    game.problems
        .iter()
        .filter(|problem| problem.to_string().starts_with(FILE))
        .map(ToString::to_string)
        .collect()
}

/// `galaxy()` with the table `json`.
fn pinned(json: &str) -> MemSource {
    galaxy().with(FILE, json)
}

/// The format galaxy's own IDs, as the load without a table gives them.
fn todays() -> IdTable {
    loaded(&galaxy()).ids
}

/// The incomplete-table problem's text for `count` and `first`.
fn incomplete(count: &str, first: &str) -> String {
    format!(
        "{FILE}: it leaves out {count}, the first `{first}`; run `nova-dump --pin-ids {}` to pin them",
        dir().display()
    )
}

/// A system file named `name`, in the rim, with no jump points.
fn system_file(name: &str) -> String {
    format!("---\nname: {name}\nregion: rim\nmap: [300, 300]\n---\n")
}

/// A stellar file in `system`.
fn stellar_file(system: &str) -> String {
    format!("---\nname: New moon\nkind: planet\nsystem: {system}\nposition: [10, 10]\n---\n")
}

// Reading the table.

#[test]
fn no_table_numbers_as_without_pins_and_reports_nothing() {
    let game = loaded(&galaxy());
    assert_eq!(problems(&game), Vec::<String>::new());
    assert_eq!(game.ids.systems[&id("alpha")], 128);
    assert_eq!(game.ids.gates[&point("beta/to-alpha")], 131);
    assert_eq!(game.ids.kits[&id("gun-mount")], 129);
}

#[test]
fn an_empty_table_numbers_as_without_pins_and_says_to_pin() {
    let game = loaded(&pinned("{}"));
    assert_eq!(game.ids, todays());
    assert_eq!(
        table_problems(&game),
        [incomplete("13 records", "factions/league")]
    );
}

#[test]
fn a_table_of_every_record_reports_nothing() {
    let full = text(&todays());
    let game = loaded(&pinned(&full));
    assert_eq!(problems(&game), Vec::<String>::new());
    assert_eq!(game.ids, todays());
}

#[test]
fn a_pinned_id_keeps_its_id_whatever_its_sort_position() {
    let game = loaded(&pinned(r#"{"systems": {"beta": 128}}"#));
    assert_eq!(game.ids.systems[&id("beta")], 128);
    assert_eq!(game.ids.systems[&id("alpha")], 129);
}

#[test]
fn unpinned_records_start_above_the_highest_pin_a_retired_one_included() {
    let game = loaded(&pinned(r#"{"systems": {"beta": 129, "gone": 140}}"#));
    assert_eq!(game.ids.systems[&id("beta")], 129);
    assert_eq!(game.ids.systems[&id("alpha")], 141);
    assert!(
        !game.ids.systems.contains_key(&id("gone")),
        "a retired entry pins no record"
    );
}

#[test]
fn an_unpinned_record_past_its_range_s_end_is_still_left_out() {
    let game = loaded(&pinned(r#"{"starts": {"first-flight": 255}}"#).with(
        "starts/aaa-flight.md",
        "---\nname: Early flight\nsystem: alpha\nship: skiff\ncash: 10\n---\n",
    ));
    assert_eq!(game.ids.starts[&id("first-flight")], 255);
    assert!(!game.ids.starts.contains_key(&id("aaa-flight")));
    assert!(
        problems(&game).contains(
            &"starts/aaa-flight.md: it is past the last chär ID, 255; it is left out".to_owned()
        ),
        "{:?}",
        problems(&game)
    );
}

#[test]
fn adding_a_system_first_leaves_the_pinned_ones_alone() {
    let game = loaded(&pinned(&text(&todays())).with("systems/aaa-system.md", &system_file("Aaa")));
    assert_eq!(game.ids.systems[&id("alpha")], 128);
    assert_eq!(game.ids.systems[&id("beta")], 129);
    assert_eq!(game.ids.systems[&id("aaa-system")], 130);
}

// Shared ranges.

#[test]
fn unpinned_gates_come_after_every_pin_and_the_unpinned_stellars() {
    let game = loaded(
        &pinned(
            r#"{"stellars": {"alpha-prime": 128, "alpha-yard": 129},
                "gates": {"alpha/to-beta": 130}}"#,
        )
        .with("stellars/aaa-moon.md", &stellar_file("alpha")),
    );
    assert_eq!(game.ids.stellars[&id("aaa-moon")], 131);
    assert_eq!(game.ids.gates[&point("alpha/to-beta")], 130);
    assert_eq!(game.ids.gates[&point("beta/to-alpha")], 132);
}

#[test]
fn adding_a_stellar_leaves_pinned_gates_alone() {
    let game =
        loaded(&pinned(&text(&todays())).with("stellars/aaa-moon.md", &stellar_file("alpha")));
    let before = todays();
    assert_eq!(game.ids.gates, before.gates);
    assert_eq!(game.ids.stellars[&id("alpha-prime")], 128);
    assert_eq!(game.ids.stellars[&id("aaa-moon")], 132);
}

#[test]
fn unpinned_kits_come_after_the_unpinned_outfits() {
    let game = loaded(&pinned(r#"{"outfits": {"blaster": 128}}"#).with(
        "outfits/aaa-cannon.md",
        "---\nname: Cannon\nmount: fixed\ndamage: 9\nrange: 300\nreload: 1\nprice: 10\n---\n",
    ));
    assert_eq!(game.ids.outfits[&id("aaa-cannon")], 129);
    assert_eq!(game.ids.kits[&id("gun-mount")], 130);
}

#[test]
fn adding_an_outfit_leaves_pinned_kits_alone() {
    let game = loaded(&pinned(&text(&todays())).with(
        "outfits/aaa-cannon.md",
        "---\nname: Cannon\nmount: fixed\ndamage: 9\nrange: 300\nreload: 1\nprice: 10\n---\n",
    ));
    assert_eq!(game.ids.kits[&id("gun-mount")], 129);
    assert_eq!(game.ids.outfits[&id("blaster")], 128);
    assert_eq!(game.ids.outfits[&id("aaa-cannon")], 130);
    assert_eq!(game.ids.weapons[&id("blaster")], 128);
    assert_eq!(game.ids.weapons[&id("aaa-cannon")], 129);
}

#[test]
fn weapons_pin_by_outfit_id_in_their_own_range() {
    let game = loaded(&pinned(r#"{"weapons": {"blaster": 200}}"#));
    assert_eq!(game.ids.weapons[&id("blaster")], 200);
    assert_eq!(
        game.ids.outfits[&id("blaster")],
        128,
        "the oütf is not pinned"
    );
}

#[test]
fn a_start_added_first_numbers_above_the_pinned_one() {
    let game = loaded(&pinned(r#"{"starts": {"first-flight": 128}}"#).with(
        "starts/aaa-flight.md",
        "---\nname: Early flight\nsystem: alpha\nship: skiff\ncash: 10\n---\n",
    ));
    assert_eq!(game.ids.starts[&id("first-flight")], 128);
    assert_eq!(
        game.ids.starts[&id("aaa-flight")],
        129,
        "so New Pilot's first chär stays the pinned start"
    );
}

#[test]
fn factions_and_ships_pin_too() {
    let game = loaded(&pinned(
        r#"{"factions": {"league": 300}, "ships": {"skiff": 128}}"#,
    ));
    assert_eq!(game.ids.factions[&id("league")], 300);
    assert_eq!(game.ids.ships[&id("skiff")], 128);
    assert_eq!(game.ids.ships[&id("armed-skiff")], 129);
}

// The table's problems.

#[test]
fn an_unparsable_table_is_ignored_with_one_problem() {
    for json in [
        "not json",
        "[1, 2]",
        r#"{"systems": {"alpha": "x"}}"#,
        r#"{"systems": 3}"#,
    ] {
        let game = loaded(&pinned(json));
        assert_eq!(game.ids, todays(), "{json}");
        let lines = table_problems(&game);
        assert_eq!(lines.len(), 1, "{json}: {lines:?}");
        assert!(
            lines[0].starts_with(&format!("{FILE}: it doesn't parse as a table of IDs (")),
            "{lines:?}"
        );
        assert!(
            lines[0].ends_with("); every record is unpinned"),
            "{lines:?}"
        );
    }
}

#[test]
fn a_table_naming_an_unknown_kind_is_ignored_with_one_problem() {
    let game = loaded(&pinned(r#"{"systems": {"beta": 128}, "planets": {}}"#));
    assert_eq!(game.ids, todays());
    assert_eq!(
        table_problems(&game),
        [format!(
            "{FILE}: it names the kind `planets`, which isn't one of {}; every record is unpinned",
            KINDS.join(", ")
        )]
    );
}

#[test]
fn an_entry_with_a_bad_key_is_ignored() {
    let game = loaded(&pinned(
        r#"{"systems": {"Bad Key": 128, "beta": 129}, "gates": {"alpha": 140}}"#,
    ));
    assert_eq!(game.ids.systems[&id("beta")], 129);
    let lines = table_problems(&game);
    assert!(
        lines.contains(&format!(
            "{FILE}: its systems entry `Bad Key` is not a kebab-case id \
             (lowercase letters, digits and single hyphens); it is ignored"
        )),
        "{lines:?}"
    );
    assert!(
        lines.contains(&format!(
            "{FILE}: its gates entry `alpha` is not a jump point written system-id/point-id; \
             it is ignored"
        )),
        "{lines:?}"
    );
    assert_eq!(
        game.ids.gates[&point("alpha/to-beta")],
        130,
        "the bad gate's 140 pins nothing"
    );
}

#[test]
fn an_entry_outside_its_range_is_ignored() {
    let game = loaded(&pinned(
        r#"{"systems": {"alpha": 127, "beta": 2176}, "starts": {"first-flight": 256}}"#,
    ));
    assert_eq!(game.ids, todays());
    let lines = table_problems(&game);
    for line in [
        "its systems entry `alpha` is 127, outside 128 to 2175; it is ignored",
        "its systems entry `beta` is 2176, outside 128 to 2175; it is ignored",
        "its starts entry `first-flight` is 256, outside 128 to 255; it is ignored",
    ] {
        assert!(
            lines.contains(&format!("{FILE}: {line}")),
            "{line}: {lines:?}"
        );
    }
    let huge = loaded(&pinned(r#"{"systems": {"alpha": 99999999999}}"#));
    assert_eq!(huge.ids, todays(), "past i16 is outside too");
}

#[test]
fn a_duplicate_id_goes_to_the_first_in_allocation_order() {
    let game = loaded(&pinned(
        r#"{"systems": {"alpha": 130, "beta": 130},
            "stellars": {"alpha-yard": 140}, "gates": {"alpha/to-beta": 140},
            "outfits": {"blaster": 150}, "kits": {"gun-mount": 150}}"#,
    ));
    assert_eq!(game.ids.systems[&id("alpha")], 130);
    assert_eq!(game.ids.systems[&id("beta")], 131, "unpinned");
    assert_eq!(
        game.ids.stellars[&id("alpha-yard")],
        140,
        "stellars come first"
    );
    assert_ne!(game.ids.gates[&point("alpha/to-beta")], 140);
    assert_eq!(game.ids.outfits[&id("blaster")], 150, "outfits come first");
    assert_eq!(game.ids.kits[&id("gun-mount")], 151);
    let lines = table_problems(&game);
    for line in [
        "its systems entry `beta` is 130, which `systems/alpha` holds; it is ignored",
        "its gates entry `alpha/to-beta` is 140, which `stellars/alpha-yard` holds; it is ignored",
        "its kits entry `gun-mount` is 150, which `outfits/blaster` holds; it is ignored",
    ] {
        assert!(
            lines.contains(&format!("{FILE}: {line}")),
            "{line}: {lines:?}"
        );
    }
}

#[test]
fn a_table_leaving_out_records_counts_them_and_names_the_first() {
    let mut partial = todays();
    partial.stellars.remove(&id("alpha-yard"));
    partial.kits.clear();
    let game = loaded(&pinned(&text(&partial)));
    assert_eq!(
        game.ids.stellars[&id("alpha-yard")],
        132,
        "above the pinned gates"
    );
    assert_eq!(game.ids.kits[&id("gun-mount")], 129, "above the blaster");
    assert_eq!(
        table_problems(&game),
        [incomplete("2 records", "stellars/alpha-yard")]
    );
    let mut one = todays();
    one.weapons.clear();
    let game = loaded(&pinned(&text(&one)));
    assert_eq!(
        table_problems(&game),
        [incomplete("1 record", "weapons/blaster")]
    );
}

#[test]
fn a_table_that_can_t_be_read_is_ignored_with_one_problem() {
    struct Unreadable(MemSource);
    impl Source for Unreadable {
        fn files(
            &self,
            dir: &str,
            ext: &str,
        ) -> std::io::Result<Vec<nova_galaxy_format::source::SourceFile>> {
            self.0.files(dir, ext)
        }
        fn bytes(&self, path: &str) -> std::io::Result<Option<Vec<u8>>> {
            if path == FILE {
                Err(std::io::Error::other("locked"))
            } else {
                self.0.bytes(path)
            }
        }
    }
    let mut lines = Vec::new();
    assert_eq!(read(&Unreadable(galaxy()), &mut lines), None);
    let lines: Vec<_> = lines.iter().map(ToString::to_string).collect();
    assert_eq!(
        lines,
        [format!(
            "{FILE}: it can't be read (locked); every record is unpinned"
        )]
    );
}

// Parsing alone.

#[test]
fn parse_reads_every_kind() {
    let table = parse(
        br#"{"factions": {"f": 128}, "systems": {"s": 129}, "stellars": {"p": 130},
             "gates": {"s/g": 131}, "ships": {"h": 132}, "starts": {"c": 133},
             "outfits": {"o": 134}, "kits": {"k": 135}, "weapons": {"o": 136}}"#,
    )
    .expect("parses");
    assert_eq!(table.problems, Vec::<String>::new());
    let one = |key: &str, nova| BTreeMap::from([(id(key), nova)]);
    assert_eq!(
        table.pins,
        IdTable {
            factions: one("f", 128),
            systems: one("s", 129),
            stellars: one("p", 130),
            gates: BTreeMap::from([(point("s/g"), 131)]),
            ships: one("h", 132),
            starts: one("c", 133),
            outfits: one("o", 134),
            kits: one("k", 135),
            weapons: one("o", 136),
        }
    );
}

#[test]
fn parse_errors_name_the_trouble() {
    assert!(matches!(parse(b"nope"), Err(TableError::Unparsable(_))));
    assert_eq!(
        parse(br#"{"moons": {}}"#),
        Err(TableError::UnknownKind("moons".to_owned()))
    );
}

// Writing.

#[test]
fn the_text_round_trips_through_the_reader() {
    let table = todays();
    let parsed = parse(text(&table).as_bytes()).expect("parses");
    assert_eq!(parsed.problems, Vec::<String>::new());
    assert_eq!(parsed.pins, table);
}

#[test]
fn the_text_is_pretty_sorted_and_ends_in_one_newline() {
    let table = todays();
    let written = text(&table);
    assert!(written.ends_with("}\n"), "{written}");
    let value: serde_json::Value = serde_json::from_str(&written).expect("json");
    assert_eq!(
        format!(
            "{}\n",
            serde_json::to_string_pretty(&value).expect("prints")
        ),
        written,
        "serde's pretty form, keys sorted (serde_json's map is sorted)"
    );
    let at = |kind: &str| written.find(&format!("\"{kind}\"")).expect(kind);
    let mut kinds = KINDS.to_vec();
    kinds.sort_unstable();
    assert!(kinds.windows(2).all(|pair| at(pair[0]) < at(pair[1])));
    assert_eq!(
        text(&parse(written.as_bytes()).expect("parses").pins),
        written
    );
}

#[test]
fn every_kind_is_written_even_an_empty_one() {
    let parsed: BTreeMap<String, BTreeMap<String, i64>> =
        serde_json::from_str(&text(&IdTable::default())).expect("json");
    let mut kinds = KINDS.to_vec();
    kinds.sort_unstable();
    assert_eq!(parsed.keys().collect::<Vec<_>>(), kinds);
}

#[test]
fn pinning_with_no_table_writes_the_load_s_ids() {
    let loaded = todays();
    let pinned = pin(None, &loaded).expect("pins");
    assert_eq!(pinned.table, loaded);
    assert_eq!(pinned.text, text(&loaded));
    assert_eq!(pinned.added, 13);
}

#[test]
fn pinning_keeps_retired_entries_and_adds_new_records() {
    let mut previous = todays();
    previous.systems.insert(id("gone"), 140);
    let mut loaded = todays();
    loaded.systems.insert(id("aaa-new"), 141);
    let pinned = pin(Some(text(&previous).as_bytes()), &loaded).expect("pins");
    assert_eq!(pinned.table.systems[&id("gone")], 140);
    assert_eq!(pinned.table.systems[&id("aaa-new")], 141);
    assert_eq!(pinned.table.systems[&id("alpha")], 128);
    assert_eq!(pinned.added, 1);
    let again = pin(Some(pinned.text.as_bytes()), &loaded).expect("pins");
    assert_eq!(again.text, pinned.text, "a second run changes nothing");
    assert_eq!(again.added, 0);
}

#[test]
fn an_entry_the_load_rejected_takes_the_load_s_id() {
    let previous = br#"{"systems": {"alpha": 5000, "beta": 129, "Bad Key": 150}}"#;
    let mut loaded = todays();
    loaded.systems.insert(id("alpha"), 130);
    let pinned = pin(Some(previous), &loaded).expect("pins");
    assert_eq!(
        pinned
            .table
            .systems
            .iter()
            .map(|(k, &v)| (k.to_string(), v))
            .collect::<Vec<_>>(),
        [("alpha".to_owned(), 130), ("beta".to_owned(), 129)]
    );
}

#[test]
fn pinning_refuses_a_table_the_loader_ignores() {
    let loaded = todays();
    assert!(matches!(
        pin(Some(b"{oops"), &loaded),
        Err(TableError::Unparsable(_))
    ));
    assert_eq!(
        pin(Some(br#"{"moons": {}}"#), &loaded),
        Err(TableError::UnknownKind("moons".to_owned()))
    );
}

#[test]
fn a_system_s_nav_def_stays_in_id_order_under_pins() {
    let game = loaded(&pinned(
        r#"{"stellars": {"alpha-yard": 128, "alpha-prime": 129},
            "gates": {"alpha/to-beta": 130}}"#,
    ));
    let alpha = game
        .data
        .get::<crate::records::system::System>(game.ids.systems[&id("alpha")])
        .expect("present")
        .expect("decoded")
        .record
        .nav_def;
    let held: Vec<_> = alpha.iter().flatten().map(|stellar| stellar.0).collect();
    assert_eq!(held, [128, 129, 130], "the yard, the prime, then the gate");
}

#[test]
fn the_civilian_traffic_lists_its_hulls_in_id_order_under_pins() {
    let game = loaded(&pinned(r#"{"ships": {"skiff": 128}}"#).with(
        "ships/aaa-hauler.md",
        "---\nname: Hauler\nstyle: civilian\nrole: shuttle\n\
         size: {length: 8, beam: 4, collision_radius: 4}\nstats: {hull: 50, cargo: 10}\n---\n",
    ));
    assert_eq!(game.ids.ships[&id("aaa-hauler")], 129);
    let dude = game
        .data
        .get::<crate::records::dude::Dude>(super::super::ids::CIVILIAN_DUDE)
        .expect("present")
        .expect("decoded")
        .record
        .clone();
    let held: Vec<_> = dude.ship_type.iter().flatten().map(|ship| ship.0).collect();
    assert_eq!(held, [128, 129], "the skiff, then the hauler");
}
