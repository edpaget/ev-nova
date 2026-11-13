//! The new game's world models built at the edge and drawn offscreen
//! (rdm:phase/engine-3d/phase-10-sample-in-3d): [`world_assets`] over the
//! 3D sample (`nova_data::new_game::fixture::modelled_sample_3d`), its
//! meshes and planets registered with a renderer, and the player's light
//! freighter carrying the gun mount (its hull and mounts drawn through the
//! view's `mounted_outfits` with the table's keys) and Kestrel Reach drawn
//! with the keys the table hands out. So the keys the table names are the
//! ones registered with real meshes and planets.
//!
//! Every region sampled is worked out from the table and the poses: the
//! hull's centre, the turret's mount, the planet's centre. A second test
//! runs the builder over the full sample when `GAME_DATA` names it, and
//! skips, passing, otherwise.
//!
//! Needs a GPU adapter for the first; skips, passing, without one.

use std::path::Path;

use nova_data::new_game::fixture::modelled_sample_3d;
use nova_data::new_game::{NewGame, load};
use nova_data::wire::id::{OutfitId, ShipId, StellarId};
use nova_galaxy_format::id::Id;
use nova_galaxy_format::model::Visual;
use nova_galaxy_format::source::DirSource;
use nova_render::camera::shade;
use nova_render::fonts::FontFaces;
use nova_render::{
    Gpu, Light, LogicalSize, Mesh, RenderReport, Renderer, Upscaler, Viewport, WorldAssets,
    world_assets,
};
use nova_view::flight::{StellarModel, mounted_outfits};
use nova_view::{DrawList, ModelPose, Orientation, PlanetPose, Point, view_axes};

use super::common::game_data;
use super::wgpu_offscreen::{Images, gpu_with};

const LOGICAL: LogicalSize = LogicalSize { w: 320, h: 96 };
const WIDTH: u32 = 640;
const HEIGHT: u32 = 192;
/// Logical units a metre, so the turret shows.
const SCALE: f32 = 8.0;
const HULL_AT: (f32, f32) = (80.0, 48.0);
const PLANET_AT: (f32, f32) = (260.0, 48.0);
const PLANET_RADIUS: f32 = 30.0;

fn id(text: &str) -> Id {
    Id::try_from(text.to_owned()).expect("an id")
}

fn built() -> (NewGame, WorldAssets) {
    let source = modelled_sample_3d();
    let game = load(&source, Path::new("sample"));
    let assets = world_assets(&game, &source);
    assert_eq!(assets.problems, Vec::<String>::new());
    (game, assets)
}

fn mesh(assets: &WorldAssets, key: nova_view::ModelKey) -> &Mesh {
    &assets
        .meshes
        .iter()
        .find(|(k, _)| *k == key)
        .expect("registered")
        .1
}

