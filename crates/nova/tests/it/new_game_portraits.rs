//! The new game's ships and stellars drawn as their models outside the
//! flight's world (rdm:phase/rendering-3d/phase-8-models-in-other-screens):
//! over the 3D sample (`nova_data::new_game::fixture::modelled_sample_3d`),
//! wired as `main` wires it (the world models' table on the router, their
//! meshes on the app), the status bar's target panel draws the target's
//! class's model in its picture's place, in the overlay; the comm dialog
//! Y opens on it draws the same class's model in item 11; the planet comm
//! dialog Y opens on Luyten Exchange draws the station hub in item 5; and
//! landed there, the spaceport's landscape draws the hub; and the
//! navigator's system viewer, entered on the crossing from the main
//! menu's galaxy map, draws the hub and Kestrel Reach's planet. The
//! escort dialog's wiring is pinned in `combat.rs`, whose harness flies
//! with an escort. The main menu's pilot panel draws the pilot's class as
//! its model (rdm:phase/rendering-3d/phase-16-pilot-panel-ship-model),
//! for a pilot just created and for one saved and opened at launch, with
//! the table given after the menu is built, as `main` gives it.
//!
//! The 3D sample's stations sell no ships, so no shipyard opens over it:
//! the shipyard's and the hire screen's models are pinned on nova-view's
//! spaceport (its `spaceport::view` tests' `world_models` module), which
//! the app hands the table through the same
//! `SpaceportView::with_world_models` the landscape's test pins here.
//!
//! Wiring only: what each screen draws, where and how big, is
//! nova-view's (its `model_picture` module and each screen's), the table's
//! keys and radii nova-render's (`new_game_world`), each pinned there.
//! This pins that the data, the edge, the app and the screens meet.

use std::path::Path;

use nova::app::Showing;
use nova::app::{Control, WindowEvent};
use nova::chance::SplitMix;
use nova::rulebook::ruleset_rulebook;
use nova::ruleset::Ruleset;
use nova_data::new_game::{self, NewGame};
use nova_render::{WorldAssets, world_assets};
use nova_sim::TargetPick;
use nova_sim::fixture::{MemoryPilots, place, put_player_at};
use nova_sim::{ShipRef, StellarId, Vec2};
use nova_view::flight::StellarModel;
use nova_view::menu::{MenuChoice, PanelPilot};
use nova_view::spaceport::layout::LANDSCAPE_ITEM;
use nova_view::{DrawCommand, DrawLayer, DrawList, Key, ModelKey, Point, Screen};
use winit::keyboard::KeyCode;

use crate::common::draw;
use crate::new_game_models::{flying_over, frame};
use crate::pilots::Harness;

/// The 3D sample, loaded, and its world models.
fn sample() -> (NewGame, WorldAssets) {
    let source = new_game::fixture::modelled_sample_3d();
    let game = new_game::load(&source, Path::new("sample"));
    let assets = world_assets(&game, &source);
    assert_eq!(assets.problems, Vec::<String>::new());
    (game, assets)
}

/// The models the screen drawn now draws in the overlay, in order.
fn overlay_models(game: &Harness) -> Vec<ModelKey> {
    let mut list = DrawList::new();
    game.app.screen().draw(&mut list);
    list.layered()
        .filter_map(|(layer, command)| match command {
            DrawCommand::Model { model, .. } if layer == DrawLayer::Overlay => Some(*model),
            _ => None,
        })
        .collect()
}

/// The app flying the 3D sample, targeting a ship of its traffic 100
/// ahead and 60 to the right; and its class's hull's key.
fn targeting() -> (Harness, ModelKey) {
    let (game, assets) = sample();
    let data = std::rc::Rc::new(game.data);
    let pilots = MemoryPilots::new();
    let mut harness = flying_over(Ruleset::NewGame, &data, &assets, &pilots);
    let target = harness
        .app
        .screen_mut()
        .flight_view_mut()
        .expect("flying")
        .change_sim(|sim| {
            let session = sim.session_mut().expect("a session");
            let npc = (0..200)
                .find_map(|seed| {
                    if session.npcs().is_empty() {
                        session.populate(&data, &mut SplitMix::new(seed));
                    }
                    session.npcs().first().map(|npc| npc.id)
                })
                .expect("a ship to target");
            put_player_at(session, Vec2::ZERO);
            place(session, npc, Vec2::new(60.0, -100.0));
            assert_eq!(session.select_target(TargetPick::Nearest), Some(npc));
            npc
        });
    draw(&mut harness);
    let class = frame(&harness)
        .ship(ShipRef::Npc(target))
        .expect("the target")
        .class;
    let key = assets.models.ship(class).expect("the class's model").key;
    (harness, key)
}

#[test]
fn the_target_panel_draws_the_target_s_class_as_its_model() {
    let (harness, key) = targeting();
    let drawn = overlay_models(&harness);
    assert!(drawn.contains(&key), "the target's portrait: {drawn:?}");
}

