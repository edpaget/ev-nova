use super::live_keys::texted_and_held;
use super::map_wait_keys::{offered_docked, offered_over_flight};
use super::*;
use nova_view::flight::view::MAP_KEY;
use nova_view::spaceport::offer::ACCEPT_ITEM;
use nova_view::ui::player_info::PLAYER_INFO_KEY;

// The menu chords typed during a press a dialog holds keys back for act
// at its release, after its replay (the `held_chords` module): wiring
// through the real router, views and flight. Which keys a dialog holds
// back, and when its replay runs, is nova-view's (`ui::held_keys`).

fn command(pressed: bool) -> Input {
    key(Key::MetaLeft, pressed)
}

fn left(pressed: bool, at: Point) -> Input {
    Input::PointerButton {
        button: MouseButton::Left,
        pressed,
        at,
    }
}

/// Command down, then `chord`'s press and release.
fn chord(screen: &mut AppScreen, chord: Key) -> [ScreenAction; 3] {
    [
        screen.input(&command(true)),
        screen.input(&key(chord, true)),
        screen.input(&key(chord, false)),
    ]
}

/// Where `item` of `dialog`, drawn over the side shown, is on the screen.
fn item_on_screen(screen: &AppScreen, dialog: &nova_view::ui::Dialog, item: usize) -> Point {
    let at = dialog.item_bounds(item).expect("the item").center();
    let offset = screen.overlay_offset();
    Point::new(at.x + offset.x, at.y + offset.y)
}

/// The mission text's Okay, held since [`texted_and_held`].
fn okay(screen: &AppScreen) -> Point {
    let text = screen.flight_mission_text().expect("shown").dialog();
    item_on_screen(screen, text, nova_view::ui::desc::DONE_ITEM)
}

/// The spaceport's offer's Accept.
fn docked_accept(screen: &AppScreen) -> Point {
    let dialog = spaceport(screen)
        .offer_dialog()
        .and_then(|offer| offer.dialog())
        .expect("laid out");
    item_on_screen(screen, dialog, ACCEPT_ITEM)
}

/// The offer over flight's Accept.
fn flight_accept(screen: &AppScreen) -> Point {
    let dialog = screen
        .offer()
        .and_then(|offer| offer.dialog())
        .expect("laid out");
    item_on_screen(screen, dialog, ACCEPT_ITEM)
}

/// Off every item of a dialog.
const OFF: Point = Point::new(1.0, 1.0);

#[test]
fn command_q_during_a_click_held_on_a_texts_okay_quits_at_the_release() {
    let mut screen = texted_and_held();
    let at = okay(&screen);
    assert_eq!(chord(&mut screen, Key::KeyQ), [ScreenAction::None; 3]);
    assert!(!screen.take_quit(), "not at the press");
    assert_eq!(screen.showing(), Showing::MissionText, "still shown");
    screen.input(&command(false));
    assert_eq!(screen.input(&left(false, at)), ScreenAction::Quit);
}

#[test]
fn command_q_during_a_click_held_on_a_docked_offers_accept_quits_after_it_answers() {
    let mut screen = offered_docked();
    let at = docked_accept(&screen);
    screen.input(&left(true, at));
    assert_eq!(chord(&mut screen, Key::KeyQ), [ScreenAction::None; 3]);
    assert!(!screen.take_quit());
    assert!(
        spaceport(&screen).offer_dialog().is_some(),
        "not answered yet"
    );
    assert_eq!(screen.input(&left(false, at)), ScreenAction::Quit);
    assert!(
        spaceport(&screen).offer_dialog().is_none(),
        "answered first"
    );
}

/// Accepting [`offered_docked`]'s mission shows no text: a text would
/// flush the held chord (`_ShowTextDialog` @0x5ae67), which the tests
/// above and below rely on it not doing. Its data has no mission 130, so
/// the accept answers the offer and queues nothing.
#[test]
fn accepting_the_docked_offer_shows_no_text() {
    let mut screen = offered_docked();
    let at = docked_accept(&screen);
    screen.input(&left(true, at));
    screen.input(&left(false, at));
    assert!(spaceport(&screen).offer_dialog().is_none(), "answered");
    assert!(spaceport(&screen).mission_text().is_none(), "no text");
}

