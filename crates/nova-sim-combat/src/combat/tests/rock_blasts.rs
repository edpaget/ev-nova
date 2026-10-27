//! A shot put onto an asteroid for the collision passes alone
//! ([`Combat::test_hits`]), for the tests of the blasts its hit makes
//! (`_HandleAsteroidHit` @0x3670d-0x368ca).

use super::*;

/// `firer`'s shot of `record`, whose last move ran from (80, 0) to
/// (95, 0), onto the asteroid in slot 0 at (100, 0), meets it in the
/// collision passes among `others`, `firer` at the centre. The ships, in
/// order, `firer` first.
pub(super) fn shot_into_rock(
    combat: &mut Combat,
    firer: ShipRef,
    record: WeaponRecord,
    others: Vec<Ship>,
) -> Vec<Ship> {
    let mut gun = Ship::at(1, 0.0, 0.0);
    gun.id = firer;
    gun.fleet = firer;
    let mut ships = vec![gun];
    ships.extend(others);
    let mut rocks = Rocks::of([rock(0, 100.0, 0.0)]);
    put(
        combat,
        record,
        firer,
        Vec2::new(80.0, 0.0),
        Vec2::new(15.0, 0.0),
    );
    phase::run(
        combat,
        &mut ships,
        &NovaDisable,
        |combat, fighters, arena, chance| {
            let scenery = Scenery {
                asteroids: Some(rocks.lend()),
                ..Scenery::default()
            };
            combat.test_hits(fighters, arena, scenery, chance);
        },
    );
    assert_eq!(combat.shots(), [], "spent on the asteroid");
    ships
}
