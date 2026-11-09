//! The conversation's forwards on [`FakeFlight`]: the commands they send
//! and what the view hands back from the flight's answers. What a
//! conversation plays is nova-sim's (`session::apply::conversation`).

use super::faked_shops::docked;
use super::*;
use nova_sim::ConversationPage;
use nova_sim::flight_frame::Converse;

fn page(lines: &[&str], choices: &[&str]) -> ConversationPage {
    ConversationPage {
        lines: lines.iter().map(|&l| l.to_owned()).collect(),
        choices: choices.iter().map(|&c| c.to_owned()).collect(),
        ended: choices.is_empty(),
        problem: None,
    }
}

#[test]
fn open_sends_converse_open_and_hands_back_the_page_or_none() {
    let mut view = docked();
    view.sim
        .queue_answer(Answer::Conversation(page(&["A job?"], &["Take it"])));
    assert_eq!(
        view.open_conversation(),
        Some(page(&["A job?"], &["Take it"]))
    );
    assert_eq!(view.open_conversation(), None, "refused");
    assert_eq!(
        sent(&view),
        [
            FlightCommand::Converse(Converse::Open),
            FlightCommand::Converse(Converse::Open),
        ]
    );
}

#[test]
fn choose_sends_its_index_and_hands_back_the_next_page_or_none() {
    let mut view = docked();
    view.sim
        .queue_answer(Answer::Conversation(page(&["Bay two."], &[])));
    assert_eq!(
        view.choose_in_conversation(2),
        Some(page(&["Bay two."], &[]))
    );
    view.sim.queue_answer(Answer::Done);
    assert_eq!(view.choose_in_conversation(0), None, "not a page");
    assert_eq!(
        sent(&view),
        [
            FlightCommand::Converse(Converse::Choose(2)),
            FlightCommand::Converse(Converse::Choose(0)),
        ]
    );
}

#[test]
fn close_sends_converse_close_and_says_whether_one_was_open() {
    let mut view = docked();
    view.sim.queue_answer(Answer::Done);
    assert!(view.close_conversation());
    assert!(!view.close_conversation(), "refused");
    assert_eq!(
        sent(&view),
        [
            FlightCommand::Converse(Converse::Close),
            FlightCommand::Converse(Converse::Close),
        ]
    );
}

#[test]
fn open_bar_scene_sends_its_index_and_hands_back_the_page_or_none() {
    let mut view = docked();
    view.sim
        .queue_answer(Answer::Conversation(page(&["A job?"], &["Take it"])));
    assert_eq!(
        view.open_bar_scene(1),
        Some(page(&["A job?"], &["Take it"]))
    );
    assert_eq!(view.open_bar_scene(0), None, "refused");
    assert_eq!(
        sent(&view),
        [
            FlightCommand::Converse(Converse::OpenListed(1)),
            FlightCommand::Converse(Converse::OpenListed(0)),
        ]
    );
}
