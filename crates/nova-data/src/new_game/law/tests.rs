//! A faction's law and the goods' contraband bits, over the format's
//! test galaxy (`super::super::tests::galaxy`), whose one faction,
//! `league`, keeps the law and names none.

use std::path::Path;

use nova_galaxy_format::source::MemSource;

use super::super::tests::galaxy;
use super::super::{NewGame, load};
use super::*;
use crate::records::govt::Govt;
use crate::records::junk::Junk;

fn loaded(source: &MemSource) -> NewGame {
    load(source, Path::new("games"))
}

fn id(text: &str) -> Id {
    Id::try_from(text.to_owned()).expect("an id")
}

fn problems(game: &NewGame) -> Vec<String> {
    game.problems.iter().map(ToString::to_string).collect()
}

/// The faction `name`'s file, with `extra` front matter.
fn faction_file(name: &str, extra: &str) -> String {
    format!("---\nname: {name}\nkind: minor\ncolor: \"#123456\"\n{extra}---\n")
}

/// A good named `name`, contraband for `factions` (the YAML list's
/// inside).
fn good_file(name: &str, factions: &str) -> String {
    format!("---\nname: {name}\nprice: 100\ncontraband: [{factions}]\n---\n")
}

fn govt(game: &NewGame, faction: &str) -> Govt {
    let nova = game.ids.factions[&id(faction)];
    game.data
        .get::<Govt>(nova)
        .expect("present")
        .expect("decodes")
        .record
        .clone()
}

/// The `gövt` fields the law sets.
fn law(game: &NewGame, faction: &str) -> GovtLaw {
    let govt = govt(game, faction);
    GovtLaw {
        smug_penalty: govt.smug_penalty,
        scan_fine: govt.scan_fine,
        scan_mask: govt.scan_mask.0,
    }
}

fn junk_mask(game: &NewGame, good: &str) -> u16 {
    let nova = game.ids.junk[&id(good)];
    let entry = game
        .data
        .get::<Junk>(nova)
        .expect("present")
        .expect("decodes");
    entry.record.scan_mask.0
}

fn rate(game: &NewGame, faction: &str) -> Option<u32> {
    let nova = GovtId(game.ids.factions[&id(faction)]);
    game.data.fittings().scan_rates.get(&nova).copied()
}

const DEFAULT_LAW: GovtLaw = GovtLaw {
    smug_penalty: LAW_SMUGGLE_PENALTY,
    scan_fine: LAW_SCAN_FINE,
    scan_mask: LAWLESS_KIT_SCAN,
};

#[test]
fn a_faction_that_names_no_law_takes_the_defaults_and_no_rate() {
    let game = loaded(&galaxy());
    assert_eq!(problems(&game), Vec::<String>::new());
    assert_eq!(law(&game, "league"), DEFAULT_LAW);
    assert_eq!(rate(&game, "league"), None);
    assert!(game.data.fittings().scan_rates.is_empty());
}

#[test]
fn a_faction_s_named_law_sets_its_penalty_fine_and_rate() {
    let source = galaxy()
        .with(
            "factions/band.md",
            &faction_file("Band", "law: {smuggling: 15, fine: 800, scan_rate: 50}\n"),
        )
        .with(
            "factions/crown.md",
            &faction_file("Crown", "law: {fine: 0, scan_rate: 250}\n"),
        );
    let game = loaded(&source);
    assert_eq!(problems(&game), Vec::<String>::new());
    assert_eq!(
        law(&game, "band"),
        GovtLaw {
            smug_penalty: 15,
            scan_fine: 800,
            scan_mask: LAWLESS_KIT_SCAN,
        }
    );
    assert_eq!(rate(&game, "band"), Some(50));
    assert_eq!(
        law(&game, "crown"),
        GovtLaw {
            scan_fine: 0,
            ..DEFAULT_LAW
        },
        "a fine of 0 is kept, the penalty left out takes the default"
    );
    assert_eq!(
        rate(&game, "crown"),
        Some(250),
        "as written: the rule caps it"
    );
    assert_eq!(law(&game, "league"), DEFAULT_LAW);
}

#[test]
fn a_penalty_or_fine_outside_i16_is_held_and_reported() {
    let source = galaxy().with(
        "factions/band.md",
        &faction_file("Band", "law: {smuggling: 40000, fine: 70000}\n"),
    );
    let game = loaded(&source);
    let band = law(&game, "band");
    assert_eq!((band.smug_penalty, band.scan_fine), (i16::MAX, i16::MAX));
    assert_eq!(
        problems(&game),
        [
            "factions/band.md: its law.smuggling 40000 is outside -32768 to 32767; it is held \
             at 32767",
            "factions/band.md: its law.fine 70000 is outside -32768 to 32767; it is held at \
             32767",
        ]
    );
}

