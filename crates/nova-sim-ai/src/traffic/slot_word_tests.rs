//! The traffic's tests of the words each ship's low level leaves at its
//! slot and `_HandleShip` flies at its step (see
//! [`slot_word`](super::slot_word)): the thrust its slot read
//! (rdm:phase/combat-ai-fidelity-cross-roadmap/phase-17-left-out-of-phase-15-handle-ship),
//! and the heading, cap and windows read there, flown on both of double
//! time's passes; a ship with no word since its spawn; and the
//! inertialess adjustment's own read at the step.

#![allow(clippy::float_cmp)]

use std::collections::BTreeSet;

use crate::ai::fixture::{PIRATES, TRADERS, ship};
use crate::ai::{Behaviour, Goal, Surroundings};
use crate::chance::{Chance, NeverFires};
use crate::combat::ShipRef;
use crate::combat::impact::TractorLock;
use crate::flight::{FlightRules, ShipState};
use crate::geometry::Vec2;
use crate::handling::{Handling, ShipFields};
use crate::stats::ShipStats;
use crate::traffic::npc::{AiType, Mode, Npc, NpcId};
use crate::traffic::ship_display::GLOW_CRUISE;
use crate::traffic::warp_out::Warp;
use crate::traffic::{Traffic, World};
use nova_sim_base::fixture::FAST;

/// Every ship keeps its goal.
#[derive(Debug)]
struct Keeps;

impl Behaviour for Keeps {
    fn decide(&self, npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
        npc.goal
    }
}

fn of(traffic: &Traffic, id: NpcId) -> &Npc {
    (traffic.npcs().iter())
        .find(|npc| npc.id == id)
        .expect("there")
}

fn of_mut(traffic: &mut Traffic, id: NpcId) -> &mut Npc {
    (traffic.npcs_mut().iter_mut())
        .find(|npc| npc.id == id)
        .expect("there")
}

/// A trader 2000 up the screen in slot 5, and an attacker of `fields`
/// at the centre in slot 3, at rest facing it (sub 7, which thrusts
/// leaving +0x34 at 0), of `IonizeMax` 400 and `Deionize` 100 a tick,
/// charged `charge`: the attacker.
fn attacking_of(traffic: &mut Traffic, fields: ShipFields, charge: f32) -> NpcId {
    let target = traffic.add_npc_at(ship(0, TRADERS, AiType::WimpyTrader, 0.0, -2000.0), 5);
    let mut attacker = ship(0, PIRATES, AiType::Warship, 0.0, 0.0);
    attacker.stats = ShipStats::new(fields, &[]);
    attacker.goal = Goal::Attack(ShipRef::Npc(target));
    attacker.target = Some(ShipRef::Npc(target));
    attacker.stats.ion.class_capacity = 400.0;
    attacker.stats.ion.class_rate = 100.0;
    attacker.ion.charge = charge;
    traffic.add_npc_at(attacker, 3)
}

/// [`attacking_of`] `FAST`.
fn attacking(traffic: &mut Traffic, charge: f32) -> NpcId {
    attacking_of(traffic, FAST, charge)
}

/// `npc` as it flies charged `charge`, unheld.
fn charged(npc: &Npc, charge: f32) -> Handling {
    let mut ship = npc.clone();
    ship.ion.charge = charge;
    ship.tractor = None;
    ship.flying_handling(ship.stats.handling.max_speed, FlightRules::default())
}

fn speed(traffic: &Traffic, id: NpcId) -> f32 {
    -of(traffic, id).state.velocity.y
}

#[test]
fn an_ionised_npcs_thrust_is_the_accel_its_low_level_read_at_its_slot_before_the_shed() {
    // Sub 7 writes +0x30 = `_ShipAccelRate` at its slot (@0x8695e),
    // before `_HandleShip`'s ion block sheds 300 to 200
    // (@0x342b2-0x34421).
    let mut traffic = Traffic::with_interval(1);
    let attacker = attacking(&mut traffic, 300.0);
    let slot = charged(of(&traffic, attacker), 300.0).accel;
    assert_ne!(slot, charged(of(&traffic, attacker), 200.0).accel);
    traffic.tick_in(&Keeps, World::new(&[]), &[], &mut NeverFires);
    assert_eq!(of(&traffic, attacker).ion.charge, 200.0);
    assert_eq!(speed(&traffic, attacker), slot);
}

#[test]
fn on_double_times_second_pass_it_thrusts_by_the_advance_passs_slot_accel() {
    // The second pass runs no `_AIDispatch` (@0x4402a), so `_HandleShip`
    // thrusts by the advance pass's +0x30 after its own shed to 100.
    let mut traffic = Traffic::with_interval(1);
    let attacker = attacking(&mut traffic, 300.0);
    let slot = charged(of(&traffic, attacker), 300.0).accel;
    traffic.tick_in(&Keeps, World::new(&[]), &[], &mut NeverFires);
    traffic.move_in(World::new(&[]), &mut NeverFires);
    assert_eq!(of(&traffic, attacker).ion.charge, 100.0);
    assert_eq!(speed(&traffic, attacker), slot + slot);
}

