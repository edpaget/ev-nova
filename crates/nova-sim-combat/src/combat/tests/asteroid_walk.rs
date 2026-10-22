//! The asteroid passes destroying an asteroid inside the walk, as
//! `_HandleAsteroidHit` does (rdm asteroids phase 10, see the
//! `asteroid_walk` module): a fragment seated mid-pass stays hidden from
//! every contact walk but not from the proximity pass, and a fragment
//! seated in its parent's own slot by a second destruction is lost, its
//! struct read by the walk's next hit.

use super::*;
use crate::asteroid::AsteroidType;

/// The collision passes alone among `rocks`, by the default rules: the
/// events they raised.
fn test_amid(combat: &mut Combat, rocks: &mut Rocks) -> Vec<CombatEvent> {
    let (arsenal, govts) = (Arsenal::default(), Governments::default());
    let arena = Arena {
        pass: GamePass::Advance { double_time: false },
        arsenal: &arsenal,
        govts: &govts,
        rules: Rules::default(),
        field: Field::default(),
    };
    let scenery = Scenery {
        asteroids: Some(rocks.lend()),
        ..Scenery::default()
    };
    combat.test_hits(&mut [], arena, scenery, &mut NeverFires);
    combat.take_events()
}

/// `rocks` with kind 0 breaking into one fragment of kind 1 (`FragCount`
/// 2: 1 + `Rand(2)`), and kind 1 `size` across, of `Strength`
/// `strength`, dropping 2 boxes of commodity 5 on average.
fn breaking(mut rocks: Rocks, size: f32, strength: i16) -> Rocks {
    rocks.types[0] = AsteroidType {
        fragments: [Some(1), None],
        frag_count: 2,
        ..rocks.types[0]
    };
    rocks.types[1] = AsteroidType {
        size: (size, size),
        strength,
        yield_qty: 2,
        yield_type: Some(5),
        ..AsteroidType::MISSING
    };
    rocks
}

/// A blaster shot of A's moving from `from` to `to`: its number.
fn blast_from(combat: &mut Combat, from: Vec2, to: Vec2) -> ShotId {
    put(combat, blaster(), A, from, to - from)
}

#[test]
fn a_second_destruction_loses_the_fragment_it_seats_in_its_own_slot_and_the_next_hit_reads_it() {
    // A0, alone in slot 0, is met by three shots in one walk. s1 destroys
    // it, its fragment taking slot 1 (slot 0 still held, @0xb00b). s2
    // meets its struct and destroys it again: its fragment takes slot 0,
    // free now (@0x3fbf7), overwriting the struct, and is cleared with it.
    // s3 then meets that fragment's struct: of `Strength` 50 it stands;
    // of 3 it is destroyed a third time, as kind 1.
    for (strength, destroyed) in [(50, 2), (3, 3)] {
        let mut combat = Combat::default();
        let mut rocks = breaking(Rocks::of([worn(0, 80.0, 0.0, 1)]), 6.0, strength);
        for x in [75.0, 80.0, 85.0] {
            blast_from(&mut combat, Vec2::new(x, -50.0), Vec2::new(x, 50.0));
        }
        test_amid(&mut combat, &mut rocks);
        assert_eq!(rocks.get(0), None, "{strength}: the slot ends empty");
        let fragment = rocks.at(1);
        assert_eq!(
            (fragment.kind, fragment.strength, fragment.position),
            (1, strength, Vec2::new(80.0, 0.0)),
            "{strength}: the first destruction's, untouched"
        );
        assert_eq!(rocks.field.active(), 1, "{strength}: the second's lost");
        assert_eq!(combat.shots(), [], "{strength}: all three spent on A0");
        let remains = combat.take_asteroid_remains();
        assert_eq!(remains.len(), destroyed, "{strength}: {remains:?}");
        if let Some(third) = remains.get(2) {
            assert_eq!(third.contents, Some(5), "kind 1's yield");
            assert_eq!(third.boxes, 1);
        }
    }
}

#[test]
fn the_contact_walks_never_meet_a_fragment_seated_in_the_passes() {
    // s1 destroys A0 in the shots layer's walk; its fragment, 100 across,
    // takes slot 1, the first free. s2, later in the same walk, crosses
    // only the fragment, beyond A0's 10-pixel radius; the homing layer's
    // walk then crosses both, but A0's slot is hidden and the fragment's
    // sprite not shown yet (`_SpawnSubAsteroid` shows nothing; only
    // `_HandleAsteroids` does, @0x3638a). Both fly on.
    let mut combat = Combat::default();
    let mut rocks = breaking(Rocks::of([worn(0, 0.0, 0.0, 1)]), 100.0, 50);
    let s1 = blast_from(&mut combat, Vec2::new(0.0, -50.0), Vec2::new(0.0, 50.0));
    let s2 = blast_from(&mut combat, Vec2::new(30.0, -50.0), Vec2::new(30.0, 50.0));
    let s3 = put(
        &mut combat,
        missile(),
        A,
        Vec2::new(-50.0, 0.0),
        Vec2::new(100.0, 0.0),
    );
    let events = test_amid(&mut combat, &mut rocks);
    assert_eq!(flying(&combat, s1), None, "spent on A0");
    assert!(
        flying(&combat, s2).is_some(),
        "the walk in progress passes it"
    );
    assert!(
        flying(&combat, s3).is_some(),
        "a later layer's walk passes both"
    );
    assert_eq!(rocks.get(0), None);
    assert_eq!(rocks.at(1).strength, 50, "untouched");
    assert_eq!(combat.take_asteroid_remains().len(), 1, "{events:?}");
}

#[test]
fn a_fuse_in_the_proximity_pass_is_set_off_by_a_fragment_a_contact_hit_seated() {
    // `_CheckShotProximities` reads each slot's +0x20 and position alone
    // (@0x376dd-0x37744): the fragment s1 seated in slot 1 is in s2's
    // reach, though its sprite is hidden.
    let mut combat = Combat::default();
    let mut rocks = breaking(Rocks::of([worn(0, 0.0, 0.0, 1)]), 6.0, 50);
    blast_from(&mut combat, Vec2::new(0.0, -50.0), Vec2::new(0.0, 50.0));
    let fused = WeaponRecord {
        prox_radius: 30,
        ..blaster()
    };
    let s2 = put(
        &mut combat,
        fused,
        A,
        Vec2::new(20.0, -60.0),
        Vec2::new(0.0, 40.0),
    );
    test_amid(&mut combat, &mut rocks);
    assert_eq!(flying(&combat, s2), None, "set off");
    assert_eq!(rocks.at(1).strength, 46, "the fragment takes its 4");
    assert_eq!(rocks.get(0), None);
}
