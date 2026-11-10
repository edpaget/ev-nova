//! The new game draws the layered sky: over the checked-in sample
//! (`nova_data::new_game::fixture::sample_folder`), the router wired with
//! the rulebook `ruleset_rulebook` gives `Ruleset::NewGame`, as `main`
//! wires it, the flight's world layer holds one `DrawCommand::Sky` and
//! none of the data's stand-in dots, and the sky's eye moves with the
//! ship. Under the stock ruleset's readings the same flight draws the
//! dots and no sky.
//!
//! Wiring only: the layered sky's rule is nova-view's (`flight::sky` and
//! the view's `sky` module), its look nova-render's (`sky`), each pinned
//! there; this pins that the ruleset, the router and the view meet.

use nova::app::screen::AppScreen;
use nova::app::{Control, WindowEvent};
use nova::rulebook::ruleset_rulebook;
use nova::ruleset::Ruleset;
use nova_audio::SettingsStore;
use nova_audio::recording::MemorySettings;
use nova_data::new_game;
use nova_sim::Vec2;
use nova_sim::fixture::MemoryPilots;
use nova_view::{DrawCommand, DrawLayer, DrawList, Key, Screen, SkyPose};

use crate::common::{draw, fly_new_pilot_wired};
use crate::pilots::Harness;

/// The app flying the sample's start under `ruleset`'s readings.
fn flying(ruleset: Ruleset, pilots: &MemoryPilots) -> Harness {
    let data = std::rc::Rc::new(
        new_game::open(&new_game::fixture::sample_folder())
            .expect("the sample lists")
            .data,
    );
    let rules = ruleset_rulebook(None, ruleset.profile().readings).0;
    let settings = MemorySettings::new();
    let (keeper, warning) =
        nova::settings::SettingsKeeper::open(Box::new(settings) as Box<dyn SettingsStore>);
    assert_eq!(warning, None);
    fly_new_pilot_wired(&data, pilots, keeper, |screen: AppScreen| {
        screen.with_rulebook(&rules)
    })
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

/// The skies in the world layer now.
fn skies(game: &Harness) -> Vec<SkyPose> {
    world(game)
        .into_iter()
        .filter_map(|command| match command {
            DrawCommand::Sky { sky } => Some(sky),
            _ => None,
        })
        .collect()
}

/// How many OR fills (the stand-in dots) the world layer holds now.
fn dots(game: &Harness) -> usize {
    world(game)
        .iter()
        .filter(|command| matches!(command, DrawCommand::OrFill { .. }))
        .count()
}

/// The player's ship's position and velocity in the world now.
fn ship(game: &Harness) -> (Vec2, Vec2) {
    let player = game
        .app
        .screen()
        .flight_view()
        .expect("flying")
        .session()
        .expect("a session")
        .player();
    (player.position, player.velocity)
}

/// Sends Up going down, or up when not `pressed`.
fn thrust(game: &mut Harness, pressed: bool) {
    let event = WindowEvent::Key {
        key: Key::ArrowUp,
        pressed,
        repeat: false,
    };
    assert_eq!(game.handle(event), Control::Continue);
}

#[test]
fn the_new_game_draws_the_layered_sky_and_no_dots() {
    let pilots = MemoryPilots::new();
    let game = flying(Ruleset::NewGame, &pilots);
    assert_eq!(skies(&game).len(), 1, "one sky");
    assert!(
        matches!(world(&game).first(), Some(DrawCommand::Sky { .. })),
        "first in the world"
    );
    assert_eq!(dots(&game), 0, "none of the data's dots");
}

#[test]
fn the_stock_readings_draw_the_dots_and_no_sky() {
    let pilots = MemoryPilots::new();
    let game = flying(Ruleset::Stock, &pilots);
    assert_eq!(skies(&game), []);
    assert!(dots(&game) > 0, "the data's dots");
}

#[test]
fn the_new_game_s_sky_moves_with_the_ship() {
    let pilots = MemoryPilots::new();
    let mut game = flying(Ruleset::NewGame, &pilots);
    // Up to speed, then coasting, so the drawn frame's lag behind the
    // session is the same at both ends of the measure. Up is pressed
    // again each frame until it takes.
    for _ in 0..2000 {
        if ship(&game).1.length() > 0.0 {
            break;
        }
        thrust(&mut game, true);
        draw(&mut game);
    }
    assert!(ship(&game).1.length() > 0.0, "Up never thrusts");
    for _ in 0..120 {
        draw(&mut game);
    }
    thrust(&mut game, false);
    for _ in 0..5 {
        draw(&mut game);
    }
    let (from, before) = (ship(&game).0, skies(&game)[0].eye);
    for _ in 0..20 {
        draw(&mut game);
    }
    let (to, after) = (ship(&game).0, skies(&game)[0].eye);
    let travel = to - from;
    assert!(travel.length() > 10.0, "the ship travels: {travel:?}");
    // The eye is over the ship as drawn, which trails the session by
    // under a step at each end: within a few units of its travel.
    let eye = (after.x - before.x, after.y - before.y);
    assert!(
        (eye.0 - travel.x).abs() < 3.0 && (eye.1 - travel.y).abs() < 3.0,
        "the eye moved {eye:?}, the ship {travel:?}"
    );
    let camera = game.app.screen().flight_view().expect("flying").camera();
    assert_eq!(after, camera.center(), "over the camera's centre");
}
