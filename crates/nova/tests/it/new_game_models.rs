//! The new game flies in 3D: over the 3D sample
//! (`nova_data::new_game::fixture::modelled_sample_3d`), wired as `main`
//! wires it (the rulebook `ruleset_rulebook` gives `Ruleset::NewGame`, the
//! world models' table on the router and their meshes and planets on the
//! app, as `nova_render::world_assets` builds them), the flight's world
//! layer draws the player's ship and the traffic as their classes'
//! models, Luyten Exchange as the station hub and Kestrel Reach as a
//! planet, each key it draws registered and uploaded; and a pilot carrying
//! the gun mount draws its turret on its hull.
//!
//! Wiring only: which ship, station or planet draws a model, where and
//! turned how, is nova-view's (its flight view's `ship_models` and
//! `stellar_models` modules), the turret's facing nova-sim's
//! (`turret_facings`), the keys nova-render's (`new_game_world`), each
//! pinned there; and the router without a table draws the stand-ins, as
//! `new_game_looks.rs` pins. This pins that the data, the edge, the app
//! and the view meet.

use std::path::Path;
use std::rc::Rc;

use nova::app::screen::AppScreen;
use nova::rulebook::ruleset_rulebook;
use nova::ruleset::Ruleset;
use nova_audio::SettingsStore;
use nova_audio::recording::MemorySettings;
use nova_data::new_game::{self, NewGame};
use nova_render::recording::GpuCall;
use nova_render::{WorldAssets, world_assets};
use nova_sim::fixture::MemoryPilots;
use nova_sim::flight_frame::FlightFrame;
use nova_sim::{OutfitId, SetExpr, ShipId, StellarId};
use nova_view::flight::StellarModel;
use nova_view::{DrawCommand, DrawLayer, DrawList, ModelKey, Screen};

use crate::common::{draw, fly_new_pilot_modelled};
use crate::pilots::Harness;

/// The 3D sample, loaded, and its world models.
fn sample() -> (NewGame, WorldAssets) {
    let source = new_game::fixture::modelled_sample_3d();
    let game = new_game::load(&source, Path::new("sample"));
    let assets = world_assets(&game, &source);
    assert_eq!(assets.problems, Vec::<String>::new());
    (game, assets)
}

/// The Nova ID `table` gives the bible id `name`.
fn nova_id<K: std::fmt::Display>(table: &std::collections::BTreeMap<K, i16>, name: &str) -> i16 {
    table
        .iter()
        .find(|(id, _)| id.to_string() == name)
        .map(|(_, &nova)| nova)
        .expect("in the sample")
}

/// The app flying the sample's start under the new game's readings, wired
/// with `assets` as `main` wires them.
fn flying(game: NewGame, assets: &WorldAssets, pilots: &MemoryPilots) -> Harness {
    let data = Rc::new(game.data);
    let rules = ruleset_rulebook(None, Ruleset::NewGame.profile().readings).0;
    let (keeper, warning) = nova::settings::SettingsKeeper::open(
        Box::new(MemorySettings::new()) as Box<dyn SettingsStore>
    );
    assert_eq!(warning, None);
    let table = Rc::new(assets.models.clone());
    fly_new_pilot_modelled(
        &data,
        pilots,
        keeper,
        |screen: AppScreen| screen.with_rulebook(&rules).with_world_models(table),
        assets.clone(),
    )
}

/// The world layer of the view's frame drawn now.
fn world(game: &Harness) -> Vec<DrawCommand> {
    let view = game.app.screen().flight_view().expect("flying");
    let mut list = DrawList::new();
    view.draw(&mut list);
    list.layered()
        .filter(|(layer, _)| *layer == DrawLayer::World)
        .map(|(_, command)| command.clone())
        .collect()
}

/// The session's frame now.
fn frame(game: &Harness) -> FlightFrame {
    let mut frame = FlightFrame::default();
    game.app
        .screen()
        .flight_view()
        .expect("flying")
        .session()
        .expect("a session")
        .fill_frame(&mut frame);
    frame
}

/// The hull models the world layer draws, in order.
fn hulls(world: &[DrawCommand]) -> Vec<ModelKey> {
    world
        .iter()
        .filter_map(|command| match command {
            DrawCommand::Model { model, .. } => Some(*model),
            _ => None,
        })
        .collect()
}

