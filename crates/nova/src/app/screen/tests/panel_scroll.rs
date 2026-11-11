//! The story log and the conversation panel scroll through the router: a
//! Down press, a wheel notch under `RuleKey::DialogTextWheel`'s other
//! reading, and a press on the drawn down arrow each reach the panel and
//! bring its end into view, the panel left open; under the engine's
//! reading the router drops the wheel before either sees it; and the
//! router's bindings reach both, so a rebound Down scrolls them. How the
//! page scrolls is pinned in `nova_view` (`ui::panel_scroll`,
//! `ui::story_log`, `spaceport::conversation`).

use std::collections::BTreeMap;
use std::sync::Arc;

use nova_data::{
    DialogPlace, Dialogs as StoredDialogs, MissionOffer, NewsItem, StoryEffects, StoryMission,
    Storylines,
};
use nova_sim::dialog::fake::{FakeChoice, FakeDialog, FakeKnot, FakeLoader};
use nova_view::input::ScrollDelta;
use nova_view::spaceport::conversation::CLOSE_HINT;
use nova_view::ui::scroll_arrows::ScrollArrow;
use nova_view::{Bindings, KeyCombo, SharedBindings, keymap::UiCommand};

use super::*;

const STORY: &str = "long story";
const FILE: &str = "dialog/long.ink";

/// The oldest of [`many_news`]'s items, listed last.
const OLDEST: &str = "Item 00";

/// [`data`] with an event releasing twelve news items from the start
/// system as the ship first lands, `Item 00` to `Item 11`, more than the
/// log's panel holds; and, with `offer`, an offer at the planet whose
/// page is one line longer than the conversation's panel holds.
fn panel_data(offer: bool) -> Rc<GameData> {
    let ids: Vec<String> = (0..12).map(|n| format!("n{n:02}")).collect();
    let news = ids
        .iter()
        .enumerate()
        .map(|(n, id)| {
            let item = NewsItem {
                headline: format!("Item {n:02}"),
                text: "Some news.".to_owned(),
                from: SystemId(128),
            };
            (id.clone(), item)
        })
        .collect();
    let mut missions = BTreeMap::from([(
        "breaks".to_owned(),
        StoryMission {
            on_offer: StoryEffects {
                news: if offer { Vec::new() } else { ids },
                ..StoryEffects::default()
            },
            ..StoryMission::default()
        },
    )]);
    if offer {
        missions.insert(
            "long".to_owned(),
            StoryMission {
                offer: Some(MissionOffer {
                    at: nova_data::StellarId(128),
                    dialog: DialogPlace {
                        file: FILE.to_owned(),
                        knot: "offer".to_owned(),
                    },
                }),
                ..StoryMission::default()
            },
        );
    }
    let data = Rc::into_inner(data()).expect("one owner");
    Rc::new(
        data.with_dialogs(StoredDialogs {
            compiled: BTreeMap::from([(FILE.to_owned(), Arc::from(STORY))]),
            stellars: BTreeMap::new(),
        })
        .with_storylines(Storylines {
            missions,
            news,
            ..Storylines::default()
        }),
    )
}

