use super::death::{landed_took_off_and_breaking_up, until_exploded};
use super::encounters::{boarded, hailing};
use super::port::{in_the_bar_by, scrolled_to};
use super::*;

use nova_sim::Rulebook;
use nova_view::ViewShape;

// The router in a 16:9 window: flight widened, every other screen
// classic, and the dialogs over flight in their 1024 x 768 frame, centred.

/// A 16:9 window's widened size.
const WIDE: (f32, f32) = (1365.0, 768.0);

/// How far right a 1024-wide frame sits in [`WIDE`].
const OFFSET: Point = Point::new(170.0, 0.0);

/// `screen` told the window is 16:9.
fn widened(mut screen: AppScreen) -> AppScreen {
    screen.set_widened_size(WIDE);
    screen
}

/// `overlay`'s own drawing, moved [`OFFSET`] right.
fn moved(overlay: &dyn Screen) -> Vec<DrawCommand> {
    let mut classic = DrawList::new();
    overlay.draw(&mut classic);
    assert!(!classic.is_empty());
    let mut list = DrawList::new();
    list.extend_translated(&classic, OFFSET);
    list.iter().cloned().collect()
}

/// Whether `screen`'s frame has flight's status bar, or why it has none,
/// at the widened view's right edge.
fn widened_bar(screen: &AppScreen) -> bool {
    let edge = Point::new(1171.0, 0.0);
    drawn(screen).iter().any(|command| match command {
        DrawCommand::Picture { top_left, .. } => *top_left == edge,
        DrawCommand::Text { text, origin, .. } => {
            *origin == edge && text.starts_with("Status bar unavailable")
        }
        _ => false,
    })
}

#[test]
fn flight_and_the_dialogs_over_it_are_widened() {
    let mut screen = widened(start_screen(data()));
    fly(&mut screen);
    assert_eq!(screen.view_shape(), ViewShape::Widened);
    let screen = hailing();
    assert_eq!(screen.view_shape(), ViewShape::Widened, "comm over flight");
    let screen = boarded();
    assert_eq!(
        screen.view_shape(),
        ViewShape::Widened,
        "plunder over flight"
    );
    let mut screen = with_dialogs(data());
    fly(&mut screen);
    screen.open_offer(OfferPrompt {
        mission: MissionId(130),
        text: "Job.".to_owned(),
        labels: nova_sim::mission::offer_labels("", "", true),
        graphic: None,
        movie: None,
    });
    assert_eq!(screen.showing(), Showing::Offer);
    assert_eq!(screen.view_shape(), ViewShape::Widened, "a mission offer");
}

#[test]
fn every_other_screen_is_classic() {
    let store = MemoryPilots::default();
    let screen = widened(menu(&store));
    assert_eq!(screen.showing(), Showing::MainMenu);
    assert_eq!(screen.view_shape(), ViewShape::Classic);
    let mut screen = widened(start_screen(data()));
    assert_eq!(screen.showing(), Showing::ShipBrowser);
    assert_eq!(screen.view_shape(), ViewShape::Classic);
    screen.input(&key(Key::Tab, true));
    assert_eq!(screen.showing(), Showing::GalaxyMap);
    assert_eq!(screen.view_shape(), ViewShape::Classic);
    let mut screen = widened(with_dialogs(data()));
    land(&mut screen);
    assert_eq!(screen.view_shape(), ViewShape::Classic, "the spaceport");
}

#[test]
fn a_comm_dialog_over_widened_flight_is_its_classic_frame_centred_and_clicked_there() {
    let mut screen = widened(hailing());
    let comm = moved(screen.comm().expect("open"));
    let list: Vec<DrawCommand> = drawn(&screen).iter().cloned().collect();
    assert!(list.ends_with(&comm), "the dialog, 170 right, over flight");
    assert!(widened_bar(&screen), "flight's bar at the widened edge");
    let close = screen
        .comm()
        .expect("open")
        .dialog()
        .item_bounds(CLOSE_ITEM)
        .expect("an item")
        .center();
    for pressed in [true, false] {
        screen.input(&Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at: Point::new(close.x + OFFSET.x, close.y),
        });
    }
    assert!(screen.comm().is_none(), "Close Channel, under the pointer");
    assert_eq!(screen.showing(), Showing::Flight);
}

