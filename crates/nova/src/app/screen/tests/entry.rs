//! Play entered from the main menu through the router: the fade out, the
//! intro, the handover with its warm-up frame and flight's fade-in
//! (`nova_view::menu::entry`, `nova_view::menu::intro`).

use super::*;
use nova_view::fade::DEATH_FADE_FOR;
use nova_view::menu::intro::{INTRO_SOUND, SKIP_KEY};

const SECOND: Duration = DEATH_FADE_FOR;

/// The intro text's `dësc`.
const INTRO_TEXT: i16 = 600;

/// The alpha of the black covering the whole view, if the last thing
/// `screen` draws is one.
#[allow(clippy::float_cmp)] // The view's height, exactly as drawn.
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

/// The intro picture `screen` draws, if any.
fn intro_picture(screen: &AppScreen) -> Option<i16> {
    drawn(screen).iter().find_map(|command| match command {
        DrawCommand::Picture { image, .. } if (8200..=8201).contains(&image.id) => Some(image.id),
        _ => None,
    })
}

/// A small blank `PICT`.
fn blank() -> Vec<u8> {
    PictBuilder::new([0, 0, 4, 8]).end().build()
}

/// A `chär` 127, first by ID, starting as [`data`]'s does, whose intro
/// is `PICT` 8200 for 2 s, then (past an unused slot) 8201 for 3 s, then
/// the text `text`.
fn intro_character(text: i16) -> Vec<u8> {
    let mut bytes = vec![0; Character::SIZE.expect("fixed")];
    let mut put = |at: usize, value: i16| bytes[at..at + 2].copy_from_slice(&value.to_be_bytes());
    put(0x04, 128);
    put(0x06, 128);
    for slot in 1..4 {
        put(0x06 + 2 * slot, -1);
    }
    for (slot, (picture, delay)) in [(8200, 2), (-1, 0), (8201, 3), (0, 0)]
        .into_iter()
        .enumerate()
    {
        put(0x20 + 2 * slot, picture);
        put(0x28 + 2 * slot, delay);
    }
    put(0x30, text);
    bytes
}

/// [`data`], whose first `chär` has an intro ([`intro_character`]) whose
/// text is `text`, a `dësc` "Welcome, <PN>." when it is [`INTRO_TEXT`];
/// with snd 30003 when `sound`.
fn intro_data_of(text: i16, sound: bool) -> Rc<GameData> {
    let mut fork = game_data_fork(false, false, false, 100, b"", b"", None)
        .resource(Character::TYPE, 127, Some(b"Intro"), &intro_character(text))
        .resource(PICT, 8200, None, &blank())
        .resource(PICT, 8201, None, &blank())
        .resource(Desc::TYPE, INTRO_TEXT, None, &desc_of("Welcome, <PN>."));
    if sound {
        fork = fork.resource(nova_rsrc::ResType::new(*b"snd "), 30_003, None, &[0; 8]);
    }
    loaded(&fork)
}

fn intro_data() -> Rc<GameData> {
    intro_data_of(INTRO_TEXT, false)
}

/// The router with dialogs and the main menu over `store`, on `data`,
/// past its appear, the rules `rules`.
fn opening_with(store: &MemoryPilots, data: Rc<GameData>, rules: Rulebook) -> AppScreen {
    with_dialogs(data)
        .with_rules(rules)
        .with_pilots(Some(keeper(store)), Rc::new(MonoMetrics))
        .appeared()
}

/// The router with dialogs on [`intro_data`], over `store`.
pub(super) fn intro_menu(store: &MemoryPilots) -> AppScreen {
    opening_with(store, intro_data(), Rulebook::default())
}

/// [`intro_menu`] with `intro_for_reopened_pilot` read as `source`.
fn intro_menu_by(store: &MemoryPilots, source: RuleSource) -> AppScreen {
    opening_with(
        store,
        intro_data(),
        Rulebook::default().with_override(RuleKey::IntroForReopenedPilot, source),
    )
}

