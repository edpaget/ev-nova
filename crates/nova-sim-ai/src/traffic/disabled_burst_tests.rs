//! Tests of a ship disabled or breaking up in a glide or a burst, and of a
//! ship disabled while it owes a burst: `_HandleShip` keeps bursting the
//! first ([`autopilot::fly`], @0x34424 → @0x34a54), and the second keeps
//! what it owes ([`held_burst`](super::held_burst)).

#![allow(clippy::float_cmp)]

use super::autopilot::{self, Beside, Readings, TopSpeeds};
use super::emerge::exit_speed;
use super::inertialess;
use super::npc::{Mode, Npc};
use super::spawn::glide_speed;
use super::{Traffic, World};
use crate::ai::{Behaviour, Goal, Surroundings};
use crate::catalog::GovtId;
use crate::chance::{Chance, NeverFires};
use crate::combat::ShipRef;
use crate::combat::hull::Condition;
use crate::flight::{self, DISABLED_DRAG, FlightRules, facing};
use crate::geometry::Vec2;
use crate::rulebook::RuleSource;
use crate::stats::{INERTIALESS_HULL, ShipStats};
use nova_sim_base::fixture::FAST;
use std::collections::BTreeMap;

const BREAKING_UP: Condition = Condition::Dying {
    ticks_left: 300,
    starting: false,
};

/// Off a whole degree, so a turn onto its own heading would show.
const HEADING: f32 = 90.5;

/// Speeds that end a burst with a lead there.
const LED: TopSpeeds = TopSpeeds {
    own: 6.0,
    lead: Some(9.0),
};

/// Speeds that end a burst with no lead.
const ALONE: TopSpeeds = TopSpeeds {
    own: 6.0,
    lead: None,
};

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

/// A ship of [`FAST`] in `mode` and `condition` at the centre, heading
/// [`HEADING`], at rest.
fn ship(mode: Mode, condition: Condition) -> Npc {
    let mut npc = crate::traffic::fixture::npc(1, ShipStats::new(FAST, &[]));
    npc.mode = mode;
    npc.condition = condition;
    npc.state.heading = HEADING;
    npc
}

/// [`ship`], inertialess, moving along its heading at 2.
fn inertialess_ship(mode: Mode, condition: Condition) -> Npc {
    let mut npc = ship(mode, condition);
    npc.stats = ShipStats::new(
        crate::handling::ShipFields {
            flags2: INERTIALESS_HULL,
            ..FAST
        },
        &[],
    );
    npc.speed = 2.0;
    npc.state.velocity = facing(HEADING) * 2.0;
    npc
}

/// One tick of `npc` through the autopilot at `speeds`.
fn fly(npc: &mut Npc, speeds: TopSpeeds) {
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
        &crate::ai::NO_WEAPONS,
        &mut NeverFires,
    );
}

fn close(a: Vec2, b: Vec2) -> bool {
    (a - b).length() < 1e-3
}

/// The mode of the tick after `mode`, on the same glide or burst.
fn next(mode: Mode) -> Mode {
    match mode {
        Mode::LeavingGate { tick } => Mode::LeavingGate { tick: tick + 1 },
        Mode::JumpingIn { tick } => Mode::JumpingIn { tick: tick + 1 },
        other => other,
    }
}

/// The speed of `mode`'s tick, on a glide or burst, and the next tick's.
fn speeds_of(mode: Mode) -> (f32, f32) {
    match mode {
        Mode::LeavingGate { tick } => (exit_speed(false, tick), exit_speed(false, tick + 1)),
        Mode::JumpingIn { tick } => (glide_speed(tick), glide_speed(tick + 1)),
        other => panic!("not bursting: {other:?}"),
    }
}

const BURSTS: [Mode; 2] = [Mode::LeavingGate { tick: 3 }, Mode::JumpingIn { tick: 3 }];

