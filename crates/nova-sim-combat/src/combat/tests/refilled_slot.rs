//! The fight's tests of a shot whose ship's slot a new ship refills (see
//! the `refilled_slot` module): it hits the newcomer, counts its damage
//! against it, steers at it and gives it to its sub-munitions, as the
//! original's +0x2a is a slot; by the other reading it is lost.

use super::*;
use crate::combat::projectile::Jam;

/// C's berth, which B takes once C has left the fight.
const BERTH: Slot = Slot::Berth(3);

/// A missile of A's at the centre, fired at C in berth 3: its number.
fn aimed_at_c(combat: &mut Combat) -> ShotId {
    let id = put(combat, missile(), A, Vec2::ZERO, Vec2::ZERO);
    let shot = shot_of(combat, id);
    shot.target = Some(C);
    shot.aimed_at = Some(C);
    shot.aimed_slot = Some(BERTH);
    id
}

fn shot_of(combat: &mut Combat, id: ShotId) -> &mut Shot {
    combat
        .shots
        .iter_mut()
        .find(|shot| shot.id == id)
        .expect("in flight")
}

/// A, the firer, in berth 1, and B, come into C's berth 3 at (`x`, `y`)
/// after C left the fight.
fn refilled(x: f32, y: f32) -> [Ship; 2] {
    [
        Ship::at(1, 0.0, 300.0),
        Ship {
            slot: BERTH,
            ..Ship::at(2, x, y)
        },
    ]
}

/// `_HandleShot` among `ships`, the guidance lent no asteroids.
fn fly(combat: &mut Combat, ships: &mut [Ship]) {
    fly_shots_with(combat, ships, &Arsenal::default());
}

#[test]
fn a_shot_whose_ships_slot_was_refilled_hits_and_counts_against_the_new_ship() {
    // Off its path, B is not hit, and the shot's damage counts against
    // it (@0x3a1b7), not against C.
    let mut ships = refilled(500.0, 0.0);
    let mut combat = Combat::default();
    let id = aimed_at_c(&mut combat);
    phase::test_hits(&mut combat, &mut ships, &NovaDisable);
    let shot = flying(&combat, id).expect("in flight");
    assert_eq!((shot.target, shot.aimed_at), (Some(B), Some(B)));
    let aimed = combat.targeted_damage();
    assert_eq!(aimed.get(&B), Some(&15));
    assert_eq!(aimed.get(&C), None);
    // On its path, B is hit (@0x44828).
    let mut ships = refilled(0.0, -60.0);
    let mut combat = Combat::default();
    let id = aimed_at_c(&mut combat);
    let shot = shot_of(&mut combat, id);
    // Its last move, from (0, -50) to (0, -70), crosses B.
    shot.moved_from = Some(Vec2::new(0.0, -50.0));
    shot.position = Vec2::new(0.0, -70.0);
    phase::test_hits(&mut combat, &mut ships, &NovaDisable);
    assert!(ships[1].reserves.shield.now < 10.0, "{:?}", combat.shots());
    assert_eq!(
        combat
            .take_strikes()
            .iter()
            .map(|strike| strike.ship)
            .collect::<Vec<_>>(),
        [B]
    );
}

#[test]
fn a_homing_shot_steers_at_the_new_ship_in_its_slot() {
    // `_HandleShotGuidance` steers at the slot's ship (@0x32134-0x3215c),
    // and the gone-ship rule passes its active slot (@0x355e0-0x35621).
    let mut combat = Combat::default();
    let id = aimed_at_c(&mut combat);
    shot_of(&mut combat, id).life = 50.0;
    let heading = flying(&combat, id).expect("in flight").heading;
    // B is off to the right: the shot turns toward it.
    let mut ships = refilled(300.0, 0.0);
    fly(&mut combat, &mut ships);
    let shot = flying(&combat, id).expect("in flight");
    assert_eq!(shot.jam, Jam::Clear);
    assert_eq!((shot.target, shot.aimed_at), (Some(B), Some(B)));
    let wanted = trig::intercept_angle(Vec2::ZERO, Vec2::new(300.0, 0.0));
    let off =
        |heading: f32| i32::from(trig::angular_difference(wanted, trig::whole(heading))).abs();
    assert!(
        off(shot.heading) < off(heading),
        "{heading} -> {}",
        shot.heading
    );
}