/// The colour `mesh`'s top face shows from above, lit as the core shades
/// a face facing the camera.
fn top_colour(mesh: &Mesh) -> [u8; 3] {
    let top = mesh
        .vertices()
        .iter()
        .filter(|v| v.normal[2] > 0.99)
        .max_by(|a, b| a.position[2].total_cmp(&b.position[2]))
        .expect("a face up");
    let [r, g, b, _] = shade([0.0, 0.0, 1.0], top.color, Light::STANDARD);
    [r, g, b].map(|c| (c * 255.0).round() as u8)
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
fn the_sample_s_ship_turret_and_planet_draw_from_the_keys_the_table_hands_out() {
    let (game, assets) = built();
    let freighter = ShipId(game.ids.ships[&id("light-freighter")]);
    let gun_mount = OutfitId(game.ids.kits[&id("gun-mount")]);
    let hull = assets
        .models
        .ship(freighter)
        .expect("the freighter's model");
    let mounts = assets.models.mounts(freighter, &[gun_mount]);
    let pose = ModelPose {
        center: Point::new(HULL_AT.0, HULL_AT.1),
        orientation: Orientation::IDENTITY,
        scale: SCALE,
    };
    let commands = mounted_outfits(hull.key, pose, 0.0, &mounts, &[], |asset| {
        assets.models.outfit(asset)
    });
    assert_eq!(commands.len(), 2, "the hull and its turret: {commands:?}");
    let reach = StellarId(game.ids.stellars[&id("kestrel-reach")]);
    let Some(StellarModel::Planet { key: planet, .. }) = assets.models.stellar(reach) else {
        panic!("Kestrel Reach's planet");
    };
    let turret = assets
        .models
        .outfit("turret-mount")
        .expect("the turret's key");
    let (deck, red) = (
        top_colour(mesh(&assets, hull.key)),
        top_colour(mesh(&assets, turret)),
    );
    assert!(!near(deck, red), "the turret shows against the deck");

    let Some(mut gpu) = gpu_with(WIDTH, HEIGHT, &FontFaces::bundled()) else {
        return;
    };
    gpu.set_upscaler(Upscaler::Native);
    let mut renderer = Renderer::new(Images);
    for (key, mesh) in &assets.meshes {
        renderer.add_model(*key, mesh.clone());
    }
    for (key, loaded) in &assets.planets {
        renderer.add_planet(*key, loaded.clone());
    }
    let mut list = DrawList::new();
    list.world(|list| {
        for command in commands {
            list.push(command);
        }
        list.planet(
            planet,
            PlanetPose {
                center: Point::new(PLANET_AT.0, PLANET_AT.1),
                radius: PLANET_RADIUS,
                time: 0.0,
            },
        );
    });
    let viewport = Viewport::new(LOGICAL, (WIDTH, HEIGHT), 2.0);
    let report = renderer.render(&list, &viewport, &mut gpu);
    assert_eq!(report, RenderReport::default(), "no key missing");
    let pixels = gpu.read_pixels().expect("read back");

    let hull_centre = color_at(&pixels, HULL_AT);
    assert!(
        near(hull_centre, deck),
        "the deck: {hull_centre:?}, not {deck:?}"
    );
    let [x, y, _] = mounts
        .iter()
        .find(|mount| mount.shows.is_some())
        .expect("the gun bed")
        .at;
    #[allow(clippy::cast_possible_truncation)]
    let [mx, my, _] = view_axes([x as f32, y as f32, 0.0]);
    let on_turret = (HULL_AT.0 + SCALE * mx, HULL_AT.1 + SCALE * my);
    let turret_shown = color_at(&pixels, on_turret);
    assert!(
        near(turret_shown, red),
        "the turret: {turret_shown:?}, not {red:?}"
    );
    let background = color_at(&pixels, (318.0, 2.0));
    let globe = color_at(&pixels, PLANET_AT);
    assert_ne!(globe, background, "the planet shows");
}

#[test]
fn the_full_sample_s_models_and_planets_are_all_registered() {
    let Some(dir) = game_data() else { return };
    let source = DirSource::new(&dir);
    let game = load(&source, &dir);
    let assets = world_assets(&game, &source);
    assert_eq!(assets.problems, Vec::<String>::new());
    if !game.lfs_pointers.is_empty() {
        eprintln!(
            "skipping the models: {} LFS pointers",
            game.lfs_pointers.len()
        );
        return;
    }
    // Every ship on a model that isn't a placeholder has its hull.
    for (ship, &nova) in &game.ids.ships {
        let modelled = nova_render::ship_request(&game.galaxy, ship).is_some_and(|request| {
            game.galaxy
                .assets
                .get(&id(&request.asset))
                .is_some_and(|asset| !asset.data.placeholder)
        });
        if modelled {
            assert!(assets.models.ship(ShipId(nova)).is_some(), "{ship}: a hull");
        }
    }
    assert!(assets.models.outfit("turret-mount").is_some(), "the turret");
    // Every station on the hub, and every generated stellar, has its own.
    for (stellar, &nova) in &game.ids.stellars {
        let at = assets.models.stellar(StellarId(nova));
        match &game.galaxy.stellars[stellar].data.visual {
            Some(Visual::Asset(asset)) if asset.as_str() == "station-hub" => {
                assert!(
                    matches!(at, Some(StellarModel::Model { .. })),
                    "{stellar}: the hub"
                );
            }
            Some(Visual::Generated(_)) => {
                assert!(
                    matches!(at, Some(StellarModel::Planet { .. })),
                    "{stellar}: a planet"
                );
            }
            _ => {}
        }
    }
}