#[test]
fn an_ion_hit_between_its_slot_and_its_step_doesnt_slow_its_thrust() {
    // `_HandleShot`'s `_IonizeShip` (@0x3612d) runs in the shot loop
    // (@0x44385), between the AI loop and the ship loop.
    let mut traffic = Traffic::with_interval(1);
    let attacker = attacking(&mut traffic, 0.0);
    let slot = charged(of(&traffic, attacker), 0.0).accel;
    traffic.think_holding(&Keeps, World::new(&[]), &[], &mut NeverFires);
    of_mut(&mut traffic, attacker).ion.charge = 400.0;
    traffic.fly_holding(World::new(&[]), &BTreeSet::new(), &mut NeverFires);
    assert_eq!(of(&traffic, attacker).ion.charge, 300.0);
    assert_eq!(speed(&traffic, attacker), slot);
}

#[test]
fn each_advance_pass_reads_it_afresh() {
    // The second tick's slot reads the charge the first tick's step
    // left: 200.
    let mut traffic = Traffic::with_interval(1);
    let attacker = attacking(&mut traffic, 300.0);
    let first = charged(of(&traffic, attacker), 300.0).accel;
    let second = charged(of(&traffic, attacker), 200.0).accel;
    traffic.tick_in(&Keeps, World::new(&[]), &[], &mut NeverFires);
    traffic.tick_in(&Keeps, World::new(&[]), &[], &mut NeverFires);
    let word = traffic.word_of(attacker).expect("kept");
    assert_eq!(word.flown().accel, second);
    assert_eq!(speed(&traffic, attacker), first + second);
}

/// `FAST` of accel `accel` and top speed `speed`.
fn fields(accel: i16, speed: i16) -> ShipFields {
    ShipFields {
        accel,
        speed,
        ..FAST
    }
}

/// The speed of an attacker of accel 900 and top speed 90, uncharged,
/// moving `speed` toward its target, before and after double time's
/// second pass, held from the advance pass's end (`_HandleBeams`
/// @0x4453d) by the player moving at its velocity, so the pull does
/// nothing; with its unheld and held handling.
fn second_pass_held(speed_at: f32) -> ([f32; 2], Handling, Handling) {
    let mut traffic = Traffic::with_interval(1);
    let attacker = attacking_of(&mut traffic, fields(900, 90), 0.0);
    of_mut(&mut traffic, attacker).state.velocity = Vec2::new(0.0, -speed_at);
    let unheld = charged(of(&traffic, attacker), 0.0);
    traffic.tick_in(&Keeps, World::new(&[]), &[], &mut NeverFires);
    let before = speed(&traffic, attacker);
    let held = {
        let npc = of_mut(&mut traffic, attacker);
        npc.tractor = Some(TractorLock::by(ShipRef::Player));
        npc.own_handling()
    };
    let mut player = crate::ai::fixture::player(0.0, 500.0);
    player.state = ShipState {
        velocity: of(&traffic, attacker).state.velocity,
        ..player.state
    };
    let world = World {
        player: Some(player),
        ..World::new(&[])
    };
    traffic.move_in(world, &mut NeverFires);
    assert!(of(&traffic, attacker).tractor.is_some(), "still held");
    let after = speed(&traffic, attacker);
    ([before, after], unheld, held)
}

#[test]
fn on_the_second_pass_a_thrust_that_leaves_0x34_at_0_caps_at_its_step_top_speed() {
    // Sub 7 leaves +0x34 at 0 (@0x86957), so the second pass's step
    // caps its thrust at its held `_ShipMaxSpeed` (@0x34aab), a third,
    // while the thrust is the advance slot's unheld +0x30. Short of the
    // held cap it gains the unheld accel, not the held third.
    let ([before, after], unheld, held) = second_pass_held(0.0);
    assert_eq!(before, unheld.accel);
    assert!(before < held.max_speed && held.accel < unheld.accel);
    assert_eq!(after, before + unheld.accel);
    // Past the held cap and short of the unheld one, it gains nothing.
    let ([before, after], unheld, held) = second_pass_held(0.2);
    assert!(held.max_speed < before && before < unheld.max_speed);
    assert_eq!(after, before);
}

#[test]
fn an_escape_pods_thrust_is_its_slot_accel() {
    // `_AIDispatch` writes a pod's +0x30 = `_ShipAccelRate`
    // (@0x8fb6d-0x8fb72) at its slot.
    let mut traffic = Traffic::with_interval(1);
    let mut pod = ship(0, TRADERS, AiType::WimpyTrader, 0.0, 0.0);
    pod.ship = crate::combat::hull::ESCAPE_POD_SHIP;
    pod.mode = Mode::Adrift { ticks_left: 10 };
    pod.stats.ion.class_capacity = 400.0;
    pod.stats.ion.class_rate = 100.0;
    pod.ion.charge = 300.0;
    let pod = traffic.add_npc_at(pod, 2);
    let slot = charged(of(&traffic, pod), 300.0).accel;
    traffic.tick_in(&Keeps, World::new(&[]), &[], &mut NeverFires);
    assert_eq!(of(&traffic, pod).ion.charge, 200.0);
    assert_eq!(speed(&traffic, pod), slot);
}

