//! The new game flies in 3D: over the 3D sample
//! (`nova_data::new_game::fixture::modelled_sample_3d`), wired as `main`
//! wires it (the rulebook `ruleset_rulebook` gives `Ruleset::NewGame`, the
//! world models' table on the router and their meshes and planets on the
//! app, as `nova_render::world_assets` builds them), the flight's world
//! layer draws the player's ship and the traffic as their classes'
//! models, Luyten Exchange as the station hub and Kestrel Reach as a
//! planet, each key it draws registered and uploaded; every ship in the
//! frame draws its class's hull, the traffic's included; and a pilot
//! carrying the gun mount draws its turret on its hull, facing the nose
//! until its first turret shot and then as the view's `mounted_outfits`
//! turns it for the frame's turret facing. Flown under the stock ruleset's
//! readings over the same data, the player's hull, the station hub and
//! the planet still draw as their models: the data picks a model, not a
//! rulebook key.
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
use nova::app::{Control, WindowEvent};
use nova::chance::SplitMix;
use nova::rulebook::ruleset_rulebook;
use nova::ruleset::Ruleset;
use nova_audio::SettingsStore;
use nova_audio::recording::MemorySettings;
use nova_data::GameData;
use nova_data::new_game::{self, NewGame};
use nova_flight::SessionViewExt as _;
use nova_render::recording::GpuCall;
use nova_render::{WorldAssets, world_assets};
use nova_sim::fixture::{MemoryPilots, place, put_player_at};
use nova_sim::flight_frame::FlightFrame;
use nova_sim::{OutfitId, SetExpr, ShipId, StellarId, TargetPick, Vec2};
use nova_view::flight::{StellarModel, mounted_outfits};
use nova_view::{DrawCommand, DrawLayer, DrawList, Key, ModelKey, Screen};

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
    flying_under(Ruleset::NewGame, game, assets, pilots)
}

/// The app flying the sample's start under `ruleset`'s readings, wired
/// with `assets` as `main` wires them.
fn flying_under(
    ruleset: Ruleset,
    game: NewGame,
    assets: &WorldAssets,
    pilots: &MemoryPilots,
) -> Harness {
    flying_over(ruleset, &Rc::new(game.data), assets, pilots)
}

/// The app flying the start of the new game `data` under `ruleset`'s
/// readings, wired with its world models `assets` as `main` wires them.
pub(crate) fn flying_over(
    ruleset: Ruleset,
    data: &Rc<GameData>,
    assets: &WorldAssets,
    pilots: &MemoryPilots,
) -> Harness {
    let rules = ruleset_rulebook(None, ruleset.profile().readings).0;
    let (keeper, warning) = nova::settings::SettingsKeeper::open(
        Box::new(MemorySettings::new()) as Box<dyn SettingsStore>
    );
    assert_eq!(warning, None);
    let table = Rc::new(assets.models.clone());
    fly_new_pilot_modelled(
        data,
        pilots,
        keeper,
        |screen: AppScreen| screen.with_rulebook(&rules).with_world_models(table),
        assets.clone(),
    )
}

/// The world layer of the view's frame drawn now.
pub(crate) fn world(game: &Harness) -> Vec<DrawCommand> {
    let view = game.app.screen().flight_view().expect("flying");
    let mut list = DrawList::new();
    view.draw(&mut list);
    list.layered()
        .filter(|(layer, _)| *layer == DrawLayer::World)
        .map(|(_, command)| command.clone())
        .collect()
}

