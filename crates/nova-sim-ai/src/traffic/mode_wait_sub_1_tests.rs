//! Tests that a ship whose stored sub 1 stands through a mode wait (a
//! ship holding its course, [`Mode::Coasting`], or held at its gate,
//! [`Mode::HeldAtGate`]) is cut by sub 1 under it, as under the AI wait:
//! `_AIDispatch` tail-jumps to the low level whatever set +0x4c
//! (@0x901ea-0x901f4), which runs sub 1 on its kept value (@0x8536d),
//! while `_HandleShip` skips the turn and thrust (@0x34a54-0x34a72). An
//! inertial ship on the soft brake within its retro heading is cut × 0.94
//! (@0x85557-0x85588), off it only goal 9 is (@0x85592-0x855e1); below
//! 0.35 on both axes it settles × 0.95 and sub 0 is stored
//! (@0x855e3-0x85624). Stock reaches it through a successor held at its
//! gate, which copies its lost lead's sub (@0x893c8-0x893e1).
#![allow(clippy::float_cmp)]

use super::autopilot::{self, Beside, Outcome, Readings, TopSpeeds};
use super::npc::{Mode, Npc, NpcId};
use crate::ai::{Behaviour, Goal, Surroundings};
use crate::chance::{Chance, NeverFires};
use crate::combat::ShipRef;
use crate::combat::hull::Condition;
use crate::flight::{FlightRules, ShipState, drag_by};
use crate::geometry::Vec2;
use crate::hail::Help;
use crate::handling::{Handling, ShipFields};
use crate::rulebook::RuleSource;
use crate::speed_mult::SpeedMult;
use crate::stats::{INERTIALESS_HULL, ShipStats};
use crate::traffic::{Traffic, World};
use nova_sim_base::fixture::{Draws, FAST};
use std::collections::BTreeMap;

/// [`FAST`] with an inertialess hull.
const RAVEN: ShipFields = ShipFields {
    flags2: INERTIALESS_HULL,
    ..FAST
};

/// The mode waits a stored sub 1 stands through, 9 ticks from their end.
const MODE_WAITS: [Mode; 2] = [
    Mode::Coasting { ticks_left: 9.0 },
    Mode::HeldAtGate { ticks_left: 9.0 },
];

/// A ship of `fields` on `goal` in `mode` moving at `velocity` facing
/// `heading`, 2.0 along it, with no AI wait and sub 1 stored.
fn waiting_in(mode: Mode, fields: ShipFields, goal: Goal, velocity: Vec2, heading: f32) -> Npc {
    Npc {
        state: ShipState {
            position: Vec2::new(10.0, 20.0),
            velocity,
            heading,
        },
        goal,
        mode,
        speed: 2.0,
        ai_wait: 0.0,
        stopping: true,
        ..super::fixture::npc(1, ShipStats::new(fields, &[]))
    }
}

/// Flies `npc` one tick at its own top speed beside nothing, under the
/// original's rules.
fn fly(npc: &mut Npc) -> Outcome {
    let speeds = TopSpeeds::own_of(npc);
    let weapons = crate::ai::fixture::records_of([&*npc]);
    autopilot::fly(
        npc,
        &[],
        &BTreeMap::new(),
        None,
        false,
        Beside::default(),
        speeds,
        FlightRules::default(),
        Readings::default(),
        &weapons,
        &mut Draws::of(&[]),
    )
}

/// `npc` and its twin with no sub 1 stored, each flown a tick.
fn flown_with_and_without(mut npc: Npc) -> (Npc, Npc) {
    let mut uncut = Npc {
        stopping: false,
        ..npc.clone()
    };
    assert_eq!(fly(&mut npc), Outcome::Flying);
    fly(&mut uncut);
    (npc, uncut)
}

/// Asserts `npc` was cut `by` before it moved, with no turn: its velocity
/// and speed, from `start` and 2.0.
fn cut(npc: &Npc, start: &ShipState, by: f64) {
    let velocity = drag_by(start.velocity, by);
    assert_eq!(npc.state.velocity, velocity, "{by} {:?}", npc.mode);
    #[allow(clippy::cast_possible_truncation)]
    let speed = (2.0_f64 * by) as f32;
    assert_eq!(npc.speed, speed, "{by} {:?}", npc.mode);
    assert_eq!(npc.state.position, start.position + velocity, "then moved");
    assert_eq!(npc.state.heading, start.heading, "no turn");
}

