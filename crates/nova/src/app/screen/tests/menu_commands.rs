//! The menu bar's commands through the router: Quit and Preferences…
//! do what Command-Q and Command-comma do, in every state, picked by
//! their keys or with the mouse; About opens the About text where
//! nothing holds input or is in front; and the menu greys each where it
//! would do nothing.

use super::background::switch_out;
use super::*;
use nova_view::MenuCommand;
use nova_view::fade::DEATH_FADE_FOR;

#[test]
fn quit_and_preferences_are_command_q_and_command_comma_and_about_has_no_chord() {
    assert_eq!(menu_chord(MenuCommand::Quit), Some(Key::Char('q')));
    assert_eq!(menu_chord(MenuCommand::Preferences), Some(PREFERENCES_KEY));
    assert_eq!(menu_chord(MenuCommand::About), None);
}

#[test]
fn a_chords_key_names_its_command() {
    assert_eq!(chord_command(Key::Char('q')), Some(MenuCommand::Quit));
    assert_eq!(
        chord_command(PREFERENCES_KEY),
        Some(MenuCommand::Preferences)
    );
    assert_eq!(chord_command(Key::Char('i')), None, "About has no chord");
    assert_eq!(chord_command(Key::Char('p')), None);
}

/// What a command left: whether the app quits (an input's action or a
/// quit taken), whether Quit's delay runs, what shows, whether the
/// Preferences or the About text are open, what the shipyard's name
/// prompt holds, if it is open, and the sounds made.
#[derive(Debug, PartialEq)]
#[allow(clippy::struct_excessive_bools)] // Each a separate thing seen.
struct Outcome {
    quit: bool,
    quitting: bool,
    showing: Showing,
    preferences: bool,
    about: bool,
    name: Option<String>,
    sounds: Vec<Sound>,
}

fn outcome(screen: &mut AppScreen, quit: bool) -> Outcome {
    let name = screen
        .spaceport_view()
        .and_then(|spaceport| spaceport.open_shipyard())
        .and_then(|shipyard| shipyard.naming())
        .map(|prompt| prompt.field().text().to_owned());
    Outcome {
        quit: quit || screen.take_quit(),
        quitting: screen.quit_in.is_some(),
        showing: screen.showing(),
        preferences: screen.preferences().is_some(),
        about: screen.about().is_some(),
        name,
        sounds: screen.take_sounds(),
    }
}

/// Feeds `inputs` in order, as the app does, stopping at the first that
/// quits. Whether one did.
fn feed(screen: &mut AppScreen, inputs: &[Input]) -> bool {
    inputs
        .iter()
        .any(|input| screen.input(input) == ScreenAction::Quit)
}

/// `chord` typed with Command, as the window sees it with no menu bar.
fn by_keys(screen: &mut AppScreen, chord: Key) -> bool {
    feed(
        screen,
        &[
            key(Key::Command, true),
            key(chord, true),
            key(chord, false),
            key(Key::Command, false),
        ],
    )
}

/// `command`'s key equivalent typed: the window sees Command go down and
/// up, and the menu bar takes the chord's press between. winit still
/// hands the window the chord's release while Command is held (as
/// [`an_f_released_after_the_view_menu_took_its_press_changes_nothing`]
/// has for F), so it arrives after the command, a second time.
fn by_equivalent(screen: &mut AppScreen, command: MenuCommand) -> bool {
    let chord = menu_chord(command).expect("a chord");
    feed(screen, &[key(Key::Command, true)])
        || screen.menu_command(command) == ScreenAction::Quit
        || feed(screen, &[key(chord, false), key(Key::Command, false)])
}

/// A state the router can be in, the step that ends any hold it is in,
/// and whether a Command-comma there opens the Preferences, at once or
/// once the hold ends, and an About from the menu opens the About text.
struct State {
    name: &'static str,
    make: fn() -> AppScreen,
    then: fn(&mut AppScreen),
    preferences: bool,
    about: bool,
}

/// Runs `drive` on a fresh `state`, then its `then`.
fn run(state: &State, drive: &dyn Fn(&mut AppScreen) -> bool) -> Outcome {
    let mut screen = (state.make)();
    screen.take_sounds();
    let quit = drive(&mut screen);
    (state.then)(&mut screen);
    outcome(&mut screen, quit)
}

