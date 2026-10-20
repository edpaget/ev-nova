//! Tests of the thrust every ship flies with and the cap on its speed:
//! `Accel` / the [`RuleValue::ThrustDivisor`] and the
//! [`RuleKey::SpeedCap`] reading, read from the session's rules at each
//! use ([`Setup::flight_rules`](super::setup::Setup::flight_rules)) for the player and, through the
//! traffic's world, for every NPC; the pushes' clamp, which
//! [`Session::with_rules`] hands the fight; and an escort of an immobile
//! class, which no push moves.
#![allow(clippy::float_cmp)]

use super::*;
use crate::ai::{Behaviour, Goal, Surroundings};
use crate::chance::NeverFires;
use crate::rulebook::{RuleKey, RuleSource, RuleValue};
use crate::session::tests::{arming, blaster, with_an_npc_at};
use crate::traffic::npc::Npc;
use core::num::NonZeroU32;

/// Keeps every goal as it is.
#[derive(Debug)]
struct Keeping;

impl Behaviour for Keeping {
    fn decide(&self, npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
        npc.goal
    }
}

/// The rules with the thrust divided by `divisor`, or the default.
fn under(divisor: Option<u32>) -> Rulebook {
    divisor.map_or_else(Rulebook::default, |divisor| {
        Rulebook::default().with_value(
            RuleValue::ThrustDivisor,
            NonZeroU32::new(divisor).expect("not 0"),
        )
    })
}

/// A whole `Accel` total from a thrust under the original's divisor.
fn accel_total(thrust: f32) -> f64 {
    (f64::from(thrust) * 5000.0).round()
}

fn close(speed: f32, expected: f64) -> bool {
    ((f64::from(speed) - expected) / expected).abs() < 1e-5
}

const DIVISORS: [(Option<u32>, f64); 4] = [
    (None, 5000.0),
    (Some(5000), 5000.0),
    (Some(3000), 3000.0),
    (Some(7919), 7919.0),
];

#[test]
fn the_player_gains_its_accel_over_the_divisor_a_tick_from_rest() {
    let catalog = arming(blaster(), blaster(), 0);
    for (divisor, by) in DIVISORS {
        let mut session = Session::start(&catalog)
            .expect("starts")
            .with_rules(under(divisor));
        // The first tick's box of none holds the ship at rest.
        session.tick(Controls::default());
        assert_eq!(session.player().velocity, Vec2::ZERO);
        let total = accel_total(session.handling().accel);
        assert!(total > 0.0);
        session.tick(Controls {
            thrust: true,
            ..Controls::default()
        });
        let speed = session.player().velocity.length();
        assert!(close(speed, total / by), "{divisor:?}: {speed}");
    }
}

#[test]
fn an_npc_gains_its_accel_over_the_divisor_a_tick_from_rest() {
    let catalog = arming(blaster(), blaster(), 0);
    for (divisor, by) in DIVISORS {
        // 100 below the player, facing down, away from the centre.
        let mut session = with_an_npc_at(&catalog, 750, 850, 180).with_rules(under(divisor));
        let id = session.npcs()[0].id;
        let npc = session.npc_mut(id).expect("there");
        npc.goal = Goal::JumpOut;
        let total = accel_total(npc.stats.handling.accel);
        assert!(total > 0.0);
        assert_eq!(npc.state.heading, 180.0);
        session.tick_traffic(&catalog, &Keeping, &mut NeverFires);
        let npc = session.npc(id).expect("there");
        let speed = npc.state.velocity.length();
        assert!(
            close(speed, total / by),
            "{divisor:?}: {speed} {:?}",
            npc.state
        );
        assert!(npc.state.velocity.y > 0.0, "out from the centre");
    }
}

/// The rules with `speed_cap` read the other way: the circle.
fn circle() -> Rulebook {
    Rulebook::default().with_override(RuleKey::SpeedCap, RuleSource::Bible)
}