#[test]
fn a_refill_seen_only_at_the_shots_phase_is_followed_there() {
    // An arrival or an AI launch takes the slot after the collision
    // passes (`_EnterMoreShips` @0x44237, `_AIDispatch` @0x4429d): the
    // passes find the slot empty, `_HandleShot` finds B in it.
    let mut combat = Combat::default();
    let id = aimed_at_c(&mut combat);
    shot_of(&mut combat, id).life = 50.0;
    let mut alone = [Ship::at(1, 0.0, 300.0)];
    phase::test_hits(&mut combat, &mut alone, &NovaDisable);
    let shot = flying(&combat, id).expect("in flight");
    assert_eq!((shot.aimed_at, shot.jam), (Some(C), Jam::Clear));
    fly(&mut combat, &mut refilled(300.0, 0.0));
    let shot = flying(&combat, id).expect("in flight");
    assert_eq!((shot.aimed_at, shot.jam), (Some(B), Jam::Clear));
}

#[test]
fn a_shot_whose_slot_stays_empty_is_lost() {
    let mut combat = Combat::default();
    let id = aimed_at_c(&mut combat);
    shot_of(&mut combat, id).life = 50.0;
    fly(&mut combat, &mut [Ship::at(1, 0.0, 300.0)]);
    let shot = flying(&combat, id).expect("in flight");
    assert_eq!((shot.aimed_at, shot.jam), (None, Jam::Lost));
}

#[test]
fn a_spent_shots_children_are_given_the_new_ship() {
    // The end-of-life release gives the slot's ship (@0x35f3e-0x35f49).
    let (shell, arsenal) = cluster(0);
    let mut combat = Combat::default();
    let id = put(&mut combat, shell, A, Vec2::ZERO, Vec2::new(10.0, 0.0));
    let parent = shot_of(&mut combat, id);
    (parent.target, parent.aimed_at, parent.aimed_slot) = (Some(C), Some(C), Some(BERTH));
    parent.life = 0.0;
    fly_shots_with(&mut combat, &mut refilled(0.0, -300.0), &arsenal);
    let subs: Vec<_> = sub_shots(&combat)
        .into_iter()
        .map(|sub| (sub.target, sub.aimed_at, sub.aimed_slot, sub.jam))
        .collect();
    assert_eq!(subs, [(Some(B), Some(B), Some(BERTH), Jam::Clear); 2]);
}

#[test]
fn by_the_other_reading_a_refilled_slots_shot_is_lost() {
    let mut ships = refilled(0.0, -60.0);
    let mut combat = Combat::default();
    combat.set_reused_slot_shot_target(RuleSource::Bible);
    let id = aimed_at_c(&mut combat);
    let shot = shot_of(&mut combat, id);
    shot.moved_from = Some(Vec2::new(0.0, -50.0));
    shot.position = Vec2::new(0.0, -70.0);
    shot.life = 50.0;
    phase::test_hits(&mut combat, &mut ships, &NovaDisable);
    assert_eq!(ships[1].reserves.shield.now, 10.0, "B is not hit");
    assert!(combat.take_strikes().is_empty());
    fly(&mut combat, &mut ships);
    let shot = flying(&combat, id).expect("in flight");
    assert_eq!((shot.aimed_at, shot.jam), (None, Jam::Lost));
}

#[test]
fn a_ship_fired_shot_holds_its_targets_slot() {
    let combat = launched_by(
        Ship::at(1, 0.0, 0.0).targeting(C),
        blaster(),
        vec![Ship::at(3, 0.0, -300.0)],
    );
    assert_eq!(combat.shots()[0].aimed_slot, Some(BERTH));
    let combat = launched_by(Ship::at(1, 0.0, 0.0), blaster(), Vec::new());
    assert_eq!(combat.shots()[0].aimed_slot, None, "no target");
}
