//! The new game's offer conversation played through in the app, over the
//! checked-in fixture (`nova_data::new_game::fixture::folder`) opened as
//! the app opens it (`DiskData::open_new_game`, which checks each
//! stellar's knot against the Ink runtime), with `nova::dialogs::loader`
//! wired as `main` wires it and the new game's rulebook.
//!
//! A new pilot's ship, put over Hearth Dock, lands with two L presses, and
//! the dock's offer (`dialog/dock-job.ink#job_offer`, compiled and checked
//! in beside its `.ink`) opens over the spaceport. The test asks about
//! the route, takes the job, and closes the ended page back to the
//! spaceport. Hearth Dock is the only stellar that names one. What the
//! screen, the session and the story do is pinned in their own crates;
//! this pins that they meet.
//!
//! The story's state goes into the pilot's save: a pilot who asked about
//! the route, then closed the window (which saves it) and was opened again
//! in a new app through Open Pilot, hears `"You know the way by now."` at
//! the dock and isn't offered the question again. A pilot saved with the
//! world flag `dock_regular` is greeted as a regular.

use std::rc::Rc;

use nova::app::{Control, Showing, WindowEvent};
use nova::rulebook::ruleset_rulebook;
use nova::ruleset::Ruleset;
use nova::startup::{DataSource, DiskData, NewGameOpened};
use nova_data::GameData;
use nova_data::records::stellar::Stellar;
use nova_sim::fixture::MemoryPilots;
use nova_sim::{PilotStore, Vec2};
use nova_view::menu::MenuChoice;
use nova_view::spaceport::conversation::CLOSE_HINT;
use winit::keyboard::KeyCode;

use crate::common::EntryHarness;
use crate::new_game_travel::fixture;
use crate::pilots::{Harness, saved, texts};

/// The offer's first page.
const OFFER: [&str; 5] = [
    "The dockmaster looks up from a slate.",
    "\"Ten tons of hull plate for Verge Camp. It pays on delivery.\"",
    "1. Take the job",
    "2. Ask about the route",
    "3. Not now",
];

/// The page after asking about the route.
const ROUTE: [&str; 3] = [
    "\"Two gates lead to Verge. Either will do.\"",
    "1. Take it",
    "2. Pass",
];

/// The offer's first page on a later visit, once the route was asked
/// about: one more line, and the question gone.
const AGAIN: [&str; 5] = [
    "The dockmaster looks up from a slate.",
    "\"Ten tons of hull plate for Verge Camp. It pays on delivery.\"",
    "\"You know the way by now.\"",
    "1. Take the job",
    "2. Not now",
];

/// The fixture folder as the app opens it, with no problems.
fn opened() -> Rc<GameData> {
    let NewGameOpened::Loaded(loaded) =
        DiskData.open_new_game(&nova_data::new_game::fixture::folder())
    else {
        panic!("the fixture opens");
    };
    assert_eq!(loaded.problems, Vec::<String>::new());
    Rc::new(loaded.data)
}

/// Where the fixture's stellar `id` sits.
fn position_of(id: &str) -> Vec2 {
    let game = fixture();
    let nova = game
        .ids
        .stellars
        .iter()
        .find_map(|(bible, &nova)| (bible.as_str() == id).then_some(nova))
        .expect("a fixture stellar");
    let stellar = game
        .data
        .get::<Stellar>(nova)
        .expect("present")
        .expect("decoded")
        .record
        .clone();
    Vec2::new(f32::from(stellar.x_pos), f32::from(stellar.y_pos))
}

/// The app over the fixture, wired as `main` wires it, keeping pilots in
/// `store`, on the main menu.
fn app(store: &MemoryPilots) -> Harness {
    let rules = ruleset_rulebook(None, Ruleset::NewGame.profile().readings).0;
    Harness::wired(opened(), store, |screen| {
        screen
            .with_rulebook(&rules)
            .with_dialog_loader(nova::dialogs::loader(1))
    })
}

/// The app over the fixture, wired as `main` wires it, with a new pilot
/// flying in its start system, Hearth.
fn flying() -> Harness {
    flying_in(&MemoryPilots::new())
}

/// [`flying`], keeping the pilot, Ada, in `store`.
fn flying_in(store: &MemoryPilots) -> Harness {
    let mut game = app(store);
    let new_pilot = game.menu_button(MenuChoice::NewPilot);
    game.click(new_pilot);
    game.type_name("Ada");
    game.press(KeyCode::Enter);
    // The ship's christening, confirmed with its default.
    game.press(KeyCode::Enter);
    game.enter_ship();
    assert_eq!(game.showing(), Showing::Flight);
    game
}

