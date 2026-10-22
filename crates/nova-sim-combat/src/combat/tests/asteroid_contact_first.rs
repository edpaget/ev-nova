//! The asteroid passes as `_DoPlayGameWork` walks them, and point
//! defence's layer in its place after them (rdm asteroids phase 9).
//!
//! The asteroid layer 0x3b53b4 is the outer list of every asteroid pass
//! (@0x43ed7, @0x43f4d, @0x43fc3, @0x43fe3), its sprites added slot by
//! slot (`_InitSprites` @0x2000d), so each asteroid in slot order meets
//! every shot of the layer it is collided with. Point defence's layer
//! meets the homing layer only after all of them (@0x43fff-0x44010), and
//! `_SWCollideSpriteLayer` skips a sprite hidden by then (@0xacba3,
//! @0xacbc6): a missile or a point-defence shot spent on an asteroid is
//! never met.

use super::*;
use crate::combat::asteroid_hit::AsteroidTarget;

/// The collision passes alone among `ships` and `rocks`, by the default
/// rules: the events they raised.
fn test_amid(
    combat: &mut Combat,
    ships: &mut [Ship],
    rocks: &mut [AsteroidTarget],
) -> Vec<CombatEvent> {
    let (arsenal, govts) = (Arsenal::default(), Governments::default());
    let mut fighters: Vec<Fighter> = ships.iter_mut().map(Ship::fighter).collect();
    let arena = Arena {
        pass: GamePass::Advance { double_time: false },
        arsenal: &arsenal,
        govts: &govts,
        rules: Rules::default(),
        field: Field::default(),
    };
    let scenery = Scenery {
        asteroids: rocks,
        ..Scenery::default()
    };
    combat.test_hits(&mut fighters, arena, scenery, &mut NeverFires);
    combat.take_events()
}

/// Where `events` say something exploded, in order.
fn explosions(events: &[CombatEvent]) -> Vec<Vec2> {
    events
        .iter()
        .filter_map(|event| match *event {
            CombatEvent::Exploded { at, .. } => Some(at),
            _ => None,
        })
        .collect()
}

/// An asteroid in slot `slot` at (`x`, `y`) with `strength` left.
fn worn(slot: usize, x: f32, y: f32, strength: i16) -> AsteroidTarget {
    AsteroidTarget {
        strength,
        ..rock(slot, x, y)
    }
}

#[test]
fn a_shot_touching_two_asteroids_is_spent_on_the_one_in_the_lower_slot() {
    // Slot 0 is the farther along the move: the asteroid layer's walk
    // meets it first all the same.
    let mut combat = Combat::default();
    let shot = put(
        &mut combat,
        rock_gun(),
        A,
        Vec2::ZERO,
        Vec2::new(100.0, 0.0),
    );
    let mut rocks = [rock(0, 80.0, 0.0), rock(1, 30.0, 0.0)];
    let events = test_amid(&mut combat, &mut [], &mut rocks);
    assert_eq!(rocks[0].strength, 6, "the lower slot's: {rocks:?}");
    assert_eq!(rocks[1].strength, 10, "the nearer untouched");
    let at = explosions(&events);
    assert_eq!(at.len(), 1, "{events:?}");
    assert!((at[0] - Vec2::new(70.0, 0.0)).length() < 1e-3, "{at:?}");
    assert_eq!(flying(&combat, shot), None, "spent");
}

#[test]
fn a_shot_passes_through_an_asteroid_an_earlier_shot_broke_in_the_same_walk() {
    // A0's walk meets s1 first, which breaks it; s2, which touches only
    // A0, passes through the broken asteroid: the engine's skip until
    // rdm:phase/asteroids/phase-10-shots-break-asteroids-inline, where the
    // original's walk spends s2 on it too (@0xacd5b-0xacd6d).
    let mut combat = Combat::default();
    let s1 = put(&mut combat, blaster(), A, Vec2::ZERO, Vec2::new(100.0, 0.0));
    let s2 = put(
        &mut combat,
        blaster(),
        A,
        Vec2::new(80.0, -50.0),
        Vec2::new(0.0, 100.0),
    );
    let mut rocks = [worn(0, 80.0, 0.0, 1), rock(1, 30.0, 0.0)];
    test_amid(&mut combat, &mut [], &mut rocks);
    assert_eq!(rocks[0].strength, -3, "s1's 4 alone: {rocks:?}");
    assert_eq!(rocks[1].strength, 10, "s1 spent on A0");
    assert_eq!(combat.take_broken_asteroids(), [0]);
    assert_eq!(flying(&combat, s1), None);
    assert!(flying(&combat, s2).is_some(), "flies on");
}

#[test]
fn a_point_defence_shot_that_crosses_an_asteroid_hits_it_and_meets_no_missile() {
    let mut combat = Combat::default();
    let pd = put(
        &mut combat,
        pd_shot(),
        A,
        Vec2::new(-40.0, 0.0),
        Vec2::new(40.0, 0.0),
    );
    let missile = put(
        &mut combat,
        still_missile(),
        B,
        Vec2::new(3.0, 0.0),
        Vec2::ZERO,
    );
    let mut rocks = [rock(0, -20.0, 0.0)];
    let events = test_amid(&mut combat, &mut [], &mut rocks);
    assert_eq!(rocks[0].strength, 9, "the Quad's 1: {rocks:?}");
    assert_eq!(shot_down(&events), []);
    assert_eq!(flying(&combat, pd), None, "spent on the asteroid");
    assert!(flying(&combat, missile).is_some(), "flies on");
    assert_eq!(combat.hidden, [], "nothing point defence ended");
}

