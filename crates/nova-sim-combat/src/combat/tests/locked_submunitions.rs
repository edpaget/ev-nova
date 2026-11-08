//! The fight's tests of a locked shot's end-of-life sub-munitions (see
//! the `locked_slot` module): given the asteroid's slot k as their
//! target (`_HandleShot` @0x35f3e-0x35f49), they are aimed at the ship in
//! ship slot k, or, at an empty slot, made lost unless their seek finds
//! a ship.

use super::*;
use crate::combat::projectile::Jam;
use crate::combat::weapon::SUBS_SEEK_NEAREST;

/// The sub-munitions a shell of A's with `flags2`, locked on asteroid
/// slot `k`, releases as its life ends among `ships`, by `source`'s
/// reading of the locked slot.
fn released(k: Option<usize>, flags2: u16, source: RuleSource, ships: &mut [Ship]) -> Vec<Shot> {
    let (shell, arsenal) = cluster(flags2);
    let mut combat = Combat::default();
    combat.set_locked_shot_ship_slot(source);
    let id = put(&mut combat, shell, A, Vec2::ZERO, Vec2::new(10.0, 0.0));
    let shot = combat
        .shots
        .iter_mut()
        .find(|shot| shot.id == id)
        .expect("in flight");
    shot.jam = Jam::Asteroid(k);
    shot.life = 0.0;
    fly_shots_with(&mut combat, ships, &arsenal);
    sub_shots(&combat).into_iter().copied().collect()
}

/// A, the firer, in berth 1, and C in berth 3, off to the side.
fn firer_and_c() -> [Ship; 2] {
    [Ship::at(1, 0.0, 0.0), Ship::at(3, 0.0, 300.0)]
}

/// Each sub-munition's target, the ship it is aimed at, and its jam.
fn aims(subs: &[Shot]) -> Vec<(Option<ShipRef>, Option<ShipRef>, Jam)> {
    subs.iter()
        .map(|sub| (sub.target, sub.aimed_at, sub.jam))
        .collect()
}

#[test]
fn a_locked_shells_sub_munitions_are_aimed_at_the_ship_in_its_slot() {
    let subs = released(Some(3), 0, RuleSource::Engine, &mut firer_and_c());
    assert_eq!(aims(&subs), [(Some(C), Some(C), Jam::Clear); 2]);
}

#[test]
fn by_the_other_reading_a_locked_shells_sub_munitions_are_aimed_at_none() {
    for k in [Some(3), Some(5)] {
        let subs = released(k, 0, RuleSource::Bible, &mut firer_and_c());
        assert_eq!(aims(&subs), [(None, None, Jam::Clear); 2], "{k:?}");
    }
}

#[test]
fn sub_munitions_released_at_an_empty_slot_are_lost() {
    // The gone-ship rule makes them lost on their first handling
    // (@0x355f7-0x3561b).
    let subs = released(Some(5), 0, RuleSource::Engine, &mut firer_and_c());
    assert_eq!(aims(&subs), [(None, None, Jam::Lost); 2]);
}

#[test]
fn a_seeking_sub_munition_at_an_empty_slot_keeps_the_ship_it_finds() {
    // `Flags2` 0x0010: the seek finds C (@0x3f3cf), and the gone-ship
    // rule tests the child's own +0x2a, C's slot.
    let subs = released(
        Some(5),
        SUBS_SEEK_NEAREST,
        RuleSource::Engine,
        &mut firer_and_c(),
    );
    assert_eq!(aims(&subs), [(Some(C), Some(C), Jam::Clear); 2]);
    // With no ship to find, it falls back to the empty slot, and is lost.
    let subs = released(
        Some(5),
        SUBS_SEEK_NEAREST,
        RuleSource::Engine,
        &mut [Ship::at(1, 0.0, 0.0)],
    );
    assert_eq!(aims(&subs), [(None, None, Jam::Lost); 2]);
}

#[test]
fn a_shell_that_let_its_slot_go_releases_its_sub_munitions_at_none() {
    let subs = released(None, 0, RuleSource::Engine, &mut firer_and_c());
    assert_eq!(aims(&subs), [(None, None, Jam::Clear); 2]);
}
