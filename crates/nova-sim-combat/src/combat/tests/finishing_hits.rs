//! The hits that start a ship dying, which an escort's victory voice reads
//! (`_HandleShipHit` @0x36c7f-0x36d45, rdm escort-fleet-fidelity phase
//! 10): a shot's direct hit on a ship not dying before it (`_IsDying`
//! @0x36ad5) and dying after it (@0x36c85). A beam's hit (`_HandleBeams`
//! → `_DamageShip` @0x316a1), a blast's (@0x36ec0) and a missile lock's
//! never note one.

use super::collision_walk::{INTO_C, fly, three};
use super::*;

/// A shell of 50 mass, enough to take C's last armour.
fn shell() -> WeaponRecord {
    WeaponRecord {
        mass_dmg: 50,
        ..blaster()
    }
}

/// Each strike of a collision pass among `ships` (its ship and
/// attacker), and the ships its hits finished, in order.
fn walk(
    combat: &mut Combat,
    ships: &mut [Ship],
) -> (Vec<(ShipRef, Option<ShipRef>)>, Vec<ShipRef>) {
    let (arsenal, govts) = (Arsenal::default(), Governments::default());
    let mut fighters: Vec<Fighter> = ships.iter_mut().map(Ship::fighter).collect();
    let arena = Arena {
        pass: GamePass::Advance { double_time: false },
        arsenal: &arsenal,
        govts: &govts,
        rules: Rules::default(),
        field: Field::default(),
    };
    combat.test_hits(&mut fighters, arena, Scenery::default(), &mut NeverFires);
    let (strikes, _, finished) = combat.take_landed();
    let struck = strikes
        .iter()
        .map(|strike| (strike.ship, strike.by))
        .collect();
    (struck, finished)
}

#[test]
fn a_shots_direct_hit_that_takes_the_last_armour_finishes_its_ship() {
    let mut ships = three();
    let mut combat = Combat::default();
    fly(&mut combat, (shell(), A, ShotLayer::Shots), INTO_C, None);
    assert_eq!(walk(&mut combat, &mut ships), (vec![(C, Some(A))], vec![C]));
    assert!(ships[1].condition.is_dying(ships[1].reserves.armor));
    assert_eq!(combat.take_landed().2, [], "taken");
}

#[test]
fn a_shots_direct_hit_that_leaves_armour_finishes_nothing() {
    let mut ships = three();
    let mut combat = Combat::default();
    fly(&mut combat, (blaster(), A, ShotLayer::Shots), INTO_C, None);
    assert_eq!(walk(&mut combat, &mut ships), (vec![(C, Some(A))], vec![]));
}

#[test]
fn a_hit_on_a_ship_already_breaking_up_finishes_nothing() {
    let mut ships = three();
    ships[1].condition = Condition::Dying {
        ticks_left: 0,
        starting: true,
    };
    let mut combat = Combat::default();
    fly(&mut combat, (shell(), A, ShotLayer::Shots), INTO_C, None);
    assert_eq!(
        walk(&mut combat, &mut ships),
        (vec![(C, Some(A))], vec![]),
        "it is hit"
    );
}

#[test]
fn a_blast_that_takes_the_last_armour_finishes_nothing() {
    let mut ships = three();
    // X beside C, sturdy enough to outlive the shell's direct hit; its
    // blast reaches C.
    let mut x = Ship::at(5, 25.0, -100.0);
    x.reserves.armor = Gauge::full(1000.0);
    ships.push(x);
    let blasting = WeaponRecord {
        blast_radius: 50,
        ..shell()
    };
    let into_x = (Vec2::new(60.0, -100.0), Vec2::new(40.0, -100.0));
    let mut combat = Combat::default();
    fly(&mut combat, (blasting, A, ShotLayer::Shots), into_x, None);
    let x = ShipRef::Npc(NpcId(5));
    assert_eq!(
        walk(&mut combat, &mut ships),
        (vec![(x, Some(A)), (C, Some(A))], vec![])
    );
    assert!(ships[1].condition.is_dying(ships[1].reserves.armor));
}

#[test]
fn a_beams_killing_tick_finishes_nothing() {
    let mut combat = Combat::default();
    let mut c = Ship::at(3, 0.0, -100.0);
    c.reserves.shield.now = 0.0;
    c.reserves.armor.now = 3.0;
    let mut ships = [Ship::at(1, -100.0, -100.0), c];
    let ray = WeaponSpec::new(&WeaponRecord {
        guidance: 0,
        count: 3,
        reload: 10,
        beam_length: 200,
        mass_dmg: 50,
        ..weapon(146)
    });
    let from = ShipState {
        position: Vec2::new(-100.0, -100.0),
        velocity: Vec2::ZERO,
        heading: 90.0,
    };
    combat
        .beams
        .push(Beam::launch(ray, A, A, &from, Aiming::Ahead));
    let (arsenal, govts) = (Arsenal::default(), Governments::default());
    let arena = Arena {
        pass: GamePass::Advance { double_time: false },
        arsenal: &arsenal,
        govts: &govts,
        rules: Rules::default(),
        field: Field::default(),
    };
    let mut fighters: Vec<Fighter> = ships.iter_mut().map(Ship::fighter).collect();
    combat.close_tick(&mut fighters, arena, &mut NeverFires);
    drop(fighters);
    assert!(ships[1].condition.is_dying(ships[1].reserves.armor));
    let (strikes, _, finished) = combat.take_landed();
    assert_eq!(strikes.len(), 1, "the beam hit");
    assert_eq!(finished, []);
}

#[test]
fn a_plunderers_spared_hit_finishes_nothing() {
    let mut ships = three();
    ships[0].plundering = true;
    ships[0].target = Some(C);
    let mut combat = Combat::default();
    fly(&mut combat, (shell(), A, ShotLayer::Shots), INTO_C, None);
    assert_eq!(walk(&mut combat, &mut ships), (vec![(C, Some(A))], vec![]));
    assert_eq!(ships[1].reserves.armor.now, damage::SPARED_ARMOR);
}

#[test]
fn a_missile_lock_finishes_nothing() {
    let mut ships = [player_with(missile()), Ship::at(2, 5000.0, 0.0)];
    ships[1].reserves.shield.now = 0.0;
    ships[1].reserves.armor.now = 0.5;
    let mut combat = Combat::default();
    let mut locks = 0;
    for _ in 0..40 {
        tick_with(
            &mut combat,
            &mut ships,
            &Arsenal::default(),
            Rules::default(),
        );
        ships[0].trigger = QUIET;
        let (strikes, _, finished) = combat.take_landed();
        locks += strikes.iter().filter(|strike| strike.lock).count();
        assert_eq!(finished, []);
    }
    assert_eq!(locks, 1);
}
