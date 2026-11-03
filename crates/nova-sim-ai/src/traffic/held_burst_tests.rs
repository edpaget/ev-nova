//! Tests of a ship held at its gate owing its gate burst and bursting out
//! on it ([`held_burst`](super::held_burst)), through each real caller:
//! the hold running out ([`autopilot::fly`]), each flight whose substate
//! keeps or writes the burst word +0x34, and the traffic's fly half,
//! which ends the burst ([`held_burst::settle`](super::held_burst::settle)
//! in `fly_pass`).

#![allow(clippy::float_cmp)]

use super::autopilot::{self, Beside, Outcome, Readings, TopSpeeds};
use super::jump_with_leader;
use super::npc::{Mode, Npc, NpcId};
use super::warp_out::{FOLLOW_BURST, GIVEN_UP_STAGE, WarpGoal};
use super::{Traffic, World};
use crate::ai::Goal;
use crate::ai::fire::Manoeuvre;
use crate::ai::lost_sight::SearchTimer;
use crate::ai::pursuit::{Body, Frame};
use crate::ai::swarm::SwarmLead;
use crate::catalog::GovtId;
use crate::chance::{Chance, NeverFires};
use crate::combat::ShipRef;
use crate::combat::hull::Condition;
use crate::escort::{EscortDuty, FormationSlot};
use crate::flight::{FlightRules, ShipState, facing};
use crate::geometry::Vec2;
use crate::hail::Help;
use crate::rulebook::RuleSource;
use crate::stats::{INERTIALESS_HULL, ShipStats};
use crate::traffic::plunder;
use crate::traffic::plunder_dock::{DockSub, Docking};
use nova_sim_base::fixture::FAST;
use std::collections::{BTreeMap, BTreeSet};

/// A ship of [`FAST`] held at its gate for `ticks_left` more ticks, with
/// `goal`, at `state`, its target the player.
fn held(goal: Goal, state: ShipState, ticks_left: u32) -> Npc {
    let mut npc = crate::traffic::fixture::npc(1, ShipStats::new(FAST, &[]));
    npc.mode = Mode::HeldAtGate {
        ticks_left: crate::traffic::ai_wait::ticks(ticks_left),
    };
    npc.goal = goal;
    npc.state = state;
    npc.target = Some(ShipRef::Player);
    npc
}

/// [`held`], its hold ended by the engine's reading
/// ([`Npc::end_wait`]): flying, owing 30.
fn released(goal: Goal, state: ShipState) -> Npc {
    let mut npc = held(goal, state, 1);
    npc.end_wait(RuleSource::Engine);
    assert_eq!(npc.owed_burst, Some(30.0));
    npc
}

/// The ship at `x`, `y`, moving `vx`, `vy`, heading `heading`.
fn at(x: f32, y: f32, vx: f32, vy: f32, heading: f32) -> ShipState {
    ShipState {
        position: Vec2::new(x, y),
        velocity: Vec2::new(vx, vy),
        heading,
    }
}

/// One tick of `npc` through the autopilot ([`autopilot::fly`]), beside
/// `other` and `beside`, by `readings`, at its own top speed, rolling on
/// `chance`.
fn fly_once(
    npc: &mut Npc,
    other: Option<&ShipState>,
    beside: Beside,
    readings: Readings,
    chance: &mut (impl Chance + ?Sized),
) -> Outcome {
    autopilot::fly(
        npc,
        &[],
        &BTreeMap::new(),
        other,
        false,
        beside,
        TopSpeeds::own_of(npc),
        FlightRules::default(),
        readings,
        &crate::ai::NO_WEAPONS,
        chance,
    )
}

/// The player far below, beyond every reach.
const FAR: ShipState = ShipState {
    position: Vec2::new(0.0, 20_000.0),
    velocity: Vec2::ZERO,
    heading: 0.0,
};

fn close(a: Vec2, b: Vec2) -> bool {
    (a - b).length() < 1e-3
}

#[test]
fn a_hold_that_runs_out_owes_its_burst_by_the_engine_only() {
    for (reading, owed) in [(RuleSource::Engine, Some(30.0)), (RuleSource::Bible, None)] {
        let mut npc = held(
            Goal::Attack(ShipRef::Player),
            at(0.0, 0.0, 0.0, 0.0, 0.0),
            1,
        );
        let readings = Readings {
            held_burst: reading,
            ..Readings::default()
        };
        fly_once(
            &mut npc,
            Some(&FAR),
            Beside::default(),
            readings,
            &mut NeverFires,
        );
        assert_eq!(npc.mode, Mode::Flying, "{reading:?}");
        assert_eq!(npc.owed_burst, owed, "{reading:?}");
    }
}

/// A swarmer [`held`] at the centre heading `heading`, its swarm lead's aim
/// straight up.
fn swarmer(heading: f32) -> (Npc, Beside) {
    let mut npc = held(
        Goal::Attack(ShipRef::Player),
        at(0.0, 0.0, 0.0, 0.0, heading),
        1,
    );
    npc.hull.swarms = true;
    let beside = Beside {
        swarm: Some(SwarmLead {
            aim: Vec2::new(0.0, -5000.0),
            leads_it: false,
            glow: 0,
        }),
        ..Beside::default()
    };
    (npc, beside)
}

