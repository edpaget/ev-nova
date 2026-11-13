//! The port services the new game has nothing behind
//! (`RuleKey::EmptyServices`), over the checked-in fixture
//! (`nova_data::new_game::fixture::folder`) with no interface file, the
//! app wired as `main` wires the new game.
//!
//! The fixture's only `mïsn` are hidden blockades (`AvailRandom` 0) and
//! no `shïp` is for hire (`HireRandom` 0), so at Hearth Dock the mission
//! computer has nothing to list and the bar no ship for hire.
//!
//! - **The new game's reading.** Hearth Dock's Mission BBS is shown
//!   greyed, and neither a click nor its key opens anything; its bar's
//!   Hire Escort is greyed, and a click asks for nothing.
//! - **The engine reading.** With the key back at the original's
//!   reading, and the rest of the new game's rulebook as it is, the BBS
//!   is enabled and says there is nothing here (`STR#` 2002 #353) in a
//!   text that closes back to the spaceport, and Hire Escort is enabled
//!   and says there are no ships for hire (#224), as before the key.
//!
//! The parts are pinned below: nova-sim's `empty_services` decides,
//! nova-view's spaceport greys, and the app's unit tests carry the
//! wiring over hand-built data.

use nova::rulebook::ruleset_rulebook;
use nova::ruleset::Ruleset;
use nova_data::new_game::fixture::folder;
use nova_sim::fixture::MemoryPilots;
use nova_sim::hire::NONE_FOR_HIRE;
use nova_sim::{ComputerRefusal, PilotStore, RuleKey, RuleSource, Rulebook, Service};
use nova_view::spaceport::bar::HIRE_ITEM;
use nova_view::spaceport::layout::service_item;
use winit::keyboard::KeyCode;

use crate::new_game_conversation::{land_at, new_pilot_flying, opened_from, resumed_in};
use crate::new_game_port::{centre_of, click_port_item, on_the_dock_job, open_over, spaceport};
use crate::pilots::{Harness, saved, texts};
use nova::app::Showing;

/// The new game's rulebook, as `main` builds it.
fn new_game_rules() -> Rulebook {
    ruleset_rulebook(None, Ruleset::NewGame.profile().readings).0
}

/// [`new_game_rules`] with `RuleKey::EmptyServices` back at the
/// original's reading: the rulebook the new game ran under before the key.
fn engine_rules() -> Rulebook {
    new_game_rules().with_override(RuleKey::EmptyServices, RuleSource::Engine)
}

/// The app over the fixture with no interface file, its rules `rules`,
/// keeping pilots in `store`, on the main menu.
fn app_by(rules: &Rulebook, store: &MemoryPilots) -> Harness {
    Harness::wired_without_interface(opened_from(&folder()), store, |screen| {
        screen
            .with_rulebook(rules)
            .with_dialog_loader(nova::dialogs::loader(1))
    })
}

/// Ada on the dock job, so Hearth Dock offers it no more, landed at
/// Hearth Dock by `rules`.
fn at_hearth_dock_by(rules: &Rulebook) -> Harness {
    let store = MemoryPilots::new();
    drop(new_pilot_flying(app_by(rules, &store)));
    let mut pilot = saved(&store, "Ada");
    on_the_dock_job(&mut pilot);
    store
        .write("Ada", &nova_sim::save::encode(&pilot))
        .expect("writes");
    let mut game = resumed_in(app_by(rules, &store));
    land_at(&mut game, "hearth-dock");
    game.frame();
    game
}

fn bbs_enabled(game: &Harness) -> bool {
    spaceport(game)
        .dialog()
        .expect("laid out")
        .item_enabled(service_item(Service::MissionBbs))
}

fn hire_enabled(game: &Harness) -> bool {
    spaceport(game)
        .open_bar()
        .and_then(nova_view::spaceport::BarScreen::dialog)
        .expect("the bar laid out")
        .item_enabled(HIRE_ITEM)
}

/// Whether the next frame draws `text`.
fn draws(game: &mut Harness, text: &str) -> bool {
    texts(&game.frame()).contains(&text.to_owned())
}

#[test]
fn hearth_dock_s_mission_bbs_is_greyed_and_opens_nothing_by_click_or_key() {
    let none_here = ComputerRefusal::NoneAvailable.message();
    let mut game = at_hearth_dock_by(&new_game_rules());
    assert!(
        spaceport(&game).offered().contains(&Service::MissionBbs),
        "shown: any inhabited stellar has one"
    );
    assert!(!bbs_enabled(&game), "greyed");
    click_port_item(&mut game, service_item(Service::MissionBbs));
    assert!(!draws(&mut game, &none_here), "a click says nothing");
    assert_eq!(open_over(&game), "nothing", "a click opens nothing");
    game.press(KeyCode::KeyN);
    assert!(!draws(&mut game, &none_here), "its key says nothing");
    assert_eq!(open_over(&game), "nothing", "its key opens nothing");
    assert_eq!(game.showing(), Showing::Spaceport);
}

#[test]
fn hearth_dock_s_hire_escort_is_greyed_and_asks_for_nothing() {
    let mut game = at_hearth_dock_by(&new_game_rules());
    click_port_item(&mut game, service_item(Service::Bar));
    game.frame();
    assert_eq!(open_over(&game), "bar");
    assert!(!hire_enabled(&game), "greyed");
    let hire = centre_of(
        spaceport(&game).open_bar().and_then(|bar| bar.dialog()),
        HIRE_ITEM,
    );
    game.click(hire);
    assert!(!draws(&mut game, NONE_FOR_HIRE), "says nothing");
    assert!(spaceport(&game).mission_text().is_none(), "no text");
    assert_eq!(open_over(&game), "bar", "still the bar");
}

/// By the engine reading the BBS is enabled and says #353 by a click or
/// its key, each closing back to the spaceport: the walk the new game's
/// port took before the key.
#[test]
fn by_the_engine_reading_the_mission_bbs_says_there_is_nothing_here_and_closes_back() {
    let none_here = ComputerRefusal::NoneAvailable.message();
    let mut game = at_hearth_dock_by(&engine_rules());
    assert!(bbs_enabled(&game), "enabled");
    click_port_item(&mut game, service_item(Service::MissionBbs));
    assert!(draws(&mut game, &none_here), "a click");
    assert_eq!(open_over(&game), "text");
    game.press(KeyCode::Enter);
    game.frame();
    assert_eq!(open_over(&game), "nothing", "closed");
    game.press(KeyCode::KeyN);
    assert!(draws(&mut game, &none_here), "its key");
    assert_eq!(open_over(&game), "text");
    game.press(KeyCode::Enter);
    game.frame();
    assert_eq!(open_over(&game), "nothing", "closed");
    assert_eq!(game.showing(), Showing::Spaceport);
}

/// By the engine reading Hire Escort is enabled with room and says #224.
#[test]
fn by_the_engine_reading_hire_escort_says_there_are_no_ships_for_hire() {
    let mut game = at_hearth_dock_by(&engine_rules());
    click_port_item(&mut game, service_item(Service::Bar));
    game.frame();
    assert!(hire_enabled(&game), "enabled");
    let hire = centre_of(
        spaceport(&game).open_bar().and_then(|bar| bar.dialog()),
        HIRE_ITEM,
    );
    game.click(hire);
    assert!(draws(&mut game, NONE_FOR_HIRE), "#224");
    assert!(spaceport(&game).mission_text().is_some(), "in a text");
}
