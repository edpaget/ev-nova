//! The landed visit's conversations over the spaceport: opened as the
//! spaceport opens, each digit's choice sent back, Escape held off while
//! choices are pending, and the ended page closed to the next one or back
//! to the spaceport.
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
    screen.input(&key(Key::Digit1, true));
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
    screen.input(&key(Key::Digit2, true));
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
    screen.input(&key(Key::Digit1, true));
    screen.input(&key(Key::Digit1, true));
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

/// A router as the new game builds it: no interface file, so the
/// spaceport is its stand-in and the conversation is laid out with the
/// main menu's metrics; its flights play on `loader`. Flight is entered
/// past the menu.
fn without_interface(loader: &FakeLoader) -> AppScreen {
    let mut screen = AppScreen::new(talking_data())
        .with_pilots(None, Rc::new(MonoMetrics))
        .with_dialog_loader(Rc::new(loader.clone()))
        .appeared();
    screen.enter_flight();
    assert_eq!(screen.showing(), Showing::Flight);
    screen
}

#[test]
fn without_an_interface_file_it_is_drawn_over_the_stand_in_and_plays_to_its_close() {
    let mut screen = without_interface(&loader());
    land_twice(&mut screen);
    assert_eq!(screen.showing(), Showing::Spaceport);
    assert_eq!(spaceport(&screen).problem(), Some(NO_INTERFACE));
    let stand_in = texts(&screen)[0].clone();
    assert!(
        stand_in.starts_with("Cannot show the spaceport"),
        "{stand_in}"
    );
    assert_eq!(
        texts(&screen),
        [
            stand_in.clone(),
            "A job?".to_owned(),
            "1. Ask".to_owned(),
            "2. Leave".to_owned()
        ],
        "over the stand-in"
    );
    // The stand-in takes off on Return or Escape; choices pending hold
    // both.
    for k in [Key::Enter, Key::Escape] {
        for pressed in [true, false] {
            assert_eq!(screen.input(&key(k, pressed)), ScreenAction::None);
        }
        assert_eq!(screen.showing(), Showing::Spaceport, "{k:?}");
    }
    assert!(shows(&screen, "A job?"));
    screen.input(&key(Key::Digit1, true));
    assert!(shows(&screen, "1. Take it"));
    screen.input(&key(Key::Digit1, true));
    assert!(shows(&screen, "Go."));
    screen.input(&key(Key::Enter, true));
    screen.input(&key(Key::Enter, false));
    assert_eq!(
        screen.showing(),
        Showing::Spaceport,
        "the close stays landed"
    );
    assert_eq!(texts(&screen), [stand_in], "the stand-in alone");
    screen.input(&key(Key::Escape, true));
    assert_eq!(screen.showing(), Showing::Flight);
}

/// [`talking_data`] with a mission offered at the planet the ship lands
/// on, by the same story's `job` knot.
fn offering_data() -> Rc<GameData> {
    let data = Rc::into_inner(talking_data()).expect("one owner");
    let job = nova_data::StoryMission {
        offer: Some(nova_data::MissionOffer {
            at: nova_data::StellarId(128),
            giver: None,
            dialog: DialogPlace {
                file: "dialog/dock.ink".to_owned(),
                knot: "job".to_owned(),
            },
        }),
        ..nova_data::StoryMission::default()
    };
    Rc::new(data.with_storylines(nova_data::Storylines {
        missions: BTreeMap::from([("job".to_owned(), job)]),
        ..nova_data::Storylines::default()
    }))
}

/// [`loader`]'s story with the mission's offer, `job`, which one choice
/// ends.
fn offering_loader() -> FakeLoader {
    let job = FakeKnot::new("job")
        .line("Work?", &[])
        .choice(FakeChoice::sticky("Yes", &[]).line("Done.", &["outcome: accept"]));
    let offer = FakeKnot::new("offer")
        .line("A job?", &[])
        .choice(FakeChoice::sticky("Leave", &[]).line("Bye.", &[]));
    FakeLoader::default().story(STORY, FakeDialog::default().knot(job).knot(offer))
}

