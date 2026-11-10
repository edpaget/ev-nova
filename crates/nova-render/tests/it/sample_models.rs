//! The new game's sample models through [`GalaxyModels`]: the six model
//! assets with files load, placed and scaled from data, and meet the
//! `ModelSource` contract; a loaded model reaches the renderer.
//!
//! The sample's manifest, ships, outfit, stations and budgets live in
//! game-bible's `data` folder, which this repository never needs. So the
//! hermetic tests build a galaxy in memory that mirrors them: the same
//! asset ids, `front`s, `paint`s and files, the ships with their roles,
//! models and lengths, the outfit and stations that use `station-hub` and
//! `turret-mount`, the placeholders, and the three budgets with their
//! numbers, copied by hand. The models themselves are boxes built in code
//! (`nova_galaxy_media::fixture`) in the Kenney space kit's four unlit
//! materials, with normals and texture coordinates, and the turret as a
//! tree of nodes; no art enters the repo.
//!
//! [`the_bible_s_sample_models_load`] reads the real sample through
//! [`DirSource`] when `GAME_DATA` names it, and skips, passing, when it is
//! unset.

use nova_galaxy_format::asset::AssetKind;
use nova_galaxy_format::id::Id;
use nova_galaxy_format::load::{Galaxy, load};
use nova_galaxy_format::source::{DirSource, MemSource, Source};
use nova_galaxy_media::fixture::{Builder, Mat, Node};
use nova_render::model_source::{PaintSlot, check_contract};
use nova_render::recording::{GpuCall, NoImages, RecordingGpu};
use nova_render::{
    GalaxyModels, LogicalSize, ModelError, ModelRequest, ModelSource, Renderer, Viewport,
    ship_request,
};
use nova_view::{DrawList, ModelKey, ModelPose, Orientation, Point};

use crate::common::game_data;

/// Where the sample's models sit.
const PACK: &str = "media/export/kenney-space-kit";

/// The Kenney space kit's materials, each unlit, in the kit's colours.
const KENNEY: [(&str, [f32; 4]); 4] = [
    ("metal", [0.8, 0.8, 0.82, 1.0]),
    ("metalRed", [0.7, 0.15, 0.1, 1.0]),
    ("metalDark", [0.35, 0.35, 0.4, 1.0]),
    ("dark", [0.1, 0.1, 0.12, 1.0]),
];

/// The paint every sample ship's model carries.
const PAINT: &str = "paint: {primary: [metal], secondary: [metalRed], trim: [metalDark]}\n";

/// The provenance every sample model carries.
const KENNEY_CC0: &str = "licence: CC0-1.0\nlicence_file: media/export/kenney-space-kit/License.txt\n\
                          author: Kenney (www.kenney.nl)\n\
                          source_url: https://opengameart.org/content/space-kit-remade\n\
                          retrieved: 2026-10-30\n";

/// One of the sample's six model assets with a file: its id, its
/// `front` (`None` for the default, +z), whether it is painted, its file
/// in the pack, and the box standing in for it.
struct SampleModel {
    asset: &'static str,
    front: Option<&'static str>,
    painted: bool,
    file: &'static str,
    model: fn() -> Vec<u8>,
}

const MODELS: [SampleModel; 6] = [
    SampleModel {
        asset: "association-carrier",
        front: Some("-z"),
        painted: true,
        file: "craft_cargoB.glb",
        model: || kenney([4.0, 1.5, 6.0]).glb(),
    },
    SampleModel {
        asset: "civilian-shuttle",
        front: Some("-z"),
        painted: true,
        file: "craft_speederB.glb",
        model: || kenney([1.5, 0.8, 2.0]).glb(),
    },
    SampleModel {
        asset: "commonwealth-corsair",
        front: Some("-z"),
        painted: true,
        file: "craft_speederA.glb",
        model: || kenney([2.0, 0.6, 2.2]).glb(),
    },
    SampleModel {
        asset: "light-freighter",
        front: Some("-z"),
        painted: true,
        file: "craft_cargoA.glb",
        model: || kenney([2.6, 1.2, 4.0]).glb(),
    },
    SampleModel {
        asset: "station-hub",
        front: None,
        painted: false,
        file: "hangar_roundA.glb",
        model: || kenney([6.0, 2.0, 6.0]).glb(),
    },
    SampleModel {
        asset: "turret-mount",
        front: Some("-z"),
        painted: false,
        file: "turret_single.glb",
        // A base drawing four parts, as the kit's turret is a tree of
        // nodes: here one mesh, drawn by four children of one group.
        model: || {
            kenney([0.4, 0.3, 0.4])
                .nodes(vec![
                    Node::group(&[1, 2, 3, 4]),
                    Node::at([0.0, 0.0, 0.0]),
                    Node::at([0.0, 0.3, 0.0]),
                    Node::at([0.0, 0.3, -0.4]),
                    Node::at([0.0, 0.3, -0.8]),
                ])
                .glb()
        },
    },
];

