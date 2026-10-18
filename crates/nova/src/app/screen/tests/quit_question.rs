//! The window's close and Command-Q: the quit question on the main
//! menu, and quitting at once everywhere else.

use super::*;
use nova_view::ui::yes_no::{NO_ITEM, QUESTION_ITEM, QUIT_QUESTION, YES_ITEM};

/// The router with the main menu over `store`, and dialogs.
fn asking_menu(store: &MemoryPilots) -> AppScreen {
    with_dialogs(data())
        .with_pilots(Some(keeper(store)), Rc::new(MonoMetrics))
        .appeared()
}

/// Clicks item `item` of the open quit question.
fn answer(screen: &mut AppScreen, item: usize) {
    let at = screen
        .question()
        .expect("asked")
        .dialog()
        .item_bounds(item)
        .expect("the item")
        .center();
    click_at(screen, at);
}

#[test]
fn closing_the_main_menu_asks_str_140_10_and_keeps_the_window_open() {
    let store = MemoryPilots::new();
    let mut screen = asking_menu(&store);
    assert!(!screen.close_requested(), "kept open");
    assert_eq!(screen.showing(), Showing::Question);
    assert_eq!(screen.now_showing(), Some(Showing::Question));
    let question = screen.question().expect("asked");
    assert_eq!(question.question(), QUIT_QUESTION);
    assert_eq!(
        question
            .dialog()
            .text(QUESTION_ITEM)
            .expect("the question")
            .lines()
            .join(" "),
        QUIT_QUESTION
    );
    let texts: Vec<String> = drawn(&screen)
        .iter()
        .filter_map(|command| match command {
            DrawCommand::Text { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect();
    assert!(
        texts
            .iter()
            .any(|text| QUIT_QUESTION.starts_with(text.as_str())),
        "drawn over the menu: {texts:?}"
    );
    assert!(!screen.take_quit());
    assert_eq!(screen.take_sounds(), [], "no sound");
}

#[test]
fn no_closes_the_question_and_stays() {
    let store = MemoryPilots::new();
    let mut screen = asking_menu(&store);
    screen.close_requested();
    answer(&mut screen, NO_ITEM);
    assert_eq!(screen.showing(), Showing::MainMenu);
    assert!(screen.question().is_none());
    screen.tick(QUIT_DELAY * 2);
    assert!(!screen.take_quit());
    assert!(!screen.close_requested(), "asked again");
}

#[test]
fn yes_by_a_click_or_return_quits_at_once() {
    let store = MemoryPilots::new();
    let mut screen = asking_menu(&store);
    screen.close_requested();
    answer(&mut screen, YES_ITEM);
    assert!(screen.take_quit(), "no delay");
    assert!(screen.question().is_none());
    let mut screen = asking_menu(&store);
    screen.close_requested();
    screen.input(&key(Key::Enter, true));
    assert!(screen.take_quit());
    assert_eq!(screen.take_sounds(), [], "silently");
}

#[test]
fn escape_and_other_keys_leave_the_question_open() {
    let store = MemoryPilots::new();
    let mut screen = asking_menu(&store);
    screen.close_requested();
    for input in [
        key(Key::Escape, true),
        key(Key::Char('n'), true),
        Input::Text('q'),
    ] {
        assert_eq!(screen.input(&input), ScreenAction::None, "{input:?}");
    }
    assert_eq!(screen.showing(), Showing::Question);
    assert!(!screen.take_quit());
}

#[test]
fn asking_cancels_the_menus_press() {
    let store = MemoryPilots::new();
    let mut screen = asking_menu(&store);
    let at = screen
        .main_menu()
        .expect("a main menu")
        .press_point(MenuChoice::NewPilot);
    screen.input(&Input::PointerButton {
        button: MouseButton::Left,
        pressed: true,
        at,
    });
    screen.take_sounds();
    screen.close_requested();
    answer(&mut screen, NO_ITEM);
    screen.input(&Input::PointerButton {
        button: MouseButton::Left,
        pressed: false,
        at,
    });
    assert_eq!(screen.showing(), Showing::MainMenu, "no New Pilot");
    assert_eq!(screen.take_sounds(), [], "the press was dropped");
}

/// Dialogs with every template but the Yes/No dialog.
struct NoYesNo;

impl DialogResources for NoYesNo {
    fn dialog_template(&self, id: i16) -> Result<DialogTemplate, String> {
        if id == YES_NO_DIALOG {
            return Err("no DLOG 3002".to_owned());
        }
        Dialogs.dialog_template(id)
    }
}

#[test]
fn without_the_yes_no_dialog_a_close_quits_at_once() {
    let store = MemoryPilots::new();
    let mut screen = menu(&store);
    assert!(screen.close_requested(), "no dialogs");
    assert_eq!(screen.showing(), Showing::MainMenu);
    let mut screen = AppScreen::new(data())
        .with_dialogs(Rc::new(NoYesNo), Rc::new(MonoMetrics))
        .with_pilots(Some(keeper(&store)), Rc::new(MonoMetrics))
        .appeared();
    assert!(screen.close_requested());
}

#[test]
fn away_from_the_main_menu_alone_a_close_quits_at_once() {
    let store = MemoryPilots::new();
    // Under a dialog over the menu.
    let mut screen = asking_menu(&store);
    open_prefs(&mut screen);
    assert!(screen.close_requested(), "under the Preferences");
    // With the question already open.
    let mut screen = asking_menu(&store);
    screen.close_requested();
    assert!(screen.close_requested(), "a second close");
    // While Quit's delay runs.
    let mut screen = asking_menu(&store);
    choose(&mut screen, MenuChoice::Quit);
    assert!(screen.close_requested(), "quitting already");
    // In flight, and on the developer's sides.
    let mut screen = asking_menu(&store);
    create(&mut screen, "Ada");
    assert_eq!(screen.showing(), Showing::Flight);
    assert!(screen.close_requested(), "in flight");
    let mut screen = asking_menu(&store);
    screen.input(&key(Key::Tab, true));
    assert_eq!(screen.showing(), Showing::ShipBrowser);
    assert!(screen.close_requested(), "on the ship browser");
}

#[test]
fn a_close_while_a_keyed_click_plays_asks_once_the_click_ends() {
    let store = MemoryPilots::new();
    let mut screen = asking_menu(&store);
    // E, refused, leaves the main screen as it was.
    screen.input(&Input::Text('e'));
    assert!(!screen.close_requested(), "kept open while the click plays");
    assert_eq!(screen.showing(), Showing::MainMenu, "nothing asked yet");
    screen.keyed_click_ended();
    assert_eq!(screen.showing(), Showing::Question);
    assert!(!screen.take_quit());
    assert_eq!(
        screen.take_sounds(),
        [
            Sound::Ui(UiSound::ButtonKeyed),
            Sound::Ui(UiSound::EnterRefused)
        ],
        "the question itself is silent"
    );
    answer(&mut screen, YES_ITEM);
    assert!(screen.take_quit());
}

#[test]
fn a_close_held_behind_a_click_waits_in_its_place_among_the_inputs() {
    let store = MemoryPilots::new();
    let mut screen = asking_menu(&store);
    // E, then A typed during its click: the close after A waits for A's
    // click too, and then lands on the About dialog A opened.
    screen.input(&Input::Text('e'));
    screen.input(&key(Key::Char('a'), true));
    assert!(!screen.close_requested());
    screen.keyed_click_ended();
    assert_eq!(screen.showing(), Showing::MainMenu, "A's click plays");
    screen.keyed_click_ended();
    assert!(screen.about().is_some(), "A's choice ran");
    assert!(screen.question().is_none(), "dropped under the dialog");
    assert!(!screen.take_quit());
}

#[test]
fn a_close_held_behind_a_click_that_opens_a_dialog_is_dropped() {
    let store = MemoryPilots::new();
    let mut screen = asking_menu(&store);
    screen.input(&Input::Text('n'));
    assert!(!screen.close_requested());
    screen.keyed_click_ended();
    assert!(screen.new_pilot().is_some(), "N's choice ran");
    assert!(screen.question().is_none(), "nothing asked");
    assert!(!screen.take_quit(), "and no quit");
    // A close under the dialog now quits at once, as before.
    assert!(screen.close_requested());
}

#[test]
fn a_close_held_behind_quits_click_leaves_the_quit_to_its_delay() {
    let store = MemoryPilots::new();
    let mut screen = asking_menu(&store);
    screen.input(&Input::Text('q'));
    assert!(!screen.close_requested());
    screen.keyed_click_ended();
    assert!(screen.question().is_none());
    assert!(!screen.take_quit(), "not at once");
    screen.tick(QUIT_DELAY);
    assert!(screen.take_quit(), "after the delay");
}

#[test]
fn two_closes_held_behind_a_click_ask_once() {
    let store = MemoryPilots::new();
    let mut screen = asking_menu(&store);
    screen.input(&Input::Text('e'));
    assert!(!screen.close_requested());
    assert!(!screen.close_requested());
    screen.keyed_click_ended();
    assert_eq!(screen.showing(), Showing::Question);
    assert!(!screen.take_quit(), "the question drops the second");
}

#[test]
fn without_the_yes_no_dialog_a_close_held_behind_a_click_quits_once_it_ends() {
    let store = MemoryPilots::new();
    let mut screen = menu(&store);
    screen.input(&Input::Text('e'));
    assert!(!screen.close_requested());
    assert!(!screen.take_quit());
    screen.keyed_click_ended();
    assert!(screen.take_quit());
}

#[test]
fn command_q_quits_at_once_silently_wherever_it_is_pressed() {
    let store = MemoryPilots::new();
    let mut screen = asking_menu(&store);
    screen.take_sounds();
    screen.input(&key(Key::Command, true));
    assert_eq!(screen.input(&key(Key::Char('q'), true)), ScreenAction::Quit);
    assert_eq!(screen.take_sounds(), [], "no click");
    // A plain Q is the menu's Quit, which clicks and waits.
    let mut screen = asking_menu(&store);
    screen.take_sounds();
    assert_eq!(screen.input(&key(Key::Char('q'), true)), ScreenAction::None);
    // Command-Q in flight, and over the question.
    let mut screen = asking_menu(&store);
    create(&mut screen, "Ada");
    screen.input(&key(Key::Command, true));
    assert_eq!(screen.input(&key(Key::Char('q'), true)), ScreenAction::Quit);
    let mut screen = asking_menu(&store);
    screen.close_requested();
    screen.input(&key(Key::Command, true));
    assert_eq!(screen.input(&key(Key::Char('q'), true)), ScreenAction::Quit);
    // Its release quits nothing.
    let mut screen = asking_menu(&store);
    screen.input(&key(Key::Command, true));
    assert_eq!(
        screen.input(&key(Key::Char('q'), false)),
        ScreenAction::None
    );
}

#[test]
fn the_question_stops_the_menu_moving_as_any_overlay_does() {
    let store = MemoryPilots::new();
    let mut screen = asking_menu(&store);
    screen.close_requested();
    screen.tick(nova_sim::mission::screens::mac_ticks(10));
    assert_eq!(
        screen.main_menu().and_then(MainMenu::logo_frame),
        Some(0),
        "frozen"
    );
}
