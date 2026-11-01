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
use crate::ai::fixture::{PIRATES, TRADERS, govts, ship, short_of_ammo};
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