#[test]
fn closing_an_offer_shows_the_stellar_s_own_conversation_next() {
    let loader = offering_loader();
    let mut screen = talking(offering_data(), &loader);
    land(&mut screen);
    assert!(shows(&screen, "Work?"), "the offer first");
    screen.input(&key(Key::Digit1, true));
    assert!(shows(&screen, "Done."));
    screen.input(&key(Key::Enter, true));
    screen.input(&key(Key::Enter, false));
    assert_eq!(screen.showing(), Showing::Spaceport);
    assert!(shows(&screen, "A job?"), "then the stellar's own");
    assert!(shows(&screen, "1. Leave"));
    screen.input(&key(Key::Digit1, true));
    screen.input(&key(Key::Enter, true));
    assert_eq!(
        drawn(&screen),
        drawn(spaceport(&screen)),
        "the last closed: the spaceport alone"
    );
    assert_eq!(
        loader.log(),
        [
            DialogCall::Enter("job".to_owned()),
            DialogCall::Choose(0),
            DialogCall::Enter("offer".to_owned()),
            DialogCall::Choose(0),
        ]
    );
}

/// `data` with a deliverable job: an event that loads ten tons of plate
/// at the first landing, bound for the planet the ship lands on, and
/// completed there by the story's `done` knot.
fn delivering(data: Rc<GameData>) -> Rc<GameData> {
    let data = Rc::into_inner(data).expect("one owner");
    let mut dialogs = data.dialogs().clone();
    dialogs
        .compiled
        .insert("dialog/dock.ink".to_owned(), Arc::from(STORY));
    let job = nova_data::StoryMission {
        objective: Some(nova_data::MissionObjective::Deliver {
            cargo: "Plate".to_owned(),
            tons: 10,
            to: nova_data::StellarId(128),
        }),
        complete: Some(nova_data::MissionCompletion {
            at: None,
            dialog: DialogPlace {
                file: "dialog/dock.ink".to_owned(),
                knot: "done".to_owned(),
            },
        }),
        ..nova_data::StoryMission::default()
    };
    Rc::new(
        data.with_dialogs(dialogs)
            .with_storylines(nova_data::Storylines {
                missions: BTreeMap::from([("job".to_owned(), job)]),
                ..nova_data::Storylines::default()
            }),
    )
}

/// [`loader`]'s story with the job's completion, `done`.
fn delivering_loader() -> FakeLoader {
    let done = FakeKnot::new("done").line("Signed for.", &[]);
    let offer = FakeKnot::new("offer")
        .line("A job?", &[])
        .choice(FakeChoice::sticky("Leave", &[]).line("Bye.", &[]));
    FakeLoader::default().story(STORY, FakeDialog::default().knot(done).knot(offer))
}

/// Takes off from the spaceport and lands at the same planet again.
fn take_off_and_land_again(screen: &mut AppScreen) {
    screen.input(&key(Key::Escape, true));
    screen.input(&key(Key::Escape, false));
    assert_eq!(screen.showing(), Showing::Flight);
    screen.tick(TICK * 30);
    land_twice(screen);
    assert_eq!(screen.showing(), Showing::Spaceport);
}

#[test]
fn a_landing_that_delivers_shows_the_completion_before_the_stellar_s_own() {
    let loader = delivering_loader();
    let mut screen = talking(delivering(talking_data()), &loader);
    land(&mut screen);
    // The job happened as the ship landed: its plate is aboard, not yet
    // delivered, so the planet's own conversation shows.
    assert!(shows(&screen, "A job?"));
    screen.input(&key(Key::Digit1, true));
    screen.input(&key(Key::Enter, true));
    screen.input(&key(Key::Enter, false));
    take_off_and_land_again(&mut screen);
    assert!(shows(&screen, "Signed for."), "the completion first");
    assert!(!shows(&screen, "A job?"));
}

#[test]
fn a_landing_that_delivers_at_a_stellar_with_no_conversation_shows_the_completion() {
    let loader = delivering_loader();
    let mut screen = talking(delivering(data()), &loader);
    land(&mut screen);
    assert_eq!(
        drawn(&screen),
        drawn(spaceport(&screen)),
        "nothing to show yet"
    );
    take_off_and_land_again(&mut screen);
    assert!(shows(&screen, "Signed for."), "the completion");
}
