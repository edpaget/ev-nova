//! The story log over the spaceport and over flight: the news heard
//! where the ship lands, opened once a visit after the visit's
//! conversations; the active storyline missions on the mission info key;
//! and an accept refused for room told on the conversation's page. What
//! the session decides is pinned in `nova_sim`, the panel in `nova_view`.

use std::collections::BTreeMap;
use std::sync::Arc;

use nova_data::{
    DialogPlace, Dialogs as StoredDialogs, MissionObjective, MissionOffer, NewsItem, StoryEffects,
    StoryMission, Storylines,
};
use nova_sim::dialog::fake::{FakeChoice, FakeDialog, FakeKnot, FakeLoader};
use nova_view::spaceport::conversation::CLOSE_HINT;
use nova_view::spaceport::layout::service_item;
use nova_view::ui::story_log::{MISSIONS_HEADING, NEWS_HEADING};

use super::*;

const STORY: &str = "dock story";
const FILE: &str = "dialog/dock.ink";
const HEADLINE: &str = "Word spreads";
const JOB: &str = "Plate run";

/// The dock's offer: a line, `Take` accepting and `Later` deferring.
fn loader() -> FakeLoader {
    let offer = FakeKnot::new("offer")
        .line("A job?", &[])
        .choice(FakeChoice::sticky("Take", &[]).line("Bay two.", &["outcome: accept"]))
        .choice(FakeChoice::sticky("Later", &[]).line("Later.", &["outcome: defer"]));
    FakeLoader::default().story(STORY, FakeDialog::default().knot(offer))
}

