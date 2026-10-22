//! The galaxy map's Find over the stock data
//! (`Session::find_map_system`, `_DoSystemSearch`). Skips, passing, when
//! `NOVA_DATA` is unset.
//!
//! - A new stock pilot starts in Kania (`sÿst` 128), which its Find
//!   finds by name.
//! - Sol (130) and Kanius (461) are visible but unexplored, so neither is
//!   found: "kan" finds Kania alone.
//! - Sol explored before the pilot flies is found, as the flight's first
//!   rebuild makes it known.

use crate::common;

use nova_data::GameData;
use nova_sim::map_search::SearchOutcome;
use nova_sim::pilot::ExploredLevel;
use nova_sim::{Pilot, Session, SystemId};

const KANIA: SystemId = SystemId(128);
const SOL: SystemId = SystemId(130);
const KANIUS: SystemId = SystemId(461);

fn opened() -> Option<GameData> {
    let dir = common::nova_data()?;
    Some(GameData::open(&dir, None).expect("the stock data opens"))
}

/// A new stock pilot, changed by `change`, flying.
fn flown(data: &GameData, change: impl FnOnce(&mut Pilot)) -> Session {
    let mut pilot = Pilot::new(data, "").expect("the stock first chär starts");
    change(&mut pilot);
    Session::fly(data, pilot).expect("flies")
}

#[test]
fn a_stock_pilot_finds_its_start_system_by_name() {
    let Some(data) = opened() else {
        return;
    };
    let session = flown(&data, |_| {});
    assert_eq!(session.system(), KANIA);
    assert_eq!(
        session.find_map_system("KANIA"),
        SearchOutcome::Found(KANIA)
    );
}

#[test]
fn a_system_the_pilot_has_not_explored_is_not_found() {
    let Some(data) = opened() else {
        return;
    };
    let session = flown(&data, |_| {});
    for system in [SOL, KANIUS] {
        assert!(session.galaxy().visible(system), "{system:?}");
        assert_eq!(
            session.pilot().explored_level(system),
            ExploredLevel::Unexplored
        );
    }
    assert_eq!(session.find_map_system("sol"), SearchOutcome::NotFound);
    assert_eq!(
        session.find_map_system("kanius"),
        SearchOutcome::Found(KANIA)
    );
    assert_eq!(session.find_map_system("kan"), SearchOutcome::Found(KANIA));
}

#[test]
fn a_system_explored_before_flying_is_found() {
    let Some(data) = opened() else {
        return;
    };
    let session = flown(&data, |pilot| {
        pilot.explore(SOL);
    });
    assert!(session.is_known(SOL), "the rebuild knows it");
    assert_eq!(session.find_map_system("sol"), SearchOutcome::Found(SOL));
}
