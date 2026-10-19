//! The stock ship types' afterburners and break-offs, and a stock attacker
//! flown through its rush and its joust (see `nova_sim::ai::pursuit` and
//! `nova_sim::ai::afterburner`). Skips, passing, when `NOVA_DATA` is unset.
//!
//! - Of the 288 stock ship types, 67 always have an afterburner (`shïp`
//!   `Flags` 0x0040), 197 more have one by the player's combat rating
//!   (0x0020), and none is planet-type (0x0400).
//! - 133 may break off a joust: an `InherentAI` above 2 and a `Mass` of
//!   199 or less.
//! - The yardstick a rating is measured by is `shïp` 128's (the
//!   Shuttle's) `Strength`, 2.
//! - A Dragon (159, `Flags` 0x0140) has an afterburner: approaching the
//!   player it rushes in from the next tick, past its top speed; meeting
//!   another Dragon head on inside 122 it breaks off.

use crate::common;

use std::collections::BTreeMap;

use nova_data::GameData;
use nova_sim::ai::fire::Manoeuvre;
use nova_sim::ai::pursuit::{Kept, RUSH_THRUST};
use nova_sim::ai::{Behaviour, Goal, PlayerSide, Surroundings, afterburner};
use nova_sim::chance::Chance;
use nova_sim::combat::ShipRef;
use nova_sim::combat::armament::Arsenal;
use nova_sim::combat::hull::{AFTERBURNER_ALWAYS, AFTERBURNER_BY_RATING};
use nova_sim::flight::ShipState;
use nova_sim::geometry::Vec2;
use nova_sim::traffic::npc::{Npc, NpcId};
use nova_sim::traffic::table::{ShipKind, kind};
use nova_sim::traffic::{Traffic, World};
use nova_sim::{PilotCatalog, ShipId};
use nova_sim_base::fixture::Ranged;

/// The Dragon: `Flags` 0x0140, `InherentAI` 3, 100 tons, no stand-off.
const DRAGON: ShipId = ShipId(159);

fn opened() -> Option<GameData> {
    let dir = common::nova_data()?;
    Some(GameData::open(&dir, None).expect("the stock data opens"))
}

/// Every stock ship type as the traffic spawns it.
fn kinds(data: &GameData) -> Vec<(ShipId, ShipKind)> {
    let outfits = data.outfits();
    let arsenal = Arsenal::read(data);
    data.ships()
        .iter()
        .map(|record| (record.id, kind(record, &outfits, &arsenal)))
        .collect()
}

#[test]
fn the_stock_ship_types_afterburners_and_break_offs() {
    let Some(data) = opened() else {
        return;
    };
    let kinds = kinds(&data);
    assert_eq!(kinds.len(), 288);
    let count =
        |test: &dyn Fn(&ShipKind) -> bool| kinds.iter().filter(|(_, kind)| test(kind)).count();
    assert_eq!(
        count(&|kind| !kind.hull.planet_type && kind.hull.afterburner & AFTERBURNER_ALWAYS != 0),
        67
    );
    assert_eq!(
        count(&|kind| kind.hull.afterburner == AFTERBURNER_BY_RATING),
        197
    );
    assert_eq!(count(&|kind| kind.hull.planet_type), 0);
    assert_eq!(
        count(&|kind| kind.hull.inherent_ai > 2 && kind.hull.mass <= 199.0),
        133
    );
    assert_eq!(Arsenal::read(&data).strength(ShipId(128)), 2);
}

#[test]
fn a_stock_class_of_flags_0x0020_has_an_afterburner_from_a_rating_of_512() {
    let Some(data) = opened() else {
        return;
    };
    // The Shuttle (128, `Flags` 0x0120) against `shïp` 128's `Strength`
    // of 2: a rating of 512 passes the best roll, 511 none.
    let shuttle = kinds(&data)
        .into_iter()
        .find(|(id, _)| *id == ShipId(128))
        .map(|(_, kind)| kind)
        .expect("the Shuttle");
    let rolled = |rating: i64| {
        afterburner::has_afterburner(&shuttle.hull, rating, 2, &mut Ranged::of(&[(1344, 0)]))
    };
    assert!(rolled(512));
    assert!(!rolled(511));
}

/// Each NPC attacks the ship its entry names; one with none idles.
#[derive(Debug)]
struct Attacks(BTreeMap<NpcId, ShipRef>);

impl Behaviour for Attacks {
    fn decide(&self, npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
        self.0
            .get(&npc.id)
            .map_or(Goal::Idle, |&ship| Goal::Attack(ship))
    }
}

