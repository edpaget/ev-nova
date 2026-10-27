//! How the traffic flies a miner's goals: `_LowLevelAIHandler`'s orders
//! 0x13, 0x14 and 0x15 (@0x88140-0x88642 in the `EV Nova` executable),
//! which `_HighLevelAIHandler` gives states 16 and 17 (see
//! [`mine_orders`]). The autopilot runs them for [`Goal::Mine`]
//! ([`mine`]) and [`Goal::Scoop`] ([`scoop`]), after its immobile, AI
//! wait, escape pod and held-burst handling.
//!
//! - **Order 0x13, the chase** (@0x88140-0x881c4): it wants the bearing
//!   to asteroid slot 0 (`_CalcInterceptAngle`) and thrusts while facing
//!   it within its turn and [`THRUST_SLACK`], strictly.
//! - **Order 0x14, the hold** (@0x881ca-0x88443): its velocity eased and
//!   its position nudged by a step of 1.5 accelerations
//!   ([`mine_orders::hold`]), written straight into its state as the
//!   dock's damping and creep are
//!   ([`plunder_dock`](crate::traffic::plunder_dock)); then it wants the
//!   lead at slot 0 for the weapon it holds
//!   ([`mine_orders::kill_heading`]), with no thrust, and moves at the
//!   eased velocity. Its fire is the fight's
//!   ([`fire_untargeted`](crate::ai::fire_untargeted)).
//! - **Order 0x15, the scoop** (@0x8844b-0x88642): with a box to steer to
//!   ([`mine_orders::nearest_box`]) it wants its bearing and thrusts while
//!   facing it within its turn and [`THRUST_SLACK`], strictly; otherwise
//!   it brakes each axis toward none by 1.75 accelerations
//!   ([`mine_orders::brake`]). With no box it takes order 0 (@0x88636): no
//!   turn and no thrust, so it coasts.
//! - **Order 1**, slot 0 empty, is the autopilot's stop.
//!
//! Each order sets only the heading it wants (+0x68), and `_HandleShip`
//! turns the ship to it ([`flight::turn_to`]); order 0 wants its own whole
//! heading, the low level's default (@0x8522d). Every order writes the
//! burst word ([`Word::Writes`](crate::traffic::held_burst::Word)). The
//! acceleration is its `_ShipAccelRate`, the flying handling's, at a frame
//! scale of 1. None of these orders has an inertialess or disabled
//! branch: a disabled ship's goal is reset before it flies one, and an
//! inertialess ship's move follows its speed whatever velocity they write
//! ([`inertialess`](crate::traffic::inertialess)). No order plays a sound.
//!
//! Double time's second pass flies them again, where the original's runs
//! no low level (rdm:task/double-time-second-pass-low-level-writes).

#[cfg(doc)]
use crate::ai::Goal;
use crate::ai::mine_orders::{self, MineOrder, THRUST_SLACK};
use crate::combat::armament::Arsenal;
use crate::flight::{self, Controls};
use crate::geometry::Vec2;
use crate::handling::Handling;
use crate::traffic::held_burst::Flight;
use crate::traffic::npc::Npc;
use crate::trig;
use nova_sim_combat::asteroid::Asteroid;

/// `npc`'s flight in [`Goal::Mine`] with `rock` in slot 0, on `handling`,
/// its lead by the global records `weapons`: order 0x13 or 0x14 (see the
/// module docs); none for order 1, which the autopilot flies as its stop.
pub(crate) fn mine(
    npc: &mut Npc,
    rock: Option<&Asteroid>,
    handling: &Handling,
    weapons: &Arsenal,
) -> Option<Flight> {
    let rock = rock?;
    let controls = match mine_orders::order(npc, Some(rock)) {
        MineOrder::Stop => return None,
        MineOrder::Chase => {
            let wanted = trig::intercept_angle(npc.state.position, rock.position);
            steer(npc, wanted, handling, true)
        }
        MineOrder::Kill => {
            mine_orders::hold(&mut npc.state, rock, mine_orders::kill_step(handling.accel));
            let pick = npc.pick.and_then(|pick| weapons.weapon(pick));
            let wanted = mine_orders::kill_heading(&npc.state, rock, pick);
            steer(npc, wanted, handling, false)
        }
    };
    Some(controls.into())
}

