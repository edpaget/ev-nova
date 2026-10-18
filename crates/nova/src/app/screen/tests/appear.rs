//! The main screen appearing through the router: the start-up fade and
//! the return from play (`nova_view::menu::appear`).

use super::*;
use nova_view::fade::DEATH_FADE_FOR;

/// The router with dialogs and the main menu over `store`, just made: its
/// start-up appear under way.
fn opening(store: &MemoryPilots) -> AppScreen {
    with_dialogs(data()).with_pilots(Some(keeper(store)), Rc::new(MonoMetrics))
}

/// [`opening`] with `main_screen_return_fade` read as `source`.
fn opening_by(store: &MemoryPilots, source: RuleSource) -> AppScreen {
    with_dialogs(data())
        .with_rules(Rulebook::default().with_override(RuleKey::MainScreenReturnFade, source))
        .with_pilots(Some(keeper(store)), Rc::new(MonoMetrics))
}

/// The alpha of the black covering the whole screen, if the last thing
/// `screen` draws is one.
#[allow(clippy::float_cmp)] // The screen's height, exactly as drawn.
fn cover(screen: &AppScreen) -> Option<u8> {
    match drawn(screen).iter().last() {
        Some(DrawCommand::Line { width, color, .. })
            if *width == 768.0 && (color.r, color.g, color.b) == (0, 0, 0) =>
        {
            Some(color.a)
        }
        _ => None,
    }
}

/// Whether `screen` draws a slide's frame.
fn draws_a_slide(screen: &AppScreen) -> bool {
    drawn(screen).iter().any(|command| match command {
        DrawCommand::Clipped { command, .. } => matches!(
            **command,
            DrawCommand::Picture { image, .. } if (8030..=8032).contains(&image.id)
        ),
        _ => false,
    })
}

/// Whether `screen` draws any of the buttons' `rlëD`s.
fn draws_a_button(screen: &AppScreen) -> bool {
    drawn(screen).iter().any(|command| {
        matches!(command, DrawCommand::Picture { image, .. }
            if image.kind == nova_view::ImageKind::Rled)
    })
}

const START: Sound = Sound::Ui(UiSound::RowSlideStart);

fn left(pressed: bool, at: Point) -> Input {
    Input::PointerButton {
        button: MouseButton::Left,
        pressed,
        at,
    }
}

/// A point off every button.
const NOWHERE: Point = Point::new(1.0, 1.0);

/// Types `typed` with its key, as the platform reports it.
fn type_key(screen: &mut AppScreen, typed: char) {
    screen.input(&key(Key::Char(typed), true));
    screen.input(&Input::Text(typed));
    screen.input(&key(Key::Char(typed), false));
}

/// Command-`typed`.
fn command(screen: &mut AppScreen, typed: char) -> ScreenAction {
    screen.input(&key(Key::Command, true));
    let action = screen.input(&key(Key::Char(typed), true));
    screen.input(&key(Key::Char(typed), false));
    screen.input(&key(Key::Command, false));
    action
}

/// [`opening`] past its start-up appear, a pilot created and flying.
fn flying(store: &MemoryPilots, source: RuleSource) -> AppScreen {
    let mut screen = appeared(opening_by(store, source));
    create(&mut screen, "Ada");
    assert_eq!(screen.showing(), Showing::Flight);
    screen.take_sounds();
    screen
}

// The start-up.

#[test]
fn the_game_opens_on_the_main_screen_fading_in_from_black_and_its_slides_play_after() {
    let store = MemoryPilots::new();
    let mut screen = opening(&store);
    assert_eq!(screen.showing(), Showing::MainMenu);
    assert_eq!(cover(&screen), Some(255));
    assert!(!draws_a_button(&screen), "no row slid in yet");
    assert!(!screen.hides_pointer());
    screen.tick(Duration::from_millis(500));
    assert_eq!(cover(&screen), Some(128), "over the main screen");
    assert!(!draws_a_slide(&screen), "nothing moves under the fade");
    screen.tick(Duration::from_millis(500));
    assert_eq!(cover(&screen), None, "faded in");
    assert_eq!(screen.take_sounds(), []);
    screen.tick(TICK);
    assert!(draws_a_slide(&screen), "the slides play after");
    assert_eq!(
        screen.take_sounds(),
        [START, START],
        "two Mac ticks and the step due"
    );
}

