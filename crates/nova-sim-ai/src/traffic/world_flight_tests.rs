//! Tests of [`World::flight`] reaching what the NPCs see,
//! [`Surroundings::flight`](crate::ai::Surroundings::flight). Kept out of
//! `traffic/mod.rs`, which is large.

use super::World;
use crate::flight::FlightRules;
use std::num::NonZeroU32;

#[test]
fn a_worlds_surroundings_carry_its_flight_rules() {
    assert_eq!(World::new(&[]).around(&[]).flight, World::new(&[]).flight);
    let strong = FlightRules {
        thrust_divisor: NonZeroU32::new(1000).expect("non-zero"),
        ..FlightRules::default()
    };
    let world = World {
        flight: strong,
        ..World::new(&[])
    };
    assert_eq!(world.around(&[]).flight, strong);
}
