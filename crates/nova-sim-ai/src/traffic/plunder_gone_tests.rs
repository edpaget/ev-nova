//! Tests of a pirate docked with a victim whose slot went, as the
//! traffic's tick flies it with [`PirateWarship`] deciding (see
//! [`plunder`](crate::traffic::plunder) and
//! [`plunder_dock`](crate::traffic::plunder_dock)): the dock kept through
//! its wait whatever goal its think writes, the think's own sub writes
//! that end it, the hand-off to a ship that takes the berth, and the
//! plunder's completion on the record.
#![allow(clippy::float_cmp)]

use super::{Traffic, World};
use crate::ai::Goal;
use crate::ai::fire::AmmoLeft;
use crate::ai::fixture::{NEUTRAL, PIRATES, TRADERS, govts, ship, short_of_ammo};
use crate::ai::pirate::PirateWarship;
use crate::combat::ShipRef;
use crate::combat::hull::Condition;
use crate::geometry::Vec2;
use crate::govt::Governments;
use crate::traffic::npc::{AiType, Npc, NpcId};
use crate::traffic::plunder_dock::{DockSub, Docking};
use nova_sim_base::fixture::Draws;

const PIRATE: NpcId = NpcId(3);
const VICTIM: NpcId = NpcId(5);

/// The victim, NPC 5 in slot 5, a disabled trader boarded by the pirate,
/// 10 pixels off it and drifting at (0.25, 0).
fn victim() -> Npc {
    let mut victim = ship(VICTIM.0, TRADERS, AiType::WimpyTrader, 10.0, 0.0);
    victim.berth = 5;
    victim.condition = Condition::Disabled;
    victim.boarded = true;
    victim.hull.crew = 1;
    victim.hull.inherent_ai = 2;
    victim.state.velocity = Vec2::new(0.25, 0.0);
    victim
}

/// The pirate, NPC 3 in slot 3 at the centre, docked with the victim
/// (sub 0xf) under its dock wait of `wait`, its velocity matched, its
/// goal the attack its think gives a boarded victim, its travel stellar
/// the victim's slot, with fuel to jump when `fuel` says.
fn pirate(wait: u32, fuel: bool) -> Npc {
    let mut pirate = ship(PIRATE.0, PIRATES, AiType::Warship, 0.0, 0.0);
    pirate.berth = 3;
    pirate.hull.class_turn = 3.0;
    pirate.goal = Goal::Attack(ShipRef::Npc(VICTIM));
    pirate.target = Some(ShipRef::Npc(VICTIM));
    pirate.travel = Some(crate::ai::travel::of_slot(5));
    pirate.ai_wait = wait;
    pirate.state.velocity = Vec2::new(0.25, 0.0);
    pirate.class_fuel = if fuel { 300 } else { 0 };
    pirate.docking = Some(Docking {
        ship: ShipRef::Npc(VICTIM),
        sub: DockSub::Dock,
    });
    pirate
}

/// The docking a pirate keeps on the victim.
const ON_VICTIM: Option<Docking> = Some(Docking {
    ship: ShipRef::Npc(VICTIM),
    sub: DockSub::Dock,
});

/// Traffic of `npcs`, deciding every tick.
fn traffic_of(npcs: Vec<Npc>) -> Traffic {
    let mut traffic = Traffic::with_interval(1);
    traffic.npcs = npcs;
    traffic
}

/// `traffic` a tick on among the traders and pirates of `govts`, the
/// pirates deciding as pirates, every draw 0.
fn tick(traffic: &mut Traffic, govts: &Governments) {
    let world = World {
        govts,
        ..World::new(&[])
    };
    traffic.tick_in(
        &PirateWarship::default(),
        world,
        &[],
        &mut Draws::of(&[0; 64]),
    );
}

fn of(traffic: &Traffic, id: NpcId) -> &Npc {
    traffic
        .npcs()
        .iter()
        .find(|npc| npc.id == id)
        .expect("in the system")
}