/// Flies `npc` from its glide or burst to its end at `speeds`, each tick's
/// move dragged and its velocity not. Its speed (+0x48), seeded at 2.0, is
/// dragged too (@0x3398f-0x339a1).
fn runs_to_its_cap_dragged(mut npc: Npc, speeds: TopSpeeds) -> Npc {
    npc.speed = 2.0;
    let mut own_speed = npc.speed;
    let mut position = npc.state.position;
    loop {
        let (speed, after) = speeds_of(npc.mode);
        let mode = npc.mode;
        fly(&mut npc, speeds);
        own_speed *= DISABLED_DRAG;
        assert_eq!(npc.speed, own_speed, "{mode:?}: +0x48 dragged");
        position = position + facing(HEADING) * (speed * DISABLED_DRAG);
        assert!(
            close(npc.state.position, position),
            "{mode:?}: {:?} at {position:?}",
            npc.state
        );
        assert_eq!(npc.state.heading, HEADING, "{mode:?}: no turn");
        assert!(
            close(npc.state.velocity, facing(HEADING) * speed),
            "{mode:?}: {:?}",
            npc.state
        );
        if after <= speeds.burst_cap() {
            return npc;
        }
        assert_eq!(npc.mode, next(mode));
    }
}

#[test]
fn a_ship_disabled_or_breaking_up_runs_its_glide_or_burst_to_its_cap_with_a_lead_then_flies() {
    for mode in BURSTS {
        for condition in [Condition::Disabled, BREAKING_UP] {
            let ended = runs_to_its_cap_dragged(ship(mode, condition), LED);
            assert_eq!(ended.mode, Mode::Flying, "{mode:?} {condition:?}");
        }
    }
}

#[test]
fn with_no_lead_it_ends_in_its_wait() {
    for mode in BURSTS {
        for condition in [Condition::Disabled, BREAKING_UP] {
            let ended = runs_to_its_cap_dragged(ship(mode, condition), ALONE);
            assert_eq!(
                ended.mode,
                Mode::Coasting { ticks_left: 59.0 },
                "{mode:?} {condition:?}"
            );
        }
    }
}

#[test]
fn an_inertialess_ship_drags_its_velocity_and_speed_and_takes_the_burst_as_its_speed() {
    for mode in BURSTS {
        for condition in [Condition::Disabled, BREAKING_UP] {
            let mut npc = inertialess_ship(mode, condition);
            let flown = npc.stats.handling;
            loop {
                let (speed, after) = speeds_of(npc.mode);
                let on = npc.mode;
                let mut moved = npc.clone();
                flight::drag(&mut moved.state);
                moved.speed *= DISABLED_DRAG;
                inertialess::drift(&mut moved, &flown, crate::speed_mult::SpeedMult::FULL);
                fly(&mut npc, ALONE);
                assert_eq!(npc.speed, speed, "{mode:?}: +0x48 follows the burst");
                assert_eq!(npc.state.heading, HEADING);
                assert!(
                    close(npc.state.position, moved.state.position),
                    "{mode:?}: {:?} at {:?}",
                    npc.state,
                    moved.state
                );
                if after <= ALONE.burst_cap() {
                    break;
                }
                assert!(npc.state.velocity.length() > 0.0, "{mode:?}");
                assert_eq!(npc.mode, next(on));
            }
            assert_eq!(npc.state.velocity, Vec2::ZERO, "{mode:?}: stopped dead");
            assert_eq!(npc.mode, Mode::Coasting { ticks_left: 59.0 }, "{mode:?}");
        }
    }
}

#[test]
fn a_destroyed_ship_still_drifts_with_no_glide_or_burst() {
    for (mode, after) in [
        (Mode::LeavingGate { tick: 3 }, Mode::Flying),
        (Mode::JumpingIn { tick: 3 }, Mode::JumpingIn { tick: 3 }),
    ] {
        let mut npc = ship(mode, Condition::Destroyed);
        npc.state.velocity = Vec2::new(1.0, -2.0);
        fly(&mut npc, LED);
        assert_eq!(npc.mode, after, "{mode:?}");
        assert_eq!(npc.state.position, Vec2::new(1.0, -2.0), "{mode:?}");
        assert_eq!(
            npc.state.velocity,
            Vec2::new(1.0, -2.0),
            "{mode:?}: no drag"
        );
    }
}

/// A ship of `npc`'s make, attacking the player far above, with its target
/// and goal set for the reset to clear, alone in a traffic.
fn alone(mut npc: Npc) -> (Traffic, World<'static>) {
    npc.goal = Goal::Attack(ShipRef::Player);
    npc.target = Some(ShipRef::Player);
    npc.govt = Some(GovtId(128));
    let mut traffic = Traffic::with_interval(1);
    traffic.add_npc(npc);
    let world = World {
        player: Some(crate::ai::fixture::player(0.0, -50_000.0)),
        ..World::new(&[])
    };
    (traffic, world)
}