#[test]
fn a_held_swarmer_bursts_out_along_its_swarm_heading_on_its_first_thrust() {
    // Sub 0x12 writes +0x30 alone (@0x8533d): `_HandleShip` bursts it
    // out on the -30.0 the hold left (@0x34bc0-0x34c63).
    let (mut npc, beside) = swarmer(0.0);
    let accel = npc.stats.handling.accel;
    fly_once(
        &mut npc,
        Some(&FAR),
        beside,
        Readings::default(),
        &mut NeverFires,
    );
    assert_eq!((npc.mode, npc.owed_burst), (Mode::Flying, Some(30.0)));
    fly_once(
        &mut npc,
        Some(&FAR),
        beside,
        Readings::default(),
        &mut NeverFires,
    );
    assert!(
        close(npc.state.velocity, Vec2::new(0.0, -30.0)),
        "{:?}",
        npc.state
    );
    assert_eq!(npc.owed_burst, Some(30.0 - accel));
    // Off its aim by more than its turn and 20 it doesn't thrust: no burst,
    // and it keeps what it owes.
    let (mut npc, beside) = swarmer(90.0);
    fly_once(
        &mut npc,
        Some(&FAR),
        beside,
        Readings::default(),
        &mut NeverFires,
    );
    fly_once(
        &mut npc,
        Some(&FAR),
        beside,
        Readings::default(),
        &mut NeverFires,
    );
    assert_eq!(npc.state.velocity, Vec2::ZERO);
    assert_eq!(npc.owed_burst, Some(30.0));
}

#[test]
fn a_held_sniper_pushed_moving_bursts_out_along_its_retro_heading() {
    // Sub 0xe's brake writes +0x30 alone (@0x87244-0x87252), so it bursts
    // along the retro heading it turned to (270°, against (2, 0)).
    let mut npc = held(
        Goal::Snipe(ShipRef::Player),
        at(0.0, 0.0, 2.0, 0.0, 270.0),
        1,
    );
    let accel = npc.stats.handling.accel;
    fly_once(
        &mut npc,
        Some(&FAR),
        Beside::default(),
        Readings::default(),
        &mut NeverFires,
    );
    assert_eq!(
        npc.state.velocity,
        Vec2::new(2.0, 0.0),
        "drifting on its push"
    );
    fly_once(
        &mut npc,
        Some(&FAR),
        Beside::default(),
        Readings::default(),
        &mut NeverFires,
    );
    assert!(
        close(npc.state.velocity, facing(270.0) * 30.0),
        "{:?}",
        npc.state
    );
    assert_eq!(npc.owed_burst, Some(30.0 - accel));
}

#[test]
fn a_held_inertialess_sniper_moving_bursts_out_in_place_of_its_brake() {
    // Inertialess, its +0x30 is -`_ShipAccelRate` (@0x8710f-0x8713a): the
    // burst sets its speed to 30 (@0x34bdf-0x34bf0), not the brake's cut.
    // Moving along its heading, its velocity follows its speed exactly.
    let mut npc = held(
        Goal::Snipe(ShipRef::Player),
        at(0.0, 0.0, 2.0, 0.0, 90.0),
        1,
    );
    npc.stats = ShipStats::new(
        crate::handling::ShipFields {
            flags2: INERTIALESS_HULL,
            ..FAST
        },
        &[],
    );
    npc.speed = 2.0;
    let accel = npc.stats.handling.accel;
    fly_once(
        &mut npc,
        Some(&FAR),
        Beside::default(),
        Readings::default(),
        &mut NeverFires,
    );
    assert!(crate::traffic::snipe::moving(&npc.state), "{:?}", npc.state);
    // Its velocity follows its speed as it stood (@0x34250-0x342aa), not
    // as the brake would have cut it.
    let mut drifted = npc.clone();
    let flown = drifted.flying_handling(TopSpeeds::own_of(&drifted).own, FlightRules::default());
    crate::traffic::inertialess::drift(&mut drifted, &flown, crate::speed_mult::SpeedMult::FULL);
    fly_once(
        &mut npc,
        Some(&FAR),
        Beside::default(),
        Readings::default(),
        &mut NeverFires,
    );
    assert_eq!(npc.speed, 30.0);
    assert_eq!(npc.owed_burst, Some(30.0 - accel));
    assert!(
        close(npc.state.velocity, drifted.state.velocity),
        "{:?}",
        npc.state
    );
    // Owing none, it brakes as ever.
    let mut braking = held(
        Goal::Snipe(ShipRef::Player),
        at(0.0, 0.0, 2.0, 0.0, 90.0),
        1,
    );
    braking.stats = npc.stats;
    braking.speed = 2.0;
    let readings = Readings {
        held_burst: RuleSource::Bible,
        ..Readings::default()
    };
    fly_once(
        &mut braking,
        Some(&FAR),
        Beside::default(),
        readings,
        &mut NeverFires,
    );
    fly_once(
        &mut braking,
        Some(&FAR),
        Beside::default(),
        readings,
        &mut NeverFires,
    );
    assert!(
        (braking.speed - (2.0 - accel)).abs() < 1e-6,
        "{}",
        braking.speed
    );
}

#[test]
fn a_released_ship_braking_to_a_stop_bursts_out_along_its_retro_heading() {
    // `brake` stands in for sub 1 (@0x8536d-0x85633), which writes +0x30
    // alone: a wait (state 6, @0x8f101) and a search for a target lost
    // from sight (@0x8d814).
    let mut searching = released(Goal::Attack(ShipRef::Player), at(0.0, 0.0, 2.0, 0.0, 270.0));
    searching.lost_sight = SearchTimer::Searching(10.0);
    for mut npc in [
        released(Goal::Wait, at(0.0, 0.0, 2.0, 0.0, 270.0)),
        searching,
    ] {
        let accel = npc.stats.handling.accel;
        fly_once(
            &mut npc,
            Some(&FAR),
            Beside::default(),
            Readings::default(),
            &mut NeverFires,
        );
        assert!(
            close(npc.state.velocity, facing(270.0) * 30.0),
            "{:?}",
            npc.goal
        );
        assert_eq!(npc.owed_burst, Some(30.0 - accel), "{:?}", npc.goal);
    }
}