fn tap(screen: &mut AppScreen, tapped: Key) {
    screen.input(&key(tapped, true));
    screen.input(&key(tapped, false));
}

fn click_anywhere(screen: &mut AppScreen) {
    for pressed in [true, false] {
        screen.input(&Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at: Point::new(500.0, 400.0),
        });
    }
}

/// Command-Q, as typed: Command, Q with its text, then both let go of.
fn command_q(screen: &mut AppScreen) {
    screen.input(&key(Key::Command, true));
    screen.input(&key(Key::Char('q'), true));
    screen.input(&Input::Text('q'));
    screen.input(&key(Key::Char('q'), false));
    screen.input(&key(Key::Command, false));
}

/// Ticks out the main screen's fade, into the intro's first picture.
fn faded_out(screen: &mut AppScreen) {
    screen.tick(SECOND);
}

/// Ticks a picture in, through its whole fade.
fn picture_in(screen: &mut AppScreen) {
    screen.tick(SECOND);
}

// Step 6: the fade out, the handover and flight's fade-in.

#[test]
fn after_enter_ship_the_menu_shows_a_second_under_a_rising_black_and_its_sounds_halt() {
    let store = MemoryPilots::new();
    let mut screen = menu(&store);
    create_held(&mut screen, "Ada");
    screen.take_sounds();
    choose(&mut screen, MenuChoice::EnterShip);
    assert_eq!(
        screen.take_sounds(),
        [
            Sound::Ui(UiSound::ButtonDown),
            Sound::Ui(UiSound::ButtonUp),
            Sound::Ui(UiSound::MainScreenSoundsHalted)
        ],
        "the halt after the click"
    );
    assert!(screen.entering());
    assert_eq!(screen.showing(), Showing::MainMenu);
    assert!(!screen.hides_pointer());
    assert_eq!(cover(&screen), None, "clear at first");
    screen.tick(SECOND / 2);
    assert_eq!(screen.showing(), Showing::MainMenu);
    assert_eq!(cover(&screen), Some(128));
    assert!(screen.game_in_progress().is_some(), "still held");
    // The menu takes nothing meanwhile: New Pilot's click is dropped.
    choose(&mut screen, MenuChoice::NewPilot);
    assert_eq!(screen.showing(), Showing::MainMenu);
    assert!(screen.new_pilot().is_none());
}

#[test]
fn flight_shows_once_the_fade_ends_under_a_black_that_clears_over_a_second_unticked() {
    let store = MemoryPilots::new();
    let mut screen = menu(&store);
    create_held(&mut screen, "Ada");
    let before = *screen
        .game_in_progress()
        .expect("held")
        .session()
        .expect("a session")
        .player();
    choose(&mut screen, MenuChoice::EnterShip);
    screen.tick(Duration::from_millis(999));
    assert_eq!(screen.showing(), Showing::MainMenu);
    screen.tick(Duration::from_millis(1));
    assert_eq!(screen.showing(), Showing::Flight);
    assert!(screen.entering(), "fading in");
    assert!(screen.game_in_progress().is_none(), "moved into flight");
    assert_eq!(cover(&screen), Some(255), "the first frame under black");
    assert_eq!(flight(&screen).message(), None, "the warm-up ran no step");
    screen.tick(SECOND / 2);
    assert_eq!(cover(&screen), Some(128));
    assert_eq!(ship(&screen), before, "not ticked");
    assert_eq!(flight(&screen).message(), None, "not ticked");
    screen.tick(SECOND / 2);
    assert!(!screen.entering());
    assert_eq!(cover(&screen), None);
    screen.tick(TICK);
    assert!(flight(&screen).message().is_some(), "it ticks now");
}