/// The sample's placeholders: two models and a texture, none with a file.
const PLACEHOLDERS: [(&str, &str); 3] = [
    ("cathedral-cruiser", "model"),
    ("patchwork-station", "model"),
    ("registry-numbers", "texture"),
];

/// A box `size` big in the kit's four unlit materials, with normals and
/// texture coordinates.
fn kenney(size: [f32; 3]) -> Builder {
    KENNEY.iter().fold(
        Builder::new(size).normals().texcoords(),
        |b, (name, color)| {
            b.material(Mat {
                unlit: true,
                ..Mat::new(name, *color)
            })
        },
    )
}

/// The sample, in memory: its assets (and the six models' files), ships,
/// outfit, stations and budgets, as game-bible's `data` gives them.
fn sample() -> MemSource {
    with_users(with_assets(MemSource::default()))
}

/// `source` with the sample's asset manifest and the six models' files.
fn with_assets(mut source: MemSource) -> MemSource {
    for model in &MODELS {
        let front = model
            .front
            .map_or(String::new(), |f| format!("front: {f}\n"));
        let paint = if model.painted { PAINT } else { "" };
        let path = format!("{PACK}/{}", model.file);
        source = source
            .with(
                &format!("assets/{}.md", model.asset),
                &format!(
                    "---\nkind: model\nplaceholder: false\n{front}{paint}files: [{path}]\n\
                     {KENNEY_CC0}---\n"
                ),
            )
            .with_bytes(&path, &(model.model)());
    }
    for (asset, kind) in PLACEHOLDERS {
        source = source.with(
            &format!("assets/{asset}.md"),
            &format!("---\nkind: {kind}\nplaceholder: true\n---\n"),
        );
    }
    let surfaces = [
        (
            "lantern-cloister-surface",
            "Rock-EQUIRECTANGULAR-7-2048x1024.png",
        ),
        (
            "marrow-fields-surface",
            "Oceanic-Clouds-EQUIRECTANGULAR-2-2048x1024.png",
        ),
    ];
    for (asset, file) in surfaces {
        source = source.with(
            &format!("assets/{asset}.md"),
            &format!(
                "---\nkind: texture\nplaceholder: false\n\
                 files: [media/export/vektordex-planets/{file}]\nlicence: CC0-1.0\n\
                 author: vektorDex\n\
                 source_url: https://opengameart.org/content/planetary-textures-2048x1024\n\
                 retrieved: 2026-10-30\n---\n"
            ),
        );
    }
    source
}

/// `source` with what uses the sample's models: its ships, stations and
/// outfit, and its budgets.
fn with_users(mut source: MemSource) -> MemSource {
    let ships = [
        (
            "cathedral-cruiser",
            "cruiser",
            "cathedral-cruiser",
            180,
            60,
            80,
        ),
        ("corsair", "raider", "commonwealth-corsair", 22, 21, 11),
        (
            "fleet-carrier",
            "carrier",
            "association-carrier",
            320,
            192,
            150,
        ),
        (
            "light-freighter",
            "freighter",
            "light-freighter",
            40,
            26,
            20,
        ),
        ("shuttle", "shuttle", "civilian-shuttle", 16, 16, 8),
    ];
    for (ship, role, model, length, beam, radius) in ships {
        source = source.with(
            &format!("ships/{ship}.md"),
            &format!(
                "---\nname: {ship}\nrole: {role}\nmodel: {model}\n\
                 size: {{length: {length}, beam: {beam}, collision_radius: {radius}}}\n---\n"
            ),
        );
    }
    for (ship, base) in [
        ("armed-freighter", "light-freighter"),
        ("blockade-runner", "light-freighter"),
        ("technical", "shuttle"),
    ] {
        source = source.with(
            &format!("ships/{ship}.md"),
            &format!("---\nname: {ship}\nbase: {base}\n---\n"),
        );
    }
    for (stellar, system) in [
        ("kestrel-hold", "kestrel-march"),
        ("luyten-exchange", "luyten-crossing"),
    ] {
        source = source.with(
            &format!("stellars/{stellar}.md"),
            &format!(
                "---\nname: {stellar}\nkind: station\nsystem: {system}\nposition: [0, 0]\n\
                 visual: {{asset: station-hub}}\n---\n"
            ),
        );
    }
    source
        .with(
            "outfits/mass-driver.md",
            "---\nname: Mass driver\nmount: turret\ndamage: 12\nrange: 600\nreload: 0.5\n\
             price: 2500\nmodel: turret-mount\n---\n",
        )
        .with(
            "budgets/capital.md",
            "---\nroles: [cruiser, carrier]\nmodel: {triangles: 60000, kib: 8192}\n---\n",
        )
        .with(
            "budgets/default.md",
            "---\nmodel: {triangles: 20000, kib: 4096}\ntexture: {kib: 4096, side: 4096}\n---\n",
        )
        .with(
            "budgets/small-craft.md",
            "---\nroles: [shuttle, courier, racer, fighter, raider, gunboat]\n\
             model: {triangles: 10000, kib: 2048}\n---\n",
        )
}