/// What `command` does in `state`, checked to be the same whether its
/// Command chord is typed with no menu bar, its key equivalent is typed,
/// or it is picked with the mouse.
fn same_by_keys_and_menu(state: &State, command: MenuCommand) -> Outcome {
    let chord = menu_chord(command).expect("a chord");
    let keys = run(state, &|screen| by_keys(screen, chord));
    let equivalent = run(state, &|screen| by_equivalent(screen, command));
    assert_eq!(equivalent, keys, "{}: its key equivalent", state.name);
    let mouse = run(state, &|screen| {
        screen.menu_command(command) == ScreenAction::Quit
    });
    assert_eq!(mouse, keys, "{}: picked with the mouse", state.name);
    keys
}

fn nothing(_: &mut AppScreen) {}

/// The main menu over a store of its own, with dialogs, appeared.
fn main_menu() -> AppScreen {
    with_dialogs(data())
        .with_pilots(Some(keeper(&MemoryPilots::new())), Rc::new(MonoMetrics))
        .appeared()
}

fn ships() -> AppScreen {
    with_dialogs(data())
}

fn galaxy() -> AppScreen {
    let mut screen = with_dialogs(data());
    screen.input(&key(Key::Tab, true));
    screen.input(&key(Key::Tab, false));
    assert_eq!(screen.showing(), Showing::GalaxyMap);
    screen
}

/// [`main_menu`], a pilot created and flying.
fn flying() -> AppScreen {
    let mut screen = main_menu();
    create(&mut screen, "Ada");
    assert_eq!(screen.showing(), Showing::Flight);
    screen
}

fn landed() -> AppScreen {
    let mut screen = with_dialogs(data());
    land(&mut screen);
    screen
}

fn about_open() -> AppScreen {
    let mut screen = with_dialogs(data());
    open_about(&mut screen);
    screen
}

fn preferences_open() -> AppScreen {
    let mut screen = with_dialogs(data());
    open_prefs(&mut screen);
    screen
}

fn player_info_open() -> AppScreen {
    let mut screen = with_dialogs(data());
    fly(&mut screen);
    screen.input(&key(PLAYER_INFO_KEY, true));
    screen.input(&key(PLAYER_INFO_KEY, false));
    assert_eq!(screen.showing(), Showing::PlayerInfo);
    screen
}

fn quit_question() -> AppScreen {
    let mut screen = main_menu();
    assert!(!screen.close_requested());
    assert_eq!(screen.showing(), Showing::Question);
    screen
}

/// The shipyard's prompt for a bought ship's name, open.
fn naming() -> AppScreen {
    let mut screen = landed_shipyard(&MemoryPilots::new());
    click_port_item(&mut screen, 9);
    select_first_ship(&mut screen);
    screen.input(&key(Key::Char('b'), true));
    screen.input(&Input::Text('b'));
    screen.input(&key(Key::Char('b'), false));
    assert!(
        spaceport(&screen)
            .open_shipyard()
            .is_some_and(|shipyard| shipyard.naming().is_some()),
        "the name prompt"
    );
    screen
}

/// A cinematic movie playing in front of the spaceport.
fn movie() -> AppScreen {
    let (mut screen, _) = super::missions::landed_filmed(0, "Job.", 0x0004, Prefs::default());
    screen.tick(Duration::from_secs(1));
    assert!(screen.hides_pointer(), "the movie plays");
    screen
}

/// Flight's course map with its Find open.
fn finding() -> AppScreen {
    let mut screen = with_dialogs(data());
    fly(&mut screen);
    for typed in ['m', 'f'] {
        screen.input(&key(Key::Char(typed), true));
        screen.input(&Input::Text(typed));
        screen.input(&key(Key::Char(typed), false));
    }
    assert!(flight(&screen).map_finding());
    screen
}

/// The main menu, with dialogs, appearing at start-up.
pub(super) fn starting_up() -> AppScreen {
    with_dialogs(data()).with_pilots(Some(keeper(&MemoryPilots::new())), Rc::new(MonoMetrics))
}

/// The main menu appearing on a return from play.
pub(super) fn returning() -> AppScreen {
    let mut screen = flying();
    screen.input(&key(Key::Escape, true));
    screen.input(&key(Key::Escape, false));
    assert_eq!(screen.showing(), Showing::MainMenu);
    assert!(screen.appear_holds_menu());
    screen
}