#[test]
fn command_comma_during_a_click_held_on_a_docked_offers_accept_opens_preferences_at_the_release() {
    let mut screen = offered_docked();
    let at = docked_accept(&screen);
    screen.input(&left(true, at));
    chord(&mut screen, PREFERENCES_KEY);
    assert!(screen.preferences().is_none(), "not at the press");
    screen.input(&command(false));
    screen.input(&left(false, at));
    assert!(screen.preferences().is_some(), "open after the release");
    assert!(
        spaceport(&screen).offer_dialog().is_none(),
        "after the answer"
    );
}

#[test]
fn a_return_held_before_command_comma_answers_the_offer_first() {
    let mut screen = offered_docked();
    let at = docked_accept(&screen);
    screen.input(&left(true, at));
    screen.input(&key(Key::Enter, true));
    screen.input(&key(Key::Enter, false));
    chord(&mut screen, PREFERENCES_KEY);
    screen.input(&command(false));
    screen.input(&left(false, OFF));
    assert!(
        spaceport(&screen).offer_dialog().is_none(),
        "answered by the Return"
    );
    assert!(screen.preferences().is_some(), "then the Preferences");
}

/// The Preferences' own state, for comparing two runs without pinning
/// their rules: open or not, what they drew and what they changed.
fn preferences_state(screen: &mut AppScreen) -> (bool, Vec<DrawCommand>, Option<nova_view::Prefs>) {
    let drawn = screen.preferences().map_or_else(Vec::new, |prefs| {
        let mut list = DrawList::new();
        prefs.draw(&mut list);
        list.iter().cloned().collect()
    });
    (screen.preferences().is_some(), drawn, screen.take_prefs())
}

/// The Preferences flush nothing as they open (`_RunPrefsDialog`
/// @0xcd401, a Cocoa modal session), so a key queued after Command-comma
/// reaches them, as a Return typed into them does.
#[test]
fn a_return_held_after_command_comma_goes_to_the_preferences() {
    let mut screen = offered_docked();
    let at = docked_accept(&screen);
    screen.input(&left(true, at));
    chord(&mut screen, PREFERENCES_KEY);
    screen.input(&command(false));
    screen.input(&key(Key::Enter, true));
    screen.input(&key(Key::Enter, false));
    screen.input(&left(false, OFF));
    assert!(spaceport(&screen).offer_dialog().is_some(), "not answered");

    let mut typed = offered_docked();
    chord(&mut typed, PREFERENCES_KEY);
    typed.input(&command(false));
    assert!(typed.preferences().is_some(), "opened");
    typed.input(&key(Key::Enter, true));
    typed.input(&key(Key::Enter, false));
    assert_eq!(
        preferences_state(&mut screen),
        preferences_state(&mut typed),
        "as a Return typed into the Preferences leaves them"
    );
}

#[test]
fn player_info_opened_before_a_held_command_q_drops_it() {
    let mut screen = offered_over_flight();
    let at = flight_accept(&screen);
    screen.input(&left(true, at));
    screen.input(&key(PLAYER_INFO_KEY, true));
    screen.input(&key(PLAYER_INFO_KEY, false));
    chord(&mut screen, Key::KeyQ);
    screen.input(&command(false));
    assert_eq!(screen.input(&left(false, OFF)), ScreenAction::None);
    assert!(!screen.take_quit(), "flushed");
    assert_eq!(screen.showing(), Showing::PlayerInfo);
}

#[test]
fn the_map_opened_before_a_held_command_q_drops_it() {
    let mut screen = offered_over_flight();
    let at = flight_accept(&screen);
    screen.input(&left(true, at));
    screen.input(&key(MAP_KEY, true));
    chord(&mut screen, Key::KeyQ);
    screen.input(&command(false));
    assert_eq!(screen.input(&left(false, OFF)), ScreenAction::None);
    assert!(!screen.take_quit(), "flushed");
    assert!(flight(&screen).map_open(), "the dialog's map");
}

#[test]
fn a_held_command_q_whose_press_is_released_outside_the_game_quits_on_the_next_tick() {
    let mut screen = texted_and_held();
    chord(&mut screen, Key::KeyQ);
    screen.cancel_pointer();
    assert!(!screen.take_quit(), "not at the cancel");
    screen.tick(TICK);
    assert!(screen.take_quit(), "on the next tick");
}

