//! Tests that a successor taking over its lost lead's follow of a jump
//! keeps sub 4 through the take-over's dispatch, as `_HighLevelAIHandler`'s
//! block order has it (in the `EV Nova` executable).
//!
//! Under the lead's government the marking pass copies goal 0xb with
//! +0x9a = -1 (`_AICheckParentExists` @0x893b0-0x893e1, @0x8946b). In the
//! successor's dispatch goal 2's block (@0x8dd99-0x8de57) is tested first,
//! against goal 0xb, and skipped; goal 0xb's block (@0x8de63) then writes
//! goal 2, sub 4 (@0x8df54-0x8df6c), and the low level runs sub 4
//! (@0x8585b-0x859c5). Goal 2's block first reads it on the next dispatch:
//! sub 3 inside 1000 (@0x8fb1e), sub 1 beyond it moving (@0x8de49).
//!
//! Under another government the check itself writes goal 2, sub 4
//! (@0x8940c-0x8941e), so goal 2's block runs on the take-over dispatch.
#![allow(clippy::float_cmp)]

use std::collections::BTreeSet;

use super::npc::{Npc, NpcId};
use super::warp_out::WarpGoal;
use super::{Traffic, World};
use crate::ai::{Behaviour, Goal, Surroundings};
use crate::catalog::GovtId;
use crate::chance::{Chance, NeverFires};
use crate::combat::ShipRef;
use crate::combat::hull::Condition;
use crate::geometry::Vec2;
use crate::speed_mult::SpeedMult;
use crate::stats::ShipStats;
use crate::traffic::fixture::npc;
use crate::trig::intercept_angle;
use nova_sim_base::fixture::FAST;

/// Every ship keeps its goal and target.
#[derive(Debug)]
struct Keep;

impl Behaviour for Keep {
    fn decide(&self, npc: &Npc, _: &Surroundings, _: &mut dyn Chance) -> Goal {
        npc.goal
    }

    fn target(&self, npc: &Npc, _: &Surroundings) -> Option<ShipRef> {
        npc.target
    }
}

const FEDS: Option<GovtId> = Some(GovtId(128));
const REBELS: Option<GovtId> = Some(GovtId(129));

/// A [`FAST`] ship of `govt` and class mass `mass`, a trader of a
/// warship's class (`InherentAI` 3), at the centre.
fn ship(govt: Option<GovtId>, mass: i16) -> Npc {
    let mut ship = npc(0, ShipStats::new(FAST, &[]));
    ship.govt = govt;
    ship.hull.signed_mass = mass;
    ship.hull.inherent_ai = 3;
    ship
}

/// A world whose warp sound runs 600 Mac ticks, so no jump ends at its
/// slot.
fn world() -> World<'static> {
    let mut world = World::new(&[]);
    world.warp.durations = [600, 600];
    world
}

/// A rebel lead, disabled at berth 1, following the jump of a far ship at
/// berth 5 (goal 0xb, sub 0xd), and its follower of `govt` at berth 2, at
/// `position` facing 90 at `velocity`: the follower takes over at the
/// next marking pass. The traffic and the follower.
fn taking_over_a_follow(govt: Option<GovtId>, position: Vec2, velocity: Vec2) -> (Traffic, NpcId) {
    let mut traffic = Traffic::with_interval(1);
    let mut far = ship(REBELS, 900);
    far.state.position = Vec2::new(5000.0, 0.0);
    far.begin_own_jump();
    let far = traffic.add_npc_at(far, 5);
    let lead = Npc {
        condition: Condition::Disabled,
        leader: Some(far),
        goal: Goal::Follow {
            lead: far,
            guard: None,
        },
        ..ship(REBELS, 500)
    };
    let mut lead = lead;
    lead.warp.goal = Some(WarpGoal::Follow(ShipRef::Npc(far)));
    let lead = traffic.add_npc_at(lead, 1);
    let mut follower = Npc {
        leader: Some(lead),
        goal: Goal::Follow { lead, guard: None },
        ..ship(govt, 300)
    };
    if govt != REBELS {
        // On its lead's jump: the other government's goal 0xb.
        follower.warp.goal = Some(WarpGoal::Follow(ShipRef::Npc(lead)));
    }
    follower.state.position = position;
    follower.state.velocity = velocity;
    follower.state.heading = 90.0;
    let follower = traffic.add_npc_at(follower, 2);
    (traffic, follower)
}

fn of(traffic: &Traffic, id: NpcId) -> &Npc {
    (traffic.npcs().iter())
        .find(|npc| npc.id == id)
        .expect("there")
}