/// [`data`] with an event that releases the news item `word` from the
/// start system as the ship first lands, and, with `job` some, an offer
/// at the planet of that many tons of plate for stellar 129.
fn story_data(job: Option<u32>) -> Rc<GameData> {
    let mut missions = BTreeMap::from([(
        "breaks".to_owned(),
        StoryMission {
            on_offer: StoryEffects {
                news: vec!["word".to_owned()],
                ..StoryEffects::default()
            },
            ..StoryMission::default()
        },
    )]);
    if let Some(tons) = job {
        missions.insert(
            "job".to_owned(),
            StoryMission {
                title: JOB.to_owned(),
                offer: Some(MissionOffer {
                    at: nova_data::StellarId(128),
                    dialog: DialogPlace {
                        file: FILE.to_owned(),
                        knot: "offer".to_owned(),
                    },
                }),
                objective: Some(MissionObjective::Deliver {
                    cargo: "Plate".to_owned(),
                    tons,
                    to: nova_data::StellarId(129),
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
            news: BTreeMap::from([(
                "word".to_owned(),
                NewsItem {
                    headline: HEADLINE.to_owned(),
                    text: "It spreads.".to_owned(),
                    from: SystemId(128),
                },
            )]),
            ..Storylines::default()
        }),
    )
}

/// A router over `data` with the interface's dialogs and metrics, its
/// flights playing conversations on [`loader`].
fn storied(data: Rc<GameData>) -> AppScreen {
    with_dialogs(data).with_dialog_loader(Rc::new(loader()))
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

fn shows(screen: &AppScreen, text: &str) -> bool {
    texts(screen).iter().any(|drawn| drawn == text)
}

fn tap(screen: &mut AppScreen, k: Key) {
    screen.input(&key(k, true));
    screen.input(&key(k, false));
}

/// Takes off with Escape and lands again: a new visit.
fn visit_again(screen: &mut AppScreen) {
    tap(screen, Key::Escape);
    assert_eq!(screen.showing(), Showing::Flight);
    screen.tick(TICK * 30);
    land_twice(screen);
    assert_eq!(screen.showing(), Showing::Spaceport);
}

#[test]
fn the_news_opens_after_the_last_conversation_closes_and_not_twice_a_visit() {
    let mut screen = storied(story_data(Some(10)));
    land(&mut screen);
    assert!(shows(&screen, "A job?"), "the offer first");
    assert!(!shows(&screen, HEADLINE));
    tap(&mut screen, Key::Digit2);
    assert!(shows(&screen, "Later."));
    tap(&mut screen, Key::Enter);
    assert!(shows(&screen, NEWS_HEADING), "{:?}", texts(&screen));
    assert!(shows(&screen, HEADLINE));
    assert!(shows(&screen, "It spreads."));
    assert!(
        texts(&screen).iter().any(|text| text.ends_with(", today.")),
        "{:?}",
        texts(&screen)
    );
    tap(&mut screen, Key::Enter);
    assert_eq!(screen.showing(), Showing::Spaceport);
    assert!(!shows(&screen, HEADLINE), "closed");
    assert_eq!(
        drawn(&screen),
        drawn(spaceport(&screen)),
        "the spaceport alone"
    );
}

#[test]
fn the_news_opens_again_at_the_next_landing() {
    let mut screen = storied(story_data(None));
    land(&mut screen);
    assert!(
        shows(&screen, HEADLINE),
        "no conversation: the news at once"
    );
    tap(&mut screen, Key::Enter);
    assert!(!shows(&screen, HEADLINE));
    visit_again(&mut screen);
    assert!(shows(&screen, HEADLINE), "{:?}", texts(&screen));
}

#[test]
fn the_news_takes_every_input_in_front_of_the_spaceport() {
    let mut screen = storied(story_data(None));
    land(&mut screen);
    assert!(shows(&screen, HEADLINE));
    tap(&mut screen, Key::Escape);
    assert_eq!(
        screen.showing(),
        Showing::Spaceport,
        "Escape closed the news"
    );
    assert!(!shows(&screen, HEADLINE));
    tap(&mut screen, Key::Escape);
    assert_eq!(screen.showing(), Showing::Flight, "then took off");
}

/// The news log's beeps among the sounds `screen` reports, taken.
fn news_beeps(screen: &mut AppScreen) -> Vec<UiSound> {
    screen
        .take_sounds()
        .into_iter()
        .filter_map(|sound| match sound {
            Sound::Ui(beep @ (UiSound::NewsOpened | UiSound::NewsClosed)) => Some(beep),
            _ => None,
        })
        .collect()
}

#[test]
fn the_bars_holovid_beeps_as_it_opens_and_closes_and_the_landings_news_never() {
    // The bar, laid out, hosts the story under `RuleKey::BarStory`'s
    // other reading.
    let rules = Rulebook::default().with_override(RuleKey::BarStory, RuleSource::Bible);
    let mut screen = AppScreen::new(story_data(None))
        .with_dialogs(Rc::new(PastTheHoldsDialogs), Rc::new(MonoMetrics))
        .with_dialog_loader(Rc::new(loader()))
        .with_rules(rules);
    land(&mut screen);
    assert!(shows(&screen, HEADLINE), "the landing's news");
    assert_eq!(news_beeps(&mut screen), [], "pops up silently");
    tap(&mut screen, Key::Enter);
    assert!(!shows(&screen, HEADLINE));
    assert_eq!(news_beeps(&mut screen), [], "and closes silently");
    click_port_item(&mut screen, service_item(nova_sim::Service::Bar));
    assert!(spaceport(&screen).open_bar().is_some(), "in the bar");
    tap(&mut screen, Key::KeyN);
    assert!(shows(&screen, HEADLINE), "{:?}", texts(&screen));
    assert_eq!(news_beeps(&mut screen), [UiSound::NewsOpened]);
    tap(&mut screen, Key::Enter);
    assert!(!shows(&screen, HEADLINE), "closed");
    assert!(spaceport(&screen).open_bar().is_some(), "back in the bar");
    assert_eq!(news_beeps(&mut screen), [UiSound::NewsClosed]);
}

#[test]
fn a_visit_with_no_news_and_no_conversation_shows_the_spaceport_alone() {
    let data = Rc::into_inner(data()).expect("one owner");
    let mut screen = storied(Rc::new(data));
    land(&mut screen);
    assert_eq!(drawn(&screen), drawn(spaceport(&screen)));
}

/// A router flying after taking a 0-ton job at the planet, the news
/// read, Escape taking off.
fn flying_on_the_job() -> AppScreen {
    flying_on_the_job_in(storied(story_data(Some(0))))
}

/// [`flying_on_the_job`] in `screen`.
fn flying_on_the_job_in(mut screen: AppScreen) -> AppScreen {
    land(&mut screen);
    tap(&mut screen, Key::Digit1);
    assert!(shows(&screen, "Bay two."));
    tap(&mut screen, Key::Enter);
    assert!(shows(&screen, HEADLINE));
    tap(&mut screen, Key::Enter);
    tap(&mut screen, Key::Escape);
    assert_eq!(screen.showing(), Showing::Flight);
    screen.tick(TICK * 30);
    screen
}

#[test]
fn i_in_flight_opens_the_missions_log_over_the_paused_flight() {
    let mut screen = flying_on_the_job();
    tap(&mut screen, MISSION_INFO_KEY);
    assert!(shows(&screen, MISSIONS_HEADING), "{:?}", texts(&screen));
    assert!(shows(&screen, JOB));
    assert!(
        shows(&screen, "Deliver 0 tons of Plate to Beta Prime."),
        "{:?}",
        texts(&screen)
    );
    let still = ship(&screen);
    screen.tick(TICK * 5);
    assert_eq!(ship(&screen), still, "flight paused");
    tap(&mut screen, Key::Enter);
    assert!(!shows(&screen, MISSIONS_HEADING), "closed");
    assert_eq!(screen.showing(), Showing::Flight);
}

#[test]
fn a_switch_in_under_the_missions_log_over_flight_gives_way_to_its_closes_zero() {
    // The log stands in for the mission info, so its close owes the
    // zero the `i` press's does (`info_dialog_gain`).
    let mut screen = flying_on_the_job();
    tap(&mut screen, MISSION_INFO_KEY);
    assert!(shows(&screen, MISSIONS_HEADING), "{:?}", texts(&screen));
    screen.focus_lost();
    screen.focus_gained();
    let gain = |screen: &AppScreen| flight(screen).session().expect("flying").speed_mult();
    assert_eq!(gain(&screen), nova_sim::SpeedMult::FULL, "the switch-in's");
    tap(&mut screen, Key::Enter);
    assert_eq!(screen.showing(), Showing::Flight);
    assert_eq!(gain(&screen), nova_sim::SpeedMult::ZERO, "the close's");
}

#[test]
fn i_in_port_opens_the_missions_log_in_place_of_the_stock_info() {
    // The interface's mission info screen is there to open, but the
    // storyline missions take the log.
    let screen = AppScreen::new(story_data(Some(0)))
        .with_dialogs(Rc::new(InfoOverEveryOpener), Rc::new(MonoMetrics))
        .with_dialog_loader(Rc::new(loader()));
    let mut screen = flying_on_the_job_in(screen);
    land_twice(&mut screen);
    assert_eq!(screen.showing(), Showing::Spaceport);
    if shows(&screen, HEADLINE) {
        tap(&mut screen, Key::Enter);
    }
    tap(&mut screen, MISSION_INFO_KEY);
    assert!(shows(&screen, MISSIONS_HEADING), "{:?}", texts(&screen));
    assert!(spaceport(&screen).mission_info().is_none(), "no DLOG 1012");
    tap(&mut screen, Key::Escape);
    assert!(!shows(&screen, MISSIONS_HEADING));
    assert_eq!(screen.showing(), Showing::Spaceport, "closed, still landed");
}

#[test]
fn an_accept_refused_for_room_says_so_on_its_page() {
    let mut screen = storied(story_data(Some(1_000_000)));
    land(&mut screen);
    tap(&mut screen, Key::Digit1);
    assert!(shows(&screen, "Bay two."));
    let notice = texts(&screen)
        .into_iter()
        .find(|text| text.starts_with("No room in the hold"));
    assert!(notice.is_some(), "{:?}", texts(&screen));
    assert!(shows(&screen, CLOSE_HINT));
    tap(&mut screen, Key::Enter);
    assert!(
        !texts(&screen)
            .iter()
            .any(|text| text.starts_with("No room")),
        "gone with its page"
    );
}