#[test]
fn a_released_ship_leaving_that_brakes_beyond_1000_bursts_out() {
    // State 2's brake beyond 1000 is sub 1 (`leave_flight`).
    let mut npc = released(Goal::JumpOut, at(2000.0, 0.0, 2.0, 0.0, 270.0));
    let accel = npc.stats.handling.accel;
    fly_once(
        &mut npc,
        None,
        Beside::default(),
        Readings::default(),
        &mut NeverFires,
    );
    assert!(
        close(npc.state.velocity, facing(270.0) * 30.0),
        "{:?}",
        npc.state
    );
    assert_eq!(npc.owed_burst, Some(30.0 - accel));
}

#[test]
fn a_released_escort_stopping_near_a_leader_it_cannot_see_bursts_out() {
    // Goal 0xa's stop within 300 of a leader unseen is sub 1
    // (`formation_flight`).
    let mut npc = released(
        Goal::Formation { guard: None },
        at(100.0, 0.0, 2.0, 0.0, 270.0),
    );
    npc.escort = Some(EscortDuty {
        formation: FormationSlot::default(),
        order: None,
    });
    let accel = npc.stats.handling.accel;
    let leader = at(0.0, 0.0, 0.0, 0.0, 0.0);
    let unseen = Beside {
        leader_seen: false,
        ..Beside::default()
    };
    fly_once(
        &mut npc,
        Some(&leader),
        unseen,
        Readings::default(),
        &mut NeverFires,
    );
    assert!(
        close(npc.state.velocity, facing(270.0) * 30.0),
        "{:?}",
        npc.state
    );
    assert_eq!(npc.owed_burst, Some(30.0 - accel));
    // With no leader it brakes, `brake` standing in for sub 1.
    let mut alone = released(
        Goal::Formation { guard: None },
        at(100.0, 0.0, 2.0, 0.0, 270.0),
    );
    fly_once(
        &mut alone,
        None,
        Beside::default(),
        Readings::default(),
        &mut NeverFires,
    );
    assert!(
        close(alone.state.velocity, facing(270.0) * 30.0),
        "{:?}",
        alone.state
    );
    assert_eq!(alone.owed_burst, Some(30.0 - accel));
}

#[test]
fn a_released_refueller_on_its_stop_bursts_out() {
    // The refuel's stop is sub 1 (`plunder_dock`), flown by the docking.
    let mut npc = released(Goal::Assist(Help::Refuel), at(0.0, 0.0, 2.0, 0.0, 270.0));
    npc.docking = Some(Docking {
        ship: ShipRef::Player,
        sub: DockSub::Stop,
    });
    let accel = npc.stats.handling.accel;
    plunder::fly_docking(&mut npc, Some(&FAR), 1.0, FlightRules::default(), None);
    assert!(
        close(npc.state.velocity, facing(270.0) * 30.0),
        "{:?}",
        npc.state
    );
    assert_eq!(npc.owed_burst, Some(30.0 - accel));
}

#[test]
fn a_dogfighters_speed_cap_drops_what_it_owes_whether_or_not_it_thrusts() {
    // Sub 6 sets +0x34 to its target's speed (@0x860a6), thrust or not.
    for (cap, owed) in [(Some(2.0), None), (None, Some(30.0))] {
        let mut npc = released(Goal::Attack(ShipRef::Player), at(0.0, 0.0, 0.0, 0.0, 0.0));
        npc.stats = ShipStats::new(
            crate::handling::ShipFields {
                flags2: INERTIALESS_HULL,
                ..FAST
            },
            &[],
        );
        // Wanting 180°, it doesn't thrust.
        npc.pursuit.frame = Some(Frame {
            goal: npc.goal,
            bodies: [
                Some(Body {
                    manoeuvre: Manoeuvre::Dogfight,
                    heading: 180,
                }),
                None,
            ],
            speed_cap: cap,
        });
        let target = at(0.0, 50.0, 0.0, 0.0, 0.0);
        fly_once(
            &mut npc,
            Some(&target),
            Beside::default(),
            Readings::default(),
            &mut NeverFires,
        );
        assert_eq!(
            npc.display.drive.thrust,
            crate::traffic::ship_display::Thrust::None
        );
        assert_eq!(npc.owed_burst, owed, "{cap:?}");
    }
}

/// Two swarmers of [`FAST`] of government 128 attacking the player, far
/// above them: NPC 0 at berth 1, the swarm leader, at the centre facing
/// up and staying put; NPC 1 at berth 2, 5000 straight below it, facing
/// up, its swarm leader berth 1, held at its gate for one more tick, so
/// its swarm aim is dead ahead. The hold ends owing as `reading` says.
fn swarm_pair(reading: RuleSource) -> (Traffic, World<'static>) {
    let swarming = |id: u32, berth: u32, y: f32| {
        let mut npc = crate::traffic::fixture::npc(id, ShipStats::new(FAST, &[]));
        npc.berth = berth;
        npc.hull.swarms = true;
        npc.govt = Some(GovtId(128));
        npc.goal = Goal::Attack(ShipRef::Player);
        npc.target = Some(ShipRef::Player);
        npc.state = at(0.0, y, 0.0, 0.0, 0.0);
        npc
    };
    let mut leader = swarming(0, 1, 0.0);
    leader.no_ai = true;
    // It stays where it is, so the aim stays dead ahead.
    leader.immobile = true;
    let mut held = swarming(1, 2, 5000.0);
    held.swarm_leader = Some(1);
    held.mode = Mode::HeldAtGate { ticks_left: 1.0 };
    let mut traffic = Traffic::with_interval(1);
    traffic.npcs = vec![leader, held];
    let world = World {
        player: Some(crate::ai::fixture::player(0.0, -50_000.0)),
        held_burst: reading,
        ..World::new(&[])
    };
    (traffic, world)
}