#[test]
fn a_lawless_faction_keeps_no_law_and_one_naming_a_law_is_reported() {
    let source = galaxy()
        .with(
            "factions/band.md",
            &faction_file("Band", "lawless: true\nlaw: {fine: 5, scan_rate: 40}\n"),
        )
        .with(
            "factions/crown.md",
            &faction_file("Crown", "lawless: true\n"),
        );
    let game = loaded(&source);
    assert_eq!(
        problems(&game),
        ["factions/band.md: it is lawless, so its `law` is left out"]
    );
    assert_eq!(law(&game, "band"), GovtLaw::default());
    assert_eq!(law(&game, "crown"), GovtLaw::default());
    assert_eq!(rate(&game, "band"), None);
}

#[test]
fn each_faction_named_contraband_gets_a_bit_in_faction_id_order() {
    let source = galaxy()
        .with("factions/crown.md", &faction_file("Crown", ""))
        .with("factions/band.md", &faction_file("Band", ""))
        .with("goods/wine.md", &good_file("Wine", "crown"))
        .with("goods/ale.md", &good_file("Ale", "crown, band"));
    let game = loaded(&source);
    assert_eq!(problems(&game), Vec::<String>::new());
    let (band, crown) = (FIRST_CONTRABAND_BIT, FIRST_CONTRABAND_BIT << 1);
    assert_eq!(junk_mask(&game, "wine"), crown);
    assert_eq!(junk_mask(&game, "ale"), band | crown);
    assert_eq!(law(&game, "band").scan_mask, LAWLESS_KIT_SCAN | band);
    assert_eq!(law(&game, "crown").scan_mask, LAWLESS_KIT_SCAN | crown);
    assert_eq!(
        law(&game, "league").scan_mask,
        LAWLESS_KIT_SCAN,
        "named by none"
    );
    assert_eq!(FIRST_CONTRABAND_BIT & LAWLESS_KIT_SCAN, 0);
}

#[test]
fn an_unknown_or_lawless_faction_is_left_out_and_reported_once_each() {
    let source = galaxy()
        .with("factions/crown.md", &faction_file("Crown", ""))
        .with(
            "factions/pirates.md",
            &faction_file("Pirates", "lawless: true\n"),
        )
        .with("factions/broken.md", "---\nname: Broken\n---\n")
        .with(
            "goods/wine.md",
            &good_file(
                "Wine",
                "ghost, pirates, crown, ghost, broken, crown, pirates",
            ),
        );
    let game = loaded(&source);
    let lines: Vec<String> = problems(&game)
        .into_iter()
        .filter(|line| line.starts_with("goods/"))
        .collect();
    assert_eq!(
        lines,
        [
            "goods/wine.md: its contraband name `ghost`, no faction; it is left out",
            "goods/wine.md: its contraband name `pirates`, lawless, so it scans nobody; it is \
             left out",
        ],
        "a faction whose file failed to load isn't reported again"
    );
    assert_eq!(junk_mask(&game, "wine"), FIRST_CONTRABAND_BIT, "crown's");
    assert_eq!(law(&game, "pirates"), GovtLaw::default());
}

#[test]
fn a_faction_past_the_fifteenth_named_is_left_out_and_reported() {
    let names: Vec<String> = (1..=16).map(|n| format!("f{n:02}")).collect();
    let source = names.iter().fold(galaxy(), |source, name| {
        source.with(&format!("factions/{name}.md"), &faction_file(name, ""))
    });
    let source = source
        .with("goods/wine.md", &good_file("Wine", &names.join(", ")))
        .with("goods/ale.md", &good_file("Ale", "f16, f01"));
    let game = loaded(&source);
    assert_eq!(
        problems(&game),
        [
            "goods/ale.md: its contraband name `f16`, past the 15th faction named contraband; \
             it is left out",
            "goods/wine.md: its contraband name `f16`, past the 15th faction named contraband; \
             it is left out",
        ]
    );
    assert_eq!(CONTRABAND_FACTIONS, 15);
    assert_eq!(
        junk_mask(&game, "wine"),
        !LAWLESS_KIT_SCAN,
        "every bit but bit 0"
    );
    assert_eq!(junk_mask(&game, "ale"), FIRST_CONTRABAND_BIT, "f01's");
    assert_eq!(law(&game, "f15").scan_mask, LAWLESS_KIT_SCAN | 0x8000);
    assert_eq!(law(&game, "f16").scan_mask, LAWLESS_KIT_SCAN);
}

#[test]
fn a_faction_named_only_by_a_good_left_without_a_junk_gets_no_bit() {
    let source = galaxy()
        .with("factions/crown.md", &faction_file("Crown", ""))
        .with("goods/wine.md", &good_file("Wine", "crown"))
        .with(super::super::pins::FILE, r#"{"commodities": {"wine": 2}}"#);
    let game = loaded(&source);
    assert_eq!(law(&game, "crown").scan_mask, LAWLESS_KIT_SCAN);
}