#[test]
fn keys_held_as_play_starts_do_nothing_until_their_next_repeat() {
    let store = MemoryPilots::new();
    let mut screen = menu(&store);
    create_held(&mut screen, "Ada");
    let before = *screen
        .game_in_progress()
        .expect("held")
        .session()
        .expect("a session")
        .player();
    choose(&mut screen, MenuChoice::EnterShip);
    screen.input(&key(Key::Up, true));
    screen.input(&key(Key::Space, true));
    screen.tick(SECOND / 2);
    screen.input(&held(Key::Up));
    screen.input(&held(Key::Space));
    screen.tick(SECOND / 2);
    assert_eq!(screen.showing(), Showing::Flight);
    assert_eq!(ship(&screen), before, "the warm-up frame moved nothing");
    screen.input(&held(Key::Up));
    screen.input(&held(Key::Space));
    screen.tick(SECOND);
    assert!(!screen.entering());
    assert_eq!(ship(&screen), before, "nor the fade-in");
    screen.tick(TICK * 3);
    let after = ship(&screen);
    assert_eq!(after.velocity, before.velocity, "no thrust");
    assert_eq!(after.position, before.position);
    assert!(
        flight(&screen)
            .session()
            .expect("flying")
            .shots()
            .is_empty(),
        "no shot"
    );
    screen.input(&held(Key::Up));
    screen.tick(TICK * 3);
    assert_ne!(
        ship(&screen).velocity,
        before.velocity,
        "the repeat thrusts"
    );
}

#[test]
fn reentering_shuts_the_escort_menu_and_starts_the_cursor_hidden() {
    let store = MemoryPilots::new();
    let data = data();
    let mut pilot = nova_sim::Pilot::new(data.as_ref(), "Ada").expect("a pilot");
    pilot.escorts.push(nova_sim::Escort {
        ship: nova_sim::ShipId(128),
        reserves: nova_sim::Reserves::full(10.0, 10.0, 100.0),
        order: None,
        carried: false,
        wage: None,
        person: None,
    });
    pilot.seen_intro = true;
    store.put("Ada", &nova_sim::save::encode(&pilot));
    let mut screen = menu(&store);
    open_held(&mut screen);
    enter_ship(&mut screen);
    screen.tick(TICK);
    tap(&mut screen, nova_view::flight::escorts::MENU_KEY);
    assert!(flight(&screen).escort_menu().is_open());
    screen.input(&Input::PointerMoved(Point::new(300.0, 300.0)));
    assert!(flight(&screen).cursor_at().is_some());
    escape_to_menu(&mut screen);
    assert!(
        screen
            .game_in_progress()
            .expect("held")
            .escort_menu()
            .is_open(),
        "left open by the return"
    );
    choose(&mut screen, MenuChoice::EnterShip);
    screen.tick(SECOND);
    assert_eq!(screen.showing(), Showing::Flight);
    assert!(!flight(&screen).escort_menu().is_open(), "shut");
    assert_eq!(flight(&screen).cursor_at(), None, "hidden");
}

#[test]
fn command_q_during_the_fade_out_quits_at_the_handover_saving_the_pilot() {
    let store = MemoryPilots::new();
    let mut screen = menu(&store);
    create_held(&mut screen, "Ada");
    choose(&mut screen, MenuChoice::EnterShip);
    command_q(&mut screen);
    assert!(!screen.take_quit(), "held");
    screen.tick(SECOND / 2);
    assert!(!screen.take_quit());
    screen.tick(SECOND / 2);
    assert_eq!(screen.showing(), Showing::Flight);
    assert!(screen.take_quit(), "replayed at the handover");
    assert!(!saved(&store, "Ada").seen_intro(), "nothing saved at entry");
    screen.quit();
    assert!(
        saved(&store, "Ada").seen_intro(),
        "the quit's save keeps the flag"
    );
}

#[test]
fn a_close_during_the_fade_out_is_replayed_at_the_handover() {
    let store = MemoryPilots::new();
    let mut screen = menu(&store);
    create(&mut screen, "Ada");
    escape_to_menu(&mut screen);
    choose(&mut screen, MenuChoice::EnterShip);
    assert!(!screen.close_requested(), "held, the window stays open");
    assert!(!screen.take_quit());
    screen.tick(SECOND);
    assert_eq!(screen.showing(), Showing::Flight);
    assert!(screen.take_quit(), "a close in flight quits");
}