#[test]
fn in_a_mode_wait_an_inertial_ship_on_the_soft_brake_within_its_retro_heading_is_cut_by_0_94() {
    for mode in MODE_WAITS {
        // Moving up at 1, its retro heading is 180.
        let ship = waiting_in(mode, FAST, Goal::Wait, Vec2::new(0.0, -1.0), 180.0);
        let start = ship.state;
        let (ship, uncut) = flown_with_and_without(ship);
        cut(&ship, &start, 0.94);
        assert!(ship.stopping, "sub 1 stands {mode:?}");
        assert_eq!(uncut.state.velocity, start.velocity, "sub 0 is not cut");
        assert_eq!(uncut.state.position, start.position + start.velocity);
    }
}

#[test]
fn in_a_mode_wait_turned_away_it_moves_on_uncut_but_in_goal_9() {
    for mode in MODE_WAITS {
        let ship = waiting_in(mode, FAST, Goal::Wait, Vec2::new(0.0, -1.0), 90.0);
        let start = ship.state;
        let (ship, uncut) = flown_with_and_without(ship);
        assert_eq!((ship.state, ship.speed), (uncut.state, uncut.speed));
        assert_eq!(ship.state.position, start.position + start.velocity);
        assert!(ship.stopping);
        // Goal 9, the refuel (+0xc8c8 @0x85595), is cut off it too.
        let goal = Goal::Assist(Help::Refuel);
        let refuelling = waiting_in(mode, FAST, goal, start.velocity, 90.0);
        let (refuelling, _) = flown_with_and_without(refuelling);
        cut(&refuelling, &start, 0.94);
    }
}

#[test]
fn in_a_mode_wait_on_the_full_brake_or_inertialess_it_moves_on_uncut() {
    for mode in MODE_WAITS {
        // 1.75 or more on an axis: the full brake only writes +0x30
        // (@0x85510); an inertialess brake writes +0x30 alone
        // (@0x853d7-0x853fe).
        for (fields, velocity, heading) in [
            (FAST, Vec2::new(0.0, -1.75), 180.0),
            (RAVEN, Vec2::new(0.0, -2.0), 0.0),
        ] {
            let ship = waiting_in(mode, fields, Goal::Wait, velocity, heading);
            let (ship, uncut) = flown_with_and_without(ship);
            assert_eq!((ship.state, ship.speed), (uncut.state, uncut.speed));
            assert!(ship.stopping, "{mode:?}");
        }
    }
}

#[test]
fn in_a_mode_wait_below_0_35_it_settles_once_by_0_95_and_stores_sub_0() {
    for mode in MODE_WAITS {
        for fields in [FAST, RAVEN] {
            let ship = waiting_in(mode, fields, Goal::Wait, Vec2::new(0.0, -0.2), 180.0);
            let start = ship.state;
            let (ship, uncut) = flown_with_and_without(ship);
            #[allow(clippy::cast_possible_truncation)]
            let speed = (2.0_f64 * 0.95) as f32;
            if fields == FAST {
                cut(&ship, &start, 0.95);
                assert_eq!(uncut.state.velocity, start.velocity, "sub 0 not settled");
            } else {
                assert_eq!(ship.speed, speed);
            }
            assert_eq!(uncut.speed, 2.0, "sub 0 not settled");
            assert!(!ship.stopping, "the settle's sub 0 (@0x85624) {mode:?}");
            // Uncut on the ticks after, sub 0 stored.
            let (after, uncut) = flown_with_and_without(ship);
            assert_eq!((after.state, after.speed), (uncut.state, uncut.speed));
            assert!(!after.stopping);
        }
    }
}

