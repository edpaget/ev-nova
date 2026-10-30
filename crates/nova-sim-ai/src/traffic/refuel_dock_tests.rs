//! Tests of a ship refuelling the player, or another ship, as the
//! traffic's tick flies it (see [`refuel_dock`](crate::traffic::refuel_dock)):
//! its high level, the docking's close it shares with the plunder, the
//! stop, the fuel in berth order and the completion.
#![allow(clippy::float_cmp)]

use super::{Traffic, World};
use crate::ai::Goal;
use crate::combat::ShipRef;
use crate::combat::hull::Condition;
use crate::flight::FlightRules;
use crate::geometry::Vec2;
use crate::hail::Help;
use crate::reserves::Gauge;
use crate::rulebook::RuleSource;
use crate::stats::ShipStats;
use crate::traffic::autopilot::TopSpeeds;
use crate::traffic::fixture::assisting::{Recording, player_in};
use crate::traffic::fixture::npc;
use crate::traffic::inertialess::Inertia;
use crate::traffic::leave_flight::{self, BRAKE_DAMPING};
use crate::traffic::npc::{Npc, NpcId};
use crate::traffic::on_line::OnLine;
use crate::traffic::plunder;
use crate::traffic::plunder_dock::{DockSub, Docking};
use nova_sim_base::fixture::{FAST, Scripted};
use nova_sim_catalog::catalog::ShipId;

const HELPER: NpcId = NpcId(1);

/// NPC `id` at `berth`, refuelling the player from (`x`, `y`) at (`vx`,
/// `vy`), facing `heading`, its class turn 3 (a box of 105).
fn refueller(id: u32, berth: u32, at: (f32, f32), moving: (f32, f32), heading: f32) -> Npc {
    let mut ship = npc(id, ShipStats::new(FAST, &[]));
    ship.berth = berth;
    ship.goal = Goal::Assist(Help::Refuel);
    ship.target = Some(ShipRef::Player);
    ship.hull.class_turn = 3.0;
    ship.state.position = Vec2::new(at.0, at.1);
    ship.state.velocity = Vec2::new(moving.0, moving.1);
    ship.state.heading = heading;
    ship
}

/// Traffic of `npcs`, deciding every tick.
fn traffic_of(npcs: Vec<Npc>) -> Traffic {
    let mut traffic = Traffic::with_interval(1);
    traffic.npcs = npcs;
    traffic
}

/// Traffic of NPC 1 alone, refuelling the player from (`x`, `y`) at rest,
/// facing up.
fn refuelling(x: f32, y: f32) -> Traffic {
    traffic_of(vec![refueller(HELPER.0, 1, (x, y), (0.0, 0.0), 0.0)])
}

fn ship(traffic: &Traffic, id: NpcId) -> &Npc {
    traffic
        .npcs()
        .iter()
        .find(|npc| npc.id == id)
        .expect("in the system")
}

/// A world holding the intact player at the centre with `fuel` of `max`,
/// refuelled as `cap` reads it.
fn world(fuel: f32, max: f32, cap: RuleSource) -> World<'static> {
    let mut player = player_in(Condition::Intact);
    player.reserves.fuel = Gauge { now: fuel, max };
    World {
        player: Some(player),
        refuel_cap: cap,
        ..World::new(&[])
    }
}

/// `traffic` a tick on in `world`.
fn tick(traffic: &mut Traffic, recording: &Recording, world: &World) {
    traffic.tick_in(recording, *world, &[], &mut Scripted::default());
}

/// `npc` flown a tick of its docking at the player, docking as `sub`.
fn docked_by_hand(npc: &Npc, sub: DockSub) -> Npc {
    let mut flown = npc.clone();
    flown.docking = Some(Docking {
        ship: ShipRef::Player,
        sub,
    });
    let speeds = TopSpeeds::own_of(&flown);
    plunder::fly_docking(
        &mut flown,
        Some(&player_in(Condition::Intact).state),
        speeds,
        FlightRules::default(),
        None,
        &mut Scripted::default(),
    );
    flown
}

#[test]
fn far_off_a_refueller_closes_on_the_player_through_sub_0xb() {
    let mut traffic = refuelling(0.0, 600.0);
    let closing = docked_by_hand(&traffic.npcs[0], DockSub::Close);
    tick(
        &mut traffic,
        &Recording::default(),
        &world(40.0, 300.0, RuleSource::Engine),
    );
    let flown = ship(&traffic, HELPER);
    assert_eq!(
        flown.docking.map(|docking| docking.sub),
        Some(DockSub::Close)
    );
    assert_eq!(flown.state, closing.state);
    assert_eq!(traffic.take_fuel_given(), 0.0);
}