#[test]
fn a_close_during_a_first_entry_is_dropped_with_the_intros_flush() {
    let store = MemoryPilots::new();
    let mut screen = menu(&store);
    create_held(&mut screen, "Ada");
    choose(&mut screen, MenuChoice::EnterShip);
    assert!(!screen.close_requested());
    through_entry(&mut screen);
    assert!(!screen.take_quit(), "the empty intro ran, and dropped it");
}

#[test]
fn command_q_during_flights_fade_in_waits_for_it() {
    let store = MemoryPilots::new();
    let mut screen = menu(&store);
    create(&mut screen, "Ada");
    escape_to_menu(&mut screen);
    choose(&mut screen, MenuChoice::EnterShip);
    screen.tick(SECOND);
    command_q(&mut screen);
    screen.tick(SECOND / 2);
    assert!(!screen.take_quit());
    screen.tick(SECOND / 2);
    assert!(screen.take_quit(), "replayed as the fade-in ends");
}

#[test]
fn a_close_during_flights_fade_in_waits_for_it() {
    let store = MemoryPilots::new();
    let mut screen = menu(&store);
    create(&mut screen, "Ada");
    escape_to_menu(&mut screen);
    choose(&mut screen, MenuChoice::EnterShip);
    screen.tick(SECOND);
    assert_eq!(screen.showing(), Showing::Flight, "handed over");
    assert!(!screen.close_requested(), "held, the window stays open");
    assert!(!screen.take_quit());
    screen.tick(SECOND / 2);
    assert!(screen.entering(), "still fading in");
    assert!(!screen.take_quit(), "held at half the fade");
    screen.tick(SECOND / 2);
    assert!(!screen.entering());
    assert!(screen.take_quit(), "replayed as the fade-in ends");
}

#[test]
fn escape_during_the_entry_doesnt_leave_play() {
    let store = MemoryPilots::new();
    let mut screen = menu(&store);
    create_held(&mut screen, "Ada");
    choose(&mut screen, MenuChoice::EnterShip);
    tap(&mut screen, Key::Escape);
    screen.tick(SECOND);
    tap(&mut screen, Key::Escape);
    screen.tick(SECOND);
    assert!(!screen.entering());
    assert_eq!(screen.showing(), Showing::Flight);
    assert!(screen.game_in_progress().is_none());
}

#[test]
fn the_developers_f_still_enters_flight_at_once() {
    let store = MemoryPilots::new();
    let mut screen = menu(&store);
    screen.input(&key(Key::Tab, true));
    screen.input(&key(Key::Char('f'), true));
    assert_eq!(screen.showing(), Showing::Flight);
    assert!(!screen.entering());
    assert_eq!(cover(&screen), None);
}

#[test]
fn letting_go_of_the_keys_or_the_pointer_during_the_entry_reaches_nothing_else() {
    let store = MemoryPilots::new();
    let mut screen = menu(&store);
    create_held(&mut screen, "Ada");
    choose(&mut screen, MenuChoice::EnterShip);
    screen.input(&key(Key::Command, true));
    screen.release_keys();
    screen.cancel_pointer();
    screen.input(&key(Key::Char('q'), true));
    through_entry(&mut screen);
    assert!(!screen.take_quit(), "Command was let go of");
    assert_eq!(screen.showing(), Showing::Flight);
}

/// The main menu on [`trading_data`] over `store`, holding a pilot that
/// has seen the intro and carries 5 tons of food, Enter Ship just
/// clicked.
fn entering_with_cargo(store: &MemoryPilots) -> AppScreen {
    let data = trading_data();
    let mut pilot = nova_sim::Pilot::new(data.as_ref(), "Ada").expect("a pilot");
    pilot.seen_intro = true;
    pilot.cargo.insert(FOOD, 5);
    store.put("Ada", &nova_sim::save::encode(&pilot));
    let mut screen =
        appeared(AppScreen::new(data).with_pilots(Some(keeper(store)), Rc::new(MonoMetrics)));
    open_held(&mut screen);
    choose(&mut screen, MenuChoice::EnterShip);
    screen
}

