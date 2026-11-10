//! The checked-in new-game folders (`tests/fixtures/new-game/`, and the
//! sample's cut, `tests/fixtures/sample/`), for other crates' tests to load
//! without retyping their paths.
//!
//! # `folder()`
//!
//! It holds the factions `guild` and `warden`; the systems `drift`, of
//! jump points alone, `hearth`, the guild's, and `verge`, ungoverned;
//! the stellars `hearth-dock`, `hearth-moon`, `verge-camp` and
//! `verge-workshop`, an unowned outpost with an outfitter and fuel, so a
//! lawless yard; jump
//! points that cover every case a link can take (two from `hearth` to
//! `verge`, a dead pair between `verge` and `drift`, and `drift`'s
//! one-way back door to `verge`); the civilian hull `hauler`, in guild
//! paint, priced 4000 and sold nowhere, the crown hull `cutter`, bare,
//! 50,000 at `hearth-dock`, and the wardens' crown hull `warden-picket`,
//! in `warden-colours`, unpriced and unsold, all on a placeholder model;
//! the variant `racked-hauler`, the hauler with `cargo-racks` and a nose
//! turret carrying a `mass-driver`, sold at `hearth-dock` for the
//! hauler's price plus the racks' (6000), so the start can buy it; the
//! outfits `dock-pass`, a Dock pass, and `mass-driver`, a turret weapon
//! sold at `hearth-dock` alone (`sold_at`), so it is listed there only
//! for a ship with a turret hardpoint; the legal mod kit `cargo-racks`,
//! fitting freighters, so Hearth Dock's outfitter lists it for the
//! hauler, the lawless kit `gun-mount`, fitting freighters, which
//! adds a turret hardpoint with no gun on it (the bare hauler has none),
//! so only Verge Workshop fits it, and the legal kit `turret-ring`
//! (1000), fitting freighters, which adds an empty turret hardpoint for
//! four tons of cargo, so Hearth Dock fits it to the racked hauler and
//! sells a second mass driver onto it; the start `dock-hand`, in the hauler
//! in `hearth`; seven goods, the six standard ones `alloys` (400 a ton),
//! `fuel-cells` (150), `grain` (100), `machinery` (600), `medicine` (800)
//! and `textiles` (250), in slots 0 to 5, and the special good
//! `vintage-wine` (1200), `jünk` 128; two trade stellars that price
//! them, `hearth-dock` (alloys and medicine medium, fuel cells and grain
//! low, machinery, textiles and vintage wine high) and `verge-camp`
//! (grain and medicine high, vintage wine low); and the storyline
//! `dock-work`, with its four missions and two news items:
//!
//! - `dock-job`: offered at `hearth-dock` (`job_offer`), ten tons of hull
//!   plate to `verge-camp`, completed there (`job_done`), each effect
//!   block used; its accept lends the `dock-pass` and its success takes
//!   it back and releases `plate-arrives`, from `verge`;
//! - `side-road-closes`: an event on day 8 that kills both ends of the
//!   side road and releases `side-road-closes`, from `hearth`;
//! - `warden-toll`: offered at `verge-camp` (`toll_offer`) once the side
//!   road is dead and the dock job done, under `all`, `any` and `none`
//!   groups, with no objective;
//! - `clear-the-road`: offered at `verge-camp` (`road_offer`) once the
//!   toll is refused, a `take_point` objective on the wardens' road,
//!   whose blockade is three warden pickets at the road's gate.
//!
//! Their conversations are one file, `dialog/dock-job.ink`, with its
//! compiled `dock-job.ink.json` beside it, which also holds `verge-camp`'s
//! own greeting (`camp_greeting`), the one stellar conversation. The
//! offer reads the world flag `dock_regular` and labels its once-only
//! `route` choice, whose read count adds a line to a later offer and
//! changes the completion's; the greeting reads `hauling_plate`. The
//! `.ink`'s header, and [`dialogs`](super::dialogs)' docs, give the
//! `inklecate` command that compiles it again.
//!
//! Each folder holds its ID table, `nova-ids.json`
//! ([`pins`](super::pins)), pinning every record it maps at the ID it had
//! before tables existed. After adding a record to either folder, run
//! `nova-dump --pin-ids` over it and commit the table with the record.
//!
//! # `modelled_sample()`
//!
//! The sample folder read into memory with its light freighter's model
//! and its mass driver's turret built in code, so its hardpoints stand on
//! a hull and show a turret: [`modelled_sample`]'s docs say what each is.
//!
//! # `modelled_sample_3d()`
//!
//! [`modelled_sample`] with a station model on both stations and a
//! generated planet in the start system, for the 3D flight screen:
//! [`modelled_sample_3d`]'s docs say what each is.

