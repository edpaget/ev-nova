//! The new game's story log and the room refusal, through the app over
//! the checked-in fixture, wired as `main` wires it (see
//! `new_game_conversation`). What the panel draws, what the router opens
//! when, and what the session decides are pinned in their own crates;
//! this pins that they meet.
//!
//! - **News.** A pilot saved six days after the side road's closing
//!   day, with `side-road-closes` released that day and reaching the
//!   Hearth at once, lands at Hearth Dock with no offer to show and
//!   reads it, six days old, every line inside the conversation panel.
//! - **Missions.** A pilot carrying the dock job's plate presses I in
//!   flight and reads the job, its objective and its cargo.
//! - **Refusal.** A pilot whose 50-ton Hauler holds 45 tons takes the dock
//!   job and is told there's no room, the job left offered and the hold
//!   as it was.

use std::collections::BTreeMap;

use nova::app::Showing;
use nova_data::new_game::fixture::folder;
use nova_sim::fixture::MemoryPilots;
use nova_sim::storylines::state::MissionState;
use nova_sim::{Good, Pilot, PilotStore, SystemId};
use nova_view::spaceport::conversation::{INSET, PANEL, SIZE};
use nova_view::text::LINE_HEIGHT;
use nova_view::{DrawCommand, DrawList, Screen};
use winit::keyboard::KeyCode;

use crate::new_game_conversation::{drawn, flying_in, land_at, resumed};
use crate::new_game_play;
use crate::new_game_travel::{nova_of, side_road_closing_day};
use crate::pilots::{Harness, saved};

/// The fixture's system `id`'s ID.
fn system(id: &str) -> SystemId {
    let (_, game) = new_game_play::opened(&folder());
    SystemId(nova_of(&game.ids.systems, id))
}

/// Ada, saved at once as a new pilot and changed by `edit`, resumed in a
/// new app, flying where she was saved.
fn resumed_after(edit: impl FnOnce(&mut Pilot)) -> (Harness, MemoryPilots) {
    let store = MemoryPilots::new();
    drop(flying_in(&store));
    let mut pilot = saved(&store, "Ada");
    edit(&mut pilot);
    store
        .write("Ada", &nova_sim::save::encode(&pilot))
        .expect("writes");
    (resumed(&store), store)
}

/// The pilot as the flight holds her.
fn pilot(game: &Harness) -> Pilot {
    game.app
        .screen()
        .flight_view()
        .expect("flying")
        .sim()
        .pilot()
        .expect("a pilot")
        .clone()
}

/// Each text the router draws, with its origin, in logical units.
fn placed(game: &Harness) -> Vec<(String, f32, f32)> {
    let mut list = DrawList::new();
    game.app.screen().draw(&mut list);
    list.iter()
        .filter_map(|command| match command {
            DrawCommand::Text { text, origin, .. } => Some((text.clone(), origin.x, origin.y)),
            _ => None,
        })
        .collect()
}

#[test]
fn landing_where_the_news_was_heard_shows_it_with_its_age_inside_the_panel() {
    let hearth = system("hearth");
    let drift = system("drift");
    let closing = side_road_closing_day();
    let (mut game, _store) = resumed_after(|pilot| {
        pilot.story.day = closing + 6;
        pilot
            .story
            .news
            .insert("side-road-closes".to_owned(), closing);
        pilot.story.reach.insert(
            "side-road-closes".to_owned(),
            BTreeMap::from([(hearth, 0), (drift, 1)]),
        );
        pilot
            .story
            .missions
            .insert("side-road-closes".to_owned(), MissionState::Done);
        pilot
            .story
            .missions
            .insert("dock-job".to_owned(), MissionState::Declined);
    });
    land_at(&mut game, "hearth-dock");
    let page = drawn(&mut game);
    assert!(
        page.contains(&"The side road has failed".to_owned()),
        "{page:?}"
    );
    assert!(
        page.contains(&"From Hearth, 6 days old.".to_owned()),
        "{page:?}"
    );
    let step = LINE_HEIGHT * SIZE;
    let news: Vec<_> = placed(&game)
        .into_iter()
        .skip_while(|(text, ..)| text != "News")
        .collect();
    assert!(!news.is_empty(), "the log drawn");
    for (text, x, y) in news {
        assert!(x >= PANEL.min.x + INSET, "{text}");
        assert!(
            y >= PANEL.min.y + INSET && y + step <= PANEL.max.y - INSET,
            "{text} at {y}"
        );
    }
    game.press(KeyCode::Enter);
    assert!(
        !drawn(&mut game).contains(&"The side road has failed".to_owned()),
        "closed"
    );
    assert_eq!(game.showing(), Showing::Spaceport);
}

#[test]
fn i_in_flight_lists_the_dock_job_its_objective_and_its_cargo() {
    let (mut game, _store) = resumed_after(|pilot| {
        pilot.story.missions.insert(
            "dock-job".to_owned(),
            MissionState::Active {
                cargo: 10,
                met: false,
            },
        );
        pilot.flags.set("hauling_plate");
    });
    game.press(KeyCode::KeyI);
    let page = drawn(&mut game);
    for line in [
        "Missions",
        "Hull plate for Verge Camp",
        "Deliver 10 tons of Hull plate to Verge Camp.",
        "Aboard: 10 tons.",
    ] {
        assert!(page.contains(&line.to_owned()), "{line}: {page:?}");
    }
    let (storyline, job) = crate::new_game_port::dock_job_titles();
    crate::new_game_port::assert_listed_under(&page, &storyline, &job);
    game.press(KeyCode::Enter);
    assert!(
        !drawn(&mut game).contains(&"Hull plate for Verge Camp".to_owned()),
        "closed"
    );
    assert_eq!(game.showing(), Showing::Flight);
}

#[test]
fn taking_the_dock_job_with_a_full_hold_says_so_and_leaves_it_offered() {
    let (mut game, _store) = resumed_after(|pilot| {
        pilot.cargo.insert(Good::Commodity(0), 45);
    });
    let hold = pilot(&game).cargo;
    land_at(&mut game, "hearth-dock");
    game.press(KeyCode::Digit1);
    // The notice wraps across the panel's lines.
    let page = drawn(&mut game).join(" ");
    assert!(
        page.contains(
            "No room in the hold: this job needs 10 tons of Hull plate, and 5 tons are free. \
             The offer stands for a later visit."
        ),
        "{page}"
    );
    let now = pilot(&game);
    assert_eq!(now.story.mission("dock-job"), Some(MissionState::Offered));
    assert_eq!(now.cargo, hold, "nothing loaded");
}