/// Ticks `screen` through the handover and flight's fade-in, and checks
/// that play runs on: flight shows, and the game wasn't handed back to
/// the main menu.
fn into_play(screen: &mut AppScreen) {
    screen.tick(SECOND);
    screen.tick(SECOND);
    assert!(!screen.entering());
    assert_eq!(screen.showing(), Showing::Flight, "play runs on");
    assert!(screen.game_in_progress().is_none(), "not returned");
}

/// Checks that flight holds no arrow down: the ship doesn't thrust or
/// turn on its own.
fn no_arrow_held(screen: &mut AppScreen) {
    let before = ship(screen);
    screen.tick(TICK * 3);
    let after = ship(screen);
    assert_eq!(after.velocity, before.velocity, "no thrust");
    assert_eq!(after.heading.to_bits(), before.heading.to_bits(), "no turn");
}

#[test]
fn command_tab_away_during_the_entry_leaves_command_up_in_flight() {
    let store = MemoryPilots::new();
    let mut screen = entering_with_cargo(&store);
    screen.input(&key(Key::Command, true));
    screen.input(&key(Key::Tab, true));
    screen.release_keys();
    into_play(&mut screen);
    no_arrow_held(&mut screen);
    assert_eq!(pilot(&screen).held(FOOD), 5, "carried into play");
    tap(&mut screen, nova_view::flight::view::JETTISON_KEY);
    screen.tick(TICK);
    assert_eq!(
        pilot(&screen).held(FOOD),
        5,
        "K alone jettisons nothing: Command isn't down"
    );
}

#[test]
fn an_arrow_let_go_of_after_command_during_the_entry_isnt_held_in_flight() {
    let store = MemoryPilots::new();
    let mut screen = entering_with_cargo(&store);
    screen.input(&key(Key::Command, true));
    screen.input(&key(Key::Up, true));
    screen.input(&key(Key::Command, false));
    screen.input(&key(Key::Up, false));
    into_play(&mut screen);
    no_arrow_held(&mut screen);
}

#[test]
fn command_comma_during_the_entry_opens_the_preferences_over_play() {
    let store = MemoryPilots::new();
    let mut screen = opening_with(&store, data(), Rulebook::default());
    create(&mut screen, "Ada");
    escape_to_menu(&mut screen);
    choose(&mut screen, MenuChoice::EnterShip);
    screen.input(&key(Key::Command, true));
    tap(&mut screen, PREFERENCES_KEY);
    screen.input(&key(Key::Command, false));
    assert!(screen.preferences().is_none(), "held");
    screen.tick(SECOND);
    assert!(screen.game_in_progress().is_none(), "handed over");
    assert!(screen.preferences().is_some(), "replayed at the handover");
    assert!(!screen.take_quit());
}

#[test]
fn command_escape_during_the_entry_doesnt_leave_play() {
    let store = MemoryPilots::new();
    let mut screen = entering_with_cargo(&store);
    screen.input(&key(Key::Command, true));
    tap(&mut screen, Key::Escape);
    screen.input(&key(Key::Command, false));
    screen.tick(SECOND);
    assert_eq!(screen.showing(), Showing::Flight, "handed over");
    screen.input(&key(Key::Command, true));
    tap(&mut screen, Key::Escape);
    screen.input(&key(Key::Command, false));
    into_play(&mut screen);
    no_arrow_held(&mut screen);
}

#[test]
fn flights_fade_in_covers_the_widened_view_and_the_fade_out_the_classic_one() {
    let store = MemoryPilots::new();
    let mut screen = menu(&store);
    screen.set_widened_size((1365.0, 768.0));
    create_held(&mut screen, "Ada");
    choose(&mut screen, MenuChoice::EnterShip);
    let span = |screen: &AppScreen| match drawn(screen).iter().last() {
        Some(DrawCommand::Line { from, to, .. }) => Some((from.x, to.x)),
        _ => None,
    };
    screen.tick(SECOND / 2);
    assert_eq!(span(&screen), Some((0.0, 1024.0)), "over the menu");
    screen.tick(SECOND / 2);
    assert_eq!(screen.showing(), Showing::Flight);
    assert_eq!(span(&screen), Some((0.0, 1365.0)), "over widened flight");
}

