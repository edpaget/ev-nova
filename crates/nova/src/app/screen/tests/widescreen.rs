use super::death::{landed_took_off_and_breaking_up, until_exploded};
use super::encounters::{boarded, hailing};
use super::missions::{REEL, filmed};
use super::port::{in_the_bar_by, scrolled_to};
use super::*;

use nova_sim::Rulebook;
use nova_view::ViewShape;
use nova_view::movie::MovieStage;

// The router in a 16:9 window: flight widened, every other screen
// classic, and the dialogs over flight in their 1024 x 768 frame, centred.

/// A 16:9 window's widened size.
pub(super) const WIDE: (f32, f32) = (1365.0, 768.0);

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

/// The alphas of the black rects drawn across `screen`'s whole frame
/// from x 0 to `width`, top to bottom, in order: a movie's or a fade's
/// cover. Which rect nova-view puts there is its own tests' to say; these
/// ask only whether the router's frame is covered.
#[allow(clippy::float_cmp)]
pub(super) fn black_across(screen: &AppScreen, width: f32) -> Vec<u8> {
    drawn(screen)
        .iter()
        .filter_map(|command| match *command {
            DrawCommand::Line {
                from,
                to,
                width: high,
                color,
            } if from.x == 0.0
                && to.x == width
                && high == 768.0
                && (color.r, color.g, color.b) == (0, 0, 0) =>
            {
                Some(color.a)
            }
            _ => None,
        })
        .collect()
}

/// Whether any black rect drawn over `screen` reaches past x `right`
/// or before x 0.
fn black_beyond(screen: &AppScreen, right: f32) -> bool {
    drawn(screen).iter().any(|command| {
        matches!(*command, DrawCommand::Line { from, to, color, .. }
            if (color.r, color.g, color.b) == (0, 0, 0)
                && (from.x < 0.0 || to.x > right))
    })
}

/// A text over flight whose movie, "Text.mov", has `Flags` `flags`.
fn filmed_text(flags: u16) -> DescShown {
    DescShown {
        text: "Well done.".to_owned(),
        graphic: None,
        movie: nova_sim::DescMovie::from_desc("Text.mov", flags),
    }
}

/// The router flying, its movies played through a mock player, told the
/// window is 16:9 on the main menu first when `widened_first`.
fn flying_filmed_widened(widened_first: bool) -> AppScreen {
    let (mut screen, _) = filmed(0, "Job.", 0, Prefs::default());
    if widened_first {
        screen = widened(screen);
    }
    create(&mut screen, "Ada");
    assert_eq!(screen.showing(), Showing::Flight);
    screen
}

/// The router flying, its movies played through a mock player.
fn flying_filmed() -> AppScreen {
    flying_filmed_widened(false)
}

/// Ticks `screen` by `millis` in 16 ms frames.
fn ticked_for(screen: &mut AppScreen, millis: u64) {
    for _ in 0..millis.div_ceil(16) {
        screen.tick(Duration::from_millis(16));
    }
}

/// The mission dialogs' own drawing, moved by the router's overlay
/// offset.
fn missions_moved(screen: &AppScreen) -> Vec<DrawCommand> {
    let missions = screen.missions.as_ref().expect("built");
    let mut own = DrawList::new();
    missions.draw(&mut own);
    assert!(!own.is_empty());
    let mut list = DrawList::new();
    list.extend_translated(&own, screen.overlay_offset());
    list.iter().cloned().collect()
}

/// A plain "Text.mov" stage's black window, as the stage draws it in its
/// frame's coordinates, playing a movie [`REEL`] in size.
fn plain_window() -> DrawList {
    let player = MockMoviePlayer::playing(REEL, 2);
    let mut stage = MovieStage::new(
        nova_sim::DescMovie::from_desc("Text.mov", 0).expect("a movie"),
        player as Rc<dyn MoviePlayer>,
    );
    stage.begin();
    let mut drawn = DrawList::new();
    stage.draw(&mut drawn);
    let mut window = DrawList::new();
    window.push(drawn.iter().next().expect("drawn").clone());
    window
}

#[test]
fn a_cinematic_mission_movie_over_widened_flight_covers_the_whole_view() {
    for widen_first in [false, true] {
        let mut screen = flying_filmed_widened(widen_first);
        screen.show_over_flight(vec![filmed_text(0x0004)]);
        if !widen_first {
            screen = widened(screen);
        }
        ticked_for(&mut screen, 500);
        assert!(
            black_across(&screen, WIDE.0)
                .iter()
                .any(|&alpha| alpha > 0 && alpha < 255),
            "{widen_first}: the fade out over the whole widened view, told the \
             size before flight showed or as it shows"
        );
        ticked_for(&mut screen, 1600);
        assert_eq!(screen.showing(), Showing::Movie);
        assert!(
            black_across(&screen, WIDE.0).contains(&255),
            "{widen_first}: the black window over the whole widened view"
        );
    }
}

#[test]
fn a_plain_mission_movie_over_widened_flight_is_centred_where_center_window_puts_it() {
    let mut screen = widened(flying_filmed());
    screen.show_over_flight(vec![filmed_text(0)]);
    assert_eq!(screen.showing(), Showing::Movie);
    let list: Vec<DrawCommand> = drawn(&screen).iter().cloned().collect();
    assert!(
        list.ends_with(&missions_moved(&screen)),
        "the movie's own drawing, moved by the dialogs' offset"
    );
    let mut window = DrawList::new();
    window.extend_translated(&plain_window(), screen.overlay_offset());
    let window = window.iter().next().expect("its black window").clone();
    assert!(
        list.contains(&window),
        "its black window, where the stage puts it, moved by the dialogs' offset"
    );
    assert!(black_across(&screen, WIDE.0).is_empty(), "no cover");
}

#[test]
fn a_cinematic_movie_in_a_4_by_3_window_draws_as_before() {
    let mut screen = flying_filmed();
    screen.show_over_flight(vec![filmed_text(0x0004)]);
    ticked_for(&mut screen, 2100);
    assert_eq!(screen.showing(), Showing::Movie);
    assert_eq!(screen.overlay_offset(), Point::new(0.0, 0.0));
    let list: Vec<DrawCommand> = drawn(&screen).iter().cloned().collect();
    assert!(list.ends_with(&missions_moved(&screen)), "its own drawing");
    assert!(
        black_across(&screen, 1024.0).contains(&255),
        "its black window over the classic frame"
    );
    assert!(!black_beyond(&screen, 1024.0), "nothing past the frame");
}

#[test]
fn the_spaceports_movies_stay_classic_after_a_resize_in_flight() {
    let (mut screen, _) = filmed(0, "Job.", 0x0004, Prefs::default());
    create(&mut screen, "Ada");
    let mut screen = widened(screen);
    land_twice(&mut screen);
    assert_eq!(screen.view_shape(), ViewShape::Classic);
    ticked_for(&mut screen, 1500);
    assert!(
        !black_across(&screen, 1024.0).is_empty(),
        "its black window over the classic screen"
    );
    assert!(!black_beyond(&screen, 1024.0), "and no wider");
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
