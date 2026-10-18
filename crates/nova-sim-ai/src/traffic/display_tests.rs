//! Tests of the NPCs' drawn state as the traffic runs it (see
//! [`ship_display`](crate::traffic::ship_display)): the spawns' writes,
//! the advance pass's fire clock and fold writers, the bank and glow after
//! each flight, and the display after the fight.
#![allow(clippy::float_cmp)]

use std::collections::BTreeMap;

use super::{Traffic, World};
use crate::ai::{Behaviour, Goal, Surroundings};
use crate::catalog::{GovtId, ShipId};
use crate::chance::Chance;
use crate::combat::armament::NEVER_FIRED;
use crate::combat::ship_frames::{ANIMATED, BANKS, FOLDS, FrameSpec, REFOLD_TICKS};
use crate::flight::ShipState;
use crate::geometry::Vec2;
use crate::rulebook::RuleSource;
use crate::stats::ShipStats;
use crate::traffic::fixture::npc;
use crate::traffic::npc::{AiType, Npc};
use crate::traffic::ship_display::{GLOW_SPAWN, Rolls};
use crate::traffic::spawn::{Entry, NewShip};
use crate::traffic::table::ShipKind;
use nova_sim_base::fixture::{Draws, FAST};

/// Keeps every NPC's goal.
#[derive(Debug)]
struct Keep;

impl Behaviour for Keep {
    fn decide(&self, npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
        npc.goal
    }
}

/// A class of `shän` `Flags` `flags` and six base sets, `AnimDelay` 5.
fn frames(flags: u16) -> FrameSpec {
    FrameSpec {
        flags,
        base_sets: 6,
        anim_delay: 5,
        ..FrameSpec::NONE
    }
}

/// Traffic deciding every tick, holding `ship`.
fn holding(ship: Npc) -> Traffic {
    let mut traffic = Traffic::with_interval(1);
    traffic.npcs = vec![ship];
    traffic
}

/// An idle NPC at rest at the centre, of class `flags`.
fn idle(flags: u16) -> Npc {
    let mut ship = npc(1, ShipStats::new(FAST, &[]));
    ship.hull.frames = frames(flags);
    ship
}

#[test]
fn a_spawn_glows_folds_and_owes_its_rolls() {
    let mut traffic = Traffic::with_interval(1);
    traffic.table.ships = BTreeMap::from([(
        ShipId(200),
        ShipKind {
            stats: ShipStats::new(FAST, &[]),
            hull: crate::combat::hull::HullSpec {
                frames: frames(FOLDS),
                ..crate::combat::hull::HullSpec::default()
            },
            ..ShipKind::default()
        },
    )]);
    let ship = |entry, booty| NewShip {
        ship: ShipId(200),
        govt: Some(GovtId(130)),
        ai_type: AiType::WimpyTrader,
        lead: None,
        state: ShipState::default(),
        entry,
        aggression: 0,
        booty,
        info_types: 0,
        person: None,
        mission: None,
        aux: None,
        derelict: false,
        held: false,
        cloaked: false,
        quote: None,
        cargo: None,
        boarded: false,
    };
    let came = traffic.bring_in_within(
        vec![ship(Entry::Placed, None), ship(Entry::JumpingIn, Some(0))],
        0,
    );
    assert_eq!(came, 2);
    let [fleet, jumping] = traffic.npcs() else {
        panic!("two ships");
    };
    assert_eq!(fleet.display.glow, GLOW_SPAWN);
    let fold = fleet.armament.fold();
    assert_eq!((fold.set, fold.dir, fold.since_fire), (5, 0, NEVER_FIRED));
    assert_eq!(
        fleet.display.rolls,
        Rolls {
            anim: true,
            jump_in: false,
            derelict_glow: true
        }
    );
    let fold = jumping.armament.fold();
    assert_eq!((fold.set, fold.dir), (0, 1), "jumping in");
    assert!(jumping.display.rolls.jump_in);
    assert!(!jumping.display.rolls.anim, "a düde's own ship rolls none");
}

#[test]
fn the_advance_pass_moves_the_fire_clock_and_folds_a_flying_folder() {
    let mut ship = idle(FOLDS);
    ship.armament.fold_mut().since_fire = 0;
    let mut traffic = holding(ship);
    traffic.move_in(World::new(&[]), &mut Draws::of(&[]));
    let fold = traffic.npcs()[0].armament.fold();
    assert_eq!(
        (fold.since_fire, fold.dir),
        (0, 0),
        "not on the second pass"
    );
    traffic.tick_in(&Keep, World::new(&[]), &[], &mut Draws::of(&[]));
    let fold = traffic.npcs()[0].armament.fold();
    assert_eq!(
        (fold.since_fire, fold.dir),
        (2, 1),
        "two Mac ticks; folding"
    );
    // A miner gliding in waits for its refold.
    let mut miner = idle(FOLDS | 0x0080);
    miner.mode = crate::traffic::npc::Mode::JumpingIn { tick: 0 };
    miner.armament.fold_mut().since_fire = REFOLD_TICKS - 4;
    let mut traffic = holding(miner);
    traffic.tick_in(&Keep, World::new(&[]), &[], &mut Draws::of(&[]));
    assert_eq!(traffic.npcs()[0].armament.fold().dir, 0, "43 Mac ticks");
    traffic.tick_in(&Keep, World::new(&[]), &[], &mut Draws::of(&[]));
    assert_eq!(traffic.npcs()[0].armament.fold().dir, 1, "45");
}

#[test]
fn each_flight_steps_the_glow_and_the_wait_holds_the_bank() {
    let mut ship = idle(BANKS);
    ship.display.glow = 10;
    ship.display.frames.bank = 6.0;
    let mut traffic = holding(ship);
    traffic.move_in(World::new(&[]), &mut Draws::of(&[]));
    let shown = traffic.npcs()[0].display;
    assert_eq!(shown.glow, 9, "braking at rest: coasting");
    assert_eq!(shown.frames.bank, 5.0, "levelling");
    traffic.npcs[0].ai_wait = 10;
    traffic.move_in(World::new(&[]), &mut Draws::of(&[]));
    let shown = traffic.npcs()[0].display;
    assert_eq!(
        (shown.glow, shown.frames.bank),
        (8, 5.0),
        "held: the bank stays"
    );
}

#[test]
fn a_ship_jumping_out_glows_up_as_it_warps() {
    let mut ship = idle(0);
    ship.state.position = Vec2::new(0.0, -2000.0);
    ship.state.heading = 0.0;
    ship.begin_own_jump();
    ship.warp.stage = 3.0;
    ship.warp.since = 100_000;
    ship.warp.aim = 0;
    let mut traffic = holding(ship);
    let mut world = World::new(&[]);
    world.warp.durations = [364, 364];
    traffic.move_in(world, &mut Draws::of(&[]));
    assert_eq!(traffic.npcs()[0].display.glow, 2, "3 up, a coast down");
}

#[test]
fn the_display_steps_every_ship_after_the_fight() {
    let mut animated = idle(ANIMATED);
    animated.display.rolls.anim = true;
    animated.govt = Some(GovtId(140));
    animated.display.rolls.derelict_glow = true;
    animated.display.glow = 32;
    let mut traffic = holding(animated);
    let mut draws = Draws::of(&[3, 2]);
    traffic.display_ships(
        |govt| govt == Some(GovtId(140)),
        RuleSource::Engine,
        &mut draws,
    );
    let shown = traffic.npcs()[0].display;
    assert_eq!(draws.asked, [6, 5], "its rolls");
    assert_eq!(shown.drawn.set, 3);
    assert_eq!(shown.glow, 0, "a derelict government's");
}
