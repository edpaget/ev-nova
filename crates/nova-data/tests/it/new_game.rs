//! The new game's folder adapter (`new_game::open`) over real folders:
//! the checked-in fixture (`tests/fixtures/new-game/`, published as
//! `new_game::fixture::folder`), and temporary
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
use nova_data::records::ship::Ship;
use nova_data::records::ship_anim::ShipAnim;
use nova_data::records::stellar::{Stellar, flags2};
use nova_data::records::system::System;
use nova_data::store::GameData;
use nova_data::{
    GovtId, Record, Registered, ShipId, StandIn, StandInOf, StellarId, SystemId, new_game,
};
use nova_galaxy_format::fleet::Size;
use nova_galaxy_media::BARE;

/// The checked-in fixture (`new_game::fixture::folder`): one region; the
/// factions `guild` and `warden`; the systems `drift` (jump points alone),
/// `hearth` (the guild's) and `verge` (ungoverned); three stellars:
/// `hearth-dock`, a guild station with every service; `hearth-moon`, with
/// none; and `verge-camp`, a warden outpost with a bar and no fuel; jump
/// points two of which lead from `hearth` to `verge`, a dead pair between
/// `verge` and `drift`, and `drift`'s one-way back door to `verge`; the
/// civilian hull `hauler` and the crown hull `cutter`; and the start
/// `dock-hand`, in the hauler in `hearth`.
fn fixture() -> PathBuf {
    new_game::fixture::folder()
}

/// Record `id`'s name and record.
fn named<T: Registered + Clone>(data: &GameData, id: i16) -> (String, T) {
    let entry = data.get::<T>(id).expect("present").expect("decoded");
    (entry.name.expect("named").to_owned(), entry.record.clone())
}

/// `table` as `(id, nova)` pairs.
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

#[test]
fn the_fixture_folder_loads_without_problems_or_pointers() {
    let game = new_game::open(&fixture()).expect("lists");
    let problems: Vec<_> = game.problems.iter().map(ToString::to_string).collect();
    assert_eq!(problems, Vec::<String>::new());
    assert_eq!(game.lfs_pointers, Vec::<String>::new());
}

#[test]
fn the_fixture_folder_holds_a_record_for_each_thing_it_maps() {
    let data = new_game::open(&fixture()).expect("lists").data;
    assert_eq!(data.ids(Govt::TYPE), [128, 129]);
    assert_eq!(data.ids(System::TYPE), [128, 129, 130]);
    assert_eq!(
        data.ids(Stellar::TYPE),
        (128..=139).collect::<Vec<_>>(),
        "three stellars, then nine jump points' gates"
    );
    assert_eq!(data.ids(Desc::TYPE), [128, 130], "the stellars with a body");
    assert_eq!(data.ids(Ship::TYPE), [128, 129], "the two hulls");
    assert_eq!(data.ids(ShipAnim::TYPE), [128, 129]);
    assert_eq!(data.ids(Dude::TYPE), [128], "the civilian traffic");
    assert_eq!(data.ids(Character::TYPE), [128], "the start");
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
    assert_eq!(ids(&verge.nav_def), [130, 137, 138, 139]);
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
        [("cutter".to_owned(), 128), ("hauler".to_owned(), 129)]
    );
    let (name, hauler) = named::<Ship>(&game.data, 129);
    assert_eq!(name, "Hauler");
    assert_eq!(
        (hauler.speed, hauler.shield, hauler.armor, hauler.holds),
        (300, 100, 200, 50)
    );
    assert_eq!((hauler.accel, hauler.maneuver, hauler.fuel), (300, 10, 300));
    let (_, anim) = named::<ShipAnim>(&game.data, 129);
    assert_eq!((anim.base_x_size, anim.base_y_size), (12, 30));
    let (name, dude) = named::<Dude>(&game.data, 128);
    assert_eq!(name, "Civilian traffic");
    assert_eq!(dude.ship_type[..2], [Some(ShipId(129)), None]);
    assert_eq!(dude.ai_type, 1);
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
fn the_ids_map_back_to_the_bible_s() {
    let ids = new_game::open(&fixture()).expect("lists").ids;
    assert_eq!(
        pairs(&ids.stellars),
        [
            ("hearth-dock".to_owned(), 128),
            ("hearth-moon".to_owned(), 129),
            ("verge-camp".to_owned(), 130)
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
    assert_eq!(game.data.ids(System::TYPE), [128, 129, 130]);
    assert!(game.problems.is_empty(), "{:?}", game.problems);
}
