//! The proximity pass as `_CheckShotProximities` runs it (rdm asteroids
//! phase 9): one loop over the shot slots (@0x371e0-0x37765), each slot's
//! shot taking its stellar branch (@0x37224-0x37527), then, live and
//! still, its ship fuse (@0x3757b-0x376a5), then its asteroid fuse
//! (@0x376dd-0x37758). A shot point defence ended at life 0, hidden,
//! keeps its slot and takes its turn among the live ones
//! (@0x371ea-0x371f6).

use super::*;
use crate::catalog::StellarId;

/// The collision passes alone among `ships`, `rocks` and `stellars`, by
/// the default rules.
fn test_amid(
    combat: &mut Combat,
    ships: &mut [Ship],
    rocks: &mut Rocks,
    stellars: &mut [StellarTarget],
) {
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
        stellars,
        asteroids: Some(rocks.lend()),
        hazards: None,
    };
    combat.test_hits(&mut fighters, arena, scenery, &mut NeverFires);
}

/// A shot of `record` that `firer` fired, still at `at` after its move,
/// that point defence ended at life 0 in `slot`: kept hidden, its slot
/// held, as `Combat::hide` keeps one.
fn hide_in(combat: &mut Combat, (record, firer, at): (WeaponRecord, ShipRef, Vec2), slot: usize) {
    let state = ShipState {
        position: at,
        ..ShipState::default()
    };
    let ended = Shot {
        id: combat.next_id(),
        position: at,
        moved_from: Some(at),
        age: 1,
        slot,
        ..Shot::launch(WeaponSpec::new(&record), firer, firer, &state, 0.0)
    };
    combat.slots.hold(slot);
    combat.hide(ended);
}

/// A live shot and a hidden one, each of its record and firer and still
/// where it is: the live one in slot 0 and the hidden one in slot 1 when
/// `live_first`, the other way about otherwise. The live one's number.
fn pair(
    combat: &mut Combat,
    live_first: bool,
    (record, firer, at): (WeaponRecord, ShipRef, Vec2),
    hidden: (WeaponRecord, ShipRef, Vec2),
) -> ShotId {
    if live_first {
        let live = put(combat, record, firer, at, Vec2::ZERO);
        hide_in(combat, hidden, 1);
        live
    } else {
        hide_in(combat, hidden, 0);
        put(combat, record, firer, at, Vec2::ZERO)
    }
}

#[test]
fn of_a_hidden_and_a_live_shot_the_lower_slot_downs_a_stellar_with_one_hit_left() {
    for live_first in [true, false] {
        let mut combat = Combat::default();
        let at = Vec2::new(0.0, 500.0);
        let live = pair(
            &mut combat,
            live_first,
            (planet_gun(), B, at),
            (planet_gun(), C, at),
        );
        assert_eq!(combat.shots()[0].slot, usize::from(!live_first));
        // 14 to a stellar of 13: shot down.
        let mut stellars = [stellar_at(0.0, 500.0, 20.0, 13)];
        test_amid(&mut combat, &mut [], &mut Rocks::of([]), &mut stellars);
        let by = if live_first { B } else { C };
        assert_eq!(
            combat.take_stellar_strikes(),
            [StellarStrike {
                stellar: StellarId(128),
                by: Some(by),
                damage: 14,
                destroyed: true,
            }],
            "live first: {live_first}"
        );
        assert_eq!(
            flying(&combat, live).is_some(),
            !live_first,
            "the live one spent only when first: {live_first}"
        );
        assert_eq!(combat.hidden, [], "the hidden one let go");
    }
}

#[test]
fn of_a_hidden_and_a_live_shot_the_lower_slot_breaks_an_asteroid_in_both_fuses_reach() {
    let live_gun = WeaponRecord {
        prox_radius: 30,
        ..blaster()
    };
    let hidden_gun = WeaponRecord {
        prox_radius: 30,
        mass_dmg: 20,
        ..blaster()
    };
    for live_first in [true, false] {
        let mut combat = Combat::default();
        let live = pair(
            &mut combat,
            live_first,
            (live_gun, B, Vec2::ZERO),
            (hidden_gun, C, Vec2::new(0.0, 5.0)),
        );
        let mut rocks = Rocks::of([worn(0, 20.0, 0.0, 1)]);
        test_amid(&mut combat, &mut [], &mut rocks, &mut []);
        assert_eq!(rocks.get(0), None, "destroyed: live first: {live_first}");
        assert_eq!(
            remains_at(&combat.take_asteroid_remains()),
            [Vec2::new(20.0, 0.0)],
            "once, by the lower slot's shot: live first: {live_first}"
        );
        assert_eq!(
            flying(&combat, live).is_some(),
            !live_first,
            "live first: {live_first}"
        );
        assert_eq!(combat.hidden, []);
    }
}

