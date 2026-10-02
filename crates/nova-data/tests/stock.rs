//! Tests against the stock EV Nova data files.
//!
//! The game data is copyrighted and never committed, so these tests read its
//! location from `NOVA_DATA` (the `Nova Files` directory) and skip, passing,
//! when it is unset.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use encoding_rs::MACINTOSH;
use nova_data::records::{
    boom::Boom, character::Character, checksum::Checksum, colors::Colors, cron::Cron, desc::Desc,
    disaster::Disaster, dude::Dude, fleet::Fleet, govt::Govt, interface::Interface, junk::Junk,
    mission::Mission, nebula::Nebula, outfit::Outfit, person::Person, rank::Rank, roid::Roid,
    ship::Ship, ship_anim::ShipAnim, spin::Spin, stellar::Stellar, string_list::StrList,
    system::System, weapon::Weapon,
};
use nova_data::{
    AnyRecord, Entry, GovtId, NovaId, OUT_OF_SCOPE, Record, Rect, ShipId, SystemId, TYPES,
    decode_all, decode_file,
};
use nova_rsrc::{ResType, ResourceFile};

/// The program edge for these tests: the only place `NOVA_DATA` is read.
fn nova_data() -> Option<PathBuf> {
    let dir = std::env::var_os("NOVA_DATA").map(PathBuf::from);
    if dir.is_none() {
        eprintln!("skipping: NOVA_DATA not set");
    }
    dir
}

/// Every `*.ndat` file in the data directory, sorted by name.
fn ndat_files(dir: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .expect("NOVA_DATA is a readable directory")
        .map(|entry| entry.expect("directory entry").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "ndat"))
        .collect();
    files.sort();
    files
}

/// Every stock file, opened.
fn stock_files(dir: &Path) -> Vec<(PathBuf, ResourceFile)> {
    ndat_files(dir)
        .into_iter()
        .map(|path| {
            let file = ResourceFile::open(&path).expect("stock file opens");
            (path, file)
        })
        .collect()
}

