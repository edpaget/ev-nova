//! Tests that under its AI wait an NPC's low level flies the sub-goal
//! (+0xc8ca) it kept: `_AIDispatch` tail-jumps to `_LowLevelAIHandler`
//! (@0x901f4) under the wait, and the high level's wait gate
//! (@0x8d4b4-0x8d4d2) writes no sub but for goals 0xa and 0xe (@0x8d4c8,
//! @0x8d4ce). The kept sub's direct writes show at the slot, and its word
//! (+0x68, +0x30) stands for the step, which skips the turn and thrust
//! while +0x4c runs (@0x34a54-0x34a72) but flies it on double time's
//! second pass once the advance pass's countdown ends the wait
//! (@0x35037-0x35048).
#![allow(clippy::float_cmp)]

use super::autopilot::{self, Beside, Outcome, Readings, TopSpeeds};
use super::npc::{Npc, NpcId};
use super::slot_word::SlotWord;
use crate::ai::Goal;
use crate::ai::fire::Manoeuvre;
use crate::ai::mine_orders::{self, MineOrder};
use crate::ai::miner::MINES;
use crate::ai::pursuit::{Body, Frame, Pursuit};
use crate::cargo_box::CargoBox;
use crate::chance::NeverFires;
use crate::combat::ShipRef;
use crate::escort::{EscortDuty, FormationSlot};
use crate::flight::{FlightRules, ShipState};
use crate::geometry::Vec2;
use crate::handling::{Handling, ShipFields};
use crate::speed_mult::SpeedMult;
use crate::stats::{INERTIALESS_HULL, ShipStats};
use crate::traffic::snipe;
use crate::traffic::{Traffic, World};
use crate::trig;
use nova_sim_base::fixture::{Draws, FAST};
use nova_sim_combat::asteroid::Asteroid;
use std::collections::BTreeMap;

/// [`FAST`] with an inertialess hull.
const RAVEN: ShipFields = ShipFields {
    flags2: INERTIALESS_HULL,
    ..FAST
};

const AT_PLAYER: Goal = Goal::Attack(ShipRef::Player);

/// A ship of `fields` on `goal` at (10, 20) moving at `velocity` facing
/// `heading`, 2.0 along it, its AI wait 10 ticks from its end.
fn waiting(fields: ShipFields, goal: Goal, velocity: Vec2, heading: f32) -> Npc {
    Npc {
        state: ShipState {
            position: Vec2::new(10.0, 20.0),
            velocity,
            heading,
        },
        goal,
        speed: 2.0,
        ai_wait: 10.0,
        ..super::fixture::npc(1, ShipStats::new(fields, &[]))
    }
}

/// `npc`'s low level at its slot beside `other`, with `beside`, at full
/// gain under the original's rules: the word it leaves.
fn low(npc: &mut Npc, other: Option<&ShipState>, beside: Beside) -> SlotWord {
    low_by(npc, (other, false), beside)
}

/// `npc`'s low level as [`low`] flies it, beside `other`, disabled or
/// not.
fn low_by(
    npc: &mut Npc,
    (other, disabled): (Option<&ShipState>, bool),
    beside: Beside,
) -> SlotWord {
    let speeds = TopSpeeds::own_of(npc);
    let weapons = crate::ai::fixture::records_of([&*npc]);
    let rules = (FlightRules::default(), Readings::default(), &weapons);
    let (word, outcome) = autopilot::low_level(
        npc,
        &[],
        &BTreeMap::new(),
        (other, disabled, beside),
        speeds,
        rules,
        &mut Draws::of(&[]),
    );
    assert_eq!(outcome, None);
    word
}

/// `npc`'s handling at its slot.
fn handling(npc: &Npc) -> Handling {
    npc.flying_handling(TopSpeeds::own_of(npc).own, FlightRules::default())
}

/// The player's ship's state at (`x`, `y`), at rest.
fn player_at(x: f32, y: f32) -> ShipState {
    ShipState {
        position: Vec2::new(x, y),
        ..ShipState::default()
    }
}

/// `npc` with its AI wait over.
fn unwaited(npc: &Npc) -> Npc {
    Npc {
        ai_wait: 0.0,
        ..npc.clone()
    }
}

