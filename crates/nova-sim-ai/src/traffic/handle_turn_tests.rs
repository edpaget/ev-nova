//! Tests of `_HandleShip`'s turn (@0x34424-0x3458e) for the NPC flights
//! the autopilot flies ([`autopilot::fly`]): every one wants a
//! whole-degree heading (+0x68) and is turned to it by
//! [`flight::turn_to`](crate::flight::turn_to), by its turn while more
//! than that off in whole degrees and onto it otherwise; one that wants
//! none has its heading truncated, the low level's default (@0x8522d).
//! While its AI wait runs the turn is skipped, but for a count wait
//! (@0x3443f-0x34454).
#![allow(clippy::float_cmp)]

use super::autopilot::{self, Beside, Outcome, Readings, TopSpeeds};
use super::npc::Npc;
use crate::ai::Goal;
use crate::combat::ShipRef;
use crate::flight::{FlightRules, ShipState};
use crate::geometry::Vec2;
use crate::handling::ShipFields;
use crate::stats::{INERTIALESS_HULL, ShipStats};
use nova_sim_base::fixture::{Draws, FAST};
use std::collections::BTreeMap;

/// [`FAST`] with an inertialess hull.
const RAVEN: ShipFields = ShipFields {
    flags2: INERTIALESS_HULL,
    ..FAST
};

fn at(x: f32, y: f32, vx: f32, vy: f32, heading: f32) -> ShipState {
    ShipState {
        position: Vec2::new(x, y),
        velocity: Vec2::new(vx, vy),
        heading,
    }
}

/// An NPC of `fields` on `goal` at `start`, holding `speed` along its
/// heading.
fn ship(fields: ShipFields, goal: Goal, start: ShipState, speed: f32) -> Npc {
    Npc {
        state: start,
        goal,
        speed,
        ..super::fixture::npc(1, ShipStats::new(fields, &[]))
    }
}

/// Flies `npc` one tick at its own top speed beside `other`, under the
/// original's rules.
fn fly(npc: &mut Npc, other: Option<&ShipState>) -> Outcome {
    let speeds = TopSpeeds::own_of(npc);
    let weapons = crate::ai::fixture::records_of([&*npc]);
    autopilot::fly(
        npc,
        &[],
        &BTreeMap::new(),
        other,
        false,
        Beside::default(),
        speeds,
        FlightRules::default(),
        Readings::default(),
        &weapons,
        &mut Draws::of(&[]),
    )
}

#[test]
fn a_moving_inertialess_sniper_braking_or_bursting_has_its_heading_truncated() {
    // Sub 0xe's inertialess brake writes no +0x68 (@0x8710f-0x8713a), so
    // `_HandleShip` truncates the heading, whether the brake or an owed
    // burst flies it.
    let target = at(500.0, 0.0, 0.0, 0.0, 0.0);
    for owed in [None, Some(30.0)] {
        let mut sniper = ship(
            RAVEN,
            Goal::Snipe(ShipRef::Player),
            at(0.0, 0.0, 0.0, -3.0, 40.6),
            3.0,
        );
        sniper.owed_burst = owed;
        assert_eq!(fly(&mut sniper, Some(&target)), Outcome::Flying);
        assert_eq!(sniper.state.heading, 40.0, "owing {owed:?}");
    }
}
