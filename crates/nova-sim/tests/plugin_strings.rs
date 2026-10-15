//! Wiring: a plug-in's `'STR '` 9000-9005 name patches and 9300-9305
//! base-price patches reach the commodities through the `GameData`
//! adapter, its `'STR '` 1000 and up the message buoys, and its `'STR '`
//! 5000 and up the persons' hail quotes.

use std::io;
use std::path::Path;

use nova_data::records::string::StrResource;
use nova_data::records::string_list::StrList;
use nova_data::store::fs::{DirLister, EntryKind, Listing};
use nova_data::{GameData, Record};
use nova_rsrc::fixture::ForkBuilder;
use nova_rsrc::{Fork, ForkReader, ResType};
use nova_sim::buoy::{buoy_text, patch_id};
use nova_sim::catalog::StringPatch;
use nova_sim::market::{Commodity, commodities};
use nova_sim::person::{HAIL_QUOTES, hail_quote_patch_id, hail_quote_text};
use nova_sim::{CommCatalog, PilotCatalog};

type Resources<'a> = &'a [(ResType, i16, Vec<u8>)];

/// A data file, `/data/Nova Data`, and, when given, a plug-in,
/// `/plugins/Patch.npif`, each holding a fork.
struct Files {
    data: Vec<u8>,
    plugin: Vec<u8>,
}

impl DirLister for Files {
    fn list(&self, dir: &Path) -> io::Result<Vec<Listing>> {
        let name = if dir == Path::new("/data") {
            "Nova Data"
        } else {
            "Patch.npif"
        };
        Ok(vec![Listing {
            name: name.into(),
            kind: EntryKind::File,
        }])
    }
}

impl ForkReader for Files {
    fn read_fork(&self, path: &Path, fork: Fork) -> io::Result<Option<Vec<u8>>> {
        let bytes = if path.starts_with("/data") {
            &self.data
        } else {
            &self.plugin
        };
        Ok((fork == Fork::Data).then(|| bytes.clone()))
    }
}

fn fork(resources: Resources<'_>) -> Vec<u8> {
    resources
        .iter()
        .fold(ForkBuilder::new(), |fork, (ty, id, data)| {
            fork.resource(*ty, *id, None, data)
        })
        .build()
        .bytes
}

/// A `STR#` of `strings`.
fn str_list(strings: &[&str]) -> Vec<u8> {
    let mut bytes = u16::try_from(strings.len())
        .expect("few")
        .to_be_bytes()
        .to_vec();
    for string in strings {
        bytes.push(u8::try_from(string.len()).expect("short"));
        bytes.extend(string.as_bytes());
    }
    bytes
}

/// A `'STR '` of `text`.
fn str_resource(text: &str) -> Vec<u8> {
    str_bytes(text.as_bytes())
}

/// A `'STR '` of these raw (Mac Roman) bytes.
fn str_bytes(text: &[u8]) -> Vec<u8> {
    let mut bytes = vec![u8::try_from(text.len()).expect("short")];
    bytes.extend(text);
    bytes
}

/// Stock `STR#` 4000's commodity names and `STR#` 4004's prices.
fn stock() -> Vec<(ResType, i16, Vec<u8>)> {
    vec![
        (
            StrList::TYPE,
            4000,
            str_list(&[
                "Food",
                "Industrial",
                "Medical Supplies",
                "Luxury Goods",
                "Metal",
                "Equipment",
            ]),
        ),
        (
            StrList::TYPE,
            4004,
            str_list(&["75", "350", "750", "900", "200", "550"]),
        ),
    ]
}

/// Each commodity, from stock data and, when given, a plug-in holding
/// `plugin`.
fn goods(plugin: Option<Resources<'_>>) -> Vec<Commodity> {
    let files = Files {
        data: fork(&stock()),
        plugin: fork(plugin.unwrap_or_default()),
    };
    let plugins = plugin.map(|_| Path::new("/plugins"));
    let data = GameData::load(&files, &files, Path::new("/data"), plugins).expect("opens");
    commodities(&data.commodity_strings())
        .into_iter()
        .map(|(_, commodity)| commodity)
        .collect()
}

/// Each commodity's base price, as [`goods`] gives them.
fn prices(plugin: Option<Resources<'_>>) -> Vec<i64> {
    goods(plugin)
        .into_iter()
        .map(|commodity| commodity.base_price)
        .collect()
}

/// Each commodity's name, as [`goods`] gives them.
fn names(plugin: Option<Resources<'_>>) -> Vec<String> {
    goods(plugin)
        .into_iter()
        .map(|commodity| commodity.name)
        .collect()
}

const STOCK_NAMES: [&str; 6] = [
    "Food",
    "Industrial",
    "Medical Supplies",
    "Luxury Goods",
    "Metal",
    "Equipment",
];

const STOCK_PRICES: [i64; 6] = [75, 350, 750, 900, 200, 550];

#[test]
fn without_a_plugin_the_names_are_stock() {
    assert_eq!(names(None), STOCK_NAMES);
}

#[test]
fn a_plugins_str_9000_renames_food() {
    let plugin = [(StrResource::TYPE, 9000, str_resource("Grain"))];
    assert_eq!(
        names(Some(&plugin)),
        [
            "Grain",
            "Industrial",
            "Medical Supplies",
            "Luxury Goods",
            "Metal",
            "Equipment"
        ]
    );
    assert_eq!(prices(Some(&plugin)), STOCK_PRICES);
}

