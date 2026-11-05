//! The new game's folder adapter (`new_game::open`) over real folders:
//! the checked-in fixture (`tests/fixtures/new-game/`), and temporary
//! folders for a root that can't be listed and for media that are Git LFS
//! pointers.

use std::fs;
use std::path::{Path, PathBuf};

use nova_data::records::desc::Desc;
use nova_data::records::govt::Govt;
use nova_data::records::stellar::Stellar;
use nova_data::records::system::System;
use nova_data::store::GameData;
use nova_data::{GovtId, Record, Registered, StellarId, new_game};

/// The checked-in fixture: one region, the factions `guild` and `warden`,
/// the systems `hearth` (the guild's) and `verge` (ungoverned), linked by
/// a jump point each, and three stellars: `hearth-dock`, a guild station
/// with every service; `hearth-moon`, with none; and `verge-camp`, a
/// warden outpost with a bar and no fuel.
fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("new-game")
}

/// Record `id`'s name and record.
fn named<T: Registered + Clone>(data: &GameData, id: i16) -> (String, T) {
    let entry = data.get::<T>(id).expect("present").expect("decoded");
    (entry.name.expect("named").to_owned(), entry.record.clone())
}

#[test]
fn the_fixture_folder_loads_without_problems_or_pointers() {
    let game = new_game::open(&fixture()).expect("lists");
    let problems: Vec<_> = game.problems.iter().map(ToString::to_string).collect();
    assert_eq!(problems, Vec::<String>::new());
    assert_eq!(game.lfs_pointers, Vec::<String>::new());
}

#[test]
fn the_fixture_folder_holds_one_record_per_faction_system_and_stellar() {
    let data = new_game::open(&fixture()).expect("lists").data;
    assert_eq!(data.ids(Govt::TYPE), [128, 129]);
    assert_eq!(data.ids(System::TYPE), [128, 129]);
    assert_eq!(data.ids(Stellar::TYPE), [128, 129, 130]);
    assert_eq!(data.ids(Desc::TYPE), [128, 130], "the stellars with a body");
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
fn the_systems_are_placed_governed_and_hold_their_stellars_with_no_links() {
    let data = new_game::open(&fixture()).expect("lists").data;
    let (name, hearth) = named::<System>(&data, 128);
    assert_eq!(name, "Hearth");
    assert_eq!((hearth.x_pos, hearth.y_pos), (0, 0));
    assert_eq!(hearth.govt, Some(GovtId(128)));
    assert_eq!(
        hearth.nav_def[..3],
        [Some(StellarId(128)), Some(StellarId(129)), None]
    );
    let (name, verge) = named::<System>(&data, 129);
    assert_eq!(name, "Verge");
    assert_eq!((verge.x_pos, verge.y_pos), (300, -120));
    assert_eq!(verge.govt, None);
    assert_eq!(verge.nav_def[..2], [Some(StellarId(130)), None]);
    for system in [hearth, verge] {
        assert_eq!(system.con, [None; 16], "the jump points are not links");
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
    assert_eq!(dock.flags.bits(), 0x5F);
    let (name, moon) = named::<Stellar>(&data, 129);
    assert_eq!(name, "Hearth's Moon");
    assert_eq!(moon.govt, None);
    assert_eq!(moon.flags.bits(), 0x20, "uninhabited, no landing");
    let (name, camp) = named::<Stellar>(&data, 130);
    assert_eq!(name, "Verge Camp");
    assert_eq!(camp.govt, Some(GovtId(129)));
    assert_eq!(
        camp.flags.bits(),
        0x71,
        "landing, a station, a bar, no fuel"
    );
    assert_eq!(camp.hyper_link, [None; 8]);
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
fn the_ids_map_back_to_the_bible_s() {
    let ids = new_game::open(&fixture()).expect("lists").ids;
    let names = |table: &std::collections::BTreeMap<nova_galaxy_format::id::Id, i16>| {
        table
            .iter()
            .map(|(id, &nova)| (id.to_string(), nova))
            .collect::<Vec<_>>()
    };
    assert_eq!(
        names(&ids.stellars),
        [
            ("hearth-dock".to_owned(), 128),
            ("hearth-moon".to_owned(), 129),
            ("verge-camp".to_owned(), 130)
        ]
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
    assert_eq!(game.data.ids(System::TYPE), [128, 129]);
    assert!(game.problems.is_empty(), "{:?}", game.problems);
}
