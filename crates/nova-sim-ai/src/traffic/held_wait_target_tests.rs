//! Tests of a pirate hit in its count wait or its drift as the traffic
//! answers the hit and then ticks: `_DamageShip` sets +0x70 (@0x3bcf6) in
//! the frame of the hit, after the thinks, so the frame's draw and the
//! next frame's lower slots read it; the ship's own next high level clears
//! it (@0x8d480, @0x8eb85; see [`board_wait::lapse`](super::board_wait::lapse)
//! and [`plunder::drift`](super::plunder::drift)). `_ExtendedIsThreatToShip`
//! reads it meanwhile; `_IsThreatToPlayer` spares the ship while its wait
//! runs (@0x7f518-0x7f51b). These pin the wiring of the clear to the tick,
//! not a rule: the hit comes already weighed and adds no provocation
//! ([`Traffic::answer`]), as a missile lock's does (`answer_flag`'s "No
//! damage" row), so nothing but the clear moves the target. As the count
//! wait runs out, the head keeps +0x70 for every type (@0x8d497-0x8d4a3),
//! which the frame ends with whatever the type's target step answers.

use super::{Traffic, World};
use crate::ai::Goal;
use crate::ai::fixture::{PIRATES, POLICE, TRADERS, govts, n, player, ship, sites};
use crate::ai::nova::NovaAi;
use crate::ai::pirate::PirateWarship;
use crate::combat::fixture::strike;
use crate::combat::{Nominal, ShipRef, Strike};
use crate::escort::{EscortDuty, FormationSlot};
use crate::traffic::fixture::count_waiting;
use crate::traffic::npc::{AiType, Npc, NpcId};
use nova_sim_base::fixture::Draws;

const HITTER: NpcId = NpcId(2);

/// NPC 2's answered hit on NPC 1, adding no provocation.
fn glancing() -> Strike {
    Strike {
        damage: 0.0,
        nominal: Nominal::default(),
        ..strike(ShipRef::Npc(NpcId(1)), ShipRef::Npc(HITTER))
    }
}

/// A pirate (1) at the centre in `goal` with 30 ticks of its wait to run,
/// and NPC 2, a ship the player leads, out of its reach; deciding every
/// tick, as pirates.
fn scene(goal: Goal) -> Traffic {
    let mut pirate = ship(1, PIRATES, AiType::Warship, 0.0, 0.0);
    pirate.berth = 1;
    count_waiting(&mut pirate, 30);
    pirate.goal = goal;
    let mut hitter = ship(2, TRADERS, AiType::Warship, 6000.0, 0.0);
    hitter.berth = 2;
    hitter.escort = Some(EscortDuty {
        formation: FormationSlot::default(),
        order: None,
    });
    let mut traffic = Traffic::with_interval(1);
    traffic.npcs = vec![pirate, hitter];
    traffic
}

fn pirate(traffic: &Traffic) -> &Npc {
    &traffic.npcs[0]
}

/// Answers the glancing hit and checks what the threat tests read of the
/// pirate holding it, then ticks once taking no strikes; the player far
/// below.
fn hit_then_tick(traffic: &mut Traffic) {
    let govts = govts(0, 0);
    let sites = sites();
    let world = World {
        player: Some(player(0.0, -8000.0)),
        govts: &govts,
        ..World::new(&sites)
    };
    traffic.answer(&PirateWarship::default(), &world, &[glancing()]);
    let hit = pirate(traffic).clone();
    assert_eq!(hit.target, Some(n(2)), "the hit's +0x70 (@0x3bcf6)");
    assert!(
        hit.threatens_npc_among(HITTER, &traffic.npcs),
        "`_ExtendedIsThreatToShip` reads no wait"
    );
    assert!(
        !hit.threatens_player_among(&traffic.npcs),
        "`_IsThreatToPlayer` spares it while its wait runs"
    );
    let out = Npc {
        ai_wait: 0.0,
        ..hit
    };
    assert!(
        out.threatens_player_among(&traffic.npcs),
        "its target is led by the player"
    );
    traffic.tick_in(
        &PirateWarship::default(),
        world,
        &[],
        &mut Draws::of(&[0; 16]),
    );
}

#[test]
fn a_count_waiting_pirate_holds_a_hits_target_until_its_next_run() {
    let mut traffic = scene(Goal::CountWait);
    hit_then_tick(&mut traffic);
    assert_eq!(pirate(&traffic).goal, Goal::CountWait);
    assert_eq!(pirate(&traffic).target, None, "@0x8d480");
}

#[test]
fn a_drifting_pirate_holds_a_hits_target_until_its_next_run() {
    let mut traffic = scene(Goal::Drift);
    hit_then_tick(&mut traffic);
    assert_eq!(pirate(&traffic).goal, Goal::Drift);
    assert_eq!(pirate(&traffic).target, None, "@0x8eb85");
}

/// A police ship (1) of `ai_type` at the centre in its count wait, the
/// wait out, holding NPC 2, a trader out of its reach, which is there as
/// `there` says; deciding every tick.
fn run_out(ai_type: AiType, there: bool) -> Traffic {
    let mut waiting = ship(1, POLICE, ai_type, 0.0, 0.0);
    waiting.berth = 1;
    count_waiting(&mut waiting, 0);
    waiting.target = Some(n(2));
    let mut trader = ship(2, TRADERS, AiType::WimpyTrader, 6000.0, 0.0);
    trader.berth = 2;
    let mut traffic = Traffic::with_interval(1);
    traffic.npcs = if there {
        vec![waiting, trader]
    } else {
        vec![waiting]
    };
    traffic
}

#[test]
fn a_count_wait_running_out_keeps_the_target_whatever_the_type() {
    // With +0x4c out the handler's head writes goal 0 and sub 0 and keeps
    // +0x70 (@0x8d497-0x8d4a3), reading no type; the slot test after it
    // clears a gone slot's (@0x8d4e4-0x8d50d).
    let govts = govts(0, 0);
    let sites = sites();
    for ai_type in [AiType::Warship, AiType::Interceptor, AiType::WimpyTrader] {
        for there in [true, false] {
            let mut traffic = run_out(ai_type, there);
            let world = World {
                player: Some(player(0.0, -8000.0)),
                govts: &govts,
                ..World::new(&sites)
            };
            traffic.tick_in(&NovaAi::default(), world, &[], &mut Draws::of(&[0; 16]));
            let ended = &traffic.npcs[0];
            assert_eq!(ended.goal, Goal::Idle, "{ai_type:?}");
            assert_eq!(
                ended.target,
                there.then_some(n(2)),
                "{ai_type:?}, there: {there}"
            );
        }
    }
}