// Sub 1.

/// A pirate turned out of its count wait onto goal 4 with sub 1 stored,
/// moving up at 1 (its retro heading 180), facing `heading`.
fn turned_pirate(heading: f32) -> Npc {
    Npc {
        stopping: true,
        ai_wait: 20.0,
        ..waiting(FAST, AT_PLAYER, Vec2::new(0.0, -1.0), heading)
    }
}

#[test]
fn under_its_ai_wait_a_stop_kept_leaves_its_retro_word() {
    let mut pirate = turned_pirate(178.0);
    // Its twin flying the stop with no wait: goal 7's state 6, sub 1.
    let mut stopping = Npc {
        goal: Goal::Wait,
        ..unwaited(&pirate)
    };
    let player = player_at(0.0, -500.0);
    let word = low(&mut pirate, Some(&player), Beside::default());
    let stopped = low(&mut stopping, None, Beside::default());
    assert_eq!(
        word.wanted(),
        Some(180),
        "the retro heading (@0x85408-0x855e1)"
    );
    assert!(word.thrusts(), "the brake's +0x30");
    assert_eq!(word, stopped, "sub 1's own word");
    assert_eq!(pirate.state, stopping.state, "cut × 0.94 as ever");
    assert_eq!(pirate.speed, stopping.speed);
    assert!(pirate.stopping, "sub 1 stands");
}

#[test]
fn under_its_ai_wait_the_step_skips_the_kept_stops_turn_and_thrust() {
    let mut pirate = turned_pirate(178.0);
    let start = pirate.state;
    let speeds = TopSpeeds::own_of(&pirate);
    let weapons = crate::ai::fixture::records_of([&pirate]);
    let player = player_at(0.0, -500.0);
    let outcome = autopilot::fly(
        &mut pirate,
        &[],
        &BTreeMap::new(),
        Some(&player),
        false,
        Beside::default(),
        speeds,
        FlightRules::default(),
        Readings::default(),
        &weapons,
        &mut Draws::of(&[]),
    );
    assert_eq!(outcome, Outcome::Flying);
    assert_eq!(pirate.state.heading, start.heading, "no turn");
    let cut = crate::flight::drag_by(start.velocity, 0.94);
    assert_eq!(pirate.state.velocity, cut, "no thrust after the cut");
    assert_eq!(pirate.state.position, start.position + cut);
}

#[test]
fn an_owed_burst_is_not_spent_while_the_wait_skips_the_stops_thrust() {
    // `_HandleShip`'s burst block (@0x34c63) comes past the wait's skip
    // (@0x34a54).
    let mut pirate = Npc {
        owed_burst: Some(5.0),
        ..turned_pirate(178.0)
    };
    let speeds = TopSpeeds::own_of(&pirate);
    let weapons = crate::ai::fixture::records_of([&pirate]);
    autopilot::fly(
        &mut pirate,
        &[],
        &BTreeMap::new(),
        None,
        false,
        Beside::default(),
        speeds,
        FlightRules::default(),
        Readings::default(),
        &weapons,
        &mut Draws::of(&[]),
    );
    assert_eq!(pirate.owed_burst, Some(5.0));
}

#[test]
fn double_time_second_pass_flies_the_stop_kept_once_the_wait_ran_out() {
    // Off its retro heading by 30: no thrust, a turn to 180.
    let half = SpeedMult::new(0.5);
    let pirate = Npc {
        ai_wait: 0.5,
        ..turned_pirate(150.0)
    };
    let turn = handling(&pirate).turn_rate;
    let mut traffic = Traffic::with_interval(1);
    let id = traffic.add_npc(pirate);
    let world = World {
        speed_mult: half,
        ..World::new(&[])
    };
    traffic.fly_tick(&world, &mut Draws::of(&[]));
    let ship = of(&traffic, id);
    assert_eq!(ship.state.heading, 150.0, "the advance pass skips the turn");
    assert!(
        !crate::traffic::ai_wait::running(ship.ai_wait),
        "its wait out"
    );
    traffic.move_in(world, &mut Draws::of(&[]));
    let ship = of(&traffic, id);
    assert_eq!(
        ship.state.heading,
        150.0 + turn * 0.5,
        "turned toward 180 by its turn × the gain"
    );
}

