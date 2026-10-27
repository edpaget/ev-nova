//! The session's tests of when an NPC's shot first moves:
//! `_DoPlayGameWork` runs `_HandleShot` (@0x44385) before the ship loop's
//! `_HandleShip` (@0x44483) fires, so a shot an NPC fires waits at its
//! launch point the pass it is fired and first moves in the next pass.

use super::tests::{Firing, armed, facing_an_npc};
use super::*;
use crate::chance::NeverFires;
use crate::combat::Rules;
use crate::flight::Controls;
use crate::testkit::FakePilotCatalog;

/// A pass of `session` in flight, by Nova's rules, its NPCs holding their
/// triggers.
fn pass(session: &mut Session, catalog: &FakePilotCatalog) {
    let rescued = session.game_work(
        Controls::default(),
        catalog,
        &Firing,
        Rules::default(),
        &mut NeverFires,
    );
    assert!(!rescued);
}

#[test]
fn an_npcs_shot_waits_at_its_launch_point_the_pass_it_is_fired() {
    let catalog = armed();
    let mut session = facing_an_npc(&catalog, 180);
    let npc = ShipRef::Npc(NpcId(0));
    pass(&mut session, &catalog);
    let shot = session
        .shots()
        .iter()
        .find(|shot| shot.firer == npc)
        .expect("fired");
    let (id, launched) = (shot.id, shot.position);
    assert_eq!(shot.moved_from, None, "unmoved");
    assert_eq!(launched, session.npcs()[0].state.position, "where it is");
    pass(&mut session, &catalog);
    let shot = session
        .shots()
        .iter()
        .find(|shot| shot.id == id)
        .expect("in flight");
    assert_eq!(shot.moved_from, Some(launched), "its first move");
    assert_ne!(shot.position, launched);
}