#[test]
fn a_held_swarmer_bursts_out_slowing_by_its_thrust_and_stops_jumping_at_its_cap() {
    let (mut traffic, world) = swarm_pair(RuleSource::Engine);
    let tick = |traffic: &mut Traffic| {
        traffic.fly_tick(&world, &mut NeverFires);
        traffic.npcs[1].clone()
    };
    let released = tick(&mut traffic);
    assert_eq!(
        (released.mode, released.owed_burst),
        (Mode::Flying, Some(30.0))
    );
    let accel = released.stats.handling.accel;
    let cap = released.stats.handling.max_speed;
    let mut speed = 30.0;
    let mut bursts = 0;
    loop {
        let ship = tick(&mut traffic);
        bursts += 1;
        assert!(
            (ship.state.velocity.length() - speed).abs() < 1e-3,
            "burst {bursts}: {:?} at {speed}",
            ship.state.velocity
        );
        let next: f32 = speed - accel;
        if next <= cap {
            // `_AIMakeShipStopJumping` (@0x82e77-0x82eba), then
            // Rand(30) + 30 (@0x34d5c-0x34d8b): 29 + 30.
            assert_eq!(ship.goal, Goal::Idle);
            assert!(!ship.travel_set());
            assert_eq!(ship.owed_burst, None);
            assert_eq!(ship.mode, Mode::Coasting { ticks_left: 59.0 });
            break;
        }
        assert_eq!(ship.goal, Goal::Attack(ShipRef::Player), "burst {bursts}");
        assert!(
            ship.owed_burst
                .is_some_and(|owed| (owed - next).abs() < 1e-3),
            "burst {bursts}: {:?}",
            ship.owed_burst
        );
        speed = next;
        assert!(bursts < 500, "never ended");
    }
    for left in (1..59).rev().map(crate::traffic::ai_wait::ticks) {
        assert_eq!(tick(&mut traffic).mode, Mode::Coasting { ticks_left: left });
    }
    assert_eq!(tick(&mut traffic).mode, Mode::Flying);
}

#[test]
fn by_the_other_reading_a_held_swarmer_flies_its_swarm_from_its_hold_with_no_burst() {
    let (mut traffic, world) = swarm_pair(RuleSource::Bible);
    let top = traffic.npcs[1].stats.handling.max_speed;
    let accel = traffic.npcs[1].stats.handling.accel;
    for tick in 0..200 {
        traffic.fly_tick(&world, &mut NeverFires);
        let ship = &traffic.npcs[1];
        assert!(
            ship.state.velocity.length() <= top + accel + 1e-3,
            "tick {tick}: {:?}",
            ship.state.velocity
        );
        assert!(!matches!(ship.mode, Mode::Coasting { .. }), "tick {tick}");
        assert_eq!(ship.goal, Goal::Attack(ShipRef::Player));
    }
    assert!(traffic.npcs[1].state.velocity.y < 0.0, "it swarms");
}

#[test]
fn a_held_sniper_pushed_moving_bursts_out_and_stops_jumping_at_its_cap() {
    let mut sniper = held(
        Goal::Snipe(ShipRef::Player),
        at(0.0, 0.0, 2.0, 0.0, 270.0),
        1,
    );
    sniper.govt = Some(GovtId(128));
    let mut traffic = Traffic::with_interval(1);
    traffic.npcs = vec![sniper];
    let world = World {
        player: Some(crate::ai::fixture::player(0.0, -50_000.0)),
        ..World::new(&[])
    };
    traffic.fly_tick(&world, &mut NeverFires);
    traffic.fly_tick(&world, &mut NeverFires);
    let first = &traffic.npcs[0];
    assert!(
        close(first.state.velocity, facing(270.0) * 30.0),
        "{:?}",
        first.state
    );
    let mut ticks = 0;
    while traffic.npcs[0].owed_burst.is_some() {
        assert_eq!(traffic.npcs[0].goal, Goal::Snipe(ShipRef::Player));
        traffic.fly_tick(&world, &mut NeverFires);
        ticks += 1;
        assert!(ticks < 50_000, "never ended");
    }
    let ended = &traffic.npcs[0];
    assert_eq!(ended.goal, Goal::Idle);
    assert_eq!(ended.mode, Mode::Coasting { ticks_left: 59.0 });
    assert!(
        ended.state.velocity.length() > ended.stats.handling.max_speed,
        "it keeps the burst's last speed: {:?}",
        ended.state.velocity
    );
}

