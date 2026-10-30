//! The router's away state, the original's `_inBackground` (see
//! `background`): flight, the dialogs and the input pause while the app
//! is in the background; the main menu idles there.

use super::*;

use nova_sim::Rulebook;
use nova_sim::mission::screens::mac_ticks;
use nova_view::fade::{DEATH_FADE_FOR, ScreenFade};

use super::menu::menu_with_dialogs;
use super::menu_commands::{entering, quitting, returning, starting_up};
use super::missions::filmed;
use super::wheel::{offering, text, wheel_over_text};

/// Flight entered from the developer's side, the ship moving with no key
/// held.
fn coasting() -> AppScreen {
    let mut screen = AppScreen::new(data());
    fly(&mut screen);
    screen.input(&key(Key::Up, true));
    screen.tick(TICK * 10);
    screen.input(&key(Key::Up, false));
    screen.tick(TICK);
    assert_ne!(
        ship(&screen).velocity,
        nova_sim::ShipState::default().velocity
    );
    screen
}

/// Sends the app to the background as the app does: its press and its
/// keys let go of first.
pub(super) fn switch_out(screen: &mut AppScreen) {
    screen.cancel_pointer();
    screen.release_keys();
    screen.focus_lost();
}

/// The `SwitchedIn` sounds among `screen`'s.
fn switched_in(screen: &mut AppScreen) -> Vec<Duration> {
    screen
        .take_sounds()
        .into_iter()
        .filter_map(|sound| match sound {
            Sound::Ui(UiSound::SwitchedIn { away }) => Some(away),
            _ => None,
        })
        .collect()
}

/// `_PlayGame` skips its whole frame while `_inBackground` is set
/// (@0x45b2d-0x45b3e).
#[test]
fn away_flight_doesnt_tick() {
    let mut screen = coasting();
    let before = ship(&screen);
    switch_out(&mut screen);
    for _ in 0..10 {
        screen.tick(Duration::from_millis(100));
    }
    assert_eq!(ship(&screen), before, "the ship stands still");
    assert_eq!(screen.showing(), Showing::Flight);
}

/// `_MyModalDialog` doesn't call the dialog's filter while
/// `_inBackground` is set (@0x59fcc-0x59fee), null events included.
#[test]
fn away_a_dialog_over_flight_doesnt_tick() {
    let (mut screen, reel) = filmed(0, "Job.", 0, Prefs::default());
    create(&mut screen, "Ada");
    screen.show_over_flight(vec![DescShown {
        text: "Well done.".to_owned(),
        graphic: None,
        movie: nova_sim::DescMovie::from_desc("Text.mov", 0),
    }]);
    let started = reel.calls();
    assert_eq!(started, [Call::Start("Text.mov".into())]);
    switch_out(&mut screen);
    for _ in 0..4 {
        screen.tick(Duration::from_millis(16));
    }
    assert_eq!(reel.calls(), started, "its movie waits");
    screen.focus_gained();
    screen.tick(Duration::from_millis(16));
    assert_eq!(
        reel.calls(),
        started,
        "the switch-in's tick is the background's"
    );
    screen.tick(Duration::from_millis(16));
    assert_eq!(
        reel.calls(),
        [Call::Start("Text.mov".into()), Call::Advance],
        "then it plays on"
    );
}

#[test]
fn away_a_key_and_the_wheel_do_nothing() {
    let mut screen =
        offering(Rulebook::default().with_override(RuleKey::DialogTextWheel, RuleSource::Bible));
    switch_out(&mut screen);
    assert_eq!(screen.input(&key(Key::Down, true)), ScreenAction::None);
    screen.input(&key(Key::Down, false));
    wheel_over_text(&mut screen);
    assert_eq!(text(&screen).first(), 0, "dropped");
    screen.focus_gained();
    screen.input(&key(Key::Down, true));
    assert_eq!(text(&screen).first(), 1, "back, Down scrolls");
}