// Step 7: the intro.

#[test]
fn a_new_pilot_sees_each_picture_for_its_delay_then_the_text_then_flight() {
    let store = MemoryPilots::new();
    let mut screen = intro_menu(&store);
    create_held(&mut screen, "Ada");
    choose(&mut screen, MenuChoice::EnterShip);
    faded_out(&mut screen);
    assert_eq!(screen.showing(), Showing::Intro);
    assert!(screen.hides_pointer());
    assert_eq!(intro_picture(&screen), Some(8200));
    assert_eq!(cover(&screen), Some(255), "fading in from black");
    picture_in(&mut screen);
    assert_eq!(cover(&screen), None);
    screen.tick(Duration::from_millis(1999));
    assert_eq!(cover(&screen), None, "not a tick before its delay");
    screen.tick(Duration::from_millis(1));
    screen.tick(SECOND);
    assert_eq!(intro_picture(&screen), Some(8201));
    picture_in(&mut screen);
    screen.tick(Duration::from_secs(3));
    screen.tick(SECOND);
    assert_eq!(screen.showing(), Showing::IntroText);
    assert!(!screen.hides_pointer(), "shown for the text");
    let text = screen.intro_text().expect("the text");
    assert_eq!(text.lines(), ["Welcome, Ada."], "its tags filled");
    // A Return during the text's fade-in reaches it once the fade ends.
    tap(&mut screen, Key::Enter);
    assert!(screen.intro_text().is_some(), "not before");
    screen.tick(SECOND);
    assert!(screen.intro_text().is_none(), "closed by the Return");
    assert_eq!(screen.showing(), Showing::IntroText, "fading to black");
    screen.tick(SECOND);
    assert_eq!(screen.showing(), Showing::Flight);
    assert!(flight(&screen).seen_intro());
    screen.tick(SECOND);
    assert!(!screen.entering());
}

#[test]
fn the_text_is_desc_dialog_over_black_under_its_fade_in() {
    let store = MemoryPilots::new();
    let mut text = intro_menu(&store);
    create_held(&mut text, "Bea");
    choose(&mut text, MenuChoice::EnterShip);
    faded_out(&mut text);
    for _ in 0..2 {
        picture_in(&mut text);
        tap(&mut text, Key::Space);
        text.tick(SECOND);
    }
    assert_eq!(text.showing(), Showing::IntroText);
    assert_eq!(cover(&text), Some(255), "its fade-in over it");
    text.tick(SECOND / 2);
    assert_eq!(cover(&text), Some(128));
    let list = drawn(&text);
    let first_black = list.iter().position(|command| {
        matches!(command, DrawCommand::Line { color, width, .. }
            if *color == Color::BLACK && (*width - 768.0).abs() < f32::EPSILON)
    });
    assert!(first_black.is_some(), "black under the dialog");
    text.tick(SECOND / 2);
    assert_eq!(cover(&text), None, "shown");
    assert!(text.intro_text().is_some());
}

#[test]
fn a_picture_advances_early_on_return_space_or_a_click() {
    for advance in [Key::Enter, Key::Space, Key::Other] {
        let store = MemoryPilots::new();
        let mut screen = intro_menu(&store);
        create_held(&mut screen, "Ada");
        choose(&mut screen, MenuChoice::EnterShip);
        faded_out(&mut screen);
        picture_in(&mut screen);
        screen.tick(Duration::from_millis(100));
        if advance == Key::Other {
            click_anywhere(&mut screen);
        } else {
            tap(&mut screen, advance);
        }
        screen.tick(SECOND);
        assert_eq!(intro_picture(&screen), Some(8201), "{advance:?}");
    }
}