#[test]
fn a_held_inertialess_sniper_bursts_out_and_stops_dead_at_its_cap() {
    // As its burst ends an inertialess ship's velocity is zeroed
    // (@0x34d37-0x34d59), then it waits Rand(30) + 30 (@0x34d5c-0x34d8b).
    let mut sniper = held(
        Goal::Snipe(ShipRef::Player),
        at(0.0, 0.0, 2.0, 0.0, 90.0),
        1,
    );
    sniper.stats = ShipStats::new(
        crate::handling::ShipFields {
            flags2: INERTIALESS_HULL,
            ..FAST
        },
        &[],
    );
    sniper.speed = 2.0;
    sniper.govt = Some(GovtId(128));
    let mut traffic = Traffic::with_interval(1);
    traffic.npcs = vec![sniper];
    let world = World {
        player: Some(crate::ai::fixture::player(0.0, -50_000.0)),
        ..World::new(&[])
    };
    traffic.fly_tick(&world, &mut NeverFires);
    assert_eq!(traffic.npcs[0].owed_burst, Some(30.0));
    let mut ticks = 0;
    while traffic.npcs[0].owed_burst.is_some() {
        assert_eq!(traffic.npcs[0].goal, Goal::Snipe(ShipRef::Player));
        assert!(
            traffic.npcs[0].state.velocity.length() > 1.0,
            "tick {ticks}: {:?}",
            traffic.npcs[0].state
        );
        traffic.fly_tick(&world, &mut NeverFires);
        ticks += 1;
        assert!(ticks < 50_000, "never ended");
    }
    let ended = &traffic.npcs[0];
    assert!(ticks > 1, "it burst out: {ticks}");
    assert_eq!(ended.goal, Goal::Idle);
    assert_eq!(ended.state.velocity, Vec2::ZERO, "stopped dead");
    assert_eq!(ended.mode, Mode::Coasting { ticks_left: 59.0 });
    for left in (1..59).rev().map(crate::traffic::ai_wait::ticks) {
        traffic.fly_tick(&world, &mut NeverFires);
        assert_eq!(traffic.npcs[0].mode, Mode::Coasting { ticks_left: left });
    }
}

/// A tick of `world` with no think, the NPCs in `still` held: each NPC's
/// work at its slot, its warp's hold, its follow and its low level
/// ([`Traffic::leave_in_dispatch`]), then the fly half.
fn dispatched(traffic: &mut Traffic, world: &World, still: &BTreeSet<NpcId>) {
    traffic.open_dispatch();
    traffic.held_still.clone_from(still);
    traffic.leave_in_dispatch(world, &|_| true, &super::NoBay::new(), &mut NeverFires);
    traffic.held_still.clear();
    traffic.fly_holding(*world, still, &mut NeverFires);
}

// Sub 0xd's burst word (@0x85bcf-0x85c53).

/// A world by `reading` of `held_burst`, the player far above, the warp
/// sound's durations set.
fn world_by(reading: RuleSource) -> World<'static> {
    let mut world = World {
        player: Some(crate::ai::fixture::player(0.0, -50_000.0)),
        held_burst: reading,
        ..World::new(&[])
    };
    world.warp.durations = [364, 252];
    world
}

/// A lead 3000 out west on its own jump, its stage at 5.0, facing and
/// wanting 90°, to be held still; and a follower 3000 out east, facing the
/// same way, its stage not started: sub 0xd's stage branch, this once, as
/// the high level's hold takes an NPC's follower under way to goal 2
/// (@0x8d557-0x8d56b).
fn following() -> (Traffic, BTreeSet<NpcId>) {
    let ship = || crate::traffic::fixture::npc(0, ShipStats::new(FAST, &[]));
    let mut traffic = Traffic::with_interval(1);
    let mut lead = ship();
    lead.state = at(-3000.0, 0.0, 0.0, 0.0, 90.0);
    lead.goal = Goal::JumpOut;
    lead.warp.goal = Some(WarpGoal::Own);
    lead.warp.stage = 5.0;
    lead.warp.aim = 90;
    let lead = traffic.add_npc(lead);
    let mut follower = ship();
    follower.state = at(3000.0, 0.0, 0.0, 0.0, 90.0);
    follower.leader = Some(lead);
    follower.goal = Goal::Follow { lead, guard: None };
    follower.warp.goal = Some(WarpGoal::Follow(ShipRef::Npc(lead)));
    follower.warp.aim = 90;
    traffic.add_npc(follower);
    (traffic, BTreeSet::from([lead]))
}

#[test]
fn a_follower_on_its_stage_owes_1_by_the_engine_only_over_anything_it_owed() {
    assert_eq!(FOLLOW_BURST, 1.0);
    for (reading, owed, now) in [
        (RuleSource::Engine, None, Some(1.0)),
        (RuleSource::Engine, Some(30.0), Some(1.0)),
        (RuleSource::Bible, None, None),
    ] {
        let (mut traffic, lead) = following();
        traffic.npcs[1].owed_burst = owed;
        let goal = traffic.npcs[1].goal;
        dispatched(&mut traffic, &world_by(reading), &lead);
        let ship = &traffic.npcs[1];
        assert_eq!(ship.owed_burst, now, "{reading:?} {owed:?}");
        // The word is no burst: nothing ended (@0x34a84).
        assert_eq!(ship.goal, goal, "{reading:?} {owed:?}");
        assert_eq!(ship.mode, Mode::Flying, "{reading:?} {owed:?}");
        assert_eq!(ship.warp.stage, 2.0, "{reading:?} {owed:?}");
    }
}

