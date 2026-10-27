//! The fight's tests of `_HandleBeams`' asteroid pass (rdm asteroids
//! phase 6, see the `asteroid_beam` module): the ship pass first, the
//! mining NPC's beam passing every ship, a later beam meeting the
//! fragments an earlier one broke, a tractor's lock on its firer, and the
//! engine's nearest weighing asteroid k by berth k's ship, live or gone.

use super::*;
use crate::asteroid::{Asteroid, AsteroidType, Asteroids, KINDS};
use crate::combat::asteroid_walk::AsteroidField;

/// The fire of every ship among `ships`, then `_HandleBeams` on `pass`
/// with `field` lent, of `types`: the events.
fn beams_amid(
    combat: &mut Combat,
    ships: &mut [Ship],
    field: &mut Asteroids,
    types: &[AsteroidType; KINDS],
    pass: GamePass,
) -> Vec<CombatEvent> {
    let (arsenal, govts) = (Arsenal::default(), Governments::default());
    let arena = Arena {
        pass,
        arsenal: &arsenal,
        govts: &govts,
        rules: Rules::default(),
        field: Field::default(),
    };
    let mut fighters: Vec<Fighter> = ships.iter_mut().map(Ship::fighter).collect();
    combat.fire_player(&mut fighters, arena, &mut NeverFires);
    combat.handle_npcs(&mut fighters, arena, &mut NeverFires);
    let field = AsteroidField {
        asteroids: field,
        types,
    };
    combat.close_tick_in(&mut fighters, arena, &mut [], Some(field), &mut NeverFires);
    combat.take_events()
}

const ADVANCE: GamePass = GamePass::Advance { double_time: false };

/// Kind 0, 30 high, of `Mass` 100, breaking into one fragment of kind 1;
/// kind 1, 20 high, of `Strength` 50.
fn types() -> [AsteroidType; KINDS] {
    let mut types = [AsteroidType::MISSING; KINDS];
    types[0] = AsteroidType {
        size: (30.0, 30.0),
        mass: 100,
        fragments: [Some(1), None],
        frag_count: 2,
        ..AsteroidType::MISSING
    };
    types[1] = AsteroidType {
        size: (20.0, 20.0),
        strength: 50,
        ..AsteroidType::MISSING
    };
    types
}

/// A field of a kind-0 asteroid of `strength` at rest at (`x`, `y`).
fn rock_at(x: f32, y: f32, strength: i16) -> Asteroids {
    let mut field = Asteroids::default();
    field.place(Asteroid {
        position: Vec2::new(x, y),
        velocity: Vec2::ZERO,
        frame: 0.0,
        spin: 0.0,
        strength,
        kind: 0,
    });
    field
}

/// A 200-pixel beam doing 5 mass damage, of `Impact` `impact`.
fn cutter(impact: i16) -> WeaponRecord {
    WeaponRecord {
        mass_dmg: 5,
        impact,
        ..laser()
    }
}

fn strength(field: &Asteroids, slot: usize) -> Option<i16> {
    field.get(slot).map(|asteroid| asteroid.strength)
}

#[test]
fn a_ship_in_the_cone_shields_a_nearer_asteroid() {
    // `_HandleBeams` reaches the asteroid pass only with no ship found
    // (@0x30f4c-0x30f94).
    let mut combat = Combat::default();
    let mut field = rock_at(60.0, 0.0, 100);
    let mut ships = [
        Ship::at(1, 0.0, 0.0).armed(cutter(0)),
        Ship::at(2, 150.0, 0.0),
    ];
    beams_amid(&mut combat, &mut ships, &mut field, &types(), ADVANCE);
    assert_eq!(strength(&field, 0), Some(100), "untouched");
    assert!(ships[1].reserves.shield.now < 10.0, "the ship took it");
    assert!(combat.beams()[0].end.x > 100.0, "{:?}", combat.beams()[0]);
}

#[test]
fn a_mining_npcs_beam_passes_the_ship_and_cuts_the_asteroid_behind_it() {
    let mut combat = Combat::default();
    let mut field = rock_at(150.0, 0.0, 100);
    let mut miner = Ship::at(1, 0.0, 0.0).armed(cutter(0));
    miner.mining = true;
    let mut ships = [miner, Ship::at(2, 60.0, 0.0)];
    beams_amid(&mut combat, &mut ships, &mut field, &types(), ADVANCE);
    assert_eq!(ships[1].reserves.shield.now, 10.0, "passed through");
    assert_eq!(strength(&field, 0), Some(95));
    // trunc(150 − 0.2 × 30).
    let end = combat.beams()[0].end;
    assert!(close_by(end, Vec2::new(144.0, 0.0)), "{end:?}");
    assert_eq!(combat.take_strikes(), [], "no `_DamageShip`");
}

#[test]
fn a_later_beam_in_the_same_pass_meets_the_fragment_an_earlier_one_broke_off() {
    // `_DestroyAsteroid` runs inside the pass (@0x31a69): the fragment is
    // in the field for the next beam.
    let mut combat = Combat::default();
    let mut field = rock_at(100.0, 0.0, 4);
    let mut ships = [
        Ship::at(1, 0.0, 0.0).armed(cutter(0)),
        Ship::at(3, 0.0, 1.0).armed(cutter(0)),
    ];
    beams_amid(&mut combat, &mut ships, &mut field, &types(), ADVANCE);
    assert_eq!(field.get(0), None, "the parent's slot freed");
    let fragment = field.get(1).expect("the fragment, in the next free slot");
    assert_eq!((fragment.kind, fragment.strength), (1, 45), "{fragment:?}");
    let ends: Vec<Vec2> = combat.beams().iter().map(|beam| beam.end).collect();
    // trunc(100 − 6), then trunc(√(100² + 1²) − 4) along the second.
    assert!(close_by(ends[0], Vec2::new(94.0, 0.0)), "{ends:?}");
    assert!((ends[1] - Vec2::new(96.0, 1.0)).length() < 1e-3, "{ends:?}");
    let remains = combat.take_asteroid_remains();
    assert_eq!(remains.len(), 1);
    assert_eq!(remains[0].at, Vec2::new(100.0, 0.0));
    assert_eq!(combat.take_asteroid_remains(), [], "taken");
}