#[test]
fn the_glow_reads_a_rush_by_its_slot_thrust_against_its_step_accel() {
    // The rush test (@0x34f57) compares +0x30, read uncharged at the
    // slot, with `_ShipAccelRate` at the step, after an ion hit to 400
    // and the shed to 300: 0.3 of it, so the plain thrust is a rush.
    // A rush raises a glow of 24 by 1 (@0x34f85), where a plain thrust
    // leaves it (@0x34fa1).
    let mut traffic = Traffic::with_interval(1);
    let attacker = attacking(&mut traffic, 0.0);
    traffic.think_holding(&Keeps, World::new(&[]), &[], &mut NeverFires);
    let npc = of_mut(&mut traffic, attacker);
    npc.ion.charge = 400.0;
    npc.display.glow = GLOW_CRUISE;
    traffic.fly_holding(World::new(&[]), &BTreeSet::new(), &mut NeverFires);
    assert_eq!(of(&traffic, attacker).display.glow, GLOW_CRUISE + 1);
    let mut uncharged = Traffic::with_interval(1);
    let attacker = attacking(&mut uncharged, 0.0);
    uncharged.think_holding(&Keeps, World::new(&[]), &[], &mut NeverFires);
    of_mut(&mut uncharged, attacker).display.glow = GLOW_CRUISE;
    uncharged.fly_holding(World::new(&[]), &BTreeSet::new(), &mut NeverFires);
    assert_eq!(of(&uncharged, attacker).display.glow, GLOW_CRUISE, "plain");
}

// At the slot.

/// The ship the sniper at berth 3 snipes, and hunters at berths 1 and 5
/// attacking the sniper, which rests moving (0.2, 0): the hunters and the
/// sniper.
fn hunted_sniper(traffic: &mut Traffic) -> ([NpcId; 2], NpcId) {
    let target = traffic.add_npc_at(ship(0, TRADERS, AiType::WimpyTrader, 0.0, -2000.0), 7);
    let mut sniper = ship(0, PIRATES, AiType::Warship, 0.0, 0.0);
    sniper.goal = Goal::Snipe(ShipRef::Npc(target));
    sniper.target = Some(ShipRef::Npc(target));
    sniper.state.velocity = Vec2::new(0.2, 0.0);
    let sniper = traffic.add_npc_at(sniper, 3);
    let hunter = |traffic: &mut Traffic, berth: u32| {
        let mut hunter = ship(0, PIRATES, AiType::Warship, 0.0, 1500.0);
        hunter.goal = Goal::Attack(ShipRef::Npc(sniper));
        hunter.target = Some(ShipRef::Npc(sniper));
        traffic.add_npc_at(hunter, berth)
    };
    ([hunter(traffic, 1), hunter(traffic, 5)], sniper)
}

#[test]
fn a_higher_slots_think_reads_a_lower_slots_low_level_write_that_tick() {
    // Sub 0xe at rest damps its velocity × 0.95 at its slot
    // (@0x8709d-0x87334): a higher slot's low level reads it damped that
    // tick, a lower one's as the tick began.
    let mut traffic = Traffic::with_interval(1);
    let ([below, above], sniper) = hunted_sniper(&mut traffic);
    traffic.think_holding(&Keeps, World::new(&[]), &[], &mut NeverFires);
    let damped = Vec2::new(0.2 * crate::traffic::snipe::DAMPING, 0.0);
    assert_eq!(of(&traffic, sniper).state.velocity, damped, "at its slot");
    let read = |hunter: NpcId| traffic.slot_reads[&hunter].0.map(|state| state.velocity);
    assert_eq!(read(below), Some(Vec2::new(0.2, 0.0)), "as the tick began");
    assert_eq!(read(above), Some(damped), "damped");
}

/// Records each ship's view of NPC `watched` as it decides: its goal and
/// whether its AI wait runs.
#[derive(Debug)]
struct Watch {
    watched: NpcId,
    seen: std::cell::RefCell<Vec<(NpcId, Goal, bool)>>,
}

impl Behaviour for Watch {
    fn decide(&self, npc: &Npc, around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
        if let Some(watched) = around.npcs.iter().find(|ship| ship.id == self.watched) {
            let seen = (npc.id, watched.goal, watched.ai_wait > 0);
            self.seen.borrow_mut().push(seen);
        }
        npc.goal
    }
}

