//! The sample's starfield drawn through the whole app
//! (rdm:phase/play-the-sample/phase-5-stand-in-starfield): New Pilot over
//! the sample's cut puts the player in flight in Luyten Crossing, whose
//! data has no star sheet, and the frame draws the shown stars as the
//! data's stand-in dots, which scroll against the ship's travel by their
//! parallax.
//!
//! Wiring only: the dots are read back from the data through the view's
//! port (`CombatLooks::star_dots`), not retyped. The palette the loader
//! sets is nova-data's (`new_game::looks::STAR_DOTS`); how a dot is
//! placed, scrolled and drawn, nova-view's (`flight::stars`). Murk
//! dimming is pinned there too, in nova-view's `flight/view/tests/stars.rs`
//! and `flight::stars`, over fakes: the new game maps no murk yet, so the
//! sample is never murky and its dots draw at full level here.

use std::rc::Rc;

use nova::app::{Control, Showing, WindowEvent};
use nova_audio::AudioSettings;
use nova_data::new_game::fixture::sample_folder;
use nova_data::{GameData, new_game};
use nova_render::ImageError;
use nova_sim::Vec2;
use nova_sim::fixture::MemoryPilots;
use nova_view::flight::catalog::{CombatLooks, StarDot};
use nova_view::geometry::Bounds;
use nova_view::menu::MenuChoice;
use nova_view::{Color, DrawCommand, DrawLayer, DrawList, Key, Screen};
use winit::keyboard::KeyCode;

use crate::pilots::Harness;

/// Draws a frame, whose only failures are pictures the data doesn't hold.
fn draw(game: &mut Harness) {
    assert_eq!(game.redraw(), Control::Continue);
    for (key, error) in game.app.take_failures() {
        assert!(matches!(error, ImageError::Missing), "{key:?}: {error:?}");
    }
}

/// The app flying the sample's start, `first-flight` in Luyten Crossing,
/// and the data it flies over.
fn flying_the_sample(store: &MemoryPilots) -> (Harness, Rc<GameData>) {
    let data = Rc::new(
        new_game::open(&sample_folder())
            .expect("the sample lists")
            .data,
    );
    let mut game = Harness::built_on(Rc::clone(&data), store, AudioSettings::default(), true);
    draw(&mut game);
    let new_pilot = game.menu_button(MenuChoice::NewPilot);
    game.click(new_pilot);
    draw(&mut game);
    game.type_name("Ada");
    game.press(KeyCode::Enter);
    game.press(KeyCode::Enter);
    assert_eq!(game.showing(), Showing::MainMenu);
    draw(&mut game);
    let enter = crate::common::enter_ship_at(game.app.screen());
    game.click(enter);
    for _ in 0..200 {
        if game.showing() == Showing::Flight {
            break;
        }
        draw(&mut game);
    }
    assert_eq!(game.showing(), Showing::Flight, "flight never shows");
    draw(&mut game);
    (game, data)
}

/// The data's stand-in dots, as the view reads them.
fn data_dots(data: &GameData) -> Vec<StarDot> {
    CombatLooks::star_dots(data)
        .expect("the sample's stars stand in as dots")
        .as_slice()
        .to_vec()
}

/// Every OR fill in the world layer of the view's frame drawn now: the
/// stars' dots, in draw order.
fn world_dots(game: &Harness) -> Vec<(Bounds, Color)> {
    let view = game.app.screen().flight_view().expect("flying");
    let mut list = DrawList::new();
    view.draw(&mut list);
    list.layered()
        .filter_map(|(layer, command)| match (layer, command) {
            (DrawLayer::World, DrawCommand::OrFill { area, color }) => Some((*area, *color)),
            _ => None,
        })
        .collect()
}

/// The player's ship's position in the world now.
fn ship_position(game: &Harness) -> Vec2 {
    game.app
        .screen()
        .flight_view()
        .expect("flying")
        .session()
        .expect("a session")
        .player()
        .position
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

/// The player's ship's velocity now.
fn ship_velocity(game: &Harness) -> Vec2 {
    game.app
        .screen()
        .flight_view()
        .expect("flying")
        .session()
        .expect("a session")
        .player()
        .velocity
}

#[test]
#[allow(clippy::float_cmp)]
fn the_sample_draws_its_stars_as_the_data_s_dots_at_full_level() {
    let store = MemoryPilots::new();
    let (game, data) = flying_the_sample(&store);
    let dots = data_dots(&data);
    let drawn = world_dots(&game);
    assert!(!drawn.is_empty(), "the stars draw");
    for (area, color) in drawn {
        assert!(
            dots.iter().any(|dot| dot.color == color
                && area.width() == f32::from(dot.side)
                && area.height() == f32::from(dot.side)),
            "{area:?} in {color:?} is none of the data's dots {dots:?}"
        );
    }
}

/// A star moves on screen by its factor of the ship's move less the move
/// itself (`flight::stars`' scroll and placement): against the travel, by
/// between nothing and the whole move. Dots that never scroll fail: at
/// least one must move.
#[test]
fn the_sample_s_dots_scroll_against_the_ship_s_travel() {
    let store = MemoryPilots::new();
    let (mut game, _) = flying_the_sample(&store);
    // Up to speed, then coasting, so the drawn frame's lag behind the
    // session is the same at both ends of the measure. Up is pressed
    // again each frame until it takes: a press sent as flight first shows
    // doesn't thrust.
    for _ in 0..2000 {
        if ship_velocity(&game).length() > 0.0 {
            break;
        }
        thrust(&mut game, true);
        draw(&mut game);
    }
    assert!(ship_velocity(&game).length() > 0.0, "Up never thrusts");
    for _ in 0..120 {
        draw(&mut game);
    }
    thrust(&mut game, false);
    for _ in 0..5 {
        draw(&mut game);
    }
    let (from, before) = (ship_position(&game), world_dots(&game));
    for _ in 0..20 {
        draw(&mut game);
    }
    let (to, after) = (ship_position(&game), world_dots(&game));
    let travel = to - from;
    assert!(travel.length() > 10.0, "the ship travels: {travel:?}");
    assert_eq!(before.len(), after.len(), "the same stars shown");

    // A wrapped dot jumps the play area's width or height, far more than
    // the travel; the others move with it. A pixel each way is the
    // placement's truncation.
    let slack = 1.5;
    let mut moved = 0;
    for ((was, _), (now, _)) in before.iter().zip(&after) {
        let shift = Vec2::new(
            now.center().x - was.center().x,
            now.center().y - was.center().y,
        );
        if shift.length() > 200.0 {
            continue;
        }
        // The shift's parts along the travel and across it.
        let along = (shift.x * travel.x + shift.y * travel.y) / travel.length();
        let across = (shift.y * travel.x - shift.x * travel.y) / travel.length();
        assert!(
            along <= slack && -along <= travel.length() + slack,
            "{shift:?} against {travel:?}: between 0 and the travel"
        );
        assert!(across.abs() <= slack, "{shift:?} across {travel:?}");
        if shift.length() > slack {
            moved += 1;
        }
    }
    assert!(moved > 0, "some dot scrolls");
}