#[test]
fn a_docked_pirate_whose_victims_slot_went_keeps_its_dock_through_its_wait_whatever_goal_its_think_writes()
 {
    // The dock wait skips the high level (@0x8d4b9-0x8d4d2), so sub 0xf
    // and the nav (+0x6c) stay whatever the think writes: with nothing to
    // find, its idle copy leaves (goal 2) or stops (goal 6) by its fuel
    // (@0x8c52d-0x8c5d2), and sub 0xf matches the record's velocity and
    // creeps to it (@0x86a3f-0x86a51, @0x8702d).
    let govts = govts(0, 0);
    for (fuel, goal) in [(true, Goal::JumpOut), (false, Goal::Wait)] {
        let mut traffic = traffic_of(vec![pirate(150, fuel), victim()]);
        traffic.remove_destroyed(VICTIM);
        tick(&mut traffic, &govts);
        let docked = of(&traffic, PIRATE);
        assert_eq!(docked.goal, goal, "fuel {fuel}");
        assert_eq!(docked.docking, ON_VICTIM, "fuel {fuel}: kept on the record");
        assert_eq!(docked.state.velocity, Vec2::new(0.25, 0.0), "fuel {fuel}");
        assert!(docked.state.position.x > 0.25, "fuel {fuel}: it crept");
        assert_eq!(docked.warp.stage, 0.0, "fuel {fuel}: no warp");
        let mut ticks = 1;
        while of(&traffic, PIRATE).ai_wait > 0 {
            assert_eq!(of(&traffic, PIRATE).docking, ON_VICTIM, "tick {ticks}");
            tick(&mut traffic, &govts);
            ticks += 1;
        }
        assert_eq!(ticks, 150, "fuel {fuel}: in the system to the wait's end");
        tick(&mut traffic, &govts);
        assert_eq!(
            of(&traffic, PIRATE).docking,
            None,
            "fuel {fuel}: the new goal's high level writes its own sub"
        );
    }
}

#[test]
fn a_docked_pirate_whose_victims_slot_went_and_who_finds_a_warship_keeps_its_dock_attacking_it() {
    // `_SelectWarshipTarget`'s pick, a warship: goal 4 (@0x8c743), sub
    // 0xf kept.
    let govts = govts(0, 0);
    let mut warship = ship(7, TRADERS, AiType::Warship, 0.0, 400.0);
    warship.berth = 7;
    warship.hull.inherent_ai = 3;
    warship.no_ai = true;
    let mut traffic = traffic_of(vec![pirate(150, true), victim(), warship]);
    traffic.remove_destroyed(VICTIM);
    tick(&mut traffic, &govts);
    let docked = of(&traffic, PIRATE);
    assert_eq!(docked.goal, Goal::Attack(ShipRef::Npc(NpcId(7))));
    assert_eq!(docked.docking, ON_VICTIM);
}

#[test]
fn a_docked_pirates_dry_drop_or_count_wait_under_its_dock_wait_drops_its_dock() {
    // The dry drop writes sub 0 (@0x8c878), as does the count wait's entry
    // (@0x8c6fe), each in the think, so the dock ends under the wait.
    let govts = govts(0, 0);
    let dry = short_of_ammo(pirate(150, true), AmmoLeft::Dry, false);
    let mut traffic = traffic_of(vec![dry, victim()]);
    tick(&mut traffic, &govts);
    let dropped = of(&traffic, PIRATE);
    assert_eq!(dropped.goal, Goal::Idle, "dry");
    assert_eq!(dropped.docking, None, "dry");
    let mut boarder = ship(8, PIRATES, AiType::Warship, 0.0, 300.0);
    boarder.berth = 8;
    boarder.goal = Goal::Plunder {
        ship: ShipRef::Npc(VICTIM),
        hopeless: false,
    };
    boarder.target = Some(ShipRef::Npc(VICTIM));
    boarder.no_ai = true;
    let mut traffic = traffic_of(vec![pirate(150, true), victim(), boarder]);
    tick(&mut traffic, &govts);
    let waiting = of(&traffic, PIRATE);
    assert_eq!(waiting.goal, Goal::CountWait, "count wait");
    assert_eq!(waiting.docking, None, "count wait");
}

#[test]
fn a_docked_pirate_whose_victims_slot_went_and_who_finds_a_disabled_ship_drops_its_dock() {
    // `_AIFindShipToPlunder` writes +0x70, goal 0xd and sub 0 itself
    // (@0x7fe98-0x7fea5), ending the dock under the wait; the high level
    // docks with the new victim once the wait is out.
    let govts = govts(0, 0);
    let mut wreck = ship(7, TRADERS, AiType::WimpyTrader, 0.0, 400.0);
    wreck.berth = 7;
    wreck.condition = Condition::Disabled;
    wreck.hull.crew = 1;
    wreck.hull.inherent_ai = 2;
    let mut traffic = traffic_of(vec![pirate(150, true), victim(), wreck]);
    traffic.remove_destroyed(VICTIM);
    tick(&mut traffic, &govts);
    let found = of(&traffic, PIRATE);
    assert!(
        matches!(found.goal, Goal::Plunder { ship, .. } if ship == ShipRef::Npc(NpcId(7))),
        "{:?}",
        found.goal
    );
    assert_eq!(found.docking, None);
}

