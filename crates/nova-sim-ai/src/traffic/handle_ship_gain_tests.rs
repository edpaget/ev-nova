//! Tests of the frame's gain as the traffic's flight reads it
//! ([`World::speed_mult`]), `_HandleShip`'s reads of `_gSpeedMult`: the
//! world's default, and the cloak's shield upkeep at the fly half's
//! begin (@0x341d5). Kept out of `traffic/mod.rs`, which is large.

#![allow(clippy::float_cmp)]

use std::collections::BTreeSet;

use super::{FlyHalf, Traffic, World};
use crate::cloak::{self, Cloak, CloakGear, FADE_FULL};
use crate::speed_mult::SpeedMult;
use crate::stats::ShipStats;
use crate::traffic::fixture::npc;
use nova_sim_base::fixture::FAST;

const HALF: SpeedMult = SpeedMult::new(0.5);

#[test]
fn a_bare_world_flies_at_a_full_frames_gain() {
    assert_eq!(World::new(&[]).speed_mult, SpeedMult::FULL);
}

#[test]
fn at_half_gain_a_cloaked_npc_pays_half_its_cloaks_shield_usage() {
    let mut ship = npc(1, ShipStats::new(FAST, &[]));
    ship.cloak = Cloak {
        fade: FADE_FULL,
        direction: 0,
    };
    ship.cloak_gear = CloakGear {
        shield_usage: 8,
        ..CloakGear::default()
    };
    ship.reserves.shield.now = 100.0;
    let mut traffic = Traffic {
        npcs: vec![ship],
        ..Traffic::default()
    };
    let world = World {
        speed_mult: HALF,
        ..World::new(&[])
    };
    let _pass = traffic.begin_fly(&world, &BTreeSet::new(), FlyHalf::Advance);
    let mut paid = 100.0;
    cloak::drain_shield(&mut paid, 8, true, HALF);
    assert_eq!(traffic.npcs()[0].reserves.shield.now, paid);
    assert!(paid > 100.0 - 8.0 * 0.03333, "half the usage: {paid}");
}