#[test]
fn clicks_and_keys_during_the_start_up_fade_are_dropped() {
    let store = MemoryPilots::new();
    let mut screen = opening(&store);
    let new = screen
        .main_menu()
        .expect("a menu")
        .press_point(MenuChoice::NewPilot);
    screen.input(&left(true, new));
    screen.input(&left(false, new));
    type_key(&mut screen, 'o');
    assert_eq!(screen.input(&key(Key::Escape, true)), ScreenAction::None);
    screen.input(&key(Key::Tab, true));
    screen.input(&key(Key::Char('i'), true));
    screen.tick(DEATH_FADE_FOR);
    assert_eq!(screen.showing(), Showing::MainMenu, "nothing opened");
    assert!(screen.main_menu().expect("a menu").slides_running());
    assert_eq!(screen.take_sounds(), [], "no click, no keyed click");
    assert!(!screen.take_quit());
}

#[test]
fn command_q_during_the_start_up_fade_quits_once_it_ends() {
    let store = MemoryPilots::new();
    let mut screen = opening(&store);
    assert_eq!(command(&mut screen, 'q'), ScreenAction::None);
    screen.tick(Duration::from_millis(999));
    assert!(!screen.take_quit(), "not during the fade");
    screen.tick(Duration::from_millis(1));
    assert!(screen.take_quit());
}

#[test]
fn command_comma_during_the_start_up_fade_opens_the_preferences_once_it_ends() {
    let store = MemoryPilots::new();
    let mut screen = opening(&store);
    command(&mut screen, ',');
    assert_eq!(screen.showing(), Showing::MainMenu);
    screen.tick(DEATH_FADE_FOR);
    assert_eq!(screen.showing(), Showing::Preferences);
}

#[test]
fn a_close_during_the_start_up_fade_asks_once_it_ends() {
    let store = MemoryPilots::new();
    let mut screen = opening(&store);
    assert!(!screen.close_requested(), "the window stays open");
    assert!(screen.question().is_none(), "not during the fade");
    screen.tick(DEATH_FADE_FOR);
    assert!(screen.question().is_some(), "asked once it ends");
    assert!(!screen.take_quit());
}

#[test]
fn the_start_up_drops_the_same_whatever_the_return_rule() {
    let store = MemoryPilots::new();
    let mut screen = opening_by(&store, RuleSource::Bible);
    assert_eq!(cover(&screen), Some(255));
    choose(&mut screen, MenuChoice::NewPilot);
    screen.tick(DEATH_FADE_FOR);
    assert_eq!(screen.showing(), Showing::MainMenu);
}

// The return, by the engine's reading.

#[test]
fn escape_from_flight_returns_to_a_black_second_then_the_main_screen_and_its_slides() {
    let store = MemoryPilots::new();
    let mut screen = flying(&store, RuleSource::Engine);
    screen.input(&key(Key::Escape, true));
    assert_eq!(screen.showing(), Showing::MainMenu);
    assert_eq!(cover(&screen), Some(255), "black");
    screen.tick(Duration::from_millis(500));
    assert_eq!(cover(&screen), Some(255), "black through the fade");
    assert!(!draws_a_slide(&screen));
    screen.tick(Duration::from_millis(500));
    assert_eq!(cover(&screen), None, "the main screen appears");
    assert!(!draws_a_button(&screen), "its rows not slid in yet");
    assert_eq!(screen.take_sounds(), []);
    screen.tick(TICK);
    assert!(draws_a_slide(&screen));
    assert_eq!(screen.take_sounds(), [START, START]);
    assert!(!screen.hides_pointer());
}

#[test]
fn the_return_keeps_the_rollovers_frame_and_clears_it() {
    let store = MemoryPilots::new();
    let mut screen = appeared(opening(&store));
    let about = screen
        .main_menu()
        .expect("a menu")
        .press_point(MenuChoice::About);
    screen.input(&Input::PointerMoved(about));
    screen.tick(Duration::from_millis(500));
    assert_eq!(
        screen.main_menu().and_then(MainMenu::rollover),
        Some((5, 32))
    );
    create(&mut screen, "Ada");
    screen.input(&key(Key::Escape, true));
    assert_eq!(
        screen.main_menu().and_then(MainMenu::rollover),
        Some((5, 0)),
        "clear, its frame kept"
    );
}

#[test]
fn a_return_with_the_button_held_stays_black_until_it_is_let_go_of() {
    let store = MemoryPilots::new();
    let mut screen = flying(&store, RuleSource::Engine);
    screen.input(&left(true, NOWHERE));
    screen.input(&key(Key::Escape, true));
    screen.tick(Duration::from_secs(5));
    assert_eq!(cover(&screen), Some(255), "the spin waits for the release");
    assert!(!screen.hides_pointer(), "the pointer shows meanwhile");
    screen.input(&left(false, NOWHERE));
    screen.tick(Duration::from_millis(999));
    assert_eq!(
        cover(&screen),
        Some(255),
        "the whole second from the release"
    );
    screen.tick(Duration::from_millis(1));
    assert_eq!(cover(&screen), None);
}