#[test]
fn a_hidden_shots_asteroid_blast_lands_before_the_next_slot_is_tested() {
    // The player's hidden shot sets off on the asteroid and blasts C
    // (`_HandleAsteroidHit` @0x3670d-0x368ca); B's live shot's fuse hits C
    // (`_HandleShipHit` @0x3768a). The strikes come in slot order.
    let blasting = WeaponRecord {
        prox_radius: 30,
        mass_dmg: 20,
        blast_radius: 40,
        ..blaster()
    };
    let fused = WeaponRecord {
        prox_radius: 40,
        ..blaster()
    };
    for live_first in [true, false] {
        let mut combat = Combat::default();
        pair(
            &mut combat,
            live_first,
            (fused, B, Vec2::new(30.0, 40.0)),
            (blasting, ShipRef::Player, Vec2::new(3.0, 0.0)),
        );
        let mut ships = [Ship::at(3, 30.0, 10.0)];
        let mut rocks = Rocks::of([rock(0, 23.0, 0.0)]);
        test_amid(&mut combat, &mut ships, &mut rocks, &mut []);
        assert_eq!(rocks.get(0), None, "destroyed: {rocks:?}");
        let struck: Vec<(ShipRef, Option<ShipRef>, bool)> = combat
            .take_strikes()
            .iter()
            .map(|strike| (strike.ship, strike.by, strike.direct))
            .collect();
        let fuse = (C, Some(B), true);
        let blast = (C, Some(ShipRef::Player), false);
        let order = if live_first {
            [fuse, blast]
        } else {
            [blast, fuse]
        };
        assert_eq!(struck, order, "live first: {live_first}");
    }
}

#[test]
fn a_planet_type_shot_meeting_a_stellar_goes_no_further_than_it() {
    // The stellar branch comes first, and a hit there ends the slot
    // (@0x3752d-0x37543): B, in its fuse's reach, is not hit.
    let mut combat = Combat::default();
    let fused = WeaponRecord {
        prox_radius: 30,
        ..planet_gun()
    };
    let shot = put(&mut combat, fused, A, Vec2::ZERO, Vec2::ZERO);
    let mut ships = [Ship::at(2, 0.0, 30.0)];
    let mut stellars = [stellar_at(0.0, 0.0, 20.0, 100)];
    test_amid(&mut combat, &mut ships, &mut Rocks::of([]), &mut stellars);
    assert_eq!(combat.take_stellar_strikes().len(), 1);
    assert_eq!(combat.take_strikes(), [], "B not hit");
    assert_eq!(flying(&combat, shot), None, "spent on the stellar");
}

#[test]
fn a_shot_meeting_two_stellars_strikes_the_first_in_the_systems_order() {
    // The stellar loop goes by the system's list (@0x37230-0x37527), not
    // along the move: the farther one, listed first, is struck.
    let mut combat = Combat::default();
    put(
        &mut combat,
        planet_gun(),
        A,
        Vec2::ZERO,
        Vec2::new(100.0, 0.0),
    );
    let mut stellars = [
        stellar_at(80.0, 0.0, 10.0, 100),
        StellarTarget {
            stellar: StellarId(129),
            ..stellar_at(30.0, 0.0, 10.0, 100)
        },
    ];
    test_amid(&mut combat, &mut [], &mut Rocks::of([]), &mut stellars);
    let strikes = combat.take_stellar_strikes();
    let struck: Vec<StellarId> = strikes.iter().map(|strike| strike.stellar).collect();
    assert_eq!(struck, [StellarId(128)]);
    assert_eq!(stellars[1].strength, 100, "the nearer untouched");
}