#[test]
fn the_skip_key_during_the_first_picture_skips_the_second_and_the_text() {
    let store = MemoryPilots::new();
    let mut screen = intro_menu(&store);
    create_held(&mut screen, "Ada");
    choose(&mut screen, MenuChoice::EnterShip);
    faded_out(&mut screen);
    picture_in(&mut screen);
    tap(&mut screen, SKIP_KEY);
    assert_eq!(screen.showing(), Showing::Intro, "its fade out");
    screen.tick(SECOND);
    assert_eq!(screen.showing(), Showing::Flight, "no 8201, no text");
    assert!(screen.intro_text().is_none());
    assert!(flight(&screen).seen_intro());
}

#[test]
fn an_empty_intro_marks_the_pilot_seen() {
    let store = MemoryPilots::new();
    let mut screen = menu(&store);
    create_held(&mut screen, "Ada");
    assert!(!screen.game_in_progress().expect("held").seen_intro());
    choose(&mut screen, MenuChoice::EnterShip);
    screen.tick(SECOND);
    assert_eq!(screen.showing(), Showing::Flight, "no picture, no dialogs");
    assert!(flight(&screen).seen_intro());
}

#[test]
fn a_seen_pilot_goes_from_the_fade_straight_to_flight_after_a_return_and_a_reopen() {
    let store = MemoryPilots::new();
    let mut screen = intro_menu(&store);
    create_held(&mut screen, "Ada");
    enter_ship(&mut screen);
    escape_to_menu(&mut screen);
    choose(&mut screen, MenuChoice::EnterShip);
    screen.tick(SECOND);
    assert_eq!(screen.showing(), Showing::Flight, "no intro again");
    screen.tick(SECOND);
    assert!(
        saved(&store, "Ada").seen_intro(),
        "the return's save kept it"
    );
    screen.quit();
    let mut screen = intro_menu(&store);
    open_held(&mut screen);
    choose(&mut screen, MenuChoice::EnterShip);
    screen.tick(SECOND);
    assert_eq!(screen.showing(), Showing::Flight, "nor after a reopen");
}

#[test]
fn an_unseen_pilot_reopened_in_a_new_run_gets_the_intro_its_rule_says() {
    for (source, sees) in [(RuleSource::Engine, false), (RuleSource::Bible, true)] {
        let store = MemoryPilots::new();
        let mut first = intro_menu(&store);
        create_held(&mut first, "Ada");
        assert!(!saved(&store, "Ada").seen_intro());
        let mut screen = intro_menu_by(&store, source);
        open_held(&mut screen);
        choose(&mut screen, MenuChoice::EnterShip);
        faded_out(&mut screen);
        assert_eq!(
            screen.showing() == Showing::Intro,
            sees,
            "{source:?}: {:?}",
            screen.showing()
        );
        through_entry(&mut screen);
        assert!(flight(&screen).seen_intro(), "{source:?}: marked seen");
    }
}

#[test]
fn a_new_pilot_made_earlier_in_the_run_gives_a_reopened_pilot_its_intro_by_either_reading() {
    for source in [RuleSource::Engine, RuleSource::Bible] {
        let store = MemoryPilots::new();
        let mut first = intro_menu(&store);
        create_held(&mut first, "Ada");
        let mut screen = intro_menu_by(&store, source);
        create_held(&mut screen, "Bea");
        open_held(&mut screen);
        assert_eq!(
            screen
                .game_in_progress()
                .expect("held")
                .pilot()
                .expect("a pilot")
                .name(),
            "Ada"
        );
        choose(&mut screen, MenuChoice::EnterShip);
        faded_out(&mut screen);
        assert_eq!(screen.showing(), Showing::Intro, "{source:?}");
    }
}

