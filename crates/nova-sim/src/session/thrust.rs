//! Tests of the thrust every ship flies with: `Accel` / the
//! [`RuleValue::ThrustDivisor`], read from the session's rules at each use
//! ([`Session::flight_rules`]) for the player and, through the traffic's
//! world, for every NPC.
#![allow(clippy::float_cmp)]

use super::*;
use crate::ai::{Behaviour, Goal, Surroundings};
use crate::chance::NeverFires;
use crate::rulebook::RuleValue;
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
