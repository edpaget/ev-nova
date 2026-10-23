//! Tests of the `Seeker` 0x0002 lock through the session (rdm asteroids
//! phase 7; see the `asteroid_seek` module of the fight): both of
//! `_HandleShot`'s call sites, the fight's and the game work's, lend the
//! shots the field as it stands before `_HandleAsteroids` moves it
//! (`Space::asteroid_spots`). A missile locks onto an asteroid near and
//! ahead and steers at it, chases whatever its slot holds, flies straight
//! on once its slot is found empty, and from its lock hits no ship and
//! counts as aimed at none.

#![allow(clippy::float_cmp)]

use crate::ai::NovaAi;
use crate::asteroid::Asteroid;
use crate::catalog::WeaponRecord;
use crate::chance::Chance;
use crate::combat::projectile::{HOMING_DELAY, Jam, Shot};
use crate::combat::weapon::SEEKS_ASTEROIDS;
use crate::combat::{NpcId, Rules};
use crate::flight::Controls;
use crate::geometry::Vec2;
use crate::session::Session;
use crate::session::tests::{FIRE, arming, blaster, homing_missile, with_an_npc_at};
use crate::testkit::{FakePilotCatalog, Ranged};

/// A homing missile, 10 pixels a tick and turning 7 degrees, of
/// `Seeker` `seeker`.
fn missile(seeker: u16) -> WeaponRecord {
    WeaponRecord {
        seeker,
        ..homing_missile(134, 1000)
    }
}

/// The player at the centre facing up, armed with `missile(seeker)`,
/// targeting its one NPC at (`x`, `y`), with no traffic coming.
fn armed(seeker: u16, x: i32, y: i32) -> (FakePilotCatalog, Session) {
    let catalog = arming(missile(seeker), blaster(), 0);
    let at = |offset: i32| u32::try_from(750 + offset).expect("on the grid");
    let mut session = with_an_npc_at(&catalog, at(x), at(y), 0);
    session.space.traffic_due = false;
    session.chart.target = Some(NpcId(0));
    (catalog, session)
}

/// Puts an asteroid at rest at (`x`, `y`) in `session`'s field: its
/// slot.
fn rock(session: &mut Session, x: f32, y: f32) -> usize {
    session
        .space
        .asteroids
        .place(Asteroid {
            position: Vec2::new(x, y),
            velocity: Vec2::ZERO,
            frame: 0.0,
            spin: 0.0,
            strength: 1000,
            kind: 0,
        })
        .expect("a free slot")
}

/// The seek's draw of none, every other draw its highest.
fn seeks() -> Ranged {
    Ranged::of(&[(10, 0)])
}

/// The player's missile in flight.
fn missile_of(session: &Session) -> Shot {
    session.shots()[0]
}

/// `ticks` of `session`'s fight, its missile fired on the first,
/// drawing on `chance`.
fn fly(session: &mut Session, ticks: u32, chance: &mut dyn Chance) {
    for _ in 0..ticks {
        session.hold_trigger(FIRE);
        session.tick_combat(Rules::default(), &mut *chance);
        session.hold_trigger(crate::combat::armament::Trigger::default());
    }
}

#[test]
fn a_seeking_missile_locks_onto_an_asteroid_ahead_and_steers_at_it() {
    // The NPC off to the left; the asteroid about 12 degrees right.
    let (_, mut session) = armed(SEEKS_ASTEROIDS, -600, 0);
    let rock_slot = rock(&mut session, 40.0, -190.0);
    let mut chance = seeks();
    fly(&mut session, 1, &mut chance);
    let shot = missile_of(&session);
    assert_eq!(shot.jam, Jam::Asteroid(Some(rock_slot)), "{shot:?}");
    assert_eq!((shot.target, shot.aimed_at), (None, None));
    fly(&mut session, HOMING_DELAY, &mut chance);
    let shot = missile_of(&session);
    assert_eq!(shot.heading, 7.0, "on, at the asteroid: {shot:?}");
    // Without the bit it homes on the NPC to the left, and never
    // locks.
    let (_, mut plain) = armed(0, -600, 0);
    rock(&mut plain, 40.0, -190.0);
    let mut chance = seeks();
    fly(&mut plain, HOMING_DELAY + 1, &mut chance);
    let shot = missile_of(&plain);
    assert_eq!(shot.jam, Jam::Clear);
    assert_eq!(shot.heading, 353.0, "back, at the NPC: {shot:?}");
}

