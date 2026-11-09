//! The sample's status bar and radar drawn through the whole app
//! (rdm:phase/play-the-sample/phase-4-status-bar-and-radar): New Pilot
//! over the sample's cut puts the player in flight in Luyten Crossing, and
//! once traffic is in, the frame draws the placeholder `ïntf` 128: the
//! player's bright pixel at the radar's centre, the stellars, gates and
//! ships in the dim radar colour inside the radar (the station and its
//! two gates as squares, the ships as pixels), the shield, armour and
//! fuel gauges, and no "Status bar unavailable".
//!
//! Wiring only: the layout is read back from the view's own status bar,
//! not retyped. What the placeholder holds is nova-data's
//! (`new_game::records::default_interface`); how a layout draws,
//! nova-view's (`flight::hud`, `flight::radar`).

use std::rc::Rc;

use nova::app::{Control, Showing};
use nova_audio::AudioSettings;
use nova_data::new_game;
use nova_data::new_game::fixture::sample_folder;
use nova_render::ImageError;
use nova_sim::fixture::MemoryPilots;
use nova_view::flight::catalog::StatusBarLayout;
use nova_view::flight::hud::bar_origin;
use nova_view::geometry::Bounds;
use nova_view::menu::MenuChoice;
use nova_view::{Color, DrawCommand, DrawList, Point, Screen};
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
/// with traffic in the system.
fn flying_the_sample(store: &MemoryPilots) -> Harness {
    let data = new_game::open(&sample_folder())
        .expect("the sample lists")
        .data;
    let mut game = Harness::built_on(Rc::new(data), store, AudioSettings::default(), true);
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
    for _ in 0..3000 {
        if !dim_ship_dots(&game).is_empty() {
            return game;
        }
        draw(&mut game);
    }
    panic!("no ship ever shows on the radar");
}

/// The view's frame drawn now, with its bar's layout and placed radar.
fn drawn(game: &Harness) -> (DrawList, StatusBarLayout, Bounds) {
    let view = game.app.screen().flight_view().expect("flying");
    let bar = view.status_bar().expect("a status bar");
    let radar = bar.layout.radar.offset(bar_origin(bar, view.view_size().0));
    let mut list = DrawList::new();
    view.draw(&mut list);
    (list, bar.layout.clone(), radar)
}

/// The dim radar pixels drawn now: the ships (the sample's one stellar is
/// a station, drawn as a square like the gates).
fn dim_ship_dots(game: &Harness) -> Vec<Point> {
    let (list, layout, _) = drawn(game);
    dots(&list, layout.dim_radar)
}

/// Whether `at` is inside `area`, its right and bottom edges included.
fn inside(area: Bounds, at: Point) -> bool {
    (area.min.x..=area.max.x).contains(&at.x) && (area.min.y..=area.max.y).contains(&at.y)
}

/// The centres of the 1-pixel dots in `color`.
fn dots(list: &DrawList, color: Color) -> Vec<Point> {
    list.iter()
        .filter_map(|command| match command {
            DrawCommand::Dot {
                center, color: c, ..
            } if *c == color => Some(*center),
            _ => None,
        })
        .collect()
}

/// The filled rects (a line as thick as the rect is tall) in `color`, as
/// their left and right midpoints.
fn fills(list: &DrawList, color: Color) -> Vec<(Point, Point)> {
    list.iter()
        .filter_map(|command| match command {
            DrawCommand::Line {
                from, to, color: c, ..
            } if *c == color => Some((*from, *to)),
            _ => None,
        })
        .collect()
}

#[test]
fn the_sample_draws_its_status_bar_and_radar() {
    let store = MemoryPilots::new();
    let game = flying_the_sample(&store);
    let (list, layout, radar) = drawn(&game);

    // The player: one bright pixel at the radar's centre.
    let centre = Point::new(
        radar.min.x.midpoint(radar.max.x).trunc() + 0.5,
        radar.min.y.midpoint(radar.max.y).trunc() + 0.5,
    );
    assert!(
        dots(&list, layout.bright_radar).contains(&centre),
        "the player at {centre:?}"
    );

    // The ships, as dim pixels, and Luyten Exchange and its two gates, as
    // dim squares, all inside the radar.
    let ships = dots(&list, layout.dim_radar);
    assert!(!ships.is_empty(), "the ships' pixels");
    assert!(ships.iter().all(|&at| inside(radar, at)), "{ships:?}");
    let squares = fills(&list, layout.dim_radar);
    assert_eq!(
        squares.len(),
        3,
        "the station's and gates' squares: {squares:?}"
    );
    assert!(
        squares
            .iter()
            .all(|&(from, to)| inside(radar, from) && inside(radar, to)),
        "{squares:?}"
    );

    // The gauges.
    for (what, color) in [
        ("shield", layout.shield_color),
        ("armour", layout.armor_color),
        ("fuel", layout.fuel_full),
    ] {
        assert_eq!(fills(&list, color).len(), 1, "the {what} gauge");
    }

    let unavailable: Vec<_> = list
        .iter()
        .filter_map(|command| match command {
            DrawCommand::Text { text, .. } if text.contains("Status bar") => Some(text),
            _ => None,
        })
        .collect();
    assert_eq!(unavailable, Vec::<&String>::new());
}

/// Every text the bar draws (every text right of the bar's origin) lies,
/// its first line's top-left corner and its height, inside one of the nav,
/// weapon, target and cargo areas as placed, and inside the view. Read
/// against nova-view's real drawing, so it holds whatever baselines
/// nova-view picks; it catches an area too short for its readout.
#[test]
fn the_sample_draws_every_readout_inside_its_area() {
    let store = MemoryPilots::new();
    let game = flying_the_sample(&store);
    let view = game.app.screen().flight_view().expect("flying");
    let bar = view.status_bar().expect("a status bar");
    let (width, height) = view.view_size();
    let origin = bar_origin(bar, width);
    let layout = &bar.layout;
    let areas = [
        ("nav", layout.nav),
        ("weapon", layout.weap),
        ("target", layout.targ),
        ("cargo", layout.cargo),
    ]
    .map(|(what, area)| (what, area.offset(origin)));
    let mut list = DrawList::new();
    view.draw(&mut list);

    let texts: Vec<(&String, Point, f32)> = list
        .iter()
        .filter_map(|command| match command {
            DrawCommand::Text {
                text,
                origin: at,
                size,
                ..
            } if at.x >= origin.x => Some((text, *at, *size)),
            _ => None,
        })
        .collect();
    assert!(!texts.is_empty(), "the bar draws its readouts");
    for &(text, at, size) in &texts {
        let bottom = Point::new(at.x, at.y + size);
        assert!(
            areas
                .iter()
                .any(|&(_, area)| inside(area, at) && inside(area, bottom)),
            "{text:?} at {at:?} (size {size}) lies outside every area: {areas:?}"
        );
        assert!(bottom.y <= height, "{text:?} at {at:?} below the view");
    }
}