/// An intact trader, NPC `id` in slot `id` at (0, `y`), of a class with a
/// crew and `InherentAI` 2: the warship's pick, which the classification
/// plunders (@0x8c738).
fn trader(id: u32, y: f32) -> Npc {
    let mut trader = ship(id, TRADERS, AiType::WimpyTrader, 0.0, y);
    trader.berth = id;
    trader.hull.crew = 1;
    trader.hull.inherent_ai = 2;
    trader.no_ai = true;
    trader
}

#[test]
fn a_slot_taken_again_moves_a_docked_pirates_dock_onto_the_newcomer() {
    // A spawn rewrites the slot (@0x3c8b1-0x3c90b), unboarded (+0xb9 0,
    // @0x3c981), and sub 0xf goes on reading the nav +0x6c
    // (@0x86a19-0x86a51): it matches the newcomer's velocity and creeps
    // to its place.
    let govts = govts(0, 0);
    let mut traffic = traffic_of(vec![pirate(150, true), victim()]);
    traffic.remove_destroyed(VICTIM);
    tick(&mut traffic, &govts);
    // Of a government neither side fights.
    let mut newcomer = ship(0, NEUTRAL, AiType::WimpyTrader, 0.0, 50.0);
    newcomer.state.velocity = Vec2::new(0.25, 0.4);
    newcomer.no_ai = true;
    let newcomer = traffic.add_npc_at(newcomer, 5);
    let held = of(&traffic, PIRATE);
    assert_eq!(
        held.docking.map(|docking| docking.ship),
        Some(ShipRef::Npc(newcomer))
    );
    assert_ne!(
        held.target,
        Some(ShipRef::Npc(newcomer)),
        "+0x70 named none"
    );
    assert!(!of(&traffic, newcomer).boarded);
    let before = held.state.position;
    tick(&mut traffic, &govts);
    let flown = of(&traffic, PIRATE);
    assert_eq!(flown.state.velocity, Vec2::new(0.25, 0.4), "the newcomer's");
    assert!(
        flown.state.position.y > before.y + 0.4,
        "crept toward the newcomer: {:?}",
        flown.state.position
    );
}

/// `traffic` ticked until the pirate drifts or `most` ticks pass, `each`
/// asked of it after every tick that doesn't: the ticks it took.
fn until_it_drifts(
    traffic: &mut Traffic,
    govts: &Governments,
    most: u32,
    each: &dyn Fn(&Npc),
) -> u32 {
    for ticks in 1..=most {
        tick(traffic, govts);
        if of(traffic, PIRATE).goal == Goal::Drift {
            return ticks;
        }
        each(of(traffic, PIRATE));
    }
    panic!("no drift in {most} ticks");
}

#[test]
fn a_docked_pirate_whose_victim_went_and_who_found_another_to_plunder_completes_on_the_record_and_drifts()
 {
    // `_AIFindShipToPlunder` finds no wreck, so the warship's pick, an
    // intact trader, is plundered (goal 0xd, @0x8c738) with sub 0xf and
    // the nav kept under the wait, goal 0xd's high level skipped (so the
    // nav, +0x6c, still names the dead slot). With the wait at 100 sub
    // 0xf completes (@0x86d54-0x86d7e, @0x86f3c-0x86f93) on the record:
    // nothing plundered, the trader untouched, the pirate drifting.
    let govts = govts(0, 0);
    let mut traffic = traffic_of(vec![pirate(120, true), victim(), trader(7, 400.0)]);
    traffic.remove_destroyed(VICTIM);
    let ticks = until_it_drifts(&mut traffic, &govts, 30, &|pirate| {
        assert!(
            matches!(pirate.goal, Goal::Plunder { ship, .. } if ship == ShipRef::Npc(NpcId(7))),
            "{:?}",
            pirate.goal
        );
        assert_eq!(pirate.docking, ON_VICTIM);
        assert_eq!(
            pirate.travel,
            Some(crate::ai::travel::of_slot(5)),
            "the high level writes no +0x6c under its wait"
        );
    });
    assert_eq!(ticks, 21, "the wait at 100 as the low level reads it");
    let drifting = of(&traffic, PIRATE);
    assert_eq!(
        drifting.ai_wait,
        crate::ai::plunder_contents::PLUNDER_HOLD - 1
    );
    assert_eq!(
        (drifting.docking, drifting.target, drifting.travel),
        (None, None, None)
    );
    assert!(!of(&traffic, NpcId(7)).boarded, "the trader untouched");
    assert_eq!(traffic.take_pirate_events(), [], "nothing plundered");
}