fn appear_ends(screen: &mut AppScreen) {
    screen.tick(DEATH_FADE_FOR);
}

/// Play being entered with a pilot that has seen the intro.
pub(super) fn entering() -> AppScreen {
    let mut screen = main_menu();
    create(&mut screen, "Ada");
    escape_to_menu(&mut screen);
    choose(&mut screen, MenuChoice::EnterShip);
    assert!(screen.entering());
    screen
}

/// Play being entered with a new pilot whose intro has a text, its
/// pictures passed with Space, the text open and fading in from black.
/// (The entry's text, once due, opens in the same tick, and the entry
/// is dropped in the tick its play runs, so neither stage is seen.)
fn intro_text_fading_in() -> AppScreen {
    let mut screen = super::entry::intro_menu(&MemoryPilots::new());
    create_held(&mut screen, "Ada");
    choose(&mut screen, MenuChoice::EnterShip);
    screen.tick(DEATH_FADE_FOR);
    for _ in 0..2 {
        screen.tick(DEATH_FADE_FOR);
        screen.input(&key(Key::Space, true));
        screen.input(&key(Key::Space, false));
        screen.tick(DEATH_FADE_FOR);
    }
    assert_eq!(screen.showing(), Showing::IntroText);
    assert!(screen.intro_text().is_some());
    screen
}

/// [`intro_text_fading_in`], the fade ended: the text takes every input.
fn intro_text() -> AppScreen {
    let mut screen = intro_text_fading_in();
    screen.tick(DEATH_FADE_FOR);
    assert!(screen.intro_text().is_some());
    screen
}

/// [`intro_text`] closed with Return, the screen fading to black.
fn after_intro_text() -> AppScreen {
    let mut screen = intro_text();
    screen.input(&key(Key::Enter, true));
    screen.input(&key(Key::Enter, false));
    assert!(screen.intro_text().is_none());
    assert_eq!(screen.showing(), Showing::IntroText, "fading to black");
    screen
}

/// The names of the states whose intro text takes Command-Q, as it
/// takes every key.
const TEXT_TAKES_QUIT: [&str; 2] = ["the intro's text, fading in", "the intro's text"];

fn handed_over(screen: &mut AppScreen) {
    screen.tick(Duration::from_secs(1));
    screen.tick(Duration::from_secs(1));
}

/// The main menu waiting on E's keyed click, which is refused with no
/// pilot, so what it holds reaches the menu.
fn keyed_click() -> AppScreen {
    let mut screen = main_menu();
    screen.input(&Input::Text('e'));
    assert!(screen.menu_hold.is_some());
    screen
}

fn click_ends(screen: &mut AppScreen) {
    screen.keyed_click_ended();
}

/// Quit chosen on the main menu, its delay running.
pub(super) fn quitting() -> AppScreen {
    let mut screen = main_menu();
    choose(&mut screen, MenuChoice::Quit);
    assert!(screen.quit_in.is_some());
    screen
}

/// The main menu without dialogs.
fn without_dialogs() -> AppScreen {
    menu(&MemoryPilots::new())
}

fn states() -> [State; 23] {
    let state = |name, make, then, preferences, about| State {
        name,
        make,
        then,
        preferences,
        about,
    };
    [
        state("the main menu", main_menu, nothing, true, true),
        state("the ship browser", ships, nothing, true, true),
        state("the galaxy viewer", galaxy, nothing, true, true),
        state("flight", flying, nothing, true, true),
        state("the spaceport", landed, nothing, true, true),
        state("the About text", about_open, nothing, false, false),
        state("the Preferences", preferences_open, nothing, false, false),
        state("player info", player_info_open, nothing, false, false),
        state("the quit question", quit_question, nothing, false, false),
        state("the shipyard's name prompt", naming, nothing, false, false),
        state("a movie", movie, nothing, false, false),
        state("a map's Find", finding, nothing, false, false),
        state("the start-up appear", starting_up, appear_ends, true, false),
        state("the return's appear", returning, appear_ends, true, false),
        state("the entry", entering, handed_over, true, false),
        state(
            "the intro's text, fading in",
            intro_text_fading_in,
            through_entry,
            false,
            false,
        ),
        state("the intro's text", intro_text, through_entry, false, false),
        state(
            "the fade after the intro's text",
            after_intro_text,
            through_entry,
            true,
            false,
        ),
        state("a keyed click", keyed_click, click_ends, true, false),
        state("Quit's delay", quitting, nothing, false, false),
        state("no dialogs", without_dialogs, nothing, false, false),
        state(
            "the start-up appear, not ended",
            starting_up,
            nothing,
            true,
            false,
        ),
        state(
            "a keyed click, not ended",
            keyed_click,
            nothing,
            true,
            false,
        ),
    ]
}

