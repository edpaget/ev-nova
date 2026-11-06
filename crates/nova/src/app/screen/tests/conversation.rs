//! The landed stellar's conversation over the spaceport: opened as the
//! spaceport opens, each digit's choice sent back, Escape held off while
//! choices are pending, and the ended page closed back to the spaceport.
//! What the conversation plays is the session's (`nova_sim`'s
//! `session::apply::conversation`), here scripted by the dialog port's
//! fake.

use std::collections::BTreeMap;
use std::sync::Arc;

use nova_data::{DialogPlace, Dialogs as StoredDialogs};
use nova_sim::dialog::fake::{DialogCall, FakeChoice, FakeDialog, FakeKnot, FakeLoader};

use super::*;

const STORY: &str = "dock story";

/// An offer: a line and two choices, `Ask` leading on to another page and
/// `Leave` ending it.
fn loader() -> FakeLoader {
    let offer = FakeKnot::new("offer")
        .line("A job?", &[])
        .choice(
            FakeChoice::sticky("Ask", &[])
                .line("Two gates.", &[])
                .then("route"),
        )
        .choice(FakeChoice::sticky("Leave", &[]).line("Bye.", &[]));
    let route = FakeKnot::new("route")
        .line("Either will do.", &[])
        .choice(FakeChoice::sticky("Take it", &[]).line("Go.", &[]));
    FakeLoader::default().story(STORY, FakeDialog::default().knot(offer).knot(route))
}

/// [`data`], where the planet the ship lands on names the offer.
fn talking_data() -> Rc<GameData> {
    let data = Rc::into_inner(data()).expect("one owner");
    Rc::new(data.with_dialogs(StoredDialogs {
        compiled: BTreeMap::from([("dialog/dock.ink".to_owned(), Arc::from(STORY))]),
        stellars: BTreeMap::from([(
            nova_data::StellarId(128),
            DialogPlace {
                file: "dialog/dock.ink".to_owned(),
                knot: "offer".to_owned(),
            },
        )]),
    }))
}

/// A router over `data` with the interface's dialogs and metrics, its
/// flights playing conversations on `loader`.
fn talking(data: Rc<GameData>, loader: &FakeLoader) -> AppScreen {
    with_dialogs(data).with_dialog_loader(Rc::new(loader.clone()))
}

/// Every text drawn.
fn texts(screen: &AppScreen) -> Vec<String> {
    drawn(screen)
        .iter()
        .filter_map(|command| match command {
            DrawCommand::Text { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect()
}

fn shows(screen: &AppScreen, text: &str) -> bool {
    texts(screen).iter().any(|drawn| drawn == text)
}

#[test]
fn landing_at_the_stellar_shows_its_lines_and_choices_over_the_spaceport() {
    let loader = loader();
    let mut screen = talking(talking_data(), &loader);
    land(&mut screen);
    assert_eq!(screen.showing(), Showing::Spaceport);
    let texts = texts(&screen);
    let spaceport_texts: Vec<String> = drawn(spaceport(&screen))
        .iter()
        .filter_map(|command| match command {
            DrawCommand::Text { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(
        texts[texts.len() - 3..],
        [
            "A job?".to_owned(),
            "1. Ask".to_owned(),
            "2. Leave".to_owned()
        ],
        "drawn last, over the spaceport"
    );
    assert!(texts.starts_with(&spaceport_texts), "{texts:?}");
    assert_eq!(loader.log(), [DialogCall::Enter("offer".to_owned())]);
}

#[test]
fn a_digit_shows_the_next_page() {
    let loader = loader();
    let mut screen = talking(talking_data(), &loader);
    land(&mut screen);
    screen.input(&key(Key::Char('1'), true));
    assert!(shows(&screen, "Two gates."));
    assert!(shows(&screen, "Either will do."));
    assert!(shows(&screen, "1. Take it"));
    assert!(!shows(&screen, "A job?"));
    assert_eq!(
        loader.log(),
        [DialogCall::Enter("offer".to_owned()), DialogCall::Choose(0)]
    );
}

#[test]
fn escape_mid_conversation_stays_landed_with_it_open() {
    let mut screen = talking(talking_data(), &loader());
    land(&mut screen);
    for pressed in [true, false] {
        assert_eq!(screen.input(&key(Key::Escape, pressed)), ScreenAction::None);
    }
    assert_eq!(screen.showing(), Showing::Spaceport);
    assert!(shows(&screen, "A job?"));
}

#[test]
fn the_end_closes_back_to_the_spaceport_and_a_second_escape_takes_off() {
    let loader = loader();
    let mut screen = talking(talking_data(), &loader);
    land(&mut screen);
    screen.input(&key(Key::Char('2'), true));
    assert!(shows(&screen, "Bye."));
    assert!(shows(
        &screen,
        nova_view::spaceport::conversation::CLOSE_HINT
    ));
    screen.input(&key(Key::Enter, true));
    assert_eq!(screen.showing(), Showing::Spaceport);
    assert_eq!(
        drawn(&screen),
        drawn(spaceport(&screen)),
        "the spaceport alone"
    );
    screen.input(&key(Key::Enter, false));
    screen.input(&key(Key::Escape, true));
    assert_eq!(screen.showing(), Showing::Flight);
}

#[test]
fn landing_at_a_stellar_with_none_shows_the_spaceport_alone() {
    let loader = loader();
    let mut screen = talking(data(), &loader);
    land(&mut screen);
    assert_eq!(drawn(&screen), drawn(spaceport(&screen)));
    assert_eq!(loader.log(), []);
}

#[test]
fn a_router_with_no_metrics_shows_no_conversation() {
    let loader = loader();
    let mut screen = AppScreen::new(talking_data()).with_dialog_loader(Rc::new(loader.clone()));
    land(&mut screen);
    assert!(!shows(&screen, "A job?"));
    screen.input(&key(Key::Escape, true));
    assert_eq!(screen.showing(), Showing::Flight, "Escape takes off");
}

#[test]
fn taking_off_and_landing_again_opens_it_afresh() {
    let loader = loader();
    let mut screen = talking(talking_data(), &loader);
    land(&mut screen);
    screen.input(&key(Key::Char('1'), true));
    screen.input(&key(Key::Char('1'), true));
    screen.input(&key(Key::Escape, true));
    screen.input(&key(Key::Escape, false));
    assert_eq!(
        screen.showing(),
        Showing::Spaceport,
        "the ended page closed"
    );
    screen.input(&key(Key::Escape, true));
    screen.input(&key(Key::Escape, false));
    assert_eq!(screen.showing(), Showing::Flight);
    screen.tick(TICK * 30);
    land_twice(&mut screen);
    assert_eq!(screen.showing(), Showing::Spaceport);
    assert!(shows(&screen, "A job?"));
}
