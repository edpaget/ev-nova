//! The original's shot slots in the fight (rdm weapons-fidelity phase
//! 34): which slot each shot takes (`_SpawnShot` @0x3e5ab-0x3e5ce), when
//! `_HandleShot` frees it (@0x3617c), and the order each layer keeps
//! (`_PutSpriteInLayer` @0x766c-0x76b1).

use super::*;

/// A long-lived shot of C's, far from every ship, never moving.
fn idle() -> Shot {
    let weapon = WeaponSpec::new(&WeaponRecord {
        count: 1000,
        speed: 0,
        ..weapon(140)
    });
    let far = ShipState {
        position: Vec2::new(50_000.0, 50_000.0),
        ..ShipState::default()
    };
    Shot::launch(weapon, C, C, &far, 0.0)
}

/// A shot of C's whose last move, from 10 to the left, ended on B at
/// (0, -200), for the collision passes to end.
fn ending_on_b() -> Shot {
    let at = Vec2::new(0.0, -200.0);
    Shot {
        position: at,
        moved_from: Some(at - Vec2::new(10.0, 0.0)),
        age: 1,
        ..Shot::launch(
            WeaponSpec::new(&blaster()),
            C,
            C,
            &ShipState::default(),
            90.0,
        )
    }
}

/// The slots of the shots in flight, as kept.
fn slots(combat: &Combat) -> Vec<usize> {
    combat.shots().iter().map(|shot| shot.slot).collect()
}

#[test]
fn a_shot_takes_the_first_free_slot_and_the_shots_are_kept_in_slot_order() {
    let mut combat = Combat::default();
    for _ in 0..3 {
        assert!(combat.put_in_flight(idle()).is_some());
    }
    assert_eq!(slots(&combat), [0, 1, 2]);
    let ended = combat.shots.remove(1);
    combat.slots.hold(ended.slot);
    let mut next = idle();
    next.id = ShotId(7);
    combat.put_in_flight(next);
    assert_eq!(slots(&combat), [0, 2, 3], "1 still held");
    combat.slots.free(1);
    next.id = ShotId(8);
    combat.put_in_flight(next);
    assert_eq!(slots(&combat), [0, 1, 2, 3], "the freed lower slot");
    assert_eq!(combat.shots()[1].id, ShotId(8));
}

#[test]
fn a_shot_fired_after_a_lower_slot_is_freed_takes_it() {
    // C's shot in slot 0 ends on B in the collision passes; `_HandleShot`
    // frees it (@0x3617c) before A's pick (`_HandleShip` @0x44483), whose
    // shot takes slot 0, under the two idle shots, not slot 3.
    for capped in [false, true] {
        let mut combat = Combat::default();
        if capped {
            combat.set_combat_caps(RuleSource::Engine);
        }
        combat.put_in_flight(ending_on_b());
        combat.put_in_flight(idle());
        combat.put_in_flight(idle());
        let mut ships = [
            Ship::at(1, 0.0, 0.0).armed(blaster()),
            Ship::at(2, 0.0, -200.0),
        ];
        tick(&mut combat, &mut ships, &NovaDisable);
        assert!(ships[1].reserves.shield.now < 10.0, "B struck");
        assert_eq!(slots(&combat), [0, 1, 2], "{capped}");
        assert_eq!(combat.shots()[0].firer, A, "{capped}");
    }
}

#[test]
fn a_spent_shots_slot_is_held_through_the_npcs_point_defence_and_freed_by_handle_shot() {
    let mut combat = Combat::default();
    combat.put_in_flight(idle());
    combat.put_in_flight(ending_on_b());
    let mut ships = [Ship::at(1, 0.0, 0.0), Ship::at(2, 0.0, -200.0)];
    let govts = Governments::default();
    let arsenal = Arsenal::default();
    let arena = Arena {
        pass: GamePass::Advance { double_time: false },
        arsenal: &arsenal,
        govts: &govts,
        rules: Rules::default(),
        field: Field::default(),
    };
    let mut fighters: Vec<Fighter> = ships.iter_mut().map(Ship::fighter).collect();
    combat.test_hits(&mut fighters, arena, Scenery::default(), &mut NeverFires);
    combat.defend_npcs(&mut fighters, arena, &mut NeverFires);
    assert_eq!(slots(&combat), [0], "spent");
    assert!(combat.slots.is_held(1), "held through point defence");
    assert_eq!(combat.first_free_slot(), Some(2));
    combat.fly_shots(&mut fighters, arena, &mut NeverFires);
    assert!(!combat.slots.is_held(1), "freed at its visit");
    assert_eq!(combat.first_free_slot(), Some(1));
}