#[test]
fn quit_does_what_command_q_does_everywhere() {
    for state in states() {
        let outcome = same_by_keys_and_menu(&state, MenuCommand::Quit);
        let held = matches!(
            state.name,
            "the start-up appear, not ended" | "a keyed click, not ended"
        ) || TEXT_TAKES_QUIT.contains(&state.name);
        assert_eq!(
            outcome.quit || outcome.quitting,
            !held,
            "{}: {outcome:?}",
            state.name
        );
    }
}

#[test]
fn quit_is_never_greyed() {
    for state in states() {
        assert!(
            (state.make)().menu_enabled(MenuCommand::Quit),
            "{}",
            state.name
        );
    }
}

#[test]
fn preferences_do_what_command_comma_does_everywhere() {
    for state in states() {
        let outcome = same_by_keys_and_menu(&state, MenuCommand::Preferences);
        let held = matches!(
            state.name,
            "the start-up appear, not ended" | "a keyed click, not ended"
        );
        let was_open = (state.make)().preferences().is_some();
        let opened = outcome.preferences && !was_open;
        assert_eq!(opened, state.preferences && !held, "{}", state.name);
        assert!(!outcome.quit, "{}", state.name);
    }
}

#[test]
fn preferences_are_greyed_wherever_command_comma_neither_opens_nor_holds_them() {
    for state in states() {
        assert_eq!(
            (state.make)().menu_enabled(MenuCommand::Preferences),
            state.preferences,
            "{}",
            state.name
        );
    }
}

#[test]
fn about_opens_the_about_text_where_nothing_holds_input_or_is_in_front() {
    for state in states() {
        let mut screen = (state.make)();
        let was_open = screen.about().is_some();
        screen.take_sounds();
        assert_eq!(
            screen.menu_command(MenuCommand::About),
            ScreenAction::None,
            "{}",
            state.name
        );
        assert_eq!(screen.take_sounds(), [], "{}: no click", state.name);
        (state.then)(&mut screen);
        let opened = !was_open && screen.about().is_some();
        assert_eq!(opened, state.about, "{}", state.name);
        if opened {
            assert_eq!(screen.showing(), Showing::About, "{}", state.name);
        }
    }
}

#[test]
fn about_is_greyed_wherever_it_opens_nothing() {
    for state in states() {
        assert_eq!(
            (state.make)().menu_enabled(MenuCommand::About),
            state.about,
            "{}",
            state.name
        );
    }
}

/// `_QuitAEVTHandler` (@0x5619e) reads no `_inBackground`, and each event
/// loop hands it the event whatever the flag says (@0x1b242, @0x568e1,
/// @0x599a9). What holds a quit in the front is paused here, or waits for
/// a key, so waiting would keep it until the app is back.
#[test]
fn away_quit_quits_at_once_everywhere() {
    for state in states() {
        let mut screen = (state.make)();
        switch_out(&mut screen);
        assert_eq!(
            screen.menu_command(MenuCommand::Quit),
            ScreenAction::Quit,
            "{}",
            state.name
        );
    }
}

/// The app is back but for its first tick: its gates hold again, and
/// that tick lets them go on.
#[test]
fn back_quit_waits_where_it_does_in_the_front() {
    let mut screen = starting_up();
    switch_out(&mut screen);
    screen.focus_gained();
    assert_eq!(screen.menu_command(MenuCommand::Quit), ScreenAction::None);
    assert!(!screen.take_quit());
}

