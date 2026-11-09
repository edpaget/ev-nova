//! The developer galaxy viewer over the new game's checked-in fixture
//! (`nova_data::new_game::fixture`), wired as the app wires it: the
//! mapped `GameData` in an `AppScreen`, with the rulebook
//! `ruleset_rulebook` gives `Ruleset::NewGame`.
//!
//! - **The new game's rules** draw a fresh pilot's gate lanes, each in
//!   the look a fresh session's map gives it, and its closed lanes.
//! - **The stock rulebook** draws the Con links, which the new game has
//!   none of, as the viewer always has.
//!
//! Which links the viewer chooses by which rule is nova-sim's
//! (`viewer_lanes`), pinned there; this pins that the data, the ruleset
//! and the app meet.

use std::collections::BTreeSet;
use std::rc::Rc;

use nova::app::AppScreen;
use nova_sim::galaxy_map::MapLink;
use nova_sim::{Rulebook, SystemId};

use crate::new_game_routes::{flying_from, look_between};
use crate::new_game_travel::{fixture, new_game_rules};

fn pairs(links: &[MapLink]) -> Vec<(SystemId, SystemId)> {
    links.iter().map(|link| (link.from, link.to)).collect()
}

#[test]
fn the_new_games_rules_draw_its_gate_lanes_on_the_viewer() {
    let game = fixture();
    let rules = new_game_rules();
    let data = Rc::new(fixture().data);
    let screen = AppScreen::new(Rc::clone(&data)).with_rules(rules);
    let picture = screen
        .navigator()
        .map()
        .picture()
        .expect("the viewer's")
        .clone();
    let gates = nova_sim::viewer_lanes::gate_links(&*data);
    let links = pairs(&picture.links);
    assert_eq!(links, gates.links);
    assert!(!links.is_empty(), "the new game has gate lanes");

    let start = links[0].0;
    let session = flying_from(&game, &rules, start);
    let galaxy = session.galaxy();
    let lanes: BTreeSet<(SystemId, SystemId)> = galaxy
        .catalog_systems()
        .flat_map(|from| {
            galaxy
                .lanes()
                .listed(from)
                .iter()
                .map(move |&to| (from.min(to), from.max(to)))
                .collect::<Vec<_>>()
        })
        .collect();
    assert_eq!(links.iter().copied().collect::<BTreeSet<_>>(), lanes);

    assert_eq!(picture.lanes.len(), links.len());
    for (look, &(from, to)) in picture.lanes.iter().zip(&links) {
        assert_eq!(*look, look_between(&session, from, to));
    }
    let closed: BTreeSet<(SystemId, SystemId)> = galaxy
        .catalog_systems()
        .flat_map(|from| {
            galaxy
                .lanes()
                .closed(from)
                .into_iter()
                .map(move |to| (from.min(to), from.max(to)))
        })
        .filter(|pair| !lanes.contains(pair))
        .collect();
    let drawn: BTreeSet<(SystemId, SystemId)> = picture
        .closed_lanes
        .iter()
        .map(|look| (look.from.min(look.to), look.from.max(look.to)))
        .collect();
    assert_eq!(drawn, closed);
    for look in &picture.closed_lanes {
        assert_eq!(*look, look_between(&session, look.from, look.to));
    }
}

#[test]
fn the_stock_rules_draw_the_new_games_con_links_which_are_none() {
    let data = Rc::new(fixture().data);
    let screen = AppScreen::new(data).with_rules(Rulebook::default());
    let picture = screen.navigator().map().picture().expect("the viewer's");
    assert_eq!(picture.links, []);
    assert_eq!(picture.lanes, []);
    assert_eq!(picture.closed_lanes, []);
}