/// One line of 800 numbered words, `w000` to `w799`: some 45 rows, more
/// than the conversation's panel holds.
fn long_line() -> String {
    (0..800)
        .map(|n| format!("w{n:03}"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// The offer: [`long_line`], then `Take` and `Later`.
fn loader() -> FakeLoader {
    let offer = FakeKnot::new("offer")
        .line(&long_line(), &[])
        .choice(FakeChoice::sticky("Take", &[]).line("Bay two.", &[]))
        .choice(FakeChoice::sticky("Later", &[]).line("Later.", &[]));
    FakeLoader::default().story(STORY, FakeDialog::default().knot(offer))
}

/// A router over [`panel_data`] reading `rules`, `bindings` and the
/// interface's dialogs, landed: the log or the conversation open.
fn landed_on(offer: bool, rules: Rulebook, bindings: Option<SharedBindings>) -> AppScreen {
    let mut screen = with_dialogs(panel_data(offer))
        .with_dialog_loader(Rc::new(loader()))
        .with_rules(rules);
    if let Some(bindings) = bindings {
        screen = screen.with_bindings(bindings);
    }
    land(&mut screen);
    if offer {
        assert!(screen.conversation.is_some(), "the offer opens");
    } else {
        assert!(screen.story_log.is_some(), "the log opens");
    }
    screen
}

fn texts(screen: &AppScreen) -> Vec<String> {
    drawn(screen)
        .iter()
        .filter_map(|command| match command {
            DrawCommand::Text { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect()
}

/// Gives `screen` `input` until the frame draws a text `ends` reads, at
/// most 100 times: how many it took.
fn input_until(screen: &mut AppScreen, input: &Input, ends: impl Fn(&str) -> bool) -> usize {
    for n in 0..100 {
        if texts(screen).iter().any(|text| ends(text)) {
            return n;
        }
        screen.input(input);
    }
    panic!("{input:?} never brought the end into view");
}

fn down() -> Input {
    key(Key::ArrowDown, true)
}

/// A notch of the wheel at `at`, as the window reports it.
fn notch(at: Point) -> Input {
    Input::Scroll {
        delta: ScrollDelta::Lines(1.0),
        at,
    }
}

/// The wheel's other reading, which hands it to the screens.
fn wheeling() -> Rulebook {
    Rulebook::default().with_override(RuleKey::DialogTextWheel, RuleSource::Bible)
}

/// The log's down arrow, where the router draws it.
fn log_arrow(screen: &AppScreen) -> Point {
    let open = screen.story_log.as_ref().expect("the log");
    let at = open
        .screen
        .arrow_bounds(ScrollArrow::Down)
        .expect("a long log's arrow")
        .center();
    let offset = screen.overlay_offset();
    Point::new(at.x + offset.x, at.y + offset.y)
}

/// The conversation's down arrow.
fn conversation_arrow(screen: &AppScreen) -> Point {
    screen
        .conversation
        .as_ref()
        .expect("the conversation")
        .arrow_bounds(ScrollArrow::Down)
        .expect("a long page's arrow")
        .center()
}

fn press(at: Point) -> Input {
    Input::PointerButton {
        button: MouseButton::Left,
        pressed: true,
        at,
    }
}

/// Whether the log's end, the hint after its oldest item, is drawn.
fn log_end(text: &str) -> bool {
    text == CLOSE_HINT
}

/// Whether the conversation's end, its last choice, is drawn.
fn conversation_end(text: &str) -> bool {
    text == "2. Later"
}

#[test]
fn a_down_press_reaches_the_log_and_brings_its_oldest_item_into_view() {
    let mut screen = landed_on(false, Rulebook::default(), None);
    assert!(!texts(&screen).contains(&OLDEST.to_owned()));
    assert!(input_until(&mut screen, &down(), log_end) > 0);
    assert!(texts(&screen).contains(&OLDEST.to_owned()));
    assert!(screen.story_log.is_some(), "still open");
}

#[test]
fn a_wheel_notch_reaches_the_log_under_the_wheel_s_other_reading() {
    let mut screen = landed_on(false, wheeling(), None);
    let over = log_arrow(&screen);
    let over_text = Point::new(over.x - 200.0, over.y - 100.0);
    assert!(input_until(&mut screen, &notch(over_text), log_end) > 0);
    assert!(texts(&screen).contains(&OLDEST.to_owned()));
    assert!(screen.story_log.is_some(), "still open");
}

#[test]
fn a_press_on_the_log_s_drawn_down_arrow_scrolls_it_and_leaves_it_open() {
    let mut screen = landed_on(false, Rulebook::default(), None);
    let arrow = log_arrow(&screen);
    assert!(input_until(&mut screen, &press(arrow), log_end) > 0);
    assert!(texts(&screen).contains(&OLDEST.to_owned()));
    assert!(screen.story_log.is_some(), "still open");
}

#[test]
fn under_the_engine_s_reading_the_wheel_moves_neither_panel() {
    let mut log = landed_on(false, Rulebook::default(), None);
    let before = texts(&log);
    let over = log_arrow(&log);
    for _ in 0..50 {
        log.input(&notch(Point::new(over.x - 200.0, over.y - 100.0)));
    }
    assert_eq!(texts(&log), before, "the log");
    let mut talk = landed_on(true, Rulebook::default(), None);
    let before = texts(&talk);
    for _ in 0..50 {
        talk.input(&notch(Point::new(512.0, 384.0)));
    }
    assert_eq!(texts(&talk), before, "the conversation");
}

#[test]
fn a_down_press_reaches_the_conversation_and_brings_its_choices_into_view() {
    let mut screen = landed_on(true, Rulebook::default(), None);
    assert!(!texts(&screen).iter().any(|text| conversation_end(text)));
    assert!(input_until(&mut screen, &down(), conversation_end) > 0);
    assert!(
        texts(&screen).iter().any(|text| text.contains("w799")),
        "the line's end"
    );
    assert!(screen.conversation.is_some(), "still open");
}

#[test]
fn a_wheel_notch_reaches_the_conversation_under_the_wheel_s_other_reading() {
    let mut screen = landed_on(true, wheeling(), None);
    let over_text = Point::new(512.0, 384.0);
    assert!(input_until(&mut screen, &notch(over_text), conversation_end) > 0);
    assert!(screen.conversation.is_some(), "still open");
}

#[test]
fn a_press_on_the_conversation_s_down_arrow_scrolls_it_and_takes_no_choice() {
    let mut screen = landed_on(true, Rulebook::default(), None);
    let arrow = conversation_arrow(&screen);
    assert!(input_until(&mut screen, &press(arrow), conversation_end) > 0);
    assert!(screen.conversation.is_some(), "still open");
    assert!(
        !texts(&screen).contains(&"Bay two.".to_owned()),
        "no choice"
    );
}

#[test]
fn the_router_s_bindings_reach_both_panels() {
    let rebound = || {
        SharedBindings::new(Bindings::default().rebound(UiCommand::Down, KeyCombo::key(Key::KeyJ)))
    };
    let j = key(Key::KeyJ, true);
    let mut log = landed_on(false, Rulebook::default(), Some(rebound()));
    assert!(input_until(&mut log, &j, log_end) > 0);
    let mut talk = landed_on(true, Rulebook::default(), Some(rebound()));
    assert!(input_until(&mut talk, &j, conversation_end) > 0);
}