#[test]
fn a_plugins_str_9000_wins_its_slot_over_its_own_str_4000() {
    let plugin = [
        (
            StrList::TYPE,
            4000,
            str_list(&["A", "B", "C", "D", "E", "F"]),
        ),
        (StrResource::TYPE, 9000, str_resource("Grain")),
    ];
    assert_eq!(names(Some(&plugin)), ["Grain", "B", "C", "D", "E", "F"]);
}

#[test]
fn a_plugins_str_list_9000_names_nothing() {
    let plugin = [(StrList::TYPE, 9000, str_list(&["Grain"]))];
    assert_eq!(names(Some(&plugin)), STOCK_NAMES);
}

#[test]
fn a_plugins_zero_byte_str_9001_names_industrial_empty() {
    let plugin = [(StrResource::TYPE, 9001, Vec::new())];
    assert_eq!(
        names(Some(&plugin)),
        [
            "Food",
            "",
            "Medical Supplies",
            "Luxury Goods",
            "Metal",
            "Equipment"
        ]
    );
    assert_eq!(
        prices(Some(&plugin)),
        STOCK_PRICES,
        "still listed, at stock prices"
    );
}

#[test]
fn a_plugins_str_9002_is_read_as_its_mac_roman_bytes() {
    let plugin = [(StrResource::TYPE, 9002, str_bytes(b"Caf\x8e"))];
    assert_eq!(names(Some(&plugin))[2], "Café");
}

#[test]
fn without_a_plugin_the_prices_are_stock() {
    assert_eq!(prices(None), STOCK_PRICES);
}

#[test]
fn a_plugins_str_9301_prices_industrial() {
    let plugin = [(StrResource::TYPE, 9301, str_resource("999"))];
    assert_eq!(prices(Some(&plugin)), [75, 999, 750, 900, 200, 550]);
}

#[test]
fn a_plugins_str_patch_wins_its_slot_over_its_own_str_4004() {
    let plugin = [
        (
            StrList::TYPE,
            4004,
            str_list(&["1", "2", "3", "4", "5", "6"]),
        ),
        (StrResource::TYPE, 9301, str_resource("999")),
    ];
    assert_eq!(prices(Some(&plugin)), [1, 999, 3, 4, 5, 6]);
}

#[test]
fn a_plugins_str_9301_that_is_no_number_is_read_as_string_to_num_reads_it() {
    let plugin = [(StrResource::TYPE, 9301, str_resource("lots"))];
    assert_eq!(prices(Some(&plugin)), [75, 13543, 750, 900, 200, 550]);
}

#[test]
fn a_plugins_str_9302_is_read_as_its_mac_roman_bytes() {
    let plugin = [(StrResource::TYPE, 9302, str_bytes(&[0x8E, b'5']))];
    assert_eq!(prices(Some(&plugin)), [75, 350, 145, 900, 200, 550]);
}

#[test]
fn a_plugins_short_str_4004_prices_the_slots_it_lacks_at_0() {
    let plugin = [(StrList::TYPE, 4004, str_list(&["1", "2", "3"]))];
    assert_eq!(prices(Some(&plugin)), [1, 2, 3, 0, 0, 0]);
}

/// The text buoy `index` shows, from stock data holding `STR#` 1000 "Base
/// buoy." and, when given, a plug-in holding `plugin`, read through
/// `GameData`.
fn buoy(index: i16, plugin: Option<Resources<'_>>) -> Option<String> {
    let files = Files {
        data: fork(&[(StrList::TYPE, 1000, str_list(&["Base buoy."]))]),
        plugin: fork(plugin.unwrap_or_default()),
    };
    let plugins = plugin.map(|_| Path::new("/plugins"));
    let data = GameData::load(&files, &files, Path::new("/data"), plugins).expect("opens");
    let patch = patch_id(index).map_or(StringPatch::Absent, |id| data.string_patch(id));
    buoy_text(index, patch, &data.buoy_strings())
}

#[test]
fn a_plugins_str_1000_wins_buoy_1_over_the_base_datas_str_list_1000() {
    assert_eq!(buoy(1, None).as_deref(), Some("Base buoy."));
    let plugin = [(StrResource::TYPE, 1000, str_resource("Plug-in buoy."))];
    assert_eq!(buoy(1, Some(&plugin)).as_deref(), Some("Plug-in buoy."));
    assert_eq!(buoy(2, Some(&plugin)), None, "1001 is buoy 2's");
}

/// The words of hail quote `hail_quote`, from stock data holding `STR#`
/// 7101 "Base quote." and, when given, a plug-in holding `plugin`, read
/// through `GameData`.
fn hail_quote(hail_quote: i16, plugin: Option<Resources<'_>>) -> String {
    let files = Files {
        data: fork(&[(StrList::TYPE, HAIL_QUOTES, str_list(&["Base quote."]))]),
        plugin: fork(plugin.unwrap_or_default()),
    };
    let plugins = plugin.map(|_| Path::new("/plugins"));
    let data = GameData::load(&files, &files, Path::new("/data"), plugins).expect("opens");
    let patch = hail_quote_patch_id(hail_quote).map_or(StringPatch::Absent, |id| {
        CommCatalog::plugin_string(&data, id)
    });
    hail_quote_text(hail_quote, patch, &data.string_list(HAIL_QUOTES))
}

#[test]
fn a_plugins_str_5000_wins_hail_quote_1_over_the_base_datas_str_list_7101() {
    assert_eq!(hail_quote(1, None), "Base quote.");
    let plugin = [(StrResource::TYPE, 5000, str_resource("Plug-in quote."))];
    assert_eq!(hail_quote(1, Some(&plugin)), "Plug-in quote.");
    assert_eq!(hail_quote(2, Some(&plugin)), "", "5001 is quote 2's");
}