#[test]
fn inside_the_box_a_moving_refueller_stops_through_sub_1_with_goal_9s_damping() {
    // Moving down at 1 a tick, facing down: off its retro heading of 0.
    let start = refueller(HELPER.0, 1, (50.0, 0.0), (0.0, 1.0), 180.0);
    let stopping = docked_by_hand(&start, DockSub::Stop);
    let mut traffic = traffic_of(vec![start]);
    tick(
        &mut traffic,
        &Recording::default(),
        &world(40.0, 300.0, RuleSource::Engine),
    );
    let flown = ship(&traffic, HELPER);
    assert_eq!(
        flown.docking.map(|docking| docking.sub),
        Some(DockSub::Stop)
    );
    assert_eq!(flown.state, stopping.state);
    // Damped, unlike a leaver braking the same way.
    let mut leaving = refueller(HELPER.0, 1, (50.0, 0.0), (0.0, 1.0), 180.0);
    let handling = leaving.stats.handling;
    leave_flight::stop(&mut leaving, &handling, Inertia::Inertial, false);
    assert_eq!(leaving.state.velocity, Vec2::new(0.0, 1.0));
    #[allow(clippy::cast_possible_truncation)]
    let cut = (f64::from(1.0_f32) * BRAKE_DAMPING) as f32;
    assert_eq!(flown.state.velocity, Vec2::new(0.0, cut), "cut, no thrust");
    assert_eq!(traffic.take_fuel_given(), 0.0);
}

#[test]
fn a_slow_refueller_inside_the_box_zeroes_its_velocity_gives_a_unit_and_closes_from_rest_the_same_tick()
 {
    // Facing the player, to its left, drifting slowly up.
    let start = refueller(HELPER.0, 1, (50.0, 0.0), (0.0, -0.3), 270.0);
    let mut at_rest = start.clone();
    at_rest.state.velocity = Vec2::ZERO;
    let closing = docked_by_hand(&at_rest, DockSub::Close);
    let mut traffic = traffic_of(vec![start]);
    tick(
        &mut traffic,
        &Recording::default(),
        &world(40.0, 300.0, RuleSource::Engine),
    );
    let flown = ship(&traffic, HELPER);
    assert_eq!(
        flown.docking.map(|docking| docking.sub),
        Some(DockSub::Close)
    );
    assert_eq!(flown.state, closing.state, "closing from rest");
    assert_ne!(flown.state.velocity, Vec2::ZERO, "it thrust");
    assert_eq!(traffic.take_fuel_given(), 1.0);
    assert_eq!(traffic.take_fuel_given(), 0.0, "taken");
}

#[test]
fn a_refuel_ends_on_the_think_after_the_fuel_passes_100_and_queues_its_note() {
    let mut traffic = refuelling(0.0, 0.0);
    let recording = Recording::default();
    tick(
        &mut traffic,
        &recording,
        &world(100.0, 300.0, RuleSource::Engine),
    );
    assert_eq!(traffic.take_fuel_given(), 1.0, "100 gains a unit");
    assert_eq!(traffic.take_on_line(), []);
    assert_eq!(ship(&traffic, HELPER).goal, Goal::Assist(Help::Refuel));
    tick(
        &mut traffic,
        &recording,
        &world(101.0, 300.0, RuleSource::Engine),
    );
    assert_eq!(traffic.take_fuel_given(), 0.0);
    assert_eq!(
        traffic.take_on_line(),
        [OnLine::RefuelDone(HELPER, ShipId(128))]
    );
    let done = ship(&traffic, HELPER);
    assert_eq!(done.goal, Goal::Idle);
    assert_eq!((done.target, done.docking), (None, None));
    assert!(!done.travel_set);
}

#[test]
fn two_refuellers_read_the_fuel_in_berth_order() {
    // The later in the list holds the lower berth, and refuels first.
    let mut traffic = traffic_of(vec![
        refueller(2, 5, (0.0, 0.0), (0.0, 0.0), 0.0),
        refueller(3, 4, (10.0, 0.0), (0.0, 0.0), 0.0),
    ]);
    tick(
        &mut traffic,
        &Recording::default(),
        &world(100.0, 300.0, RuleSource::Engine),
    );
    assert_eq!(traffic.take_fuel_given(), 1.0);
    assert_eq!(ship(&traffic, NpcId(3)).goal, Goal::Assist(Help::Refuel));
    assert_eq!(ship(&traffic, NpcId(2)).goal, Goal::Idle, "it read 101");
    assert_eq!(
        traffic.take_on_line(),
        [OnLine::RefuelDone(NpcId(2), ShipId(128))]
    );
}