/// `npc`'s flight in [`Goal::Scoop`], the box it steers to at
/// `scoop_box`, if any, on `handling`: order 0x15, or order 0 with no box
/// (see the module docs).
pub(crate) fn scoop(npc: &mut Npc, scoop_box: Option<Vec2>, handling: &Handling) -> Flight {
    let Some(at) = scoop_box else {
        // Order 0: the low level's own whole heading, no thrust.
        let wanted = trig::whole(npc.state.heading);
        return steer(npc, wanted, handling, false).into();
    };
    let wanted = trig::intercept_angle(npc.state.position, at);
    let controls = steer(npc, wanted, handling, true);
    if !controls.thrust {
        let step = mine_orders::scoop_brake_step(handling.accel);
        npc.state.velocity = mine_orders::brake(npc.state.velocity, step);
    }
    controls.into()
}

/// The controls of `npc` wanting `wanted` on `handling`, thrusting, when
/// `thrusts`, while it faces `wanted` within its turn and
/// [`THRUST_SLACK`], strictly, as it heads before its turn; it is turned
/// to `wanted` as `_HandleShip` turns it ([`flight::turn_to`]).
fn steer(npc: &mut Npc, wanted: i16, handling: &Handling, thrusts: bool) -> Controls {
    let thrust =
        thrusts && mine_orders::faces(npc.state.heading, wanted, handling.turn_rate, THRUST_SLACK);
    let turn = flight::turn_to(&mut npc.state, wanted, handling.turn_rate);
    Controls {
        turn,
        thrust,
        ..Controls::default()
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::ai::miner::MINES;
    use crate::ai::{Goal, NovaAi};
    use crate::cargo_box::CargoBox;
    use crate::flight::{FlightRules, ShipState};
    use crate::stats::ShipStats;
    use crate::traffic::autopilot::{self, Beside, Outcome, Readings, TopSpeeds};
    use crate::traffic::{Traffic, World};
    use nova_sim_base::fixture::{Draws, FAST};

    /// A live asteroid at (`x`, `y`) moving by (`vx`, `vy`).
    fn rock(x: f32, y: f32, vx: f32, vy: f32) -> Asteroid {
        Asteroid {
            position: Vec2::new(x, y),
            velocity: Vec2::new(vx, vy),
            frame: 0.0,
            spin: 0.0,
            strength: 30,
            kind: 0,
        }
    }

    /// A miner of [`FAST`] in `goal` at `state`, its class turn 2 (a mining
    /// box of 120).
    fn miner(goal: Goal, state: ShipState) -> Npc {
        let mut npc = crate::traffic::fixture::npc(1, ShipStats::new(FAST, &[]));
        npc.goal = goal;
        npc.miner = MINES;
        npc.hull.class_turn = 2.0;
        npc.state = state;
        npc
    }

    fn at(x: f32, y: f32, vx: f32, vy: f32, heading: f32) -> ShipState {
        ShipState {
            position: Vec2::new(x, y),
            velocity: Vec2::new(vx, vy),
            heading,
        }
    }

    /// `npc` flown a tick by the autopilot with `beside`.
    fn flown(npc: &mut Npc, beside: Beside) -> Outcome {
        flown_among(npc, beside, &Arsenal::EMPTY)
    }

    /// `npc` flown a tick by the autopilot with `beside` and the global
    /// weapon records `weapons`.
    fn flown_among(npc: &mut Npc, beside: Beside, weapons: &Arsenal) -> Outcome {
        let speeds = TopSpeeds::own_of(npc);
        autopilot::fly(
            npc,
            &[],
            &BTreeMap::new(),
            None,
            false,
            beside,
            speeds,
            FlightRules::default(),
            Readings::default(),
            weapons,
            &mut Draws::of(&[]),
        )
    }

    fn handling(npc: &Npc) -> Handling {
        npc.flying_handling(TopSpeeds::own_of(npc).own, FlightRules::default())
    }

    fn mining(slot0: Asteroid) -> Beside {
        Beside {
            rock: Some(slot0),
            ..Beside::default()
        }
    }

    #[test]
    fn the_chase_turns_to_slot_0_and_thrusts_only_within_its_turn_and_15() {
        // Slot 0 straight right: bearing 90.
        let slot0 = rock(1000.0, 0.0, 0.0, 0.0);
        let probe = miner(Goal::Mine, at(0.0, 0.0, 0.0, 0.0, 90.0));
        let turn = handling(&probe).turn_rate;
        let within = 90.0 - ((turn + THRUST_SLACK).ceil() - 1.0);
        let mut facing = miner(Goal::Mine, at(0.0, 0.0, 0.0, 0.0, within));
        assert_eq!(flown(&mut facing, mining(slot0)), Outcome::Flying);
        assert!(facing.state.velocity.x > 0.0, "{:?}", facing.state);
        assert_eq!(facing.state.heading, within + turn, "turned toward 90");
        let off = 90.0 - (turn + THRUST_SLACK).ceil();
        let mut away = miner(Goal::Mine, at(0.0, 0.0, 0.0, 0.0, off));
        flown(&mut away, mining(slot0));
        assert_eq!(away.state.velocity, Vec2::ZERO, "no thrust");
        assert_eq!(away.state.heading, off + turn);
    }

    #[test]
    fn the_hold_eases_the_velocity_nudges_into_the_band_and_moves() {
        let slot0 = rock(0.0, 0.0, 1.0, -0.5);
        let mut held = miner(Goal::Mine, at(50.0, 100.0, 3.0, -0.5, 0.0));
        let step = mine_orders::kill_step(handling(&held).accel);
        flown(&mut held, mining(slot0));
        // vx eased by a step; vy snapped; x pushed out of 80 by a step,
        // then the move by the eased velocity.
        let vx = 3.0 - step;
        assert_eq!(held.state.velocity, Vec2::new(vx, -0.5));
        assert_eq!(
            held.state.position,
            Vec2::new(50.0 + step + vx, 100.0 - 0.5)
        );
        let mut far = miner(Goal::Mine, at(120.0, -120.0, 1.0, -0.5, 0.0));
        let mut inside = far.clone();
        flown(&mut far, mining(slot0));
        assert_eq!(far.state.position, Vec2::new(121.0, -120.5), "in the band");
        inside.state.position = Vec2::new(119.0, 0.0);
        flown(&mut inside, mining(slot0));
        assert_eq!(inside.state.position, Vec2::new(120.0, -0.5), "on its line");
    }

    #[test]
    fn the_hold_turns_to_the_lead_and_never_thrusts() {
        let slot0 = rock(0.0, 0.0, 0.0, 0.0);
        let mut held = miner(Goal::Mine, at(100.0, 100.0, 0.0, 0.0, 300.0));
        flown(&mut held, mining(slot0));
        assert_eq!(held.state.velocity, Vec2::ZERO);
        let turn = handling(&held).turn_rate;
        assert_eq!(held.state.heading, 300.0 + turn, "toward 314");
    }

    #[test]
    fn the_hold_turns_to_the_lead_for_its_pick_not_the_bearing() {
        // Slot 0 crosses the line of fire by (3, −3); the miner at rest
        // at (100, 100) faces the bearing, 314. Eased to (0.27, −0.27),
        // its lead for gun 128 at 10 px a tick is 337.
        let slot0 = rock(0.0, 0.0, 3.0, -3.0);
        let gun = crate::combat::weapon::WeaponSpec::new(&crate::catalog::WeaponRecord {
            guidance: -1,
            speed: 1000,
            ..nova_sim_catalog::fixture::weapon(128)
        });
        let weapons = Arsenal::of_specs([gun]);
        let mut picking = miner(Goal::Mine, at(100.0, 100.0, 0.0, 0.0, 314.0));
        picking.pick = Some(crate::catalog::WeaponId(128));
        let mut bare = picking.clone();
        let turn = handling(&picking).turn_rate;
        flown_among(&mut picking, mining(slot0), &weapons);
        assert_eq!(picking.state.heading, 314.0 + turn, "toward 337");
        bare.pick = None;
        flown_among(&mut bare, mining(slot0), &weapons);
        assert_eq!(bare.state.heading, 314.0, "on the bearing with no pick");
    }

    #[test]
    fn with_slot_0_empty_a_miner_brakes_as_order_1() {
        let start = at(0.0, 0.0, 4.0, -3.0, 10.0);
        let mut mining_ship = miner(Goal::Mine, start);
        let mut idle = miner(Goal::Idle, start);
        for _ in 0..50 {
            flown(&mut mining_ship, Beside::default());
            flown(&mut idle, Beside::default());
            assert_eq!(mining_ship.state, idle.state);
        }
    }

    fn scooping(at_box: Option<Vec2>) -> Beside {
        Beside {
            scoop_box: at_box,
            ..Beside::default()
        }
    }

    #[test]
    fn the_scoop_steers_to_its_box_and_thrusts_facing_it() {
        let mut facing = miner(Goal::Scoop, at(0.0, 0.0, 0.0, 0.0, 90.0));
        flown(&mut facing, scooping(Some(Vec2::new(500.0, 0.0))));
        assert!(facing.state.velocity.x > 0.0, "{:?}", facing.state);
        assert_eq!(facing.state.heading, 90.0);
    }

    #[test]
    fn the_scoop_off_its_heading_brakes_both_axes_and_turns() {
        let mut off = miner(Goal::Scoop, at(0.0, 0.0, 2.0, -2.0, 250.0));
        let step = mine_orders::scoop_brake_step(handling(&off).accel);
        let turn = handling(&off).turn_rate;
        flown(&mut off, scooping(Some(Vec2::new(500.0, 0.0))));
        assert_eq!(off.state.velocity, Vec2::new(2.0 - step, -2.0 + step));
        assert_eq!(off.state.heading, 250.0 - turn, "the short way, left");
    }

    #[test]
    fn with_no_box_a_scooping_miner_coasts() {
        let mut coasting = miner(Goal::Scoop, at(0.0, 0.0, 2.0, -1.0, 45.5));
        flown(&mut coasting, scooping(None));
        assert_eq!(coasting.state.velocity, Vec2::new(2.0, -1.0));
        assert_eq!(coasting.state.position, Vec2::new(2.0, -1.0));
        assert_eq!(coasting.state.heading, 45.0, "its whole heading");
    }

    #[test]
    fn in_traffic_a_miner_closes_on_slot_0_then_holds_by_it_at_its_velocity() {
        let mut traffic = Traffic::with_interval(1);
        let mut zephyr = miner(Goal::Idle, at(-900.0, 700.0, 0.0, 0.0, 0.0));
        zephyr.goal = Goal::Idle;
        let id = traffic.add_npc(zephyr);
        let ai = NovaAi::default();
        let velocity = Vec2::new(0.3, -0.2);
        let mut slot0 = rock(400.0, 300.0, velocity.x, velocity.y);
        let mut closed = false;
        for _ in 0..3000 {
            let world = World {
                mining_rock: Some(slot0),
                ..World::new(&[])
            };
            traffic.tick_in(&ai, world, &[], &mut Draws::of(&[]));
            slot0.position = slot0.position + slot0.velocity;
            let ship = traffic
                .npcs()
                .iter()
                .find(|npc| npc.id == id)
                .expect("stays");
            assert_eq!(ship.goal, Goal::Mine);
            let off = ship.state.position - slot0.position;
            closed |= off.x.abs() <= 150.0 && off.y.abs() <= 150.0;
        }
        assert!(closed, "it closed in");
        let ship = traffic
            .npcs()
            .iter()
            .find(|npc| npc.id == id)
            .expect("stays");
        let off = ship.state.position - slot0.position;
        for axis in [off.x.abs(), off.y.abs()] {
            assert!((79.0..=151.0).contains(&axis), "{off:?}");
        }
        assert!(
            (ship.state.velocity - velocity).length() < 1e-5,
            "{:?}",
            ship.state
        );
    }

    #[test]
    fn in_traffic_a_miner_that_can_scoop_steers_to_the_nearest_scoopable_box() {
        let mut traffic = Traffic::with_interval(1);
        let mut scooper = miner(Goal::Idle, at(0.0, 0.0, 0.0, 0.0, 0.0));
        scooper.miner = crate::ai::miner::MINER_BITS;
        scooper.can_scoop = true;
        scooper.holds = 20;
        let id = traffic.add_npc(scooper);
        let cargo = |x: f32, y: f32, scoopable: bool| CargoBox {
            position: Vec2::new(x, y),
            velocity: Vec2::ZERO,
            life: 500.0,
            frame: 0,
            spin: 0,
            graphic: 1,
            scoopable,
            contents: Some(0),
        };
        let boxes = [
            cargo(0.0, -300.0, false),
            cargo(2000.0, 0.0, true),
            cargo(-600.0, 0.0, true),
        ];
        let ai = NovaAi::default();
        for _ in 0..200 {
            let world = World {
                boxes: &boxes,
                ..World::new(&[])
            };
            traffic.tick_in(&ai, world, &[], &mut Draws::of(&[]));
        }
        let ship = traffic
            .npcs()
            .iter()
            .find(|npc| npc.id == id)
            .expect("stays");
        assert_eq!(ship.goal, Goal::Scoop);
        assert!(ship.state.position.x < -50.0, "{:?}", ship.state);
        assert!(ship.state.position.y.abs() < 50.0, "{:?}", ship.state);
    }
}