#[test]
fn every_stock_record_decodes_cleanly_and_round_trips_through_json() {
    let Some(dir) = nova_data() else { return };
    let mut problems = Vec::new();
    let mut decoded = 0;
    for (path, file) in stock_files(&dir) {
        let report = decode_file(&file);
        let name = path.file_name().expect("file name").to_string_lossy();
        problems.extend(report.errors.iter().map(|e| format!("{name}: error: {e}")));
        problems.extend(
            report
                .warnings
                .iter()
                .map(|w| format!("{name}: warning: {w}")),
        );
        for entry in &report.records {
            let json = serde_json::to_string(&entry.record).expect("serializes");
            let back: AnyRecord = serde_json::from_str(&json).expect("deserializes");
            if back != entry.record {
                problems.push(format!(
                    "{name}: {} {} does not round-trip through JSON",
                    entry.record.res_type(),
                    entry.id
                ));
            }
        }
        decoded += report.records.len();
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
    assert!(decoded > 0, "decoded some records");
}

/// Every record of type `T` across the stock files.
fn all<T: Record>(files: &[(PathBuf, ResourceFile)]) -> Vec<Entry<T>> {
    files
        .iter()
        .flat_map(|(_, file)| {
            let report = decode_all::<T>(file);
            assert!(report.errors.is_empty(), "{:?}", report.errors);
            report.entries
        })
        .collect()
}

fn mac_roman(text: &str) -> Vec<u8> {
    let (bytes, _, had_errors) = MACINTOSH.encode(text);
    assert!(!had_errors, "{text:?} is Mac Roman");
    bytes.into_owned()
}

/// Every stock `dësc` and `STR#` text maps back to exactly the bytes it was
/// decoded from, so no Mac Roman character was lost or substituted. The
/// stock text has no curly quotes (the synthetic tests cover those) but does
/// use accented letters.
#[test]
fn stock_text_decodes_mac_roman_losslessly() {
    let Some(dir) = nova_data() else { return };
    let files = stock_files(&dir);
    let mut non_ascii = String::new();

    for (_, file) in &files {
        for res in file.resources(Desc::TYPE) {
            let raw_text = &res.data()[..res
                .data()
                .iter()
                .position(|&b| b == 0)
                .expect("NUL-terminated")];
            let (entry, _) = nova_data::decode::<Desc>(&res).expect("decodes");
            assert_eq!(mac_roman(&entry.record.text), raw_text, "dësc {}", res.id());
            non_ascii.extend(entry.record.text.chars().filter(|c| !c.is_ascii()));
        }
        for res in file.resources(StrList::TYPE) {
            let data = res.data();
            let (entry, _) = nova_data::decode::<StrList>(&res).expect("decodes");
            let mut pos = 2;
            for text in &entry.record.strings {
                let len = usize::from(data[pos]);
                assert_eq!(
                    mac_roman(text),
                    &data[pos + 1..pos + 1 + len],
                    "STR# {}",
                    res.id()
                );
                pos += 1 + len;
                non_ascii.extend(text.chars().filter(|c| !c.is_ascii()));
            }
            assert_eq!(pos, data.len(), "STR# {} has no leftover bytes", res.id());
        }
    }

    for expected in ['é', 'ë', 'ö', 'æ', '°'] {
        assert!(non_ascii.contains(expected), "{expected} in stock text");
    }
    assert!(!non_ascii.contains('\u{FFFD}'));
    assert_eq!(all::<Desc>(&files).len(), 3032);
    assert_eq!(all::<StrList>(&files).len(), 227);
}

/// Every stock type is either decoded here or deliberately out of scope,
/// and all 25 in-scope stock types are registered.
#[test]
fn every_stock_type_is_registered_or_out_of_scope() {
    let Some(dir) = nova_data() else { return };
    let stock: BTreeSet<ResType> = stock_files(&dir)
        .iter()
        .flat_map(|(_, file)| file.types().collect::<Vec<_>>())
        .collect();
    let unknown: Vec<_> = stock
        .iter()
        .filter(|ty| !TYPES.contains(ty) && !OUT_OF_SCOPE.contains(ty))
        .collect();
    assert!(unknown.is_empty(), "unregistered stock types: {unknown:?}");
    let in_scope: Vec<_> = stock.iter().filter(|ty| TYPES.contains(ty)).collect();
    assert_eq!(in_scope.len(), 25, "{in_scope:?}");
}

/// Checks `T`'s fixed size and every stock record's length against the
/// counts and sizes measured from the stock data while planning.
fn sizes<T: Record>(files: &[(PathBuf, ResourceFile)], count: usize, size: usize) {
    assert_eq!(T::SIZE, Some(size), "{} layout size", T::TYPE);
    let lengths: Vec<usize> = files
        .iter()
        .flat_map(|(_, file)| {
            file.resources(T::TYPE)
                .map(|r| r.data().len())
                .collect::<Vec<_>>()
        })
        .collect();
    assert_eq!(lengths.len(), count, "{} count", T::TYPE);
    assert!(
        lengths.iter().all(|&len| len == size),
        "{} lengths",
        T::TYPE
    );
}

#[test]
fn every_fixed_layout_matches_the_stock_record_size() {
    let Some(dir) = nova_data() else { return };
    let files = stock_files(&dir);
    sizes::<Boom>(&files, 15, 6);
    sizes::<Character>(&files, 1, 362);
    sizes::<Cron>(&files, 125, 822);
    sizes::<Colors>(&files, 1, 244);
    sizes::<Dude>(&files, 147, 88);
    sizes::<Fleet>(&files, 128, 306);
    sizes::<Govt>(&files, 68, 192);
    sizes::<Junk>(&files, 23, 676);
    sizes::<Mission>(&files, 791, 1970);
    sizes::<Nebula>(&files, 4, 534);
    sizes::<Outfit>(&files, 242, 1028);
    sizes::<Person>(&files, 516, 400);
    sizes::<Rank>(&files, 31, 152);
    sizes::<Roid>(&files, 16, 40);
    sizes::<ShipAnim>(&files, 288, 192);
    sizes::<Ship>(&files, 288, 1860);
    sizes::<Spin>(&files, 136, 12);
    sizes::<Stellar>(&files, 411, 1118);
    sizes::<System>(&files, 545, 428);
    sizes::<Weapon>(&files, 81, 134);
    sizes::<Interface>(&files, 7, 166);
    sizes::<Disaster>(&files, 19, 282);
    assert_eq!(all::<Checksum>(&files).len(), 1);
}

/// Every record of type `T`, keyed by ID.
fn by_id<T: Record>(files: &[(PathBuf, ResourceFile)]) -> HashMap<i16, Entry<T>> {
    all::<T>(files).into_iter().map(|e| (e.id, e)).collect()
}

/// The starter ship. Expected values are from the EVN Wiki "Shuttle" page,
/// Version A (#128) stats table (evn.fandom.com/wiki/Shuttle), not from
/// this decoder.
#[test]
fn spot_check_the_shuttle() {
    let Some(dir) = nova_data() else { return };
    let ships = by_id::<Ship>(&stock_files(&dir));
    let shuttle = &ships[&128];
    let ship = &shuttle.record;
    assert_eq!(shuttle.name.as_deref(), Some("Shuttle"));
    assert_eq!(ship.cost, 10_000, "wiki: Purchase Cost 10K");
    assert_eq!(ship.speed, 400, "wiki: Speed 400");
    assert_eq!(ship.accel, 500, "wiki: Acceleration 500");
    assert_eq!(ship.shield, 30, "wiki: Shields 30");
    assert_eq!(ship.armor, 30, "wiki: Armor 30");
    assert_eq!(ship.holds, 10, "wiki: Cargo Space 10t");
    assert_eq!(ship.fuel, 300, "wiki: Energy 300 (3 jumps)");
    assert_eq!(ship.free_mass, 8, "wiki: Free Outfit Space 8t");
    assert_eq!(ship.max_gun, 2, "wiki: Max Guns 2");
    assert_eq!(ship.tech_level, 3, "wiki: Tech Level 3");
    assert_eq!(ship.crew, 1, "wiki: Crew 1");
    assert_eq!(ship.mass, 15, "wiki: Mass 15t");
    assert_eq!(ship.length, 8, "wiki: Length 8m");
    assert_eq!(ship.strength, 2, "wiki: Strength 2");
}

/// Sol. Expected values are from the EVN Wiki "Sol" page infobox for system
/// 130 (evn.fandom.com/wiki/Sol), not from this decoder. Selected by ID:
/// system 531 is also named Sol.
#[test]
fn spot_check_sol() {
    let Some(dir) = nova_data() else { return };
    let files = stock_files(&dir);
    let systems = by_id::<System>(&files);
    let govts = by_id::<Govt>(&files);
    let sol = &systems[&130];
    assert_eq!(sol.name.as_deref(), Some("Sol"));
    let syst = &sol.record;
    let links: BTreeSet<&str> = syst
        .con
        .iter()
        .flatten()
        .map(|id| systems[&id.raw()].name.as_deref().expect("named"))
        .collect();
    let expected: BTreeSet<&str> = [
        "Alphara",
        "Archenar",
        "Kerella",
        "Nesre Primus",
        "Nesre Secundus",
        "Tau Ceti",
        "Tichel",
        "Wolf 359",
    ]
    .into();
    assert_eq!(links, expected, "wiki: Links");
    assert_eq!((syst.x_pos, syst.y_pos), (0, 0), "wiki: x 0, y 0");
    let govt = syst.govt.expect("governed");
    assert_eq!(
        govts[&govt.raw()].name.as_deref(),
        Some("Federation"),
        "wiki: Government"
    );
    assert_eq!(
        (syst.interference, syst.murk),
        (0, 0),
        "wiki: Interference 0, Murk 0"
    );
    assert_eq!(syst.bkgnd_color, 0, "wiki: BG colour #000000");
    assert_eq!(syst.avg_ships, 6, "wiki: 6 ships on average");
    assert_eq!(syst.asteroids, 2, "wiki: 2 roids");
}

/// Hyperlinks that point one way only even after allowing for replacement
/// systems: genuine one-way links in the stock data (found by this test,
/// listed rather than loosening the check).
const ONE_WAY_LINKS: &[(i16, i16)] = &[(558, 557), (561, 563), (584, 590), (585, 582), (625, 616)];

/// Hyperlinks name existing systems and are symmetric. The Bible says
/// replacement systems share their original's exact coordinates and Nova
/// updates links by position, so a link back to any system at the same
/// coordinates counts.
#[test]
fn system_links_and_nav_defaults_resolve() {
    let Some(dir) = nova_data() else { return };
    let files = stock_files(&dir);
    let systems = by_id::<System>(&files);
    let stellars = by_id::<Stellar>(&files);
    let position = |id: i16| {
        let s = &systems[&id].record;
        (s.x_pos, s.y_pos)
    };
    let mut one_way = Vec::new();
    for (&id, entry) in &systems {
        for link in entry.record.con.iter().flatten() {
            let target = systems
                .get(&link.raw())
                .expect("link to an existing system");
            let links_back = target
                .record
                .con
                .iter()
                .flatten()
                .any(|back| position(back.raw()) == position(id));
            if !links_back {
                one_way.push((id, link.raw()));
            }
        }
        for nav in entry.record.nav_def.iter().flatten() {
            assert!(stellars.contains_key(&nav.raw()), "sÿst {id} nav {nav:?}");
        }
    }
    one_way.sort_unstable();
    assert_eq!(one_way, ONE_WAY_LINKS);
}

/// Whether `id` is unused under the Bible's "0 or -1 if unused" convention.
fn unused<T: NovaId>(id: Option<T>) -> bool {
    id.is_none_or(|id| id.raw() == 0)
}

#[test]
fn ship_references_resolve() {
    let Some(dir) = nova_data() else { return };
    let files = stock_files(&dir);
    let weapons = by_id::<Weapon>(&files);
    let outfits = by_id::<Outfit>(&files);
    let govts = by_id::<Govt>(&files);
    for (id, entry) in by_id::<Ship>(&files) {
        let ship = &entry.record;
        for weapon in ship.weap_type1_4.iter().chain(&ship.weap_type5_8) {
            assert!(
                unused(*weapon) || weapons.contains_key(&weapon.map_or(0, NovaId::raw)),
                "shïp {id} {weapon:?}"
            );
        }
        for item in ship.default_items1_4.iter().chain(&ship.default_items5_8) {
            assert!(
                unused(*item) || outfits.contains_key(&item.map_or(0, NovaId::raw)),
                "shïp {id} {item:?}"
            );
        }
        // InherentGovt: -1, or a gövt ID plus 0, 1000 or 2000 (Bible).
        if ship.inherent_govt != -1 {
            let govt = ship.inherent_govt % 1000;
            assert!(
                ship.inherent_govt / 1000 <= 2,
                "shïp {id} {}",
                ship.inherent_govt
            );
            assert!(
                govts.contains_key(&govt),
                "shïp {id} govt {}",
                ship.inherent_govt
            );
        }
    }
}

/// Dude ship types that name no `shïp` in the stock data: düde 267
/// "Duellists" lists ship type 20 at 25%. Found by this test and listed
/// rather than loosening the check.
const DANGLING_DUDE_SHIPS: &[(i16, i16)] = &[(267, 20)];

#[test]
fn person_and_dude_references_resolve() {
    let Some(dir) = nova_data() else { return };
    let files = stock_files(&dir);
    let govts = by_id::<Govt>(&files);
    let ships = by_id::<Ship>(&files);
    let known = |govt: Option<GovtId>| govt.is_none_or(|g| govts.contains_key(&g.raw()));
    for (id, entry) in by_id::<Person>(&files) {
        assert!(known(entry.record.govt), "përs {id}");
        let ship = entry.record.ship_type.map(NovaId::raw);
        assert!(ship.is_some_and(|s| ships.contains_key(&s)), "përs {id}");
    }
    let mut dangling = Vec::new();
    for (id, entry) in by_id::<Dude>(&files) {
        assert!(known(entry.record.govt), "düde {id}");
        for ship in entry.record.ship_type {
            let ship: Option<ShipId> = ship;
            if !unused(ship) && !ship.is_some_and(|s| ships.contains_key(&s.raw())) {
                dangling.push((id, ship.map_or(0, NovaId::raw)));
            }
        }
    }
    dangling.sort_unstable();
    assert_eq!(dangling, DANGLING_DUDE_SHIPS);
}

/// Every stock status-bar and progress-bar rectangle has its edges in
/// QuickDraw order, which pins the rectangle field order.
#[test]
fn stock_rectangles_are_well_formed() {
    let Some(dir) = nova_data() else { return };
    let files = stock_files(&dir);
    let ordered = |r: &Rect| r.top <= r.bottom && r.left <= r.right;
    for entry in all::<Interface>(&files) {
        let i = &entry.record;
        for area in [
            &i.radar_area,
            &i.shield_area,
            &i.armor_area,
            &i.fuel_area,
            &i.nav_area,
            &i.weap_area,
            &i.targ_area,
            &i.cargo_area,
        ] {
            assert!(ordered(area), "ïntf {} {area:?}", entry.id);
        }
    }
    for entry in all::<Colors>(&files) {
        assert!(ordered(&entry.record.progress_bar), "cölr {}", entry.id);
    }
}

/// The starting character's systems exist.
#[test]
fn character_start_systems_resolve() {
    let Some(dir) = nova_data() else { return };
    let files = stock_files(&dir);
    let systems = by_id::<System>(&files);
    let names: BTreeMap<i16, Vec<Option<SystemId>>> = all::<Character>(&files)
        .into_iter()
        .map(|e| (e.id, e.record.system.to_vec()))
        .collect();
    for (id, starts) in names {
        for start in starts.into_iter().flatten() {
            assert!(systems.contains_key(&start.raw()), "chär {id} {start:?}");
        }
    }
}
