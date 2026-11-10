//! Tests that a ship placed in the system starts with its turrets facing
//! the nose ([`Npc::turret_facing`]), whatever record it was placed from:
//! a facing never carries to a ship placed anew. Kept out of
//! `traffic/mod.rs`, which is large.

use super::Traffic;
use super::fixture::npc;
use crate::stats::ShipStats;

#[test]
fn a_ship_placed_from_a_faced_record_faces_the_nose() {
    let mut faced = npc(0, ShipStats::default());
    faced.turret_facing.aim(40.0, 9);
    assert_eq!(faced.turret_facing.heading(), Some(40.0));
    let mut traffic = Traffic::with_interval(1);
    let id = traffic.add_npc_at(faced, 1);
    let placed = traffic
        .npcs()
        .iter()
        .find(|npc| npc.id == id)
        .expect("placed");
    assert_eq!(placed.turret_facing.heading(), None);
}