use std::path::{Path, PathBuf};

use nova_galaxy_format::source::MemSource;
use nova_galaxy_media::fixture::{Builder, Mat, Node};

/// The fixture folder's path.
#[must_use]
pub fn folder() -> PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("new-game")
}

/// The sample fixture folder's path: a three-system cut of game-bible's
/// sample galaxy, holding its "Holding the point" storyline and its first
/// failure, copied in so no test reads game-bible.
///
/// It holds the factions `commonwealth`, `association`, `harrow-band` and
/// `marrow-commons`, the last two lawless; the regions `commonwealth-rim` and
/// `association-border`; the systems `luyten-crossing` (the
/// commonwealth's), `brightwater` (the association's) and `marrow`, with
/// the sample's jump points between them alone: the Luyten road
/// (`luyten-crossing/association-road` to `brightwater/luyten-road`), the
/// Marrow road (`luyten-crossing/marrow-road` to `marrow/luyten-road`) and
/// the far gate (`marrow/far-gate` to `brightwater/marrow-gate`), every
/// one stable, the roads to the sample's other systems left out; the
/// stellars `luyten-exchange`, the commonwealth's, a legal yard, and
/// `marrow-yard`, the commons', a lawless one, each with a bar, trade, an
/// outfitter and fuel (the full sample's Marrow Yard has no outfitter:
/// the cut adds one, so a lawless yard is in reach); the character
/// `sector-organizer`, the missions' giver; the hull `light-freighter`, on
/// a placeholder model, and its variant `harrow-gunship`, in the band's
/// `harrow-raider` livery with a gun mount; the sample's mod kits `armour-plating` (legal) and
/// `gun-mount` (lawless, with a turret bed carrying a `mass-driver`), and
/// the outfit `mass-driver` (its model left out); the start
/// `first-flight`, in it, at `luyten-crossing` (the sample starts at
/// Ninefold, which the cut leaves out); and the storylines
/// `holding-the-point` and `the-first-failure`, with the sample's four
/// missions and one news item:
///
/// - `supply-run`: offered at `luyten-exchange` (`supply_run_offer`),
///   twelve tons to `marrow-yard`, completed there (`supply_run_done`),
///   setting `met_organizer`;
/// - `the-first-failure`: an event from day 60 that makes both ends of the
///   Luyten road unstable and releases `luyten-road-fails`, from
///   `luyten-crossing`;
/// - `the-crossroads-offer`: offered at `marrow-yard` (`crossroads_offer`)
///   once the Luyten road is unstable, its `on_offer` giving the far gate
///   to `harrow-band`, with no objective;
/// - `take-the-point`: offered at `marrow-yard`
///   (`take_the_point_briefing`) once the crossroads is done, a
///   `take_point` objective on the far gate, whose blockade is three
///   Harrow gunships at the gate, completed at `marrow-yard`
///   (`take_the_point_done`), giving the far gate to `marrow-commons`.
///
/// Their conversations are one file, `dialog/holding-the-point.ink`, with
/// its compiled `holding-the-point.ink.json` beside it, whose header gives
/// the `inklecate` command that compiles it again. The crossroads offer
/// reads the world flag `met_organizer` and the read count of
/// `supply_run_done`'s labelled `monists` choice.
///
/// Its `nova-ids.json` pins every record it maps, as the module docs say.
#[must_use]
pub fn sample_folder() -> PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("sample")
}

/// The light freighter's model file in the modelled sample.
pub const LIGHT_FREIGHTER_GLB: &str = "media/export/kenney-space-kit/craft_cargoA.glb";

/// The turret's model file in the modelled sample.
pub const TURRET_MOUNT_GLB: &str = "media/export/kenney-space-kit/turret_single.glb";

/// The light freighter's model in the modelled sample: 2.6 wide, 1.2
/// tall and 4 long (model x, y, z), as the Kenney kit's `craft_cargoA` is.
pub const LIGHT_FREIGHTER: [f32; 3] = [2.6, 1.2, 4.0];

