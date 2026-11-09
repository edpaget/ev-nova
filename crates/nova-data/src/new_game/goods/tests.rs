//! Goods and prices over the format's test galaxy
//! (`super::super::tests::galaxy`), whose stellars `alpha-prime` (128)
//! and `alpha-yard` (129) are given the `trade` service and prices here,
//! with more trade stellars in `alpha` where a test needs them.

use std::path::Path;

use nova_galaxy_format::source::MemSource;

use super::super::tests::galaxy;
use super::super::{NewGame, load};
use super::*;
use crate::records::stellar::Stellar;

fn loaded(source: &MemSource) -> NewGame {
    load(source, Path::new("games"))
}

fn id(text: &str) -> Id {
    Id::try_from(text.to_owned()).expect("an id")
}

/// The problems, but the ID table's: a test that pins some IDs leaves
/// the rest out of its table.
fn problems(game: &NewGame) -> Vec<String> {
    let table = super::super::pins::FILE;
    (game.problems.iter())
        .filter(|problem| problem.file != table)
        .map(ToString::to_string)
        .collect()
}

/// A good file: `name`, at `price`.
fn good(name: &str, price: u32) -> String {
    format!("---\nname: {name}\nprice: {price}\n---\n")
}

/// A trade stellar in `alpha` named `name`, at `x`, with `prices` (the
/// YAML map's inside).
fn trade_stellar(name: &str, x: i32, prices: &str) -> String {
    format!(
        "---\nname: {name}\nkind: planet\nsystem: alpha\nposition: [{x}, 40]\n\
         services: [trade, refuel]\nprices: {{{prices}}}\n---\n"
    )
}

/// `source` with each of `goods`, `(id, name, price)`, in `goods/`.
fn with_goods(source: MemSource, goods: &[(&str, &str, u32)]) -> MemSource {
    goods.iter().fold(source, |source, &(id, name, price)| {
        source.with(&format!("goods/{id}.md"), &good(name, price))
    })
}

/// The test galaxy with the ID table `json`.
fn pinned(source: MemSource, json: &str) -> MemSource {
    source.with(super::super::pins::FILE, json)
}

fn table(map: &BTreeMap<Id, i16>) -> Vec<(String, i16)> {
    map.iter()
        .map(|(id, &nova)| (id.to_string(), nova))
        .collect()
}

fn strings(game: &NewGame, list: i16) -> Option<Vec<String>> {
    let entry = game.data.get::<StrList>(list)?.expect("decoded");
    Some(
        entry
            .record
            .strings
            .iter()
            .map(ToString::to_string)
            .collect(),
    )
}

fn spob_flags(game: &NewGame, stellar: &str) -> u32 {
    let nova = game.ids.stellars[&id(stellar)];
    let entry = game
        .data
        .get::<Stellar>(nova)
        .expect("present")
        .expect("decoded");
    entry.record.flags.bits()
}

fn junk_of(game: &NewGame, good: &str) -> (String, Junk) {
    let nova = game.ids.junk[&id(good)];
    let entry = game
        .data
        .get::<Junk>(nova)
        .expect("present")
        .expect("decoded");
    (entry.name.expect("named").to_owned(), entry.record.clone())
}

const SEVEN: [(&str, &str, u32); 7] = [
    ("alloys", "Alloys", 400),
    ("fuel-cells", "Fuel cells", 150),
    ("grain", "Grain", 100),
    ("machinery", "Machinery", 600),
    ("medicine", "Medicine", 800),
    ("textiles", "Textiles", 250),
    ("vintage-wine", "Vintage wine", 1200),
];

// Numbering.

#[test]
fn seven_goods_fill_the_six_slots_in_id_order_then_junk_128() {
    let game = loaded(&with_goods(galaxy(), &SEVEN));
    assert_eq!(problems(&game), Vec::<String>::new());
    assert_eq!(
        table(&game.ids.commodities),
        [
            ("alloys".to_owned(), 0),
            ("fuel-cells".to_owned(), 1),
            ("grain".to_owned(), 2),
            ("machinery".to_owned(), 3),
            ("medicine".to_owned(), 4),
            ("textiles".to_owned(), 5)
        ]
    );
    assert_eq!(table(&game.ids.junk), [("vintage-wine".to_owned(), 128)]);
}