#[test]
fn the_lock_takes_199_pixels_and_15_degrees_and_not_more() {
    // Measured from the player's centre, where the missile is as its
    // guidance runs on the tick it is fired.
    for (x, y, locks) in [
        (0.0, -199.0, true),
        (0.0, -199.9, true),
        (0.0, -200.0, false),
        (35.0, -150.0, true),
        (50.0, -150.0, false),
    ] {
        let (_, mut session) = armed(SEEKS_ASTEROIDS, -600, 0);
        rock(&mut session, x, y);
        fly(&mut session, 1, &mut seeks());
        assert_eq!(
            missile_of(&session).jam == Jam::Asteroid(Some(0)),
            locks,
            "({x}, {y})"
        );
    }
}

#[test]
fn a_locked_missile_whose_asteroid_is_destroyed_flies_straight_on_and_never_locks_again() {
    let (catalog, mut session) = armed(SEEKS_ASTEROIDS, -600, 0);
    let rock_slot = rock(&mut session, 40.0, -190.0);
    let mut chance = seeks();
    fly(&mut session, 3, &mut chance);
    assert_eq!(missile_of(&session).jam, Jam::Asteroid(Some(rock_slot)));
    let types = std::rc::Rc::clone(&session.statics.asteroid_types);
    session
        .space
        .asteroids
        .destroy(rock_slot, &types, &mut chance)
        .expect("destroyed");
    // The game work's own pass reads the field too.
    session.hold_trigger(crate::combat::armament::Trigger::default());
    let rescued = session.game_work(
        Controls::default(),
        &catalog,
        &NovaAi::default(),
        Rules::default(),
        &mut chance,
    );
    assert!(!rescued);
    let shot = missile_of(&session);
    assert_eq!(shot.jam, Jam::Asteroid(None));
    let heading = shot.heading;
    // An asteroid ahead now, in the cone but off the path, and every
    // seek a roll of none.
    rock(&mut session, 25.0, shot.position.y - 150.0);
    fly(&mut session, HOMING_DELAY, &mut chance);
    let shot = missile_of(&session);
    assert_eq!(shot.jam, Jam::Asteroid(None), "never again");
    assert_eq!(shot.heading, heading, "straight on");
}

#[test]
fn a_locked_missile_chases_a_new_asteroid_in_its_slot() {
    let (_, mut session) = armed(SEEKS_ASTEROIDS, -600, 0);
    let rock_slot = rock(&mut session, 40.0, -190.0);
    let mut chance = seeks();
    fly(&mut session, 1, &mut chance);
    assert_eq!(missile_of(&session).jam, Jam::Asteroid(Some(rock_slot)));
    // Its asteroid replaced by one to the left.
    session.space.asteroids.clear();
    assert_eq!(rock(&mut session, -60.0, -190.0), rock_slot);
    fly(&mut session, HOMING_DELAY, &mut chance);
    let shot = missile_of(&session);
    assert_eq!(shot.jam, Jam::Asteroid(Some(rock_slot)));
    assert_eq!(shot.heading, 353.0, "back, at the new one: {shot:?}");
}

#[test]
fn a_locked_missile_passes_through_the_ship_it_was_fired_at_and_counts_as_aimed_at_none() {
    // The NPC 100 above, the asteroid straight on past it.
    for (seeker, hit) in [(SEEKS_ASTEROIDS, false), (0, true)] {
        let (_, mut session) = armed(seeker, 0, -100);
        let rock_slot = rock(&mut session, 0.0, -190.0);
        let mut chance = seeks();
        fly(&mut session, 1, &mut chance);
        let aimed = session.space.combat.targeted_damage();
        assert_eq!(aimed.is_empty(), !hit, "{seeker:#x}: {aimed:?}");
        let shield = session.npcs()[0].reserves.shield.now;
        fly(&mut session, 20, &mut chance);
        let struck = session.npcs()[0].reserves.shield.now < shield;
        assert_eq!(struck, hit, "{seeker:#x}");
        if !hit {
            let left = session.space.asteroids.get(rock_slot).map(|a| a.strength);
            assert!(
                left.is_none_or(|strength| strength < 1000),
                "the locked missile hit its asteroid instead: {left:?}"
            );
        }
    }
}

#[test]
fn the_game_works_pass_lends_the_field_too() {
    let (catalog, mut session) = armed(SEEKS_ASTEROIDS, -600, 0);
    let rock_slot = rock(&mut session, 40.0, -190.0);
    let mut chance = seeks();
    for _ in 0..2 {
        session.hold_trigger(FIRE);
        let rescued = session.game_work(
            Controls::default(),
            &catalog,
            &NovaAi::default(),
            Rules::default(),
            &mut chance,
        );
        assert!(!rescued);
    }
    let shot = missile_of(&session);
    assert_eq!(shot.jam, Jam::Asteroid(Some(rock_slot)), "{shot:?}");
}