/// `_PrefAEVTHandler` (@0x5617b) and `_AppEventHandler`'s `pref`
/// (@0xcccf1) read no `_inBackground` either: the Preferences open as in
/// the front, or wait where the front's wait, and stand still until the
/// app is back.
#[test]
fn away_preferences_do_what_they_do_in_the_front() {
    for state in states() {
        let mut screen = (state.make)();
        let was_open = screen.preferences().is_some();
        switch_out(&mut screen);
        assert_eq!(
            screen.menu_command(MenuCommand::Preferences),
            ScreenAction::None,
            "{}",
            state.name
        );
        comes_back(&mut screen);
        (state.then)(&mut screen);
        let held = matches!(
            state.name,
            "the start-up appear, not ended" | "a keyed click, not ended"
        );
        let opened = !was_open && screen.preferences().is_some();
        assert_eq!(opened, state.preferences && !held, "{}", state.name);
        assert!(!screen.take_quit(), "{}", state.name);
    }
}

/// `_AppEventHandler`'s `abou` (@0xcccb0) reads no `_inBackground` either.
#[test]
fn away_about_does_what_it_does_in_the_front() {
    for state in states() {
        let mut screen = (state.make)();
        let was_open = screen.about().is_some();
        switch_out(&mut screen);
        screen.menu_command(MenuCommand::About);
        comes_back(&mut screen);
        (state.then)(&mut screen);
        let opened = !was_open && screen.about().is_some();
        assert_eq!(opened, state.about, "{}", state.name);
    }
}

/// Past the gate what the entry and the appear hold wait for the return,
/// as they do for their ends; a keyed click's sound ends while away too,
/// and what it held has already passed the gate, the command's chord
/// among it.
#[test]
fn away_preferences_held_by_a_keyed_click_open_as_it_ends() {
    let mut screen = keyed_click();
    switch_out(&mut screen);
    screen.menu_command(MenuCommand::Preferences);
    assert!(screen.preferences().is_none(), "held");
    screen.keyed_click_ended();
    assert!(screen.preferences().is_some());
}

/// Only the menu's commands pass: the chords typed are keys, dropped as
/// any key is, on every side.
#[test]
fn away_the_chords_typed_do_nothing() {
    for state in states() {
        let mut screen = (state.make)();
        let was_open = screen.preferences().is_some();
        switch_out(&mut screen);
        assert!(!by_keys(&mut screen, Key::Char('q')), "{}", state.name);
        assert!(!by_keys(&mut screen, PREFERENCES_KEY), "{}", state.name);
        assert_eq!(screen.preferences().is_some(), was_open, "{}", state.name);
        assert!(!screen.take_quit(), "{}", state.name);
    }
}

/// The app comes back and its first tick, the last away, passes.
fn comes_back(screen: &mut AppScreen) {
    screen.focus_gained();
    screen.tick(Duration::ZERO);
}

#[test]
fn quit_from_the_menu_in_flight_saves_the_pilot_as_command_q_does() {
    let store = MemoryPilots::new();
    let mut screen = with_dialogs(data())
        .with_pilots(Some(keeper(&store)), Rc::new(MonoMetrics))
        .appeared();
    create(&mut screen, "Ada");
    screen.tick(TICK);
    let writes = store.writes();
    assert_eq!(screen.menu_command(MenuCommand::Quit), ScreenAction::Quit);
    screen.quit();
    assert_eq!(store.writes(), writes + 1, "saved as the app quits");
}

#[test]
fn an_f_released_after_the_view_menu_took_its_press_changes_nothing() {
    // The View menu's Enter Full Screen takes Control+Command+F's press;
    // the window still sees the modifiers and F's release.
    for (name, make) in [
        ("the main menu", main_menu as fn() -> AppScreen),
        ("the ship browser", ships),
        ("the galaxy viewer", galaxy),
        ("flight", flying),
        ("the spaceport", landed),
    ] {
        let mut screen = make();
        screen.take_sounds();
        let before = screen.showing();
        let action = feed(
            &mut screen,
            &[
                key(Key::Control, true),
                key(Key::Command, true),
                key(Key::Char('f'), false),
                key(Key::Command, false),
                key(Key::Control, false),
            ],
        );
        assert!(!action, "{name}");
        assert_eq!(screen.showing(), before, "{name}");
        assert_eq!(screen.take_sounds(), [], "{name}");
        assert!(!screen.take_quit(), "{name}");
        assert!(screen.menu_hold.is_none(), "{name}: no click waits");
        screen.tick(TICK);
        assert_eq!(screen.showing(), before, "{name}: after a tick");
    }
}