#[test]
fn a_good_pinned_as_junk_takes_no_free_slot() {
    let source = with_goods(galaxy(), &[("ale", "Ale", 10), ("bread", "Bread", 5)]);
    let game = loaded(&pinned(source, r#"{"junk": {"ale": 200}}"#));
    assert_eq!(table(&game.ids.commodities), [("bread".to_owned(), 0)]);
    assert_eq!(table(&game.ids.junk), [("ale".to_owned(), 200)]);
}

#[test]
fn a_pinned_slot_keeps_its_good_when_an_earlier_good_is_added() {
    let source = with_goods(galaxy(), &[("aaa-ale", "Ale", 10), ("grain", "Grain", 5)]);
    let game = loaded(&pinned(source, r#"{"commodities": {"grain": 0}}"#));
    assert_eq!(
        table(&game.ids.commodities),
        [("aaa-ale".to_owned(), 1), ("grain".to_owned(), 0)]
    );
}

#[test]
fn goods_past_the_slots_and_the_last_junk_id_are_left_out() {
    let source = with_goods(
        galaxy(),
        &[
            ("a", "A", 1),
            ("b", "B", 1),
            ("c", "C", 1),
            ("d", "D", 1),
            ("e", "E", 1),
            ("f", "F", 1),
            ("g", "G", 1),
            ("h", "H", 1),
        ],
    );
    let game = loaded(&pinned(source, r#"{"junk": {"h": 255}}"#));
    assert_eq!(game.ids.commodities.len(), 6);
    assert_eq!(table(&game.ids.junk), [("h".to_owned(), 255)]);
    let lines = problems(&game);
    assert!(
        lines.contains(&"goods/g.md: it is past the last jünk ID, 255; it is left out".to_owned()),
        "{lines:?}"
    );
}

// The standard goods' lists.

#[test]
fn the_lists_hold_each_slot_s_good_in_slot_order_and_an_unused_slot_empty() {
    let source = with_goods(
        galaxy(),
        &[("grain", "Grain", 100), ("salt", "Rock Salt", 32)],
    );
    let game = loaded(&pinned(
        source,
        r#"{"commodities": {"grain": 0, "salt": 3}}"#,
    ));
    let list = |items: [&str; 6]| Some(items.map(str::to_owned).to_vec());
    assert_eq!(
        strings(&game, NAMES),
        list(["Grain", "", "", "Rock Salt", "", ""])
    );
    assert_eq!(
        strings(&game, LC_NAMES),
        list(["grain", "", "", "rock salt", "", ""])
    );
    assert_eq!(
        strings(&game, LABELS),
        list(["Grain:", "", "", "Rock Salt:", "", ""])
    );
    assert_eq!(strings(&game, PRICES), list(["100", "", "", "32", "", ""]));
    assert_eq!(strings(&game, 4002), None, "mission cargo isn't written");
}

#[test]
fn with_no_standard_good_no_list_is_written() {
    let game = loaded(&galaxy());
    let source = pinned(
        with_goods(galaxy(), &[("ale", "Ale", 10)]),
        r#"{"junk": {"ale": 128}}"#,
    );
    let all_junk = loaded(&source);
    for list in [NAMES, LC_NAMES, LABELS, PRICES] {
        assert_eq!(strings(&game, list), None, "{list}");
        assert_eq!(strings(&all_junk, list), None, "{list}");
    }
}

// A stellar's price levels.

/// Six goods in the six slots, and three trade stellars pricing them at
/// each level in turn.
fn six_priced() -> MemSource {
    let goods = [
        ("g0", "G0", 10),
        ("g1", "G1", 10),
        ("g2", "G2", 10),
        ("g3", "G3", 10),
        ("g4", "G4", 10),
        ("g5", "G5", 10),
    ];
    let pattern = |levels: [&str; 3]| {
        (0..6)
            .map(|n| format!("g{n}: {}", levels[n % 3]))
            .collect::<Vec<_>>()
            .join(", ")
    };
    with_goods(galaxy(), &goods)
        .with(
            "stellars/one.md",
            &trade_stellar("One", 10, &pattern(["low", "medium", "high"])),
        )
        .with(
            "stellars/two.md",
            &trade_stellar("Two", 20, &pattern(["high", "low", "medium"])),
        )
        .with(
            "stellars/three.md",
            &trade_stellar("Three", 30, &pattern(["medium", "high", "low"])),
        )
}

#[test]
fn each_level_sets_its_bit_in_each_slot_s_nibble() {
    let game = loaded(&six_priced());
    assert_eq!(problems(&game), Vec::<String>::new());
    let services = flags::CAN_LAND | flags::TRADE_CENTER;
    assert_eq!(spob_flags(&game, "one"), 0x1241_2400 | services);
    assert_eq!(spob_flags(&game, "two"), 0x4124_1200 | services);
    assert_eq!(spob_flags(&game, "three"), 0x2412_4100 | services);
}

#[test]
fn a_stellar_with_no_prices_keeps_its_service_bits_alone() {
    let game = loaded(&six_priced());
    assert_eq!(
        spob_flags(&game, "alpha-yard"),
        flags::STATION | flags::UNINHABITED
    );
}

// Special goods.

#[test]
fn a_special_good_is_sold_at_its_low_stellars_and_bought_at_its_high_ones_in_id_order() {
    let source = pinned(
        with_goods(galaxy(), &[("wine", "Vintage Wine", 1200)]),
        r#"{"junk": {"wine": 140}}"#,
    )
    .with(
        "stellars/aaa-cellar.md",
        &trade_stellar("Cellar", 10, "wine: low"),
    )
    .with(
        "stellars/beta-vat.md",
        &trade_stellar("Vat", 20, "wine: low"),
    )
    .with(
        "stellars/bistro.md",
        &trade_stellar("Bistro", 30, "wine: high"),
    );
    let game = loaded(&source);
    assert_eq!(problems(&game), Vec::<String>::new());
    let nova = |stellar: &str| Some(StellarId(game.ids.stellars[&id(stellar)]));
    let (name, wine) = junk_of(&game, "wine");
    assert_eq!(name, "Vintage Wine");
    let mut sold_at = [None; 8];
    sold_at[0] = nova("aaa-cellar");
    sold_at[1] = nova("beta-vat");
    let mut bought_at = [None; 8];
    bought_at[0] = nova("bistro");
    assert_eq!(
        wine,
        Junk {
            sold_at,
            bought_at,
            base_price: 1200,
            flags: Flags16(0),
            scan_mask: Flags16(0),
            lc_name: "vintage wine".into(),
            abbrev: "Vintage Wine".into(),
            buy_on: MacString::default(),
            sell_on: MacString::default(),
        }
    );
    assert_eq!(
        spob_flags(&game, "aaa-cellar"),
        flags::CAN_LAND | flags::TRADE_CENTER,
        "a special good sets no nibble"
    );
}

#[test]
fn a_special_good_s_sides_follow_the_spob_ids_not_the_file_names() {
    let source = pinned(
        with_goods(galaxy(), &[("wine", "Wine", 50)]),
        r#"{"junk": {"wine": 128}, "stellars": {"zzz-late": 200, "aaa-early": 300}}"#,
    )
    .with(
        "stellars/aaa-early.md",
        &trade_stellar("Early", 10, "wine: low"),
    )
    .with(
        "stellars/zzz-late.md",
        &trade_stellar("Late", 20, "wine: low"),
    );
    let game = loaded(&source);
    let (_, wine) = junk_of(&game, "wine");
    assert_eq!(
        wine.sold_at[..2],
        [Some(StellarId(200)), Some(StellarId(300))]
    );
}

#[test]
fn a_ninth_stellar_at_one_level_gets_no_price_and_is_reported() {
    let mut source = pinned(
        with_goods(galaxy(), &[("wine", "Wine", 50)]),
        r#"{"junk": {"wine": 128}}"#,
    );
    for n in 0..10 {
        source = source.with(
            &format!("stellars/s{n}.md"),
            &trade_stellar(&format!("S{n}"), n * 10, "wine: high"),
        );
    }
    let game = loaded(&source);
    let (_, wine) = junk_of(&game, "wine");
    let first: Vec<_> = (0..8)
        .map(|n| Some(StellarId(game.ids.stellars[&id(&format!("s{n}"))])))
        .collect();
    assert_eq!(wine.bought_at.to_vec(), first);
    assert_eq!(wine.sold_at, [None; 8]);
    let lines = problems(&game);
    for n in [8, 9] {
        assert!(
            lines.contains(&format!(
                "stellars/s{n}.md: the good `wine` is priced high at more than 8 stellars; \
                 it has no price here"
            )),
            "{lines:?}"
        );
    }
    assert_eq!(lines.len(), 2, "{lines:?}");
}

// Problems.

#[test]
fn a_price_for_a_good_that_is_not_there_is_reported_and_the_rest_maps() {
    let source = with_goods(galaxy(), &[("grain", "Grain", 100)]).with(
        "stellars/market.md",
        &trade_stellar("Market", 10, "grain: high, rice: low"),
    );
    let game = loaded(&source);
    assert_eq!(
        problems(&game),
        ["stellars/market.md: the good `rice` is not in goods/; its price is left out"]
    );
    assert_eq!(
        spob_flags(&game, "market"),
        flags::price_bits(0, flags::HIGH_PRICE) | flags::CAN_LAND | flags::TRADE_CENTER
    );
    assert_eq!(strings(&game, NAMES).expect("written")[0], "Grain");
}

#[test]
fn a_price_for_a_good_whose_file_failed_to_load_is_not_reported_again() {
    let source = galaxy()
        .with("goods/rice.md", "---\nname: Rice\n---\n")
        .with(
            "stellars/market.md",
            &trade_stellar("Market", 10, "rice: low"),
        );
    let game = loaded(&source);
    let lines = problems(&game);
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(lines[0].starts_with("goods/rice.md: "), "{lines:?}");
}

#[test]
fn a_special_good_priced_medium_is_reported_and_its_other_prices_map() {
    let source = pinned(
        with_goods(galaxy(), &[("wine", "Wine", 50)]),
        r#"{"junk": {"wine": 128}}"#,
    )
    .with("stellars/inn.md", &trade_stellar("Inn", 10, "wine: medium"))
    .with("stellars/pub.md", &trade_stellar("Pub", 20, "wine: low"));
    let game = loaded(&source);
    assert_eq!(
        problems(&game),
        [
            "stellars/inn.md: the good `wine` takes no standard slot, so it is priced only low \
             or high; its medium price is left out"
        ]
    );
    let (_, wine) = junk_of(&game, "wine");
    let at_pub = Some(StellarId(game.ids.stellars[&id("pub")]));
    assert_eq!((wine.sold_at[0], wine.sold_at[1]), (at_pub, None));
    assert_eq!(wine.bought_at, [None; 8]);
}

#[test]
fn a_price_above_the_most_a_record_holds_is_lowered_and_reported() {
    let source = pinned(
        with_goods(
            galaxy(),
            &[("gold", "Gold", 40_000), ("gems", "Gems", 32_768)],
        ),
        r#"{"junk": {"gems": 128}}"#,
    );
    let game = loaded(&source);
    assert_eq!(
        problems(&game),
        [
            "goods/gold.md: its price 40000 is above 32767, the most a good's record holds; \
             it is priced at 32767",
            "goods/gems.md: its price 32768 is above 32767, the most a good's record holds; \
             it is priced at 32767",
        ]
    );
    assert_eq!(strings(&game, PRICES).expect("written")[0], "32767");
    assert_eq!(junk_of(&game, "gems").1.base_price, 32767);
    let at_most = with_goods(galaxy(), &[("gold", "Gold", 32_767)]);
    assert_eq!(problems(&loaded(&at_most)), Vec::<String>::new());
}

#[test]
fn a_stellar_that_prices_goods_without_trade_trades_nothing_and_is_reported() {
    let source = pinned(
        with_goods(galaxy(), &[("grain", "Grain", 100), ("wine", "Wine", 50)]),
        r#"{"junk": {"wine": 128}}"#,
    )
    .with(
        "stellars/shed.md",
        "---\nname: Shed\nkind: planet\nsystem: alpha\nposition: [10, 40]\n\
         services: [bar]\nprices: {grain: low, wine: medium, rice: high}\n---\n",
    );
    let game = loaded(&source);
    assert_eq!(
        problems(&game),
        [
            "stellars/shed.md: it prices goods but has no `trade` service; nothing is traded here",
            "stellars/shed.md: the good `rice` is not in goods/; its price is left out",
        ]
    );
    assert_eq!(
        spob_flags(&game, "shed"),
        flags::CAN_LAND | flags::BAR | flags::UNINHABITED
    );
    let shed = game.data.get::<Stellar>(game.ids.stellars[&id("shed")]);
    assert!(shed.is_some());
    let source = pinned(
        with_goods(galaxy(), &[("wine", "Wine", 50)]),
        r#"{"junk": {"wine": 128}}"#,
    )
    .with(
        "stellars/shed.md",
        "---\nname: Shed\nkind: planet\nsystem: alpha\nposition: [10, 40]\n\
         services: [bar]\nprices: {wine: low}\n---\n",
    );
    let (_, wine) = junk_of(&loaded(&source), "wine");
    assert_eq!(
        wine.sold_at, [None; 8],
        "not listed where nothing is traded"
    );
}