// Subs 0x13, 0x14 and 0x15.

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

/// A miner whose think under its wait wrote goal 0 (`_MinerAI`
/// @0x8b45d), holding `order`, at (50, 100) moving (3, −0.5), its class
/// turn 2.
fn miner(order: MineOrder) -> Npc {
    let mut npc = waiting(FAST, Goal::Idle, Vec2::new(3.0, -0.5), 0.0);
    npc.state.position = Vec2::new(50.0, 100.0);
    npc.miner = MINES;
    npc.hull.class_turn = 2.0;
    npc.mine_order = order;
    npc
}

#[test]
fn under_its_ai_wait_a_miner_holding_order_0x14_eases_and_nudges_by_slot_0() {
    let slot0 = rock(0.0, 0.0, 1.0, -0.5);
    let mut held = miner(MineOrder::Kill);
    let mut expected = held.state;
    let step = mine_orders::kill_step(handling(&held).accel, SpeedMult::FULL);
    mine_orders::hold(&mut expected, &slot0, step);
    let beside = Beside {
        rock: Some(slot0),
        ..Beside::default()
    };
    let word = low(&mut held, None, beside);
    assert_eq!(held.state, expected, "eased and nudged (@0x881d8-0x883b1)");
    assert_eq!(
        word.wanted(),
        Some(mine_orders::kill_heading(&held.state, &slot0, None))
    );
    assert!(!word.thrusts());
    assert_eq!(held.mine_order, MineOrder::Kill);
}

#[test]
fn under_its_ai_wait_a_miner_holding_order_0x13_wants_slot_0s_bearing() {
    let slot0 = rock(1000.0, 100.0, 0.0, 0.0);
    let mut chasing = miner(MineOrder::Chase);
    let start = chasing.state;
    let beside = Beside {
        rock: Some(slot0),
        ..Beside::default()
    };
    let word = low(&mut chasing, None, beside);
    assert_eq!(
        word.wanted(),
        Some(trig::intercept_angle(start.position, slot0.position))
    );
    assert_eq!(chasing.state, start, "no direct write");
}

#[test]
fn under_its_ai_wait_a_miner_holding_order_0x14_with_slot_0_empty_leaves_the_heads_word() {
    // The original reads slot 0's stale record (no activity test
    // @0x881ca-0x881e0), which the engine doesn't keep.
    let mut held = miner(MineOrder::Kill);
    held.state.heading = 33.6;
    let start = held.state;
    let word = low(&mut held, None, Beside::default());
    assert_eq!(word.wanted(), Some(33));
    assert!(!word.thrusts());
    assert_eq!(held.state, start);
}

#[test]
fn under_its_ai_wait_a_scooper_holding_order_0x15_brakes_off_its_box() {
    // Its box behind it: off its bearing, so the brake (@0x8858b-0x88634).
    let at = Vec2::new(50.0, 300.0);
    let mut scooping = miner(MineOrder::Scoop);
    let start = scooping.state;
    let step = mine_orders::scoop_brake_step(handling(&scooping).accel, SpeedMult::FULL);
    let beside = Beside {
        scoop_box: Some(at),
        ..Beside::default()
    };
    let word = low(&mut scooping, None, beside);
    assert_eq!(
        scooping.state.velocity,
        mine_orders::brake(start.velocity, step)
    );
    assert_eq!(
        word.wanted(),
        Some(trig::intercept_angle(start.position, at))
    );
    assert!(!word.thrusts());
    assert_eq!(scooping.mine_order, MineOrder::Scoop);
}

#[test]
fn under_its_ai_wait_a_scooper_holding_order_0x15_with_no_box_stores_sub_0() {
    // @0x88636: order 0, the head's word, and sub 0 stored.
    let mut scooping = miner(MineOrder::Scoop);
    scooping.state.heading = 33.6;
    let start = scooping.state;
    let word = low(&mut scooping, None, Beside::default());
    assert_eq!(word.wanted(), Some(33));
    assert!(!word.thrusts());
    assert_eq!(scooping.state, start);
    assert_eq!(scooping.mine_order, MineOrder::Stop);
}