#[test]
fn quit_by_its_key_equivalent_waits_for_the_held_click() {
    let mut screen = texted_and_held();
    let at = okay(&screen);
    screen.input(&command(true));
    assert_eq!(screen.menu_command(MenuCommand::Quit), ScreenAction::None);
    assert!(!screen.take_quit());
    assert_eq!(screen.showing(), Showing::MissionText);
    assert_eq!(screen.input(&left(false, at)), ScreenAction::Quit);
}

/// Command let go of and pressed again after a held Command-comma, then
/// the keys let go of with the window: the replay lets go of the second
/// press, so Q alone doesn't quit afterwards.
#[test]
fn a_key_pressed_after_a_held_chord_and_let_go_of_with_the_window_isnt_left_down() {
    let mut screen = offered_docked();
    let at = docked_accept(&screen);
    screen.input(&left(true, at));
    chord(&mut screen, PREFERENCES_KEY);
    screen.input(&command(false));
    screen.input(&command(true));
    screen.release_keys();
    screen.input(&left(false, OFF));
    assert!(screen.preferences().is_some(), "the chord ran");
    assert_eq!(screen.input(&key(Key::KeyQ, true)), ScreenAction::None);
    assert!(!screen.take_quit());
}

/// A held Command-Q whose click answers an offer that shows a text: the
/// text flushes the queue as it opens (`_ShowTextDialog` @0x5ae67), so
/// the chord's key-down is dropped and nothing quits.
#[test]
fn an_answer_that_shows_a_text_drops_the_held_command_q() {
    let mut screen = super::missions::hailing_briefer(Rc::new(super::missions::PictureDialogs));
    let at = flight_accept(&screen);
    screen.input(&left(true, at));
    assert_eq!(chord(&mut screen, Key::KeyQ), [ScreenAction::None; 3]);
    screen.input(&command(false));
    assert_eq!(screen.input(&left(false, at)), ScreenAction::None);
    screen.tick(TICK);
    assert!(!screen.take_quit(), "flushed");
    assert_eq!(screen.showing(), Showing::MissionText, "the brief");
}

/// Over flight's offer, released off its buttons, the offer stays in
/// front and takes the held Command-comma, greyed there. What follows it
/// replays: Command down again, then Return, which accepts and shows the
/// brief, a text that flushes the rest as it opens (`_ShowTextDialog`
/// @0x5ae67). Command's queued release still goes.
#[test]
fn a_press_replayed_before_a_flush_still_gets_its_queued_release() {
    let mut screen = super::missions::hailing_briefer(Rc::new(super::missions::PictureDialogs));
    let at = flight_accept(&screen);
    screen.input(&left(true, at));
    chord(&mut screen, PREFERENCES_KEY);
    screen.input(&command(false));
    screen.input(&command(true));
    screen.input(&key(Key::Enter, true));
    screen.input(&command(false));
    screen.input(&left(false, OFF));
    assert_eq!(screen.showing(), Showing::MissionText, "the brief");
    assert_eq!(
        screen.input(&key(Key::KeyQ, true)),
        ScreenAction::None,
        "Command let go of"
    );
}

/// As [`a_press_replayed_before_a_flush_still_gets_its_queued_release`],
/// but the keys are let go of with the window before Command's release:
/// the let-go still runs past the flush.
#[test]
fn a_let_go_after_a_flush_still_lets_go_of_the_presses_replayed() {
    let mut screen = super::missions::hailing_briefer(Rc::new(super::missions::PictureDialogs));
    let at = flight_accept(&screen);
    screen.input(&left(true, at));
    chord(&mut screen, PREFERENCES_KEY);
    screen.input(&command(false));
    screen.input(&command(true));
    screen.input(&key(Key::Enter, true));
    screen.release_keys();
    screen.tick(TICK);
    assert_eq!(screen.showing(), Showing::MissionText, "the brief");
    assert_eq!(
        screen.input(&key(Key::KeyQ, true)),
        ScreenAction::None,
        "Command let go of"
    );
}
