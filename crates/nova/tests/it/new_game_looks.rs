//! The new game's ships and stellars drawn as their stand-ins through the
//! whole app (rdm:phase/new-game-engine/phase-4-draw-flat-outlines): New
//! Pilot over the checked-in fixture puts the player in flight in
//! `hearth`, in the hauler, and the flight draws the hauler as solid
//! geometry in its livery and `hearth-dock` as its stand-in, with no
//! "Sprite unavailable" anywhere.
//!
//! Wiring only: what each stand-in is, is nova-data's
//! (`new_game::looks`); how one is drawn and turned, nova-view's
//! (`outline`, the flight and scene tests); how a triangle reaches the
//! GPU, nova-render's.

use std::rc::Rc;

use nova::app::{Control, Showing};
use nova_audio::AudioSettings;
use nova_data::new_game;
use nova_render::ImageError;
use nova_sim::fixture::MemoryPilots;
use nova_view::menu::MenuChoice;
use nova_view::{Color, DrawCommand, DrawLayer, DrawList, Point, Screen};
use winit::keyboard::KeyCode;

use crate::pilots::Harness;

/// The guild's colour, `#c08040`: the hauler's livery's primary, and the
/// dock's owner's.
const GUILD: Color = Color::rgba(0xc0, 0x80, 0x40, 255);

/// Draws a frame, whose only failures are pictures the data doesn't hold.
fn draw(game: &mut Harness) {
    assert_eq!(game.redraw(), Control::Continue);
    for (key, error) in game.app.take_failures() {
        assert!(matches!(error, ImageError::Missing), "{key:?}: {error:?}");
    }
}

/// The app flying the fixture's start: New Pilot, named and confirmed,
/// then entered.
fn flying_the_fixture(store: &MemoryPilots) -> Harness {
    let data = new_game::open(&new_game::fixture::folder())
        .expect("the fixture lists")
        .data;
    let mut game = Harness::built_on(Rc::new(data), store, AudioSettings::default(), true);
    draw(&mut game);
    let new_pilot = game.menu_button(MenuChoice::NewPilot);
    game.click(new_pilot);
    draw(&mut game);
    game.type_name("Ada");
    game.press(KeyCode::Enter);
    // The ship's christening, confirmed with its default.
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
    game
}

/// The world's triangles in `color` that flight draws, as their corners.
fn world_triangles(list: &DrawList, color: Color) -> Vec<[Point; 3]> {
    list.layered()
        .filter_map(|(layer, command)| match command {
            DrawCommand::Triangle { corners, color: c }
                if layer == DrawLayer::World && *c == color =>
            {
                Some(*corners)
            }
            _ => None,
        })
        .collect()
}

/// Whether `at` is inside the box round `corners`.
fn boxes(corners: &[Point; 3], at: Point) -> bool {
    let (xs, ys) = (corners.map(|p| p.x), corners.map(|p| p.y));
    let low = |v: [f32; 3]| v.into_iter().fold(f32::MAX, f32::min);
    let high = |v: [f32; 3]| v.into_iter().fold(f32::MIN, f32::max);
    (low(xs)..=high(xs)).contains(&at.x) && (low(ys)..=high(ys)).contains(&at.y)
}

#[test]
fn the_new_game_s_ships_and_stellars_draw_as_their_stand_ins() {
    let store = MemoryPilots::new();
    let game = flying_the_fixture(&store);
    let view = game.app.screen().flight_view().expect("flying");
    let mut list = DrawList::new();
    view.draw(&mut list);
    let guild = world_triangles(&list, GUILD);

    // The hauler: solid geometry in its livery round where the player is
    // drawn.
    let player = view.camera().world_to_screen(view.shown_position());
    assert!(
        guild.iter().any(|corners| boxes(corners, player)),
        "the hauler round {player:?}: {guild:?}"
    );

    // The dock: its stand-in, in the guild's colour too, told from the
    // hauler by its shape: a disc's triangles fan out from its centre, so
    // a corner three or more of them share, where no corner of a box is.
    let shared = guild.iter().flatten().find(|&&corner| {
        guild
            .iter()
            .filter(|corners| corners.contains(&corner))
            .count()
            >= 3
    });
    assert!(shared.is_some(), "the dock's disc: {guild:?}");

    let unavailable: Vec<_> = list
        .iter()
        .filter_map(|command| match command {
            DrawCommand::Text { text, .. } if text.contains("Sprite unavailable") => Some(text),
            _ => None,
        })
        .collect();
    assert_eq!(unavailable, Vec::<&String>::new());
}