#[test]
fn a_ship_landing_at_its_slot_is_landed_for_a_higher_slots_think_and_reported_at_its_step() {
    // State 1's landing (@0x8dadc-0x8db5a) is the high level's, at the
    // lander's slot: a higher slot's think reads it landed, a lower one's
    // as it was. Its step reports it, and flies it nothing.
    let sites = [nova_sim_catalog::fixture::planet(140, 0.0, 0.0)];
    let world = World::new(&sites);
    let mut traffic = Traffic::with_interval(1);
    let mut lander = ship(0, TRADERS, AiType::WimpyTrader, 0.0, 0.0);
    lander.goal = Goal::Land(crate::catalog::StellarId(140));
    let lander = traffic.add_npc_at(lander, 3);
    let below = traffic.add_npc_at(ship(0, TRADERS, AiType::WimpyTrader, 500.0, 0.0), 1);
    let above = traffic.add_npc_at(ship(0, TRADERS, AiType::WimpyTrader, -500.0, 0.0), 5);
    let watch = Watch {
        watched: lander,
        seen: std::cell::RefCell::default(),
    };
    traffic.think_holding(&watch, world, &[], &mut NeverFires);
    let seen = watch.seen.into_inner();
    let land = Goal::Land(crate::catalog::StellarId(140));
    assert!(seen.contains(&(below, land, false)), "{seen:?}");
    assert!(seen.contains(&(above, Goal::Idle, true)), "{seen:?}");
    let at = of(&traffic, lander).state;
    let mut pass = traffic.begin_fly(&world, &BTreeSet::new(), crate::traffic::FlyHalf::Advance);
    let outcome = traffic.fly_npc(&mut pass, lander, &world, &mut NeverFires);
    assert_eq!(
        outcome,
        Some(crate::traffic::autopilot::Outcome::Landed(
            crate::catalog::StellarId(140)
        ))
    );
    assert_eq!(of(&traffic, lander).state, at, "no flight that pass");
}

#[test]
fn under_its_ai_wait_its_low_level_leaves_the_heads_word() {
    // The head zeroes +0x30 and sets +0x68 to its own whole heading
    // (@0x851f1, @0x8522d); `_HandleShip` skips the turn and thrust under
    // the wait (@0x34a54-0x34a72).
    let mut traffic = Traffic::with_interval(1);
    let attacker = attacking(&mut traffic, 0.0);
    of_mut(&mut traffic, attacker).ai_wait = 5;
    of_mut(&mut traffic, attacker).state.heading = 33.6;
    traffic.think_holding(&Keeps, World::new(&[]), &[], &mut NeverFires);
    let word = traffic.word_of(attacker).expect("kept");
    assert_eq!(word.wanted(), Some(33));
    assert!(!word.thrusts());
}

// Double time's second pass flies the advance pass's word.

/// An attacker at the centre heading `heading`, its target 2000 up the
/// screen (bearing 0): sub 7, whose window is four turns, 12 for `FAST`.
fn attacking_from(traffic: &mut Traffic, heading: f32) -> NpcId {
    let attacker = attacking(traffic, 0.0);
    of_mut(traffic, attacker).state.heading = heading;
    attacker
}

#[test]
fn on_double_times_second_pass_it_turns_toward_the_advance_passs_heading_and_thrusts_only_by_its_word()
 {
    // The second pass runs no `_AIDispatch` (`je 0x44350` @0x4402a): it
    // turns toward the advance pass's +0x68 (@0x34460-0x3458e) and thrusts
    // only if its +0x30 isn't 0 (@0x34a7b-0x34a84).
    let world = World::new(&[]);
    // Out of its window at its slot, 13 off: turned into it, 10 off, by
    // the advance pass, it thrusts nothing on the second.
    let mut traffic = Traffic::with_interval(1);
    let out = attacking_from(&mut traffic, 13.0);
    traffic.tick_in(&Keeps, world, &[], &mut NeverFires);
    assert_eq!(of(&traffic, out).state.heading, 10.0);
    traffic.move_in(world, &mut NeverFires);
    let ship = of(&traffic, out);
    assert_eq!(ship.state.velocity, Vec2::ZERO, "no thrust");
    assert_eq!(ship.state.heading, 7.0, "it turns again");
    // In its window at its slot, 11 off: the second pass thrusts again,
    // though a fresh window, its target moved round behind it, would close.
    let mut traffic = Traffic::with_interval(1);
    let inside = attacking_from(&mut traffic, 11.0);
    traffic.tick_in(&Keeps, world, &[], &mut NeverFires);
    let advanced = of(&traffic, inside).state;
    assert_ne!(advanced.velocity, Vec2::ZERO, "it thrust");
    let target = traffic.npcs()[0].id;
    of_mut(&mut traffic, target).state.position = Vec2::new(0.0, 2000.0);
    traffic.move_in(world, &mut NeverFires);
    let ship = of(&traffic, inside);
    assert!(
        ship.state.velocity.length() > advanced.velocity.length(),
        "again"
    );
    assert_eq!(ship.state.heading, 5.0, "toward 0, not its target's 180");
}

#[test]
fn the_second_pass_reads_no_ship_its_low_level_would() {
    // `_HandleShip` reads no other ship: a quarry moved after the advance
    // pass doesn't change the heading the second pass wants.
    let world = World::new(&[]);
    let mut moved = Traffic::with_interval(1);
    let attacker = attacking_from(&mut moved, 30.0);
    moved.tick_in(&Keeps, world, &[], &mut NeverFires);
    let mut kept = moved.clone();
    let target = moved.npcs()[0].id;
    of_mut(&mut moved, target).state.position = Vec2::new(2000.0, 0.0);
    moved.move_in(world, &mut NeverFires);
    kept.move_in(world, &mut NeverFires);
    assert_eq!(of(&moved, attacker).state, of(&kept, attacker).state);
}