#[test]
fn a_ship_the_player_leads_owes_1_every_frame_of_its_stage() {
    // Held to the player's jump (@0x8d542-0x8d54c), it stays on sub 0xd.
    let mut ship = crate::traffic::fixture::npc(0, ShipStats::new(FAST, &[]));
    ship.state = at(3000.0, 0.0, 0.0, 0.0, 90.0);
    ship.escort = Some(EscortDuty {
        formation: FormationSlot::default(),
        order: None,
    });
    ship.goal = Goal::Formation { guard: None };
    ship.warp.goal = Some(WarpGoal::Follow(ShipRef::Player));
    ship.warp.stage = 3.0;
    let mut traffic = Traffic::with_interval(1);
    traffic.add_npc(ship);
    let mut world = world_by(RuleSource::Engine);
    world.warp.player.stage = 5.0;
    world.warp.player.heading = 90.0;
    world.warp.player.aim = 90;
    for frame in 0..3 {
        traffic.npcs[0].owed_burst = Some(20.0);
        dispatched(&mut traffic, &world, &BTreeSet::new());
        assert_eq!(traffic.npcs[0].owed_burst, Some(1.0), "frame {frame}");
        assert_eq!(
            traffic.npcs[0].goal,
            Goal::Formation { guard: None },
            "frame {frame}: no burst ended"
        );
    }
}

#[test]
fn a_followers_formation_waiting_and_detach_frames_write_no_burst_word() {
    let lead_at_1: fn(&mut Traffic) = |traffic| traffic.npcs[0].warp.stage = 1.0;
    let lead_turning: fn(&mut Traffic) = |traffic| traffic.npcs[0].warp.aim = 270;
    let lead_turning_within_its_turn: fn(&mut Traffic) = |traffic| {
        traffic.npcs[0].warp.aim = 102;
        traffic.npcs[1].stats.handling.turn_rate = 20.0;
    };
    // Past 30 only under a wait, which skips the hold that would take it to
    // goal 2.
    let past_30: fn(&mut Traffic) = |traffic| {
        traffic.npcs[1].warp.stage = 31.0;
        traffic.npcs[1].ai_wait = 5.0;
    };
    for (name, setup) in [
        ("formation", lead_at_1),
        ("gave up", lead_turning),
        ("waiting", lead_turning_within_its_turn),
        ("detach", past_30),
    ] {
        for owed in [None, Some(12.0)] {
            let (mut traffic, lead) = following();
            setup(&mut traffic);
            traffic.npcs[1].owed_burst = owed;
            dispatched(&mut traffic, &world_by(RuleSource::Engine), &lead);
            assert_eq!(traffic.npcs[1].owed_burst, owed, "{name}");
            let detached = traffic.npcs[1].warp.goal == Some(WarpGoal::Own);
            assert_eq!(detached, name == "detach", "{name}");
        }
    }
}

#[test]
fn every_given_up_jump_keeps_the_word() {
    // None of these writes +0x34.
    let world = world_by(RuleSource::Engine);
    let owing = || {
        let (mut traffic, lead) = following();
        traffic.npcs[1].owed_burst = Some(1.0);
        (traffic, lead)
    };
    // Goal 0xb's give-up (@0x8df31-0x8df49): its lead is on no way out.
    let (mut traffic, lead) = owing();
    traffic.npcs[0].goal = Goal::Attack(ShipRef::Player);
    traffic.npcs[0].warp.goal = None;
    dispatched(&mut traffic, &world, &lead);
    assert_eq!(traffic.npcs[1].goal, Goal::Idle);
    assert_eq!(traffic.npcs[1].warp.goal, None);
    assert_eq!(traffic.npcs[1].owed_burst, Some(1.0), "goal 0xb's give-up");
    // `_AIDispatch`'s cancel (@0x90125-0x9019c).
    let (mut traffic, _) = owing();
    traffic.npcs[0].goal = Goal::Attack(ShipRef::Player);
    traffic.npcs[1].warp.stage = 1.0;
    traffic.cancel_jump_follows(&world, &[true; 64]);
    assert_eq!(traffic.npcs[1].warp.stage, GIVEN_UP_STAGE);
    assert_eq!(traffic.npcs[1].owed_burst, Some(1.0), "the cancel");
    // Sub 0xd's own give-up while its leader turns (@0x85ba5).
    let (mut traffic, lead) = owing();
    traffic.npcs[0].warp.aim = 270;
    dispatched(&mut traffic, &world, &lead);
    assert_eq!(traffic.npcs[1].warp.stage, GIVEN_UP_STAGE);
    assert_eq!(traffic.npcs[1].owed_burst, Some(1.0), "the turning lead");
    // The fighter turn-back (@0x8d605-0x8d621).
    let (mut traffic, _) = owing();
    let fighter = &mut traffic.npcs[1];
    fighter.carrier = Some(crate::bay::Carrier {
        ship: ShipRef::Player,
        window: 100.0,
        reach: 40.0,
    });
    fighter.class_fuel = 0;
    jump_with_leader::turn_back(fighter, RuleSource::Engine, Some(0));
    assert_eq!(fighter.goal, Goal::Dock(ShipRef::Player));
    assert_eq!(fighter.owed_burst, Some(1.0), "the turn-back");
    // An inertialess follower's detach (@0x83c11-0x83c51).
    let (mut traffic, _) = owing();
    traffic.npcs[1].stats = ShipStats::new(
        crate::handling::ShipFields {
            flags2: INERTIALESS_HULL,
            ..FAST
        },
        &[],
    );
    jump_with_leader::enter(&mut traffic.npcs, &[1], FlightRules::default());
    assert_eq!(traffic.npcs[1].warp.goal, Some(WarpGoal::Own));
    assert_eq!(traffic.npcs[1].owed_burst, Some(1.0), "the detach");
    // The disabled warp cancel (@0x8fce1).
    let (mut traffic, _) = owing();
    traffic.npcs[1].condition = Condition::Disabled;
    traffic.npcs[1].warp.stage = 3.0;
    traffic.cancel_warps(&[true; 64]);
    assert_eq!(traffic.npcs[1].warp.stage, GIVEN_UP_STAGE);
    assert_eq!(traffic.npcs[1].owed_burst, Some(1.0), "the warp cancel");
}

