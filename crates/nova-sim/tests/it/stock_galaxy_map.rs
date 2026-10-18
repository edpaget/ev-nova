//! The galaxy map's drawing over the stock data: a new pilot knows its
//! start system's neighbours and nothing far off, its start system is
//! framed blue (the player may land), and the map draws only what is
//! known or explored. Skips, passing, when `NOVA_DATA` is unset.

use crate::common;

use std::collections::BTreeSet;

use nova_data::GameData;
use nova_sim::iff::SystemColorCode;
use nova_sim::{NeverFires, Session, SystemId};

/// A new stock pilot, begun as the game begins one.
fn begun(data: &GameData) -> Session {
    let mut session = Session::start(data).expect("the stock first chär starts");
    session.begin(data, &mut NeverFires);
    session
}

#[test]
fn a_new_stock_pilot_knows_its_neighbours_and_nothing_far_off() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let session = begun(&data);
    let galaxy = session.galaxy();
    let start = session.system();
    let neighbours: BTreeSet<SystemId> = galaxy
        .con_slots(start)
        .iter()
        .filter(|&&slot| galaxy.exists(slot))
        .filter_map(|&slot| galaxy.same_place(slot))
        .collect();
    assert!(!neighbours.is_empty());
    let known = session.known();
    assert!(known.contains(&start));
    assert!(neighbours.is_subset(known), "{neighbours:?} in {known:?}");
    // A system no explored system lists, at no place of one, is unknown.
    let listed: BTreeSet<SystemId> = session
        .pilot()
        .explored()
        .flat_map(|explored| galaxy.con_slots(explored).to_vec())
        .filter(|&slot| galaxy.exists(slot))
        .filter_map(|slot| galaxy.same_place(slot))
        .collect();
    let far: Vec<SystemId> = galaxy
        .catalog_systems()
        .filter(|&system| galaxy.visible(system))
        .filter(|system| !listed.contains(system) && !session.pilot().has_explored(*system))
        .collect();
    assert!(far.len() > 100, "most of the galaxy is far off");
    assert!(far.iter().all(|system| !session.is_known(*system)));
    let drawing = session.map_drawing(None, &[], nova_sim::MapMarks::NONE);
    let drawn: BTreeSet<SystemId> = drawing.systems.iter().map(|system| system.id).collect();
    assert!(drawn.contains(&start));
    assert!(
        far.iter().all(|system| !drawn.contains(system)),
        "none far drawn"
    );
    assert_eq!(drawing.current, start);
}

#[test]
fn the_stock_start_system_is_framed_blue() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let session = begun(&data);
    assert_eq!(
        session.system_color_code(session.system()),
        SystemColorCode::Cleared
    );
    let drawing = session.map_drawing(None, &[], nova_sim::MapMarks::NONE);
    let start = drawing
        .systems
        .iter()
        .find(|system| system.id == session.system())
        .expect("the start system drawn");
    assert_eq!(start.code, SystemColorCode::Cleared);
}