/// The first idle after a resume reads `GetMouse`: where the pointer is
/// now, wherever its moves went while away.
#[test]
fn back_the_last_pointer_move_reaches_the_main_menu() {
    let store = MemoryPilots::new();
    let mut screen = menu(&store);
    let point =
        |screen: &AppScreen, choice| screen.main_menu().expect("a main menu").press_point(choice);
    let (new_pilot, quit) = (
        point(&screen, MenuChoice::NewPilot),
        point(&screen, MenuChoice::Quit),
    );
    switch_out(&mut screen);
    screen.input(&Input::PointerMoved(new_pilot));
    screen.input(&Input::PointerMoved(quit));
    screen.tick(mac_ticks(20));
    assert_eq!(rollover(&screen), Some((6, 32)), "not yet");
    screen.focus_gained();
    assert!(!screen.take_quit(), "over Quit, not on it");
    screen.tick(mac_ticks(1));
    screen.tick(mac_ticks(20));
    assert_eq!(rollover(&screen), Some((2, 32)), "over Quit, the last move");
}

#[test]
fn away_a_pointer_leaving_is_kept_too() {
    let store = MemoryPilots::new();
    let mut screen = menu(&store);
    let quit = screen
        .main_menu()
        .expect("a main menu")
        .press_point(MenuChoice::Quit);
    screen.input(&Input::PointerMoved(quit));
    switch_out(&mut screen);
    screen.input(&Input::PointerLeft);
    screen.focus_gained();
    assert!(!screen.take_quit());
    screen.tick(mac_ticks(1));
    screen.tick(mac_ticks(20));
    assert_eq!(rollover(&screen), Some((6, 32)), "over no button");
}

fn rollover(screen: &AppScreen) -> Option<(u16, u8)> {
    screen.main_menu().expect("a main menu").rollover()
}

/// `_MainScreenRolloverIdle` shows the none frame at 0x20 while
/// `_inBackground` is set (@0x16593-0x165bc).
#[test]
fn away_the_main_menu_idles_in_the_background() {
    let store = MemoryPilots::new();
    let mut screen = menu(&store);
    let quit = screen
        .main_menu()
        .expect("a main menu")
        .press_point(MenuChoice::Quit);
    screen.input(&Input::PointerMoved(quit));
    screen.tick(mac_ticks(4));
    assert_eq!(rollover(&screen), Some((2, 16)));
    switch_out(&mut screen);
    screen.tick(Duration::ZERO);
    assert_eq!(rollover(&screen), Some((6, 32)), "the none frame in full");
}

/// No dialog loop idles the main screen (`_MyModalDialog` calls no
/// filter in the background, and the main screen's idle is not its).
#[test]
fn away_the_main_menu_under_the_preferences_doesnt_idle() {
    let store = MemoryPilots::new();
    let mut screen = menu_with_dialogs(&store);
    let quit = screen
        .main_menu()
        .expect("a main menu")
        .press_point(MenuChoice::Quit);
    screen.input(&Input::PointerMoved(quit));
    screen.tick(mac_ticks(4));
    open_prefs(&mut screen);
    switch_out(&mut screen);
    screen.tick(mac_ticks(10));
    assert_eq!(rollover(&screen), Some((2, 16)), "as it stood");
}

/// Under its appear the menu waits unseen: nothing idles it, away or not,
/// and it stands as it stood, its rollover not put on the none frame in
/// full.
#[test]
fn away_the_main_menu_under_its_appear_doesnt_idle() {
    let store = MemoryPilots::new();
    let mut screen = with_dialogs(data()).with_pilots(Some(keeper(&store)), Rc::new(MonoMetrics));
    assert!(screen.appear_holds_menu());
    let quit = screen
        .main_menu()
        .expect("a main menu")
        .press_point(MenuChoice::Quit);
    screen.input(&Input::PointerMoved(quit));
    let (shown, picture) = (rollover(&screen), drawn(&screen));
    assert_eq!(shown, Some((6, 0)));
    switch_out(&mut screen);
    screen.tick(mac_ticks(20));
    assert_eq!(rollover(&screen), shown, "as it stood");
    assert_eq!(drawn(&screen), picture, "its slides too");
}