#[test]
fn a_tractor_on_a_heavy_asteroid_pulls_its_firer_and_touches_its_lock() {
    for (held, after) in [
        (None, TractorLock::by(A)),
        (
            Some(TractorLock {
                holder: B,
                ticks_since_touch: 7,
            }),
            TractorLock::by(B),
        ),
    ] {
        let mut combat = Combat::default();
        let mut field = rock_at(100.0, 0.0, 100);
        let mut firer = of_mass(Ship::at(1, 0.0, 0.0).armed(cutter(-5)), 10.0);
        firer.tractor = held;
        let mut ships = [firer];
        beams_amid(&mut combat, &mut ships, &mut field, &types(), ADVANCE);
        assert_eq!(ships[0].tractor, Some(after), "{held:?}");
        let pulled = shoves_on(&combat.take_shoves(), A);
        assert_eq!(pulled.len(), 1, "{held:?}");
        assert!(close_by(pulled[0], Vec2::new(0.5, 0.0)), "{pulled:?}");
        let rock = field.get(0).expect("there");
        assert_eq!(rock.velocity, Vec2::ZERO, "the asteroid stays");
    }
}

#[test]
fn double_times_second_pass_cuts_nothing() {
    let mut combat = Combat::default();
    let mut field = rock_at(100.0, 0.0, 100);
    let mut ships = [Ship::at(1, 0.0, 0.0).armed(cutter(0))];
    beams_amid(&mut combat, &mut ships, &mut field, &types(), ADVANCE);
    assert_eq!(strength(&field, 0), Some(95));
    let events = beams_amid(
        &mut combat,
        &mut ships,
        &mut field,
        &types(),
        GamePass::Again,
    );
    assert_eq!(strength(&field, 0), Some(95));
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, CombatEvent::Exploded { .. })),
        "{events:?}"
    );
}

/// A field of two kind-0 asteroids at rest, slot 0 at (60, 0) and slot 1,
/// farther along the beam, at (150, 0).
fn near_and_far() -> Asteroids {
    let mut field = rock_at(60.0, 0.0, 100);
    field.place(Asteroid {
        position: Vec2::new(150.0, 0.0),
        ..*field.get(0).expect("placed")
    });
    field
}

/// A fight weighing the nearest asteroid by `BeamConeQuirks`' engine
/// reading, by ship slot k's distance.
fn by_the_engines_nearest() -> Combat {
    let mut combat = Combat::default();
    combat.set_beam_cone_quirks(RuleSource::Engine);
    combat
}

/// The player in slot 0, unarmed, at (`x`, 0), behind the beam.
fn player_at(x: f32) -> Ship {
    Ship {
        id: ShipRef::Player,
        fleet: ShipRef::Player,
        slot: Slot::Player,
        ..Ship::at(0, x, 0.0)
    }
}

/// Where the beam of C, in berth 3 at the centre facing right, ends among
/// `near_and_far`, with `others` in the fight.
fn cut_by_berth_3(combat: &mut Combat, others: Vec<Ship>) -> Vec2 {
    let mut field = near_and_far();
    let mut ships = others;
    ships.push(Ship::at(3, 0.0, 0.0).armed(cutter(0)));
    beams_amid(combat, &mut ships, &mut field, &types(), ADVANCE);
    combat.beams()[0].end
}

#[test]
fn by_the_engines_nearest_asteroid_1_is_weighed_by_the_npc_in_berth_1() {
    // The player (slot 0) 100 behind weighs asteroid 0 at 100² = 10,000;
    // A in berth 1, 10 behind, weighs asteroid 1 at 10² = 100, so the
    // farther asteroid wins. Its own distance, 150² = 22,500, would lose.
    let others = || vec![player_at(-100.0), Ship::at(1, -10.0, 0.0)];
    let end = cut_by_berth_3(&mut by_the_engines_nearest(), others());
    // trunc(150 − 0.2 × 30).
    assert!(close_by(end, Vec2::new(144.0, 0.0)), "{end:?}");
    // By the other reading, each by its own distance: the nearer.
    let end = cut_by_berth_3(&mut Combat::default(), others());
    assert!(close_by(end, Vec2::new(54.0, 0.0)), "{end:?}");
}

#[test]
fn by_the_engines_nearest_a_berth_its_ship_left_weighs_by_where_it_was_noted() {
    // A, noted in berth 1, leaves the fight: the slot keeps its position
    // (`_HandleShip` @0x3359a), and asteroid 1 is weighed by it.
    for (noted, end) in [(-10.0, 144.0), (-200.0, 54.0)] {
        let mut combat = by_the_engines_nearest();
        let mut left = [Ship::at(1, noted, 0.0)];
        combat.note_slots(&left.iter_mut().map(Ship::fighter).collect::<Vec<_>>());
        let cut = cut_by_berth_3(&mut combat, vec![player_at(-100.0)]);
        assert!(close_by(cut, Vec2::new(end, 0.0)), "{noted}: {cut:?}");
    }
}