/// A new app over `store`, with the pilot Ada opened through Open Pilot
/// and flying where she was saved.
fn resumed(store: &MemoryPilots) -> Harness {
    let mut game = app(store);
    let open_pilot = game.menu_button(MenuChoice::OpenPilot);
    game.click(open_pilot);
    assert_eq!(game.showing(), Showing::OpenPilot);
    game.press(KeyCode::Enter);
    game.enter_ship();
    assert_eq!(game.showing(), Showing::Flight);
    game
}

/// Puts the ship over the fixture's stellar `id` and lands there.
fn land_at(game: &mut Harness, id: &str) {
    let at = position_of(id);
    game.app
        .screen_mut()
        .flight_view_mut()
        .expect("flying")
        .change_sim(|sim| {
            nova_sim::fixture::put_player_at(sim.session_mut().expect("a session"), at);
        });
    game.land();
    assert_eq!(game.showing(), Showing::Spaceport, "landed at {id}");
}

/// The texts the next frame draws.
fn drawn(game: &mut Harness) -> Vec<String> {
    texts(&game.frame())
}

/// Whether `page`'s texts are drawn, in order, at the end of the frame:
/// over the spaceport.
fn ends_with(drawn: &[String], page: &[&str]) {
    let tail: Vec<&str> = drawn[drawn.len().saturating_sub(page.len())..]
        .iter()
        .map(String::as_str)
        .collect();
    assert_eq!(tail, page, "{drawn:?}");
}

#[test]
fn the_dock_s_offer_plays_through_choice_by_choice_and_closes_back_to_the_spaceport() {
    let mut game = flying();
    land_at(&mut game, "hearth-dock");
    let page = drawn(&mut game);
    ends_with(&page, &OFFER);

    game.press(KeyCode::Digit2);
    let page = drawn(&mut game);
    ends_with(&page, &ROUTE);
    assert!(!page.contains(&OFFER[0].to_owned()), "a new page: {page:?}");

    game.press(KeyCode::Digit1);
    let page = drawn(&mut game);
    ends_with(&page, &["\"Bay two, then.\"", CLOSE_HINT]);

    game.press(KeyCode::Enter);
    assert_eq!(game.showing(), Showing::Spaceport);
    let page = drawn(&mut game);
    for line in OFFER
        .iter()
        .chain(&ROUTE)
        .chain(&["\"Bay two, then.\"", CLOSE_HINT])
    {
        assert!(
            !page.contains(&(*line).to_owned()),
            "{line} still drawn: {page:?}"
        );
    }
}

#[test]
fn only_hearth_dock_names_a_conversation_once_opened_and_checked() {
    // Hearth Moon offers no services, so it can't be landed on, and Verge
    // Camp is a jump away: the router's own tests land at a stellar with
    // no conversation (`app::screen::tests::conversation`).
    let data = opened();
    let talking: Vec<i16> = data.dialogs().stellars.keys().map(|id| id.0).collect();
    let dock = fixture()
        .ids
        .stellars
        .iter()
        .find_map(|(bible, &nova)| (bible.as_str() == "hearth-dock").then_some(nova))
        .expect("Hearth Dock");
    assert_eq!(talking, [dock]);
}

#[test]
fn a_read_count_is_still_read_after_saving_quitting_and_loading() {
    let store = MemoryPilots::new();
    let mut game = flying_in(&store);
    land_at(&mut game, "hearth-dock");
    ends_with(&drawn(&mut game), &OFFER);
    game.press(KeyCode::Digit2);
    game.press(KeyCode::Digit1);
    game.press(KeyCode::Enter);
    assert_eq!(game.showing(), Showing::Spaceport);
    assert_eq!(game.handle(WindowEvent::CloseRequested), Control::Exit);

    let mut game = resumed(&store);
    land_at(&mut game, "hearth-dock");
    let page = drawn(&mut game);
    ends_with(&page, &AGAIN);
    assert!(
        !page.contains(&"2. Ask about the route".to_owned()),
        "{page:?}"
    );
}

#[test]
fn a_flag_in_the_pilot_sets_its_ink_variable() {
    let store = MemoryPilots::new();
    // A new pilot is saved at once, before she ever lands.
    drop(flying_in(&store));
    let mut pilot = saved(&store, "Ada");
    pilot.flags.set("dock_regular");
    store
        .write("Ada", &nova_sim::save::encode(&pilot))
        .expect("writes");

    let mut game = resumed(&store);
    land_at(&mut game, "hearth-dock");
    let page = drawn(&mut game);
    let mut regular = OFFER;
    regular[0] = "The dockmaster nods. \"You again.\"";
    ends_with(&page, &regular);
}