/// The model assets of `galaxy` that aren't placeholders.
fn model_assets(galaxy: &Galaxy) -> Vec<&Id> {
    galaxy
        .assets
        .iter()
        .filter(|(_, a)| a.data.kind == AssetKind::Model && !a.data.placeholder)
        .map(|(id, _)| id)
        .collect()
}

/// The request for `asset` from data: the length of the first ship that
/// names it as its own model, or as modelled for a station's or an
/// outfit's model.
fn request_for(galaxy: &Galaxy, asset: &Id) -> ModelRequest {
    galaxy
        .ships
        .iter()
        .find(|(_, ship)| ship.data.model.as_ref() == Some(asset))
        .and_then(|(ship, _)| ship_request(galaxy, ship))
        .unwrap_or_else(|| ModelRequest::new(asset.as_str(), None))
}

/// How far `vertices` reach along each axis.
fn spans(mesh: &nova_render::Mesh) -> [f32; 3] {
    [0, 1, 2].map(|axis| {
        let values = mesh.vertices().iter().map(|v| v.position[axis]);
        let max = values.clone().fold(f32::MIN, f32::max);
        let min = values.fold(f32::MAX, f32::min);
        max - min
    })
}

#[test]
fn the_sample_s_six_models_load_through_the_adapter_and_meet_the_contract() {
    let source = sample();
    let (galaxy, problems) = load(&source);
    assert!(problems.is_empty(), "{problems:?}");
    let assets: Vec<&str> = model_assets(&galaxy).iter().map(|id| id.as_str()).collect();
    let expected: Vec<&str> = MODELS.iter().map(|m| m.asset).collect();
    assert_eq!(assets, expected, "exactly the six");

    let models = GalaxyModels::new(&galaxy, &source);
    for asset in model_assets(&galaxy) {
        let request = request_for(&galaxy, asset);
        check_contract(&models, &request, "no-such-asset");
    }
    let station = request_for(&galaxy, &Id::try_from("station-hub".to_owned()).unwrap());
    assert_eq!(station, ModelRequest::new("station-hub", None));
    let turret = request_for(&galaxy, &Id::try_from("turret-mount".to_owned()).unwrap());
    assert_eq!(turret, ModelRequest::new("turret-mount", None));

    // The light freighter, 40 m long, its nose up the screen: 2.6 by
    // 1.2 by 4 units, so 26 m across and 12 m deep.
    let freighter = request_for(
        &galaxy,
        &Id::try_from("light-freighter".to_owned()).unwrap(),
    );
    assert_eq!(freighter, ModelRequest::new("light-freighter", Some(40.0)));
    let mesh = models.model(&freighter).unwrap().mesh();
    let [across, along, deep] = spans(&mesh);
    assert!((along - 40.0).abs() < 1e-3, "{along}");
    assert!((across - 26.0).abs() < 1e-3, "{across}");
    assert!((deep - 12.0).abs() < 1e-3, "{deep}");
    // The turret's four parts, each in the kit's four materials.
    let turret = models.model(&turret).unwrap();
    assert_eq!(turret.parts.len(), 16);
    let names: Vec<_> = turret.materials.iter().map(|m| m.name.as_deref()).collect();
    assert_eq!(
        names,
        [
            Some("metal"),
            Some("metalRed"),
            Some("metalDark"),
            Some("dark")
        ]
    );
    assert!(turret.materials.iter().all(|m| m.unlit));

    // The painted models put the kit's metals in the livery's three slots
    // and leave `dark` as drawn; the station and the turret paint nothing.
    for sample in &MODELS {
        let asset = Id::try_from(sample.asset.to_owned()).unwrap();
        let model = models.model(&request_for(&galaxy, &asset)).unwrap();
        let slots: Vec<_> = model
            .materials
            .iter()
            .map(|m| (m.name.as_deref(), m.paint))
            .collect();
        let expected = if sample.painted {
            [
                Some(PaintSlot::Primary),
                Some(PaintSlot::Secondary),
                Some(PaintSlot::Trim),
                None,
            ]
        } else {
            [None; 4]
        };
        let names = ["metal", "metalRed", "metalDark", "dark"].map(Some);
        let expected: Vec<_> = names.into_iter().zip(expected).collect();
        assert_eq!(slots, expected, "{}", sample.asset);
    }
}

