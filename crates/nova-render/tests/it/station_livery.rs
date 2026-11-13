//! A new-game station drawn offscreen in its owner's livery
//! (rdm:phase/rendering-3d/phase-9-stations-in-owner-livery):
//! [`world_assets`] over the liveried-stations sample
//! (`nova_data::new_game::fixture::liveried_stations_3d`), every mesh
//! registered with a renderer, and the two stations' keys drawn side by
//! side, unturned, from above, at native scale. Luyten Exchange, the
//! commonwealth's, shows the commonwealth's livery on its top; Marrow
//! Yard, whose owner no livery names, shows the Kenney kit's own colour.
//!
//! Each expected colour is worked out from the meshes and the livery, lit
//! as the core shades a face facing the camera: the exchange's from the
//! slot its top's material is in, cross-checked against the livery's
//! colour for that slot.
//!
//! Needs a GPU adapter; skips, passing, without one.

use std::path::Path;

use nova_data::new_game::fixture::liveried_stations_3d;
use nova_data::new_game::{NewGame, load};
use nova_data::wire::id::StellarId;
use nova_galaxy_format::id::Id;
use nova_render::camera::shade;
use nova_render::fonts::FontFaces;
use nova_render::livery::paint_color;
use nova_render::model_source::vertex_color;
use nova_render::{
    GalaxyModels, Gpu, Light, LoadedModel, LogicalSize, Mesh, ModelRequest, ModelSource,
    RenderReport, Renderer, Upscaler, Viewport, WorldAssets, station_livery, world_assets,
};
use nova_view::flight::StellarModel;
use nova_view::{DrawList, ModelKey, ModelPose, Orientation, Point};

use super::wgpu_offscreen::{Images, gpu_with};

const LOGICAL: LogicalSize = LogicalSize { w: 160, h: 64 };
const WIDTH: u32 = 320;
const HEIGHT: u32 = 128;
/// Logical units a model unit: the 6 m hub spans 36.
const SCALE: f32 = 6.0;
const EXCHANGE_AT: (f32, f32) = (40.0, 32.0);
const YARD_AT: (f32, f32) = (120.0, 32.0);

fn id(text: &str) -> Id {
    Id::try_from(text.to_owned()).expect("an id")
}

fn built() -> (NewGame, WorldAssets) {
    let source = liveried_stations_3d();
    let game = load(&source, Path::new("sample"));
    let assets = world_assets(&game, &source);
    assert_eq!(assets.problems, Vec::<String>::new());
    (game, assets)
}

fn station_key(game: &NewGame, assets: &WorldAssets, name: &str) -> ModelKey {
    let at = StellarId(game.ids.stellars[&id(name)]);
    match assets.models.stellar(at) {
        Some(StellarModel::Model { key, .. }) => key,
        other => panic!("{name}: {other:?}"),
    }
}

fn mesh(assets: &WorldAssets, key: ModelKey) -> &Mesh {
    &assets
        .meshes
        .iter()
        .find(|(k, _)| *k == key)
        .expect("registered")
        .1
}

/// The colour `color` shows on a face facing the camera, lit as the core
/// shades it.
fn lit(color: [f32; 4]) -> [u8; 3] {
    let [r, g, b, _] = shade([0.0, 0.0, 1.0], color, Light::STANDARD);
    [r, g, b].map(|c| (c * 255.0).round() as u8)
}

/// The highest vertex facing the camera in `vertices`.
fn top<'a>(vertices: impl Iterator<Item = &'a nova_render::Vertex>) -> &'a nova_render::Vertex {
    vertices
        .filter(|v| v.normal[2] > 0.99)
        .max_by(|a, b| a.position[2].total_cmp(&b.position[2]))
        .expect("a face up")
}

/// The colour `mesh`'s top face shows from above.
fn top_colour(mesh: &Mesh) -> [u8; 3] {
    lit(top(mesh.vertices().iter()).color)
}

/// The material index `model`'s top face is drawn in.
#[allow(clippy::float_cmp, reason = "the same vertex's own height")]
fn top_material(model: &LoadedModel) -> usize {
    let height = top(model.parts.iter().flat_map(|p| p.mesh.vertices())).position[2];
    model
        .parts
        .iter()
        .find(|p| {
            p.mesh
                .vertices()
                .iter()
                .any(|v| v.normal[2] > 0.99 && v.position[2] == height)
        })
        .and_then(|p| p.material)
        .expect("the top is in a material")
}

fn color_at(pixels: &[u8], (x, y): (f32, f32)) -> [u8; 3] {
    let (px, py) = ((x * 2.0) as u32, (y * 2.0) as u32);
    let at = ((py * WIDTH + px) * 4) as usize;
    [pixels[at], pixels[at + 1], pixels[at + 2]]
}

fn near(a: [u8; 3], b: [u8; 3]) -> bool {
    a.iter().zip(&b).all(|(p, q)| p.abs_diff(*q) <= 2)
}

#[test]
fn the_exchange_s_top_shows_its_owner_s_livery_and_the_yard_s_the_kit_s_own_colour() {
    let source = liveried_stations_3d();
    let (game, assets) = built();
    let exchange = station_key(&game, &assets, "luyten-exchange");
    let yard = station_key(&game, &assets, "marrow-yard");
    assert_ne!(exchange, yard);

    // What each top should show, from the hub as loaded and the livery.
    let hub = GalaxyModels::new(&game.galaxy, &source)
        .model(&ModelRequest::new("station-hub", None))
        .expect("the hub loads");
    let material = &hub.materials[top_material(&hub)];
    let slot = material.paint.expect("the hub's top takes paint");
    let livery = station_livery(&game.galaxy, &id("luyten-exchange")).expect("a livery");
    let painted = lit(vertex_color(paint_color(
        livery.color(slot),
        material.base_color[3],
    )));
    let own = lit(vertex_color(material.base_color));
    assert_eq!(top_colour(mesh(&assets, exchange)), painted);
    assert_eq!(top_colour(mesh(&assets, yard)), own);
    assert!(!near(painted, own), "the livery shows against the kit");

    let Some(mut gpu) = gpu_with(WIDTH, HEIGHT, &FontFaces::bundled()) else {
        return;
    };
    gpu.set_upscaler(Upscaler::Native);
    let mut renderer = Renderer::new(Images);
    for (key, mesh) in &assets.meshes {
        renderer.add_model(*key, mesh.clone());
    }
    let pose = |(x, y): (f32, f32)| ModelPose {
        center: Point::new(x, y),
        orientation: Orientation::IDENTITY,
        scale: SCALE,
    };
    let mut list = DrawList::new();
    list.world(|list| {
        list.model(exchange, pose(EXCHANGE_AT))
            .model(yard, pose(YARD_AT));
    });
    let viewport = Viewport::new(LOGICAL, (WIDTH, HEIGHT), 2.0);
    let report = renderer.render(&list, &viewport, &mut gpu);
    assert_eq!(report, RenderReport::default(), "no key missing");
    let pixels = gpu.read_pixels().expect("read back");

    let shown = color_at(&pixels, EXCHANGE_AT);
    assert!(
        near(shown, painted),
        "the exchange: {shown:?}, not {painted:?}"
    );
    let shown = color_at(&pixels, YARD_AT);
    assert!(near(shown, own), "the yard: {shown:?}, not {own:?}");
}