#[test]
fn a_release_outside_the_game_ends_the_wait() {
    let store = MemoryPilots::new();
    let mut screen = flying(&store, RuleSource::Engine);
    screen.input(&left(true, NOWHERE));
    screen.input(&key(Key::Escape, true));
    screen.cancel_pointer();
    screen.tick(DEATH_FADE_FOR);
    assert_eq!(cover(&screen), None);
}

#[test]
fn a_button_let_go_of_in_flight_is_not_waited_for() {
    let store = MemoryPilots::new();
    let mut screen = flying(&store, RuleSource::Engine);
    screen.input(&left(true, NOWHERE));
    screen.input(&left(false, NOWHERE));
    screen.input(&key(Key::Escape, true));
    screen.tick(DEATH_FADE_FOR);
    assert_eq!(cover(&screen), None);
}

#[test]
fn clicks_during_the_return_are_dropped() {
    let store = MemoryPilots::new();
    let mut screen = flying(&store, RuleSource::Engine);
    screen.input(&key(Key::Escape, true));
    let new = screen
        .main_menu()
        .expect("a menu")
        .press_point(MenuChoice::NewPilot);
    screen.tick(Duration::from_millis(500));
    screen.input(&left(true, new));
    screen.input(&left(false, new));
    screen.tick(Duration::from_millis(500));
    assert_eq!(screen.showing(), Showing::MainMenu, "no dialog");
    assert_eq!(screen.take_sounds(), [], "no 600 or 601");
    assert!(
        screen.main_menu().expect("a menu").slides_running(),
        "nor a skip"
    );
}

#[test]
fn a_key_typed_during_the_return_reaches_the_main_screen_after_it() {
    let store = MemoryPilots::new();
    let mut screen = flying(&store, RuleSource::Engine);
    screen.input(&key(Key::Escape, true));
    screen.input(&key(Key::Escape, false));
    type_key(&mut screen, 'n');
    assert_eq!(screen.take_sounds(), [], "not yet");
    screen.tick(DEATH_FADE_FOR);
    assert_eq!(
        screen.take_sounds(),
        [Sound::Ui(UiSound::ButtonKeyed)],
        "N's keyed click, once the fade ends"
    );
    screen.keyed_click_ended();
    assert_eq!(screen.showing(), Showing::NewPilot);
}

#[test]
fn command_q_during_the_return_quits_once_it_ends() {
    let store = MemoryPilots::new();
    let mut screen = flying(&store, RuleSource::Engine);
    screen.input(&key(Key::Escape, true));
    assert_eq!(command(&mut screen, 'q'), ScreenAction::None);
    screen.tick(Duration::from_millis(999));
    assert!(!screen.take_quit());
    screen.tick(Duration::from_millis(1));
    assert!(screen.take_quit());
}

#[test]
fn a_close_during_the_return_is_dropped() {
    let store = MemoryPilots::new();
    let mut screen = flying(&store, RuleSource::Engine);
    screen.input(&key(Key::Escape, true));
    assert!(!screen.close_requested(), "the window stays open");
    screen.tick(DEATH_FADE_FOR);
    assert!(screen.question().is_none(), "never asked");
    assert!(!screen.take_quit());
}

// The return, by the other reading.

#[test]
fn by_the_other_reading_the_return_fades_over_a_menu_that_takes_input() {
    let store = MemoryPilots::new();
    let mut screen = flying(&store, RuleSource::Bible);
    screen.input(&left(true, NOWHERE));
    screen.input(&key(Key::Escape, true));
    assert_eq!(cover(&screen), Some(255));
    screen.tick(Duration::from_millis(50));
    assert_eq!(cover(&screen), Some(242), "no wait for the button");
    assert!(draws_a_slide(&screen), "the slides run under it");
    assert!(screen.take_sounds().contains(&START));
    screen.tick(Duration::from_millis(450));
    assert_eq!(cover(&screen), Some(128));
    assert_eq!(
        choose(&mut screen, MenuChoice::NewPilot),
        ScreenAction::None
    );
    assert_eq!(screen.showing(), Showing::NewPilot, "the menu takes input");
}

#[test]
fn by_the_other_reading_a_cancel_under_the_fade_reaches_the_live_menu() {
    let store = MemoryPilots::new();
    let mut screen = flying(&store, RuleSource::Bible);
    screen.input(&key(Key::Escape, true));
    let new = screen
        .main_menu()
        .expect("a menu")
        .press_point(MenuChoice::NewPilot);
    screen.input(&left(true, new));
    screen.cancel_pointer();
    screen.input(&left(false, new));
    assert_eq!(
        screen.showing(),
        Showing::MainMenu,
        "the press was cancelled"
    );
}
