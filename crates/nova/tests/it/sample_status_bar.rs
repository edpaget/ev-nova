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

use nova_data::new_game;
use nova_data::records::stellar::Stellar;
use nova_data::records::system::System;
use nova_render::FontFaces;
use nova_render::wgpu::GlyphonMetrics;
use nova_sim::Reserves;
use nova_sim::fixture::MemoryPilots;
use nova_view::flight::catalog::StatusBarLayout;
use nova_view::flight::hud::{self, HudState, NavDestination, NavDisplay, bar_origin};
use nova_view::flight::radar::RadarScene;
use nova_view::geometry::Bounds;
use nova_view::text::TextMetrics;
use nova_view::{Color, DrawCommand, DrawList, Point, Screen};

use crate::common::{draw, fly_the_sample};
use crate::pilots::Harness;

/// The app flying the sample's start, `first-flight` in Luyten Crossing,
/// with traffic in the system.
fn flying_the_sample(store: &MemoryPilots) -> Harness {
    let (mut game, _) = fly_the_sample(store);
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

/// The gate autopilot's two nav lines (rdm:phase/play-the-sample/phase-10-new-game-autopilot-hud),
/// drawn by nova-view's real HUD on the sample's placeholder bar for each
/// of the sample's systems, and for an unexplored one, over its longest
/// gate name, fit the nav area: each line, as wide as the bundled font
/// measures it at the layout's size, lies between the area's sides, and
/// each line's box (its top, a line height deep) inside the area. What the
/// lines say is nova-view's (`flight::hud`); the names are the sample's
/// own, read from its data.
#[test]
fn the_autopilots_nav_lines_fit_the_placeholder_nav_area() {
    let sample = new_game::open(&new_game::fixture::sample_folder()).expect("the sample lists");
    let systems: Vec<String> = sample
        .data
        .records::<System>()
        .filter_map(|(_, entry)| entry.ok()?.name.map(str::to_owned))
        .collect();
    assert!(!systems.is_empty(), "the sample's systems");
    let gate = sample
        .ids
        .gates
        .values()
        .filter_map(|&id| sample.data.get::<Stellar>(id)?.ok()?.name)
        .max_by(|a, b| a.len().cmp(&b.len()))
        .expect("the sample's gates")
        .to_owned();

    let store = MemoryPilots::new();
    let (flying, _) = fly_the_sample(&store);
    let view = flying.app.screen().flight_view().expect("flying");
    let bar = view.status_bar().expect("a status bar");
    let width = view.view_size().0;
    let layout = &bar.layout;
    let nav = layout.nav.offset(bar_origin(bar, width));
    let metrics = GlyphonMetrics::new(&FontFaces::bundled());
    let radar = RadarScene::default();
    let line_height = metrics.line_height(layout.font, layout.font_size);

    let destinations = systems
        .into_iter()
        .map(NavDestination::Named)
        .chain([NavDestination::Unexplored]);
    for to in destinations {
        let state = HudState {
            radar: &radar,
            blink_on: false,
            reserves: Reserves::default(),
            nav: NavDisplay::GatePilot {
                to: to.clone(),
                gate: Some(gate.clone()),
            },
            cargo: None,
            disabled: false,
        };
        let mut list = DrawList::new();
        hud::draw(&mut list, bar, &state, Some(&metrics), width);
        let lines: Vec<(&String, Point)> = list
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text {
                    text, origin: at, ..
                } if (nav.min.x..=nav.max.x).contains(&at.x)
                    && (nav.min.y - line_height..=nav.max.y).contains(&at.y) =>
                {
                    Some((text, *at))
                }
                _ => None,
            })
            .collect();
        assert_eq!(
            lines
                .iter()
                .map(|(text, _)| text.as_str())
                .collect::<Vec<_>>(),
            [hud::gate_pilot_label(&to).as_str(), gate.as_str()],
            "{to:?}: the autopilot's two lines"
        );
        for (text, at) in lines {
            let right = at.x + metrics.width(layout.font, layout.font_size, text);
            assert!(
                at.x >= nav.min.x && right <= nav.max.x,
                "{text:?} spans {}..{right}, outside the nav area's {}..{}",
                at.x,
                nav.min.x,
                nav.max.x
            );
            assert!(
                at.y >= nav.min.y && at.y + line_height <= nav.max.y,
                "{text:?} spans {}..{}, outside the nav area's {}..{}",
                at.y,
                at.y + line_height,
                nav.min.y,
                nav.max.y
            );
        }
    }
}