/// Under an entry the menu isn't ticked, away or not.
#[test]
fn away_the_main_menu_under_an_entry_doesnt_idle() {
    let store = MemoryPilots::new();
    let mut screen = menu_with_dialogs(&store);
    create(&mut screen, "Ada");
    escape_to_menu(&mut screen);
    let quit = screen
        .main_menu()
        .expect("a main menu")
        .press_point(MenuChoice::Quit);
    screen.input(&Input::PointerMoved(quit));
    screen.tick(mac_ticks(4));
    choose(&mut screen, MenuChoice::EnterShip);
    assert!(screen.entering());
    let (shown, picture) = (rollover(&screen), drawn(&screen));
    assert_ne!(shown, Some((6, 32)), "not already the none frame in full");
    switch_out(&mut screen);
    screen.tick(mac_ticks(20));
    assert_eq!(rollover(&screen), shown, "as it stood");
    assert_eq!(drawn(&screen), picture, "its slides too");
}

/// The fade back from black after a death with no main menu.
fn fading_in() -> AppScreen {
    let mut screen = with_dialogs(data());
    screen.fade_in = Some(ScreenFade::from_black(DEATH_FADE_FOR));
    screen
}

/// How far a fade or an entry has come, by what it draws.
fn picture(screen: &AppScreen) -> String {
    format!("{:?}", drawn(screen))
}

/// How far Quit's delay has come. The menu under it idles away, so its
/// picture changes.
fn quit_left(screen: &AppScreen) -> String {
    format!("{:?}", screen.quit_in)
}

/// The appear, the entry, the fade back from black and Quit's delay are
/// synchronous loops in the original, and the engine's tick-driven forms
/// of them wait while away (see `background`): well past their ends they
/// haven't moved, through the switch-in's tick too, and go on after it.
#[test]
fn away_the_fades_the_entry_and_quits_delay_wait_for_the_return() {
    type Row = (&'static str, fn() -> AppScreen, fn(&AppScreen) -> String);
    let rows: [Row; 5] = [
        ("the fade back from black", fading_in, picture),
        ("the entry", entering, picture),
        ("the start-up appear", starting_up, picture),
        ("the return's appear", returning, picture),
        ("Quit's delay", quitting, quit_left),
    ];
    for (name, make, progress) in rows {
        let mut screen = make();
        let before = progress(&screen);
        switch_out(&mut screen);
        for _ in 0..8 {
            screen.tick(DEATH_FADE_FOR);
        }
        assert_eq!(progress(&screen), before, "{name}: away");
        assert!(!screen.take_quit(), "{name}: away");
        screen.focus_gained();
        screen.tick(DEATH_FADE_FOR);
        assert_eq!(progress(&screen), before, "{name}: the switch-in's tick");
        assert!(!screen.take_quit(), "{name}: the switch-in's tick");
        screen.tick(DEATH_FADE_FOR);
        assert_ne!(progress(&screen), before, "{name}: back");
        assert_eq!(screen.take_quit(), name == "Quit's delay", "{name}: back");
    }
}

#[test]
fn back_the_first_tick_says_how_long_the_app_was_away_and_flight_ticks_after() {
    let mut screen = coasting();
    let before = ship(&screen);
    switch_out(&mut screen);
    screen.tick(Duration::from_millis(300));
    screen.tick(Duration::from_millis(200));
    assert_eq!(switched_in(&mut screen), []);
    screen.focus_gained();
    screen.tick(Duration::from_millis(100));
    assert_eq!(switched_in(&mut screen), [Duration::from_millis(600)]);
    assert_eq!(ship(&screen), before, "the switch-in's tick is away time");
    screen.tick(TICK);
    assert_ne!(ship(&screen), before, "then flight goes on");
    assert_eq!(switched_in(&mut screen), [], "once");
}

#[test]
fn going_away_again_before_the_first_tick_keeps_counting() {
    let mut screen = coasting();
    let before = ship(&screen);
    switch_out(&mut screen);
    screen.tick(Duration::from_millis(300));
    screen.focus_gained();
    switch_out(&mut screen);
    screen.tick(Duration::from_millis(200));
    assert_eq!(ship(&screen), before, "still away");
    screen.focus_gained();
    screen.tick(Duration::from_millis(100));
    assert_eq!(switched_in(&mut screen), [Duration::from_millis(600)]);
}

#[test]
fn coming_to_the_front_without_going_away_says_nothing() {
    let mut screen = coasting();
    screen.focus_gained();
    let before = ship(&screen);
    screen.tick(TICK);
    assert_eq!(switched_in(&mut screen), []);
    assert_ne!(ship(&screen), before, "flight ticks");
}
