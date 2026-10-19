//! A stock ship refuelling the player (`_HighLevelAIHandler`'s goal 9
//! @0x8f3e5-0x8f6d4, flown through the docking's substate 0xb and substate
//! 1; see `nova_sim::traffic::refuel_dock`). Skips, passing, when
//! `NOVA_DATA` is unset.
//!
//! - A stock Federation Shuttle asked to refuel the player from far off
//!   closes on it (sub 0xb), stops (sub 1) and refuels it a unit a think
//!   inside its class's refuel box, and is done on the think after the
//!   player's fuel passes 100.
//! - A stock hull whose tank holds 100 (the Sigma Shipyards Shuttle,
//!   `shïp` 188) never passes 100 once its frame's clamp cuts the add
//!   back, so its refuel goes on for ever.

use crate::common::{opened, player, stock_npc};

use nova_sim::combat::ShipRef;
use nova_sim::combat::armament::Arsenal;
use nova_sim::geometry::Vec2;
use nova_sim::govt::Governments;
use nova_sim::hail::Help;
use nova_sim::reserves::Gauge;
use nova_sim::traffic::npc::Npc;
use nova_sim::traffic::on_line::OnLine;
use nova_sim::traffic::plunder_dock::DockSub;
use nova_sim::traffic::refuel_dock::refuel_box;
use nova_sim::traffic::{Traffic, World};
use nova_sim::{Condition, Goal, GovtId, NovaAi, PilotCatalog, Rulebook, ShipId};
use nova_sim_base::fixture::Draws;

const FEDERATION: GovtId = GovtId(128);
const SHUTTLE: ShipId = ShipId(128);
/// The Sigma Shipyards Alpha class Shuttle, whose `Fuel` is 100.
const SMALL_TANK: ShipId = ShipId(188);

/// What a run of the refuel saw, think by think.
#[derive(Debug, Default)]
struct Run {
    /// It closed through sub 0xb.
    closed: bool,
    /// It stopped through sub 1.
    stopped: bool,
    /// Each think's fuel given, with where the refueller was.
    given: Vec<(f32, Vec2)>,
    /// The think its note was queued on, and the fuel the think read.
    done: Option<(usize, f32)>,
}

/// A stock Federation Shuttle refuelling the player, holding `fuel` of
/// `max`, from `at`, for up to `thinks` thinks, the player's fuel clamped
/// to its tank before each, as its flight step does.
fn refuelled(fuel: f32, max: f32, at: Vec2, thinks: usize) -> Option<(Run, Npc, f32)> {
    let data = opened()?;
    let arsenal = Arsenal::read(&data);
    let govts = Governments::read(&data);
    let ai = NovaAi::from_rulebook(&Rulebook::default());
    let mut shuttle = stock_npc(&data, &arsenal, 1, (SHUTTLE, FEDERATION), at);
    shuttle.goal = Goal::Assist(Help::Refuel);
    shuttle.target = Some(ShipRef::Player);
    let mut traffic = Traffic::with_interval(1);
    let helper = traffic.add_npc(shuttle);
    let ship = |traffic: &Traffic| -> Npc {
        traffic
            .npcs()
            .iter()
            .find(|npc| npc.id == helper)
            .expect("here")
            .clone()
    };
    let mut tank = Gauge { now: fuel, max };
    let mut run = Run::default();
    for think in 0..thinks {
        tank.now = tank.now.min(tank.max);
        let read = tank.now;
        let mut side = player(Condition::Intact);
        side.reserves.fuel = tank;
        let world = World {
            govts: &govts,
            player: Some(side),
            ..World::new(&[])
        };
        let before = ship(&traffic).state.position;
        traffic.tick_in(&ai, world, &[], &mut Draws::of(&[0; 256]));
        let after = ship(&traffic);
        match after.docking.map(|docking| docking.sub) {
            Some(DockSub::Close) if before.length() > 200.0 => run.closed = true,
            Some(DockSub::Stop) => run.stopped = true,
            _ => {}
        }
        let given = traffic.take_fuel_given();
        if given > 0.0 {
            run.given.push((given, before));
        }
        tank.now += given;
        if traffic
            .take_on_line()
            .contains(&OnLine::RefuelDone(helper, SHUTTLE))
        {
            run.done = Some((think, read));
            return Some((run, ship(&traffic), tank.now));
        }
    }
    Some((run, ship(&traffic), tank.now))
}

#[test]
#[allow(clippy::float_cmp)]
fn a_stock_shuttle_refuelling_the_player_closes_stops_refuels_and_finishes_a_think_after_100() {
    let Some((run, helper, fuel)) = refuelled(90.0, 300.0, Vec2::new(0.0, 600.0), 3000) else {
        return;
    };
    assert!(run.closed, "it closed through sub 0xb");
    assert!(run.stopped, "it stopped through sub 1");
    let side = refuel_box(helper.hull.class_turn) as f32;
    assert!(side > 0.0, "{side}");
    assert_eq!(run.given.len(), 11, "90 to 101: {:?}", run.given);
    for &(given, from) in &run.given {
        assert_eq!(given, 1.0);
        assert!(
            from.x.abs() <= side && from.y.abs() <= side,
            "refuelled from {from:?}, within {side}"
        );
    }
    let (_, read) = run.done.expect("it finished");
    assert_eq!(read, 101.0, "the think after it passed 100");
    assert_eq!(fuel, 101.0);
    assert_eq!(helper.goal, Goal::Idle);
    assert_eq!((helper.target, helper.docking), (None, None));
}

#[test]
fn a_stock_hull_of_100_fuel_or_less_is_refuelled_for_ever() {
    let Some(data) = opened() else {
        return;
    };
    let tank = data
        .ships()
        .into_iter()
        .find(|record| record.id == SMALL_TANK)
        .expect("the ship type reads")
        .fields
        .fuel;
    assert_eq!(tank, 100, "shïp 188's Fuel");
    let Some((run, helper, _)) = refuelled(95.0, f32::from(tank), Vec2::new(0.0, 50.0), 2000)
    else {
        return;
    };
    assert_eq!(run.done, None, "never done");
    assert!(run.given.len() > 100, "{}", run.given.len());
    assert_eq!(helper.goal, Goal::Assist(Help::Refuel));
}
