//! Gravity and deadly stellars through a flight session, headless: a
//! system whose catalog gives a stellar `Gravity` pulls the player toward
//! it on each fight pass, and one whose catalog gives `Flags2` 0x0100
//! destroys the player touching it, as `_HandleGravity` and
//! `_HandleDeadlyStellars` do.

use nova_sim::deadly_stellar::DEADLY_STELLAR;
use nova_sim::gravity::{SPEED_MULT, pull};
use nova_sim::{LandingSite, PlayerFate, Session, SystemId, Vec2};
use nova_sim_catalog::fixture::{FakePilotCatalog, catalog};

/// The fixture catalog, whose ship starts over planet 128 at (30, -40) in
/// system 130, with `change` made to that system's stellars: 128, then
/// 129 at (2000, 0).
fn with(change: impl Fn(&mut [LandingSite])) -> FakePilotCatalog {
    let mut catalog = catalog();
    let (_, sites) = catalog
        .sites
        .iter_mut()
        .find(|(system, _)| *system == SystemId(130))
        .expect("system 130");
    change(sites);
    catalog
}

#[test]
fn a_gravity_stellar_from_the_catalog_pulls_the_player_on_each_fight_pass() {
    let catalog = with(|sites| sites[1].gravity = 40);
    let mut session = Session::start(&catalog).expect("starts");
    assert!(session.gravity_sources_present());
    let at = session.player().position;
    let was = session.player().velocity;
    crate::passes::work(&mut session, &catalog, nova_sim::Controls::default());
    let pulled = pull(at, Vec2::new(2000.0, 0.0), 40, SPEED_MULT);
    assert!(pulled.x > 0.0, "toward 129: {pulled:?}");
    let velocity = session.player().velocity;
    assert!((velocity - (was + pulled)).length() < 1e-6, "{velocity:?}");
}

#[test]
fn a_plain_system_has_no_gravity_source_and_pulls_nothing() {
    let catalog = catalog();
    let mut session = Session::start(&catalog).expect("starts");
    assert!(!session.gravity_sources_present());
    let was = session.player().velocity;
    crate::passes::work(&mut session, &catalog, nova_sim::Controls::default());
    assert_eq!(session.player().velocity, was);
}

#[test]
fn a_deadly_stellar_from_the_catalog_destroys_the_player_over_it() {
    let catalog = with(|sites| sites[0].flags2 = DEADLY_STELLAR);
    let mut session = Session::start(&catalog).expect("starts");
    assert_eq!(session.player_fate(), PlayerFate::Flying);
    crate::passes::work(&mut session, &catalog, nova_sim::Controls::default());
    assert_eq!(session.player_fate(), PlayerFate::Exploded);
}