#[test]
fn a_docked_pirate_whose_victim_went_and_who_found_a_warship_plunders_the_record_at_its_waits_end_and_drifts()
 {
    // Goal 4 with sub 0xf completes nothing at the slot; with the wait out
    // the type think's plunder (@0x8c74c-0x8c7ae) runs on the dead nav,
    // which `_AIPlunderShipContents` passes over (@0x83fcd-0x83fd4).
    let govts = govts(0, 0);
    let mut warship = ship(7, TRADERS, AiType::Warship, 0.0, 400.0);
    warship.berth = 7;
    warship.hull.inherent_ai = 3;
    warship.no_ai = true;
    let mut traffic = traffic_of(vec![pirate(30, true), victim(), warship]);
    traffic.remove_destroyed(VICTIM);
    let ticks = until_it_drifts(&mut traffic, &govts, 40, &|pirate| {
        assert_eq!(pirate.goal, Goal::Attack(ShipRef::Npc(NpcId(7))));
        assert_eq!(pirate.docking, ON_VICTIM);
    });
    assert_eq!(ticks, 31, "the think after its wait ran out");
    assert_eq!(of(&traffic, PIRATE).docking, None);
    assert_eq!(traffic.take_pirate_events(), [], "nothing plundered");
}

#[test]
fn a_docked_pirates_completion_after_its_victims_berth_was_taken_plunders_the_newcomer() {
    // `_AIPlunderShipContents` tests only `_IsDying` and +0xb8
    // (@0x83fbd-0x83fd4), so the nav's newcomer, a live ship, is
    // plundered in full, and the event queued at the pirate's berth.
    let govts = govts(0, 0);
    let mut traffic = traffic_of(vec![pirate(120, true), victim()]);
    traffic.remove_destroyed(VICTIM);
    tick(&mut traffic, &govts);
    // An intact trader beside the pirate, matched: the warship's pick, and
    // docked with within the box.
    let mut beside = trader(0, 0.0);
    beside.state = of(&traffic, PIRATE).state;
    beside.state.position.y += 1.0;
    let newcomer = traffic.add_npc_at(beside, 5);
    until_it_drifts(&mut traffic, &govts, 30, &|_| {});
    assert!(matches!(
        traffic.take_pirate_events()[..],
        [crate::traffic::plunder::PirateEvent::Plundered { pirate: PIRATE, victim, .. }]
            if victim == newcomer
    ));
}

#[test]
fn the_players_hit_on_a_docked_pirate_turns_it_on_the_player_and_its_completion_plunders_the_live_victim()
 {
    // The player's hit is answered (only an NPC's spares sub 0xf,
    // @0x3bb0a-0x3bb1a): `_AIMakeShipAttackPlayer` writes goal 4 and +0x70
    // 0, leaving sub 0xf (@0x89d11-0x89d4d), and the wait is cut to 20
    // (@0x3be6e-0x3be7c). The think after it (here the same tick's, the
    // strikes answered first) plunders the player, unboarded with a crew
    // (@0x8c738), and once it has crept inside the box with the wait at
    // most 100 sub 0xf completes. The original's call also writes +0x6c -1 (@0x89d47), so
    // its completion reads the slot before the first (@0x86f51), which
    // can't be replicated: the docking here still names the victim, which
    // is plundered.
    let govts = govts(0, 0);
    let mut player = crate::ai::fixture::player(0.0, 500.0);
    player.hull.crew = 1;
    let world = World {
        govts: &govts,
        player: Some(player),
        ..World::new(&[])
    };
    let mut traffic = traffic_of(vec![pirate(150, true), victim()]);
    let hit = crate::combat::fixture::strike(ShipRef::Npc(PIRATE), ShipRef::Player);
    let behaviour = PirateWarship::default();
    traffic.tick_in(&behaviour, world, &[hit], &mut Draws::of(&[0; 64]));
    let turned = of(&traffic, PIRATE);
    assert_eq!(
        turned.goal,
        Goal::Plunder {
            ship: ShipRef::Player,
            hopeless: false
        }
    );
    assert_eq!(turned.docking, ON_VICTIM);
    assert_eq!(turned.ai_wait, crate::traffic::autopilot::HIT_WAIT - 1);
    let mut ticks = 1;
    while of(&traffic, PIRATE).goal != Goal::Drift {
        assert_eq!(traffic.take_pirate_events(), [], "tick {ticks}");
        assert!(ticks < 10, "no completion");
        traffic.tick_in(&behaviour, world, &[], &mut Draws::of(&[0; 64]));
        ticks += 1;
    }
    assert!(matches!(
        traffic.take_pirate_events()[..],
        [crate::traffic::plunder::PirateEvent::Plundered {
            pirate: PIRATE,
            victim: VICTIM,
            ..
        }]
    ));
}
