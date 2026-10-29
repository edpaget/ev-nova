//! The fight's tests of the `Seeker` 0x0002 lock (rdm asteroids phase 7,
//! see the `asteroid_seek` module): the shots seek among the asteroids
//! the fight is lent, and a locked shot neither hits the ship it was
//! fired at nor counts as aimed at it.

use super::*;
use crate::combat::projectile::Jam;
use crate::combat::weapon::SEEKS_ASTEROIDS;

/// A homing missile of `Seeker` 0x0002.
fn seeker() -> WeaponRecord {
    WeaponRecord {
        seeker: SEEKS_ASTEROIDS,
        ..missile()
    }
}

/// `_HandleShot` among `ships`, the guidance lent `asteroids`, drawing on
/// `chance`.
fn fly_amid(
    combat: &mut Combat,
    ships: &mut [Ship],
    asteroids: &[Option<Vec2>],
    chance: &mut dyn Chance,
) {
    let (arsenal, govts) = (Arsenal::default(), Governments::default());
    let arena = Arena {
        pass: GamePass::Advance { double_time: false },
        arsenal: &arsenal,
        govts: &govts,
        rules: Rules::default(),
        field: Field::default(),
    };
    let mut fighters: Vec<Fighter> = ships.iter_mut().map(Ship::fighter).collect();
    combat.fly_shots_in(&mut fighters, arena, asteroids, chance);
}

/// A fight with a seeking missile of A's at the centre, heading up, fired
/// at B: its number.
fn aimed_at_b(combat: &mut Combat) -> ShotId {
    let id = put(combat, seeker(), A, Vec2::ZERO, Vec2::ZERO);
    let shot = combat
        .shots
        .iter_mut()
        .find(|shot| shot.id == id)
        .expect("in flight");
    shot.target = Some(B);
    shot.aimed_at = Some(B);
    id
}

#[test]
fn the_fights_shots_seek_among_the_asteroids_it_is_lent() {
    let mut ships = [Ship::at(1, 0.0, 300.0), Ship::at(2, 300.0, 0.0)];
    let rocks = [None, Some(Vec2::new(0.0, -100.0))];
    let mut combat = Combat::default();
    let id = aimed_at_b(&mut combat);
    assert_eq!(combat.targeted_damage().get(&B), Some(&15), "aimed at B");
    fly_amid(&mut combat, &mut ships, &rocks, &mut Ranged::of(&[(10, 0)]));
    let shot = flying(&combat, id).expect("in flight");
    assert_eq!(shot.jam, Jam::Asteroid(Some(1)));
    assert_eq!(combat.targeted_damage(), BTreeMap::new(), "aimed at none");
    let mut unlent = Combat::default();
    let id = aimed_at_b(&mut unlent);
    let mut fighters: Vec<Fighter> = ships.iter_mut().map(Ship::fighter).collect();
    let (arsenal, govts) = (Arsenal::default(), Governments::default());
    let arena = Arena {
        pass: GamePass::Advance { double_time: false },
        arsenal: &arsenal,
        govts: &govts,
        rules: Rules::default(),
        field: Field::default(),
    };
    unlent.fly_shots(&mut fighters, arena, &mut Ranged::of(&[(10, 0)]));
    assert_eq!(flying(&unlent, id).expect("in flight").jam, Jam::Clear);
}

#[test]
fn a_locked_missile_passes_through_the_ship_it_was_fired_at() {
    for (jam, hits) in [(Jam::Asteroid(None), false), (Jam::Clear, true)] {
        let mut ships = [Ship::at(1, 0.0, 300.0), Ship::at(2, 0.0, -60.0)];
        let mut combat = Combat::default();
        let id = aimed_at_b(&mut combat);
        combat
            .shots
            .iter_mut()
            .find(|shot| shot.id == id)
            .expect("in flight")
            .jam = jam;
        // Its last move, from (0, -50) to (0, -70), crosses B.
        let shot = combat.shots_mut().first_mut().expect("in flight");
        shot.moved_from = Some(Vec2::new(0.0, -50.0));
        shot.position = Vec2::new(0.0, -70.0);
        phase::test_hits(&mut combat, &mut ships, &NovaDisable);
        assert_eq!(
            ships[1].reserves.shield.now < 10.0,
            hits,
            "{jam:?}: {:?}",
            combat.shots()
        );
        assert_eq!(combat.take_strikes().is_empty(), !hits, "{jam:?}");
    }
}