/// A lander of top speed 6 and thrust 0.9 at the centre heading up,
/// moving `speed` up toward planet 140 300 above it, within 500 (sub 2's
/// quarter of its slot's top speed, @0x8579e): its speed before and after
/// double time's second pass, held from the advance pass's end
/// (`_HandleBeams` @0x4453d) by the player moving at its velocity, so the
/// pull does nothing; with its held top speed.
fn landing_held(speed_at: f32) -> ([f32; 2], f32) {
    let sites = [nova_sim_catalog::fixture::planet(140, 0.0, -300.0)];
    let mut traffic = Traffic::with_interval(1);
    let mut lander = ship(0, TRADERS, AiType::WimpyTrader, 0.0, 0.0);
    lander.stats = ShipStats::new(fields(900, 600), &[]);
    lander.goal = Goal::Land(crate::catalog::StellarId(140));
    lander.state.velocity = Vec2::new(0.0, -speed_at);
    let lander = traffic.add_npc_at(lander, 3);
    traffic.tick_in(&Keeps, World::new(&sites), &[], &mut NeverFires);
    let before = speed(&traffic, lander);
    let held = {
        let npc = of_mut(&mut traffic, lander);
        npc.tractor = Some(TractorLock::by(ShipRef::Player));
        npc.own_handling().max_speed
    };
    let mut player = crate::ai::fixture::player(0.0, 500.0);
    player.state = ShipState {
        velocity: of(&traffic, lander).state.velocity,
        ..player.state
    };
    let world = World {
        player: Some(player),
        ..World::new(&sites)
    };
    traffic.move_in(world, &mut NeverFires);
    assert!(of(&traffic, lander).tractor.is_some(), "still held");
    ([before, speed(&traffic, lander)], held)
}

#[test]
fn on_the_second_pass_a_slot_cap_stays_the_advance_slots_top_speed() {
    // Sub 2 within 500 writes +0x34 = 0.25 × `_ShipMaxSpeed` at its slot
    // (@0x85783-0x8579e): the second pass caps by it, not by its held
    // `_ShipMaxSpeed` at its step, a third of 6. Moving 1.6, past the
    // slot's 1.5 and short of the held 2, it gains nothing.
    let ([before, after], held) = landing_held(1.6);
    assert!(1.5 < before && before < held, "{before} {held}");
    assert_eq!(after, before);
    // Below the slot's quarter, it gains its thrust.
    let accel = ShipStats::new(fields(900, 600), &[]).handling.accel;
    let ([before, after], _) = landing_held(0.3);
    assert!(before < 1.5, "{before}");
    assert!((after - (before + accel)).abs() < 1e-5, "{before} {after}");
}

#[test]
fn a_burst_settled_on_the_advance_pass_thrusts_nothing_on_the_second() {
    // `_AIMakeShipStopJumping` (@0x82eb3-0x82eb6) zeroes +0x30 and +0x34
    // as the burst ends: the second pass, with no low level, thrusts
    // nothing. A wait's sub 1 brake bursts an owed burst out (@0x8551e;
    // goal 0's sub 0 writes no +0x30); a ship with a lead flies on after
    // it (@0x34d5c).
    let mut traffic = Traffic::with_interval(1);
    let lead = traffic.add_npc_at(ship(0, TRADERS, AiType::WimpyTrader, 0.0, 3000.0), 1);
    let mut braking = ship(0, TRADERS, AiType::WimpyTrader, 0.0, 0.0);
    braking.goal = Goal::Wait;
    braking.leader = Some(lead);
    braking.state.velocity = Vec2::new(0.0, 3.0);
    braking.state.heading = 0.0;
    let top = braking.stats.handling.max_speed;
    let accel = braking.stats.handling.accel;
    braking.owed_burst = Some(top + accel / 2.0);
    let braking = traffic.add_npc_at(braking, 2);
    traffic.fly_tick(&World::new(&[]), &mut NeverFires);
    let ended = of(&traffic, braking).clone();
    assert_eq!((ended.owed_burst, ended.mode), (None, Mode::Flying));
    let word = traffic.word_of(braking).expect("kept");
    assert!(!word.thrusts(), "+0x30 = 0");
    traffic.move_in(World::new(&[]), &mut NeverFires);
    assert_eq!(of(&traffic, braking).state.velocity, ended.state.velocity);
}

#[test]
fn the_warp_outs_heading_write_is_the_berths_last() {
    // `_HandleShip`'s warp-out writes +0x68 (@0x347ba): its aim is the
    // heading the berth's next ship turns toward.
    let mut traffic = Traffic::with_interval(1);
    let mut runner = ship(0, TRADERS, AiType::WimpyTrader, 2000.0, 0.0);
    runner.goal = Goal::JumpOut;
    runner.begin_own_jump();
    runner.warp.aim = 123;
    runner.state.heading = 100.0;
    runner.warp.stage = 1.0;
    let runner = traffic.add_npc_at(runner, 6);
    traffic.fly_holding(World::new(&[]), &BTreeSet::new(), &mut NeverFires);
    assert!(traffic.npcs().iter().any(|npc| npc.id == runner));
    assert_eq!(traffic.berth_headings.left(6), Some(123));
}