#[test]
fn the_sample_s_placeholders_have_no_file() {
    // `cathedral-cruiser` and `patchwork-station` are placeholder models;
    // `registry-numbers` is a placeholder texture.
    let source = sample();
    let (galaxy, _) = load(&source);
    let models = GalaxyModels::new(&galaxy, &source);
    for (asset, kind) in PLACEHOLDERS {
        let expected = match kind {
            "model" => ModelError::NoFile(asset.to_owned()),
            _ => ModelError::NotAModel(asset.to_owned()),
        };
        assert_eq!(
            models.model(&ModelRequest::new(asset, None)),
            Err(expected),
            "{asset}"
        );
    }
    // The cathedral cruiser's request comes from data all the same.
    let cruiser = request_for(
        &galaxy,
        &Id::try_from("cathedral-cruiser".to_owned()).unwrap(),
    );
    assert_eq!(cruiser, ModelRequest::new("cathedral-cruiser", Some(180.0)));
    assert_eq!(
        models.model(&cruiser),
        Err(ModelError::NoFile("cathedral-cruiser".into()))
    );
}

#[test]
fn a_loaded_model_reaches_the_renderer() {
    const LOGICAL: LogicalSize = LogicalSize { w: 64, h: 48 };
    const SHUTTLE: ModelKey = ModelKey(7);
    let source = sample();
    let (galaxy, _) = load(&source);
    let request = request_for(
        &galaxy,
        &Id::try_from("civilian-shuttle".to_owned()).unwrap(),
    );
    let loaded = GalaxyModels::new(&galaxy, &source).model(&request).unwrap();

    let mut renderer = Renderer::new(NoImages);
    renderer.add_model(SHUTTLE, loaded.mesh());
    let mut list = DrawList::new();
    list.model(
        SHUTTLE,
        ModelPose {
            center: Point::new(32.0, 24.0),
            orientation: Orientation::about([0.0, 0.0, 1.0], 0.0),
            scale: 1.0,
        },
    );
    let mut gpu = RecordingGpu::new();
    let report = renderer.render(&list, &Viewport::new(LOGICAL, (128, 96), 2.0), &mut gpu);

    assert!(report.missing_models.is_empty());
    let uploads: Vec<usize> = gpu
        .calls
        .iter()
        .filter_map(|call| match call {
            GpuCall::UploadMesh { data, .. } => Some(data.vertices().len()),
            _ => None,
        })
        .collect();
    assert_eq!(uploads, [loaded.mesh().vertices().len()]);
}

#[test]
fn the_bible_s_sample_models_load() {
    let Some(dir) = game_data() else { return };
    let source = DirSource::new(&dir);
    let (galaxy, _) = load(&source);
    let models = GalaxyModels::new(&galaxy, &source);
    for asset in model_assets(&galaxy) {
        let request = request_for(&galaxy, asset);
        match models.model(&request) {
            Err(ModelError::LfsPointer(asset)) => {
                eprintln!("`{asset}` is a Git LFS pointer, not fetched");
            }
            _ => check_contract(&models, &request, "no-such-asset"),
        }
    }
    // The format's Source reads the folder as the hermetic sample does.
    assert!(
        source
            .bytes("budgets/default.md")
            .is_ok_and(|b| b.is_some())
    );
}