fn think(traffic: &mut Traffic) {
    traffic.think_holding(&Keep, world(), &[], &mut NeverFires);
}

/// Asserts that `taker` runs sub 4 at its slot this dispatch: goal 2 on
/// its own jump, the stage begun and grown by the gain (@0x858a4-0x858f9),
/// +0x68 outward (@0x85892-0x8589e).
fn runs_sub_4(taker: &Npc) {
    assert_eq!(taker.goal, Goal::JumpOut);
    assert_eq!(taker.warp.goal, Some(WarpGoal::Own), "sub 4");
    assert_eq!(taker.warp.stage, SpeedMult::FULL.add_times(1.0, 1.0));
    assert_eq!(
        taker.warp.aim,
        intercept_angle(Vec2::ZERO, taker.state.position)
    );
}

#[test]
fn inside_1000_a_successor_on_a_copied_follow_jumps_on_the_take_over_frame_and_runs_out_on_the_next()
 {
    let (mut traffic, taker) = taking_over_a_follow(REBELS, Vec2::new(300.0, 0.0), Vec2::ZERO);
    think(&mut traffic);
    runs_sub_4(of(&traffic, taker));
    think(&mut traffic);
    let ship = of(&traffic, taker);
    assert_eq!(ship.warp.goal, None, "goal 2's block wrote sub 3");
    assert!(ship.warp.under_way(), "its stage runs on");
    let word = traffic.word_of(taker).expect("kept");
    assert!(!word.thrusts(), "the run-out under way takes no thrust");
    assert_eq!(
        word.wanted(),
        Some(intercept_angle(Vec2::ZERO, ship.state.position)),
        "outward"
    );
}

#[test]
fn beyond_1000_moving_a_successor_on_a_copied_follow_jumps_on_the_take_over_frame_and_brakes_on_the_next()
 {
    let (mut traffic, taker) =
        taking_over_a_follow(REBELS, Vec2::new(2000.0, 0.0), Vec2::new(1.0, 0.0));
    think(&mut traffic);
    runs_sub_4(of(&traffic, taker));
    think(&mut traffic);
    let ship = of(&traffic, taker);
    assert_eq!(ship.warp.goal, None, "goal 2's block wrote sub 1");
    assert!(ship.warp.under_way());
}

#[test]
fn beyond_1000_at_rest_a_successor_jumps_on_both_frames() {
    let (mut traffic, taker) = taking_over_a_follow(REBELS, Vec2::new(2000.0, 0.0), Vec2::ZERO);
    think(&mut traffic);
    runs_sub_4(of(&traffic, taker));
    think(&mut traffic);
    let ship = of(&traffic, taker);
    assert_eq!(
        ship.warp.goal,
        Some(WarpGoal::Own),
        "goal 2's block keeps sub 4"
    );
    assert!(ship.warp.under_way());
}

#[test]
fn under_another_government_a_successor_following_its_leads_jump_runs_out_on_the_take_over_frame() {
    let (mut traffic, taker) = taking_over_a_follow(FEDS, Vec2::new(300.0, 0.0), Vec2::ZERO);
    think(&mut traffic);
    let ship = of(&traffic, taker);
    assert_eq!(ship.goal, Goal::JumpOut);
    assert_eq!(ship.warp.goal, None, "goal 2's block wrote sub 3");
    assert!(!ship.warp.under_way(), "no sub 4 ran");
}

#[test]
fn the_marker_lives_one_dispatch() {
    // Held still on the take-over's tick, it runs no low level there; on
    // the next dispatch goal 2's block reads it as any goal 2 ship.
    let (mut traffic, taker) = taking_over_a_follow(REBELS, Vec2::new(300.0, 0.0), Vec2::ZERO);
    let still = BTreeSet::from([taker]);
    traffic.tick_holding(&Keep, world(), &[], &still, &mut NeverFires);
    let ship = of(&traffic, taker);
    assert_eq!(
        ship.warp.goal,
        Some(WarpGoal::Own),
        "sub 4 kept at its slot"
    );
    assert!(!ship.warp.under_way(), "no low level ran");
    assert!(
        !ship.warp.sub_4_past_leave_block,
        "cleared at its slot's end"
    );
    think(&mut traffic);
    let ship = of(&traffic, taker);
    assert_eq!(ship.warp.goal, None, "goal 2's block wrote sub 3");
    assert!(!ship.warp.under_way(), "it runs out before it jumps");
}