// The fight subs, 0xe included.

/// A pursuit whose frame for `goal` ends in `manoeuvre` wanting
/// `heading`, as a think's `waiting_pursuit::under_wait` stores it.
fn pursuing(goal: Goal, manoeuvre: Manoeuvre, heading: i16) -> Pursuit {
    Pursuit {
        kept: None,
        frame: Some(Frame {
            goal,
            bodies: [Some(Body { manoeuvre, heading }), None],
            speed_cap: None,
        }),
    }
}

#[test]
fn under_its_ai_wait_a_sniper_at_rest_damps_by_0_95() {
    let goal = Goal::Snipe(ShipRef::Player);
    let mut sniper = waiting(FAST, goal, Vec2::new(0.2, 0.0), 10.0);
    sniper.pursuit = pursuing(goal, Manoeuvre::Snipe, 0);
    let start = sniper.state;
    let player = player_at(400.0, 20.0);
    let word = low(&mut sniper, Some(&player), Beside::default());
    assert_eq!(
        sniper.state.velocity,
        start.velocity * snipe::DAMPING,
        "@0x8725a-0x8728e"
    );
    assert_eq!(
        word.wanted(),
        Some(trig::intercept_angle(start.position, player.position)),
        "the plain bearing (@0x872aa)"
    );
}

#[test]
fn under_its_ai_wait_an_inertialess_sniper_moving_leaves_its_brake_word() {
    let goal = Goal::Snipe(ShipRef::Player);
    let mut sniper = waiting(RAVEN, goal, Vec2::new(2.0, 0.0), 10.4);
    sniper.pursuit = pursuing(goal, Manoeuvre::Snipe, 0);
    let player = player_at(400.0, 20.0);
    let word = low(&mut sniper, Some(&player), Beside::default());
    assert!(
        word.thrusts(),
        "+0x30 = −`_ShipAccelRate` (@0x87121-0x87137)"
    );
    assert_eq!(word.wanted(), Some(10));
}

#[test]
fn under_its_ai_wait_a_fight_body_leaves_its_word() {
    let mut fighter = waiting(FAST, AT_PLAYER, Vec2::ZERO, 85.0);
    fighter.pursuit = pursuing(AT_PLAYER, Manoeuvre::Dogfight, 90);
    let player = player_at(400.0, 20.0);
    let word = low(&mut fighter, Some(&player), Beside::default());
    assert_eq!(word.wanted(), Some(90));
    assert!(word.thrusts(), "within a turn and the dogfight's slack");
}

#[test]
fn under_its_ai_wait_a_fight_sub_with_its_quarry_gone_leaves_the_heads_word() {
    let mut fighter = waiting(FAST, AT_PLAYER, Vec2::ZERO, 85.0);
    fighter.pursuit = pursuing(AT_PLAYER, Manoeuvre::Dogfight, 90);
    let word = low(&mut fighter, None, Beside::default());
    assert_eq!(word.wanted(), Some(85));
    assert!(!word.thrusts());
}

#[test]
fn under_its_ai_wait_a_fight_sub_at_a_disabled_quarry_leaves_the_heads_word() {
    // +0x70 not disabled (@0x870ab-0x870b9, @0x8641e-0x8642c).
    let goal = Goal::Snipe(ShipRef::Player);
    let mut sniper = waiting(FAST, goal, Vec2::new(0.2, 0.0), 10.4);
    sniper.pursuit = pursuing(goal, Manoeuvre::Snipe, 0);
    let start = sniper.state;
    let player = player_at(400.0, 20.0);
    let word = low_by(&mut sniper, (Some(&player), true), Beside::default());
    assert_eq!(word.wanted(), Some(10));
    assert_eq!(sniper.state, start, "no damping");
}

// Sub 0, and the exempt goals.

#[test]
fn under_its_ai_wait_a_ship_with_sub_0_kept_leaves_the_heads_word() {
    // A landed ship's stay and a launched fighter's glide (goal 0), a ship
    // a pirate took into its fleet (goal 0, AI type 6), and a goal a think
    // wrote under the wait over sub 0.
    for goal in [Goal::Idle, AT_PLAYER, Goal::Wait, Goal::Mine] {
        let mut ship = waiting(FAST, goal, Vec2::new(1.0, 0.5), 33.6);
        let start = ship.state;
        let player = player_at(400.0, 20.0);
        let word = low(&mut ship, Some(&player), Beside::default());
        assert_eq!(word.wanted(), Some(33), "{goal:?}");
        assert!(!word.thrusts(), "{goal:?}");
        assert_eq!(ship.state, start, "{goal:?}");
    }
}