/// A shot of C's in `layer`, numbered `id` in firing order.
fn in_layer(layer: ShotLayer, id: u32) -> Shot {
    Shot {
        layer,
        id: ShotId(id),
        ..idle()
    }
}

/// A combat whose shot layers overlap as `source` reads them.
fn overlapping(source: RuleSource) -> Combat {
    let mut combat = Combat::default();
    combat.set_shot_overlap(source);
    combat
}

/// The numbers of the shots in the order they are drawn.
fn drawn(combat: &Combat) -> Vec<u32> {
    combat.shots_drawn().iter().map(|shot| shot.id.0).collect()
}

/// Ends and frees the shot in `slot`, as `_HandleShot`'s visit does.
fn end_and_free(combat: &mut Combat, slot: usize) {
    combat.shots.retain(|shot| shot.slot != slot);
}

#[test]
fn a_reused_slot_whose_sprite_is_in_the_layer_keeps_its_place() {
    // X (0) takes slot 0 and Y (1) slot 1, both moved to the homing
    // layer's tail; X ends, and Z (2) takes slot 0, already in the layer
    // (@0x768c): drawn under Y, though fired after it.
    for (source, order) in [(RuleSource::Engine, [2, 1]), (RuleSource::Bible, [1, 2])] {
        let mut combat = overlapping(source);
        combat.put_in_flight(in_layer(ShotLayer::Homing, 0));
        combat.put_in_flight(in_layer(ShotLayer::Homing, 1));
        end_and_free(&mut combat, 0);
        combat.put_in_flight(in_layer(ShotLayer::Homing, 2));
        assert_eq!(combat.shots()[0].id, ShotId(2), "slot 0");
        assert_eq!(drawn(&combat), order, "{source:?}");
    }
}

#[test]
fn a_reused_slot_from_another_layer_goes_to_the_tail() {
    for source in RuleSource::ALL {
        let mut combat = overlapping(source);
        combat.put_in_flight(in_layer(ShotLayer::Shots, 0));
        combat.put_in_flight(in_layer(ShotLayer::Homing, 1));
        end_and_free(&mut combat, 0);
        combat.put_in_flight(in_layer(ShotLayer::Homing, 2));
        assert_eq!(drawn(&combat), [1, 2], "{source:?}");
    }
}

#[test]
fn the_layers_are_drawn_bottom_first() {
    for source in RuleSource::ALL {
        let mut combat = overlapping(source);
        combat.put_in_flight(in_layer(ShotLayer::OverShips, 0));
        combat.put_in_flight(in_layer(ShotLayer::Shots, 1));
        combat.put_in_flight(in_layer(ShotLayer::PointDefence, 2));
        combat.put_in_flight(in_layer(ShotLayer::Homing, 3));
        assert_eq!(drawn(&combat), [2, 3, 1, 0], "{source:?}");
    }
}

#[test]
fn a_point_defence_shot_fired_at_a_ship_goes_to_the_shot_layers_tail() {
    // Guns in slots 0..2; slot 0's ends. The point-defence shot at the
    // fighter takes slot 0, is put in point defence's layer and then moved
    // to the shot layer (`_HandleShipPointDefense` @0x398bd): at its tail,
    // over slots 1 and 2, where a gun in slot 0 would be under them.
    for source in RuleSource::ALL {
        let mut combat = overlapping(source);
        for id in 0..3 {
            combat.put_in_flight(in_layer(ShotLayer::Shots, id));
        }
        end_and_free(&mut combat, 0);
        combat.next_shot = 10;
        let mut ships = fighter_attack(quad(), 0.0, -200.0);
        tick(&mut combat, &mut ships, &NovaDisable);
        let fired = pd_shots(&combat);
        assert_eq!(fired.len(), 1, "{:?}", combat.shots());
        assert_eq!((fired[0].slot, fired[0].layer), (0, ShotLayer::Shots));
        let pd = fired[0].id.0;
        assert_eq!(drawn(&combat), [1, 2, pd], "{source:?}");
    }
    let mut combat = Combat::default();
    for id in 0..3 {
        combat.put_in_flight(in_layer(ShotLayer::Shots, id));
    }
    end_and_free(&mut combat, 0);
    combat.put_in_flight(in_layer(ShotLayer::Shots, 3));
    assert_eq!(drawn(&combat), [3, 1, 2], "a gun keeps slot 0's place");
}
