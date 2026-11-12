//! The new game's news read state over the checked-in fixture
//! (`nova_data::new_game::fixture::folder`), through the app as `main`
//! builds it (`crate::new_game_port`'s app).
//!
//! - **Once.** The side road's news, released, opens the log as the pilot
//!   lands at Hearth Dock; closed, it doesn't open at the next landing
//!   there.
//! - **Per pilot.** The plate news, read at Verge Camp as the delivery
//!   releases it, is heard at Hearth Dock after the side road home, read,
//!   so no log opens there.
//! - **Saved.** The news read is in the pilot the window's close saves,
//!   and a new app over the same store opens no log at the next landing.
//! - **Dropped.** A log the window's close drops, unclosed, marks
//!   nothing: the saved pilot has read nothing, and the next landing
//!   opens it again.
//!
//! When an item is marked and what the router opens are pinned in
//! nova-sim's `session/apply/news.rs` and nova's `app/screen/story_log.rs`
//! tests; this pins that they meet the fixture, the window and the store.
//! Every id is read from the opened data, and only release days and
//! mission states are planted: the load freezes a reach saved without one.

use nova::app::{Control, Showing, WindowEvent};
use nova_data::new_game::fixture::folder;
use nova_sim::fixture::MemoryPilots;
use nova_sim::storylines::state::MissionState;
use nova_sim::{Pilot, SystemId};
use nova_view::spaceport::layout::LEAVE_ITEM;
use winit::keyboard::KeyCode;

use crate::new_game_conversation::{drawn, land_at, opened_from, position_of, resumed_in};
use crate::new_game_port::{
    app_over, click_port_item, in_the_verge_on_the_dock_job, pilot, resumed_after,
};
use crate::new_game_port_loop::{fly_the_autopilot, plot_course_to};
use crate::new_game_travel::{fixture, nova_of};
use crate::pilots::{Harness, saved};

/// The news ids the event closing the side road releases.
fn side_road_news() -> Vec<String> {
    let news = opened_from(&folder()).storylines().missions["side-road-closes"]
        .on_offer
        .news
        .clone();
    assert!(!news.is_empty(), "the side road's event releases news");
    news
}

/// The news ids the dock job's success releases.
fn plate_news() -> Vec<String> {
    let news = opened_from(&folder()).storylines().missions["dock-job"]
        .on_success
        .news
        .clone();
    assert!(!news.is_empty(), "the delivery releases news");
    news
}

/// News item `id`'s headline.
pub(crate) fn headline(id: &str) -> String {
    opened_from(&folder()).storylines().news[id]
        .headline
        .clone()
}

/// The side road's news released today, its event happened, and the dock
/// job declined, so Hearth Dock offers nothing.
fn side_road_released(pilot: &mut Pilot) {
    let today = pilot.story.day;
    for id in side_road_news() {
        pilot.story.news.insert(id, today);
    }
    pilot
        .story
        .missions
        .insert("side-road-closes".to_owned(), MissionState::Done);
    pilot
        .story
        .missions
        .insert("dock-job".to_owned(), MissionState::Declined);
}

/// Whether the next frame draws any of `ids`' headlines.
fn shows_any(game: &mut Harness, ids: &[String]) -> bool {
    let page = drawn(game);
    ids.iter().any(|id| page.contains(&headline(id)))
}

/// A resumed pilot with the side road's news released, landed at Hearth
/// Dock, its news log open.
fn landed_with_the_side_road_s_news() -> (Harness, MemoryPilots) {
    let (mut game, store) = resumed_after(side_road_released);
    land_at(&mut game, "hearth-dock");
    assert!(shows_any(&mut game, &side_road_news()), "the log opens");
    (game, store)
}

#[test]
fn the_news_opens_at_the_first_landing_only() {
    let (mut game, _store) = landed_with_the_side_road_s_news();
    game.press(KeyCode::Enter);
    assert!(!shows_any(&mut game, &side_road_news()), "closed");
    click_port_item(&mut game, LEAVE_ITEM);
    assert_eq!(game.showing(), Showing::Flight, "Leave takes off");
    land_at(&mut game, "hearth-dock");
    assert!(
        !shows_any(&mut game, &side_road_news()),
        "the spaceport alone"
    );
}

/// The fixture's system `id`.
fn system(id: &str) -> SystemId {
    SystemId(nova_of(&fixture().ids.systems, id))
}

#[test]
fn news_read_in_the_verge_does_not_open_the_log_at_hearth() {
    let (mut game, _store) = resumed_after(in_the_verge_on_the_dock_job);
    // An ungoverned outpost: one L lands.
    let camp = position_of("verge-camp");
    game.app
        .screen_mut()
        .flight_view_mut()
        .expect("flying")
        .change_sim(|sim| {
            nova_sim::fixture::put_player_at(sim.session_mut().expect("a session"), camp);
        });
    game.press(KeyCode::KeyL);
    for what in ["the completion", "the greeting", "the news log"] {
        assert_eq!(game.showing(), Showing::Spaceport, "{what}");
        game.press(KeyCode::Enter);
    }
    let read = pilot(&game).story.read;
    for id in plate_news() {
        assert!(read.contains(&id), "{id} read at the camp");
    }
    click_port_item(&mut game, LEAVE_ITEM);
    assert_eq!(game.showing(), Showing::Flight, "off the camp");
    let hearth = system("hearth");
    plot_course_to(&mut game, hearth);
    game.press(KeyCode::KeyJ);
    fly_the_autopilot(&mut game);
    assert_eq!(pilot(&game).system, hearth, "home by the side road");
    land_at(&mut game, "hearth-dock");
    let session = game.ship_session();
    let heard = session.heard_news(hearth);
    for id in plate_news() {
        let item = heard.iter().find(|item| item.id == id);
        assert!(item.is_some_and(|item| item.read), "{id} heard here, read");
    }
    assert!(
        !shows_any(&mut game, &plate_news()),
        "no log: {:?}",
        drawn(&mut game)
    );
}

#[test]
fn the_news_read_survives_a_save_and_a_load() {
    let (mut game, store) = landed_with_the_side_road_s_news();
    game.press(KeyCode::Enter);
    assert_eq!(game.handle(WindowEvent::CloseRequested), Control::Exit);
    let read = saved(&store, "Ada").story.read;
    for id in side_road_news() {
        assert!(read.contains(&id), "{id} saved read");
    }
    let mut resumed = resumed_in(app_over(opened_from(&folder()), &store));
    land_at(&mut resumed, "hearth-dock");
    assert!(
        !shows_any(&mut resumed, &side_road_news()),
        "no log after the load"
    );
}

#[test]
fn a_log_the_window_s_close_drops_marks_nothing() {
    let (mut game, store) = landed_with_the_side_road_s_news();
    assert_eq!(game.handle(WindowEvent::CloseRequested), Control::Exit);
    assert!(saved(&store, "Ada").story.read.is_empty(), "nothing read");
    let mut resumed = resumed_in(app_over(opened_from(&folder()), &store));
    land_at(&mut resumed, "hearth-dock");
    assert!(
        shows_any(&mut resumed, &side_road_news()),
        "the log opens again"
    );
}