#[test]
fn a_plunder_dialog_over_widened_flight_is_its_classic_frame_centred() {
    let screen = widened(boarded());
    let plunder = moved(screen.plunder().expect("open"));
    let list: Vec<DrawCommand> = drawn(&screen).iter().cloned().collect();
    assert!(
        list.ends_with(&plunder),
        "the dialog, 170 right, over flight"
    );
    assert!(widened_bar(&screen));
}

#[test]
fn landing_shows_the_classic_spaceport_and_taking_off_widens_again() {
    let data = loaded(&barred(game_data_fork(
        false, false, false, 100, b"", b"", None,
    )));
    let mut screen = widened(with_dialogs(data));
    land(&mut screen);
    assert_eq!(screen.view_shape(), ViewShape::Classic);
    let mut bar = DrawList::new();
    flight(&screen).draw_status_bar(&mut bar);
    let list: Vec<DrawCommand> = drawn(&screen).iter().cloned().collect();
    let bar: Vec<DrawCommand> = bar.iter().cloned().collect();
    assert!(!bar.is_empty());
    assert!(list.starts_with(&bar), "the status bar at the classic edge");
    assert!(bar.iter().any(|command| matches!(
        command,
        DrawCommand::Picture { top_left, .. } if *top_left == Point::new(830.0, 0.0)
    )));
    screen.input(&key(Key::Escape, true));
    assert_eq!(screen.showing(), Showing::Flight);
    assert_eq!(screen.view_shape(), ViewShape::Widened);
}

#[test]
fn a_flight_built_later_takes_the_widened_size_told_before() {
    let mut screen = widened(start_screen(data()));
    fly(&mut screen);
    assert_eq!(flight(&screen).view_size(), WIDE);
    screen.set_widened_size((1366.0, 768.0));
    assert_eq!(
        flight(&screen).view_size(),
        (1366.0, 768.0),
        "and every resize"
    );
}

#[test]
fn the_bars_map_over_the_spaceport_stays_classic_at_a_widened_size() {
    let mut screen = widened(in_the_bar_by(
        Rulebook::default().with_override(RuleKey::MapPlotting, RuleSource::Bible),
    ));
    screen.input(&key(MAP, true));
    assert_eq!(screen.showing(), Showing::FlightMap);
    assert_eq!(screen.view_shape(), ViewShape::Classic);
    let mut list = DrawList::new();
    flight(&screen).draw_status_bar(&mut list);
    spaceport(&screen).draw(&mut list);
    flight(&screen).course_map().draw(&mut list);
    assert_eq!(
        drawn(&screen).iter().cloned().collect::<Vec<_>>(),
        list.iter().cloned().collect::<Vec<_>>(),
        "the map where the classic spaceport has it"
    );
    screen.input(&key(MAP, false));
    let beta = scrolled_to(&mut screen, SystemId(129));
    for pressed in [true, false] {
        screen.input(&Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at: beta,
        });
    }
    assert_eq!(
        flight(&screen)
            .session()
            .ok()
            .map(nova_sim::Session::nav_mode),
        Some(nova_sim::NavMode::Hyperspace(Some(SystemId(129)))),
        "a click at its classic place plots"
    );
}

#[test]
fn deaths_fade_out_of_widened_flight_ends_on_the_classic_main_menu() {
    let store = MemoryPilots::new();
    let mut screen = widened(landed_took_off_and_breaking_up(&store));
    assert_eq!(screen.view_shape(), ViewShape::Widened, "dying in flight");
    until_exploded(&mut screen);
    for _ in 0..=nova_sim::HOLD_TICKS {
        screen.tick(TICK);
    }
    screen.tick(Duration::from_millis(1000));
    assert_eq!(screen.showing(), Showing::MainMenu, "back to the menu");
    assert_eq!(screen.view_shape(), ViewShape::Classic, "in a tick");
}