/// A ship of [`FAST`] with `goal` at `state`, having given a jump up: it
/// owes 1.0 and its stage is at -1.0.
fn gave_up(goal: Goal, state: ShipState) -> Npc {
    let mut npc = crate::traffic::fixture::npc(0, ShipStats::new(FAST, &[]));
    npc.goal = goal;
    npc.state = state;
    npc.speed = state.velocity.length();
    npc.target = Some(ShipRef::Player);
    npc.govt = Some(GovtId(128));
    npc.owed_burst = Some(1.0);
    npc.warp.stage = GIVEN_UP_STAGE;
    npc
}

/// `npc` after a tick with no think ([`dispatched`]), alone with the player
/// far above.
fn flown(npc: Npc) -> Npc {
    let mut traffic = Traffic::with_interval(1);
    traffic.npcs = vec![npc];
    dispatched(
        &mut traffic,
        &world_by(RuleSource::Engine),
        &BTreeSet::new(),
    );
    traffic.npcs.remove(0)
}

#[test]
fn its_next_stop_or_snipe_thrust_bursts_at_1_and_stops_jumping() {
    for goal in [Goal::Wait, Goal::Snipe(ShipRef::Player)] {
        let ship = flown(gave_up(goal, at(0.0, 0.0, 2.0, 0.0, 270.0)));
        assert!(
            close(ship.state.velocity, facing(270.0) * 1.0),
            "{goal:?}: {:?}",
            ship.state
        );
        assert_eq!(ship.goal, Goal::Idle, "{goal:?}");
        assert_eq!(ship.owed_burst, None, "{goal:?}");
        assert_eq!(ship.warp.stage, 0.0, "{goal:?}: +0x50 = 0 (@0x82eb0)");
        assert_eq!(ship.warp.goal, None, "{goal:?}");
        assert_eq!(ship.mode, Mode::Coasting { ticks_left: 59.0 }, "{goal:?}");
    }
}

#[test]
fn its_next_swarm_thrust_bursts_at_1_and_stops_jumping() {
    let (mut traffic, world) = swarm_pair(RuleSource::Engine);
    let ship = &mut traffic.npcs[1];
    ship.mode = Mode::Flying;
    ship.owed_burst = Some(1.0);
    ship.warp.stage = GIVEN_UP_STAGE;
    dispatched(&mut traffic, &world, &BTreeSet::new());
    let ship = &traffic.npcs[1];
    assert!(
        close(ship.state.velocity, Vec2::new(0.0, -1.0)),
        "{:?}",
        ship.state
    );
    assert_eq!(ship.goal, Goal::Idle);
    assert_eq!(ship.warp.stage, 0.0);
    assert_eq!(ship.mode, Mode::Coasting { ticks_left: 59.0 });
}

#[test]
fn with_a_lead_it_flies_on_from_the_burst_at_1() {
    let (mut traffic, lead) = following();
    let ship = &mut traffic.npcs[1];
    ship.goal = Goal::Wait;
    ship.warp.goal = None;
    ship.warp.stage = GIVEN_UP_STAGE;
    ship.owed_burst = Some(1.0);
    ship.state = at(3000.0, 0.0, 2.0, 0.0, 270.0);
    dispatched(&mut traffic, &world_by(RuleSource::Engine), &lead);
    let ship = &traffic.npcs[1];
    assert!(
        close(ship.state.velocity, facing(270.0) * 1.0),
        "{:?}",
        ship.state
    );
    assert_eq!(ship.goal, Goal::Idle);
    assert_eq!(ship.warp.stage, 0.0);
    assert_eq!(ship.mode, Mode::Flying);
}

#[test]
fn an_inertialess_ship_ends_its_burst_at_1_stopped_dead() {
    let mut ship = gave_up(Goal::Snipe(ShipRef::Player), at(0.0, 0.0, 2.0, 0.0, 90.0));
    ship.stats = ShipStats::new(
        crate::handling::ShipFields {
            flags2: INERTIALESS_HULL,
            ..FAST
        },
        &[],
    );
    let ship = flown(ship);
    assert_eq!(ship.state.velocity, Vec2::ZERO);
    assert_eq!(ship.goal, Goal::Idle);
    assert_eq!(ship.warp.stage, 0.0);
    assert_eq!(ship.mode, Mode::Coasting { ticks_left: 59.0 });
}

#[test]
fn an_assister_keeps_its_goal_through_the_burst_at_1_and_its_stage_still_ends() {
    let mut ship = gave_up(Goal::Assist(Help::Refuel), at(0.0, 0.0, 2.0, 0.0, 270.0));
    let docking = Some(Docking {
        ship: ShipRef::Player,
        sub: DockSub::Stop,
    });
    ship.docking = docking;
    let ship = flown(ship);
    assert_eq!(ship.goal, Goal::Assist(Help::Refuel));
    assert_eq!(ship.docking, docking);
    assert_eq!(ship.owed_burst, None);
    assert_eq!(ship.warp.stage, 0.0);
}