/// An escort in slot 2 of 3, spaced 30 apart, keeping formation on the
/// player, its AI wait running, 20 off its slot.
fn waiting_escort(leader: &ShipState) -> Npc {
    let slot = crate::escort::slot_position(leader, 2, 3, 30.0);
    Npc {
        escort: Some(EscortDuty {
            formation: FormationSlot {
                slot: 2,
                ships: 3,
                spacing: 30.0,
            },
            order: None,
        }),
        ..waiting(
            FAST,
            Goal::Formation { guard: None },
            leader.velocity,
            leader.heading,
        )
    }
    .at(slot + Vec2::new(20.0, 0.0))
}

trait At {
    fn at(self, position: Vec2) -> Self;
}

impl At for Npc {
    fn at(mut self, position: Vec2) -> Self {
        self.state.position = position;
        self
    }
}

#[test]
fn a_formation_ship_under_its_ai_wait_flies_its_formation() {
    // Goal 0xa passes the wait gate (@0x8d4c8): its high level runs, and
    // its low level's nudge and glow copy show.
    let leader = ShipState {
        position: Vec2::new(0.0, 0.0),
        velocity: Vec2::new(1.0, -2.0),
        heading: 90.0,
    };
    let mut escort = waiting_escort(&leader);
    let mut unwaited = unwaited(&escort);
    let start = escort.state;
    let word = low(&mut escort, Some(&leader), Beside::default());
    let flown = low(&mut unwaited, Some(&leader), Beside::default());
    assert_ne!(escort.state, start, "nudged");
    assert_eq!(escort.state, unwaited.state);
    assert_eq!(escort.display, unwaited.display, "its glow copied");
    assert_eq!(word, flown);
}

// The dispatch.

fn of(traffic: &Traffic, id: NpcId) -> &Npc {
    (traffic.npcs().iter())
        .find(|npc| npc.id == id)
        .expect("there")
}

#[test]
fn under_its_ai_wait_the_dispatch_keeps_close_form_and_the_mine_order() {
    // The high level writes no sub under the wait (@0x8d4b4-0x8d4d2), so
    // `_IsShipInCloseForm` (@0x82f54) and the cloak think
    // (@0x834d6, @0x835e9-0x835fb) read the kept one.
    let mut traffic = Traffic::with_interval(1);
    let mut ship = waiting(FAST, AT_PLAYER, Vec2::ZERO, 0.0);
    ship.close_form = true;
    ship.mine_order = MineOrder::Kill;
    let id = traffic.add_npc(ship);
    traffic.low_levels(&World::new(&[]), &mut NeverFires);
    let ship = of(&traffic, id);
    assert!(ship.close_form);
    assert_eq!(ship.mine_order, MineOrder::Kill);
    // With its wait over, the dispatch writes them again.
    traffic.npcs_mut()[0].ai_wait = 0.0;
    traffic.low_levels(&World::new(&[]), &mut NeverFires);
    let ship = of(&traffic, id);
    assert!(!ship.close_form);
    assert_eq!(ship.mine_order, MineOrder::Stop);
}

#[test]
fn a_scooping_ship_stores_order_0x15_with_a_box_and_sub_0_without() {
    // Goal 0x11 writes 0x15 (@0x8faf3-0x8fafd), and its low level sub 0
    // with no box to steer to (@0x88636).
    let boxes = [CargoBox {
        position: Vec2::new(300.0, 0.0),
        velocity: Vec2::ZERO,
        life: 100.0,
        frame: 0.0,
        spin: 0,
        graphic: 1,
        scoopable: true,
        contents: Some(0),
    }];
    let stored = |boxes: &[CargoBox]| {
        let mut traffic = Traffic::with_interval(1);
        let mut ship = waiting(FAST, Goal::Scoop, Vec2::ZERO, 0.0);
        ship.ai_wait = 0.0;
        let id = traffic.add_npc(ship);
        let world = World {
            boxes,
            ..World::new(&[])
        };
        traffic.low_levels(&world, &mut NeverFires);
        of(&traffic, id).mine_order
    };
    assert_eq!(stored(&boxes), MineOrder::Scoop);
    assert_eq!(stored(&[]), MineOrder::Stop);
}