#[test]
fn a_refueller_targeting_an_npc_raises_that_ships_fuel_and_ends_silently() {
    let mut helper = refueller(HELPER.0, 1, (10.0, 0.0), (0.0, 0.0), 0.0);
    helper.target = Some(ShipRef::Npc(NpcId(2)));
    let mut target = npc(2, ShipStats::new(FAST, &[]));
    target.berth = 2;
    target.reserves.fuel = Gauge {
        now: 50.0,
        max: 300.0,
    };
    let mut traffic = traffic_of(vec![helper, target]);
    let recording = Recording::default();
    tick(
        &mut traffic,
        &recording,
        &world(40.0, 300.0, RuleSource::Engine),
    );
    assert_eq!(ship(&traffic, NpcId(2)).reserves.fuel.now, 51.0);
    assert_eq!(traffic.take_fuel_given(), 0.0, "none to the player");
    assert_eq!(
        ship(&traffic, HELPER).docking,
        Some(Docking {
            ship: ShipRef::Npc(NpcId(2)),
            sub: DockSub::Close
        })
    );
    traffic.npcs[1].reserves.fuel.now = 101.0;
    tick(
        &mut traffic,
        &recording,
        &world(40.0, 300.0, RuleSource::Engine),
    );
    assert_eq!(ship(&traffic, NpcId(2)).reserves.fuel.now, 101.0);
    assert_eq!(traffic.take_on_line(), [], "silent");
    assert_eq!(ship(&traffic, HELPER).goal, Goal::Idle);
}

#[test]
fn a_refueller_with_no_target_flies_no_high_level_and_coasts() {
    let mut traffic = refuelling(50.0, 0.0);
    traffic.npcs[0].target = None;
    traffic.npcs[0].state.velocity = Vec2::new(1.0, -0.5);
    tick(
        &mut traffic,
        &Recording::default(),
        &world(40.0, 300.0, RuleSource::Engine),
    );
    let flown = ship(&traffic, HELPER);
    assert_eq!(flown.docking, None);
    assert_eq!(flown.state.velocity, Vec2::new(1.0, -0.5));
    assert_eq!(flown.state.position, Vec2::new(51.0, -0.5));
    assert_eq!(traffic.take_fuel_given(), 0.0);
}

#[test]
fn a_refuellers_high_level_skips_while_its_wait_runs() {
    let mut traffic = refuelling(0.0, 0.0);
    traffic.npcs[0].ai_wait = 5;
    tick(
        &mut traffic,
        &Recording::default(),
        &world(40.0, 300.0, RuleSource::Engine),
    );
    assert_eq!(ship(&traffic, HELPER).docking, None);
    assert_eq!(traffic.take_fuel_given(), 0.0);
}

#[test]
fn a_refueller_unprovoked_flies_its_high_level_though_it_never_decides() {
    let mut traffic = refuelling(0.0, 600.0);
    let recording = Recording::default();
    tick(
        &mut traffic,
        &recording,
        &world(40.0, 300.0, RuleSource::Engine),
    );
    assert_eq!(*recording.decided.borrow(), []);
    assert_eq!(
        ship(&traffic, HELPER).docking.map(|docking| docking.sub),
        Some(DockSub::Close)
    );
}

#[test]
fn the_refuel_cap_reading_ends_a_full_tanks_refuel() {
    let mut capped = refuelling(0.0, 0.0);
    let recording = Recording::default();
    tick(
        &mut capped,
        &recording,
        &world(80.0, 80.0, RuleSource::Bible),
    );
    assert_eq!(capped.take_fuel_given(), 0.0);
    assert_eq!(ship(&capped, HELPER).goal, Goal::Idle);
    let mut uncapped = refuelling(0.0, 0.0);
    tick(
        &mut uncapped,
        &recording,
        &world(80.0, 80.0, RuleSource::Engine),
    );
    assert_eq!(uncapped.take_fuel_given(), 1.0, "past the tank");
    assert_eq!(ship(&uncapped, HELPER).goal, Goal::Assist(Help::Refuel));
}

#[test]
fn an_npcs_hit_turns_a_refueller_onto_its_attacker_which_it_boxes_on_and_refuels() {
    let helper = refueller(HELPER.0, 1, (10.0, 0.0), (0.0, 0.0), 0.0);
    let mut attacker = npc(2, ShipStats::new(FAST, &[]));
    attacker.berth = 2;
    attacker.reserves.fuel = Gauge {
        now: 99.5,
        max: 300.0,
    };
    let mut traffic = traffic_of(vec![helper, attacker]);
    let recording = Recording::default();
    let world = world(40.0, 300.0, RuleSource::Engine);
    traffic.tick_in(
        &recording,
        world,
        &[crate::combat::fixture::strike(
            ShipRef::Npc(HELPER),
            ShipRef::Npc(NpcId(2)),
        )],
        &mut Scripted::default(),
    );
    let turned = ship(&traffic, HELPER);
    assert_eq!(turned.goal, Goal::Assist(Help::Refuel), "no goal write");
    assert_eq!(turned.target, Some(ShipRef::Npc(NpcId(2))), "@0x3bcf6");
    assert_eq!(turned.provoked, 24.0, "its nominal damage");
    assert_eq!(ship(&traffic, NpcId(2)).reserves.fuel.now, 100.5);
    assert_eq!(traffic.take_fuel_given(), 0.0, "none to the player");
    tick(&mut traffic, &recording, &world);
    assert_eq!(ship(&traffic, NpcId(2)).reserves.fuel.now, 100.5, "done");
    assert_eq!(traffic.take_on_line(), [], "silent");
    assert_eq!(ship(&traffic, HELPER).goal, Goal::Idle);
    assert_eq!(ship(&traffic, HELPER).target, None);
}
