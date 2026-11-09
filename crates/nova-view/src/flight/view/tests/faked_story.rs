//! The new game's story readouts on [`FakeFlight`]: what the view takes
//! of the flight's room refusals and asks of its news, and hands back as
//! the flight gave them. What the session raises and answers is
//! nova-sim's (`session/storylines.rs`, `session/story_screens.rs`).

use super::faked_shops::docked;
use super::*;
use nova_sim::flight_frame::{
    FlightEvent, RoomRefusal, ScreenAnswer, ScreenAsk, StoryObjective, StoryRow,
};
use nova_sim::storylines::news::HeardNews;
use nova_sim::{MissionInfo, MissionInfoPress, SystemId};

/// A job of 10 tons refused with `room` free.
fn refused(room: i64) -> RoomRefusal {
    RoomRefusal {
        mission: "job".to_owned(),
        title: "Plate".to_owned(),
        cargo: "Plate".to_owned(),
        tons: 10,
        room,
    }
}

#[test]
fn a_room_refusal_the_flight_raised_is_taken_once_in_order() {
    let mut view = docked();
    view.sim.queue_event(FlightEvent::RoomRefused(refused(1)));
    view.sim.queue_event(FlightEvent::RoomRefused(refused(2)));
    assert_eq!(view.take_room_refusals(), [refused(1), refused(2)]);
    assert_eq!(view.take_room_refusals(), []);
}

/// An item heard `age` days after it broke in system 130.
fn heard(id: &str, age: u32) -> HeardNews {
    HeardNews {
        id: id.to_owned(),
        headline: format!("{id} headline"),
        text: format!("{id} text"),
        from: SystemId(130),
        age,
    }
}

#[test]
fn the_news_asked_is_handed_back_as_the_flight_answered() {
    let mut view = docked();
    let news = vec![heard("one", 3), heard("two", 0)];
    view.sim.queue_screen(ScreenAnswer::HeardNews(news.clone()));
    assert_eq!(view.heard_news(), news);
    assert_eq!(view.sim.screens_asked(), [ScreenAsk::HeardNews]);
    assert_eq!(view.heard_news(), [], "nothing answered");
}

/// A deliver row of 10 tons of Plate to 129, aboard.
pub(super) fn plate_row() -> StoryRow {
    StoryRow {
        mission: "job".to_owned(),
        title: "Plate run".to_owned(),
        objective: StoryObjective::Deliver {
            cargo: "Plate".to_owned(),
            tons: 10,
            to: StellarId(129),
        },
        cargo: 10,
        met: false,
        complete_at: Some(StellarId(129)),
    }
}

#[test]
fn i_opening_on_storyline_rows_alone_hands_the_info_to_the_router() {
    let mut view = faked(in_sol());
    let info = MissionInfo::shown(Vec::new(), "today").with_story(vec![plate_row()]);
    view.sim
        .queue_answer(Answer::MissionInfo(MissionInfoPress::Opens(info.clone())));
    view.ask_mission_info();
    assert_eq!(view.take_mission_info_request(), Some(info));
    assert_eq!(view.message(), None, "nothing said");
}