#[test]
fn the_texts_opening_drops_a_command_q_held_from_the_pictures() {
    let store = MemoryPilots::new();
    let mut screen = intro_menu(&store);
    create_held(&mut screen, "Ada");
    choose(&mut screen, MenuChoice::EnterShip);
    faded_out(&mut screen);
    command_q(&mut screen);
    for _ in 0..2 {
        picture_in(&mut screen);
        tap(&mut screen, Key::Space);
        screen.tick(SECOND);
    }
    assert_eq!(screen.showing(), Showing::IntroText);
    through_entry(&mut screen);
    assert!(!screen.take_quit(), "dropped");
}

#[test]
fn a_text_that_cant_be_found_shows_an_empty_dialog() {
    let store = MemoryPilots::new();
    let mut screen = opening_with(&store, intro_data_of(999, false), Rulebook::default());
    create_held(&mut screen, "Ada");
    choose(&mut screen, MenuChoice::EnterShip);
    faded_out(&mut screen);
    tap(&mut screen, Key::Space);
    for _ in 0..2 {
        picture_in(&mut screen);
        tap(&mut screen, Key::Space);
        screen.tick(SECOND);
    }
    assert_eq!(screen.showing(), Showing::IntroText);
    assert!(
        screen
            .intro_text()
            .expect("open")
            .lines()
            .iter()
            .all(String::is_empty),
        "empty"
    );
}

#[test]
fn letting_go_of_the_keys_or_the_pointer_during_a_picture_starts_its_timer() {
    for let_go in [false, true] {
        let store = MemoryPilots::new();
        let mut screen = intro_menu(&store);
        create_held(&mut screen, "Ada");
        choose(&mut screen, MenuChoice::EnterShip);
        faded_out(&mut screen);
        if let_go {
            screen.input(&key(Key::Space, true));
        } else {
            screen.input(&Input::PointerButton {
                button: MouseButton::Left,
                pressed: true,
                at: Point::new(500.0, 400.0),
            });
        }
        picture_in(&mut screen);
        screen.tick(Duration::from_secs(10));
        assert_eq!(intro_picture(&screen), Some(8200), "held, it waits");
        if let_go {
            screen.release_keys();
        } else {
            screen.cancel_pointer();
        }
        screen.tick(Duration::from_secs(2));
        screen.tick(SECOND);
        assert_eq!(intro_picture(&screen), Some(8201), "let go: {let_go}");
    }
}

#[test]
fn without_dialogs_the_text_is_passed_over() {
    let store = MemoryPilots::new();
    let mut screen = appeared(
        AppScreen::new(intro_data()).with_pilots(Some(keeper(&store)), Rc::new(MonoMetrics)),
    );
    create_held(&mut screen, "Ada");
    choose(&mut screen, MenuChoice::EnterShip);
    faded_out(&mut screen);
    for _ in 0..2 {
        picture_in(&mut screen);
        tap(&mut screen, Key::Space);
        screen.tick(SECOND);
    }
    assert_eq!(screen.showing(), Showing::Flight);
    assert!(flight(&screen).seen_intro());
}

#[test]
fn the_intro_pictures_sound_reaches_the_sounds_only_when_the_data_has_it() {
    for sound in [true, false] {
        let store = MemoryPilots::new();
        let mut screen = opening_with(
            &store,
            intro_data_of(INTRO_TEXT, sound),
            Rulebook::default(),
        );
        create_held(&mut screen, "Ada");
        choose(&mut screen, MenuChoice::EnterShip);
        screen.take_sounds();
        faded_out(&mut screen);
        picture_in(&mut screen);
        tap(&mut screen, Key::Space);
        let raised: Vec<Sound> = screen
            .take_sounds()
            .into_iter()
            .filter(|sound| {
                matches!(
                    sound,
                    Sound::Ui(UiSound::IntroPicture(_) | UiSound::IntroPictureHalted(_))
                )
            })
            .collect();
        let want = if sound {
            vec![
                Sound::Ui(UiSound::IntroPicture(INTRO_SOUND)),
                Sound::Ui(UiSound::IntroPictureHalted(INTRO_SOUND)),
            ]
        } else {
            Vec::new()
        };
        assert_eq!(raised, want, "sound {sound}");
    }
}