#[test]
fn a_writing_thrust_drops_the_word() {
    // The approach, facing its target, writes +0x34 with +0x30.
    let mut ship = gave_up(Goal::Attack(ShipRef::Player), at(0.0, 0.0, 0.0, 0.0, 0.0));
    let accel = ship.stats.handling.accel;
    ship.owed_burst = Some(1.0);
    let ship = flown(ship);
    assert_eq!(ship.owed_burst, None);
    assert!(
        close(ship.state.velocity, facing(0.0) * accel),
        "its thrust, no burst: {:?}",
        ship.state
    );
    assert_eq!(ship.goal, Goal::Attack(ShipRef::Player));
}

#[test]
fn a_follower_under_way_on_goal_2s_brake_ends_its_stage_with_the_burst() {
    // The hold puts it in goal 2, whose block picks sub 1 beyond 1000 while
    // it moves 0.35 or more (@0x8ddeb-0x8de49); sub 1 leaves +0x34.
    let mut ship = gave_up(Goal::JumpOut, at(2000.0, 0.0, 2.0, 0.0, 270.0));
    ship.warp.stage = 5.0;
    ship.target = None;
    let ship = flown(ship);
    assert!(
        close(ship.state.velocity, facing(270.0) * 1.0),
        "{:?}",
        ship.state
    );
    assert_eq!(ship.warp.stage, 0.0);
    assert_eq!(ship.goal, Goal::Idle);
    assert_eq!(ship.owed_burst, None);
}

#[test]
fn goal_0s_stand_in_keeps_what_it_owes_with_no_burst() {
    // Goal 0 flies sub 0, which writes no +0x30 (@0x34a84): the engine's
    // stop must not burst.
    let start = at(0.0, 0.0, 2.0, 0.0, 270.0);
    let mut plain = gave_up(Goal::Idle, start);
    plain.owed_burst = None;
    let plain = flown(plain);
    let ship = flown(gave_up(Goal::Idle, start));
    assert_eq!(ship.owed_burst, Some(1.0));
    assert_eq!(ship.state, plain.state, "it flies as it would owing none");
    assert_eq!(ship.goal, Goal::Idle);
    assert_eq!(ship.mode, Mode::Flying, "no burst ended it");
    // Its controls are the stop's, recorded as ever.
    assert_eq!(ship.display.drive, plain.display.drive);
}

// The frame's gain, as `_HandleShip` reads it in the flight it hands on.

const HALF: crate::speed_mult::SpeedMult = crate::speed_mult::SpeedMult::new(0.5);

/// Top speed 6, thrust 0.2, 4 degrees a tick.
const GAIN_HANDLING: crate::handling::Handling = crate::handling::Handling {
    max_speed: 6.0,
    accel: 0.2,
    turn_rate: 4.0,
};

/// [`held_burst::step`](super::held_burst::step) of a ship at the centre
/// moving (2, 0), facing east and owing `owed`, under `controls` on
/// [`GAIN_HANDLING`] at half gain, its substate doing `word`; and its
/// state as a plain step at half gain leaves it.
fn stepped_at_half(
    owed: Option<f32>,
    controls: crate::flight::Controls,
    word: super::held_burst::Word,
) -> (Npc, ShipState) {
    let mut npc = released(Goal::Idle, at(0.0, 0.0, 2.0, 0.0, 90.0));
    npc.owed_burst = owed;
    let rules = FlightRules::default();
    let by = (
        super::inertialess::Inertia::Inertial,
        word,
        GAIN_HANDLING.accel,
    );
    super::held_burst::step(&mut npc, &GAIN_HANDLING, controls, rules, by, HALF);
    let mut plain = at(0.0, 0.0, 2.0, 0.0, 90.0);
    let flyer = crate::flight::Flyer::Npc;
    crate::flight::step(&mut plain, &GAIN_HANDLING, controls, rules, flyer, HALF);
    (npc, plain)
}

#[test]
fn at_half_gain_a_kept_word_that_thrusts_nothing_flies_a_plain_step_at_half() {
    let controls = crate::flight::Controls {
        turn: crate::flight::Turn::Right,
        ..crate::flight::Controls::default()
    };
    let kept = super::held_burst::Word::Keeps { thrust: 0.0 };
    let (npc, plain) = stepped_at_half(Some(20.0), controls, kept);
    assert_eq!(npc.state, plain);
    assert_eq!(npc.state.heading, 92.0, "half a turn");
    assert_eq!(npc.state.position, Vec2::new(1.0, 0.0), "half a move");
}

#[test]
fn at_half_gain_a_written_word_flies_a_plain_step_at_half() {
    let controls = crate::flight::Controls {
        thrust: true,
        turn: crate::flight::Turn::Left,
        ..crate::flight::Controls::default()
    };
    let (npc, plain) = stepped_at_half(None, controls, super::held_burst::Word::Writes);
    assert_eq!(npc.state, plain);
    assert_eq!(npc.state.heading, 88.0, "half a turn");
}

#[test]
fn at_half_gain_a_burst_turns_half_and_moves_half_its_speed() {
    let controls = crate::flight::Controls {
        thrust: true,
        turn: crate::flight::Turn::Right,
        ..crate::flight::Controls::default()
    };
    let kept = super::held_burst::Word::Keeps { thrust: 1.0 };
    let (npc, _) = stepped_at_half(Some(20.0), controls, kept);
    assert_eq!(npc.state.heading, 92.0, "half a turn");
    let along = facing(92.0) * 20.0;
    assert_eq!(
        npc.state.position,
        crate::flight::moved(Vec2::ZERO, along, HALF)
    );
    assert_eq!(npc.owed_burst, Some(19.0), "the word's whole growth");
}