/// A stock Dragon at `at` facing `heading`, its afterburner rolled.
fn dragon(data: &GameData, at: Vec2, heading: f32) -> Npc {
    let kind = kinds(data)
        .into_iter()
        .find(|(id, _)| *id == DRAGON)
        .map(|(_, kind)| kind)
        .expect("the Dragon");
    let mut npc = nova_sim::traffic::fixture::npc(0, kind.stats);
    npc.ship = DRAGON;
    npc.hull = kind.hull;
    npc.armament = kind.armament;
    npc.rounds = kind.rounds;
    npc.state = ShipState {
        position: at,
        velocity: Vec2::ZERO,
        heading,
    };
    npc.afterburner = afterburner::has_afterburner(&npc.hull, 0, 2, &mut Ranged::of(&[]));
    npc
}

/// The player at `at`, at rest.
fn player(at: Vec2) -> PlayerSide {
    PlayerSide {
        state: ShipState {
            position: at,
            ..ShipState::default()
        },
        condition: nova_sim::combat::hull::Condition::Intact,
        reserves: nova_sim::Reserves::full(100.0, 100.0, 100.0),
        hull: nova_sim::combat::hull::HullSpec::default(),
        handling: nova_sim::stats::ShipStats::default().handling,
        rating: 0,
        yardstick: 2,
        untargetable: false,
        blessing: nova_sim::rank::Blessing::NONE,
        ship: ShipId(128),
        combat_govt: None,
        target: None,
        class: nova_sim::escort::EscortClass::Freighter,
        boarded: false,
    }
}

#[test]
fn a_stock_dragon_approaching_the_player_rushes_in_past_its_top_speed() {
    let Some(data) = opened() else {
        return;
    };
    let mut ship = dragon(&data, Vec2::ZERO, 0.0);
    assert!(ship.afterburner, "Flags 0x0040");
    let top = ship.stats.handling.max_speed;
    let accel = ship.stats.handling.accel;
    // The player 300 left and 2000 up: its bearing about 351, within 30
    // of the Dragon's heading of 0, and over 82 off on both axes.
    let at = Vec2::new(-300.0, -2000.0);
    ship.state.heading = 351.0;
    let mut traffic = Traffic::with_interval(1);
    let id = traffic.add_npc(ship);
    let behaviour = Attacks(BTreeMap::from([(id, ShipRef::Player)]));
    let arsenal = Arsenal::read(&data);
    let world = World {
        player: Some(player(at)),
        weapons: &arsenal,
        ..World::new(&[])
    };
    let mut chance = Ranged::of(&[]);
    traffic.tick_in(&behaviour, world, &[], &mut chance);
    assert_eq!(traffic.npcs()[0].pursuit.kept, Some(Kept::Rush));
    for _ in 0..60 {
        traffic.tick_in(&behaviour, world, &[], &mut chance);
    }
    let rushing = &traffic.npcs()[0];
    let frame = rushing.pursuit.frame.expect("a frame");
    assert_eq!(
        frame.last().map(|body| body.manoeuvre),
        Some(Manoeuvre::Rush)
    );
    let velocity = rushing.state.velocity;
    let speed = velocity.length();
    assert!(speed > top, "{speed} over {top}");
    // Each axis is pushed only while it is short of its share of 1.8 ×
    // the top speed, and nothing clamps an NPC's (`_AdjustedAccel`, the
    // per-axis reading of `RuleKey::SpeedCap`), so neither axis runs past
    // that by more than one rush push.
    let cap = top * 1.8 + accel * RUSH_THRUST + 1e-3;
    assert!(
        velocity.x.abs() <= cap && velocity.y.abs() <= cap,
        "{velocity:?} within 1.8 × {top} on each axis"
    );
}

#[test]
fn a_stock_dragon_meeting_another_head_on_breaks_off() {
    let Some(data) = opened() else {
        return;
    };
    // Slot 1 faces down at slot 2 facing up, 100 apart: slot 2 jousts the
    // lower slot, and a `Rand(2)` of 0 breaks it off, clockwise from its
    // even slot.
    let mut traffic = Traffic::with_interval(1);
    let target = traffic.add_npc(dragon(&data, Vec2::new(0.0, -100.0), 180.0));
    let jouster = traffic.add_npc(dragon(&data, Vec2::ZERO, 0.0));
    let behaviour = Attacks(BTreeMap::from([(jouster, ShipRef::Npc(target))]));
    let arsenal = Arsenal::read(&data);
    let world = World {
        weapons: &arsenal,
        ..World::new(&[])
    };
    traffic.tick_in(&behaviour, world, &[], &mut Ranged::of(&[(2, 0)]));
    let ship = traffic
        .npcs()
        .iter()
        .find(|npc| npc.id == jouster)
        .expect("the jouster");
    assert_eq!(ship.berth, 2);
    assert_eq!(ship.pursuit.kept, Some(Kept::BreakOff { heading: 135 }));
    let frame = ship.pursuit.frame.expect("a frame");
    assert_eq!(
        frame.iter().map(|body| body.manoeuvre).collect::<Vec<_>>(),
        [Manoeuvre::Dogfight, Manoeuvre::BreakOff]
    );
}