#[test]
fn a_held_ship_owing_its_burst_owes_the_same_with_sub_1_kept() {
    // Sub 1 writes +0x30, +0x68 and +0x48, never +0x34 (F3).
    let ship = Npc {
        owed_burst: Some(5.0),
        bursts_own_word: true,
        ..waiting_in(
            Mode::HeldAtGate { ticks_left: 9.0 },
            FAST,
            Goal::Wait,
            Vec2::new(0.0, -1.0),
            180.0,
        )
    };
    let (ship, uncut) = flown_with_and_without(ship);
    assert_eq!(ship.owed_burst, Some(5.0));
    assert_eq!(ship.owed_burst, uncut.owed_burst);
    assert_eq!(ship.burst_word(), uncut.burst_word());
}

/// `npc`'s handling at its slot.
fn handling(npc: &Npc) -> Handling {
    npc.flying_handling(TopSpeeds::own_of(npc).own, FlightRules::default())
}

fn of(traffic: &Traffic, id: NpcId) -> &Npc {
    traffic
        .npcs()
        .iter()
        .find(|npc| npc.id == id)
        .expect("there")
}

#[test]
fn double_time_second_pass_flies_the_stop_kept_once_a_mode_wait_ran_out() {
    // Off its retro heading by 30: no thrust, a turn to 180.
    let half = SpeedMult::new(0.5);
    for mode in [
        Mode::Coasting { ticks_left: 0.5 },
        Mode::HeldAtGate { ticks_left: 0.5 },
    ] {
        let ship = waiting_in(mode, FAST, Goal::Wait, Vec2::new(0.0, -1.0), 150.0);
        let turn = handling(&ship).turn_rate;
        let mut headings = Vec::new();
        for stopping in [true, false] {
            let mut traffic = Traffic::with_interval(1);
            let id = traffic.add_npc(Npc {
                stopping,
                ..ship.clone()
            });
            let world = World {
                speed_mult: half,
                ..World::new(&[])
            };
            traffic.fly_tick(&world, &mut Draws::of(&[]));
            let flown = of(&traffic, id);
            assert_eq!(
                flown.state.heading, 150.0,
                "the advance pass skips the turn"
            );
            assert_eq!(flown.mode, Mode::Flying, "its wait out {mode:?}");
            traffic.move_in(world, &mut Draws::of(&[]));
            headings.push(of(&traffic, id).state.heading);
        }
        assert_eq!(
            headings,
            [150.0 + turn * 0.5, 150.0],
            "turned toward 180 by its turn × the gain, sub 0 kept its heading ({mode:?})"
        );
    }
}

#[test]
fn an_emerging_ship_holds_sub_0() {
    let mut ship = Npc {
        stopping: true,
        ..super::fixture::npc(1, ShipStats::new(FAST, &[]))
    };
    ship.emerge_from_gate(crate::catalog::StellarId(129), 30.0);
    assert!(!ship.stopping, "sub 0 (@0x8953b)");
}

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

/// The lead of [`successor_of`]'s fleet.
const LEAD: NpcId = NpcId(0);

/// The follower NPC 1 of a lead, braking in sub 1 as `lead_stopping` says,
/// the follower waiting in its gate moving up at 1 (its retro heading
/// 180), after a tick of the traffic in which the lead is lost, disabled.
fn successor_of(lead_stopping: bool) -> Npc {
    let mut traffic = Traffic::with_interval(1);
    let govt = Some(crate::catalog::GovtId(128));
    let mut lead = super::fixture::npc(0, ShipStats::new(FAST, &[]));
    lead.govt = govt;
    lead.hull.signed_mass = 500;
    lead.hull.inherent_ai = 3;
    lead.condition = Condition::Disabled;
    lead.goal = Goal::Wait;
    lead.stopping = lead_stopping;
    traffic.add_npc_at(lead, 1);
    let mut follower = super::fixture::npc(0, ShipStats::new(FAST, &[]));
    follower.govt = govt;
    follower.hull.signed_mass = 300;
    follower.hull.inherent_ai = 3;
    follower.leader = Some(LEAD);
    follower.goal = Goal::Follow {
        lead: LEAD,
        guard: None,
    };
    follower.mode = Mode::Emerging { ticks_left: 30.0 };
    follower.state.velocity = Vec2::new(0.0, -1.0);
    follower.state.heading = 180.0;
    let id = traffic.add_npc_at(follower, 2);
    let mut world = World::new(&[]);
    world.warp.durations = [600, 600];
    traffic.tick_in(&Keep, world, &[], &mut NeverFires);
    of(&traffic, id).clone()
}