/// A ship jumping out at rest 2000 east, beyond the leave's reach, heading
/// 100, of `warp` (its aim and stage), in berth 6, not yet on its own
/// jump: after its slot, where its low level begins it (sub 4,
/// [`Leg::Jump`](crate::ai::leave::Leg::Jump)), and after its step on the
/// advance pass, which reports it.
fn jump_begun_at_slot(warp: Warp) -> (Traffic, NpcId, Npc) {
    let mut traffic = Traffic::with_interval(1);
    let mut runner = ship(0, TRADERS, AiType::WimpyTrader, 2000.0, 0.0);
    runner.goal = Goal::JumpOut;
    runner.warp = warp;
    runner.state.heading = 100.0;
    let runner = traffic.add_npc_at(runner, 6);
    traffic.think_holding(&Keeps, World::new(&[]), &[], &mut NeverFires);
    let at_slot = of(&traffic, runner).clone();
    assert!(at_slot.jumping(), "its jump begun at its slot");
    let world = World::new(&[]);
    let mut pass = traffic.begin_fly(&world, &BTreeSet::new(), crate::traffic::FlyHalf::Advance);
    let outcome = traffic.fly_npc(&mut pass, runner, &world, &mut NeverFires);
    assert_eq!(outcome, Some(crate::traffic::autopilot::Outcome::Flying));
    (traffic, runner, at_slot)
}

#[test]
fn a_jump_begun_at_its_slot_flies_nothing_at_its_step() {
    // Sub 4 begun at the slot (state 2 @0x8dd99) moves the ship nothing
    // that tick: its warp-out flies it from the next pass, so its step
    // neither turns it toward its warp's aim nor writes that aim as its
    // berth's last heading (@0x347ba).
    let stale = Warp {
        aim: 123,
        ..Warp::default()
    };
    let (traffic, runner, at_slot) = jump_begun_at_slot(stale);
    assert_eq!(of(&traffic, runner).state, at_slot.state);
    assert_eq!(traffic.berth_headings.left(6), Some(100));
}

#[test]
fn a_flight_ended_at_its_slot_still_settles_its_warp_under_way_at_its_step() {
    // `_HandleShip`'s warp-out block (@0x347ba on) runs at every step, a
    // flight's that ended at its slot too: a stage under way, on its aim,
    // moves the ship out along its heading as its jump begun at its slot
    // flies nothing else.
    let under_way = Warp {
        aim: 100,
        stage: 3.0,
        since: 200,
        goal: None,
        joining: false,
    };
    let (traffic, runner, at_slot) = jump_begun_at_slot(under_way);
    let stepped = of(&traffic, runner);
    assert_eq!(stepped.state.heading, at_slot.state.heading);
    assert_ne!(stepped.state.position, at_slot.state.position, "moved out");
}

// A ship flown before its first low level.

/// A traffic whose berth 4's last ship wanted 90, its word kept from an
/// attack on a ship 2000 east, and gone since.
fn berth_4_wanting_90() -> Traffic {
    let mut traffic = Traffic::with_interval(1);
    let target = traffic.add_npc_at(ship(0, TRADERS, AiType::WimpyTrader, 2000.0, 0.0), 6);
    let mut last = ship(0, PIRATES, AiType::Warship, 0.0, 0.0);
    last.goal = Goal::Attack(ShipRef::Npc(target));
    last.target = Some(ShipRef::Npc(target));
    let last = traffic.add_npc_at(last, 4);
    traffic.think_holding(&Keeps, World::new(&[]), &[], &mut NeverFires);
    assert_eq!(
        traffic.word_of(last).and_then(|word| word.wanted()),
        Some(90)
    );
    traffic.remove(last);
    traffic
}

/// A ship spawned into berth 4 heading 10.5, attacking the ship there
/// was, which a steered flight would thrust for: its state after a fly
/// half with no think and after double time's second pass.
fn spawned_in_berth_4(traffic: &mut Traffic) -> [ShipState; 2] {
    let target = traffic.npcs()[0].id;
    let mut fresh = ship(0, PIRATES, AiType::Warship, 0.0, 0.0);
    fresh.goal = Goal::Attack(ShipRef::Npc(target));
    fresh.state.heading = 10.5;
    let fresh = traffic.add_npc_at(fresh, 4);
    assert_eq!(traffic.word_of(fresh), None, "no word since its spawn");
    traffic.fly_holding(World::new(&[]), &BTreeSet::new(), &mut NeverFires);
    let first = of(traffic, fresh).state;
    traffic.move_in(World::new(&[]), &mut NeverFires);
    [first, of(traffic, fresh).state]
}

