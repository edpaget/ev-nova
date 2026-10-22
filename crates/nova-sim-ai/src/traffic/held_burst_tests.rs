//! Tests of a ship held at its gate owing its gate burst and bursting out
//! on it ([`held_burst`](super::held_burst)), through each real caller:
//! the hold running out ([`autopilot::fly`]), each flight whose substate
//! keeps or writes the burst word +0x34, and the traffic's fly half,
//! which ends the burst ([`held_burst::settle`](super::held_burst::settle)
//! in `fly_pass`).

#![allow(clippy::float_cmp)]

use super::autopilot::{self, Beside, Outcome, Readings, TopSpeeds};
use super::npc::{Mode, Npc};
use super::{Traffic, World};
use crate::ai::Goal;
use crate::ai::fire::Manoeuvre;
use crate::ai::lost_sight::SearchTimer;
use crate::ai::pursuit::{Body, Frame};
use crate::ai::swarm::SwarmLead;
use crate::catalog::GovtId;
use crate::chance::{Chance, NeverFires};
use crate::combat::ShipRef;
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
    npc.mode = Mode::HeldAtGate { ticks_left };
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
    crate::traffic::inertialess::drift(&mut drifted, &flown);
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
    searching.lost_sight = SearchTimer::Searching(10);
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
    let speeds = TopSpeeds::own_of(&npc);
    plunder::fly_docking(
        &mut npc,
        Some(&FAR),
        speeds,
        FlightRules::default(),
        None,
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
    held.mode = Mode::HeldAtGate { ticks_left: 1 };
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
        traffic.fly_holding(world, &BTreeSet::new(), &mut NeverFires);
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
            assert!(!ship.travel_set);
            assert_eq!(ship.owed_burst, None);
            assert_eq!(ship.mode, Mode::Coasting { ticks_left: 59 });
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
    for left in (1..59).rev() {
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
        traffic.fly_holding(world, &BTreeSet::new(), &mut NeverFires);
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
    traffic.fly_holding(world, &BTreeSet::new(), &mut NeverFires);
    traffic.fly_holding(world, &BTreeSet::new(), &mut NeverFires);
    let first = &traffic.npcs[0];
    assert!(
        close(first.state.velocity, facing(270.0) * 30.0),
        "{:?}",
        first.state
    );
    let mut ticks = 0;
    while traffic.npcs[0].owed_burst.is_some() {
        assert_eq!(traffic.npcs[0].goal, Goal::Snipe(ShipRef::Player));
        traffic.fly_holding(world, &BTreeSet::new(), &mut NeverFires);
        ticks += 1;
        assert!(ticks < 50_000, "never ended");
    }
    let ended = &traffic.npcs[0];
    assert_eq!(ended.goal, Goal::Idle);
    assert_eq!(ended.mode, Mode::Coasting { ticks_left: 59 });
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
    traffic.fly_holding(world, &BTreeSet::new(), &mut NeverFires);
    assert_eq!(traffic.npcs[0].owed_burst, Some(30.0));
    let mut ticks = 0;
    while traffic.npcs[0].owed_burst.is_some() {
        assert_eq!(traffic.npcs[0].goal, Goal::Snipe(ShipRef::Player));
        assert!(
            traffic.npcs[0].state.velocity.length() > 1.0,
            "tick {ticks}: {:?}",
            traffic.npcs[0].state
        );
        traffic.fly_holding(world, &BTreeSet::new(), &mut NeverFires);
        ticks += 1;
        assert!(ticks < 50_000, "never ended");
    }
    let ended = &traffic.npcs[0];
    assert!(ticks > 1, "it burst out: {ticks}");
    assert_eq!(ended.goal, Goal::Idle);
    assert_eq!(ended.state.velocity, Vec2::ZERO, "stopped dead");
    assert_eq!(ended.mode, Mode::Coasting { ticks_left: 59 });
    for left in (1..59).rev() {
        traffic.fly_holding(world, &BTreeSet::new(), &mut NeverFires);
        assert_eq!(traffic.npcs[0].mode, Mode::Coasting { ticks_left: left });
    }
}