/// Presses and releases `key`.
fn tap(harness: &mut Harness, key: Key) {
    for pressed in [true, false] {
        let event = WindowEvent::Key {
            key,
            pressed,
            repeat: false,
        };
        assert_eq!(harness.handle(event), Control::Continue);
    }
}

/// The models `screen` draws, in order.
fn models_of(screen: &impl Screen) -> Vec<ModelKey> {
    let mut list = DrawList::new();
    screen.draw(&mut list);
    list.iter()
        .filter_map(|command| match command {
            DrawCommand::Model { model, .. } => Some(*model),
            _ => None,
        })
        .collect()
}

#[test]
fn the_comm_dialog_draws_the_hailed_ship_s_class_as_its_model() {
    let (mut harness, key) = targeting();
    // Y hails the target, pressed again each frame until it takes, as the
    // flight may not step its first frames.
    for _ in 0..2000 {
        if harness.app.screen().comm().is_some() {
            break;
        }
        tap(&mut harness, Key::KeyY);
        draw(&mut harness);
    }
    let comm = harness.app.screen().comm().expect("hailed");
    assert_eq!(models_of(comm), [key], "the hailed ship's portrait");
}

#[test]
fn the_planet_comm_dialog_draws_the_hailed_station_s_model() {
    let (game, assets) = sample();
    let exchange = StellarId(nova_id(&game.ids.stellars, "luyten-exchange"));
    let Some(StellarModel::Model { key: hub, .. }) = assets.models.stellar(exchange) else {
        panic!("the exchange's model");
    };
    let data = std::rc::Rc::new(game.data);
    let pilots = MemoryPilots::new();
    let mut harness = flying_over(Ruleset::NewGame, &data, &assets, &pilots);
    harness
        .app
        .screen_mut()
        .flight_view_mut()
        .expect("flying")
        .change_sim(|sim| {
            let session = sim.session_mut().expect("a session");
            put_player_at(session, Vec2::ZERO);
        });
    // 1 selects the crossing's first stellar, the exchange, and Y hails
    // it; pressed again each frame until it takes.
    for _ in 0..2000 {
        if harness.app.screen().planet_comm().is_some() {
            break;
        }
        tap(&mut harness, Key::Digit1);
        tap(&mut harness, Key::KeyY);
        draw(&mut harness);
    }
    let dialog = harness.app.screen().planet_comm().expect("hailed");
    assert_eq!(models_of(dialog), [hub], "the hub in item 5");
}

#[test]
fn the_system_viewer_draws_the_start_system_s_station_and_planet() {
    let (game, assets) = sample();
    let exchange = StellarId(nova_id(&game.ids.stellars, "luyten-exchange"));
    let reach = StellarId(nova_id(&game.ids.stellars, "kestrel-reach"));
    let Some(StellarModel::Model { key: hub, .. }) = assets.models.stellar(exchange) else {
        panic!("the exchange's model");
    };
    let Some(StellarModel::Planet { key: giant, .. }) = assets.models.stellar(reach) else {
        panic!("Kestrel Reach's planet");
    };
    let crossing = nova_sim::SystemId(nova_id(&game.ids.systems, "luyten-crossing"));
    let data = std::rc::Rc::new(game.data);
    let pilots = MemoryPilots::new();
    let table = std::rc::Rc::new(assets.models.clone());
    let mut harness = Harness::built_wired(data, &pilots, |screen| screen.with_world_models(table));
    // Tab steps from the main menu through the developer viewers to the
    // galaxy map; a click selects the crossing, and Return enters it.
    for _ in 0..4 {
        if harness.showing() == Showing::GalaxyMap {
            break;
        }
        tap(&mut harness, Key::Tab);
    }
    assert_eq!(harness.showing(), Showing::GalaxyMap);
    let map = harness.app.screen().galaxy_map();
    let system = map.model().system(crossing).expect("the crossing");
    let at = map.transform().world_to_screen(system.position());
    harness.click(at);
    tap(&mut harness, Key::Enter);
    assert_eq!(harness.showing(), Showing::System);
    let view = harness.app.screen().system_view().expect("open");
    let mut list = DrawList::new();
    view.draw(&mut list);
    assert!(
        list.iter()
            .any(|command| matches!(command, DrawCommand::Model { model, .. } if *model == hub)),
        "the hub"
    );
    assert!(
        list.iter().any(
            |command| matches!(command, DrawCommand::Planet { planet, .. } if *planet == giant)
        ),
        "Kestrel Reach"
    );
}

/// The Nova ID `table` gives the bible id `name`.
fn nova_id<K: std::fmt::Display>(table: &std::collections::BTreeMap<K, i16>, name: &str) -> i16 {
    table
        .iter()
        .find(|(id, _)| id.to_string() == name)
        .map(|(_, &nova)| nova)
        .expect("in the sample")
}