/// The Kenney space kit's materials, in the kit's colours.
pub const KENNEY: [(&str, [f32; 4]); 4] = [
    ("metal", [0.8, 0.8, 0.82, 1.0]),
    ("metalRed", [0.7, 0.15, 0.1, 1.0]),
    ("metalDark", [0.35, 0.35, 0.4, 1.0]),
    ("dark", [0.1, 0.1, 0.12, 1.0]),
];

/// The provenance every Kenney model carries.
const KENNEY_CC0: &str = "licence: CC0-1.0\nlicence_file: media/export/kenney-space-kit/License.txt\n\
                          author: Kenney (www.kenney.nl)\n\
                          source_url: https://opengameart.org/content/space-kit-remade\n\
                          retrieved: 2026-10-30\n";

/// The sample fixture folder ([`sample_folder`]) read into memory, with
/// its two models made real, as game-bible's manifest gives them, so the
/// hardpoints have a hull to stand on and a turret to show:
///
/// - the `light-freighter` asset is a model facing −z, painted as the
///   sample's ships are (`primary: [metal]`, `secondary: [metalRed]`,
///   `trim: [metalDark]`), its file ([`LIGHT_FREIGHTER_GLB`]) a
///   [`LIGHT_FREIGHTER`] box in the kit's four materials
///   ([`KENNEY`], its top in `metalDark`);
/// - the `mass-driver` shows the new `turret-mount` asset
///   ([`TURRET_MOUNT_GLB`]), a model facing −z: one 0.4 by 0.3 by 0.4 box
///   drawn by four nodes of one group, at (0, 0, 0), (0, 0.3, 0),
///   (0, 0.3, −0.4) and (0, 0.3, −0.8), as the kit's turret is a tree of
///   nodes, so it is 0.4 wide, 0.6 tall and 1.2 long. It is all in the
///   kit's `metalRed`, so it shows against the hull's deck.
///
/// The boxes are built in code (`nova_galaxy_media::fixture`); no art is
/// checked in. They carry no normals, so a reader gives each triangle its
/// flat one, and every face seen from above is lit evenly.
///
/// # Panics
///
/// When the sample folder can't be read.
#[must_use]
pub fn modelled_sample() -> MemSource {
    let kenney = KENNEY
        .iter()
        .fold(Builder::new(LIGHT_FREIGHTER), |b, (name, color)| {
            b.material(Mat::new(name, *color))
        });
    let (red, red_color) = KENNEY[1];
    let turret = Builder::new([0.4, 0.3, 0.4])
        .material(Mat::new(red, red_color))
        .nodes(vec![
            Node::group(&[1, 2, 3, 4]),
            Node::at([0.0, 0.0, 0.0]),
            Node::at([0.0, 0.3, 0.0]),
            Node::at([0.0, 0.3, -0.4]),
            Node::at([0.0, 0.3, -0.8]),
        ]);
    read_into_memory(&sample_folder())
        .with(
            "assets/light-freighter.md",
            &format!(
                "---\nkind: model\nplaceholder: false\nfront: \"-z\"\n\
                 paint: {{primary: [metal], secondary: [metalRed], trim: [metalDark]}}\n\
                 files: [{LIGHT_FREIGHTER_GLB}]\n{KENNEY_CC0}---\n"
            ),
        )
        .with_bytes(LIGHT_FREIGHTER_GLB, &kenney.glb())
        .with(
            "assets/turret-mount.md",
            &format!(
                "---\nkind: model\nplaceholder: false\nfront: \"-z\"\n\
                 files: [{TURRET_MOUNT_GLB}]\n{KENNEY_CC0}---\n"
            ),
        )
        .with_bytes(TURRET_MOUNT_GLB, &turret.glb())
        .with(
            "outfits/mass-driver.md",
            "---\nname: Mass driver\nmount: turret\ndamage: 12\nrange: 600\nreload: 0.5\n\
             price: 2500\nmodel: turret-mount\n---\n",
        )
}

/// The station hub's model file in [`modelled_sample_3d`].
pub const STATION_HUB_GLB: &str = "media/export/kenney-space-kit/hangar_largeA.glb";

/// The station hub's model in [`modelled_sample_3d`]: 6 wide, 2 tall and
/// 6 long (model x, y, z), in metres.
pub const STATION_HUB: [f32; 3] = [6.0, 2.0, 6.0];

/// The ID the in-memory `nova-ids.json` of [`modelled_sample_3d`] pins
/// Kestrel Reach at: the next free `spöb` ID, after the sample's two
/// stations and its six gates.
pub const KESTREL_REACH_ID: i16 = 136;