/// A's still point-defence shot at (3, 0), and B's missile of `record`
/// whose move from (-40, 0) to the centre crosses an asteroid at
/// (-20, 0) and ends overlapping it: their numbers.
fn missile_across_a_rock(combat: &mut Combat, record: WeaponRecord) -> (ShotId, ShotId) {
    let pd = put(combat, pd_shot(), A, Vec2::new(3.0, 0.0), Vec2::ZERO);
    let missile = put(
        combat,
        record,
        B,
        Vec2::new(-40.0, 0.0),
        Vec2::new(40.0, 0.0),
    );
    (pd, missile)
}

#[test]
fn a_missile_that_crosses_an_asteroid_hits_it_and_point_defence_never_meets_it() {
    let mut combat = Combat::default();
    let (pd, missile) = missile_across_a_rock(&mut combat, still_missile());
    let mut rocks = [rock(0, -20.0, 0.0)];
    let events = test_amid(&mut combat, &mut [], &mut rocks);
    assert_eq!(rocks[0].strength, -10, "the missile's 20: {rocks:?}");
    assert_eq!(shot_down(&events), []);
    assert_eq!(flying(&combat, missile), None, "spent on the asteroid");
    assert!(flying(&combat, pd).is_some(), "not spent");
    assert_eq!(combat.hidden, []);
}

#[test]
fn a_missile_whose_move_spent_its_life_is_spent_on_an_asteroid_and_never_met() {
    // `_AsteroidCollisionHandler` tests no life (@0x369c8-0x36a44).
    let mut combat = Combat::default();
    let expiring = WeaponRecord {
        count: 1,
        ..still_missile()
    };
    let (pd, missile) = missile_across_a_rock(&mut combat, expiring);
    assert!(combat.shots[1].expired());
    let mut rocks = [rock(0, -20.0, 0.0)];
    let events = test_amid(&mut combat, &mut [], &mut rocks);
    assert!(rocks[0].strength < 10, "{rocks:?}");
    assert_eq!(shot_down(&events), []);
    assert_eq!(flying(&combat, missile), None);
    assert!(flying(&combat, pd).is_some(), "not spent");
}

#[test]
fn a_missile_crossing_only_an_asteroid_a_shot_broke_in_an_earlier_layers_pass_is_met() {
    // A's blaster shot, in the shots layer, breaks the asteroid in its
    // pass (@0x43eec); `_DestroyAsteroid` hides its sprite (@0xb01c)
    // before the homing layer's asteroid pass (@0x43f62), which then
    // skips it (@0xacba3). So the missile crosses it untouched, and point
    // defence's layer (@0x44010) meets it.
    let mut combat = Combat::default();
    let gun = put(
        &mut combat,
        blaster(),
        A,
        Vec2::new(-20.0, -40.0),
        Vec2::new(0.0, 40.0),
    );
    let (pd, missile) = missile_across_a_rock(&mut combat, still_missile());
    let mut rocks = [worn(0, -20.0, 0.0, 1)];
    let events = test_amid(&mut combat, &mut [], &mut rocks);
    assert_eq!(rocks[0].strength, -3, "the blaster's 4 alone: {rocks:?}");
    assert_eq!(combat.take_broken_asteroids(), [0]);
    assert_eq!(shot_down(&events), [Vec2::new(3.0, 0.0)]);
    assert_eq!(flying(&combat, gun), None);
    assert_eq!(flying(&combat, missile), None, "shot down");
    assert_eq!(flying(&combat, pd), None, "spent on it");
}

#[test]
fn point_defence_shoots_missiles_down_after_the_asteroid_passes_hits() {
    // `ShotDown` follows the asteroid passes' events in the tick, as
    // point defence's layer follows them (@0x43fff).
    let mut combat = Combat::default();
    put(
        &mut combat,
        rock_gun(),
        A,
        Vec2::new(100.0, -40.0),
        Vec2::new(0.0, 40.0),
    );
    put(&mut combat, pd_shot(), A, Vec2::ZERO, Vec2::ZERO);
    put(
        &mut combat,
        still_missile(),
        B,
        Vec2::new(3.0, 0.0),
        Vec2::ZERO,
    );
    let mut rocks = [rock(0, 100.0, 0.0)];
    let events = test_amid(&mut combat, &mut [], &mut rocks);
    let kinds: Vec<&str> = events
        .iter()
        .map(|event| match event {
            CombatEvent::Exploded { .. } => "exploded",
            CombatEvent::Particles(_) => "particles",
            CombatEvent::ShotDown { .. } => "shot down",
            _ => "other",
        })
        .collect();
    assert_eq!(kinds, ["exploded", "particles", "shot down"]);
}