#[test]
fn the_player_at_top_speed_turned_a_quarter_and_thrusting_ends_near_c_by_minus_c() {
    let catalog = arming(blaster(), blaster(), 0);
    for (rules, circled) in [(Rulebook::default(), false), (circle(), true)] {
        let mut session = Session::start(&catalog).expect("starts").with_rules(rules);
        // The first tick's box of none holds the ship at rest.
        session.tick(Controls::default());
        let c = session.top_speed();
        session.ship.player.velocity = Vec2::new(c, 0.0);
        session.ship.player.heading = 0.0;
        for _ in 0..200 {
            session.tick(Controls {
                thrust: true,
                ..Controls::default()
            });
        }
        let velocity = session.player().velocity;
        if circled {
            assert!((velocity.length() - c).abs() < 1e-4, "{velocity:?} at {c}");
        } else {
            assert!(
                (velocity - Vec2::new(c, -c)).length() < 1e-4,
                "{velocity:?} at {c}"
            );
        }
    }
}

#[test]
fn an_npc_coasting_past_its_top_speed_keeps_it_by_default_and_is_rescaled_under_the_circle() {
    let catalog = arming(blaster(), blaster(), 0);
    for (rules, circled) in [(Rulebook::default(), false), (circle(), true)] {
        let mut session = with_an_npc_at(&catalog, 750, 850, 180).with_rules(rules);
        let id = session.npcs()[0].id;
        let npc = session.npc_mut(id).expect("there");
        npc.goal = Goal::Hold;
        let c = npc.top_speed(1.0);
        npc.state.velocity = Vec2::new(-2.0 * c, 3.0 * c);
        session.tick_traffic(&catalog, &Keeping, &mut NeverFires);
        let velocity = session.npc(id).expect("there").state.velocity;
        if circled {
            assert!((velocity.length() - c).abs() < 1e-4, "{velocity:?} at {c}");
        } else {
            assert_eq!(velocity, Vec2::new(-2.0 * c, 3.0 * c), "unclamped");
        }
    }
}

#[test]
fn the_rules_hand_the_fight_the_speed_caps_reading() {
    let catalog = arming(blaster(), blaster(), 0);
    let engine = Session::start(&catalog)
        .expect("starts")
        .with_rules(Rulebook::default());
    let circled = Session::start(&catalog)
        .expect("starts")
        .with_rules(circle());
    let mut expected = engine.combat.clone();
    assert_ne!(circled.combat, expected);
    expected.set_speed_cap(RuleSource::Bible);
    assert_eq!(circled.combat, expected);
}

/// A class of no `Speed` and no `Accel`, ship 204.
fn immobile_class() -> crate::catalog::ShipRecord {
    crate::testkit::ship(
        204,
        crate::handling::ShipFields {
            speed: 0,
            accel: 0,
            ..crate::testkit::FAST
        },
    )
}

#[test]
fn an_escort_of_a_class_of_no_speed_and_no_accel_flies_immobile_and_stays_put() {
    let shoved = |record: crate::catalog::ShipRecord| {
        let mut catalog = crate::testkit::catalog();
        let ship = record.id;
        catalog.ship_records.push(record);
        let mut pilot = crate::pilot::Pilot::new(&catalog, "Ada").expect("starts");
        pilot.escorts = vec![crate::pilot::Escort {
            ship,
            reserves: crate::reserves::Reserves::full(30.0, 45.0, 300.0),
            order: None,
            carried: false,
            wage: None,
            person: None,
        }];
        let mut session = Session::fly(&catalog, pilot).expect("flies");
        // The first tick places it on its slot.
        session.tick_traffic(&catalog, &Keeping, &mut NeverFires);
        let npc = session
            .traffic
            .npcs_mut()
            .iter_mut()
            .find(|npc| npc.ship == ship)
            .expect("the escort");
        let (id, immobile, at) = (npc.id, npc.immobile, npc.state.position);
        // Held, so the push alone could move it.
        npc.goal = Goal::Hold;
        npc.state.velocity = Vec2::new(3.0, -1.0);
        for _ in 0..3 {
            session.tick_traffic(&catalog, &Keeping, &mut NeverFires);
        }
        let state = session.npc(id).expect("there").state;
        (immobile, at, state)
    };
    let (immobile, at, state) = shoved(immobile_class());
    assert!(immobile, "its class's");
    assert_eq!(state.position, at, "a push moves it not at all");
    assert_eq!(state.velocity, Vec2::ZERO, "and lasts only until its tick");
    let (immobile, at, state) = shoved(crate::testkit::ship(
        204,
        crate::handling::ShipFields {
            speed: 0,
            ..crate::testkit::FAST
        },
    ));
    assert!(!immobile, "it can thrust");
    assert_ne!(state.position, at, "a push moves it");
}