#[test]
fn a_ship_with_no_low_level_since_its_spawn_thrusts_nothing_and_turns_toward_its_berths_last_heading()
 {
    // Every spawn zeroes +0x30 and +0x34 (`_GenericRandomShipSpawn`
    // @0x3cb54-0x3cb5a, `_RandomShipSpawn` @0x3c639-0x3c643,
    // `_RespawnEscort` @0x3d4ee-0x3d4f1) and writes no +0x68.
    let mut traffic = berth_4_wanting_90();
    let turn = FlightRules::default()
        .handling(ShipStats::new(FAST, &[]).handling)
        .turn_rate;
    let [first, second] = spawned_in_berth_4(&mut traffic);
    assert_eq!(first.velocity, Vec2::ZERO, "no thrust");
    assert_eq!(first.heading, 10.5 + turn, "a turn toward 90");
    assert_eq!(second.velocity, Vec2::ZERO);
    assert_eq!(second.heading, 10.5 + 2.0 * turn, "and another");
}

#[test]
fn by_the_other_reading_it_keeps_its_own_whole_heading() {
    let mut traffic = berth_4_wanting_90();
    traffic.set_reused_slot_heading(crate::rulebook::RuleSource::Bible);
    assert_eq!(
        traffic.reused_slot_heading(),
        crate::rulebook::RuleSource::Bible
    );
    let [first, second] = spawned_in_berth_4(&mut traffic);
    assert_eq!(first.velocity, Vec2::ZERO);
    assert_eq!((first.heading, second.heading), (10.0, 10.0));
}

#[test]
fn a_fighter_launched_below_its_carriers_slot_flies_the_spawn_word() {
    // Launched into a berth the AI loop has passed (`_LaunchFighter`
    // through `_GenericRandomShipSpawn` @0x3d657), it has had no low level
    // when the ship loop flies it: a berth no ship has held turns it
    // toward 0, as `_InitObjects` left it (@0x1c315).
    let mut traffic = Traffic::with_interval(1);
    let _carrier = attacking(&mut traffic, 0.0);
    traffic.think_holding(&Keeps, World::new(&[]), &[], &mut NeverFires);
    let mut fighter = ship(0, PIRATES, AiType::Warship, 100.0, 0.0);
    fighter.state.heading = 20.0;
    fighter.state.velocity = Vec2::new(1.0, 0.0);
    fighter.goal = Goal::Attack(ShipRef::Player);
    let fighter = traffic.add_npc_at(fighter, 1);
    traffic.fly_holding(World::new(&[]), &BTreeSet::new(), &mut NeverFires);
    let turn = FlightRules::default()
        .handling(ShipStats::new(FAST, &[]).handling)
        .turn_rate;
    let flown = of(&traffic, fighter).state;
    assert_eq!(flown.velocity, Vec2::new(1.0, 0.0), "no thrust");
    assert_eq!(flown.heading, 20.0 - turn, "toward 0");
    assert_eq!(flown.position, Vec2::new(101.0, 0.0));
}

// What double time's second pass doesn't run (rdm phase
// play-feel-9/phase-13-double-time-second-pass-runs-no-low-level).

#[test]
fn on_the_second_pass_a_miners_0x14_ease_and_nudge_and_0x15_brake_dont_run() {
    use crate::cargo_box::CargoBox;
    use nova_sim_combat::asteroid::Asteroid;
    // Order 0x14 eases and nudges by asteroid slot 0 (@0x881ca-0x883b1)
    // and 0x15 brakes off its box (@0x8858b-0x8862d), each at the slot: on
    // the second pass the ship only flies its word.
    let rock = Asteroid {
        position: Vec2::new(50.0, 0.0),
        velocity: Vec2::ZERO,
        frame: 0.0,
        spin: 0.0,
        strength: 30,
        kind: 0,
    };
    let boxes = [CargoBox {
        position: Vec2::new(2000.0, 0.0),
        velocity: Vec2::ZERO,
        life: 100.0,
        frame: 0,
        spin: 0,
        graphic: 1,
        scoopable: true,
        contents: Some(0),
    }];
    for goal in [Goal::Mine, Goal::Scoop] {
        let mut traffic = Traffic::with_interval(1);
        let mut miner = ship(0, TRADERS, AiType::WimpyTrader, 0.0, 0.0);
        miner.goal = goal;
        miner.miner = crate::ai::miner::MINES;
        miner.hull.class_turn = 2.0;
        miner.state.velocity = Vec2::new(0.0, 1.0);
        let miner = traffic.add_npc_at(miner, 2);
        let world = World {
            mining_rock: Some(rock),
            boxes: &boxes,
            ..World::new(&[])
        };
        traffic.think_holding(&Keeps, world, &[], &mut NeverFires);
        let at_slot = of(&traffic, miner).state;
        assert_ne!(
            at_slot.velocity,
            Vec2::new(0.0, 1.0),
            "{goal:?}: at its slot"
        );
        traffic.fly_holding(world, &BTreeSet::new(), &mut NeverFires);
        let advanced = of(&traffic, miner).state;
        traffic.move_in(world, &mut NeverFires);
        let again = of(&traffic, miner).state;
        assert_eq!(
            again.velocity, advanced.velocity,
            "{goal:?}: no brake or ease"
        );
        assert_eq!(
            again.position,
            advanced.position + advanced.velocity,
            "{goal:?}: no nudge"
        );
    }
}