#[test]
fn it_is_not_reset_through_its_burst_and_is_on_the_think_after() {
    for mode in BURSTS {
        let (mut traffic, world) = alone(ship(mode, Condition::Disabled));
        let mut ticks = 0;
        while matches!(
            traffic.npcs[0].mode,
            Mode::LeavingGate { .. } | Mode::JumpingIn { .. }
        ) {
            assert_eq!(
                traffic.npcs[0].goal,
                Goal::Attack(ShipRef::Player),
                "{mode:?} tick {ticks}: its goal 8 is no reset's"
            );
            assert_eq!(traffic.npcs[0].target, Some(ShipRef::Player));
            traffic.tick_in(&Keep, world, &[], &mut NeverFires);
            ticks += 1;
            assert!(ticks < 200, "{mode:?}: never ended");
        }
        assert!(ticks > 1, "{mode:?}: it burst");
        traffic.tick_in(&Keep, world, &[], &mut NeverFires);
        assert_eq!(traffic.npcs[0].goal, Goal::Idle, "{mode:?}: the reset");
        assert_eq!(traffic.npcs[0].target, None, "{mode:?}");
    }
}

/// A lead, and a ship it leads at the centre owing `owed`, disabled,
/// moving east at 2 and heading its retro heading.
fn owing_disabled(owed: f32) -> Traffic {
    let mut traffic = Traffic::with_interval(1);
    let lead = crate::traffic::fixture::npc(0, ShipStats::new(FAST, &[]));
    let lead = traffic.add_npc(lead);
    let mut npc = ship(Mode::Flying, Condition::Disabled);
    npc.goal = Goal::Wait;
    npc.leader = Some(lead);
    npc.owed_burst = Some(owed);
    npc.state.heading = 270.0;
    npc.state.velocity = Vec2::new(2.0, 0.0);
    traffic.add_npc(npc);
    traffic
}

#[test]
fn a_ship_disabled_owing_a_burst_keeps_it_through_its_reset_and_bursts_it_out_once_repaired() {
    // Its +0x50 is -1.0, so the ordinary path and the reset take it. Subs
    // 1, 0xe and 0x12 all test not disabled (@0x85378, @0x870b3, @0x85252),
    // so it writes no +0x30 and the preamble keeps the word.
    for owed in [30.0, 12.5] {
        let world = World {
            held_burst: RuleSource::Engine,
            ..World::new(&[])
        };
        let mut traffic = owing_disabled(owed);
        let mut velocity = Vec2::new(2.0, 0.0);
        let mut position = Vec2::ZERO;
        for tick in 0..3 {
            traffic.tick_in(&Keep, world, &[], &mut NeverFires);
            velocity = velocity * DISABLED_DRAG;
            position = position + velocity;
            let ship = &traffic.npcs[1];
            assert_eq!(ship.owed_burst, Some(owed), "tick {tick}");
            assert!(close(ship.state.velocity, velocity), "tick {tick}");
            assert!(close(ship.state.position, position), "tick {tick}");
            assert_eq!(ship.state.heading, 270.0, "tick {tick}");
            assert_eq!((ship.goal, ship.leader), (Goal::Idle, None), "tick {tick}");
        }
        // Repaired, its next stop's thrust bursts it out, with no lead by
        // then as the reset cleared +0x9a.
        let ship = &mut traffic.npcs[1];
        ship.condition = Condition::Intact;
        ship.goal = Goal::Wait;
        let accel = ship.stats.handling.accel;
        traffic.fly_tick(&world, &mut NeverFires);
        let ship = &traffic.npcs[1];
        assert!(
            close(ship.state.velocity, facing(270.0) * owed),
            "{owed}: {:?}",
            ship.state
        );
        assert_eq!(ship.owed_burst, Some(owed - accel), "{owed}");
        // It turns back on its burst's own speed, and bursts again as it
        // faces its retro heading, until it is down to its cap.
        let mut ticks = 0;
        while traffic.npcs[1].owed_burst.is_some() {
            traffic.fly_tick(&world, &mut NeverFires);
            ticks += 1;
            assert!(ticks < 50_000, "{owed}: never ended");
        }
        let ship = &traffic.npcs[1];
        assert_eq!(ship.owed_burst, None, "{owed}");
        assert_eq!(ship.goal, Goal::Idle, "{owed}");
        assert_eq!(ship.mode, Mode::Coasting { ticks_left: 59.0 }, "{owed}");
    }
}