/// [`modelled_sample`], with what the 3D flight screen draws besides the
/// ships, so every kind of world model has one in the start system:
///
/// - the `station-hub` asset, a model facing −z, its file
///   ([`STATION_HUB_GLB`]) a [`STATION_HUB`] box in the Kenney kit's four
///   materials ([`KENNEY`]), as both stations' `visual`
///   (`visual: {asset: station-hub}`), so each draws the hub, from above
///   as its stand-in;
/// - the generated gas giant `kestrel-reach` in `luyten-crossing`, the
///   start system, with the look the full sample gives it (palette
///   `#c9a36b`, `#8a5a3c`, `#e8d8b0`, clouds 0.6, rings 1.4 to 2.3 in
///   `#d8c8a8`, an atmosphere of `#f0e0c0` 0.04 thick, seed 4417),
///   placed where this cut puts it, not where the full sample does, and
///   pinned at [`KESTREL_REACH_ID`] in the in-memory `nova-ids.json`, so
///   the load reports no unpinned record.
///
/// [`modelled_sample`] stays as it is, so the tests over it keep their
/// meaning.
///
/// # Panics
///
/// When the sample folder can't be read.
#[must_use]
pub fn modelled_sample_3d() -> MemSource {
    let hub = KENNEY
        .iter()
        .fold(Builder::new(STATION_HUB), |b, (name, color)| {
            b.material(Mat::new(name, *color))
        });
    let read =
        |file: &str| std::fs::read_to_string(sample_folder().join(file)).expect("a sample file");
    // Each station's file, its `visual` the hub, added to its front matter.
    let station = |file: &str| {
        let text = read(file);
        let (front, body) = text
            .strip_prefix("---\n")
            .and_then(|rest| rest.split_once("\n---\n"))
            .expect("front matter");
        format!("---\n{front}\nvisual: {{asset: station-hub}}\n---\n{body}")
    };
    let ids = read("nova-ids.json");
    let pinned = ids.replacen(
        "\"marrow-yard\": 129",
        &format!("\"marrow-yard\": 129,\n    \"kestrel-reach\": {KESTREL_REACH_ID}"),
        1,
    );
    assert_ne!(pinned, ids, "Kestrel Reach is pinned");
    modelled_sample()
        .with(
            "assets/station-hub.md",
            &format!(
                "---\nkind: model\nplaceholder: false\nfront: \"-z\"\n\
                 files: [{STATION_HUB_GLB}]\n{KENNEY_CC0}---\n"
            ),
        )
        .with_bytes(STATION_HUB_GLB, &hub.glb())
        .with(
            "stellars/luyten-exchange.md",
            &station("stellars/luyten-exchange.md"),
        )
        .with(
            "stellars/marrow-yard.md",
            &station("stellars/marrow-yard.md"),
        )
        .with(
            "stellars/kestrel-reach.md",
            "---\nname: Kestrel Reach\nkind: planet\nsystem: luyten-crossing\n\
             position: [700, -500]\nvisual:\n  generated:\n    type: gas-giant\n\
             \x20   palette: [\"#c9a36b\", \"#8a5a3c\", \"#e8d8b0\"]\n    clouds: 0.6\n\
             \x20   rings: {inner: 1.4, outer: 2.3, color: \"#d8c8a8\"}\n\
             \x20   atmosphere: {color: \"#f0e0c0\", thickness: 0.04}\n    seed: 4417\n\
             ---\n\nA banded giant the crossing's traffic steers around.\n",
        )
        .with("nova-ids.json", &pinned)
}

/// Every file under `dir`, by its path from `dir` with `/` between its
/// parts.
fn read_into_memory(dir: &Path) -> MemSource {
    fn walk(dir: &Path, prefix: &str, mut into: MemSource) -> MemSource {
        let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)
            .expect("the fixture folder lists")
            .map(|entry| entry.expect("a fixture entry").path())
            .collect();
        entries.sort();
        for path in entries {
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .expect("a fixture name");
            let at = if prefix.is_empty() {
                name.to_owned()
            } else {
                format!("{prefix}/{name}")
            };
            into = if path.is_dir() {
                walk(&path, &at, into)
            } else {
                into.with_bytes(&at, &std::fs::read(&path).expect("a fixture file"))
            };
        }
        into
    }
    walk(dir, "", MemSource::default())
}