/// The app flying the 3D sample, landed at Luyten Exchange, at the
/// crossing's centre; and the hub's key.
fn at_the_exchange() -> (Harness, ModelKey) {
    let (game, assets) = sample();
    let exchange = StellarId(nova_id(&game.ids.stellars, "luyten-exchange"));
    let Some(StellarModel::Model { key: hub, .. }) = assets.models.stellar(exchange) else {
        panic!("the exchange's model");
    };
    (landed_at_the_exchange(game, &assets), hub)
}

/// The app flying the new game `game` (a cut of the sample, whose start
/// is at Luyten Crossing), wired with its world models `assets` as `main`
/// wires them, landed at Luyten Exchange from the crossing's centre.
pub(crate) fn landed_at_the_exchange(game: NewGame, assets: &WorldAssets) -> Harness {
    let data = std::rc::Rc::new(game.data);
    let pilots = MemoryPilots::new();
    let mut harness = flying_over(Ruleset::NewGame, &data, assets, &pilots);
    harness
        .app
        .screen_mut()
        .flight_view_mut()
        .expect("flying")
        .change_sim(|sim| put_player_at(sim.session_mut().expect("a session"), Vec2::ZERO));
    // L asks for clearance, the approach clears it, and L lands; drawn
    // as `draw` draws, since the new game holds no `PICT`s. L is pressed
    // again each frame until it lands, as the flight may not step its
    // first frames.
    for _ in 0..2000 {
        if harness.showing() == Showing::Spaceport {
            break;
        }
        tap(&mut harness, Key::KeyL);
        draw(&mut harness);
    }
    assert_eq!(harness.showing(), Showing::Spaceport, "landed");
    harness
}

#[test]
fn the_spaceport_s_landscape_draws_the_station_s_model() {
    let (mut harness, hub) = at_the_exchange();
    draw(&mut harness);
    let port = harness.app.screen().spaceport_view().expect("landed");
    let area = port
        .dialog()
        .expect("laid out")
        .item_bounds(LANDSCAPE_ITEM)
        .expect("the landscape item");
    let mut list = DrawList::new();
    port.draw(&mut list);
    let centres: Vec<Point> = list
        .iter()
        .filter_map(|command| match command {
            DrawCommand::Model { model, pose, .. } if *model == hub => Some(pose.center),
            _ => None,
        })
        .collect();
    assert_eq!(centres, [area.center()], "the hub in the landscape");
}

/// The app on the main menu over the 3D sample, keeping pilots in
/// `pilots`, wired as `main` wires it: the world models' table given
/// after the menu is built, then the last pilot opened at launch; and the
/// sample's world models.
fn on_the_menu(pilots: &MemoryPilots) -> (Harness, WorldAssets) {
    let (game, assets) = sample();
    let rules = ruleset_rulebook(None, Ruleset::NewGame.profile().readings).0;
    let table = std::rc::Rc::new(assets.models.clone());
    let harness = Harness::built_wired(std::rc::Rc::new(game.data), pilots, |screen| {
        screen.with_rulebook(&rules).with_world_models(table)
    });
    (harness, assets)
}

/// The hull key of the class the main menu's pilot panel shows, from
/// `assets`, and the models the main menu draws.
fn the_menu_s_class_and_models(
    harness: &Harness,
    assets: &WorldAssets,
) -> (ModelKey, Vec<ModelKey>) {
    let menu = harness.app.screen().main_menu().expect("the main menu");
    let PanelPilot::Alive(panel) = menu.pilot_shown() else {
        panic!("an alive pilot: {:?}", menu.pilot_shown());
    };
    let key = assets
        .models
        .ship(panel.ship)
        .expect("the class's model")
        .key;
    (key, models_of(menu))
}

#[test]
fn the_main_menu_s_pilot_panel_draws_a_new_pilot_s_class_as_its_model() {
    let pilots = MemoryPilots::new();
    let (mut harness, assets) = on_the_menu(&pilots);
    draw(&mut harness);
    // New Pilot, named and christened by the keyboard, saves the pilot
    // and comes back to the menu holding it.
    let new_pilot = harness.menu_button(MenuChoice::NewPilot);
    harness.click(new_pilot);
    draw(&mut harness);
    harness.type_name("Ada");
    harness.press(KeyCode::Enter);
    harness.press(KeyCode::Enter);
    assert_eq!(harness.showing(), Showing::MainMenu);
    draw(&mut harness);
    let (key, drawn) = the_menu_s_class_and_models(&harness, &assets);
    assert!(drawn.contains(&key), "the pilot's class: {drawn:?}");
}

#[test]
fn the_main_menu_draws_a_saved_pilot_opened_at_launch_as_its_model() {
    let pilots = MemoryPilots::new();
    let (mut first, _) = on_the_menu(&pilots);
    draw(&mut first);
    first.create_and_close("Ada");
    let (mut harness, assets) = on_the_menu(&pilots);
    draw(&mut harness);
    let (key, drawn) = the_menu_s_class_and_models(&harness, &assets);
    assert!(drawn.contains(&key), "the saved pilot's class: {drawn:?}");
}