/// The player's escort (berth 2) in close formation beside the player at
/// the centre, glowing 2, the player glowing 9.
fn close_escort() -> (Traffic, World<'static>, NpcId) {
    let mut traffic = Traffic::with_interval(1);
    let mut escort = ship(0, TRADERS, AiType::WimpyTrader, 10.0, 0.0);
    escort.escort = Some(crate::escort::EscortDuty {
        formation: crate::escort::FormationSlot::default(),
        order: None,
    });
    escort.goal = Goal::Formation { guard: None };
    escort.display.glow = 2;
    let escort = traffic.add_npc_at(escort, 2);
    let mut world = World {
        player: Some(crate::ai::fixture::player(0.0, 0.0)),
        ..World::new(&[])
    };
    world.warp.player.glow = 9;
    (traffic, world, escort)
}

#[test]
fn on_the_second_pass_a_close_formation_escort_takes_no_glow_copy() {
    // Sub 0xc's glow copy (@0x87dce-0x87deb) is its low level's.
    let (mut traffic, world, escort) = close_escort();
    traffic.tick_in(&Keeps, world, &[], &mut NeverFires);
    let copied = of(&traffic, escort).display.glow;
    assert_eq!(copied, 8, "the player's 9, faded a step");
    traffic.move_in(world, &mut NeverFires);
    assert_eq!(
        of(&traffic, escort).display.glow,
        copied - 1,
        "faded, no copy"
    );
}

#[test]
fn on_the_second_pass_a_docking_fighter_is_neither_pulled_nor_docked_and_thrusts_only_by_its_word()
{
    // Sub 8's pull (@0x87ff9) and dock (@0x87f0b-0x87fef) run at its slot;
    // the second pass flies its word (@0x87e74-0x880b8). Facing its
    // carrier, its word bursts it at its pull, which ends within its cap
    // and zeroes +0x30 (@0x82eb3-0x82eb6): the second pass thrusts
    // nothing.
    let mut traffic = Traffic::with_interval(1);
    let carrier = traffic.add_npc_at(ship(0, TRADERS, AiType::WimpyTrader, 0.0, -50.0), 1);
    let mut fighter = ship(0, PIRATES, AiType::Warship, 0.0, 0.0);
    fighter.goal = Goal::Dock(ShipRef::Npc(carrier));
    fighter.carrier = Some(crate::bay::Carrier {
        ship: ShipRef::Npc(carrier),
        window: 100.0,
        reach: 10.0,
    });
    fighter.state.heading = 0.0;
    let fighter = traffic.add_npc_at(fighter, 3);
    let world = World::new(&[]);
    traffic.think_holding(&Keeps, world, &[], &mut NeverFires);
    let pulled = of(&traffic, fighter).state.position;
    assert_ne!(pulled, Vec2::ZERO, "pulled at its slot");
    traffic.fly_holding(world, &BTreeSet::new(), &mut NeverFires);
    let advanced = of(&traffic, fighter).state;
    traffic.move_in(world, &mut NeverFires);
    let again = of(&traffic, fighter).state;
    assert!(
        traffic.npcs().iter().any(|npc| npc.id == fighter),
        "not docked"
    );
    assert_eq!(again.velocity, advanced.velocity);
    assert_eq!(
        again.position,
        advanced.position + again.velocity,
        "unpulled"
    );
}

// The inertialess adjustment's own read.

#[test]
fn the_inertialess_adjustment_reads_its_accel_at_the_step_before_the_shed() {
    // `_AdjustInertialessShipVelocity` reads `_ShipAccelRate` at the step
    // (@0x334e1), before its ion block (@0x342b2-0x34421): a hit to 200
    // after the slot pulls by the 200-charged accel × 4, its speed moving
    // by the slot's +0x30, and the step sheds 200 to 100.
    use crate::inertialess::{ADJUST_RATE, adjust_velocity};
    let mut traffic = Traffic::with_interval(1);
    let attacker = attacking(&mut traffic, 0.0);
    {
        let npc = of_mut(&mut traffic, attacker);
        npc.stats.inertialess_hull = true;
        npc.state.velocity = Vec2::new(1.0, 0.0);
    }
    traffic.think_holding(&Keeps, World::new(&[]), &[], &mut NeverFires);
    let slot = charged(of(&traffic, attacker), 0.0).accel;
    let hit = charged(of(&traffic, attacker), 200.0).accel;
    let shed = charged(of(&traffic, attacker), 100.0).accel;
    assert!(hit < shed && shed < slot);
    of_mut(&mut traffic, attacker).ion.charge = 200.0;
    traffic.fly_holding(World::new(&[]), &BTreeSet::new(), &mut NeverFires);
    let npc = of(&traffic, attacker);
    assert_eq!(npc.ion.charge, 100.0);
    assert_eq!(npc.speed, slot, "the slot's +0x30");
    let pulled = adjust_velocity(Vec2::new(1.0, 0.0), 0.0, slot, hit * ADJUST_RATE);
    assert_eq!(npc.state.velocity, pulled);
}
