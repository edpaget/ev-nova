//! The bar race's component tests on [`FakeFlight`]: the ask and the
//! commands Gamble, a race, its collect and its Help send, and what the
//! view hands back from the flight's answers. What the race draws, takes
//! and pays is nova-sim's (`session/gamble.rs`).

use super::*;
use nova_sim::DescMovie;
use nova_sim::gamble::{Bet, GambleOpening, RaceRun, winnings_line};

use crate::flight::RaceShown;
use crate::ui::mission_dialogs::DescShown;

/// A view docked, reading the race's `dësc`s: 32762 "Racer 3 wins.",
/// naming "Race 3.mov", and the Help, 32764.
fn racing() -> Faked {
    let movie = DescMovie::from_desc("Race 3.mov", 0).expect("named");
    let catalog = FakeCatalog {
        descriptions: vec![
            (32_762, "Racer 3 wins.".to_owned()),
            (32_764, "How to bet.".to_owned()),
        ],
        desc_movies: vec![(32_762, movie)],
        ..catalog()
    };
    faked_in(catalog, in_sol())
}

#[test]
fn gamble_opens_what_the_flight_answers_and_nothing_unanswered() {
    let mut view = racing();
    view.sim
        .queue_screen(ScreenAnswer::Gamble(GambleOpening::Race { asked_max: 70 }));
    assert_eq!(
        view.gamble_opening(),
        Some(GambleOpening::Race { asked_max: 70 })
    );
    assert_eq!(view.gamble_opening(), None, "nothing answered");
    assert_eq!(
        view.sim.screens_asked(),
        [ScreenAsk::Gamble, ScreenAsk::Gamble]
    );
}

#[test]
fn a_race_sends_its_bet_then_loads_the_winners_desc_with_the_winnings_line() {
    let mut view = racing();
    let line = winnings_line(400, nova_sim::RuleSource::Engine);
    view.sim.queue_answer(Answer::Raced(RaceRun {
        chosen: 2,
        winner: 2,
        bet: 100,
        winnings_line: Some(line.clone()),
        asked_max: 900,
    }));
    view.sim.queue_description("Racer 3 wins, loaded.");
    let shown = view.race(2, Bet::Asked(100)).expect("a race");
    assert_eq!(
        shown,
        RaceShown {
            winner: 2,
            asked_max: 900,
            desc: DescShown {
                text: format!("Racer 3 wins, loaded.{line}"),
                graphic: None,
                movie: DescMovie::from_desc("Race 3.mov", 0),
            },
        }
    );
    assert!(view.sim.applied().contains(&FlightCommand::Race {
        racer: 2,
        bet: Bet::Asked(100)
    }));
    assert_eq!(
        view.sim.asks(),
        [FlightCommand::LoadDescription {
            raw: Some("Racer 3 wins.".to_owned()),
            movie: DescMovie::from_desc("Race 3.mov", 0),
        }],
        "the winner's dësc loaded through the flight"
    );
}

#[test]
fn a_lost_race_shows_the_desc_alone_and_none_run_loads_nothing() {
    let mut view = racing();
    view.sim.queue_answer(Answer::Raced(RaceRun {
        chosen: 0,
        winner: 2,
        bet: 100,
        winnings_line: None,
        asked_max: 900,
    }));
    view.sim.queue_description("Racer 3 wins.");
    let shown = view.race(0, Bet::House).expect("a race");
    assert_eq!(shown.desc.text, "Racer 3 wins.");
    view.sim.queue_answer(Answer::Refused);
    assert_eq!(view.race(1, Bet::Asked(0)), None);
    assert_eq!(view.sim.asks().len(), 1, "no second load");
}

#[test]
fn the_collect_sends_its_command_and_hands_back_the_asked_most_credited() {
    let mut view = racing();
    view.sim
        .queue_answer(Answer::WinningsCollected { asked_max: 4_000 });
    assert_eq!(view.collect_winnings(), Some(4_000));
    view.sim.queue_answer(Answer::Refused);
    assert_eq!(view.collect_winnings(), None);
    let sent = view
        .sim
        .applied()
        .iter()
        .filter(|command| **command == FlightCommand::CollectWinnings)
        .count();
    assert_eq!(sent, 2);
}

#[test]
fn the_races_help_is_its_desc_loaded_through_the_flight() {
    let mut view = racing();
    view.sim.queue_description("How to bet, loaded.");
    assert_eq!(view.race_help(), "How to bet, loaded.");
    assert_eq!(
        view.sim.asks(),
        [FlightCommand::LoadDescription {
            raw: Some("How to bet.".to_owned()),
            movie: None,
        }]
    );
}