#[test]
fn the_cloak_think_reads_order_0x15_as_a_stop() {
    // It tests 0x13 and 0x14 alone (@0x834d6, @0x835e9-0x835fb).
    assert_eq!(
        crate::cloak::MineOrder::from(MineOrder::Scoop),
        crate::cloak::MineOrder::Stop
    );
}

#[test]
fn under_its_ai_wait_a_fight_sub_reads_the_target_it_holds_at_its_slot() {
    // `waiting_pursuit::under_wait` aims at the goal's quarry or else the
    // target +0x70 holds; the slot reads the same ship for the body's word.
    let mut traffic = Traffic::with_interval(1);
    let mut fighter = waiting(FAST, Goal::Idle, Vec2::ZERO, 85.0);
    fighter.target = Some(ShipRef::Player);
    fighter.pursuit = pursuing(AT_PLAYER, Manoeuvre::Dogfight, 90);
    let id = traffic.add_npc(fighter);
    let world = World {
        player: Some(crate::ai::fixture::player(400.0, 20.0)),
        ..World::new(&[])
    };
    traffic.low_levels(&world, &mut NeverFires);
    let left = traffic.word_of(id).expect("kept");
    assert_eq!(left.wanted(), Some(90));
    assert!(left.thrusts());
}

#[test]
fn an_exempt_goal_under_its_ai_wait_drops_a_stored_sub_1() {
    // Goals 0xa and 0xe write their own sub under the wait (@0x8d4c8,
    // @0x8d4ce, goal 0xe's sub 0 @0x8eb91), so a sub 1 stored before is
    // gone and its cuts don't run.
    for goal in [Goal::Hold, Goal::Drift] {
        let mut ship = Npc {
            stopping: true,
            ..waiting(FAST, goal, Vec2::new(0.0, -1.0), 178.0)
        };
        let start = ship.state;
        let word = low(&mut ship, None, Beside::default());
        assert!(!ship.stopping, "{goal:?}");
        assert_eq!(ship.state, start, "{goal:?}: uncut");
        assert_eq!(word.wanted(), Some(178), "{goal:?}");
    }
}

#[test]
fn a_formation_ship_under_its_ai_wait_has_its_store_written_and_reads_its_leader() {
    // Goal 0xa's high level runs under the wait (@0x8d4c8): the dispatch
    // writes its store, and its slot reads its leader, not a fight sub's
    // quarry.
    let mut traffic = Traffic::with_interval(1);
    let leader = player_at(0.0, 0.0);
    let mut escort = waiting_escort(&leader);
    escort.mine_order = MineOrder::Kill;
    escort.pursuit = pursuing(AT_PLAYER, Manoeuvre::Dogfight, 90);
    let id = traffic.add_npc(escort);
    let world = World {
        player: Some(crate::ai::fixture::player(0.0, 0.0)),
        ..World::new(&[])
    };
    let (read, ..) = traffic.reads_at_slot(0, &world);
    assert_eq!(read.map(|state| state.position), Some(leader.position));
    traffic.low_levels(&world, &mut NeverFires);
    assert_eq!(of(&traffic, id).mine_order, MineOrder::Stop);
}

#[test]
fn a_returning_fighter_under_its_ai_wait_with_no_fight_sub_reads_its_carrier() {
    let mut traffic = Traffic::with_interval(1);
    traffic.add_npc(waiting(FAST, Goal::Dock(ShipRef::Player), Vec2::ZERO, 0.0));
    let world = World {
        player: Some(crate::ai::fixture::player(300.0, 20.0)),
        ..World::new(&[])
    };
    let (read, ..) = traffic.reads_at_slot(0, &world);
    assert_eq!(
        read.map(|state| state.position),
        Some(Vec2::new(300.0, 20.0))
    );
}