/// The session's frame now.
pub(crate) fn frame(game: &Harness) -> FlightFrame {
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
fn the_data_not_the_ruleset_picks_the_models() {
    let (game, assets) = sample();
    let exchange = StellarId(nova_id(&game.ids.stellars, "luyten-exchange"));
    let reach = StellarId(nova_id(&game.ids.stellars, "kestrel-reach"));
    let pilots = MemoryPilots::new();
    let mut harness = flying_under(Ruleset::Stock, game, &assets, &pilots);
    draw(&mut harness);
    let world = world(&harness);
    let player = assets
        .models
        .ship(frame(&harness).ships[0].class)
        .expect("a model")
        .key;
    let drawn = hulls(&world);
    assert!(drawn.contains(&player), "the player's hull: {drawn:?}");
    let Some(StellarModel::Model { key: hub, .. }) = assets.models.stellar(exchange) else {
        panic!("the exchange's model");
    };
    assert!(drawn.contains(&hub), "the station hub: {drawn:?}");
    let Some(StellarModel::Planet { key: giant }) = assets.models.stellar(reach) else {
        panic!("Kestrel Reach's planet");
    };
    assert!(
        world
            .iter()
            .any(|c| matches!(c, DrawCommand::Planet { planet, .. } if *planet == giant)),
        "Kestrel Reach"
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

/// The commands the view's [`mounted_outfits`] gives the player's ship
/// as `frame` has it, at the pose `world` draws its hull `hull` at: what
/// the drawn hull and its turret should be.
fn player_as_mounted(
    harness: &Harness,
    assets: &WorldAssets,
    world: &[DrawCommand],
    hull: ModelKey,
) -> Vec<DrawCommand> {
    let view = harness.app.screen().flight_view().expect("flying");
    let frame = frame(harness);
    let player = &frame.ships[0];
    let pose = world
        .iter()
        .rev()
        .find_map(|command| match command {
            DrawCommand::Model { model, pose } if *model == hull => Some(*pose),
            _ => None,
        })
        .expect("the player's hull");
    let angle = player
        .frames
        .hull_angle(view.shown_heading(), frame.hull_turn);
    let mounts = assets.models.mounts(player.class, &frame.player_kits);
    let facings = vec![player.turret_facing; mounts.len()];
    mounted_outfits(hull, pose, angle, &mounts, &facings, |asset| {
        assets.models.outfit(asset)
    })
}

/// The turn of the turret the player's hull `hull` draws, the last
/// drawn of it in `world`.
fn turret_turn(world: &[DrawCommand], hull: ModelKey, turret: ModelKey) -> f32 {
    world
        .windows(2)
        .rev()
        .find_map(|pair| match pair {
            [
                DrawCommand::Model { model, .. },
                DrawCommand::Mounted {
                    model: part,
                    mounting,
                },
            ] if *model == hull && *part == turret => Some(mounting.turn),
            _ => None,
        })
        .expect("the player's turret")
}

#[test]
fn the_player_s_turret_turns_to_its_first_shot_s_aim() {
    let (game, assets) = sample();
    let gun_mount = nova_id(&game.ids.kits, "gun-mount");
    let mass_driver = nova_id(&game.ids.outfits, "mass-driver");
    let freighter = assets
        .models
        .ship(ShipId(nova_id(&game.ids.ships, "light-freighter")))
        .expect("a model")
        .key;
    let turret = assets.models.outfit("turret-mount").expect("the turret");
    // The same data again, to fill the system from.
    let data = sample().0.data;
    let pilots = MemoryPilots::new();
    let mut harness = flying(game, &assets, &pilots);
    let target = harness
        .app
        .screen_mut()
        .flight_view_mut()
        .expect("flying")
        .change_sim(|sim| {
            let session = sim.session_mut().expect("a session");
            for outfit in [gun_mount, mass_driver] {
                session.run_set(
                    &SetExpr::parse(&format!("G{outfit}")).expect("parses"),
                    &mut nova_sim::NeverFires,
                );
            }
            (0..200)
                .find_map(|seed| {
                    if session.npcs().is_empty() {
                        session.populate(&data, &mut SplitMix::new(seed));
                    }
                    session.npcs().first().map(|npc| npc.id)
                })
                .expect("a ship to aim at")
        });
    // The target 100 ahead and 60 to the right, off the nose.
    let stand = |harness: &mut Harness| {
        harness
            .app
            .screen_mut()
            .flight_view_mut()
            .expect("flying")
            .change_sim(|sim| {
                let session = sim.session_mut().expect("a session");
                put_player_at(session, Vec2::ZERO);
                place(session, target, Vec2::new(60.0, -100.0));
                assert_eq!(session.select_target(TargetPick::Nearest), Some(target));
            });
    };
    stand(&mut harness);
    draw(&mut harness);
    assert_eq!(frame(&harness).ships[0].turret_facing, None);
    let before = world(&harness);
    let nose = turret_turn(&before, freighter, turret);
    assert!(nose.abs() < f32::EPSILON, "the nose: {nose}");

    // Space pressed again each frame until it takes, as the flight may
    // not step its first frames.
    for _ in 0..2000 {
        stand(&mut harness);
        hold_fire(&mut harness, true);
        draw(&mut harness);
        if frame(&harness).ships[0].turret_facing.is_some() {
            break;
        }
    }
    hold_fire(&mut harness, false);
    assert!(
        frame(&harness).ships[0].turret_facing.is_some(),
        "the player fired its turret"
    );
    let after = world(&harness);
    let turn = turret_turn(&after, freighter, turret);
    assert!(turn.abs() > 1e-3, "turned off the nose: {turn}");
    let want = player_as_mounted(&harness, &assets, &after, freighter);
    assert!(
        after
            .windows(want.len())
            .any(|drawn| drawn == want.as_slice()),
        "the hull and its turret as the view's function draws them: {want:?} in {after:?}"
    );
}

#[test]
fn every_ship_in_the_frame_draws_its_class_s_model() {
    let (game, assets) = sample();
    // The same data again, to fill the system from.
    let data = sample().0.data;
    let pilots = MemoryPilots::new();
    let mut harness = flying(game, &assets, &pilots);
    harness
        .app
        .screen_mut()
        .flight_view_mut()
        .expect("flying")
        .change_sim(|sim| {
            let session = sim.session_mut().expect("a session");
            for seed in 0..200 {
                if !session.npcs().is_empty() {
                    break;
                }
                session.populate(&data, &mut SplitMix::new(seed));
            }
            // Each on screen, in a row ahead of the player.
            put_player_at(session, Vec2::ZERO);
            let ids: Vec<_> = session.npcs().iter().map(|npc| npc.id).collect();
            for (i, id) in ids.into_iter().enumerate() {
                #[allow(clippy::cast_precision_loss)]
                let x = 60.0 * i as f32 - 120.0;
                place(session, id, Vec2::new(x, -120.0));
            }
        });
    draw(&mut harness);
    let frame = frame(&harness);
    assert!(frame.ships.len() > 1, "the player and traffic");
    let stellar_models: Vec<ModelKey> = assets
        .models
        .stellars
        .values()
        .filter_map(|model| match model {
            StellarModel::Model { key, .. } => Some(*key),
            StellarModel::Planet { .. } => None,
        })
        .collect();
    let mut drawn: Vec<u32> = hulls(&world(&harness))
        .into_iter()
        .filter(|key| !stellar_models.contains(key))
        .map(|key| key.0)
        .collect();
    let mut want: Vec<u32> = frame
        .ships
        .iter()
        .filter_map(|ship| assets.models.ship(ship.class))
        .map(|model| model.key.0)
        .collect();
    drawn.sort_unstable();
    want.sort_unstable();
    assert!(want.len() > 1, "keyed traffic: {want:?}");
    assert_eq!(drawn, want, "one hull each, its class's");
}

/// Presses (`held`) or lets go of the fire key.
fn hold_fire(harness: &mut Harness, held: bool) {
    let event = WindowEvent::Key {
        key: Key::Space,
        pressed: held,
        repeat: false,
    };
    assert_eq!(harness.handle(event), Control::Continue);
}
