//! The traffic taking a think's leave write ([`Decision::leaves`]) as it
//! takes the goal: `_AIMakeShipLeave`'s inlined +0x50 floor and +0xa4
//! stamp ([`leave_write`](super::leave_write)), made only where the think
//! wrote goal 2.
#![allow(clippy::float_cmp)]

use super::npc::{AiType, Npc, NpcId};
use super::{Traffic, World};
use crate::ai::fixture::{TRADERS, ship};
use crate::ai::{Behaviour, Decision, Goal, Surroundings};
use crate::chance::{Chance, NeverFires};

/// Makes its decision for everyone.
#[derive(Debug)]
struct Deciding(Decision);

impl Behaviour for Deciding {
    fn decide(&self, _: &Npc, _: &Surroundings, _: &mut dyn Chance) -> Goal {
        self.0.goal
    }

    fn decision(&self, _: &Npc, _: &Surroundings, _: &mut dyn Chance) -> Decision {
        self.0
    }
}

/// A traffic of an idle trader at the centre whose jump was given up
/// (+0x50 = -1.0), its clock 9 passes old.
fn given_up() -> (Traffic, NpcId) {
    let mut traffic = Traffic::with_interval(1);
    let mut npc = ship(0, TRADERS, AiType::WimpyTrader, 0.0, 0.0);
    npc.warp.give_up();
    npc.warp.since = 9;
    let id = traffic.add_npc_at(npc, 1);
    (traffic, id)
}

fn warp_after(leaves: bool) -> (f32, u32) {
    let (mut traffic, id) = given_up();
    let decision = Decision {
        goal: Goal::JumpOut,
        leaves,
        ..Decision::default()
    };
    traffic.tick_in(&Deciding(decision), World::new(&[]), &[], &mut NeverFires);
    let npc = traffic
        .npcs()
        .iter()
        .find(|npc| npc.id == id)
        .expect("still here");
    assert_eq!(npc.goal, Goal::JumpOut);
    (npc.warp.stage, npc.warp.since)
}

#[test]
fn a_think_that_writes_the_leave_floors_its_stage_and_stamps_its_clock() {
    assert_eq!(warp_after(true), (0.0, 0), "+0x50 floored, +0xa4 stamped");
}

#[test]
fn a_think_that_keeps_its_leave_keeps_its_stage_and_clock() {
    assert_eq!(
        warp_after(false),
        (-1.0, 10),
        "no write: the given-up stage stands, the clock moved on a tick"
    );
}
