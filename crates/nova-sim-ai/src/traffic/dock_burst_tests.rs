//! Tests of a docking fighter's burst word over the traffic's real think
//! and fly halves, [`NovaAi`] deciding: sub 8 bursts it at its pull
//! ([`dock_flight`](super::dock_flight)), and
//! [`held_burst::settle`](super::held_burst::settle) ends that burst in
//! the same tick once it is within its cap, `_AIMakeShipStopJumping`
//! writing goal 0, which the next think's Return writes back to goal 5.

#![allow(clippy::float_cmp)]

use super::npc::{Mode, Npc, NpcId};
use super::{Traffic, World};
use crate::ai::Goal;
use crate::ai::nova::NovaAi;
use crate::bay::Carrier;
use crate::chance::Chance;
use crate::combat::ShipRef;
use crate::escort::{EscortDuty, EscortOrder, FORMATION_NUDGE, FormationSlot};
use crate::fleet::OrderWord;
use crate::flight::{ShipState, facing};
use crate::geometry::Vec2;
use crate::handling::ShipFields;
use crate::stats::ShipStats;
use crate::traffic::autopilot::Outcome;
use crate::traffic::fixture::npc;
use nova_sim_base::fixture::FAST;
use std::collections::BTreeSet;

/// A chance any draw of which fails the test: a fighter's burst ends with
/// no Rand(30) + 30, as it has a lead (@0x34d5c-0x34d5f).
struct NoDraws;

impl Chance for NoDraws {
    fn fires(&mut self, percent: u8) -> bool {
        panic!("asked to fire {percent}%");
    }

    fn below(&mut self, n: u32) -> u32 {
        panic!("asked to draw below {n}");
    }

    fn roll(&mut self, sides: u16) -> u16 {
        panic!("asked to roll {sides}");
    }
}

/// A fighter of [`FAST`] 60 above `carrier`'s ship at the centre, at rest
/// and facing it, its dock window 100 and the carrier's sprite 40 across.
fn fighter(id: u32, carrier: ShipRef) -> Npc {
    let mut ship = npc(id, ShipStats::new(FAST, &[]));
    ship.state = ShipState {
        position: Vec2::new(0.0, -60.0),
        velocity: Vec2::ZERO,
        heading: 180.0,
    };
    ship.carrier = Some(Carrier {
        ship: carrier,
        window: 100.0,
        reach: 40.0,
    });
    ship.goal = Goal::Dock(carrier);
    ship
}

/// One think half then one fly half of `traffic` in `world`: `NPC id` as
/// the think left it, then as the flight left it, if it is still there.
fn tick(traffic: &mut Traffic, world: &World, id: NpcId) -> (Npc, Option<Npc>) {
    let ai = NovaAi::default();
    traffic.think_holding(&ai, *world, &[], &mut NoDraws);
    let thought = of(traffic, id).unwrap_or_else(|| panic!("{id:?} gone"));
    traffic.fly_holding(*world, &BTreeSet::new(), &mut NoDraws);
    (thought, of(traffic, id))
}

fn of(traffic: &Traffic, id: NpcId) -> Option<Npc> {
    traffic.npcs().iter().find(|npc| npc.id == id).cloned()
}

fn close(a: Vec2, b: Vec2) -> bool {
    (a - b).length() < 1e-4
}

#[test]
fn the_players_fighter_returning_bursts_at_its_pull_and_drops_to_goal_0_each_frame_until_it_docks()
{
    let mut ship = fighter(0, ShipRef::Player);
    ship.escort = Some(EscortDuty {
        formation: FormationSlot::default(),
        order: Some(EscortOrder::Dock),
    });
    let id = ship.id;
    let pull = ship.stats.handling.accel * FORMATION_NUDGE;
    let mut traffic = Traffic::with_interval(1);
    traffic.npcs = vec![ship];
    let world = World {
        player: Some(crate::ai::fixture::player(0.0, 0.0)),
        ..World::new(&[])
    };
    let mut frames = 0;
    loop {
        let (thought, flown) = tick(&mut traffic, &world, id);
        assert_eq!(thought.goal, Goal::Dock(ShipRef::Player), "frame {frames}");
        let Some(flown) = flown else {
            break;
        };
        frames += 1;
        // 9 × 0.18 is within its cap, the lower of its own top speed and
        // the player's, 6: `_AIMakeShipStopJumping` (@0x82e77-0x82eba).
        assert_eq!(flown.goal, Goal::Idle, "frame {frames}");
        assert!(!flown.travel_set, "frame {frames}: +0x6c = -1");
        assert_eq!(flown.warp.stage, 0.0, "frame {frames}: +0x50 = 0");
        assert_eq!(flown.owed_burst, None, "frame {frames}");
        assert_eq!(flown.mode, Mode::Flying, "frame {frames}: no wait");
        assert!(
            close(flown.state.velocity, facing(flown.state.heading) * pull),
            "frame {frames}: {:?}",
            flown.state
        );
        assert!(frames < 50, "never docked");
    }
    assert_eq!(
        traffic.departed(),
        [(
            id,
            Outcome::Docked {
                carrier: ShipRef::Player,
                ship: of_ship(),
            }
        )]
    );
    // 60 - 40 at 1.8 pulled and 1.8 burst a frame.
    assert_eq!(frames, 6);
}

/// The ship class [`npc`] gives every ship.
fn of_ship() -> crate::catalog::ShipId {
    npc(0, ShipStats::new(FAST, &[])).ship
}

#[test]
fn an_npc_carriers_fighter_returning_to_a_slow_carrier_keeps_its_burst_and_goal_5() {
    // A carrier of top speed 1.0, below the fighter's 9 × 0.18: the burst
    // never comes down to its cap (`TopSpeeds::burst_cap`) inside the
    // window, so it goes on and the next frame's sub 8 writes it again.
    const SLOW: ShipFields = ShipFields { speed: 100, ..FAST };
    let mut carrier = npc(0, ShipStats::new(SLOW, &[]));
    carrier.no_ai = true;
    carrier.immobile = true;
    let carrier_id = carrier.id;
    let mut ship = fighter(1, ShipRef::Npc(carrier_id));
    ship.leader = Some(carrier_id);
    ship.order_word = OrderWord::of(Some(EscortOrder::Dock));
    let id = ship.id;
    let accel = ship.stats.handling.accel;
    let pull = accel * FORMATION_NUDGE;
    let mut traffic = Traffic::with_interval(1);
    traffic.npcs = vec![carrier, ship];
    let world = World {
        player: Some(crate::ai::fixture::player(0.0, 5000.0)),
        ..World::new(&[])
    };
    for frame in 1..=3 {
        let (thought, flown) = tick(&mut traffic, &world, id);
        let flown = flown.expect("not docked");
        assert_eq!(
            thought.goal,
            Goal::Dock(ShipRef::Npc(carrier_id)),
            "{frame}"
        );
        assert_eq!(flown.goal, Goal::Dock(ShipRef::Npc(carrier_id)), "{frame}");
        assert!(
            flown
                .owed_burst
                .is_some_and(|owed| (owed - (pull - accel)).abs() < 1e-5),
            "{frame}: {:?}",
            flown.owed_burst
        );
        assert!(
            close(flown.state.velocity, facing(flown.state.heading) * pull),
            "{frame}: {:?}",
            flown.state
        );
    }
}