#[test]
fn the_traffic_cuts_a_successor_held_at_its_gate_with_its_leads_sub_1() {
    let taker = successor_of(true);
    let own = successor_of(false);
    assert_eq!(taker.leader, None, "it leads");
    assert!(
        matches!(taker.mode, Mode::HeldAtGate { .. }),
        "{:?}",
        taker.mode
    );
    assert!(taker.stopping, "its lead's sub 1 (@0x893c8-0x893e1)");
    let start = Vec2::new(0.0, -1.0);
    assert_eq!(
        taker.state.velocity,
        drag_by(start, 0.94),
        "cut at its slot"
    );
    assert_eq!(own.state.velocity, start, "sub 0 copied, not cut");
}

/// A disabled ship holding its course, boarded, moving up at 1 facing its
/// retro heading 180, sub 1 stored as `stopping` says, after a tick of
/// the traffic.
fn boarded(stopping: bool) -> Npc {
    let ship = Npc {
        condition: Condition::Disabled,
        ..waiting_in(
            Mode::Coasting { ticks_left: 100.0 },
            FAST,
            Goal::Wait,
            Vec2::new(0.0, -1.0),
            180.0,
        )
    };
    let mut traffic = Traffic::with_interval(1);
    let id = traffic.add_npc(Npc { stopping, ..ship });
    traffic.tick_in(&Keep, World::new(&[]), &[], &mut NeverFires);
    of(&traffic, id).clone()
}

#[test]
fn the_traffic_cuts_a_boarded_disabled_ship_with_sub_0_after_its_reset() {
    // The dispatch's disabled reset writes sub 0 (@0x8ff0a) at its slot
    // before the tail jump to the low level (@0x901f4).
    let ship = boarded(true);
    let twin = boarded(false);
    assert_eq!(
        (ship.state, ship.speed),
        (twin.state, twin.speed),
        "dragged alone"
    );
    assert!(!ship.stopping);
}

#[test]
fn the_disabled_reset_stores_sub_0() {
    let mut ship = Npc {
        condition: Condition::Disabled,
        ..waiting_in(
            Mode::Coasting { ticks_left: 100.0 },
            FAST,
            Goal::Wait,
            Vec2::new(0.0, -1.0),
            180.0,
        )
    };
    super::disabled_reset::reset(&mut ship, RuleSource::Engine);
    assert!(!ship.stopping, "sub 0 (@0x8ff0a)");
}

/// `npc`'s low level at its slot beside nothing, at full gain under the
/// original's rules.
fn low(npc: &mut Npc) {
    let speeds = TopSpeeds::own_of(npc);
    let weapons = crate::ai::fixture::records_of([&*npc]);
    let rules = (FlightRules::default(), Readings::default(), &weapons);
    let (_, outcome) = autopilot::low_level(
        npc,
        &[],
        &BTreeMap::new(),
        (None, false, Beside::default()),
        speeds,
        rules,
        &mut Draws::of(&[]),
    );
    assert_eq!(outcome, None);
}

#[test]
fn a_ship_entering_its_gate_flies_no_kept_stop() {
    // A sub 1 written over its sub 0x17 would end the entry (F2), so its
    // wait runs no kept stop: moving, its bit stands uncut, as no sub is
    // written; at rest the clear drops it, with no settle.
    let mode = Mode::EnteringGate {
        gate: crate::catalog::StellarId(129),
        ticks_left: 9.0,
    };
    let mut moving = waiting_in(mode, FAST, Goal::Wait, Vec2::new(0.0, -1.0), 180.0);
    let start = moving.state;
    low(&mut moving);
    assert_eq!((moving.state, moving.speed), (start, 2.0), "uncut");
    assert!(moving.stopping, "no sub written");
    let mut at_rest = waiting_in(mode, FAST, Goal::Wait, Vec2::default(), 180.0);
    low(&mut at_rest);
    assert_eq!(at_rest.speed, 2.0, "no settle");
    assert!(!at_rest.stopping, "the clear");
}