#[test]
fn the_ships_the_station_and_the_planet_draw_as_their_models() {
    let (game, assets) = sample();
    let exchange = StellarId(nova_id(&game.ids.stellars, "luyten-exchange"));
    let reach = StellarId(nova_id(&game.ids.stellars, "kestrel-reach"));
    let pilots = MemoryPilots::new();
    let mut harness = flying(game, &assets, &pilots);
    draw(&mut harness);
    let world = world(&harness);
    let frame = frame(&harness);
    let class_key = |class: ShipId| assets.models.ship(class).expect("a model").key;
    let drawn = hulls(&world);
    assert!(
        drawn.contains(&class_key(frame.ships[0].class)),
        "the player's hull: {drawn:?}"
    );
    let Some(StellarModel::Model { key: hub, .. }) = assets.models.stellar(exchange) else {
        panic!("the exchange's model");
    };
    assert!(drawn.contains(&hub), "the station hub: {drawn:?}");
    let ship_keys: Vec<ModelKey> = frame
        .ships
        .iter()
        .map(|ship| class_key(ship.class))
        .collect();
    assert!(
        drawn
            .iter()
            .all(|key| *key == hub || ship_keys.contains(key)),
        "each hull its class's: {drawn:?} of {ship_keys:?}"
    );
    let Some(StellarModel::Planet { key: giant }) = assets.models.stellar(reach) else {
        panic!("Kestrel Reach's planet");
    };
    assert!(
        world
            .iter()
            .any(|c| matches!(c, DrawCommand::Planet { planet, .. } if *planet == giant)),
        "Kestrel Reach"
    );

    // Every model drawn was registered, and the frame uploaded its mesh
    // and the planet's textures.
    let uploads: Vec<&nova_render::Mesh> = harness
        .gpu_calls()
        .iter()
        .filter_map(|call| match call {
            GpuCall::UploadMesh { data, .. } => Some(data),
            _ => None,
        })
        .collect();
    let keys = world.iter().filter_map(|command| match command {
        DrawCommand::Model { model, .. } | DrawCommand::Mounted { model, .. } => Some(*model),
        _ => None,
    });
    for key in keys {
        let (_, mesh) = assets
            .meshes
            .iter()
            .find(|(k, _)| *k == key)
            .expect("registered");
        assert!(uploads.contains(&mesh), "{key:?} uploaded");
    }
    assert!(
        harness
            .gpu_calls()
            .iter()
            .any(|call| matches!(call, GpuCall::UploadPlanet { .. })),
        "the planet uploaded"
    );
}

#[test]
fn a_pilot_carrying_the_gun_mount_draws_its_turret_on_its_hull() {
    let (game, assets) = sample();
    let gun_mount = nova_id(&game.ids.kits, "gun-mount");
    let freighter = assets
        .models
        .ship(ShipId(nova_id(&game.ids.ships, "light-freighter")))
        .expect("a model")
        .key;
    let turret = assets.models.outfit("turret-mount").expect("the turret");
    let pilots = MemoryPilots::new();
    let mut harness = flying(game, &assets, &pilots);
    // The player is drawn after the traffic, which may fly freighters too.
    let after_hull = |world: &[DrawCommand]| {
        world
            .windows(2)
            .rev()
            .find_map(|pair| match pair {
                [DrawCommand::Model { model, .. }, next] if *model == freighter => {
                    Some(next.clone())
                }
                _ => None,
            })
            .expect("the player's hull")
    };
    assert!(
        !matches!(after_hull(&world(&harness)), DrawCommand::Mounted { model, .. } if model == turret),
        "no turret before the kit"
    );
    harness
        .app
        .screen_mut()
        .flight_view_mut()
        .expect("flying")
        .change_sim(|sim| {
            let session = sim.session_mut().expect("a session");
            session.run_set(
                &SetExpr::parse(&format!("G{gun_mount}")).expect("parses"),
                &mut nova_sim::NeverFires,
            );
            assert_eq!(session.pilot().outfits.get(&OutfitId(gun_mount)), Some(&1));
        });
    for _ in 0..3 {
        draw(&mut harness);
    }
    assert!(
        matches!(after_hull(&world(&harness)), DrawCommand::Mounted { model, .. } if model == turret),
        "the turret right after the hull"
    );
}
