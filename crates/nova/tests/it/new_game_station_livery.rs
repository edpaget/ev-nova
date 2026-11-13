//! A new-game station drawn in its owner's livery
//! (rdm:phase/rendering-3d/phase-9-stations-in-owner-livery): over the
//! liveried-stations sample
//! (`nova_data::new_game::fixture::liveried_stations_3d`), wired as `main`
//! wires it (the world models' table on the router, their meshes on the
//! app, as `nova_render::world_assets` builds them), with a second,
//! unowned hub station put in Luyten Crossing here: flying there, the
//! world layer draws Luyten Exchange under the key the table gives it,
//! which names the hub in the commonwealth's livery, and the unowned
//! station under its own, which names the hub unpainted; both keys'
//! meshes are uploaded; and landed at the exchange, the spaceport's
//! landscape draws the exchange's key.
//!
//! Wiring only: which mesh a station's key names, and the colours it
//! shows, are nova-render's (`new_game_world`'s tests and the offscreen
//! `station_livery` test); that each screen draws a stellar's own key is
//! nova-view's. This pins that the data, the edge, the app and the screens
//! meet.

use std::path::Path;

use nova::ruleset::Ruleset;
use nova_data::new_game::{self, NewGame};
use nova_render::recording::GpuCall;
use nova_render::{Mesh, WorldAssets, world_assets};
use nova_sim::StellarId;
use nova_sim::fixture::MemoryPilots;
use nova_view::flight::StellarModel;
use nova_view::spaceport::layout::LANDSCAPE_ITEM;
use nova_view::{DrawCommand, DrawList, ModelKey, Point, Screen};

use crate::common::draw;
use crate::new_game_models::{flying_over, world};
use crate::new_game_portraits::landed_at_the_exchange;

/// The stellar [`sample`] adds to Luyten Crossing: a second station
/// drawing the hub, owned by nobody, so it wears no livery.
const DRIFTING_HUB: &str = "drifting-hub";

/// The liveried-stations sample with [`DRIFTING_HUB`] put in Luyten
/// Crossing beside the exchange, loaded, and its world models. So one
/// system holds the hub twice, once in the commonwealth's livery and once
/// unpainted.
fn sample() -> (NewGame, WorldAssets) {
    let source = new_game::fixture::liveried_stations_3d().with(
        &format!("stellars/{DRIFTING_HUB}.md"),
        "---\nname: Drifting Hub\nkind: station\nsystem: luyten-crossing\n\
         position: [120, 0]\nvisual: {asset: station-hub}\n---\n",
    );
    let game = new_game::load(&source, Path::new("sample"));
    let assets = world_assets(&game, &source);
    assert_eq!(assets.problems, Vec::<String>::new());
    (game, assets)
}

/// The model key the table gives the stellar `name`.
fn station_key(game: &NewGame, assets: &WorldAssets, name: &str) -> ModelKey {
    let (_, &nova) = game
        .ids
        .stellars
        .iter()
        .find(|(id, _)| id.to_string() == name)
        .expect("in the sample");
    match assets.models.stellar(StellarId(nova)) {
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

/// The exchange's key and the drifting hub's, checked to name different
/// meshes. Both come from the one table `assets` holds: a key means
/// nothing outside the table that handed it out.
fn keys(game: &NewGame, assets: &WorldAssets) -> (ModelKey, ModelKey) {
    let exchange = station_key(game, assets, "luyten-exchange");
    let drifting = station_key(game, assets, DRIFTING_HUB);
    assert_ne!(exchange, drifting);
    assert_ne!(
        mesh(assets, exchange),
        mesh(assets, drifting),
        "the exchange's hub is painted, the drifting one's not"
    );
    (exchange, drifting)
}

#[test]
fn flying_in_the_crossing_draws_each_hub_at_its_own_key_and_uploads_both() {
    let (game, assets) = sample();
    let (exchange, drifting) = keys(&game, &assets);
    let data = std::rc::Rc::new(game.data);
    let pilots = MemoryPilots::new();
    let mut harness = flying_over(Ruleset::NewGame, &data, &assets, &pilots);
    draw(&mut harness);
    let drawn: Vec<ModelKey> = world(&harness)
        .iter()
        .filter_map(|command| match command {
            DrawCommand::Model { model, .. } => Some(*model),
            _ => None,
        })
        .collect();
    // Two stations on one model, side by side: each draws under its own
    // key, the exchange's painted and the drifting hub's not.
    let hubs = |key| drawn.iter().filter(|drawn| **drawn == key).count();
    assert_eq!(hubs(exchange), 1, "the exchange, painted: {drawn:?}");
    assert_eq!(hubs(drifting), 1, "the drifting hub, unpainted: {drawn:?}");
    for (key, what) in [(exchange, "painted"), (drifting, "unpainted")] {
        let mesh = mesh(&assets, key);
        assert!(
            harness.gpu_calls().iter().any(|call| matches!(
                call,
                GpuCall::UploadMesh { data, .. } if data == mesh
            )),
            "the {what} hub uploaded"
        );
    }
}

#[test]
fn landed_at_the_exchange_the_landscape_draws_its_painted_key() {
    let (game, assets) = sample();
    let (exchange, _) = keys(&game, &assets);
    let mut harness = landed_at_the_exchange(game, &assets);
    draw(&mut harness);
    let port = harness.app.screen().spaceport_view().expect("landed");
    let area = port
        .dialog()
        .expect("laid out")
        .item_bounds(LANDSCAPE_ITEM)
        .expect("the landscape item");
    let mut list = DrawList::new();
    port.draw(&mut list);
    let drawn: Vec<(ModelKey, Point)> = list
        .iter()
        .filter_map(|command| match command {
            DrawCommand::Model { model, pose, .. } => Some((*model, pose.center)),
            _ => None,
        })
        .collect();
    assert_eq!(drawn, [(exchange, area.center())], "the painted hub");
}
